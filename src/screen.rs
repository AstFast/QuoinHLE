//! The screen as bytes a foreign UI can upload.
//!
//! A host that embeds this machine — a libretro core, a GPUI window, anything that owns its own
//! event loop and its own GPU — needs one thing from the core: the picture, in memory it can read,
//! with some idea of how much of it moved. Everything else it already has a door for (`pump` to
//! advance the machine, `post_key_press`/`post_key_release` to feed it, `Speaker::mix` to pull audio).
//! This module is that one thing, and it exists because the door that was here before —
//! [`Display::screenshot`] — allocates a fresh 230 400-byte `Vec` and converts all 76 800 pixels on
//! **every** presentation, which is 97 % of its own cost (measured: 215 µs for a 240x320 screen, of
//! which reading the guest's bytes is 7.5 µs).
//!
//! # Whose format it is
//!
//! [`Format`] is settled when the [`Screen`] is made, and it decides both the stride and whether any
//! conversion happens at all. `Rgb565` is the guest's own words copied out: a 16-bpp `IBitmap` *is*
//! little-endian 5-6-5, which is what a GPU texture format and a libretro front end's
//! `RETRO_PIXEL_FORMAT_RGB565` both already understand, so the unpack is theirs to do — on the GPU,
//! for nothing. `Rgba8888` is what a `Color32` array and an `rgba8unorm` texture want, so the widget
//! toolkit's own conversion never happens. `Rgb888` unpacks in software, for the paths that have no
//! texture at all — a PNG, a BMP, a test.
//!
//! The measured case for making this a choice rather than a constant, 240x320, one frame that changed
//! everywhere, one machine: `Rgb565` costs about 30 µs — 7.5 to read the guest's rows through the page
//! table, then two 11 µs `memcpy`s of each row that moved (one into the shadow, one into the output) —
//! against 198 µs for the same screen unpacked to `Rgb888`, and 160 µs for `Rgba8888`. A libretro core
//! that then packed `R G B` into `0x00RRGGBB` added 67 µs to that, so one frame went from 265 µs to 30.
//! The `memcmp` is not in that figure because it stops at the first differing word: it costs 9.5 µs on
//! a row that did *not* move, which is what a still screen pays instead of the copies.
//!
//! The rule the three of them follow: a consumer gets the bytes its own upload wants, and no consumer
//! stands in a queue behind a universal intermediate. `Rgb888` is the one nobody needs at 60 Hz, and
//! it survives because a picture on disk is a different job from a picture on a screen.
//!
//! What this cannot do is hand out a pointer into guest memory. [`crate::mem::Mem`] holds 4 KiB pages
//! from 1 MiB super pages, so a bitmap usually *is* contiguous, but "usually" is not a contract, and
//! `IBitmap::nPitch` may be negative or padded besides. The row read below is the floor: 7.5 µs, or
//! 0.05 % of a 60 Hz frame, which is not worth a second addressing mode.
//!
//! # Why rows, and why by comparison
//!
//! Guest pixels are written by four different paths — the blitter in [`crate::display`], the GLES
//! rasterizer, `IGraphics`, and the module itself through a raw `IBitmap*` — so a damage mark placed
//! at the host's own draw calls would miss the last two, and a missed mark is a pixel that stays
//! stale for the rest of the run. Comparing the bytes the guest holds is the only damage signal that
//! cannot be wrong, and it costs one read plus one `memcmp` per row when nothing moved.
//!
//! The comparison is against the guest's own 16-bit words rather than against the output: doing it
//! the other way round would pay the conversion this is trying to avoid. The shadow is therefore
//! `width * 2` bytes a row whatever the output format is, and it is what makes a 565 output cheaper
//! than an 888 one — the copy into the output is the same bytes the comparison already read.
//!
//! A row is the unit because it is the largest run the guest guarantees. `IBitmap::nPitch` may be
//! negative or wider than the image, so rows are not contiguous; inside a row the pixels are.
//!
//! # What this is not
//!
//! It is not a window, a texture, or a swap chain. It holds no lock and spawns no thread, so the
//! owner of the machine can call [`Screen::refresh`] on the thread that already owns it and hand
//! [`Screen::pixels`] straight to a renderer. A consumer on another thread wants an owned copy —
//! that is [`crate::session`]'s `Frame`, and the copy it pays for is a `memcpy` rather than a
//! conversion.

use crate::display::{pack565, unpack565, Display};
use crate::hle::HResult;
use crate::mem::Mem;

/// Row bands closer together than this are reported as one, because two uploads of three rows each
/// cost more than one upload of eleven — a GPU rect write is not free, and neither is the code that
/// splits a frame into them.
const MERGE_GAP: u32 = 8;

