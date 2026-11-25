//! The built-in face: a 5x8 pixel grid per character, plus the metrics that fall out of it.
//!
//! A real handset gets its glyphs from the MIF: `IDisplay::DrawText` takes an `AEEFont` naming one
//! of the OEM's built-in faces, and anything else lives in a resource the shell loads on request.
//! This module implements the first half of that — one face, the four `AEEFont`
//! ids it can serve, and nothing pretending to the rest. An id with no face here is refused by the
//! caller rather than drawn with the wrong one, because a silently substituted font is how an
//! emulator ends up "working" while every screenshot is wrong.
//!
//! ## The grid
//!
//! Eight rows, the top seven above the baseline and the eighth below it, so `ascent + descent` is
//! the whole cell height exactly as `IDisplay_GetFontMetrics` documents it. Five columns, and one
//! column of leading between characters, so the advance is six.
//!
//! Each row is five bits with **bit 4 as the leftmost pixel** — the same order the plot loop walks,
//! which is what makes the table readable: `0b10001, 0b10001, 0b11111, …` *is* an `H`. Writing the
//! rows as binaries rather than bytes is deliberate; a hex table cannot be proofread.
//!
//! The shapes are hand-authored from the grid, not taken from any handset's font resource.

use crate::abi::generated as g;
use crate::bbf;
use crate::encoding::TextEncoding;
use crate::makebbf::{self, Cell};
use std::sync::Arc;
use std::sync::OnceLock;

/// What one code unit draws as, from the built-in face.
///
/// One shape, because the 5x8 grid is all [`Font`] itself holds: a wide code unit has nothing here
/// and draws [`BOX`]. [`Face`] is what puts a wide bitmap behind it.
pub enum Shape {
    /// The 5x8 face: [`ROWS`] rows of five bits, the leftmost pixel in bit 4 of each.
    Narrow([u8; ROWS]),
}

/// Ink columns per character.
pub const GLYPH_W: usize = 5;
/// Rows per character cell, including the one below the baseline.
pub const GLYPH_H: usize = 8;
/// Pixels between characters, so [`ADVANCE`] is the whole step.
pub const LEADING: usize = 1;
/// Columns a character occupies, ink plus leading.
pub const ADVANCE: usize = GLYPH_W + LEADING;
/// Rows above the baseline.
pub const ASCENT: usize = GLYPH_H - 1;
/// Rows below it.
pub const DESCENT: usize = 1;

const ROWS: usize = GLYPH_H;

