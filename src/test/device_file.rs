// Moved out of `src/device_file.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and that is what
// lets these tests reach its private items with `use super::*`.

use super::*;
use crate::device::{DeviceFlags, DeviceProfile};

fn profile(text: &str) -> Result<DeviceProfile, String> {
    from_json(&json::parse(text).expect("the test's own text is JSON"))
}

/// A scratch directory, unique per test, removed first so a previous aborted run cannot be read.
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("quoin-device-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp directory");
    dir
}

/// The round trip that makes the file worth having: everything the profile answers, out and back.
///
/// The input has to be a handset that differs from [`DeviceProfile::standard`] in **every** field,
/// because that is what `from_json` starts from: a field the reader forgot leaves its standard value
/// in place, which is invisible if the document was the standard handset to begin with. The loop
/// below enforces that, so a field added to the profile later fails here until it is given a
/// non-default value here too — and once it is, the round trip is what proves it is read.
#[test]
fn a_profile_goes_out_and_comes_back_whole() {
    fn avk(name: &str) -> u16 {
        crate::abi::const_value(name).unwrap_or_else(|| panic!("{name} is in the ABI tables"))
            as u16
    }

    let mut different = DeviceProfile::standard();
    different.width = 176;
    different.height = 220;
    different.color_depth = 18;
    different.alt_width = 240;
    different.alt_height = 320;
    different.scroll_bar = 11;
    different.encoding = 1;
    different.menu_text_scroll = 40;
    different.menu_image_delay = 3000;
    different.ram = 32 * 1024 * 1024;
    different.prompt_props = 0x21;
    different.key_close_app = avk("AVK_CLR");
    different.key_close_all_apps = avk("AVK_SOFT2");
    different.lang = 2;
    different.net_linger = 5000;
    different.sleep_defer = 7000;
    different.max_path = 200;
    different.platform_id = 0x0102;
    // Every one of the nine bits the other way round from the standard handset, so a flag that is
    // not read comes back as the standard one and this test says so.
    different.flags = DeviceFlags {
        alt_display: true,
        flip: true,
        vibrator: false,
        ext_speaker: false,
        voice_recognition: true,
        position_location: true,
        midi: true,
        cmx: true,
        pen: false,
    };
    different.chip_id = "MSM6280".to_string();
    different.model_name = "VX8300".to_string();
    different.manufacturer = "LG Electronics".to_string();
    different.friendly_name = "Chocolate".to_string();
    different.firmware_version = "2.1".to_string();
    different.carrier_id = 7;
    different.esn = 0x1122_3344;
    different.imei = "000000000000000".to_string();
    different.softkey_count = 1;
    different.manner_mode = crate::abi::const_value("AEE_MANNER_MODE_SILENT").unwrap_or(0);
    different.repeat_start = 300;
    different.repeat_rate = 40;
    different.key_repeat_start = 500;
    different.key_repeat_rate = 50;
    different.storage_total = 1024 * 1024 * 1024;
    different.storage_free = 512 * 1024 * 1024;
    different.keys = vec![avk("AVK_SOFT1"), avk("AVK_SELECT")];

    let document = to_json(&different);
    let standard = to_json(&DeviceProfile::standard());
    for (key, value) in document.as_object().expect("an object").iter() {
        if key == "schema" {
            // Not a field: it names the shape of the document, and both sides write the same one.
            continue;
        }
        assert_ne!(
            Some(value),
            standard.as_object().expect("an object").get(key),
            "the fixture leaves \"{key}\" at the built-in value, so this test would not notice a \
             reader that never looked at it"
        );
    }

    let back = from_json(&json::parse(&to_text(&different)).expect("parses")).expect("reads");
    assert_eq!(to_json(&back), document);
    assert_eq!(back.device_info_bytes(), different.device_info_bytes());
}

