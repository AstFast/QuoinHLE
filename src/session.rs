//! The session: one thread runs the guest, another draws.
//!
//! Before this file existed, the window's repaint callback called [`Runtime::pump`] itself
//! (`egui_window.rs`, `window.rs`, `manager.rs`). That is one thread doing both jobs, and it made a
//! pack that spins inside one guest function unkillable: the ceilings that end such a call are
//! measured in instructions (`GAME_STEP_LIMIT`, `GAME_STEP_CEILING`), which at the interpreter's
//! measured 60-86 M instr/s is twelve to seventeen seconds of a window that answers to nothing — and a
//! loop that keeps reaching the host resets the gap bound every time, so it has no upper end at all.
//! Esc, the close box and `--serve`'s `quit` were all read *between* pumps, so none of them could be
//! read while the pump was the thing that was stuck.
//!
//! So the machine has one owner — this module's worker thread — and the window never drives it. The one
//! thing the UI thread does with the runtime is read `device.width`/`device.height` to size its own
//! window, before handing the machine over (`window.rs`, `manager.rs`). What crosses the boundary is
//! four short-lock things: a frame the worker published, an inbox of keys and contacts, a status the
//! HUD reads, and one atomic bit that says "end this". The locks are held for a
//! memcpy or a `Vec::append` and never across a call into the guest, which is the whole reason the
//! window can still answer while the guest cannot.
//!
//! The worker owns the [`Speaker`] too, and not by accident: a `cpal::Stream` does not move between
//! threads, and the mixer is reached through a channel precisely so that no lock is taken on the
//! realtime path.

use crate::audio::{AudioBackend, Playing, Speaker};
use crate::control::Control;
use crate::profile::{Flame, Options, Weight, Who};
use crate::runtime::{Dispatched, Runtime};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The frame interval the guest is advanced on, in wall-clock milliseconds.
///
/// This is the number the window used to ask egui for (`request_repaint_after`), and keeping it here
/// rather than pumping as fast as a loop can is deliberate: a pump turn advances the virtual clock by
/// the milliseconds it is handed, so a finer turn would reschedule every timer and every held-key
/// repeat in the machine. How fast a pack is played is not this module's to change as a side effect of
/// moving threads — and not to change at all as a favour either: see `pace`.
pub const FRAME: Duration = Duration::from_millis(16);

/// How deep the rows the developer page draws may go.
///
/// A window has the height for about this many bands once the keypad and the plate are in it, and the
/// tree gets deeper than this the moment a pack calls a host method that calls back into it. Rows past
/// the cut are counted and not drawn (`Flame::elided`), which is the difference between a picture that
/// is short and one that lies about being complete.
const FLAME_DEPTH: usize = 10;

/// How long the worker sleeps before looking at its bit and its inbox again.
///
/// This buys cancel latency and key latency, not throughput: the guest still only advances on `FRAME`
/// boundaries, but a press lands within a frame and `cancel` is obeyed between turns rather than at the
/// end of a run nobody can interrupt.
const POLL: Duration = Duration::from_millis(2);

/// One turn's simulated time. The worker always advances by one frame, never by the whole wall-clock
/// delay: a CPU-bound turn that takes longer than a frame should cost the next frames, not make the
/// following turn simulate a large burst at once.
const STEP: Duration = FRAME;

/// How much simulated time this turn should ask for, or `None` when it is too early to ask at all.
///
/// The wall clock gates *when* a turn may run; it does not decide how much one turn may simulate.
/// Advancing by `since` (even capped) makes a slow turn hand the next turn a backlog, which is what
/// turns a 300 ms CPU stall into one giant `Simulate` reading and a visibly frozen window. A fixed
/// frame-sized step keeps the turn latency bounded and lets the window publish between turns; the
/// simulation runs slower when the host cannot keep up, rather than replaying the backlog in bursts.
///
/// This bounds **inter-turn** pacing only. A single guest call can still hold one turn far longer
/// than `FRAME`: `GAME_STEP_LIMIT` / `GAME_STEP_CEILING` count instructions, not wall time, and the
/// CPU's checkpoints do not compare a clock. So a window must not infer a run's health from the gap
/// between two presentations — it reads the turn activity in [`Status`] and the CPU's progress
/// channel instead. See [`crate::egui_window::fps_badge`].
/// How much simulated time this turn should ask for, or `None` when it is too early to ask at all.
///
/// `speed` is thousandths of real time (`SPEED_ONE` = 1x). Above 1x the **gate** shrinks and the
/// step stays one frame, so the guest is visited more often rather than handed a bigger slice;
/// below 1x the gate stays one frame and the step shrinks. Both keep a single turn at or under 16
/// virtual milliseconds, which is what stops a timer from being fired in bulk — the burst that made
/// the wall-clock step (`ad66b61`) get reverted. The wall clock still only decides *when* a turn may
/// run; the rate decides how much it simulates.
fn pace(since: Duration, speed: u32) -> Option<u32> {
    let frame = STEP.as_millis() as u32;
    let speed = speed.clamp(SPEED_MIN, SPEED_MAX);
    let (gate, step) = if speed >= SPEED_ONE {
        (frame.saturating_mul(SPEED_ONE) / speed, frame)
    } else {
        (frame, (frame * speed / SPEED_ONE).max(1))
    };
    (since >= Duration::from_millis(u64::from(gate.max(1)))).then_some(step)
}

/// Thousandths of real time: `1000` is 1x.
pub const SPEED_ONE: u32 = 1000;

/// The slowest and fastest clock rates the CLI and the audio mixer agree on.
///
/// 0.25x keeps a turn at 4 ms of simulated time; 4x is the fastest a 2 ms poll can feed
/// (`speed * POLL <= 16 ms`), so a larger number would only make the loop wait on its own sleep.
pub const SPEED_MIN: u32 = 250;
pub const SPEED_MAX: u32 = 4000;

/// How much of the debt `--align-clock` may pay back in one turn, in virtual milliseconds.
///
/// A frame is 16 ms, so 8 ms more is at most a 1.5x turn. That is the whole difference between
/// catching up and `ad66b61`: handing one slow turn's entire 400 ms to the *next* `pump` bursts every
/// timer due in that span at once and freezes the window again, which is why that change was reverted.
/// Paid over ~50 turns instead, a 400 ms stall is caught up in well under a second of ordinary frames.
pub(crate) const ALIGN_CATCHUP_MS: u32 = 8;

/// The most debt `--align-clock` will carry. A host that can never keep up must not accumulate an
/// unbounded obligation and then fast-forward for minutes when it briefly recovers.
pub(crate) const ALIGN_DEBT_CAP_MS: u32 = 1_000;

/// A real-time gap longer than this is a suspended window, not a slow turn. The original `pace`
/// deliberately does not replay a hidden window's absence (`pace(60s) == Some(16)`), and alignment
/// keeps that rule. Five seconds still covers the longest JIT cold-compile stall measured on this
/// host (the old arena limit's ~2.7 s warm pass), so a stall is paid back while a lunch break is not.
pub(crate) const ALIGN_RESUME_GAP: Duration = Duration::from_secs(5);

