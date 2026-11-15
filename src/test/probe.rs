// Moved out of `src/probe.rs` so that file is implementation only. The `#[path]` attribute on the
// `mod tests;` declaration in it is what keeps this a child module of that file, and that is what lets
// these tests reach its private items with `use super::*`.

use super::*;
use crate::loader::LoadError;
use std::path::{Path, PathBuf};

/// A PNG of one flat colour, written the way a pack stores one.
fn png_of(w: u32, h: u32) -> Vec<u8> {
    let px: Vec<u8> = (0..w * h).flat_map(|_| [1u8, 2, 3]).collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&px).unwrap();
    }
    out
}

/// A 24-bit bottom-up BMP.
fn bmp_of(w: u32, h: u32) -> Vec<u8> {
    let stride = (w as usize * 3 + 3) & !3;
    let mut rows = Vec::new();
    for _ in 0..h {
        for _ in 0..w {
            rows.extend_from_slice(&[7, 8, 9]);
        }
        rows.resize(rows.len() + stride - w as usize * 3, 0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"BM");
    for word in [
        (54 + rows.len()) as u32,
        0,
        54,
        40,
        w,
        h,
        0x0018_0001,
        0,
        rows.len() as u32,
        0,
        0,
        0,
        0,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&rows);
    out
}

/// An `AEEAppInfo` as a manifest stores it, at the offsets the ABI table publishes.
fn app_info(cls: u32, id_base: u16, app_type: u16, flags: u16) -> Vec<u8> {
    let mut raw = vec![0u8; 20];
    let at = |member: &str| {
        usize::try_from(abi::offset_of("AEEAppInfo", member).expect("AEEAppInfo is in the tables"))
            .unwrap()
    };
    raw[at("cls")..at("cls") + 4].copy_from_slice(&cls.to_le_bytes());
    raw[at("wIDBase")..at("wIDBase") + 2].copy_from_slice(&id_base.to_le_bytes());
    raw[at("wAppType")..at("wAppType") + 2].copy_from_slice(&app_type.to_le_bytes());
    raw[at("wFlags")..at("wFlags") + 2].copy_from_slice(&flags.to_le_bytes());
    raw
}

/// A `.mif` holding the given records, numbered 1..=n in the order they are given.
///
/// A `RESTYPE_BINARY` record is written the way the container holds one: payload from the first byte,
/// no property block at all (`src/resource.rs`). The other two kinds carry their block.
fn mif(parts: &[(u16, &str, Vec<u8>)]) -> Vec<u8> {
    assert!(
        !parts.is_empty(),
        "an empty container is not what these cases write"
    );
    let bodies: Vec<Vec<u8>> = parts
        .iter()
        .map(|(kind, mime, payload)| {
            if *kind == resource::RESTYPE_BINARY {
                return payload.clone();
            }
            let block = format!("{mime}\0");
            let mut b = Vec::new();
            b.extend_from_slice(&((2 + block.len()) as u16).to_le_bytes());
            b.extend_from_slice(block.as_bytes());
            b.extend_from_slice(payload);
            b
        })
        .collect();
    let (map, entries) = (0x20usize, parts.len());
    let offsets = map + entries * 8;
    let data = offsets + (entries + 1) * 4;
    let mut out = Vec::new();
    out.extend_from_slice(&[0x11, 0]);
    for word in [1u16, 1, entries as u16] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in [
        map as u32,
        (entries * 8) as u32,
        offsets as u32,
        entries as u32,
        data as u32,
        bodies.iter().map(|b| b.len()).sum::<usize>() as u32,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    // One map row per record, each covering one id, so `all` walks them in order.
    for (i, (kind, _, _)) in parts.iter().enumerate() {
        for word in [*kind, (i as u16) + 1, 0u16, i as u16] {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
    let mut at = data;
    out.extend_from_slice(&(at as u32).to_le_bytes());
    for b in &bodies {
        at += b.len();
        out.extend_from_slice(&(at as u32).to_le_bytes());
    }
    for b in &bodies {
        out.extend_from_slice(b);
    }
    out
}

/// Guards the fixture above rather than the module: a header that does not add up would make every
/// case below read as "this pack has no art", which is the same words as a real answer.
#[test]
fn a_container_of_one_row_per_record_is_mounted() {
    // Guards the fixture itself: a header that does not add up makes every case below read as "no art".
    let bytes = mif(&[(resource::RESTYPE_IMAGE, "image/png", png_of(16, 16))]);
    let c = resource::Container::parse(&bytes).expect("the fixture is a container");
    let all = c.all().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, 1);
    assert_eq!(all[0].props, b"image/png\0");
}

/// An image whose first word is a plausible ARM instruction, so the loader has something to take.
fn arm_image() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0xe92d_00f0u32.to_le_bytes());
    out.resize(64, 0);
    out
}

/// A pack laid out the way the corpus lays one out: `<root>/<title>/<appid>/<title>.mod`, its sibling
/// files in that same directory, and the `<appid>.mif` manifest one level above it.
fn pack(
    tag: &str,
    title: &str,
    appid: &str,
    siblings: &[(&str, Vec<u8>)],
    manifest: Option<Vec<u8>>,
) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quoin-probe-{tag}-{}",
        // Two `cargo test` runs in parallel would otherwise delete each other's fixtures.
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(title).join(appid);
    std::fs::create_dir_all(&dir).unwrap();
    let module = dir.join(format!("{title}.mod"));
    std::fs::write(&module, arm_image()).unwrap();
    for (name, bytes) in siblings {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    if let Some(bytes) = manifest {
        std::fs::write(dir.parent().unwrap().join(format!("{appid}.mif")), bytes).unwrap();
    }
    module
}

/// Delete the whole fixture tree. `ancestors` counts the path itself as the first one, so the root is
/// three parents up from the `.mod`: `root/title/appid/title.mod`.
fn cleanup(path: &Path) {
    if let Some(root) = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
    {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// A picture as this module's own type holds it, for the rules that are pure arithmetic.
fn pic(from: &str, id: u32, width: usize, height: usize) -> Picture {
    Picture {
        from: from.to_owned(),
        id,
        width,
        height,
    }
}

#[test]
fn word0_is_read_both_ways_and_the_host_says_which_it_would_use() {
    // The claim this makes is only as strong as the decoder, so it prints both readings rather than one
    // bit test. `push {r4, lr}` — the word every `AEEMod_Load` in this corpus opens with — reads as ARM,
    // and is entered as ARM.
    let e = entry(&0xe92d_00f0u32.to_le_bytes());
    assert_eq!(e.word, 0xe92d_00f0);
    assert!(
        !e.refuses_arm,
        "0xe… is a condition field the loader accepts"
    );
    assert_eq!(e.suggests, "ARM");
    assert!(e.arm.is_some(), "the ARM side refused: {:?}", e.arm);
    // A word whose condition field is `0xf…` is no ARM instruction since ARMv4. That is the one firm
    // statement here, and it is the case `--thumb-entry` exists for.
    let e = entry(&0xf000_0000u32.to_le_bytes());
    assert!(e.refuses_arm);
    assert_ne!(e.suggests, "ARM");
    // Too short to hold a word: the ARM side cannot read at all, a stray halfword still decodes as
    // `movs r0, r0`, and that still does not choose an instruction set — the loader calls three bytes
    // empty, and the report says unknown rather than picking the one reading that happened to work.
    let e = entry(&[0u8; 3]);
    assert_eq!(e.arm.as_deref(), None);
    assert!(e.thumb.is_some(), "a halfword does decode: {:?}", e.thumb);
    assert_eq!(e.suggests, "unknown");
    assert_eq!(entry(&[]).suggests, "unknown");
}

#[test]
fn the_loader_says_what_a_run_would_have_said() {
    // `Probe::refusal` is `loader::precheck`'s answer, not a copy of it — which is the whole reason the
    // function is public: a probe that re-decided these rules would go green while `load` changed.
    assert!(matches!(
        loader::precheck(&[1, 2], false),
        Some(LoadError::Empty)
    ));
    assert!(matches!(
        loader::precheck(b"\x7fELF\xff\xff\xff\xff", false),
        Some(LoadError::NotAFlatImage { .. })
    ));
    assert!(matches!(
        loader::precheck(&0xf000_0000u32.to_le_bytes(), false),
        Some(LoadError::EntryNotArm { .. })
    ));
    // The same word entered as Thumb is not refused — `--thumb-entry` exists for exactly that.
    assert!(loader::precheck(&0xf000_0000u32.to_le_bytes(), true).is_none());
    assert!(loader::precheck(&arm_image(), false).is_none());
}

#[test]
fn the_report_prints_the_warning_once() {
    // A duplicated `⚠` line went unnoticed for a whole corpus run because nothing asserted over the
    // report, so the report is now a list of lines and this counts it.
    let mod_path = pack(
        "report",
        "1003",
        "1073741827",
        &[],
        Some(mif(&[(
            resource::RESTYPE_IMAGE,
            "image/png",
            png_of(65, 42),
        )])),
    );
    let p = Probe::read(&mod_path);
    let lines = p.lines(None);
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("evidence too thin"))
            .count(),
        1,
        "{lines:#?}"
    );
    assert!(lines.iter().any(|l| l.starts_with("module:")));
    assert!(lines
        .iter()
        .any(|l| l.contains("1 picture(s) in mounted containers; the largest 5")));
    assert!(
        lines.iter().any(|l| l.contains("the host will enter as")),
        "{:?}",
        lines
    );
    cleanup(&mod_path);
}

#[test]
fn the_scroll_strip_is_not_the_screen() {
    // One background and two wider strips at the *same* height, which is the shape that would otherwise
    // be read as a 531-pixel handset.
    let pics = [
        pic("a", 1, 16, 16),
        pic("a", 2, 320, 223),
        pic("a", 3, 510, 223),
        pic("a", 4, 531, 223),
    ];
    let s = infer_screen(&pics).expect("a background is evidence");
    assert_eq!(s.width, 320);
    assert_eq!(s.height, 223);
    assert_eq!(
        s.bar, None,
        "no picture here is a span, so the rule falls back on the height the big pictures agree on"
    );
    assert_eq!(s.at_that_width, 1);
    assert_eq!(
        s.largest,
        Some((531, 223)),
        "the strip is still the biggest thing there"
    );
    assert_eq!(s.considered, 3, "a 16x16 icon is not wide enough to count");
}

#[test]
fn a_bar_spanning_the_canvas_sets_the_width() {
    // The shape most of the corpus is: one full-width strip near the top or bottom, and sprites that
    // are taller than it is wide. The strip is the only picture whose width is the screen's width.
    let pics = [
        pic("a", 1, 176, 38),
        pic("a", 2, 176, 182),
        pic("a", 3, 73, 160),
        pic("a", 4, 26, 26),
    ];
    let s = infer_screen(&pics).expect("a bar is evidence");
    assert_eq!(s.bar, Some(176), "the widest span sets the width");
    assert_eq!(
        (s.width, s.height),
        (176, 182),
        "the tallest picture at that width is the canvas"
    );
    assert_eq!(s.at_that_width, 2);
    assert_eq!(s.considered, 3, "the 26x26 is below the bound on width");
}

#[test]
fn a_portrait_pack_is_read_the_same_way() {
    let pics = [
        pic("a", 1, 240, 320),
        pic("a", 2, 240, 100),
        pic("a", 3, 32, 32),
    ];
    let s = infer_screen(&pics).expect("a background is evidence");
    assert_eq!((s.width, s.height), (240, 320));
    assert_eq!(
        s.considered, 3,
        "32x32 is exactly at the bound, so it counts"
    );
}

#[test]
fn thin_evidence_is_called_thin() {
    // Thirteen of the corpus's thirty-five builds mount nothing but their three icons, and the width
    // answer still prints for them. The report has to say the evidence is an icon, not the art.
    let mod_path = pack(
        "thin",
        "1002",
        "1073741826",
        &[],
        Some(mif(&[
            (resource::RESTYPE_IMAGE, "image/png", png_of(65, 42)),
            (resource::RESTYPE_IMAGE, "image/png", png_of(26, 26)),
            (resource::RESTYPE_IMAGE, "image/png", png_of(16, 16)),
        ])),
    );
    let p = Probe::read(&mod_path);
    assert_eq!(
        p.screen.as_ref().map(|s| (s.width, s.height)),
        Some((65, 42)),
        "the icon is still the widest thing mounted, which is exactly the misleading part"
    );
    assert!(
        p.caveat().is_some(),
        "the 65x42 mounted art must still be called thin"
    );
    let warning = p.caveat().expect("a warning");
    assert!(warning.contains("under 4096"), "{warning}");
    assert!(
        !warning.contains("64x64"),
        "the threshold is an area, not a pair of dimensions: {warning}"
    );
    cleanup(&mod_path);
}

#[test]
fn thin_mounted_art_with_anonymous_art_says_where_the_width_came_from() {
    let sound = par_pack(&[par_pack(&[gzip(&png_of(200, 100))])]);
    let mod_path = pack(
        "thin-anon",
        "1002",
        "1073741826",
        &[("sound.par", sound)],
        Some(mif(&[
            (resource::RESTYPE_IMAGE, "image/png", png_of(65, 42)),
            (resource::RESTYPE_IMAGE, "image/png", png_of(26, 26)),
            (resource::RESTYPE_IMAGE, "image/png", png_of(16, 16)),
        ])),
    );
    let p = Probe::read(&mod_path);
    assert_eq!(p.pictures.len(), 3, "the mounted icons are still there");
    let warning = p.caveat().expect("mounted art is thin");
    assert!(
        warning.contains("1 anonymous .par picture(s) also voted"),
        "the warning must name the other witness: {warning}"
    );
    assert!(
        !warning.contains("describes an icon"),
        "the width is not only an icon when anonymous art participates: {warning}"
    );
    cleanup(&mod_path);
}

#[test]
fn the_widest_bar_wins_and_a_marquee_steals_the_answer() {
    // Pinned, not praised: two spans here, and the wider one is a ticker while the 176 column carries
    // the 205-row canvas. The rule believes the widest span, so this pack is answered 320x36 and would
    // be played on the wrong canvas. Ranking bars by the picture under them fixed exactly this case in
    // the lab and scored worse on the corpus (5 agreements against 7), so it is not what ships — and
    // the packs it disagrees on stay on the report's disagreement list until a rule wins on both.
    let pics = [
        pic("a", 1, 320, 36),
        pic("a", 2, 176, 44),
        pic("a", 3, 176, 205),
    ];
    let s = infer_screen(&pics).expect("both spans are evidence");
    assert_eq!(s.bar, Some(320));
    assert_eq!((s.width, s.height), (320, 36));
}

#[test]
fn nothing_is_invented_when_the_pack_shows_no_art() {
    assert_eq!(infer_screen(&[pic("a", 1, 16, 16)]), None);
    assert_eq!(infer_screen(&[]), None);
}

#[test]
fn a_whole_pack_is_read_from_its_own_directory() {
    let manifest = mif(&[
        (resource::RESTYPE_IMAGE, "image/png", png_of(16, 16)),
        (
            resource::RESTYPE_BINARY,
            "",
            app_info(0x1234_5678, 9, 1, 0x20),
        ),
        (
            resource::RESTYPE_STRING,
            "text/plain",
            vec![2, b'e', b'n', b'^', b'z', b'h', b'~'],
        ),
    ]);
    let mod_path = pack(
        "full",
        "1001",
        "1073741824",
        &[
            (
                "art.bar",
                mif(&[
                    (resource::RESTYPE_IMAGE, "image/png", png_of(240, 320)),
                    (resource::RESTYPE_IMAGE, "image/png", png_of(240, 120)),
                    (
                        resource::RESTYPE_IMAGE,
                        "audio/mid",
                        b"MThd\x00\x00\x00\x06".to_vec(),
                    ),
                ]),
            ),
            ("notes.txt", b"not a container".to_vec()),
        ],
        Some(manifest),
    );
    let p = Probe::read(&mod_path);
    assert_eq!(p.bytes, 64, "the .mod itself");
    assert_eq!(p.word0.suggests, "ARM", "the entry word is an ARM push");
    assert!(
        p.refusal.is_none(),
        "the loader would take it: {:?}",
        p.refusal
    );
    let mut names: Vec<&str> = p.containers.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["1073741824.mif", "art.bar"],
        "both containers, whichever order the directory came back in"
    );
    assert_eq!(
        p.manifest.as_deref(),
        Some("1073741824.mif"),
        "the manifest is named for the directory, not the module"
    );
    // Which container lands in which slot is the directory's doing, not the report's, so the cases
    // below look the files up by name rather than by index.
    let census = |want: &str| {
        p.containers
            .iter()
            .find(|(n, _)| n == want)
            .map(|(_, c)| c)
            .expect("the container is in the report")
    };
    let art = census("art.bar");
    assert_eq!(art.records, 3);
    assert_eq!(
        art.pictures.len(),
        2,
        "the MIDI is filed under the image kind and is not one"
    );
    assert_eq!(art.undecodable, 1);
    assert_eq!(art.kinds.get("IMAGE"), Some(&3));
    assert_eq!(
        p.screen.as_ref().map(|s| (s.width, s.height)),
        Some((240, 320)),
        "the art answers, and the 16x16 icon in the manifest does not outvote it"
    );
    assert!(
        p.caveat().is_none(),
        "a 240x320 background is not thin evidence"
    );
    let app = p.app_info.expect("the manifest holds the app's own record");
    assert_eq!(
        (app.cls, app.id_base, app.app_type, app.flags),
        (0x1234_5678, 9, 1, 0x20)
    );
    assert_eq!(
        p.strings
            .iter()
            .map(|(f, id, n, first)| (f.as_str(), *id, *n, first.as_str()))
            .collect::<Vec<_>>(),
        vec![("1073741824.mif", 3, 2, "en")],
        "two languages, and the first one printed as far as it is sayable"
    );
    assert_eq!(
        p.audio,
        vec![AudioCensus {
            mime: "audio/mid".to_string(),
            declared: 1,
            declared_bytes: 8,
            ..Default::default()
        }],
        "the MIME decides the line, and 8 bytes of header is a declaration this host cannot play: \
         the `playable` half stays empty because the parse, not the name, owns that count"
    );
    assert_eq!(
        p.unmounted
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>(),
        vec!["notes.txt"],
        "the module it is reporting on is not listed among its own findings"
    );
    assert_eq!(
        p.hint, None,
        "nobody typed a size into these directory names"
    );
    assert_eq!(p.agrees(), None, "one witness is not a disagreement");
    assert_eq!(p.verdict(), "the art answered; the directory named no size");
    cleanup(&mod_path);
}

