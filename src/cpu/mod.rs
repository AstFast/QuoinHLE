//! Guest CPU: an ARMv5TE executor in interpreter form.
//!
//! Two states are modelled because `--apcs=/interwork` lets a single module mix them. The
//! executor deliberately knows nothing about BREW: when the guest executes an `SVC` the run
//! loop hands control back to the caller with [`Stop::Hle`], and the caller is free to mutate
//! registers and memory before resuming. That seam is what lets the same images run unchanged
//! on a translating backend later.
//!
//! No pipeline is modelled. Instead `pc` always points at the *next* instruction to fetch, so
//! an operand that names R15 reads `pc + 4` (ARM) or `pc + 2` (Thumb) — the same +8/+4 bias a
//! real ARM9 core presents.

pub mod arm;
pub mod backend;
#[cfg(feature = "jit")]
pub mod jit;
pub mod thumb;

use crate::mem::{Fault as MemFault, Mem};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// How often the run loop looks at [`Cpu::cancel`], in retired instructions.
///
/// A load per instruction was not chosen: this is the hottest loop in the emulator, and one more load
/// per instruction is exactly the kind of addition that costs here. At 65 536 the gap between
/// asking and stopping is about a millisecond of guest work at the measured 60-86 M instr/s, which is
/// far below what a person perceives as "it is not answering", and a run that never arms a flag pays
/// one comparison against a value that is never reached.
pub const CANCEL_LOOK: u64 = 1 << 16;
/// How many `--sample` entries a run keeps: the tail, at the same 4 096 the other bounded records in
/// this host use. At the smallest interval this host accepts (64 instructions) that is a window of 256 k
/// instructions — a few frames of a guest pack, which is what "where did it stop" is about.
pub const SAMPLE_CAP: usize = 4096;

pub const FLAG_N: u32 = 1 << 31;
pub const FLAG_Z: u32 = 1 << 30;
pub const FLAG_C: u32 = 1 << 29;
pub const FLAG_V: u32 = 1 << 28;
pub const FLAG_T: u32 = 1 << 5;

pub type CResult<T> = Result<T, CpuFault>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    /// A word the decoder refuses to guess at, with its encoding.
    Illegal { word: u32 },
    /// Fetch from an unmapped address.
    UnmappedFetch { addr: u32 },
    /// A defined instruction we have not implemented yet.
    Unimplemented { group: &'static str, word: u32 },
    /// Data abort surfaced as a fault so the caller can report both PC and address.
    Data { addr: u32, reason: MemFault },
}

#[derive(Debug, Clone, Copy)]
pub struct CpuFault {
    pub pc: u32,
    pub thumb: bool,
    pub kind: FaultKind,
}

impl std::fmt::Display for CpuFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {:#010x} ({})",
            match self.kind {
                FaultKind::Illegal { word } => format!("illegal instruction {word:#010x}"),
                FaultKind::UnmappedFetch { addr } => format!("instruction fetch from {addr:#010x}"),
                FaultKind::Unimplemented { group, word } =>
                    format!("unimplemented {group} {word:#010x}"),
                FaultKind::Data { addr, ref reason } =>
                    format!("data abort at {addr:#010x}: {reason}"),
            },
            self.pc,
            if self.thumb { "Thumb" } else { "ARM" }
        )
    }
}

impl std::error::Error for CpuFault {}

/// What `--jit-arena` — or the game library's `QUOIN_JIT_ARENA` — asked the compiled-code ceiling to
/// be. `None` wherever this is carried means "nobody asked", which is the backend's own default.
///
/// The value is parsed here, in a module that compiles under every feature combination, because a
/// build with no backend still has to *refuse* the flag at the point it is read: taking it and then
/// discovering it means nothing once the pack has loaded is the "accepted, then silent" shape this
/// CLI is built to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitArena {
    /// A block count, or `0` for no ceiling at all.
    Blocks(usize),
    /// Sized from the modules' own image bytes; see [`jit_arena_blocks`].
    Auto,
}

/// The largest ceiling worth accepting: past this the count cannot be reached before the code arena
/// itself is full, so a bigger number would be a promise the backend cannot keep.
const JIT_ARENA_MAX_ASK: usize = 1 << 20;

/// Bytes of module image to charge per compiled block when the ceiling is sized from the code
/// (`jit_arena_blocks`).
///
/// Four, because that is the instruction width: a block is keyed by the address it starts at, an
/// image of `n` bytes therefore holds at most `n / 4` distinct ARM entry points, and so this is an
/// **upper bound** rather than a prediction — no ARM-state pack can want more blocks than the
/// estimate hands it, which means the ceiling can never be the thing that flushes the cache.
///
/// A Thumb module is the one shape this under-sizes (two-byte instructions, twice the bound); no pack
/// in this sample set is driven Thumb-first — the known Thumb case, cod5, does not run — and
/// `--jit-arena N` is the answer for one that is.
///
/// The bound is loose on real code, deliberately. `pvz.mod` is 351 152 bytes and reports **2 563**
/// blocks after five minutes of virtual time at its own title screen (`QUOIN_STATS` over
/// `--serve`) — 34× below the 87 788 the bound allows. A prediction of
/// what a pack *will* use would need a session of that pack, and an estimate that lands low is the
/// expensive error: the old 16384-block default flushed again on every pass, 3.2 s at 17 000 blocks.
const ARENA_BYTES_PER_BLOCK: u64 = 4;

/// Floor for the same estimate. A 5 KiB pack computes 1 129 blocks, and its boot, its menu and its
/// first scene are far more entry points than its size suggests; the floor costs at most ~700 KiB
/// of the code arena in the worst case (~170 bytes a block).
const ARENA_FLOOR_BLOCKS: usize = 4096;