/// Whether the guest's virtual clock should follow the wall clock.
///
/// An entry-level switch, read once per run: `--align-clock` sets `QUOIN_ALIGN_CLOCK` (the same
/// `std::env::set_var` bridge `--record-input` uses, so the window and the game library share one
/// implementation), and the game library can be started with that variable directly. **Off by
/// default**: with it off the sequence of `pump` calls is a fixed 16 ms, so a scripted run replays
/// byte for byte. With it on, the sequence depends on how fast this host ran, and it does not.
/// The clock rate this run was started with, in thousandths of real time, or `None` for the default
/// fixed 16 ms step.
///
/// `--speed 2` sets `QUOIN_SPEED=2000`; `--align-clock` is the same thing at `1000`, kept because it
/// is the spelling for "pay a stall back" that already reference. Read once per run: the rate is a
/// property of the run, not something a turn may change.
pub fn clock_speed() -> Option<u32> {
    static WANT: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *WANT.get_or_init(|| {
        std::env::var("QUOIN_SPEED")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .map(|p| p.clamp(SPEED_MIN, SPEED_MAX))
            .or_else(|| {
                std::env::var_os("QUOIN_ALIGN_CLOCK")
                    .is_some()
                    .then_some(SPEED_ONE)
            })
    })
}

/// Whether the run is following the wall clock at real time — also the only rate where the stall
/// debt is paid back (`--align-clock` / `--speed 1`).
pub fn clock_alignment() -> bool {
    clock_speed() == Some(SPEED_ONE)
}

/// Pay back real time that passed without being simulated, a little at a time.
///
/// `since` is measured from the start of the previous turn, so it contains that turn's whole host
/// cost: a 410 ms cold-compile turn owes 394 ms. Returns `(extra virtual ms for this turn, debt left)`.
/// Only [`pace`]'s fixed frame is ever enlarged, by at most [`ALIGN_CATCHUP_MS`], so one `pump` never
/// has to deliver hundreds of milliseconds of timers at once.
pub(crate) fn catch_up(since: Duration, debt: u32) -> (u32, u32) {
    if since > ALIGN_RESUME_GAP {
        // A suspended window resumes at one frame and owes nothing. Clearing the debt here is what
        // keeps "the window was hidden for a minute" from becoming two seconds of fast-forward.
        return (0, 0);
    }
    // `since <= 5 s` above keeps this cast far inside u32.
    let elapsed = since.as_millis() as u32;
    let owed = elapsed.saturating_sub(STEP.as_millis() as u32);
    let debt = debt.saturating_add(owed).min(ALIGN_DEBT_CAP_MS);
    let extra = debt.min(ALIGN_CATCHUP_MS);
    (extra, debt - extra)
}

/// Virtual milliseconds this run advanced per real second, divided by 1000.
///
/// A rate above 1x is a request, not a promise: a guest whose frame costs more than the frame
/// interval cannot be advanced faster, and this is the number that says so.
fn measured_speed(clock_ms: u32, base_clock: u32, base_wall: Instant) -> f32 {
    let elapsed = base_wall.elapsed().as_secs_f32().max(1.0e-3);
    (clock_ms.wrapping_sub(base_clock) as f32) / elapsed / 1000.0
}

/// Whether this `presentations` count is a frame worth taking a screenshot for.
///
/// Zero **is** worth publishing, and that is a decision with a test behind it rather than an oversight:
/// `session::tests::a_run_that_has_not_drawn_yet_still_hands_the_window_a_frame_and_gives_the_machine_back`
/// pins that the worker's first turn hands the window the screen as it came up — the raw Win32 window
/// used to paint that itself, and could only do it by holding the machine. So the boot framebuffer
/// travels, and what says "the pack has not drawn yet" is the window, which has the count in the frame
/// (`Frame::presentations`) and writes the line over it. See [`crate::egui_window`].
fn worth_publishing(presentations: u32, painted: Option<u32>) -> bool {
    Some(presentations) != painted
}

/// Why the worker stopped feeding the guest.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Still running.
    #[default]
    Running,
    /// The guest came back on its own, or faulted and the error was recorded.
    Finished,
    /// Somebody set [`Shared::cancel`] — the close box, Esc, a harness.
    Cancelled,
    /// `--serve`'s `quit` was served.
    Quit,
}

/// A frame the guest has presented, as the pixels the window will hand to the GPU.
///
/// Deliberately not an `egui::ColorImage` and not uploaded here: the drawing context belongs to the
/// thread that owns the window, so this is the one piece of work the worker cannot do for it. The bytes
/// come from [`crate::screen::Screen`], which is where the pixel path stays whichever thread reads it,
/// and [`Frame::format`] says how to read them.
pub struct Frame {
    /// The `Display::presentations` these bytes belong to. The window uploads only when this differs
    /// from what it uploaded last, so a still screen costs nothing — the rule the single-threaded
    /// window already ran on.
    pub presentations: u32,
    /// The screen's size, so a frame is self-describing: a consumer on another thread has no
    /// `Runtime` to ask, and a texture sized from a stale constant is a wrong picture rather than a
    /// missing one.
    pub width: u32,
    pub height: u32,
    /// The picture, row-major, `width * format.bytes_per_pixel()` bytes a row — the whole screen.
    ///
    /// Named `pixels` rather than `rgb` because it is not necessarily `RGB` any more: the format is
    /// the one [`Frame::format`] names, and it is chosen once, when the worker builds its `Screen`.
    /// The consumer chose it when it started the session ([`Guest::start_with_backend`]), which is
    /// why it is carried rather than assumed: the two windows ask for `Rgba8888` because that is the
    /// surface they upload into, and a front end whose GPU unpacks 5-6-5 asks for `Rgb565` and saves
    /// the conversion entirely.
    pub pixels: Vec<u8>,
    /// How those bytes are spelled, from [`crate::screen::Screen::format`].
    ///
    /// Carried rather than assumed: a frame crosses a thread boundary and the reader has no `Runtime`
    /// to ask, which is the same reason `width` and `height` are here. A consumer that cannot take the
    /// format is expected to say so — [`crate::screen::expand565`] exists for the one case that is
    /// cheap to serve, a 5-6-5 screen bound for a texture that wants four bytes a pixel.
    pub format: crate::screen::Format,
    /// The row ranges (`y0..y1`, `y1` exclusive) that moved for this frame, from
    /// [`crate::screen::Screen::bands`]. A consumer that mirrors the buffer into a texture can upload
    /// only these; one that does not may ignore them. Empty is legal and means the guest presented
    /// the same picture again.
    pub bands: Vec<(u32, u32)>,
}

/// One diagnostic reading of the JIT code cache, published at the end of a turn.
///
/// This is an **inference**, not a counter Rynarmic hands us: a full `invalidate_code_cache()`
/// (whether the host asked for it because the windows changed, or Rynarmic did because its arena
/// filled) drops the cached block count from thousands to almost nothing. Watching for that drop is
/// what lets a person say "the freeze I just saw was a code-cache flush and a hot-path recompile",
/// which is the one thing `simulate`, `steps` and `turn` cannot distinguish on their own.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct JitView {
    /// Blocks with machine code right now. Deliberately not "blocks decoded": a block held back by
    /// the per-turn compile cap is decoded and cached but interpreted, so counting it would make a
    /// capped turn look like it compiled thousands of blocks.
    pub blocks: u32,
    /// Blocks compiled during this sampling interval. After a flush that is the whole rebuilt cache,
    /// not zero — the recompile is exactly what made the turn slow.
    pub compiled: u32,
    /// How many full flushes have been inferred so far.
    pub invalidations: u32,
}

