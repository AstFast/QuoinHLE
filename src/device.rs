//! The device profile: what this emulated handset says about itself.
//!
//! A BREW module never reads a config file. It learns the screen size, the colour depth, the
//! close-key code and the heap budget from `IShell`, so this module is where those numbers live
//! and the only place that knows how to put them into a guest `AEEDeviceInfo`. Every byte position
//! comes from [`crate::abi`]'s generated tables rather than from a literal, which is what keeps a
//! 64-byte struct ending in a nine-bit bitfield run honest after a header change.
//!
//! The default profile uses the values the Melange front end shipped with (240x320, RGB565, the
//! `AVK_*` keypad, a 64 MiB advertised heap) because those are the numbers the sample applets were
//! tuned against. None of them are ABI: they are choices, and a real target would read its own.

use crate::abi::{self, generated as g, StructInfo};
use crate::hle::HResult;
use crate::mem::Mem;

/// `#define RGB_NONE (0xffffffffL)` from the header. It has two documented meanings:
/// `IDisplay_SetColor` reads it as "give me the current value and change nothing", and
/// `IDisplay_DrawRect` reads it as "use the active colour for this part" — which is how
/// `IDisplay_ClearScreen` can pass it as the fill colour and still clear the screen.
/// It can never collide with a real colour, because `MAKE_RGB` leaves the alpha byte zero.
pub const RGB_NONE: u32 = 0xffff_ffff;

/// `MAKE_RGB(r, g, b)` from the same header. BREW's `RGBVAL` is *not* the usual `0x00RRGGBB`:
/// red occupies bits 8..15, green 16..23, blue 24..31, and the low byte is the alpha slot
/// `MAKE_RGBA` fills. Reading a captured screen back as if it were `0xRRGGBB` rotates the
/// channels, so the packing lives here and nowhere else.
pub const fn make_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 8) | ((g as u32) << 16) | ((b as u32) << 24)
}

/// Inverse of [`make_rgb`]. The low byte is dropped: it is alpha, and the display has none.
pub const fn split_rgb(v: u32) -> (u8, u8, u8) {
    (
        ((v >> 8) & 0xff) as u8,
        ((v >> 16) & 0xff) as u8,
        ((v >> 24) & 0xff) as u8,
    )
}

pub const RGB_BLACK: u32 = 0;
pub const RGB_WHITE: u32 = make_rgb(255, 255, 255);
/// `RGB_MASK_COLOR`: magenta, the transparent colour of a colour display's bitmaps. Monochrome
/// and greyscale devices use white instead.
pub const RGB_MASK_COLOR: u32 = make_rgb(255, 0, 255);

/// The nine `flg b*:1` members of `AEEDeviceInfo`, held as named booleans because which bit each
/// one occupies is table data, not something to hand-transcribe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceFlags {
    /// An alternate display (a pager), which is what makes `cxAltScreen`/`cyAltScreen` mean
    /// anything.
    pub alt_display: bool,
    /// Sends `EVT_FLIP` when the clamshell opens or closes.
    pub flip: bool,
    pub vibrator: bool,
    pub ext_speaker: bool,
    pub voice_recognition: bool,
    pub position_location: bool,
    pub midi: bool,
    pub cmx: bool,
    pub pen: bool,
}

/// Reads one `b*:1` flag out of a [`DeviceFlags`].
type Flag = fn(&DeviceFlags) -> bool;

impl DeviceFlags {
    /// `(AEEDeviceInfo member, is it set)`, in the order the struct declares them, so the
    /// packing run is walked the way the compiler laid it out.
    const MEMBERS: &'static [(&'static str, Flag)] = &[
        ("bAltDisplay", |f| f.alt_display),
        ("bFlip", |f| f.flip),
        ("bVibrator", |f| f.vibrator),
        ("bExtSpeaker", |f| f.ext_speaker),
        ("bVR", |f| f.voice_recognition),
        ("bPosLoc", |f| f.position_location),
        ("bMIDI", |f| f.midi),
        ("bCMX", |f| f.cmx),
        ("bPen", |f| f.pen),
    ];
}