/// A document that would be swallowed by its own answers is refused instead of half-answered: an
/// identity string is handed to the guest as a terminated buffer, so a U+0000 in it means the file
/// says one thing and the pack reads a shorter one.
#[test]
fn an_identity_string_cannot_hide_half_of_itself_behind_a_nul() {
    for key in [
        "chip_id",
        "model_name",
        "manufacturer",
        "friendly_name",
        "firmware_version",
    ] {
        let text = format!("{{\"{key}\": \"Quoin\\u0000HLE\"}}");
        let err = profile(&text).expect_err("an embedded NUL");
        assert!(err.contains("U+0000"), "{key}: {err}");
        assert!(err.contains(key), "{key}: {err}");
    }
    // The IMEI is digits only, which already excludes it, and that refusal names the field.
    let err = profile(r#"{"imei": "123\u0000456"}"#).expect_err("a NUL in the imei");
    assert!(err.contains("\"imei\""), "{err}");
}

/// The file is a set of overrides: a document that names three things is a handset that differs in
/// three things, which is what makes it small enough to keep and read.
#[test]
fn a_short_file_changes_only_what_it_names() {
    let back = profile(r#"{"width": 176, "height": 220, "model_name": "VX8300"}"#).expect("reads");
    let standard = DeviceProfile::standard();
    assert_eq!((back.width, back.height), (176, 220));
    assert_eq!(back.model_name, "VX8300");
    assert_eq!(back.ram, standard.ram);
    assert_eq!(back.keys, standard.keys);
    assert_eq!(back.flags, standard.flags);
    // And an empty document is the built-in handset, not an error: `{}` is a valid answer to "which
    // phone", it is just the one this host already knew.
    assert_eq!(to_json(&profile("{}").expect("reads")), to_json(&standard));
}

#[test]
fn a_flag_names_one_bit_and_leaves_its_neighbours() {
    let back = profile(r#"{"flags": {"pen": false}}"#).expect("reads");
    assert!(!back.flags.pen);
    assert!(
        back.flags.vibrator,
        "the standard handset's vibrator is not this file's business"
    );
    assert!(back.flags.ext_speaker);
    let err = profile(r#"{"flags": {"pentab": true}}"#).expect_err("no such bit");
    assert!(err.contains("\"pentab\""), "{err}");
}

/// A misspelled field that is quietly ignored is a run reporting this host's default while its file
/// says otherwise — the failure shape this repository refuses everywhere else.
#[test]
fn an_unknown_key_is_named_rather_than_ignored() {
    let err = profile(r#"{"modle_name": "x"}"#).expect_err("unknown");
    assert!(err.contains("\"modle_name\""), "{err}");
    // JSON has no comments, so an underscore is the note-taking convention and it is honoured.
    let ok =
        profile(r#"{"_why": "this pack is 176x220", "width": 176}"#).expect("a note is not a key");
    assert_eq!(ok.width, 176);
    assert_eq!(ok.height, DeviceProfile::standard().height);
}

#[test]
fn a_field_of_the_wrong_shape_says_which_field_and_what_it_found() {
    let err = profile(r#"{"width": "240"}"#).expect_err("a string is not a width");
    assert!(
        err.contains("\"width\"") && err.contains("found a string"),
        "{err}"
    );
    let err = profile(r#"{"width": 240.5}"#).expect_err("a half pixel");
    assert!(err.contains("whole number"), "{err}");
    let err = profile(r#"{"width": 70000}"#).expect_err("does not fit");
    assert!(err.contains("16 bits"), "{err}");
    let err = profile(r#"{"ram": -1}"#).expect_err("negative");
    assert!(err.contains("whole number"), "{err}");
    let err = profile("[1, 2]").expect_err("not an object");
    assert!(err.contains("an object"), "{err}");
}

/// Coherent-but-odd is fine; incoherent is not. Each of these is arithmetic the host would have to
/// answer for, not a choice about a handset.
#[test]
fn a_device_that_cannot_be_one_is_refused() {
    for (text, why) in [
        (r#"{"width": 0}"#, "a screen with no columns"),
        (r#"{"height": 0}"#, "a screen with no rows"),
        (r#"{"color_depth": 0}"#, "no bits per pixel"),
        (
            r#"{"storage_total": 1024, "storage_free": 2048}"#,
            "more free than total",
        ),
        (r#"{"imei": "1234567812345678"}"#, "sixteen digits"),
        (r#"{"imei": "12345abc"}"#, "not digits"),
    ] {
        assert!(profile(text).is_err(), "{why}");
    }
    // Empty is not incoherent: a handset that does not report an IMEI is a handset.
    assert!(profile(r#"{"imei": ""}"#).is_ok());
}

#[test]
fn the_keypad_reads_by_name_and_by_code() {
    let by_name = profile(r#"{"keys": ["AVK_SOFT1", "AVK_SELECT"]}"#).expect("reads");
    let codes = format!(
        r#"{{"keys": [{}, {}]}}"#,
        crate::keymap::avk("AVK_SOFT1"),
        crate::keymap::avk("AVK_SELECT")
    );
    assert_eq!(by_name.keys, profile(&codes).expect("reads").keys);
    // Written back, a key that has a name is that name: the file reads as a keypad.
    assert!(
        to_text(&by_name).contains("\"AVK_SOFT1\""),
        "{}",
        to_text(&by_name)
    );

    let err = profile(r#"{"keys": ["AVK_ZORK"]}"#).expect_err("no such key");
    assert!(err.contains("AVK_ZORK"), "{err}");
    // A real constant that is not a key is still not a key: `ABI` has nine thousand names.
    assert!(profile(r#"{"keys": ["AEE_SUCCESS"]}"#).is_err());
    assert!(profile(r#"{"keys": [70000]}"#).is_err());
    assert!(profile(r#"{"keys": "AVK_SOFT1"}"#).is_err());
}

#[test]
fn a_schema_from_the_future_is_refused_here() {
    let err = profile(r#"{"schema": 2}"#).expect_err("newer");
    assert!(err.contains("schema"), "{err}");
    assert!(profile(r#"{"schema": 1}"#).is_ok());
}

/// The listing, in the order the struct declares its fields — which is what makes the file readable
/// next to `device.rs`.
#[test]
fn the_file_lists_the_fields_in_the_order_the_struct_declares_them() {
    let value = to_json(&DeviceProfile::standard());
    let keys: Vec<&str> = value
        .as_object()
        .expect("an object")
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(keys[0], "schema");
    assert_eq!(keys[1], "width");
    assert_eq!(keys[2], "height");
    assert_eq!(keys[keys.len() - 1], "keys");
    // Thirty-six fields plus the schema word. A field added to `DeviceProfile` has to arrive here
    // with a key of its own, and this count is what asks for one.
    assert_eq!(keys.len(), 37);
}

#[test]
fn a_missing_file_is_written_and_then_used() {
    let dir = scratch("create");
    let path = dir.join("device.json");
    let loaded = load(&path).expect("creates the file");
    assert!(loaded.created);
    let text = std::fs::read_to_string(&path).expect("the file is there now");
    assert!(text.contains("\"model_name\": \"QuoinHLE\""), "{text}");
    // A second load reads what the first wrote rather than writing again.
    let again = load(&path).expect("reads");
    assert!(!again.created);
    assert_eq!(
        again.profile.device_info_bytes(),
        loaded.profile.device_info_bytes()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_saved_profile_is_the_one_that_loads_back() {
    let dir = scratch("save");
    let path = dir.join("device.json");
    let mut profile = DeviceProfile::standard();
    profile.width = 176;
    profile.height = 220;
    profile.model_name = "VX8300".to_string();
    profile.keys.truncate(4);
    save(&path, &profile).expect("writes");
    let back = load(&path).expect("reads").profile;
    assert_eq!(to_json(&back), to_json(&profile));
    // Saving twice is the same file: a re-save is how a person keeps a file they edited, and it must
    // not shuffle keys or reformat numbers under them.
    let first = std::fs::read_to_string(&path).expect("read");
    save(&path, &back).expect("writes again");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), first);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `_`-prefixed notes are the one part of the file this module does not own, so a re-save keeps them
/// and puts them back at the top, where the reason a file exists belongs.
#[test]
fn a_save_keeps_the_notes_a_person_wrote() {
    let dir = scratch("notes");
    let path = dir.join("device.json");
    std::fs::write(
        &path,
        "{\n  \"_why\": \"176x220, the pack expects it\",\n  \"width\": 176\n}\n",
    )
    .expect("writes");
    let loaded = load(&path).expect("reads");
    save(&path, &loaded.profile).expect("writes");
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(
        text.contains("\"_why\": \"176x220, the pack expects it\""),
        "{text}"
    );
    assert!(text.contains("\"width\": 176"), "{text}");
    // `expect`, not `assert!(a < b)`: an `Option` comparison is `None < Some(_)`, so the plain form
    // would pass on a file where `_why` had been dropped entirely.
    let note = text.find("_why").expect("the note is in the file");
    let schema = text
        .find("\"schema\"")
        .expect("the schema key is in the file");
    assert!(note < schema, "the note belongs above the fields:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file the writer cannot read is refused rather than overwritten: "no notes found" and "this file
/// did not parse" are the same thing to a writer that shrugs, and only one of them is true.
#[test]
fn a_save_refuses_to_overwrite_a_file_it_cannot_read() {
    let dir = scratch("corrupt");
    let path = dir.join("device.json");
    let broken = "{\n  \"_why\": \"a reason worth keeping\",\n  \"width\": 240,\n";
    std::fs::write(&path, broken).expect("writes");
    let err = save(&path, &DeviceProfile::standard()).expect_err("refuses");
    assert!(err.contains("notes"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("still there"),
        broken,
        "the file it refused to read is the file that has to survive"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The write goes through a temporary file, so a run that dies mid-write leaves the previous file
/// whole instead of half of the new one.
#[test]
fn a_save_leaves_no_temporary_file_behind() {
    let dir = scratch("temp");
    let path = dir.join("device.json");
    save(&path, &DeviceProfile::standard()).expect("writes");
    let names: Vec<String> = std::fs::read_dir(&dir)
        .expect("lists")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["device.json"], "{names:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A broken file has to say which file, which line, and which column — the file is edited by a
/// person, and "expected `,` or `}`" without a position is a search.
#[test]
fn a_broken_file_names_the_path_and_the_position() {
    let dir = scratch("broken");
    let path = dir.join("device.json");
    std::fs::write(&path, "{\n  \"width\": 240\n  \"height\": 320\n}\n").expect("writes");
    let err = load(&path).expect_err("no comma");
    let shown = path.display().to_string();
    assert!(err.contains(&shown), "{err}");
    assert!(err.contains("line 3"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
