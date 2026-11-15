//! Turn a system true-type/OpenType face into the bitmap container the guest already reads.
//!
//! [`crate::bbf`] is the reader: it parses the four sections the delivered `*_NewFromBBF` bodies
//! parse and answers glyph rectangles out of them. This module is the other direction — the tool
//! that produces such a file from a font a person actually has, so the host can supply a face
//! with the coverage a Simplified Chinese, Japanese or Korean pack expects.
//!
//! The output is **version 2** on purpose. Versions 0 and 1 put every glyph on one bitmap row and
//! describe that row with a `u16` width, which caps the repertoire at roughly four thousand
//! sixteen-pixel glyphs; a CJK face does not fit. Version 2 wraps at 256 glyphs per row and is a
//! single-cell face, with ASCII half-width by code point (`IS_HALF_WIDTH` in `AEEMonoBitFont.c`
//! tests `ch < wHalfWidths`, not the glyph index).
//!
//! The encoder takes rasterized cells rather than a font, so its tests can build one without a
//! system font on disk. [`from_ttf`] is the half that needs `ttf-parser` and a coverage
//! rasterizer; both are pure Rust.

use ab_glyph_rasterizer::{point, Point, Rasterizer};

/// Coverage a squeezed half-width outline needs to keep a pixel. A 16-pixel glyph compressed into
/// an 8-pixel cell has its strokes at well under half a pixel of coverage, so the face's own
/// threshold would erase them; this is the lowest value that keeps a Latin letter whole.
const HALF_THRESHOLD: f32 = 64.0 / 255.0;

/// What [`from_ttf`] is asked to produce.
#[derive(Debug, Clone)]
pub struct Options {
    /// Cell height in pixels, and the cell width of a full-width glyph.
    pub size: u16,
    /// Face index inside a TrueType Collection (`.ttc`); zero for a plain `.ttf`/`.otf`.
    pub face_index: u32,
    /// Code points below this are half-width. `0x80` keeps ASCII half-width and every CJK
    /// character full-width, which is the rule the delivered single-width body applies.
    pub half_widths: u16,
    /// Code-point ranges to keep; empty means every BMP code point the face names. BBF's range
    /// table is `u16`, so anything above `0xFFFF` is outside the format regardless.
    pub ranges: Vec<(u16, u16)>,
    /// Coverage at or above this (0..=255) becomes ink. One bit per pixel leaves no grey, so this
    /// is the only knob between a thin face and a heavy one.
    pub threshold: u8,
    /// 1 for the variable-width layout (one row, a `u16` left edge per glyph) or 2 for the
    /// single-cell layout (256 glyphs per row). Version 1 is the only one that expresses a
    /// per-glyph advance, and its `wWidth` is a `u16`, so it is for small repertoires.
    pub version: u8,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            size: 16,
            face_index: 0,
            half_widths: 0x80,
            ranges: Vec::new(),
            threshold: 128,
            version: 2,
        }
    }
}

/// What one conversion kept, and what the container could not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub glyphs: usize,
    /// Distinct code points above `0xFFFF` that the cmap names. BBF's range base is a `u16`, so
    /// these cannot be represented; the count is reported rather than silently dropped.
    pub outside_bmp: usize,
}

/// One rasterized character cell: a code point, its advance in pixels, and `width * size` ink
/// bits, top row first. Version 2 uses a fixed `width == size`; version 1 uses the face's own
/// advance for each cell.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: u16,
    pub width: u16,
    pub ink: Vec<bool>,
}