#[test]
fn a_directory_hint_that_disagrees_with_the_art_is_called_a_disagreement() {
    // The folder says 176x220 and the art says 240x320. Both numbers get printed rather than one of
    // them being chosen silently.
    let mod_path = pack(
        "conflict",
        "mygame-176x220",
        "1073741825",
        &[],
        Some(mif(&[(
            resource::RESTYPE_IMAGE,
            "image/png",
            png_of(240, 320),
        )])),
    );
    let p = Probe::read(&mod_path);
    assert_eq!(p.hint, Some((176, 220)));
    assert_eq!(p.agrees(), Some(false));
    assert!(p.verdict().contains("disagrees"), "{}", p.verdict());
    cleanup(&mod_path);
}

#[test]
fn a_par_leaf_with_no_property_block_is_still_measured() {
    // `.par` stores no type, so a probe that only trusted declared MIMEs would report the whole pack
    // as having no art. This is the shape the corpus's `ui.par` has: 85 BMPs and no blocks at all.
    let leaf = Record {
        kind: resource::RESTYPE_IMAGE,
        id: 4,
        mime: None,
        body: bmp_of(64, 64),
    };
    let p = picture("ui.par", &leaf).expect("the payload names itself");
    assert_eq!((p.width, p.height), (64, 64));
    assert_eq!(p.from, "ui.par");
}

