//! The framebuffer and the host `IDisplay` / `IBitmap` objects.
//!
//! Pixels live in *guest* memory. That is the whole trick: `IDisplay_GetDeviceBitmap` can hand the
//! module a real `IBitmap*` whose bits the module may reach through `IBitmap_GetPixels`-style
//! calls, the emulated screen and a module's off-screen bitmap are the same kind of object, and a
//! dump of the guest heap *is* a dump of the screen. Nothing is mirrored on the host side.
//!
//! Host-side state is only what a handset keeps behind the vtable — the current destination
//! bitmap, the clip rectangle, the colour table — and it is keyed by the guest object address. The
//! structs here are small enough to clone in and write back, which is what keeps a draw operation
//! from holding a borrow of the table it is also mutating.
//!
//! Every method in [`crate::abi`]'s `IDisplay`/`IBitmap` vtable is either implemented here or
//! reaches `Unimplemented`, with one exception the ABI forces: `DrawRect`, `BitBlt` and `Update`
//! return *void*, so a request this host cannot honour has nowhere to complain to. Those go into
//! [`Display::quiet`] instead of being silently dropped.

use std::collections::HashMap;
use std::sync::Arc;

use crate::abi::{self, generated as g};
use crate::device::{make_rgb, split_rgb, RGB_BLACK, RGB_NONE, RGB_WHITE};
use crate::encoding::TextEncoding;
use crate::font::{self, Face, Faces, GLYPH_W};
use crate::hle::{Error as HleError, HResult, Hle};
use crate::mem::Mem;

/// `IDIB_COLORSCHEME_565`: 5 red, 6 green, 5 blue. The bit pack is the ARM/embedded convention
/// (`R<<11 | G<<5 | B`), which is what the reference front end uploads verbatim as
/// `GL_UNSIGNED_SHORT_5_6_5`. The header only calls `nColorDepth` the bits per pixel, so this is
/// the one layout claim here that is a choice rather than a header fact — and the only one that
/// has to match whatever reads the buffer.
pub const fn pack565(r: u8, g: u8, b: u8) -> u16 {
    (((r >> 3) as u16) << 11) | (((g >> 2) as u16) << 5) | ((b >> 3) as u16)
}

/// The inverse, expanded back to 8 bits per channel with the low bits replicated so white stays
/// white.
pub const fn unpack565(v: u16) -> (u8, u8, u8) {
    let r = (v >> 11) as u8;
    let g = (v >> 5) & 0x3f;
    let b = (v & 0x1f) as u8;
    (r << 3 | r >> 2, (g << 2 | g >> 4) as u8, b << 3 | b >> 2)
}

/// The depth the device's own framebuffer uses: one `u16` per pixel, the RGB565 the packers above
/// assume. A profile may back a 32-bpp frame instead (`screen.frame_bpp`); this stays the fast
/// path's format and the answer for a handset that names no other.
pub const NATIVE_DEPTH: u16 = 16;

/// Where `DIB_NewEx` puts pixel (0, 0) inside the object it allocates, and the default this host
/// publishes: the delivered body allocates `pixel_bytes + 0x6B` and points `pBmp` just past that
/// record. A firmware whose header is a different size names the offset in its profile.
pub const DIB_PIXEL_OFFSET: u32 = 0x6B;

/// The first byte after `IDIB`'s own fields (`reserved` ends at 36). A pixel offset below this
/// would put pixels inside the record a pack reads through.
pub const MIN_DIB_PIXEL_OFFSET: u32 = 36;

/// The largest screen `IDisplay::SetPrefs` will resize the device bitmap to, in pixels.
///
/// This is a ceiling, not a shape: `w`/`h` arrive as untrusted text from the guest, and the pixels
/// are allocated from the same arena the rest of the host lives in, so a settings string asking for
/// 65535x65535 must be answered rather than attempted. 4096 square is a panel no BREW handset ever
/// had, and past it the honest reply is that the display cannot accommodate the request.
pub const MAX_SCREEN_PIXELS: u32 = 4096 * 4096;

/// The `IDIB_COLORSCHEME_*` word for a bitmap of this depth — what a device bitmap of it reports in
/// `nDepth`'s high half, and what `IDisplay::SetPrefs`'s `c` name asks for.
pub const fn device_scheme(depth: u16) -> u32 {
    if is_888(depth) {
        g::IDIB_COLORSCHEME_888
    } else {
        g::IDIB_COLORSCHEME_565
    }
}

/// What an index in an 8-bpp bitmap means *when the bitmap carries no palette of its own*: three bits
/// of red, three of green, two of blue, the packing an eight-bit handset display had without loading a
/// table. The header publishes no default table to copy, so this is what `default_palette` seeds
/// `IDIB::pRGB` with — and a pack that decodes its own palette overwrites these entries.
pub const fn index8_to_native(i: u8) -> u32 {
    let r = ((i >> 5) & 0x07) as u32 * 255 / 7;
    let g = ((i >> 2) & 0x07) as u32 * 255 / 7;
    let b = (i & 0x03) as u32 * 255 / 3;
    pack565(r as u8, g as u8, b as u8) as u32
}

/// The inverse of [`index8_to_native`], so an RGB colour drawn into an 8-bpp bitmap lands on the
/// nearest index that bitmap has.
pub const fn native_to_index8(native: u32) -> u8 {
    let (r, g, b) = unpack565(native as u16);
    (((r as u32 * 7 / 255) << 5) | ((g as u32 * 7 / 255) << 2) | (b as u32 * 3 / 255)) as u8
}

/// RGBVAL to `NativeColor`. `NativeColor` is a `uint32` even on a 16-bpp device;
/// the pixel itself is the low half.
pub const fn rgb_to_native(rgbval: u32) -> u32 {
    let (r, g, b) = split_rgb(rgbval);
    pack565(r, g, b) as u32
}

/// `IDIB::pRGB` holds each colour as a 32-bit word whose value is `0x00RRGGBB`. The header says
/// "an array of 32-bit R-G-B color values", and a reference front end's OEM palette reader unpacks it in exactly
/// that shape — `(rgb >> 16) & 0xff` for red. An `RGBVAL` is the other rotation: `MAKE_RGB` puts red
/// at bits 8-15, so an `RGBVAL` sitting in guest memory reads as the bytes `pad, R, G, B`. The two
/// disagree by three bytes reversed, and only one of them can paint a screen.
///
/// Which one is not a tint question. The bytes `f1 d2 a1 00` name one index, and the handset shows
/// `(164,210,238)` there — this reading's `(161,210,241)`, not the reversed reading's warm
/// `(241,210,161)`.
const fn palette_to_rgbval(w: u32) -> u32 {
    ((w & 0xff) << 24) | (((w >> 8) & 0xff) << 16) | (((w >> 16) & 0xff) << 8)
}

/// The other direction, for the default table this host publishes at `IDIB::pRGB`: a pack that reads
/// the array back has to find the same word in ours that it wrote in its own.
const fn rgbval_to_palette(rgb: u32) -> u32 {
    (((rgb >> 8) & 0xff) << 16) | (((rgb >> 16) & 0xff) << 8) | ((rgb >> 24) & 0xff)
}

/// `NativeColor` to `RGBVAL`, i.e. `IBitmap_NativeToRGB`.
pub const fn native_to_rgb(native: u32) -> u32 {
    let (r, g, b) = unpack565(native as u16);
    crate::device::make_rgb(r, g, b)
}

/// An `RGBVAL` as the word a 32-bpp bitmap stores: `0x00RRGGBB`, the packing
/// `IDIB_COLORSCHEME_888` names and the delivered `DIB_RGBToNative` builds.
const fn rgbval_to_888(rgbval: u32) -> u32 {
    let (r, g, b) = crate::device::split_rgb(rgbval);
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// A 32-bpp stored word as an `RGBVAL`.
const fn stored888_to_rgbval(v: u32) -> u32 {
    crate::device::make_rgb(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    )
}

/// The canonical (565) colour a 32-bpp stored word denotes. Drawing state in this host is 565 —
/// `RGBToNative` on a 16-bpp destination, every palette entry, every `IGraphics` colour — so a
/// mixed-depth blit converts at this boundary; a 32→32 blit has its own raw row path and does not
/// pass through here.
const fn stored888_to_565(v: u32) -> u32 {
    pack565(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    ) as u32
}

/// The 32-bpp stored word a canonical (565) colour denotes.
const fn native565_to_888(v: u32) -> u32 {
    let (r, g, b) = unpack565(v as u16);
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Whether a stored pixel is a whole 32-bit `0x00RRGGBB` word rather than a packed 565 one.
///
/// Strictly 32: `nDepth` is a public field a pack may restate, and treating 24 as four bytes would
/// read one byte past a surface the host allocated as two.
const fn is_888(depth: u16) -> bool {
    depth == 32
}

/// Squared distance between two `RGBVAL`s, for the one place a colour has to be matched rather than
/// converted: a palettized bitmap's table is fixed, so a colour asked of it is not always in it.
fn rgb_distance(a: u32, b: u32) -> u32 {
    let d = |sh: u32| -> u32 {
        let x = ((a >> sh) & 0xff) as i32 - ((b >> sh) & 0xff) as i32;
        (x * x) as u32
    };
    d(8) + d(16) + d(24)
}

/// The raster operations this host can carry out, which is exactly the set [`raster`] answers for.
///
/// Written as a projection of one definition rather than a second list: the blit entry points refuse
/// an op they do not support before a pixel is touched, and the pixel write decides the same question
/// again afterwards, so two lists that disagreed would let a blit promise an answer it then silently
/// dropped — the exact shape of bug this file keeps having to hunt.
pub fn rop_supported(rop: u32) -> bool {
    raster(rop, 0, 0).is_some()
}

/// One pixel through a raster op: the source pixel, the destination pixel standing there now, and the
/// answer — or `None` for an op this host does not carry out.
///
/// The header *buffers*, so the operands here are the packed words, padding bits and all: a handset's 565
/// blitter inverts and ORs the halfword it has, and that is what the guest packs were tuned
/// against. `AEE_RO_TRANSPARENT` is listed because it is a plain copy once the caller has skipped the
/// key pixel, which the source-bitmap blits do in [`source_pixel`] and the row paths. It is not a
/// reached arm from [`Display::put`]: every slot whose "source" is a colour argument answers
/// transparency at its own door, and `put` never gets there with this op.
///
/// Two are deliberately absent from this table. `AEE_RO_BLEND` is alpha compositing, and it is not a
/// word-level operation: what composites is the source's own coverage, which lives in
/// [`Display::alphas`] and is applied by [`blend`] before the value reaches here — a 565 destination
/// has no alpha of its own, so there is nothing for the packed words to combine. `AEE_RO_OLDMASK` is
/// excluded by the enum's own comment — it is not supported for `IBitmap`.
fn raster(rop: u32, src: Native, dst: Native) -> Option<Native> {
    raster_word(rop, src, dst).map(|v| v & 0xffff)
}

/// The same operation without narrowing the answer to a 16-bit word. A 32-bpp bitmap combines
/// `0x00RRGGBB` words, and `AEE_RO_NOT` on one has to complement the whole colour rather than leave
/// the high byte zeroed.
fn raster_word(rop: u32, src: Native, dst: Native) -> Option<Native> {
    Some(match rop {
        g::AEE_RO_OR => src | dst,
        g::AEE_RO_XOR => src ^ dst,
        g::AEE_RO_COPY => src,
        g::AEE_RO_NOT => !src,
        g::AEE_RO_MERGENOT => dst | !src,
        g::AEE_RO_ANDNOT => dst & !src,
        g::AEE_RO_TRANSPARENT => src,
        g::AEE_RO_AND => src & dst,
        _ => return None,
    })
}

/// An `AEERect`, whose four `int16` members the ABI table positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub dx: i16,
    pub dy: i16,
}

impl Default for Rect {
    /// A rectangle with no area is also the rectangle nothing has been placed in yet, which is what
    /// the dialog records start with before a `DialogInfo` says where they go.
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Rect {
    /// A rectangle with no area, which is what "no clip" and "nothing left to draw" both mean.
    pub const EMPTY: Rect = Rect {
        x: 0,
        y: 0,
        dx: 0,
        dy: 0,
    };

    pub fn new(x: i16, y: i16, dx: i16, dy: i16) -> Rect {
        Rect { x, y, dx, dy }
    }

    /// The whole of a `width` x `height` bitmap, which is a display's initial clip.
    pub fn whole(width: u16, height: u16) -> Rect {
        Rect::new(0, 0, width as i16, height as i16)
    }

    pub fn is_empty(self) -> bool {
        self.dx <= 0 || self.dy <= 0
    }

    /// Inclusive pixel range, or `None` when the extent is empty.
    pub fn span(self) -> Option<(i32, i32, i32, i32)> {
        if self.is_empty() {
            return None;
        }
        Some((
            self.x as i32,
            self.y as i32,
            self.x as i32 + self.dx as i32 - 1,
            self.y as i32 + self.dy as i32 - 1,
        ))
    }

    /// The overlap with `other`, `None` when either is empty or they do not intersect. This is the
    // one place clipping happens, so a request partly off-screen draws its visible part instead
    /// of faulting or wrapping into the next row.
    pub fn clipped(self, other: Rect) -> Option<Rect> {
        let (ax0, ay0, ax1, ay1) = self.span()?;
        let (bx0, by0, bx1, by1) = other.span()?;
        let (x0, y0) = (ax0.max(bx0), ay0.max(by0));
        let (x1, y1) = (ax1.min(bx1), ay1.min(by1));
        if x0 > x1 || y0 > y1 {
            return None;
        }
        let extent = |v: i32| v.clamp(0, i16::MAX as i32) as i16;
        Some(Rect::new(
            x0 as i16,
            y0 as i16,
            extent(x1 - x0 + 1),
            extent(y1 - y0 + 1),
        ))
    }

    pub fn read(mem: &Mem, addr: u32) -> HResult<Rect> {
        // `IDisplay::DrawRect` arrives thousands of times in one 500 ms frame of the sample set pack: four
        // separate `read_u16` calls meant four page-table walks for every rectangle. `AEERect` is
        // four consecutive little-endian `int16`s, so one `read_into` gets the same bytes — it
        // already handles a read straddling two pages, and drops to byte-at-a-time reads while a
        // watchpoint is installed. Offsets still come from the ABI table; if a table ever laid the
        // struct out past these eight bytes, fall back to the four reads instead of indexing out of
        // bounds.
        let offsets = &field_layout().rect;
        let mut bytes = [0u8; 8];
        mem.read_into(addr, &mut bytes)?;
        let word = |off: u32| -> Option<i16> {
            let o = off as usize;
            let pair = bytes.get(o..o + 2)?;
            Some(u16::from_le_bytes([pair[0], pair[1]]) as i16)
        };
        if let (Some(x), Some(y), Some(dx), Some(dy)) = (
            word(offsets[0]),
            word(offsets[1]),
            word(offsets[2]),
            word(offsets[3]),
        ) {
            return Ok(Rect { x, y, dx, dy });
        }
        let r = |off: u32| -> HResult<i16> { Ok(mem.read_u16(addr + off)? as i16) };
        Ok(Rect {
            x: r(offsets[0])?,
            y: r(offsets[1])?,
            dx: r(offsets[2])?,
            dy: r(offsets[3])?,
        })
    }

    pub fn write(&self, mem: &mut Mem, addr: u32) -> HResult<()> {
        let offsets = &field_layout().rect;
        for (off, v) in offsets
            .iter()
            .copied()
            .zip([self.x, self.y, self.dx, self.dy])
        {
            mem.write_u16(addr + off, v as u16 as u32)?;
        }
        Ok(())
    }
}

/// A `u32`'s worth of pixel data: `NativeColor` is 32 bits, the stored pixel is 16.
type Native = u32;

/// One host `IBitmap`: an object the guest holds a pointer to, plus where its bits are.
#[derive(Debug, Clone, Copy)]
pub struct Bitmap {
    /// The `IBitmap*` the guest uses.
    pub obj: u32,
    /// Guest address of pixel (0, 0), published as `IDIB::pBmp`.
    pub px: u32,
    /// Guest address of this bitmap's own palette, published as `IDIB::pRGB`, or zero when the pixels
    /// carry native colours. An app writes the array in place, which is the whole reason it lives in
    /// the guest rather than in this record: the header makes the palette a *public data member*.
    pub rgb: u32,
    /// Entries in that array, published as `IDIB::cntRGB`. Zero means no palette, so pixel values are
    /// colours rather than indices.
    pub cnt_rgb: u16,
    pub width: u16,
    pub height: u16,
    /// Bytes from one row to the next, so a row wider than the image works. Published as
    /// `IDIB::nPitch`, and — like it — allowed to be negative: the header's own example hands over a
    /// bottom-up bitmap as `pBmp` at the last row with a pitch of `-12`, and `pBmp[y * nPitch + x]` is
    /// the reading it tells an app to use.
    pub pitch: i32,
    pub depth: u16,
    /// `NativeColor` this bitmap treats as transparent under `AEE_RO_TRANSPARENT`, in **its own
    /// pixel space** — the number stored in the pixel array, which on a palettized bitmap is an index
    /// and not a colour.
    ///
    /// That is what the header says twice: the header calls `ncTransparent` "not in RGB
    /// form ... a `NativeColor`, which corresponds directly to values in the pixel array", and
    /// The header corresponding R-G-B intensities". An earlier version of this record kept the *colour* the key
    /// named, which agreed with those sentences only while no two palette entries packed to the same
    /// 5-6-5 word — and made a `SetTransparencyColor` / `GetTransparencyColor` pair round-trip
    /// through `NativeToRGB`/`RGBToNative` instead of returning the number that was written.
    pub trans: Native,
}

/// Per-pixel transparency the *file a picture came from* named — a PNG `tRNS` entry or an alpha
/// channel, one byte per pixel, row-major, with the summary the blitter needs to choose between
/// "skip this pixel" and "compose this pixel".
///
/// It lives in [`Display::alphas`] and not inside [`Bitmap`] for one mechanical reason: `Bitmap` is
/// `Copy` and is handed out by value from `Display::bitmap`, so a buffer in it would be rebuilt on
/// every lookup — and the row-batched blit path exists precisely to avoid per-pixel work. Reading it
/// from the side table costs one `Arc` clone per blit.
///
/// Keeping it beside the pixels rather than blending it into them is the other half of the same
/// choice: the pack is entitled to read the colours it reference, and rewriting a see-through pixel
/// into some mask colour would fabricate a byte the app never stored.
#[derive(Debug)]
pub struct AlphaMap {
    bytes: Vec<u8>,
    width: usize,
    /// Some pixel is partly covered: an alpha strictly between 0 and 255.
    partial: bool,
}

impl AlphaMap {
    /// Take the alpha of a decoded picture and keep it only if it says anything at all. A picture
    /// whose every alpha is 255 costs nothing extra to paint, so it keeps `None` and with it the
    /// batched row path.
    pub fn new(bytes: Vec<u8>, width: usize) -> Option<Self> {
        let partial = bytes.iter().any(|&a| a != 0 && a != u8::MAX);
        let any = bytes.iter().any(|&a| a != u8::MAX);
        any.then(|| Self {
            bytes,
            width,
            partial,
        })
    }

    /// The alpha of one pixel; a coordinate the map does not cover is opaque, not invisible.
    fn at(&self, x: usize, y: usize) -> u8 {
        self.bytes
            .get(y * self.width + x)
            .copied()
            .unwrap_or(u8::MAX)
    }

    pub fn partial(&self) -> bool {
        self.partial
    }
}

impl Bitmap {
    /// The byte holding pixel (x, y). The offset is counted in bits rather than bytes, because 4-bpp
    /// pixels are two to a byte and 8-bpp ones are whole bytes: `x * depth / 8` covers 4, 8 and 16
    /// alike. The high nibble is the even pixel.
    fn at(&self, x: u32, y: u32) -> u32 {
        self.px
            .wrapping_add((y as i32 * self.pitch + ((x * u32::from(self.depth)) / 8) as i32) as u32)
    }

    /// Guest address of the first byte of row `y`.
    ///
    /// Two readers: the row-batched screenshot and [`crate::screen::Screen`], which converts a row
    /// at a time. Only for a 16-bit bitmap — [`Bitmap::at`], which this calls, folds the depth in,
    /// so a byte-addressed call is right for 565 and wrong for 8-bpp. The pitch is allowed to be
    /// negative (a bottom-up bitmap) and wider than the image, so this is the only arithmetic that
    /// knows where a row begins.
    pub(crate) fn row_start(&self, y: u32) -> u32 {
        self.at(0, y)
    }

    /// The palette index pixel (x, y) holds, for a bitmap whose depth is 8, 4 or 1.
    ///
    /// The one place the packing of an indexed bitmap is written down: 8-bpp is a byte an index,
    /// 4-bpp puts two in a byte with the **high** nibble first, 1-bpp puts eight in a byte most
    /// significant bit first. [`read_pixel`] and [`Display::screenshot_pixels`] both come through
    /// here, which is what keeps a blit and a capture of the same screen from disagreeing. The order
    /// is not a guess: replaying a 4-bpp font's 1584 blits against a real-device screenshot left
    /// 51.5 % of the destination pixels wrong with the halves the other way round.
    ///
    /// Only for a depth that has indices. A 16-bpp pixel is a colour, and a 0-bpp bitmap has no
    /// pixels at all — its `nPitch` is zero, so every coordinate would land on the object's own
    /// header.
    fn index_at(&self, mem: &Mem, x: u32, y: u32) -> HResult<u8> {
        let byte = mem.read_u8(self.at(x, y))?;
        Ok(match self.depth {
            4 => {
                if x % 2 == 0 {
                    byte >> 4
                } else {
                    byte & 0x0f
                }
            }
            1 => (byte >> (7 - x % 8)) & 1,
            _ => byte,
        })
    }

    /// What one index means as a `NativeColor`: the entry this bitmap's own `IDIB::pRGB` names, or the
    /// host's default mapping when it has no table or the index runs past it. A pack that decodes its
    /// art writes that array in guest memory, so reading it here is the difference between its colours
    /// and a screen of whatever the fixed 3-3-2 table happens to render.
    fn colour_of(&self, mem: &Mem, index: u8) -> HResult<Native> {
        if self.rgb != 0 && u32::from(index) < u32::from(self.cnt_rgb) {
            // `wrapping_add`, like [`Bitmap::at`]: `pRGB` is a pointer the guest wrote, and an index
            // scaled by four off a pointer near the top of the address space would otherwise overflow
            // in a debug build instead of faulting like every other bad guest address.
            let at = self.rgb.wrapping_add(u32::from(index) * 4);
            return Ok(rgb_to_native(palette_to_rgbval(mem.read_u32(at)?)));
        }
        Ok(ramp_colour(self.depth, index))
    }

    /// The inverse of [`Bitmap::colour_of`]: an `RGBVAL` asked of *this* bitmap, answered in the
    /// bitmap's own pixel space — a palette index where it has a palette, the packed colour where it
    /// does not. `IDIB::ncTransparent` and `IBitmap_RGBToNative` are both in that space, and a pack that
    /// stores one and reads the palette with it turns a packed answer into a wild subscript.
    ///
    /// Searching the table rather than inverting the fixed 3-3-2 ramp is what makes the two functions
    /// actually inverse. A pack keys its sprites in guest code by asking this question once, storing
    /// the answer in `ncTransparent`, and comparing every pixel against it on the way out; the ramp
    /// answers magenta as 227 while its atlas names it 3, so
    /// the comparison never fired and the mask colour was painted as a solid block over its title art
    /// and its overworld.
    ///
    /// The table is compared as BYTES before anything is quantised, because the two halves of that
    /// question are different. A pack's table can hold a colour one step off the key it means — a
    /// `(254,0,255)` beside its `(255,0,255)` — and both pack to the same RGB565 word `0xf81f`, so a
    /// distance search on quantised values ties on whichever it walks first and answers the wrong
    /// index, so the pack paints its own magenta as a rectangle framing the sprite.
    pub fn pixel_value(&self, mem: &Mem, rgbval: u32) -> HResult<Native> {
        if self.depth < 16 && self.rgb != 0 && self.cnt_rgb != 0 {
            let room = 1u32 << self.depth.min(8);
            let n = u32::from(self.cnt_rgb).min(room);
            let exact = rgbval_to_palette(rgbval);
            for i in 0..n {
                // `wrapping_add` for the reason [`Bitmap::colour_of`] gives: `pRGB` is a pointer the
                // guest wrote, and an index scaled by four off a pointer near the top of the address
                // space should fault like any other bad address rather than overflow in a debug build.
                if mem.read_u32(self.rgb.wrapping_add(i * 4))? == exact {
                    return Ok(i);
                }
            }
            // No entry says that colour, so the nearest one does: a handset's table is fixed and a
            // requested colour is not always in it. What must never happen is an answer past the end of
            // the array the caller then subscripts.
            let mut best: Option<(u32, u32)> = None;
            for i in 0..n {
                let d = rgb_distance(
                    rgbval,
                    palette_to_rgbval(mem.read_u32(self.rgb.wrapping_add(i * 4))?),
                );
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, i));
                }
            }
            if let Some((_, i)) = best {
                return Ok(i);
            }
        }
        let native = rgb_to_native(rgbval);
        if is_888(self.depth) {
            return Ok(rgbval_to_888(rgbval));
        }
        Ok(match self.depth {
            8 => u32::from(native_to_index8(native)),
            4 => u32::from(native_to_index4(native)),
            1 => u32::from(native_to_index1(native)),
            _ => native & 0xffff,
        })
    }

    /// A value an `IBitmap` or `IFont` slot was handed, read as the colour it stands for.
    ///
    /// The header stored in the pixel array* — an index for a palettized bitmap, not a colour. Everything below
    /// this point draws in colours (that is what `raster` requires and what a blitted sprite arrives
    /// as), so the two spaces meet here, once per call rather than once per pixel.
    ///
    /// It is not the exact inverse of [`Bitmap::stored`] in every case, and the cases are worth
    /// knowing: two entries that pack to the same 5-6-5 word, or an index at or past `cnt_rgb`, come
    /// back as whichever entry is nearest rather than as themselves. A table a pack wrote from its own
    /// art is normally distinct enough that the round trip is exact — and reading a value that was in
    /// fact a colour (a pack that skipped `IBitmap_RGBToNative`) as an index would be worse than any
    /// of those: it would paint `colour & 0xff`, which names nothing at all.
    fn colour_of_pixel(&self, mem: &Mem, value: Native) -> HResult<Native> {
        if !self.keeps_indices() {
            return Ok(if is_888(self.depth) {
                stored888_to_565(value)
            } else {
                value & 0xffff
            });
        }
        self.colour_of(mem, value as u8)
    }

    /// Whether a pixel here is an index into `pRGB` rather than a colour.
    ///
    /// The same three depths [`read_pixel`] and [`write_pixel`] treat as packed, and deliberately
    /// *only* those: an `IDIB`'s header is public data a pack may restate, so `nDepth` can arrive as
    /// something this host never allocates. A 12-bpp bitmap is read and written as a whole 16-bit
    /// word by both of them, so depth alone cannot be the test — selecting indices by
    /// `depth < NATIVE_DEPTH` would send 9 through 15 to the palette and dress a 16-bit pixel up as
    /// an 8-bit index, which is the same class of mistake in the other direction.
    fn keeps_indices(&self) -> bool {
        matches!(self.depth, 8 | 4 | 1)
    }

    /// The value this bitmap stores for a colour: the palette index that names it, or the packed
    /// colour itself when the pixels are colours. The inverse of [`Bitmap::colour_of`], and the same
    /// question [`Bitmap::pixel_value`] answers with an `RGBVAL` in hand.
    ///
    /// Drawing needs it because a colour arrives from somewhere else — a sprite, a fill parameter, a
    /// text colour — and the bitmap it is going into may keep indices. Asking the table is what makes
    /// the two directions agree: without it a colour drawn into a palettized bitmap lands on the fixed
    /// 3-3-2 ramp's idea of that colour, which is index 227 for magenta in a bitmap whose own table
    /// names it 3 — so the pack reads back a pixel it cannot find, and paints its transparency key as
    /// a solid block. That is the read-side bug `pixel_value` was written for, arriving from the other
    /// end.
    fn stored(&self, mem: &Mem, native: Native) -> HResult<Native> {
        // The screen's own case, first and without a conversion: `pixel_value` would take the colour
        // to `RGBVAL` and back, which is the identity on a 5-6-5 word and a waste of it in a loop that
        // runs per pixel of every blit.
        if !self.keeps_indices() {
            return Ok(if is_888(self.depth) {
                native565_to_888(native)
            } else {
                native & 0xffff
            });
        }
        self.pixel_value(mem, native_to_rgb(native))
    }
}

