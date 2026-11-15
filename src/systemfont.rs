//! The face a host with no built-in ideographs draws with.
//!
//! The 5x8 grid in [`crate::font`] covers ASCII. A Chinese pack's menus are written for the OEM
//! face a handset answers `AEE_FONT_NORMAL` with — one that covers GB2312 — and the pack carries
//! no fallback of its own, so without a wide face every one of those code units is a notdef box.
//! This module takes the whole face from the machine's own font, rasterizes it into the same BBF
//! the pack-side foundry reads, and caches it under `data/fonts/` so a machine pays for it once.
//!
//! The face is one file, not a Latin grid with a wide table bolted on: `--font` already draws a
//! run entirely from a BBF, and the machine's font covers Latin as well, so there is no second
//! metrics rule to keep in step. `AEE_FONT_LARGE` gets its own rasterization at [`LARGE_SIZE`]
//! because a BBF cell is already the size it was drawn at — the format has no scale.
//!
//! It is an approximation and says so: the shapes are Microsoft YaHei's or SimSun's or Noto's,
//! not the handset's, and a real 16x16 OEM face is still the one thing a firmware dump would
//! give us that this cannot.
//!
//! Nothing is vendored. The font stays where the OS put it, the rasterization happens on the
//! machine that owns the copy, and the cache is that machine's own file — which is also why the
//! cache key carries the source's path, size and timestamp: a replaced font gets a new face.

use crate::abi::generated as g;
use crate::bbf;
use crate::font::Face;
use crate::makebbf;
use crate::runtime::Runtime;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cell height the normal face is rasterized at, matching the 16-pixel OEM face the packs lay out
/// against.
pub const SIZE: u16 = 16;

/// Cell height for `AEE_FONT_LARGE`. The built-in grid magnifies, but a BBF cell cannot: the
/// designator's face has to be drawn at its own size.
pub const LARGE_SIZE: u16 = 32;

/// Bumped whenever the encoder changes what a cached face means. It is part of the cache key, so a
/// face made by an older encoder is never read back as if it were this one's.
const CACHE_VERSION: u32 = 2;

/// Install a face under every system designator the caller did not name itself.
///
/// `keep` is the designators an explicit `--font`/`--font-bold`/`--font-large` already filled;
/// those are the caller's choice and are left alone. A designator that stays [`Face::Builtin`]
/// because no font could be read draws boxes for wide code units, and the run says so once rather
/// than leaving a screenshot to explain it.
pub fn install(rt: &mut Runtime, keep: &[u32]) {
    let wanted = [g::AEE_FONT_NORMAL, g::AEE_FONT_BOLD, g::AEE_FONT_LARGE];
    let missing: Vec<u32> = wanted
        .iter()
        .copied()
        .filter(|id| !keep.contains(id))
        .collect();
    if missing.is_empty() {
        return;
    }
    let wants_large = missing.contains(&g::AEE_FONT_LARGE);
    let mut why = Vec::new();
    // A candidate that carries Latin only rasterizes into a perfectly readable face and then
    // answers every ideograph with the undefined glyph. That is the failure this module exists
    // to prevent, so the source face is asked before the rasterizer is paid for: one lookup for
    // a Han or a kana code point is enough to tell a CJK face from a Latin one.
    let mut latin_only = 0usize;
    let mut first_latin_only: Option<String> = None;
    for path in crate::platform::fonts::cjk_font_candidates() {
        if !Path::new(&path).is_file() {
            continue;
        }
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(error) => {
                why.push(format!("{path}: {error}"));
                continue;
            }
        };
        if !source_has_wide_scripts(&data) {
            latin_only += 1;
            first_latin_only.get_or_insert(path.clone());
            continue;
        }
        let normal = match wide_face_from(&path, &data, SIZE) {
            Ok(face) => face,
            Err(error) => {
                why.push(error);
                continue;
            }
        };
        if !face_has_wide_scripts(&normal) {
            why.push(format!("{path}: the rasterized face has no CJK glyph"));
            continue;
        }
        // A 32-pixel cell is four times the pixels; a font that rasterizes at 16 and not at 32
        // still gives a readable large face, so it is used and the run says what happened rather
        // than leaving "large" the same size as "normal" without a word.
        let large = if wants_large {
            match wide_face_from(&path, &data, LARGE_SIZE) {
                Ok(face) => face,
                Err(error) => {
                    rt.display.note(format!(
                        "no {LARGE_SIZE}-pixel large face ({error}); AEE_FONT_LARGE uses the \
                         {SIZE}-pixel face"
                    ));
                    Arc::clone(&normal)
                }
            }
        } else {
            Arc::clone(&normal)
        };
        for id in &missing {
            let face = if *id == g::AEE_FONT_LARGE {
                Arc::clone(&large)
            } else {
                Arc::clone(&normal)
            };
            rt.display.install_face(*id, Face::Bbf(face));
        }
        return;
    }
    if latin_only > 0 {
        why.push(format!(
            "{latin_only} candidate face(s) carry no CJK glyph (first: {})",
            first_latin_only.as_deref().unwrap_or("?")
        ));
    }
    rt.display.note(format!(
        "no CJK face: {}; text outside the built-in 5x8 grid will draw as boxes",
        if why.is_empty() {
            "no candidate font file exists".to_string()
        } else {
            why.join("; ")
        }
    ));
}

