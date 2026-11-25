// Moved out of `src/device_file.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and that is what
// lets these tests reach its private items with `use super::*`.

use super::*;
use crate::device::{DeviceFlags, DeviceProfile};
use crate::json::Value;

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

/// Every leaf of a JSON value, keyed by its dotted path, so a section cannot hide a field the writer
/// forgot: the round-trip fixture below has to differ from the built-in handset at every leaf.
fn flatten(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    if let Some(object) = value.as_object() {
        for (key, value) in object.iter() {
            let next = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            flatten(value, &next, out);
        }
    } else {
        out.push((path.to_string(), format!("{value:?}")));
    }
}

/// The backing fields are optional: a 1.0 file that predates them keeps the built-in handset.
#[test]
fn a_1_0_file_keeps_the_defaults_for_optional_backing_fields() {
    let p = profile(r#"{"version": "1.0", "screen": {"width": 176}, "abi": {"ktf": true}}"#)
        .expect("an older minor is still this layout");
    let standard = DeviceProfile::standard();
    assert_eq!(p.width, 176);
    assert!(p.ktf);
    assert_eq!(p.frame_bpp, standard.frame_bpp);
    assert_eq!(p.dib_pixel_offset, standard.dib_pixel_offset);
    assert_eq!(p.dib_depth, standard.dib_depth);
    assert_eq!(p.graphics_canvas_offset, standard.graphics_canvas_offset);
}

#[test]
fn the_frame_and_dib_backing_are_checked_before_a_run_uses_them() {
    for (text, why) in [
        (
            r#"{"version": "1.0", "screen": {"frame_bpp": 24}}"#,
            "24 bpp frame",
        ),
        (
            r#"{"version": "1.0", "abi": {"dib_depth": 24}}"#,
            "24 bpp DIB",
        ),
        (
            r#"{"version": "1.0", "screen": {"frame_bpp": 32}, "abi": {"dib_depth": 16}}"#,
            "a 32-bpp screen with 16-bpp DIBs",
        ),
        (
            r#"{"version": "1.0", "abi": {"dib_pixel_offset": 35}}"#,
            "pixels inside IDIB",
        ),
        (
            r#"{"version": "1.0", "abi": {"graphics_canvas_offset": 4}}"#,
            "the refcount word",
        ),
        (
            r#"{"version": "1.0", "abi": {"graphics_canvas_offset": 21}}"#,
            "an unaligned IGraphics field",
        ),
    ] {
        assert!(profile(text).is_err(), "{why}");
    }
    let wide = profile(
        r#"{"version": "1.0", "screen": {"frame_bpp": 32}, "abi": {"dib_depth": 32, "dib_pixel_offset": 128, "graphics_canvas_offset": 20}}"#,
    )
    .expect("a coherent 32-bpp device");
    assert_eq!(wide.frame_bpp, 32);
    assert_eq!(wide.dib_pixel_offset, 128);
    assert_eq!(wide.dib_depth, 32);
    assert_eq!(wide.graphics_canvas_offset, 20);
}

/// The round trip that makes the file worth having: everything the profile answers, out and back.
///
/// The input differs from [`DeviceProfile::standard`] in **every** field, so a reader that forgets a
/// key leaves its standard value in place and the leaf comparison says so.
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
    different.frame_bpp = 32;
    different.alt_width = 240;
    different.alt_height = 320;
    different.scroll_bar = 11;
    different.encoding = 1;
    different.text_bytes = true;
    different.font_normal = 20;
    different.font_large = 24;
    different.stack_reserve = 0x2000;
    different.module_base = 0x1000_0000;
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
    different.device_info_size = 60;
    different.ktf = true;
    different.dib_pixel_offset = 128;
    different.dib_depth = 32;
    different.graphics_canvas_offset = 24;
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
    different.subscriber_number = "08099998888".to_string();
    different.softkey_count = 1;
    different.manner_mode = crate::abi::const_value("AEE_MANNER_MODE_SILENT").unwrap_or(0);
    different.repeat_start = 300;
    different.repeat_rate = 40;
    different.key_repeat_start = 500;
    different.key_repeat_rate = 50;
    different.storage_total = 1024 * 1024 * 1024;
    different.storage_free = 512 * 1024 * 1024;
    different.fs_case_insensitive = true;
    different.overlay = crate::device::OverlayCaps {
        present: false,
        max_overlay: 2,
        max_underlay: 3,
        max_surface_per_layer: 5,
        max_total_surfaces: 7,
        pbuffer: false,
        pixmap: false,
    };
    different.gl = crate::device::GlCaps::none();
    different.gl.alpha_bits = 4;
    different.keys = vec![avk("AVK_SOFT1"), avk("AVK_SELECT")];
    different.ram_windows = vec![crate::device::RamWindow {
        base: 0x0180_0000,
        len: 0x0080_0000,
    }];
    different.heap_premap = 1024 * 1024;
    different.object_premap = 2 * 1024 * 1024;

    let document = to_json(&different);
    let standard = to_json(&DeviceProfile::standard());
    let mut doc_leaves = Vec::new();
    let mut std_leaves = Vec::new();
    flatten(&document, "", &mut doc_leaves);
    flatten(&standard, "", &mut std_leaves);
    let std_map: std::collections::HashMap<_, _> = std_leaves.into_iter().collect();
    for (path, value) in &doc_leaves {
        if path == "version" {
            continue;
        }
        let expected = std_map.get(path).unwrap_or_else(|| {
            panic!("the fixture writes {path}, which the standard handset does not")
        });
        assert_ne!(
            value, expected,
            "the fixture leaves {path} at the built-in value, so this test would not notice a reader that never looked at it"
        );
    }

    let back = from_json(&json::parse(&to_text(&different)).expect("parses")).expect("reads");
    assert_eq!(to_json(&back), document);
    assert_eq!(back.device_info_bytes(), different.device_info_bytes());
}

