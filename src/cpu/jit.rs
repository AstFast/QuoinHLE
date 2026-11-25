//! Optional execution backend: instruction execution is handed to [Rynarmic](https://github.com/AstFast/rynarmic)'s
//! block cache and x64 JIT.
//!
//! The split deliberately keeps "the state still belongs to this `Cpu`, the memory still belongs to
//! that `Mem`":
//!
//! - registers, `pc`, `cpsr`, `spsr` and `steps` stay on [`Cpu`], and the HLE layer goes on reading
//!   and writing `cpu.r[i]` / `cpu.pc` directly — not one line of it changes;
//! - every entry into a slice copies the state **into** Rynarmic and copies it **back** afterwards.
//!   The block cache and the compiled code live on Rynarmic's side and survive across slices, so
//!   translation is paid only the first time a block is met;
//! - memory still belongs to [`Mem`]: the bridge [`Bridge`] holds a single pointer to it, rebound on
//!   each entry (`Mem` is owned by `Runtime`, so its address is not guaranteed to hold still, and
//!   the pointer must not be cached across calls);
//! - `SVC` keeps the interpreter's semantics point for point: when it returns [`Stop::Hle`], `pc`
//!   already points past it and `at` is the `SVC`'s own address, so the HLE layer cannot tell the
//!   two apart.
//!
//! How it is turned on: in a binary built with the `jit` feature the CLI (`--jit` / `--no-jit`) and
//! the GUI library (`library::boot_at`) enable it by default; the library itself reads no
//! environment variable, so tests and embedding callers use [`Cpu::enable_jit`] explicitly. A build
//! without the feature has no such backend.
//!
//! Diagnostic paths (`--trace` / `--break` / watchpoint / `--sample`) do not come through here and
//! still interpret one instruction at a time — those features are "look at every instruction" by
//! construction, the opposite of what a block cache assumes.

use rynarmic::memory::{AccessFault, AccessKind, FastWindow, Memory, MAX_WINDOWS};
use rynarmic::{Cpsr, Cpu as Core, ExceptionKind};

use super::backend::Backend;
use super::{Cpu, CpuFault, FaultKind, JitArena, Stop, CANCEL_LOOK};
use crate::mem::{Fault as MemFault, Mem, PAGE_SIZE};
use std::sync::atomic::Ordering;

/// Guest registers (r0..r14; `pc` lives separately in `Cpu::pc`).
const REGS: usize = 15;

/// Upper bound on compiled blocks, aligned with Rynarmic's `BLOCK_CACHE_SIZE` (65536 slots).
///
/// Rynarmic's default is 16384, and when the arena fills `lookup_or_translate` does not evict the
/// oldest block — it calls `invalidate_code_cache()` and **throws all of them away**; once the
/// working set passes that number, every following pass recompiles the hot path (permanent
/// thrashing). `benches/jit_frame.rs` measured it: a second pass over 16000 blocks is 7 ms, over
/// 17000 blocks 3.2 s. Raising the limit to the same order as the slot table is what keeps the
/// arena from filling before the table does.
///
/// What it costs now is a share of one fixed reservation: machine code is appended to a single
/// 32 MiB RWX arena at roughly 170 bytes a block, so 65536 blocks are ~11 MiB of it and the pages
/// behind them are committed only as they are written. The ceiling therefore bounds *growth*, not a
/// reservation — which is what makes it a knob worth having (`--jit-arena`, and
/// [`crate::cpu::jit_arena_blocks`] for a pack-sized estimate) and why the default errs high: below
/// the working set every pass pays the recompile, above it a few MiB sit untouched.
pub const ARENA_BLOCKS: usize = 65536;

/// How many new blocks a single turn may compile.
///
/// Compilation happens on the worker's execution thread, so "a new scene needs a few thousand
/// blocks at once" turns into a stall of a few hundred milliseconds: 3000 blocks cost 150–180 ms
/// before the allocator change, and the guest retires not one instruction meanwhile — what the
/// window shows is `FPS -- (busy/quiet …)` with a frozen picture.
///
/// The budget converts **per-block compile cost** into a share of a frame: 192 blocks were ~11 ms
/// (51–59 µs a block at the time), the order of a frame. After executable memory was pooled a block
/// costs **3.9 µs** (`benches/jit_frame.rs`, 13× measured), so the same 192 buys 0.75 ms — the same
/// protection, with three quarters of the frame budget thrown away. 512 blocks are ~2 ms, still an
/// eighth of a frame, and a scene change has 2.7× fewer turns to spread over.
///
/// Blocks over the budget go through the interpreter and are compiled when the next turn refreshes
/// the budget. The price is a few slow frames on first entering a new region (interpreter
/// 55 M instr/s against JIT 900 M); what it buys is that no stall exceeds ~30 ms. `0` turns the
/// budget off and goes back to "compile everything it is handed".
pub const COMPILE_BUDGET_PER_TURN: u32 = 512;

