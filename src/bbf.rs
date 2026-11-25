//! The delivered bitmap-font container: `BBF`, as the font declarations describes it and the three
//! `*_NewFromBBF` bodies in `AEEBitFont.c`, `AEEVarBitFont.c` and `AEEMonoBitFont.c` read it.
//!
//! A BBF file is one header followed by four sections, each starting on a 32-bit boundary:
//!
//! ```text
//! header (20 bytes) | additional | ranges | glyph widths | bitmap
//! ```
//!
//! Three versions share that layout and differ only in how a glyph's horizontal position is
//! expressed:
//!
//! - version 0: a run-length encoded byte stream, one width per glyph;
//! - version 1: a `u16` table of left edges, one entry per glyph plus a sentinel;
//! - version 2: one cell width for the whole face, with 256 glyphs per bitmap row.
//!
//! Versions 0 and 1 put every glyph on one bitmap row, so `wWidth` — a `u16` — caps the
//! repertoire at roughly four thousand 16-pixel glyphs. Version 2 wraps at 256 glyphs and
//! divides its bitmap height by the number of rows, which is what makes a CJK-sized face fit.
//!
//! Two conventions are easy to get backwards and are worth stating here because every reader
//! below depends on them:
//!
//! - the bitmap is stored bottom-up, the way a Windows DIB is: the first row in the file is the
//!   bottom row of the image;
//! - inside a row the leftmost pixel is the highest bit of the first byte, and each row is
//!   padded to a 32-bit boundary.
//!
//! A range entry maps character values to glyph indices. Two `wCount` values are not counts:
//! [`INDEX_RESET`] moves the glyph cursor to `wCharBase`, and [`INDEX_FREEZE`] moves it there and
//! stops it from advancing. They are how one bitmap can hold several repertoire blocks without
//! one entry per character.

/// Bytes of one `AEEBBFHdr`: six `u8` members then seven `u16`s, two-byte aligned.
pub const HEADER_LEN: usize = 20;

/// `wCount` marking a range that moves the glyph cursor to its `base`.
pub const INDEX_RESET: u16 = 0xffff;
/// `wCount` marking a range that moves the cursor and then stops advancing it.
pub const INDEX_FREEZE: u16 = 0xfffe;

const COOKIE: [u8; 2] = [b'B', b'F'];
const BEWIDTHS: u8 = 0x02;

/// The parsed header, kept whole so a report can name the numbers a face was built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub flags: u8,
    pub descent: u8,
    pub depth: u8,
    pub width: u16,
    pub height: u16,
    pub undef_glyph: u16,
    pub glyph_count: u16,
    pub additional: u16,
    pub range_count: u16,
    pub width_count: u16,
}

/// One `AEEBBFRange`: characters `base .. base + count` map to consecutive glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub base: u16,
    pub count: u16,
}

/// What a range table's keys name.
///
/// A face the host builds — the 5x8 grid and the system font it rasterizes — is keyed by Unicode
/// code points. A face a handset reference is keyed by `AECHAR` units, which are the device's own
/// encoding and whose byte order is the guest's. The two cannot be told apart from the file's
/// numbers alone (a device unit such as `0xcd82` also names a Unicode code point), so a loader that
/// knows says with [`Bbf::with_key_space`]; a file parsed without that hint is Unicode-keyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySpace {
    /// Keys are Unicode code points.
    Unicode,
    /// Keys are `AECHAR` units in the device's encoding.
    Device,
}