/// `0x20` through `0x7e`. Everything outside that range draws [`BOX`], which is what a handset
/// does with a code unit its face has no glyph for — and the reason a bad string conversion is
/// visible in a screenshot instead of missing from it.
const GLYPHS: [[u8; ROWS]; 95] = [
    [0, 0, 0, 0, 0, 0, 0, 0],                                     // (space)
    [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0, 0b00100], // !
    [0b01010, 0b01010, 0, 0, 0, 0, 0, 0],                         // "
    [0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0, 0], // #
    [
        0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100, 0,
    ], // $
    [0b11001, 0b11010, 0b00100, 0b01011, 0b10011, 0, 0, 0],       // %
    [
        0b01000, 0b10100, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101, 0,
    ], // &
    [0b00100, 0b00100, 0, 0, 0, 0, 0, 0],                         // '
    [
        0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010, 0,
    ], // (
    [
        0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000, 0,
    ], // )
    [0, 0b00100, 0b10101, 0b11111, 0b10101, 0b00100, 0, 0],       // *
    [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0, 0],       // +
    [0, 0, 0, 0, 0, 0b00100, 0b00100, 0b01000],                   // ,
    [0, 0, 0, 0b11111, 0, 0, 0, 0],                               // -
    [0, 0, 0, 0, 0, 0b00100, 0, 0],                               // .
    [
        0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000, 0,
    ], // /
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110, 0,
    ], // 0
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0,
    ], // 1
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111, 0,
    ], // 2
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110, 0,
    ], // 3
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010, 0,
    ], // 4
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110, 0,
    ], // 5
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110, 0,
    ], // 6
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0,
    ], // 7
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110, 0,
    ], // 8
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100, 0,
    ], // 9
    [0, 0, 0b00100, 0, 0, 0b00100, 0, 0],                         // :
    [0, 0, 0b00100, 0, 0, 0b00100, 0b00100, 0b01000],             // ;
    [
        0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010, 0,
    ], // <
    [0, 0, 0b11111, 0, 0b11111, 0, 0, 0],                         // =
    [
        0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000, 0,
    ], // >
    [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0, 0b00100, 0], // ?
    [
        0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110, 0,
    ], // @
    [
        0b00100, 0b01010, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0,
    ], // A
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110, 0,
    ], // B
    [
        0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110, 0,
    ], // C
    [
        0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100, 0,
    ], // D
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111, 0,
    ], // E
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0,
    ], // F
    [
        0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111, 0,
    ], // G
    [
        0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0,
    ], // H
    [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0,
    ], // I
    [
        0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100, 0,
    ], // J
    [
        0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001, 0,
    ], // K
    [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111, 0,
    ], // L
    [
        0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001, 0,
    ], // M
    [
        0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0,
    ], // N
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0,
    ], // O
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000, 0,
    ], // P
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101, 0,
    ], // Q
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001, 0,
    ], // R
    [
        0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110, 0,
    ], // S
    [
        0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0,
    ], // T
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0,
    ], // U
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0,
    ], // V
    [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b01010, 0,
    ], // W
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001, 0,
    ], // X
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100, 0,
    ], // Y
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111, 0,
    ], // Z
    [
        0b00110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b00110, 0,
    ], // [
    [
        0b10000, 0b10000, 0b01000, 0b00100, 0b00010, 0b00010, 0b00001, 0,
    ], // (backslash)
    [
        0b01100, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01100, 0,
    ], // ]
    [0b00100, 0b01010, 0b10001, 0, 0, 0, 0, 0],                   // ^
    [0, 0, 0, 0, 0, 0, 0, 0b11111],                               // _
    [0b01000, 0b00100, 0, 0, 0, 0, 0, 0],                         // `
    [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111, 0],       // a
    [
        0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110, 0,
    ], // b
    [0, 0, 0b01110, 0b10001, 0b10000, 0b10001, 0b01110, 0],       // c
    [
        0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111, 0,
    ], // d
    [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110, 0],       // e
    [
        0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000, 0,
    ], // f
    [0, 0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010], // g
    [
        0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0,
    ], // h
    [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0], // i
    [
        0b00010, 0, 0b00110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00100,
    ], // j
    [
        0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0,
    ], // k
    [
        0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0,
    ], // l
    [0, 0, 0b11110, 0b10101, 0b10101, 0b10101, 0b10101, 0],       // m
    [0, 0, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0],       // n
    [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110, 0],       // o
    [0, 0, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000], // p
    [0, 0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001], // q
    [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000, 0],       // r
    [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110, 0],       // s
    [
        0b00100, 0b00100, 0b11110, 0b00100, 0b00100, 0b00101, 0b00010, 0,
    ], // t
    [0, 0, 0b10001, 0b10001, 0b10001, 0b10001, 0b01111, 0],       // u
    [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0],       // v
    [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010, 0],       // w
    [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0],       // x
    [0, 0, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010], // y
    [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111, 0],       // z
    [
        0b00110, 0b01000, 0b01000, 0b10000, 0b01000, 0b01000, 0b00110, 0,
    ], // {
    [
        0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0,
    ], // |
    [
        0b01100, 0b00010, 0b00010, 0b00001, 0b00010, 0b00010, 0b01100, 0,
    ], // }
    [0, 0, 0b01100, 0b10011, 0, 0, 0, 0],                         // ~
];

/// Hollow box, plotted for every code unit outside [`GLYPHS`].
const BOX: [u8; ROWS] = [0, 0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111, 0];