/// What a handset keeps behind one `IDisplay*`.
#[derive(Debug, Clone)]
pub struct DisplayState {
    /// The `IBitmap*` drawing lands in: the device bitmap until something else is set.
    pub dest: u32,
    pub clip: Rect,
    /// The `AEEClrItem` table: three active drawing colours then the system colours. Indexed by
    /// the item number, so entry 0 is unused.
    pub colors: Vec<u32>,
    pub annunciators: u16,
    pub backlight: bool,
    /// Each `IFont` an applet has swapped in with `IDisplay_SetFont`, keyed by the designator it
    /// named. The key is kept as it arrived rather than treated as an index: `AEEFont` starts at
    /// `0x8000`, and an applet is allowed to install under any of the eight.
    pub fonts: Vec<(u32, u32)>,
    /// `Update(po, TRUE)` messages posted but not yet taken by the UI task. Counted, not booled: the
    /// header's own macro defers every call, so a pack that presents twice a turn expects two
    /// presentations — only the moment they land changes.
    pub pending: u32,
}

/// One `IDisplay::DrawText` that reached a destination, recorded after the alignment flags have
/// been resolved. A headless run cannot look at the screen, so this is what it asserts on: which
/// string, in which font, at which pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub dest: u32,
    pub font: u32,
    /// The `AECHAR` units as text, with the lossy conversion a viewer would apply.
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub flags: u32,
    /// How far the guest had run when it drew this. A report's reader wants the words in the order
    /// the player saw them — which title screen came first, what the last thing on screen was — and
    /// that ordering is the instruction count, not the position on the bitmap.
    pub step: u64,
}

/// The host display subsystem: the device bitmap, every live `IBitmap`, and one state record per
/// `IDisplay`.
#[derive(Debug)]
pub struct Display {
    /// `IBitmap*` of the screen itself, held alive by this struct.
    pub device: Bitmap,
    /// Where every DIB this display allocates keeps its pixels, from the device profile.
    dib_pixel_offset: u32,
    /// The deepest pixel depth an off-screen DIB may request, from the device profile.
    dib_depth: u16,
    /// The encoding an `AECHAR` argument is in: `AEEDeviceInfo::wEncoding`.
    ///
    /// A display built by hand — in a test — has no device behind it and keeps the default
    /// [`TextEncoding::Unicode`], so a unit written into guest memory is read as the code point it
    /// looks like. [`crate::runtime::Runtime::with_device_and_heap`] sets the device's answer on
    /// the display it builds, which is the only place a handset's encoding arrives from.
    encoding: TextEncoding,
    /// Whether `IDisplay::DrawText`'s `pcText` is the device's 8-bit string (the BREW 1.x ABI)
    /// rather than an `AECHAR` run (4.0.x). Set from [`crate::device::DeviceProfile::text_bytes`].
    text_bytes: bool,
    /// The `IDisplay*` applets are handed when the runtime starts one, so `IShell::Clone` and a
    /// second `CreateInstance` do not fight over who owns which state.
    displays: HashMap<u32, DisplayState>,
    /// The display BREW itself holds a reference on, i.e. the one `AEECLSID_DISPLAY` answered for.
    /// `IDisplay_MakeDefault` moves this reference, and it is why a display outlives its applet.
    default: u32,
    bitmaps: HashMap<u32, Bitmap>,
    /// Pixel start → object, for the OEM `CONVERTBMP` answer that is the pixel run itself.
    pixel_owner: HashMap<u32, u32>,
    /// The transparency a decoded picture named, by the bitmap object it was laid out into. Absent
    /// is the normal case and the fast one: every BMP and every PNG that says nothing about
    /// transparency, and those pictures keep the batched row path.
    alphas: HashMap<u32, Arc<AlphaMap>>,
    /// Faces installed under an `AEEFont` designator, replacing the built-in table for that
    /// number. A decoded BBF face lives here; the built-in grids stay in [`crate::font::Font::of`].
    /// Kept outside [`DisplayState`] because a face belongs to the host, not to one `IDisplay`.
    faces: HashMap<u32, Face>,
    /// How many times [`Self::install_face`] has run.
    ///
    /// A page height is cached against the parse and the column width, and a face installed after
    /// that measurement changes the height without touching either — a handset's 16-pixel CJK face
    /// fits half the words a line that the built-in grid did. The counter is what the cache can key
    /// on, the way it keys on the number of pictures that have settled.
    face_epoch: u32,
    /// Every `IDisplay::DrawText` that drew, oldest first.
    pub runs: Vec<TextRun>,
    /// How many text runs the run has drawn, including the ones the capped list above has dropped.
    /// A pack that paints a score every second outdraws anything anyone reads.
    pub runs_seen: u64,
    /// `IDisplay::Update` calls that reached the screen. A headless run asserts on this.
    pub presentations: u32,
    /// Requests a `void`-returning slot could not honour. Visible in a report because the ABI
    /// gives them no other way out. Bounded by [`Display::NOTES`].
    pub quiet: Vec<String>,
    /// How many notes the cap above has dropped, so a short list cannot be read as a complete one.
    pub notes_dropped: u64,
    /// The instruction count the host last saw, set once per call from the runtime. Everything here
    /// is dated from it — the text runs and the screens below — because the display has no CPU of its
    /// own and the alternative was threading a step count through `draw_text` and every widget that
    /// calls it.
    pub now: u64,
    /// `IDisplay::Update` calls, in the order they came and stamped with [`Display::now`]. One entry
    /// per screen the pack put up, which is what a report groups its text under. Capped the same way
    /// `runs` is, by dropping the oldest.
    pub screens: Vec<u64>,
    /// How many screen boundaries were recorded in total, including the ones the bounded window above
    /// has already dropped. `screens.len()` is the window, this is the run.
    pub screens_seen: u64,
    /// How many of this record's own entries were dropped to keep the window bounded.
    pub screens_dropped: u64,
    /// Every distinct `IFileMgr::OpenFile` the volume refused, first-seen order, bounded by
    /// [`Display::REFUSALS`]. Kept out of [`Display::quiet`] on purpose: that list is capped and
    /// evicts, and the runs long enough to evict it are exactly the ones where "the pack asked for a
    /// file we did not have" is the whole finding. A refusal has no other way out either — the slot
    /// answers `NULL`, and a pack that never asks `GetLastError` leaves nothing to read.
    pub refusals: Vec<String>,
    /// How many refusals there were, including names past the bound above. The count is the record;
    /// the list is only its first pages.
    pub refused: u64,
    /// Scratch row for the whole-colour `COPY` fill fast path (16 or 32 bpp). A full-screen clear and a
    /// frame of thousands of `DrawRect` calls used to allocate a `Vec` per call; one buffer turns that into a
    /// `resize` which is free once the widest row so far has been seen. Every pixel is rewritten on
    /// each fill, so a shorter row cannot inherit a stale byte. Holds no guest state and is
    /// deliberately absent from the hand-written `write_state` / `read_state` field lists.
    fill_row: Vec<u8>,
}

impl Display {
    /// Build the device bitmap. `hle` and `mem` are needed because the object and its pixels both
    /// live in the guest.
    pub fn new(
        mem: &mut Mem,
        hle: &mut Hle,
        width: u16,
        height: u16,
        depth: u16,
    ) -> HResult<Display> {
        Self::new_with_layout(
            mem,
            hle,
            width,
            height,
            depth,
            depth.max(NATIVE_DEPTH),
            DIB_PIXEL_OFFSET,
        )
    }

    /// Build a display at the profile's real frame depth, DIB ceiling and DIB pixel offset.
    ///
    /// `dib_depth` is the deepest off-screen depth this device backs; it has to cover `depth`, or
    /// the screen's own `CreateCompatibleBitmap` would be refused. `pixel_offset` is where
    /// `DIB_NewEx` puts pixels inside the object — see [`crate::device::DeviceProfile`].
    pub fn new_with_layout(
        mem: &mut Mem,
        hle: &mut Hle,
        width: u16,
        height: u16,
        depth: u16,
        dib_depth: u16,
        pixel_offset: u32,
    ) -> HResult<Display> {
        // The screen itself may be 32 even when DIBs are not, but never below what this host packs.
        check_depth(depth, 32)?;
        if pixel_offset < MIN_DIB_PIXEL_OFFSET {
            return Err(HleError::Unsupported {
                what: format!(
                    "a DIB pixel offset of {pixel_offset}: it has to be {MIN_DIB_PIXEL_OFFSET} or more, past IDIB's own fields"
                ),
            });
        }
        let stride = row_bytes(width, depth);
        let seed = Bitmap {
            obj: 0,
            px: 0,
            rgb: 0,
            cnt_rgb: 0,
            width,
            height,
            pitch: stride as i32,
            depth,
            // In this bitmap's own colour space — see [`default_transparent`].
            trans: default_transparent(depth),
        };
        let (obj, px) = new_bitmap_object(mem, hle, &seed, pixel_offset, true)?;
        let device = Bitmap { obj, px, ..seed };
        let mut bitmaps = HashMap::new();
        bitmaps.insert(obj, device);
        let mut pixel_owner = HashMap::new();
        pixel_owner.insert(px, obj);
        Ok(Display {
            device,
            dib_pixel_offset: pixel_offset,
            dib_depth: dib_depth.max(depth),
            encoding: TextEncoding::Unicode,
            text_bytes: false,
            displays: HashMap::new(),
            default: 0,
            bitmaps,
            pixel_owner,
            alphas: HashMap::new(),
            faces: HashMap::new(),
            face_epoch: 0,
            runs: Vec::new(),
            runs_seen: 0,
            presentations: 0,
            now: 0,
            screens: Vec::new(),
            screens_seen: 0,
            screens_dropped: 0,
            refusals: Vec::new(),
            refused: 0,
            fill_row: Vec::new(),
            quiet: Vec::new(),
            notes_dropped: 0,
        })
    }

    fn state(&self, disp: u32) -> HResult<DisplayState> {
        self.displays
            .get(&disp)
            .cloned()
            .ok_or(HleError::NotAnObject { addr: disp })
    }

    /// The size of the bitmap a display is currently drawing into.
    ///
    /// This used to be what an off-screen bitmap was sized to, on the inference that a DIB belongs
    /// to the screen it was asked from. The reference `IDisplay_CreateDIBitmap` wrapper passes `w` and
    /// `h` straight to `DIB_New`, so that inference is withdrawn and this has no caller; it is kept
    /// as a plain query rather than as a rule. See `Runtime`'s `CreateDIBitmap` arm for the evidence.
    pub fn dest_size(&self, disp: u32) -> HResult<(u16, u16)> {
        let state = self.state(disp)?;
        let b = self
            .bitmap(state.dest)
            .ok_or(HleError::NotAnObject { addr: state.dest })?;
        Ok((b.width, b.height))
    }

    fn put_state(&mut self, disp: u32, state: DisplayState) {
        self.displays.insert(disp, state);
    }

    /// `IDisplay::SetFont(nFont, piNewFont)`: swap the `IFont` installed under one designator and
    /// hand back the one that stood there before.
    ///
    /// Neither pointer is reference counted by this call — the header is explicit that "application
    /// and IDisplay are merely swapping pointers with already positive refcounts" — so the record is
    /// a pair of guest addresses and nothing here `AddRef`s. A designator nobody installed answers 0,
    /// which is the `NULL` the documented "if failed" path returns, so a caller that installed a face
    /// where [`Self::BOUND_FACES`] left none is told its swap failed. That is the honest answer for a
    /// face this host does not have, and the reason the three it does have are bound up front.
    pub fn set_font(&mut self, disp: u32, nfont: u32, install: u32) -> HResult<u32> {
        // The gate is `if ( (unsigned)(nFont - 0x8000) >= 8 ) return 0;` — one line, and it does two
        // things at once: an id outside the eight system designators answers "there was no font
        // here", and **nothing is stored**. So a made-up designator is not a private slot an applet can
        // park a font in; the drawing calls keep using the face the designator they were given names.
        if nfont < g::AEE_FONT_NORMAL || nfont >= g::AEE_FONT_TOTAL {
            return Ok(0);
        }
        let mut state = self.state(disp)?;
        let prev = match state.fonts.iter_mut().find(|(d, _)| *d == nfont) {
            Some(swap) => std::mem::replace(&mut swap.1, install),
            None => {
                state.fonts.push((nfont, install));
                0
            }
        };
        self.put_state(disp, state);
        Ok(prev)
    }

    /// Install a face under one `AEEFont` designator. `IDisplay::SetFont` installs an object,
    /// which is a different thing: this replaces the host's own answer for a designator, and is
    /// how a decoded BBF file becomes the face a pack draws with.
    pub fn install_face(&mut self, nfont: u32, face: Face) {
        self.faces.insert(nfont, face);
        self.face_epoch = self.face_epoch.wrapping_add(1);
    }

    /// How many faces have been installed, which is what a cached measurement of text keys on.
    pub fn face_epoch(&self) -> u32 {
        self.face_epoch
    }

    /// Set what an `AECHAR` argument is in. The runtime does this once from the device profile,
    /// and again after restoring a state block whose display was replaced.
    pub fn set_encoding(&mut self, encoding: TextEncoding) {
        self.encoding = encoding;
    }

    /// Set whether `IDisplay::DrawText`'s `pcText` is the device's 8-bit string (BREW 1.x) rather
    /// than an `AECHAR` run. The runtime does this once from the device profile.
    pub fn set_text_bytes(&mut self, bytes: bool) {
        self.text_bytes = bytes;
    }

    /// The encoding an `AECHAR` argument is read in.
    pub fn encoding(&self) -> TextEncoding {
        self.encoding
    }