/// What went wrong while reading a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header, or a section that runs past the end.
    Truncated,
    /// `byCookie0/1` are not `'B','F'`.
    BadCookie,
    /// `byDepth` is not 1: the format stores one bit per pixel.
    BadDepth,
    /// `wGlyphCount` is zero, which describes no font.
    NoGlyphs,
    /// A version this reader does not know.
    UnsupportedVersion,
    /// A version-2 file whose additional section is not one `AEEMonoData`.
    BadAdditional,
    /// A version-0 width stream that ends before every glyph has a width.
    BadWidths,
    /// The range table's size or the bitmap's size does not fit the file.
    BadRanges,
    /// The bitmap section is shorter than `wWidth`/`wHeight` require.
    BadBitmap,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let words = match self {
            Error::Truncated => "the file ends before a section does",
            Error::BadCookie => "the first two bytes are not 'BF'",
            Error::BadDepth => "byDepth is not 1, and only 1-bit bitmaps are supported",
            Error::NoGlyphs => "wGlyphCount is zero",
            Error::UnsupportedVersion => "byVersion is not 0, 1 or 2",
            Error::BadAdditional => "a version-2 file must carry exactly one AEEMonoData",
            Error::BadWidths => "the glyph-width stream ends early",
            Error::BadRanges => "the range table does not fit the file",
            Error::BadBitmap => "the bitmap does not fit the file",
        };
        f.write_str(words)
    }
}

impl std::error::Error for Error {}

/// Where one code unit's cell sits in the bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// Index in the face, after the undefined-glyph fallback.
    pub index: u16,
    /// Left edge of the cell in the bitmap.
    pub x: u16,
    /// Top edge of the cell in the bitmap. Always 0 for versions 0 and 1.
    pub y: u16,
    /// Width of the cell. Version 2 reports the full cell even for a half-width glyph; [`Bbf::ink`]
    /// narrows it to the half the glyph may occupy.
    pub width: u16,
    /// Height of the cell.
    pub height: u16,
}

#[derive(Debug, Clone)]
enum Widths {
    /// Left edge per glyph plus one sentinel, as versions 0 and 1 express it.
    Positions(Vec<u16>),
    /// One cell width for every glyph, as version 2 expresses it.
    Cell(u16),
}

/// A parsed face. The bitmap is shared by `Arc` because a font object can be handed to more than
/// one holder and the bytes never change after the parse.
#[derive(Debug, Clone)]
pub struct Bbf {
    header: Header,
    ranges: Vec<Range>,
    /// What the range table is addressed by; see [`KeySpace`].
    key_space: KeySpace,
    widths: Widths,
    bitmap: std::sync::Arc<[u8]>,
    /// Bytes per bitmap row, padded to a 32-bit boundary.
    pitch: usize,
    /// Version 2 only: glyph rows are `cell_height` tall, and the file's `wHeight` is their sum.
    cell_height: u16,
    /// Version 2 only: code units below this are half-width.
    half_widths: u16,
    ascent: u16,
    descent: u16,
}