/// Everything this handset will answer through `IShell::GetDeviceInfo` and `GetDeviceInfoEx`.
#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub width: u16,
    pub height: u16,
    /// Bits per pixel of the device bitmap. `AEEDeviceInfo` documents `nColorDepth` as
    /// "1 = mono, 2 = grey, etc.", i.e. the same quantity, so one field answers both it and
    /// `AEEBitmapInfo::nDepth` rather than letting the two disagree.
    pub color_depth: u16,
    pub alt_width: u16,
    pub alt_height: u16,
    /// Width of the standard scroll bar, in pixels.
    pub scroll_bar: u16,
    /// `AEE_ENC_*`: the character set `AECHAR` values are in.
    pub encoding: u16,
    /// Milliseconds per step of menu text auto-scroll.
    pub menu_text_scroll: u16,
    /// Milliseconds an icon-view image is held when `MP_ICON_DELAY_IMAGE` is set.
    pub menu_image_delay: u32,
    /// `dwRAM`: the size of the BREW heap a module is told it has to live in.
    pub ram: u32,
    /// Default prompt properties.
    pub prompt_props: u32,
    pub key_close_app: u16,
    pub key_close_all_apps: u16,
    /// `AEE_LNG_*`, which the resource loader uses to pick a language block.
    pub lang: u32,
    pub net_linger: u32,
    pub sleep_defer: u32,
    pub max_path: u16,
    pub platform_id: u32,
    pub flags: DeviceFlags,
    /// Identity strings, each reported through the `AEE_DEVICEITEM_*` documented for it.
    ///
    /// Owned rather than `&'static str`, which they were until [`crate::device_file`] gave them a
    /// source other than this file. A profile loaded from JSON carries text whose lifetime is the
    /// load, and the alternative — leaking it — would make "where does this string live" a question
    /// with two answers depending on how the run was started.
    pub chip_id: String,
    pub model_name: String,
    pub manufacturer: String,
    pub friendly_name: String,
    pub firmware_version: String,
    pub carrier_id: u32,
    pub esn: u32,
    /// Reported as a null-terminated ASCII string, 15 digits plus the terminator.
    pub imei: String,
    pub softkey_count: u8,
    /// One of `AEE_MANNER_MODE_*`.
    pub manner_mode: u32,
    /// `AEEREPEAT_SCROLLBAR` defaults: first repeat delay and subsequent rate, in ms.
    pub repeat_start: u32,
    pub repeat_rate: u32,
    /// the header's `KB_AUTOREPEAT_START`: how long after the first `EVT_KEY` a key that is
    /// still down starts repeating, in ms.
    pub key_repeat_start: u32,
    /// `KB_AUTOREPEAT_RATE`: the interval of those repeats, in ms. Each one arrives as `EVT_KEY`
    /// with `KB_AUTOREPEAT` set, which is how an app tells a held key from a fresh press. Not the
    /// pair above, which is a pen held beside a scrollbar thumb.
    pub key_repeat_rate: u32,
    pub storage_total: u64,
    pub storage_free: u64,
    /// The key codes `AEE_DEVICEITEM_KEY_SUPPORT` answers `true` for: this is the emulated
    /// keypad, and a module that probes before drawing softkey labels reads it.
    pub keys: Vec<u16>,
}

impl Default for DeviceProfile {
    fn default() -> Self {
        DeviceProfile::standard()
    }
}

impl DeviceProfile {
    /// A 240x320 RGB565 candybar handset.
    pub fn standard() -> Self {
        DeviceProfile {
            width: 240,
            height: 320,
            // The depth of the screen this handset actually hands out, because a module is entitled to
            // write that screen without asking again. `IDisplay::GetDestination` returns an `IDIB` that
            // says 16 and `IDIB_COLORSCHEME_565`; if this word disagrees, the two answers are not one
            // device, and a pack that composites straight into `pBmp` trusts this one instead and lays
            // three bytes where the host reads two. Melange's `OEM_GetDeviceInfo` answers 24, but its
            // own screen is a 32-bit Android surface, so on that host the two answers do agree.
            color_depth: 16,
            alt_width: 0,
            alt_height: 0,
            scroll_bar: 8,
            encoding: 253,
            menu_text_scroll: 200,
            menu_image_delay: 1000,
            // Melange advertised 64 MiB. The arena the loader actually maps is far smaller, and a
            // module only uses this number to decide how much to preallocate.
            ram: 64 * 1024 * 1024,
            prompt_props: 0,
            key_close_app: g::AVK_END as u16,
            key_close_all_apps: g::AVK_END as u16,
            lang: 0,
            net_linger: 0,
            sleep_defer: 0,
            max_path: 255,
            platform_id: 0,
            flags: DeviceFlags {
                vibrator: true,
                ext_speaker: true,
                // A pen, which the handset in melange's the delivered body reports and which a pack may
                // branch its layout on: a pack may lay its About text out against the panel when this
                // is set and against the whole screen when it is not.
                pen: true,
                ..DeviceFlags::default()
            },
            chip_id: "MSM7201A".to_string(),
            model_name: "QuoinHLE".to_string(),
            manufacturer: "QuoinHLE".to_string(),
            friendly_name: "QuoinHLE".to_string(),
            firmware_version: "1.0".to_string(),
            carrier_id: 0,
            esn: 0xab2b_3c4f,
            imei: "123456781234567".to_string(),
            softkey_count: 2,
            manner_mode: g::AEE_MANNER_MODE_NORMAL,
            repeat_start: 200,
            repeat_rate: 75,
            key_repeat_start: 1500,
            key_repeat_rate: 100,
            storage_total: 512 * 1024 * 1024,
            storage_free: 256 * 1024 * 1024,
            keys: KEYS.to_vec(),
        }
    }

