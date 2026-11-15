//! Backend cross-check: for the same `.mod`, the interpreter and Rynarmic backend must emit the same HLE call sequence.
//!
//! Only does work with `--features jit` and `BREW_TEST_MOD` set.
//!
//! Why "call sequence + register snapshot" rather than "final result": a final result only says
//! "wrong" at the last step, while a sequence aligns both sides at method boundaries - the first differing method or register is the divergence.
//!
//! It also pins down three things that only show up when the execution core changes:
//! - `SVC` resume semantics (the stub returns from `LR` like a function, bit0 carrying interwork state);
//! - state sync (`r0..r14`, `pc`, `cpsr` aligned on every slice entry and exit);
//! - memory equivalence (every byte read by both sides comes from the same `Mem`).

#![cfg(feature = "jit")]

use quoin::abi;
use quoin::cpu::Stop;
use quoin::runtime::Runtime;

/// The register snapshot of one HLE call.
#[derive(Clone, PartialEq, Eq)]
struct Point {
    label: &'static str,
    r: [u32; 16],
    cpsr: u32,
}

fn image() -> Option<Vec<u8>> {
    let path = std::env::var_os("BREW_TEST_MOD")?;
    if path.is_empty() {
        return None;
    }
    Some(std::fs::read(&path).expect("BREW_TEST_MOD readable"))
}

/// Drive the stop/dispatch loop by hand (isomorphic to `Runtime::run`'s quiet path) and record each HLE call's snapshot.
fn run(image: &[u8], jit: bool) -> (Vec<Point>, String) {
    let mut rt = Runtime::new();
    if jit {
        rt.cpu.enable_jit();
        assert!(rt.cpu.jit_enabled());
    }
    rt.load(image, false).expect("load the .mod");

    let mut points = Vec::new();
    let summary = loop {
        match rt.cpu.run(200_000) {
            Stop::Hle { id, at } => {
                let mut r = [0u32; 16];
                r[..15].copy_from_slice(&rt.cpu.r);
                r[15] = rt.cpu.pc;
                points.push(Point {
                    label: abi::label(id),
                    r,
                    cpsr: rt.cpu.cpsr,
                });
                rt.dispatch(id, at).expect("dispatch");
            }
            // The stop address must match; `steps` is not compared - block granularity makes them differ by a few instructions.
            Stop::Halted { addr } => break format!("Halted at {addr:#010x}"),
            Stop::Budget { .. } => {}
            Stop::Cancelled { pc } => break format!("Cancelled pc={pc:#010x}"),
            Stop::Fault(f) => break format!("Fault {f}"),
            Stop::Stopped { why } => break format!("Stopped {why:?}"),
        }
        if rt.cpu.steps > 4_000_000 {
            break "still not stopped after 4 million instructions".to_string();
        }
    };
    (points, summary)
}

#[test]
fn jit_backend_matches_the_interpreter_call_by_call() {
    let Some(image) = image() else {
        eprintln!("skipped: set BREW_TEST_MOD to a classic BREW .mod to run the parity check");
        return;
    };

    let (interp, interp_summary) = run(&image, false);
    let (jit, jit_summary) = run(&image, true);

    for (index, (a, b)) in interp.iter().zip(jit.iter()).enumerate() {
        assert_eq!(
            a.label, b.label,
            "HLE call {index} is already a different method: interpreter {} / JIT {}",
            a.label, b.label
        );
        assert_eq!(
            a.cpsr, b.cpsr,
            "CPSR differs at call {index} ({}): {:#010x} / {:#010x}",
            a.label, a.cpsr, b.cpsr
        );
        assert_eq!(
            a.r, b.r,
            "registers differ at call {index} ({})\n  interpreter {:08x?}\n  JIT    {:08x?}",
            a.label, a.r, b.r
        );
    }
    assert_eq!(interp.len(), jit.len(), "the number of calls differs");
    assert_eq!(interp_summary, jit_summary, "call snapshots match but the stop result differs");
}