#[test]
fn the_strings_of_a_manifest_are_printed_as_far_as_they_are_sayable() {
    // A counted resource is one byte per character and a UTF-16 manifest holds real ones; either way a
    // byte that is not printable stays an escape rather than becoming a box or a wrong character.
    assert_eq!(text_of(&[0x76, 0x31, 0x2e, 0x30]), "v1.0");
    assert_eq!(text_of(&[0x76, 0x94, 0x88]), "v\\u0094\\u0088");
}

#[test]
fn the_report_marks_its_warning_exactly_once() {
    // The library's pane colours whatever `is_caution` accepts, and the report printed this line twice
    // for a whole corpus run because nothing asserted over the report. Counted, not assumed.
    let thin = pack(
        "caution",
        "1004",
        "1073741828",
        &[],
        Some(mif(&[(
            resource::RESTYPE_IMAGE,
            "image/png",
            png_of(65, 42),
        )])),
    );
    let lines = Probe::read(&thin).lines(None);
    assert_eq!(
        lines.iter().filter(|l| is_caution(l)).count(),
        1,
        "one warning line: {lines:#?}"
    );
    cleanup(&thin);

    // A pack whose art really is mounted carries no warning at all — the same predicate has to say so.
    let thick = pack(
        "thick",
        "1005",
        "1073741829",
        &[],
        Some(mif(&[(
            resource::RESTYPE_IMAGE,
            "image/png",
            png_of(240, 320),
        )])),
    );
    let lines = Probe::read(&thick).lines(None);
    assert_eq!(lines.iter().filter(|l| is_caution(l)).count(), 0);
    cleanup(&thick);
}

