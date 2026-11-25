//! The character encoding a handset puts in its `AECHAR` units, and the three projections every
//! text path needs.
//!
//! Three encodings are easy to mistake for one:
//!
//! - the **byte stream** a pack's `char *` points at;
//! - the **character units** its `AECHAR` buffer holds, named by `AEEDeviceInfo::wEncoding`;
//! - the **keys** a bitmap face's range table is addressed by.
//!
//! A Japanese handset answers `AEE_ENC_S_JIS` and its `AECHAR` units carry the two Shift-JIS bytes
//! in the order they sit in guest memory. The word is read little-endian on the guest's ARM, so the
//! unit's **low** byte is the first byte of the stream: `82 cd` is `は`, and the unit's value is
//! `0xcd82` — not the numeric Shift-JIS code `0x82cd`. The same rule holds for the other
//! double-byte encodings. This module is the one place that turns such a unit into a character and
//! back; every other module keeps text in Unicode and calls in here only at the guest boundary.
//!
//! ## The one invariant
//!
//! **One unit decodes to exactly one character.** A unit whose bytes name nothing, or name more
//! than one character when read under the wrong encoding, answers `U+FFFD` — never zero characters
//! and never two. Layout measures a character at a time, so this is what keeps a run of text the
//! same number of cells whichever interpretation the device advertises; wrong text shows as
//! replacement characters rather than as a reflowed page.

use crate::abi::generated as g;

const REPLACEMENT: char = '\u{FFFD}';

/// `AEEDeviceInfo::wEncoding` as the encoding of an `AECHAR` unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextEncoding {
    /// A wide unit is a Unicode code point (BMP).
    #[default]
    Unicode,
    /// Wide units are UTF-16, while a `char *` is UTF-8.
    Utf8,
    /// Shift-JIS: one or two bytes packed into a unit.
    Sjis,
    /// KSC5601 / EUC-KR: one or two bytes packed into a unit.
    EucKr,
    /// GB2312 / GBK (`AEE_ENC_EUC_CN`): one or two bytes packed into a unit.
    Gbk,
    /// One byte per unit, read through windows-1252.
    Latin1,
    /// One byte per unit, read through windows-1255.
    Hebrew,
}

impl TextEncoding {
    /// The encoding `wEncoding` names.
    ///
    /// `0xfe` is both `AEE_ENC_KSC5601` and `AEE_ENC_UNICODE2` in the reference headers, so the two
    /// cannot be told apart from the number alone. Korean is chosen because the byte-stream table
    /// this host already reads does the same; no handset in the sample set reports `UNICODE2`.
    pub const fn of(aee: u32) -> Self {
        match aee {
            g::AEE_ENC_UNICODE => TextEncoding::Unicode,
            g::AEE_ENC_UTF8 => TextEncoding::Utf8,
            g::AEE_ENC_S_JIS => TextEncoding::Sjis,
            g::AEE_ENC_KSC5601 => TextEncoding::EucKr,
            g::AEE_ENC_EUC_CN => TextEncoding::Gbk,
            g::AEE_ENC_ISOLATIN1 | g::AEE_ENC_OCTET => TextEncoding::Latin1,
            g::AEE_ENC_LATIN_HEBREW => TextEncoding::Hebrew,
            _ => TextEncoding::Unicode,
        }
    }

