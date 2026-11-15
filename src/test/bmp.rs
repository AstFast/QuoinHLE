// Moved out of `src/bmp.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

#[test]
fn rows_are_bottom_up_and_padded_to_four_bytes() {
    // Two rows of three pixels: six bytes of BGR plus two of padding per row, which is the
    // only reason the width is three here.
    let rgb: Vec<u8> = [
        10, 20, 30, 40, 50, 60, 70, 80, 90, // row 0
        1, 2, 3, 4, 5, 6, 7, 8, 9, // row 1
    ]
    .to_vec();
    let bmp = encode(3, 2, &rgb);
    assert_eq!(&bmp[..2], b"BM");
    assert_eq!(word(&bmp, 2), bmp.len() as u32, "file size");
    assert_eq!(word(&bmp, 10), 54, "pixel data offset");
    assert_eq!(word(&bmp, 14), 40, "info header size");
    assert_eq!(word(&bmp, 18), 3);
    assert_eq!(word(&bmp, 22), 2, "positive height means bottom-up");
    assert_eq!(word(&bmp, 28), 24);
    // Two rows of 9 stored bytes padded to 12, so the image is 24 bytes, not 18.
    assert_eq!(word(&bmp, 34), 24, "biSizeImage includes row padding");
    assert_eq!(bmp.len(), 54 + 24);
    // The last source row is the first one stored, in BGR.
    assert_eq!(&bmp[54..60], &[3, 2, 1, 6, 5, 4]);
    assert_eq!(&bmp[63..66], &[0, 0, 0], "three bytes of padding per row");
    assert_eq!(&bmp[66..72], &[30, 20, 10, 60, 50, 40]);
}