#[test]
fn the_brew_header_reports_where_its_own_words_lands() {
    let mut image = arm_image();
    image.resize(0x100, 0);
    image[0x08..0x0c].copy_from_slice(b"BREW");
    for (off, value) in [(0x18, 0x200u32), (0x1c, 0x40), (0x20, 0x10), (0x28, 0x40)] {
        image[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }
    let b = brew_header(&image).expect("the magic is there");
    assert_eq!(
        (b.image, b.ref_off, b.fixup_off, b.ref_size),
        (0x200, 0x40, 0x10, 0x40)
    );
    // The loader's own bound ignores `IMAGE`'s shift: it asks whether the reference area's offsets add up
    // inside the file (0x40 + 0x10 + 0x40 = 0x90 ≤ 0x100), so this header **does** describe the file —
    // while the fixup word sits at 0x290, past the end, and cannot be read. Two different facts, and the
    // report prints both rather than collapsing them into one.
    assert!(b.describes);
    assert_eq!(b.fixup_word, None);
    image.resize(0x294, 0);
    image[0x290..0x294].copy_from_slice(&0xe92d_00f0u32.to_le_bytes());
    let b = brew_header(&image).expect("the magic is still there");
    assert_eq!((b.describes, b.fixup_word), (true, Some(0xe92d_00f0)));
    // A zero `REFSIZE` is the loader's "this header describes nothing".
    image[0x28..0x2c].copy_from_slice(&0u32.to_le_bytes());
    assert!(!brew_header(&image).expect("still here").describes);
    // And a flat image has no header at all.
    assert_eq!(brew_header(&arm_image()), None);
}

/// The audio census counts a clip **once**, under the name the parse gave it, and says which of the
/// two ways it was known.
///
/// The line used to be one hand-written `starts_with("audio/")` over declared MIMEs, which got both
/// facts wrong in opposite directions: a pack whose ring tones are registered under the SDK's older
/// `SOUND_MIME_BASE` (and spell the whole set that way) reported no
/// audio at all, while a pack whose payloads are declared `audio/mid` but are 8 bytes of header was
/// reported as music this host could play. Neither mistake is visible in a number that does not say
/// how it was arrived at, which is why the line carries both counts.
#[test]
fn the_audio_census_counts_a_clip_once_and_says_which_way_it_was_known() {
    let score = crate::audio::one_note_midi();
    let bytes = score.len();
    let mod_path = pack(
        "audio-census",
        "1004",
        "1073741828",
        &[(
            "sound.bar",
            mif(&[
                // Real MIDI, honestly declared: the playable count.
                (resource::RESTYPE_IMAGE, "audio/mid", score.clone()),
                // Real MIDI with no property block at all — every `.par` leaf looks like this, and the
                // old declaration-only census could not see it.
                (resource::RESTYPE_IMAGE, "", score.clone()),
                // Real MIDI under a lie. It still counts once, as what it is.
                (resource::RESTYPE_IMAGE, "audio/mp3", score.clone()),
                // A sound the container declares and this host cannot decode: declared, not playable.
                (resource::RESTYPE_IMAGE, "snd/rmf", b"not a score".to_vec()),
                // The same name with a space on the end. One line, not two: the census files it under
                // the trimmed name, the way `IShell::GetHandler` folds it.
                (resource::RESTYPE_IMAGE, "audio/mid ", b"silence".to_vec()),
                // The family this host has no decoder for, and two records that show why a count of
                // them has to be asked twice. Both are filed by declaration, but only one of them is
                // MPEG audio: the third field is what separates "the pack calls these MP3s" from
                // "these are MP3s".
                (
                    resource::RESTYPE_IMAGE,
                    "audio/mpeg",
                    vec![0xFF, 0xFB, 0x90, 0x00, 0x00, 0x01, 0x02, 0x03],
                ),
                (
                    resource::RESTYPE_IMAGE,
                    "audio/mpeg",
                    b"an empty plane".to_vec(),
                ),
            ]),
        )],
        None,
    );
    let p = Probe::read(&mod_path);
    // `BTreeMap` order, so the names sort: `audio/mid`, `audio/mpeg`, then `snd/rmf`.
    assert_eq!(
        p.audio
            .iter()
            .map(|a| (a.mime.as_str(), a.playable, a.declared))
            .collect::<Vec<_>>(),
        vec![
            ("audio/mid", 3, 1),
            ("audio/mp3", 1, 0),
            ("audio/mpeg", 0, 1),
            ("snd/rmf", 0, 1)
        ],
        "four clips a parse took, three known only by declaration — and nothing counted twice"
    );
    assert_eq!(
        p.audio[0].declared_bytes,
        b"silence".len(),
        "the padded declaration lands on the same line, not beside it"
    );
    assert_eq!(
        p.audio[0].playable_bytes,
        bytes * 3,
        "the playable tally carries the bytes it counted"
    );
    // The MPEG-family records now split: the one with a real frame is playable MP3, the empty one
    // remains a declaration the host cannot use. Neither is counted twice.
    assert_eq!(
        p.audio[1].playable, 1,
        "the real MPEG frame is now a playable MP3, not merely a declaration"
    );
    assert_eq!(
        p.audio[1].declared, 0,
        "and it is not also counted under its declaring name"
    );
    assert_eq!(
        p.audio[2].declared, 1,
        "the empty `audio/mpeg` record stays declared-only"
    );
    assert_eq!(
        p.audio[3].declared_bytes,
        b"not a score".len(),
        "the `snd/rmf` record has no decoder behind it"
    );
    let report = p.lines(None);
    let lines = report.join("\n");
    assert!(
        lines.contains("audio audio/mid: 3 playable"),
        "the report says which way: {lines}"
    );
    assert!(
        lines.contains("audio audio/mp3: 1 playable"),
        "the recognised MPEG frame is now a playable row: {lines}"
    );
    assert!(
        lines.contains("audio snd/rmf: 1 by declaration"),
        "and says it for the undecodable kind too: {lines}"
    );
    assert!(
        lines.contains("audio audio/mpeg: 1 by declaration"),
        "the empty MPEG declaration is still an inventory row: {lines}"
    );
    cleanup(&mod_path);
}

/// A gzip member as a `.par` leaf stores one.
fn gzip(payload: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::new();
    let mut enc = flate2::write::GzEncoder::new(&mut out, flate2::Compression::fast());
    enc.write_all(payload).unwrap();
    enc.finish().unwrap();
    out
}

/// A `PAR` header holding `children`, with the offsets the format requires.
fn par_pack(children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"PAR\0".to_vec();
    out.extend_from_slice(&(children.len() as u32).to_le_bytes());
    let mut at = 8 + 4 * children.len();
    for child in children {
        out.extend_from_slice(&(at as u32).to_le_bytes());
        at += child.len();
    }
    for child in children {
        out.extend_from_slice(child);
    }
    out
}

/// A `.par` the host recognises but cannot mount must not read as "not a container".
///
/// A shape the corpus contains: a bare nested header whose gzip leaves have no id above them. `read` is right to return nothing — an invented id is a name no pack asks for — but the
/// report has to name the file and the leaves it could not file, or a silent mount looks like a
/// missing resource.
#[test]
fn a_par_that_names_no_leaves_is_reported_instead_of_silently_absent() {
    let midi = crate::audio::one_note_midi();
    // Two unnamed pictures of different areas (a BMP and a PNG), plus an unnamed MIDI: this makes
    // the "largest" comparison and the PNG branch observable instead of relying on one picture.
    let sound = par_pack(&[par_pack(&[
        gzip(&bmp_of(16, 16)),
        gzip(&png_of(32, 32)),
        gzip(&midi),
    ])]);
    let mod_path = pack(
        "par-no-ids",
        "mytitle",
        "1073741826",
        &[("sound.par", sound)],
        None,
    );
    let p = Probe::read(&mod_path);
    assert_eq!(p.par_census.len(), 1, "{:?}", p.par_census);
    assert_eq!(p.par_census[0].0, "sound.par");
    assert_eq!(p.par_census[0].1.census.emitted, 0);
    assert_eq!(p.par_census[0].1.census.unnamed, 3);
    assert_eq!(p.par_census[0].1.census.unreadable, 0);
    assert_eq!(
        p.par_census[0].1.unnamed_audio, 1,
        "the leaf's bytes are a MIDI file even though no id names it"
    );
    assert_eq!(
        p.par_census[0].1.unnamed_pictures, 2,
        "both the BMP and the PNG decode"
    );
    assert_eq!(
        p.par_census[0].1.largest_picture,
        Some((32, 32)),
        "32x32 wins on area, so a reversed comparison would fail"
    );
    assert_eq!(
        p.par_census[0].1.picture_sizes,
        vec![(16, 16), (32, 32)],
        "the screen inference needs every size, not just the largest"
    );
    let report = &p.par_census[0].1;
    assert_eq!(
        report.unnamed_pictures,
        report.picture_sizes.len(),
        "the count and the list describe the same set"
    );
    assert_eq!(
        report.largest_picture,
        report
            .picture_sizes
            .iter()
            .copied()
            .max_by_key(|(w, h)| w * h),
        "the largest is derived from the same sizes"
    );
    assert!(p.pictures.is_empty(), "no mounted art in this fixture");
    assert_eq!(
        p.screen.as_ref().map(|s| (s.width, s.height)),
        Some((32, 32)),
        "the anonymous pictures are screen evidence now"
    );
    let screen = p
        .screen
        .as_ref()
        .expect("a wide anonymous picture answered");
    assert_eq!(screen.considered, 1);
    assert_eq!(screen.at_that_width, 1);
    let lines = p.lines(None);
    assert!(
        lines.iter().any(|l| l.contains("PAR sound.par")
            && l.contains("will not mount")
            && l.contains("among them 1 playable MIDI by bytes")
            && l.contains("2 decodable picture(s)")
            && l.contains("largest 32x32")),
        "{lines:?}"
    );
    assert!(
        is_module_file(
            std::path::Path::new("D:/packs/1073743380/tank.mod"),
            std::path::Path::new("tank.mod")
        ),
        "a bare module name and the absolute sibling path name the same file"
    );
    assert!(
        !is_module_file(
            std::path::Path::new("D:/packs/1073743380/tank.mif"),
            std::path::Path::new("tank.mod")
        ),
        "a sibling with another name is not the module"
    );
    assert!(
        lines.iter().any(|l| l
            .contains("anonymous .par picture(s): 2, of which 1 are wide enough to vote")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("screen inference")
            && l.contains("1 wide picture(s) voted (of which 1 anonymous .par)")),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("#0")),
        "the anonymous sentinel id must never be displayed: {lines:?}"
    );
    let warning = p
        .caveat()
        .expect("no mounted picture, but anonymous art answered");
    assert!(warning.contains("mounted no pictures"), "{warning}");
    assert!(warning.contains("anonymous .par"), "{warning}");
    cleanup(&mod_path);
}