/// How [`Screen::pixels`] is spelled.
///
/// One variant per consumer that actually exists — 5-6-5 for a texture unit, four bytes for a widget
/// toolkit, three for a file on disk. Anything else is `Rgb888` plus a conversion at the far end, and
/// adding a variant before that consumer exists would be an untested arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The guest's own words: little-endian 16-bit 5-6-5, two bytes a pixel.
    ///
    /// No conversion while the device bitmap is 16-bpp. An indexed one (8/4/1 bpp) is converted into
    /// this shape instead, because those bytes are palette indices and nothing can hand them on.
    Rgb565,
    /// Three bytes a pixel, `R G B`, unpacked in software.
    Rgb888,
    /// Four bytes a pixel, `R G B A` with alpha `255` — the shape a `Color32` array and a
    /// `rgba8unorm` texture both want, so a consumer that needs one pays no second conversion.
    Rgba8888,
}

impl Format {
    /// Bytes one pixel occupies, which is also what the stride is a multiple of.
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Format::Rgb565 => 2,
            Format::Rgb888 => 3,
            Format::Rgba8888 => 4,
        }
    }
}

/// One guest screen in the format a consumer asked for, kept across frames.
///
/// The buffer is reused, so a run that presents sixty times a second allocates once. After
/// [`Screen::refresh`], [`Screen::pixels`] is the whole screen — `height` rows of [`Screen::stride`]
/// bytes, top row first — and [`Screen::bands`] names the row ranges in it that differ from the
/// previous refresh.
#[derive(Debug)]
pub struct Screen {
    width: u32,
    height: u32,
    /// The device depth the shadow/row buffers are sized for: 16 or 32 for the fast path, and any
    /// other depth takes the reference capture instead.
    depth: u16,
    format: Format,
    /// One of the three, row-major, `width * format.bytes_per_pixel()` bytes a row.
    pixels: Vec<u8>,
    /// The guest's own bytes for the same rows: `width * 2` a row at 16 bpp, `width * 4` at 32.
    /// This is what a refresh compares against, and it is also what the output is converted from.
    shadow: Vec<u8>,
    /// One row read out of guest memory, reused so a still screen does not allocate either.
    row: Vec<u8>,
    /// Whether `shadow` holds a picture. False before the first refresh and after a fallback that
    /// bypassed it, and it makes the first refresh convert every row rather than none.
    have_shadow: bool,
    /// The `Display::presentations` these pixels belong to.
    epoch: u32,
    /// Row ranges (`y0..y1`, `y1` exclusive) that the last refresh touched.
    bands: Vec<(u32, u32)>,
}

impl Screen {
    /// A screen in [`Format::Rgb888`], which is what a PNG or a BMP wants.
    pub fn new() -> Self {
        Self::with_format(Format::Rgb888)
    }

    /// A screen in the format the consumer can take as it comes.
    ///
    /// Choosing here rather than per call is the whole point: the alternative is one buffer per
    /// format, kept in step, or a conversion in the middle of a frame.
    pub fn with_format(format: Format) -> Self {
        Self {
            width: 0,
            height: 0,
            depth: 0,
            format,
            pixels: Vec::new(),
            shadow: Vec::new(),
            row: Vec::new(),
            have_shadow: false,
            epoch: 0,
            bands: Vec::new(),
        }
    }

    /// The screen's size, as of the last refresh. `(0, 0)` before the first one.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The format [`Screen::pixels`] is in, i.e. the one this screen was built with.
    pub fn format(&self) -> Format {
        self.format
    }

    /// Bytes a row of [`Screen::pixels`] occupies — `width * 2`, `* 3` or `* 4`.
    pub fn stride(&self) -> usize {
        (self.width as usize) * self.format.bytes_per_pixel()
    }

    /// The whole screen in [`Screen::format`], row-major, top row first — not just the bands.
    ///
    /// The whole screen is handed over on purpose: a consumer that only wants the bands can slice
    /// them out (they are row ranges, so the arithmetic is one multiply), but a consumer that
    /// mirrors the buffer — a texture, a screenshot, a diff against the last frame — needs the rest
    /// of it to be current too, and it is: rows outside the bands are the ones that did not move.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The row ranges (`y0..y1`, `y1` exclusive) that moved in the last [`Screen::refresh`].
    ///
    /// Empty means nothing needs uploading, which covers both "the guest has not presented since
    /// the last refresh" and "it presented the same picture again".
    pub fn bands(&self) -> &[(u32, u32)] {
        &self.bands
    }