/// Whether a source face can draw a script a CJK pack writes in.
///
/// One Han and one kana code point are the whole test on purpose: a face that names neither is a
/// Latin face, and a face that names either is a candidate this host can draw with. Asking for
/// the full repertoire would reject a Chinese-only or a Japanese-only face, and both are useful
/// on the packs that carry them.
fn source_has_wide_scripts(data: &[u8]) -> bool {
    let Ok(face) = ttf_parser::Face::parse(data, 0) else {
        return false;
    };
    [0x4E2Du16, 0x3042].iter().any(|code| {
        face.glyph_index(char::from_u32(u32::from(*code)).unwrap())
            .is_some()
    })
}

/// The same question asked of the rasterized face, as a backstop: a source that passes the
/// test above but whose `--range`/`half_widths` options dropped the wide cells must not be
/// installed as if it covered them.
fn face_has_wide_scripts(face: &bbf::Bbf) -> bool {
    [0x4E2Du16, 0x3042]
        .iter()
        .any(|code| face.glyph_index(*code).is_some())
}

/// The face for one font file at one cell height, from the cache when this machine has already
/// rasterized it.
pub fn wide_face(path: &str, size: u16) -> Result<Arc<bbf::Bbf>, String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    wide_face_from(path, &data, size)
}

/// The same, for a file the caller has already read — the install loop asks the bytes one
/// question before it pays for a rasterization.
fn wide_face_from(path: &str, data: &[u8], size: u16) -> Result<Arc<bbf::Bbf>, String> {
    let cache = cache_file(path, data, size);
    if let Some(file) = cache.as_deref() {
        if let Ok(bytes) = std::fs::read(file) {
            if let Ok(face) = bbf::Bbf::parse(&bytes) {
                return Ok(Arc::new(face));
            }
        }
    }
    // A 16-pixel CJK face is drawn with ~0.4-pixel horizontal strokes (a Song face's design),
    // which a 50% coverage cut erases; 96 keeps them while a thick face like YaHei stays clean.
    // At 32 pixels the same stroke is twice as tall, so 64 — which a Song face needs to keep its
    // bars — is comfortable rather than bold.
    let threshold = if size >= LARGE_SIZE { 64 } else { 96 };
    let options = makebbf::Options {
        size,
        threshold,
        ..makebbf::Options::default()
    };
    let (bytes, _report) = makebbf::from_ttf(data, &options)?;
    if let Some(file) = cache {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Best effort: a cache that cannot be written costs the next start a rasterization, and
        // that is not a reason to have no face now.
        let _ = std::fs::write(file, &bytes);
    }
    bbf::Bbf::parse(&bytes)
        .map(Arc::new)
        .map_err(|e| format!("{path}: {e}"))
}

/// Where this font's rasterized face lives, keyed by everything that would change it.
fn cache_file(path: &str, data: &[u8], size: u16) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let dir = crate::platform::data::program_data_dir()?.join("fonts");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    data.len().hash(&mut hasher);
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .hash(&mut hasher);
    size.hash(&mut hasher);
    CACHE_VERSION.hash(&mut hasher);
    Some(dir.join(format!("cjk-{:016x}.bbf", hasher.finish())))
}

#[cfg(test)]
#[path = "test/systemfont.rs"]
mod tests;