/// The 5x8 grid encoded once into the same BBF container as external faces.
///
/// The hand-authored bits above are the source, but every read after the first goes through
/// `bbf::Bbf`. That keeps one decoder for built-in ASCII, built-in CJK and `--font`/foundry faces.
fn ascii_face() -> &'static bbf::Bbf {
    static FACE: OnceLock<bbf::Bbf> = OnceLock::new();
    FACE.get_or_init(|| {
        let cells: Vec<Cell> = GLYPHS
            .iter()
            .enumerate()
            .map(|(i, rows)| {
                let mut ink = Vec::with_capacity(ADVANCE * ROWS);
                for y in 0..ROWS {
                    for x in 0..ADVANCE {
                        ink.push(x < GLYPH_W && rows[y] & (1 << (GLYPH_W - 1 - x)) != 0);
                    }
                }
                Cell {
                    ch: 0x20 + i as u16,
                    width: ADVANCE as u16,
                    ink,
                }
            })
            .collect();
        let bytes = makebbf::encode_variable(&cells, ROWS as u16, DESCENT as u16)
            .expect("the built-in ASCII grid is a valid variable BBF");
        bbf::Bbf::parse(&bytes).expect("the encoded ASCII grid parses")
    })
}

/// One `AEEFont` this host can actually serve: a face, a scale, and a heavier stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Font {
    /// The `AEEFont` value this face was resolved from, for reporting.
    pub id: u32,
    /// Integer pixel magnification of the grid. `AEE_FONT_LARGE` is the same face at 2.
    pub scale: u32,
    /// Strokes thickened one pixel to the right, which is how a handset's bold differs from its
    /// normal face at this size.
    pub bold: bool,
}

impl Font {
    /// The face for an `AEEFont`, or `None` for an id this host has no glyphs for — including the
    /// two user-defined slots, whose real answers come from a MIF resource this project does not
    /// read.
    ///
    /// The three italic designators are answered with the upright face of the same size and weight.
    /// This host has no slanted cut, and a handset with a missing private face falls back to the
    /// designator's own grid rather than refusing to draw; the caller still gets a working
    /// `IFont`, which is what a pack that only wants to measure or draw its text needs.
    pub const fn of(nfont: u32) -> Option<Font> {
        let (scale, bold) = match nfont {
            g::AEE_FONT_NORMAL | g::AEE_FONT_ITALIC => (1, false),
            g::AEE_FONT_BOLD | g::AEE_FONT_BOLD_ITALIC => (1, true),
            g::AEE_FONT_LARGE | g::AEE_FONT_LARGE_ITALIC => (2, false),
            _ => return None,
        };
        Some(Font {
            id: nfont,
            scale,
            bold,
        })
    }

    /// Columns one character of the 5x8 face steps forward.
    pub const fn advance(self) -> u32 {
        (ADVANCE * self.scale as usize) as u32
    }

    /// Columns one code unit steps forward on the 5x8 face alone.
    ///
    /// A run with a CJK face behind it answers through [`Face::advance_of`] instead: a 16x16
    /// ideograph is not a magnified `H`, and a handset that has both steps the wide one by its own
    /// advance.
    pub fn advance_of(self, _ch: u16) -> u32 {
        self.advance()
    }

    /// The width a run of Unicode text occupies, trailing leading excluded.
    ///
    /// This is the host's own text — a clock, an HTML page — so its characters are code points and
    /// no device encoding is involved. Guest `AECHAR` runs go through [`Face::measure_text`], which
    /// is given the device's encoding.
    pub fn measure_text(self, text: &str) -> u32 {
        let units: Vec<u16> = text
            .chars()
            .map(|ch| bmp_key(ch).unwrap_or(u16::MAX))
            .collect();
        self.measure_chars(&units)
    }

    /// The width a run of characters occupies, trailing leading excluded — so one `H` is five
    /// pixels wide and an empty run is nothing, which is what an alignment calculation has to see.
    ///
    /// Per character rather than `count * advance`: the sample set is mixed, and a Chinese menu whose
    /// every code unit was measured as a Latin one comes out at a fifth of its real width - which
    /// is what a caller then centres and clips by.
    pub fn measure_chars(self, chars: &[u16]) -> u32 {
        let mut total = 0u32;
        let mut last = 0u32;
        for ch in chars {
            let step = self.advance_of(*ch);
            total += step;
            last = step;
        }
        let leading = if chars.is_empty() {
            0
        } else {
            LEADING as u32 * self.scale
        };
        total.saturating_sub(last.min(leading))
    }

