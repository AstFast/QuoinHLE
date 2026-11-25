// Tests for the BBF reader. The fixtures are hand-built so every offset in the format is
// exercised by a number the test itself wrote, not by a file copied from somewhere else.

use super::*;

fn put_u16(at: &mut Vec<u8>, value: u16) {
    at.extend_from_slice(&value.to_le_bytes());
}

/// A version-0 face: one range, two glyphs of five columns, a 10x8 bitmap.
fn version0() -> Vec<u8> {
    let mut b = vec![b'B', b'F', 0, 0, 1, 1];
    put_u16(&mut b, 10); // wWidth
    put_u16(&mut b, 8); // wHeight
    put_u16(&mut b, 0); // wUndefGlyph
    put_u16(&mut b, 2); // wGlyphCount
    put_u16(&mut b, 0); // cbAdditional
    put_u16(&mut b, 1); // wRangeCount
    put_u16(&mut b, 2); // cbWidths
    assert_eq!(b.len(), HEADER_LEN);
    put_u16(&mut b, u16::from(b'A')); // range base
    put_u16(&mut b, 2); // range count
    b.extend_from_slice(&[5, 5]); // two widths
    b.extend_from_slice(&[0, 0]); // pad the widths section to four bytes
    let mut bitmap = vec![0u8; 32];
    bitmap[7 * 4] = 0x80; // top row, leftmost pixel
    b.extend_from_slice(&bitmap);
    b
}

/// A version-1 face: left edges instead of widths, same two glyphs.
fn version1() -> Vec<u8> {
    let mut b = vec![b'B', b'F', 1, 0, 1, 1];
    put_u16(&mut b, 10);
    put_u16(&mut b, 8);
    put_u16(&mut b, 0);
    put_u16(&mut b, 2);
    put_u16(&mut b, 0);
    put_u16(&mut b, 1);
    put_u16(&mut b, 3); // three positions, including the sentinel
    assert_eq!(b.len(), HEADER_LEN);
    put_u16(&mut b, u16::from(b'A'));
    put_u16(&mut b, 2);
    for edge in [0u16, 5, 10] {
        put_u16(&mut b, edge);
    }
    b.extend_from_slice(&[0, 0]); // pad six position bytes to eight
    let mut bitmap = vec![0u8; 32];
    bitmap[7 * 4] = 0x80;
    b.extend_from_slice(&bitmap);
    b
}

/// A version-2 face: one eight-pixel cell, ASCII half-width, two glyphs on one 256-cell row.
fn version2() -> Vec<u8> {
    let mut b = vec![b'B', b'F', 2, 0, 1, 1];
    put_u16(&mut b, 2048); // 256 cells of eight columns
    put_u16(&mut b, 8); // one row of cells
    put_u16(&mut b, 0); // wUndefGlyph
    put_u16(&mut b, 2); // wGlyphCount
    put_u16(&mut b, 4); // cbAdditional: one AEEMonoData
    put_u16(&mut b, 1);
    put_u16(&mut b, 1); // cbWidths: one cell-width entry
    assert_eq!(b.len(), HEADER_LEN);
    put_u16(&mut b, 0x80); // wHalfWidths
    put_u16(&mut b, 0); // wReserved
    put_u16(&mut b, u16::from(b'A'));
    put_u16(&mut b, 2);
    put_u16(&mut b, 8); // cell width
    b.extend_from_slice(&[0, 0]); // pad the widths section
    let mut bitmap = vec![0u8; 256 * 8];
    bitmap[7 * 256] = 0x80; // first cell, top row
    bitmap[7 * 256 + 1] = 0x80; // second cell, top row
    b.extend_from_slice(&bitmap);
    b
}