/// A drop smaller than this is ordinary cache-slot churn, not a flush.
const JIT_DROP_MARGIN: usize = 16;

/// Fold two consecutive block counts into a reading. Pure so the inference can be asserted without
/// a machine, a window, or a JIT build. The first sample has no history and declares neither growth
/// nor a flush.
pub(crate) fn observe_jit_blocks(prev: Option<usize>, now: usize, invalidations: u32) -> JitView {
    // Two shapes of flush: a large cache collapses to almost nothing, and a tiny cache goes to zero
    // (the latter is what a short synthetic run — or a run that just flushed last turn — looks like).
    let dropped = prev.is_some_and(|p| (now == 0 && p > 0) || now + JIT_DROP_MARGIN < p);
    let compiled = match prev {
        // After a flush every block now in the cache was compiled during this interval. Reporting
        // zero here would hide the very recompile that made the turn slow.
        Some(_) if dropped => now,
        Some(p) => now.saturating_sub(p),
        None => 0,
    };
    JitView {
        blocks: now as u32,
        compiled: compiled as u32,
        invalidations: invalidations + u32::from(dropped),
    }
}

/// Take one diagnostic sample of the code cache. Kept separate from the worker so the whole chain —
/// drop inference, bookkeeping — can be asserted without a window or a running session.
///
/// `blocks` is the reading itself ([`Runtime::jit_cached_blocks`]), not the machine it came from:
/// this way the function needs no `Cpu`, and a caller that already asked the machine cannot forget
/// which backend it was asking about. `None` — no translating backend — is answered `None`, and the
/// previous sample is left untouched.
pub(crate) fn sample_jit_blocks(
    blocks: Option<usize>,
    prev: &mut Option<usize>,
    invalidations: &mut u32,
) -> Option<JitView> {
    let now = blocks?;
    let view = observe_jit_blocks(*prev, now, *invalidations);
    *prev = Some(now);
    *invalidations = view.invalidations;
    Some(view)
}

/// Whether to sample the JIT cache at all.
///
/// Counting cached blocks scans 65536 slots, so it is off by default and rides on the existing
/// `QUOIN_STATS` switch — the same one that draws the HUD line and enables `/status`'s `jit_blocks`.
/// Read once, like the window's own switch.
///
/// Lives here rather than in `egui_window`, where it began: it is one question about one
/// environment variable, and it is asked from the session, from `control`, and from the HUD. Having
/// the session boundary call *into* the UI crate for it was the one edge that made `eframe`
/// impossible to leave out of a build — see the `ui` feature in `Cargo.toml`.
pub(crate) fn stats_wanted() -> bool {
    static WANT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *WANT.get_or_init(|| std::env::var_os("QUOIN_STATS").is_some())
}

fn jit_stats_wanted() -> bool {
    stats_wanted()
}

/// What the clock is doing, for the HUD.
///
/// `None` in [`Status`] means the default fixed step, which has no rate to report and is still the
/// behaviour a scripted run relies on. When this is present the run was started with `--speed`
/// (or its `--align-clock` spelling).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockView {
    /// Thousandths of real time the run was started with (`1000` = 1x).
    pub target: u32,
    /// Measured virtual milliseconds per real second, divided by 1000. Host throughput is the
    /// ceiling, so this is the number to read — not `target`.
    pub actual: f32,
    /// Stall debt still owed, in virtual milliseconds. Non-zero only at 1x.
    pub debt: u32,
}

/// What the window can say about a run it cannot touch.
#[derive(Default, Clone)]
pub struct Status {
    /// Everything the run has delivered, folded across its turns.
    pub pumped: Dispatched,
    /// Where the last frame's simulated time went, for the `QUOIN_STATS` line.
    pub emulator: Duration,
    /// The virtual clock, so the HUD can say how far a stuck run got.
    pub clock_ms: u32,
    /// How many turns of the guest the worker has actually run.
    ///
    /// `clock_ms` alone cannot tell forty turns of 100 ms from two hundred and fifty of 16 ms, and the
    /// pacing contract in `pace` is stated **per turn** — so this is the number that says whether a
    /// run is being paced at all, which is what makes it readable from outside the thread. The
    /// `QUOIN_STATS` line shows it as "turn T" beside the window's own frame count, because a picture
    /// that updates slowly and a guest that is being advanced slowly are different complaints.
    pub turns: u32,
    /// Whether the worker is inside a turn right now, and the wall-clock moment that turn began.
    ///
    /// These are published at the **start** of a turn, unlike everything above, which is written when
    /// the turn ends. A guest that spends a second inside one function stops all of those numbers from
    /// moving; without these two the window cannot tell "the guest is computing this frame" from "the
    /// run has stopped", and reports a healthy machine as a frame-rate collapse. See
    /// [`crate::egui_window`]'s badge.
    pub turn_active: bool,
    pub turn_started_at: Option<Instant>,
    pub why: End,
    /// The error that ended the run, as the report will print it. `None` while it is still going.
    pub ended: Option<String>,
    /// JIT code-cache diagnostics. `None` means "not watching": no JIT backend, or no diagnostic
    /// switch. A jump in `invalidations` right after a long turn is the code-cache flush that made
    /// the host recompile its hot path inside one frame.
    pub jit: Option<JitView>,
    /// How many fast-memory windows the backend has published, out of `MAX_WINDOWS`. `None` when
    /// there is no backend or no diagnostic switch.
    ///
    /// A guest address space is cut into spans at 1 MiB boundaries, so an image, a heap of a few
    /// MiB and the stack easily exceed the window budget — and the region that loses out is
    /// whichever is shortest, usually the stack. Every access there then leaves the generated
    /// code and calls back into Rust (measured at 1.88 ns per instruction against 0.79 inline),
    /// which is a throttle nobody can see from the frame rate. This number is how it becomes
    /// visible.
    pub jit_windows: Option<u8>,
    /// `--speed`'s rate, the measured rate, and any unpaid stall debt, or `None` on the default
    /// fixed step. `QUOIN_STATS` shows it so a person can see whether the option took effect.
    pub clock: Option<ClockView>,
}

/// One input event the window collected and the guest has not been told about yet.
#[derive(Debug, Clone, Copy)]
pub enum Queued {
    /// A key going down (`down = true`) or up.
    Key { code: u32, down: bool },
    /// A pointer contact, as [`Runtime::post_pointer`] takes it: `(event, x, y)`.
    Contact { event: u32, x: i32, y: i32 },
}