    /// The shape one code unit draws as on the 5x8 grid alone.
    ///
    /// A code unit outside [`GLYPHS`] draws [`BOX`], which is what a face with no glyph for it
    /// does. A run that installed a BBF under the designator never reaches this — that face is
    /// the one `paint_glyph` reads.
    pub fn shape(self, ch: u16) -> Shape {
        Shape::Narrow(self.rows(ch))
    }

    /// The full cell height: `ascent + descent`, which is what `IDisplay_GetFontMetrics` returns.
    ///
    /// The width and height of the tallest face this font answers with, not the narrow one alone.
    /// A handset's `AEE_FONT_NORMAL` on a Chinese phone is a 16x16 CJK face, and a pack that lays its
    /// text out itself steps x by 17 (a 16-wide cell plus one leading column) and expects
    /// `GetFontMetrics` to describe the same face. Answering 8, the height of an ASCII face, puts a
    /// pack's two menu lines one pixel apart, so they overlap. A pack carries no font of its own, so
    /// there is no BDF to copy a header from — what it does carry is that 17-pixel step, which is the
    /// handset's face speaking.
    pub fn height(self) -> u32 {
        (GLYPH_H * self.scale as usize) as u32
    }

    /// The rows above the baseline: one less than the cell, as the ASCII face has it.
    pub fn ascent(self) -> u32 {
        (ASCENT * self.scale as usize) as u32
    }

    pub fn descent(self) -> u32 {
        (DESCENT * self.scale as usize) as u32
    }

    /// The width `n` characters occupy, trailing leading excluded — so one `H` is five pixels wide
    /// and `measure(0)` is nothing, which is what an alignment calculation has to see. The leading
    /// scales with the face, because a magnified cell has a magnified gap after it.
    pub const fn measure(self, n: usize) -> u32 {
        if n == 0 {
            return 0;
        }
        let leading = LEADING as u32 * self.scale;
        self.advance() * n as u32 - leading
    }

    /// The eight rows for one code unit, most significant bit of each row being its leftmost
    /// pixel. Never `None`: an unknown code unit answers with [`BOX`].
    pub fn rows(self, ch: u16) -> [u8; ROWS] {
        let face = ascii_face();
        let mut rows = match face.glyph_index(ch) {
            Some(_) => {
                let glyph = face.glyph(ch);
                let mut out = [0u8; ROWS];
                for y in 0..ROWS as u16 {
                    for x in 0..GLYPH_W as u16 {
                        if face.bit(glyph.x.saturating_add(x), glyph.y.saturating_add(y)) {
                            out[y as usize] |= 1 << (GLYPH_W - 1 - x as usize);
                        }
                    }
                }
                out
            }
            None => BOX,
        };
        if self.bold {
            for r in rows.iter_mut() {
                *r |= *r >> 1;
            }
        }
        rows
    }
}

/// The upright designator an italic one falls back to, or `None` for any other id.
const fn upright_designator(nfont: u32) -> Option<u32> {
    match nfont {
        g::AEE_FONT_ITALIC => Some(g::AEE_FONT_NORMAL),
        g::AEE_FONT_BOLD_ITALIC => Some(g::AEE_FONT_BOLD),
        g::AEE_FONT_LARGE_ITALIC => Some(g::AEE_FONT_LARGE),
        _ => None,
    }
}

/// A character as the `u16` a Unicode-keyed face can address, or `None` above the BMP.
fn bmp_key(ch: char) -> Option<u16> {
    let cp = ch as u32;
    (cp <= u32::from(u16::MAX)).then_some(cp as u16)
}

/// A resolved face: one of the built-in grids, or a bitmap font decoded from a BBF file.
///
/// The built-in ASCII grid is encoded once into [`bbf::Bbf`]; this variant only keeps the
/// `AEE_FONT` id, scale and bold policy. Every glyph pixel the display reads comes from a
/// `bbf::Bbf`, whether the face was built in, loaded with `--font`, made by the foundry, or
/// rasterized from the machine's own font by [`crate::systemfont`].
#[derive(Clone, Debug)]
pub enum Face {
    Builtin(Font),
    Bbf(Arc<bbf::Bbf>),
    // A layered face used to sit here: the built-in 5x8 grid for Latin with a rasterized BBF
    // behind it for everything else. The installed face is now the rasterized one whole — Latin
    // included — so nothing constructs this variant and it is kept only as a record of the
    // alternative (its arms are written out at the end of `impl Face`).
    //
    // Mixed {
    //     text: Font,
    //     cjk: Arc<bbf::Bbf>,
    // },
}