#[test]
fn a_version_0_face_reads_widths_metrics_and_ink() {
    let face = Bbf::parse(&version0()).expect("a well-formed version-0 file");
    assert_eq!(face.header().version, 0);
    assert_eq!(face.height(), 8);
    assert_eq!(face.ascent(), 7);
    assert_eq!(face.descent(), 1);
    assert_eq!(face.glyph_count(), 2);

    let a = face.glyph(u16::from(b'A'));
    assert_eq!((a.index, a.x, a.y, a.width, a.height), (0, 0, 0, 5, 8));
    let b = face.glyph(u16::from(b'B'));
    assert_eq!((b.index, b.x, b.width), (1, 5, 5));
    assert_eq!(face.advance(u16::from(b'A')), 5);
    assert_eq!(face.measure(&[u16::from(b'A'), u16::from(b'B')]), 10);

    assert!(face.bit(0, 0), "the fixture's ink is at the top-left");
    assert!(!face.bit(1, 0));
    assert!(!face.bit(0, 1));
}

#[test]
fn a_version_1_face_reads_left_edges() {
    let face = Bbf::parse(&version1()).expect("a well-formed version-1 file");
    assert_eq!(face.header().version, 1);
    let a = face.glyph(u16::from(b'A'));
    assert_eq!((a.index, a.x, a.width), (0, 0, 5));
    let b = face.glyph(u16::from(b'B'));
    assert_eq!((b.index, b.x, b.width), (1, 5, 5));
    assert_eq!(face.advance(u16::from(b'B')), 5);
}

#[test]
fn a_version_2_face_wraps_at_256_cells_and_halves_ascii() {
    let face = Bbf::parse(&version2()).expect("a well-formed version-2 file");
    assert_eq!(face.header().version, 2);
    assert_eq!(face.height(), 8, "one row of cells");
    assert_eq!(face.ascent(), 7);

    let a = face.glyph(u16::from(b'A'));
    assert_eq!((a.index, a.x, a.y, a.width), (0, 0, 0, 8));
    let b = face.glyph(u16::from(b'B'));
    assert_eq!((b.index, b.x, b.y, b.width), (1, 8, 0, 8));

    // The half-width test is on the character value, not the glyph index.
    assert_eq!(face.advance(u16::from(b'A')), 4);
    assert_eq!(face.advance(0x4e00), 8);
    assert_eq!(face.ink(u16::from(b'A')).1, 4);
    assert_eq!(face.ink(0x4e00).1, 8);

    // A code unit the ranges do not name falls back to wUndefGlyph, which is cell zero.
    let undef = face.glyph(0x4e00);
    assert_eq!((undef.index, undef.x, undef.y), (0, 0, 0));

    assert!(face.bit(0, 0));
    assert!(face.bit(8, 0), "the second cell starts at eight columns");
}

#[test]
fn reset_and_freeze_move_the_glyph_cursor() {
    // Two ordinary glyphs, a reset to glyph 5, a freeze at glyph 2, then three characters that
    // must all resolve to the frozen index.
    let mut b = vec![b'B', b'F', 1, 0, 1, 1];
    put_u16(&mut b, 40);
    put_u16(&mut b, 8);
    put_u16(&mut b, 0);
    put_u16(&mut b, 8);
    put_u16(&mut b, 0);
    put_u16(&mut b, 4);
    put_u16(&mut b, 9);
    assert_eq!(b.len(), HEADER_LEN);
    for (base, count) in [
        (u16::from(b'A'), 2u16),
        (5, INDEX_RESET),
        (2, INDEX_FREEZE),
        (0x50, 3),
    ] {
        put_u16(&mut b, base);
        put_u16(&mut b, count);
    }
    for edge in [0u16, 5, 10, 15, 20, 25, 30, 35, 40] {
        put_u16(&mut b, edge);
    }
    b.extend_from_slice(&[0, 0]);
    b.extend_from_slice(&vec![0u8; 64]);

    let face = Bbf::parse(&b).expect("a well-formed file with cursor entries");
    assert_eq!(face.glyph_index(u16::from(b'A')), Some(0));
    assert_eq!(face.glyph_index(u16::from(b'B')), Some(1));
    assert_eq!(face.glyph_index(u16::from(b'C')), None);
    assert_eq!(face.glyph_index(0x50), Some(2));
    assert_eq!(face.glyph_index(0x52), Some(2));
}

