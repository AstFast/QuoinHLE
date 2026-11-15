// Moved out of `src/manager.rs` so that file is implementation only; the `#[path]` attribute on its
// `mod tests;` declaration is what keeps this a child of that module, and `use super::*` is how it
// reaches the private items. Rows are built by hand here: listing is not starting, so nothing in this
// file reads a pack, needs a corpus, or can boot one.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::*;
use crate::library::{Candidate, Entry, Icon, Pick, Row, Witness};

/// A pack as the list would hold it: everything the list can see, and nothing that needs a file.
fn pack(title: &str, build: &str) -> Row {
    Row {
        entry: Entry {
            title: title.to_string(),
            build: build.to_string(),
            mod_path: PathBuf::from(format!("Game/{title}/{build}/{title}.mod")),
            screen: Some((240, 320)),
            icon: None,
        },
        subtitle: format!("{build}  240x320"),
        detail: Vec::new(),
        sizes: Vec::new(),
        caution: None,
    }
}

fn search(text: &str) -> Query {
    Query {
        text: text.to_string(),
        filter: Filter::All,
    }
}

/// Run a query and hand back the titles it kept, in order — what a person would read down the list.
fn kept(rows: &[Row], query: &Query) -> Vec<String> {
    Plan::of(rows, query)
        .hits
        .iter()
        .map(|&i| rows[i].entry.title.clone())
        .collect()
}

#[test]
fn the_game_window_switch_is_keyed_on_the_desktop_and_defaults_to_on() {
    // Two of the three inputs `Manager::game_window_wanted` ANDs together; the third is
    // `ctx.embed_viewports()`, which a headless test has no backend to ask. What is worth pinning here is
    // that the condition is written the right way round — desktop, not phone — and that the answer a file
    // gives when it has never been asked is "yes".
    assert!(
        SEPARATE_WINDOW_POSSIBLE,
        "the test runner is a desktop build, and a desktop build can open a game window"
    );
    assert!(Data::default().uses_separate_window(), "default is yes");
    let mut off = Data::default();
    off.set_separate_window(false);
    assert!(!off.uses_separate_window(), "and a person can say no");
}

#[test]
fn a_search_finds_a_pack_by_its_name_its_build_or_its_folder() {
    let rows = vec![
        pack("alpha", "motQA30-320x240"),
        pack("bravo", "[240×320]"),
        pack("charlie", "283491"),
    ];

    // By name, and without caring how it was typed.
    assert_eq!(kept(&rows, &search("lph")), ["alpha"]);
    assert_eq!(kept(&rows, &search("ALPHA")), ["alpha"]);
    // By build: the model and the handset a pack was filed under are not in its name, and they are
    // exactly what a person types when they are looking for the build that matches their own phone.
    assert_eq!(kept(&rows, &search("motqa30")), ["alpha"]);
    // By the folder it sits in — the full-width multiplication sign included, since that is what a
    // phone keyboard produced for that pack's directory and what a person would copy.
    assert_eq!(kept(&rows, &search("240×320")), ["bravo"]);
    // And the path search finds every pack filed under a size, whatever they are called.
    let under = kept(&rows, &search("240x320"));
    assert!(under.is_empty(), "that build's own folder uses ×, not x");
    assert_eq!(kept(&rows, &search("Game/")), ["alpha", "bravo", "charlie"]);
    // Nothing matches nothing.
    assert!(kept(&rows, &search("no such pack")).is_empty());
    // Whitespace around a query is not part of it: a pasted name usually arrives with some.
    assert_eq!(kept(&rows, &search("  charlie  ")), ["charlie"]);
}