/// Read a font and return the bytes of a version-2 BBF whose cells are `options.size` square.
///
/// Every BMP code point the face's cmap names is kept, in code-point order. A code point with no
/// outline (a space) still gets a cell, because its advance is part of the face; a code point the
/// cmap does not name is left out rather than blank-filled, which is the same convention the
/// committed CJK tables use.
pub fn from_ttf(data: &[u8], options: &Options) -> Result<(Vec<u8>, Report), String> {
    if options.version != 1 && options.version != 2 {
        return Err(format!(
            "version is 1 (variable) or 2 (single cell), not {}",
            options.version
        ));
    }
    let face = ttf_parser::Face::parse(data, options.face_index)
        .map_err(|e| format!("not a readable TrueType/OpenType font: {e}"))?;
    let size = options.size.clamp(4, 255);
    let units = f32::from(face.units_per_em());
    if units <= 0.0 {
        return Err("the font header has no unitsPerEm".into());
    }
    // Scale by the face's own line height, not by `unitsPerEm`: a 16-pixel cell is 16 pixels from
    // the top of the ascender to the bottom of the descender, and a CJK face whose em box is taller
    // than that would otherwise rasterize past the cell and overflow the rasterizer.
    let ascender = f32::from(face.ascender());
    let descender = f32::from(face.descender());
    let span = ascender - descender;
    if span <= 0.0 {
        return Err("the font header has no ascender/descender".into());
    }
    let scale = f32::from(size) / span;
    let baseline = ascender * scale;
    let descent = (f32::from(size) - baseline)
        .round()
        .clamp(1.0, f32::from(size)) as u16;

    let mut cells = Vec::new();
    let threshold = f32::from(options.threshold) / 255.0;
    let wanted = |cp: u32| {
        options.ranges.is_empty()
            || options
                .ranges
                .iter()
                .any(|(lo, hi)| cp >= u32::from(*lo) && cp <= u32::from(*hi))
    };
    for cp in 0x20u32..=0xFFFF {
        if cp == 0x7F || !wanted(cp) {
            continue;
        }
        let Some(ch) = char::from_u32(cp) else {
            continue;
        };
        let Some(gid) = face.glyph_index(ch) else {
            continue;
        };
        let advance_units = face.glyph_hor_advance(gid).map_or(units, f32::from);
        // Version 1 expresses each glyph's own advance; version 2 uses the square cell, with the
        // half-width rule applied by the reader at draw time.
        let cell_w = if options.version == 1 {
            (advance_units * scale).round().max(1.0) as u16
        } else {
            size
        };
        // A version-2 half-width cell is only `size / 2` wide when it is drawn, so the glyph has
        // to be fitted into that much — rasterising it at full width and cutting the cell in half
        // is what clipped every wide Latin letter. Fitting is horizontal only: the vertical scale
        // is the face's, and the ink is centred in whatever width it was given.
        let target = if options.version == 2 && cp < u32::from(options.half_widths) {
            f32::from(size) / 2.0
        } else {
            f32::from(cell_w)
        };
        let advance_px = advance_units * scale;
        let x_scale = if advance_px > target && advance_px > 0.0 {
            scale * target / advance_px
        } else {
            scale
        };
        // What gets centred is the advance box; the ink sits inside it the way its own font
        // placed it, side bearing and all (a CJK glyph's margin lives in that bearing).
        let x_origin = ((target - advance_units * x_scale) / 2.0).max(0.0);
        // An embedded bitmap strike is the face's own pixel design at this size — hinted, snapped
        // and hand-tuned — and it is what a 16-pixel CJK face is actually made of. Use it when it
        // was drawn at exactly this ppem: a strike at another size is a different glyph, not a
        // scaled one, and it would be the wrong size in the cell.
        let mut ink = face
            .glyph_raster_image(gid, size)
            .filter(|image| image.pixels_per_em == size)
            .and_then(|image| strike_ink(&image, target as u16, cell_w, size, baseline));
        if ink.is_none() {
            let mut outline = Outline::new(cell_w, size, x_scale, scale, x_origin, baseline);
            // A glyph without a contour is a legal character (space, a combining mark); the cell
            // is still emitted, blank, so the advance exists.
            face.outline_glyph(gid, &mut outline);
            let mut bits = vec![false; usize::from(cell_w) * usize::from(size)];
            // A half-width cell squeezes the glyph to half its advance, which thins every
            // diagonal and vertical; the outline is binarized against a lower coverage there so a
            // letter keeps its strokes instead of turning into dots.
            let limit = if target < f32::from(cell_w) {
                HALF_THRESHOLD
            } else {
                threshold
            };
            outline.raster.for_each_pixel_2d(|x, y, alpha| {
                if alpha < limit {
                    return;
                }
                let at = y as usize * usize::from(cell_w) + x as usize;
                if let Some(pixel) = bits.get_mut(at) {
                    *pixel = true;
                }
            });
            ink = Some(bits);
        }
        let ink = ink.expect("a strike or an outline always produces a cell");
        cells.push(Cell {
            ch: cp as u16,
            width: cell_w,
            ink,
        });
    }
    if cells.is_empty() {
        return Err("no character in the selected range".into());
    }
    if cells.len() > usize::from(u16::MAX) {
        return Err(format!(
            "{} glyphs exceed BBF's u16 limit ({})",
            cells.len(),
            u16::MAX
        ));
    }
    // Code points the range table cannot address. Counted rather than silently dropped: a person
    // who pointed this at a font with CJK extension characters should know why they are absent.
    let mut outside = std::collections::HashSet::new();
    if let Some(cmap) = face.tables().cmap {
        for subtable in cmap.subtables {
            subtable.codepoints(|cp| {
                if cp > 0xFFFF {
                    outside.insert(cp);
                }
            });
        }
    }
    let report = Report {
        glyphs: cells.len(),
        outside_bmp: outside.len(),
    };
    let bytes = if options.version == 1 {
        encode_variable(&cells, size, descent)?
    } else {
        encode(&cells, size, descent, options.half_widths)
    };
    Ok((bytes, report))
}

