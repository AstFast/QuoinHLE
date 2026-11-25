//! CJK font discovery, which is different on every desktop OS.

use std::path::Path;

/// The font files this host looks for, in the order it prefers them.
///
/// A person's own font comes first — the manager's saved choice, then anything in the host's
/// `fonts` directory — a face the build reference in `ui-fonts` comes next, and the OS's own faces
/// last. The order is a preference rather than a filter: `load_cjk_fonts` reads the first one and,
/// when that face turns out to be Latin-only (a bundled webfont usually is), keeps going until it
/// finds one that can draw Chinese and loads that one behind it.
pub(crate) fn cjk_font_candidates() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(path) = crate::platform::font_file() {
        out.push(path);
    }
    out.extend(installed_fonts());
    out.extend(bundled_fonts());
    let builtin: &[&'static str] = if cfg!(windows) {
        &[
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/simhei.ttf",
            "C:/Windows/Fonts/simsun.ttc",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ]
    };
    out.extend(builtin.iter().map(|path| (*path).to_string()));
    let mut seen = std::collections::HashSet::new();
    out.retain(|path| seen.insert(path.clone()));
    out
}

/// Font files a person put in the host's own `fonts` directory, in name order.
///
/// This is the directory the generated BBF caches also live in, and they are deliberately not
/// listed: a `.bbf` is a rasterized output keyed by its source, not a choice someone makes, and
/// the manager's own interface cannot draw from one. What is listed are the files a person can
/// actually pick — a `.ttf`/`.ttc`/`.otf`/`.otc` copied beside the executable, which is how a
/// downloaded family arrives.
fn installed_fonts() -> Vec<String> {
    let Some(root) = crate::platform::data::program_data_dir().map(|dir| dir.join("fonts")) else {
        return Vec::new();
    };
    fonts_in(&root)
}

/// The interface's own faces, under `ui-fonts` in the data directory beside the executable.
///
/// `build.ps1 -Font`/`-FontDir` is what puts anything here, so this is empty for a build that
/// named no face — which is the default, and why an archive without one is still complete. It is
/// a directory of its own rather than `data/fonts` because that one is the *guest's* font
/// choice: a pack may ask for a file there, while these belong to the interface and are never
/// offered to a pack.
fn bundled_fonts() -> Vec<String> {
    let Some(root) = crate::platform::data::program_data_dir().map(|dir| dir.join("ui-fonts"))
    else {
        return Vec::new();
    };
    bundled_fonts_in(&root)
}

/// The bundled faces under one root, with the body weight first.
///
/// A family ships several weights and only one becomes the body face, so a regular one is moved to
/// the front: alphabetical order alone would hand the interface `...-Bold.ttf`, and every string
/// on the page would be bold.
fn bundled_fonts_in(root: &Path) -> Vec<String> {
    let mut out = fonts_in(root);
    out.sort_by(|a, b| face_rank(a).cmp(&face_rank(b)).then_with(|| a.cmp(b)));
    out
}

/// Where a file name sits between "this is the body weight" and "this is a display weight".
///
/// Read only to order a family that ships many weights. Three tiers rather than a yes/no: a name
/// that says `Regular` beats one that says nothing, and both beat `Light` or `Bold` — sorting by
/// name alone would put `...-Bold`, or `...-Light`, in front of the face meant for body text.
fn face_rank(path: &str) -> u8 {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let named_body = ["regular", "book", "roman", "normal"]
        .iter()
        .any(|word| stem.contains(word));
    let off_body = [
        "bold",
        "italic",
        "oblique",
        "light",
        "thin",
        "medium",
        "semibold",
        "extrabold",
        "extralight",
        "black",
        "hairline",
    ]
    .iter()
    .any(|word| stem.contains(word));
    match (named_body, off_body) {
        (true, _) => 0,
        (false, true) => 2,
        (false, false) => 1,
    }
}

/// Whether a face can draw Chinese, Japanese or Korean text.
///
/// Asked before a single face is treated as enough. `ttf_parser` is already a dependency for
/// `--make-bbf`, and this costs one glyph lookup per probe rather than a scan of the file.
///
/// Only the interface asks: a headless build draws no text, so it has nothing to fall back for and
/// this would be dead code with nobody to read it — the same reason `cjk_font_files` is gated.
#[cfg(feature = "ui")]
pub(crate) fn covers_cjk(bytes: &[u8]) -> bool {
    let Ok(face) = ttf_parser::Face::parse(bytes, 0) else {
        return false;
    };
    let probes = ['\u{4e2d}', '\u{65e5}', '\u{d55c}'];
    probes
        .into_iter()
        .any(|probe| face.glyph_index(probe).is_some())
}