#[test]
fn a_filter_lists_only_what_the_bytes_can_actually_show() {
    // Five packs, one per observable difference. Nothing here is a guess about whether a pack *runs*:
    // this host has no way to know that, which is why it is not offered.
    let mut with_caution = pack("caution", "240x320");
    with_caution.caution = Some("evidence too thin".to_string());
    let mut with_art = pack("art", "240x320");
    with_art.entry.icon = Some(Icon {
        width: 16,
        height: 16,
        rgb: vec![0; 16 * 16 * 3],
    });
    let mut claimed = pack("claimed", "240x320");
    claimed.sizes = vec![Candidate {
        width: 240,
        height: 320,
        witness: Witness::Folder,
        evidence: "from the directory: 240x320".to_string(),
    }];
    let plain = pack("plain", "240x320");
    let rows = vec![with_caution, with_art, claimed, plain];

    let under = |filter: Filter| {
        kept(
            &rows,
            &Query {
                text: String::new(),
                filter,
            },
        )
    };
    assert_eq!(under(Filter::All), ["caution", "art", "claimed", "plain"]);
    assert_eq!(under(Filter::Caution), ["caution"]);
    assert_eq!(under(Filter::Clean), ["art", "claimed", "plain"]);
    assert_eq!(under(Filter::WithArt), ["art"]);
    assert_eq!(under(Filter::WithoutArt), ["caution", "claimed", "plain"]);
    assert_eq!(under(Filter::Claimed), ["claimed"]);
    assert_eq!(under(Filter::Unclaimed), ["caution", "art", "plain"]);
    // Every filter has a name a player can read, and they are all distinct — two options reading the
    // same word is a selector nobody can use.
    let names: Vec<&str> = Filter::ALL.iter().map(|f| f.name(UiLang::Zh)).collect();
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(names.len(), unique.len(), "{names:?}");
}

#[test]
fn the_same_game_filed_for_four_handsets_is_one_group() {
    // This is what grouping is for: a game may ship several builds for several screens, and a list
    // that shows them as unrelated games has buried the fact that it is one game.
    let rows = vec![
        pack("alpha", "motQA30-320x240"),
        pack("alpha", "motQA30-240x320"),
        pack("alpha", "283491"),
        pack("alpha", "motZ6-176x220"),
        pack("charlie", "283491"),
    ];
    let plan = Plan::of(&rows, &Query::default());
    assert_eq!(plan.packs(), 5);
    assert_eq!(plan.games(), 2);
    assert_eq!(plan.groups[0].title, "alpha");
    assert_eq!(plan.groups[0].rows, [0, 1, 2, 3]);
    assert_eq!(plan.groups[1].title, "charlie");
    assert_eq!(plan.groups[1].rows, [4]);
    // The groups come out in the order their first build does, so the list does not reshuffle itself
    // when a search narrows it: searching inside a game keeps that game where it was.
    let narrowed = Plan::of(&rows, &search("motZ6"));
    assert_eq!(narrowed.games(), 1);
    assert_eq!(narrowed.groups[0].title, "alpha");
    assert_eq!(narrowed.groups[0].rows, [3]);
}

// ────────────────────────────── the panes ──────────────────────────────

/// One layout pass over the whole window with no window at all, and the text a person would read back.
///
/// No eframe, no frame, no guest: this is how these panes get checked while nobody is allowed to boot a
/// pack. It reads painted text only — it cannot say whether the layout looks good.
fn painted_text(app: &mut Manager) -> String {
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(1280.0, 800.0),
        )),
        ..Default::default()
    };
    // **Two frames**, and the second one is the one that counts: a free-floating layer — which is what a
    // card's window is — measures its own contents on the frame it first appears and only paints them on
    // the next one. A panel does not have that habit, which is why one frame was enough before there was
    // a window to draw.
    let _ = ctx.run(input(), |ctx| app.panels(ctx));
    let out = ctx.run(input(), |ctx| app.panels(ctx));
    out.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A manager with hand-made rows: listing a pack is not reading it, and neither is starting it.
fn manager(rows: Vec<Row>) -> Manager {
    let query = Query::default();
    let plan = Plan::of(&rows, &query);
    let picks = vec![None; rows.len()];
    Manager {
        root: PathBuf::from("Game"),
        rows,
        query,
        plan,
        picks,
        typed: (String::new(), String::new()),
        icons: Vec::new(),
        selected: None,
        note: None,
        page: Page::Games,
        nav_open: true,
        fonts: Vec::new(),
        data: Data::default(),
        data_path: None,
        editing: None,
        capturing: None,
        dragging: None,
        drag_over: None,
        theme: Theme::Dark,
        lang: UiLang::Zh,
        saves: Vec::new(),
        save_dir: None,
        saves_for: None,
        playing: None,
        loading: None,
        current: None,
        skin: &crate::handset::SKINS[0],
        sound_checked: BTreeSet::new(),
        sound_add: String::new(),
        sound_drag: None,
        sound_over: None,
        dirty: false,
        left: false,
        dev_saved: None,
    }
}