impl Face {
    /// The built-in face a designator names, when this host has one.
    pub fn builtin(nfont: u32) -> Option<Face> {
        Font::of(nfont).map(Face::Builtin)
    }

    /// The gap this face leaves out of a measured run's tail.
    ///
    /// A built-in grid ends a run one leading column short of the sum of its advances — that column
    /// is the space between words, not part of the last one. A BBF cell is exactly as wide as it
    /// advances, and [`bbf::Bbf::measure`] adds the whole sum, so nothing is taken off. The two
    /// have to be named here rather than in a caller: whoever folds a line and whoever measures it
    /// must agree to the pixel, and this is the only difference between the arms.
    pub fn trailing_leading(&self) -> u32 {
        match self {
            Face::Builtin(font) => LEADING as u32 * font.scale,
            Face::Bbf(_) => 0,
        }
    }

    /// Full cell height, which is what `GetFontMetrics` reports as ascent plus descent.
    pub fn height(&self) -> u32 {
        match self {
            Face::Builtin(font) => font.height(),
            Face::Bbf(face) => u32::from(face.height()),
        }
    }

    /// Rows above the baseline.
    pub fn ascent(&self) -> u32 {
        match self {
            Face::Builtin(font) => font.ascent(),
            Face::Bbf(face) => u32::from(face.ascent()),
        }
    }

    /// Rows below the baseline.
    pub fn descent(&self) -> u32 {
        match self {
            Face::Builtin(font) => font.descent(),
            Face::Bbf(face) => u32::from(face.descent()),
        }
    }

    /// Columns one character steps forward.
    ///
    /// `enc` is the encoding the run arrived in. A Unicode-keyed face looks the character up by
    /// its code point; a handset's own face is keyed by `AECHAR` units, so the character is
    /// converted to the device's unit first. A character the device cannot name still advances by
    /// the face's undefined-glyph width, which is what a handset does with it.
    pub fn advance_of_char(&self, ch: char, enc: TextEncoding) -> u32 {
        match self {
            Face::Builtin(font) => font.advance_of(bmp_key(ch).unwrap_or(u16::MAX)),
            Face::Bbf(face) => {
                let key = self.glyph_key(ch, enc).unwrap_or(u16::MAX);
                u32::from(face.advance(key))
            }
        }
    }

    /// The width `text` occupies in this face, without the trailing gap.
    ///
    /// Host text — an HTML page, a clock — passes [`TextEncoding::Unicode`] because its characters
    /// are already code points. A guest `AECHAR` run passes the device's encoding, which is what
    /// lets a face dumped from a handset answer the units it is actually keyed by.
    pub fn measure_text(&self, text: &str, enc: TextEncoding) -> u32 {
        if text.is_empty() {
            return 0;
        }
        let mut total = 0u32;
        let mut last = 0u32;
        for ch in text.chars() {
            let step = self.advance_of_char(ch, enc);
            total += step;
            last = step;
        }
        total.saturating_sub(last.min(self.trailing_leading()))
    }

    /// The key this face's glyph table is addressed by for `ch`.
    ///
    /// `None` when the character has no key in the face's space: a non-BMP character on a Unicode
    /// face, or a character the device encoding cannot name on a handset-keyed face. Both draw the
    /// face's undefined glyph, which is the handset's own answer.
    pub fn glyph_key(&self, ch: char, enc: TextEncoding) -> Option<u16> {
        match self {
            Face::Builtin(_) => bmp_key(ch),
            Face::Bbf(face) => match face.key_space() {
                bbf::KeySpace::Unicode => bmp_key(ch),
                bbf::KeySpace::Device => enc.char_to_unit(ch),
            },
        }
    }

