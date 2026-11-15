//! M2b-ii and M3 against a real `.mod`: the runtime calls *into* the guest, the applet lives and
//! dies by the shell's rules, and what it draws arrives as pixels.
//!
//! Skipped unless `BREW_TEST_MOD` names a readable image, because the module is linker output
//! from the proprietary SDK samples and is not vendored here. Every address this test asserts on
//! is read back out of guest memory instead of being hardcoded, so a host that builds an
//! `_AEEApplet` the module cannot use fails here rather than at some later PC.

use quoin::abi::{self, generated as g};
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

/// The class id this test asks for. `helloworld`'s module answers whatever class the shell names, so
/// here the id is the shell's choice — but it is not one that travels: the samples that compare the
/// class against a literal of their own need the id read back out of their image, which is what
/// `m4_sample_apps.rs` pins down.
const CLS: u32 = 0x0a00_0001;

#[test]
fn hello_world_starts_paints_and_closes() {
    let Some(image) = image() else {
        eprintln!("skipped: set BREW_TEST_MOD to a classic BREW .mod to run M2b-ii and M3");
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
    assert!(
        matches!(rt.run().expect("AEEMod_Load"), Outcome::Halted { .. }),
        "the load has to finish before the shell can start anything"
    );

    let create = abi::method_of("IShell", "CreateInstance").expect("IShell::CreateInstance");
    rt.start_app(CLS, None).expect("StartApplet");
    assert_ne!(rt.applet, 0, "an applet is running");
    assert_eq!(
        rt.hle.call_count(create),
        1,
        "the applet came from the module, not from the host\n{}",
        rt.call_log()
    );

    let field = |name: &str| {
        abi::offset_of("_AEEApplet", name)
            .unwrap_or_else(|| panic!("_AEEApplet.{name} in the tables"))
    };
    let word = |addr: u32| {
        rt.cpu
            .mem
            .read_u32(addr)
            .unwrap_or_else(|e| panic!("{addr:#010x}: {e}"))
    };
    assert_eq!(word(rt.applet + field("clsID")), CLS);
    assert_eq!(word(rt.applet + field("m_pIShell")), rt.shell);
    assert_ne!(
        word(rt.applet + field("m_pIDisplay")),
        0,
        "AEEApplet_New hands the applet a display"
    );
    // The applet is the guest's own object, so its vtable is too: `AEEApplet_New` writes one just
    // past the struct, holding the module's `AEEAPPLETVTABLE`. The host dispatches events the same
    // way a handset does — by reading a slot and branching to it.
    let vtbl = word(rt.applet);
    assert_eq!(
        vtbl,
        rt.applet + 32,
        "_AEEApplet and its vtable, one allocation"
    );
    let release = word(vtbl + 4);
    let handle = word(vtbl + 8);
    // Odd because the sample is Thumb code from here on: `branch_to` takes the tag off the
    // address, and a host that dropped it would run the first halfword as an ARM instruction.
    assert_eq!(handle & 1, 1, "IApplet::HandleEvent is a Thumb pointer");
    // The stored `AEEHANDLER` is a third address: the shim above loads it and tail-branches, so
    // the events below are counted at the shim and the drawing happens inside the handler.
    let handler = word(rt.applet + field("pAppHandleEvent"));
    assert_eq!(handler & 1, 1, "the handler the module registered");
    assert_ne!(
        handler, handle,
        "the host calls the vtable, the guest calls the handler"
    );

    let (screen_w, screen_h) = (rt.device.width as i32, rt.device.height as i32);
    let run = {
        let r = rt
            .display
            .runs
            .first()
            .expect("the applet drew at startup, and once only");
        (
            r.text.clone(),
            r.font,
            r.flags,
            r.x,
            r.y,
            r.width as i32,
            r.height as i32,
        )
    };
    assert_eq!(
        rt.hle.downcall_count(handle),
        1,
        "one event so far: EVT_APP_START\n{:?}",
        rt.hle.downcalls()
    );
    assert_eq!(rt.display.presentations, 1, "and the Update that followed");
    let (text, font, flags, x, y, width, height) = run;
    assert_eq!(
        text, "Hello World",
        "the AECHAR units came out of guest memory"
    );
    assert_eq!(font, g::AEE_FONT_BOLD);
    assert_eq!(flags, g::IDF_ALIGN_CENTER | g::IDF_ALIGN_MIDDLE);
    assert_eq!(x, (screen_w - width) / 2, "centred in the screen");
    assert_eq!(y, (screen_h - height) / 2, "and in the middle of it");

    // The pixels are where that says they are. The top row of an `H` is its two stems, five
    // columns apart in a five-wide cell, with the bold face thickening the left one.
    let shot = rt
        .display
        .screenshot(&rt.cpu.mem)
        .expect("the device bitmap");
    let white = [255u8, 255, 255];
    let px = |col: i32, row: i32| {
        let i = ((y + row) as usize * screen_w as usize + (x + col) as usize) * 3;
        [shot[i], shot[i + 1], shot[i + 2]]
    };
    assert_eq!(px(0, 0), [0, 0, 0], "the H's left stem");
    assert_eq!(px(2, 0), white, "the gap inside the H");
    assert_eq!(px(4, 0), [0, 0, 0], "its right stem");
    assert_eq!(px(0, 3), [0, 0, 0], "the crossbar");
    assert_eq!(px(5, 0), white, "the leading between characters");
    assert_eq!(px(0, height - 1), white, "the cell below the baseline");

    // A key the sample ignores still costs the three downcalls the shell makes for it — the
    // press, the `EVT_KEY` the shell puts between them, and the release — and nothing is drawn
    // on the way.
    assert_eq!(
        rt.press_key(g::AVK_DOWN).expect("a key press"),
        0,
        "helloworld handles no keys, and says so"
    );
    assert_eq!(
        rt.hle.downcall_count(handle),
        4,
        "a press, a key and a release on top of the start\n{:?}",
        rt.hle.downcalls()
    );
    assert_eq!(rt.display.presentations, 1, "no further drawing");

    // The stop event goes to the same handler, and a handler that does not refuse it lets the
    // shell hand the applet back its own `Release`.
    rt.close_applet(false).expect("CloseApplet");
    assert_eq!(rt.applet, 0, "nothing is running any more");
    assert_eq!(rt.app_cls, 0);
    assert_eq!(
        rt.hle.downcall_count(handle),
        5,
        "and it was asked to stop\n{:?}",
        rt.hle.downcalls()
    );
    assert_eq!(
        rt.hle.downcall_count(release),
        1,
        "the guest released the applet it allocated"
    );
}