/// A version is required, and the reader only accepts the major it knows and minors it has seen.
#[test]
fn a_document_has_to_name_a_version_this_build_reads() {
    let err = profile("{}").expect_err("no version");
    assert!(err.contains("version"), "{err}");
    let err = profile(r#"{"version": "2.0"}"#).expect_err("another layout");
    assert!(err.contains("2.0"), "{err}");
    let err = profile(r#"{"version": "1.1"}"#).expect_err("a newer minor");
    assert!(err.contains("1.0"), "{err}");
    let err = profile(r#"{"version": "1"}"#).expect_err("not major.minor");
    assert!(err.contains("MAJOR.MINOR"), "{err}");
    assert!(profile(r#"{"version": "1.0"}"#).is_ok());
}

/// The file is a set of overrides: a document that names three things is a handset that differs in
/// three things, which is what makes it small enough to keep and read.
#[test]
fn a_short_file_changes_only_what_it_names() {
    let back = profile(
        r#"{"version": "1.0", "screen": {"width": 176, "height": 220}, "identity": {"model_name": "VX8300"}}"#,
    )
    .expect("reads");
    let standard = DeviceProfile::standard();
    assert_eq!((back.width, back.height), (176, 220));
    assert_eq!(back.model_name, "VX8300");
    assert_eq!(back.ram, standard.ram);
    assert_eq!(back.keys, standard.keys);
    assert_eq!(back.flags, standard.flags);
    // A document that only names the layout is the built-in handset.
    assert_eq!(
        to_json(&profile(r#"{"version": "1.0"}"#).expect("reads")),
        to_json(&standard)
    );
}

#[test]
fn a_flag_names_one_bit_and_leaves_its_neighbours() {
    let back =
        profile(r#"{"version": "1.0", "capabilities": {"flags": {"pen": false}}}"#).expect("reads");
    assert!(!back.flags.pen);
    assert!(back.flags.vibrator, "the standard vibrator is untouched");
    assert!(back.flags.ext_speaker);
    let err = profile(r#"{"version": "1.0", "capabilities": {"flags": {"pentab": true}}}"#)
        .expect_err("no such bit");
    assert!(err.contains("pentab"), "{err}");
}

#[test]
fn a_misspelled_key_is_named_rather_than_ignored() {
    let err = profile(r#"{"version": "1.0", "modle_name": "x"}"#).expect_err("unknown");
    assert!(err.contains("modle_name"), "{err}");
    let err = profile(r#"{"version": "1.0", "screen": {"widht": 176}}"#).expect_err("unknown");
    assert!(err.contains("widht"), "{err}");
    // JSON has no comments, so an underscore is the note-taking convention at every level.
    let ok = profile(
        r#"{"version": "1.0", "_why": "this pack is 176x220", "screen": {"_note": "width first", "width": 176}}"#,
    )
    .expect("notes are not keys");
    assert_eq!(ok.width, 176);
    assert_eq!(ok.height, DeviceProfile::standard().height);
}

#[test]
fn a_field_of_the_wrong_shape_says_which_field_and_what_it_found() {
    let err = profile(r#"{"version": "1.0", "screen": {"width": "240"}}"#)
        .expect_err("a string is not a width");
    assert!(
        err.contains("\"width\"") && err.contains("found a string"),
        "{err}"
    );
    let err =
        profile(r#"{"version": "1.0", "screen": {"width": 240.5}}"#).expect_err("a half pixel");
    assert!(err.contains("whole number"), "{err}");
    let err =
        profile(r#"{"version": "1.0", "screen": {"width": 70000}}"#).expect_err("does not fit");
    assert!(err.contains("16 bits"), "{err}");
    let err = profile(r#"{"version": "1.0", "system": {"ram": -1}}"#).expect_err("negative");
    assert!(err.contains("whole number"), "{err}");
    let err = profile("[1, 2]").expect_err("not an object");
    assert!(err.contains("an object"), "{err}");
}

/// Coherent-but-odd is fine; incoherent is not.
#[test]
fn a_device_that_cannot_be_one_is_refused() {
    for (text, why) in [
        (
            r#"{"version": "1.0", "screen": {"width": 0}}"#,
            "no columns",
        ),
        (r#"{"version": "1.0", "screen": {"height": 0}}"#, "no rows"),
        (
            r#"{"version": "1.0", "screen": {"color_depth": 0}}"#,
            "no bits",
        ),
        (
            r#"{"version": "1.0", "system": {"storage_total": 1024, "storage_free": 2048}}"#,
            "more free than total",
        ),
        (
            r#"{"version": "1.0", "identity": {"imei": "1234567812345678"}}"#,
            "sixteen digits",
        ),
        (
            r#"{"version": "1.0", "identity": {"imei": "12345abc"}}"#,
            "not digits",
        ),
        (
            r#"{"version": "1.0", "loader": {"stack_reserve": 0}}"#,
            "no loader frame",
        ),
        (
            r#"{"version": "1.0", "loader": {"stack_reserve": 1048576}}"#,
            "loader frame larger than the stack",
        ),
    ] {
        assert!(profile(text).is_err(), "{why}");
    }
    assert!(profile(r#"{"version": "1.0", "identity": {"imei": ""}}"#).is_ok());
}

#[test]
fn the_keypad_reads_by_name_and_by_code() {
    let by_name = profile(r#"{"version": "1.0", "input": {"keys": ["AVK_SOFT1", "AVK_SELECT"]}}"#)
        .expect("reads");
    let codes = format!(
        r#"{{"version": "1.0", "input": {{"keys": [{}, {}]}}}}"#,
        crate::keymap::avk("AVK_SOFT1"),
        crate::keymap::avk("AVK_SELECT")
    );
    assert_eq!(by_name.keys, profile(&codes).expect("reads").keys);
    assert!(
        to_text(&by_name).contains("\"AVK_SOFT1\""),
        "{}",
        to_text(&by_name)
    );

    let err =
        profile(r#"{"version": "1.0", "input": {"keys": ["AVK_ZORK"]}}"#).expect_err("no such key");
    assert!(err.contains("AVK_ZORK"), "{err}");
    assert!(profile(r#"{"version": "1.0", "input": {"keys": ["AEE_SUCCESS"]}}"#).is_err());
    assert!(profile(r#"{"version": "1.0", "input": {"keys": [70000]}}"#).is_err());
    assert!(profile(r#"{"version": "1.0", "input": {"keys": "AVK_SOFT1"}}"#).is_err());
}

/// An identity string with U+0000 would end before the guest sees the rest.
#[test]
fn an_identity_string_cannot_hide_half_of_itself_behind_a_nul() {
    for key in [
        "chip_id",
        "model_name",
        "manufacturer",
        "friendly_name",
        "firmware_version",
    ] {
        let text = format!(r#"{{"version": "1.0", "identity": {{"{key}": "Quoin\u0000HLE"}}}}"#);
        let err = profile(&text).expect_err("an embedded NUL");
        assert!(err.contains("U+0000"), "{key}: {err}");
        assert!(err.contains(key), "{key}: {err}");
    }
    let err = profile(r#"{"version": "1.0", "identity": {"imei": "123\u0000456"}}"#)
        .expect_err("a NUL in the imei");
    assert!(err.contains("imei"), "{err}");
}

#[test]
fn the_file_lists_the_sections_in_the_order_the_format_declares() {
    let value = to_json(&DeviceProfile::standard());
    let object = value.as_object().expect("an object");
    let keys: Vec<&str> = object.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        [
            "version",
            "screen",
            "text",
            "loader",
            "abi",
            "input",
            "identity",
            "system",
            "memory",
            "capabilities"
        ]
    );
    let input = object
        .get("input")
        .and_then(Value::as_object)
        .expect("the input section");
    let input_keys: Vec<&str> = input.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(input_keys.last(), Some(&"keys"), "{input_keys:?}");
}

#[test]
fn a_missing_file_is_written_and_then_used() {
    let dir = scratch("create");
    let path = dir.join("device.json");
    let loaded = load(&path).expect("creates the file");
    assert!(loaded.created);
    let text = std::fs::read_to_string(&path).expect("the file is there now");
    assert!(text.contains("\"version\": \"1.0\""), "{text}");
    assert!(text.contains("\"model_name\": \"QuoinHLE\""), "{text}");
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
    let first = std::fs::read_to_string(&path).expect("read");
    save(&path, &back).expect("writes again");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), first);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_keeps_the_notes_a_person_wrote() {
    let dir = scratch("notes");
    let path = dir.join("device.json");
    std::fs::write(
        &path,
        "{\n  \"_why\": \"176x220, the pack expects it\",\n  \"version\": \"1.0\",\n  \"screen\": {\"width\": 176}\n}\n",
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
    let note = text.find("_why").expect("the note is in the file");
    let version = text
        .find("\"version\"")
        .expect("the version is in the file");
    assert!(note < version, "the note belongs above the fields:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_refuses_to_overwrite_a_file_it_cannot_read() {
    let dir = scratch("corrupt");
    let path = dir.join("device.json");
    let broken = "{\n  \"_why\": \"a reason worth keeping\",\n  \"version\": \"1.0\",\n";
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

/// An `AECHAR` device item is in the handset's own units, like every other string the guest reads.
#[test]
fn a_device_name_is_written_in_the_units_the_handset_advertises() {
    use crate::abi::generated as g;
    use crate::mem::Mem;

    let mut profile = DeviceProfile::standard();
    profile.friendly_name = "テスト端末".to_string();
    let mem = Mem::new();

    let sjis = profile
        .device_item(&mem, g::AEE_DEVICEITEM_FRIENDLY_NAME, 0)
        .unwrap()
        .expect("the item is supported");
    assert_eq!(
        sjis,
        vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67, 0x92, 0x5b, 0x96, 0x96, 0, 0],
        "the Shift-JIS stream, terminated"
    );

    profile.encoding = g::AEE_ENC_UNICODE as u16;
    let unicode = profile
        .device_item(&mem, g::AEE_DEVICEITEM_FRIENDLY_NAME, 0)
        .unwrap()
        .expect("the item is supported");
    assert_eq!(
        unicode,
        vec![0xc6, 0x30, 0xb9, 0x30, 0xc8, 0x30, 0xef, 0x7a, 0x2b, 0x67, 0, 0],
        "and code points on a device that names them"
    );
}

#[test]
fn a_gl_preset_names_a_firmware_family() {
    let w64sa = DeviceProfile::standard()
        .with_gl_preset("w64sa")
        .expect("a known preset");
    assert_eq!(w64sa.gl, crate::device::GlCaps::w64sa());
    assert_eq!(w64sa.overlay, crate::device::OverlayCaps::w64sa());
    assert_eq!(w64sa.width, 240, "the rest of the handset is untouched");

    let w64s = DeviceProfile::standard()
        .with_gl_preset("W64S")
        .expect("preset names are case-insensitive");
    assert_eq!(w64s.gl, crate::device::GlCaps::none());
    assert_eq!(w64s.overlay, crate::device::OverlayCaps::none());

    let host = DeviceProfile::standard()
        .with_gl_preset("host")
        .expect("a known preset");
    assert_eq!(host.gl, crate::device::GlCaps::all());

    assert!(DeviceProfile::standard().with_gl_preset("psp").is_err());
}

/// The reference Carrier profile is the carrier's 2002 handset, not the standard one with a new screen: a
/// 128x128 panel, KSC5601 units, Korean resources, one softkey, no pen and no GL/overlay.
#[test]
fn the_shipped_ktf_profile_is_a_korean_feature_phone() {
    let p = profile(include_str!("../../assets/devices/ktf-kp2100.json"))
        .expect("the shipped profile loads");

    assert_eq!((p.width, p.height), (128, 128));
    assert_eq!(p.color_depth, 16);
    assert_eq!(p.frame_bpp, 16, "the KP2100's screen is RGB565");
    assert_eq!(
        p.dib_pixel_offset, 43,
        "KTF BREW 1.2.3 computes DIB pixels at object+0x2B"
    );
    assert_eq!(
        p.dib_depth, 32,
        "DIB_NewEx takes a 32-bpp pixel size even on a 16-bpp screen"
    );
    assert_eq!(p.graphics_canvas_offset, 20, "QGraphics::m_pbmDst");
    assert_eq!(p.encoding, crate::abi::generated::AEE_ENC_KSC5601 as u16);
    assert_eq!(p.lang, crate::abi::generated::LNG_KOREAN);
    assert_eq!(p.model_name, "LG-KP2100");
    assert_eq!(p.firmware_version, "BREW/1.2.3");
    assert!(
        p.text_bytes,
        "the 1.2.3 DrawText slot takes the device's own 8-bit string"
    );
    assert_eq!(
        p.device_info_size, 60,
        "BREW 1.2.3's AEEDeviceInfo is 60 bytes"
    );
    assert!(
        p.declared.device_info_size,
        "the file said 60, so a run must not replace it with its generation's default"
    );
    let bytes = p.device_info_bytes();
    assert_eq!(bytes.len(), 60);
    let wso = crate::abi::offset_of("AEEDeviceInfo", "wStructSize").unwrap() as usize;
    assert_eq!(
        u16::from_le_bytes([bytes[wso], bytes[wso + 1]]),
        60,
        "the struct size the pack reads back is its own, not the 4.0.x one"
    );
    assert!(p.ktf);
    assert_eq!(p.ram, 256 * 1024, "the KTF guide's per-app heap ceiling");
    assert_eq!(p.softkey_count, 1);
    assert!(p.flags.midi && !p.flags.cmx && !p.flags.pen);
    assert_eq!(p.gl, crate::device::GlCaps::none());
    assert_eq!(p.overlay, crate::device::OverlayCaps::none());
    assert_eq!(
        p.stack_reserve, 4096,
        "the KTF startup's 0x400-byte load context needs room below the stack top"
    );
    assert!(
        p.fs_case_insensitive,
        "KTF packs ask for `Img.bar` while the shipped file is `img.bar`"
    );
}

/// A file that never mentions the structure size must leave the field to the generation, not to the
/// built-in 64: the flag is what tells the two cases apart, and a defaulted `true` would make every
/// handset look like it had spoken.
#[test]
fn a_file_that_says_nothing_about_the_struct_size_leaves_it_to_the_generation() {
    let text = r#"{"version":"1.0"}"#;
    let profile = from_json(&crate::json::parse(text).unwrap()).unwrap();
    assert!(!profile.declared.device_info_size);
    assert!(
        profile.declared.named().is_empty(),
        "a file with no keys claimed nothing at all"
    );

    let said = r#"{"version":"1.0","abi":{"device_info_size":60}}"#;
    let profile = from_json(&crate::json::parse(said).unwrap()).unwrap();
    assert!(profile.declared.device_info_size);
    assert_eq!(profile.device_info_size, 60);
}

/// Each text key claims only itself. A file that names the character set but not the faces has to
/// leave the faces to the carrier — that is the whole reason the bits are per field and not one
/// "the file said something about text" flag.
#[test]
fn each_text_key_claims_only_itself() {
    let text = r#"{"version":"1.0","text":{"encoding":254,"font_large":24}}"#;
    let p = from_json(&crate::json::parse(text).unwrap()).unwrap();
    assert_eq!(p.declared.named(), ["encoding", "font_large"]);
    assert_eq!(p.encoding, 254);
    assert_eq!(p.font_large, 24);
    assert!(
        !p.declared.font_normal && !p.declared.text_bytes,
        "the keys the file did not name stay open to the generation and carrier"
    );
}

/// The windows a pack's saves point into can be named per handset, and a file that names them is
/// saying it knows better than the carrier — including when the answer is "this handset has none".
#[test]
fn a_file_can_name_the_hands_memory_and_says_so_in_the_round_trip() {
    let dir = scratch("windows");
    let path = dir.join("device.json");
    let text = r#"{"version":"1.0","memory":{"windows":[{"base":25165824,"len":8388608}]}}"#;
    let p = profile(text).expect("a handset with one window loads");
    assert_eq!(
        p.ram_windows,
        [crate::device::RamWindow {
            base: 0x0180_0000,
            len: 0x0080_0000
        }]
    );
    assert!(p.declared.ram_windows);

    // What this host writes back is the statement it just read: the windows survive a save, and
    // they survive it as the handset's own rather than as something to be refilled from a carrier.
    save(&path, &p).expect("writes");
    let back = load(&path).expect("reads").profile;
    assert_eq!(back.ram_windows, p.ram_windows);
    assert!(back.declared.ram_windows);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bringing heap pages in early is a handset's statement, not a constant of the host: a file can
/// say how many, including none, and the save has to keep saying it.
#[test]
fn a_file_can_name_the_heap_pages_that_are_ram_from_the_start() {
    let dir = scratch("heap-premap");
    let path = dir.join("device.json");
    let p = profile(r#"{"version":"1.0","system":{"heap_premap":1048576}}"#)
        .expect("a handset that names its own heap RAM loads");
    assert_eq!(p.heap_premap, 0x10_0000);
    assert!(p.declared.heap_premap);
    assert_eq!(p.declared.named(), ["heap_premap"]);

    save(&path, &p).expect("writes");
    let back = load(&path).expect("reads").profile;
    assert_eq!(back.heap_premap, p.heap_premap);
    assert!(back.declared.heap_premap);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The host-object region is RAM on a handset too, and a file that says how much has to keep saying
/// it. Zero stays the host's lazy per-object mapping.
#[test]
fn a_file_can_name_the_host_object_region_that_is_ram_from_the_start() {
    let dir = scratch("object-premap");
    let path = dir.join("device.json");
    let p = profile(r#"{"version":"1.0","system":{"object_premap":1048576}}"#)
        .expect("a handset that names its own object RAM loads");
    assert_eq!(p.object_premap, 0x10_0000);
    assert!(p.declared.object_premap);
    assert_eq!(p.declared.named(), ["object_premap"]);

    save(&path, &p).expect("writes");
    let back = load(&path).expect("reads").profile;
    assert_eq!(back.object_premap, p.object_premap);
    assert!(back.declared.object_premap);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A window inside the guest heap would be handed out by the allocator as well, and the two would
/// write over each other with nothing to say which won. A file that asks for one is a file mistake.
#[test]
fn a_window_on_top_of_the_guest_heap_is_refused() {
    let text = r#"{"version":"1.0","memory":{"windows":[{"base":536870912,"len":4096}]}}"#;
    let err = profile(text).unwrap_err();
    assert!(err.contains("overlaps the guest heap"), "{err}");

    let empty = r#"{"version":"1.0","memory":{"windows":[{"base":25165824,"len":0}]}}"#;
    let err = profile(empty).unwrap_err();
    assert!(err.contains("no length"), "{err}");

    let half = r#"{"version":"1.0","memory":{"windows":[{"base":25165824}]}}"#;
    let err = profile(half).unwrap_err();
    assert!(err.contains("has no \"len\""), "{err}");

    let stray = r#"{"version":"1.0","memory":{"windows":[{"base":1,"len":4096,"note":1}]}}"#;
    let err = profile(stray).unwrap_err();
    assert!(err.contains("unknown key"), "{err}");
}
