//! Can an external crate plug into the execution seam - using only public `quoin::` paths, no private interface touched.
//!
//! This is not a test that the author can add another backend (`src/cpu/jit.rs` covers that);
//! it is a test that **someone else** can add one. An integration test under `tests/` has
//! exactly the visibility of a third-party crate: no `pub(crate)`, no `#[cfg(test)]` helpers.
//! So this file compiling and producing the right result is itself the proof the `Backend`
//! seam holds from outside - `Cpu::set_backend`, `Stop` / `layout::HALT`, `Mem::page_ptr` and `window_generation`.
//!
//! It is also an example: `Forwarding` is the usual starting point for an external backend
//! ("this slice is not for my engine, hand it back"); adapt it to attach QEMU TCG, Unicorn, etc.

use std::any::Any;

use quoin::cpu::backend::Backend;
use quoin::cpu::{Cpu, Stop};
use quoin::layout;
use quoin::Mem;

/// A real backend: forwards every slice to the interpreter, recording how many it received and retired.
///
/// `run_interpreter` is `pub` precisely so a wrapping backend like this can exist - it runs the
/// same loop as [`Cpu::run`], so "my engine cannot do this instruction" has a correct answer, not a fake `Budget`.
#[derive(Debug, Default)]
struct Forwarding {
    slices: u32,
    retired: u64,
}

impl Backend for Forwarding {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "external-forwarding"
    }

    fn run(&mut self, cpu: &mut Cpu, budget: u64) -> Stop {
        self.slices += 1;
        let before = cpu.steps;
        let stop = cpu.run_interpreter(budget);
        self.retired += cpu.steps - before;
        stop
    }
}

/// Code page, on the same page as `HALT`; see [`machine`].
const CODE: u32 = 0x0FFF_F000;

/// A two-word machine: `MOV r0, #7` at `CODE`, `B HALT` at `CODE + 4`.
///
/// `HALT` shares the page instead of sitting at `0x1000`: ARM `B` has only a 24-bit **signed**
/// offset (+/-32 MiB), and `0x1000` to `0x0FFF_FFF0` is 268 MiB. The first version jumped to
/// `0xFFFF_FFF0` and failed at fetch. Same-page offset is `0xFE4`, so the short branch works.
///
/// The `HALT` page is mapped explicitly: an unmapped page fails at fetch first, not at the backend's stop condition.
fn machine() -> Cpu {
    let mut mem = Mem::new();
    mem.map(CODE, 0x1000);
    mem.write_u32(CODE, 0xE3A0_0007).expect("MOV r0, #7");
    // `B` targets `HALT`: offset = (0x0FFF_FFF0 - (0x0FFF_F004 + 8)) >> 2 = 0x3F9.
    mem.write_u32(CODE + 4, 0xEA00_03F9).expect("B HALT");
    let mut cpu = Cpu::new(mem);
    cpu.pc = CODE;
    cpu
}

#[test]
fn an_outside_backend_can_be_installed_and_drives_the_machine_to_halt() {
    let mut cpu = machine();
    cpu.set_backend(Box::new(Forwarding::default()));
    assert_eq!(
        cpu.backend_name(),
        "external-forwarding",
        "the machine reports what an embedder installed"
    );

    let stop = cpu.run(64);

    match stop {
        Stop::Halted { addr } => assert_eq!(addr, layout::HALT, "stopped on the sentinel"),
        other => panic!("expected Halted, got {other:?}"),
    }
    assert_eq!(
        cpu.r[0], 7,
        "the MOV really ran through the installed backend"
    );
    assert_eq!(
        cpu.steps, 2,
        "two instructions retired — `steps` is a clock, so the count is exact, not a guess"
    );

    // Read the backend's own bookkeeping back. `as_any` is the only way an external crate reads
    // details the trait does not name; this also proves it is usable outside the crate.
    let seen = cpu
        .backend
        .as_ref()
        .expect("the box is only empty during a call")
        .as_any()
        .downcast_ref::<Forwarding>()
        .expect("that is the backend we installed");
    assert_eq!(seen.slices, 1, "one turn, one entry into the backend");
    assert_eq!(seen.retired, 2, "and it saw both retirements itself");
}

#[test]
fn the_budget_splits_the_run_and_the_outside_backend_sees_every_slice() {
    let mut cpu = machine();
    cpu.set_backend(Box::new(Forwarding::default()));

    match cpu.run(1) {
        Stop::Budget { steps } => assert_eq!(steps, 1, "the ceiling was honoured exactly"),
        other => panic!("expected Budget, got {other:?}"),
    }
    assert_eq!(cpu.r[0], 7, "the first instruction retired before the stop");

    // The second instruction is `B HALT`, sending pc to the sentinel. The budget also runs out here,
    // so the answer is `Budget`; `HALT` is seen on the **next entry**, like the JIT's pre-entry check.
    match cpu.run(1) {
        Stop::Budget { steps } => assert_eq!(steps, 2),
        other => panic!("expected Budget, got {other:?}"),
    }
    assert_eq!(
        cpu.pc,
        layout::HALT,
        "the branch really landed on the sentinel"
    );

    match cpu.run(1) {
        Stop::Halted { addr } => assert_eq!(addr, layout::HALT),
        other => panic!("expected Halted, got {other:?}"),
    }

    let seen = cpu
        .backend
        .as_ref()
        .unwrap()
        .as_any()
        .downcast_ref::<Forwarding>()
        .unwrap();
    assert_eq!(seen.slices, 3, "every turn enters the backend again");
    assert_eq!(seen.retired, 2, "and only two instructions ever retired");
}

#[test]
fn an_embedder_can_put_the_interpreter_back() {
    // The external crate removes its own backend through the same public piece, not by clearing a field.
    //
    // Install its own then switch back: the default backend **already is** the interpreter, so
    // asserting only the final name would let a no-op `set_backend` pass - the first version did.
    let mut cpu = machine();
    cpu.set_backend(Box::new(Forwarding::default()));
    assert_eq!(
        cpu.backend_name(),
        "external-forwarding",
        "the door really swaps"
    );

    cpu.set_backend(Box::new(quoin::cpu::backend::Interpreter));
    assert_eq!(cpu.backend_name(), "interpreter", "and swaps back");

    match cpu.run(64) {
        Stop::Halted { addr } => assert_eq!(addr, layout::HALT),
        other => panic!("expected Halted, got {other:?}"),
    }
    assert_eq!(cpu.r[0], 7);
}

#[test]
fn an_outside_backend_can_mirror_guest_memory_through_the_public_seam() {
    // The minimal QEMU/Unicorn shape: those backends own a guest address space and need host memory
    // pointers page by page. That path is open from outside, and "when to fetch again" has an answer.
    let cpu = machine();
    let generation = cpu.mem.window_generation();

    let page = cpu
        .mem
        .page_ptr((CODE >> 12) as usize)
        .expect("the code page is mapped");
    // SAFETY: `page_ptr` promises the page stays valid for the borrow and holds the 4 KiB at its start;
    // this is the first word of that page. Read-only; nothing that could move during the borrow is dereferenced.
    let word = unsafe { std::ptr::read_unaligned(page as *const u32) };
    assert_eq!(
        word, 0xE3A0_0007,
        "the outside view and the guest see one memory"
    );

    assert_eq!(
        generation,
        cpu.mem.window_generation(),
        "reading a page invalidates nothing — only unmap/remap does"
    );
}