impl JitArena {
    /// Parse `auto`, `0` (no ceiling) or a block count. `--jit-arena` and `QUOIN_JIT_ARENA` are the
    /// same choice in two places, so they are read by the same code.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("auto") {
            return Ok(JitArena::Auto);
        }
        let n: usize = text
            .parse()
            .map_err(|_| format!("wants a block count or `auto`, got {text:?}"))?;
        if n > JIT_ARENA_MAX_ASK {
            return Err(format!(
                "{n} blocks is past anything the 32 MiB code arena can hold (about 190 000 at the \
                 measured ~170 bytes a block) — use 0 for no ceiling at all"
            ));
        }
        Ok(JitArena::Blocks(n))
    }
}

/// How many compiled blocks to allow for `code_bytes` of module image: one per
/// `ARENA_BYTES_PER_BLOCK` bytes, floored at `ARENA_FLOOR_BLOCKS` and never above `ceiling` (the
/// backend's own default, so an estimate cannot hand out more than a run that never asked for this).
///
/// What it is for: the measured packs in this tree run from 4.5 KiB to 351 KiB of image, and the
/// fixed 65536-block default is sized for the largest of them — a small pack is given ten times the
/// room it can use, and the ceiling is the only thing bounding how much of the 32 MiB code arena is
/// written before a flush.
pub fn jit_arena_blocks(code_bytes: u64, ceiling: usize) -> usize {
    let wanted = code_bytes / ARENA_BYTES_PER_BLOCK;
    let wanted = usize::try_from(wanted).unwrap_or(usize::MAX);
    wanted.clamp(ARENA_FLOOR_BLOCKS.min(ceiling), ceiling)
}

/// Why the run loop gave back control.
#[derive(Debug, Clone, Copy)]
pub enum Stop {
    /// The guest branched to the sentinel the loader parked in the return address.
    Halted {
        addr: u32,
    },
    /// The guest executed `SVC #id`; the HLE layer owns what happens next.
    Hle {
        id: u32,
        at: u32,
    },
    Fault(CpuFault),
    /// Instruction budget ran out; resume by calling `run` again.
    Budget {
        steps: u64,
    },
    /// Somebody outside the guest asked this run to stop, and [`Cpu::cancel`] was the channel they
    /// used. Deliberately *not* a [`Stop::Stopped`]: those are "the run did the thing you named", and
    /// everything downstream reports them as a diagnosis worth reading. This one says the session is
    /// being ended — a player closed the window, the harness asked for the report — and `pc` is not an
    /// accusation but the answer to the question a hang actually poses: where was it spinning.
    Cancelled {
        pc: u32,
    },
    /// The run stopped where it was asked to and stayed put, so the state that made the address
    /// interesting is still live — which is the whole difference between stopping here and finding out
    /// about it afterwards.
    Stopped {
        why: StopWhy,
    },
}

/// Why a run stopped where it was asked to.
///
/// Two ways in, one shape out. `--watch-stop` stops on a *data* access and `--break` stops on an
/// *instruction*; once the machine has stopped, the two are the same event to everything downstream —
/// a set of registers worth reading. Keeping them apart here and together everywhere else is what stops
/// the report from growing a branch per reason for stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopWhy {
    /// A byte being watched was touched.
    Watch(u32),
    /// The program counter reached an address `--break` named. The instruction at it has *not* run yet:
    /// a breakpoint is a place to stand before something happens, not after.
    Break(u32),
}