/// The halt trap: `BKPT #0`.
///
/// The interpreter tests `pc == layout::HALT` **before fetching**, so it never sees this
/// instruction; Rynarmic does not know that sentinel, and one slice can run straight past it (the
/// page HALT lives on is mapped, so stepping over it neither faults nor stops) — after which a long
/// slice never returns to a place that checks the PC. Seeding a `BKPT` here hands control back the
/// moment the backend reaches it.
const HALT_TRAP: u32 = 0xE120_0070;

/// The diagnostic flag is read once at startup: `SVC` is the hot path and `env::var` has no
/// business being there.
fn debug_on() -> bool {
    static FLAG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FLAG.get_or_init(|| std::env::var_os("QUOIN_JIT_DEBUG").is_some())
}

fn slice_cap() -> Option<u64> {
    static CAP: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var("QUOIN_JIT_SLICE")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
    })
}

/// Connect [`Mem`] to Rynarmic's `Memory` trait.
///
/// The pointer is rebound by [`Cpu::run_jit`] on every entry into the backend; `Mem`'s own methods
/// take care of the page table, watchpoints and heap exhaustion, so all this layer does is turn an
/// `Err` into "access-fault flag plus detail".
pub struct Bridge {
    mem: *mut Mem,
    slot: Option<*mut u32>,
    last: Option<AccessFault>,
    /// The **frozen** window set: recomputed by `run_jit` only when `window_generation` changes.
    ///
    /// Rynarmic's block linking skips the prologue and jumps into a block body, and every block on
    /// the chain reuses the same primary window base (the `WINDOW` register); in other words the
    /// window set must be byte-identical between two whole-cache invalidations, and a new block may
    /// not use a new layout while old blocks use the old one. So it is cached here instead of being
    /// asked of `Mem::fast_spans()` on every compile — that one grows or reorders each time the
    /// heap crosses a new page, which is exactly where periodic recompilation came from.
    windows: [Option<FastWindow>; MAX_WINDOWS],
}

/// Publish `Mem`'s current fully-mapped spans as the JIT's windows (the substance of
/// `fast_windows`).
///
/// Called once per whole-cache invalidation; the result is fixed for the life of that generation.
fn publish_windows(mem: &Mem) -> [Option<FastWindow>; MAX_WINDOWS] {
    let mut out = [None; MAX_WINDOWS];
    for (slot, (base, page, count)) in mem.fast_spans().iter().enumerate() {
        let len = count * PAGE_SIZE;
        let watched = mem
            .watch
            .iter()
            .any(|&(start, wlen)| start < *base + (len as u32) && *base < start + wlen);
        if watched {
            continue;
        }
        let Some(ptr) = mem.page_ptr(*page) else {
            continue;
        };
        out[slot] = Some(FastWindow {
            ptr: ptr as *mut u8,
            base: *base,
            len,
        });
    }
    out
}

impl Bridge {
    fn mem(&mut self) -> &mut Mem {
        // SAFETY: the pointer is set to `&mut Cpu::mem` by `Cpu::run_jit` before each entry into
        // the backend, and `Cpu::mem` is not borrowed anywhere else for the whole
        // `run_cached_counted` call.
        unsafe { &mut *self.mem }
    }

    fn raise(&mut self, addr: u32, kind: AccessKind) {
        if let Some(slot) = self.slot {
            // SAFETY: the slot is bound to the backend's own `ArmState::fault` before it executes.
            unsafe { *slot = 1 };
        }
        if self.last.is_none() {
            self.last = Some(AccessFault { addr, kind });
        }
    }
}

impl Memory for Bridge {
    fn read_u8(&mut self, addr: u32) -> u8 {
        match self.mem().read_u8(addr) {
            Ok(value) => value,
            Err(_) => {
                self.raise(addr, AccessKind::Read);
                0
            }
        }
    }

    fn write_u8(&mut self, addr: u32, value: u8) {
        if self.mem().write_u8(addr, value).is_err() {
            self.raise(addr, AccessKind::Write);
        }
    }

    // Halfword and word accesses go through `Mem`'s own implementation: one page-table lookup
    // beats building the value out of four `read_u8` calls by a wide margin.
    fn read_u16(&mut self, addr: u32) -> u16 {
        match self.mem().read_u16(addr) {
            Ok(value) => value as u16,
            Err(_) => {
                self.raise(addr, AccessKind::Read);
                0
            }
        }
    }

    fn read_u32(&mut self, addr: u32) -> u32 {
        match self.mem().read_u32(addr) {
            Ok(value) => value,
            Err(_) => {
                self.raise(addr, AccessKind::Read);
                0
            }
        }
    }

    fn write_u16(&mut self, addr: u32, value: u16) {
        if self.mem().write_u16(addr, value as u32).is_err() {
            self.raise(addr, AccessKind::Write);
        }
    }

    fn write_u32(&mut self, addr: u32, value: u32) {
        if self.mem().write_u32(addr, value).is_err() {
            self.raise(addr, AccessKind::Write);
        }
    }