#[test]
fn a_saved_tree_is_named_for_the_pack_and_the_moment() {
    assert_eq!(
        flame_file_name(Some("delta"), 1_770_000_000),
        "flame-delta-1770000000.txt"
    );
    // A Chinese title survives: a pack's title usually is one, and dropping it would leave a directory
    // of saves nobody can tell apart.
    assert_eq!(flame_file_name(Some("echo"), 42), "flame-echo-42.txt");
    // The characters a Windows name cannot hold are dropped rather than replaced — a title padded with
    // `-` says less than a shorter one that still reads.
    assert_eq!(
        flame_file_name(Some(r#"a/b\c:d*e?f"g<h>i|j"#), 7),
        "flame-abcdefghij-7.txt"
    );
    // Nothing to go on: still a name, and still unique by the second.
    assert_eq!(flame_file_name(None, 9), "flame-9.txt");
    assert_eq!(flame_file_name(Some("   "), 9), "flame-9.txt");
    assert_eq!(flame_file_name(Some("..."), 9), "flame-9.txt");
    // A long title is cut rather than allowed to run into the path limit beside a long install path.
    assert_eq!(
        flame_file_name(Some(&"x".repeat(80)), 1),
        format!("flame-{}-1.txt", "x".repeat(32))
    );
}

#[test]
fn the_pane_paints_the_selected_packs_report_and_nothing_boots() {
    let mut row = pack("pane", "240x320");
    row.detail = vec![
        "  screen inference: **canvas candidate 240x320**".to_string(),
        "  ⚠ evidence too thin: the largest picture in a readable container is only 65x42"
            .to_string(),
    ];
    row.caution = Some("evidence too thin".to_string());
    let mut app = manager(vec![row]);
    app.open_card("pane");
    assert!(
        app.editing
            .as_ref()
            .is_some_and(|held| held.build == Some(0)),
        "open_card points the window at the game's only build: {:?}",
        app.editing
    );
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("canvas candidate 240x320"),
        "the pane shows the report: {painted}"
    );
    assert!(
        painted.contains("pane"),
        "the card carries the name: {painted}"
    );
    assert_eq!(
        painted.matches("evidence too thin").count(),
        1,
        "the warning is painted once, not once per place that mentions it: {painted}"
    );
    // Listing a pack reads its bytes; it does not start them.
    assert!(app.playing.is_none(), "a layout pass must not boot a pack");

    // After the window closes, only cards remain in the library: the report belonged to the window, not the wall.
    app.editing = None;
    let painted = painted_text(&mut app);
    assert!(
        !painted.contains("canvas candidate"),
        "nothing of a report is painted once the window is shut: {painted}"
    );
    assert!(
        painted.contains("pane"),
        "the card is still there: {painted}"
    );
}

#[test]
fn the_picker_names_the_size_a_launch_would_use() {
    // Two witnesses that disagree, so the label proves which one a boot would take. The row is built by
    // hand: listing is not starting, and nothing here reads a pack.
    let mut row = pack("picker", "picker-240x320");
    row.detail = vec!["  screen inference: **canvas candidate 320x240**".to_string()];
    row.sizes = vec![
        Candidate {
            width: 240,
            height: 320,
            witness: Witness::Folder,
            evidence: "from the directory: picker-240x320".to_string(),
        },
        Candidate {
            width: 320,
            height: 240,
            witness: Witness::Art,
            evidence:
                "width from a 320-pixel-wide bar; 2 picture(s) at that width, the tallest gives 240"
                    .to_string(),
        },
    ];
    let mut app = manager(vec![row]);
    app.open_card("picker");

    let painted = painted_text(&mut app);
    for want in [
        "directory 240x320",
        "art 320x240",
        "from the directory: picker-240x320",
        "width from a 320-pixel-wide bar",
        "手填",
    ] {
        assert!(
            painted.contains(want),
            "the pane must show {want}:\n{painted}"
        );
    }
    // Nothing is preselected, and the button says which number that leaves: the fallback is named rather
    // than quietly applied, because a wrong canvas shows up as a wrong picture and nothing else.
    assert!(app.picks[0].is_none(), "a fresh row has no pick");
    assert!(
        painted.contains("240x320 from the directory (not chosen)"),
        "the button carries the fallback in words:\n{painted}"
    );

    // Choosing the art claim changes the number the launch would use — the one step here that changes
    // what a run gets. Asserted on the label, because booting a pack is the thing this repository is not
    // allowed to do.
    let art = app.rows[0].sizes[1].pick();
    app.picks[0] = Some(art);
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("320x240 from art's evidence"),
        "the button follows the pick:\n{painted}"
    );
    assert!(
        !painted.contains("from the directory (not chosen)"),
        "and stops offering the fallback once something is chosen:\n{painted}"
    );

    // A half-typed size is not a third claim: the boxes stay, the radio does not appear, and the reason
    // is painted where the radio would have been.
    app.picks[0] = None;
    app.typed = ("320".to_string(), String::new());
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("宽、高都要是"),
        "a one-sided size is refused out loud:\n{painted}"
    );
    app.typed = ("320".to_string(), "240".to_string());
    let painted = painted_text(&mut app);
    assert!(
        !painted.contains("宽、高都要是"),
        "a complete size is offered instead of the reason:\n{painted}"
    );
    assert!(
        painted.contains("240x320 from the directory (not chosen)"),
        "typing is not choosing: the button still promises the fallback until a claim is clicked:\n{painted}"
    );
    app.picks[0] = Some(Pick::Manual {
        width: 320,
        height: 240,
    });
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("320x240 typed by hand"),
        "the hand-typed claim boots at its own number:\n{painted}"
    );
    assert!(app.playing.is_none(), "a layout pass must not boot a pack");
}