impl StopWhy {
    /// The address that was named, whichever kind of stop it was.
    pub fn addr(&self) -> u32 {
        match self {
            StopWhy::Watch(a) | StopWhy::Break(a) => *a,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

/// What a run was asked to print of the guest's own instructions.
///
/// The two knobs are the two questions that actually get asked. "What does it do *there*" is a range —
/// something branched somewhere unexpected and the code at the far end is the subject. "How much of
/// this can I read" is a limit, because a run that goes wrong early fills a terminal faster than anyone
/// can read it, and a trace still scrolling at the point of interest has told nobody anything.
#[derive(Debug, Clone, Copy)]
pub struct Trace {
    /// Print only instructions whose address lies in this half-open span. `None` is all of them.
    pub range: Option<(u32, u32)>,
    /// Stop printing after this many lines. Zero is what "no trace was asked for" looks like.
    pub limit: u64,
    /// How many lines have gone out, so that the limit is a limit rather than a hope. Whoever set the
    /// limit prints this when it stops, because a trace that ends in silence cannot be told apart from
    /// a run where nothing more happened.
    pub printed: u64,
}

impl Default for Trace {
    fn default() -> Self {
        Trace {
            range: None,
            limit: 0,
            printed: 0,
        }
    }
}

impl Trace {
    /// Everything, up to `limit` lines — what a bare `--trace` means.
    pub fn all(limit: u64) -> Self {
        Trace {
            range: None,
            limit,
            printed: 0,
        }
    }

    /// One span of addresses, up to `limit` lines.
    pub fn in_range(lo: u32, hi: u32, limit: u64) -> Self {
        Trace {
            range: Some((lo, hi)),
            limit,
            printed: 0,
        }
    }

    /// Whether any instruction at all is being traced.
    pub fn on(&self) -> bool {
        self.limit != 0
    }

    /// Whether this instruction is one of the ones that were asked about.
    #[inline]
    pub fn wants(&self, pc: u32) -> bool {
        if self.printed >= self.limit {
            return false;
        }
        match self.range {
            Some((lo, hi)) => pc >= lo && pc < hi,
            None => true,
        }
    }
}

/// Where the guest was executing, taken at a fixed instruction interval.
///
/// A pack that hangs is asked a question a trace cannot answer: `--trace` prints what happened from the
/// first instruction out, which for a run that played twenty minutes before stalling is both unreadable
/// and gone, and `--break` *stops* the run — a person who wants to keep playing cannot arm one. So this
/// records nothing but the program counter, every `every` instructions, into a ring that keeps the last
/// `cap` of them. What comes out at the end is "where was it, at the end": an address the reader can
/// disassemble, and a spread that separates a pack spinning inside one function from one that keeps
/// returning to the shell and simply never gets what it is waiting for.
#[derive(Debug)]
pub struct Samples {
    /// Instructions between two samples. Zero is not armed, and the run's inner loop compares this
    /// first so an unarmed run pays one load and no write.
    pub every: u64,
    /// The instruction count at which the next sample is due.
    next: u64,
    /// Oldest first, so the last entry is the last thing seen.
    pub ring: VecDeque<Sample>,
    /// How many entries to keep. A frozen pack is answered by its tail, and keeping more would only
    /// cost memory the run does not need.
    pub cap: usize,
}

/// One sample: the instruction that retired, in the state it retired in.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub pc: u32,
    pub thumb: bool,
    /// The run's instruction count at it, so two addresses far apart in the ring can be told apart by
    /// how much time lay between them.
    pub at: u64,
}

impl Default for Samples {
    fn default() -> Self {
        Samples {
            every: 0,
            next: u64::MAX,
            ring: VecDeque::new(),
            cap: 4096,
        }
    }
}

impl Samples {
    /// Sample every `every` instructions, keeping the last `cap`.
    pub fn armed(every: u64, cap: usize) -> Self {
        Samples {
            every,
            next: every,
            ring: VecDeque::new(),
            cap,
        }
    }

    /// Take a sample if this instruction is due for one. Called once per retired instruction.
    #[inline]
    pub fn note(&mut self, steps: u64, pc: u32, thumb: bool) {
        if self.every == 0 || steps < self.next {
            return;
        }
        self.next = steps.saturating_add(self.every);
        if self.ring.len() == self.cap {
            self.ring.pop_front();
        }
        self.ring.push_back(Sample {
            pc,
            thumb,
            at: steps,
        });
    }

    /// The window folded to `(address, state) -> how many samples`, busiest first and ties by ascending
    /// address so the answer does not depend on hash order. Kept apart from the printing because the
    /// claim worth testing is this ordering, not the words wrapped around it.
    pub fn hottest(&self) -> Vec<((u32, bool), usize)> {
        let mut counts: std::collections::HashMap<(u32, bool), usize> =
            std::collections::HashMap::new();
        for x in self.ring.iter() {
            *counts.entry((x.pc, x.thumb)).or_insert(0) += 1;
        }
        let mut hot: Vec<((u32, bool), usize)> = counts.into_iter().collect();
        hot.sort_by(|a, b| b.1.cmp(&a.1).then(a.0 .0.cmp(&b.0 .0)));
        hot
    }
}

/// One `--break` request.
///
/// Stopping at the *first* arrival is the right default, but the calls that explain a pack are usually
/// the second or the third: a function that runs once per record in a table only says anything on the
/// interesting one. So a request carries which arrival it waits for and counts the ones it passed, and
/// the count lives beside the address for the same reason the address does — this is what has the
/// program counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Break {
    pub addr: u32,
    /// Stop at this many-th arrival. 1 is "the first", which is what a bare `--break ADDR` asks for.
    pub after: u32,
    /// Arrivals so far, including the one that stopped the run.
    pub seen: u32,
    /// Stop only at an arrival where a register holds this value.
    ///
    /// `after` counts arrivals blindly, which is the wrong question when a function runs thousands of
    /// times and exactly one of those calls is the one that failed: binary-searching the index costs a
    /// run per step, and a pack's startup is eight million instructions. So the request carries the
    /// register state instead. `None` in every run that did not name one.
    pub when: Option<RegTest>,
}

/// `reg == val`, evaluated on the registers as they stand *before* the instruction at the breakpoint runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegTest {
    /// Architectural register number, 0..14 — `r0`…`r12`, `sp`, `lr`. R15 is excluded: a condition on
    /// the program counter is a different address, which `--break` already takes.
    pub reg: u32,
    pub val: u32,
}

impl Break {
    /// Whether this arrival is the one asked for: at least `after` arrivals deep, and holding the value
    /// named if one was named. Takes the register file rather than the whole machine because the caller
    /// is already holding it mutably.
    fn is_awaited(&self, r: &[u32; 15]) -> bool {
        self.seen >= self.after.max(1)
            && self
                .when
                .is_none_or(|t| r.get(t.reg as usize).copied() == Some(t.val))
    }
}

#[derive(Debug)]
pub struct Cpu {
    /// r0..r12, r13=sp, r14=lr. r15 lives in [`Cpu::pc`].
    pub r: [u32; 15],
    pub cpsr: u32,
    /// Saved status register; modelled but never entered, since we do not raise interrupts.
    pub spsr: u32,
    pub pc: u32,
    pub mem: Mem,
    pub steps: u64,
    /// Addresses `--break` asked the run to stop at, before the instruction at them runs.
    ///
    /// Kept here rather than in the front end because this is what has the program counter: a breakpoint
    /// the front end held would need the front end to step the machine one instruction at a time, which is
    /// exactly the arrangement the whole `SVC` seam exists to avoid. Empty in every ordinary run, so the
    /// test for one is a length check.
    pub break_at: Vec<Break>,
    /// What `--trace` was asked to print, if anything.
    ///
    /// Every retired instruction is the single most useful debugging facility in this project — an ABI
    /// mistake shows up in it immediately — and it is also tens of thousands of lines for a reference
    /// pack's startup, which is more than anyone reads. So the request carries *which* instructions and
    /// *how many*, and a run that prints none of them is the default rather than a special case.
    pub trace: Trace,
    /// Where the guest was, taken every so often — see [`Samples`]. Unarmed in every ordinary run, and
    /// the inner loop's check for that is one load.
    pub samples: Samples,
    /// Counts SVCs taken, so a test can assert the guest really reached the HLE seam.
    pub hle_calls: u64,
    /// Set by whoever wants to be able to stop this machine from outside it, and read once every
    /// [`CANCEL_LOOK`] retired instructions.
    ///
    /// It has to live here rather than in the session loop for the one case it exists for: a guest
    /// function that never returns. A caller that only checks a flag *between* visits to the host can
    /// wait forever on a pack spinning inside one function, which is precisely the freeze being
    /// removed. `None` — which is every headless run and every test that does not arm one — makes the
    /// loop's second deadline `u64::MAX`, so nothing is loaded and nothing is compared except the
    /// `min` the budget check already does.
    pub cancel: Option<Arc<AtomicBool>>,
    /// The instruction count at which [`Cpu::cancel`] is worth loading again.
    cancel_at: u64,
    /// Optional publication of how far the guest has got, for a window that has to tell
    /// "still computing" from "stopped". A turn's [`crate::session::Status`] is written when the turn
    /// *ends*, so a guest that spends a second inside one function leaves the window with nothing to
    /// read but a frozen picture. Written once per [`CANCEL_LOOK`] retired instructions at the same
    /// checkpoints `cancel` already uses — never per instruction — and armed only by a windowed run.
    /// It is a host-side observation channel: it changes no guest state and is not part of a snapshot.
    pub progress: Option<Arc<AtomicU64>>,
    /// The instruction count at which [`Cpu::progress`] is worth publishing again.
    progress_at: u64,
    /// How instructions are carried out: the interpreter, or a translator a caller installed.
    ///
    /// `None` only while [`Cpu::run`] has it taken out for the duration of one turn — see there.
    /// Every `Cpu` leaves [`Cpu::new`] with the interpreter installed; a build with the `jit` feature
    /// swaps the translator in through `Cpu::enable_jit`.
    pub backend: Option<Box<dyn backend::Backend>>,
}