    /// The `Display::presentations` the pixels belong to. A consumer that keeps its own copy uses
    /// this to tell "same frame" from "a new frame that happens to look identical".
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Bring the pixels up to date, converting only the rows that moved.
    ///
    /// Returns without reading anything when the guest has not presented since the last call, which
    /// is the case a UI running at 60 Hz against a 30 Hz menu hits every other frame. Otherwise it
    /// reads each row of the guest's bitmap once, compares it with what that row held last time, and
    /// converts the ones that differ — or, in [`Format::Rgb565`], copies them, because the guest's
    /// words already are the answer.
    pub fn refresh(&mut self, display: &Display, mem: &Mem) -> HResult<()> {
        let wide = u32::from(display.device.width);
        let high = u32::from(display.device.height);
        let depth = display.device.depth;
        let presented = display.presentations;
        // A size change has to be picked up even without a present: a device bitmap swapped under
        // the host is a different screen, and mirroring the old geometry would slice it wrong.
        let resized = self.adopt(wide, high, depth);
        if !resized && self.have_shadow && presented == self.epoch {
            self.bands.clear();
            return Ok(());
        }
        if wide == 0 || high == 0 {
            self.bands.clear();
            self.epoch = presented;
            return Ok(());
        }
        // A depth the row path does not pack — 8, 4, 1 — cannot be compared as words, and neither
        // can it be copied out as words. The reference path knows how to read it, and it is rare
        // enough that paying for a full conversion there is the honest trade.
        let bytes = match depth {
            16 => 2,
            32 => 4,
            _ => 0,
        };
        if bytes == 0 {
            let rgb = display.screenshot(mem)?;
            write_indexed(&rgb, &mut self.pixels, self.format);
            self.have_shadow = false;
            self.epoch = presented;
            self.bands.clear();
            self.bands.push((0, high));
            return Ok(());
        }

        self.bands.clear();
        let have_shadow = self.have_shadow;
        let row_len = (wide as usize) * bytes;
        let out_len = (wide as usize) * self.format.bytes_per_pixel();
        let format = self.format;
        let Self {
            row,
            shadow,
            pixels,
            ..
        } = self;
        let mut open: Option<(u32, u32)> = None;
        for y in 0..high {
            mem.read_into(display.device.row_start(y), &mut row[..row_len])?;
            let at = (y as usize) * row_len;
            if have_shadow && shadow[at..at + row_len] == row[..row_len] {
                continue;
            }
            shadow[at..at + row_len].copy_from_slice(&row[..row_len]);
            let out = (y as usize) * out_len;
            // A `match` on a value fixed before the loop rather than a function pointer: an indirect
            // call cannot be inlined into this loop, while three direct calls let the compiler pick
            // one arm for the whole refresh (and vectorize each one's body, which is what the
            // equal-length slices are for — see `convert_row`).
            match (bytes, format) {
                // The guest's words, straight through. `copy_from_slice` on equal-length slices is
                // one `memcpy`; the comparison above already read these bytes, so this is the
                // cheapest an output can be that is not a pointer into guest memory.
                (2, Format::Rgb565) => pixels[out..out + out_len].copy_from_slice(&row[..row_len]),
                (2, Format::Rgb888) => {
                    convert_row(&row[..row_len], &mut pixels[out..out + out_len])
                }
                (2, Format::Rgba8888) => {
                    expand565(&row[..row_len], &mut pixels[out..out + out_len])
                }
                (4, Format::Rgb565) => {
                    convert_row32_to_565(&row[..row_len], &mut pixels[out..out + out_len])
                }
                (4, Format::Rgb888) => {
                    convert_row32(&row[..row_len], &mut pixels[out..out + out_len])
                }
                (4, Format::Rgba8888) => expand32(&row[..row_len], &mut pixels[out..out + out_len]),
                _ => unreachable!("the depth gate above admits only 16 and 32"),
            }
            open = match open {
                Some((y0, y1)) if y - y1 <= MERGE_GAP => Some((y0, y + 1)),
                Some(band) => {
                    self.bands.push(band);
                    Some((y, y + 1))
                }
                None => Some((y, y + 1)),
            };
        }
        if let Some(band) = open {
            self.bands.push(band);
        }
        self.have_shadow = true;
        self.epoch = presented;
        Ok(())
    }

    /// Take the display's geometry and depth, and say whether either changed.
    fn adopt(&mut self, wide: u32, high: u32, depth: u16) -> bool {
        if self.width == wide && self.height == high && self.depth == depth {
            return false;
        }
        self.width = wide;
        self.height = high;
        self.depth = depth;
        let stride = (wide as usize) * self.format.bytes_per_pixel();
        // A shadow exists only for the two depths the row path packs. A switch to 8/4/1 leaves it
        // empty; `refresh` takes the reference path and never indexes it.
        let bytes = match depth {
            16 => 2,
            32 => 4,
            _ => 0,
        };
        self.pixels = vec![0u8; stride * (high as usize)];
        self.shadow = vec![0u8; (wide as usize) * (high as usize) * bytes];
        self.row = vec![0u8; (wide as usize) * bytes];
        // A new buffer has no previous picture to compare against, so every row is a change. Saying
        // so here rather than leaving `bands` to look empty is what keeps a resized screen from
        // being uploaded as "nothing moved".
        self.have_shadow = false;
        true
    }
}