#[test]
fn the_list_follows_the_query_and_the_icon_wall_shows_the_same_packs() {
    let alpha = |build: &str| pack("alpha", build);
    let mut app = manager(vec![
        alpha("motQA30-320x240"),
        alpha("283491"),
        pack("charlie", "283491"),
    ]);
    // Two builds of one game are one **card**, which is what a library is: a card per game, and the count on
    // it says how many builds there are rather than hiding them.
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("2 build(s)"),
        "the card says how many builds of the game there are:\n{painted}"
    );
    assert!(
        painted.contains("charlie"),
        "and the other game is still its own row"
    );

    // A query narrows the list, and the heading goes with it rather than staying behind.
    app.query = search("charlie");
    app.rebuild();
    let painted = painted_text(&mut app);
    assert!(painted.contains("charlie"), "{painted}");
    assert!(
        !painted.contains("2 build(s)"),
        "one build left is not a group:\n{painted}"
    );

    // Clearing the query brings both cards back — cards are per game, not per build.
    app.query = Query::default();
    app.rebuild();
    let painted = painted_text(&mut app);
    assert!(painted.contains("alpha"), "{painted}");
    assert!(
        painted.contains("charlie"),
        "both games have a card: {painted}"
    );
}

#[test]
fn the_saves_page_lists_what_the_pack_saved() {
    // The listing is handed over rather than read: reading the real directory would need the pack to
    // have been played, and `saves_for` is what says a listing is already in hand.
    let mut app = manager(vec![pack("charlie", "283491")]);
    app.selected = Some("charlie".to_string());
    app.page = Page::Saves;
    app.saves_for = Some("charlie/283491/charlie.mod".to_string());
    app.save_dir = Some(PathBuf::from("saves/charlie-1a2b3c4d"));
    app.saves = vec![crate::library::Save {
        name: "charlie".to_string(),
        bytes: 12,
        modified: None,
    }];
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("charlie"),
        "the save is listed:\n{painted}"
    );
    assert!(painted.contains("12 字节"), "with its size:\n{painted}");
    assert!(
        painted.contains("删除"),
        "and a way to remove it:\n{painted}"
    );
    assert!(
        painted.contains("清空这个包的存档"),
        "and the way to start the game over:\n{painted}"
    );
    // Nothing here starts a pack, and nothing here reads one either.
    assert!(app.playing.is_none());
}

