// Moved out of `src/font.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

fn ink(font: Font, ch: u16) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for (y, row) in font.rows(ch).iter().enumerate() {
        for x in 0..GLYPH_W as u32 {
            if row & (1 << (GLYPH_W as u32 - 1 - x)) != 0 {
                out.push((x, y as u32));
            }
        }
    }
    out
}

/// Every row is five bits and no wider, or the plot loop would read a pixel that is not there.
#[test]
fn the_table_stays_inside_its_grid() {
    assert_eq!(GLYPHS.len(), 0x7f - 0x20);
    assert_eq!(GLYPHS[0][0], 0, "index 0 has to be the space");
    for (i, glyph) in GLYPHS.iter().enumerate() {
        for row in glyph {
            assert!(*row & !0b11111 == 0, "{:#x} row {row:#010b}", 0x20 + i);
        }
    }
    // No printable character may be indistinguishable from the space, or a run looks empty.
    for ch in 0x21u16..0x7f {
        assert!(
            !ink(Font::of(g::AEE_FONT_NORMAL).unwrap(), ch).is_empty(),
            "{ch:#x} draws nothing"
        );
    }
}

#[test]
fn metrics_follow_the_grid() {
    let normal = Font::of(g::AEE_FONT_NORMAL).unwrap();
    assert_eq!(normal.ascent() + normal.descent(), normal.height());
    // The built-in face is the 5x8 ASCII grid: eight rows, seven above the baseline and one below.
    assert_eq!(normal.height(), 8);
    assert_eq!(normal.ascent(), 7);
    assert_eq!(normal.descent(), 1);
    assert_eq!(normal.measure(0), 0);
    assert_eq!(normal.measure(1), 5, "one glyph, no trailing leading");
    assert_eq!(normal.measure(2), 11, "and the step between them");
    let large = Font::of(g::AEE_FONT_LARGE).unwrap();
    assert_eq!(large.height(), 16, "the same face at 2");
    assert_eq!(large.measure(2), 22, "scaled, not interpolated");
    // The ids with no face here must be refused, not served by the nearest one.
    assert_eq!(Font::of(g::AEE_FONT_ITALIC), None);
    assert_eq!(Font::of(g::AEE_FONT_BOLD_ITALIC), None);
    assert_eq!(Font::of(g::AEE_FONT_USER_1), None);
    assert_eq!(Font::of(0x1234), None);
    assert!(Font::of(g::AEE_FONT_BOLD).unwrap().bold);
}

#[test]
fn a_glyph_reads_as_its_letter() {
    let font = Font::of(g::AEE_FONT_NORMAL).unwrap();
    // Rendered as text, because the only honest test of a font is whether it reads.
    let show = |ch: u16| -> String {
        font.rows(ch)
            .iter()
            .map(|row| {
                (0..GLYPH_W)
                    .map(|x| {
                        if row & (1 << (GLYPH_W - 1 - x)) != 0 {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        show('H' as u16),
        "#...#\n#...#\n#...#\n#####\n#...#\n#...#\n#...#\n....."
    );
    assert_eq!(
        show('o' as u16),
        ".....\n.....\n.###.\n#...#\n#...#\n#...#\n.###.\n....."
    );
    // Bold thickens without growing the cell, and keeps every column the plain face had.
    let bold = Font::of(g::AEE_FONT_BOLD).unwrap();
    for (x, y) in ink(font, 'H' as u16) {
        assert!(ink(bold, 'H' as u16).contains(&(x, y)), "bold lost ink");
    }
    assert!(ink(bold, 'l' as u16).len() > ink(font, 'l' as u16).len());
    // And a code unit with no glyph is visible rather than absent.
    assert_eq!(
        show(0x4e2d),
        ".....\n#####\n#...#\n#...#\n#...#\n#...#\n#####\n....."
    );
}
