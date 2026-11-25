use super::*;

#[test]
fn the_aee_numbers_map_to_the_encodings_they_name() {
    assert_eq!(TextEncoding::of(g::AEE_ENC_UNICODE), TextEncoding::Unicode);
    assert_eq!(TextEncoding::of(g::AEE_ENC_UTF8), TextEncoding::Utf8);
    assert_eq!(TextEncoding::of(g::AEE_ENC_S_JIS), TextEncoding::Sjis);
    assert_eq!(TextEncoding::of(g::AEE_ENC_EUC_CN), TextEncoding::Gbk);
    assert_eq!(TextEncoding::of(g::AEE_ENC_KSC5601), TextEncoding::EucKr);
    assert_eq!(TextEncoding::of(g::AEE_ENC_ISOLATIN1), TextEncoding::Latin1);
    assert_eq!(TextEncoding::of(g::AEE_ENC_OCTET), TextEncoding::Latin1);
    assert_eq!(
        TextEncoding::of(g::AEE_ENC_LATIN_HEBREW),
        TextEncoding::Hebrew
    );
    assert_eq!(TextEncoding::of(0x1234), TextEncoding::Unicode);
}

#[test]
fn the_byte_stream_table_is_the_one_the_html_viewer_already_parsed_with() {
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_S_JIS),
        Some(encoding_rs::SHIFT_JIS)
    );
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_EUC_CN),
        Some(encoding_rs::GBK)
    );
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_KSC5601),
        Some(encoding_rs::EUC_KR)
    );
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_UTF8),
        Some(encoding_rs::UTF_8)
    );
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_ISOLATIN1),
        Some(encoding_rs::WINDOWS_1252)
    );
    assert_eq!(
        TextEncoding::stream_codec(g::AEE_ENC_LATIN_HEBREW),
        Some(encoding_rs::WINDOWS_1255)
    );
    assert_eq!(TextEncoding::stream_codec(g::AEE_ENC_UNICODE), None);
}

#[test]
fn a_shift_jis_unit_reads_as_the_character_the_handset_stored() {
    // The word's bytes are the stream in guest-memory order: 82 cd is は, so the word is 0xcd82.
    let units = [0xcd82, 0xb682, 0xdf82, 0xa982, 0xe782];
    assert_eq!(TextEncoding::Sjis.units_to_string(&units), "はじめから");
    assert_eq!(
        TextEncoding::Sjis.units_to_string(&[0xc282, 0xc382, 0xab82, 0xa982, 0xe782]),
        "つづきから"
    );
    assert_eq!(
        TextEncoding::Sjis.units_to_string(&[0x4983, 0x7683, 0x5683, 0x8783, 0x9383]),
        "オプション"
    );
    assert_eq!(
        TextEncoding::Sjis
            .units_to_string(&[0x4783, 0x4f83, 0x5b83, 0x5483, 0x4383, 0x6783, 0xd682,]),
        "エグゼサイトへ"
    );
    assert_eq!(TextEncoding::Sjis.char_to_unit('は'), Some(0xcd82));
    assert_eq!(TextEncoding::Sjis.char_to_unit('テ'), Some(0x6583));
    assert_eq!(
        TextEncoding::Sjis.string_to_units("はじめから"),
        vec![0xcd82, 0xb682, 0xdf82, 0xa982, 0xe782]
    );
}

#[test]
fn a_unit_never_decodes_to_more_or_fewer_than_one_character() {
    // A lone lead byte, a truncated pair and a pair that names nothing all answer one U+FFFD.
    for units in [vec![0x0081], vec![0x81], vec![0x8120], vec![0xffff]] {
        let text = TextEncoding::Sjis.units_to_string(&units);
        assert_eq!(
            text.chars().count(),
            units.len(),
            "units {units:04x?} produced {text:?}"
        );
        assert!(text.chars().all(|ch| ch == '\u{fffd}'), "{text:?}");
    }
    // Two half-width katakana bytes are two units, and they stay two characters.
    let text = TextEncoding::Sjis.units_to_string(&[0xb0, 0xa1]);
    assert_eq!(text.chars().count(), 2);
    assert_eq!(text, "ｰ｡");
}

