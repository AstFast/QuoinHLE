//! Tests for the face built from the machine's own font. The rasterizer needs a real font, so
//! what is pinned here without one is the face's metrics over a synthetic BBF: one file answers
//! Latin and ideographs, and the two widths come from the format's half-width rule.

use super::*;
use crate::encoding::TextEncoding;
use crate::font::Face;
use crate::makebbf::Cell;

/// A face with one ideograph whose ink sits in the middle of its cell, so a test can tell where
/// the cell landed without knowing which way the bitmap rows run.
fn cjk_face(size: u16) -> Arc<bbf::Bbf> {
    let ink_at = usize::from(size / 2) * usize::from(size) + usize::from(size / 2);
    let mut ink = vec![false; usize::from(size) * usize::from(size)];
    ink[ink_at] = true;
    let blank = vec![false; usize::from(size) * usize::from(size)];
    let cells = vec![
        Cell {
            ch: 0x20,
            width: size,
            ink: blank.clone(),
        },
        Cell {
            ch: 0x41,
            width: size,
            ink: blank,
        },
        Cell {
            ch: 0x4E2D,
            width: size,
            ink,
        },
    ];
    let bytes = makebbf::encode(&cells, size, size / 4, 0x80);
    Arc::new(bbf::Bbf::parse(&bytes).expect("the encoder's own output"))
}

#[test]
fn one_rasterized_face_answers_latin_and_ideographs_at_their_own_widths() {
    let face = Face::Bbf(cjk_face(SIZE));
    assert_eq!(
        face.advance_of_char('\u{4e2d}', TextEncoding::Unicode),
        16,
        "an ideograph is a full cell"
    );
    assert_eq!(
        face.advance_of_char('A', TextEncoding::Unicode),
        8,
        "Latin is half of one, by code point"
    );
    assert_eq!(face.height(), 16, "the cell a pack lays lines out by");
    assert_eq!(
        face.scale(),
        1,
        "a cell is already the size it was drawn at"
    );
    assert_eq!(
        face.measure_text("A\u{4e2d}", TextEncoding::Unicode),
        8 + 16,
        "a BBF face measures a run as the sum of its own advances; its cell already carries the gap"
    );
}

/// `AEE_FONT_LARGE` has no magnification available: the designator's face is rasterized at its own
/// size, so its metrics come from that file and not from the normal one.
#[test]
fn the_large_designator_comes_from_its_own_rasterization() {
    let face = Face::Bbf(cjk_face(LARGE_SIZE));
    assert_eq!(face.height(), 32);
    assert_eq!(face.advance_of_char('\u{4e2d}', TextEncoding::Unicode), 32);
    assert_eq!(face.advance_of_char('A', TextEncoding::Unicode), 16);
}

/// The cell the synthetic face carries is where the face says it is: the advance and the bitmap
/// are the same object, so a caller that steps by `advance_of` cannot drift.
#[test]
fn the_cell_is_what_the_face_parsed() {
    let cjk = cjk_face(SIZE);
    let glyph = cjk.glyph(0x4E2D);
    assert_eq!(cjk.advance(0x4E2D), 16);
    assert!(
        cjk.bit(glyph.x + 8, glyph.y + 8),
        "the cell's own ink, halfway across"
    );
    assert!(
        !cjk.bit(glyph.x, glyph.y),
        "and nothing where the cell is blank"
    );
    assert_eq!(cjk.advance(0x41), 8, "ASCII is half-width on this face");
}

/// The whole path a run takes: a real font file on this machine → a BBF face with ideographs in
/// it. Named through the environment like the sample set audio samples, because a machine without the
/// font is not a failure of this host.
#[test]
fn a_named_system_font_rasterizes_into_a_face() {
    let Ok(path) = std::env::var("QUOIN_FONT") else {
        eprintln!("skipped: QUOIN_FONT is not set");
        return;
    };
    let face = Face::Bbf(wide_face(&path, SIZE).expect("the font named by QUOIN_FONT rasterizes"));
    assert!(
        source_has_wide_scripts(&std::fs::read(&path).expect("read QUOIN_FONT")),
        "a face selected for the guest must name a CJK code point"
    );
    assert_eq!(face.height(), u32::from(SIZE));
    assert_eq!(
        face.advance_of_char('\u{4e2d}', TextEncoding::Unicode),
        u32::from(SIZE),
        "the face has a full-width cell for a CJK character"
    );
    assert_eq!(
        face.advance_of_char('A', TextEncoding::Unicode),
        u32::from(SIZE) / 2,
        "and a half-width one for A"
    );

    let large = Face::Bbf(wide_face(&path, LARGE_SIZE).expect("the large face rasterizes"));
    assert_eq!(large.height(), u32::from(LARGE_SIZE));
    assert_eq!(
        large.advance_of_char('A', TextEncoding::Unicode),
        u32::from(LARGE_SIZE) / 2,
        "the large designator is its own file, not a magnified one"
    );
}

/// A Latin face rasterizes into a readable face and then answers every ideograph with the
/// undefined glyph. The install loop must be able to tell the two apart *before* it pays for a
/// rasterization, which is what the synthetic faces here pin.
#[test]
fn a_face_with_only_latin_is_not_a_wide_face() {
    let blank = vec![false; usize::from(SIZE) * usize::from(SIZE)];
    let bytes = makebbf::encode(
        &[Cell {
            ch: 0x41,
            width: SIZE,
            ink: blank,
        }],
        SIZE,
        SIZE / 4,
        0x80,
    );
    let latin = bbf::Bbf::parse(&bytes).expect("the encoder's own output");
    assert!(!face_has_wide_scripts(&latin), "A alone is not a CJK face");
    assert!(
        face_has_wide_scripts(&cjk_face(SIZE)),
        "the face with a CJK character is"
    );
}
