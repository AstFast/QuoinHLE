//! A 24-bit Windows BMP writer, because that is the one image format a headless run can prove
//! itself with: no dependency, no encoder, and any viewer on any platform opens the result.
//!
//! Only the top-down case is unusual here and it is worth stating why the rows come out reversed.
//! A framebuffer's first row is the top of the screen; a BMP with a positive height is bottom-up.
//! So [`encode`] walks the source rows back to front and leaves the height signed as it found it,
//! which is the whole conversion.

/// Bytes of the file header and the `BITMAPINFOHEADER` this writer emits, i.e. where the pixels
/// start.
const PIXEL_OFFSET: usize = 14 + 40;

/// `rgb` is `width * height * 3` bytes, top row first, in R,G,B order.
pub fn encode(width: u16, height: u16, rgb: &[u8]) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    assert_eq!(
        rgb.len(),
        w * h * 3,
        "a {}x{} image needs {} bytes, not {}",
        width,
        height,
        w * h * 3,
        rgb.len()
    );
    // Each row is padded to a multiple of four bytes, which is the only real work a BMP writer
    // does.
    let row_bytes = (w * 3).next_multiple_of(4);
    let mut out = Vec::with_capacity(PIXEL_OFFSET + row_bytes * h);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((PIXEL_OFFSET + row_bytes * h) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(PIXEL_OFFSET as u32).to_le_bytes());

    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&((row_bytes * h) as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes()); // 72 dpi, both axes
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..h).rev() {
        for x in 0..w {
            let i = (y * w + x) * 3;
            out.extend_from_slice(&[rgb[i + 2], rgb[i + 1], rgb[i]]);
        }
        out.resize(out.len() + row_bytes - w * 3, 0);
    }
    out
}

#[cfg(test)]
#[path = "test/bmp.rs"]
mod tests;