#[test]
fn an_unmappable_character_is_refused_rather_than_becoming_a_character_reference() {
    // CP932 has no 开; encoding_rs would otherwise write `&#24320;` for it.
    assert_eq!(TextEncoding::Sjis.char_to_unit('开'), None);
    assert_eq!(TextEncoding::Sjis.string_to_units("开"), vec![0xfffd]);
}

#[test]
fn unicode_units_are_the_code_points_and_round_trip_unchanged() {
    let text = "A中は";
    let units = TextEncoding::Unicode.string_to_units(text);
    assert_eq!(units, vec![0x41, 0x4e2d, 0x306f]);
    assert_eq!(TextEncoding::Unicode.units_to_string(&units), text);
    assert_eq!(TextEncoding::Unicode.char_to_unit('A'), Some(0x41));
    // A surrogate is not a character, and a pair is not two units.
    assert_eq!(TextEncoding::Unicode.unit_to_char(0xd800), '\u{fffd}');
    assert_eq!(TextEncoding::Unicode.string_to_units("😀"), vec![0xfffd]);
}

#[test]
fn utf8_is_the_byte_side_of_a_unicode_device() {
    assert_eq!(TextEncoding::Utf8.char_to_unit('中'), Some(0x4e2d));
    assert_eq!(
        TextEncoding::Utf8.bytes_to_units("中".as_bytes()),
        vec![0x4e2d]
    );
    assert_eq!(
        TextEncoding::Utf8.units_to_bytes(&[0x4e2d]),
        "中".as_bytes()
    );
    assert_eq!(TextEncoding::Utf8.units_to_string(&[0x4e2d]), "中");
}

#[test]
fn the_other_double_byte_encodings_pack_their_bytes_the_same_way() {
    assert_eq!(TextEncoding::Gbk.units_to_string(&[0xd0d6]), "中");
    assert_eq!(TextEncoding::Gbk.char_to_unit('中'), Some(0xd0d6));
    assert_eq!(TextEncoding::EucKr.units_to_string(&[0xa1b0]), "가");
    assert_eq!(TextEncoding::EucKr.char_to_unit('가'), Some(0xa1b0));
}

#[test]
fn a_single_byte_device_widens_one_byte_per_unit() {
    assert_eq!(TextEncoding::Latin1.unit_to_char(0xe9), 'é');
    assert_eq!(TextEncoding::Latin1.char_to_unit('é'), Some(0xe9));
    assert_eq!(
        TextEncoding::Latin1.bytes_to_units(b"A\xe9"),
        vec![0x41, 0xe9]
    );
    assert_eq!(TextEncoding::Latin1.units_to_bytes(&[0x41, 0xe9]), b"A\xe9");
}

#[test]
fn expand_and_compress_are_inverse_for_a_well_formed_shift_jis_run() {
    let bytes = b"\x83\x65Z";
    let units = TextEncoding::Sjis.bytes_to_units(bytes);
    assert_eq!(units, vec![0x6583, 0x005a]);
    assert_eq!(TextEncoding::Sjis.units_to_bytes(&units), bytes);
}

#[test]
fn a_byte_stream_is_read_with_the_device_encoding() {
    assert_eq!(stream_text(g::AEE_ENC_S_JIS, b"\x82\xcd"), "は");
    assert_eq!(stream_text(g::AEE_ENC_UNICODE, b"A\xe9"), "Aé");
}

/// The boundary this module owns is a rule, not a convention: outside `src/encoding.rs` and the
/// tests, no module may turn guest `AECHAR` units into text with `String::from_utf16_lossy`. A
/// reintroduced one compiles and looks right on an English screen, which is exactly why a scan is
/// worth having.
#[test]
fn only_the_encoding_module_reads_guest_units_as_utf16() {
    use std::path::{Path, PathBuf};

    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    collect(Path::new("src"), &mut files);
    let offenders: Vec<String> = files
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name != "encoding.rs"))
        .filter(|path| !path.components().any(|part| part.as_os_str() == "test"))
        .filter(|path| {
            std::fs::read_to_string(path)
                .map(|text| text.contains("from_utf16_lossy("))
                .unwrap_or(false)
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "guest units are read as UTF-16 in {offenders:?}; convert through TextEncoding instead"
    );
}