#[test]
fn a_corrupt_anonymous_leaf_is_reported_as_unreadable() {
    let bad = b"\x1f\x8b\x08bad".to_vec();
    let sound = par_pack(&[par_pack(&[bad])]);
    let mod_path = pack(
        "par-corrupt",
        "mytitle",
        "1073741830",
        &[("sound.par", sound)],
        None,
    );
    let p = Probe::read(&mod_path);
    let report = &p.par_census[0].1;
    assert_eq!(report.census.unnamed, 0, "the bad stream is not unnamed");
    assert_eq!(
        report.census.unreadable, 1,
        "the inflating walk decides it cannot be read"
    );
    assert!(
        p.lines(None)
            .iter()
            .any(|l| l.contains("PAR sound.par") && l.contains("1 unreadable")),
        "{:?}",
        p.lines(None)
    );
    cleanup(&mod_path);
}

/// Anonymous art that exists but is too small to answer must say so, not claim it participated.
#[test]
fn a_par_with_only_small_anonymous_art_says_none_are_wide_enough() {
    let sound = par_pack(&[par_pack(&[gzip(&bmp_of(16, 16))])]);
    let mod_path = pack(
        "par-small",
        "mytitle",
        "1073741829",
        &[("sound.par", sound)],
        None,
    );
    let p = Probe::read(&mod_path);
    assert!(p.screen.is_none(), "16x16 is below MIN_ART");
    assert_eq!(p.par_census[0].1.unnamed_pictures, 1);
    let lines = p.lines(None);
    assert!(
        lines.iter().any(|l| l
            .contains("anonymous .par picture(s): 1, of which 0 are wide enough to vote")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("cannot be asked")
            && l.contains("1 anonymous .par picture(s), of which 0 are wide enough to vote")),
        "{lines:?}"
    );
    cleanup(&mod_path);
}