    // Watchpoints are gated per span: accesses inside a window are inlined and never reach these
    // read/write callbacks, so they leave no `--watch` record. Only spans overlapping an armed range
    // are dropped, and unarmed ones stay inlined — arming one or two bytes should not cost the whole
    // mapping its fast path.
    //
    // What comes back is the copy `run_jit` froze at publish time: the window set may not change
    // between two whole-cache invalidations, or linked blocks reusing the old `WINDOW` base would
    // read the wrong span.
    fn fast_windows(&self) -> [Option<FastWindow>; MAX_WINDOWS] {
        self.windows
    }

    fn set_fault_flag(&mut self, slot: *mut u32) {
        self.slot = Some(slot);
    }

    fn take_fault(&mut self) -> Option<AccessFault> {
        self.last.take()
    }
}

/// One Rynarmic core: the block cache and the compiled code live inside it, surviving slices.
pub struct Jit {
    core: Core<Bridge>,
    /// The `pc` last synchronised out, for diagnostics only.
    last_pc: u32,
    /// The `Mem::window_generation` in force when the windows were last published. The cache is
    /// invalidated and the windows republished only when they can really be stale; mapping a new
    /// page does not advance this generation.
    /// `u64::MAX` is the "not published yet" sentinel, forcing the windows to be set up on the first
    /// entry into the backend.
    window_gen: u64,
    /// How many non-empty spans the last published window set held. Diagnostic: `QUOIN_STATS`
    /// shows it as `win 7`, because "does the stack have a window at all" is invisible once the
    /// window count is truncated — and it decides whether each access on the stack is inlined
    /// (0.79 ns/instruction) or goes back into a Rust callback (1.88 ns/instruction).
    windows: usize,
}

// Move safety: `Bridge::mem` is set only in `run_jit` and read only during
// `run_cached_counted`; the pointer can go stale when `Cpu` is moved, but every entry into the
// backend rebinds it and it is never dereferenced while stale. `Bridge::slot` points at the
// backend's own `ArmState::fault` and is rebound by Rynarmic before each execution, for the same
// reason.
// `Cpu` as a whole is moved into the worker thread (`thread::spawn` in `manager.rs`), and only that
// thread touches it afterwards.
unsafe impl Send for Jit {}

impl std::fmt::Debug for Jit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Jit {{ last_pc: {:#010x} }}", self.last_pc)
    }
}

impl Jit {
    fn new() -> Self {
        let mut core = Core::new(Bridge {
            mem: std::ptr::null_mut(),
            slot: None,
            last: None,
            windows: [None; MAX_WINDOWS],
        });
        // Raising the limit has to happen before any block is compiled: `set_jit_arena_limit`
        // clears the block cache. This core was just built, so its cache is empty and the call has
        // no side effect beyond replacing the default budget.
        core.set_jit_arena_limit(ARENA_BLOCKS);
        // Diagnostic switch: `QUOIN_JIT_INTERP=1` restricts this backend to Rynarmic's own
        // interpreter, which is how "the adapter's state sync is wrong" is told apart from "the
        // generated code is wrong".
        if std::env::var_os("QUOIN_JIT_INTERP").is_some() {
            core.set_jit_enabled(false);
        }
        Self {
            core,
            last_pc: 0,
            window_gen: u64::MAX,
            windows: 0,
        }
    }
}

impl Cpu {
    /// Turn the translating backend on. The silent `run`/`run_quiet` paths are executed by Rynarmic
    /// from here on.
    ///
    /// Diagnostic paths (trace / breakpoint / watchpoint / sample) still interpret one instruction
    /// at a time, because those features start from the premise that every instruction is seen.
    /// Idempotent: a machine already carrying a translating backend keeps it, which is the rule the
    /// old `if self.jit.is_none()` had — it means "do not rebuild a warm block cache".
    pub fn enable_jit(&mut self) {
        if !self.jit_enabled() {
            self.backend = Some(Box::new(Jit::new()));
        }
    }

    /// Turn the translating backend off (back to the plain interpreter), for same-machine A/B.
    pub fn disable_jit(&mut self) {
        self.backend = Some(Box::new(super::backend::Interpreter));
    }

    /// Whether instructions are currently being translated.
    ///
    /// Asked of the backend rather than matched against a name, so a second translating backend
    /// answers without this function learning about it.
    pub fn jit_enabled(&self) -> bool {
        self.backend.as_ref().is_some_and(|b| b.translating())
    }

