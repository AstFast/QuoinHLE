//! The execution-backend seam: what carries an instruction out, kept apart from what the machine *is*.
//!
//! A [`Cpu`] is machine state — registers, `cpsr`, the memory map, the cancel and progress channels,
//! the diagnostic hooks. How an instruction gets carried out is a [`Backend`]: the interpreter in the
//! parent module, or Rynarmic's translator behind the `jit` feature. That line used to be a
//! `#[cfg(feature = "jit")] pub jit: Option<jit::Jit>` field plus five `#[cfg]` pairs on `Cpu`'s
//! diagnostics, which meant the machine's own type knew the name of one translator. It does not any
//! more.
//!
//! What that buys, in the order the project needs it:
//!
//! * **A second host, not a second core.** An embedder (the libretro core, a RetroArch port, anything
//!   that drives frames) reaches `Runtime`, whose execution path is one `Cpu::run` call. Which backend
//!   is inside changes nothing above it — same `Stop` values, same `pc`/`steps`/`cancel` contract.
//! * **A third backend is an `impl`.** An ARM64 dynarec, a wasm one, one that forwards a slice to a
//!   remote debugger: each is a new file with one `run` and the capability methods it can answer.
//!   No `Cpu` field, no new `#[cfg]` arm. (A *recording* backend is the one shape that does not fit,
//!   and deliberately: seeing every instruction is what `Cpu::per_instruction` is for, and that path
//!   goes to the interpreter whatever is installed.)
//! * **Capabilities are asked, not assumed.** `--stats`'s `jit B blocks`, `--jit-arena`, `--jit-budget`
//!   all used to be compiled in or out with the feature. They are methods now, and a backend that has
//!   no code cache answers `None` rather than the build failing to mention it.
//!
//! The borrow shape is the one thing worth knowing before reading on. `run` takes `&mut Cpu` rather
//! than owning it because the machine state is shared by every backend *and* by the HLE seam: after a
//! `Stop::Hle` the caller writes registers and memory before resuming, and it must not have to care
//! which backend is installed. Since the backend lives inside the `Cpu` it is handed, `Cpu::run` takes
//! it out of its box for the duration of one turn and puts it back — see the comment there.
//!
//! # Writing a backend
//!
//! The trait is small; what it owes is not. Every rule below is a contract with whatever called
//! [`Cpu::run`], and a backend that breaks one breaks the machine somewhere three subsystems away
//! from the mistake. They are listed here rather than discovered one at a time by whoever ports the
//! next engine, because the failure modes are all silent.
//!
//! **State lives in `cpu`, never in the backend.** Registers, `pc`, `cpsr`, memory, `steps` — read
//! them at the top of `run`, write them back before returning. Anything cached between calls is
//! stale the moment the HLE layer writes a register after a `Stop::Hle`, and nothing tells the
//! backend that happened. A translation cache is the one exception, and it caches *code*, not
//! machine state.
//!
//! **`Stop::Hle` has exactly one correct shape.** The guest reached an `SVC` stub, which was
//! entered with `BLX`/`BX`; the stub therefore *returns through `LR` like a function*, with bit 0
//! carrying the interwork state. So before returning, the backend sets `cpu.pc` to `cpu.r[14]` with
//! [`Cpu::branch_to`] and reports `at` as the address of the `SVC` itself. Reporting `pc` as the
//! instruction after the `SVC` — the obvious reading — resumes at the *next stub* and the guest
//! runs into the wrong method. `id` is the `SVC` immediate, which is how the HLE layer picks the
//! method; an engine that only reports "supervisor call" and drops the number cannot drive this
//! machine.
//!
//! **`steps` is a clock, not a counter.** The virtual clock, `--sample`, the progress channel and
//! every scheduled timeout derive from it. It must grow by the number of instructions actually
//! retired, exactly — a slice that runs 1000 of a requested 1024 must say 1000. An engine whose
//! slice length is approximate (many TCG designs are) can still be installed, but the error has to
//! be accounted for rather than rounded away, and `--deterministic` replay will not be reliable.
//!
//! **`cancel` is sampled, not awaited.** Load `cpu.cancel` once every [`super::CANCEL_LOOK`]
//! instructions and stop with `Stop::Cancelled { pc }` where the guest had got to. Checking per
//! instruction is correct and costs the thing the whole design is built to avoid; checking only at
//! slice ends means a guest spinning inside one function — the exact freeze this channel exists to
//! break — cannot be stopped.
//!
//! **Test `HALT` before entering, not after leaving.** One slice may be 65 536 instructions, and
//! [`crate::layout::HALT`] is a *mapped* address, so a guest that branches there does not fault: it
//! executes whatever is in that page and runs on. The interpreter tests on every fetch; a
//! block-at-a-time backend tests at the top of each slice and stops with `Stop::Halted { addr }`
//! when `cpu.pc` is already there.
//!
//! **`cpu.mem` is the only memory.** There is no second address space and no way to register one.
//! An engine that insists on owning guest memory mirrors it into its own space, and refreshes that
//! mirror when [`crate::Mem::window_generation`] moves — that counter is precisely "the thing your
//! cached pointers describe has changed", and the Rynarmic adapter uses it for the same reason.
//! Reads and writes that fall outside the mirror have to be routed back to `Mem`, not faulted.
//!
//! **Do not call [`Cpu::run`] from inside `run`.** `Cpu::run` takes the backend out of its box
//! before calling, so a nested call finds the box empty and falls back to the interpreter. Wrap by
//! calling [`Cpu::run_interpreter`] instead, which is public for exactly this.
//!
//! **A turn is not free, and it is not priced per instruction.** Measured on the development host
//! (`benches/jit_frame.rs`), one `Cpu::run` that retires a *single* instruction costs about **29 ns**
//! end to end: 9.6 ns of entry (the diagnostic gate, the box, the halt-trap confirmation, the window
//! generation compare, the memory rebind), 17.3 ns of state synchronisation plus the backend's own
//! call, and 2.1 ns inside Rynarmic's trap. None of it depends on how many instructions the turn
//! retires. A backend that hands control back every ten instructions therefore pays ~2.9 ns per
//! instruction in overhead alone, against the ~0.7 ns Rynarmic charges to execute one — four times
//! the work for the privilege. Return where the guest gives you a boundary (an `SVC`, an exhausted
//! budget, a cancel checkpoint), not on a slice size picked for symmetry.
//!
//! **Diagnostics are not the backend's problem.** [`Cpu::run`] sends any run with trace,
//! breakpoints, watchpoints or sampling armed straight to the interpreter, whatever is installed,
//! because those features start from "every instruction is seen". A backend never has to report a
//! breakpoint hit it did not stop at — it never gets the chance.
//!
//! What an embedder does is therefore: implement the trait, hand the box to [`Cpu::set_backend`],
//! and ask nothing else. `tests/external_backend.rs` is that in thirty lines, using only public
//! paths, and it is a test rather than a claim so that "someone else can port an engine" stays true
//! as this file changes.

