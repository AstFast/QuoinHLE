// The encoder is tested against the reader in `crate::bbf`: a file this module writes is only
// correct if the delivered parser accepts it and finds the cells where the ranges say they are.

use super::*;
use crate::bbf::Bbf;

fn cell(ch: u16, ink: bool) -> Cell {
    let side = 16 * 16;
    let mut bits = vec![false; side];
    if ink {
        bits[0] = true;
    }
    Cell {
        ch,
        width: 16,
        ink: bits,
    }
}

#[test]
fn the_encoder_writes_a_version_2_face_the_reader_accepts() {
    let cells = vec![
        cell(0x20, false),
        cell(0x41, true),
        cell(0x42, false),
        cell(0x4E00, false),
    ];
    let bytes = encode(&cells, 16, 4, 0x80);
    let face = Bbf::parse(&bytes).expect("the encoder's own output");
    assert_eq!(face.header().version, 2);
    assert_eq!(face.glyph_count(), 4);
    assert_eq!(face.height(), 16);
    assert_eq!(face.ascent(), 12);
    assert_eq!(face.descent(), 4);

    // The half-width rule is on the character, which is what lets one fixed cell carry ASCII and
    // ideographs: `A` is below 0x80, `一` is not.
    assert_eq!(face.advance(0x41), 8);
    assert_eq!(face.advance(0x4E00), 16);

    let a = face.glyph(0x41);
    assert_eq!((a.index, a.x, a.y), (1, 16, 0));
    let han = face.glyph(0x4E00);
    assert_eq!((han.index, han.x, han.y), (3, 48, 0));

    // Cell 1's first ink bit is the top-left pixel of its cell, one column past the first cell.
    assert!(face.bit(16, 0));
    assert!(!face.bit(17, 0));
}

#[test]
fn the_variable_encoder_writes_left_edges_and_advances() {
    let mut a = vec![false; 4 * 16];
    a[0] = true;
    let cells = vec![
        Cell {
            ch: 0x41,
            width: 4,
            ink: a,
        },
        Cell {
            ch: 0x42,
            width: 6,
            ink: vec![false; 6 * 16],
        },
        Cell {
            ch: 0x4E00,
            width: 8,
            ink: vec![false; 8 * 16],
        },
    ];
    let bytes = encode_variable(&cells, 16, 4).expect("the widths fit");
    let face = Bbf::parse(&bytes).expect("the encoder's own output");
    assert_eq!(face.header().version, 1);
    assert_eq!(face.glyph_count(), 3);

    // Each glyph keeps its own left edge and advance; the rows are one continuous strip.
    let a = face.glyph(0x41);
    assert_eq!((a.x, a.width), (0, 4));
    let b = face.glyph(0x42);
    assert_eq!((b.x, b.width), (4, 6));
    let han = face.glyph(0x4E00);
    assert_eq!((han.x, han.width), (10, 8));
    assert_eq!(face.advance(0x41), 4);
    assert_eq!(face.advance(0x42), 6);
    assert_eq!(face.advance(0x4E00), 8);
    assert!(face.bit(0, 0));

    // The one-row bitmap has a `u16` width, so a repertoire that overflows it is refused rather
    // than wrapped.
    let too_wide = vec![
        Cell {
            ch: 0x41,
            width: 65535,
            ink: Vec::new(),
        },
        Cell {
            ch: 0x42,
            width: 1,
            ink: Vec::new(),
        },
    ];
    assert!(encode_variable(&too_wide, 16, 4).is_err());
}

#[test]
fn a_run_of_code_points_becomes_one_range_and_a_gap_becomes_two() {
    let cells = vec![cell(0x41, false), cell(0x42, false), cell(0x4E00, false)];
    let bytes = encode(&cells, 8, 2, 0x80);
    let face = Bbf::parse(&bytes).expect("the encoder's own output");
    assert_eq!(face.glyph_index(0x41), Some(0));
    assert_eq!(face.glyph_index(0x42), Some(1));
    assert_eq!(face.glyph_index(0x43), None);
    assert_eq!(face.glyph_index(0x4E00), Some(2));
}