    /// Set the compiled-code ceiling directly, for a caller that already computed it.
    pub fn set_jit_arena_limit(&mut self, limit: usize) {
        if let Some(backend) = self.backend.as_mut() {
            backend.set_arena_limit(limit);
        }
    }
}

impl Jit {
    /// Make sure the halt sentinel really carries a `BKPT`.
    ///
    /// Written only when the word is wrong: the load path can map this page again
    /// (`load_library`), so every entry into the backend confirms it, at the price of one read and
    /// a compare.
    fn install_halt_trap(&self, cpu: &mut Cpu) {
        let halt = crate::layout::HALT;
        if cpu.mem.read_u32(halt).ok() != Some(HALT_TRAP) {
            let _ = cpu.mem.write_u32(halt, HALT_TRAP);
        }
    }
}

impl Backend for Jit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Execute at most `budget` instructions on the translating backend.
    ///
    /// Point for point with the interpreter loop's contract: a `Stop::Hle` return leaves `pc` on the
    /// next instruction and `at` on the `SVC` itself; `steps` only grows; `cancel` is sampled once
    /// every `CANCEL_LOOK` instructions. A `BKPT` sits at `layout::HALT` so the backend can stop on
    /// the halt sentinel.
    fn run(&mut self, cpu: &mut Cpu, budget: u64) -> Stop {
        let limit = cpu.steps.saturating_add(budget);

        // The halt trap: confirmed on every entry (loading remaps pages), one read and a compare.
        self.install_halt_trap(cpu);

        // The window set is republished only when the invalidation generation changes; mapping the
        // heap across a new page does not advance it (see `Mem::window_generation`), so once this is
        // set up a long run never invalidates the cache again. On the first entry `window_gen` is
        // the sentinel, so a publish is guaranteed.
        let gen = cpu.mem.window_generation();
        if gen != self.window_gen {
            // The window pointers and bases baked into compiled blocks may be stale, and block
            // linking skips the prologue to reuse the `WINDOW` base the previous block installed:
            // every block on the chain has to share one primary window. So the whole cache is
            // invalidated and the set republished as a unit; a new block cannot change the layout
            // by itself.
            self.core.invalidate_code_cache();
            let published = publish_windows(&cpu.mem);
            self.windows = published.iter().filter(|w| w.is_some()).count();
            self.core.memory.windows = published;
            self.window_gen = gen;
        }

        // The memory pointer is rebound on every call: `Mem` is owned by `Runtime`, so its address
        // is not guaranteed to hold still.
        {
            let mem_ptr: *mut Mem = &mut cpu.mem;
            self.core.memory.mem = mem_ptr;
        }

        loop {
            // The halt sentinel is tested **before entering the backend**: the interpreter tests it on
            // every fetch, while one slice here runs 65536 instructions — landing on `HALT` and
            // testing afterwards means it has already been executed (HALT is a mapped address, so
            // running past it does not fault).
            if cpu.pc == crate::layout::HALT {
                return Stop::Halted { addr: cpu.pc };
            }
            if cpu.steps >= limit {
                return Stop::Budget { steps: cpu.steps };
            }

            // The cancel flag is sampled every CANCEL_LOOK instructions, the interpreter's rhythm.
            // Progress publication rides the same checkpoint: one slice is exactly CANCEL_LOOK
            // instructions, so every slice hands `steps` to the window and `QUOIN_STATS` and the
            // badge can still answer "is the guest moving" during a long frame.
            cpu.checkpoint_progress();
            if cpu.steps >= cpu.cancel_at {
                match &cpu.cancel {
                    Some(flag) if flag.load(Ordering::Relaxed) => {
                        return Stop::Cancelled { pc: cpu.pc }
                    }
                    Some(_) => cpu.cancel_at = cpu.steps.saturating_add(CANCEL_LOOK),
                    None => cpu.cancel_at = u64::MAX,
                }
            }

            let mut slice = (limit - cpu.steps).min(CANCEL_LOOK).min(u32::MAX as u64);
            // Diagnostic: clamp the slice to N instructions, to see "where did it actually go".
            if let Some(n) = slice_cap() {
                slice = slice.min(n);
            }

            // Enter the backend, run one slice, take the state back. `core` borrows this `Jit`; the
            // machine state it reads and writes is `cpu`'s, a different object.
            let (executed, result) = {
                let core = &mut self.core;
                core.state.set_pc(cpu.pc);
                core.state.cpsr = Cpsr::from_bits(cpu.cpsr);
                core.state.r[..REGS].copy_from_slice(&cpu.r);

                let (executed, result) = core.run_cached_counted(slice);

                cpu.r.copy_from_slice(&core.state.r[..REGS]);
                cpu.pc = core.state.pc();
                cpu.cpsr = core.state.cpsr.bits();
                self.last_pc = cpu.pc;
                (executed, result)
            };
            cpu.steps = cpu.steps.saturating_add(executed);

            let thumb = cpu.cpsr & (1 << 5) != 0;
            match result {
                Ok(()) => {
                    if debug_on() {
                        eprintln!("JIT slice end pc={:#010x} steps={}", cpu.pc, cpu.steps);
                    }
                    if cpu.pc == crate::layout::HALT {
                        return Stop::Halted { addr: cpu.pc };
                    }
                }
                Err(ExceptionKind::SoftwareInterrupt(id)) => {
                    // Point for point with the interpreter's `supervisor()`: the stub was called by
                    // `BLX`/`BX`, so it **returns from LR like a function** (bit0 carries the interwork
                    // state) instead of resuming after the `SVC` — which would be the next stub.
                    let at = cpu.pc;
                    cpu.hle_calls += 1;
                    let lr = cpu.r[14];
                    cpu.branch_to(lr, true);
                    if debug_on() {
                        eprintln!(
                            "JIT HLE id={id:#x} at={at:#010x} lr={lr:#010x} -> pc={:#010x} steps={}",
                            cpu.pc, cpu.steps
                        );
                    }
                    return Stop::Hle { id, at };
                }
                // When the page HALT lives on is unmapped, the fetch fails — and that failure is the
                // halt signal.
                Err(ExceptionKind::AccessFault { addr, kind })
                    if addr == crate::layout::HALT && kind == AccessKind::Fetch =>
                {
                    return Stop::Halted {
                        addr: crate::layout::HALT,
                    };
                }
                Err(ExceptionKind::AccessFault { addr, kind }) => {
                    let pc = cpu.pc;
                    return Stop::Fault(CpuFault {
                        pc,
                        thumb,
                        kind: match kind {
                            AccessKind::Fetch => FaultKind::UnmappedFetch { addr },
                            _ => FaultKind::Data {
                                addr,
                                reason: MemFault::Unmapped { addr },
                            },
                        },
                    });
                }
                Err(ExceptionKind::UndefinedInstruction(word)) => {
                    let pc = cpu.pc;
                    return Stop::Fault(CpuFault {
                        pc,
                        thumb,
                        kind: FaultKind::Illegal { word },
                    });
                }
                Err(ExceptionKind::Breakpoint(_)) if cpu.pc == crate::layout::HALT => {
                    return Stop::Halted { addr: cpu.pc };
                }
                Err(other) => {
                    let pc = cpu.pc;
                    return Stop::Fault(CpuFault {
                        pc,
                        thumb,
                        kind: FaultKind::Unimplemented {
                            group: match other {
                                ExceptionKind::UnsupportedInstruction(_) => "rynarmic decode",
                                ExceptionKind::Breakpoint(_) => "breakpoint",
                                _ => "rynarmic",
                            },
                            word: 0,
                        },
                    });
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "rynarmic"
    }

    fn translating(&self) -> bool {
        true
    }

    fn cached_blocks(&self) -> Option<usize> {
        Some(self.core.compiled_block_count())
    }

    fn set_compile_budget(&mut self, budget: Option<u32>) {
        self.core.set_compile_budget(budget);
    }

    fn default_compile_budget(&self) -> Option<u32> {
        Some(COMPILE_BUDGET_PER_TURN)
    }

    fn size_arena(&mut self, choice: JitArena, code_bytes: u64) -> Option<String> {
        let default = ARENA_BLOCKS;
        // `0` is the documented "no ceiling": the code arena's own fullness is then the only thing
        // that ever flushes the cache.
        let line = match choice {
            JitArena::Blocks(0) => {
                self.core.set_jit_arena_limit(usize::MAX);
                format!(
                    "jit: arena uncapped, the 32 MiB code arena is the only limit (default {default})"
                )
            }
            JitArena::Blocks(n) => {
                self.core.set_jit_arena_limit(n);
                format!("jit: arena {n} blocks, as asked (default {default})")
            }
            JitArena::Auto => {
                let n = super::jit_arena_blocks(code_bytes, default);
                self.core.set_jit_arena_limit(n);
                format!(
                    "jit: arena {n} blocks, sized from {} KiB of module image (default {default})",
                    code_bytes / 1024
                )
            }
        };
        Some(line)
    }

    fn set_arena_limit(&mut self, limit: usize) {
        self.core.set_jit_arena_limit(limit);
    }

    fn windows(&self) -> Option<usize> {
        Some(self.windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Cpu, JitArena};
    use crate::mem::Mem;

    const CODE_A: u32 = 0x1000_0000;
    const CODE_B: u32 = 0x1000_2000;
    const NEW_PAGE: u32 = 0x1001_0000;

    /// The translating backend `c` carries, for the assertions that read its published window array.
    fn jit_of(c: &Cpu) -> &Jit {
        c.backend
            .as_ref()
            .and_then(|b| b.as_any().downcast_ref::<Jit>())
            .expect("enable_jit installed the Rynarmic backend")
    }

    /// `FastWindow` has no `PartialEq`, so compare the part that can be compared.
    fn shape(windows: &[Option<FastWindow>; MAX_WINDOWS]) -> [(usize, u32, usize); MAX_WINDOWS] {
        let mut out = [(0usize, 0u32, 0usize); MAX_WINDOWS];
        for (slot, entry) in windows.iter().enumerate() {
            if let Some(w) = entry {
                out[slot] = (w.ptr as usize, w.base, w.len);
            }
        }
        out
    }

    /// The heap maps a page after the windows were published: the layout changed but the window
    /// generation did not, so the JIT may neither republish nor swallow the new page into an old
    /// window. The new page has to fall back to the `Memory` callback and read the real value.
    #[test]
    fn frozen_windows_fall_back_to_callbacks_for_newly_mapped_pages() {
        let mut c = Cpu::new(Mem::new());
        c.mem.map(CODE_A, 0x8000);
        // add r0,r0,#1 ; b .
        for (i, w) in [0xe280_0001u32, 0xeaff_fffe].iter().enumerate() {
            c.mem.write_u32(CODE_A + i as u32 * 4, *w).unwrap();
        }
        // ldr r0,[r1] ; b .
        for (i, w) in [0xe591_0000u32, 0xeaff_fffe].iter().enumerate() {
            c.mem.write_u32(CODE_B + i as u32 * 4, *w).unwrap();
        }
        c.enable_jit();

        // First pass: compile and publish the windows (the first entry's sentinel forces a publish).
        c.pc = CODE_A;
        let _ = c.run(64);
        let published = shape(&jit_of(&c).core.memory.windows);
        assert_ne!(
            published[0],
            (0, 0, 0),
            "a published main window is required"
        );
        assert_eq!(c.mem.window_generation(), 0);

        // The heap maps a new page: the layout generation advances, the window generation must not.
        c.mem.map(NEW_PAGE, 0x1000);
        c.mem.write_u32(NEW_PAGE, 0xdead_beef).unwrap();
        assert!(c.mem.map_generation() > 0);
        assert_eq!(c.mem.window_generation(), 0);

        let (_, base, len) = published[0];
        assert!(
            NEW_PAGE < base || NEW_PAGE >= base + len as u32,
            "the test premise failed: the new page unexpectedly landed in a published window"
        );

        // Second pass: the frozen windows are byte-identical and the new page reads through the callback.
        c.pc = CODE_B;
        c.r[0] = 0;
        c.r[1] = NEW_PAGE;
        let _ = c.run(64);
        assert_eq!(
            shape(&jit_of(&c).core.memory.windows),
            published,
            "mapping a new page must not change the window set: block links reuse the old main window base"
        );
        assert_eq!(
            c.r[0], 0xdead_beef,
            "a page mapped after freezing the windows must read the real value through the callback"
        );
    }

    /// `--compat unmapped-read` is answered inside `Mem::at`, which every guest access reaches, so
    /// the translating backend inherits the switch without a line of its own: an address outside the
    /// published windows falls back to `Bridge`'s callbacks, those call `Mem`, and `Mem` answers
    /// zero. This test pins *the seam*, not the switch — `Bridge` is the one place that could take
    /// the switch back off, by turning "no window covers this address" into a fault of its own.
    ///
    /// The second half is the promise the switch makes: a store to the same address still faults.
    /// That belongs here and not only in `mem.rs`, because compiled code touches a window inline and
    /// no callback runs: a window stretched over an unmapped page would swallow the store silently.
    /// So the window has to end exactly where the mapping ends, under the switch too.
    #[test]
    fn compat_unmapped_read_reaches_the_translating_backend() {
        const FAR: u32 = 0x5000_0000; // mapped by nothing this test builds
        const LDR_R0_R1: u32 = 0xe591_0000;
        const STR_R0_R1: u32 = 0xe581_0000;
        const SPIN: u32 = 0xeaff_fffe;

        let machine = |insn: u32, compat: bool| {
            let mut c = Cpu::new(Mem::new());
            // Two pages, because `Mem::fast_spans` drops a one-page run: "an extra comparison on
            // every access to save one callback does not pay". The window is what this test is
            // about, so it has to exist.
            c.mem.map(CODE_A, 0x2000);
            c.mem.write_u32(CODE_A, insn).unwrap();
            c.mem.write_u32(CODE_A + 4, SPIN).unwrap();
            c.mem.set_compat_read_zero(compat);
            c.pc = CODE_A;
            c.r[0] = 0x1234;
            c.r[1] = FAR;
            c.r[13] = CODE_A + 0x800;
            c.enable_jit();
            c
        };

        // Without the switch: the load is a data abort, the way the interpreter reports it.
        match machine(LDR_R0_R1, false).run(64) {
            Stop::Fault(CpuFault {
                kind: FaultKind::Data { addr, reason },
                ..
            }) => {
                assert_eq!(addr, FAR, "the fault names the address the load read");
                assert_eq!(
                    reason,
                    MemFault::Unmapped { addr: FAR },
                    "and says why: nothing is mapped there"
                );
            }
            other => panic!("expected a data abort from the load, got {other:?}"),
        }

        // With it: the load answers zero and the guest keeps running to the end of the budget.
        let mut loose = machine(LDR_R0_R1, true);
        assert!(
            matches!(loose.run(64), Stop::Budget { .. }),
            "under the switch an unmapped load is not a fault"
        );
        assert_eq!(loose.r[0], 0, "it answers zero, not whatever was in r0");

        // The switch must not be implemented by widening a window. The mapped pages publish
        // exactly `[CODE_A, CODE_A + 0x2000)`, so the address past them still goes through `Mem` —
        // and a store there faults, as the switch promises.
        let published = jit_of(&loose).core.memory.windows;
        let page = published
            .iter()
            .flatten()
            .find(|w| CODE_A >= w.base && CODE_A < w.base + w.len as u32)
            .expect("the mapped pages are published as a window");
        assert_eq!(
            page.base + page.len as u32,
            CODE_A + 0x2000,
            "the window ends where the mapping ends: past it, accesses have to reach `Mem`"
        );
        assert!(
            !published
                .iter()
                .flatten()
                .any(|w| FAR >= w.base && FAR < w.base + w.len as u32),
            "nothing is mapped at {FAR:#010x}, so no window may cover it"
        );

        match machine(STR_R0_R1, true).run(64) {
            Stop::Fault(CpuFault {
                kind: FaultKind::Data { addr, reason },
                ..
            }) => {
                assert_eq!(addr, FAR);
                assert_eq!(reason, MemFault::Unmapped { addr: FAR });
            }
            other => panic!("a store past the map still faults under the switch, got {other:?}"),
        }
    }

    /// With the compile budget clamped to two blocks, almost every block runs into an
    /// `invalidate_code_cache()`. That makes this a deliberate semantic gate: invalidation may force a
    /// retranslation, but it may not make the guest run one instruction fewer or more, nor
    /// materialise a lazy flag into a different cpsr. Compared item by item against the same program
    /// under a wide budget; both sides run hand-written ARM words and touch no HLE, so this stays
    /// inside what §1 allows.
    #[test]
    fn a_tiny_arena_keeps_guest_state_identical_across_full_invalidations() {
        const BASE: u32 = 0x1000_0000;
        // Eight blocks: add r0,r0,#1 / b next, with the last one branching back to BASE.
        let mut words = Vec::new();
        for i in 0..8 {
            words.push(0xe280_0001u32);
            let at = BASE + words.len() as u32 * 4;
            words.push(if i == 7 {
                let delta = (BASE as i64 - (at as i64 + 8)) / 4;
                0xea00_0000 | ((delta as u32) & 0x00ff_ffff)
            } else {
                0xea00_0000
            });
        }

        let build = |arena: Option<usize>| {
            let mut c = Cpu::new(Mem::new());
            c.mem.map(BASE, 0x1000);
            for (i, w) in words.iter().enumerate() {
                c.mem.write_u32(BASE + i as u32 * 4, *w).unwrap();
            }
            c.pc = BASE;
            c.r[0] = 0;
            c.r[13] = 0x1000_2800;
            c.r[14] = BASE;
            c.enable_jit();
            if let Some(n) = arena {
                c.set_jit_arena_limit(n);
            }
            c
        };

        let mut wide = build(None);
        let mut tiny = build(Some(2));
        assert!(matches!(wide.run(500), Stop::Budget { .. }));
        assert!(matches!(tiny.run(500), Stop::Budget { .. }));
        assert_eq!(
            tiny.r, wide.r,
            "a cache invalidation must not rewrite registers"
        );
        assert_eq!(tiny.pc, wide.pc, "a cache invalidation must not rewrite pc");
        assert_eq!(
            tiny.cpsr, wide.cpsr,
            "lazy flag materialisation must be equivalent"
        );
        assert_eq!(
            tiny.steps, wide.steps,
            "invalidation must not swallow or add instructions"
        );
    }

    /// The HUD's JIT line is only as good as this accessor: it must see a real block count, and a
    /// full invalidation must be visible as a drop the session's `observe_jit_blocks` can read.
    #[test]
    fn cached_block_count_is_observable_and_a_flush_drops_it_to_zero() {
        let mut c = Cpu::new(Mem::new());
        c.mem.map(CODE_A, 0x1000);
        for (i, w) in [0xe280_0001u32, 0xeaff_fffe].iter().enumerate() {
            c.mem.write_u32(CODE_A + i as u32 * 4, *w).unwrap();
        }
        c.pc = CODE_A;
        c.r[13] = 0x1000_2800;
        assert_eq!(
            c.jit_cached_blocks(),
            None,
            "the interpreter has no code cache, so there is no reading"
        );
        c.enable_jit();
        assert_eq!(
            c.jit_cached_blocks(),
            Some(0),
            "the cache is empty right after enabling the backend"
        );
        assert!(matches!(c.run(64), Stop::Budget { .. }));
        assert!(
            c.jit_cached_blocks().is_some_and(|n| n > 0),
            "some blocks must have been compiled after running hand-written code"
        );
        // `set_jit_arena_limit` clears the cache; that is the same drop a real arena-full flush makes.
        c.set_jit_arena_limit(2);
        assert_eq!(
            c.jit_cached_blocks(),
            Some(0),
            "a full-table invalidation must be observable, otherwise the HUD's flushes stay at 0"
        );
    }

    /// A turn compiles at most a budget's worth of blocks, and the budget must not change the
    /// guest's result.
    ///
    /// These are the two properties the "scene change hiccups" fix rests on: compilation happens on
    /// the execution thread, so spreading a few thousand blocks over several turns is what turns one
    /// stall of hundreds of milliseconds into a few slightly slower frames. Spread out, the guest
    /// must still reach exactly the same state — the budget decides who gets machine code first, not
    /// which instructions run.
    #[test]
    fn a_compile_budget_bounds_blocks_per_turn_and_keeps_results_identical() {
        const BASE: u32 = 0x1000_0000;
        const BLOCKS: usize = 64;
        const CHUNK: u64 = 400;
        // 64 blocks: `add r0,r0,#1` / `b next`, the last one jumping back to BASE. One lap is 128
        // instructions and visits all 64 blocks once, so a single CHUNK needs 64 compilations — a
        // small budget is bound to cut that short.
        let mut words = Vec::new();
        for i in 0..BLOCKS {
            words.push(0xe280_0001u32);
            let at = BASE + words.len() as u32 * 4;
            words.push(if i + 1 == BLOCKS {
                let delta = (BASE as i64 - (at as i64 + 8)) / 4;
                0xea00_0000 | ((delta as u32) & 0x00ff_ffff)
            } else {
                0xea00_0000
            });
        }

        let build = || {
            let mut c = Cpu::new(Mem::new());
            c.mem.map(BASE, 0x1000);
            for (i, w) in words.iter().enumerate() {
                c.mem.write_u32(BASE + i as u32 * 4, *w).unwrap();
            }
            c.pc = BASE;
            c.r[0] = 0;
            c.r[13] = 0x1000_2800;
            c.r[14] = BASE;
            c.enable_jit();
            c
        };

        let mut tight = build();
        tight.set_jit_compile_budget(Some(8));
        let _ = tight.run(CHUNK);
        let per_turn = tight
            .jit_cached_blocks()
            .expect("the backend must report a reading once enabled");
        assert!(
            per_turn <= 8,
            "the number of blocks compiled in one turn must be capped by the quota: quota 8, actual {per_turn}"
        );

        // Continuing after a refresh must compile more: a denied block cannot become "never compiled".
        tight.set_jit_compile_budget(Some(8));
        let _ = tight.run(CHUNK);
        assert!(
            tight.jit_cached_blocks().unwrap() > per_turn,
            "the next turn must keep compiling the remaining blocks"
        );

        // Same instruction count, same refresh rhythm, only the budget differs: results must match.
        let mut loose = build();
        for _ in 0..2 {
            loose.set_jit_compile_budget(None);
            let _ = loose.run(CHUNK);
        }
        for _ in 2..6 {
            tight.set_jit_compile_budget(Some(8));
            let _ = tight.run(CHUNK);
            loose.set_jit_compile_budget(None);
            let _ = loose.run(CHUNK);
        }
        assert_eq!(tight.r, loose.r, "the quota must not change registers");
        assert_eq!(tight.pc, loose.pc, "the quota must not change pc");
        assert_eq!(tight.cpsr, loose.cpsr, "the quota must not change flags");
        assert_eq!(
            tight.steps, loose.steps,
            "the quota must not swallow or add instructions"
        );
        assert!(
            tight.jit_cached_blocks().unwrap() <= loose.jit_cached_blocks().unwrap(),
            "the quota can only reduce compilation, never increase it"
        );
    }

    /// `--jit-arena` has to reach the ceiling it names, and `auto` has to become a number rather than
    /// a promise: both are read here through the same call the front ends make, and the flush that
    /// proves the backend took it is the documented cost of changing the ceiling.
    #[test]
    fn sizing_the_arena_reaches_the_backend_and_says_what_it_chose() {
        const CODE: u32 = 0x1000_0000;
        let mut c = Cpu::new(Mem::new());
        assert_eq!(
            c.size_jit_arena(JitArena::Auto, 351_152),
            None,
            "no backend means nothing to size, and the caller has nothing to log"
        );

        c.mem.map(CODE, 0x1000);
        for (i, w) in [0xe280_0001u32, 0xeaff_fffe].iter().enumerate() {
            c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
        }
        c.pc = CODE;
        c.r[13] = 0x1000_2800;
        c.enable_jit();
        assert!(matches!(c.run(64), Stop::Budget { .. }));
        assert!(c.jit_cached_blocks().is_some_and(|n| n > 0));

        let line = c
            .size_jit_arena(JitArena::Blocks(2), 0)
            .expect("a backend to size");
        assert!(line.contains(" 2 blocks"), "{line}");
        assert_eq!(
            c.jit_cached_blocks(),
            Some(0),
            "changing the ceiling has to reach the block cache, which is what clears it"
        );

        // `auto` reads the image: 157 404 bytes is one block per 4 bytes at most, and the line says
        // both the number and what it was read from — a ceiling nobody can see is a knob nobody can
        // check.
        let line = c.size_jit_arena(JitArena::Auto, 157_404).unwrap();
        assert!(line.contains("39351 blocks"), "{line}");
        assert!(line.contains("153 KiB"), "{line}");
        let line = c.size_jit_arena(JitArena::Blocks(0), 0).unwrap();
        assert!(line.contains("uncapped"), "{line}");
    }
}