impl Bbf {
    /// Read a whole file. The bytes are copied out, so the caller's buffer may be released.
    pub fn parse(bytes: &[u8]) -> Result<Bbf, Error> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated);
        }
        if bytes[0] != COOKIE[0] || bytes[1] != COOKIE[1] {
            return Err(Error::BadCookie);
        }
        let header = read_header(bytes);
        if header.depth != 1 {
            return Err(Error::BadDepth);
        }
        if header.glyph_count == 0 {
            return Err(Error::NoGlyphs);
        }
        let additional = round4(header.additional as usize);
        let ranges_at = HEADER_LEN.checked_add(additional).ok_or(Error::BadRanges)?;
        let ranges_len = (header.range_count as usize)
            .checked_mul(4)
            .ok_or(Error::BadRanges)?;
        let widths_at = ranges_at.checked_add(ranges_len).ok_or(Error::BadRanges)?;
        if widths_at > bytes.len() {
            return Err(Error::BadRanges);
        }
        let ranges = read_ranges(&bytes[ranges_at..widths_at], header.range_count);
        let key_space = KeySpace::Unicode;
        let (widths, widths_len, _data_end, cell_height, half_widths) = match header.version {
            0 => {
                let encoded = header.width_count as usize;
                let end = widths_at.checked_add(encoded).ok_or(Error::BadWidths)?;
                if end > bytes.len() {
                    return Err(Error::BadWidths);
                }
                let positions = read_rle(&bytes[widths_at..end], header.glyph_count)?;
                (
                    Widths::Positions(positions),
                    round4(encoded),
                    end,
                    header.height,
                    0,
                )
            }
            1 => {
                let entries = header.width_count as usize;
                let end = widths_at.checked_add(entries * 2).ok_or(Error::BadWidths)?;
                if end > bytes.len() {
                    return Err(Error::BadWidths);
                }
                let mut positions = Vec::with_capacity(entries);
                for at in (widths_at..end).step_by(2) {
                    let word = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
                    positions.push(if header.flags & BEWIDTHS != 0 {
                        word.swap_bytes()
                    } else {
                        word
                    });
                }
                (
                    Widths::Positions(positions),
                    round4(entries * 2),
                    end,
                    header.height,
                    0,
                )
            }
            2 => {
                if header.additional != round4(2 * std::mem::size_of::<u16>()) as u16 {
                    return Err(Error::BadAdditional);
                }
                if header.width_count == 0 {
                    return Err(Error::BadWidths);
                }
                let end = widths_at
                    .checked_add(header.width_count as usize)
                    .ok_or(Error::BadWidths)?;
                if widths_at + 2 > bytes.len() {
                    return Err(Error::Truncated);
                }
                let cell = u16::from_le_bytes([bytes[widths_at], bytes[widths_at + 1]]);
                if end > bytes.len() {
                    return Err(Error::BadWidths);
                }
                // `AEEMonoBitFont_NewFromBBF` divides `wHeight` by the number of 256-glyph rows
                // before it reports any metric, because `wHeight` is the whole bitmap's height.
                let rows = (header.glyph_count as usize + 255) / 256;
                if rows == 0 {
                    return Err(Error::NoGlyphs);
                }
                let cell_height = header.height / rows as u16;
                if cell_height == 0 {
                    return Err(Error::BadBitmap);
                }
                let half = u16::from_le_bytes([bytes[HEADER_LEN], bytes[HEADER_LEN + 1]]);
                (
                    Widths::Cell(cell),
                    round4(header.width_count as usize),
                    end,
                    cell_height,
                    half,
                )
            }
            _ => return Err(Error::UnsupportedVersion),
        };
        let bitmap_at = widths_at.checked_add(widths_len).ok_or(Error::BadBitmap)?;
        let pitch = ((header.width as usize) + 31) / 32 * 4;
        let bitmap_len = pitch
            .checked_mul(header.height as usize)
            .ok_or(Error::BadBitmap)?;
        let bitmap_end = bitmap_at.checked_add(bitmap_len).ok_or(Error::BadBitmap)?;
        if bitmap_len == 0 || bitmap_end > bytes.len() {
            return Err(Error::BadBitmap);
        }
        let descent = (header.descent as u16).min(cell_height);
        let ascent = cell_height - descent;
        Ok(Bbf {
            header,
            ranges,
            key_space,
            widths,
            bitmap: bytes[bitmap_at..bitmap_end].into(),
            pitch,
            cell_height,
            half_widths,
            ascent,
            descent,
        })
    }

    pub fn header(&self) -> Header {
        self.header
    }

    /// Rows from the top of a cell to the baseline.
    pub fn ascent(&self) -> u16 {
        self.ascent
    }

    /// Rows from the baseline to the bottom of a cell.
    pub fn descent(&self) -> u16 {
        self.descent
    }

    /// Cell height. For versions 0 and 1 this is the bitmap's height; for version 2 it is one
    /// glyph row, which is what the delivered body reports.
    pub fn height(&self) -> u16 {
        self.cell_height
    }

    pub fn glyph_count(&self) -> u16 {
        self.header.glyph_count
    }

    /// What the range table is addressed by.
    pub fn key_space(&self) -> KeySpace {
        self.key_space
    }

    /// Name the key space instead of accepting the parsed default.
    ///
    /// A face the host rasterized itself is Unicode-keyed by construction. A face read from a
    /// handset is keyed by the device's units, which the file does not identify; the loader that
    /// knows what it read says so here.
    pub fn with_key_space(mut self, key_space: KeySpace) -> Self {
        self.key_space = key_space;
        self
    }

    /// A copy with every ink pixel extended one column to the right, within its own cell.
    ///
    /// This is the host's synthetic bold for a CJK face whose source font has no bold cut — the
    /// same one-column thickening the built-in grid's bold applies to its five-bit rows. Only a
    /// version-2 single-cell face is dilated: a variable-width face is not laid out on a fixed
    /// grid, and a pixel at one cell's edge must not bleed into its neighbour.
    pub fn emboldened(&self) -> Bbf {
        let Widths::Cell(cell) = &self.widths else {
            return self.clone();
        };
        let cell = *cell as usize;
        if cell == 0 {
            return self.clone();
        }
        let width = self.header.width as usize;
        let height = self.header.height as usize;
        let pitch = self.pitch;
        let mut bytes = self.bitmap.to_vec();
        let cols = width / cell;
        for row in 0..height {
            let base = row * pitch;
            for col in 0..cols {
                let x0 = col * cell;
                let x1 = (x0 + cell).min(width);
                if x1 <= x0 + 1 {
                    continue;
                }
                // Right to left, so a pixel this pass sets is not itself a source.
                for x in (x0..x1 - 1).rev() {
                    if bytes[base + x / 8] & (0x80 >> (x % 8)) != 0 {
                        bytes[base + (x + 1) / 8] |= 0x80 >> ((x + 1) % 8);
                    }
                }
            }
        }
        let mut out = self.clone();
        out.bitmap = bytes.into();
        out
    }

    /// The glyph a code unit maps to, or `None` when the range table has no entry for it.
    /// This is `CharToGlyph` from `AEEBitFont.c`, including its two cursor entries. A code unit
    /// the table does not name is *not* an error yet: the caller substitutes
    /// [`Header::undef_glyph`], which is what the delivered `GetGlyph` does.
    pub fn glyph_index(&self, ch: u16) -> Option<u16> {
        let mut next: u32 = 0;
        let mut frozen = false;
        for (i, range) in self.ranges.iter().enumerate() {
            match range.count {
                INDEX_RESET => {
                    next = u32::from(range.base);
                    frozen = false;
                }
                INDEX_FREEZE => {
                    next = u32::from(range.base);
                    frozen = true;
                }
                count => {
                    let offset = u32::from(ch).wrapping_sub(u32::from(range.base));
                    if offset < u32::from(count) {
                        return Some(if frozen { next } else { next + offset } as u16);
                    }
                    if !frozen {
                        next = next.saturating_add(u32::from(count));
                    }
                    // A freeze stops the automatic increment for every range after it until a
                    // reset. `CharToGlyph` achieves that by not adding the count; the loop above
                    // already did that, so the flag only has to survive to the lookup.
                    let _ = i;
                }
            }
        }
        None
    }

    /// The cell a code unit draws from, with the undefined-glyph fallback applied.
    pub fn glyph(&self, ch: u16) -> Glyph {
        let index = self
            .glyph_index(ch)
            .unwrap_or(self.header.undef_glyph.min(self.header.glyph_count - 1))
            .min(self.header.glyph_count - 1);
        match &self.widths {
            Widths::Positions(positions) => {
                let left = positions.get(index as usize).copied().unwrap_or(0);
                let right = positions.get(index as usize + 1).copied().unwrap_or(left);
                Glyph {
                    index,
                    x: left,
                    y: 0,
                    width: right.saturating_sub(left),
                    height: self.cell_height,
                }
            }
            Widths::Cell(cell) => Glyph {
                index,
                x: (index & 0xff) * *cell,
                y: (index >> 8) * self.cell_height,
                width: *cell,
                height: self.cell_height,
            },
        }
    }

    /// Columns the code unit advances. Version 2 answers half a cell for a code unit below
    /// `wHalfWidths` — the test is on the *character*, not the glyph index, which is how the
    /// delivered `CHAR_WIDTH` lets one fixed-cell face carry ASCII and ideographs together.
    pub fn advance(&self, ch: u16) -> u16 {
        match &self.widths {
            Widths::Positions(_) => self.glyph(ch).width,
            Widths::Cell(cell) => {
                if ch < self.half_widths {
                    cell / 2
                } else {
                    *cell
                }
            }
        }
    }

    /// Width of a run, without a trailing gap.
    pub fn measure(&self, chars: &[u16]) -> u32 {
        chars.iter().map(|ch| u32::from(self.advance(*ch))).sum()
    }

    /// Whether the pixel at `(x, y)` is ink. `y` counts from the top of the bitmap.
    pub fn bit(&self, x: u16, y: u16) -> bool {
        if x >= self.header.width || y >= self.header.height {
            return false;
        }
        let row = self.header.height - 1 - y;
        let at = row as usize * self.pitch + (x as usize) / 8;
        match self.bitmap.get(at) {
            Some(byte) => byte & (0x80 >> (x % 8)) != 0,
            None => false,
        }
    }

    /// The ink columns of one glyph. A version-2 half-width glyph only occupies the left half of
    /// its cell, even though the cell itself is a full width in the bitmap.
    pub fn ink(&self, ch: u16) -> (Glyph, u16) {
        let glyph = self.glyph(ch);
        let width = match &self.widths {
            Widths::Positions(_) => glyph.width,
            Widths::Cell(cell) if ch < self.half_widths => cell / 2,
            Widths::Cell(cell) => *cell,
        };
        (glyph, width)
    }
}