/// What the window gathered between two turns of the guest.
///
/// **One list, in the order the window saw things**, because two lists — presses and releases — quietly
/// rewrite history: a frame that contains tap A then tap B arrives as `press A, press B, release A,
/// release B`, which is a moment when two keys are down that never were together. The physical keyboard
/// path can keep the real order (its events come to it in order); the drawn keypad cannot, because it is
/// a comparison of two states rather than a stream, and says so where it builds its entries.
#[derive(Default)]
struct Inbox {
    events: Vec<Queued>,
}

/// The machine, parked, once the run is over.
///
/// The `Runtime` comes back rather than being dropped, because the report is read out of it — the call
/// log, the audio census, the heap reached. Closing a window is not a run whose findings are gone.
pub struct Finished {
    pub rt: Runtime,
    pub pumped: Dispatched,
    pub error: Option<String>,
    /// What ended the run, which is **not** the same question as whether it reported an error.
    ///
    /// A windowed run is ended by the person closing it far more often than by a fault, and the two
    /// used to arrive here indistinguishable: `error` is `Some` either way when the ask landed while
    /// a turn was still executing, so the report dressed a deliberate close up as a crash — the one
    /// thing the person reading it already knew was not true. Carrying the reason lets the report say
    /// it. `End::Finished` is the only value that means "the guest or a fault ended this"; it is also
    /// what a front end writes when the blame is its own (a window that would not open).
    pub why: End,
}

/// What both threads may see. Everything here is held for the length of a copy, never across a guest
/// call.
/// What the network page draws: the socket rows, and the request backend's own count -- a script run
/// has no sockets to show, and "0 connections" would be the wrong answer to "did anything go out".
#[derive(Debug, Clone, Default)]
pub struct NetView {
    pub sockets: crate::socket::Report,
    pub requests: u64,
}

pub struct Shared {
    frame: Mutex<Option<Frame>>,
    inbox: Mutex<Inbox>,
    status: Mutex<Status>,
    /// The size the module was told it has, and the device's own name. Copied out of the machine
    /// before it moved: a window sizes its handset against these, and after [`Guest::start`] there is no
    /// `Runtime` left on this thread to ask.
    pub wide: u32,
    pub high: u32,
    /// `DeviceProfile::model_name`, for the plate under the keypad.
    pub model: String,
    /// Set by the window, read by the worker — and inside the interpreter too, which is the part that
    /// made this worth building: a pack spinning in one function would otherwise never look at it.
    cancel: Arc<AtomicBool>,
    /// Retired guest instructions, published by the CPU's own checkpoints rather than by this loop.
    ///
    /// The worker can only write [`Status`] between turns, and a single guest call can hold the
    /// machine for far longer than a frame. This is the one number that keeps moving during that
    /// call, so a window can tell a guest that is still computing from one that has stalled. A
    /// relaxed load on the UI thread; one relaxed store per [`crate::cpu::CANCEL_LOOK`] instructions
    /// on the guest thread, and only in a windowed run.
    progress: Arc<AtomicU64>,
    /// The decoder backend the worker's speaker should use for clips it has not decoded yet.
    ///
    /// It lives here rather than only in the window because the speaker belongs to the guest thread;
    /// an `AtomicU8` lets a window change the choice for subsequent clips without touching the
    /// machine or the audio callback.
    audio_backend: Arc<AtomicU8>,
    /// Whether the window wants the run's call tree. Written by the window (the developer page's
    /// switch), read by the worker once a turn.
    ///
    /// It has to cross threads rather than live in the window because the tree is folded *inside* the
    /// machine — the dispatches and downcalls it counts happen on the guest thread — and the machine
    /// has exactly one owner ([`Guest`]).
    profile_on: AtomicBool,
    /// A one-shot "throw away what you have folded", taken by the worker.
    profile_reset: AtomicBool,
    /// The tree as rows, published by the worker for the window to draw. A value rather than a borrow
    /// for the same reason [`Frame`] is: the window may not touch the machine.
    ///
    /// Rebuilt only while the switch is on, and only once a frame. A reader arriving between two
    /// publishings sees the previous one rather than nothing, which is what keeps a caller from having
    /// to interpret an empty picture as "the run did nothing".
    flame: Mutex<Option<Flame>>,
    /// What the guest's sockets and requests are doing, published once a turn for the network page. A
    /// value for the same reason the flame is: the window may not touch the machine.
    network: Mutex<Option<NetView>>,
    /// Where the window wants guest sockets to connect. `None` is the loopback model, which is what a
    /// run that was pointed nowhere must keep doing; `Some` is the address the window typed, applied to
    /// the machine once a turn the same way the flame switch is.
    net_target: Mutex<Option<std::net::SocketAddr>>,
    /// A one-shot "zero the counters", taken by the worker.
    net_clear: std::sync::atomic::AtomicBool,
    /// Filled by the worker before it returns, taken by whoever owns the run. A lock rather than a
    /// channel so a reader arriving early simply finds `None` and does not have to interpret a closed
    /// sender.
    finished: Mutex<Option<Finished>>,
    /// Optional turn-by-turn record of the keys and contacts a person actually made.
    ///
    /// It lives beside the inbox because that is the one door both front ends use; recording in a
    /// front end would miss whichever one is not drawing the current handset.
    input_log: Mutex<Option<InputRecorder>>,
}

/// A human replay record, written as events arrive rather than at the end: a run that is killed
/// still leaves the keys up to that point.
struct InputRecorder {
    file: File,
    started: Instant,
    /// The down time of each key still held, keyed by its `AVK_*` code.
    held: BTreeMap<u32, u32>,
}

impl InputRecorder {
    fn open(path: &PathBuf, wide: u32, high: u32, model: &str) -> std::io::Result<Self> {
        let mut file = File::create(path)?;
        // The first two columns are both clocks: the virtual one is what a replay can reproduce,
        // and the wall one is what the person actually felt. A `--wait`/`--hold` replay should use
        // the virtual column; the wall column is there to diagnose pacing.
        writeln!(file, "# quoin input log v1")?;
        writeln!(file, "# device {wide}x{high} {model}")?;
        writeln!(file, "# virtual_ms\twall_ms\tevent\tkey\theld_ms\tdetail")?;
        Ok(Self {
            file,
            started: Instant::now(),
            held: BTreeMap::new(),
        })
    }

    fn event(
        &mut self,
        virtual_ms: u32,
        event: &str,
        key: &str,
        held_ms: Option<u32>,
        detail: &str,
    ) {
        let wall_ms = self.started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
        let _ = writeln!(
            self.file,
            "{virtual_ms}\t{wall_ms}\t{event}\t{key}\t{}\t{detail}",
            held_ms.map(|h| h.to_string()).unwrap_or_default()
        );
        // One line per event is small; flushing here is what makes a crash mid-session leave a
        // usable tail instead of a buffered beginning.
        let _ = self.file.flush();
    }
}

fn input_recorder_from_env(wide: u32, high: u32, model: &str) -> Option<InputRecorder> {
    let path = std::env::var_os("QUOIN_INPUT_LOG")?;
    let path = PathBuf::from(path);
    match InputRecorder::open(&path, wide, high, model) {
        Ok(recorder) => {
            eprintln!("input log: {}", path.display());
            Some(recorder)
        }
        Err(e) => {
            eprintln!("input log {}: {e}", path.display());
            None
        }
    }
}