impl Cpu {
    pub fn new(mem: Mem) -> Self {
        #[allow(unused_mut)]
        let mut cpu = Cpu {
            r: [0; 15],
            cpsr: FLAG_Z | FLAG_C,
            spsr: 0,
            pc: 0,
            mem,
            steps: 0,
            break_at: Vec::new(),
            trace: Trace::default(),
            samples: Samples::default(),
            hle_calls: 0,
            cancel: None,
            cancel_at: u64::MAX,
            progress: None,
            progress_at: u64::MAX,
            backend: Some(Box::new(backend::Interpreter)),
        };
        // Library code reads no environment variable: the backend is turned on explicitly by the
        // caller through `Cpu::enable_jit()`, so semantics do not drift with the environment. The
        // `QUOIN_JIT` switch the integration tests use lives in tests/.
        cpu
    }

    /// Install a backend, replacing whatever is there.
    ///
    /// This is the door an embedder uses, and the reason it exists rather than "write the field":
    /// the field is `pub`, so writing it works, but a method can be documented, deprecated and
    /// tested, and a field name is a promise nobody can change. Between the two, only one of them is
    /// an interface.
    ///
    /// Whatever is displaced is dropped, which for a translating backend means its warm code cache
    /// goes with it. Install once, before the first [`Cpu::run`].
    ///
    /// # Which backends fit
    ///
    /// The contract is [`backend::Backend`], and the module docs there list what an implementation
    /// owes. A CPU emulator from another project — QEMU's TCG behind a C ABI, Unicorn, an ARM64
    /// dynarec — fits if it can carry out guest instructions against this crate's `Mem` and stop at
    /// the boundaries [`crate::cpu::Stop`] names. One that insists on owning the guest's address
    /// space has to mirror it, page by page, on [`Mem::window_generation`] changes; one that only
    /// speaks whole programs rather than slices does not fit at all.
    pub fn set_backend(&mut self, backend: Box<dyn backend::Backend>) {
        self.backend = Some(backend);
    }