    /// The same handset at another screen size.
    ///
    /// A shipped app is built for one screen and sizes its own buffers from what `IShell` reports:
    /// the `VX8300-176x220` pack of a real game indexes a structure at `176 * 128` bytes, which only
    /// its own width explains, and answering `240x320` for it takes a branch that leaves that
    /// pointer null. Only the screen changes here, so a run that selects a size is still being told
    /// the truth about everything else the handset claims.
    pub fn with_screen(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Override what the handset **says** about its colour depth and its `AECHAR` encoding, either of
    /// which a pack can read to choose a code path.
    ///
    /// This is a diagnostic and it deliberately makes an incoherent device: the frame this host hands
    /// out stays RGB565 (`display::NATIVE_DEPTH`, which is what `AEEBitmapInfo::nDepth` keeps
    /// reporting), and the text helpers stay the ones implemented here. Asking a pack what it does with
    /// a different answer is the point; running a 24 bpp or Shift-JIS device is not, and a run that
    /// draws wrong under it has told us the pack's path, not this host's pixels.
    pub fn with_advertised(mut self, color_depth: Option<u16>, encoding: Option<u16>) -> Self {
        if let Some(d) = color_depth {
            self.color_depth = d;
        }
        if let Some(e) = encoding {
            self.encoding = e;
        }
        self
    }

    /// Answer `ISHELL_GetDeviceInfo(pShell, pInfo)`.
    ///
    /// The slot returns void, so a module cannot be told it got a short answer — which is exactly why
    /// the negotiation has to be read correctly. The header puts `wStructSize` at the end of the
    /// fields a handset always reports and marks only what follows it — `dwNetLinger`,
    /// `dwSleepDefer`, `wMaxPath`, `dwPlatformID` — as needing the caller's declared size. So the
    /// prefix through `dwNetLinger` is written unconditionally and only the tail is gated; melange's
    /// OEM routine is the same, filling every field and never consulting the caller's word.
    ///
    /// Treating that word as a limit on the *whole* struct is a real defect, not a corner case:
    /// nothing obliges a module to initialise a struct whose fields are all outputs, so a stale word
    /// left in that stack slot silently shortens the write, and the caller never sees the fields past
    /// it.
    pub fn write_device_info(&self, mem: &mut Mem, pi: u32) -> HResult<()> {
        if pi == 0 {
            return Ok(());
        }
        let bytes = self.device_info_bytes();
        let st = info_struct();
        let declared = mem.read_u16(pi + field(st, "wStructSize") as u32)? as usize;
        let always = field(st, "dwNetLinger");
        let n = declared.max(always).min(bytes.len());
        mem.write_bytes(pi, &bytes[..n])?;
        Ok(())
    }

    /// A complete `AEEDeviceInfo`, laid out from the generated field table.
    pub fn device_info_bytes(&self) -> Vec<u8> {
        let st = info_struct();
        let mut b = vec![0u8; st.size as usize];
        put16(st, &mut b, "cxScreen", self.width);
        put16(st, &mut b, "cyScreen", self.height);
        put16(st, &mut b, "cxAltScreen", self.alt_width);
        put16(st, &mut b, "cyAltScreen", self.alt_height);
        put16(st, &mut b, "cxScrollBar", self.scroll_bar);
        put16(st, &mut b, "wEncoding", self.encoding);
        put16(st, &mut b, "wMenuTextScroll", self.menu_text_scroll);
        put16(st, &mut b, "nColorDepth", self.color_depth);
        put32(st, &mut b, "wMenuImageDelay", self.menu_image_delay);
        put32(st, &mut b, "dwRAM", self.ram);
        for (name, set) in DeviceFlags::MEMBERS {
            if set(&self.flags) {
                set_bit(st, &mut b, name);
            }
        }
        put32(st, &mut b, "dwPromptProps", self.prompt_props);
        put16(st, &mut b, "wKeyCloseApp", self.key_close_app);
        put16(st, &mut b, "wKeyCloseAllApps", self.key_close_all_apps);
        put32(st, &mut b, "dwLang", self.lang);
        put16(st, &mut b, "wStructSize", st.size as u16);
        put32(st, &mut b, "dwNetLinger", self.net_linger);
        put32(st, &mut b, "dwSleepDefer", self.sleep_defer);
        put16(st, &mut b, "wMaxPath", self.max_path);
        put32(st, &mut b, "dwPlatformID", self.platform_id);
        b
    }

    /// The bytes an `AEE_DEVICEITEM_*` answer occupies, or `None` when this handset does not
    /// report that item — which becomes `AEE_EUNSUPPORTED`, the documented answer for an item the
    /// device does not support.
    ///
    /// Only items whose payload the published header documents are answered here; the rest of the
    /// `AEE_DEVICEITEM_*` list has no width in the header, and guessing one would be a silent
    /// ABI claim.
    ///
    /// `pbuff` is *read* as well as written, because key support and auto-repeat take their
    /// selector inside the structure they return.
    pub fn device_item(&self, mem: &Mem, item: u32, pbuff: u32) -> HResult<Option<Vec<u8>>> {
        let read_in = |off: usize, width: usize| -> HResult<u32> {
            if pbuff == 0 {
                return Ok(0);
            }
            let a = pbuff + off as u32;
            Ok(match width {
                1 => mem.read_u8(a)? as u32,
                2 => mem.read_u16(a)?,
                _ => mem.read_u32(a)?,
            })
        };
        let bytes = match item {
            g::AEE_DEVICEITEM_ESN => self.esn.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_CARRIER_ID => self.carrier_id.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_MANNER_MODE => self.manner_mode.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_SOFTKEY_COUNT => vec![self.softkey_count],
            g::AEE_DEVICEITEM_CHIP_ID => utf16(&self.chip_id),
            g::AEE_DEVICEITEM_MODEL_NAME => utf16(&self.model_name),
            g::AEE_DEVICEITEM_MANUFACTURER_NAME => utf16(&self.manufacturer),
            g::AEE_DEVICEITEM_FRIENDLY_NAME => utf16(&self.friendly_name),
            g::AEE_DEVICEITEM_IMEI => ascii0(&self.imei),
            g::AEE_DEVICEITEM_FIRMWARE_VERSION => ascii0(&self.firmware_version),
            g::AEE_DEVICEITEM_DISPINFO1 => self.bitmap_info(),
            g::AEE_DEVICEITEM_KEY_SUPPORT => {
                let st = table("KeySupportType");
                let key = read_in(field(st, "key"), 2)? as u16;
                let mut b = vec![0u8; st.size as usize];
                put16(st, &mut b, "key", key);
                b[field(st, "supported")] = u8::from(self.keys.contains(&key));
                b
            }
            g::AEE_DEVICEITEM_REPEAT_TIME => {
                let st = table("AEEAutoRepeat");
                let sel = read_in(field(st, "wItem"), 2)? as u16;
                let mut b = vec![0u8; st.size as usize];
                put16(st, &mut b, "wItem", sel);
                put32(st, &mut b, "dwStart", self.repeat_start);
                put32(st, &mut b, "dwRate", self.repeat_rate);
                b
            }
            g::AEE_DEVICEITEM_FSSPACE_ROOT | g::AEE_DEVICEITEM_FSSPACE_CARD0 => {
                let st = table("AEEFSFreeSpace");
                let mut b = vec![0u8; st.size as usize];
                b[..8].copy_from_slice(&self.storage_total.to_le_bytes());
                b[8..].copy_from_slice(&self.storage_free.to_le_bytes());
                b
            }
            _ => return Ok(None),
        };
        Ok(Some(bytes))
    }

    fn bitmap_info(&self) -> Vec<u8> {
        let st = table("AEEBitmapInfo");
        let mut b = vec![0u8; st.size as usize];
        put32(st, &mut b, "cx", u32::from(self.width));
        put32(st, &mut b, "cy", u32::from(self.height));
        // `AEEBitmapInfo.nDepth` is the depth of the bitmap the item *is*, i.e. this host's own frame,
        // and `AEEDeviceInfo::nColorDepth` reports the same screen through the same number for the same
        // reason: a module that composites into `pBmp` has no way to reconcile two answers.
        put32(
            st,
            &mut b,
            "nDepth",
            u32::from(crate::display::NATIVE_DEPTH),
        );
        b
    }
}

/// The documented size negotiation of `ISHELL_GetDeviceInfoEx(pShell, nItem, pBuff, pnSize)`, which
/// `IAppletCtl_GetRunningInfo` spells the same way.
///
/// `pnSize` is in/out: on entry it is the buffer's capacity, on exit either the number of bytes
/// written or the number that *would* have been needed. A null `pBuff` is a legitimate query for
/// that size, which is how a module allocates for a variable-length item. The cell is a 32-bit `int`
/// here — unlike `IShell::GetProperty`'s `pwSize`, which is a `uint16`, so the two doors next to each
/// in the same header do not share a width and must not share an accessor.
///
/// ⚠ The shipped door does not refuse a small buffer: `CShell_GetDeviceInfoEx` copies
/// `min(capacity, need)` and answers SUCCESS, and on the string items that copy is a `strncpy` of the
/// truncated length, so the caller can be handed a buffer with no terminator. This host answers
/// `EBUFFERTOOSMALL` and writes nothing — a refusal the caller can act on, rather than a value it
/// would read as complete.
pub fn put_sized_item(mem: &mut Mem, bytes: &[u8], pbuff: u32, pnsize: u32) -> HResult<u32> {
    if pnsize == 0 {
        return Ok(g::AEE_EBADPARM);
    }
    let need = bytes.len() as u32;
    let have = mem.read_u32(pnsize)?;
    mem.write_u32(pnsize, need)?;
    if pbuff == 0 {
        return Ok(g::AEE_SUCCESS);
    }
    if have < need {
        return Ok(g::AEE_EBUFFERTOOSMALL);
    }
    mem.write_bytes(pbuff, bytes)?;
    Ok(g::AEE_SUCCESS)
}

fn info_struct() -> &'static StructInfo {
    table("AEEDeviceInfo")
}