fn key_name(code: u32) -> String {
    crate::keymap::KEYS
        .iter()
        .find(|key| key.avk() == code)
        .map(|key| key.name.to_string())
        .unwrap_or_else(|| format!("{code:#010x}"))
}

impl Shared {
    /// The window's half of a frame: take what the worker last presented, leaving nothing until the
    /// next presentation.
    pub fn take_frame(&self) -> Option<Frame> {
        self.frame.lock().expect("nothing poisons this lock").take()
    }

    /// How many instructions the guest has retired, read without waiting on the machine.
    ///
    /// Unlike every field of [`Status`], this one moves *inside* a guest function. A window compares
    /// two samples to tell a long-but-productive frame from a stall; it never reads a lock and can
    /// never be blocked by the guest.
    pub fn guest_steps(&self) -> u64 {
        self.progress.load(Ordering::Relaxed)
    }

    /// A copy of the run's state, taken briefly. Copied rather than borrowed so no caller draws while
    /// holding this lock — a window that paints from inside a lock is a window that can be blocked by
    /// the thing it is guarding.
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .expect("nothing poisons this lock")
            .clone()
    }

    pub fn press(&self, code: u32) {
        self.record_key(code, true);
        self.inbox
            .lock()
            .expect("nothing poisons this lock")
            .events
            .push(Queued::Key { code, down: true });
    }

    pub fn release(&self, code: u32) {
        self.record_key(code, false);
        self.inbox
            .lock()
            .expect("nothing poisons this lock")
            .events
            .push(Queued::Key { code, down: false });
    }

    pub fn contact(&self, event: u32, x: i32, y: i32) {
        self.record_contact(event, x, y);
        self.inbox
            .lock()
            .expect("nothing poisons this lock")
            .events
            .push(Queued::Contact { event, x, y });
    }

    /// Write one key edge to the optional input log. The virtual clock is the replay clock; the
    /// wall clock in the file tells a person how the run was actually paced.
    fn record_key(&self, code: u32, down: bool) {
        let now = self
            .status
            .lock()
            .expect("nothing poisons this lock")
            .clock_ms;
        let Ok(mut slot) = self.input_log.lock() else {
            return;
        };
        let Some(log) = slot.as_mut() else {
            return;
        };
        let name = key_name(code);
        if down {
            log.event(now, "key down", &name, None, "");
            log.held.insert(code, now);
        } else {
            let held = log.held.remove(&code).map(|at| now.saturating_sub(at));
            log.event(now, "key up", &name, held, "");
        }
    }

    fn record_contact(&self, event: u32, x: i32, y: i32) {
        let now = self
            .status
            .lock()
            .expect("nothing poisons this lock")
            .clock_ms;
        let Ok(mut slot) = self.input_log.lock() else {
            return;
        };
        if let Some(log) = slot.as_mut() {
            log.event(
                now,
                "pointer",
                &format!("{event:#x}"),
                None,
                &format!("x={x} y={y}"),
            );
        }
    }

    /// Ask the run to end. Returns at once: the worker notices within one [`POLL`], and a guest spinning
    /// inside one function stops within [`crate::cpu::CANCEL_LOOK`] instructions of it.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// The decoder backend currently selected for this run.
    pub fn audio_backend(&self) -> AudioBackend {
        AudioBackend::from_u8(self.audio_backend.load(Ordering::Relaxed))
    }

    /// Change the decoder backend for subsequent clips. The speaker reads this on its next play;
    /// clips already decoded keep playing.
    pub fn set_audio_backend(&self, backend: AudioBackend) {
        self.audio_backend.store(backend as u8, Ordering::Relaxed);
    }

    pub(crate) fn audio_backend_handle(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.audio_backend)
    }

    /// Ask for the run's call tree, or stop asking. Takes effect on the worker's next turn — the
    /// switch is read where the machine is, so a window that flips it does not fold anything itself.
    pub fn want_profile(&self, on: bool) {
        self.profile_on.store(on, Ordering::Relaxed);
    }

    pub fn profiling(&self) -> bool {
        self.profile_on.load(Ordering::Relaxed)
    }

    /// Throw away what has been folded so far, on the worker's next turn. One shot: a reader that
    /// asked once gets one reset, rather than a run that is cleared every frame because the wound-up
    /// flag was never taken back.
    pub fn reset_profile(&self) {
        self.profile_reset.store(true, Ordering::Relaxed);
    }

    /// Where the window wants guest sockets to go, or `None` for the loopback model.
    pub fn net_target(&self) -> Option<std::net::SocketAddr> {
        *self.net_target.lock().expect("nothing poisons this lock")
    }

    pub fn set_net_target(&self, target: Option<std::net::SocketAddr>) {
        *self.net_target.lock().expect("nothing poisons this lock") = target;
    }

    pub fn request_net_clear(&self) {
        self.net_clear
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// The window's clear request, if it made one since this was last asked.
    pub fn take_net_clear(&self) -> bool {
        self.net_clear
            .swap(false, std::sync::atomic::Ordering::Relaxed)
    }

    /// The last socket report the worker published, or `None` if it has published none yet.
    pub fn network(&self) -> Option<NetView> {
        self.network
            .lock()
            .expect("nothing poisons this lock")
            .clone()
    }

    /// The last tree the worker published, or `None` if it has published none yet.
    pub fn flame(&self) -> Option<Flame> {
        self.flame
            .lock()
            .expect("nothing poisons this lock")
            .clone()
    }
}

/// Whether this run is to make sound, and what it makes sound out of.
///
/// Three cases and not two, because the question "should this run open the machine's audio device?"
/// is not answered by "which tone bank should it use?". A window says yes — that is what a window
/// with a person in front of it is for. A test of how the guest thread hands back a stuck pack says
/// no: opening the audio device to prove a point about threads makes the test's timing belong to
/// whoever else is using the box, and a unit test that fails on a machine with no sound card is not a
/// unit test.
pub enum Sound {
    /// No speaker is built and no device is opened. The run still answers every `ISoundPlayer` call.
    Silent,
    /// The tables this crate generates ([`crate::audio::soundbank`]).
    BuiltIn,
    /// A SoundFont file the player named.
    File(PathBuf),
}

impl Sound {
    /// What a window should speak with, given the flag a person may or may not have set. One place
    /// for that choice, because three front ends make it and a run whose sound depends on which
    /// window it was opened from is a run nobody can reproduce.
    pub fn named(path: Option<&str>) -> Sound {
        match path {
            Some(path) => Sound::File(path.into()),
            None => Sound::BuiltIn,
        }
    }
}