#[test]
fn a_list_icon_is_always_at_a_whole_multiple_of_its_own_pixels() {
    // The list used to draw a 16x16 icon into a 40-point box through a linear filter — 2.5x, exactly the
    // resampling this file promises not to do to pixel art.
    for (w, h) in [(16, 16), (8, 8), (24, 16), (26, 26), (64, 64), (1, 1)] {
        let s = icon_size(w, h);
        let (wide, high) = (w as f32, h as f32);
        assert_eq!((s.x % wide, s.y % high), (0.0, 0.0), "{w}x{h} → {s:?}");
        assert_eq!(
            s.x / wide,
            s.y / high,
            "{w}x{h} → {s:?}: one scale for both axes, or the icon is stretched"
        );
        assert!(s.x >= wide && s.y >= high, "{w}x{h} → {s:?} shrinks");
    }
    // The specific sizes, so the loop above cannot be satisfied by an implementation that always answers
    // 1x, and "the *largest* whole multiple" cannot be satisfied by one that leaves room on the table.
    assert_eq!(icon_size(16, 16), Vec2::splat(48.0));
    assert_eq!(icon_size(8, 8), Vec2::splat(48.0));
    assert_eq!(icon_size(24, 16), Vec2::new(48.0, 32.0));
    assert_eq!(
        icon_size(26, 26),
        Vec2::splat(26.0),
        "one step, because two would not fit"
    );
    // 16 is what every build in the corpus carries, and it lands exactly on the box. An icon larger than
    // the box has no whole multiple below one, so it keeps its own pixels and the cell gives way to it
    // rather than the picture being shrunk through a filter.
    assert_eq!(icon_size(16, 16), Vec2::splat(ICON_BOX));
    assert_eq!(icon_size(64, 64), Vec2::splat(64.0));
}

#[test]
fn an_ending_note_names_the_pack_and_says_nothing_when_there_is_nothing_to_say() {
    // A game that ended the way a game ends is not news, and an empty red line above a healthy list
    // reads as a fault that is not there.
    assert_eq!(ending_note(Some("alpha".into()), None), None);
    assert_eq!(ending_note(None, None), None);
    assert_eq!(
        ending_note(Some("delta".into()), Some("guest fault".into())),
        Some("delta: guest fault".to_string())
    );
    // The name is dropped once it has been used: a later failure with no pack behind it is reported as
    // itself rather than under the name of the game before it.
    assert_eq!(
        ending_note(None, Some("no applet".into())),
        Some("no applet".to_string())
    );
}

/// A directory of this test's own, somewhere the file system will not mind losing.
///
/// Deleting a save is a **write** action on a real directory, so the one test that covers it needs one.
/// Named by test and by process, so two tests — and two runs of the suite at once — cannot land on each
/// other's files.
fn scratch(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("quoin-manager-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory can be made");
    dir
}

#[test]
fn forgetting_a_save_deletes_it_and_says_so_when_it_cannot() {
    // This is the path that removes a player's files. It is checked here rather than by clicking a
    // button, because a button needs a window, a pointer and a layout pass, and this needs none of them.
    let dir = scratch("forget");
    std::fs::write(dir.join("charlie"), b"a save").unwrap();
    let mut app = manager(vec![pack("charlie", "283491")]);
    app.save_dir = Some(dir.clone());
    app.saves_for = Some("charlie/283491/charlie.mod".to_string());
    app.saves = crate::library::saves_in(&dir);
    assert_eq!(app.saves.len(), 1, "the row holds what is really there");

    // A name that is not there is reported rather than passing for a delete.
    app.forget(&dir, Forgets::One("nope".into()));
    assert!(app.note.is_some(), "a delete that did nothing says so");
    assert!(dir.join("charlie").exists(), "and nothing else was touched");

    // The real thing.
    app.note = Some("the previous failure".to_string());
    app.forget(&dir, Forgets::One("charlie".into()));
    assert!(!dir.join("charlie").exists(), "the file is gone");
    assert!(
        app.note.is_none(),
        "and the previous failure is no longer being read: {:#?}",
        app.note
    );
    assert!(
        app.saves.is_empty(),
        "and the listing is read again straight away, so the deleted save is gone from it: {:#?}",
        app.saves
    );

    // And the whole directory, which is "this game starts over".
    std::fs::write(dir.join("charlie"), b"played again").unwrap();
    app.saves_for = Some("charlie/283491/charlie.mod".to_string());
    app.forget(&dir, Forgets::All);
    assert!(crate::library::saves_in(&dir).is_empty(), "nothing is left");
    assert!(dir.is_dir(), "but the home is still there");
    std::fs::remove_dir_all(&dir).unwrap();
}