/// Emit a version-2 BBF from already-rasterized cells. `cells` must be sorted by code point and
/// every `ink` must be `size * size` bits.
pub fn encode(cells: &[Cell], size: u16, descent: u16, half_widths: u16) -> Vec<u8> {
    let cell_w = size;
    let cell_h = size;
    let rows = cells.len().div_ceil(256);
    let width = 256u32 * u32::from(cell_w);
    let height = rows as u32 * u32::from(cell_h);
    let pitch = width as usize / 8;
    let mut bitmap = vec![0u8; pitch * height as usize];

    for (index, cell) in cells.iter().enumerate() {
        let cx = (index & 0xFF) * usize::from(cell_w);
        let cy = (index >> 8) * usize::from(cell_h);
        let ink_w = if cell.ch < half_widths {
            usize::from(cell_w / 2)
        } else {
            usize::from(cell_w)
        };
        for y in 0..usize::from(cell_h) {
            for x in 0..ink_w {
                if !cell
                    .ink
                    .get(y * usize::from(cell_w) + x)
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }
                let px = cx + x;
                let py = cy + y;
                // The bitmap is stored bottom-up, the way a Windows DIB is.
                let file_row = height as usize - 1 - py;
                bitmap[file_row * pitch + px / 8] |= 0x80 >> (px % 8);
            }
        }
    }

    let ranges = ranges(cells);

    let mut out = Vec::with_capacity(20 + 8 + ranges.len() * 4 + 4 + bitmap.len());
    out.extend_from_slice(b"BF");
    out.push(2); // byVersion: single cell, 256 glyphs per row
    out.push(0); // byFlags
    out.push(descent as u8);
    out.push(1); // byDepth: one bit per pixel
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // wUndefGlyph: the first cell
    out.extend_from_slice(&(cells.len() as u16).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes()); // cbAdditional: one AEEMonoData
    out.extend_from_slice(&(ranges.len() as u16).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // cbWidths: one cell-width byte
                                                // AEEMonoData
    out.extend_from_slice(&half_widths.to_le_bytes());
    out.extend_from_slice(&cell_w.to_le_bytes());
    for (base, count) in &ranges {
        out.extend_from_slice(&base.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
    }
    out.push(cell_w as u8);
    out.extend_from_slice(&[0, 0, 0]); // pad the widths section to four bytes
    out.extend_from_slice(&bitmap);
    out
}

/// Emit a version-1 BBF: one bitmap row, a `u16` left edge per glyph, and the face's own advance
/// in the width table. `wWidth` is a `u16`, so a repertoire whose advances sum past 65 535 does
/// not fit; the caller is told to narrow the range or use version 2.
pub fn encode_variable(cells: &[Cell], size: u16, descent: u16) -> Result<Vec<u8>, String> {
    let mut positions = Vec::with_capacity(cells.len() + 1);
    let mut total = 0u32;
    positions.push(0u16);
    for cell in cells {
        total += u32::from(cell.width);
        if total > u32::from(u16::MAX) {
            return Err(format!(
                "version 1 needs one row of at most 65535 pixels; these advances sum to {total} — \
                 narrow --range or use --version 2"
            ));
        }
        positions.push(total as u16);
    }
    if total == 0 {
        return Err("no glyphs to encode".into());
    }
    let width = total as u16;
    let height = size;
    let pitch = ((usize::from(width) + 31) / 32) * 4;
    let mut bitmap = vec![0u8; pitch * usize::from(height)];
    for (index, cell) in cells.iter().enumerate() {
        let x0 = usize::from(positions[index]);
        for y in 0..usize::from(height) {
            for x in 0..usize::from(cell.width) {
                if !cell
                    .ink
                    .get(y * usize::from(cell.width) + x)
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }
                let px = x0 + x;
                // Bottom-up, like the version-2 bitmap and like a Windows DIB.
                let file_row = usize::from(height) - 1 - y;
                bitmap[file_row * pitch + px / 8] |= 0x80 >> (px % 8);
            }
        }
    }

    let ranges = ranges(cells);
    let mut out =
        Vec::with_capacity(20 + ranges.len() * 4 + positions.len() * 2 + 4 + bitmap.len());
    out.extend_from_slice(b"BF");
    out.push(1); // byVersion: variable width, one bitmap row
    out.push(0); // byFlags
    out.push(descent as u8);
    out.push(1); // byDepth: one bit per pixel
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // wUndefGlyph: the first glyph
    out.extend_from_slice(&(cells.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // cbAdditional
    out.extend_from_slice(&(ranges.len() as u16).to_le_bytes());
    out.extend_from_slice(&(positions.len() as u16).to_le_bytes()); // cbWidths entries
    for (base, count) in &ranges {
        out.extend_from_slice(&base.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
    }
    for position in &positions {
        out.extend_from_slice(&position.to_le_bytes());
    }
    out.extend_from_slice(&vec![0u8; (4 - (positions.len() * 2) % 4) % 4]);
    out.extend_from_slice(&bitmap);
    Ok(out)
}

/// One range per run of consecutive code points. The glyph cursor advances by each range's count,
/// so the runs line up with the cells' own indices without a RESET entry; a run longer than
/// `0xFFFD` is split because `0xFFFE`/`0xFFFF` are the cursor commands.
fn ranges(cells: &[Cell]) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < cells.len() {
        let mut end = start + 1;
        while end < cells.len()
            && u32::from(cells[end].ch) == u32::from(cells[end - 1].ch) + 1
            && end - start < 0xFFFD
        {
            end += 1;
        }
        out.push((cells[start].ch, (end - start) as u16));
        start = end;
    }
    out
}

/// A `ttf_parser::OutlineBuilder` that scales font units into one cell and draws them with the
/// coverage rasterizer. Font y grows upwards and the rasterizer's grows downwards, so every point
/// is reflected about the baseline once here rather than in the encoder.
struct Outline {
    raster: Rasterizer,
    /// Horizontal and vertical pixels per font unit. They differ only for a glyph that was fitted
    /// into a narrower cell than its own advance asked for — the half-width Latin of a version-2
    /// face — where squeezing is what keeps the letter whole instead of cutting it in half.
    x_scale: f32,
    y_scale: f32,
    /// Where the glyph's own left edge lands: its left side bearing removed, then the advance
    /// box centred in the cell.
    x_origin: f32,
    baseline: f32,
    width: f32,
    height: f32,
    start: Point,
    current: Point,
}

impl Outline {
    fn new(
        width: u16,
        height: u16,
        x_scale: f32,
        y_scale: f32,
        x_origin: f32,
        baseline: f32,
    ) -> Self {
        Outline {
            raster: Rasterizer::new(usize::from(width), usize::from(height)),
            x_scale,
            y_scale,
            x_origin,
            baseline,
            width: f32::from(width),
            height: f32::from(height),
            start: point(0.0, 0.0),
            current: point(0.0, 0.0),
        }
    }

    /// A point in the font's units, reflected about the baseline and clamped to the cell. The
    /// rasterizer indexes a fixed grid, so a contour that reaches past the cell has to be cut here
    /// rather than at an overflowing array write inside it.
    fn at(&self, x: f32, y: f32) -> Point {
        let px = (self.x_origin + x * self.x_scale).clamp(0.0, self.width - 0.001);
        let py = (self.baseline - y * self.y_scale).clamp(0.0, self.height - 0.001);
        point(px, py)
    }
}

/// One glyph from an embedded bitmap strike, placed in the cell the way its own bearings say.
///
/// The strike is the face's pixel design at this ppem — hinted and stem-snapped by whoever drew
/// it — so nothing here rasterizes anything except a gray strike's own coverage. `None` means the
/// strike cannot be used as it stands: wider than the cell it would have to fit (a full-width
/// strike for a half-width cell), or a format this does not read. The caller falls back to the
/// outline then.
fn strike_ink(
    image: &ttf_parser::RasterGlyphImage,
    target: u16,
    cell_w: u16,
    size: u16,
    baseline: f32,
) -> Option<Vec<bool>> {
    let w = usize::from(image.width);
    let h = usize::from(image.height);
    if w == 0 || h == 0 {
        return None;
    }
    let alpha = |x: usize, y: usize| -> Option<u8> {
        let data = image.data;
        Some(match image.format {
            ttf_parser::RasterImageFormat::BitmapMonoPacked => {
                let bit = y * w + x;
                let ink = data
                    .get(bit / 8)
                    .is_some_and(|b| b & (0x80 >> (bit % 8)) != 0);
                if ink {
                    255
                } else {
                    0
                }
            }
            ttf_parser::RasterImageFormat::BitmapMono => {
                let pitch = (w + 7) / 8;
                let ink = data
                    .get(y * pitch + x / 8)
                    .is_some_and(|b| b & (0x80 >> (x % 8)) != 0);
                if ink {
                    255
                } else {
                    0
                }
            }
            ttf_parser::RasterImageFormat::BitmapGray8 => *data.get(y * w + x).unwrap_or(&0),
            ttf_parser::RasterImageFormat::BitmapGray4Packed => {
                let bit = (y * w + x) * 4;
                let b = *data.get(bit / 8)?;
                ((b >> (4 - bit % 8)) & 0xf) * 17
            }
            ttf_parser::RasterImageFormat::BitmapGray2Packed => {
                let bit = (y * w + x) * 2;
                let b = *data.get(bit / 8)?;
                ((b >> (6 - bit % 8)) & 0x3) * 85
            }
            _ => return None,
        })
    };
    // The bitmap's bottom sits `y` pixels above the baseline, so its top row is
    // `baseline - (y + height)` in the cell's own top-down rows.
    let bearing = f32::from(image.y) + f32::from(image.height);
    let top = (baseline - bearing).round() as i32;
    // Centring the bitmap in the cell is the same answer an ideograph's own bearing gives, and it
    // is the safe one for a half-width cell. A strike wider than the cell it must fit (a 9-pixel
    // Latin strike in an 8-pixel half cell) is sampled down column by column rather than cut: the
    // clash is between two of the face's own rules, and dropping one column keeps the letter.
    let target = usize::from(target);
    let fit = w.min(target);
    let left = ((target - fit) / 2) as i32;
    let mut ink = vec![false; usize::from(cell_w) * usize::from(size)];
    for row in 0..h {
        let y = top + row as i32;
        if y < 0 || y >= i32::from(size) {
            continue;
        }
        for col in 0..fit {
            let src = col * w / fit;
            let x = left + col as i32;
            if x < 0 || x >= i32::from(cell_w) {
                continue;
            }
            if alpha(src, row)? >= 128 {
                ink[y as usize * usize::from(cell_w) + x as usize] = true;
            }
        }
    }
    Some(ink)
}

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.start = p;
        self.current = p;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.raster.draw_line(self.current, p);
        self.current = p;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let control = self.at(x1, y1);
        let p = self.at(x, y);
        self.raster.draw_quad(self.current, control, p);
        self.current = p;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c1 = self.at(x1, y1);
        let c2 = self.at(x2, y2);
        let p = self.at(x, y);
        self.raster.draw_cubic(self.current, c1, c2, p);
        self.current = p;
    }

    fn close(&mut self) {
        self.raster.draw_line(self.current, self.start);
        self.current = self.start;
    }
}

#[cfg(test)]
#[path = "test/makebbf.rs"]
mod tests;