    /// The faces this display answers with, in the form a caller that only measures can hold.
    ///
    /// Layout has no `&mut Display` and must not have one: it runs from the widget side too, where
    /// there is no display in reach. It still has to measure with the face the painter will draw,
    /// so the table travels on its own — see [`crate::font::Faces`] for what goes wrong when a
    /// caller answers the designator question itself.
    pub fn faces(&self) -> Faces<'_> {
        Faces::over(&self.faces)
    }

    /// The face a designator names: an installed one first, then the built-in table.
    pub fn face(&self, nfont: u32) -> Option<Face> {
        self.faces().face(nfont)
    }

    /// The `IFont` installed under one designator, or 0 where nobody has installed one.
    pub fn font_installed(&self, disp: u32, nfont: u32) -> u32 {
        self.displays
            .get(&disp)
            .and_then(|s| s.fonts.iter().find(|(d, _)| *d == nfont))
            .map(|(_, f)| *f)
            .unwrap_or(0)
    }

    pub fn bitmap(&self, obj: u32) -> Option<Bitmap> {
        let obj = self.pixel_owner.get(&obj).copied().unwrap_or(obj);
        self.bitmaps.get(&obj).copied()
    }

    /// `IBitmap::RGBToNative`: an `RGBVAL` expressed in *this bitmap's* pixel space. For the device's
    /// own 16-bpp bitmap that is the packed colour; for a palettized one it is a palette index, because
    /// The header Answering with a packed colour instead hands the caller a subscript past the end of its table —
    /// the pack reads `pRGB[ncTransparent]` straight back out of what it stored there.
    pub fn pixel_value(&self, mem: &Mem, obj: u32, rgbval: u32) -> HResult<Native> {
        self.bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?
            .pixel_value(mem, rgbval)
    }

    /// `IBitmap::NativeToRGB`, the same distance back: a value in the bitmap's own pixel space, read
    /// through its palette when it has one, and turned into an `RGBVAL`.
    pub fn pixel_rgb(&self, mem: &Mem, obj: u32, value: Native) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        let native = if is_888(b.depth) {
            return Ok(stored888_to_rgbval(value));
        } else if b.depth >= 16 {
            value & 0xffff
        } else {
            b.colour_of(mem, value as u8)?
        };
        Ok(native_to_rgb(native))
    }

    /// Re-read the `IDIB` an app may have rewritten, and make this host's copy agree with it.
    ///
    /// The header through `pBmp[y * nPitch + x]`, so the struct — not the host — is the record of what a bitmap is.
    /// A pack uses that licence wholesale: after asking for a DIB it points `pBmp` at the art it decoded
    /// itself, restates `cx`/`cy` for the sprite it unpacked, sets `nPitch` *negative* because its BMP is
    /// bottom-up, and names its own `pRGB`. A host that kept its own geometry and never looked again
    /// blitted the empty buffer it had allocated at creation time, which is why this pack drew solid
    /// black shapes for a whole frame. An app that never touches the struct gets back the bytes we wrote,
    /// so for every other pack in the sample set this is a no-op.
    pub fn sync_bitmap(&mut self, mem: &Mem, obj: u32) -> HResult<()> {
        let obj = self.pixel_owner.get(&obj).copied().unwrap_or(obj);
        let Some(mut b) = self.bitmaps.get(&obj).copied() else {
            return Ok(());
        };
        let o = field_layout();
        b.px = mem.read_u32(obj + o.idib_pbmp)?;
        b.rgb = mem.read_u32(obj + o.idib_prgb)?;
        // `cx`/`cy` and `nPitch`/`cntRGB` each share a word, which is how the header lays them out.
        let dims = mem.read_u32(obj + o.idib_cx)?;
        b.width = dims as u16;
        b.height = (dims >> 16) as u16;
        let pitch = mem.read_u32(obj + o.idib_pitch)?;
        b.pitch = pitch as u16 as i16 as i32;
        b.cnt_rgb = (pitch >> 16) as u16;
        let depth = mem.read_u32(obj + o.idib_depth)?;
        b.depth = (depth & 0xff) as u16;
        // `ncTransparent` is a value in the bitmap's own pixel space, and the header is emphatic that
        // this is the space the comparison happens in: "the matching is performed on pixel values, not
        // on the corresponding R-G-B intensities". Taken as-is, without asking
        // the palette what it means — on an 8-bpp surface entry 7 and entry 200 are different keys
        // even if the table happens to give them the same 5-6-5 word.
        let trans = mem.read_u32(obj + o.idib_trans)?;
        b.trans = if is_888(b.depth) {
            trans
        } else {
            trans & 0xffff
        };
        self.bitmaps.insert(obj, b);
        self.pixel_owner.insert(b.px, obj);
        Ok(())
    }

    /// Whether a bitmap's pixel extent is actually mapped. A pack may restate `pBmp` to a word it
    /// never decoded — jump does exactly that before an `IDisplay::BitBlt` — and the blit has to
    /// answer the same way it answers a null source rather than faulting the run.
    fn readable_pixels(&self, mem: &Mem, b: &Bitmap) -> bool {
        if b.px == 0 || b.px == u32::MAX || !mem.is_mapped(b.px) {
            return false;
        }
        let stride = b.pitch.unsigned_abs().max(1);
        let rows = u32::from(b.height).saturating_sub(1);
        let run = row_bytes(b.width, b.depth).saturating_sub(1);
        let start = if b.pitch < 0 {
            b.px.saturating_sub(rows.saturating_mul(stride))
        } else {
            b.px
        };
        let end = start
            .saturating_add(rows.saturating_mul(stride))
            .saturating_add(run);
        mem.is_mapped(end)
    }

    /// A DIB the pack built itself. the bitmap declarations publishes the header and its inline helpers, and
    /// `IDisplay::BitBlt` takes any `IBitmap*`, not only the ones this host handed out: chtycoon
    /// copies the device bitmap's header into its own heap block, repoints `pBmp` at the screen
    /// framebuffer and blits that. Requiring an object-table entry faults a pack doing exactly what
    /// the header says it may.
    fn guest_bitmap(&self, mem: &Mem, obj: u32) -> Option<Bitmap> {
        let o = field_layout();
        let px = mem.read_u32(obj + o.idib_pbmp).ok()?;
        if px == 0 || px == u32::MAX || !mem.is_mapped(px) {
            return None;
        }
        let dims = mem.read_u32(obj + o.idib_cx).ok()?;
        let (width, height) = (dims as u16, (dims >> 16) as u16);
        if width == 0 || height == 0 {
            return None;
        }
        let pitch = mem.read_u32(obj + o.idib_pitch).ok()?;
        let depth = mem.read_u32(obj + o.idib_depth).ok()? as u16 & 0xff;
        if !matches!(depth, 1 | 2 | 4 | 8 | 16 | 32) {
            return None;
        }
        let cnt_rgb = (pitch >> 16) as u16;
        let trans = mem.read_u32(obj + o.idib_trans).ok()?;
        Some(Bitmap {
            obj,
            px,
            rgb: mem.read_u32(obj + o.idib_prgb).ok()?,
            cnt_rgb,
            width,
            height,
            pitch: pitch as u16 as i16 as i32,
            depth,
            trans: if is_888(depth) {
                trans
            } else {
                trans & 0xffff
            },
        })
    }

    /// The `IBitmap*` of the screen, i.e. what `IDisplay::GetDeviceBitmap` hands out.
    pub fn device_bitmap(&self) -> u32 {
        self.device.obj
    }

    /// The display BREW holds a reference on, or 0 before the first `CreateInstance(AEECLSID_DISPLAY)`
    /// or `MakeDefault` names one.
    pub fn default_display(&self) -> u32 {
        self.default
    }

    /// The bitmap a display draws into, whether or not we know the object.
    fn dest(&self, state: &DisplayState) -> HResult<Bitmap> {
        self.bitmap(state.dest)
            .ok_or(HleError::NotAnObject { addr: state.dest })
    }

    /// How many notes [`Display::note`] keeps before it starts dropping the oldest.
    pub const NOTES: usize = 256;

    /// How many distinct refusals [`Display::refuse`] lists. The count in [`Display::refused`] is not
    /// bounded, so a short list still says how much it left out.
    pub const REFUSALS: usize = 12;

    /// Say that a file was refused — or that one opened but holds nothing, the shape a missing
    /// resource takes when the file is there and the payload is not — in a place the note cap cannot
    /// evict. Same text once per distinct name-and-reason, but every one is counted.
    pub fn refuse(&mut self, what: impl Into<String>) {
        self.refused += 1;
        let what = what.into();
        if self.refusals.len() < Self::REFUSALS && !self.refusals.iter().any(|seen| *seen == what) {
            self.refusals.push(what);
        }
    }

    /// Record something the ABI gave no failure channel for. Public because the runtime's own
    /// shell-side shortcuts (a `CloseApplet(TRUE)` with no idle applet) belong in the same report.
    ///
    /// Said once, however many times it happens. The same blit of the same rectangle is made on
    /// every frame a game draws, so a report that repeated it once per call would be thousands of
    /// identical lines with every other line buried under them — the call log already folds its
    /// repeats, and this is the same rule for the things the ABI leaves no channel to report.
    ///
    /// What that costs is the count: a note now means "this happened" rather than "this happened n
    /// times". The count is a `(String, u32)` away if anything ever needs it, and nothing does yet.
    ///
    /// The list is bounded, because the dedup below is an *exact* match and the keys carry the rects
    /// and object addresses a note is about — so a session that draws its way through a hundred
    /// slightly different rectangles writes a hundred notes. Oldest are dropped first, which keeps the
    /// tail: that is where a run that has just gone
    /// wrong is, and [`Display::notes_dropped`] says the list stopped being the whole record.
    pub fn note(&mut self, what: impl Into<String>) {
        let what = what.into();
        if self.quiet.iter().any(|seen| *seen == what) {
            return;
        }
        if self.quiet.len() >= Self::NOTES {
            self.notes_dropped += self.quiet.drain(..Self::NOTES / 2).count() as u64;
        }
        self.quiet.push(what);
    }

    /// The designators that stand on a fresh display already holding a face, as the `IFont` object
    /// behind each. A handset's display has its system faces bound before an applet touches it, and
    /// that is not a detail: `IDisplay_SetFont` promises "if successful, returns the old IFont
    /// associated with nFont" and reserves `NULL` for *failure*, so a swap that answers nothing
    /// tells the caller the installation failed. A pack reads exactly that return value, treats 0 as
    /// failure, releases the font it was given and abandons its own start-up — never drawing a frame.
    ///
    /// All six system designators this host can draw with are bound. The three italic ones point
    /// at the upright classes of the same weight and size, because this host has no slanted cut;
    /// [`crate::font::Face`] answers them with the upright face. The two user slots stay unbound:
    /// nothing in this host has a face for them, and a designator with nothing behind it is the
    /// same answer the drawing path gives.
    const BOUND_FACES: &'static [(u32, u32)] = &[
        (g::AEE_FONT_NORMAL, g::AEECLSID_FONTSYSNORMAL),
        (g::AEE_FONT_BOLD, g::AEECLSID_FONTSYSBOLD),
        (g::AEE_FONT_LARGE, g::AEECLSID_FONTSYSLARGE),
        (g::AEE_FONT_ITALIC, g::AEECLSID_FONTSYSITALIC),
        (g::AEE_FONT_LARGE_ITALIC, g::AEECLSID_FONTSYSLARGEITALIC),
        (g::AEE_FONT_BOLD_ITALIC, g::AEECLSID_FONTSYSBOLDITALIC),
    ];

    /// A carrier build's `IDisplay` is not the eight-byte handle the public header describes. Its
    /// constructor at `0x159b24` builds a singleton with an embedded mode block at `+0xc` and a
    /// pointer at `+0x70` to the screen-info record the display driver filled in: `0x156cd0` writes
    /// `strh #0x78,[+4]`, `strh #0x92,[+6]`, `str #framebuffer,[+0xc]`. Packs built for that
    /// firmware read those words directly instead of calling `GetDeviceBitmap`, so the handle has
    /// to carry the same shape — a pack that dereferences the missing `+0x70` reads a null.
    const PRIVATE_INFO_WORD: u32 = 0x70;
    const PRIVATE_INFO: u32 = 0x74;
    const PRIVATE_END: u32 = 0x84;

    /// Fill the firmware-private half of a display object. `state` may be the fresh one, a clone's,
    /// or one whose destination just changed; the screen-info record follows the destination so a
    /// pack reading the framebuffer pointer sees the surface the display currently draws into.
    fn publish_private(&self, mem: &mut Mem, disp: u32, state: &DisplayState) -> HResult<()> {
        let width = u32::from(self.device.width);
        let height = u32::from(self.device.height);
        let px = self
            .bitmap(state.dest)
            .map(|b| b.px)
            .unwrap_or(self.device.px);
        // The embedded block the firmware's constructor fills: width and height as the first
        // halfwords, the mode word at +8, and the depth at +0xe.
        mem.write_u32(disp + 0x0c, width | (height << 16))?;
        mem.write_u32(disp + 0x14, 3)?;
        mem.write_u16(disp + 0x1a, u32::from(self.device.depth))?;
        mem.write_u32(disp + Self::PRIVATE_INFO_WORD, disp + Self::PRIVATE_INFO)?;
        mem.write_u32(disp + Self::PRIVATE_INFO, 0)?;
        mem.write_u32(
            disp + Self::PRIVATE_INFO + 4,
            width | (height << 16),
        )?;
        mem.write_u32(disp + Self::PRIVATE_INFO + 0x0c, px)?;
        Ok(())
    }

    /// `IShell::CreateInstance` gives every caller its own reference, so a fresh object is built
    /// per display rather than shared.
    pub fn create_display(&mut self, mem: &mut Mem, hle: &mut Hle) -> HResult<u32> {
        let addr = hle.new_object(
            mem,
            "IDisplay",
            g::AEECLSID_DISPLAY,
            Self::PRIVATE_END,
            4,
            &[],
        )?;
        let device = self.device.obj;
        let mut fonts = Vec::new();
        for (designator, class) in Self::BOUND_FACES {
            fonts.push((
                *designator,
                hle.new_object(mem, "IFont", *class, 8, 4, &[])?,
            ));
        }
        let state = DisplayState {
            dest: device,
            clip: Rect::whole(self.device.width, self.device.height),
            colors: default_colors(),
            annunciators: 0,
            backlight: true,
            fonts,
            pending: 0,
        };
        self.publish_private(mem, addr, &state)?;
        self.put_state(addr, state);
        hle.change_refs(mem, device, 1)?;
        // The header defines the default display as "the one that is returned for
        // AEECLSID_DISPLAY", so the first one this host answers for is the one BREW holds a
        // reference on. A later `MakeDefault` moves that reference; creating a second display
        // deliberately does not.
        if self.default == 0 {
            self.set_default(mem, hle, addr)?;
        }
        Ok(addr)
    }

    /// `IDisplay::Clone`: a second handle with its own clip and colours on the same screen.
    pub fn clone_display(&mut self, mem: &mut Mem, hle: &mut Hle, disp: u32) -> HResult<u32> {
        let state = self.state(disp)?;
        let dest = state.dest;
        let addr = hle.new_object(
            mem,
            "IDisplay",
            g::AEECLSID_DISPLAY,
            Self::PRIVATE_END,
            4,
            &[],
        )?;
        self.publish_private(mem, addr, &state)?;
        self.put_state(addr, state);
        hle.change_refs(mem, dest, 1)?;
        Ok(addr)
    }

    /// Move one display's destination reference. `IDisplay_SetDestination` documents the rule for
    /// both ends of it — "IBITMAP_AddRef will be called on the new destination and IBITMAP_Release
    /// will be called on the old" — and every binding in this struct follows it, so the counts
    /// cannot drift when a display is rebound or destroyed.
    fn bind(
        &mut self,
        mem: &mut Mem,
        hle: &mut Hle,
        state: &mut DisplayState,
        want: u32,
    ) -> HResult<()> {
        if state.dest != want {
            self.drop_ref(mem, hle, state.dest)?;
            hle.change_refs(mem, want, 1)?;
            state.dest = want;
        }
        Ok(())
    }

    /// Drop one reference the host itself took. A reference taken by the host ends the same way the
    /// guest's `Release` slot ends one, so the last of them destroys the object: otherwise a
    /// rebind could leave a dead bitmap's pixels in `bitmaps` and its record dispatchable.
    fn drop_ref(&mut self, mem: &mut Mem, hle: &mut Hle, addr: u32) -> HResult<u32> {
        let n = hle.change_refs(mem, addr, -1)?;
        if n == 0 {
            self.forget(mem, hle, addr)?;
        }
        Ok(n)
    }

    /// Hand BREW's one default-display reference to `disp`, dropping the previous owner's reference
    /// and forgetting it if that was its last.
    ///
    /// The answer is the display's own **new reference count**: the body ends with
    /// `result = *(a1 + 4) + 1; *(a1 + 4) = result; return result;`. The header types the slot `void`,
    /// so only a pack that reads `r0` anyway can see it — which is still a reason to hand back the
    /// number the body computes rather than a bare zero. Naming the display that is already the
    /// default is the body's own no-op with a net of zero: it releases the previous owner (itself),
    /// installs it again and takes the reference back, so the count comes out where it started.
    fn set_default(&mut self, mem: &mut Mem, hle: &mut Hle, disp: u32) -> HResult<u32> {
        if self.default == disp {
            return Ok(hle.change_refs(mem, disp, 0)?);
        }
        let prev = std::mem::replace(&mut self.default, disp);
        let n = hle.change_refs(mem, disp, 1)?;
        if prev != 0 {
            self.drop_ref(mem, hle, prev)?;
        }
        Ok(n)
    }

    /// `IDisplay::MakeDefault`, which the header documents as moving BREW's own reference: "This
    /// causes BREW to release its reference on the old default IDisplay and AddRef this IDisplay."
    /// It is *not* a state reset — clip, colours and destination are the display's own business.
    pub fn make_default(&mut self, mem: &mut Mem, hle: &mut Hle, disp: u32) -> HResult<u32> {
        self.state(disp)?;
        self.set_default(mem, hle, disp)
    }

    /// `IDisplay::SetColor(item, rgb)` — returns the item's previous colour, or `RGB_NONE`.
    ///
    /// Two quirks the header documents and both matter: `rgb` may be *another item number*, in
    /// which case the item takes that one's colour; and `rgb == RGB_NONE` is a read-only probe.
    pub fn set_color(&mut self, disp: u32, item: u32, rgb: u32) -> HResult<u32> {
        // AEEClrItem is `int8` on ARM, so the guest's argument is sign extended.
        let item = (item as u8) as i8 as usize;
        let mut state = self.state(disp)?;
        // The gate is `if ((a2 - 1) <= 0xF)` and it has two halves the name does not: items are
        // `1..=16`, so **item 0 is not a colour at all** — the body answers `RGB_NONE` (the same 32
        // bits as its `-1`) and stores nothing. A made-up high item is answered the same way.
        if item < 1 || item >= state.colors.len() {
            return Ok(RGB_NONE);
        }
        let prev = state.colors[item];
        // And the new value is tested as `if (a3 != 0 && a3 <= 0x10)` before the alias, so `0` is not
        // "item zero's colour" but plain `RGB_BLACK`, stored as a colour; only `1..=16` is a reference
        // to another item. `RGB_NONE` falls through the `a3 != -1` test and leaves the table alone.
        if rgb != RGB_NONE {
            let max_sys = (g::CLR_SYS_LAST - 1) as usize;
            state.colors[item] = if rgb >= 1 && (rgb as usize) <= max_sys {
                state.colors.get(rgb as usize).copied().unwrap_or(prev)
            } else {
                rgb
            };
            self.put_state(disp, state);
        }
        Ok(prev)
    }

    pub fn color(&self, disp: u32, item: u32) -> HResult<u32> {
        let state = self.state(disp)?;
        Ok(state.colors.get(item as usize).copied().unwrap_or(RGB_NONE))
    }

    /// The `AEE_DEVICEITEM_SYS_COLORS_DISP1` answer: the array of `RGBVAL`s the header says is
    /// indexed by `AEEClrItem`. This is the table as the handset ships it, not one handle's
    /// overrides, because the device item names the device's display and not the caller's.
    pub fn oem_sys_colors_bytes() -> Vec<u8> {
        default_colors()
            .into_iter()
            .flat_map(|c| c.to_le_bytes())
            .collect()
    }

    /// `IDisplay::NotifyEnable(pcb)` — the documented behaviour stores nothing.
    ///
    /// What it does is forward to the device object this display wraps (record `+124`) and answer
    /// `AEE_EUNSUPPORTED` when there is none; this host has no such object, so the two answers a
    /// caller can get here are `AEE_EBADPARM` and `AEE_EUNSUPPORTED`, and neither leaves a
    /// registration behind. `Runtime::idisplay` says which delivered function that reading comes
    /// from.
    ///
    /// `IDisplay::IsEnabled` is the other half of this pair, and it answers `TRUE` always, on
    /// purpose: a display becomes disabled when something else takes the screen away from the
    /// applet, and this host runs one applet against no window manager. Answering anything else would
    /// tell a pack that its drawing goes nowhere when it does — the frame is a real buffer that the
    /// harness reads back.
    pub fn is_enabled(&self, disp: u32) -> HResult<u32> {
        let _ = self.state(disp)?;
        Ok(1)
    }

    pub fn set_annunciators(&mut self, disp: u32, val: u16, mask: u16) -> HResult<()> {
        let mut state = self.state(disp)?;
        state.annunciators = (state.annunciators & !mask) | (val & mask);
        self.put_state(disp, state);
        Ok(())
    }

    pub fn set_backlight(&mut self, disp: u32, on: u32) -> HResult<()> {
        let mut state = self.state(disp)?;
        state.backlight = on != 0;
        self.put_state(disp, state);
        Ok(())
    }

    /// `IDisplay::SetPrefs(po, pchSettings, cSettings)` — "change settings for a device bitmap",
    /// with the four names the bitmap-device declarations publishes: `w` screen width, `h` screen height, `c`
    /// colour scheme, `a` annunciators.
    ///
    /// `w` and `h` are the ones with behaviour, and they are the reason this slot is served at
    /// all: a pack whose layout is for a panel taller than the one it finds asks for that height
    /// here and then lays its frame out from the number it asked for. The header is explicit that
    /// the display "may not be able to accommodate all values of all parameters, so it is
    /// important that the app not assume that it received what it asked for" — so a size this host
    /// will not back is refused rather than accepted and dropped. Answering SUCCESS and keeping
    /// the old geometry is the failure shape this repository treats as the worst one: the caller
    /// reads "yes", clears the screen it believes it now has, and the write lands outside the
    /// buffer the host made.
    ///
    /// The list is applied whole or not at all. One pair this host cannot serve makes the call
    /// `AEE_EUNSUPPORTED` and changes nothing, because a half-applied list answers "your screen is
    /// 376 rows now" while quietly dropping the rest of what was asked.
    pub fn set_prefs(&mut self, mem: &mut Mem, hle: &mut Hle, text: &str) -> HResult<u32> {
        let mut width = u32::from(self.device.width);
        let mut height = u32::from(self.device.height);
        let mut resize = false;
        for pair in text.split(',') {
            // An empty element names nothing: `"w:120,"` is what a caller that appends a separator
            // per setting produces, and skipping it is not guessing at a value.
            if pair.is_empty() {
                continue;
            }
            let Some((name, value)) = pair.split_once(':') else {
                return Ok(g::AEE_EBADPARM);
            };
            match name {
                "w" | "h" => {
                    // `<empty>` is "use the default value" and `*` "the maximum meaningful value".
                    // Both name the firmware's own idea of the panel, and this host has no record
                    // of it once the screen has been resized, so they are refused rather than
                    // guessed at.
                    let Ok(n) = value.parse::<u32>() else {
                        return Ok(g::AEE_EUNSUPPORTED);
                    };
                    if n == 0 {
                        return Ok(g::AEE_EBADPARM);
                    }
                    let Ok(n) = u16::try_from(n) else {
                        return Ok(g::AEE_EUNSUPPORTED);
                    };
                    if name == "w" {
                        width = u32::from(n);
                    } else {
                        height = u32::from(n);
                    }
                    resize = true;
                }
                "c" => {
                    let Ok(scheme) = value.parse::<u32>() else {
                        return Ok(g::AEE_EUNSUPPORTED);
                    };
                    // The frame has one depth, fixed by the profile: naming it is the no-op the
                    // header describes, and any other scheme asks for a screen whose pixels this
                    // host does not have.
                    if scheme != device_scheme(self.device.depth) {
                        return Ok(g::AEE_EUNSUPPORTED);
                    }
                }
                // `a` says whether annunciators are shown at all. This host draws no annunciator
                // bar, so there is no state the name could set; any other name is not one the
                // header publishes.
                _ => return Ok(g::AEE_EUNSUPPORTED),
            }
        }
        if width.saturating_mul(height) > MAX_SCREEN_PIXELS {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        if resize {
            self.resize_device(mem, hle, width as u16, height as u16)?;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// Give the screen a device bitmap of `width` x `height`, at the frame's own depth.
    ///
    /// A size is a new object, not a longer one: `DIB_NewEx` allocates the header and the pixels as
    /// one block, so `pBmp` is inside the object and the object's span is the bitmap's span. Every
    /// `IDisplay` that was drawing into the old screen moves onto the new one, with the reference
    /// the header's "AddRef the new destination, Release the old" rule owes each end — and the old
    /// screen goes when its last display has let go, the same way any other bitmap does.
    fn resize_device(
        &mut self,
        mem: &mut Mem,
        hle: &mut Hle,
        width: u16,
        height: u16,
    ) -> HResult<()> {
        let old = self.device;
        if old.width == width && old.height == height {
            return Ok(());
        }
        let seed = Bitmap {
            obj: 0,
            px: 0,
            rgb: 0,
            cnt_rgb: 0,
            width,
            height,
            pitch: row_bytes(width, old.depth) as i32,
            depth: old.depth,
            trans: default_transparent(old.depth),
        };
        let (obj, px) = new_bitmap_object(mem, hle, &seed, self.dib_pixel_offset, true)?;
        let device = Bitmap { obj, px, ..seed };
        self.bitmaps.insert(obj, device);
        self.pixel_owner.insert(px, obj);
        self.device = device;

        // The screen's own reference, plus one per display that was drawing into it. Collected
        // first and dropped once at the end: releasing inside the loop could destroy the object a
        // later iteration still has to name.
        let mut owed = 1i32;
        let default = self.default;
        for addr in self.displays.keys().copied().collect::<Vec<_>>() {
            let Some(mut state) = self.displays.get(&addr).cloned() else {
                continue;
            };
            if state.dest == old.obj {
                hle.change_refs(mem, obj, 1)?;
                owed += 1;
                state.dest = obj;
            }
            // A clip that was the whole old screen is the whole new one. A clip a pack set itself
            // is the pack's, and a resize is not a reason to move it.
            if state.clip == Rect::whole(old.width, old.height) {
                state.clip = Rect::whole(width, height);
            }
            self.publish_private(mem, addr, &state)?;
            self.put_state(addr, state);
        }
        let left = hle.change_refs(mem, old.obj, -owed)?;
        if left == 0 {
            self.forget(mem, hle, old.obj)?;
        }
        let bg = self
            .displays
            .get(&default)
            .map(background)
            .unwrap_or(RGB_WHITE);
        self.clear_device(mem, bg)
    }

    /// `IDisplay::Update(po, bDefer)`, with both of the header's conditions honoured.
    ///
    /// **Which bitmap is the destination decides whether there is a screen at all**: `Update`
    /// "updates the screen *if the destination bitmap is the device bitmap*",
    /// so a pack that is composing into one of its own DIBs gets nothing here, as it would on a
    /// handset — and the header is explicit that without this call the drawing is not shown,
    /// which is why this host does not refresh on its own.
    ///
    /// **`bDefer` decides when.** TRUE posts the update "onto the queue of the UI task, thereby
    /// allowing all the drawings to be done before updating the screen" (`1219-1220`, and the
    /// `IDisplay_UpdateEx` block at `1172-1182` says the same thing with the flag spelled out), while
    /// FALSE refreshes now. Every call that comes through the header's own `IDisplay_Update(p)` macro
    /// arrives with TRUE — the wrapper hands the vtable a literal `TRUE` —
    /// so deferring is the common path and the counting stays honest by queueing one message per call
    /// and letting [`Display::flush_deferred`] spend them at the turn boundary.
    pub fn update(&mut self, mem: &Mem, disp: u32, defer: u32) -> HResult<()> {
        let mut state = self.state(disp)?;
        // The re-read happens whether or not this call is owed a screen: it is about *where the device
        // bitmap's pixels are*, which a module may repoint at any time, and no answer about refreshing
        // has any bearing on it.
        self.adopt_device_pixels(mem)?;
        if state.dest != self.device.obj {
            return Ok(());
        }
        if defer != 0 {
            state.pending += 1;
            self.put_state(disp, state);
            return Ok(());
        }
        self.present();
        Ok(())
    }

    /// The UI task taking what the pack posted: every queued `Update` gets its screen now, so the
    /// drawing that followed the call is in the frame it refreshes.
    pub fn flush_deferred(&mut self, mem: &Mem) -> HResult<()> {
        let waiting: Vec<(u32, u32)> = self
            .displays
            .iter()
            .filter(|(_, state)| state.pending > 0)
            .map(|(disp, state)| (*disp, state.pending))
            .collect();
        for (disp, messages) in waiting {
            let Some(mut state) = self.displays.get(&disp).cloned() else {
                continue;
            };
            state.pending = 0;
            self.put_state(disp, state);
            // The count is the caller's: `n` posted messages are `n` presentations, which is what
            // the report's `Presents` column has always meant. Only *when* the screen is taken moves.
            for _ in 0..messages {
                self.adopt_device_pixels(mem)?;
                self.present();
            }
        }
        Ok(())
    }

    /// Adopt wherever the module's device bitmap points now.
    ///
    /// The header own can repoint it with no host call to record the change; a host that never re-reads keeps
    /// presenting the buffer the module abandoned. The same licence applies to
    /// `IDisplay::BitBlt`, where [`Display::sync_bitmap`] already re-reads the struct on every call.
    fn adopt_device_pixels(&mut self, mem: &Mem) -> HResult<()> {
        let obj = self.device.obj;
        self.sync_bitmap(mem, obj)?;
        if let Some(live) = self.bitmaps.get(&obj).copied() {
            self.device = live;
        }
        Ok(())
    }

    /// `IBitmapDev::Update` — "This function copies the bitmap to its associated display
    /// synchronously" (the header's own description), and "has no effect if the bitmap is
    /// disabled".
    ///
    /// This is the face a pack presents through when it draws straight into the device bitmap
    /// rather than going by way of `IDisplay::Update`; answering SUCCESS without taking the screen
    /// leaves every pixel it drew unrecorded. `a guest module` presents 5539 times through this call and
    /// this host counted none of them, which reads as "0 screens" for a pack that was drawing the
    /// whole time.
    ///
    /// There is no bitmap address to check here: `QueryInterface` hands out `AEEIID_IBitmapDev`
    /// only when the bitmap it was asked on *is* the device bitmap ("It is only implemented for
    /// device bitmaps"), and what reaches this slot is that face's own object, not the bitmap. A
    /// call arriving here is already a call about the screen. The header's "no effect if the bitmap
    /// is disabled" needs no branch either: this host's device bitmap is enabled always, which is
    /// what both `IsEnabled` slots answer.
    ///
    /// The copy is synchronous — this call carries no `bUpdate` flag to defer with — so the screen
    /// is taken here rather than queued for the UI task.
    pub fn present_device_bitmap(&mut self) {
        self.present();
    }

    /// One presentation: the counter, and the mark of the step the screen was taken at.
    fn present(&mut self) {
        self.presentations += 1;
        self.screens_seen += 1;
        if self.screens.len() >= crate::hle::Hle::HISTORY {
            let drop = self.screens.len() - crate::hle::Hle::HISTORY / 2;
            self.screens.drain(..drop);
            // The window is what `report_story` numbers its screens from, and past this point the
            // oldest marks are gone: a run of four thousand presentations has no way to be told from
            // one of two thousand, so the count is kept separately and the report says so.
            self.screens_dropped += drop as u64;
        }
        self.screens.push(self.now);
    }

    /// Paint the destination with the display's background colour.
    ///
    /// `IShell::StartApplet` documents that the display is cleared before the applet is started, so
    /// this is the state a module expects to draw onto rather than whatever the previous applet left
    /// behind. It ignores the clip: clearing is the shell's business, not the applet's.
    pub fn clear(&mut self, mem: &mut Mem, disp: u32) -> HResult<()> {
        let state = self.state(disp)?;
        let dest = self.dest(&state)?;
        let bg = background(&state);
        self.fill(
            mem,
            &dest,
            Rect::whole(dest.width, dest.height),
            rgb_to_native(bg),
            g::AEE_RO_COPY,
        )
    }

    /// Paint the screen itself, with no `IDisplay` involved. The two callers are both shell-side:
    /// the boot state, and the clear before an applet that has not necessarily made a display yet.
    pub fn clear_device(&mut self, mem: &mut Mem, rgb: u32) -> HResult<()> {
        let dest = self.device;
        let native = rgb_to_native(rgb);
        self.fill(
            mem,
            &dest,
            Rect::whole(dest.width, dest.height),
            native,
            g::AEE_RO_COPY,
        )
    }

    pub fn get_font_metrics(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        nfont: u32,
        pn_ascent: u32,
        pn_descent: u32,
    ) -> HResult<u32> {
        self.state(disp)?;
        let Some(font) = self.face(nfont) else {
            return Ok(g::AEE_EUNSUPPORTED);
        };
        for (p, v) in [(pn_ascent, font.ascent()), (pn_descent, font.descent())] {
            if p != 0 {
                mem.write_u32(p, v)?;
            }
        }
        Ok(font.height())
    }

    /// The `AECHAR` units behind a text argument. `nChars < 0` means count to the terminator; the
    /// 4096 cap is there so a wild pointer *or* a garbage length costs a bounded read, not the run.
    fn read_text(mem: &Mem, text: u32, nchars: i32) -> HResult<Vec<u16>> {
        if text == 0 {
            return Ok(Vec::new());
        }
        let cap = if nchars < 0 { 4096 } else { nchars as usize }.min(4096);
        Ok(mem.read_u16str(text, cap)?)
    }

    /// The device's 8-bit string behind a `DrawText` argument on a BREW 1.x handset. `nChars < 0`
    /// means count to the NUL; the same 4096 cap bounds a wild pointer or a garbage length.
    fn read_text_bytes(mem: &Mem, text: u32, nchars: i32) -> HResult<Vec<u8>> {
        if text == 0 {
            return Ok(Vec::new());
        }
        let cap = if nchars < 0 { 4096 } else { nchars as usize }.min(4096);
        let mut out = Vec::new();
        for i in 0..cap {
            let b = mem.read_u8(text + i as u32)?;
            if b == 0 {
                break;
            }
            out.push(b);
        }
        Ok(out)
    }

    /// `IDisplay::MeasureTextEx(nFont, pcText, nChars, nMaxWidth, pnFits)`.
    ///
    /// A negative `nMaxWidth` measures the whole string; otherwise `pnFits` receives the longest
    /// prefix that fits and the return value is that prefix's width, which is the pair a layout loop
    /// iterates on. Fitting is per character rather than per word — the widths are exact for
    /// whatever this reports as fitting, and a module that wants word-wrapped text gets the greedy
    /// line break a handset with the same face and no dictionary would give it.
    #[allow(clippy::too_many_arguments)]
    pub fn measure_text(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        nfont: u32,
        text: u32,
        nchars: i32,
        max_width: i32,
        pn_fits: u32,
    ) -> HResult<u32> {
        self.state(disp)?;
        let Some(font) = self.face(nfont) else {
            return Ok(0);
        };
        Self::measure_face(mem, &font, text, nchars, max_width, pn_fits, self.encoding)
    }

    /// The measuring loop, for a caller that has **no display object**. Both reference font bodies are
    /// display-free -- the delivered body walks the face's own range and width tables, and
    /// The delivered body
    /// `IFont::MeasureText` is answered by this function rather than by [`Display::measure_text`],
    /// which first asks whether the handle it was given is a live display.
    #[allow(clippy::too_many_arguments)]
    pub fn measure_face(
        mem: &mut Mem,
        face: &Face,
        text: u32,
        nchars: i32,
        max_width: i32,
        pn_fits: u32,
        enc: TextEncoding,
    ) -> HResult<u32> {
        let units = Self::read_text(mem, text, nchars)?;
        let text = enc.units_to_string(&units);
        let mut width = 0u32;
        let mut fits = 0u32;
        // Measured a character at a time rather than `count * advance`: the two faces are different
        // widths, and a run that mixes them is measured by walking it. The trailing gap is taken
        // off the prefix exactly as [`Face::measure_text`] would for the same prefix.
        let mut total = 0u32;
        let trailing = face.trailing_leading();
        for (n, ch) in text.chars().enumerate() {
            let step = face.advance_of_char(ch, enc);
            total += step;
            let w = total.saturating_sub(step.min(trailing));
            if max_width >= 0 && w > max_width as u32 {
                break;
            }
            width = w;
            fits = n as u32 + 1;
        }
        if pn_fits != 0 {
            mem.write_u32(pn_fits, fits)?;
        }
        Ok(width)
    }

    #[allow(clippy::too_many_arguments)]
    /// `IFont::DrawText` — the same glyphs as [`draw_text`], with every decision `IDisplay` keeps in its
    /// own state arriving as an argument instead.
    ///
    /// The header draws the line between the two interfaces sharply: "All
    /// parameters to `IFont_DrawText()` should be valid. Invalid or special values are not interpreted to
    /// have special meanings, as in `IDisplay`." So `x`/`y` are the top-left of the left-most cell and
    /// nothing else — no centring, no alignment bits — the two colours are `NativeColor`s the caller
    /// already converted (, "not RGBVALs"), and the clip is the caller's rectangle alone rather
    /// than an intersection with a display state. What is shared is the part that must not differ.
    ///
    /// ⚠ A negative `prcClip` width or height draws nothing — that is "no area", which is not the
    /// same as a clip that happens to exclude the text, and it is answered `SUCCESS` because the caller
    /// asked for nothing and nothing is what happened.
    ///
    /// `AEE_EUNSUPPORTED` is the header's own answer for a bitmap the font cannot work with;
    /// this host's faces plot straight into a pixel buffer, so the one thing that can refuse here is a
    /// face that is not one of ours.
    pub fn draw_text_into(
        &mut self,
        mem: &mut Mem,
        face: &Face,
        dst: u32,
        text: u32,
        nchars: i32,
        x: i32,
        y: i32,
        fg: u32,
        bg: u32,
        prc: u32,
        flags: u32,
    ) -> HResult<u32> {
        let font = face;
        let b = self
            .bitmap(dst)
            .ok_or(HleError::NotAnObject { addr: dst })?;
        let clip = if prc == 0 {
            Rect::whole(b.width, b.height)
        } else {
            Rect::read(mem, prc)?
        };
        if clip.is_empty() {
            return Ok(g::AEE_SUCCESS);
        }
        let units = Self::read_text(mem, text, nchars)?;
        let text = self.encoding.units_to_string(&units);
        if text.is_empty() {
            return Ok(g::AEE_SUCCESS);
        }
        let width = font.measure_text(&text, self.encoding) as i32;
        let height = font.height() as i32;
        let signed = |v: i32| v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        let cell = Rect::new(signed(x), signed(y), signed(width), signed(height));
        // ⚠ "Clipping affects whether or not pixels are drawn; clipping never affects where things are
        // drawn" — so the *area* comes from the clip while the pen stays at `x`, and text
        // that starts left of or above the bitmap (legal) keeps its own character positions.
        //
        // The clip is the caller's rectangle alone, and it need not intersect the bitmap
        // — a pack can clip to the screen while drawing into a smaller bitmap. `put` drops a pixel
        // outside the bitmap anyway, so intersecting here changes nothing that gets painted; what it
        // changes is whether the ink below is read for a run that cannot land on a single pixel.
        let area = match cell
            .clipped(clip)
            .and_then(|r| r.clipped(Rect::whole(b.width, b.height)))
            .and_then(|r| r.span())
        {
            Some(span) => span,
            None => return Ok(g::AEE_SUCCESS),
        };
        // The two colours are `NativeColor`s the caller converted — values this
        // bitmap stores, so indices when it keeps a table, and reading one is a guest memory access
        // through `pRGB`. Neither is read here: `bg` is only painted when the flag below asks for it,
        // and `fg` is resolved at the first pixel a glyph actually puts down. A run of blanks, or one
        // whose glyphs fall wholly outside `area`, draws nothing — and a call that draws nothing must
        // not fault on a table entry it never used, which is the rule `draw_pixel`, `fill_rect`,
        // `h_scanline` and `set_pixels` already keep at their doors.
        let mut ink = Ink::stored(fg);
        // "When `IFONT_DRAW_TRANSPARENT` is specified, only the foreground (the glyph) is drawn and the
        // background is not filled. Otherwise, the background area of each character cell is also drawn."
        // Undefined flag bits are ignored rather than refused.
        if flags & g::IFONT_DRAW_TRANSPARENT == 0 {
            let bg = b.colour_of_pixel(mem, bg)?;
            self.fill(
                mem,
                &b,
                cell.clipped(clip).unwrap_or(Rect::EMPTY),
                bg,
                g::AEE_RO_COPY,
            )?;
        }
        let (ax0, ay0, ax1, ay1) = area;
        let mut pen = x;
        // One memo for the whole run: every glyph of it is drawn in the same ink, so a palettized
        // destination pays the table search once rather than once per glyph.
        let mut memo = IndexMemo::new();
        for ch in text.chars() {
            let origin = (pen, y);
            pen += font.advance_of_char(ch, self.encoding) as i32;
            let key = font.glyph_key(ch, self.encoding).unwrap_or(u16::MAX);
            self.paint_glyph(
                mem,
                &b,
                font,
                key,
                origin,
                &mut ink,
                (ax0, ay0, ax1, ay1),
                &mut memo,
            )?;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// One character's ink, plotted at `origin`.
    ///
    /// The three faces differ only in how wide a row is and how its bits are packed, so the loop that
    /// walks a character is shared by `draw_text` (which answers `IDisplay`'s slot) and `draw_text_into`
    /// (which answers `IFont`'s). One copy is not tidiness: both interfaces have to agree on which pixels
    /// make up a glyph, and they are drawing the same glyphs.
    fn paint_glyph(
        &mut self,
        mem: &mut Mem,
        dest: &Bitmap,
        face: &Face,
        ch: u16,
        origin: (i32, i32),
        ink: &mut Ink,
        area: (i32, i32, i32, i32),
        memo: &mut IndexMemo,
    ) -> HResult<()> {
        match face {
            Face::Builtin(font) => {
                self.paint_narrow(mem, dest, *font, ch, origin, ink, area, memo)?
            }
            Face::Bbf(face) => self.paint_bbf(mem, dest, face, ch, origin, ink, area, memo)?,
        }
        Ok(())
    }

    /// One glyph from the built-in 5x8 grid, magnified by its designator's scale.
    fn paint_narrow(
        &mut self,
        mem: &mut Mem,
        dest: &Bitmap,
        font: font::Font,
        ch: u16,
        origin: (i32, i32),
        ink: &mut Ink,
        area: (i32, i32, i32, i32),
        memo: &mut IndexMemo,
    ) -> HResult<()> {
        // The narrow face's rows are five bits in a byte; the leftmost pixel is in the highest bit
        // that is part of the row.
        match font.shape(ch) {
            font::Shape::Narrow(rows) => {
                for (ry, bits) in rows.iter().enumerate() {
                    self.plot_row(
                        mem,
                        dest,
                        origin,
                        ry as u32,
                        *bits as u32,
                        GLYPH_W as u32,
                        font.scale,
                        ink,
                        area,
                        memo,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// One glyph from a BBF cell.
    ///
    /// A BBF cell is a rectangle of arbitrary width, so it is plotted one pixel at a time rather
    /// than through `plot_row`'s single-word row. The cell's own top-left is the bitmap coordinate
    /// `glyph` carries; `origin` is where the character starts on the destination. A cell is
    /// already the size it was drawn at, so there is no magnification here: `AEE_FONT_LARGE` is a
    /// face rasterized at its own size.
    #[allow(clippy::too_many_arguments)]
    fn paint_bbf(
        &mut self,
        mem: &mut Mem,
        dest: &Bitmap,
        face: &crate::bbf::Bbf,
        ch: u16,
        origin: (i32, i32),
        ink: &mut Ink,
        area: (i32, i32, i32, i32),
        memo: &mut IndexMemo,
    ) -> HResult<()> {
        let (glyph, width) = face.ink(ch);
        let (ax0, ay0, ax1, ay1) = area;
        for row in 0..glyph.height {
            for col in 0..width {
                if !face.bit(glyph.x.saturating_add(col), glyph.y.saturating_add(row)) {
                    continue;
                }
                let px = origin.0 + col as i32;
                let py = origin.1 + row as i32;
                if px < ax0 || px > ax1 || py < ay0 || py > ay1 {
                    continue;
                }
                // Resolved here, not at the call's door: a cell with no ink never reaches this
                // line, and on an indexed destination resolving the ink is a read of `pRGB`.
                let ink = ink.of(dest, mem)?;
                self.put(mem, dest, px, py, ink, g::AEE_RO_COPY, Some(&mut *memo))?;
            }
        }
        Ok(())
    }

    /// `IDisplay::DrawText(nFont, pcText, nChars, x, y, prcBackground, dwFlags)` — the glyph engine,
    /// which is what [`crate::font`] exists for.
    ///
    /// The units are decoded once, here, into the text the drawing works in. Everything below —
    /// alignment, clipping, underline, the report — sees characters rather than units, and a face
    /// keyed by `AECHAR` units is reached through [`Face::glyph_key`] with the same encoding.
    ///
    /// Four documented rules shape the implementation more than the plotting does:
    ///
    /// - `nChars < 0` computes the length, and `x`/`y` name the top-left of the rectangle the text
    ///   goes *into* — ignored entirely once the matching alignment flag is set.
    /// - A null `prcBackground` means the destination bitmap's bounds, and that rectangle is both the
    ///   clip and the box alignment is measured against. Drawing is clipped to its intersection with
    ///   the display's own clip rectangle, which the header is careful to call out as a second,
    ///   separate rectangle.
    /// - Text blocks what is behind it unless `IDF_TEXT_TRANSPARENT`, and `IDF_TEXT_INVERTED` swaps
    ///   the two colours rather than complementing pixels.
    /// - `IDF_ALIGN_FILL` and `IDF_ALIGN_SPREAD` are declared unsupported by the header itself, so
    ///   they take the positional answer rather than being guessed at.
    ///
    /// The return value is a **result code**, not a count: `IDisplay_DrawText` "returns SUCCESS /
    /// error code" and `IFont_DrawText` spells the same thing out.
    ///
    /// **This used to answer the number of characters drawn** — a reading in which zero meant "drew
    /// nothing" — and that inverts the contract at both ends: a draw that worked came back with a
    /// non-zero value (an error code, to a caller that checks one) while a run that could not be
    /// drawn at all came back `SUCCESS`. The count a reader wants is [`Display::runs_seen`], which the
    /// report already prints.
    ///
    /// Two early returns stay `AEE_SUCCESS` on purpose, and the asymmetry with the font case is the
    /// point: empty text, and a rectangle that clips everything away, are *legal* outcomes — the
    /// header says a negative-sized clip rectangle simply draws nothing — while a
    /// font id this host has no face for means the call could not be carried out at all.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        nfont: u32,
        text: u32,
        nchars: i32,
        x: i32,
        y: i32,
        prc_bg: u32,
        flags: u32,
    ) -> HResult<u32> {
        let enc = self.encoding;
        let text = if self.text_bytes {
            let bytes = Self::read_text_bytes(mem, text, nchars)?;
            enc.units_to_string(&enc.bytes_to_units(&bytes))
        } else {
            let units = Self::read_text(mem, text, nchars)?;
            enc.units_to_string(&units)
        };
        self.draw_text_impl(mem, disp, nfont, &text, x, y, prc_bg, flags, enc)
    }

    /// The same drawing for text the host already holds as Unicode — an `IHtmlViewer` page.
    ///
    /// Host text does not cross the guest boundary. Writing it into an `AECHAR` buffer and
    /// decoding it again would turn every character the device cannot name into a box, and the
    /// face installed under a designator is keyed by the code point it was rasterized from.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_unicode(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        nfont: u32,
        text: &str,
        x: i32,
        y: i32,
        prc_bg: u32,
        flags: u32,
    ) -> HResult<u32> {
        self.draw_text_impl(
            mem,
            disp,
            nfont,
            text,
            x,
            y,
            prc_bg,
            flags,
            TextEncoding::Unicode,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text_impl(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        nfont: u32,
        text: &str,
        x: i32,
        y: i32,
        prc_bg: u32,
        flags: u32,
        enc: TextEncoding,
    ) -> HResult<u32> {
        let Some(font) = self.face(nfont) else {
            self.note(format!(
                "no built-in face for AEEFont {nfont:#08x}; nothing was drawn, and the call answers \
                 an error code"
            ));
            return Ok(g::AEE_EUNSUPPORTED);
        };
        let state = self.state(disp)?;
        let dest = self.dest(&state)?;
        if text.is_empty() {
            return Ok(0);
        }
        let bounds = if prc_bg == 0 {
            Rect::whole(dest.width, dest.height)
        } else {
            Rect::read(mem, prc_bg)?
        };
        let width = font.measure_text(text, enc) as i32;
        let height = font.height() as i32;
        let signed = |v: i32| v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        let left = match flags & g::IDF_ALIGNHORZ_MASK {
            g::IDF_ALIGN_LEFT => i32::from(bounds.x),
            g::IDF_ALIGN_CENTER => i32::from(bounds.x) + (i32::from(bounds.dx) - width) / 2,
            g::IDF_ALIGN_RIGHT => i32::from(bounds.x) + i32::from(bounds.dx) - width,
            _ => x,
        };
        let top = match flags & g::IDF_ALIGNVERT_MASK {
            g::IDF_ALIGN_TOP => i32::from(bounds.y),
            g::IDF_ALIGN_MIDDLE => i32::from(bounds.y) + (i32::from(bounds.dy) - height) / 2,
            g::IDF_ALIGN_BOTTOM => i32::from(bounds.y) + i32::from(bounds.dy) - height,
            _ => y,
        };
        // The two clip rectangles, intersected: what the caller boxed the text into, and what the
        // display is allowed to touch at all.
        let area = match bounds.clipped(state.clip).and_then(|r| r.span()) {
            Some(span) => span,
            None => return Ok(0),
        };
        let (ax0, ay0, ax1, ay1) = area;
        if prc_bg != 0 {
            let frame = state
                .colors
                .get(g::CLR_USER_LINE as usize)
                .copied()
                .unwrap_or(RGB_BLACK);
            let bg = background(&state);
            let rect = Rect::new(bounds.x, bounds.y, bounds.dx, bounds.dy);
            if flags & g::IDF_RECT_FILL != 0 {
                self.fill(mem, &dest, rect, rgb_to_native(bg), g::AEE_RO_COPY)?;
            }
            if flags & g::IDF_RECT_FRAME != 0 {
                let native = rgb_to_native(frame);
                self.h_line(mem, &dest, ay0, ax0, ax1, native)?;
                self.h_line(mem, &dest, ay1, ax0, ax1, native)?;
                self.v_line(mem, &dest, ax0, ay0, ay1, native)?;
                self.v_line(mem, &dest, ax1, ay0, ay1, native)?;
            }
            if flags & g::IDF_RECT_INVERT != 0 {
                self.invert_rect(mem, &dest, rect)?;
            }
        }
        let ink = state
            .colors
            .get(g::CLR_USER_TEXT as usize)
            .copied()
            .unwrap_or(RGB_BLACK);
        let bg = background(&state);
        let (ink, cell) = if flags & g::IDF_TEXT_INVERTED != 0 {
            (bg, ink)
        } else {
            (ink, bg)
        };
        if flags & g::IDF_TEXT_TRANSPARENT == 0 {
            let box_ = Rect::new(signed(left), signed(top), signed(width), signed(height))
                .clipped(state.clip)
                .unwrap_or(Rect::EMPTY);
            self.fill(mem, &dest, box_, rgb_to_native(cell), g::AEE_RO_COPY)?;
        }
        // `IDisplay`'s ink arrives as a colour (the header's `RGBVAL`), already resolved — the
        // same `Ink` carries it so both text slots plot through one shape.
        let mut ink = Ink::colour(rgb_to_native(ink));
        let mut memo = IndexMemo::new();
        let mut pen = left;
        for ch in text.chars() {
            let origin = (pen, top);
            pen += font.advance_of_char(ch, enc) as i32;
            let key = font.glyph_key(ch, enc).unwrap_or(u16::MAX);
            // The three faces are plotted by the same loop: each yields a list of (row, bit-count,
            // row-bits) that the magnification below walks. The narrow Latin face's rows are five
            // bits in a byte, the CJK faces' are eight and sixteen bits, so in every case the bit
            // order is the same - leftmost pixel in the highest bit that is part of the row.
            self.paint_glyph(
                mem,
                &dest,
                &font,
                key,
                origin,
                &mut ink,
                (ax0, ay0, ax1, ay1),
                &mut memo,
            )?;
        }
        if flags & (g::IDF_TEXT_UNDERLINE | g::IDF_TEXT_DASHED_UNDERLINE) != 0 {
            let dashed = flags & g::IDF_TEXT_DASHED_UNDERLINE != 0;
            let baseline = top + font.ascent() as i32;
            for i in 0..width {
                if dashed && (i / (2 * font.scale() as i32)) % 2 == 1 {
                    continue;
                }
                let px = left + i;
                if px < ax0 || px > ax1 || baseline < ay0 || baseline > ay1 {
                    continue;
                }
                let ink = ink.of(&dest, mem)?;
                self.put(
                    mem,
                    &dest,
                    px,
                    baseline,
                    ink,
                    g::AEE_RO_COPY,
                    Some(&mut memo),
                )?;
            }
        }
        self.runs_seen += 1;
        if self.runs.len() >= crate::hle::Hle::HISTORY {
            // Oldest first, so what a report shows is the end of the run — which is where a frame that
            // stopped looking right actually is. `runs_seen` is the number that was really drawn.
            let drop = self.runs.len() - crate::hle::Hle::HISTORY / 2;
            self.runs.drain(..drop);
        }
        self.runs.push(TextRun {
            dest: state.dest,
            font: nfont,
            text: text.to_string(),
            x: left,
            y: top,
            width: width as u32,
            height: font.height(),
            flags,
            step: self.now,
        });
        Ok(g::AEE_SUCCESS)
    }

    /// Plot one magnified row of a glyph, clipped to the area.
    ///
    /// Both faces walk the same way - the leftmost pixel of a row in the highest bit that belongs to
    /// it - so one function serves the five-bit face and the sixteen-bit one; only `cols` differs.
    /// The magnification is integer, so `AEE_FONT_LARGE` is the same face rather than a resampled
    /// one: every source pixel becomes a `scale` x `scale` block.
    #[allow(clippy::too_many_arguments)]
    fn plot_row(
        &mut self,
        mem: &mut Mem,
        dest: &Bitmap,
        origin: (i32, i32),
        row: u32,
        bits: u32,
        cols: u32,
        scale: u32,
        ink: &mut Ink,
        area: (i32, i32, i32, i32),
        memo: &mut IndexMemo,
    ) -> HResult<()> {
        let (ax0, ay0, ax1, ay1) = area;
        for rx in 0..cols {
            if bits & (1 << (cols - 1 - rx)) == 0 {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = origin.0 + (rx * scale + sx) as i32;
                    let py = origin.1 + (row * scale + sy) as i32;
                    if px < ax0 || px > ax1 || py < ay0 || py > ay1 {
                        continue;
                    }
                    // Resolved at the first pixel that survives the clip. A blank row, or one
                    // wholly outside the area, never gets here — and on an indexed destination
                    // resolving the ink is a read of `pRGB` the call does not need.
                    let ink = ink.of(dest, mem)?;
                    self.put(mem, dest, px, py, ink, g::AEE_RO_COPY, Some(&mut *memo))?;
                }
            }
        }
        Ok(())
    }

    /// The device framebuffer as 8-bit RGB, top row first — the bytes behind a headless screenshot.
    /// Read from *guest* memory, because that is where the pixels are.
    ///
    /// This runs **every frame a game draws** and sits inside `session`'s turn, so it is the one
    /// piece of host work charged to every presentation. `240x320` is 76 800 pixels and 230 400
    /// bytes; the row-batched path turns 76 800 page lookups into roughly one per row.
    /// [`Display::screenshot_pixels`] is kept both as the non-native fallback and as the oracle the
    /// tests compare against. The time this host spends on its own drawing path is a separate
    /// measurement and has no bearing on that oracle.
    pub fn screenshot(&self, mem: &Mem) -> HResult<Vec<u8>> {
        if self.device.depth == NATIVE_DEPTH {
            self.screenshot_rows(mem)
        } else if is_888(self.device.depth) {
            self.screenshot_rows32(mem)
        } else {
            self.screenshot_pixels(mem)
        }
    }

    /// Read one row at a time. `Bitmap::at` walks rows by `pitch`, which the header allows to be
    /// negative (bottom-up) or wider than the image, so the row address is still asked of the guest
    /// for every y; only the pixels *within* a row stop paying for a page-table lookup each.
    /// `Mem::read_into` keeps the same paging and watch rules as `read_u16`, including a halfword
    /// that straddles a page boundary.
    fn screenshot_rows(&self, mem: &Mem) -> HResult<Vec<u8>> {
        let (wide, high) = (self.device.width as usize, self.device.height as usize);
        let mut out = vec![0u8; wide * high * 3];
        let mut row = vec![0u8; wide * 2];
        for y in 0..high {
            mem.read_into(self.device.row_start(y as u32), &mut row)?;
            for x in 0..wide {
                let px = u16::from_le_bytes([row[x * 2], row[x * 2 + 1]]);
                let (r, g, b) = unpack565(px);
                let at = (y * wide + x) * 3;
                out[at] = r;
                out[at + 1] = g;
                out[at + 2] = b;
            }
        }
        Ok(out)
    }

    /// [`Display::screenshot_rows`] for a 32-bpp device: four bytes a pixel, `0x00RRGGBB`, read one
    /// row at a time. The row is the unit for the same reason — inside it the pixels are contiguous,
    /// so the page-table lookup is once per row instead of once per pixel.
    fn screenshot_rows32(&self, mem: &Mem) -> HResult<Vec<u8>> {
        let (wide, high) = (self.device.width as usize, self.device.height as usize);
        let mut out = vec![0u8; wide * high * 3];
        let mut row = vec![0u8; wide * 4];
        for y in 0..high {
            mem.read_into(self.device.row_start(y as u32), &mut row)?;
            for x in 0..wide {
                let at = x * 4;
                let to = (y * wide + x) * 3;
                out[to] = row[at + 2];
                out[to + 1] = row[at + 1];
                out[to + 2] = row[at];
            }
        }
        Ok(out)
    }

    /// The reference path: one pixel at a time, read the way the blitter reads it.
    ///
    /// Also the fallback for a device bitmap whose depth is not native RGB565. A pixel there is an
    /// *index* into the bitmap's own `IDIB::pRGB`, not a colour, so reading row bytes as 5-6-5
    /// fabricates one — which is what this did: an 8-bpp screen's capture was pairs of neighbouring
    /// indices read as a red, a green and a blue, while the blitter (`read_pixel`, which has always
    /// gone through the palette) painted the same screen correctly. Two readers of one bitmap, and
    /// only one of them knew what a pixel was.
    fn screenshot_pixels(&self, mem: &Mem) -> HResult<Vec<u8>> {
        let b = &self.device;
        let (wide, high) = (b.width as usize, b.height as usize);
        let mut out = vec![0u8; wide * high * 3];
        // Decoded once per capture rather than once per pixel: the table lives in guest memory behind
        // the page map, so an 8-bpp screen would otherwise pay 76 800 reads of it to produce one
        // picture. `colour_of` is still the only decoder — this caches its answers, and a palette
        // cannot change under a single capture.
        //
        // An entry that cannot be read fails the capture, the same rule the blitter gets from
        // `read_pixel`: a pointer the guest wrote that does not resolve is a fact about that guest,
        // and answering it with a colour would be the one thing this host does not do with bad
        // addresses.
        //
        // This failure surface is *new*, and worth saying out loud. It used to be that a palettized
        // device with an unreadable `pRGB` was caught earlier: `sync_bitmap` resolved `ncTransparent`
        // through that same table before every `IBitmap` slot, so the fault arrived on the `Update`
        // that preceded any drawing. Since the transparency key moved to the stored-value space
        // (§7.3d) that resolution is gone — correctly, because the key is an index and never needed
        // the palette — so a capture is now the first thing that reads all 256 entries and says so.
        let mut palette = [0 as Native; 256];
        // `cnt_rgb` — the header's own count, and the same function `default_palette` builds from —
        // rather than `1 << depth`: `depth` is a public field of a struct the guest writes, and a
        // value this host does not back must not turn into a slice long enough to panic on.
        let entries = usize::from(cnt_rgb(b.depth));
        for (i, slot) in palette[..entries].iter_mut().enumerate() {
            *slot = b.colour_of(mem, i as u8)?;
        }
        for y in 0..high {
            for x in 0..wide {
                let native = match b.depth {
                    // A 0-bpp DIB has no pixels, which `read_pixel` says as well: black, rather than
                    // whatever the word at `pBmp` happens to be.
                    0 => 0,
                    8 | 4 | 1 => palette[usize::from(b.index_at(mem, x as u32, y as u32)?)],
                    32 => {
                        let v = mem.read_u32(b.at(x as u32, y as u32))?;
                        let at = (y * wide + x) * 3;
                        out[at] = ((v >> 16) & 0xff) as u8;
                        out[at + 1] = ((v >> 8) & 0xff) as u8;
                        out[at + 2] = (v & 0xff) as u8;
                        continue;
                    }
                    _ => mem.read_u16(b.at(x as u32, y as u32))?,
                };
                let (r, g, bl) = unpack565(native as u16);
                let at = (y * wide + x) * 3;
                out[at] = r;
                out[at + 1] = g;
                out[at + 2] = bl;
            }
        }
        Ok(out)
    }

    /// [`Display::screenshot`] in a container any viewer opens without a dependency.
    pub fn to_bmp(&self, mem: &Mem) -> HResult<Vec<u8>> {
        let rgb = self.screenshot(mem)?;
        Ok(crate::bmp::encode(
            self.device.width,
            self.device.height,
            &rgb,
        ))
    }

    /// `IDisplay::SetClipRect(prc)`. The stored clip is the intersection of `prc` with the
    /// destination bitmap; a null `prc` is the documented way back to the whole of it.
    pub fn set_clip_rect(&mut self, mem: &Mem, disp: u32, prc: u32) -> HResult<()> {
        let mut state = self.state(disp)?;
        let whole = {
            let b = self.dest(&state)?;
            Rect::whole(b.width, b.height)
        };
        // The header defines the new clip as the *intersection* of `prc` and the destination
        // bitmap, and a non-positive or non-overlapping rectangle as a Null one — so what
        // `GetClipRect` reports is exactly what drawing will honour, not what was passed in.
        state.clip = if prc == 0 {
            whole
        } else {
            Rect::read(mem, prc)?.clipped(whole).unwrap_or(Rect::EMPTY)
        };
        self.put_state(disp, state);
        Ok(())
    }

    pub fn get_clip_rect(&mut self, mem: &mut Mem, disp: u32, prc: u32) -> HResult<()> {
        let clip = self.state(disp)?.clip;
        if prc != 0 {
            clip.write(mem, prc)?;
        }
        Ok(())
    }

    /// The clip a display is drawing under, for a caller that has to put it back.
    ///
    /// [`Self::set_clip_rect`] replaces the clip rather than narrowing it, so a host-side painter
    /// that clips to its own rectangle — which is what a control does on a handset — can save this,
    /// set its own, and restore exactly what it found.
    pub fn clip_of(&self, disp: u32) -> HResult<Rect> {
        Ok(self.state(disp)?.clip)
    }

    /// `IDisplay::SetDestination(pDst)` / `GetDestination()`.
    ///
    /// The display holds a reference to its destination: without the `AddRef` a module that sets a
    /// bitmap and then drops its own reference would leave the display drawing into a freed
    /// object.
    pub fn set_destination(
        &mut self,
        mem: &mut Mem,
        hle: &mut Hle,
        disp: u32,
        dst: u32,
    ) -> HResult<u32> {
        let mut state = self.state(disp)?;
        let want = if dst == 0 { self.device.obj } else { dst };
        let b = self
            .bitmap(want)
            .ok_or(HleError::NotAnObject { addr: want })?;
        if state.dest != want {
            self.bind(mem, hle, &mut state, want)?;
            // A clip inherited from a bitmap of another size would silently hide the new frame,
            // so the destination and its clip change together.
            state.clip = Rect::whole(b.width, b.height);
            self.publish_private(mem, disp, &state)?;
            self.put_state(disp, state);
        }
        Ok(g::AEE_SUCCESS)
    }

    pub fn destination(&self, disp: u32) -> HResult<u32> {
        Ok(self.state(disp)?.dest)
    }

    /// `IDisplay::DrawRect(prc, clrFrame, clrFill, dwFlags)`.
    ///
    /// `void`-returning, so the documented rules do all the error handling: a null rectangle with
    /// the fill flag means the whole clip, a null rectangle without it means nothing at all, and
    /// `RGB_NONE` means the active colour rather than "skip".
    /// `GetColor(po, color, nIndex)`: every drawing call's colour arguments arrive as
    /// `RGBVAL`s *or* as indexes into the display's `AEEClrItem` table, and only the index form is
    /// ambiguous. The documented behaviour says which is which in two rules:
    ///
    /// * `color == 0xFFFFFFFF` — `RGB_NONE` — asks for the **system** colour at `nIndex`, which the
    ///   caller chooses per call site (`DrawRect` names 3 for the frame and 2 for the fill).
    /// * A non-zero value `<= 0x10` is itself a table index, whatever `nIndex` says.
    ///
    /// Everything else is an `RGBVAL` and goes through untouched. This is why `DrawRect(rect, -1, 2,
    /// IDF_RECT_FILL)` means "fill with `CLR_USER_BACKGROUND`": reading the 2 as a colour instead
    /// paints `RGBVAL 0x00000002`, which is black — and a menu that repaints its own rectangle before
    /// every row came out as a black band.
    ///
    /// ⚠ `RGB_BLACK` is 0, so a caller asking for colour number zero gets black rather than
    /// `colors[0]`. That is the reference test (`a2 != 0`), not an oversight here: entry 0 of the table
    /// is unused by the header's own numbering.
    fn resolve_color(state: &DisplayState, color: u32, index: u32) -> u32 {
        let table = |i: u32| state.colors.get(i as usize).copied().unwrap_or(RGB_BLACK);
        if color == RGB_NONE && (1..=0x10).contains(&index) {
            table(index)
        } else if color != 0 && color <= 0x10 {
            table(color)
        } else {
            color
        }
    }

    pub fn draw_rect(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        prc: u32,
        clr_frame: u32,
        clr_fill: u32,
        flags: u32,
    ) -> HResult<()> {
        let flags = flags & g::IDF_RECT_MASK;
        // `state()` clones a whole `DisplayState` (two `Vec`s) for every call, and `DrawRect` is
        // the sample set' hottest display call — thousands of per 500 ms frame. Borrow the record once and
        // copy out only the `Copy` values this call needs: the drawing methods below take `&mut
        // self`, so the borrow must end here. The state and destination lookups still happen before
        // the early returns, exactly as they did when `state()`/`dest()` were called up front.
        let (dest, clip, fill_color, frame_color) = {
            let state = self
                .displays
                .get(&disp)
                .ok_or(HleError::NotAnObject { addr: disp })?;
            let dest = self
                .bitmap(state.dest)
                .ok_or(HleError::NotAnObject { addr: state.dest })?;
            (
                dest,
                state.clip,
                if flags & g::IDF_RECT_FILL != 0 {
                    Self::resolve_color(state, clr_fill, g::CLR_USER_BACKGROUND)
                } else {
                    0
                },
                if flags & g::IDF_RECT_FRAME != 0 {
                    Self::resolve_color(state, clr_frame, g::CLR_USER_LINE)
                } else {
                    0
                },
            )
        };
        if flags == 0 {
            return Ok(());
        }
        let given = if prc == 0 {
            None
        } else {
            Some(Rect::read(mem, prc)?)
        };
        if given.is_none() && flags & g::IDF_RECT_FILL == 0 {
            return Ok(());
        }
        let area = given.unwrap_or(clip).clipped(clip).unwrap_or(Rect::EMPTY);
        if area.is_empty() {
            return Ok(());
        }
        if flags & g::IDF_RECT_INVERT != 0 {
            self.invert_rect(mem, &dest, area)?;
            return Ok(());
        }
        if flags & g::IDF_RECT_FILL != 0 {
            self.fill(mem, &dest, area, rgb_to_native(fill_color), g::AEE_RO_COPY)?;
        }
        if flags & g::IDF_RECT_FRAME != 0 {
            let native = rgb_to_native(frame_color);
            let (x0, y0, x1, y1) = area.span().unwrap_or((0, 0, -1, -1));
            self.h_line(mem, &dest, y0, x0, x1, native)?;
            self.h_line(mem, &dest, y1, x0, x1, native)?;
            self.v_line(mem, &dest, x0, y0, y1, native)?;
            self.v_line(mem, &dest, x1, y0, y1, native)?;
        }
        Ok(())
    }

    /// `IDisplay::DrawFrame(prc, ft, rgbFill)` — the eleven styles listed by what they *look* like
    /// rather than by pixels: `NONE`/`EMPTY`/`3D_EMPTY` draw no frame
    /// at all, `RAISED`/`LOWERED` are a two-pixel bevel in one direction or the other, `BOX` is one
    /// pixel of one colour, `INDENT` is a one-pixel lowered bevel, and the four `TAB_*` styles are one
    /// pixel with the tab against an edge — the unselected two of which draw nothing.
    ///
    /// The two shades of a bevel come from the system colours the header names for exactly this —
    /// `CLR_SYS_FRAME_HI` and `CLR_SYS_FRAME_LO` ( shows a pack reading one out through
    /// `SetColor`) — and not from a formula invented here. "Raised" puts the light shade on the top
    /// and left; "lowered" puts it on the bottom and right. The inside is filled in `rgbFill` when the
    /// caller named one, `RGB_NONE` meaning "no fill" exactly as `draw_rect` reads it.
    ///
    /// ⚠ A corner pixel belongs to one edge or the other, and the header says nothing about which:
    /// this draws the two horizontal edges first and the two vertical ones after, so a corner is the
    /// vertical edge's. Stated here because a caller reading a corner back would otherwise have to
    /// guess which of the two shades it should be.
    pub fn draw_frame(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        prc: u32,
        ft: u8,
        fill: u32,
    ) -> HResult<()> {
        // (edge width in pixels, bevel direction — `Some(light_on_top_left)`, `None` for one colour —
        // and whether anything is drawn at all).
        let (width, bevel, draw) = match ft {
            0 | 1 | 2 => (0u16, None, false),
            3 => (2, Some(true), true),
            4 => (2, Some(false), true),
            5 => (1, None, true),
            6 => (1, Some(false), true),
            7 => (1, Some(false), true),
            8 => (0, None, false),
            9 => (1, Some(true), true),
            10 => (0, None, false),
            _ => (0, None, false),
        };
        if !draw || width == 0 {
            return Ok(());
        }
        let state = self.state(disp)?;
        let dest = self.dest(&state)?;
        let given = if prc == 0 {
            state.clip
        } else {
            Rect::read(mem, prc)?
                .clipped(state.clip)
                .unwrap_or(Rect::EMPTY)
        };
        let (mut x0, mut y0, mut x1, mut y1) = match given.span() {
            Some(span) => span,
            None => return Ok(()),
        };
        let line = state
            .colors
            .get(g::CLR_USER_LINE as usize)
            .copied()
            .unwrap_or(RGB_BLACK);
        let hi = state
            .colors
            .get(g::CLR_SYS_FRAME_HI as usize)
            .copied()
            .unwrap_or(line);
        let lo = state
            .colors
            .get(g::CLR_SYS_FRAME_LO as usize)
            .copied()
            .unwrap_or(line);
        for _ in 0..width as i32 {
            if x0 > x1 || y0 > y1 {
                break;
            }
            let (top, bottom, left, right) = match bevel {
                Some(true) => (hi, lo, hi, lo),
                Some(false) => (lo, hi, lo, hi),
                None => (line, line, line, line),
            };
            self.h_line(mem, &dest, y0, x0, x1, rgb_to_native(top))?;
            self.h_line(mem, &dest, y1, x0, x1, rgb_to_native(bottom))?;
            self.v_line(mem, &dest, x0, y0, y1, rgb_to_native(left))?;
            self.v_line(mem, &dest, x1, y0, y1, rgb_to_native(right))?;
            x0 += 1;
            y0 += 1;
            x1 -= 1;
            y1 -= 1;
        }
        if fill != RGB_NONE && x0 <= x1 && y0 <= y1 {
            let inside = Rect::new(
                x0 as i16,
                y0 as i16,
                (x1 - x0 + 1) as i16,
                (y1 - y0 + 1) as i16,
            );
            self.fill(mem, &dest, inside, rgb_to_native(fill), g::AEE_RO_COPY)?;
        }
        Ok(())
    }

    fn invert_rect(&mut self, mem: &mut Mem, b: &Bitmap, area: Rect) -> HResult<()> {
        let (x0, y0, x1, y1) = match area.span() {
            Some(s) => s,
            None => return Ok(()),
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                if is_888(b.depth) {
                    // A 32-bpp pixel is the `0x00RRGGBB` word itself, and the complement is taken
                    // in that space. Going through the canonical 565 colour first would drop eight
                    // bits of green and five of each primary before the inversion.
                    if let Some(v) = read_stored(mem, b, x, y)? {
                        write_pixel(mem, b, x, y, !v & 0x00ff_ffff)?;
                    }
                } else if let Some(v) = read_pixel(mem, b, x, y)? {
                    // A complement of the *colour*: `read_pixel` answers one at every depth — it
                    // resolves a palettized pixel through the bitmap's own table — so inverting it and
                    // storing the answer back through the same table is the whole operation. An
                    // `!index` would not be: it is a number in the index space, and the entry it names
                    // is whatever happens to sit at the far end of the table, usually past `cnt_rgb`.
                    // The answer is the entry nearest the complement, which is as close as a fixed
                    // table can come; when two entries are equidistant the first wins.
                    write_pixel(mem, b, x, y, b.stored(mem, !v & 0xffff)?)?;
                }
            }
        }
        Ok(())
    }

    fn fill(
        &mut self,
        mem: &mut Mem,
        b: &Bitmap,
        area: Rect,
        native: Native,
        rop: u32,
    ) -> HResult<()> {
        let (x0, y0, x1, y1) = match area.span() {
            Some(s) => s,
            None => return Ok(()),
        };
        // A transparent fill paints nothing at all — `case AEE_RO_TRANSPARENT: return SUCCESS;` in
        // the inline `FillRect` (that arm at) which
        // `OEMBitmap_FillRect` ends by calling. The slots that take a colour already answer
        // `SUCCESS` for that op at their doors, so this is the belt for a caller inside the host.
        // There is no per-pixel source here to compare against a key, and comparing the fill colour
        // against one was this host's reading, not the handset's.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(());
        }
        // The row path, for the same reason the blit has one: a fill is a single value written over
        // a rectangle, and the pixel loop below costs an address computation, a bounds check and a
        // page lookup per *pixel* of it. A row of a whole-colour bitmap is contiguous memory holding
        // the same two or four bytes repeated, so a whole row is one `write_bytes` — and this,
        // rather than the blit, is what the measurement found the emulator spending its wall clock
        // on: `clear` and `IDisplay::DrawRect` fill most of a screen several times a frame.
        let bytes = if is_888(b.depth) {
            4
        } else if b.depth == NATIVE_DEPTH {
            2
        } else {
            0
        };
        if bytes != 0 && rop == g::AEE_RO_COPY {
            let (xa, xb) = (x0.max(0), x1.min(b.width as i32 - 1));
            let (ya, yb) = (y0.max(0), y1.min(b.height as i32 - 1));
            if xa > xb || ya > yb {
                // A rectangle entirely outside the bitmap is the pixel loop's own answer: nothing,
                // and not an error — `put` skips a pixel it cannot address.
                return Ok(());
            }
            let stored = if bytes == 4 {
                native565_to_888(native)
            } else {
                native & 0xffff
            };
            let value = stored.to_le_bytes();
            let width = (xb - xa + 1) as usize;
            self.fill_row.resize(width * bytes, 0);
            for px in self.fill_row.chunks_exact_mut(bytes) {
                px.copy_from_slice(&value[..bytes]);
            }
            for y in ya..=yb {
                mem.write_bytes(b.at(xa as u32, y as u32), &self.fill_row)?;
            }
            return Ok(());
        }
        // One colour over the whole rectangle, so one search and then the memo — which also keeps a
        // full-screen fill of a palettized bitmap from costing a table walk per pixel.
        let mut memo = IndexMemo::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.put(mem, b, x, y, native, rop, Some(&mut memo))?;
            }
        }
        Ok(())
    }

    fn h_line(
        &mut self,
        mem: &mut Mem,
        b: &Bitmap,
        y: i32,
        x0: i32,
        x1: i32,
        native: Native,
    ) -> HResult<()> {
        let mut memo = IndexMemo::new();
        for x in x0..=x1 {
            self.put(mem, b, x, y, native, g::AEE_RO_COPY, Some(&mut memo))?;
        }
        Ok(())
    }

    fn v_line(
        &mut self,
        mem: &mut Mem,
        b: &Bitmap,
        x: i32,
        y0: i32,
        y1: i32,
        native: Native,
    ) -> HResult<()> {
        let mut memo = IndexMemo::new();
        for y in y0..=y1 {
            self.put(mem, b, x, y, native, g::AEE_RO_COPY, Some(&mut memo))?;
        }
        Ok(())
    }

    /// One pixel, clipped and raster-op'd. Off-screen coordinates are dropped, not an error: BREW
    /// has no way to report them from the void slots and an applet is expected to over-draw.
    fn put(
        &mut self,
        mem: &mut Mem,
        b: &Bitmap,
        x: i32,
        y: i32,
        native: Native,
        rop: u32,
        memo: Option<&mut IndexMemo>,
    ) -> HResult<()> {
        if x < 0 || y < 0 || x >= b.width as i32 || y >= b.height as i32 {
            return Ok(());
        }
        let (x, y) = (x as u32, y as u32);
        // The two-operand ops are the reason a write has to read: a handset's blitter combines the
        // source pixel with whatever is standing in the destination, so the destination is an input
        // and not something to be overwritten. Outside those it is not read, which keeps the common
        // copy at one access per pixel.
        let needs = matches!(
            rop,
            g::AEE_RO_OR | g::AEE_RO_XOR | g::AEE_RO_AND | g::AEE_RO_MERGENOT | g::AEE_RO_ANDNOT
        );
        // A transparent write paints nothing, and there is no key to compare here. Every slot whose
        // "source" is a colour argument answers `SUCCESS` for `AEE_RO_TRANSPARENT` at its own door —
        // the inline `FillRect`, `_DrawPixel` and `_DrawHScanline`
        // each hold a `case AEE_RO_TRANSPARENT: return SUCCESS;`
        // — and a blit between two guest bitmaps filters on the *source's* key in `source_pixel` /
        // `copy_rows` before it ever gets here. Nothing in this host reaches this branch today —
        // those slots answer at their doors above, and `Graphics::stretch` writes with `COPY` — so it
        // is a guard rather than a path. It stays so that a future colour-argument slot cannot quietly
        // bring back the "fill the key colour" bug.
        //
        // Comparing the fill colour against this bitmap's own key was this host's reading of the op,
        // and the published description says otherwise. It is what painted `IBITMAP_Invalidate`'s argument
        // black instead of leaving the rectangle alone.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(());
        }
        if b.keeps_indices() {
            // One space for the whole pixel, and it is the one the pixel array is written in.
            // The header `DST = (!SRC)*`), not on intensities, and the header says a bitmap with no
            // palette maps pixel values literally — so on a palettized bitmap an index is what takes
            // part in the operation. Combining colours and then hunting the nearest entry gives a
            // different answer: index 1 and 2 are red and blue, `1 ^ 2` is 3, while `red ^ blue` is
            // magenta, and which entry that lands on depends on the table. It also used to make `COPY`
            // exact and `XOR` an approximation inside the same function, which is the shape this whole
            // change is about.
            let src = match memo {
                Some(memo) => memo.index(b, mem, native)?,
                None => b.stored(mem, native)?,
            };
            let dst = if needs {
                read_stored(mem, b, x as i32, y as i32)?.unwrap_or(0)
            } else {
                0
            };
            let Some(v) = raster(rop, src, dst) else {
                self.note(format!("unsupported raster op {rop}"));
                return Ok(());
            };
            // `write_pixel` keeps the value inside the pixel's own width — `& 0x0f`, `& 1`, or the
            // byte — which is what makes `NOT` a complement of the index rather than of the word.
            write_pixel(mem, b, x as i32, y as i32, v)?;
            return Ok(());
        }
        // At 16 bpp and above a pixel is a whole colour word, so the operands go in their stored
        // space: 565 for a 16-bpp bitmap, `0x00RRGGBB` for a 32-bpp one. The memo is the caller's,
        // and only a caller drawing many indexed pixels has one — see [`IndexMemo`] for why the
        // search it spares cannot outlive the operation.
        let src = b.stored(mem, native)?;
        let dst = if needs {
            read_stored(mem, b, x as i32, y as i32)?.unwrap_or(0)
        } else {
            0
        };
        let Some(v) = (if is_888(b.depth) {
            raster_word(rop, src, dst)
        } else {
            raster(rop, src, dst)
        }) else {
            self.note(format!("unsupported raster op {rop}"));
            return Ok(());
        };
        write_pixel(mem, b, x as i32, y as i32, v)?;
        Ok(())
    }

    /// Attach the transparency a decoded picture named to the bitmap it was laid out into.
    ///
    /// Called by the loader, not by anything the pack can reach: this is information the *file*
    /// carries, and no `IBitmap` slot lets an app set or clear it. `None` is the opaque answer and
    /// puts the bitmap back on the batched row path.
    pub fn set_alpha(&mut self, obj: u32, map: Option<AlphaMap>) -> HResult<()> {
        self.bitmaps
            .get(&obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        match map {
            Some(m) => {
                self.alphas.insert(obj, Arc::new(m));
            }
            None => {
                self.alphas.remove(&obj);
            }
        }
        Ok(())
    }

    /// The coverage map a picture was loaded with, or `None` when it is safe to ignore it.
    ///
    /// The width test is not paranoia: [`Display::sync_bitmap`] re-reads `cx`/`cy` from guest memory
    /// before every blit because packs do restate their own `IDIB` headers, and a map indexed by the
    /// width it was made for would then put row `y` of one picture on row `y` of another. A mismatch
    /// is dropped and said — a host that painted the wrong pixels transparent would be the same
    /// unreadable result as one that painted none.
    pub fn picture_alpha(&mut self, obj: u32, width: usize) -> Option<Arc<AlphaMap>> {
        let map = self.alphas.get(&obj).cloned()?;
        if map.width == width {
            return Some(map);
        }
        self.note(format!(
            "ignoring the transparency of bitmap {obj:#010x}: it is {width} pixels wide now, the map \
             was made for {} — the pack restated this IDIB's own geometry",
            map.width
        ));
        None
    }

    /// The raster op this host would use for a picture of this bitmap, which is the op the header
    /// says the decoder picks on the pack's behalf (COPY for opaque,
    /// TRANSPARENT or BLEND for a transparent image, "depending on the transparency type").
    pub fn default_rop(&self, obj: u32) -> u32 {
        match self.alphas.get(&obj) {
            None => g::AEE_RO_COPY,
            Some(map) if map.partial() => g::AEE_RO_BLEND,
            Some(_) => g::AEE_RO_TRANSPARENT,
        }
    }

    /// Lay one source pixel down, composing it with whatever stands there when the file says the
    /// pixel only partly covers the destination.
    ///
    /// `a` is the coverage [`source_pixel`] read off the picture's own alpha. Once that coverage has
    /// been applied a `AEE_RO_BLEND` has done everything the header asks of it — "the alpha channel
    /// in the source should be used in copying the source color to the destination"
    /// — so what is left is a plain copy. The word-level table in [`raster`] has no
    /// entry for BLEND and adding one would be inventing a number the ABI never defines, because
    /// blending is not an operation on packed 565 words.
    fn lay(
        &mut self,
        mem: &mut Mem,
        dst: &Bitmap,
        x: i32,
        y: i32,
        native: Native,
        a: u8,
        rop: u32,
        memo: Option<&mut IndexMemo>,
    ) -> HResult<()> {
        // Resolved at the door, for *every* coverage including a fully opaque pixel: the table in
        // [`raster`] has no BLEND row, so a BLEND handed down with an `a == 255` pixel would be
        // noted and dropped — the solid pixels of an alpha picture vanishing, which is worse than
        // the white box this started from. `IPARM_GETROP` and the pack's own choice are unaffected.
        let rop = if rop == g::AEE_RO_BLEND {
            g::AEE_RO_COPY
        } else {
            rop
        };
        if a == u8::MAX {
            return self.put(mem, dst, x, y, native, rop, memo);
        }
        let standing = read_pixel(mem, dst, x, y)?.unwrap_or(0);
        self.put(mem, dst, x, y, blend(native, standing, a), rop, memo)
    }

    /// The rectangular copy behind `IBitmap::BltIn`, `BltOut` and `IDisplay::BitBlt`. No scaling:
    /// a handset blits one source pixel per destination pixel, and a source rectangle that reaches
    /// past its own bitmap simply contributes nothing for the pixels it cannot name.
    #[allow(clippy::too_many_arguments)]
    fn copy(
        &mut self,
        mem: &mut Mem,
        dst: &Bitmap,
        xdst: i32,
        ydst: i32,
        dx: i32,
        dy: i32,
        src: &Bitmap,
        xsrc: i32,
        ysrc: i32,
        rop: u32,
        clip: Option<Rect>,
    ) -> HResult<()> {
        // Read once per blit: the map belongs to the source picture, and holding an `Arc` here is
        // what lets the loops below consult it while `self` is busy writing. It is also what decides
        // whether `AEE_RO_BLEND` can be carried out at all — see the gate below.
        let alpha = self.picture_alpha(src.obj, src.width as usize);
        // `rop_supported` is the word-level table: ops that combine two packed 565 words. BLEND is
        // not one of those — it composites by the source's coverage, which lives in `alpha` — so it
        // is allowed exactly when the source names a coverage, and still said out loud when the
        // caller asked for a blend of a picture that has no alpha to blend with.
        if !rop_supported(rop) && !(rop == g::AEE_RO_BLEND && alpha.is_some()) {
            self.note(format!("unsupported raster op {rop}"));
            return Ok(());
        }
        if dx <= 0 || dy <= 0 {
            return Ok(());
        }
        if src.width == 0 || src.height == 0 {
            // Every source coordinate is then out of bounds and the blit paints nothing while
            // answering success — the one silence worse than a refusal, because the caller sees a
            // drawn frame and an empty screen with nothing to explain it.
            self.note(format!(
                "blit of {dx}x{dy} from a {}x{} source bitmap",
                src.width, src.height
            ));
            return Ok(());
        }
        if xsrc >= src.width as i32 || ysrc >= src.height as i32 || xsrc + dx <= 0 || ysrc + dy <= 0
        {
            // The same silence, reached a different way: a game indexing a sprite sheet past its
            // own extent asks for a rectangle the source cannot name, and contributes nothing.
            self.note(format!(
                "blit of a {dx}x{dy} rectangle at ({xsrc},{ysrc}) from a {}x{} source bitmap",
                src.width, src.height
            ));
            return Ok(());
        }
        // `int` coordinates against a 16-bit clip rectangle: intersect wide, then clamp to the
        // destination's own extent, so a partly-off-screen blit draws what it can reach.
        let shown = clip;
        // Which of the two "no clip" cases this is, is the whole question. `None` is a blit nobody
        // clipped — `IBitmap::BltIn`/`BltOut` are not display operations — and the destination's own
        // extent is the right area. `Some(rect)` with no area is the **Null rectangle**
        // `IDisplay_SetClipRect` describes, and the header is explicit about what it costs: "the
        // resultant clipping rectangle is a Null rectangle (0,0,0,0) and subsequent IDisplay
        // operations will not affect the destination bitmap". Falling back
        // to the destination for *that* one drew over the exact region the pack had asked to have
        // hidden, and agreed with nothing else in this file: `IDisplay::DrawRect` and the text path
        // both take the same empty clip and paint nothing. (`IBitmap::FillRect` does not — it is not a
        // display operation, and clips only against its own bitmap.)
        let clip = match shown.map(|c| c.span()) {
            Some(Some(area)) => area,
            Some(None) => (1, 1, 0, 0),
            None => (0, 0, dst.width as i32 - 1, dst.height as i32 - 1),
        };
        let (x0, y0) = (xdst.max(clip.0).max(0), ydst.max(clip.1).max(0));
        let (x1, y1) = (
            (xdst + dx - 1).min(clip.2).min(dst.width as i32 - 1),
            (ydst + dy - 1).min(clip.3).min(dst.height as i32 - 1),
        );
        if x0 > x1 || y0 > y1 {
            if let Some(clip) = shown {
                // The third way a blit paints nothing and says nothing. The two above blame the
                // source; this one blames the destination's clip, which is the difference between
                // "the pack asked for a tile it does not have" and "the pack asked for a part of the
                // screen it had already clipped away" — and a screen that is black because of the
                // second of those reads exactly like a screen that is black because of the first.
                //
                // The destination's own size rides along because an empty clip has two causes and only
                // one of them is the pack's: a `0x0` here means the *host* handed that display a
                // bitmap with no pixels, so its clip is empty forever and every blit into it is dropped
                // — which is a picture that never moves, and is ours to fix, not the pack's. Without
                // this number the two causes print the same line.
                self.note(format!(
                    "blit of a {dx}x{dy} rectangle at ({xdst},{ydst}) outside the clip \
                     ({},{},{}x{}) of a {}x{} destination",
                    clip.x, clip.y, clip.dx, clip.dy, dst.width, dst.height
                ));
            }
            return Ok(());
        }
        // The row path: a source and destination of the same whole-colour depth — 16 or 32 — one whole
        // row at a time.
        //
        // The loop below costs an address computation, a bounds check and a page lookup per *pixel* —
        // twice over for a transparent blit, which reads the destination too — and on a pack that
        // blits a 105x113 sprite ten times a frame that is most of the emulator's wall clock
        // A row is
        // contiguous memory in both bitmaps, so the same work per *row* is one `read_into` and one
        // `write_bytes`, and what is left per pixel is a comparison in a buffer.
        //
        // Conditions, each checked rather than assumed:
        //   * both bitmaps have the same 16- or 32-bit depth, which makes a pixel exactly two or
        //     four bytes and keeps a 32→32 copy from rounding through the 565 colour space;
        //   * the source coordinates start inside the picture. A blit *larger* than its own source
        //     is legal — `read_stored` answers None past the picture's edge and the pixel loop
        //     simply skips those coordinates — so the batched path clips the shown rectangle to
        //     that same intersection instead of refusing the whole call. `x0 >= xdst` and
        //     `y0 >= ydst` make the surviving area a contiguous span of rows, and `copy_rows`
        //     derives the source address from `x0 - xdst`, so it starts at exactly the same source
        //     pixel and just stops at the edge. For a source that already contained the request,
        //     `src_right >= x1` and `src_bottom >= y1`: the clip is the identity, so widening the
        //     gate cannot change any answer that used to take the row path.
        //   * the two bitmaps share no memory, because reading a whole row before writing it is the
        //     same thing as the pixel loop only when the write cannot reach a pixel not yet read.
        //   * an alpha map is read per pixel, which is the one thing the row path cannot do; an
        //     opaque source — every BMP and every PNG that names no transparency — keeps the
        //     batched path, and that is the case the sample set runs on.
        // i64 throughout: a wild `xdst` / `xsrc` from the guest must clamp, not wrap, before it is
        // compared with the already-clipped `x1`/`y1`.
        let clamp = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let src_right = clamp(xdst as i64 + src.width as i64 - xsrc as i64 - 1);
        let src_bottom = clamp(ydst as i64 + src.height as i64 - ysrc as i64 - 1);
        let (rx1, ry1) = (x1.min(src_right), y1.min(src_bottom));
        if src.depth == dst.depth
            && matches!(src.depth, 16 | 32)
            && matches!(rop, g::AEE_RO_COPY | g::AEE_RO_TRANSPARENT)
            && alpha.is_none()
            && xsrc >= 0
            && ysrc >= 0
            && !share_memory(src, dst)
            && x0 <= rx1
            && y0 <= ry1
        {
            self.copy_rows(
                mem,
                dst,
                src,
                (x0, y0, rx1, ry1),
                (xdst, ydst, xsrc, ysrc),
                rop,
            )?;
            return Ok(());
        }
        // One memo for the whole pixel loop, and the reason it can be this cheap: a sprite drawn into a
        // palettized bitmap asks the same handful of colours over and over, so the second pixel of a
        // colour costs a hash and a compare instead of a walk through the table.
        let mut memo = IndexMemo::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let sx = xsrc + (x - xdst);
                let sy = ysrc + (y - ydst);
                let Some((native, a)) = source_pixel(mem, src, alpha.as_deref(), sx, sy, rop)?
                else {
                    continue;
                };
                let rop = if rop == g::AEE_RO_TRANSPARENT {
                    g::AEE_RO_COPY
                } else {
                    rop
                };
                self.lay(mem, dst, x, y, native, a, rop, Some(&mut memo))?;
            }
        }
        Ok(())
    }

    /// Paint a decoded picture at `(x, y)` on `disp`, one source pixel per destination pixel.
    ///
    /// `IHtmlViewer` is the caller: a document's `<img>` arrives as bytes, not as a guest `IBitmap`,
    /// so the pixels go through the same [`Display::lay`] a blit uses without building an object the
    /// guest would then have to release. `clip` is the display's own clip in `Rect` terms — `None`
    /// means the destination's own extent.
    pub fn draw_picture(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        at: (i32, i32),
        img: &crate::resource::Bitmap,
        clip: Option<Rect>,
    ) -> HResult<()> {
        // `disp` names a *display*, not a bitmap: the destination is the bitmap that display draws
        // into, which is the same [`Display::dest`] every other draw call reaches.
        let dst = match self.displays.get(&disp) {
            Some(state) => self.dest(state)?,
            None => return Ok(()),
        };
        if img.width == 0 || img.height == 0 {
            return Ok(());
        }
        let (x, y) = at;
        // The same intersection [`Display::copy`] performs: wide first, then the destination's own
        // extent. A `Some` clip with no area is the Null rectangle `SetClipRect` documents, and
        // paints nothing.
        let (cx, cy, cw, ch) = match clip {
            Some(rect) => (rect.x as i32, rect.y as i32, rect.dx as i32, rect.dy as i32),
            None => (0, 0, dst.width as i32, dst.height as i32),
        };
        if cw <= 0 || ch <= 0 {
            return Ok(());
        }
        let x0 = x.max(cx).max(0);
        let y0 = y.max(cy).max(0);
        let x1 = (x + img.width as i32 - 1)
            .min(cx + cw - 1)
            .min(dst.width as i32 - 1);
        let y1 = (y + img.height as i32 - 1)
            .min(cy + ch - 1)
            .min(dst.height as i32 - 1);
        if x0 > x1 || y0 > y1 {
            return Ok(());
        }
        let mut memo = IndexMemo::new();
        for py in y0..=y1 {
            for px in x0..=x1 {
                let (sx, sy) = ((px - x) as usize, (py - y) as usize);
                let (r, g, b) = img.rgb(sx, sy);
                let native = rgb_to_native(make_rgb(r, g, b));
                let a = img.alpha_of(sx, sy);
                self.lay(
                    mem,
                    &dst,
                    px,
                    py,
                    native,
                    a,
                    g::AEE_RO_BLEND,
                    Some(&mut memo),
                )?;
            }
        }
        Ok(())
    }

    /// [`Display::copy`]'s row path: the same result as the pixel loop, for the case the conditions
    /// above describe.
    fn copy_rows(
        &mut self,
        mem: &mut Mem,
        dst: &Bitmap,
        src: &Bitmap,
        shown: (i32, i32, i32, i32),
        from: (i32, i32, i32, i32),
        rop: u32,
    ) -> HResult<()> {
        let (x0, y0, x1, y1) = shown;
        let (xdst, ydst, xsrc, ysrc) = from;
        let width = (x1 - x0 + 1) as usize;
        let bytes = if is_888(dst.depth) { 4 } else { 2 };
        if rop == g::AEE_RO_COPY {
            // Both sides are guest memory and the gate above has already proved the two bitmaps do
            // not overlap. `Mem::copy_bytes` moves a row page-run to page-run: no staging `Vec`,
            // one page lookup per row instead of a read and a write, and half the bytes copied.
            // It is a memmove, honours watchpoints (that branch routes through a byte-wise staged
            // read) and keeps `touch_high` updated. A row that faults still leaves the rows above
            // it written, as the old loop did. One narrow difference: when one row straddles a page
            // and only its first page is mapped, `copy_bytes` can write the first run before
            // faulting, where `read_into` used to read the whole row before writing any of it.
            // Both report the fault; only a damaged IDIB pointing at an unmapped page sees it.
            for y in y0..=y1 {
                let s = src.at((xsrc + (x0 - xdst)) as u32, (ysrc + (y - ydst)) as u32);
                let d = dst.at(x0 as u32, y as u32);
                mem.copy_bytes(d, s, width * bytes)?;
            }
            return Ok(());
        }
        // A transparent blit has to merge the destination row into the source row, so it still
        // stages one row pair. One buffer pair for the whole blit: a row is the unit of work here,
        // and a `Vec` per row would put the allocator back into the loop whose per-pixel work just
        // came out of it.
        let mut row = vec![0u8; width * bytes];
        let mut keep = vec![0u8; width * bytes];
        for y in y0..=y1 {
            let s = src.at((xsrc + (x0 - xdst)) as u32, (ysrc + (y - ydst)) as u32);
            let d = dst.at(x0 as u32, y as u32);
            mem.read_into(s, &mut row)?;
            if rop == g::AEE_RO_TRANSPARENT {
                // A skipped pixel keeps what the destination held, so the destination row is read
                // once and merged into the source row — which is the same answer the pixel loop
                // gives by not writing it, and one pass instead of one read per pixel.
                mem.read_into(d, &mut keep)?;
                for i in 0..width {
                    // A stored word against a stored word: the two spaces coincide at 16 bpp, and
                    // at 32 bpp `trans` was kept as the full `0x00RRGGBB` word by `sync_bitmap`.
                    let pixel = if bytes == 4 {
                        u32::from_le_bytes([
                            row[4 * i],
                            row[4 * i + 1],
                            row[4 * i + 2],
                            row[4 * i + 3],
                        ])
                    } else {
                        u32::from(u16::from_le_bytes([row[2 * i], row[2 * i + 1]]))
                    };
                    if pixel == src.trans {
                        row[bytes * i..bytes * (i + 1)]
                            .copy_from_slice(&keep[bytes * i..bytes * (i + 1)]);
                    }
                }
            }
            mem.write_bytes(d, &row)?;
        }
        Ok(())
    }

    // ---- IBitmap surface ---------------------------------------------------------------

    /// `IBitmap::GetInfo(pinfo, nSize)`, where `nSize` is the *caller's* `sizeof(AEEBitmapInfo)`.
    ///
    /// The header spells out a version negotiation: the struct "may grow over time", a smaller
    /// `nSize` is an older caller that gets a shorter fill. **The documented behaviour does not do that.**
    /// `DIB_GetInfo` compares `nSize != sizeof(*pInfo)` and answers `EUNSUPPORTED`,
    /// so on 3.1.2 a caller built against a shorter struct gets no record at all — and this host
    /// answering a truncated one would be inventing an answer no handset gives.
    pub fn get_info(&self, mem: &mut Mem, obj: u32, pinfo: u32, nsize: u32) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if pinfo == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        let st = abi::struct_info("AEEBitmapInfo").expect("AEEBitmapInfo in the ABI tables");
        if nsize != st.size {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        let mut bytes = vec![0u8; st.size as usize];
        for (name, v) in [
            ("cx", u32::from(b.width)),
            ("cy", u32::from(b.height)),
            ("nDepth", u32::from(b.depth)),
        ] {
            let o = field_off("AEEBitmapInfo", name);
            bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        mem.write_bytes(pinfo, &bytes)?;
        Ok(g::AEE_SUCCESS)
    }

    pub fn draw_pixel(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        x: u32,
        y: u32,
        color: Native,
        rop: u32,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        // `draw_pixel_at` refuses an unsupported op before it writes; ask here as well, so that
        // refusal stays the answer even when the coordinate is out of bounds.
        if !rop_supported(rop) {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        // Nothing is drawn and nothing is read: `OEMBitmap_DrawPixel`'s switch answers
        // `case AEE_RO_TRANSPARENT: return SUCCESS;`. A
        // single-colour slot has no per-pixel source to compare against a key, so the colour — which
        // may name an unmapped palette entry — is never resolved.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(g::AEE_SUCCESS);
        }
        // `put` drops a pixel outside the bitmap (a coordinate above `i32::MAX` reaches it as a
        // negative one and is dropped the same way). Such a call cannot paint, so it must not read
        // `pRGB` on the way to being ignored — the same rule the scanline and point slots follow.
        if x >= u32::from(b.width) || y >= u32::from(b.height) {
            return Ok(g::AEE_SUCCESS);
        }
        // `IBitmap::DrawPixel`'s colour is a value this bitmap stores — an index when it keeps a
        // table — not a packed colour. See [`Bitmap::colour_of_pixel`].
        let color = b.colour_of_pixel(mem, color)?;
        // No cache offered, and none wanted: this entry draws the one pixel its caller asked for, so
        // a table search is paid once and there is no second pixel to spare it.
        self.draw_pixel_at(mem, obj, x, y, color, rop, None)
    }

    /// One pixel from a **colour**, for a caller that has no bitmap in hand: `IGraphics`.
    ///
    /// `IGraphics::SetColor` takes an `RGBVAL` and every drawing slot after it paints with what that
    /// resolved to, on whatever destination it was last pointed at — so the value arriving here is the
    /// same kind of word `IDisplay` draws with, and reading it as a stored pixel value (the way
    /// `IBitmap::DrawPixel`'s argument must be) would take the low byte of a colour and paint the
    /// palette entry it happens to name.
    ///
    /// `cache` belongs to the caller because the caller knows what this host cannot: that the pixels
    /// about to follow go to the same destination, and that nothing the guest can run sits between
    /// them. A shape asks this once per pixel with the same one, which is the difference between one
    /// palette search and one per pixel — about 64 ns against 1310 ns on a 256-entry table that
    /// does not name the colour, release, seven rounds taking the fastest.
    pub(crate) fn draw_pixel_colour(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        x: u32,
        y: u32,
        colour: Native,
        rop: u32,
        cache: &mut IndexMemo,
    ) -> HResult<u32> {
        self.draw_pixel_at(mem, obj, x, y, colour, rop, Some(cache))
    }

    /// The body both of the above share, in colours: `put` resolves whatever the destination stores.
    fn draw_pixel_at(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        x: u32,
        y: u32,
        colour: Native,
        rop: u32,
        cache: Option<&mut IndexMemo>,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if !rop_supported(rop) {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        // Passed straight through, not gated on `keeps_indices` here: `put` is the one place that
        // decides whether a pixel is an index, and it only asks the memo in that branch — so a
        // destination that stores colours never touches it, which is every shape on the screen.
        self.put(mem, &b, x as i32, y as i32, colour, rop, cache)?;
        Ok(g::AEE_SUCCESS)
    }

    pub fn get_pixel(&self, mem: &mut Mem, obj: u32, x: u32, y: u32, pcolor: u32) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if pcolor == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        // The value, not the colour: the header promises "the value of the specified
        // pixel" and hands it back through a `NativeColor*`, and the header defines that as
        // what the pixel array holds. Reading the colour here instead would break the loop the same
        // header describes — `RGBToNative` → `DrawPixel` → `GetPixel` — on every palettized bitmap:
        // draw index 3, read back the colour of entry 3.
        let Some(v) = read_stored(mem, &b, x as i32, y as i32)? else {
            return Ok(g::AEE_EBADPARM);
        };
        mem.write_u32(pcolor, v)?;
        Ok(g::AEE_SUCCESS)
    }

    pub fn fill_rect(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        prc: u32,
        color: Native,
        rop: u32,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if prc == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        if !rop_supported(rop) {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        let area = Rect::read(mem, prc)?.clipped(Rect::whole(b.width, b.height));
        // `AEE_RO_TRANSPARENT` paints nothing at all in this slot. The reference OEM bitmap says so in
        // one line — the inline `FillRect` at the header is `case
        // AEE_RO_TRANSPARENT: return SUCCESS;` — and that is what makes `IBITMAP_Invalidate`, which
        // expands to
        // `FillRect(po, prc, 0, AEE_RO_TRANSPARENT)`, a way to widen a dirty
        // rectangle rather than a way to paint the whole rectangle black.
        //
        // A fill has one colour for the entire rectangle, so there is no per-pixel source to compare
        // against a key: the handset answers by not drawing, and never looks at the colour. Returning
        // before `colour_of_pixel` is part of that answer — an `Invalidate` whose colour argument
        // names an unmapped palette entry still succeeds, and still reads nothing.
        //
        // The rectangle is still read, because the handset reads it too: `OEMBitmap_FillRect` takes
        // `pRect->x` before it ever looks at `rop`, which is also why the dirty-rectangle expansion
        // an `Invalidate` asks for happens on that path.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(g::AEE_SUCCESS);
        }
        let Some(area) = area else {
            // A rectangle entirely outside the bitmap: nothing to draw, and nothing to read either.
            return Ok(g::AEE_SUCCESS);
        };
        // As in the other `IBitmap` slots: the colour arrives as a value this bitmap stores.
        let color = b.colour_of_pixel(mem, color)?;
        self.fill(mem, &b, area, color, rop)?;
        Ok(g::AEE_SUCCESS)
    }

    /// `IBitmap::DrawHScanline(po, y, xMin, xMax, clr, rop)` — a horizontal run, inclusive of
    /// both endpoints.
    #[allow(clippy::too_many_arguments)]
    pub fn h_scanline(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        y: u32,
        x_min: u32,
        x_max: u32,
        color: Native,
        rop: u32,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if !rop_supported(rop) {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        // `OEMBitmap_DrawHScanline` ends in `return FillRect(pMe, xMin, y, xMax - xMin + 1, 1, color,
        // rop);`, so a transparent run takes `FillRect`'s
        // answer: nothing drawn, `SUCCESS`, colour never resolved.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(g::AEE_SUCCESS);
        }
        // An empty run, a row past the bottom, or a run that starts past the right edge paints
        // nothing — `put` would drop every pixel — and resolving the colour first would read `pRGB`
        // for a call that cannot use it. The same rule `fill_rect` applies through `Rect::clipped`,
        // only this slot has no rectangle to clip; the run is the rectangle, so it is clipped here.
        //
        // Clipping is not only about the colour read: `x_max` is a guest number, and `u32::MAX` is a
        // legal one. Looping `x_min..=x_max` meant the blitter walked four billion coordinates to
        // discard each of them; the clipped run is at most one row wide.
        let wide = u32::from(b.width);
        if x_min > x_max || y >= u32::from(b.height) || x_min >= wide {
            return Ok(g::AEE_SUCCESS);
        }
        // `x_min < wide` makes `wide - 1` safe, and a zero-width bitmap has no run to paint at all.
        let x_last = x_max.min(wide - 1);
        let color = b.colour_of_pixel(mem, color)?;
        let mut memo = IndexMemo::new();
        for x in x_min..=x_last {
            self.put(mem, &b, x as i32, y as i32, color, rop, Some(&mut memo))?;
        }
        Ok(g::AEE_SUCCESS)
    }

    pub fn set_pixels(
        &mut self,
        mem: &mut Mem,
        obj: u32,
        cnt: u32,
        points: u32,
        color: Native,
        rop: u32,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if points == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        if !rop_supported(rop) {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        // The one slot in this family that *refuses* transparency rather than ignoring it:
        // `OEMBitmap_SetPixels`' switch has arms only for COPY, XOR and BLEND, so
        // `AEE_RO_TRANSPARENT` falls into `default: return EUNSUPPORTED;`.
        //Answering `SUCCESS` here would tell a pack the
        // points were laid down when the handset says it cannot do it at all.
        if rop == g::AEE_RO_TRANSPARENT {
            return Ok(g::AEE_EUNSUPPORTED);
        }
        // Zero points is an empty loop, not a bad pointer: `points` has already been checked, and
        // reading the colour would touch `pRGB` for a call that writes no pixel.
        if cnt == 0 {
            return Ok(g::AEE_SUCCESS);
        }
        let (ox, oy) = (field_off("AEEPoint", "x"), field_off("AEEPoint", "y"));
        let stride = abi::struct_info("AEEPoint").map(|s| s.size).unwrap_or(4);
        // A list whose every point lands outside the bitmap paints nothing — `put` drops each of
        // them — so the colour is resolved after this probe rather than before it. Resolving a
        // `NativeColor` on an indexed bitmap is a read of `pRGB`, and a call that writes no pixel
        // must not fault on an entry it never uses. The points themselves are guest data the slot
        // was handed, so walking them is the call's own work; only the *table* read moves.
        //
        // One extra pass, but only over a list that is about to be walked anyway, and it stops at
        // the first point that will paint — the common case, where point 0 is on the bitmap, pays a
        // single read.
        let mut inside = false;
        for i in 0..cnt {
            let at = points + i * stride;
            let x = mem.read_u16(at + ox as u32)? as i16;
            let y = mem.read_u16(at + oy as u32)? as i16;
            if x >= 0
                && y >= 0
                && (x as u32) < u32::from(b.width)
                && (y as u32) < u32::from(b.height)
            {
                inside = true;
                break;
            }
        }
        if !inside {
            return Ok(g::AEE_SUCCESS);
        }
        let color = b.colour_of_pixel(mem, color)?;
        let mut memo = IndexMemo::new();
        for i in 0..cnt {
            let at = points + i * stride;
            let x = mem.read_u16(at + ox as u32)? as i16;
            let y = mem.read_u16(at + oy as u32)? as i16;
            self.put(mem, &b, x as i32, y as i32, color, rop, Some(&mut memo))?;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// `IBitmap::SetTransparencyColor`. The colour is already in this bitmap's own pixel space — an
    /// index where it has a palette — so it is stored and published exactly as given, which is the
    /// whole body of `DIB_SetTransparencyColor` (`m_bmp.ncTransparent = color`,
    /// then `SUCCESS`). Re-deriving it through `NativeToRGB`/`RGBToNative` looks harmless on a
    /// 16-bpp bitmap, where the two are inverses, but on a palettized one it turns an index into the
    /// nearest entry of the palette to *that index's colour*: a pack that did what the header tells
    /// it (ask `RGBToNative`, hand the answer back) would read a different number out of the public
    /// field than it wrote.
    ///
    /// The record holds the same number, in the same space, so there is nothing to derive in either
    /// direction: [`source_pixel`] and the batched row paths compare against it after resolving their
    /// operand to a stored value, which is the comparison the header asks for. Note where it
    /// is *not* read: a fill, a drawn pixel or a run of them paints nothing at all under
    /// `AEE_RO_TRANSPARENT` — the reference `FillRect`/`DrawPixel`/`DrawHScanline` each answer
    /// `SUCCESS` without touching a pixel — so this key belongs to the source side of a blit only.
    pub fn transparency(&mut self, mem: &mut Mem, obj: u32, color: Native) -> HResult<u32> {
        let mut b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        let at = abi::offset_of("IDIB", "ncTransparent").expect("IDIB is in the tables");
        mem.write_u32(obj + at, color)?;
        b.trans = if is_888(b.depth) {
            color
        } else {
            color & 0xffff
        };
        self.bitmaps.insert(obj, b);
        self.pixel_owner.insert(b.px, obj);
        Ok(g::AEE_SUCCESS)
    }

    pub fn get_transparency(&self, mem: &mut Mem, obj: u32, pcolor: u32) -> HResult<u32> {
        // The object still has to be one of ours; its record is not what gets answered.
        self.bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        if pcolor == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        // The public field, not the record: `DIB_GetTransparencyColor` answers with
        // `m_bmp.ncTransparent` — the number the packet wrote. The two now
        // agree in value, but the field is still the authority: a pack may restate `IDIB` without
        // going through any slot, and then the field is right and the record is stale until
        // `sync_bitmap` reads it again.
        let at = abi::offset_of("IDIB", "ncTransparent").expect("IDIB is in the tables");
        let v = mem.read_u32(obj + at)?;
        mem.write_u32(pcolor, v)?;
        Ok(g::AEE_SUCCESS)
    }

    /// `IBitmap::CreateCompatibleBitmap`, i.e. another buffer with the same depth — and, per the
    /// header, the same transparency colour.
    pub fn create_compatible(
        &mut self,
        mem: &mut Mem,
        hle: &mut Hle,
        obj: u32,
        width: u16,
        height: u16,
    ) -> HResult<u32> {
        let b = self
            .bitmap(obj)
            .ok_or(HleError::NotAnObject { addr: obj })?;
        let addr = self.new_bitmap(mem, hle, width, height, b.depth)?;
        // The delivered body
        // and the palette itself is carried over (`MEMMOVE(pRGB, pRGB, cntRGB * 4)`). Without this a
        // compatible surface of a palettized picture starts on this host's default ramp and paints
        // every index a different colour than the picture it was asked of.
        let made = self.bitmap(addr).ok_or(HleError::NotAnObject { addr })?;
        if b.rgb != 0 && made.rgb != 0 {
            for i in 0..u32::from(b.cnt_rgb.min(made.cnt_rgb)) {
                let word = mem.read_u32(b.rgb + i * 4)?;
                mem.write_u32(made.rgb + i * 4, word)?;
            }
        }
        // The new surface is declared with the source's palette size, not the one this host seeds by
        // default: the bitmap's own palette size is where the count comes from, and `cntRGB` shares
        // the word `sync_bitmap` reads `nPitch` from.
        let mut made = made;
        made.cnt_rgb = b.cnt_rgb;
        let pitch_at = abi::offset_of("IDIB", "nPitch").expect("IDIB is in the tables");
        let pitch = mem.read_u32(addr + pitch_at)?;
        mem.write_u32(
            addr + pitch_at,
            (pitch & 0xffff) | (u32::from(b.cnt_rgb) << 16),
        )?;
        self.bitmaps.insert(addr, made);
        self.pixel_owner.insert(made.px, addr);
        // `DIB_NewEx` carries `ncTransparent` across. The **field** is what gets copied, not the
        // record's colour: a pack reads this number back through `GetTransparencyColor`, and an index
        // that went out through `NativeToRGB`/`RGBToNative` does not always come back as itself.
        let at = abi::offset_of("IDIB", "ncTransparent").expect("IDIB is in the tables");
        let trans = mem.read_u32(obj + at)?;
        self.transparency(mem, addr, trans)?;
        Ok(addr)
    }

    /// Allocate the pixels and the object for a host bitmap. The returned address already carries
    /// the reference the caller asked for; see [`Hle::new_object`] for who owns the rest.
    pub fn new_bitmap(
        &mut self,
        mem: &mut Mem,
        hle: &mut Hle,
        width: u16,
        height: u16,
        depth: u16,
    ) -> HResult<u32> {
        check_depth(depth, self.dib_depth)?;
        let stride = row_bytes(width, depth);
        // Both dimensions are the guest's here, so the product is checked rather than trusted:
        // `stride * height` in `u32` wraps for a 65535x65535 request, and a wrapped product hands back
        // a small buffer that every later walk treats as the whole picture — memory corruption with no
        // fault to report, which is the worst shape a bug like this can take.
        let span = stride as usize * height as usize;
        let _span = u32::try_from(span).map_err(|_| HleError::Unsupported {
            what: format!(
                "a {width}x{height} {depth}-bpp bitmap needs {span} bytes, past what an address names"
            ),
        })?;
        // The palette is a guest array even when nobody asks for one, because `IDIB::pRGB` is a
        // pointer the app writes its own colours into. Seeding it with the host's default mapping is
        // what keeps a pack that never touches the table drawing what it drew before the table existed.
        let cnt_rgb = cnt_rgb(depth);
        let rgb = if cnt_rgb == 0 {
            0
        } else {
            let table = default_palette(depth);
            let addr = hle
                .alloc(mem, table.len() as u32 * 4)
                .map_err(HleError::Mem)?;
            for (i, entry) in table.iter().enumerate() {
                mem.write_u32(addr + i as u32 * 4, *entry)?;
            }
            addr
        };
        let seed = Bitmap {
            obj: 0,
            px: 0,
            rgb,
            cnt_rgb,
            width,
            height,
            pitch: stride as i32,
            depth,
            trans: default_transparent(depth),
        };
        let (obj, px) = new_bitmap_object(mem, hle, &seed, self.dib_pixel_offset, false)?;
        self.bitmaps.insert(obj, Bitmap { obj, px, ..seed });
        self.pixel_owner.insert(px, obj);
        Ok(obj)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn bit_blt(
        &mut self,
        mem: &mut Mem,
        disp: u32,
        xdst: i32,
        ydst: i32,
        dx: i32,
        dy: i32,
        src: u32,
        xsrc: i32,
        ysrc: i32,
        rop: u32,
    ) -> HResult<()> {
        // `IDisplay::BitBlt` is `void`: a null source is a caller mistake with no error channel,
        // and a handset paints nothing. Aborting here turned a failed image load in the pack into
        // a host fault, because the pack kept running and the next blit handed us the null it had
        // stored.
        if src == 0 || src == u32::MAX {
            self.note("IDisplay::BitBlt: a null or poison source bitmap, and nothing to do");
            return Ok(());
        }
        // Both sides of a blit are `IDIB`s the app may have restated since it last drew with them, so
        // this is where each one's geometry is read back rather than trusted from creation.
        self.sync_bitmap(mem, src)?;
        let state = self.state(disp)?;
        let clip = state.clip;
        let dst = self.dest(&state)?;
        self.sync_bitmap(mem, dst.obj)?;
        let dst = self
            .bitmap(dst.obj)
            .ok_or(HleError::NotAnObject { addr: dst.obj })?;
        let Some(src) = self.bitmap(src).or_else(|| self.guest_bitmap(mem, src)) else {
            self.note("IDisplay::BitBlt: a source bitmap this host cannot read, and nothing to do");
            return Ok(());
        };
        if src.px == u32::MAX || !self.readable_pixels(mem, &src) {
            // BREW's empty-DIB poison can survive in a public `pBmp`, and a pack can restate it to
            // an address it never decoded; either way there is no pixel to read.
            self.note("IDisplay::BitBlt: a source bitmap with no readable pixel buffer, and nothing to do");
            return Ok(());
        }
        self.copy(
            mem,
            &dst,
            xdst,
            ydst,
            dx,
            dy,
            &src,
            xsrc,
            ysrc,
            rop,
            Some(clip),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn blt_in(
        &mut self,
        mem: &mut Mem,
        dst: u32,
        xdst: i32,
        ydst: i32,
        dx: i32,
        dy: i32,
        src: u32,
        xsrc: i32,
        ysrc: i32,
        rop: u32,
    ) -> HResult<u32> {
        if dst == 0 || dst == u32::MAX || src == 0 || src == u32::MAX {
            // `IBitmap::BltIn/BltOut` return an `int`; a null or BREW's poison pointer is a caller
            // mistake and the handset answers a parameter error. Returning `NotAnObject` threw the
            // pack out of the run instead of letting it handle the failure it had just been given.
            return Ok(g::AEE_EBADPARM);
        }
        // Both sides, not just the one the call is made through. `IBitmap::BltIn` arrives with
        // `dst` as `this`, which the runtime's own head has already re-read, but the source is only
        // ever a number in an argument register — and for a pack whose whole art pipeline is
        // "decode a file into a DIB whose `pBmp` I rewrote, then `BltIn` it out of that DIB" the
        // source is exactly the bitmap whose record went stale when the app restated it. Reading it
        // from the creation-time copy hands back the empty buffer the host allocated before the app
        // ever put art in it, which is a black sprite with a success code.
        self.sync_bitmap(mem, src)?;
        self.sync_bitmap(mem, dst)?;
        let dst = self
            .bitmap(dst)
            .ok_or(HleError::NotAnObject { addr: dst })?;
        let src = self
            .bitmap(src)
            .ok_or(HleError::NotAnObject { addr: src })?;
        if dst.px == u32::MAX
            || src.px == u32::MAX
            || !self.readable_pixels(mem, &dst)
            || !self.readable_pixels(mem, &src)
        {
            return Ok(g::AEE_EBADPARM);
        }
        if !rop_supported(rop)
            && !(rop == g::AEE_RO_BLEND
                && self.picture_alpha(src.obj, src.width as usize).is_some())
        {
            // The same rule as [`Display::copy`]: a blend is only something this host can carry out
            // when the source names a coverage. The four fill paths keep refusing BLEND, because
            // there is no source there to composite from. Refusing here is the honest answer;
            // painting a plain copy and returning `AEE_SUCCESS` would make "composited" and
            // "ignored" the same sight.
            return Ok(g::AEE_EUNSUPPORTED);
        }
        self.copy(mem, &dst, xdst, ydst, dx, dy, &src, xsrc, ysrc, rop, None)?;
        Ok(g::AEE_SUCCESS)
    }

    /// `ITransform::TransformBltSimple`: the same copy, with the source rectangle read through a
    /// reflection or a rotation first.
    ///
    /// The geometry is a reference front end's `TransformSimple`, which is the same OEM routine a handset runs: it
    /// keeps a *step* from one destination column to the next and one to the next row, in source
    /// pixels, and every case below changes only those two steps and where the sampling starts. The
    /// header's one-line rule — "all transformation use the source bitmap's center as the origin" — is
    /// why the quarter turns slide the destination by half the difference of its own axes rather than
    /// drawing it where it was pointed.
    ///
    /// Scaling is the third family the same routine carries, and it is why the two rules above are not
    /// enough on their own: a scale changes the destination's extent as well as the steps, so it is
    /// applied *after* the rotation and to the rotated rectangle — the order a reference front end's
    /// `TransformSimple` uses, and the order that keeps a quarter-turned sprite centred.
    #[allow(clippy::too_many_arguments)]
    pub fn transform_in(
        &mut self,
        mem: &mut Mem,
        dst: u32,
        mut xdst: i32,
        mut ydst: i32,
        src: u32,
        xsrc: i32,
        ysrc: i32,
        dxsrc: i32,
        dysrc: i32,
        xform: u32,
        rop: u32,
    ) -> HResult<u32> {
        let (mut dx, mut dy) = (dxsrc, dysrc);
        // Where the source rectangle is sampled from, and how one destination pixel moves that point.
        let (mut ox, mut oy) = (xsrc, ysrc);
        let (mut stepx, mut stepy) = (1, 1);
        if xform & g::TRANSFORM_FLIP_X != 0 {
            oy += dysrc - 1;
            stepy = -1;
        }
        match xform & 0x03 {
            g::TRANSFORM_ROTATE_90 => {
                ox += dxsrc - 1;
                (stepx, stepy) = (stepy, -1);
                let n = (dx - dy) / 2;
                xdst += n;
                ydst -= n;
                (dx, dy) = (dy, dxsrc);
            }
            g::TRANSFORM_ROTATE_180 => {
                ox += dxsrc - 1;
                oy += dysrc - 1;
                (stepx, stepy) = (-1, -stepy);
            }
            g::TRANSFORM_ROTATE_270 => {
                oy += dysrc - 1;
                (stepx, stepy) = (-stepy, 1);
                let n = (dx - dy) / 2;
                xdst += n;
                ydst -= n;
                (dx, dy) = (dy, dxsrc);
            }
            _ => {}
        }
        // A scale is a numerator/denominator pair packed into a 3-bit field, which makes it an enum
        // and not a mask: `0x38` means one half, not `SCALE_8 | SCALE_QUARTER | SCALE_HALF`'s bits
        // combined, so the whole field has to be read at once. `0x20` is the single value inside the
        // field that the header never defines; a reference front end's switch has no case for it either and falls
        // through unscaled, which is what is done here — with a note, since nothing may go unsaid.
        let (num, den) = match xform & 0x38 {
            0 => (1, 1),
            g::TRANSFORM_SCALE_2 => (2, 1),
            g::TRANSFORM_SCALE_4 => (4, 1),
            g::TRANSFORM_SCALE_8 => (8, 1),
            g::TRANSFORM_SCALE_EIGHTH => (1, 8),
            g::TRANSFORM_SCALE_QUARTER => (1, 4),
            g::TRANSFORM_SCALE_HALF => (1, 2),
            other => {
                self.note(format!(
                    "TransformBltSimple asked with an undefined scale field ({other:#04x}); drawn at its own size"
                ));
                (1, 1)
            }
        };
        if num != 1 || den != 1 {
            // The rectangle grows or shrinks about its own centre. a reference front end's rounding is reproduced
            // rather than tidied up, because the two directions floor differently — going up it slides
            // by a floored *half* extent times the extra copies, going down by a floored proportion of
            // the whole extent — and a pack that placed magnified art by eye on a handset was placed
            // against that, not against a cleaner half-open centre that can differ by a pixel.
            if num > 1 {
                xdst -= (dx / 2) * (num - 1);
                ydst -= (dy / 2) * (num - 1);
                (dx, dy) = (dx * num, dy * num);
            } else {
                xdst += (dx * (den - 1)) / (2 * den);
                ydst += (dy * (den - 1)) / (2 * den);
                (dx, dy) = (dx / den, dy / den);
            }
        }
        // Both sides may have been restyled by the app, exactly as [`Display::bit_blt`] assumes.
        self.sync_bitmap(mem, src)?;
        self.sync_bitmap(mem, dst)?;
        let dst_b = self
            .bitmap(dst)
            .ok_or(HleError::NotAnObject { addr: dst })?;
        let src_b = self
            .bitmap(src)
            .ok_or(HleError::NotAnObject { addr: src })?;
        // The transparent colour belongs to the source, as it does in every other blit, and the skip
        // is decided here so the write itself is a plain copy. `key_rop` keeps the caller's op for
        // [`source_pixel`], which is the one place that asks; the write below runs under the
        // equivalent plain copy.
        let key_rop = rop;
        let rop = if rop == g::AEE_RO_TRANSPARENT {
            g::AEE_RO_COPY
        } else {
            rop
        };
        let alpha = self.picture_alpha(src, src_b.width as usize);
        // After the objects have been looked up, so a bad destination is still the error it is — but
        // before the walk, because a shrink can erase a sprite: 4 source pixels at one eighth leave a
        // zero-wide rectangle, and a blit that paints nothing and says nothing is the class of thing
        // this file keeps having to rule out.
        if dx <= 0 || dy <= 0 {
            self.note(format!(
                "a {dxsrc}x{dysrc} source scaled {num}/{den} has no room left to draw in"
            ));
            return Ok(g::AEE_SUCCESS);
        }
        let mut memo = IndexMemo::new();
        for j in 0..dy {
            let sy = oy + (j * den / num) * stepy;
            for i in 0..dx {
                // Nearest source pixel, not an interpolated one. a reference front end walks its source pointer with
                // a counter that only steps it once every `num` destination pixels, and this integer
                // quotient is the same walk: at 8X a sprite is 64 copies of one pixel, which is the
                // blocky result a pack that magnified its own art was counting on.
                let sx = ox + (i * den / num) * stepx;
                let Some((native, a)) =
                    source_pixel(mem, &src_b, alpha.as_deref(), sx, sy, key_rop)?
                else {
                    continue;
                };
                let (x, y) = (xdst + i, ydst + j);
                if x < 0 || y < 0 || x >= dst_b.width as i32 || y >= dst_b.height as i32 {
                    continue;
                }
                self.lay(mem, &dst_b, x, y, native, a, rop, Some(&mut memo))?;
            }
        }
        Ok(g::AEE_SUCCESS)
    }

    /// `ITransform::TransformBltComplex`: the same blit through a 2x2 matrix.
    ///
    /// The header location is really specified by the center of the source area, which is drawn at
    /// `(xDst + dxSrc/2, yDst + dySrc/2)`", so both rectangles are measured from their own centres.
    /// And what the numbers mean: `A` to `D` are "fixed point values with an 8-bit integer part and an
    /// 8-bit fractional part", i.e. 8.8, so the matrix is scaled by 256 and its determinant by 65536.
    ///
    /// It samples backwards — every destination pixel asks for a source pixel — because a matrix that
    /// magnifies would otherwise leave holes the size of the magnification, and because the clip the
    /// handset applies is a clip on the source rectangle, which a backwards walk can test directly.
    #[allow(clippy::too_many_arguments)]
    pub fn transform_complex_in(
        &mut self,
        mem: &mut Mem,
        dst: u32,
        xdst: i32,
        ydst: i32,
        src: u32,
        xsrc: i32,
        ysrc: i32,
        dxsrc: i32,
        dysrc: i32,
        matrix: u32,
        rop: u32,
    ) -> HResult<u32> {
        let at = |field: &str| {
            abi::offset_of("AEETransformMatrix", field)
                .expect("AEETransformMatrix is in the tables")
        };
        // Worked in 64-bit throughout: `A` to `D` are 8.8, so a product of two of them is 16.16 and
        // an `i32` would overflow on a matrix that merely magnifies.
        let (mut a, mut b, mut c, mut d) = (
            i64::from(mem.read_u16(matrix + at("A"))? as i16),
            i64::from(mem.read_u16(matrix + at("B"))? as i16),
            i64::from(mem.read_u16(matrix + at("C"))? as i16),
            i64::from(mem.read_u16(matrix + at("D"))? as i16),
        );
        let mut det = a * d - b * c;
        if det == 0 {
            // A singular matrix maps the whole rectangle onto a line. There is no area to sample,
            // and `EUNSUPPORTED` is the answer the header itself offers.
            return Ok(g::AEE_EUNSUPPORTED);
        }
        if det < 0 {
            // Negating the matrix and its determinant leaves the inverse identical and makes the
            // rounding below one rule instead of two.
            (a, b, c, d, det) = (-a, -b, -c, -d, -det);
        }
        // The destination box is the source rectangle's four corners, mapped forward. Each corner is
        // measured from the source's own centre, which is the origin the header names — and the far
        // corners are at `extent - 1 - centre`, because they are pixels, not the width.
        let (left_edge, top_edge) = (-dxsrc / 2, -dysrc / 2);
        let (right_edge, bottom_edge) = (dxsrc - 1 + left_edge, dysrc - 1 + top_edge);
        let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (ox, oy) in [
            (left_edge, top_edge),
            (right_edge, top_edge),
            (left_edge, bottom_edge),
            (right_edge, bottom_edge),
        ] {
            // Forward, in 8.8: X = (A·u + B·v)/256 pixels.
            let px = ((a * i64::from(ox) + b * i64::from(oy)) / 256) as i32;
            let py = ((c * i64::from(ox) + d * i64::from(oy)) / 256) as i32;
            let (cx, cy) = (xdst + dxsrc / 2 + px, ydst + dysrc / 2 + py);
            left = left.min(cx);
            right = right.max(cx);
            top = top.min(cy);
            bottom = bottom.max(cy);
        }
        self.sync_bitmap(mem, src)?;
        self.sync_bitmap(mem, dst)?;
        let dst_b = self
            .bitmap(dst)
            .ok_or(HleError::NotAnObject { addr: dst })?;
        let src_b = self
            .bitmap(src)
            .ok_or(HleError::NotAnObject { addr: src })?;
        let key_rop = rop;
        let rop = if rop == g::AEE_RO_TRANSPARENT {
            g::AEE_RO_COPY
        } else {
            rop
        };
        let alpha = self.picture_alpha(src, src_b.width as usize);
        let half = det / 2;
        let mut memo = IndexMemo::new();
        for py in top..=bottom {
            for px in left..=right {
                // Destination offsets from the centre, in the matrix's own 8.8 units, mapped back
                // through the inverse: u = (D·x − B·y)/det, v = (A·y − C·x)/det.
                let (x, y) = (
                    i64::from(px - (xdst + dxsrc / 2)) << 8,
                    i64::from(py - (ydst + dysrc / 2)) << 8,
                );
                // Round to nearest, away from zero on both sides: the offset is signed, and adding
                // half a determinant unconditionally would bias every negative sample one pixel down.
                let round = |num: i64| (num + if num >= 0 { half } else { -half }) / det;
                let u = round(d * x - b * y);
                let v = round(a * y - c * x);
                // The inverse gives an offset from the source rectangle's centre, which is where the
                // forward mapping was measured from; the corner itself is `xSrc + dxSrc/2` away.
                let (sx, sy) = (xsrc + dxsrc / 2 + u as i32, ysrc + dysrc / 2 + v as i32);
                if !(xsrc..xsrc + dxsrc).contains(&sx) || !(ysrc..ysrc + dysrc).contains(&sy) {
                    continue;
                }
                let Some((native, a)) =
                    source_pixel(mem, &src_b, alpha.as_deref(), sx, sy, key_rop)?
                else {
                    continue;
                };
                if px < 0 || py < 0 || px >= dst_b.width as i32 || py >= dst_b.height as i32 {
                    continue;
                }
                self.lay(mem, &dst_b, px, py, native, a, rop, Some(&mut memo))?;
            }
        }
        Ok(g::AEE_SUCCESS)
    }

    /// `IBitmap::BltOut` is the same copy seen from the other end: the header makes `po` the
    /// *source* and `pDst` the destination, while `xDst`/`yDst`/`xSrc`/`ySrc` keep their meaning.
    /// Hence the positional delegation, and hence why a real handset's `BltIn` falls back to the
    /// source's `BltOut` rather than the reverse.
    #[allow(clippy::too_many_arguments)]
    pub fn blt_out(
        &mut self,
        mem: &mut Mem,
        src: u32,
        xdst: i32,
        ydst: i32,
        dx: i32,
        dy: i32,
        dst: u32,
        xsrc: i32,
        ysrc: i32,
        rop: u32,
    ) -> HResult<u32> {
        self.blt_in(mem, dst, xdst, ydst, dx, dy, src, xsrc, ysrc, rop)
    }

    /// Destroy an object whose last reference has just gone: its pixels, its display state, the
    /// destination reference it held, and finally its record, so a later call through the stale
    /// pointer faults instead of drawing into memory nobody owns any more.
    ///
    /// The screen bitmap is the one exception. It outlives every applet, so its count reaching zero
    /// means a module released a reference it was never handed — faulting the *screen* for that
    /// would turn a module's leak into the emulator losing its own framebuffer.
    pub fn forget(&mut self, mem: &mut Mem, hle: &mut Hle, addr: u32) -> HResult<()> {
        if addr == self.device.obj {
            return Ok(());
        }
        if self.default == addr {
            self.default = 0;
        }
        if let Some(state) = self.displays.remove(&addr) {
            // `state.dest` cannot be `addr`: a destination is always a bitmap this struct knows.
            self.drop_ref(mem, hle, state.dest)?;
        }
        if let Some(b) = self.bitmaps.remove(&addr) {
            self.pixel_owner.remove(&b.px);
            // A picture's transparency record belongs to the bitmap it was laid out into, so it
            // leaves with it — nothing else can name this object again.
            self.alphas.remove(&addr);
            // The pixels and the palette were taken from the guest arena for this bitmap alone, so
            // they go back with it. A pack that makes a scratch surface and destroys it every frame
            // otherwise walks the arena at frame rate, and the only symptom is `malloc()` answering
            // null an hour into a session.
            // The palette was taken from the guest arena for this bitmap alone, so it goes back
            // with it. The pixels do not: they live inside the object block (`DIB_NewEx` allocates
            // `pixel_bytes + 0x6B` by default; `abi.dib_pixel_offset` moves that header), so
            // `forget` above already returned them with the object.
            hle.dealloc(mem, b.rgb);
        }
        hle.forget(addr);
        Ok(())
    }
}

/// One pixel inside the bitmap's own extent, or `None` when the coordinate is outside it. A read
/// that *should* land and cannot is still an error, so a caller never mistakes a fault for
/// clipping.
/// The byte range a bitmap's pixels can occupy.
///
/// Not simply `pBmp` to `pBmp + size`: a negative pitch runs the rows the other way — the header's
/// own example hands over a bottom-up bitmap as `pBmp` at the *last* row with a pitch of `-12`.
fn pixel_extent(b: &Bitmap) -> (u32, u32) {
    let row = (((b.width as i32) * (b.depth as i32) + 7) / 8).max(0) as u32;
    let rows = (b.height as i32 - 1).max(0) as i64;
    let last = b.px.wrapping_add((rows * i64::from(b.pitch)) as u32);
    if b.pitch >= 0 {
        (b.px, last.wrapping_add(row))
    } else {
        (last, b.px.wrapping_add(row))
    }
}

/// Whether two bitmaps can touch the same byte.
///
/// Asked only to decide whether a blit may be done a row at a time: a source and a destination that
/// overlap have to go pixel by pixel, in the order the pixel loop reads and writes them.
fn share_memory(a: &Bitmap, b: &Bitmap) -> bool {
    let (a0, a1) = pixel_extent(a);
    let (b0, b1) = pixel_extent(b);
    a0 < b1 && b0 < a1
}

/// The ink a glyph run paints with, resolved the first time a pixel actually needs it.
///
/// The two text slots hand the ink over in different spaces. `IDisplay::DrawText` reads it out of the
/// display's own colour state, so it arrives as a colour; `IFont::DrawText` takes `NativeColor`s the
/// caller already converted, which on a palettized destination are **indices**,
/// and turning one into a colour means reading that bitmap's `pRGB`.
///
/// That read is a guest memory access, and a run does not always paint: a blank glyph has no ink and
/// a glyph can sit wholly outside the clip rectangle, yet both are legal and both answer `SUCCESS`.
/// Resolving at the door made such a call fault on a palette entry it never used — the shape
/// [`Display::draw_pixel`]'s out-of-bounds return and `h_scanline`'s clipped run were written for.
/// Keeping the value as it arrived, and asking on the first pixel that survives the clip, is what
/// moves the read to where the pixel is.
///
/// The answer is remembered: a run pays one lookup, not one per pixel.
struct Ink {
    /// What the caller handed over: a colour for `IDisplay`, a stored value for `IFont`.
    raw: Native,
    /// The colour it stands for, once something has asked.
    resolved: Option<Native>,
}

impl Ink {
    /// Ink that is already a colour: nothing to resolve.
    fn colour(colour: Native) -> Self {
        Self {
            raw: colour,
            resolved: Some(colour),
        }
    }

    /// Ink in the destination's own pixel space: an index where the bitmap keeps a table.
    fn stored(value: Native) -> Self {
        Self {
            raw: value,
            resolved: None,
        }
    }

    /// The colour to paint with, reading `pRGB` at most once per run.
    fn of(&mut self, b: &Bitmap, mem: &Mem) -> HResult<Native> {
        match self.resolved {
            Some(colour) => Ok(colour),
            None => {
                let colour = b.colour_of_pixel(mem, self.raw)?;
                self.resolved = Some(colour);
                Ok(colour)
            }
        }
    }
}

/// Colour-to-index answers for one drawing pass over a palettized bitmap.
///
/// [`Bitmap::stored`] answers by searching the table — a couple of hundred guest reads, which is
/// nothing once per fill and unaffordable per pixel of a blit: measured on a 256-entry table, the
/// search costs 300–770 ns a pixel against 4 ns for the fixed ramp it replaces, so a 240x320 blit
/// would go from microseconds to tens of milliseconds.
///
/// Living for exactly one pass is what makes it safe. A pack rewrites `pRGB` whenever it likes —
/// palette animation is a real thing on this hardware — so a cache that outlived the call would go
/// stale behind the host's back, and there is no cheap way to notice. A blit or a fill cannot be
/// interrupted by the guest, so a table that lives inside one cannot be out of date.
///
/// What it *does* invalidate on is the bitmap's `obj`: an index is an answer about one particular
/// table, and two palettes name one colour at two different entries. It does **not** watch the words
/// behind `pRGB`. So the one-pass lifetime is not a convenience, it is the whole proof: nothing that
/// can rewrite a palette may run between the first pixel and the last. `Display`'s own loops are safe
/// because a blit cannot be interrupted by the guest; a `Ctx` is safe because it is built and dropped
/// inside one ABI call and never writes its own `st`. Lengthen
/// either one and this type starts answering a table that no longer exists, with nothing to show for
/// it. There is no type or test that enforces this — only those two facts.
///
/// Two tiers, because the two shapes it serves are different sizes. The first colour a pass asks
/// about is kept by itself: a fill, a line, a glyph run is one colour, and answering that costs a
/// comparison per pixel and no allocation at all. The rest go into a 512-slot direct-mapped table,
/// bought the first time a *second* colour turns up — a blit or a stretched picture has hundreds, a
/// filled rectangle has one and never allocates.
pub(crate) struct IndexMemo {
    /// Which bitmap these answers are for. A memo handed a different one starts over: an answer is
    /// an index into *that* bitmap's table, and two palettes name one colour at two different
    /// entries, so reusing one across them writes a pixel of the wrong colour with nothing to show
    /// for it. No single call can do that today — but the type should not be the thing that knows.
    obj: Option<u32>,
    /// The first colour this pass asked about, and the value that bitmap stores for it.
    first: Option<(u32, u8)>,
    /// The rest, once there is more than one. `None` until then, because the table is four
    /// kilobytes of slots filled with `(EMPTY, 0)` — a pass that only ever asks one question does
    /// not need it, and most passes only ask one.
    slots: Option<Box<[(u32, u8); IndexMemo::SLOTS]>>,
}

impl IndexMemo {
    const SLOTS: usize = 512;
    /// No 5-6-5 colour has this bit, which is what makes it a usable "nothing here" key.
    const EMPTY: u32 = 0x1_0000;

    pub(crate) fn new() -> Self {
        Self {
            obj: None,
            first: None,
            slots: None,
        }
    }

    /// Where a colour goes in the direct-mapped table.
    fn slot(key: u32) -> usize {
        (key as usize).wrapping_mul(2_654_435_761) >> 23 & (Self::SLOTS - 1)
    }

    /// The value `b` stores for `native`, remembered across the pixels of one pass.
    fn index(&mut self, b: &Bitmap, mem: &Mem, native: Native) -> HResult<Native> {
        if self.obj != Some(b.obj) {
            *self = Self {
                obj: Some(b.obj),
                ..Self::new()
            };
        }
        let key = native & 0xffff;
        match self.first {
            Some((known, index)) if known == key => return Ok(u32::from(index)),
            None => {
                let index = b.stored(mem, key)? as u8;
                self.first = Some((key, index));
                return Ok(u32::from(index));
            }
            Some(_) => {}
        }
        // The first colour is carried into the table rather than dropped: a two-colour pass (text is
        // the usual one, `fg` then `bg`) would otherwise re-search `fg` on its next appearance.
        let first = self.first;
        let slots = self.slots.get_or_insert_with(|| {
            let mut table = Box::new([(Self::EMPTY, 0); Self::SLOTS]);
            if let Some((known, index)) = first {
                table[Self::slot(known)] = (known, index);
            }
            table
        });
        let (known, index) = slots[Self::slot(key)];
        if known == key {
            return Ok(u32::from(index));
        }
        let index = b.stored(mem, key)? as u8;
        slots[Self::slot(key)] = (key, index);
        Ok(u32::from(index))
    }
}

/// The colour of a pixel, at every depth: a palettized bitmap's index goes through its own palette
/// on the way out, which is what the callers that *combine* pixels want — a raster op combines
/// colours, a blend averages colours, an inversion complements a colour.
///
/// Not the mirror of [`write_pixel`]. The mirror is [`read_stored`], and the two differ below 16 bpp
/// by exactly the palette: this one asks `pRGB` what the index means, that one hands the index back.
/// Which of the two a path takes is not a matter of taste — see [`source_pixel`], where reading the
/// value first is what lets a keyed-out pixel leave without a palette lookup at all.
fn read_pixel(mem: &Mem, b: &Bitmap, x: i32, y: i32) -> HResult<Option<Native>> {
    if x < 0 || y < 0 || x >= b.width as i32 || y >= b.height as i32 {
        return Ok(None);
    }
    Ok(Some(match b.depth {
        // A 0-bpp DIB has a zero pitch and therefore no pixels at all. Falling through to the
        // 16-bpp arm would read two bytes past the object; BREW's own DIB_NewEx leaves pBmp at a
        // poison pointer in this case, so there is nothing to read here either.
        0 => 0,
        // An index, resolved through the bitmap's own palette — the packing of the byte is
        // `Bitmap::index_at`'s business, and so is the nibble order a 4-bpp font depends on.
        8 | 4 | 1 => b.colour_of(mem, b.index_at(mem, x as u32, y as u32)?)?,
        32 => stored888_to_565(mem.read_u32(b.at(x as u32, y as u32))?),
        _ => mem.read_u16(b.at(x as u32, y as u32))?,
    }))
}

/// The value a pixel **is**: the index a palettized bitmap keeps, the packed word otherwise. The
/// mirror of [`write_pixel`], and a space three callers ask in.
///
/// `IBitmap::GetPixel` is one — "Retrieves the value of the specified pixel"
/// through a `NativeColor*`, which the header defines as "values stored in pixels in the
/// pixel array". A pack that writes an index and reads back a colour would see `RGBToNative` →
/// `DrawPixel` → `GetPixel` fail to close on a palettized bitmap: write 3, read back the colour of
/// entry 3.
///
/// [`source_pixel`]'s transparency test is the second, and it is where reading the value first earns
/// its keep twice over — the comparison happens in this space rather than in the palette's, and the
/// pixels that lose it never reach [`Bitmap::colour_of`] at all. [`Display::put`]'s two-operand path
/// is the third: `OR`/`XOR`/`AND` combine pixel *values*, so the destination
/// operand is read here rather than through the palette and converted back.
fn read_stored(mem: &Mem, b: &Bitmap, x: i32, y: i32) -> HResult<Option<Native>> {
    if x < 0 || y < 0 || x >= b.width as i32 || y >= b.height as i32 {
        return Ok(None);
    }
    Ok(Some(match b.depth {
        0 => 0,
        8 | 4 | 1 => u32::from(b.index_at(mem, x as u32, y as u32)?),
        32 => mem.read_u32(b.at(x as u32, y as u32))?,
        _ => u32::from(mem.read_u16(b.at(x as u32, y as u32))?),
    }))
}

/// What a source bitmap contributes at one of its own coordinates: the colour to lay down and how
/// much of the destination it may cover, or nothing at all.
///
/// Two independent kinds of transparency are honoured here, and neither is invented. The colour key
/// (`trans`, under `AEE_RO_TRANSPARENT`) is the one a *pack* sets — keying a sprite on a value is the
/// pack's own choice, and that path is unchanged. The alpha map is the one the *file* set: an indexed
/// PNG's `tRNS` entries or a real alpha channel, which no call in the ABI lets a pack name, so a host
/// that does not carry it from the decoder simply paints the see-through background.
/// A truecolour PNG's `tRNS` names a *colour* rather than a set of entries, and there the ABI does
/// have a call — `IBitmap::SetTransparencyColor` — but that is the *pack* naming one; the file's
/// answer is turned into coverage at decode time and reaches this function through the alpha map
/// like any other, never through `trans`.
///
/// The key is compared against the pixel's **stored value**, not its colour. Two palette entries that
/// happen to pack to the same 5-6-5 word are still two keys: the header says the matching
/// "is performed on pixel values, not on the corresponding R-G-B intensities", and a sprite whose
/// transparent row is index 0 must not lose the pixel of index 200 that its table also calls magenta.
/// What is copied *on* afterwards is still a colour, and has to be: the destination has its own
/// palette, so index 3 of the source is not index 3 of the destination — resolving the source pixel
/// and letting [`Bitmap::stored`] find the destination's entry for it is the only mapping the two
/// tables have in common.
///
/// The returned byte is the source's coverage: `255` copies the colour as stored, anything less
/// composes it with what is already standing in the destination.
fn source_pixel(
    mem: &Mem,
    src: &Bitmap,
    alpha: Option<&AlphaMap>,
    sx: i32,
    sy: i32,
    rop: u32,
) -> HResult<Option<(Native, u8)>> {
    // The value first, the colour second, and that order is worth more than it looks. The colour key
    // is compared against a *value*, so resolving the pixel to a colour before
    // the comparison would decode every skipped pixel of every keyed sprite — a palette search per
    // transparent pixel of a sprite that is mostly transparent. Read as stored, a pixel the key
    // names leaves here having cost one byte lookup and nothing else.
    let Some(stored) = read_stored(mem, src, sx, sy)? else {
        return Ok(None);
    };
    // Coverage first, and it is the authority when the picture has one. The colour key is a *separate*
    // claim the pack makes (`IBitmap::SetTransparencyColor`, `IDIB::ncTransparent`) and it stays in
    // force for every picture without a map — sprites keyed on magenta, usually.
    // Keying on top of a map would drop pixels the file never said were transparent, and a rastered
    // picture's key defaults to magenta, so any exact-magenta pixel in alpha art would vanish the
    // moment the decoder's default op became `AEE_RO_TRANSPARENT`.
    let a = match alpha {
        Some(map) => map.at(sx.max(0) as usize, sy.max(0) as usize),
        None => {
            // The header source bitmap of a transparent bit blit operation." The key therefore belongs to `src`,
            // even when the destination's `ncTransparent` names a different value in its own table.
            if rop == g::AEE_RO_TRANSPARENT && stored == src.trans {
                return Ok(None);
            }
            u8::MAX
        }
    };
    if a == 0 {
        return Ok(None);
    }
    Ok(Some((src.colour_of_pixel(mem, stored)?, a)))
}

/// Mix one source colour into the colour already in the destination, per channel, by the source's
/// own coverage. Both operands go through RGB first because the destination is a packed 565 word and
/// averaging the halves of a 5-bit field is not the same question as averaging the colour.
fn blend(src: Native, dst: Native, a: u8) -> Native {
    let (sr, sg, sb) = split_rgb(native_to_rgb(src));
    let (dr, dg, db) = split_rgb(native_to_rgb(dst));
    let mix = |s: u8, d: u8| -> u8 {
        let num = u32::from(s) * u32::from(a) + u32::from(d) * u32::from(u8::MAX - a);
        (num / u32::from(u8::MAX)) as u8
    };
    rgb_to_native(make_rgb(mix(sr, dr), mix(sg, dg), mix(sb, db)))
}

/// Lay one pixel down, in the bitmap's **own** pixel space: an index for a palettized bitmap, the
/// packed colour for the screen.
///
/// Note what this means next to [`read_pixel`], which is *not* its mirror below 16 bpp: that one
/// resolves an index through the table and answers a colour at every depth, because what its four
/// drawing callers want is a colour. The value-for-value mirror is [`read_stored`], and the way a
/// colour becomes one of these is [`Bitmap::stored`] — the two callers here (`Display::put`,
/// `Display::invert_rect`) take it. The body used to do that conversion itself, which read correctly
/// for the screen and quietly wrong for every off-screen bitmap that keeps indices: a colour drawn
/// there landed on the fixed 3-3-2 ramp's index for it rather than on the entry that names it, so the
/// pack read back a value its own table did not contain.
fn write_pixel(mem: &mut Mem, b: &Bitmap, x: i32, y: i32, v: Native) -> HResult<()> {
    if x < 0 || y < 0 || x >= b.width as i32 || y >= b.height as i32 {
        return Ok(());
    }
    let at = b.at(x as u32, y as u32);
    match b.depth {
        // No pixels to write in a 0-bpp DIB; see the matching arm in `read_pixel`.
        0 => return Ok(()),
        8 => mem.write_u8(at, v as u8)?,
        4 => {
            let byte = mem.read_u8(at)?;
            let nibble = (v as u8) & 0x0f;
            // Only the half this pixel owns changes: the other half belongs to the pixel beside it.
            // The same high-nibble-first order [`read_pixel`] uses, so a bitmap the host draws into
            // and then reads back does not transpose its own pixels.
            let byte = if x % 2 == 0 {
                (byte & 0x0f) | (nibble << 4)
            } else {
                (byte & 0xf0) | nibble
            };
            mem.write_u8(at, byte)?;
        }
        // Eight pixels to a byte, most significant bit first: only this pixel's bit changes.
        1 => {
            let byte = mem.read_u8(at)?;
            let shift = 7 - x % 8;
            let byte = if v & 1 == 0 {
                byte & !(1 << shift)
            } else {
                byte | (1 << shift)
            };
            mem.write_u8(at, byte)?;
        }
        32 => mem.write_u32(at, v)?,
        _ => mem.write_u16(at, v & 0xffff)?,
    }
    Ok(())
}

/// The depths this host backs, and the packing each reads and writes. 16 and 32 bpp are whole-colour
/// pixels (RGB565 and `0x00RRGGBB`); 8, 4 and 1 are indices into the bitmap's own table. `ceiling`
/// is the deepest DIB the device profile says its firmware allocates (`abi.dib_depth`), so a
/// handset that never hands out a 32-bpp DIB can keep refusing one.
///
/// Zero is a depth BREW itself accepts, which is a different question from whether it is useful:
/// `DIB_NewEx` in the delivered source rejects only a negative `nPixelSize`, so
/// `IDISPLAY_CreateDIBitmap(…, 0, …)` returns SUCCESS with `nDepth = 0`, `nPitch = 0` and no
/// pixels. Refusing it here stopped six guest packs at their first off-screen bitmap. Such a DIB
/// carries no pixels to read or write — [`read_pixel`] and [`write_pixel`] answer that explicitly
/// rather than falling through to the 16-bpp arm at an address past the object.
fn check_depth(depth: u16, ceiling: u16) -> HResult<()> {
    match depth {
        0 | NATIVE_DEPTH | 8 | 4 | 1 => Ok(()),
        32 if ceiling >= 32 => Ok(()),
        other => Err(HleError::Unsupported {
            what: format!(
                "{other} bpp host bitmaps are not supported, only 16, 32, 8, 4 or 1 (or 0 for an empty DIB)"
            ),
        }),
    }
}

/// A 1-bpp pixel: an index of 1 is white and 0 is black, which is how a monochrome handset bitmap reads.
pub const fn index1_to_native(i: u8) -> u32 {
    if i & 1 == 0 {
        pack565(0, 0, 0) as u32
    } else {
        pack565(255, 255, 255) as u32
    }
}

/// The inverse of [`index1_to_native`]: a colour bright enough to be meant as white becomes 1.
pub const fn native_to_index1(native: u32) -> u8 {
    let (r, g, b) = unpack565(native as u16);
    let luma = (r as u32 + g as u32 * 2 + b as u32) / 4;
    if luma >= 128 {
        1
    } else {
        0
    }
}

/// A 4-bpp index read through the same 3-3-2 table [`index8_to_native`] describes, with the index's four
/// bits repeated to fill the byte. The header publishes no 4-bpp table either, and repeating the bits at
/// least keeps the sixteen colours ordered and distinct.
pub const fn index4_to_native(i: u8) -> u32 {
    index8_to_native((i & 0x0f) * 17)
}

/// The inverse direction, for a colour being drawn into a 4-bpp bitmap.
pub const fn native_to_index4(native: u32) -> u8 {
    native_to_index8(native) >> 4
}

/// Palette entries a bitmap of this depth has — one per pixel value, and none at all when the pixels
/// already carry native colours. This is `IDIB::cntRGB`, and the header is explicit that a nonzero
/// count is what makes a pixel value an index rather than a colour.
pub const fn cnt_rgb(depth: u16) -> u16 {
    match depth {
        1 => 2,
        4 => 16,
        8 => 256,
        _ => 0,
    }
}

/// What an index means when no palette names it: the fixed 3-3-2 ramp for 8 and 4 bpp, black and
/// white for 1.
///
/// The one place that choice is written down. [`Bitmap::colour_of`] answers with it for an index past
/// the table a bitmap declares, and [`default_palette`] publishes it as `IDIB::pRGB` for a bitmap that
/// brought none — those two must agree, or a pack that reads the table back would find our colours
/// shifted against the ones it drew with. A depth without indices (`_`) answers with the 1-bpp pair,
/// which is what the three inline copies of this match used to do.
const fn ramp_colour(depth: u16, index: u8) -> Native {
    match depth {
        8 => index8_to_native(index),
        4 => index4_to_native(index),
        _ => index1_to_native(index),
    }
}

/// The table a palettized bitmap is born with: [`index8_to_native`] and its siblings, restated as the
/// RGBVALs `IDIB::pRGB` holds. A pack that brings its own palette overwrites these entries — it may
/// decode one out of its own art — and a pack that does not sees exactly the colours this host drew
/// before it had a loadable table at all.
fn default_palette(depth: u16) -> Vec<u32> {
    (0..u32::from(cnt_rgb(depth)))
        .map(|i| {
            let (r, g, b) = unpack565(ramp_colour(depth, i as u8) as u16);
            // Published in the byte order [`palette_to_rgbval`] reads, because this array is handed
            // to the guest as `IDIB::pRGB` and a pack that reads it back would otherwise find our
            // own default table shifted against the one it wrote itself.
            rgbval_to_palette(make_rgb(r, g, b))
        })
        .collect()
}

/// The transparency key a fresh bitmap starts with, in **its own pixel space** — the number the pixel
/// array holds, which on a palettized bitmap is an index.
///
/// The header greyscale displays, the transparent color is `RGBToNative(RGB_WHITE)`, and for color displays the
/// default transparent color is `RGBToNative(RGB_MASK_COLOR)`." The `RGBToNative` is the whole
/// content of this function — without it a colour display's bitmap would start with `0xF81F` in a
/// field the size of a pixel, which on an 8-bpp surface names no entry at all and on a 1-bpp one
/// names whatever the low byte happens to be.
///
/// Written against the ramp rather than by searching the table, for two reasons. [`default_palette`]
/// *is* the ramp restated, and [`Bitmap::colour_of`] answers from it when a bitmap declares no table,
/// so the two agree by construction. And one of the two callers — [`Display::new`] — builds the device
/// bitmap before any table exists, so there would be nothing to search.
///
/// Note that 1-bpp is the monochrome case here and takes white, not magenta: a 1-bpp bitmap *is* the
/// monochrome display the header describes. `native_to_index1` reads magenta as luma 127 and
/// answers black, so getting this branch wrong is not a rounding difference — it is the opposite bit.
fn default_transparent(depth: u16) -> Native {
    let mask = if depth == 1 {
        crate::device::RGB_WHITE
    } else {
        crate::device::RGB_MASK_COLOR
    };
    if is_888(depth) {
        return rgbval_to_888(mask);
    }
    let native = rgb_to_native(mask);
    match depth {
        1 => u32::from(native_to_index1(native)),
        4 => u32::from(native_to_index4(native)),
        8 => u32::from(native_to_index8(native)),
        _ => native & 0xffff,
    }
}

/// Bytes in one row of `width` pixels at `depth`. Counted in bits and rounded up, because a 4-bpp or
/// 1-bpp row does not end on a byte boundary — `width * bytes_per_pixel` would give a 4-bpp bitmap a
/// stride twice as wide as its own pixels, so every row after the first would start in the wrong place.
fn row_bytes(width: u16, depth: u16) -> u32 {
    (u32::from(width) * u32::from(depth)).div_ceil(8)
}

fn new_bitmap_object(
    mem: &mut Mem,
    hle: &mut Hle,
    b: &Bitmap,
    pixel_offset: u32,
    page_round: bool,
) -> HResult<(u32, u32)> {
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    // A palettized bitmap names its colours through `pRGB`, so its scheme is left empty; the two being
    // nonzero at once is what the header calls an undefined mapping. A 32-bpp bitmap is
    // `IDIB_COLORSCHEME_888`, whose `NativeColor` is `0x00RRGGBB`, not the 5-6-5 word.
    let scheme = if b.cnt_rgb != 0 {
        0
    } else if is_888(b.depth) {
        g::IDIB_COLORSCHEME_888
    } else {
        g::IDIB_COLORSCHEME_565
    };
    let span = (b.pitch.max(0) as u32).saturating_mul(u32::from(b.height));
    let mut want = pixel_offset.saturating_add(span);
    // The screen is the one bitmap whose block is a reservation rather than an exact size. A panel's
    // framebuffer is handed out page-granular by the firmware's own allocator, and a pack that walks
    // the rows it can see sometimes takes one more: reading a row past `pBmp + nPitch * cy` finds the
    // tail of that reservation on a handset, and a host that packs the block to the byte turns the
    // same read into the end of the run. Only the screen is rounded — an off-screen DIB is an
    // allocation like any other, and a pack that over-reads one of those should still be told.
    if page_round {
        let page = crate::mem::PAGE_SIZE as u32;
        want = want.saturating_add(page - 1) & !(page - 1);
    }
    let obj = hle.new_object_reusing(
        mem,
        "IBitmap",
        g::AEECLSID_BITMAP,
        want,
        at("reserved") + 2,
        &[],
    )?;
    let px = obj + pixel_offset;
    // A fresh surface is black, not whatever the arena had in it a moment ago: this used to be a
    // separate `Hle::alloc` that handed out zeroed space, and the object now carries the pixels. The
    // whole reservation is cleared, so the tail of a screen reads as the black it looks like rather
    // than as the picture a reused block last held.
    mem.fill(px, want - pixel_offset, 0)?;
    mem.write_u32(obj + at("pPaletteMap"), 0)?;
    mem.write_u32(obj + at("pBmp"), px)?;
    mem.write_u32(obj + at("pRGB"), b.rgb)?;
    // Already a value of this bitmap's pixel space, so it goes out as it stands. Deriving it from a
    // colour here would be the one place that has to guess: `b.trans` may have come from a pack that
    // named an index, and asking the palette what that index *means* only to turn the answer back
    // into an index is the round trip `SetTransparencyColor` exists to avoid.
    mem.write_u32(obj + at("ncTransparent"), b.trans)?;
    mem.write_u32(
        obj + at("cx"),
        u32::from(b.width) | (u32::from(b.height) << 16),
    )?;
    mem.write_u32(
        obj + at("nPitch"),
        b.pitch as u32 | (u32::from(b.cnt_rgb) << 16),
    )?;
    mem.write_u32(obj + at("nDepth"), u32::from(b.depth) | (scheme << 8))?;
    Ok((obj, px))
}

/// What a cleared screen and a text cell are painted in. `CLR_USER_BACKGROUND` is a settable item,
/// so this reads the display's own table rather than the boot default.
fn background(state: &DisplayState) -> u32 {
    state
        .colors
        .get(g::CLR_USER_BACKGROUND as usize)
        .copied()
        .unwrap_or(RGB_WHITE)
}

/// The colour table as a handset boots: black text on white, and the greys a frame is drawn in.
/// These are the OEM's defaults, not ABI, which is why they are one function and one test.
fn default_colors() -> Vec<u32> {
    let mut c = vec![RGB_WHITE; g::CLR_SYS_LAST as usize];
    for (item, rgb) in [
        (g::CLR_USER_TEXT, RGB_BLACK),
        (g::CLR_USER_BACKGROUND, RGB_WHITE),
        (g::CLR_USER_LINE, RGB_BLACK),
        (g::CLR_SYS_TITLE_TEXT, RGB_BLACK),
        (g::CLR_SYS_ITEM_TEXT, RGB_BLACK),
        (g::CLR_SYS_ITEM_SEL, RGB_BLACK),
        (g::CLR_SYS_ITEM_SEL_TEXT, RGB_WHITE),
        (g::CLR_SYS_FRAME_LO, RGB_BLACK),
        (g::CLR_SYS_DK_SHADOW, RGB_BLACK),
    ] {
        c[item as usize] = rgb;
    }
    c
}

/// Field offsets for the two guest structs the display reads on *every* draw.
///
/// `abi::offset_of` walks `STRUCTS` (594 entries in the reference tables) doing a string comparison
/// per entry, then walks that struct's fields the same way. `Rect::read` used to pay four of those
/// per `IDisplay::DrawRect` — 374,400 walks in a 400-frame run — and `sync_bitmap` six more per
/// bitmap, twice per `BitBlt`. Resolving them once keeps the ABI table the single source of truth
/// (a regenerated table changes these values automatically) without a string search in the frame
/// loop. Nothing here is guest state, so it is deliberately absent from `write_state`/`read_state`.
struct Layout {
    /// `AEERect`: x / y / dx / dy, in that order.
    rect: [u32; 4],
    /// `IDIB`: the six words `sync_bitmap` reads back from a bitmap object.
    idib_pbmp: u32,
    idib_prgb: u32,
    idib_cx: u32,
    idib_pitch: u32,
    idib_depth: u32,
    idib_trans: u32,
}

impl Layout {
    fn from_abi() -> Self {
        let o = |s: &str, f: &str| {
            abi::offset_of(s, f).unwrap_or_else(|| panic!("{s}.{f} in the tables"))
        };
        Self {
            rect: [
                o("AEERect", "x"),
                o("AEERect", "y"),
                o("AEERect", "dx"),
                o("AEERect", "dy"),
            ],
            idib_pbmp: o("IDIB", "pBmp"),
            idib_prgb: o("IDIB", "pRGB"),
            idib_cx: o("IDIB", "cx"),
            idib_pitch: o("IDIB", "nPitch"),
            idib_depth: o("IDIB", "nDepth"),
            idib_trans: o("IDIB", "ncTransparent"),
        }
    }
}

fn field_layout() -> &'static Layout {
    static L: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();
    L.get_or_init(Layout::from_abi)
}

fn abi_offset(struct_name: &str, field: &str) -> u32 {
    abi::offset_of(struct_name, field)
        .unwrap_or_else(|| panic!("{struct_name}.{field} in the tables"))
}

fn field_off(struct_name: &str, field: &str) -> usize {
    abi_offset(struct_name, field) as usize
}

/// A rectangle as four `i16`s, and back.
///
/// With `graphics::put_rgba` this is the pair every stateful record in the display layer reads
/// through; `Rect` lives here, so its reader does too, and a change to the struct changes one file.
#[allow(dead_code)]
pub(crate) fn put_rect(out: &mut Vec<u8>, r: Rect) {
    for v in [r.x, r.y, r.dx, r.dy] {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

#[allow(dead_code)]
pub(crate) fn get_rect(st: &mut crate::mem::State) -> Result<Rect, String> {
    let b = st.take(8)?;
    let v = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
    Ok(Rect {
        x: v(0),
        y: v(2),
        dx: v(4),
        dy: v(6),
    })
}

/// A bitmap's *record*: where its pixels are, its geometry, and the colour it treats as transparent.
///
/// Not the pixels. Those live in guest memory at `px`, and the state block already carries that whole
/// address space; copying them here as well would double the size of a state whose whole point is
/// being the size of what the run touched, and the two copies could disagree after a restore.
#[allow(dead_code)]
fn put_bitmap(out: &mut Vec<u8>, b: &Bitmap) {
    use crate::mem::put_u32;
    put_u32(out, b.obj);
    put_u32(out, b.px);
    put_u32(out, b.rgb);
    out.extend_from_slice(&b.cnt_rgb.to_le_bytes());
    out.extend_from_slice(&b.width.to_le_bytes());
    out.extend_from_slice(&b.height.to_le_bytes());
    out.extend_from_slice(&b.pitch.to_le_bytes());
    out.extend_from_slice(&b.depth.to_le_bytes());
    put_u32(out, b.trans);
}

#[allow(dead_code)]
fn get_bitmap(st: &mut crate::mem::State) -> Result<Bitmap, String> {
    let obj = st.u32()?;
    let px = st.u32()?;
    let rgb = st.u32()?;
    let b = st.take(2)?;
    let cnt_rgb = u16::from_le_bytes([b[0], b[1]]);
    let b = st.take(2)?;
    let width = u16::from_le_bytes([b[0], b[1]]);
    let b = st.take(2)?;
    let height = u16::from_le_bytes([b[0], b[1]]);
    let b = st.take(4)?;
    let pitch = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let b = st.take(2)?;
    let depth = u16::from_le_bytes([b[0], b[1]]);
    let trans = st.u32()?;
    Ok(Bitmap {
        obj,
        px,
        rgb,
        cnt_rgb,
        width,
        height,
        pitch,
        depth,
        trans,
    })
}

#[allow(dead_code)]
fn put_display_state(out: &mut Vec<u8>, s: &DisplayState) {
    use crate::mem::{put_bool, put_u32};
    put_u32(out, s.dest);
    put_rect(out, s.clip);
    put_u32(out, s.colors.len() as u32);
    for colour in &s.colors {
        put_u32(out, *colour);
    }
    out.extend_from_slice(&s.annunciators.to_le_bytes());
    put_bool(out, s.backlight);
    put_u32(out, s.fonts.len() as u32);
    for (designator, font) in &s.fonts {
        put_u32(out, *designator);
        put_u32(out, *font);
    }
    put_u32(out, s.pending);
}

#[allow(dead_code)]
fn get_display_state(st: &mut crate::mem::State) -> Result<DisplayState, String> {
    let dest = st.u32()?;
    let clip = get_rect(st)?;
    let count = st.small()?;
    let mut colors = Vec::with_capacity(count);
    for _ in 0..count {
        colors.push(st.u32()?);
    }
    let b = st.take(2)?;
    let annunciators = u16::from_le_bytes([b[0], b[1]]);
    let backlight = st.u8()? != 0;
    let count = st.small()?;
    let mut fonts = Vec::with_capacity(count);
    for _ in 0..count {
        fonts.push((st.u32()?, st.u32()?));
    }
    Ok(DisplayState {
        dest,
        clip,
        colors,
        annunciators,
        backlight,
        fonts,
        pending: st.u32()?,
    })
}

/// The per-pixel transparency of one picture, as the blob it was decoded into plus its width.
///
/// `partial` is not written: [`AlphaMap::new`] derives it from the bytes, and a state that carried a
/// copy could disagree with them after a restore. A blob that turns out to be fully opaque is dropped
/// on the way in, which is exactly what [`AlphaMap::new`] does when the picture is first decoded.
#[allow(dead_code)]
fn put_alpha(out: &mut Vec<u8>, a: &AlphaMap) {
    use crate::mem::put_u32;
    put_u32(out, a.bytes.len() as u32);
    out.extend_from_slice(&a.bytes);
    put_u32(out, a.width as u32);
}

#[allow(dead_code)]
fn get_alpha(st: &mut crate::mem::State) -> Result<Option<AlphaMap>, String> {
    let len = st.small()?;
    let bytes = st.take(len)?.to_vec();
    let width = st.small()?;
    Ok(AlphaMap::new(bytes, width))
}

/// The display state a restored run needs: which bitmap is the device, which `IDisplay` objects exist
/// and what each is drawing with, every live `IBitmap` record, the alpha blobs, and the two counters a
/// headless run asserts on (`presentations` and the instruction count they are dated by).
///
/// What is deliberately not here: `runs`, `screens`, `quiet`, the refusal list and every `*_seen` /
/// `*_dropped` counter. Those are what a *report* is made of — a restored machine has no use for the
/// previous one's text log, and carrying it would make a state grow with how long a run has been
/// watched rather than with what it holds.
impl Display {
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        use crate::mem::{put_u32, put_u64, sorted_u32};
        put_bitmap(out, &self.device);
        put_u32(out, self.dib_pixel_offset);
        put_u32(out, u32::from(self.dib_depth));
        put_u32(out, self.default);
        let displays = sorted_u32(&self.displays);
        put_u32(out, displays.len() as u32);
        for (addr, state) in displays {
            put_u32(out, addr);
            put_display_state(out, state);
        }
        let bitmaps = sorted_u32(&self.bitmaps);
        put_u32(out, bitmaps.len() as u32);
        for (addr, bitmap) in bitmaps {
            put_u32(out, addr);
            put_bitmap(out, bitmap);
        }
        let alphas = sorted_u32(&self.alphas);
        put_u32(out, alphas.len() as u32);
        for (addr, map) in alphas {
            put_u32(out, addr);
            put_alpha(out, map);
        }
        put_u32(out, self.presentations);
        put_u64(out, self.now);
    }

    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut crate::mem::State) -> Result<(), String> {
        let device = get_bitmap(st)?;
        let dib_pixel_offset = st.u32()?;
        let dib_depth = st.u32()? as u16;
        let default = st.u32()?;
        let count = st.small()?;
        let mut displays = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            displays.insert(addr, get_display_state(st)?);
        }
        let count = st.small()?;
        let mut bitmaps = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            bitmaps.insert(addr, get_bitmap(st)?);
        }
        let count = st.small()?;
        let mut alphas = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            if let Some(map) = get_alpha(st)? {
                alphas.insert(addr, Arc::new(map));
            }
        }
        let presentations = st.u32()?;
        let now = st.u64()?;

        self.device = device;
        self.dib_pixel_offset = dib_pixel_offset;
        self.dib_depth = dib_depth;
        self.default = default;
        self.displays = displays;
        self.bitmaps = bitmaps;
        self.pixel_owner = self
            .bitmaps
            .iter()
            .map(|(addr, b)| (b.px, *addr))
            .collect();
        self.alphas = alphas;
        self.presentations = presentations;
        self.now = now;
        Ok(())
    }

    /// A display with nothing in it, for [`Runtime::restore`](crate::Runtime::restore) to read a state
    /// block into before anything live is touched: a block that turns out to be short must not have
    /// half-replaced the display a run is drawing into.
    pub(crate) fn empty() -> Display {
        Display {
            device: Bitmap {
                obj: 0,
                px: 0,
                rgb: 0,
                cnt_rgb: 0,
                width: 0,
                height: 0,
                pitch: 0,
                depth: 0,
                trans: 0,
            },
            dib_pixel_offset: DIB_PIXEL_OFFSET,
            dib_depth: NATIVE_DEPTH,
            encoding: TextEncoding::Unicode,
            text_bytes: false,
            displays: HashMap::new(),
            default: 0,
            bitmaps: HashMap::new(),
            pixel_owner: HashMap::new(),
            alphas: HashMap::new(),
            faces: HashMap::new(),
            face_epoch: 0,
            runs: Vec::new(),
            runs_seen: 0,
            presentations: 0,
            quiet: Vec::new(),
            notes_dropped: 0,
            now: 0,
            screens: Vec::new(),
            screens_seen: 0,
            screens_dropped: 0,
            refusals: Vec::new(),
            refused: 0,
            fill_row: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "test/display.rs"]
mod tests;
