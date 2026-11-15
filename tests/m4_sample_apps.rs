//! M4 against the shipped examples: every official sample module has to load, and its applet has to
//! be reachable at the class id the module itself answers for.
//!
//! Skipped unless `BREW_EXAMPLES` names the SDK's `examples` directory, because the images are
//! linker output from the proprietary samples and are not vendored here — and must not be: this
//! repository contains no SDK file and does not need one to build or to run the default suite.
//! The variable is a maintainer's opt-in, not a requirement; unset, every test in this file prints
//! `skipped` and returns.
//!
//! The point of this file is that a green run means something. An earlier version accepted "either
//! the applet started, or the host named something it owes" — and with five of the seven samples
//! that was true while none of them drew anything, so the suite was green against a target nobody
//! had reached. Two changes make the red visible:
//!
//!   * each app declares the *exact* gap it is expected to stop on, so a new reason for failing is
//!     a failure rather than a passing test; and
//!   * an app declared as starting has to have put a pixel on the screen, not just returned success.

use quoin::abi::generated as g;
use quoin::hle::Error as HleError;
use quoin::runtime::{Error, Outcome, Runtime};

/// What a sample is expected to do today, measured rather than assumed.
#[derive(Debug)]
enum Expect {
    /// The applet starts, and the framebuffer is no longer one flat colour.
    Paints,
    /// The applet does not start, and the reason is exactly this: a resource file this host has no
    /// copy of, a service class it has not written, or a slot it has not implemented. The string is
    /// what the run reports, so changing *why* an app fails turns this test red on purpose.
    Gap(&'static str),
    /// The same gap, named by the host — and then the app dereferenced the null the host handed it
    /// and faulted. Still the host's gap to close, but a separate declaration: "it stopped where it
    /// was told" and "it crashed in the hole we left" are different facts, and the previous version
    /// of this test accepted both as a pass.
    GapThenFault(&'static str),
}

/// How a run ended, as far as the host can account for it.
#[derive(Debug, PartialEq)]
enum Stopped {
    /// `StartApplet` answered success and an applet is running.
    Started,
    /// The module took the class and answered a failure value, having asked for something owed.
    Refused(String),
    /// The host stopped the run by naming the slot it has not implemented.
    Named(String),
    /// The host named what it owed, and the app then faulted on the null it was given.
    FaultedAfterGap(String),
    /// A guest fault with nothing owed to account for it. That is a host bug, and always a failure.
    Unreportable(String),
}

/// Each module answers `IModule::CreateInstance` by comparing the class the shell asked for with a
/// literal in its own image, so these ids are the shell's choice for nobody: they are read back out
/// of the shipped binary, from the `ldr` that feeds the compare at the head of the module's own
/// `CreateInstance`. The AppGen default `0x0a000001` is what the small samples ship, and the two
/// Qualcomm apps below carry ids of their own and reject the default.
struct App {
    name: &'static str,
    cls: u32,
    expect: Expect,
}

const APPS: &[App] = &[
    App {
        name: "helloworld",
        cls: 0x0a00_0001,
        expect: Expect::Paints,
    },
    App {
        name: "usermodeapp",
        cls: 0x0a00_0001,
        expect: Expect::Paints,
    },
    App {
        name: "expensetracker",
        cls: 0x0a00_0001,
        // The `(image)` on a resource name is the report's own wording rather than this test's: the
        // id alone is half the question the pack asked, so `resources the host could not serve`
        // carries the kind it was filed under too. All three of these packs ask for a table under
        // the image kind whatever the payload turns out to be.
        expect: Expect::Gap("ExpenseTracker.bar#0x0010 (image)"),
    },
    App {
        name: "netdiagnostics",
        cls: 0x0a00_0001,
        expect: Expect::Gap("AEECLSID_HTML"),
    },
    App {
        name: "whiteboard",
        cls: 0x0a00_0001,
        expect: Expect::GapThenFault("Whiteboard.bar#0x001a (image)"),
    },
    App {
        name: "mediaplayer",
        cls: 0x0101_0ef6,
        expect: Expect::GapThenFault("MediaPlayer.bar#0x0003 (image)"),
    },
    App {
        name: "roadwarrior",
        cls: 0x0101_0133,
        // `IShell::GetPrefs` stood here until that slot was implemented; the pack gets past it now
        // and stops on the art file this host has no copy of, which is a gap a sample tree without
        // the app's own `.bar` can do nothing about.
        expect: Expect::Gap("roadwarrior.bar#0x0016 (image)"),
    },
];

fn examples_dir() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("BREW_EXAMPLES")?;
    if dir.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(&dir);
    assert!(
        path.is_dir(),
        "BREW_EXAMPLES={dir:?} has to name the SDK's examples directory"
    );
    Some(path)
}

fn load_and_run(path: &std::path::Path) -> Runtime {
    let image = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut rt = Runtime::new();
    // Backend selection: with QUOIN_JIT=1 this whole integration test runs on the Rynarmic backend (build with `--features jit`).
    // The library itself never reads this variable; only integration tests do.
    #[cfg(feature = "jit")]
    if std::env::var_os("QUOIN_JIT").is_some() {
        rt.cpu.enable_jit();
    }
    rt.load(&image, false).expect("map the .mod");
    match rt.run().expect("AEEMod_Load") {
        Outcome::Halted { retval, .. } => {
            assert_eq!(
                retval,
                g::AEE_SUCCESS,
                "{} published its module",
                path.display()
            );
        }
        Outcome::Stepped { steps, spun } => {
            panic!(
                "{} did not finish AEEMod_Load in {steps} instructions (spun: {spun})",
                path.display()
            )
        }
        // Nothing here arms a watchpoint or a breakpoint, so a stop would mean this test is lying about
        // what it ran.
        Outcome::Stopped { why, steps } => panic!(
            "{} stopped at {why:?} after {steps} instructions",
            path.display()
        ),
        // Nor a cancel: no session is watching this run, so a cancel means something armed it by mistake.
        Outcome::Cancelled { pc, steps } => panic!(
            "{} was cancelled at {pc:#010x} after {steps} instructions",
            path.display()
        ),
    }
    rt
}

/// Whether anything reached the screen. A fresh display boots to the system background and
/// `StartApplet` clears it to that same colour, so a framebuffer that is not one flat colour can
/// only mean something was drawn into it. Success out of `StartApplet` is not this: an applet can
/// build its whole world and paint nothing.
fn painted(rt: &Runtime) -> bool {
    let rgb = rt
        .display
        .screenshot(&rt.cpu.mem)
        .expect("the device bitmap is readable");
    assert_eq!(rgb.len() % 3, 0, "one RGB triple per pixel");
    let (pixels, _) = rgb.as_chunks::<3>();
    !pixels.iter().all(|p| p == &pixels[0])
}

/// How the run ended, as far as the host can account for it.
fn stopped(rt: &Runtime, outcome: &Result<u32, Error>) -> Stopped {
    // What this host owes, in the order it was first asked for. Both ledgers are append-only and
    // deduplicated by the runtime, so the first entry is the gap the run reached, not the last.
    let owed = || {
        rt.missing_resources.first().cloned().or_else(|| {
            rt.missing_classes.first().map(|c| {
                g::CONST_TABLE
                    .iter()
                    .find(|k| k.value == *c && k.name.starts_with("AEECLSID_"))
                    .map(|k| k.name.to_string())
                    .unwrap_or_else(|| format!("{c:#010x}"))
            })
        })
    };
    match outcome {
        Ok(g::AEE_SUCCESS) => Stopped::Started,
        Ok(rv) => match owed() {
            Some(why) => Stopped::Refused(why),
            None => Stopped::Unreportable(format!("{rv:#010x} with nothing owed")),
        },
        Err(Error::Hle(HleError::Unimplemented { label, .. }))
        | Err(Error::Hle(HleError::Unsupported { what: label })) => Stopped::Named(label.clone()),
        Err(other) => match owed() {
            // A shipped app that treats the null we handed it for a missing resource as a pointer is
            // the app's own bug — a handset with the file on disk never reaches it. But the host has
            // to have said what it could not serve *first*, or the fault is ours, not its.
            Some(why) => Stopped::FaultedAfterGap(why),
            None => Stopped::Unreportable(other.to_string()),
        },
    }
}

/// Every sample either paints, or stops on the one gap this host declares it owes — and never takes
/// the emulator down doing it.
#[test]
fn each_sample_either_paints_or_stops_on_the_gap_it_declares() {
    let Some(dir) = examples_dir() else {
        eprintln!("skipped: set BREW_EXAMPLES to the SDK's examples directory to run M4");
        return;
    };
    for app in APPS {
        let path = dir.join(app.name).join(format!("{}.mod", app.name));
        let mut rt = load_and_run(&path);
        let outcome = rt.start_app(app.cls, None);
        let report = || {
            format!(
                "{}: StartApplet({:#010x}) -> {:?}\n{}",
                app.name,
                app.cls,
                outcome.as_ref().map_err(|e| e.to_string()),
                rt.call_log()
            )
        };
        // The invariant that holds however much of the handset is implemented: no run-away nesting
        // and no exhausted instruction budget. Those are the emulator failing, not the app.
        if let Err(Error::Hle(HleError::TooDeep { .. } | HleError::NoReturn { .. })) = &outcome {
            panic!("{} ran away: {}", report(), outcome.as_ref().unwrap_err());
        }
        let how = stopped(&rt, &outcome);
        match (&app.expect, &how) {
            (Expect::Paints, Stopped::Started) => {
                assert_ne!(rt.applet, 0, "{} has an applet", report());
                assert!(
                    painted(&rt),
                    "{} started its applet but drew nothing\n{}",
                    app.name,
                    report()
                );
            }
            (Expect::Gap(want), Stopped::Refused(got) | Stopped::Named(got)) => {
                assert_eq!(want, got, "{} stopped on something else", report());
                assert_eq!(rt.applet, 0, "{} did not start an applet", report());
            }
            (Expect::GapThenFault(want), Stopped::FaultedAfterGap(got)) => {
                assert_eq!(want, got, "{} faulted in something else", report());
            }
            (want, got) => panic!(
                "{}: declared {}, the run did {got:?}\n{}",
                app.name,
                match want {
                    Expect::Paints => "paints".to_string(),
                    Expect::Gap(w) => format!("stops cleanly on {w:?}"),
                    Expect::GapThenFault(w) => format!("faults after {w:?}"),
                },
                report()
            ),
        }
    }
}

/// The paint check has to be able to say no. Without this, `painted` could be vacuously true — a
/// framebuffer that boots non-uniform would let every `Paints` declaration pass without anything
/// having been drawn — and the tightening this file exists for would be an illusion.
#[test]
fn the_paint_check_is_false_until_something_is_drawn() {
    let Some(dir) = examples_dir() else {
        eprintln!("skipped: set BREW_EXAMPLES to the SDK's examples directory to run M4");
        return;
    };
    let path = dir.join("helloworld").join("helloworld.mod");
    let mut rt = load_and_run(&path);
    assert!(
        !painted(&rt),
        "a module that has only loaded has drawn nothing"
    );
    rt.start_app(0x0a00_0001, None).expect("helloworld starts");
    assert!(painted(&rt), "and helloworld draws its text");
}

/// The goal, kept honest and visible: every shipped sample is supposed to start *and* put pixels on
/// the screen. Today five of seven stop on a gap this host owes, so this is `#[ignore]`d rather
/// than quietly dropped — run `cargo test -- --ignored` to see how far that still is, and un-ignore
/// it when it passes.
#[test]
#[ignore = "the target, not the current state: five samples still stop on gaps this host owes"]
fn every_sample_applet_starts_and_paints() {
    let Some(dir) = examples_dir() else {
        eprintln!("skipped: set BREW_EXAMPLES to the SDK's examples directory to run M4");
        return;
    };
    for app in APPS {
        let path = dir.join(app.name).join(format!("{}.mod", app.name));
        let mut rt = load_and_run(&path);
        let rv = rt
            .start_app(app.cls, None)
            .unwrap_or_else(|e| panic!("{}: {e}", app.name));
        assert_eq!(rv, g::AEE_SUCCESS, "{} started its applet", app.name);
        assert!(painted(&rt), "{} drew something", app.name);
    }
}

/// The ids in `APPS` were read out of each module's own class comparison by hand. This is the test
/// that means nobody has to do that again: asking the module which of the class ids in its image it
/// owns has to arrive at the same answer, for the modules that compare *and* the ones that do not.
#[test]
fn discovery_names_the_id_the_module_compares_against() {
    let Some(dir) = examples_dir() else {
        eprintln!("skipped: set BREW_EXAMPLES to the SDK's examples directory to run M4");
        return;
    };
    for app in APPS {
        let path = dir.join(app.name).join(format!("{}.mod", app.name));
        let mut rt = load_and_run(&path);
        let found = rt
            .discover_applet_class()
            .unwrap_or_else(|e| panic!("{}: discovery faulted: {e}", app.name));
        assert_eq!(
            found.cls, app.cls,
            "{}: the module claims {:#010x} after {} of its own class ids, not {:#010x}",
            app.name, found.cls, found.tried, app.cls
        );
        assert_ne!(found.cls, 0, "{} answered the question", app.name);
        assert_eq!(
            rt.applet, 0,
            "{}: discovery asks which class is owned, it does not start it",
            app.name
        );
    }
}

/// The two apps that carry their own id are the reason the id cannot be guessed: the module compares
/// the class before it does anything else, and hands back what it does not own.
#[test]
fn an_app_that_carries_its_own_id_rejects_the_default() {
    let Some(dir) = examples_dir() else {
        eprintln!("skipped: set BREW_EXAMPLES to the SDK's examples directory to run M4");
        return;
    };
    for name in ["mediaplayer", "roadwarrior"] {
        let path = dir.join(name).join(format!("{name}.mod"));
        let mut rt = load_and_run(&path);
        // The decline is the module's own answer, in the value it returns: `AEEModule_CreateInstance`
        // compares the class first and hands back anything it does not own.
        let rv = rt
            .start_app(0x0a00_0001, None)
            .expect("the module answers a class it does not own with a value");
        assert_ne!(
            rv,
            g::AEE_SUCCESS,
            "{name} accepted a class it does not own"
        );
        assert!(
            rt.missing_classes.is_empty(),
            "{name} declined the class before it wanted anything: {:?}",
            rt.missing_classes
        );
        assert_eq!(rt.applet, 0, "and nothing is running");
    }
}