/// A named struct from the generated tables. Panicking means this file asked for a field that
/// does not exist — a bug in the host, never a guest condition, and every name used here is
/// covered by [`every_table_field_the_profile_writes_exists`].
fn table(name: &str) -> &'static StructInfo {
    match abi::struct_info(name) {
        Some(st) => st,
        None => panic!("{name} is not in the ABI tables"),
    }
}

fn field(st: &StructInfo, name: &str) -> usize {
    for f in st.fields {
        if f.name == name {
            return f.off as usize;
        }
    }
    panic!("{} has no field {name}", st.name)
}

fn put16(st: &StructInfo, b: &mut [u8], name: &str, v: u16) {
    let o = field(st, name);
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn put32(st: &StructInfo, b: &mut [u8], name: &str, v: u32) {
    let o = field(st, name);
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// Set one bit of a bitfield run at its table position. A run packs LSB-first into a unit whose
/// size is the *declared* type's size, little-endian — the rule a compiler probe measured
/// with the compiler class that built the samples.
fn set_bit(st: &StructInfo, b: &mut [u8], name: &str) {
    for f in st.fields {
        if f.name == name && f.bits != 0 {
            let (o, unit) = (f.off as usize, f.size as usize);
            let mut u = [0u8; 4];
            u[..unit].copy_from_slice(&b[o..o + unit]);
            let mut v = u32::from_le_bytes(u);
            v |= 1 << f.bit_off;
            b[o..o + unit].copy_from_slice(&v.to_le_bytes()[..unit]);
            return;
        }
    }
    panic!("{} has no bitfield {name}", st.name);
}

/// An item documented as an `AECHAR` string: UTF-16 code units, null terminated.
fn utf16(s: &str) -> Vec<u8> {
    let mut b = Vec::with_capacity(s.len() * 2 + 2);
    for u in s.encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    b.extend_from_slice(&0u16.to_le_bytes());
    b
}

/// An item documented as a null-terminated ASCII string.
fn ascii0(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    b
}

/// The keypad: digits, call keys, the pad, and the two softkeys. The header defines the codes
/// themselves, so this is a choice of *which* keys exist, not of values.
static KEYS: [u16; 23] = [
    g::AVK_0 as u16,
    g::AVK_1 as u16,
    g::AVK_2 as u16,
    g::AVK_3 as u16,
    g::AVK_4 as u16,
    g::AVK_5 as u16,
    g::AVK_6 as u16,
    g::AVK_7 as u16,
    g::AVK_8 as u16,
    g::AVK_9 as u16,
    g::AVK_STAR as u16,
    g::AVK_POUND as u16,
    g::AVK_END as u16,
    g::AVK_SEND as u16,
    g::AVK_CLR as u16,
    g::AVK_UP as u16,
    g::AVK_DOWN as u16,
    g::AVK_LEFT as u16,
    g::AVK_RIGHT as u16,
    g::AVK_SELECT as u16,
    g::AVK_SOFT1 as u16,
    g::AVK_SOFT2 as u16,
    g::AVK_MENU as u16,
];

#[cfg(test)]
#[path = "test/device.rs"]
mod tests;