fn read_header(bytes: &[u8]) -> Header {
    let mut header = Header {
        version: 0,
        flags: 0,
        descent: 0,
        depth: 0,
        width: 0,
        height: 0,
        undef_glyph: 0,
        glyph_count: 0,
        additional: 0,
        range_count: 0,
        width_count: 0,
    };
    header.version = bytes[2];
    header.flags = bytes[3];
    header.descent = bytes[4];
    header.depth = bytes[5];
    let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    header.width = word(6);
    header.height = word(8);
    header.undef_glyph = word(10);
    header.glyph_count = word(12);
    header.additional = word(14);
    header.range_count = word(16);
    header.width_count = word(18);
    header
}

fn read_ranges(bytes: &[u8], count: u16) -> Vec<Range> {
    let mut ranges = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let at = i * 4;
        ranges.push(Range {
            base: u16::from_le_bytes([bytes[at], bytes[at + 1]]),
            count: u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]),
        });
    }
    ranges
}

/// `ReadWidths` from `AEEVarBitFont.c`, which turns a version-0 byte stream into one left edge per
/// glyph plus a sentinel.
fn read_rle(bytes: &[u8], glyph_count: u16) -> Result<Vec<u16>, Error> {
    let count = glyph_count as usize;
    let mut positions = vec![0u16; count + 1];
    let mut index = 0usize;
    let mut current = 0u32;
    let mut total = 0u32;
    let mut at = 0usize;
    while index < count {
        // The delivered loop treats a short stream as zero widths rather than refusing the file;
        // a malformed stream is still a file this reader can describe, and refusing it here would
        // make a face that renders nothing look like a face that cannot be read.
        let byte = bytes.get(at).copied().unwrap_or(0);
        at += 1;
        let repeats = if byte < 128 {
            current = u32::from(byte);
            1usize
        } else if (130..160).contains(&byte) {
            (byte - 128) as usize
        } else if (160..192).contains(&byte) {
            ((byte - 159) as usize) * 32
        } else {
            0
        };
        for _ in 0..repeats {
            if index >= count {
                break;
            }
            total = total.saturating_add(current);
            if total > u32::from(u16::MAX) {
                return Err(Error::BadWidths);
            }
            positions[index + 1] = total as u16;
            index += 1;
        }
        if repeats == 0 && at >= bytes.len() + 1 {
            break;
        }
    }
    Ok(positions)
}

fn round4(n: usize) -> usize {
    (n + 3) & !3
}

#[cfg(test)]
#[path = "test/bbf.rs"]
mod tests;