    /// The codec for an 8-bit stream (`char *`, an HTML document, a resource string body).
    ///
    /// This is exactly the table `IHtmlViewer` already parsed with. `UNICODE` answers `None`
    /// because it names no 8-bit encoding; `UTF8` names one.
    pub fn stream_codec(aee: u32) -> Option<&'static encoding_rs::Encoding> {
        match aee {
            g::AEE_ENC_UTF8 => Some(encoding_rs::UTF_8),
            g::AEE_ENC_ISOLATIN1 | g::AEE_ENC_OCTET => Some(encoding_rs::WINDOWS_1252),
            g::AEE_ENC_LATIN_HEBREW => Some(encoding_rs::WINDOWS_1255),
            g::AEE_ENC_EUC_CN => Some(encoding_rs::GBK),
            g::AEE_ENC_S_JIS => Some(encoding_rs::SHIFT_JIS),
            g::AEE_ENC_KSC5601 => Some(encoding_rs::EUC_KR),
            _ => None,
        }
    }

    /// The codec a unit's bytes are read with, for the encodings that have one.
    fn codec(self) -> Option<&'static encoding_rs::Encoding> {
        match self {
            TextEncoding::Unicode | TextEncoding::Utf8 => None,
            TextEncoding::Sjis => Some(encoding_rs::SHIFT_JIS),
            TextEncoding::EucKr => Some(encoding_rs::EUC_KR),
            TextEncoding::Gbk => Some(encoding_rs::GBK),
            TextEncoding::Latin1 => Some(encoding_rs::WINDOWS_1252),
            TextEncoding::Hebrew => Some(encoding_rs::WINDOWS_1255),
        }
    }

    /// One character unit as the character it names.
    ///
    /// Strict on purpose: a malformed pair, a lone lead byte, or a byte sequence that would read
    /// as more than one character is `U+FFFD`. See the invariant at the top of this module.
    pub fn unit_to_char(self, unit: u16) -> char {
        match self {
            TextEncoding::Unicode | TextEncoding::Utf8 => {
                char::from_u32(u32::from(unit)).unwrap_or(REPLACEMENT)
            }
            TextEncoding::Latin1 | TextEncoding::Hebrew => {
                if unit > 0xff {
                    return REPLACEMENT;
                }
                let codec = self.codec().expect("a one-byte encoding has a codec");
                single_char(codec, &[unit as u8]).unwrap_or(REPLACEMENT)
            }
            TextEncoding::Sjis | TextEncoding::EucKr | TextEncoding::Gbk => {
                let codec = self.codec().expect("a multi-byte encoding has a codec");
                if unit <= 0xff {
                    single_char(codec, &[unit as u8]).unwrap_or(REPLACEMENT)
                } else {
                    // The unit's bytes in guest memory are the encoding's byte stream in the
                    // order the pack wrote them, which little-endian makes the low byte first.
                    // A handset's `DrawText` reads `82 cd` as `は`; the unit's value is 0xcd82.
                    single_char(codec, &unit.to_le_bytes()).unwrap_or(REPLACEMENT)
                }
            }
        }
    }

    /// The unit a character occupies, or `None` when this encoding cannot name it.
    ///
    /// `encoding_rs` writes an unmappable character as an HTML character reference; the
    /// `had_errors` flag is what keeps one of those from becoming a unit.
    pub fn char_to_unit(self, ch: char) -> Option<u16> {
        match self {
            TextEncoding::Unicode | TextEncoding::Utf8 => {
                let cp = ch as u32;
                (cp <= 0xffff && !(0xd800..=0xdfff).contains(&cp)).then_some(cp as u16)
            }
            _ => {
                let codec = self.codec().expect("a byte encoding has a codec");
                let mut buf = [0u8; 4];
                let (bytes, _, had_errors) = codec.encode(ch.encode_utf8(&mut buf));
                if had_errors {
                    return None;
                }
                match bytes.len() {
                    1 => Some(u16::from(bytes[0])),
                    // The unit is the pair of bytes as they sit in guest memory: little-endian,
                    // so the first byte of the encoding is the unit's low byte.
                    2 => Some(u16::from_le_bytes([bytes[0], bytes[1]])),
                    _ => None,
                }
            }
        }
    }

    /// A run of units as text, one character per unit.
    ///
    /// The fast path decodes the run's packed bytes in one pass; it is taken only when the result
    /// has exactly one character per unit. Anything else — a wrong encoding, a malformed pair —
    /// falls back to the per-unit rule, which is what guarantees the invariant.
    pub fn units_to_string(self, units: &[u16]) -> String {
        // ASCII is one byte in every encoding this host serves and one unit in the guest's own.
        // Menus and scores are mostly ASCII, and this is the path a frame takes thousands of
        // times, so it does not pay for a decoder.
        if units.iter().all(|unit| *unit < 0x80) {
            return units.iter().map(|unit| char::from(*unit as u8)).collect();
        }
        match self {
            TextEncoding::Unicode | TextEncoding::Utf8 => {
                units.iter().map(|u| self.unit_to_char(*u)).collect()
            }
            _ => {
                let bytes = self.units_to_bytes(units);
                let codec = self.codec().expect("a byte encoding has a codec");
                if let Some(text) =
                    codec.decode_without_bom_handling_and_without_replacement(&bytes)
                {
                    if text.chars().count() == units.len() {
                        return text.into_owned();
                    }
                }
                units.iter().map(|u| self.unit_to_char(*u)).collect()
            }
        }
    }

    /// Text as the units a guest buffer holds, one unit per character.
    ///
    /// A character the encoding cannot name becomes `U+FFFD` rather than a character reference or
    /// a surrogate pair: the guest's `AECHAR` is one cell per unit, and a layout that counts cells
    /// must not be handed two for a character it asked to store.
    pub fn string_to_units(self, text: &str) -> Vec<u16> {
        match self {
            TextEncoding::Unicode | TextEncoding::Utf8 => text
                .chars()
                .map(|ch| {
                    let cp = ch as u32;
                    if cp <= 0xffff && !(0xd800..=0xdfff).contains(&cp) {
                        cp as u16
                    } else {
                        0xfffd
                    }
                })
                .collect(),
            _ => text
                .chars()
                .map(|ch| self.char_to_unit(ch).unwrap_or(0xfffd))
                .collect(),
        }
    }

    /// A native byte stream (`char *`) as `AECHAR` units — the `STREXPAND` direction.
    ///
    /// Multi-byte encodings pack a character's bytes into one unit, which is what a handset's own
    /// wide string holds; a single-byte encoding widens one byte per unit.
    pub fn bytes_to_units(self, bytes: &[u8]) -> Vec<u16> {
        match self {
            TextEncoding::Unicode | TextEncoding::Latin1 | TextEncoding::Hebrew => {
                bytes.iter().map(|b| u16::from(*b)).collect()
            }
            TextEncoding::Utf8 => String::from_utf8_lossy(bytes)
                .chars()
                .map(|ch| {
                    let cp = ch as u32;
                    if cp <= 0xffff && !(0xd800..=0xdfff).contains(&cp) {
                        cp as u16
                    } else {
                        0xfffd
                    }
                })
                .collect(),
            TextEncoding::Sjis | TextEncoding::EucKr | TextEncoding::Gbk => {
                let codec = self.codec().expect("a multi-byte encoding has a codec");
                codec
                    .decode(bytes)
                    .0
                    .chars()
                    .map(|ch| self.char_to_unit(ch).unwrap_or(0xfffd))
                    .collect()
            }
        }
    }

    /// `AECHAR` units as the native byte stream a `char *` buffer holds — the `WSTRCOMPRESS`
    /// direction, and the inverse of [`TextEncoding::bytes_to_units`] for well-formed units.
    pub fn units_to_bytes(self, units: &[u16]) -> Vec<u8> {
        match self {
            TextEncoding::Utf8 => self.units_to_string(units).into_bytes(),
            TextEncoding::Unicode | TextEncoding::Latin1 | TextEncoding::Hebrew => {
                units.iter().map(|u| *u as u8).collect()
            }
            TextEncoding::Sjis | TextEncoding::EucKr | TextEncoding::Gbk => {
                let mut out = Vec::with_capacity(units.len() * 2);
                for unit in units {
                    if *unit <= 0xff {
                        out.push(*unit as u8);
                    } else {
                        // The bytes the guest's buffer holds, in its own order.
                        out.extend_from_slice(&unit.to_le_bytes());
                    }
                }
                out
            }
        }
    }
}

/// An `AEE_ENC_*` byte stream as text, for a report or a `%s` conversion.
pub fn stream_text(aee: u32, bytes: &[u8]) -> String {
    match TextEncoding::stream_codec(aee) {
        Some(codec) => codec.decode(bytes).0.into_owned(),
        None => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}

/// One byte sequence's single character, or `None` when it is malformed or names more than one.
fn single_char(codec: &'static encoding_rs::Encoding, bytes: &[u8]) -> Option<char> {
    let text = codec.decode_without_bom_handling_and_without_replacement(bytes)?;
    let mut chars = text.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

#[cfg(test)]
#[path = "test/encoding.rs"]
mod tests;