    /// What the installed backend calls itself — for a report, or an embedder's own diagnostics.
    pub fn backend_name(&self) -> &'static str {
        self.backend.as_ref().map_or("none", |b| b.name())
    }

    /// How many blocks the installed backend wants to compile per turn, when it has an opinion.
    ///
    /// Asked once by `Runtime::new`; the interpreter answers `None`, which is the same "nothing to
    /// spend" a build without a translating backend used to get from a `#[cfg]`.
    pub fn default_compile_budget(&self) -> Option<u32> {
        self.backend
            .as_ref()
            .and_then(|b| b.default_compile_budget())
    }

    /// Number of blocks in the translating backend's code cache; `None` when it has no cache.
    ///
    /// Diagnostic: one call walks Rynarmic's 65536 cache slots, so it has no business on the
    /// per-instruction hot path. It lives here rather than in `jit.rs` because `session` is compiled
    /// unconditionally and needs an entry point that exists under every feature combination.
    pub fn jit_cached_blocks(&self) -> Option<usize> {
        self.backend.as_ref().and_then(|b| b.cached_blocks())
    }

    /// How many new blocks this turn may still compile; `None` means no limit.
    ///
    /// `Runtime::pump` sets it from `jit_compile_budget` at the top of every turn and restores "no
    /// limit" at the end, which spreads "a scene change wants a few thousand blocks" over several
    /// turns: each turn pays at most a frame's worth of compile time, blocks over the budget go
    /// through the interpreter this turn and are compiled on the next. A no-op when the JIT is off.
    pub fn set_jit_compile_budget(&mut self, budget: Option<u32>) {
        if let Some(backend) = self.backend.as_mut() {
            backend.set_compile_budget(budget);
        }
    }

    /// Size the compiled-code ceiling from a parsed `--jit-arena` / `QUOIN_JIT_ARENA` choice, and
    /// answer the line a caller should log for it.
    ///
    /// Call it **after** the modules are loaded — the `auto` estimate reads their sizes — and before
    /// the guest runs: Rynarmic's `set_jit_arena_limit` flushes the block cache, so applying it late
    /// costs exactly the recompiles it was meant to save.
    ///
    /// `None` means there was nothing to size: a build with no backend, or a run that did not arm it.
    /// That is the same silence `--no-jit` already gives `--jit-budget`, and the flag parsing is what
    /// refuses the combination rather than this.
    pub fn size_jit_arena(&mut self, choice: JitArena, code_bytes: u64) -> Option<String> {
        self.backend
            .as_mut()
            .and_then(|b| b.size_arena(choice, code_bytes))
    }

    /// How many spans of the last published fast-access window set were non-empty; `None` when the
    /// JIT is off or the build has no such feature.
    ///
    /// Diagnostic (`win N` in `QUOIN_STATS`): the guest address space is cut at 1 MiB, so an image, a
    /// heap of a few MiB and the stack already exceed four spans, and when there are not enough slots
    /// the **stack is what gets pushed out** — after which every push and pop on it goes back into a
    /// Rust callback (measured 1.88 ns/instruction against 0.79). This number dropping is the signal
    /// that it happened.
    pub fn jit_windows(&self) -> Option<usize> {
        self.backend.as_ref().and_then(|b| b.windows())
    }

    #[inline(always)]
    pub fn thumb(&self) -> bool {
        self.cpsr & FLAG_T != 0
    }

    #[inline]
    pub fn set_thumb(&mut self, on: bool) {
        if on {
            self.cpsr |= FLAG_T;
        } else {
            self.cpsr &= !FLAG_T;
        }
    }

    #[inline]
    pub fn flag(&self, bit: u32) -> bool {
        self.cpsr & bit != 0
    }

    #[inline]
    pub fn set_flag(&mut self, bit: u32, on: bool) {
        self.cpsr = if on {
            self.cpsr | bit
        } else {
            self.cpsr & !bit
        };
    }

    #[inline(always)]
    pub fn carry(&self) -> bool {
        self.cpsr & FLAG_C != 0
    }

    #[inline(always)]
    pub fn set_nz(&mut self, v: u32) {
        self.cpsr =
            (self.cpsr & !(FLAG_N | FLAG_Z)) | if v == 0 { FLAG_Z } else { 0 } | (v & FLAG_N);
    }

    /// Operand fetch. Naming R15 reads the biased program counter.
    #[inline(always)]
    pub fn reg(&self, n: u32) -> u32 {
        match n & 0xf {
            15 => self.pc + if self.thumb() { 2 } else { 4 },
            i => self.r[i as usize],
        }
    }

    /// Write an architectural register; a write to R15 becomes a branch.
    #[inline(always)]
    pub fn set_reg(&mut self, n: u32, v: u32) {
        let n = n & 0xf;
        if n == 15 {
            self.branch_to(v, false);
        } else {
            self.r[n as usize] = v;
        }
    }

    /// Sized load. `pc` is the address of the instruction being executed, carried through so a
    /// data abort names its cause instead of just its address.
    pub(crate) fn load(&self, pc: u32, addr: u32, size: u32) -> Result<u32, Stop> {
        let result = match size {
            1 => self.mem.read_u8(addr).map(u32::from),
            2 => self.mem.read_u16(addr),
            _ => self.mem.read_u32(addr),
        };
        result.map_err(|e| self.mem_stop(pc, addr, e))
    }

    /// Sized store. ARM halfword and byte stores simply discard the upper bits.
    pub(crate) fn store(&mut self, pc: u32, addr: u32, size: u32, value: u32) -> Result<(), Stop> {
        let result = match size {
            1 => self.mem.write_u8(addr, value as u8),
            2 => self.mem.write_u16(addr, value),
            _ => self.mem.write_u32(addr, value),
        };
        result.map_err(|e| self.mem_stop(pc, addr, e))
    }

    /// `BX`/`BLX` honour bit 0 as the state selector; plain R15 writes in ARM state ignore it.
    #[inline]
    pub fn branch_to(&mut self, v: u32, interwork: bool) {
        if interwork {
            self.set_thumb(v & 1 != 0);
            self.pc = v & !1;
        } else if self.thumb() {
            self.pc = v & !1;
        } else {
            self.pc = v & !3;
        }
    }

    /// Hand this machine a way to be stopped from outside. See [`Cpu::cancel`].
    pub fn arm_cancel(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_at = self.steps.saturating_add(CANCEL_LOOK);
        self.cancel = Some(flag);
    }

    /// Let the outside world watch guest progress *while* one guest function is still running.
    ///
    /// [`Cpu::arm_cancel`] is the other direction: it lets a window stop the machine. This one lets it
    /// say why a picture has not moved — the guest is still retiring instructions, or it is not. The
    /// value published is the retired-instruction count, not a clock, so a reader can compare two
    /// samples without agreeing on a time base.
    pub fn arm_progress(&mut self, out: Arc<AtomicU64>) {
        out.store(self.steps, Ordering::Relaxed);
        self.progress = Some(out);
        self.progress_at = self.steps.saturating_add(CANCEL_LOOK);
    }

    /// Publish the progress counter, if one is due.
    ///
    /// Kept out of line so the quiet loop's comparison stays a single `min` and a branch; a windowed
    /// run pays one relaxed store per 65 536 instructions.
    ///
    /// Public for the same reason [`Cpu::run_interpreter`] is: a backend that runs its own slice
    /// loop owns the checkpoints inside it, and this is the one place that knows both when a
    /// publication is due and how often. It is cheap to call and does nothing when nothing is due —
    /// but call it at the rhythm [`CANCEL_LOOK`] describes, not per instruction.
    #[inline]
    pub fn checkpoint_progress(&mut self) {
        if self.steps >= self.progress_at {
            match &self.progress {
                Some(out) => {
                    out.store(self.steps, Ordering::Relaxed);
                    self.progress_at = self.steps.saturating_add(CANCEL_LOOK);
                }
                None => self.progress_at = u64::MAX,
            }
        }
    }

    /// Whether this run has to be interpreted one instruction at a time.
    ///
    /// Trace, breakpoints, watchpoints and sampling all start from the premise that every
    /// instruction is seen, which is the opposite of what a block cache assumes. A machine with any
    /// of them armed therefore never reaches a translating backend, whatever is installed — that is
    /// the rule this method exists to keep in one place rather than in each backend.
    fn per_instruction(&self) -> bool {
        !(self.break_at.is_empty()
            && self.mem.watch.is_empty()
            && self.mem.watch_tripped.get().is_none()
            && self.samples.every == 0
            && !self.trace.on())
    }

    /// Execute one turn through whichever backend is installed.
    ///
    /// Two things happen before the backend is asked. A run with diagnostics armed goes to the
    /// interpreter directly — see [`Cpu::per_instruction`]. Otherwise the backend is taken out of
    /// its box for the duration of the call: it has to be handed `&mut Cpu`, and it lives inside
    /// that same `Cpu`, which is the one borrow this type cannot express.
    ///
    /// The swap costs one pointer move per *entry into the backend*, which is once per turn in the
    /// quiet case and once per HLE call in a busy frame — thousands a frame, not millions. What it
    /// is deliberately not is per instruction: `run_interpreter`'s inner loop and the backend's own
    /// slice loop never touch the box.
    pub fn run(&mut self, budget: u64) -> Stop {
        if self.per_instruction() {
            return self.run_interpreter(budget);
        }
        let Some(mut backend) = self.backend.take() else {
            // Unreachable in a run that has not panicked: the box is empty only while this function
            // holds it. A panic inside a backend leaves it empty, and answering by interpreting is
            // the one recovery that keeps a machine usable instead of spinning on `Budget { 0 }`.
            debug_assert!(false, "a Cpu always carries a backend");
            return self.run_interpreter(budget);
        };
        let stop = backend.run(self, budget);
        self.backend = Some(backend);
        stop
    }

    /// Execute until the guest stops, traps, exhausts `budget`, or is cancelled — the interpreter's
    /// own turn.
    ///
    /// Public because a [`backend::Backend`] is allowed to be a *wrapper*: one that measures, one
    /// that decides per slice, one that forwards a stretch its own engine has no support for. Every
    /// such backend needs an answer to "carry these instructions some other way", and this is that
    /// answer — the same loop [`Cpu::run`] would have run with the interpreter installed.
    ///
    /// **Not** the entry a front end should call. Ask [`Cpu::run`]; the installed backend decides.
    /// Calling this directly walks past the backend box, which is right inside an impl of that trait
    /// and wrong anywhere else — a caller that does it silently disables whatever the embedder
    /// installed.
    pub fn run_interpreter(&mut self, budget: u64) -> Stop {
        // The ordinary windowed/test run has no breakpoints, watchpoints, samples or trace. The
        // diagnostic path below deliberately checks all four once per instruction; keep that cost
        // out of the quiet path, which is what a guest pack actually executes.
        if !self.per_instruction() {
            return self.run_quiet(budget);
        }
        let limit = self.steps.saturating_add(budget);
        loop {
            // Two deadlines, one comparison: the budget the caller set, and — only for a machine that
            // was handed a cancel flag — the moment that flag is due to be loaded again. With nothing
            // armed the second is `u64::MAX`, so `deadline` is just `limit` and the arms below are
            // never taken.
            let deadline = limit.min(self.cancel_at).min(self.progress_at);
            if self.steps >= deadline {
                if self.steps >= limit {
                    return Stop::Budget { steps: self.steps };
                }
                self.checkpoint_progress();
                // The deadline may have fired for `progress_at` alone, so the cancel flag is loaded
                // only on its own cadence: advancing `cancel_at` early would make a window wait
                // longer than CANCEL_LOOK to stop a spinning guest.
                if self.steps >= self.cancel_at {
                    match &self.cancel {
                        Some(flag) if flag.load(Ordering::Relaxed) => {
                            return Stop::Cancelled { pc: self.pc }
                        }
                        Some(_) => self.cancel_at = self.steps.saturating_add(CANCEL_LOOK),
                        // Only reachable if `cancel_at` stops tracking `cancel` being `None`; said out
                        // loud rather than left to spin.
                        None => self.cancel_at = u64::MAX,
                    }
                }
            }
            let pc = self.pc;
            if pc == crate::layout::HALT {
                return Stop::Halted { addr: pc };
            }
            // A breakpoint stops *before* the instruction at that address runs, which is the whole point
            // of naming one: what happens next is the thing being looked at. The test is a length check
            // on a `Vec` that is empty in every run that did not ask for one.
            if !self.break_at.is_empty() {
                // Every arrival at a watched address counts, not only the awaited one — `after` names
                // which arrival the caller wants, so the earlier ones have to be tallied to know.
                let mut awaited = None;
                for b in self.break_at.iter_mut() {
                    if b.addr != pc {
                        continue;
                    }
                    b.seen += 1;
                    if b.is_awaited(&self.r) {
                        awaited = Some(pc);
                    }
                }
                if let Some(addr) = awaited {
                    return Stop::Stopped {
                        why: StopWhy::Break(addr),
                    };
                }
            }
            // Read once: the loop asks "is this Thumb?" three times per instruction, and the answer cannot
            // change under it — `BX`/`BLX` set the state bit for the *next* instruction, not for this one.
            // The fault below still reports the state it entered with, which is what a fault report is for.
            let thumb = self.thumb();
            let word = match if thumb {
                self.mem.read_u16(pc)
            } else {
                self.mem.read_u32(pc)
            } {
                Ok(w) => w,
                Err(_) => {
                    return Stop::Fault(CpuFault {
                        pc,
                        thumb,
                        kind: FaultKind::UnmappedFetch { addr: pc },
                    })
                }
            };
            self.pc = pc + if thumb { 2 } else { 4 };
            // Only while a watchpoint is armed. `watch_by` is what attributes an access to the
            // instruction that ordered it, and with nothing armed there is no record to attribute
            // anything to — so the unconditional store this used to be bought nothing and cost one
            // write per retired instruction, on the hottest path in the emulator, every run.
            if !self.mem.watch.is_empty() {
                self.mem.watch_by = crate::mem::WatchBy::Guest { pc, insn: word };
                // The same gate carries the clock, so an arena record (`--watch`'s other half —
                // `alloc` and `free` touch no bytes) lands on the same timeline as the byte records.
                self.mem.watch_steps.set(self.steps);
            }
            let stop = if thumb {
                self.step_thumb(pc, word)
            } else {
                self.step_arm(pc, word)
            };
            self.steps += 1;
            // The sample is taken after the step, so its `pc` is the instruction that just retired and
            // `steps` counts it. Unarmed this is one load of `every` and a compare against `u64::MAX`.
            self.samples.note(self.steps, pc, thumb);
            // A tripped watchpoint ends the run where it happened — checked at the end of the step that
            // touched the byte rather than at the top of the loop, so the registers are exactly those of
            // the instruction that did it. The `Option` costs one load per instruction and is `None` on
            // every run that was not asked to stop.
            if let Some(addr) = self.mem.watch_tripped.get() {
                return Stop::Stopped {
                    why: StopWhy::Watch(addr),
                };
            }
            if self.trace.wants(pc) {
                self.trace.printed += 1;
                self.trace_line(pc, word);
            }
            match stop {
                Ok(()) => {}
                Err(stop) => return stop,
            }
        }
    }

    /// The ordinary execution loop: no breakpoints, watchpoints, samples or trace. Keeping this
    /// separate from [`Cpu::run`]'s diagnostic loop removes four per-instruction checks from the
    /// path a guest pack actually uses.
    ///
    /// `pub(crate)` for [`backend::Interpreter`], which is reached only through [`Cpu::run`] — the
    /// one caller that has already answered [`Cpu::per_instruction`]. Asking the same five
    /// questions a second time was about a quarter of what the backend seam costs per call;
    /// `benches/interp.rs` has the measurement and the before/after.
    pub(crate) fn run_quiet(&mut self, budget: u64) -> Stop {
        let limit = self.steps.saturating_add(budget);
        loop {
            let deadline = limit.min(self.cancel_at).min(self.progress_at);
            if self.steps >= deadline {
                if self.steps >= limit {
                    return Stop::Budget { steps: self.steps };
                }
                self.checkpoint_progress();
                if self.steps >= self.cancel_at {
                    match &self.cancel {
                        Some(flag) if flag.load(Ordering::Relaxed) => {
                            return Stop::Cancelled { pc: self.pc }
                        }
                        Some(_) => self.cancel_at = self.steps.saturating_add(CANCEL_LOOK),
                        None => self.cancel_at = u64::MAX,
                    }
                }
            }
            let pc = self.pc;
            if pc == crate::layout::HALT {
                return Stop::Halted { addr: pc };
            }
            let thumb = self.thumb();
            let word = match if thumb {
                self.mem.fetch_u16(pc)
            } else {
                self.mem.fetch_u32(pc)
            } {
                Ok(w) => w,
                Err(_) => {
                    return Stop::Fault(CpuFault {
                        pc,
                        thumb,
                        kind: FaultKind::UnmappedFetch { addr: pc },
                    })
                }
            };
            self.pc = pc + if thumb { 2 } else { 4 };
            let stop = if thumb {
                self.step_thumb(pc, word)
            } else {
                self.step_arm(pc, word)
            };
            self.steps += 1;
            match stop {
                Ok(()) => {}
                Err(stop) => return stop,
            }
        }
    }

    /// One traced instruction, one line. Out of line: `run` reaches this only while `--trace` is armed,
    /// and inlined it costs the ordinary run a `String` and a formatter inside the hottest function in
    /// the emulator. A run with `--trace` armed now
    /// pays one call per printed line instead of falling through inlined code — diagnostic runs get a
    /// little slower, which is the trade this file keeps making on purpose.
    #[cold]
    #[inline(never)]
    fn trace_line(&self, pc: u32, word: u32) {
        // r4 onwards is where a fault's answer usually is: a compiled ARM function keeps its saved
        // pointers and locals there, and "unmapped guest address 0x5c10" only becomes diagnosable
        // once the base register that was added to it is visible too.
        let mut rest = String::new();
        for i in 4..13 {
            rest.push_str(&format!(" r{i}={:08x}", self.r[i]));
        }
        eprintln!(
            "{:>8} {:08x} {:08x}  r0={:08x} r1={:08x} r2={:08x} r3={:08x}{rest} sp={:08x} lr={:08x} f={}",
            self.steps,
            pc,
            word,
            self.r[0],
            self.r[1],
            self.r[2],
            self.r[3],
            self.r[13],
            self.r[14],
            if self.thumb() { "t" } else { "a" },
        );
    }

    /// Shared helper for load/store: turn a memory fault into a CPU fault carrying the PC.
    ///
    /// `#[cold]`: this builds a report for a run that is about to end. Inlined, its field writes sat on
    /// the paths of every instruction that *can* fault: four stack slots end up written per instruction
    /// on the success path. Outlining it takes those writes away and buys 20.9 % on the Thumb
    /// measurement.
    #[cold]
    #[inline(never)]
    fn mem_stop(&self, pc: u32, addr: u32, err: MemFault) -> Stop {
        Stop::Fault(CpuFault {
            pc,
            thumb: self.thumb(),
            kind: FaultKind::Data { addr, reason: err },
        })
    }

    #[cold]
    #[inline(never)]
    fn illegal(&self, pc: u32, word: u32) -> Stop {
        Stop::Fault(CpuFault {
            pc,
            thumb: self.thumb(),
            kind: FaultKind::Illegal { word },
        })
    }

    #[cold]
    #[inline(never)]
    fn unimplemented(&mut self, pc: u32, word: u32, group: &'static str) -> Stop {
        self.pc = pc;
        Stop::Fault(CpuFault {
            pc,
            thumb: self.thumb(),
            kind: FaultKind::Unimplemented { group, word },
        })
    }

    /// Take an SVC. `id` is the immediate; the run loop reports the address *of* the SVC.
    ///
    /// A stub stands in for a real guest function reached with `BLX`/`BX`, so it returns the way
    /// such a function does: through `LR`, whose bit 0 carries the interwork state. Continuing
    /// at the next word would walk into the *following* stub instead of back to the caller.
    fn supervisor(&mut self, pc: u32, id: u32) -> Stop {
        self.hle_calls += 1;
        let lr = self.r[14];
        self.branch_to(lr, true);
        Stop::Hle { id, at: pc }
    }

    /// Write the registers and the counters into `out`, then the memory behind them.
    ///
    /// What is deliberately *not* here: `trace`, `samples` and `break_at` (instruments rather than
    /// state — a restored machine has no use for the previous one's debug stream), and
    /// `cancel`/`cancel_at`, which *ask* a run to stop: carrying one across a restore would stop a
    /// machine nobody asked to stop.
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        use crate::mem::{put_bool, put_u32, put_u64};
        for r in &self.r {
            put_u32(out, *r);
        }
        put_u32(out, self.cpsr);
        put_u32(out, self.spsr);
        put_u32(out, self.pc);
        put_bool(out, self.thumb());
        put_u64(out, self.steps);
        put_u64(out, self.hle_calls);
        self.mem.write_state(out);
    }

    /// Read back what [`Cpu::write_state`] wrote.
    ///
    /// The registers are held in locals until the memory has been read too, so a block that turns out
    /// to be short leaves the machine exactly as it was rather than with new registers over old memory.
    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut crate::mem::State) -> Result<(), String> {
        let mut r = [0u32; 15];
        for slot in &mut r {
            *slot = st.u32()?;
        }
        let cpsr = st.u32()?;
        let spsr = st.u32()?;
        let pc = st.u32()?;
        let thumb = st.u8()? != 0;
        let steps = st.u64()?;
        let hle_calls = st.u64()?;
        self.mem.read_state(st)?;
        self.r = r;
        self.cpsr = cpsr;
        self.spsr = spsr;
        self.pc = pc;
        // Through the setter rather than the `cpsr` word: the Thumb bit lives in `cpsr`, and setting
        // one without the other is how a machine ends up half in one instruction set.
        self.set_thumb(thumb);
        self.steps = steps;
        self.hle_calls = hle_calls;
        // The observation channel is not part of a snapshot, but its cadence is measured against
        // `steps`. A restore that rewinds the guest would otherwise leave `progress_at` ahead of
        // the machine and publish nothing until the guest caught up.
        if self.progress.is_some() {
            self.progress_at = steps.saturating_add(CANCEL_LOOK);
        }
        Ok(())
    }
}