/// One row of guest `RGB565` words into one row of `RGB888`.
///
/// A free function taking slices rather than a loop inside `refresh` for one measured reason: given
/// exact-length slices the bounds check is hoisted out of the loop and the stores vectorize, and a
/// full-screen change is the common case — a game repaints everything every frame. Written inline it
/// measured 336 µs against the 184 µs of the one-shot path it replaced, which would have made
/// "only convert what moved" a regression for every full-screen frame.
fn convert_row(row: &[u8], out: &mut [u8]) {
    for (px, to) in row.chunks_exact(2).zip(out.chunks_exact_mut(3)) {
        let (r, g, b) = unpack565(u16::from_le_bytes([px[0], px[1]]));
        to[0] = r;
        to[1] = g;
        to[2] = b;
    }
}

/// [`convert_row`] with an opaque alpha byte on the end, four bytes out for every two in.
///
/// Public because it is the one thing a consumer that cannot take 5-6-5 needs, and giving it a name
/// here is what keeps that consumer from writing its own half of the conversion: a texture upload
/// that wants `RGBA8888` calls this on [`Screen::pixels`] instead of asking for a second `Screen`.
/// Alpha is `255` — a handset screen has no transparency, and a premultiplied consumer is therefore
/// looking at the same picture as an unpremultiplied one.
pub fn expand565(words: &[u8], out: &mut [u8]) {
    for (px, to) in words.chunks_exact(2).zip(out.chunks_exact_mut(4)) {
        let (r, g, b) = unpack565(u16::from_le_bytes([px[0], px[1]]));
        to[0] = r;
        to[1] = g;
        to[2] = b;
        to[3] = 255;
    }
}

/// One row of guest `0x00RRGGBB` words into `RGB888`.
fn convert_row32(row: &[u8], out: &mut [u8]) {
    for (px, to) in row.chunks_exact(4).zip(out.chunks_exact_mut(3)) {
        to[0] = px[2];
        to[1] = px[1];
        to[2] = px[0];
    }
}

/// One row of `0x00RRGGBB` words into 5-6-5, for a consumer that wants the guest's cheaper shape.
fn convert_row32_to_565(row: &[u8], out: &mut [u8]) {
    for (px, to) in row.chunks_exact(4).zip(out.chunks_exact_mut(2)) {
        to.copy_from_slice(&pack565(px[2], px[1], px[0]).to_le_bytes());
    }
}

/// [`convert_row32`] with an opaque alpha byte on the end.
fn expand32(row: &[u8], out: &mut [u8]) {
    for (px, to) in row.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        to[0] = px[2];
        to[1] = px[1];
        to[2] = px[0];
        to[3] = 255;
    }
}

/// A screen [`Display::screenshot`] read out of a non-16-bpp bitmap, into whatever this screen wants.
///
/// Two `chunks_exact` walks rather than a conversion into `RGB888` and then out of it, because this
/// path is already the slow one and there is no reason to make it slower.
///
/// It decodes no palette — and no longer needs to: `screenshot` reads an indexed bitmap through its
/// own `IDIB::pRGB`, so what arrives here is already colour, and the only thing left is the shape.
/// What this function guarantees is that a consumer gets *the same picture* either way — for
/// `Rgb888` the bytes are copied, for `Rgb565` they are re-packed, and `pack565` of an `unpack565` is
/// the original word, so the two agree exactly rather than approximately.
fn write_indexed(rgb: &[u8], out: &mut [u8], format: Format) {
    // The caller sizes `out` from the same `display.device` the capture was taken from, so this is a
    // contract rather than a hope — and it is worth an assertion because a mismatch would be silent in
    // the two `zip` arms (they stop at the shorter slice) while panicking in the `Rgb888` one. What it
    // checks is the pixel *count*, not the geometry: two buffers with the same number of pixels and
    // different shapes would pass, and the caller is the only thing standing between that and a
    // stretched picture.
    debug_assert_eq!(rgb.len() * format.bytes_per_pixel(), out.len() * 3);
    match format {
        Format::Rgb888 => out.copy_from_slice(&rgb[..out.len()]),
        Format::Rgb565 => {
            for (px, to) in rgb.chunks_exact(3).zip(out.chunks_exact_mut(2)) {
                to.copy_from_slice(&pack565(px[0], px[1], px[2]).to_le_bytes());
            }
        }
        Format::Rgba8888 => {
            for (px, to) in rgb.chunks_exact(3).zip(out.chunks_exact_mut(4)) {
                to[0] = px[0];
                to[1] = px[1];
                to[2] = px[2];
                to[3] = 255;
            }
        }
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "test/screen.rs"]
mod tests;