// ────────────────────────────── the nav ──────────────────────────────

#[test]
fn what_start_up_cannot_honour_is_said_out_loud() {
    // The manager is a window with no console behind it, and a double-clicked exe cannot be given a flag:
    // for these two there is no other place the mistake could surface.
    assert_eq!(complaints("graphite", None), None);
    assert_eq!(
        complaints("", None),
        None,
        "an empty name is unset, not wrong"
    );

    let unknown = complaints("nope", None).expect("an unknown skin is said");
    assert!(
        unknown.contains("nope") && unknown.contains("graphite"),
        "it names what was asked for and what is used instead: {unknown}"
    );

    let missing = complaints("silver", Some(std::path::Path::new("no-such.sf2")))
        .expect("a SoundFont that is not there is said");
    assert!(missing.contains("no-such.sf2"), "{missing}");

    // Both wrong is two things wrong, not the first one.
    let both = complaints("nope", Some(std::path::Path::new("no-such.sf2"))).unwrap();
    assert!(
        both.contains("nope") && both.contains("no-such.sf2"),
        "{both}"
    );
}

// ────────────────────────────── the cards ──────────────────────────────

#[test]
fn a_drop_lands_where_the_pointer_last_was_over() {
    // Where a drag lands has to be recorded **on the frames the pointer is moving**: by the frame the drag
    // ends, egui no longer knows what was being held, so asking then always answers "nowhere" — and a drop
    // that lands nowhere is a reorder that silently does not happen. This is the half of that which is worth
    // a test: what happens once a drop has been seen.
    let mut app = manager(vec![pack("a", "1"), pack("b", "1"), pack("c", "1")]);
    app.moved(Some("a".to_string()), Some("c".to_string()));
    assert_eq!(app.data.order, ["b", "c", "a"], "a was dragged to the end");

    // Dropped on nothing, on itself, or with nothing being dragged: the order is not a thing that changed.
    let before = app.data.order.clone();
    app.moved(Some("a".to_string()), None);
    app.moved(Some("b".to_string()), Some("b".to_string()));
    app.moved(None, Some("c".to_string()));
    assert_eq!(app.data.order, before);
}

#[test]
fn the_sound_page_lists_the_builtin_bank_first_and_can_add() {
    let mut app = manager(vec![pack("charlie", "283491")]);
    app.page = Page::Sounds;
    let painted = painted_text(&mut app);
    assert!(painted.contains("音效库"), "{painted}");
    assert!(
        painted.contains("内置音色表"),
        "the first row is the host's own tables, which is what it opens with: {painted}"
    );
    // The old wording promised silence for that row. Since `#19` it is a bank like any other, and a
    // picker that says "no sound" over a run that is about to make some is the kind of lie this
    // repository keeps being told to hunt down — so the phrase is pinned out, not just left behind.
    assert!(
        !painted.contains("无（静音）"),
        "the page still offers silence as the default: {painted}"
    );
    assert!(
        painted.contains("Add"),
        "the bottom row can add one: {painted}"
    );

    // One in the library: it is listed, it is the one in use, and a path that is not there says so.
    assert!(app.data.add_sound("C:/nowhere/TimGM6mb.sf2"));
    let painted = painted_text(&mut app);
    assert!(painted.contains("TimGM6mb.sf2"), "{painted}");
    assert!(
        painted.contains("在用"),
        "the one in use says so: {painted}"
    );

    // A second one does not take over from it, and a listed path whose file is not there says that rather
    // than doing nothing quietly.
    assert!(app.data.add_sound("C:/nowhere/Other.sf2"));
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("Other.sf2") && painted.contains("文件不在了"),
        "a path that is not there is said out loud: {painted}"
    );
}

