//! A libretro core: `quoin` behind the API RetroArch speaks.
//!
//! The front end owns the window, audio, input and clock and calls `retro_run`
//! once per frame. This file is a shell around one `Runtime` driven by `pump`:
//! it draws into `XRGB8888` (or the guest's `RGB565` when the front end asks),
//! mixes audio itself, and never links the desktop UI or sound device.

use std::os::raw::{c_char, c_double, c_float, c_uint, c_void};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

use quoin::abi::generated as g;
use quoin::audio::{Playing, Speaker};
use quoin::device::DeviceProfile;
use quoin::runtime::Runtime;
use quoin::screen::{Format, Screen};

/// The libretro API version this core is written against. 1 is the only one there has ever been.
const RETRO_API_VERSION: u32 = 1;

/// `RETRO_MEMORY_SAVE_RAM`: the one memory block this core exposes. A front end writes it out as a
/// `.srm` and copies one back in before the first frame.
const MEMORY_SAVE_RAM: c_uint = 0;

/// Save RAM size: the host's `VOLUME_LIMIT`, the largest block `Volume::written_block` can produce.
const SRAM_CAPACITY: usize = quoin::file::VOLUME_LIMIT;

/// One frame's simulated time in milliseconds, spelled out so this core never imports `session`.
const FRAME_MS: u32 = 16;

/// The frame rate the front end is told: `1000 / FRAME_MS`.
const FPS: f64 = 1000.0 / FRAME_MS as f64;

/// Mixer sample rate; one frame yields `SAMPLE_RATE * FRAME_MS / 1000` samples.
const SAMPLE_RATE: f64 = 44_100.0;

// libretro C types: the subset used here, spelled as in `libretro.h`.

