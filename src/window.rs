//! A window to look at the run in, and a keyboard to answer it with.
//!
//! Everything else here is headless on purpose: a scripted session is reproducible to the byte,
//! which is what makes it evidence. This module is the one place a *wall clock* appears, and it is
//! the player's clock rather than the emulator's — it decides only how much virtual time the shell
//! may take between two repaints. The runtime's own clock stays "milliseconds of this run", so
//! `--window` is purely additive: without it, nothing in this file runs.
//!
//! It links `user32` and `gdi32` directly instead of taking a windowing crate, for two reasons. The
//! project's dependency rule is about The platform's, and this is the operating system's own window
//! manager; and a crate that pulls in a renderer to blit one rectangle is a larger thing to trust
//! than the FFI below, which does exactly four jobs — create a window, copy bits, read keys, quit.

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::Duration;

use crate::keymap;
use crate::runtime::{Dispatched, Runtime};
use crate::screen::Format;
use crate::session::Frame;

type Hwnd = *mut c_void;
type Hdc = *mut c_void;
type HBitmap = *mut c_void;
type HInstance = *mut c_void;

const WM_DESTROY: u32 = 0x0002;
const WM_SIZE: u32 = 0x0005;
const WM_PAINT: u32 = 0x000F;
const WM_CLOSE: u32 = 0x0010;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_QUIT: u32 = 0x0012;

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const SW_SHOWNORMAL: i32 = 1;
const SRCCOPY: u32 = 0x00CC_0020;
const COLORONCOLOR: i32 = 3;
/// `MAKEINTATOM(0xBEE1)`: the private window property the one session pointer is hung on.
const SESSION: *const u16 = 0xBEE1usize as *const u16;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const PM_REMOVE: u32 = 1;
/// `MAKEINTRESOURCE(32512)`, which is `IDI_APPLICATION` and `IDC_ARROW` at once. The macro only
/// zero-extends the ordinal — `IS_INTRESOURCE` is "no bits above bit 15" — so shifting it up, which
/// the 16-bit ancestor of this macro did, hands the function a pointer into nowhere instead.
const STANDARD_OBJECT: *const c_void = 32512usize as *const c_void;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    /// `UINT` followed by a pointer, so the compiler inserts the padding MSVC's layout has here.
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: Point,
}

#[repr(C)]
#[allow(non_snake_case)]
struct WndClassExW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, usize, isize) -> usize>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HInstance,
    h_icon: *const c_void,
    h_cursor: *const c_void,
    h_brush: *const c_void,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *const c_void,
}

#[repr(C)]
struct PaintStruct {
    hdc: Hdc,
    f_erase: u32,
    rc_paint: Rect,
    f_restore: u32,
    f_inc_update: u32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
#[allow(non_snake_case)]
struct BitMapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_ppm: i32,
    bi_y_ppm: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct BitMapInfo {
    bmi_header: BitMapInfoHeader,
    bmi_colors: [[u8; 4]; 1],
}

#[link(name = "user32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> HInstance;
    /// Without this, a session at 133% display scaling has its window stretched by the desktop
    /// compositor rather than by us, and pixel art turned to mush by someone else's filter.
    fn SetProcessDPIAware() -> i32;
    fn RegisterClassExW(cls: *const WndClassExW) -> u16;
    #[allow(non_snake_case)]
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *const c_void,
        instance: HInstance,
        param: *const c_void,
    ) -> Hwnd;
    fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
    fn UpdateWindow(hwnd: Hwnd) -> i32;
    fn DestroyWindow(hwnd: Hwnd) -> i32;
    fn PostQuitMessage(code: u32);
    fn DefWindowProcW(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> usize;
    /// The x64 way to hang a pointer off a window: the platform's `GetWindowLongPtrW(hwnd, GWLP_USERDATA)`
    /// is only a macro over these, and only these are actually exported.
    fn SetPropW(hwnd: Hwnd, name: *const u16, data: *mut c_void) -> i32;
    fn GetPropW(hwnd: Hwnd, name: *const u16) -> *mut c_void;
    fn RemovePropW(hwnd: Hwnd, name: *const u16) -> *mut c_void;
    fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32;
    fn BeginPaint(hwnd: Hwnd, ps: *mut PaintStruct) -> Hdc;
    fn EndPaint(hwnd: Hwnd, ps: *const PaintStruct) -> i32;
    fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: i32) -> i32;
    fn PeekMessageW(msg: *mut Msg, hwnd: Hwnd, first: u32, last: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> usize;
    fn LoadCursorW(instance: *const c_void, id: *const c_void) -> *const c_void;
    fn LoadIconW(instance: *const c_void, id: *const c_void) -> *const c_void;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: Hdc) -> Hdc;
    fn CreateDIBSection(
        hdc: Hdc,
        info: *const BitMapInfo,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> HBitmap;
    fn SelectObject(hdc: Hdc, obj: *mut c_void) -> *mut c_void;
    fn DeleteObject(obj: *mut c_void) -> i32;
    fn DeleteDC(hdc: Hdc) -> i32;
    fn StretchBlt(
        dst: Hdc,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: Hdc,
        xs: i32,
        ys: i32,
        ws: i32,
        hs: i32,
        rop: u32,
    ) -> i32;
    fn SetStretchBltMode(hdc: Hdc, mode: i32) -> i32;
}