#[test]
fn the_nav_has_two_groups_and_collapsing_only_takes_the_words_away() {
    let mut app = manager(vec![pack("charlie", "283491")]);
    let painted = painted_text(&mut app);
    assert!(
        painted.contains("主菜单"),
        "the first group has a heading: {painted}"
    );
    assert!(
        painted.contains("菜单"),
        "and so does the second: {painted}"
    );
    for page in Page::MAIN.iter().chain(Page::MENU) {
        // The whole label, not just the word: `nav_label` is what decides whether the character is there,
        // and asserting the name alone would pass for a nav that had dropped every icon.
        let label = page.name(UiLang::Zh);
        assert!(
            painted.contains(&label),
            "{page:?} is an entry in the nav: {painted}"
        );
    }
    assert!(
        painted.contains("重新扫描"),
        "the action is here: {painted}"
    );
    assert!(
        painted.contains("主题："),
        "and the theme entry says which one is in use rather than just naming itself: {painted}"
    );

    // Collapsed hides the page entries: their one-character icons are gone for now, and the rail
    // keeps only the actions that still have a word (theme and rescan).
    app.nav_open = false;
    let painted = painted_text(&mut app);
    for page in Page::MAIN.iter().chain(Page::MENU) {
        assert!(
            painted.contains(page.short(UiLang::Zh)),
            "{page:?} keeps a short entry in the rail: {painted}"
        );
    }
    assert!(
        !painted.contains("重新扫描") && !painted.contains("主菜单"),
        "the words are what collapsing takes away: {painted}"
    );
}

#[test]
fn the_manager_speaks_english_when_asked() {
    let mut app = manager(vec![pack("charlie", "283491")]);
    app.lang = UiLang::En;
    let painted = painted_text(&mut app);
    for want in [
        "Main",
        "Menu",
        "Games",
        "Sounds",
        "Fonts",
        "Keys",
        "Saves",
        "Settings",
        "Developer",
        "Search",
        "All",
    ] {
        assert!(
            painted.contains(want),
            "the English nav must show {want}: {painted}"
        );
    }

    app.page = Page::Settings;
    let painted = painted_text(&mut app);
    for want in [
        "Settings",
        "Interface language",
        "Theme",
        "Skin (next run)",
        "Audio backend (next run)",
        "Game window (next run)",
        "Pack root",
    ] {
        assert!(
            painted.contains(want),
            "the English settings must show {want}: {painted}"
        );
    }
}

#[test]
fn a_nav_entry_decides_what_the_right_hand_column_draws() {
    let mut row = pack("charlie", "283491");
    row.detail = vec!["  screen inference: **canvas candidate 240x320**".to_string()];
    let mut app = manager(vec![row]);
    app.open_card("charlie");

    // The library is the pack: its claims and its report.
    let painted = painted_text(&mut app);
    assert!(painted.contains("canvas candidate 240x320"), "{painted}");

    // Settings is the host itself — and it is not the report, which is the point of it being its own page.
    // The card window is shut first: it floats over whatever page is showing, so leaving it open would
    // leave its report in the reading.
    app.editing = None;
    app.page = Page::Settings;
    let painted = painted_text(&mut app);
    assert!(painted.contains("包根目录"), "{painted}");
    assert!(
        painted.contains("Game"),
        "the directory it reads: {painted}"
    );
    assert!(
        painted.contains("graphite"),
        "and the skin in use is offered by name: {painted}"
    );
    assert!(
        !painted.contains("canvas candidate 240x320"),
        "about is not a reading page: {painted}"
    );

    // The skin page offers every skin there is, which is the only way to see what the names mean.
    app.page = Page::Settings;
    let painted = painted_text(&mut app);
    for skin in crate::handset::SKINS {
        assert!(painted.contains(skin.name), "{painted}");
    }

    // The sound page says where it stands, and the state it says out loud is no longer "silent": the
    // default is the host's own tables now, and a page that called that state silence would be saying
    // the opposite of what the run does.
    app.page = Page::Sounds;
    let painted = painted_text(&mut app);
    assert!(painted.contains("内置音色表"), "{painted}");

    // None of that starts a pack.
    assert!(app.playing.is_none());
}

