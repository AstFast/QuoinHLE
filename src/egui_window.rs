//! The screen in a window, drawn as the handset it belongs to.
//!
//! This is **one game's** picture and keypad. [`crate::manager`] is the other half of the front end —
//! the list of packs, the size picker and the report pane — and it draws this into a corner of its own
//! window, which is why [`Frontend::view`] takes a `ui` and a scale instead of owning a window.
//! [`crate::window`] is the same job against the raw operating-system window, and it stays because it
//! needs no dependency and no GPU. Both keep the first two rules below and differ on the third —
//! resizing the raw window stretches the picture by whatever factor the client rect works out to
//! (`SetStretchBltMode`/`StretchBlt` at `src/window.rs:292-305`; its *initial* size is the integer one,
//! `src/window.rs:335`). Those rules are why this file is short:
//!
//! - The emulator runs on [`crate::session`]'s guest thread. This thread never drives it — no
//!   `Runtime` method is called from here — but it does hold the machine long enough to read
//!   `device.width`/`device.height` and to hand it to [`crate::session::Guest::start`], which is what
//!   sizes the window. It used to run here, inside the repaint callback, which made a pack that spins
//!   inside one guest function unkillable — see the header of [`crate::session`]. What the
//!   single-thread arrangement was really protecting still holds: this thread never drives the machine,
//!   and the guest advances on the worker's own schedule — `FRAME` boundaries
//!   (`crate::session::FRAME`), and nothing else (`session::pace` has no bypass) — so a run's result does
//!   not depend on how fast this window happens to repaint.
//! - Keys leave through [`Runtime::post_key_press`] and [`Runtime::post_key_release`], the same door
//!   a scripted `--key` uses, so an applet that answers a key by arming a timer behaves here exactly
//!   as it does in a recorded session.
//! - The pixels are the device bitmap as the worker's own [`crate::display::Display::screenshot`] read
//!   it (`crate::session` lifts it into a `Frame`); this thread only scales it, by an integer with
//!   nearest-neighbour filtering. Nothing here redraws, recolours or resamples the game's picture.
//!   "Integer" is in egui **points**: at a display scale of 125 % the multiple of physical pixels is
//!   not a whole number either, and nothing here pretends otherwise. The pack's own icon is the next
//!   piece of pixel art a person sees, and it is drawn in `crate::manager` under the same rule — that
//!   is where the multiple it is drawn at is worked out.
//!
//! The phone around that picture is laid out by [`crate::handset`], which says where each key sits in
//! *device pixels*; this file multiplies those by one integer and paints what it is told. That split
//! is what keeps a wide pack and a narrow one from needing two front ends, and it is why [`place`]
//! is the only function here that knows what a scale factor is.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, FontId, Pos2, Rect as PointRect, Shape,
    Stroke, TextureOptions, Vec2,
};

use crate::handset::{self, Handset, Rect, Skin};
#[cfg(not(target_arch = "wasm32"))]
use crate::keymap;
use crate::runtime::Runtime;
use crate::screen::Format;
use crate::session::Finished;

/// Where one frame's time went, in the two places it can go.
///
/// Drawn in the corner when `QUOIN_STATS` is set. It exists because "the picture runs badly" is otherwise
/// a sentence nobody can act on: the guest runs on another thread and the frame loop is not visible from
/// this window at all, so whether it is the guest, the upload or the window is not something anybody can
/// see from outside. `emulator` is carried in from the worker through [`crate::session::Status`];
/// `upload` is timed here.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FrameCost {
    /// Running guest instructions — the emulator itself, which is the heavy one when it is heavy.
    pub emulator: Duration,
    /// Reading the screen out of guest memory and handing it to the GPU.
    pub upload: Duration,
}

/// Whether to draw that line: `QUOIN_STATS` in the environment.
///
/// Read once and remembered: an environment lookup per frame is not free, and the answer cannot change
/// under a running process in any way worth honouring.
pub(crate) use crate::session::stats_wanted;

/// A presentation gap beyond this is an *absence* — a freeze, a load, a suspended applet — not a
/// slow frame. One second rather than one frame: a pack may legitimately present only a few times
/// a second, and calling that "no frame rate" was another way the badge lied.
pub(crate) const PRESENTATION_STALE: Duration = Duration::from_millis(1000);

/// A turn that has held the machine this long is worth naming in the badge.
pub(crate) const SLOW_TURN: Duration = Duration::from_millis(250);

/// How long guest progress may be absent before a long turn is described as quiet.
pub(crate) const PROGRESS_STALE: Duration = Duration::from_millis(250);

/// The shortest window in which a frame is still called current, before the run has a rhythm.
const MIN_FRESH: Duration = Duration::from_millis(250);

/// Whether the number on the badge still describes the run's current rhythm.
///
/// A fixed 250 ms threshold called a pack presenting every 400 ms "no frame rate" forever. Once
/// two presentations have established a gap, allow twice that gap (with a floor) before the number
/// goes stale; an isolated freeze then stops being quoted instead of lingering on screen.
pub(crate) fn presentation_fresh(
    now: Instant,
    last_frame: Option<Instant>,
    last_gap: Option<Duration>,
) -> bool {
    let Some(at) = last_frame else {
        return false;
    };
    let allow = match last_gap {
        Some(g) => g.saturating_mul(2).max(MIN_FRESH),
        None => MIN_FRESH,
    };
    now.duration_since(at) <= allow
}

/// Everything the FPS badge decides from, gathered so the decision can be asserted without an egui
/// context. `busy_for` is the age of the turn the worker is inside, and is `None` between turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FpsReading {
    pub running: bool,
    pub turn_active: bool,
    pub advancing: bool,
    pub busy_for: Option<Duration>,
}