/// ARM barrel shifter with an immediate amount. `amt` is the encoded field, so 0 means "no
/// shift" for LSL, LSR#32 for LSR, ASR#32 for ASR, and RRX for ROR.
pub fn shift_imm(value: u32, kind: Shift, amt: u32, carry_in: bool) -> (u32, bool) {
    let bit = |i: u32| value & (1 << i) != 0;
    match kind {
        Shift::Lsl => match amt {
            0 => (value, carry_in),
            32 => (0, bit(0)),
            a if a > 32 => (0, false),
            a => (value << a, bit(32 - a)),
        },
        Shift::Lsr => match amt {
            0 | 32 => (0, bit(31)),
            // Unreachable from an immediate operand — the field is five bits, so the most it can
            // encode beyond 31 is the `0` above — but kept saying what the register form says,
            // because two arms of this file answering the same amount differently is how the
            // register form stayed wrong.
            a if a > 32 => (0, false),
            a => (value >> a, bit(a - 1)),
        },
        Shift::Asr => {
            let sign = bit(31);
            // `ASR #0` is not a no-shift: the field is degenerate here exactly as it is for
            // `LSR`, so it means a full sign fill. Thumb's `ASRS` reaches the same encoding.
            if amt == 0 || amt >= 32 {
                ((if sign { u32::MAX } else { 0 }), sign)
            } else {
                // The carry the architecture leaves is the *last bit shifted out*, which is
                // bit `amt - 1` — not the sign. They coincide only at the extremes, and the
                // all-ones value the earlier tests used could never tell them apart. A reference
                // pack's `ldrblt ip, [ip, r1, asr #15]` was what found it: 0xffff there has to
                // leave 1 in C, and taking the sign instead said 0.
                (((value as i32) >> amt) as u32, bit(amt - 1))
            }
        }
        Shift::Ror => match amt {
            0 => ((value >> 1) | ((carry_in as u32) << 31), bit(0)),
            32 => (value, bit(31)),
            a => (value.rotate_right(a & 31), bit((a - 1) & 31)),
        },
    }
}