    /// Integer magnification of the built-in grid. A BBF cell is already the size it was drawn
    /// at, so its scale is one.
    pub fn scale(&self) -> u32 {
        match self {
            Face::Builtin(font) => font.scale,
            Face::Bbf(_) => 1,
        }
    }

    /// A copy with every ink pixel one column wider, which is what this host can offer a CJK face
    /// under `AEE_FONT_BOLD` when the source font has no bold cut. The built-in grid already has a
    /// bold policy of its own; this returns that.
    pub fn emboldened(&self) -> Face {
        match self {
            Face::Builtin(font) => Face::Builtin(Font {
                bold: true,
                ..*font
            }),
            Face::Bbf(face) => Face::Bbf(Arc::new(face.emboldened())),
        }
    }

    // ── The layered face, kept for reference ────────────────────────────────────────────────
    //
    // `Face::Mixed { text: Font, cjk: Arc<bbf::Bbf> }` was the first answer to "the built-in grid
    // has no ideographs": the 5x8 grid kept answering Latin so a menu laid out against the old
    // metrics did not move, and the rasterized BBF answered the rest. The installed face is now
    // the rasterized one whole — Latin included — so nothing builds `Mixed`. These are its arms,
    // should it come back:
    //
    //   height()       u32::from(cjk.height()) * text.scale
    //   ascent()       text.ascent()
    //   descent()      text.descent()
    //   advance_of()   text.advance_of(ch) when the grid has ch, else
    //                  (u32::from(cjk.advance(ch)) + LEADING as u32) * text.scale
    //   scale()        text.scale
    //   measure_chars() the sum of those advances, minus `LEADING * text.scale` when the run
    //                   ends on a code unit the grid has
    //
    // Its two helpers were `Font::has_glyph(ch)` (`0x20..=0x7e`) and
    // `wide_advance(cjk, ch, scale) = (advance + LEADING) * scale`. In the display it plotted the
    // grid's rows for Latin and, for the rest, the BBF cell magnified by `text.scale` — the one
    // magnification a BBF cell cannot do on its own, which is why `AEE_FONT_LARGE` now gets a face
    // rasterized at its own size instead.
}

/// What a font designator measures as: the faces a display has installed, backed by the built-in
/// table.
///
/// There is exactly one answer to "which face does `AEE_FONT_NORMAL` mean" and it lives on the
/// display ([`crate::display::Display::face`]) — an installed face first, the built-in grid after.
/// Layout used to answer it for itself with [`Font::of`], which is only the built-in half: a run
/// measured against the 5x8 grid and painted with a handset's 16-pixel CJK face wraps at the wrong
/// word, leaves the page too short to scroll to its own bottom, and draws a `<textarea>` whose
/// lines overlap. Handing layout the display's answer is what keeps measuring and painting the same
/// number.
///
/// It is a borrowed table rather than a clone of it so that a frame does not copy a map to lay a
/// page out; the built-in table needs no storage at all.
#[derive(Clone, Copy, Default)]
pub struct Faces<'a> {
    installed: Option<&'a std::collections::HashMap<u32, Face>>,
}

impl<'a> Faces<'a> {
    /// The built-in table alone, for a caller with no display behind it.
    pub const fn builtin() -> Self {
        Faces { installed: None }
    }

    /// A display's installed faces, with the built-in table behind them.
    pub const fn over(installed: &'a std::collections::HashMap<u32, Face>) -> Self {
        Faces {
            installed: Some(installed),
        }
    }

    /// The face a designator names: the installed one first, the built-in table after — the same
    /// order, and the same answer, as [`crate::display::Display::face`]. An italic designator
    /// falls back to the installed upright face of the same weight and size before the built-in
    /// grid, so a CJK face installed under `AEE_FONT_NORMAL` still answers italic text.
    pub fn face(self, nfont: u32) -> Option<Face> {
        let installed = self.installed;
        installed
            .and_then(|faces| faces.get(&nfont))
            .cloned()
            .or_else(|| {
                let upright = upright_designator(nfont)?;
                installed.and_then(|faces| faces.get(&upright)).cloned()
            })
            .or_else(|| Face::builtin(nfont))
    }
}

#[cfg(test)]
#[path = "test/font.rs"]
mod tests;