#[test]
fn a_file_that_is_not_a_bbf_is_refused_by_name() {
    let mut bad_cookie = version0();
    bad_cookie[0] = b'X';
    assert_eq!(Bbf::parse(&bad_cookie).unwrap_err(), Error::BadCookie);

    let mut bad_depth = version0();
    bad_depth[5] = 2;
    assert_eq!(Bbf::parse(&bad_depth).unwrap_err(), Error::BadDepth);

    let mut bad_version = version0();
    bad_version[2] = 3;
    assert_eq!(
        Bbf::parse(&bad_version).unwrap_err(),
        Error::UnsupportedVersion
    );

    let mut short = version0();
    short.truncate(10);
    assert_eq!(Bbf::parse(&short).unwrap_err(), Error::Truncated);

    let mut bad_mono = version2();
    bad_mono[14] = 0; // cbAdditional
    assert_eq!(Bbf::parse(&bad_mono).unwrap_err(), Error::BadAdditional);
}

#[test]
fn a_range_table_that_runs_past_the_file_is_refused() {
    let mut bad = version0();
    bad[16..18].copy_from_slice(&100u16.to_le_bytes()); // wRangeCount
    assert_eq!(Bbf::parse(&bad).unwrap_err(), Error::BadRanges);
}

#[test]
fn a_width_stream_that_ends_early_is_refused() {
    let mut bad = version1();
    bad.truncate(HEADER_LEN + 4); // the left-edge stream needs six bytes
    assert_eq!(Bbf::parse(&bad).unwrap_err(), Error::BadWidths);
}

#[test]
fn a_bitmap_that_does_not_fit_the_file_is_refused() {
    let mut bad = version0();
    bad.truncate(40); // the 10x8 bitmap needs 32 bytes after column 28
    assert_eq!(Bbf::parse(&bad).unwrap_err(), Error::BadBitmap);
}

#[test]
fn a_face_with_no_glyphs_is_refused() {
    let mut bad = version2();
    bad[12..14].copy_from_slice(&0u16.to_le_bytes()); // wGlyphCount
    assert_eq!(Bbf::parse(&bad).unwrap_err(), Error::NoGlyphs);
}

#[test]
fn an_undefined_code_point_uses_the_fallback_cell() {
    let face = Bbf::parse(&version2()).unwrap();
    assert_eq!(face.glyph_index(0xffff), None);
    assert_eq!(face.glyph(0xffff).index, 0, "the undefined-glyph fallback");
    assert_eq!(face.advance(0xffff), 8, "a full cell");
    assert_eq!(face.glyph(u16::from(b'B')).index, 1);
    assert_eq!(face.advance(u16::from(b'A')), 4, "ASCII is half a cell");
}

#[test]
fn a_face_is_unicode_keyed_unless_its_loader_says_otherwise() {
    let parsed = Bbf::parse(&version2()).expect("a well-formed version-2 file");
    assert_eq!(parsed.key_space(), KeySpace::Unicode);
    let device = parsed.with_key_space(KeySpace::Device);
    assert_eq!(device.key_space(), KeySpace::Device);
}

/// The synthetic bold used for a CJK face with no bold cut: every ink pixel grows one column,
/// and the growth stops at its own cell's edge.
#[test]
fn a_single_cell_face_thickens_inside_its_cells() {
    let face = Bbf::parse(&version2()).expect("a well-formed version-2 file");
    assert!(
        face.bit(0, 0),
        "the fixture's first cell has ink at its left edge"
    );
    assert!(!face.bit(1, 0));

    let bold = face.emboldened();
    assert!(bold.bit(1, 0), "the pixel grew one column to the right");
    assert!(bold.bit(9, 0), "and the second cell's pixel did too");
    assert!(!bold.bit(2, 0), "but not two columns");
    assert!(!bold.bit(10, 0));
    assert_eq!(
        bold.advance(0x41),
        face.advance(0x41),
        "the cell's advance is unchanged"
    );
}