/// Fold one presentation gap into the smoothed rate. A gap beyond [`PRESENTATION_STALE`] is an
/// *absence*, not a slow frame: it restarts the measurement rather than becoming a tiny `instant`
/// that formats as `0 FPS` and then takes a dozen ordinary frames to climb back. Anything up to
/// that bound is a real presentation rate and is quoted as one, even if it is only a few FPS.
pub(crate) fn fold_fps(fps: f32, gap: Option<Duration>) -> f32 {
    match gap {
        Some(g) if g <= PRESENTATION_STALE && g.as_secs_f32() > 0.0 => {
            let instant = 1.0 / g.as_secs_f32();
            if fps == 0.0 {
                instant
            } else {
                fps * 0.9 + instant * 0.1
            }
        }
        Some(_) => 0.0,
        None => fps,
    }
}

/// The badge's whole decision. `--` means "there is no current presentation rate"; the word after it
/// reports only what was *observed*, never a diagnosis: a still screen, a guest computing one
/// expensive frame, and host work inside a turn are three different things, and none of them is
/// evidence that the machine has failed.
pub(crate) fn fps_badge(fps: f32, fresh: bool, r: FpsReading) -> String {
    if !r.running {
        return "FPS --".to_string();
    }
    if fresh && fps > 0.0 {
        return format!("{:.0} FPS", fps);
    }
    // A turn shorter than SLOW_TURN is an ordinary frame that happened to present nothing; the
    // guest may spend it waiting on a timer and retire no instructions at all. Only a turn that
    // has held the machine long enough is worth naming.
    let long_turn = r.turn_active && r.busy_for.is_some_and(|d| d >= SLOW_TURN);
    let why = if !r.turn_active || !long_turn {
        "idle"
    } else if !r.advancing {
        // `quiet`, not `stalled`: the host does screenshot, audio and network work *inside* the
        // same turn, and none of it retires guest instructions. Saying the machine stalled there
        // would blame the guest for what the host was doing.
        "quiet"
    } else {
        "busy"
    };
    match r.busy_for {
        Some(d) if long_turn => {
            format!("FPS -- ({why} {:.0} ms)", d.as_secs_f64() * 1000.0)
        }
        _ => format!("FPS -- ({why})"),
    }
}

/// The gap between the body and the window's edge, in points. The body's own outline is the margin a
/// player reads, so this only keeps that outline off the title bar.
///
/// `pub(crate)` because two windows need the same number now: the solo one ([`Solo`]) and the game
/// window the manager opens (`manager.rs`'s `game_window`). Two literals is how they drift apart.
pub(crate) const PAD: f32 = 10.0;

/// What the desktop takes for itself: the window's title bar and the taskbar. egui's `screen_rect` is
/// the *monitor*, and a body sized to just fit the monitor ends with its last row of keys under the
/// taskbar, which is the same failure a fixed initial guess used to have.
const CHROME_Y: f32 = 84.0;

/// What the window and the run share is now [`crate::session::Shared`], and the machine itself belongs
/// to the guest thread. What used to live here — a `Session` holding the `Runtime`, the `Speaker`, the
/// folded `Dispatched` and the first error, behind one `Arc<Mutex<_>>` that the repaint callback
/// grabbed every frame — cannot be a lock any more: while the callback held it and was inside
/// `Runtime::pump`, the window's own thread *was* the guest, and a pack that spun inside one function
/// took the close box with it.
///
/// Run `rt` in a window until the player closes it, and hand the machine back.
///
/// The runtime comes back whichever way the session ended, because what the run *did* — the call
/// log, the gaps it named, the frame it left on screen — is read out of it. So does *why* it ended:
/// a window that was closed and a window that closed itself because the guest faulted are the same
/// shape from here, and only the worker knows which happened. See [`Finished::why`].
#[cfg(target_arch = "wasm32")]
pub fn show(
    rt: Runtime,
    _title: &str,
    _skin: &str,
    _soundfont: Option<&str>,
    _audio_backend: crate::audio::AudioBackend,
    _control: Option<crate::control::Control>,
    _show_fps: bool,
) -> Finished {
    Finished {
        rt,
        pumped: Dispatched::default(),
        error: Some("a native window is not available in this build".into()),
        // This front end's own refusal, not a cancel: nothing asked the run to stop, it just could
        // not be shown. `End::Finished` is the value that says "not the person's doing".
        why: crate::session::End::Finished,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn show(
    rt: Runtime,
    title: &str,
    skin: &str,
    soundfont: Option<&str>,
    audio_backend: crate::audio::AudioBackend,
    control: Option<crate::control::Control>,
    show_fps: bool,
) -> Finished {
    let skin = handset::skin(skin).unwrap_or(&handset::SKINS[0]);
    // The machine goes to the guest thread here and is not seen by this thread again. The handset, the
    // speaker and the layout rules that used to be "put together in one place" are still in one place —
    // [`crate::session::Guest::start`] for the machine, [`Frontend::new`] for what is drawn — which is
    // how a `--window` run and a manager run keep agreeing.
    let guest = crate::session::Guest::start_with_backend(
        rt,
        crate::session::Sound::named(soundfont),
        audio_backend,
        control,
        crate::screen::Format::Rgba8888,
    );
    let shared = Arc::clone(&guest.shared);
    let frontend = Frontend::new(shared, skin, BTreeMap::new(), show_fps);
    let (wide, high) = (frontend.wide, frontend.high);
    // An integer multiple, never a fraction: a 176x220 pack is unreadable at 1:1, and anything
    // between the integers smears the art no matter who does the stretching. It is chosen against
    // the *body*, so a keypad cannot end up behind the taskbar because the screen alone fitted. This
    // is only the size the window opens at; the first frame corrects it against the monitor.
    let scale = fit(Vec2::new(1024.0, 768.0), frontend.body());
    // The size the module was told it has, where a player can read it: a pack drawn on a screen too
    // small for its own art looks broken rather than mis-sized, and this is the number to change.
    let caption = format!("{title} {wide}x{high}{}", keymap::HINT);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(frontend.body() * scale + Vec2::splat(2.0 * PAD))
            .with_icon(app_icon())
            .with_title(&caption),
        ..Default::default()
    };
    let opened = eframe::run_native(
        &caption,
        options,
        Box::new(move |cc| {
            load_cjk_fonts(&cc.egui_ctx);
            // The body is what a player looks at, so what is behind it has to be darker than the
            // darkest skin offered. egui's default light panel leaves a silver handset with no edge.
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Solo(frontend)))
        }),
    );
    // The guest thread is asked to stop and waited for. This is the one place in a windowed run that
    // joins, and it is after `run_native` has returned, so the person who closed the window is not the
    // one waiting: the wait is bounded by one poll plus however long the interpreter takes to notice,
    // which is [`crate::cpu::CANCEL_LOOK`] instructions.
    let mut finished = guest.finish();
    // A runtime the guest has been executing against is still worth reporting on, so a failed run
    // comes back rather than being lost along with the error.
    if let Err(e) = opened {
        // The blame here is this front end's: the run was over because the window would not open, not
        // because anybody asked it to stop, and `why` says so — `guest.finish` has just cancelled it,
        // so the worker's own answer would have been `Cancelled` and the report would have opened
        // with a sentence about a window the person never saw.
        finished.error = Some(format!("could not open a window: {e}"));
        finished.why = crate::session::End::Finished;
    }
    finished
}