/// Every font file under a root, nested included, in name order.
///
/// The walk is shared by the host's `fonts` directory and the interface's bundled `ui-fonts`, so
/// both answer the same question the same way: which files here are faces a person could pick.
fn fonts_in(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 4 || out.len() >= FONT_LIMIT {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push((path, depth + 1));
            } else if is_font_file(&path) {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
    out.sort();
    out.truncate(FONT_LIMIT);
    out
}

/// How many files the page will list, so a directory someone unpacked a font archive into cannot
/// turn the page into a list of thousands of rows.
const FONT_LIMIT: usize = 64;

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ttf" | "ttc" | "otf" | "otc"
            )
        })
}

/// Every font this host would look for, and whether it is on this machine.
///
/// The font page in the library is its only caller, so it rides on the `ui` feature: a headless
/// build has nobody to show a font list to, and listing files it will not use would be work with
/// no reader.
#[cfg(feature = "ui")]
pub(crate) fn cjk_font_files() -> Vec<(String, bool)> {
    cjk_font_candidates()
        .into_iter()
        .map(|path| {
            let found = Path::new(&path).exists();
            (path, found)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hosts_fonts_directory_is_listed_without_the_generated_caches() {
        // A directory of its own, so the walk is proven without reading the machine's fonts.
        let root = std::env::temp_dir().join(format!(
            "quoin-fonts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let nested = root
            .join("family")
            .join("fonts")
            .join("complete")
            .join("ttf");
        std::fs::create_dir_all(&nested).expect("a scratch directory");
        for path in [
            root.join("a.ttf"),
            root.join("B.TTC"),
            nested.join("c.otf"),
            root.join("cjk-0123456789abcdef.bbf"),
            root.join("notes.txt"),
        ] {
            std::fs::write(&path, b"x").expect("a scratch file");
        }

        let listed = fonts_in(&root);

        assert_eq!(
            listed,
            vec![
                root.join("B.TTC").to_string_lossy().into_owned(),
                root.join("a.ttf").to_string_lossy().into_owned(),
                nested.join("c.otf").to_string_lossy().into_owned(),
            ],
            "three fonts, nested included, sorted; the cache and the text file are not fonts"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_font_file_is_recognised_by_its_extension_not_its_name() {
        for name in ["a.ttf", "B.TTC", "c.otf", "d.Otc"] {
            assert!(is_font_file(Path::new(name)), "{name}");
        }
        for name in ["cjk-0123.bbf", "notes.txt", "ttf", "x.ttf.bak"] {
            assert!(!is_font_file(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn a_regular_weight_outranks_a_named_one_and_a_bare_family_name() {
        // Three tiers, because a family ships weights in both directions from the body face.
        for (name, rank) in [
            ("IBMPlexSans-Regular.ttf", 0),
            ("NotoSansCJK-Book.otf", 0),
            ("DejaVuSans.ttf", 1),
            ("IBMPlexSans-Light.otf", 2),
            ("IBMPlexSans-Bold.ttf", 2),
            ("IBMPlexSans-BoldItalic.ttf", 2),
            ("IBMPlexSans-ExtraLight.ttf", 2),
        ] {
            assert_eq!(face_rank(name), rank, "{name}");
        }
    }

    #[test]
    fn the_bundled_directory_puts_its_regular_face_first() {
        let root = scratch_directory("quoin-ui-fonts");
        for name in [
            "IBMPlexSans-Bold.ttf",
            "IBMPlexSans-Light.otf",
            "IBMPlexSans-Regular.ttf",
        ] {
            std::fs::write(root.join(name), b"x").expect("a scratch file");
        }

        let listed = bundled_fonts_in(&root);

        assert_eq!(
            listed
                .first()
                .map(|path| Path::new(path).file_name().unwrap().to_string_lossy()),
            Some("IBMPlexSans-Regular.ttf".into()),
            "the body weight is first even though `Bold` and `Light` sort before it: {listed:?}"
        );
        assert_eq!(listed.len(), 3, "every face is still listed: {listed:?}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    #[cfg(feature = "ui")]
    fn a_file_that_is_not_a_font_does_not_claim_to_draw_chinese() {
        // No font is committed, so the probe is exercised on the thing it must refuse rather than
        // on a face this test does not own.
        assert!(!covers_cjk(b""));
        assert!(!covers_cjk(b"not a font, just bytes"));
        assert!(!covers_cjk(&[0u8; 512]));
    }

    /// A directory of this test run's own, so nothing reads the machine's real font folders.
    fn scratch_directory(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).expect("a scratch directory");
        root
    }
}