#[repr(C)]
pub struct retro_system_info {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
pub struct retro_game_geometry {
    base_width: c_uint,
    base_height: c_uint,
    max_width: c_uint,
    max_height: c_uint,
    aspect_ratio: c_float,
}

#[repr(C)]
pub struct retro_system_timing {
    fps: c_double,
    sample_rate: c_double,
}

#[repr(C)]
pub struct retro_system_av_info {
    geometry: retro_game_geometry,
    timing: retro_system_timing,
}

#[repr(C)]
pub struct retro_game_info {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

type EnvironmentT = Option<unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool>;
type VideoRefreshT =
    Option<unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize)>;
type AudioSampleT = Option<unsafe extern "C" fn(left: i16, right: i16)>;
type AudioSampleBatchT = Option<unsafe extern "C" fn(data: *const i16, frames: usize) -> usize>;
type InputPollT = Option<unsafe extern "C" fn()>;
type InputStateT =
    Option<unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16>;

// `retro_environment` command numbers; only the ones this core uses.
const ENV_GET_CAN_DUPE: c_uint = 3;
const ENV_SHUTDOWN: c_uint = 7;
const ENV_SET_PERFORMANCE_LEVEL: c_uint = 8;
const ENV_SET_PIXEL_FORMAT: c_uint = 10;
const ENV_GET_VARIABLE: c_uint = 15;
const ENV_SET_VARIABLES: c_uint = 16;
const ENV_GET_SAVE_DIRECTORY: c_uint = 31;
const ENV_SET_GEOMETRY: c_uint = 37;

/// One named choice the front end may offer: a key and a string whose first field is the label
/// and whose remaining fields are the values, the first being the default.
#[repr(C)]
struct retro_variable {
    key: *const c_char,
    value: *const c_char,
}

/// Static C strings in a `static`, because the front end keeps the table's address.
unsafe impl Sync for retro_variable {}

/// Core options: `auto` reads the panel from the pack's directory name; the rest are fixed sizes.
static VARIABLES: [retro_variable; 2] = [
    retro_variable {
        key: c"quoin_screen_size".as_ptr(),
        value: c"Screen size (Restart); auto|240x320|320x240|176x220|220x176|176x208|208x176|128x128|128x160|160x128|128x96|240x240|240x400|400x240|320x480|480x240".as_ptr(),
    },
    retro_variable {
        key: ptr::null(),
        value: ptr::null(),
    },
];

const PIXEL_FORMAT_XRGB8888: c_uint = 1;
/// Little-endian 16-bit 5-6-5, which is exactly what the guest's 16-bpp `IBitmap` holds.
const PIXEL_FORMAT_RGB565: c_uint = 2;

const DEVICE_JOYPAD: c_uint = 1;
/// The all-buttons-in-one id, kept only for the diagnostic that compares its answer with the
/// per-button answers. A front end that does not implement it answers zero.
const ID_JOYPAD_MASK: c_uint = 256;

/// Joypad buttons mapped to `AVK_*`, as `(libretro id, AEE key code)`.
const KEYS: [(c_uint, u32); 12] = [
    (4, g::AVK_UP),
    (5, g::AVK_DOWN),
    (6, g::AVK_LEFT),
    (7, g::AVK_RIGHT),
    (8, g::AVK_SOFT1),
    (0, g::AVK_SOFT2),
    (3, g::AVK_SEND),
    (2, g::AVK_SELECT),
    (9, g::AVK_CLR),
    (1, g::AVK_END),
    (10, g::AVK_STAR),
    (11, g::AVK_POUND),
];

// Core state.

/// Everything the callbacks share, behind a `Mutex` (the front end may call from any thread).
struct Core {
    env: EnvironmentT,
    video: VideoRefreshT,
    audio: AudioSampleT,
    audio_batch: AudioSampleBatchT,
    input_poll: InputPollT,
    input_state: InputStateT,
    /// The machine, once `retro_load_game` has built one.
    rt: Option<Runtime>,
    /// The pack path, kept so `retro_reset` can boot the machine again.
    path: Option<String>,
    /// The directory the front end named for this run's saves, kept for the same reason: a re-boot
    /// has to attach it *before* the load, exactly as the first one did.
    save_dir: Option<String>,
    /// Whether the front end took our offer to stop; a core that has been told to shut down keeps
    /// answering `retro_run` with a still frame rather than pumping a machine nobody is watching.
    failed: bool,
    /// Buttons held at the end of the previous frame, for press/release edge detection.
    held: u32,
    /// `XRGB8888` frame buffer reused across calls.
    frame: Vec<u32>,
    /// Guest screen, reused across calls.
    screen: Screen,
    /// Speaker without a device; `None` when no sound bank can be built.
    speaker: Option<Speaker>,
    /// Last clip each voice was told to play, so re-pointing is seen as a new clip.
    playing: Playing,
    /// The mixer's output, allocated once: the audio callback must not allocate, and neither must a
    /// turn that happens sixty times a second.
    left: Vec<f32>,
    right: Vec<f32>,
    /// Interleaved samples for `audio_sample_batch`, reused across calls.
    pcm: Vec<i16>,
    /// Save RAM block handed to the front end, allocated lazily.
    sram: Vec<u8>,
    /// Whether this run has adopted the front end's `.srm` block.
    sram_adopted: bool,
    /// Whether the front end accepts `NULL` from the video callback for an unchanged frame.
    can_dupe: bool,
    /// Presentation id of the last frame handed over.
    last_presentations: u32,
    /// Whether the front end took the guest's `RGB565` instead of `XRGB8888`.
    native: bool,
    /// Whether the optional diagnostic frame has been written for this run.
    dumped: bool,
    /// Frames driven since the last load, so a diagnostic frame can be taken once the run has
    /// settled rather than from its first, half-built picture.
    frames: u32,
    /// Geometry last reported to the front end.
    geometry: (u32, u32),
}

impl Default for Core {
    fn default() -> Self {
        Core {
            env: None,
            video: None,
            audio: None,
            audio_batch: None,
            input_poll: None,
            input_state: None,
            rt: None,
            path: None,
            save_dir: None,
            failed: false,
            held: 0,
            frame: Vec::new(),
            screen: Screen::new(),
            speaker: None,
            playing: Playing::new(),
            left: Vec::new(),
            right: Vec::new(),
            pcm: Vec::new(),
            sram: Vec::new(),
            sram_adopted: false,
            can_dupe: false,
            last_presentations: u32::MAX,
            native: false,
            geometry: (0, 0),
            dumped: false,
            frames: 0,
        }
    }
}

/// The one core this library is. libretro's API is a set of free functions with no handle, so the
/// state has to live somewhere global; this is that somewhere.
static CORE: Mutex<Option<Core>> = Mutex::new(None);

/// Run `f` against the core, creating an empty one if `retro_init` has not run yet.
fn with_core<R>(f: impl FnOnce(&mut Core) -> R) -> Option<R> {
    let mut guard = CORE.lock().ok()?;
    Some(f(guard.get_or_insert_with(Core::default)))
}

/// Tell the front end something, if it asked to be told.
unsafe fn say(core: &Core, cmd: c_uint, data: *mut c_void) -> bool {
    match core.env {
        Some(env) => unsafe { env(cmd, data) },
        None => false,
    }
}

/// Borrow a NUL-terminated C string as a `&str`, or `None` when there is no string there.
unsafe fn cstr(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes();
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

/// The panel a pack should be started on: the front end's answer when it named one, otherwise the
/// size the pack's own directory tree claims for it.
fn screen_for(path: &str, choice: Option<&str>) -> Option<(u16, u16)> {
    match choice {
        Some(value) if value != "auto" => size_of(value),
        _ => quoin::library::screen_of_dir(Path::new(path)),
    }
}

/// The `WxH` a player picked, with the separators a name typed by hand may carry.
fn size_of(value: &str) -> Option<(u16, u16)> {
    let mut parts = value.split(|c| c == 'x' || c == 'X' || c == '×');
    let width = parts.next()?.parse::<u16>().ok()?;
    let height = parts.next()?.parse::<u16>().ok()?;
    (width > 0 && height > 0 && parts.next().is_none()).then_some((width, height))
}

/// Ask the front end which panel the player picked. A front end with no options answers nothing,
/// and that is not an error: the directory hint is still there.
fn screen_choice(env: EnvironmentT) -> Option<String> {
    let Some(env) = env else { return None };
    let mut var = retro_variable {
        key: c"quoin_screen_size".as_ptr(),
        value: ptr::null(),
    };
    if !unsafe {
        env(
            ENV_GET_VARIABLE,
            &mut var as *mut retro_variable as *mut c_void,
        )
    } {
        return None;
    }
    unsafe { cstr(var.value) }
}

/// Tell the front end the picture's shape. Said at load as well as on a mid-run change, because a
/// front end that sized itself before the load answered with the stand-in numbers.
fn set_geometry(env: EnvironmentT, width: u32, height: u32) {
    let Some(env) = env else { return };
    let geometry = retro_game_geometry {
        base_width: width,
        base_height: height,
        max_width: width,
        max_height: height,
        aspect_ratio: width as f32 / height as f32,
    };
    unsafe {
        env(
            ENV_SET_GEOMETRY,
            &geometry as *const retro_game_geometry as *mut c_void,
        );
    }
}

/// Whether the run is a diagnostic one. A normal run pays one cached lookup, not one environment
/// read per frame; the answer cannot change while the library is loaded.
static DIAG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
fn diag() -> bool {
    *DIAG.get_or_init(|| std::env::var_os("QUOIN_DIAG").is_some())
}

/// Optional 24-bit frame dump for diagnosing a wrong picture.
fn dump_frame(screen: &Screen, width: u32, height: u32) {
    let Ok(path) = std::env::var("QUOIN_DUMP_FRAME") else {
        return;
    };
    if width == 0 || height == 0 {
        return;
    }
    let (width, height) = (width as usize, height as usize);
    let stride = (width * 3 + 3) & !3;
    let mut bmp = Vec::with_capacity(54 + stride * height);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((54 + stride * height) as u32).to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(width as i32).to_le_bytes());
    bmp.extend_from_slice(&(height as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&((stride * height) as u32).to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    let pixels = screen.pixels();
    let native = screen.format() == Format::Rgb565;
    let mut row = Vec::with_capacity(stride);
    for y in (0..height).rev() {
        row.clear();
        for x in 0..width {
            let (r, g, b) = if native {
                let i = (y * width + x) * 2;
                if i + 1 >= pixels.len() {
                    (0, 0, 0)
                } else {
                    let word = u16::from_le_bytes([pixels[i], pixels[i + 1]]);
                    (
                        ((((word >> 11) & 0x1f) * 255) / 31) as u8,
                        ((((word >> 5) & 0x3f) * 255) / 63) as u8,
                        (((word & 0x1f) * 255) / 31) as u8,
                    )
                }
            } else {
                let i = (y * width + x) * 3;
                if i + 2 >= pixels.len() {
                    (0, 0, 0)
                } else {
                    (pixels[i], pixels[i + 1], pixels[i + 2])
                }
            };
            row.push(b);
            row.push(g);
            row.push(r);
        }
        row.resize(stride, 0);
        bmp.extend_from_slice(&row);
    }
    if let Err(e) = std::fs::write(&path, &bmp) {
        eprintln!("quoin: cannot write the frame: {e}");
    }
}

/// Boot a `.mod` through the same calls the command line uses.
fn boot(
    path: &str,
    save_dir: Option<&str>,
    screen: Option<(u16, u16)>,
) -> Result<Runtime, String> {
    let path = Path::new(path);
    let save_dir = save_dir.map(Path::new);
    // The panel comes from the pack's directory name or a core option.
    let profile = match screen {
        Some((width, height)) => DeviceProfile::standard().with_screen(width, height),
        None => DeviceProfile::standard(),
    };
    // Use `Runtime::open_pack` and `Runtime::start_pack`; do not duplicate the boot sequence.
    let (mut rt, mounted) = Runtime::open_pack(path, profile, save_dir);
    if let Some(dir) = save_dir {
        eprintln!(
            "quoin: keeping what the app writes in {} ({} entries loaded from it)",
            dir.display(),
            mounted.saves.len()
        );
    }
    let started = rt.start_pack(path, false)?;
    // One judgement, in the library, of the two answers that mean there is nothing to run — see
    // `AutoStart::refusal`. A core that turned either into a plain `false` would lose which one it saw.
    if let Some(why) = started.refusal() {
        return Err(why);
    }
    Ok(rt)
}

/// Put a freshly booted machine in place (used by first load and reset).
fn adopt(core: &mut Core, rt: Runtime, path: String, save_dir: Option<String>) {
    core.path = Some(path);
    core.save_dir = save_dir;
    core.rt = Some(rt);
    core.failed = false;
    core.held = 0;
    // A new machine means a new block to take in: the front end may have brought a different `.srm`.
    core.sram_adopted = false;
    // The new machine's own counter starts at zero, so the first frame has to be sent rather than
    // named as a duplicate of one nobody has seen.
    core.last_presentations = u32::MAX;
    core.dumped = false;
    core.frames = 0;
    // Tell the front end the new panel geometry before the first frame.
    let (width, height) = core
        .rt
        .as_ref()
        .map(|rt| (u32::from(rt.device.width), u32::from(rt.device.height)))
        .unwrap_or((240, 320));
    core.geometry = (width, height);
    set_geometry(core.env, width, height);
    // Stop the mixer so clips from the previous run cannot hold slots in this one.
    if let Some(speaker) = core.speaker.as_mut() {
        speaker.stop_all(&mut core.playing);
    }
}

// libretro entry points.

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    RETRO_API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_get_system_info(info: *mut retro_system_info) {
    if info.is_null() {
        return;
    }
    unsafe {
        *info = retro_system_info {
            library_name: c"QuoinHLE".as_ptr(),
            // Straight from the manifest, so the version a front end reports cannot
            // disagree with the crate it was built from.
            library_version: concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast(),
            // `.breq` first: `.mod` is also an Amiga tracker extension.
            valid_extensions: c"breq|mod".as_ptr(),
            // A pack needs its directory: art and fonts live in `.bar`/`.par` files beside it.
            need_fullpath: true,
            block_extract: false,
        };
    }
}

#[no_mangle]
pub extern "C" fn retro_get_system_av_info(info: *mut retro_system_av_info) {
    if info.is_null() {
        return;
    }
    // The shape the front end was last told about, which is also the one a resize announced; before
    // any load the standard panel's numbers stand in.
    let (width, height) = with_core(|core| {
        if core.geometry != (0, 0) {
            core.geometry
        } else if let Some(rt) = &core.rt {
            (u32::from(rt.device.width), u32::from(rt.device.height))
        } else {
            (240, 320)
        }
    })
    .unwrap_or((240, 320));
    unsafe {
        *info = retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: width,
                base_height: height,
                max_width: width,
                max_height: height,
                // Pixel aspect 1.0; handset panels are not square.
                aspect_ratio: width as f32 / height as f32,
            },
            timing: retro_system_timing {
                fps: FPS,
                sample_rate: SAMPLE_RATE,
            },
        };
    }
}

#[no_mangle]
pub extern "C" fn retro_set_environment(cb: EnvironmentT) {
    with_core(|core| {
        core.env = cb;
        // The one choice a player has: which panel the pack was drawn for. Declared here because
        // this is the call a front end makes before a load, and it builds its options menu from it.
        unsafe {
            say(
                core,
                ENV_SET_VARIABLES,
                VARIABLES.as_ptr() as *const c_void as *mut c_void,
            );
        }
        // Declare the pixel format and performance level rather than letting the front end guess.
        unsafe {
            // A host can opt into the 16-bit path. It is not the default: the same picture through
            // two texture shapes is what showed which half of the display path owned a broken frame.
            let try_native = std::env::var_os("QUOIN_PIXEL_565").is_some();
            let native = try_native
                && say(
                    core,
                    ENV_SET_PIXEL_FORMAT,
                    &PIXEL_FORMAT_RGB565 as *const c_uint as *mut c_void,
                );
            if !native {
                say(
                    core,
                    ENV_SET_PIXEL_FORMAT,
                    &PIXEL_FORMAT_XRGB8888 as *const c_uint as *mut c_void,
                );
            }
            core.native = native;
            // Rebuild the buffers only when the pixel-format answer changes.
            let want = if native {
                Format::Rgb565
            } else {
                Format::Rgb888
            };
            if core.screen.format() != want {
                core.screen = Screen::with_format(want);
            }
            let level: c_uint = 1;
            say(
                core,
                ENV_SET_PERFORMANCE_LEVEL,
                &level as *const c_uint as *mut c_void,
            );
        }
    });
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(cb: VideoRefreshT) {
    with_core(|core| core.video = cb);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(cb: AudioSampleT) {
    with_core(|core| core.audio = cb);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(cb: AudioSampleBatchT) {
    with_core(|core| core.audio_batch = cb);
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(cb: InputPollT) {
    with_core(|core| core.input_poll = cb);
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(cb: InputStateT) {
    with_core(|core| core.input_state = cb);
}

#[no_mangle]
pub extern "C" fn retro_init() {
    with_core(|core| {
        core.failed = false;
        core.frame.clear();
        // The speaker is built here rather than at load, so that a *failed* load still leaves a core
        // that can be initialised again, and so the cost is paid once a session rather than per load.
        match Speaker::without_device(SAMPLE_RATE as i32, None) {
            Ok(speaker) => core.speaker = Some(speaker),
            Err(e) => {
                // Warn once on stderr and keep running when no sound bank can be built.
                eprintln!("quoin: no sound: {e}");
                core.speaker = None;
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    if let Ok(mut guard) = CORE.lock() {
        *guard = None;
    }
}

/// Restart the loaded pack from its initial state.
#[no_mangle]
pub extern "C" fn retro_reset() {
    with_core(|core| {
        if let Some(path) = core.path.clone() {
            let save_dir = core.save_dir.clone();
            let screen = screen_for(&path, screen_choice(core.env).as_deref());
            match boot(&path, save_dir.as_deref(), screen) {
                Ok(rt) => adopt(core, rt, path, save_dir),
                // The machine that was running is left in place: a failed re-boot should not also
                // take away the run the player had.
                Err(e) => eprintln!("quoin: cannot reset {path}: {e}"),
            }
            return;
        }
        let held = core.held;
        if let Some(rt) = core.rt.as_mut() {
            for (bit, code) in KEYS {
                if held & (1u32 << bit) != 0 {
                    rt.post_key_release(code);
                }
            }
        }
        core.held = 0;
    });
}

#[no_mangle]
pub extern "C" fn retro_load_game(info: *const retro_game_info) -> bool {
    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };
    let Some(path) = (unsafe { cstr(info.path) }) else {
        eprintln!("quoin: retro_load_game was given no path (this core asks for a full path)");
        return false;
    };
    // Ask for the save directory, `NULL` frames and the pixel format before booting.
    let asked = with_core(|core| {
        let choice = screen_choice(core.env);
        let mut can_dupe = false;
        if unsafe {
            say(
                core,
                ENV_GET_CAN_DUPE,
                &mut can_dupe as *mut bool as *mut c_void,
            )
        } {
            core.can_dupe = can_dupe;
        }
        let mut dir: *const c_char = ptr::null();
        let answered = unsafe {
            say(
                core,
                ENV_GET_SAVE_DIRECTORY,
                &mut dir as *mut *const c_char as *mut c_void,
            )
        };
        let save_dir = if answered { unsafe { cstr(dir) } } else { None };
        (save_dir, choice)
    });
    // Report `false` when the core lock cannot be taken.
    let Some((save_dir, choice)) = asked else {
        eprintln!("quoin: cannot reach this core's own state, so nothing was loaded");
        return false;
    };
    if save_dir.is_none() {
        // Said out loud because the consequence is a symptom a player reports as something else: a
        // game that keeps starting over is a game whose save had nowhere to go.
        eprintln!(
            "quoin: the front end named no save directory — this run's writes to the app's own \
             volume live in memory and are lost when it ends"
        );
    }
    let screen = screen_for(&path, choice.as_deref());
    match boot(&path, save_dir.as_deref(), screen) {
        Ok(rt) => {
            let stored = with_core(|core| adopt(core, rt, path.clone(), save_dir.clone()));
            if stored.is_none() {
                eprintln!("quoin: cannot store the machine that was just built");
                return false;
            }
            true
        }
        Err(e) => {
            eprintln!("quoin: cannot start {path}: {e}");
            false
        }
    }
}

/// Refuse the multi-slot loader: this core runs a single `.mod`.
#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _game_type: c_uint,
    _info: *const retro_game_info,
    _num_info: usize,
) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    with_core(|core| core.rt = None);
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> c_uint {
    // `RETRO_REGION_NTSC` keeps the core's own fps in charge.
    0
}

/// Save states are refused on purpose.
#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    0
}

#[no_mangle]
pub extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: c_uint) -> *mut c_void {
    if id != MEMORY_SAVE_RAM {
        return ptr::null_mut();
    }
    let asked = with_core(|core| {
        // Sized on the first ask rather than at init: this is the call that says somebody is about to
        // use it, and 16 MiB of zeroes for a front end that never saves is 16 MiB of nothing.
        if core.sram.len() < SRAM_CAPACITY {
            core.sram.resize(SRAM_CAPACITY, 0);
        }
        // Refresh the buffer from the volume only after this run adopted the `.srm`.
        if core.sram_adopted {
            if let Some(rt) = core.rt.as_ref() {
                let block = rt.volume.written_block();
                // A block longer than the claim cannot happen — `SRAM_CAPACITY` *is* the volume's own
                // write ceiling — but if it ever did, saying so beats truncating a save in silence.
                if block.len() > core.sram.len() {
                    eprintln!(
                        "quoin: the app wrote {} bytes of save and this core claims {}",
                        block.len(),
                        core.sram.len()
                    );
                }
                let n = block.len().min(core.sram.len());
                core.sram[..n].copy_from_slice(&block[..n]);
                for byte in &mut core.sram[n..] {
                    *byte = 0;
                }
            }
        }
        core.sram.as_mut_ptr() as *mut c_void
    });
    asked.unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: c_uint) -> usize {
    if id == MEMORY_SAVE_RAM {
        SRAM_CAPACITY
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {
    // One input shape is served, and it is the joypad. A front end that announces something else (a
    // keyboard, a mouse) is not refused — it is simply not answered, because nothing here reads it.
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

/// One frame: poll input, advance the machine, hand over the picture and audio.
#[no_mangle]
pub extern "C" fn retro_run() {
    let _ = with_core(|core| {
        if core.failed {
            return;
        }
        core.frames = core.frames.wrapping_add(1);
        // Take the callback out of `core` before borrowing the runtime mutably.
        let env = core.env;
        let video = core.video;
        let audio_batch = core.audio_batch;
        let poll = core.input_poll;
        let state = core.input_state;
        let Some(rt) = core.rt.as_mut() else {
            return;
        };
        // Adopt the front end's `.srm` on the first turn, when it has been filled.
        if !core.sram_adopted {
            core.sram_adopted = true;
            if !core.sram.is_empty() {
                match rt.volume.adopt_block(&core.sram) {
                    Ok(0) => {}
                    Ok(n) => eprintln!("quoin: took {n} file(s) out of the front end's save"),
                    Err(e) => eprintln!("quoin: cannot use the front end's save: {e}"),
                }
            }
        }
        // Input comes first: libretro's contract is that a core polls once a frame, and a button that
        // is held between two frames has to be seen by the frame that follows it.
        if let (Some(poll), Some(state)) = (poll, state) {
            unsafe { poll() };
            // Poll buttons individually; the all-buttons id is optional and may answer zero.
            let mut mask = 0u32;
            for (bit, _) in KEYS {
                if unsafe { state(0, DEVICE_JOYPAD, 0, bit) } != 0 {
                    mask |= 1u32 << bit;
                }
            }
            if diag() {
                let packed =
                    unsafe { state(0, DEVICE_JOYPAD, 0, ID_JOYPAD_MASK) as u32 } & 0xffff;
                if core.frames % 30 == 0 || mask != 0 || packed != 0 {
                    eprintln!(
                        "quoin diag input frame={} packed={:04x} buttons={:04x}",
                        core.frames, packed, mask
                    );
                }
            }
            let changed = mask ^ core.held;
            if changed != 0 {
                for (bit, code) in KEYS {
                    if changed & (1u32 << bit) != 0 {
                        if mask & (1u32 << bit) != 0 {
                            rt.post_key_press(code);
                        } else {
                            rt.post_key_release(code);
                        }
                    }
                }
            }
            core.held = mask;
        }
        if let Err(e) = rt.pump(FRAME_MS) {
            // A run error has no report channel: log it and ask the front end to shut down.
            eprintln!("quoin: the run ended: {e}");
            core.failed = true;
            if let Some(env) = env {
                unsafe { env(ENV_SHUTDOWN, ptr::null_mut()) };
            }
            return;
        }
        if rt.idle {
            // A pack that closed itself has nothing left to draw; say so once and stop.
            eprintln!(
                "quoin: the pack returned to the shell's idle applet (frame {}, applet {})",
                core.frames, rt.applet
            );
            core.failed = true;
            if let Some(env) = env {
                unsafe { env(ENV_SHUTDOWN, ptr::null_mut()) };
            }
            return;
        }
        // Render this frame's audio now, because `retro_run` is the frame.
        if let (Some(speaker), Some(batch)) = (core.speaker.as_mut(), audio_batch) {
            let frames = (SAMPLE_RATE as usize * FRAME_MS as usize) / 1000;
            if core.left.len() < frames {
                core.left.resize(frames, 0.0);
                core.right.resize(frames, 0.0);
            }
            // `follow` turns the run's playback state into mixer commands.
            speaker.mix(
                rt,
                &mut core.playing,
                &mut core.left[..frames],
                &mut core.right[..frames],
            );
            Speaker::interleave_into(&core.left[..frames], &core.right[..frames], &mut core.pcm);
            // The return value is how many frames the front end consumed. There is nothing to do about
            // an answer that is not `frames`, and saying otherwise would be a lie in a report.
            let consumed = unsafe { batch(core.pcm.as_ptr(), frames) };
            if diag() && core.frames % 30 == 0 {
                let nonzero = core.pcm.iter().filter(|s| **s != 0).count();
                let peak = core
                    .pcm
                    .iter()
                    .map(|s| s.unsigned_abs() as u32)
                    .max()
                    .unwrap_or(0);
                eprintln!(
                    "quoin diag audio frame={} samples={} nonzero={} peak={} consumed={} players={} playing={} sources={} idle={} applet={}",
                    core.frames,
                    core.pcm.len(),
                    nonzero,
                    peak,
                    consumed,
                    rt.players.len(),
                    rt.players.values().filter(|p| p.playing).count(),
                    rt.audio.len(),
                    rt.idle,
                    rt.applet
                );
            }
        }
        let Some(video) = video else {
            return;
        };
        // A still frame must report the same geometry as a fresh one.
        let (width, height) = match core.screen.size() {
            (w, h) if w > 0 && h > 0 => (w, h),
            _ => core.geometry,
        };
        let pitch = pixels_of(width, core.native);
        let presentations = rt.presentations();
        if core.can_dupe && presentations == core.last_presentations {
            // `NULL` is legal because `GET_CAN_DUPE` was answered yes.
            unsafe { video(ptr::null(), width, height, pitch) };
            return;
        }
        match rt.capture(&mut core.screen) {
            Ok(()) => {
                let (width, height) = core.screen.size();
                if width > 0 && height > 0 && (width, height) != core.geometry {
                    core.geometry = (width, height);
                    set_geometry(env, width, height);
                }
                if !core.dumped && core.frames >= 120 {
                    core.dumped = true;
                    dump_frame(&core.screen, width, height);
                }
                if core.native {
                    // Hand over the guest's own `RGB565` words with the screen's stride.
                    core.last_presentations = presentations;
                    unsafe {
                        video(
                            core.screen.pixels().as_ptr() as *const c_void,
                            width,
                            height,
                            core.screen.stride(),
                        )
                    };
                    return;
                }
                // The fallback needs the three-byte screen shape; assert it.
                debug_assert_eq!(core.screen.format(), Format::Rgb888, "the fallback packs RGB888");
                let pitch = pixels_of(width, false);
                let pixels = (width as usize) * (height as usize);
                // Pack only the rows that changed.
                if core.frame.len() != pixels {
                    core.frame.resize(pixels, 0);
                }
                let rgb = core.screen.pixels();
                for (y0, y1) in core.screen.bands() {
                    for y in *y0..*y1 {
                        for x in 0..width as usize {
                            let i = (y as usize) * (width as usize) + x;
                            let b = &rgb[i * 3..i * 3 + 3];
                            core.frame[i] =
                                (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
                        }
                    }
                }
                core.last_presentations = presentations;
                unsafe { video(core.frame.as_ptr() as *const c_void, width, height, pitch) };
            }
            Err(e) => eprintln!("quoin: cannot read the screen: {e}"),
        }
    });
}

/// Stride in bytes per pixel for the active frame format.
fn pixels_of(width: u32, native: bool) -> usize {
    (width as usize) * if native { 2 } else { 4 }
}