/// The largest integer multiple of the body that fits `available`. A fraction would blur; an integer
/// that overflows hides the keypad behind the taskbar, which is what a fixed guess did.
///
/// Public because the manager asks the same question about its own panel: a phone drawn at a
/// different multiple than the one its caller measured stops lining up with the space it was given.
pub(crate) fn fit(available: Vec2, body: Vec2) -> f32 {
    let room = (available - Vec2::new(2.0 * PAD, 2.0 * PAD + CHROME_Y)).max(Vec2::ZERO);
    (room.x / body.x)
        .floor()
        .min((room.y / body.y).floor())
        .clamp(1.0, 4.0)
}

/// One device rectangle, in points, at `origin`. The only place the scale is applied, which is what
/// guarantees the screen and the keys around it are multiplied by the *same* number: a key drawn at
/// a different scale stops lining up with the picture it is answering.
fn place(r: Rect, origin: Pos2, scale: f32) -> PointRect {
    PointRect::from_min_max(
        origin + Vec2::new(r.x * scale, r.y * scale),
        origin + Vec2::new((r.x + r.wide) * scale, (r.y + r.high) * scale),
    )
}

fn ink(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// How round a key's corners are: a fifth of its own short side, so a digit and a soft-key bar both
/// read as moulded rather than one of them reading as a pill. Whole points, because that is what egui
/// measures a corner in.
fn cell_radius(r: PointRect) -> u8 {
    (r.size().min_elem() / 5.0).clamp(1.0, 9.0) as u8
}

/// A triangle's three points, pulled `pad` away from the cell that contains them. An arrow drawn to
/// the corners of its box would touch the ring it sits on.
fn shrink(points: [Pos2; 3], cell: PointRect, pad: f32) -> Vec<Pos2> {
    let centre = cell.center();
    points
        .map(|pt| {
            let d = pt - centre;
            let len = d.length().max(0.001);
            centre + d * ((len - pad).max(1.0) / len)
        })
        .to_vec()
}

/// The largest label that fits the key it belongs to. egui cannot be asked to fit text to a box, so
/// this measures it: shrink until it is inside, which is a few steps for a label of two characters.
fn label_size(painter: &egui::Painter, text: &str, cell: PointRect) -> f32 {
    let mut size = (cell.height() * 0.55).clamp(6.0, 18.0);
    while size > 6.5 {
        let got = painter
            .layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE)
            .size();
        if got.x <= cell.width() - 4.0 && got.y <= cell.height() - 2.0 {
            break;
        }
        size *= 0.85;
    }
    size
}

/// The PC key a handset key stands for.
///
/// The names are the platform's and the codes come from the generated ABI table, so this table and the
/// keypad laid out by [`crate::handset`] cannot drift apart, and a name that stops existing is a
/// panic rather than a key that silently sends something else.
#[cfg(test)]
fn handset_of(key: egui::Key) -> Option<u32> {
    handset_of_with(key, &BTreeMap::new())
}

/// The effective binding: a custom `AVK_*` entry first, then the default list for that key.
fn handset_of_with(key: egui::Key, bindings: &BTreeMap<String, String>) -> Option<u32> {
    let name = format!("{key:?}");
    for handset in crate::keymap::KEYS {
        if bindings
            .get(handset.name)
            .is_some_and(|bound| bound == &name)
        {
            return Some(handset.avk());
        }
    }
    for handset in crate::keymap::KEYS {
        if !bindings.contains_key(handset.name)
            && crate::keymap::default_bindings(handset.name).contains(&name.as_str())
        {
            return Some(handset.avk());
        }
    }
    None
}

/// What one frame of input says to the phone.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Keys {
    /// `AVK_*` codes that went down this frame, in the order they arrived.
    pressed: Vec<u32>,
    /// The same for the ones that came up.
    released: Vec<u32>,
    /// The player pressed the key that leaves — Esc, which no handset has.
    leaving: bool,
}

/// Read one frame of egui's input the way the phone reads it.
///
/// Nothing at all while something else in the window wants the keyboard. The manager draws a search box
/// beside this phone, and a key that reaches both is a search box nobody can type in *and* a game
/// answering keystrokes aimed at a text field. Escape goes with it: while a box has the keyboard, Escape
/// belongs to the box.
///
/// Auto-repeat is a held key being re-reported, not a new press, and a handset does not call it one
/// either — so repeats are dropped here.
fn phone_keys(ctx: &egui::Context, bindings: &BTreeMap<String, String>) -> Keys {
    let mut keys = Keys::default();
    if ctx.wants_keyboard_input() {
        return keys;
    }
    for event in ctx.input(|i| i.events.clone()) {
        if let egui::Event::Key {
            key,
            pressed,
            repeat,
            ..
        } = event
        {
            if key == egui::Key::Escape {
                keys.leaving = true;
            }
            if !repeat {
                if let Some(code) = handset_of_with(key, bindings) {
                    if pressed {
                        keys.pressed.push(code);
                    } else {
                        keys.released.push(code);
                    }
                }
            }
        }
    }
    keys
}

