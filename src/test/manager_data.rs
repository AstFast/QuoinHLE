// Moved out of `src/manager_data.rs` so that file is implementation only. The `#[path]` attribute on its
// `mod tests;` declaration is what keeps this a child module of that file, and `use super::*` is how it
// reaches the private items. The reader is worth this much testing because it is the one thing here that
// can lose something a person typed.

use super::*;
use std::path::{Path, PathBuf};

/// A directory of this test's own.
fn scratch(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("quoin-data-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory can be made");
    dir
}

#[test]
fn what_the_manager_writes_it_can_read_back() {
    let mut data = Data::default();
    data.order = vec![
        "alpha".to_string(),
        "charlie".to_string(),
        "bravo".to_string(),
    ];
    data.set_name("alpha", "Pack A 豪华版");
    data.set_icon("alpha", "packs/alpha/icon.png");
    data.set_screen("charlie/283491/charlie.mod", Some((320, 240)));

    let text = data.to_json();
    let back = Data::from_json(&parse(&text).expect("what it wrote, it can read")).unwrap();
    assert_eq!(back, data, "round trip");

    // And the text itself is what a person would want to read: the names are not escaped into `\uXXXX`,
    // because the names in this file are mostly Chinese and an escaped one is one nobody recognises.
    assert!(text.contains("Pack A 豪华版"), "{text}");
    assert!(text.contains("bravo"), "{text}");
    assert!(text.contains("\"width\": 320"), "{text}");
}

#[test]
fn the_audio_backend_is_written_and_an_unknown_value_falls_back_to_rust() {
    let mut data = Data::default();
    assert_eq!(
        data.audio_backend(),
        AudioBackend::Rust,
        "an older manager.json has no backend key and means Rust"
    );
    data.set_audio_backend(AudioBackend::Ffmpeg);
    let text = data.to_json();
    assert!(text.contains("\"audio_backend\": \"ffmpeg\""), "{text}");
    let back = Data::from_json(&parse(&text).expect("what it wrote, it can read")).expect("shape");
    assert_eq!(back.audio_backend(), AudioBackend::Ffmpeg);

    let unknown = Data::from_json(&parse("{\"audio_backend\": \"vlc\"}").expect("document"))
        .expect("a shape this reader knows");
    assert_eq!(
        unknown.audio_backend(),
        AudioBackend::Rust,
        "a backend this build does not know is the default, not a crash"
    );
}

#[test]
fn a_game_window_is_separate_until_somebody_says_otherwise() {
    // Absent, `null`, and a value this build does not understand all read the same way: yes. That is what
    // every file written before this setting existed gets, and a person who has just installed this build
    // must not find the switch showing a state nobody chose.
    let never_asked = Data::from_json(
        &parse("{\"order\": [], \"sound_in_use\": null, \"separate_window\": null}")
            .expect("a document"),
    )
    .expect("a shape this reader knows");
    assert!(
        never_asked.uses_separate_window(),
        "absent or null says yes"
    );

    // The key being *absent* is a different document from the key being `null`, and it is the one every
    // file written before this setting existed is — so it gets its own assertion rather than riding on
    // the `null` one above.
    let older_file = Data::from_json(&parse("{\"order\": []}").expect("a document"))
        .expect("a shape this reader knows");
    assert!(
        older_file.uses_separate_window(),
        "a file with no such key says yes"
    );

    let nonsense = Data::from_json(&parse("{\"separate_window\": \"yes\"}").expect("a document"))
        .expect("a shape this reader knows");
    assert!(
        nonsense.uses_separate_window(),
        "a value it does not understand is the default, not a failure"
    );

    // Turning it off is written down as `false` and comes back as `false`. That an absent key and an
    // explicit `false` are different states is the whole reason the field is an `Option`.
    let mut off = Data::default();
    off.set_separate_window(false);
    let text = off.to_json();
    assert!(text.contains("\"separate_window\": false"), "{text}");
    let back =
        Data::from_json(&parse(&text).expect("what it wrote, it can read")).expect("a shape");
    assert!(!back.uses_separate_window());
    assert_eq!(back, off, "round trip");

    // And on again, which is what the checkbox does on the way back.
    let mut on = Data::default();
    on.set_separate_window(true);
    assert!(on.uses_separate_window());
    assert!(on.to_json().contains("\"separate_window\": true"));
}