/// The owner of the guest thread.
pub struct Guest {
    pub shared: Arc<Shared>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Guest {
    /// Take the machine and run it on its own thread.
    ///
    /// The speaker is built *there*, not here, because the stream it opens cannot be moved; the
    /// failure to open one is still said once, on the worker, in the same words the window used to
    /// print. It is also the reason [`Sound`] exists: this is the point where a run decides whether to
    /// touch the machine's audio device at all.
    pub fn start(rt: Runtime, sound: Sound, control: Option<Control>) -> Guest {
        Self::start_with_backend(
            rt,
            sound,
            AudioBackend::Rust,
            control,
            crate::screen::Format::Rgba8888,
        )
    }

    /// Take the machine and run it with an explicit decoder backend and an explicit frame format.
    ///
    /// `format` is the consumer's declaration, the same one [`crate::screen::Screen`] already asks of
    /// whoever builds it: a window that uploads a 32-bit surface wants `Rgba8888`, a front end that
    /// hands the picture to a GPU that unpacks 5-6-5 wants `Rgb565` and saves the whole conversion.
    /// Naming it here is what keeps this module from being "the egui window's session".
    pub fn start_with_backend(
        rt: Runtime,
        sound: Sound,
        backend: AudioBackend,
        control: Option<Control>,
        format: crate::screen::Format,
    ) -> Guest {
        // Read out before the machine moves: nothing on this thread reaches a `Runtime` again.
        let (wide, high, model) = (
            u32::from(rt.device.width),
            u32::from(rt.device.height),
            rt.device.model_name.clone(),
        );
        // The call-tree switch is read out here for the same reason, and it is *not* a formality: the
        // worker compares `Shared::profile_on` against the machine's own switch every turn and writes
        // the machine's copy when they differ, so starting this at `false` would silently disarm a run
        // the command line had armed (`--window --flame`) — and the report at the end, which trusts the
        // flag rather than this field, would then print the sliver that was folded before the worker's
        // first turn as though it were the whole run.
        let armed = rt.profile.on();
        let cancel = Arc::new(AtomicBool::new(false));
        // Opened before the struct is built, because `model` moves into it below and this is the last
        // place that still has it by reference.
        let input_log = input_recorder_from_env(wide, high, &model);
        let shared = Arc::new(Shared {
            frame: Mutex::new(None),
            inbox: Mutex::new(Inbox::default()),
            status: Mutex::new(Status::default()),
            wide,
            high,
            model,
            cancel: Arc::clone(&cancel),
            progress: Arc::new(AtomicU64::new(0)),
            audio_backend: Arc::new(AtomicU8::new(backend as u8)),
            profile_on: AtomicBool::new(armed),
            profile_reset: AtomicBool::new(false),
            flame: Mutex::new(None),
            network: Mutex::new(None),
            net_target: Mutex::new(None),
            net_clear: std::sync::atomic::AtomicBool::new(false),
            finished: Mutex::new(None),
            input_log: Mutex::new(input_log),
        });
        let worker_shared = Arc::clone(&shared);
        let join = std::thread::Builder::new()
            // Named, because this is the thread a frozen picture gets blamed on, and a stack sample
            // that says `guest` answers the question such a sample usually leaves open.
            .name("guest".to_string())
            .spawn(move || run(rt, sound, control, cancel, worker_shared, format))
            .expect("one thread per run is not a resource limit worth handling");
        Guest {
            shared,
            join: Some(join),
        }
    }

    /// Ask the run to end, wait for the thread to park the machine, and take it back.
    pub fn finish(mut self) -> Finished {
        self.shared.cancel();
        if let Some(join) = self.join.take() {
            join.join()
                .expect("the guest thread reports its failures as a value; it does not panic");
        }
        self.shared
            .finished
            .lock()
            .expect("nothing poisons this lock")
            .take()
            .expect("the worker parks the machine before it returns")
    }
}

impl Drop for Guest {
    /// A dropped owner asks the run to stop; it cannot wait, because `Drop` has no caller to explain a
    /// block to. The machine is then parked in a `Shared` nobody holds and goes with it, which loses
    /// the report rather than the run — every path that wants the report calls [`Guest::finish`].
    fn drop(&mut self) {
        self.shared.cancel();
    }
}

/// The turn the guest is run in.
///
/// A free function so the machine, the speaker and the report stay with one owner on one thread:
/// `spawn` moves them into the closure, and none of them is ever reached from the window again.
fn run(
    mut rt: Runtime,
    sound: Sound,
    mut control: Option<Control>,
    cancel: Arc<AtomicBool>,
    shared: Arc<Shared>,
    format: crate::screen::Format,
) {
    // The clock rate is a property of the run: read once, before the first turn, so the audio thread
    // and the pacing loop can never disagree about it.
    let speed = clock_speed();
    let align = speed == Some(SPEED_ONE);
    let target_speed = speed.unwrap_or(SPEED_ONE);
    // Measured rate: virtual milliseconds advanced per real second since the first turn. `target`
    // is what was asked for; this is what the host actually delivered, and it is the number the HUD
    // shows because throughput is the ceiling on any rate above 1x.
    let wall_base = Instant::now();
    let clock_base = rt.clock_ms;
    // The interpreter looks at this same flag inside a guest function, which is the only place a pack
    // that never returns can be caught.
    rt.arm_cancel(Arc::clone(&cancel));
    // The same checkpoints also publish how far the guest has got. Without this, a turn that holds
    // the machine for a second leaves the window with a frozen picture and no number that moves —
    // which is what made a healthy long frame look like a frame-rate collapse.
    rt.arm_progress(Arc::clone(&shared.progress));
    let backend_handle = shared.audio_backend_handle();
    // Measured on this box: opening the device and building the eight synthesizers costs 120 to 170 ms
    // the first time and about the same after, and dropping the stream costs 5 to 110 ms. That is why
    // this happens before the first turn rather than at the first play: a game's own first sound would
    // otherwise stall its simulation for the same time, with the picture frozen on it.
    let opened = match &sound {
        Sound::Silent => None,
        Sound::BuiltIn => Some(Speaker::new_with_backend_handle(
            None,
            Arc::clone(&backend_handle),
        )),
        Sound::File(path) => Some(Speaker::new_with_backend_handle(
            Some(path),
            Arc::clone(&backend_handle),
        )),
    };
    let mut speaker = match opened {
        None => None,
        Some(Err(e)) => {
            eprintln!("no sound: {e}");
            None
        }
        Some(Ok(built)) => {
            eprintln!("sound: {}", built.source());
            for note in built.notes() {
                eprintln!("sound: {note}");
            }
            Some(built)
        }
    };
    // The audio rate is a property of the run like the clock rate: set once, before the first
    // turn, so a clip that starts on the first frame already plays at the right tape speed.
    if let (Some(speaker), Some(permille)) = (speaker.as_mut(), speed) {
        speaker.set_speed(permille);
    }
    let mut started = Playing::default();
    let mut pumped = Dispatched::default();
    // The frame last handed to a window, and `None` while the pack has not presented one yet — which is
    // a state the window is allowed to say out loud, not one to paint over. See [`worth_publishing`].
    let mut painted: Option<u32> = None;
    // The screen, kept across turns so a still row is not converted twice: `Screen::refresh` reads a
    // row, compares it with what that row held last time, and converts only the ones that moved. It
    // used to be a fresh `Display::screenshot` every presentation — 215 µs for a 240x320 screen, of
    // which the reading is 7.5 µs and the rest was the conversion of rows that had not changed.
    //
    // The format is the caller's declaration. `Rgba8888` is what both windows ask for because they
    // end at a 32-bit surface: `egui` fills a `Vec<Color32>` and the Win32 path fills a `BITMAPINFO`
    // of `[b, g, r, a]`. Producing three-byte pixels for them would mean either converting again on
    // the thread that owns the display — the two-pass shape this module's `Screen` exists to remove.
    // Measured on a 240x320 screen, publishing one frame: 160 µs here against 170, the copy 12.9 µs
    // against 8.8, and the window's own pass 16 µs against 75. So this thread ends 5.9 µs ahead
    // (a 10 µs faster unpack, a 4.1 µs bigger copy) and the window's own pass is 59 µs shorter —
    // 65 µs in all, on two threads that are not both this one.
    let mut screen = crate::screen::Screen::with_format(format);
    // Counted here rather than derived, because the two things a reader can see from outside — the
    // virtual clock and this number — are what separates a run being advanced in 100 ms turns from one
    // being advanced in 16 ms turns. See [`Status::turns`].
    let mut turns: u32 = 0;
    // Code-cache sampling is diagnostic-only: when the switch is off these three words are the whole
    // cost, and the 65536-slot scan never happens. See `observe_jit_blocks`.
    let jit_wanted = jit_stats_wanted();
    let mut jit_prev: Option<usize> = None;
    let mut jit_invalidations: u32 = 0;
    // Read once: an option is a property of the run, not something a turn may switch mid-flight.
    // When it is off these two words are the whole cost and the `pump` sequence stays the fixed
    // 16 ms that makes a scripted run reproducible.
    let mut align_debt: u32 = 0;
    // The last moment the guest was advanced. `Instant` rather than a count, because the question this
    // answers — how much real time the run is being asked to simulate — is a wall-clock one. `pace`
    // keeps that answer to one frame, so a window hidden for a minute does not replay the minute;
    // `--align-clock` is the explicit opt-in to paying a *stall* back, a little per turn.
    let mut last = Instant::now();
    let mut why = End::Finished;
    let mut error: Option<String> = None;

    loop {
        if cancel.load(Ordering::Relaxed) {
            why = End::Cancelled;
            break;
        }
        let since = last.elapsed();
        if let Some(base) = pace(since, target_speed) {
            turns += 1;
            last = Instant::now();
            let began = Instant::now();
            // The fixed frame is the default and stays untouched. Alignment only ever *adds* a
            // bounded catch-up to it; it never replaces it with the whole elapsed wall time, which
            // is the burst that made the old wall-clock step get reverted.
            let ms = if align {
                let (extra, next) = catch_up(since, align_debt);
                align_debt = next;
                base.saturating_add(extra)
            } else {
                base
            };
            // Published *before* the machine is entered, not after it returns: this is the one
            // observable that says a long turn is in progress rather than finished or dead. One short
            // lock for two words, never held across a call into the guest.
            {
                let mut status = shared.status.lock().expect("nothing poisons this lock");
                status.turn_active = true;
                status.turn_started_at = Some(began);
            }
            // The turn itself is a frame of the tree, and it is the one frame that is the host's own
            // work rather than the pack's: what is left of it after its calls are subtracted is the
            // emulator's overhead — the screenshot, the mixer's follow, the bookkeeping. Without it
            // the tree would have no row for "the host itself", which is half of what `--flame` is
            // read for.
            let steps = rt.steps();
            rt.profile.enter(Who::Span("frame"), steps);
            let turn = rt.pump(ms);
            let steps = rt.steps();
            rt.profile.exit(steps);
            match turn {
                Ok(done) => pumped.accumulate(done),
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
            let emulator = began.elapsed();
            // The control interface is served here, between two turns of the guest, because it writes
            // the machine: on the window thread it would be a second owner of a `Runtime`, which is
            // exactly the thing this module is built to avoid.
            let asked = match control.as_mut() {
                Some(control) => control.poll(&mut rt),
                None => crate::control::Asked::default(),
            };
            if asked.quit {
                why = End::Quit;
                break;
            }
            if asked.advance_ms > 0 {
                match rt.pump(asked.advance_ms) {
                    Ok(done) => {
                        let paid = done.ms;
                        pumped.accumulate(done);
                        // `/advance` moved the virtual clock by hand. Alignment must not also count
                        // that span as time the host owes: it was paid by the caller, not lost to a
                        // stall. Without this, `--window --serve` would chase the same milliseconds
                        // twice and run the guest fast.
                        align_debt = align_debt.saturating_sub(paid);
                    }
                    Err(e) => {
                        error = Some(e.to_string());
                        break;
                    }
                }
            }
            // Sound follows the same edge the guest does: this turn is the one that ran whatever code
            // asked for a clip, and `follow` acts on a `Play` once rather than once per look.
            if let Some(speaker) = speaker.as_mut() {
                let sound_at = rt.clock_ms;
                let asked = speaker.follow(&rt, &mut started);
                // Behind the same switch as the mixer's own trace. These lines used to print
                // unconditionally, which in a window is a log with no reader and no end: a pack
                // that plays a sound per frame grows stdout by the second. `QUOIN_AUDIO_TRACE` is
                // the development switch, and `/log?kind=music` on the control port is the
                // supported way to ask the same question of a running game.
                if crate::audio::tracing() {
                    for request in asked {
                        match request {
                            crate::audio::Request::Play { index, .. } => {
                                let bytes = rt.audio.get(index).map_or(0, |s| s.bytes.len());
                                println!(
                                    "sound: @{sound_at}ms clip {} ({} bytes)",
                                    index + 1,
                                    bytes
                                );
                            }
                            crate::audio::Request::Stop { voice } => {
                                println!("sound: @{sound_at}ms stop voice {voice:#010x}");
                            }
                            crate::audio::Request::Volume { voice, volume } => {
                                println!(
                                    "sound: @{sound_at}ms volume {volume} on voice {voice:#010x}"
                                );
                            }
                            crate::audio::Request::Tempo(percent) => {
                                println!("sound: @{sound_at}ms tempo {percent}%");
                            }
                            crate::audio::Request::Rate { voice, num, den } => {
                                println!(
                                    "sound: @{sound_at}ms rate {num}/{den} on voice {voice:#010x}"
                                );
                            }
                            crate::audio::Request::Tune { voice, step } => {
                                println!(
                                    "sound: @{sound_at}ms tune {step:+} semitones on voice {voice:#010x}"
                                );
                            }
                            crate::audio::Request::Tone {
                                object,
                                hi,
                                lo,
                                volume,
                            } => {
                                println!(
                                    "sound: @{sound_at}ms tone {hi}+{lo} Hz at {volume} on {object:#010x}"
                                );
                            }
                            crate::audio::Request::Midi { voice, bytes } => {
                                println!(
                                    "sound: @{sound_at}ms midi {} bytes on voice {voice:#010x}",
                                    bytes.len()
                                );
                            }
                            crate::audio::Request::Window {
                                voice,
                                start,
                                stop,
                                reverse,
                            } => {
                                println!(
                                    "sound: @{sound_at}ms window {start}..{stop} ms ({}) on voice {voice:#010x}",
                                    if reverse { "reverse" } else { "forward" }
                                );
                            }
                        }
                    }
                }
            }
            if worth_publishing(rt.presentations(), painted) {
                painted = Some(rt.presentations());
                // Reading the screen out of guest memory is the host's own copy of pixels — the one
                // piece of the frame the window cannot do for it — so it gets a row of its own rather
                // than hiding inside the frame's.
                let steps = rt.steps();
                rt.profile.enter(Who::Span("screenshot"), steps);
                let shot = rt.capture(&mut screen);
                let steps = rt.steps();
                rt.profile.exit(steps);
                if let Err(e) = shot {
                    error = Some(e.to_string());
                    break;
                }
                let (width, height) = screen.size();
                // A copy, because the window is on another thread and this buffer is about to be
                // rewritten in place. It is the one thing the embedded case does not pay: a UI that
                // owns the machine borrows `Screen::pixels` instead.
                *shared.frame.lock().expect("nothing poisons this lock") = Some(Frame {
                    presentations: rt.presentations(),
                    width,
                    height,
                    pixels: screen.pixels().to_vec(),
                    format: screen.format(),
                    bands: screen.bands().to_vec(),
                });
            }
            // The call tree, while the developer page's switch is on. The switch and the one-shot
            // reset are read here, on the thread that owns the machine, and the rows are built here
            // too — the window may not borrow a `Tree` that another thread is folding into.
            let wanted = shared.profile_on.load(Ordering::Relaxed);
            if shared.profile_reset.swap(false, Ordering::Relaxed) {
                rt.profile.clear();
            }
            if rt.profile.on() != wanted {
                rt.profile.set(wanted);
            }
            if wanted {
                let handle_event = rt
                    .guest_slot(rt.applet, "IApplet", "HandleEvent")
                    .unwrap_or(u32::MAX);
                let flame = rt.profile.tree().flame(
                    handle_event,
                    Options {
                        weight: Weight::Nanos,
                        max_depth: FLAME_DEPTH,
                        min_share: 0.0,
                    },
                );
                *shared.flame.lock().expect("nothing poisons this lock") = Some(flame);
            }
            // The window's policy, applied to the machine it may not touch: a target means real
            // connections through it, no target means the loopback model the run started with.
            let target = shared.net_target();
            let wanted = crate::socket::Policy {
                real: target.is_some(),
                proxy: target,
            };
            if rt.host_sockets.policy() != wanted {
                rt.host_sockets.set_policy(wanted);
            }
            if shared.take_net_clear() {
                rt.host_sockets.clear_traffic(rt.clock_ms as u64);
            }
            *shared.network.lock().expect("nothing poisons this lock") = Some(NetView {
                sockets: rt.host_sockets.snapshot(),
                requests: rt.net.as_ref().map(|net| net.asked()).unwrap_or(0),
            });
            // Sampled outside the status lock: counting cached blocks walks Rynarmic's whole slot
            // table, so holding a lock across it would let a diagnostic slow the thread it watches.
            let jit_view = if jit_wanted {
                sample_jit_blocks(
                    rt.jit_cached_blocks(),
                    &mut jit_prev,
                    &mut jit_invalidations,
                )
            } else {
                None
            };
            let mut status = shared.status.lock().expect("nothing poisons this lock");
            status.jit = jit_view;
            status.jit_windows = if jit_wanted {
                rt.jit_windows().map(|n| n as u8)
            } else {
                None
            };
            status.pumped = pumped;
            status.emulator = emulator;
            status.clock_ms = rt.clock_ms;
            status.turns = turns;
            status.clock = speed.map(|target| ClockView {
                target,
                actual: measured_speed(rt.clock_ms, clock_base, wall_base),
                debt: align_debt,
            });
            status.turn_active = false;
        }
        // Keys and contacts are posted *after* the turn they were collected in, which is what the
        // single-threaded window did: a press takes effect on the next visit to the guest. Packs that
        // read key timing — a menu that wants seconds between presses — are reading that order, not a
        // about threads.
        let inbox = {
            let mut held = shared.inbox.lock().expect("nothing poisons this lock");
            std::mem::take(&mut *held)
        };
        // One pass in the order the window collected them. Sorting presses before releases would put
        // two keys down at a moment no finger made them both down, and a pack that keeps a mask of the
        // keys currently down reads exactly that.
        if let Err(e) = deliver(&mut rt, inbox.events) {
            error = Some(e);
        }
        std::thread::sleep(POLL);
    }

    // Park the machine with its answer, whatever ended the loop, before this thread returns: that is
    // what lets [`Guest::finish`] wait on the join rather than on a lock the guest might still hold.
    //
    // A windowed run has no console of its own, so the one-line reason also goes to stderr. A person
    // who started the game from a terminal should see why it stopped without opening the manager's
    // report; a cancel is not an error and stays silent.
    if !cancel.load(Ordering::Relaxed) {
        if let Some(e) = &error {
            eprintln!("run ended: {e}");
        }
    }
    // The ask is read first and the error second. A cancelled run almost always ends *as* an error —
    // the interpreter stops the guest function, which travels up as `Error::Cancelled` — and a reader
    // that then saw `End::Finished` would be told the pack came back on its own. It did not: somebody
    // ended it, and `why` exists to say which of the two happened.
    //
    // Computed once and used twice, because the window and the report have to agree: `Status::why` is
    // what the front end reads to decide it is over, `Finished::why` is what the report says, and two
    // `if`s that could drift apart would let a window close itself for one reason while the console
    // named another.
    let reason = if cancel.load(Ordering::Relaxed) {
        End::Cancelled
    } else if matches!(why, End::Quit) {
        End::Quit
    } else {
        End::Finished
    };
    {
        let mut status = shared.status.lock().expect("nothing poisons this lock");
        status.pumped = pumped;
        // Written again here, because a turn that ended on an error never reached the per-turn write
        // above and its turn count would otherwise be one short of the machine it parks.
        status.turns = turns;
        // A turn that broke on an error never reached its per-turn close either. Leaving this set
        // would tell the window the machine is still computing after the run has ended.
        status.turn_active = false;
        status.clock = speed.map(|target| ClockView {
            target,
            actual: measured_speed(rt.clock_ms, clock_base, wall_base),
            debt: align_debt,
        });
        status.why = reason;
        status.ended = error.clone();
    }
    *shared.finished.lock().expect("nothing poisons this lock") = Some(Finished {
        rt,
        pumped,
        error,
        why: reason,
    });
}

/// Hand the guest what the window collected, in order, through the same doors the scripted route uses.
///
/// A function of its own because "in order" is the claim, and a claim about order belongs next to the
/// code that has it: the loop used to be three `for`s over three lists, which is where the ordering went
/// missing without anybody deciding it.
fn deliver(rt: &mut Runtime, events: Vec<Queued>) -> Result<(), String> {
    for q in events {
        match q {
            Queued::Key { code, down: true } => rt.post_key_press(code),
            Queued::Key { code, down: false } => rt.post_key_release(code),
            Queued::Contact { event, x, y } => {
                if let Err(e) = rt.post_pointer(event, x, y) {
                    return Err(e.to_string());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "test/session.rs"]
mod tests;
