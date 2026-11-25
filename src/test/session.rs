//! The two-thread session, measured on a machine that is deliberately made to hang.
//!
//! These run hand-written ARM words, not a pack: the whole point is a guest function that never
//! returns, and the only honest way to have one is to write one. Nothing here reads the sample set.

use crate::abi::generated as g;
use crate::runtime::{Job, Runtime};
use crate::session::{
    catch_up, deliver, pace, End, Guest, Queued, Sound, Status, ALIGN_CATCHUP_MS,
    ALIGN_DEBT_CAP_MS, ALIGN_RESUME_GAP, SPEED_ONE,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The window thread's own budget for a claim about it. Generous by orders of magnitude: what is being
/// asserted is that a status read costs a lock rather than a run, and a run that cannot return costs
/// twelve seconds or more on this interpreter.
const ANSWERED: Duration = Duration::from_millis(250);

/// How long a test will wait for the worker to take a turn before calling it a hang.
const SETTLING: Duration = Duration::from_millis(400);

/// One instruction, forever: `b .`.
fn stuck(rt: &mut Runtime) -> u32 {
    let addr = rt.cpu.mem.alloc(4, 4).unwrap();
    rt.cpu.mem.write_u32(addr, 0xeaff_fffe).unwrap();
    addr
}

/// The claim this whole restructure was built for, said as a measurement rather than as a design.
/// A pack spinning inside one guest function used to take the window with it, because the same thread
/// was doing both. Here the guest thread is inside that function, and the window thread asks it twice
/// how it is doing and then ends it.
///
/// The budget is set to the size a *played* run uses rather than the 2-million probe default, and for
/// the reason that distinction exists: with the probe default this hang is over in a few hundred
/// milliseconds, so the measurement below would be timing a run that had already ended on its own.
#[test]
fn a_guest_that_never_returns_cannot_take_the_window_with_it() {
    let mut rt = Runtime::new();
    rt.step_limit = 200 * crate::cpu::CANCEL_LOOK;
    rt.step_ceiling = u64::MAX;
    let loop_at = stuck(&mut rt);
    // A queued callback into the loop is exactly what a stuck pack looks like from in here: the turn
    // starts and does not end.
    rt.queue.push_back(Job::Notify {
        pfn: loop_at,
        user: 0,
    });
    // `Sound::Silent`, and it is load-bearing for the measurement below rather than tidy: a run that
    // opens the machine's audio device costs 120 to 170 ms more to start, which lands squarely on the
    // second that `finish()` is held to, and it makes this test's timing depend on what else on the
    // box is using the sound card.
    let guest = Guest::start(rt, Sound::Silent, None);
    let shared = Arc::clone(&guest.shared);
    std::thread::sleep(SETTLING);

    let began = Instant::now();
    let status = shared.status();
    let cost = began.elapsed();
    assert_eq!(
        status.why,
        End::Running,
        "the guest is still inside the function that has no return in it"
    );
    assert!(
        cost < ANSWERED,
        "asking the run how it was going cost {cost:?}; that question must never wait on the guest"
    );

    let began = Instant::now();
    let finished = guest.finish();
    let waited = began.elapsed();
    // Both halves, and they are different claims. This one says the ask was obeyed while the guest was
    // mid-function — the turn would otherwise have run for seconds more.
    assert!(
        waited < Duration::from_secs(1),
        "ending a guest that could not return took {waited:?}"
    );
    // And this one says *what* ended it: a cadence that had broken would let the gap budget answer
    // first, and the run would report a pack that failed to return rather than a person who left.
    assert!(
        finished
            .error
            .as_deref()
            .is_some_and(|e| e.contains("cancelled by request")),
        "the run says who ended it, in words that do not blame the pack: {:?}",
        finished.error
    );
    assert_eq!(
        shared.status().why,
        End::Cancelled,
        "and it is remembered as a cancel rather than as a run that came home"
    );
    // The same fact, carried out where the console report can read it. `Status::why` answers the
    // window's question ("is it over"), and until this field existed that was the only copy — the
    // front end had to hand the machine back with nothing but the error string, so a run somebody
    // ended was printed as a crash by `main`.
    assert_eq!(
        finished.why,
        End::Cancelled,
        "the reason travels with the machine, not only into the status the window reads"
    );
}

/// The other half of the same field: a run the guest killed itself is **not** one somebody ended.
///
/// The two arrive here in the same shape — an `Err` out of the run — and that is precisely why the
/// reason has to be carried rather than inferred. Closing a window and dying both stop the session; only
/// one of them is a decision, and a report that guessed wrong either way would be worse than the
/// parenthesis it replaced.
#[test]
fn a_run_the_guest_killed_is_not_reported_as_one_somebody_ended() {
    let mut rt = Runtime::new();
    rt.step_limit = 200 * crate::cpu::CANCEL_LOOK;
    rt.step_ceiling = u64::MAX;
    // A queued callback at address zero, which this layout deliberately leaves unmapped: the fetch
    // faults instead of running whatever happened to be there, so this is a guest death and not a
    // refusal.
    rt.queue.push_back(Job::Notify { pfn: 0, user: 0 });
    let guest = Guest::start(rt, Sound::Silent, None);
    let shared = Arc::clone(&guest.shared);
    let deadline = Instant::now() + Duration::from_secs(5);
    // Waited for rather than cancelled into: `finish` sets the cancel bit, and a cancel that arrives
    // first would end the run as somebody's ask and make this test assert the opposite of its name.
    while shared.status().why == End::Running {
        assert!(
            Instant::now() < deadline,
            "the guest never reached an ending of its own"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let finished = guest.finish();
    assert!(finished.error.is_some(), "a fault is reported as one");
    assert_eq!(
        finished.why,
        End::Finished,
        "and nothing claims a person ended it, because none did"
    );
}

/// The frame the window needs before the pack has drawn anything, and the machine that has to come
/// back for the report to exist.
///
/// This is the case the raw Win32 window used to serve by painting the booted framebuffer itself before
/// showing anything — which it could only do by holding the machine. Now the worker's first turn
/// publishes it, and a run that never presents a frame still leaves the window with a picture and the
/// caller with its `Runtime`.
#[test]
fn a_run_that_has_not_drawn_yet_still_hands_the_window_a_frame_and_gives_the_machine_back() {
    let guest = Guest::start(Runtime::new(), Sound::Silent, None);
    let shared = Arc::clone(&guest.shared);
    let deadline = Instant::now() + Duration::from_secs(5);
    let frame = loop {
        if let Some(frame) = shared.take_frame() {
            break frame;
        }
        assert!(
            Instant::now() < deadline,
            "the worker never published the frame the machine booted with"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(
        frame.presentations, 0,
        "nothing has been presented, so this is the screen as it came up"
    );
    // The format is pinned, not derived from itself: `pixels.len() == w * h * format.bytes_per_pixel()`
    // holds for every variant, so asserting only that would let a change of `Format` through with the
    // test still green — and the windows upload four bytes a pixel, which is why the worker asks for it.
    assert_eq!(
        frame.format,
        crate::screen::Format::Rgba8888,
        "`Guest::start` gives a window the layout both of them upload"
    );
    assert_eq!(
        frame.pixels.len(),
        shared.wide as usize * shared.high as usize * 4,
        "and it is one pixel per spot the device said it has, four bytes each"
    );
    let finished = guest.finish();
    assert_eq!(
        finished.error, None,
        "an idle run that was asked to stop is not a run that failed"
    );
    assert_eq!(
        finished.why,
        End::Cancelled,
        "and it was ended by the ask, which is what the report has to be able to say"
    );
    assert_eq!(
        finished.rt.display.presentations, 0,
        "the machine comes back with its counts, which is what the report reads"
    );
}

#[test]
fn a_session_takes_the_frame_format_its_consumer_declared() {
    // `Screen` has taken a format from whoever builds it since the embedded-port work; the worker was
    // the one place that still decided on the consumer's behalf. A front end that hands 5-6-5 to a
    // GPU which unpacks it in its texture unit should not pay for a conversion it throws away.
    let guest = Guest::start_with_backend(
        Runtime::new(),
        Sound::Silent,
        crate::audio::AudioBackend::Rust,
        None,
        crate::screen::Format::Rgb565,
    );
    let shared = Arc::clone(&guest.shared);
    let deadline = Instant::now() + Duration::from_secs(5);
    let frame = loop {
        if let Some(frame) = shared.take_frame() {
            break frame;
        }
        assert!(
            Instant::now() < deadline,
            "the worker never published the frame the machine booted with"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(
        frame.format,
        crate::screen::Format::Rgb565,
        "the format is the caller's, not the window's"
    );
    assert_eq!(
        frame.pixels.len(),
        shared.wide as usize * shared.high as usize * 2,
        "two bytes a pixel, one per spot the device said it has"
    );
    let _ = guest.finish();
}

#[test]
fn an_armed_call_tree_survives_the_handover_to_the_guest_thread() {
    // The switch belongs to the machine, and the machine moves to the worker. A `Shared` that started
    // this at `false` would have the worker write `false` back on its very first turn, disarming a run
    // the command line had armed — and the report trusts the flag, so it would print the sliver folded
    // before that turn as though it were the whole run.
    let mut rt = Runtime::new();
    rt.profile.set(true);
    let guest = Guest::start(rt, Sound::Silent, None);
    assert!(
        guest.shared.profiling(),
        "the switch the command line threw has to cross with the machine, or `--window --flame` \
         quietly reports a fraction of the run"
    );
    let finished = guest.finish();
    assert!(finished.rt.profile.on(), "and the machine comes back armed");
}

/// The inbox is one stream, so a frame that contains two taps delivers them as two taps.
///
/// This is the shape that used to be rewritten by the collection itself: presses and releases lived in
/// separate lists, so `tap A, tap B` inside one frame reached the guest as `press A, press B, release A,
/// release B` — a moment with both keys down that no finger produced. An app keeping a mask of the keys
/// *currently* down is reading precisely that.
#[test]
fn a_frame_of_two_taps_is_posted_in_the_order_it_was_collected() {
    let mut rt = Runtime::new();
    deliver(
        &mut rt,
        vec![
            Queued::Key {
                code: g::AVK_SOFT1,
                down: true,
            },
            Queued::Contact {
                event: g::EVT_POINTER_DOWN,
                x: 4,
                y: 6,
            },
            Queued::Key {
                code: g::AVK_SOFT1,
                down: false,
            },
            Queued::Key {
                code: g::AVK_SOFT2,
                down: true,
            },
        ],
    )
    .expect("the contact lands on a mapped address, so nothing here fails");
    let got: Vec<(u32, u32)> = rt
        .queue
        .iter()
        .map(|j| match j {
            Job::Event { evt, wp, .. } => (*evt, *wp),
            _ => (u32::MAX, u32::MAX),
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (g::EVT_KEY_PRESS, g::AVK_SOFT1),
            (g::EVT_KEY, g::AVK_SOFT1),
            // A pointer event's `wParam` is the length of the string it points at, `AEEEvent.h`.
            (g::EVT_POINTER_DOWN, 45),
            (g::EVT_KEY_PRESS, g::AVK_SOFT2),
            (g::EVT_KEY, g::AVK_SOFT2),
        ],
        "A is tapped through before B goes down, and the contact sits where it happened"
    );
    assert_eq!(
        rt.pending_release
            .iter()
            .map(|(code, _)| *code)
            .collect::<Vec<_>>(),
        vec![g::AVK_SOFT1],
        "and A's release is still owed, because its press had not been handed over yet"
    );
}

/// The screen as it came up travels to the window once, and a silent run is not re-screenshotted every
/// turn.
///
/// Zero presentations **is** published — that is the contract
/// [`a_run_that_has_not_drawn_yet_still_hands_the_window_a_frame_and_gives_the_machine_back`] pins (the
/// raw Win32 window used to paint the booted screen itself, and could only do it by holding the
/// machine). What that test does not pin, and what this one adds, is the other half: publishing it
/// **once**. The window is the one that says "the pack has not drawn yet", from the count the frame
/// carries.
#[test]
fn the_boot_screen_is_published_once_and_not_re_taken_until_a_presentation() {
    assert!(
        super::worth_publishing(0, None),
        "the worker's first turn owes the window the screen as it came up"
    );
    assert!(
        !super::worth_publishing(0, Some(0)),
        "and a run that goes on presenting nothing does not pay it every turn"
    );
    assert!(
        super::worth_publishing(1, Some(0)),
        "the first real presentation is a new frame"
    );
    assert!(
        !super::worth_publishing(2, Some(2)),
        "a still screen costs no screenshot"
    );
    assert!(super::worth_publishing(3, Some(2)), "and a moved one does");
}

#[test]
fn an_input_log_records_virtual_time_edges_and_hold_duration() {
    let path = std::env::temp_dir().join(format!("quoin-input-log-{}.tsv", std::process::id()));
    let mut log = super::InputRecorder::open(&path, 240, 320, "test")
        .expect("a temporary input log can be created");
    log.event(10, "key down", "AVK_UP", None, "");
    log.held.insert(0x1234, 10);
    log.event(160, "key up", "AVK_UP", Some(150), "");
    log.event(160, "pointer", "0x3", None, "x=1 y=2");
    drop(log);

    let text = std::fs::read_to_string(&path).expect("the log can be read back");
    assert!(text.contains("# device 240x320 test"), "{text}");
    assert!(text.contains("\tkey down\tAVK_UP\t\t"), "{text}");
    assert!(text.contains("\tkey up\tAVK_UP\t150\t"), "{text}");
    assert!(text.contains("\tpointer\t0x3\t\tx=1 y=2"), "{text}");
    let _ = std::fs::remove_file(path);

    assert_eq!(
        super::key_name(crate::keymap::avk("AVK_UP")),
        "AVK_UP",
        "a recorded key is named in the same vocabulary --key accepts"
    );
}

/// The wall clock decides *when* a turn runs, never how much simulated time it asks for.
///
/// `pace` gates on `FRAME` and always returns the one fixed `STEP` (also `FRAME`): a window hidden
/// for a minute resumes at one frame rather than replaying the absence. Advancing by `since` behind a
/// `MAX_STEP` cap was tried and reverted — it handed a slow turn a backlog and made the next turn
/// simulate a burst, so no such constant exists any more. There is deliberately **no** third case: a
/// boot phase can retire tens of millions of instructions with `clock_ms` still 0, so its silent
/// phase lives *inside one turn* where no inter-turn pacing can reach it — which is exactly why the
/// window now reads the worker's activity signals instead of inferring health from a frame gap.
/// `pace(ZERO) == None` is the assertion that keeps that bypass from coming back as "just a boot
/// exception".
#[test]
fn a_turn_asks_for_the_wall_clock_and_no_more() {
    use crate::session::SPEED_ONE;
    use std::time::Duration;
    assert_eq!(
        super::pace(Duration::ZERO, SPEED_ONE),
        None,
        "no simulated time before FRAME has passed — including on the very first turn of a boot"
    );
    assert_eq!(
        super::pace(Duration::from_millis(4), SPEED_ONE),
        None,
        "below FRAME"
    );
    assert_eq!(
        super::pace(Duration::from_millis(40), SPEED_ONE),
        Some(16),
        "a slow turn advances one frame, not the whole wall-clock delay"
    );
    assert_eq!(
        super::pace(Duration::from_secs(60), SPEED_ONE),
        Some(16),
        "a hidden window resumes at one frame; it does not replay the absence"
    );
}

/// The rate changes exactly two things: speeding up shortens "how long until a turn opens",
/// slowing down shortens "how many virtual milliseconds one turn covers". A single turn never
/// advances more than one frame, so timers never fire in a burst.
#[test]
fn speed_moves_the_gate_not_the_slice() {
    use crate::session::{SPEED_MAX, SPEED_MIN, SPEED_ONE};
    use std::time::Duration;
    assert_eq!(
        super::pace(Duration::from_millis(4), 2000),
        None,
        "at 2x, 4 ms is not enough to open a turn (8 ms gate)"
    );
    assert_eq!(
        super::pace(Duration::from_millis(8), 2000),
        Some(16),
        "at 2x, 8 ms of wall clock opens a turn and still advances one frame"
    );
    assert_eq!(
        super::pace(Duration::from_millis(4), SPEED_MAX),
        Some(16),
        "at 4x, 4 ms opens a turn and virtual time runs at 4x a frame"
    );
    assert_eq!(
        super::pace(Duration::from_millis(16), SPEED_MIN),
        Some(4),
        "at 0.25x, still one turn per frame, but a turn runs only 4 ms"
    );
    assert_eq!(
        super::pace(Duration::from_millis(16), 500),
        Some(8),
        "at 0.5x, 8 ms per turn"
    );
    assert_eq!(
        super::pace(Duration::from_millis(15), SPEED_ONE),
        None,
        "the 1x gate is unchanged"
    );
}

/// One budgeted JIT run has **exactly one exit, the instruction count; there is no wall-clock exit**.
///
/// `session::pace` only decides "when does a turn open" and "how many virtual milliseconds a turn
/// simulates"; once inside a turn, the loop in `Cpu::run_jit` (`src/cpu/jit.rs:367-533`) returns
/// `Budget` only when `steps >= limit`, and coming back into Rust every 65536 instructions serves
/// solely to synchronise state and sample the cancel bit — it never compares an `Instant`.
/// `Runtime::run_bounded` moreover restarts its allowance at every HLE trap
/// (`src/runtime/mod.rs:5114`). So `GAME_STEP_LIMIT` (32 M) / `GAME_STEP_CEILING` (1 G) become,
/// verbatim, the wall-clock time the worker is monopolised: at the interpreter's measured
/// 60–86 M instr/s that is 0.4 s to 17 s, while the window's `FPS --` threshold is only 250 ms
/// (`src/egui_window.rs:82`).
///
/// The program here is a hand-written `b .` and reads no sample set: what is asserted is the semantics
/// "the return happens exactly at the instruction budget", which depends neither on machine speed nor
/// on a guest game (no-guest-run rule).
#[cfg(feature = "jit")]
#[test]
fn jit_run_has_no_wall_clock_exit_only_the_instruction_budget_ends_it() {
    use crate::cpu::{Cpu, Stop, CANCEL_LOOK};
    use crate::mem::Mem;

    const CODE: u32 = 0x1000_0000;
    let mut c = Cpu::new(Mem::new());
    c.mem.map(CODE, 0x1000);
    c.mem.write_u32(CODE, 0xeaff_fffe).unwrap(); // b .
    c.pc = CODE;
    c.enable_jit();
    assert!(
        c.jit_enabled(),
        "this reproduction must really run on the JIT backend"
    );

    let budget = 8 * CANCEL_LOOK;
    let before = c.steps;
    let stop = c.run(budget);
    assert!(
        matches!(stop, Stop::Budget { .. }),
        "the spin loop retires the full budget; wall clock did not stop it early: {stop:?}"
    );
    assert_eq!(
        c.steps - before,
        budget,
        "the backend returns exactly at the instruction limit, not at some time slice"
    );
}

/// One reproduction of the three symptoms a user sees, and of what the window **can** read once the
/// L1 observation layer is fixed.
///
/// The chain is fixed: at the top of a turn the worker does `turns += 1`, but `frame` is published
/// only when `worth_publishing` says so, and the `status` snapshot is written in full at the end of
/// the turn. While stuck inside `rt.pump`:
///   - `take_frame()` stays `None` → 250 ms later there is no frame rate to report;
///   - `why` is still `Running` → the window does not close and the picture stays on the last frame;
///   - `turns` / `clock_ms` / `emulator` are still 0;
///   - **but** `turn_active` / `turn_started_at` are published at the start of the turn, and
///     `guest_steps()` keeps advancing through the CPU's `CANCEL_LOOK` checkpoints, so the window can
///     tell "this frame is being computed" from "stopped" instead of reporting the former as a frame
///     rate collapse.
///
/// Without the `jit` feature this test runs the interpreter (same root cause, not backend-specific);
/// with it, `enable_jit()` puts Rynarmic in the path — both builds pin the same causality.
/// `Sound::Silent` is deliberate: bringing the sound card up costs 120–170 ms and would pollute the
/// observation window.
#[test]
fn a_turn_that_never_returns_starves_frames_but_publishes_activity() {
    let mut rt = Runtime::new();
    // The budget magnitude of a played run: the probe's small default allowance would declare the
    // function dead before the observation window ends.
    rt.step_limit = u64::MAX / 4;
    rt.step_ceiling = u64::MAX;
    rt.session_ceiling = u64::MAX;
    let loop_at = stuck(&mut rt);
    rt.queue.push_back(Job::Notify {
        pfn: loop_at,
        user: 0,
    });
    #[cfg(feature = "jit")]
    rt.cpu.enable_jit();

    let guest = Guest::start(rt, Sound::Silent, None);
    let shared = Arc::clone(&guest.shared);

    // Past the badge's 250 ms threshold is enough to reproduce "no current frame rate". The window
    // ends when *the turn* is that old, not after a fixed wait: the turn starts on the worker, so a
    // fixed wait can end before the worker has started it, which is how this test used to fail on a
    // loaded machine rather than on a broken one. The cap only stops a genuine stall from hanging.
    let cap = Instant::now() + Duration::from_secs(2);
    let mut frames = 0;
    let mut first_steps = None;
    let mut last_steps = 0;
    loop {
        if shared.take_frame().is_some() {
            frames += 1;
        }
        let steps = shared.guest_steps();
        if first_steps.is_none() {
            first_steps = Some(steps);
        }
        last_steps = steps;
        let old_enough = shared
            .status()
            .turn_started_at
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(250));
        if old_enough || Instant::now() >= cap {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        frames, 0,
        "the worker published no frame in the whole observation window: even the presentations=0 startup frame never reaches the publish point"
    );
    assert!(
        last_steps > first_steps.unwrap_or(0),
        "guest instruction count keeps advancing within this turn: {first_steps:?} -> {last_steps};\
         this is how the window tells computing from stalled"
    );

    let status = shared.status();
    assert_eq!(
        status.why,
        End::Running,
        "the window is told 'still running', so it neither closes nor raises an error"
    );
    assert!(
        status.turn_active,
        "the worker publishes 'inside a turn' at the start of the turn, not after pump returns"
    );
    assert!(
        status.turn_started_at.is_some(),
        "and includes this turn's wall-clock start so the window can report how long it has been computing"
    );
    assert!(
        status
            .turn_started_at
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(250)),
        "this turn has run past the badge's 250 ms threshold"
    );
    assert_eq!(
        status.turns, 0,
        "the turn counter is still written only at the end; reads during a long turn still see 0"
    );
    assert_eq!(
        status.clock_ms, 0,
        "the virtual clock likewise advances only after pump returns"
    );
    assert_eq!(
        status.emulator,
        Duration::ZERO,
        "the QUOIN_STATS simulate line still holds the previous turn's value"
    );

    let finished = guest.finish();
    assert!(
        !shared.status().turn_active,
        "cleanup must clear 'inside a turn', on the error exit path too"
    );
    assert_eq!(
        shared.status().turns,
        1,
        "the turn number is filled in at cleanup, which is exactly why it is not a liveness signal while running"
    );
    assert_eq!(shared.status().why, End::Cancelled, "ended by cancel");
    assert!(
        finished
            .error
            .as_deref()
            .is_some_and(|e| e.contains("cancelled by request")),
        "the error wording does not blame the pack for a stall: {:?}",
        finished.error
    );
}

/// The acceptance contract for L2 (wall-clock slicing of guest execution), currently red and kept
/// behind `#[ignore]` until that lands.
///
/// The two tests above are characterisation: they record what happens now. This one states what must
/// hold after the fix: even when the guest spins inside one function, the worker has to hand control
/// back once within a frame budget and publish liveness, and `turns` has to keep growing, so the
/// window does not misread "computing" as "frame rate collapsed".
#[cfg(feature = "jit")]
#[test]
#[ignore = "needs wall-clock time slicing of guest execution (L2); red by design"]
fn a_spinning_guest_still_publishes_liveness_within_one_frame_budget() {
    let mut rt = Runtime::new();
    rt.step_limit = u64::MAX / 4;
    rt.step_ceiling = u64::MAX;
    rt.session_ceiling = u64::MAX;
    let loop_at = stuck(&mut rt);
    rt.queue.push_back(Job::Notify {
        pfn: loop_at,
        user: 0,
    });
    rt.cpu.enable_jit();

    let guest = Guest::start(rt, Sound::Silent, None);
    let shared = Arc::clone(&guest.shared);
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        if shared.status().turns >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        shared.status().turns >= 2,
        "within one frame budget the worker must finish and publish at least two liveness turns, or FPS drops"
    );
    let _ = guest.finish();
}

/// The code-cache observation rule is a pure function: it needs no JIT, no window and no guest game.
/// What this chain has to pin down is "one flush is recorded, and only for a whole-cache clear";
/// otherwise ordinary slot collisions read as faults, while the one event that really recompiles
/// every hot block gets swallowed by `saturating_sub`.
#[test]
fn a_block_count_drop_is_read_as_exactly_one_code_cache_flush() {
    use crate::session::{observe_jit_blocks, JitView};

    let view = |blocks, compiled, invalidations| JitView {
        blocks,
        compiled,
        invalidations,
    };

    assert_eq!(
        observe_jit_blocks(None, 100, 0),
        view(100, 0, 0),
        "the first sample has no history, so it claims neither growth nor invalidation"
    );
    assert_eq!(
        observe_jit_blocks(Some(100), 137, 0),
        view(137, 37, 0),
        "37 blocks were compiled this turn, which is the cold-compilation amount"
    );
    assert_eq!(
        observe_jit_blocks(Some(4096), 12, 2),
        view(12, 12, 3),
        "dropping from 4096 to 12 is a full-table flush: one flush is counted, and those 12 blocks were recompiled this turn"
    );
    assert_eq!(
        observe_jit_blocks(Some(100), 92, 3),
        view(92, 0, 3),
        "a small drop within the margin is a slot collision, not a failure; saturating subtraction must not yield a negative compile count"
    );
    assert_eq!(
        observe_jit_blocks(Some(100), 84, 0),
        view(84, 0, 0),
        "exactly the margin is not a flush"
    );
    assert_eq!(
        observe_jit_blocks(Some(1), 0, 0),
        view(0, 0, 1),
        "with only one or two blocks cached, a flush to zero must still be visible and not swallowed by the margin"
    );
    assert_eq!(
        observe_jit_blocks(Some(100), 83, 0),
        view(83, 83, 1),
        "one block fewer crosses the boundary: spotting a flush one frame earlier is what makes it diagnostically useful"
    );
    let after = observe_jit_blocks(Some(4096), 12, 2);
    assert_eq!(
        observe_jit_blocks(Some(12), 30, after.invalidations),
        view(30, 18, 3),
        "the hot path keeps compiling after a flush, and the cumulative invalidation count never goes backwards"
    );
}

/// The sampling chain end to end (only in a JIT build): read the block count from the CPU, decide,
/// update the running totals. Hand-written ARM words, no window and no guest game. This test is what
/// keeps the HUD's `flushes` from being an ornament that always reads 0.
#[cfg(feature = "jit")]
#[test]
fn jit_sampling_sees_compilation_and_a_full_flush() {
    use crate::cpu::{Cpu, Stop};
    use crate::mem::Mem;
    use crate::session::sample_jit_blocks;

    const CODE: u32 = 0x1000_0000;
    let mut c = Cpu::new(Mem::new());
    c.mem.map(CODE, 0x1000);
    for (i, w) in [0xe280_0001u32, 0xeaff_fffe].iter().enumerate() {
        c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
    }
    c.pc = CODE;
    c.r[13] = 0x1000_2800;

    let mut prev: Option<usize> = None;
    let mut invalidations: u32 = 0;
    assert_eq!(
        sample_jit_blocks(c.jit_cached_blocks(), &mut prev, &mut invalidations),
        None,
        "with no backend enabled there is no reading, so the HUD must not show this line"
    );
    c.enable_jit();
    let first = sample_jit_blocks(c.jit_cached_blocks(), &mut prev, &mut invalidations)
        .expect("the backend must report a reading once enabled");
    assert_eq!(first.blocks, 0, "the cache is empty right after enabling");

    assert!(matches!(c.run(64), Stop::Budget { .. }));
    let grown = sample_jit_blocks(c.jit_cached_blocks(), &mut prev, &mut invalidations).unwrap();
    assert!(
        grown.blocks > 0 && grown.compiled > 0,
        "after running hand-written code the per-turn compile count must be visible: {grown:?}"
    );

    // Lowering the limit clears the whole table: the same path Rynarmic takes when its own arena
    // fills.
    c.set_jit_arena_limit(2);
    let flushed = sample_jit_blocks(c.jit_cached_blocks(), &mut prev, &mut invalidations).unwrap();
    assert_eq!(
        flushed.invalidations, 1,
        "a full-table flush must be recorded as one flush"
    );
    assert_eq!(flushed.blocks, 0, "the block count is zero after a flush");
}

/// Every constraint of `--align-clock`: the default is still a fixed frame; a slow turn is paid back
/// across turns rather than in one burst; the steady state charges POLL's rounding error to the turn
/// that caused it; a hidden window's absence is not replayed.
#[test]
fn alignment_pays_a_stall_back_in_bounded_turns_not_one_giant_pump() {
    let base =
        pace(Duration::from_millis(410), SPEED_ONE).expect("a slow turn still opens one round");
    assert_eq!(
        base, 16,
        "pace is still a fixed frame and ignores wall clock entirely"
    );
    assert_eq!(
        pace(Duration::from_secs(60), SPEED_ONE),
        Some(16),
        "a minute hidden does not replay - the alignment option cannot change that either"
    );

    let (extra, debt) = catch_up(Duration::from_millis(410), 0);
    assert_eq!(
        (extra, debt),
        (8, 386),
        "410 ms of real time owes 394 ms; this turn pays back only 8"
    );
    assert_eq!(
        base + extra,
        24,
        "at most 1.5 frames per turn: never hand the whole 410 ms to pump (this is the shape that got ad66b61 reverted)"
    );

    let mut debt = debt;
    let mut paid = extra;
    let mut turns = 1;
    while debt > 0 {
        let (e, next) = catch_up(Duration::from_millis(16), debt);
        assert!(
            base + e <= 16 + ALIGN_CATCHUP_MS,
            "a single turn must still stay within bounds during amortisation"
        );
        paid += e;
        debt = next;
        turns += 1;
        assert!(
            turns < 200,
            "the debt must converge, not grow with every payment"
        );
    }
    assert_eq!(
        paid, 394,
        "a 410 ms stall owes 394 ms, recovered exactly in the end"
    );
    assert_eq!(
        turns, 50,
        "about fifty turns, under a second, rather than one burst"
    );
}

#[test]
fn alignment_follows_the_clock_but_refuses_a_hidden_window() {
    assert_eq!(
        catch_up(Duration::from_millis(16), 0),
        (0, 0),
        "steady state is frame for frame and creates no debt"
    );
    assert_eq!(
        catch_up(Duration::from_millis(17), 0),
        (1, 0),
        "the extra 1 ms from POLL rounding is repaid in the same turn; the 6% steady-state lag is aligned away"
    );
    assert_eq!(
        catch_up(Duration::from_millis(8), 0),
        (0, 0),
        "an interval shorter than a frame creates no debt"
    );
    assert_eq!(
        catch_up(Duration::from_secs(60), 999),
        (0, 0),
        "a one-minute window suspension does not replay absence, and old debt is cleared too"
    );
    assert_eq!(
        catch_up(ALIGN_RESUME_GAP + Duration::from_millis(1), 999),
        (0, 0),
        "just past the suspension threshold also clears the debt"
    );
    assert_eq!(
        catch_up(ALIGN_RESUME_GAP, 999),
        (ALIGN_CATCHUP_MS, ALIGN_DEBT_CAP_MS - ALIGN_CATCHUP_MS),
        "within the threshold the debt cap still applies, so it never accumulates into unlimited fast-forward"
    );
    assert_eq!(
        catch_up(Duration::from_millis(16), ALIGN_DEBT_CAP_MS),
        (ALIGN_CATCHUP_MS, ALIGN_DEBT_CAP_MS - ALIGN_CATCHUP_MS),
        "at the cap, each turn repays only the capped amount"
    );
}

#[test]
fn a_default_status_does_not_claim_a_clock_rate() {
    assert_eq!(
        Status::default().clock,
        None,
        "deterministic stepping by default: QUOIN_STATS shows no clock line, and the HUD matches the old text verbatim"
    );
}