#[test]
fn the_reader_takes_what_a_person_can_type_into_the_file() {
    // Whitespace, a field order of its own, fields this version does not know, and escapes.
    let json = parse(
        r#"{
            "note": "a field from some later build",
            "packs": { "x/y.mod": { "width": 240, "height": 320 } },
            "order": ["a", "b"],
            "games": { "a": { "name": "A\u00e9\ud83d\ude00", "unknown": 3 } }
        }"#,
    )
    .unwrap();
    let data = Data::from_json(&json).unwrap();
    assert_eq!(
        data.order,
        ["a", "b"],
        "the order is taken whatever it is written next to"
    );
    assert_eq!(
        data.name_of("a"),
        Some("Aé😀"),
        "escapes, including a surrogate pair"
    );
    assert_eq!(data.screen_of("x/y.mod"), Some((240, 320)));

    // Missing is default, and a number that is not a screen size is not a screen size.
    assert_eq!(
        Data::from_json(&parse("{}").unwrap()).unwrap(),
        Data::default()
    );
    let silly = parse(r#"{ "packs": { "k": { "width": -3, "height": 99999 } } }"#).unwrap();
    assert_eq!(
        Data::from_json(&silly).unwrap().screen_of("k"),
        None,
        "a negative width is not a width, and neither is one past what a screen can be"
    );
}