/// What the window owns: the bits to show, and the keys the player has pressed and the loop has not
/// yet handed to the applet. Deliberately *no* `Runtime` in here — the window procedure must not be
/// able to reach the emulator, because it runs at moments the loop is not expecting it to.
struct Session {
    wide: i32,
    high: i32,
    memory: Hdc,
    bits: *mut c_void,
    /// `Display::presentations` the pixels in `bits` were taken from, so a still screen repaints
    /// nothing and a moving one repaints once.
    painted: u32,
    /// `(AVK, is it going down)` in the order the player did them.
    keys: Vec<(u32, bool)>,
    quit: bool,
}

impl Session {
    /// Copy a frame the guest thread presented into the DIB. `screenshot` is RGB24 top-down and the DIB
    /// is BGR0, which is the whole of the conversion.
    ///
    /// Copy a frame into the window's bitmap, `[b, g, r, a]` per pixel.
    ///
    /// The bytes arrive rather than being read out of a `Runtime`: this thread has not held the machine
    /// since the run moved to [`crate::session::Guest`], and that is the point — the window is free to
    /// pump messages, repaint and be closed while the guest is busy inside a function that does not
    /// return.
    ///
    /// The DIB is 32-bit `BI_RGB`, so the alpha byte is ignored by `SRCCOPY` — it is written as the
    /// frame's own alpha rather than a constant so that an `Rgba8888` frame keeps its meaning if this
    /// surface ever becomes an `AlphaBlend` one.
    ///
    /// The format is matched rather than assumed because [`Frame`] carries it: `session` produces
    /// `Rgba8888` (one swizzle, no widening), a caller feeding their own 8-bit screen gets the same
    /// swizzle from three bytes, and a 565 screen is widened first. All three end at the same four
    /// bytes a pixel, which is all this surface can hold.
    fn upload(&mut self, frame: &Frame) -> Result<(), String> {
        let count = self.wide as usize * self.high as usize;
        if frame.pixels.len() < count * frame.format.bytes_per_pixel() || self.bits.is_null() {
            return Err("the framebuffer is smaller than the window's bitmap".into());
        }
        let dst = self.bits as *mut [u8; 4];
        match frame.format {
            Format::Rgba8888 => {
                for i in 0..count {
                    let s = &frame.pixels[i * 4..i * 4 + 4];
                    unsafe { *dst.add(i) = [s[2], s[1], s[0], s[3]] };
                }
            }
            Format::Rgb888 => {
                for i in 0..count {
                    let s = &frame.pixels[i * 3..i * 3 + 3];
                    unsafe { *dst.add(i) = [s[2], s[1], s[0], 255] };
                }
            }
            Format::Rgb565 => {
                // Only reachable from a caller that asked the session for 5-6-5 — this window asks
                // for `Rgba8888` — but the frame carries its own format, so the arm has to exist and
                // has to be right. Widened into a scratch buffer rather than unrolled here, so the
                // conversion has one home.
                let mut wide = vec![0u8; count * 4];
                crate::screen::expand565(&frame.pixels[..count * 2], &mut wide);
                for i in 0..count {
                    let s = &wide[i * 4..i * 4 + 4];
                    unsafe { *dst.add(i) = [s[2], s[1], s[0], s[3]] };
                }
            }
        }
        self.painted = frame.presentations;
        Ok(())
    }
}

/// The one way in and out of the heap the window procedure shares with the loop. Sound because every
/// call is on the thread that owns the box, and no two uses of it are ever live at once: the loop
/// finishes with the session before it dispatches a message, and the procedure is finished before
/// the loop takes it up again. `None` until the loop has hung the session on the window, which is
/// after the window exists and before it can be seen, painted or typed into.
unsafe fn session(hwnd: Hwnd) -> Option<&'static mut Session> {
    let held = GetPropW(hwnd, SESSION);
    (!held.is_null()).then(|| &mut *(held as *mut Session))
}