/// The emulator icon, embedded from the source SVG's 256-pixel raster.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/app-icon-256.png"
    )))
    .expect("the build generated a valid app-icon PNG")
}

/// How many candidate files are opened while looking for a face that can draw everything.
///
/// A read can be tens of megabytes and the candidate list can hold dozens of installed faces, so
/// the search is bounded rather than exhaustive. It counts reads, not candidates: a path that is
/// not there costs nothing and must not use up the budget that a real face behind it needs.
const FACE_LIMIT: usize = 4;

/// Hand egui a font that can draw CJK, if this machine has one. Silent when it does not: every label
/// the emulator itself writes is ASCII, so a missing font costs a Chinese game's text and nothing
/// else, and refusing to start over a font would be the worse outcome.
///
/// The first candidate that exists is the one the host meant — the manager's saved choice, a font
/// the person installed, or a face the build reference in `ui-fonts`. The paths are per platform
/// because the manager and this window are both meant to build and run somewhere other than
/// Windows. One face is not always enough: a bundled webfont family is Latin-only, and loading it
/// alone would turn every Chinese string into boxes that used to be text. So when the chosen face
/// cannot draw CJK, the search continues for one that can and it is loaded behind it. egui picks per
/// glyph, which is what makes that order work: Latin from the chosen face, Chinese from the
/// fallback.
pub(crate) fn load_cjk_fonts(ctx: &egui::Context) {
    let mut faces: Vec<Vec<u8>> = Vec::new();
    let mut opened = 0usize;
    for path in crate::platform::fonts::cjk_font_candidates() {
        if opened >= FACE_LIMIT {
            break;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        opened += 1;
        let covers_cjk = crate::platform::fonts::covers_cjk(&bytes);
        if faces.is_empty() {
            faces.push(bytes);
            if covers_cjk {
                break;
            }
        } else if covers_cjk {
            faces.push(bytes);
            break;
        }
    }
    if faces.is_empty() {
        return;
    }

    let mut fonts = egui::FontDefinitions::default();
    for (index, bytes) in faces.into_iter().enumerate() {
        let name = format!("quoin-face-{index}");
        fonts
            .font_data
            .insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(index, name);
    }
    ctx.set_fonts(fonts);
}

/// The window's half of the session: what it keeps between frames to avoid redoing.
///
/// It holds no machine. Everything here is drawing state plus one handle to
/// [`crate::session::Shared`], which is the only thing this thread is allowed to touch about a run —
/// and the borrow is a copy of a frame or a `push` into an inbox, never a call into the guest.
pub(crate) struct Frontend {
    shared: Arc<crate::session::Shared>,
    /// The manager's custom PC-key bindings, copied when this session starts.
    bindings: BTreeMap<String, String>,
    texture: Option<egui::TextureHandle>,
    /// The `Display::presentations` the texture was last filled at, kept so the corner readout can say
    /// which frame the player is looking at. A still screen uploads nothing: the worker only publishes
    /// when that number moves. `None` is "no frame has come yet"; `Some(0)` is the screen as it came up,
    /// which is not a picture the pack drew — see [`Frontend::body`], which says so on the glass.
    painted: Option<u32>,
    /// The keys the pointer is holding down, from the previous frame. A key's press and release are
    /// two moments, and the gap between them is what an app that polls held keys reads.
    held: Vec<u32>,
    /// The same for the physical keyboard, kept so that a key pressed there also **lights up on the
    /// phone**. Two views of one state, and a player who cannot see which key they are holding will
    /// press it again.
    phys_held: Vec<u32>,
    /// The contact the player's finger is holding on the glass, in device pixels, or `None` when
    /// nothing is down. A pointer contact is three moments like a key is — down, the drag between,
    /// up — and a pack that reads *which* contacts are down sees nothing from a press and a release
    /// in the same frame, which is why this is remembered across frames rather than derived from
    /// the mouse position alone.
    touch: Option<(i32, i32)>,
    /// The integer multiple in use. Chosen against a guess before the window exists, then corrected
    /// on the first frame, when egui can say how much room the monitor actually has.
    scale: f32,
    /// The body at the current scale, which is what the window is sized to.
    body: Vec2,
    wide: u32,
    high: u32,
    handset: Handset,
    skin: &'static Skin,
    /// The device's own name, for the plate under the keypad: a run reports what it is pretending to
    /// be, and the window should say so too. Owned, since `--device-json` can name any handset.
    model: String,
    /// Where the last frame's time went, and how many frames there have been — drawn in the corner when
    /// `QUOIN_STATS` is set, and the only way anybody outside this process can see the split. The
    /// emulator half is the *guest thread's* last turn, read out of [`crate::session::Shared`]; the
    /// upload half is this thread's, timed here.
    cost: FrameCost,
    frames: u64,
    /// The last guest presentation and the smoothed presentation rate. This is deliberately a
    /// *presentation* rate: the UI can repaint while the pack has not drawn a new frame, and calling
    /// that a game FPS would flatter a stalled machine.
    last_frame: Option<Instant>,
    fps: f32,
    /// The guest's retired-instruction count as of the last window frame, and when it last moved.
    /// Together they say whether a turn with no new picture is computing or has stopped; these are
    /// the only readings that advance while one guest function holds the machine.
    last_guest_steps: u64,
    progress_since: Instant,
    /// The gap between the last two presentations. Freshness is judged against the run's own rhythm
    /// rather than one fixed frame budget, so a pack that draws a few times a second still gets a
    /// number instead of a permanent `--`.
    last_gap: Option<Duration>,
    /// Whether this window draws the FPS badge. The manager persists the preference; a solo run
    /// takes it from the command line.
    show_fps: bool,
    /// Set when the player has left the game — Esc, or the run ending in an error. What that *means*
    /// belongs to the window: a solo window closes, the manager goes back to its list. The manager reads
    /// it through [`Frontend::view`]'s return value rather than by reaching in here.
    left: bool,
}

/// A game window on its own: what `quoin <file.mod> --window` opens. The game runs until the player
/// leaves it, and leaving it closes the window.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct Solo(Frontend);

impl eframe::App for Solo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // A solo window *is* the handset's window, so it sizes itself to a whole multiple of the phone
        // — that is what `--window` has always done, and why the keys cannot end up behind the taskbar.
        // The manager does no such thing: it draws the phone into a corner of a window it already has.
        let want = fit(ctx.screen_rect().size(), self.0.body());
        if want != self.0.scale {
            self.0.scale = want;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                self.0.body() * want + Vec2::splat(2.0 * PAD),
            ));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.0.view(ui, want);
        });
        if self.0.left {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl Frontend {
    /// A session that has just come up, and the other half of it.
    ///
    /// Both windows need this: the solo one in [`show`], and the manager in its own launch. They need
    /// the same thing — a machine that has started, a speaker if the run was given a SoundFont, and a
    /// handset laid out for the size that machine reports — so it lives here rather than twice.
    ///
    /// The returned `Arc` is the same session the front end holds. A solo window keeps it because it
    /// has to read the run out of a window that has closed; the manager drops it, because a session
    /// that ends takes its machine with it.
    ///
    /// The speaker is built here, before the window exists, because a SoundFont that will not load is
    /// something to say once rather than a failure per frame. It is given no clips, and could not be:
    /// the guest has not run yet, so the run has named nothing and every clip is still in its future —
    /// the speaker parses each one the first time it is asked to play it.
    pub(crate) fn new(
        shared: Arc<crate::session::Shared>,
        skin: &'static Skin,
        bindings: BTreeMap<String, String>,
        show_fps: bool,
    ) -> Frontend {
        let (wide, high, model) = (shared.wide, shared.high, shared.model.clone());
        let handset = Handset::for_device(wide, high);
        let body = Vec2::new(handset.body.wide, handset.body.high);
        Frontend {
            shared,
            bindings,
            texture: None,
            painted: None,
            held: Vec::new(),
            phys_held: Vec::new(),
            touch: None,
            // A starting point; the first frame replaces it with a number worked out from the space
            // the caller actually has.
            scale: 1.0,
            body,
            wide,
            high,
            handset,
            skin,
            model,
            cost: FrameCost::default(),
            frames: 0,
            last_frame: None,
            fps: 0.0,
            last_guest_steps: 0,
            progress_since: Instant::now(),
            last_gap: None,
            show_fps,
            left: false,
        }
    }

    /// The handset as this session laid it out, in device pixels. The caller works out its own
    /// integer scale from this, because only the caller knows how much room there is.
    pub(crate) fn body(&self) -> Vec2 {
        self.body
    }

    /// The screen the module was told it has, in device pixels — what a caption reports, and what a
    /// person compares against the handset in front of them.
    pub(crate) fn device(&self) -> (u32, u32) {
        (self.wide, self.high)
    }

    /// One frame of the game, drawn into whatever room `ui` has, at an integer multiple of the
    /// device's own pixels.
    ///
    /// Not an `eframe::App` itself, because two windows want it: the one a `quoin <file.mod> --window`
    /// run opens ([`Solo`]), and the manager's, which shows this beside its list while a game runs.
    /// What both need to know is whether the game is over, and that is the returned flag rather than a
    /// viewport command — only the window knows whether the end means "quit" or "back to the list",
    /// and a front end that issued the command here would turn "return" into "exit".
    ///
    /// `scale` is the caller's integer multiple, and this sends no viewport command of its own: how
    /// big the window is is the window's business.
    pub(crate) fn view(&mut self, ui: &mut egui::Ui, scale: f32) -> bool {
        self.scale = scale;
        // Cloned because egui hands out a handle, and the drawing below needs `ui` mutably while this
        // is still live. The same clone carries the keys at the end, so nothing here borrows `self`
        // while the panel is alive.
        let ctx = ui.ctx().clone();
        let shared = Arc::clone(&self.shared);
        // One short lock for everything this frame can know about the run. Held for a copy, never
        // across a draw and never across the machine — which is the whole point of the split: this
        // thread cannot be blocked by a pack that has stopped returning.
        //
        // The worker writes its frame *before* it writes the status below it, so whichever order this
        // window reads them in, the two can be one turn apart (status-then-frame can pair these numbers
        // with a picture a turn newer, frame-then-status with a picture a turn older). They are a
        // liveness reading — "is the run still advancing?" — and not a caption for these pixels, so the
        // skew is stated rather than fixed; taking them under one lock would need the two halves
        // published together, which is a bigger change than this line is worth.
        let now = Instant::now();
        let status = shared.status();
        self.cost.emulator = status.emulator;
        if status.why != crate::session::End::Running {
            // The run is over: the guest came back, faulted, was cancelled, or `--serve`'s `quit` was
            // served. Whether that means "close" or "back to the list" is still the window's call, so
            // it is told rather than told to close.
            self.left = true;
        }
        // The one signal that moves *inside* a long guest function. Two samples apart are enough to
        // tell "the pack is computing this frame" from "the machine has stopped advancing", which a
        // frozen `Status` cannot say: all of its fields are written between turns.
        let steps = shared.guest_steps();
        if steps > self.last_guest_steps {
            self.last_guest_steps = steps;
            self.progress_since = now;
        }
        let advancing = now.duration_since(self.progress_since) <= PROGRESS_STALE;
        // Only while a turn is in progress is its start time a duration worth quoting. Between turns
        // that start is history: quoting its age as "idle 5000 ms" was a fake stall timer that made
        // a perfectly healthy still screen look like a five-second freeze.
        let busy_for = status
            .turn_active
            .then_some(status.turn_started_at)
            .flatten()
            .map(|at| now.duration_since(at));
        let reading = FpsReading {
            running: status.why == crate::session::End::Running,
            turn_active: status.turn_active,
            advancing,
            busy_for,
        };
        // The frame the guest presented, if this window has not taken it. Taking it *is* the upload
        // decision: the worker publishes only when `presentations` moves, so a still screen costs
        // nothing, and the pixels are converted on the thread that owns the drawing context.
        if let Some(frame) = shared.take_frame() {
            let gap = self.last_frame.map(|p| now.duration_since(p));
            self.fps = fold_fps(self.fps, gap);
            // An absence is not a rhythm: it must not widen the freshness window and let a stale
            // number linger through the next freeze.
            if gap.is_some_and(|g| g <= PRESENTATION_STALE) {
                self.last_gap = gap;
            }
            self.last_frame = Some(now);
            self.painted = Some(frame.presentations);
            let began = Instant::now();
            self.upload(&ctx, &frame.pixels, frame.format);
            self.cost.upload = began.elapsed();
        }
        // Collected here and posted after the panel closes, so the widget code never holds a borrow
        // of the runtime while the layout it is building is alive.
        let mut down_now = Vec::new();
        let mut presses = Vec::new();
        let mut releases = Vec::new();
        // Pointer contacts collected the same way, for the same reason: the panel's closure must not
        // hold a borrow of the runtime while it is building layout.
        let mut contacts: Vec<(u32, i32, i32)> = Vec::new();
        let mut touch_now = self.touch;
        // The physical keyboard, which arrives as events rather than as state, read the way the phone
        // reads it — and not at all while something else in the window is typing.
        let keys = phone_keys(&ctx, &self.bindings);
        if keys.leaving {
            // Esc leaves the game, which for the manager means the list and for a solo window the end of
            // the run. Same reason as the error above: the key is the front end's, the consequence is
            // the window's. It also asks the guest thread to stop, which is the whole point of having
            // one: this is a window that can leave a pack that cannot leave it.
            shared.cancel();
            self.left = true;
        }
        for code in keys.pressed {
            if !self.phys_held.contains(&code) {
                self.phys_held.push(code);
            }
            presses.push(code);
        }
        for code in keys.released {
            self.phys_held.retain(|k| *k != code);
            releases.push(code);
        }
        // Everything the drawing below reads is taken out of `self` first, so the drawing never
        // borrows the frontend it is running inside of.
        let skin = self.skin;
        let model = self.model.clone();
        let handset = self.handset.clone();
        let screen_tex = self.texture.as_ref().map(|t| t.id());
        // What the screen says while the host has nothing but the boot frame to show. The two numbers are
        // the point: a loader that is running and a run that has stopped look identical otherwise, and
        // the person in front of the window is the only one who can tell them apart. `session.rs` no
        // longer paints a never-presented screen as a black frame, so this line is what they see during
        // it. The count is stated as **this host's own**, which is all the window can prove:
        // `Display::update` returns without counting a presentation when the destination is not the
        // device bitmap, so a pack that draws into its own DIB and calls it every frame still owes this
        // screen nothing — and saying "the pack has not called IDisplay::Update" would be a claim about the pack that
        // these pixels cannot support.
        //
        // A run that has already ended gets its own sentence, because the frame it stopped on may well be
        // the boot one: `run` breaks out of a turn on an error before it publishes, so the last picture
        // here is older than the news, and "no frame yet" would be the true count and the wrong story.
        let waiting = match status.ended.as_deref() {
            Some(why) => format!(
                "this run has ended: {why}\nvirtual time {} ms, {} callback(s)",
                status.clock_ms, status.pumped.callbacks
            ),
            None => format!(
                "no frame yet: the host recorded {} presentation(s)\nvirtual time {} ms, {} callback(s)",
                self.painted.unwrap_or(0),
                status.clock_ms,
                status.pumped.callbacks
            ),
        };
        // Has the pack drawn anything the player is looking at? `Some(0)` is the screen as it came up,
        // which the worker publishes on purpose (`session::worth_publishing` and the test behind it),
        // so the count — not the presence of a texture — is what separates "booted, silent" from
        // "drawing".
        let drawn = matches!(self.painted, Some(n) if n > 0);
        let phys = self.phys_held.clone();
        let (dev_w, dev_h) = (self.wide, self.high);
        {
            let (area, _) =
                ui.allocate_exact_size(window_size(handset.body, scale), egui::Sense::hover());
            let origin = area.min + Vec2::splat(PAD);
            let p = ui.painter();
            // The body, then the glass, then the keys: a handset reads as an object with something
            // inset in it, and that is the order the shapes have to be painted in.
            let body = place(handset.body, origin, scale);
            p.add(Shape::rect_filled(
                body,
                CornerRadius::same(14),
                ink(skin.body),
            ));
            p.add(Shape::rect_stroke(
                body,
                CornerRadius::same(14),
                Stroke::new(1.5_f32, ink(skin.edge)),
                egui::StrokeKind::Inside,
            ));
            for plate in [handset.ear, handset.chin] {
                p.add(Shape::rect_filled(
                    place(plate, origin, scale),
                    CornerRadius::same(2),
                    ink(skin.bezel),
                ));
            }
            let screen = place(handset.screen, origin, scale);
            p.add(Shape::rect_filled(
                screen.expand(4.0),
                CornerRadius::same(4),
                ink(skin.bezel),
            ));
            match screen_tex {
                Some(id) => {
                    // `Color32::WHITE` is a multiplier, so it is the one tint that leaves a game
                    // pixel exactly as the guest wrote it.
                    p.image(
                        id,
                        screen,
                        PointRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                None => {}
            }
            if !drawn {
                // Either the screen as it came up, or no pixels at all yet. Both look like "started
                // but drew nothing" to a person, and neither is a pack that has stopped — so the line goes over the
                // top of it, with the two numbers that say the machine is moving.
                p.text(
                    screen.center(),
                    Align2::CENTER_CENTER,
                    &waiting,
                    FontId::proportional(13.0),
                    ink(skin.ink),
                );
            }
            // The glass itself. A pack that reads `touch=1` out of its own `config.txt` and then
            // waits on `EVT_POINTER_*` has no key to press for what it wants: where the player
            // touches *is* the input, so the screen has to answer for a contact across its whole
            // area rather than only where something was drawn to be clicked.
            let glass = ui.interact(screen, ui.id().with("glass"), egui::Sense::click_and_drag());
            let to_dev = |pos: Pos2| {
                let x = ((pos.x - screen.min.x) / screen.width() * dev_w as f32) as i32;
                let y = ((pos.y - screen.min.y) / screen.height() * dev_h as f32) as i32;
                (x.clamp(0, dev_w as i32 - 1), y.clamp(0, dev_h as i32 - 1))
            };
            if let Some(pos) = glass.interact_pointer_pos() {
                let at = to_dev(pos);
                if glass.drag_started() {
                    contacts.push((crate::abi::generated::EVT_POINTER_DOWN, at.0, at.1));
                    touch_now = Some(at);
                } else if glass.dragged() && touch_now != Some(at) {
                    contacts.push((crate::abi::generated::EVT_POINTER_MOVE, at.0, at.1));
                    touch_now = Some(at);
                }
            }
            // Released from the *global* button state rather than from the widget's own, because a
            // finger that leaves the glass mid-drag must not leave a contact down forever: the
            // handset would go on steering toward a point nobody is touching.
            if touch_now.is_some() && !ui.input(|i| i.pointer.any_down()) {
                let (x, y) = touch_now.take().unwrap();
                contacts.push((crate::abi::generated::EVT_POINTER_UP, x, y));
            }
            let chin = place(handset.chin, origin, scale);
            p.text(
                chin.center(),
                Align2::CENTER_CENTER,
                model,
                FontId::proportional((chin.height() * 1.6).clamp(7.0, 11.0)),
                ink(skin.ink),
            );
            // One ring behind the five navigation cells, because that is what a rocker is: a single
            // piece of moulded plastic with five switches under it.
            if let (Some(select), Some(up)) = (handset.key("AVK_SELECT"), handset.key("AVK_UP")) {
                let (cx, cy) = select.rect.centre();
                let radius = (cy - up.rect.centre().1).abs() + select.rect.high / 2.0 + 2.0;
                let centre = origin + Vec2::new(cx * scale, cy * scale);
                p.add(Shape::circle_filled(centre, radius * scale, ink(skin.ring)));
                p.add(Shape::circle_stroke(
                    centre,
                    radius * scale,
                    Stroke::new(1.0_f32, ink(skin.edge)),
                ));
            }
            for placed in &handset.keys {
                let rect = place(placed.rect, origin, scale);
                // Not `clicked`: a handset reports a key going down as well as coming up, and an app
                // that polls which keys are held cannot see a tap with no duration in it. So a key is
                // down for as long as the pointer is held on it, or for as long as the PC key that
                // stands for it is held.
                let response =
                    ui.interact(rect, ui.id().with(placed.key.name), egui::Sense::click());
                let avk = placed.key.avk();
                let down = (response.hovered() && ui.input(|i| i.pointer.primary_down()))
                    || phys.contains(&avk);
                // `down_now` is the virtual keypad's state only. The physical keyboard already
                // produced its press/release edges above and is kept as a stream; adding it here too
                // would send the same edge to `Shared` twice (and made the input log show every
                // physical key as a duplicate pair).
                if down && !phys.contains(&avk) && !down_now.contains(&avk) {
                    down_now.push(avk);
                }
                let face = if down { skin.key_down } else { skin.key };
                let legend = if down { skin.edge } else { skin.ink };
                // A key that is in has also moved: the same rect one point down, which is the whole
                // of what makes it read as pressed rather than merely recoloured.
                let pressed_rect = if down { rect.translate(Vec2::Y) } else { rect };
                match placed.face {
                    handset::Face::Glyph(glyph) => {
                        let round = CornerRadius::same(cell_radius(rect));
                        p.add(Shape::rect_filled(pressed_rect, round, ink(face)));
                        p.add(Shape::rect_stroke(
                            pressed_rect,
                            round,
                            Stroke::new(0.8_f32, ink(skin.edge)),
                            egui::StrokeKind::Inside,
                        ));
                        let size = label_size(p, glyph, rect);
                        p.text(
                            pressed_rect.center(),
                            Align2::CENTER_CENTER,
                            glyph,
                            FontId::proportional(size),
                            ink(legend),
                        );
                    }
                    handset::Face::Arrow(dir) => {
                        // The tip is the edge mid-point it points at and the base is the two either
                        // side of it, so an arrow is a third of its cell rather than a diagonal
                        // across it — which is what a rocker's engraving looks like.
                        let (tip, a, b) = match dir {
                            handset::Arrow::Up => {
                                (rect.center_top(), rect.left_center(), rect.right_center())
                            }
                            handset::Arrow::Down => (
                                rect.center_bottom(),
                                rect.left_center(),
                                rect.right_center(),
                            ),
                            handset::Arrow::Left => {
                                (rect.left_center(), rect.center_top(), rect.center_bottom())
                            }
                            handset::Arrow::Right => {
                                (rect.right_center(), rect.center_top(), rect.center_bottom())
                            }
                        };
                        let pad = rect.size().min_elem() * 0.28;
                        p.add(Shape::convex_polygon(
                            shrink([tip, a, b], rect, pad),
                            ink(legend),
                            Stroke::NONE,
                        ));
                    }
                    handset::Face::Select => {
                        let r = rect.size().min_elem() / 2.0 * 0.62;
                        let centre = pressed_rect.center();
                        p.add(Shape::circle_filled(centre, r, ink(face)));
                        p.add(Shape::circle_stroke(
                            centre,
                            r,
                            Stroke::new(0.8_f32, ink(skin.edge)),
                        ));
                    }
                }
            }
        }
        let was_down = std::mem::replace(&mut self.held, down_now.clone());
        // This half is a comparison of two states, not a stream, so it has no order to hand on: all the
        // presses come before all the releases. The physical keyboard above it *is* a stream and keeps
        // its real order. See [`crate::session::Queued`].
        presses.extend(down_now.iter().filter(|k| !was_down.contains(k)));
        releases.extend(was_down.iter().filter(|k| !down_now.contains(k)));
        // Keys and contacts go into the inbox rather than into the machine: the guest thread posts them
        // on its next turn, which is the same order the single-threaded window had (a press takes effect
        // on the visit to the guest *after* the frame it was collected in) without this thread ever
        // holding the runtime.
        for avk in presses {
            shared.press(avk);
        }
        for avk in releases {
            shared.release(avk);
        }
        for (evt, x, y) in contacts {
            shared.contact(evt, x, y);
        }
        self.touch = touch_now;
        self.frames += 1;
        if self.show_fps {
            // The game's own presentation rate, always visible. The detailed `QUOIN_STATS` line below
            // answers *where* a slow frame went; this badge is the one number a player reports.
            // When there is no current rate it says why — a still screen, a long computation and a
            // stalled machine are three different things, and only the last one is a fault.
            let fresh = presentation_fresh(now, self.last_frame, self.last_gap);
            let fps_text = fps_badge(self.fps, fresh, reading);
            let fps_at = ui.max_rect().left_top() + Vec2::new(6.0, 6.0);
            ui.painter().text(
                fps_at + Vec2::new(1.0, 1.0),
                Align2::LEFT_TOP,
                &fps_text,
                FontId::monospace(12.0),
                Color32::BLACK,
            );
            ui.painter().text(
                fps_at,
                Align2::LEFT_TOP,
                &fps_text,
                FontId::monospace(12.0),
                Color32::LIGHT_GREEN,
            );
        }
        // Where the frame's time went, in the corner, when asked for. The counter is the other half of it: a
        // frame number that climbs ten times slower than it should is a frame rate, and that is the number a
        // person actually reports as "runs badly". The guest's own turn count rides with it because the two
        // slowings are different complaints: this window repainting slowly, and the worker advancing the
        // machine slowly, need opposite fixes.
        if stats_wanted() {
            // Only present when `--speed` / `--align-clock` is on. The measured rate is the honest
            // number: a 4x request on a frame the host cannot finish inside 16 ms is not 4x.
            let clock_note = match status.clock {
                Some(clock) => format!(
                    " · clock {:.2}x (target {:.2}x, debt {} ms)",
                    clock.actual,
                    clock.target as f32 / 1000.0,
                    clock.debt
                ),
                None => String::new(),
            };
            ui.painter().text(
                ui.max_rect().right_top() + Vec2::new(-6.0, 6.0),
                Align2::RIGHT_TOP,
                format!(
                    "simulate {:.1} ms · upload {:.1} ms · frame {} · guest {} turns · turn {:.0} ms · {} steps{}",
                    self.cost.emulator.as_secs_f64() * 1000.0,
                    self.cost.upload.as_secs_f64() * 1000.0,
                    self.frames,
                    status.turns,
                    busy_for.unwrap_or_default().as_secs_f64() * 1000.0,
                    steps,
                    clock_note,
                ),
                FontId::monospace(11.0),
                Color32::LIGHT_GREEN,
            );
            // A second line, only when the worker is actually watching the JIT cache. `flushes` is
            // the number that answers "was that freeze a code-cache rebuild?" — it jumps exactly
            // once per full invalidation, while `blocks` and `+N` say how much was recompiled after.
            if let Some(jit) = status.jit {
                ui.painter().text(
                    ui.max_rect().right_top() + Vec2::new(-6.0, 22.0),
                    Align2::RIGHT_TOP,
                    format!(
                        "jit {} blocks · +{} this turn · {} flushes · win {}",
                        jit.blocks,
                        jit.compiled,
                        jit.invalidations,
                        status.jit_windows.unwrap_or(0)
                    ),
                    FontId::monospace(11.0),
                    Color32::LIGHT_GREEN,
                );
            }
        }
        // Ask for the next frame rather than waiting for the mouse: the difference between an
        // emulator that animates and a picture that happens to be on screen.
        ctx.request_repaint_after(Duration::from_millis(16));
        !self.left
    }
}

/// The window's ask: the body at the current scale, plus the gap around it.
fn window_size(body: Rect, scale: f32) -> Vec2 {
    Vec2::new(body.wide * scale, body.high * scale) + Vec2::splat(2.0 * PAD)
}

impl Frontend {
    /// Hand the frame to the texture, in the layout it arrived in.
    ///
    /// `Rgba8888` is the case that runs — `session` asks its `Screen` for four bytes a pixel precisely
    /// so that this call is a copy and not a conversion. The other two arms exist so the door is total:
    /// a caller who built their own `Screen` can still show it here, and the 8-bit one costs the same
    /// `from_rgb` pass this window used to pay on every frame.
    fn upload(&mut self, ctx: &egui::Context, pixels: &[u8], format: Format) {
        let size = [self.wide as usize, self.high as usize];
        let image = match format {
            Format::Rgba8888 => ColorImage::from_rgba_premultiplied(size, pixels),
            Format::Rgb888 => ColorImage::from_rgb(size, pixels),
            Format::Rgb565 => {
                // No 5-6-5 image exists in epaint, so this is the one format that has to be widened
                // here. The allocation is stated rather than hidden: nothing in this repository hands
                // this window a 565 frame, and a caller who wants to is asking for the widening.
                //
                // The assertion is not decoration: `expand565` stops at the shorter slice, and the
                // arrival buffer below is sized from the window, so a frame of a different shape would
                // produce a correctly-shaped image with half of it black rather than a panic.
                debug_assert_eq!(
                    pixels.len(),
                    size[0] * size[1] * 2,
                    "a 5-6-5 frame of this size"
                );
                let mut wide = vec![0u8; size[0] * size[1] * 4];
                crate::screen::expand565(pixels, &mut wide);
                ColorImage::from_rgba_premultiplied(size, &wide)
            }
        };
        match &mut self.texture {
            Some(texture) => texture.set(image, TextureOptions::NEAREST),
            None => {
                self.texture = Some(ctx.load_texture("device", image, TextureOptions::NEAREST));
            }
        }
    }
}

#[cfg(test)]
#[path = "test/egui_window.rs"]
mod tests;
