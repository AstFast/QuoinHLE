// Moved out of `src/library.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn a_build_directory_says_which_screen_the_pack_was_built_for() {
    assert_eq!(screen_of("motQA30-320x240"), Some((320, 240)));
    assert_eq!(screen_of("VX8300-176x220"), Some((176, 220)));
    assert_eq!(screen_of("с781(from bundle)-240x320"), Some((240, 320)));
    assert_eq!(screen_of("vx9600(from bundle)-480x240"), Some((480, 240)));
    // An application id carries no size, which is not a failure: the list says nothing about the
    // screen and the pack is loaded with the default one.
    assert_eq!(screen_of("283491"), None);
    assert_eq!(screen_of("alpha-240x320"), Some((240, 320)));
    // A directory that merely has an `x` in it is not a screen size.
    assert_eq!(screen_of("0x0"), None);
    assert_eq!(screen_of("axb"), None);
    // Four of the ten guest builds are filed under a bracketed, space-separated name that
    // carries the size as `[240×320]` — a full-width multiplication sign, because whoever typed it
    // was typing Chinese. The old `-`/`_` anchor plus an ASCII-only separator found nothing in
    // any of them, so the list loaded all four at the default screen without saying so.
    assert_eq!(
        screen_of("[晓风君] [酷派S60] [240×320] [V1.0.142017]"),
        Some((240, 320)),
        "bravo, 240x320"
    );
    assert_eq!(
        screen_of("[圣莫尼卡] [中兴D90+] [176×220] [V1.0.40063]"),
        Some((176, 220)),
        "bravo, the other build; `90+` must not be read as a size"
    );
    assert_eq!(
        screen_of("[charlie] [华为C7100] [240×320] [arm] [v1.0.90086]"),
        Some((240, 320)),
        "charlie, which also has an ASCII `C7100` in it"
    );
    // A version number is not a screen just because a digit sits next to one, and a run too long
    // for a `u16` must not end the scan — the real size may be further along the name.
    assert_eq!(screen_of("[酷派S60] [V1.0.142017]"), None);
    assert_eq!(screen_of("[7100x240]"), Some((7100, 240)));
    assert_eq!(screen_of("[V1.0.142017] [240×320]"), Some((240, 320)));
}