#[test]
fn a_file_that_is_not_json_is_refused_rather_than_half_read() {
    // Half-reading is the dangerous one: an order that was quietly dropped looks exactly like an order
    // that was respected.
    assert!(parse("{").is_err());
    assert!(
        parse(r#"{"order": [1, 2"#).is_err(),
        "an unterminated array"
    );
    assert!(parse("{} {}").is_err(), "two documents are not one");
    assert!(parse("").is_err());
    // A well-formed document of the wrong shape is refused too.
    let array = parse("[1, 2]").unwrap();
    assert!(Data::from_json(&array).is_err());
    let order_is_a_number = parse(r#"{"order": 3}"#).unwrap();
    assert!(Data::from_json(&order_is_a_number).is_err());
    let order_holds_numbers = parse(r#"{"order": [1, 2]}"#).unwrap();
    assert!(
        Data::from_json(&order_holds_numbers).is_err(),
        "an order that is not names is not an order"
    );
}

#[test]
fn a_game_the_file_has_never_heard_of_goes_last() {
    let mut data = Data::default();
    data.order = vec!["b".to_string(), "a".to_string()];
    assert!(data.rank("b") < data.rank("a"), "the file decides");
    assert!(
        data.rank("never seen") > data.rank("a"),
        "a new pack does not jump the queue"
    );
}

#[test]
fn dragging_a_game_moves_it_and_the_games_that_were_not_shown_follow() {
    let mut data = Data::default();
    data.order = vec!["a".to_string(), "b".to_string(), "c".to_string()];

    // What the window was drawing was the whole library: `a` dragged to the end.
    let drawn = data.order.clone();
    data.move_game(&drawn, 0, 2);
    assert_eq!(data.order, ["b", "c", "a"]);

    // A window showing a subset (a search was on) can only speak for what it showed. The games it did not
    // show keep their order and land after the ones it did — a filter is not a statement about where the
    // hidden games belong.
    let drawn = vec!["c".to_string(), "a".to_string()];
    data.move_game(&drawn, 0, 1);
    assert_eq!(data.order, ["a", "c", "b"]);

    // Out of range is not a panic and not a move.
    let drawn = data.order.clone();
    let before = data.order.clone();
    data.move_game(&drawn, 9, 0);
    data.move_game(&drawn, 0, 9);
    data.move_game(&drawn, 1, 1);
    assert_eq!(data.order, before);
}

#[test]
fn a_pack_is_filed_under_its_path_below_the_root() {
    let root = Path::new("packs");
    assert_eq!(
        key_of(root, Path::new("packs/charlie/283491/charlie.mod")),
        "charlie/283491/charlie.mod"
    );
    // Something that is not below the root still has to have a key: `--info` can be pointed at any `.mod`
    // anywhere, and a note about it needs somewhere to live.
    assert!(key_of(root, Path::new("D:/elsewhere/x.mod")).ends_with("x.mod"));
}

#[test]
fn an_empty_override_is_not_an_override() {
    let mut data = Data::default();
    data.set_name("a", "   ");
    assert_eq!(data.name_of("a"), None, "a blank name is no name of my own");
    data.set_icon("a", "");
    assert_eq!(data.icon_of("a"), None);

    data.set_screen("k", Some((240, 320)));
    assert_eq!(data.screen_of("k"), Some((240, 320)));
    data.set_screen("k", None);
    assert_eq!(data.screen_of("k"), None, "and it can be forgotten again");
}

#[test]
fn the_file_lives_beside_the_emulator_and_a_missing_one_is_not_a_problem() {
    let dir = scratch("load-none");
    let path = dir.join("manager.json");
    let (data, why) = Data::load(&path);
    assert_eq!(data, Data::default());
    assert_eq!(why, None, "a first run is not something to report");

    let mut saved = Data::default();
    saved.order = vec!["charlie".to_string()];
    saved.save(&path).expect("it can write");
    let (back, why) = Data::load(&path);
    assert_eq!(back, saved);
    assert_eq!(why, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_record_that_cannot_be_read_is_moved_aside_rather_than_lost() {
    let dir = scratch("load-bad");
    let path = dir.join("manager.json");
    std::fs::write(&path, "{ this is not json").unwrap();

    let (data, why) = Data::load(&path);
    assert_eq!(data, Data::default(), "the defaults are used");
    let why = why.expect("and it is said out loud");
    assert!(why.contains("manager.json"), "{why}");
    assert!(
        !path.exists(),
        "the unreadable file is out of the way: {why}"
    );
    let aside = dir.join("manager.bad");
    assert!(aside.exists(), "but kept, not deleted: {why}");
    assert_eq!(
        std::fs::read_to_string(&aside).unwrap(),
        "{ this is not json",
        "with its contents intact"
    );
    // And the next save starts from a clean note.
    Data::default().save(&path).expect("it can write again");
    assert!(Data::load(&path).0.order.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_sound_library_keeps_its_order_and_loses_what_is_deleted() {
    let mut data = Data::default();
    assert!(data.sounds.is_empty());
    assert_eq!(data.sound_in_use, None, "the opening state is silence");

    assert!(data.add_sound("C:/a.sf2"));
    assert_eq!(
        data.sound_in_use.as_deref(),
        Some("C:/a.sf2"),
        "the first one in is the one in use — a drop that makes no sound reads as a failed drop"
    );
    assert!(
        !data.add_sound("C:/a.sf2"),
        "the same path does not go in twice"
    );
    assert!(data.add_sound("D:/b.sf2"));
    assert_eq!(data.sounds, ["C:/a.sf2", "D:/b.sf2"]);
    assert_eq!(
        data.sound_in_use.as_deref(),
        Some("C:/a.sf2"),
        "adding another does not take over from the one in use"
    );

    data.move_sound(0, 1);
    assert_eq!(
        data.sounds,
        ["D:/b.sf2", "C:/a.sf2"],
        "the order is a person's"
    );
    data.move_sound(9, 0);
    data.move_sound(0, 9);
    data.move_sound(1, 1);
    assert_eq!(
        data.sounds,
        ["D:/b.sf2", "C:/a.sf2"],
        "out of range is not a move"
    );

    let mut gone = BTreeSet::new();
    gone.insert("C:/a.sf2".to_string());
    assert_eq!(data.drop_sounds(&gone), 1);
    assert_eq!(data.sounds, ["D:/b.sf2"]);
    assert_eq!(
        data.sound_in_use, None,
        "deleting the one in use goes back to silence rather than pointing at nothing"
    );

    // What is in the library and which one is in use both have to survive the file.
    data.use_sound(Some("D:/b.sf2"));
    let text = data.to_json();
    assert!(text.contains("\"sound_in_use\": \"D:/b.sf2\""), "{text}");
    assert_eq!(
        Data::from_json(&parse(&text).unwrap()).unwrap(),
        data,
        "round trip"
    );
}

#[test]
fn the_ui_language_is_written_and_read_back() {
    let mut data = Data::default();
    data.set_ui_lang("en");
    let text = data.to_json();
    assert!(text.contains("\"ui_lang\": \"en\""), "{text}");
    let back = Data::from_json(&parse(&text).expect("what it wrote, it can read")).expect("shape");
    assert_eq!(back.ui_lang.as_deref(), Some("en"));

    // An unknown tag is kept as text, so a later build that knows it can still report what the
    // person asked for; `UiLang::from_tag` falls back to the system language at the point of use.
    let unknown =
        Data::from_json(&parse("{\"ui_lang\": \"xx\"}").expect("document")).expect("shape");
    assert_eq!(unknown.ui_lang.as_deref(), Some("xx"));

    // A file written before this key existed means "follow the system".
    let older = Data::from_json(&parse("{\"order\": []}").expect("a document")).expect("shape");
    assert_eq!(older.ui_lang, None);
}

#[test]
fn the_font_choice_and_key_bindings_survive_the_file() {
    let font = "C:/fonts/QuoinHLE.ttf".to_string();
    let mut data = Data::default();
    data.set_font(Some(font.clone()));
    data.set_key("AVK_SOFT1", "Q");
    data.set_key("AVK_SELECT", "F1");

    let text = data.to_json();
    assert!(
        text.contains("\"font\": \"C:/fonts/QuoinHLE.ttf\""),
        "{text}"
    );
    let back = Data::from_json(&parse(&text).expect("what it wrote, it can read")).expect("shape");
    assert_eq!(back.font, Some(font));
    assert_eq!(back.key_binding("AVK_SOFT1"), Some("Q"));
    assert_eq!(back.key_binding("AVK_SELECT"), Some("F1"));
}
