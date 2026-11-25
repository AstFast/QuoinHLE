//! M1: a real Qualcomm-built `.mod` must come up against the emulated AEE runtime.
//!
//! `helloworld.mod` is linker output from the proprietary Sample builds, so it is not vendored
//! here; point `BREW_TEST_MOD` at a copy to run the test, which then skips silently when the
//! variable is absent. What it pins down is the part that cannot be faked: the guest has to
//! decode its own prologue, walk the host-synthesised `IShell` vtable, dispatch slot 0 through
//! the `SVC` seam, and hand back `AEESUCCESS` with an `IModule*` in the caller's out-param.

use quoin::abi;
use quoin::abi::generated as g;
use quoin::runtime::{Outcome, Runtime};

fn image() -> Option<Vec<u8>> {
    let path = std::env::var_os("BREW_TEST_MOD")?;
    if path.is_empty() {
        return None;
    }
    match std::fs::read(&path) {
        Ok(image) => Some(image),
        Err(e) => panic!("BREW_TEST_MOD={path:?} is not readable: {e}"),
    }
}

#[test]
fn aeemod_load_reaches_ishell_and_returns_aeetsuccess() {
    let Some(image) = image() else {
        eprintln!("skipped: set BREW_TEST_MOD to a classic BREW .mod to run M1");
        return;
    };

    let mut rt = Runtime::new();
    // Backend selection: with QUOIN_JIT=1 this whole integration test runs on the Rynarmic backend (build with `--features jit`).
    // The library itself never reads this variable; only integration tests do.
    #[cfg(feature = "jit")]
    if std::env::var_os("QUOIN_JIT").is_some() {
        rt.cpu.enable_jit();
    }
    rt.load(&image, false).expect("load the .mod");
    let retval = match rt.run().expect("the guest must not fault") {
        Outcome::Halted { retval, steps } => {
            assert!(
                steps > 30,
                "the loader's stub run should do real work: {steps}"
            );
            retval
        }
        Outcome::Stepped { steps, spun } => {
            panic!("still running after {steps} instructions, without reaching the host: {spun}")
        }
        // Nothing here arms a watchpoint or a breakpoint, so a stop would mean this test is lying about
        // what it ran — which is why it says so rather than being folded into one of the others.
        Outcome::Stopped { why, steps } => {
            panic!("stopped at {why:?} after {steps} instructions")
        }
        // Same for a cancel: nobody set the flag, so one arriving means something armed it by accident.
        Outcome::Cancelled { pc, steps } => {
            panic!("cancelled at {pc:#010x} after {steps} instructions, and nothing armed it")
        }
    };
    assert_eq!(retval, g::AEE_SUCCESS, "AEEMod_Load must return AEESUCCESS");

    // Slot 0 of IShell, reached through the synthesised vtable and the stub page — not by us
    // calling into the runtime ourselves.
    let addref = abi::method_of("IShell", "AddRef").expect("IShell::AddRef");
    assert_eq!(
        rt.hle.call_count(addref),
        1,
        "the guest should dispatch IShell::AddRef exactly once, got {:?}",
        rt.call_log()
    );
    // The module came in holding one reference and AddRef'd for the one it keeps.
    assert_eq!(
        rt.cpu.mem.read_u32(rt.shell + 4).expect("shell refcount"),
        2,
        "the shell's guest-visible refcount should have moved to 2"
    );

    let malloc = abi::helper_id(abi::offset_of("AEEHelperFuncs", "malloc").expect("malloc slot"));
    assert_eq!(
        rt.hle.call_count(malloc),
        1,
        "the applet allocation goes through the helper table"
    );

    let module = rt.created_module().expect("*ppMod is mapped");
    assert_ne!(module, 0, "AEEMod_Load must publish an IModule*");
    assert_eq!(
        rt.cpu.mem.read_u32(module + 4).expect("module refcount"),
        1,
        "the new object starts life with the one reference its caller owns"
    );
}