/// One payload with the id it answers for, the shape `ui.par` uses.
fn indexed(id: u32, payload: Vec<u8>) -> Vec<u8> {
    let mut out = 1u16.to_le_bytes().to_vec();
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// A `.par` the mount can use completely must not appear in the new report.
///
/// This is the other side of the rule above: the report exists for the leaves a mount cannot name,
/// not as a second line for every `.par` a run already mounted.
#[test]
fn a_par_that_mounts_completely_is_not_reported() {
    let sound = par_pack(&[indexed(7, gzip(b"BM\x00\x00"))]);
    let mod_path = pack(
        "par-mounted",
        "mytitle",
        "1073741827",
        &[("sound.par", sound)],
        None,
    );
    let p = Probe::read(&mod_path);
    assert!(p.par_census.is_empty(), "{:?}", p.par_census);
    cleanup(&mod_path);
}

/// The audio count is for leaves a mount cannot reach, not for everything in the file.
///
/// One named MIDI and one unnamed MIDI: the mount already serves the first, so only the second is
/// new information the report should add.
#[test]
fn only_the_unnamed_midi_leaves_are_counted_in_the_par_report() {
    let midi = crate::audio::one_note_midi();
    let sound = par_pack(&[indexed(7, gzip(&midi)), gzip(&midi)]);
    let mod_path = pack(
        "par-mixed",
        "mytitle",
        "1073741828",
        &[("sound.par", sound)],
        None,
    );
    let p = Probe::read(&mod_path);
    assert_eq!(p.par_census.len(), 1, "{:?}", p.par_census);
    assert_eq!(p.par_census[0].1.census.emitted, 1);
    assert_eq!(p.par_census[0].1.census.unnamed, 1);
    assert_eq!(
        p.par_census[0].1.unnamed_audio, 1,
        "the named MIDI is already mounted; only the unnamed one is added"
    );
    assert_eq!(p.par_census[0].1.unnamed_pictures, 0);
    cleanup(&mod_path);
}