use super::{Cpu, JitArena, Stop};

/// One way of executing guest instructions.
///
/// Implementors answer only what they can: every capability method has a default, and the default
/// means "this backend has no such thing" rather than "not implemented". That is what lets the
/// interpreter be the empty `impl` at the bottom of this file.
pub trait Backend: Send + std::fmt::Debug + 'static {
    /// The backend as `Any`, so a caller that knows which one it installed can get at the rest of it.
    ///
    /// Two callers need this and neither is on a hot path: this crate's own tests read the published
    /// window array out of the Rynarmic adapter, and an embedder that wants a diagnostic detail the
    /// trait does not name (an arena's fill, a trace's length) should not have to make the trait
    /// grow a method for it. Everything a caller is *supposed* to use is on the trait.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Execute until the guest stops, traps, exhausts `budget`, or is cancelled.
    ///
    /// The contract is the interpreter's, and every backend owes it exactly: a `Stop::Hle` leaves
    /// `cpu.pc` on the instruction after the `SVC` call site and reports `at` as the `SVC` itself;
    /// `cpu.steps` only grows; `cpu.cancel` is sampled once every [`super::CANCEL_LOOK`]
    /// instructions; `Stop::Halted` means `pc` reached [`crate::layout::HALT`].
    fn run(&mut self, cpu: &mut Cpu, budget: u64) -> Stop;

    /// The name a report or an embedder's own diagnostics should print.
    fn name(&self) -> &'static str;

    /// Whether instructions are being translated rather than interpreted one at a time.
    ///
    /// This is the question `Cpu::jit_enabled` asks, and it is deliberately not `name() == "jit"`:
    /// the Rynarmic adapter can be told to run its own interpreter (`QUOIN_JIT_INTERP=1`), and that
    /// is still the translating backend as far as state synchronisation is concerned.
    fn translating(&self) -> bool {
        false
    }

    /// Blocks in the compiled-code cache, when this backend has one.
    ///
    /// Diagnostic: one call may walk the whole cache, so it has no business on a hot path.
    fn cached_blocks(&self) -> Option<usize> {
        None
    }

    /// How many new blocks this turn may compile; `None` means no limit.
    fn set_compile_budget(&mut self, _budget: Option<u32>) {}

    /// What this backend wants `set_compile_budget` to be when a caller has no opinion.
    ///
    /// The machine keeps a per-turn budget and asks this once, at construction. A backend that
    /// compiles things answers `Some(n)`; one that does not answers `None`, so the field reads as
    /// "nothing to spend" rather than as a knob with no effect.
    fn default_compile_budget(&self) -> Option<u32> {
        None
    }

    /// Size the code cache from a parsed `--jit-arena` / `QUOIN_JIT_ARENA` choice, answering the line
    /// the caller should log. `None` means there was nothing to size.
    fn size_arena(&mut self, _choice: JitArena, _code_bytes: u64) -> Option<String> {
        None
    }

    /// Set the code-cache ceiling directly, for callers that already computed it.
    fn set_arena_limit(&mut self, _limit: usize) {}

    /// Non-empty spans of the last published fast-access window set.
    fn windows(&self) -> Option<usize> {
        None
    }
}

/// The plain ARM executor: one instruction at a time, no cache, no translation.
///
/// It is a zero-sized type on purpose — there is nothing to keep between turns, because the machine
/// state is all in the `Cpu` it is handed, and the interpreter's own loop reads it from there.
#[derive(Debug)]
pub struct Interpreter;

impl Backend for Interpreter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn run(&mut self, cpu: &mut Cpu, budget: u64) -> Stop {
        // Straight to the quiet loop. `Cpu::run` answered the diagnostic gate before it reached
        // this box — a per-instruction run never gets here — and asking the same five questions a
        // second time is about a quarter of what this seam costs per call (`benches/interp.rs`).
        //
        // [`Cpu::run_interpreter`] is the entry that asks, and it is the one an outside backend
        // should call: that caller has not necessarily answered the gate itself.
        cpu.run_quiet(budget)
    }

    fn name(&self) -> &'static str {
        "interpreter"
    }
}