/// Register-amount shifting, which clamps differently: amounts of 32 or more are not the
/// rotate-modulo-32 that the immediate form degenerates into.
pub fn shift_reg(value: u32, kind: Shift, amt: u32, carry_in: bool) -> (u32, bool) {
    let bit = |i: u32| value & (1 << i) != 0;
    if amt == 0 {
        return (value, carry_in);
    }
    match kind {
        Shift::Lsl => {
            if amt >= 32 {
                (0, amt == 32 && bit(0))
            } else {
                (value << amt, bit(32 - amt))
            }
        }
        Shift::Lsr => {
            if amt == 32 {
                // A whole word's worth: the value is gone and bit 31 is the bit that left.
                (0, bit(31))
            } else if amt > 32 {
                // Past a whole word there is no bit that left, so the carry is clear. Unicorn
                // answers this and the old `amt >= 32` arm did not: it kept saying bit 31 for
                // every amount above 32, which is a word no immediate can encode and nothing in
                // the sample set was measured to reach.
                (0, false)
            } else {
                (value >> amt, bit(amt - 1))
            }
        }
        Shift::Asr => {
            // The carry is the last bit shifted out — bit `amt - 1` — and not the sign, which is what
            // [`shift_imm`] already does and what a guest pack's `asr #15` measured. The two
            // coincide only for an amount of 32 or more, where there is no last bit left to name and
            // the sign is the answer; that arm keeps the reading this had before.
            if amt >= 32 {
                (((value as i32) >> 31) as u32, bit(31))
            } else {
                (((value as i32) >> amt) as u32, bit(amt - 1))
            }
        }
        Shift::Ror => {
            let a = amt % 32;
            // A rotate by a whole number of words changes no bit and still replaces the carry with
            // the bit that went past the top — bit 31. Only an amount of 0 leaves the carry alone,
            // and that returned above; the two were the same case here, which is how a `ror` by 32
            // came to answer with the caller's carry-in, and how a test that pinned it "leaving the
            // carry alone" passed on a value whose bit 31 happened to be 1.
            if a == 0 {
                (value, bit(31))
            } else {
                (value.rotate_right(a), bit(a - 1))
            }
        }
    }
}

#[cfg(test)]
#[path = "../test/cpu/mod.rs"]
mod tests;