unsafe extern "system" fn wndproc(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> usize {
    let Some(s) = session(hwnd) else {
        return DefWindowProcW(hwnd, msg, w, l);
    };
    match msg {
        WM_KEYDOWN => {
            // Bit 30 is the autorepeat flag. A held key on a handset sends its press once and then
            // repeats as `EVT_KEY`, which is a different thing from a new press, so only the first
            // one is forwarded and the repeats are left to the default handler.
            let repeat = l & 0x4000_0000 != 0;
            if !repeat {
                if let Some(code) = keymap::physical(w) {
                    s.keys.push((code, true));
                }
            }
            if w == keymap::QUIT {
                s.quit = true;
                PostQuitMessage(0);
            }
            0
        }
        // The other half of a key: the handset reports when it comes up, and an app that polls
        // which keys are held only sees the hold if the release is a separate moment.
        WM_KEYUP => {
            if let Some(code) = keymap::physical(w) {
                s.keys.push((code, false));
            }
            0
        }
        // Swallowed, so that Enter and the arrow keys do not reach the dialog manager's beep.
        WM_CHAR => 0,
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            InvalidateRect(hwnd, null_mut(), 0);
            0
        }
        WM_PAINT => {
            let mut ps = std::mem::zeroed::<PaintStruct>();
            let dst = BeginPaint(hwnd, &mut ps);
            let mut client = std::mem::zeroed::<Rect>();
            GetClientRect(hwnd, &mut client);
            SetStretchBltMode(dst, COLORONCOLOR);
            StretchBlt(
                dst,
                0,
                0,
                client.right,
                client.bottom,
                s.memory,
                0,
                0,
                s.wide,
                s.high,
                SRCCOPY,
            );
            EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE | WM_DESTROY => {
            s.quit = true;
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Show `rt` in a window and drive it from the keyboard, until the window is closed.
///
/// `title` names the pack, because the title bar is the only place a player can look to find out
/// what they are watching and which keys mean what. `soundfont` is the bank the run's clips are
/// rendered through, or `None` for the built-in tables — the same choice the egui window is given, so
/// that `--soundfont` means the same thing to both and to neither is a silent no-op.
pub fn show(
    rt: Runtime,
    title: &str,
    soundfont: Option<&str>,
    audio_backend: crate::audio::AudioBackend,
    control: Option<crate::control::Control>,
) -> crate::session::Finished {
    // Every way this file can fail to make a window hands the machine back with the reason, and with
    // `End::Finished` rather than a cancel: nobody asked the run to stop, so a report that said the
    // close box ended it would be describing something that never happened.
    let no_window = |rt: Runtime, why_not: &str| crate::session::Finished {
        rt,
        pumped: Dispatched::default(),
        error: Some(why_not.to_string()),
        why: crate::session::End::Finished,
    };
    let (wide_img, high_img) = (i32::from(rt.device.width), i32::from(rt.device.height));
    // An integer multiple, never a fraction: a 176x220 pack is unreadable at 1:1 on a modern panel,
    // and anything that is not an integer multiple smears the art. The box is a guess at a comfortable
    // window rather than a measured limit — the player can resize it, which stretches to fit.
    let scale = (1024 / wide_img.max(1))
        .min(768 / high_img.max(1))
        .clamp(1, 4);
    let caption = wide(&format!("{title}{}", keymap::HINT));
    let class = wide("quoinHleWindow");

    let instance = unsafe {
        // Before any handle or rectangle exists, so every coordinate below is a physical pixel.
        SetProcessDPIAware();
        GetModuleHandleW(null_mut())
    };
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: wide_img * scale,
        bottom: high_img * scale,
    };
    unsafe {
        AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
        let cls = WndClassExW {
            cb_size: std::mem::size_of::<WndClassExW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(wndproc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: LoadIconW(null_mut(), STANDARD_OBJECT),
            h_cursor: LoadCursorW(null_mut(), STANDARD_OBJECT),
            // The stock black brush, `GetStockObject(BLACK_BRUSH)`, is one more import to avoid:
            // the client is fully painted before anything could show through.
            h_brush: null_mut(),
            lpsz_menu_name: null_mut(),
            lpsz_class_name: class.as_ptr(),
            h_icon_sm: null_mut(),
        };
        if RegisterClassExW(&cls) == 0 {
            return no_window(rt, "RegisterClassExW failed");
        }
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            caption.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            // CW_USEDEFAULT for a positioned window is -4 for x and -5 for y, and only x takes it.
            -4,
            -4,
            rect.right - rect.left,
            rect.bottom - rect.top,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        if hwnd.is_null() {
            return no_window(rt, "CreateWindowExW failed");
        }

        // The DIB is created against the screen DC and then lives in its own memory DC, so nothing
        // about the window's depth can change what the pixels mean.
        let screen = CreateCompatibleDC(null_mut());
        let info = BitMapInfo {
            bmi_header: BitMapInfoHeader {
                bi_size: std::mem::size_of::<BitMapInfoHeader>() as u32,
                bi_width: wide_img,
                // Negative height is the top-down convention, and the framebuffer is top-down.
                bi_height: -high_img,
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_ppm: 0,
                bi_y_ppm: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [[0; 4]; 1],
        };
        let mut bits: *mut c_void = null_mut();
        let bitmap = CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() || bits.is_null() {
            DestroyWindow(hwnd);
            return no_window(rt, "CreateDIBSection failed");
        }
        SelectObject(screen, bitmap);

        let held = Box::new(Session {
            wide: wide_img,
            high: high_img,
            memory: screen,
            bits,
            painted: u32::MAX,
            keys: Vec::new(),
            quit: false,
        });
        let owned = Box::into_raw(held);
        // Before it is shown, so the procedure cannot be asked anything it does not yet have.
        SetPropW(hwnd, SESSION, owned as *mut c_void);

        ShowWindow(hwnd, SW_SHOWNORMAL);
        UpdateWindow(hwnd);

        // The machine goes to its own thread here, after every fallible Windows call above: an early
        // failure hands the caller back the `Runtime` it was given, and only a run that got as far as a
        // window has a thread to join.
        let guest = crate::session::Guest::start_with_backend(
            rt,
            crate::session::Sound::named(soundfont),
            audio_backend,
            control,
            crate::screen::Format::Rgba8888,
        );
        let shared = Arc::clone(&guest.shared);
        // What the DIB is before the guest thread's first turn lands: black, for one frame. The
        // single-threaded loop painted the booted framebuffer before showing the window at all, and
        // painting it here would mean this thread holding the machine — which is the thing that made a
        // stuck pack unkillable.
        let mut failure: Option<String> = None;
        let mut msg = std::mem::zeroed::<Msg>();
        loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let s = &mut *owned;
            if s.quit {
                // The close box, `WM_QUIT`, or the key that means leave. This is the ask the whole
                // restructure exists for: it is acted on by a thread that is not inside the guest.
                shared.cancel();
                break;
            }
            // Keys enter through the same door a scripted `--key` does, so an applet that answers a key
            // by arming a timer behaves here exactly as it does in a recorded session. They go into the
            // inbox; the guest thread posts them on its next turn, as the other front end does.
            for (code, down) in s.keys.drain(..) {
                if down {
                    shared.press(code);
                } else {
                    shared.release(code);
                }
            }
            let mut painted = false;
            if let Some(frame) = shared.take_frame() {
                if let Err(e) = s.upload(&frame) {
                    failure = Some(e);
                    break;
                }
                InvalidateRect(hwnd, null_mut(), 0);
                painted = true;
            }
            // A still screen has nothing new to paint; calling `UpdateWindow` every 2 ms makes the
            // window thread re-enter the paint path without new pixels and steals time from the
            // guest. The guest-side clock already owns the frame cadence.
            if painted {
                UpdateWindow(hwnd);
            }
            // The run ending on its own — returned, faulted, cancelled, or quit over `--serve` — is what
            // ends this loop now. Nothing here asks the guest anything, so the answer is read rather
            // than waited on.
            if shared.status().why != crate::session::End::Running {
                break;
            }
            // Unconditional, because an app with nothing armed would otherwise have this loop spinning
            // at full speed; two milliseconds is far below the frame time anything here can show.
            std::thread::sleep(Duration::from_millis(2));
        }
        RemovePropW(hwnd, SESSION);
        let _ = Box::from_raw(owned);
        DeleteObject(bitmap);
        DeleteDC(screen);
        DestroyWindow(hwnd);
        let mut finished = guest.finish();
        // A window that failed to paint says so over a run that ended cleanly; both still hand the
        // machine back, because the report is read out of it.
        if let Some(why_not) = failure {
            // And a window this file could not paint is this file's failure, not the person's: the
            // worker would otherwise report `Cancelled`, because `Guest::finish` is what asked it to
            // stop. See [`Finished::why`].
            finished.error = Some(why_not);
            finished.why = crate::session::End::Finished;
        }
        finished
    }
}