#[test]
fn a_rescan_forgets_what_was_chosen_and_what_was_read() {
    // A rescan rebuilds the rows, so everything kept *by row index* has to go with them: a size pick or an
    // icon left at index 3 lands on whichever pack is at index 3 now.
    let mut app = manager(vec![pack("charlie", "283491")]);
    app.selected = Some("charlie".to_string());
    app.picks[0] = Some(Pick::Manual {
        width: 320,
        height: 240,
    });
    app.saves_for = Some("charlie/283491/charlie.mod".to_string());
    app.typed = ("320".to_string(), String::new());
    // A directory that certainly has no packs in it, so the test does not depend on where it was run.
    app.root = PathBuf::from("no-such-directory-quoin-test");
    app.note = Some("the previous failure".to_string());
    app.rescan();

    assert!(app.rows.is_empty(), "the root it reads has nothing in it");
    assert!(app.picks.is_empty(), "and the picks went with the rows");
    assert!(app.icons.is_empty(), "and so did the icons");
    assert_eq!(app.selected, None, "and the selection");
    assert_eq!(app.saves_for, None, "and the save listing's owner");
    assert_eq!(
        app.typed,
        (String::new(), String::new()),
        "and the half-typed size, which belonged to the pack that was chosen"
    );
    assert!(
        app.note.is_some(),
        "a rescan says nothing about itself: it is not a failure, and the note is for failures"
    );
}

/// A 1x1 24-bit BMP — the one shape `resource::bmp` takes.
///
/// Built here rather than imported because the library's own BMP fixture lives inside `runtime`'s test
/// module, and a second fixture is cheaper than no test at all: if `resource::bmp` ever changes what it
/// accepts, this builder fails **loudly** (the icon stops loading) rather than quietly agreeing with a
/// shape nothing else checks.
fn tiny_bmp() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&58u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&1i32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&4u32.to_le_bytes());
    for _ in 0..4 {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    out.extend_from_slice(&[7, 170, 40, 0]);
    out
}

/// The icon a person picks is judged **by its bytes first**, exactly as `IShell::DetectType` judges a
/// payload, and each refusal says which of the two questions it fell out of.
///
/// `custom_icon` used to read the file name's extension and nothing else. That is the shape
/// this host refuses: a BMP saved as `icon.png` was
/// refused by the name table and then handed to a decoder that would have taken it gladly. All three
/// arms are here — bytes decide, suffix decides, neither does — because the two refusals mean
/// different things to the person looking at the card: one is the host's gap, the other is their file.
#[test]
fn a_custom_icon_is_decided_by_its_bytes_and_says_which_half_refused_it() {
    let dir = scratch("icon-authority");
    let ctx = egui::Context::default();
    let mut app = manager(vec![pack("alpha", "284265")]);
    app.root = dir.clone();
    app.lang = UiLang::En;

    // (1) a real BMP wearing a `.png` name: the bytes answer first, so it loads and says nothing.
    std::fs::write(dir.join("lies.png"), tiny_bmp()).unwrap();
    app.data.set_icon("alpha", "lies.png");
    app.note = None;
    assert!(
        app.custom_icon("alpha", &ctx).is_some(),
        "the bytes, not the suffix, decide: {:?}",
        app.note
    );
    assert!(
        app.note.is_none(),
        "a loaded icon stays quiet: {:?}",
        app.note
    );

    // (2) bytes no authority reads and a suffix no table knows: the host's own gap, said as such.
    std::fs::write(dir.join("score.mid"), b"MThd\x00\x00\x00\x06").unwrap();
    app.data.set_icon("alpha", "score.mid");
    app.note = None;
    assert!(app.custom_icon("alpha", &ctx).is_none());
    assert_eq!(
        app.note.as_deref(),
        Some("icon score.mid: this host has no decoder for it")
    );

    // (3) the reverse — a suffix that *does* name a format this host decodes, over bytes that fail
    // under it. The note here must be the decode one, so the two sayings cannot be confused.
    std::fs::write(
        dir.join("half.png"),
        [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'],
    )
    .unwrap();
    app.data.set_icon("alpha", "half.png");
    app.note = None;
    assert!(app.custom_icon("alpha", &ctx).is_none());
    assert_eq!(app.note.as_deref(), Some("icon half.png cannot be decoded"));
}

#[test]
fn the_language_tag_round_trips_and_unknown_tags_are_none() {
    assert_eq!(UiLang::from_tag("en"), Some(UiLang::En));
    assert_eq!(UiLang::from_tag("zh"), Some(UiLang::Zh));
    assert_eq!(UiLang::from_tag("fr"), None);
    assert_eq!(UiLang::En.tag(), "en");
    assert_eq!(UiLang::Zh.tag(), "zh");
}