#[test]
fn the_scan_finds_every_mod_and_names_it_after_where_it_sits() {
    let root = library_dir("scan");
    let _ = std::fs::remove_dir_all(&root);
    let one = root.join("alpha/motQA30-320x240/39637");
    let two = root.join("charlie/283491");
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();
    std::fs::write(one.join("alpha.mod"), b"not really a module").unwrap();
    std::fs::write(two.join("charlie.mod"), b"not really a module").unwrap();
    // Something that is not a module at all: a library that listed it would offer to start a file
    // that is not a program.
    std::fs::write(two.join("notes.txt"), b"hello").unwrap();

    let found = scan(&root);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(found[0].title, "alpha");
    assert_eq!(found[0].build, "motQA30-320x240");
    assert_eq!(found[0].screen, Some((320, 240)));
    assert!(found[0].icon.is_none(), "no mif beside it, so no icon");
    assert_eq!(found[1].title, "charlie");
    assert_eq!(found[1].screen, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_root_is_an_empty_library_rather_than_a_refusal() {
    assert!(scan(Path::new("no-such-directory-here")).is_empty());
}

#[test]
fn a_png_only_container_gives_the_pack_its_own_picture() {
    // Ten of the sample set's thirty-five builds carry three `image/png` images and no BMP, and the list
    // showed a blank space for each of them. The host already had a PNG reader; the library was asking
    // only for BMPs.
    let root = library_dir("png-icon");
    std::fs::create_dir_all(root.join("7")).unwrap();
    std::fs::write(root.join("7/icon7.mod"), b"not really a module").unwrap();
    std::fs::write(
        root.join("7.mif"),
        mif_of(&[
            ("image/png", png_of(32, 32, [255, 0, 0])),
            ("image/png", png_of(8, 8, [0, 0, 255])),
        ]),
    )
    .unwrap();

    let found = scan(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    let icon = found[0].icon.as_ref().expect("a PNG is an icon");
    assert_eq!((icon.width, icon.height), (8, 8), "the smallest picture");
    assert_eq!(icon.rgb, vec![[0u8, 0, 255]; 64].concat());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_smallest_decodable_picture_wins_whichever_order_it_is_stored_in() {
    // "The smallest" is the claim, not "the first" and not "the BMP": a container that stores a
    // full-screen picture ahead of its icon has to give up the icon anyway, in either order.
    // A colour per size, so the assertion says *which* picture was picked rather than which slot
    // it sat in — the two orders have to give the same answer. The 62 is not arbitrary: 62 * 3 is not
    // a multiple of four, so the BMP's rows carry padding, which a width of 64 would never exercise.
    for (first, second) in [
        (("image/bmp", 62), ("image/png", 12)),
        (("image/png", 12), ("image/bmp", 62)),
    ] {
        let colour = |side: u32| if side == 12 { [4, 5, 6] } else { [1, 2, 3] };
        let root = library_dir("smallest-wins");
        std::fs::create_dir_all(root.join("7")).unwrap();
        std::fs::write(root.join("7/icon7.mod"), b"not really a module").unwrap();
        std::fs::write(
            root.join("7.mif"),
            mif_of(&[
                (first.0, picture(first.0, first.1, colour(first.1))),
                (second.0, picture(second.0, second.1, colour(second.1))),
            ]),
        )
        .unwrap();
        let found = scan(&root);
        let icon = found[0]
            .icon
            .as_ref()
            .unwrap_or_else(|| panic!("{first:?} then {second:?} produced no icon"));
        assert_eq!((icon.width, icon.height), (12, 12), "{first:?} {second:?}");
        assert_eq!(
            icon.rgb,
            vec![[4u8, 5, 6]; 144].concat(),
            "the small picture's own colours, whichever decoder read it"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
    // And that the padded BMP above is being *decoded* rather than silently skipped: alone in a
    // container it has to come back at its own size, in its own colours.
    let root = library_dir("padded-bmp");
    std::fs::create_dir_all(root.join("7")).unwrap();
    std::fs::write(root.join("7/icon7.mod"), b"not really a module").unwrap();
    std::fs::write(
        root.join("7.mif"),
        mif_of(&[("image/bmp", bmp_of(62, 62, [1, 2, 3]))]),
    )
    .unwrap();
    let found = scan(&root);
    let icon = found[0].icon.as_ref().expect("a 62x62 BMP");
    assert_eq!((icon.width, icon.height), (62, 62));
    assert_eq!(icon.rgb, vec![[1u8, 2, 3]; 62 * 62].concat());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_row_lists_every_evidence_and_says_what_it_cannot_back() {
    // `<root>/mygame-240x320/1001/mygame.mod` with its manifest one level up, the way the sample set files
    // a pack. Two records now claim 240x320 for it, and the row has to show both, folder first: P1's
    // picker offers them as equals and boots with whichever one a player picks.
    let root = library_dir("rows");
    let build = root.join("mygame-240x320");
    let dir = build.join("1001");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mygame.mod"), arm_image()).unwrap();
    std::fs::write(
        build.join("1001.mif"),
        mif_of(&[("image/png", png_of(240, 320, [1, 2, 3]))]),
    )
    .unwrap();

    let found = rows(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    let r = &found[0];
    assert_eq!(r.subtitle, "mygame-240x320  240x320");
    assert!(
        r.caution.is_none(),
        "a full-screen background is evidence: {:?}",
        r.caution
    );
    assert_eq!(
        r.sizes,
        vec![
            Candidate {
                width: 240,
                height: 320,
                claim: Claim::Folder,
                evidence: "from the directory: mygame-240x320".to_string()
            },
            Candidate {
                width: 240,
                height: 320,
                claim: Claim::Art,
                // The same sentence the report line carries, from `Screen::evidence` — no bar in this
                // fixture, so the fallback rule is named as the width's source rather than a bar.
                evidence: "width from the narrowest width at the shared height of the large pictures; 1 picture(s) at that width, the tallest gives 320"
                    .to_string()
            },
        ]
    );
    assert!(
        r.detail
            .iter()
            .any(|l| l.contains("canvas candidate 240x320")),
        "the detail pane is the probe's own report, numbers and all: {:?}",
        r.detail
    );
    // One wording for one fact: the picker's evidence line is a substring of the report's own line, so
    // the two cannot drift into telling different stories about the same number.
    let art = &r.sizes[1];
    assert!(
        r.detail
            .iter()
            .any(|l| l.contains(&art.evidence) && l.contains("canvas candidate 240x320")),
        "the candidate quotes the report, not a second sentence of its own: {:?}",
        r.detail
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_icon_is_not_offered_as_a_screen() {
    // The case for 17 of the sample set's sample builds: all that mounts is the icon set, so the widest picture
    // is 65x42. Offering that as a screen candidate would be a number looking like evidence, which is
    // the shape this repository keeps being bitten by; the row reports it through `caution` instead.
    let root = library_dir("rows-thin");
    let build = root.join("mygame-240x320");
    let dir = build.join("1002");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mygame.mod"), arm_image()).unwrap();
    std::fs::write(
        build.join("1002.mif"),
        mif_of(&[("image/png", png_of(65, 42, [1, 2, 3]))]),
    )
    .unwrap();

    let found = rows(&root);
    let r = &found[0];
    assert_eq!(
        r.sizes,
        vec![Candidate {
            width: 240,
            height: 320,
            claim: Claim::Folder,
            evidence: "from the directory: mygame-240x320".to_string()
        }],
        "the folder still speaks, the art does not get a vote"
    );
    assert!(r.caution.is_some(), "and the row says why");
    assert!(
        r.detail.iter().any(|l| l.contains("evidence too thin")),
        "the report carries it too: {:?}",
        r.detail
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pack_with_no_evidence_at_all_offers_nothing() {
    // No size in the directory name and nothing decodable beside it: `default screen` in the subtitle and an
    // empty candidate list, rather than a row that implies the host knows.
    let root = library_dir("rows-blind");
    let dir = root.join("283491");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("charlie.mod"), arm_image()).unwrap();

    let found = rows(&root);
    let r = &found[0];
    assert_eq!(r.subtitle, "283491  default screen");
    assert!(r.sizes.is_empty(), "{:?}", r.sizes);
    assert!(
        r.caution.is_none(),
        "nothing to be thin about: {:?}",
        r.caution
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The first word of `AEEMod_Load` as an ARM `push`, so the loader would not refuse the image.
fn arm_image() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0xe92d_00f0u32.to_le_bytes());
    out.resize(64, 0);
    out
}

/// A row with two records that **disagree**, built by hand so no pack is read. The art claims a wider
/// canvas than the directory does: a test whose two claims are the same number cannot tell which one a
/// boot used.
fn two_claim_row() -> Row {
    Row {
        entry: Entry {
            title: "g".to_string(),
            build: "g-240x320".to_string(),
            mod_path: std::path::PathBuf::from("g.mod"),
            screen: Some((240, 320)),
            icon: None,
        },
        subtitle: "g-240x320  240x320".to_string(),
        detail: Vec::new(),
        sizes: vec![
            Candidate {
                width: 240,
                height: 320,
                claim: Claim::Folder,
                evidence: "from the directory: g-240x320".to_string(),
            },
            Candidate {
                width: 320,
                height: 240,
                claim: Claim::Art,
                evidence: "width from a 320-pixel-wide bar; 2 picture(s) at that width, the tallest gives 240".to_string(),
            },
        ],
        caution: None,
    }
}

#[test]
fn the_launch_decision_names_the_size_and_who_said_it() {
    // `decide` is the one place that turns a pick into the size a boot takes, and every branch has to
    // say its source out loud. A wrong canvas is silent — one pack at 240x320 and at 320x240 loads in
    // the same 17 629 instructions, presents 464 times and peaks at the same heap high, with only the
    // frame bytes different — so the label under a person's cursor is the only moment the number is
    // stated before something starts.
    let row = two_claim_row();

    let d = decide(&row, None);
    assert_eq!(
        d.screen,
        Some((240, 320)),
        "nothing picked falls back to the folder"
    );
    assert_eq!(
        d.words, "240x320 from the directory (not chosen)",
        "and says it was not chosen"
    );

    let d = decide(&row, Some(row.sizes[1].pick()));
    assert_eq!(
        d.screen,
        Some((320, 240)),
        "the art claim is booted as itself"
    );
    assert!(
        d.words.contains("art") && d.words.contains("320x240"),
        "{}",
        d.words
    );

    let d = decide(
        &row,
        Some(Pick::Manual {
            width: 240,
            height: 400,
        }),
    );
    assert_eq!(d.screen, Some((240, 400)));
    assert_eq!(d.words, "240x400 typed by hand");

    // The third state: nobody claimed anything, so the host's own default is used — and it is named as
    // the host's guess rather than as a size the pack has.
    let mut blind = two_claim_row();
    blind.entry.screen = None;
    blind.sizes.truncate(1);
    let d = decide(&blind, None);
    assert_eq!(
        d.screen, None,
        "`None` is what [`boot_at`] takes for the default profile"
    );
    let standard = DeviceProfile::standard();
    assert!(
        d.words.contains("host default")
            && d.words
                .contains(&format!("{}x{}", standard.width, standard.height)),
        "{}",
        d.words
    );
}

#[test]
fn a_half_typed_size_is_not_a_screen() {
    assert_eq!(manual_size("240", "400"), Some((240, 400)));
    // The boxes carry whatever the keyboard typed, including the trailing newline a paste leaves.
    assert_eq!(manual_size(" 240 ", "400\n"), Some((240, 400)));
    assert_eq!(
        manual_size("0", "320"),
        None,
        "a zero-high screen is not a handset"
    );
    assert_eq!(manual_size("320", ""), None);
    assert_eq!(manual_size("abc", "320"), None);
    assert_eq!(manual_size("70000", "320"), None, "outside a `u16`");
    // The harness's own bound, `40..=1024` on each axis (`tools/games.ps1:156`): a frame is allocated in
    // the host, so a typed `65535x65535` is an eight-gigabyte canvas, and this machine's failure mode is
    // that it stops rather than that it errors.
    assert_eq!(manual_size("39", "320"), None, "below the smallest handset");
    assert_eq!(
        manual_size("40", "40"),
        Some((40, 40)),
        "the bound is inclusive"
    );
    assert_eq!(manual_size("1024", "1024"), Some((1024, 1024)));
    assert_eq!(manual_size("1025", "320"), None);
    assert_eq!(manual_size("320", "65535"), None);
}

#[test]
fn a_sound_or_a_caption_under_the_image_kind_is_still_not_an_icon() {
    // The sample set files MIDI and text under `RESTYPE_IMAGE`, so the kind is not the test of what a
    // picture is. A container holding only those answers "no icon"; one holding them beside a real
    // picture answers with the picture; and a declared format this host has no reader for — the
    // private `image/zbm` class — is refused rather than guessed at.
    for (parts, expect) in [
        (vec![("audio/mid", b"MThd\x00\x00\x00\x06".to_vec())], None),
        (
            vec![
                ("text/plain", b"Loading.".to_vec()),
                ("image/png", png_of(4, 4, [7, 8, 9])),
            ],
            Some((4, 4)),
        ),
        (vec![("image/zbm", vec![0u8; 32])], None),
    ] {
        let root = library_dir("not-a-picture");
        std::fs::create_dir_all(root.join("7")).unwrap();
        std::fs::write(root.join("7/icon7.mod"), b"not really a module").unwrap();
        std::fs::write(root.join("7.mif"), mif_of(&parts)).unwrap();
        let found = scan(&root);
        assert_eq!(
            found[0].icon.as_ref().map(|i| (i.width, i.height)),
            expect,
            "{:?}",
            parts.iter().map(|(m, _)| *m).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn a_resource_that_declares_nothing_is_decided_by_its_own_bytes() {
    // The last arm of `resource::image`: with no property block the payload names itself. The library
    // inherits that answer, which is a **widening** over the old BMP-prefix test — those resources used
    // to be refused outright. Pinned here rather than left implied by a comment.
    for (parts, expect) in [
        (vec![("", png_of(6, 6, [9, 8, 7]))], Some((6, 6))),
        (vec![("", b"MThd\x00\x00\x00\x06".to_vec())], None),
    ] {
        let root = library_dir("undeclared");
        std::fs::create_dir_all(root.join("7")).unwrap();
        std::fs::write(root.join("7/icon7.mod"), b"not really a module").unwrap();
        std::fs::write(root.join("7.mif"), mif_of(&parts)).unwrap();
        let found = scan(&root);
        assert_eq!(
            found[0].icon.as_ref().map(|i| (i.width, i.height)),
            expect,
            "payload {}",
            String::from_utf8_lossy(&parts[0].1[..4.min(parts[0].1.len())])
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// A throwaway library root, cleaned first so a previous run cannot be mistaken for this one.
fn library_dir(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "QuoinHLE-library-{tag}-{}",
        // Two `cargo test` runs in parallel would otherwise delete each other's fixtures.
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A picture of the declared format, at `side` x `side` and one flat colour.
fn picture(mime: &str, side: u32, rgb: [u8; 3]) -> Vec<u8> {
    match mime {
        "image/png" => png_of(side, side, rgb),
        "image/bmp" => bmp_of(side, side, rgb),
        other => panic!("{other} is not a format these cases write"),
    }
}

/// A PNG written the way a pack stores one, so the reader is exercised against a real stream.
fn png_of(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let px: Vec<u8> = (0..w * h).flat_map(|_| rgb).collect();
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

/// A 24-bit bottom-up BMP, which is the shape every `image/bmp` resource in the sample set is.
fn bmp_of(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let stride = (w as usize * 3 + 3) & !3;
    let mut rows = Vec::new();
    for _ in 0..h {
        for _ in 0..w {
            rows.extend_from_slice(&[rgb[2], rgb[1], rgb[0]]);
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
        // `planes` = 1 at 0x1a and `bitcount` = 24 at 0x1c, which one little-endian word covers.
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

/// A `.mif` holding the given `RESTYPE_IMAGE` resources, each written the way a container writes
/// one: a u16 property-block length that counts itself, the MIME, a NUL, then the payload.
fn mif_of(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    assert!(
        !parts.is_empty(),
        "an empty container is not what these cases write"
    );
    let bodies: Vec<Vec<u8>> = parts
        .iter()
        .map(|(mime, payload)| {
            let block = format!("{mime}\0");
            let mut b = Vec::new();
            b.extend_from_slice(&((2 + block.len()) as u16).to_le_bytes());
            b.extend_from_slice(block.as_bytes());
            b.extend_from_slice(payload);
            b
        })
        .collect();
    let (map, offsets) = (0x20usize, 0x28usize);
    let data = offsets + (bodies.len() + 1) * 4;
    let mut out = Vec::new();
    out.extend_from_slice(&[0x11, 0]);
    // version, oldest readable, and one map entry covering ids 1..=n at consecutive slots.
    for word in [1u16, 1, 1] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in [
        map as u32,
        8,
        offsets as u32,
        bodies.len() as u32,
        data as u32,
        bodies.iter().map(|b| b.len()).sum::<usize>() as u32,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in [
        resource::RESTYPE_IMAGE,
        1u16,
        (bodies.len() - 1) as u16,
        0u16,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
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
    assert_eq!(
        out.len(),
        at,
        "the table's last entry is the end of the data"
    );
    out
}

/// A directory of this test's own, somewhere the file system will not mind losing.
///
/// The save functions are the first thing in this module with a **write** side, so they are the first
/// ones that need a real directory rather than a value. Named by test and by process, so two tests —
/// and two runs of the suite at once — cannot land on each other's files.
fn scratch(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("quoin-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory can be made");
    dir
}

#[test]
fn a_pack_gets_one_save_directory_and_its_name_is_the_one_a_person_would_look_for() {
    let mac = std::path::Path::new("packs/alpha/motQA30-240x320/alpha.mod");
    let mine = default_save_dir(mac).expect("this build knows where its own executable is");
    // Stable, because the name is something a player will find on disk and come back to.
    assert_eq!(
        default_save_dir(mac).as_ref(),
        Some(&mine),
        "stable across calls"
    );
    // Two builds of one game do not share a save: a game may ship several, and a save one build wrote
    // is not something another reads.
    let theirs = default_save_dir(std::path::Path::new(
        "packs/alpha/motQA30-320x240/alpha.mod",
    ))
    .unwrap();
    assert_ne!(theirs, mine, "one directory per build, not one per game");
    // Named in the pack's own script, because that is the name a person would look for, and beside
    // the emulator, because that is where the run puts it.
    let name = mine.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("alpha-"), "{name}");
    assert!(
        mine.parent().unwrap().ends_with("saves"),
        "{}",
        mine.display()
    );
}

#[test]
fn the_saves_shown_are_the_ones_a_run_could_reach() {
    let dir = scratch("saves-listing");
    std::fs::write(dir.join("charlie"), b"a save").unwrap();
    // The last directory a run mounts: its files are in the volume, so they are saves.
    let reachable = (0..crate::file::MOUNT_DEPTH).fold(dir.clone(), |at, _| at.join("a"));
    std::fs::create_dir_all(&reachable).unwrap();
    std::fs::write(reachable.join("reachable"), b"x").unwrap();
    // One level deeper and the volume never gets there, so neither does this listing: offering it
    // would be offering to delete a file the game itself cannot see.
    let unreachable = reachable.join("a");
    std::fs::create_dir_all(&unreachable).unwrap();
    std::fs::write(unreachable.join("too-deep"), b"x").unwrap();

    let saves = saves_in(&dir);
    let names: Vec<&str> = saves.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["a/a/a/a/reachable", "charlie"],
        "sorted, and named the way the volume names them"
    );
    assert_eq!(saves[1].bytes, 6);
    assert!(
        saves[1].modified.is_some(),
        "a file just written has a time"
    );
    // A pack that has never been played has no directory, which is an answer and not an error.
    assert!(saves_in(&dir.join("never")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn deleting_a_save_cannot_be_aimed_outside_the_directory() {
    let dir = scratch("saves-delete");
    std::fs::write(dir.join("charlie"), b"x").unwrap();
    // These names come off the disk and are then used to delete something, which is the one way this
    // could stop being "delete a save".
    for bad in [
        "../charlie",
        "..",
        "/charlie",
        "",
        "nested\\win",
        "a/../../b",
    ] {
        assert!(delete_save(&dir, bad).is_err(), "{bad:?} was accepted");
    }
    assert!(delete_save(&dir, "charlie").is_ok());
    assert!(!dir.join("charlie").exists(), "the file is gone");
    assert!(
        dir.is_dir(),
        "the directory stays, so the next run finds a home"
    );
    // A file that is not there is worth saying out loud rather than reporting as deleted.
    assert!(delete_save(&dir, "charlie").is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn wiping_a_pack_forgets_it_and_leaves_it_somewhere_to_write() {
    let dir = scratch("saves-wipe");
    std::fs::write(dir.join("charlie"), b"x").unwrap();
    assert_eq!(saves_in(&dir).len(), 1);
    wipe_saves(&dir).unwrap();
    assert!(saves_in(&dir).is_empty(), "nothing is left");
    assert!(
        dir.is_dir(),
        "but the home is still there: a game played again starts over"
    );
    // Wiping what was never there asks for the state it is already in, so it is not a failure.
    assert!(wipe_saves(&dir).is_ok());
    assert!(wipe_saves(&dir.join("never-existed")).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The GUI library is the one way to start a pack that never sees the CLI's `--jit`:
/// `boot_at` builds its own `Runtime` and hands it straight to the session. If it
/// forgot to arm the backend, clicking a game in the manager would quietly keep the
/// interpreter while every command-line run sped up.
#[test]
#[cfg(feature = "jit")]
fn a_game_booted_from_the_library_arms_the_jit_backend() {
    let Some(path) = std::env::var_os("BREW_TEST_MOD") else {
        eprintln!("skipped: set BREW_TEST_MOD to a classic BREW .mod to check the library boot");
        return;
    };
    let entry = Entry {
        title: "probe".into(),
        build: "probe".into(),
        mod_path: std::path::PathBuf::from(path),
        screen: None,
        icon: None,
    };
    let rt = boot_at(&entry, None).expect("the library boots the pack");
    assert!(
        rt.cpu.jit_enabled(),
        "a pack launched from the GUI must use the JIT backend"
    );
}

#[test]
fn installing_a_pack_copies_it_into_the_device_tree_with_its_mif_beside_the_mod_directories() {
    let root = std::env::temp_dir().join(format!("quoin-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let source = root.join("Game/title/1073744343");
    std::fs::create_dir_all(source.join("res")).unwrap();
    std::fs::write(source.join("twins.mod"), b"image").unwrap();
    std::fs::write(source.join("program.bin"), b"program").unwrap();
    std::fs::write(source.join("res/data.bin"), b"data").unwrap();
    std::fs::write(root.join("Game/title/1073744343.mif"), b"mif").unwrap();

    let device = root.join("device");
    let installed = install_pack(&source.join("twins.mod"), &device).unwrap();
    assert_eq!(installed.id, "1073744343");
    assert_eq!(installed.files, 3);
    assert_eq!(
        std::fs::read(device.join("fs/mod/1073744343/twins.mod")).unwrap(),
        b"image"
    );
    assert_eq!(
        std::fs::read(device.join("fs/mod/1073744343/program.bin")).unwrap(),
        b"program"
    );
    assert_eq!(
        std::fs::read(device.join("fs/mod/1073744343/res/data.bin")).unwrap(),
        b"data"
    );
    assert_eq!(
        std::fs::read(device.join("fs/mif/1073744343.mif")).unwrap(),
        b"mif"
    );
    assert_eq!(
        app_info_of(&device.join("fs/mod/1073744343/twins.mod")),
        Some(device.join("fs/mif/1073744343.mif"))
    );
    let found = scan(&device.join("fs/mod"));
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].mod_path,
        device.join("fs/mod/1073744343/twins.mod")
    );
    assert!(
        source.join("twins.mod").is_file(),
        "the source pack is left in place"
    );
    assert!(
        install_pack(&source.join("twins.mod"), &device).is_err(),
        "a second install of the same id must not overwrite the first one's saves"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A `.mif` with one `BINARY` row, the shape an `AEEAppInfo` lives in.
fn app_mif(cls: u32) -> Vec<u8> {
    let mut raw = vec![0u8; 20];
    let at = usize::try_from(
        crate::abi::offset_of("AEEAppInfo", "cls").expect("AEEAppInfo has a cls field"),
    )
    .unwrap();
    raw[at..at + 4].copy_from_slice(&cls.to_le_bytes());
    let bodies = vec![raw];
    let (map, entries) = (0x20usize, 1usize);
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
    for word in [crate::resource::RESTYPE_BINARY, 2u16, 0u16, 0u16] {
        out.extend_from_slice(&word.to_le_bytes());
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

/// An ARM image whose entry returns immediately, with the class it aims at written as a literal.
fn image_naming(marker: Option<u32>) -> Vec<u8> {
    let mut out = vec![0u8; 64];
    out[..4].copy_from_slice(&0xe92d_00f0u32.to_le_bytes()); // push {r4, r5, r6, r7, lr}
    if let Some(cls) = marker {
        out[16..20].copy_from_slice(&cls.to_le_bytes());
    }
    out[60..64].copy_from_slice(&0xe12f_ff1eu32.to_le_bytes()); // bx lr
    out
}

/// `<root>/bundle/{app,other}`, with `other.mif` present only when it claims to be an applet.
fn two_app_bundle(
    tag: &str,
    other_cls: Option<u32>,
    marker: Option<u32>,
) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(tag);
    let bundle = root.join("bundle");
    std::fs::create_dir_all(bundle.join("app")).unwrap();
    std::fs::create_dir_all(bundle.join("other")).unwrap();
    std::fs::write(bundle.join("app.mif"), app_mif(0x0100_0001)).unwrap();
    let app_mod = bundle.join("app/app.mod");
    let other_mod = bundle.join("other/other.mod");
    std::fs::write(&app_mod, image_naming(marker)).unwrap();
    std::fs::write(&other_mod, image_naming(None)).unwrap();
    if let Some(cls) = other_cls {
        std::fs::write(bundle.join("other.mif"), app_mif(cls)).unwrap();
    }
    (root, app_mod, other_mod)
}

#[test]
fn an_unrelated_game_in_the_bundle_is_not_loaded_as_a_library() {
    let (root, app_mod, _other_mod) =
        two_app_bundle("sibling-unrelated", Some(0x0103_46d4), Some(0x0180_00fe));
    let mut rt = Runtime::with_device(DeviceProfile::standard());
    let loaded = load_sibling_modules(&mut rt, &app_mod);
    assert!(loaded.is_empty(), "{loaded:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_shim_loads_the_app_whose_applet_class_it_names() {
    let cls = 0x0107_f149;
    let (root, app_mod, other_mod) = two_app_bundle("sibling-named", Some(cls), Some(cls));
    let mut rt = Runtime::with_device(DeviceProfile::standard());
    let loaded = load_sibling_modules(&mut rt, &app_mod);
    assert_eq!(loaded.len(), 1, "{loaded:?}");
    assert_eq!(std::path::Path::new(&loaded[0]), other_mod, "{loaded:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_sibling_with_no_applet_class_is_still_loaded_as_a_library() {
    let (root, app_mod, other_mod) = two_app_bundle("sibling-library", None, None);
    let mut rt = Runtime::with_device(DeviceProfile::standard());
    let loaded = load_sibling_modules(&mut rt, &app_mod);
    assert_eq!(loaded.len(), 1, "{loaded:?}");
    assert_eq!(std::path::Path::new(&loaded[0]), other_mod, "{loaded:?}");
    let _ = std::fs::remove_dir_all(root);
}