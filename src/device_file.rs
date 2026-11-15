//! `device.json`: what the handset says about itself, in a file instead of in this crate.
//!
//! [`DeviceProfile::standard`] is one handset compiled into the host — 240x320, RGB565, a 23-key
//! pad, `MSM7201A`. That was the right shape while the only way to describe another phone was to
//! recompile, and the four flags that touch it (`--device`, `--color-depth`, `--encoding`, `--ram-mb`)
//! only reach four of its thirty-five fields. A pack that branches on the model name, the carrier id,
//! the storage figures or which keys exist had no way to be tried against a different handset at all.
//!
//! This module is that way: a document whose keys are the [`DeviceProfile`] fields, in the order the
//! struct declares them, read at start-up and written back on request.
//!
//! # The file is a set of overrides, not a record
//!
//! A key that is absent keeps the standard handset's value, so `{}` is a valid file and means "the
//! same phone as before". That is what makes the file small enough to keep in a repository and
//! readable enough to edit by hand — a person who wants a 176x220 device writes two lines, not
//! thirty-five. `--save-device` writes the *whole* profile back, which is how a file gets its full
//! listing: run once with the flags that describe the handset you want, and the file that comes out
//! is the same handset spelled out. It keeps whatever `_` notes were already in the file, because
//! those are the one part of it a person wrote.
//!
//! # What the key list changes, and what it does not
//!
//! `keys` is the answer to `AEE_DEVICEITEM_KEY_SUPPORT` — what a pack asks before it draws softkey
//! labels or decides which keys it can offer. It is **not** what the window draws: the phone beside
//! the screen is one fixed keypad ([`crate::handset`], built from [`crate::keymap::KEYS`]), and a key
//! pressed on it is delivered whether or not this list admits to having it. A file that names a
//! smaller keypad than the drawn one is a pack that hides more than the window does: consistent with
//! itself, not with the phone it is being played on. Making the drawing and the hit test obey this
//! list is a change to `handset.rs` and the window, not to this file, and it is not made here.
//!
//! # What is refused, and why the refusals are worth having
//!
//! - **An unknown key.** A misspelled field that is quietly ignored is a run that reports the
//!   standard handset while its file says otherwise. Keys starting with `_` are the escape hatch for
//!   a note, since JSON has no comments.
//! - **A wrong type, or a number that does not fit the field.** `"width": 240.5` is not a width.
//! - **An incoherent device.** Free storage larger than total, or a screen with no pixels: these are
//!   not choices about a handset, they are arithmetic the host would then have to answer for.
//! - **A `schema` from the future.** A file written by a build that knew fields this one does not
//!   would otherwise load with those fields silently dropped.
//!
//! What is *not* refused is an odd but coherent device: a 96x64 mono screen, a keypad of one key, an
//! IMEI of zeroes. Those are handsets that could exist, and answering for a strange device is the
//! point of the file.

use std::path::Path;

use crate::abi;
use crate::device::{DeviceFlags, DeviceProfile};
use crate::json::{self, Map, Value};

/// The version of this document that this build reads and writes.
///
/// Bumped when a key changes meaning or disappears. Adding a key does not bump it: an older build
/// meets an unknown key and says so, which is a better outcome than a version check that silently
/// ignores the whole file.
pub const SCHEMA: u64 = 1;

/// The document for one profile, keys in the order [`DeviceProfile`] declares its fields.
pub fn to_json(profile: &DeviceProfile) -> Value {
    let mut m = Map::new();
    m.insert("schema", Value::from(SCHEMA));
    m.insert("width", Value::from(profile.width));
    m.insert("height", Value::from(profile.height));
    m.insert("color_depth", Value::from(profile.color_depth));
    m.insert("alt_width", Value::from(profile.alt_width));
    m.insert("alt_height", Value::from(profile.alt_height));
    m.insert("scroll_bar", Value::from(profile.scroll_bar));
    m.insert("encoding", Value::from(profile.encoding));
    m.insert("menu_text_scroll", Value::from(profile.menu_text_scroll));
    m.insert("menu_image_delay", Value::from(profile.menu_image_delay));
    m.insert("ram", Value::from(profile.ram));
    m.insert("prompt_props", Value::from(profile.prompt_props));
    m.insert("key_close_app", Value::from(profile.key_close_app));
    m.insert(
        "key_close_all_apps",
        Value::from(profile.key_close_all_apps),
    );
    m.insert("lang", Value::from(profile.lang));
    m.insert("net_linger", Value::from(profile.net_linger));
    m.insert("sleep_defer", Value::from(profile.sleep_defer));
    m.insert("max_path", Value::from(profile.max_path));
    m.insert("platform_id", Value::from(profile.platform_id));
    m.insert("flags", flags_json(&profile.flags));
    m.insert("chip_id", Value::from(profile.chip_id.as_str()));
    m.insert("model_name", Value::from(profile.model_name.as_str()));
    m.insert("manufacturer", Value::from(profile.manufacturer.as_str()));
    m.insert("friendly_name", Value::from(profile.friendly_name.as_str()));
    m.insert(
        "firmware_version",
        Value::from(profile.firmware_version.as_str()),
    );
    m.insert("carrier_id", Value::from(profile.carrier_id));
    m.insert("esn", Value::from(profile.esn));
    m.insert("imei", Value::from(profile.imei.as_str()));
    m.insert("softkey_count", Value::from(profile.softkey_count));
    m.insert("manner_mode", Value::from(profile.manner_mode));
    m.insert("repeat_start", Value::from(profile.repeat_start));
    m.insert("repeat_rate", Value::from(profile.repeat_rate));
    m.insert("key_repeat_start", Value::from(profile.key_repeat_start));
    m.insert("key_repeat_rate", Value::from(profile.key_repeat_rate));
    m.insert("storage_total", Value::from(profile.storage_total));
    m.insert("storage_free", Value::from(profile.storage_free));
    m.insert(
        "keys",
        Value::Array(
            profile
                .keys
                .iter()
                .map(|code| match key_name(*code) {
                    // A name where the ABI has one: the file then reads as a keypad rather than as
                    // fifteen `57392`-shaped numbers, and a person can edit it without a table.
                    Some(name) => Value::from(name),
                    None => Value::from(*code),
                })
                .collect(),
        ),
    );
    Value::Object(m)
}

/// The document as text, ending in a newline.
pub fn to_text(profile: &DeviceProfile) -> String {
    json::write(&to_json(profile))
}

/// The flags object, in the order [`DeviceFlags::MEMBERS`] walks the bitfield run.
fn flags_json(flags: &DeviceFlags) -> Value {
    let mut m = Map::new();
    m.insert("alt_display", Value::from(flags.alt_display));
    m.insert("flip", Value::from(flags.flip));
    m.insert("vibrator", Value::from(flags.vibrator));
    m.insert("ext_speaker", Value::from(flags.ext_speaker));
    m.insert("voice_recognition", Value::from(flags.voice_recognition));
    m.insert("position_location", Value::from(flags.position_location));
    m.insert("midi", Value::from(flags.midi));
    m.insert("cmx", Value::from(flags.cmx));
    m.insert("pen", Value::from(flags.pen));
    Value::Object(m)
}

/// Read a document into a profile: the standard handset, with every key the document names replaced.
///
/// The error is a sentence rather than an [`json::Error`], because by this point the file has parsed
/// and what is wrong is a *field* — which field, what it holds, and what it should have held.
pub fn from_json(value: &Value) -> Result<DeviceProfile, String> {
    let Some(object) = value.as_object() else {
        return Err(format!(
            "the document has to be an object, and this is {}",
            value.kind()
        ));
    };
    let mut r = Reader::new(object);
    if let Some(schema) = r.u64("schema")? {
        if schema != SCHEMA {
            return Err(format!(
                "\"schema\" is {schema} and this build reads {SCHEMA}; the file was written by a \
                 build whose keys this one does not know"
            ));
        }
    }

    let mut p = DeviceProfile::standard();
    if let Some(v) = r.u16("width")? {
        p.width = v;
    }
    if let Some(v) = r.u16("height")? {
        p.height = v;
    }
    if let Some(v) = r.u16("color_depth")? {
        p.color_depth = v;
    }
    if let Some(v) = r.u16("alt_width")? {
        p.alt_width = v;
    }
    if let Some(v) = r.u16("alt_height")? {
        p.alt_height = v;
    }
    if let Some(v) = r.u16("scroll_bar")? {
        p.scroll_bar = v;
    }
    if let Some(v) = r.u16("encoding")? {
        p.encoding = v;
    }
    if let Some(v) = r.u16("menu_text_scroll")? {
        p.menu_text_scroll = v;
    }
    if let Some(v) = r.u32("menu_image_delay")? {
        p.menu_image_delay = v;
    }
    if let Some(v) = r.u32("ram")? {
        p.ram = v;
    }
    if let Some(v) = r.u32("prompt_props")? {
        p.prompt_props = v;
    }
    if let Some(v) = r.u16("key_close_app")? {
        p.key_close_app = v;
    }
    if let Some(v) = r.u16("key_close_all_apps")? {
        p.key_close_all_apps = v;
    }
    if let Some(v) = r.u32("lang")? {
        p.lang = v;
    }
    if let Some(v) = r.u32("net_linger")? {
        p.net_linger = v;
    }
    if let Some(v) = r.u32("sleep_defer")? {
        p.sleep_defer = v;
    }
    if let Some(v) = r.u16("max_path")? {
        p.max_path = v;
    }
    if let Some(v) = r.u32("platform_id")? {
        p.platform_id = v;
    }
    // Flags are overridden one at a time inside their object too: a file that says `"pen": false`
    // is describing one difference from this handset, not turning off its vibrator.
    if let Some(flags) = r.flags("flags", p.flags)? {
        p.flags = flags;
    }
    if let Some(v) = r.text("chip_id")? {
        p.chip_id = v;
    }
    if let Some(v) = r.text("model_name")? {
        p.model_name = v;
    }
    if let Some(v) = r.text("manufacturer")? {
        p.manufacturer = v;
    }
    if let Some(v) = r.text("friendly_name")? {
        p.friendly_name = v;
    }
    if let Some(v) = r.text("firmware_version")? {
        p.firmware_version = v;
    }
    if let Some(v) = r.u32("carrier_id")? {
        p.carrier_id = v;
    }
    if let Some(v) = r.u32("esn")? {
        p.esn = v;
    }
    if let Some(v) = r.text("imei")? {
        p.imei = v;
    }
    if let Some(v) = r.u8("softkey_count")? {
        p.softkey_count = v;
    }
    if let Some(v) = r.u32("manner_mode")? {
        p.manner_mode = v;
    }
    if let Some(v) = r.u32("repeat_start")? {
        p.repeat_start = v;
    }
    if let Some(v) = r.u32("repeat_rate")? {
        p.repeat_rate = v;
    }
    if let Some(v) = r.u32("key_repeat_start")? {
        p.key_repeat_start = v;
    }
    if let Some(v) = r.u32("key_repeat_rate")? {
        p.key_repeat_rate = v;
    }
    if let Some(v) = r.u64("storage_total")? {
        p.storage_total = v;
    }
    if let Some(v) = r.u64("storage_free")? {
        p.storage_free = v;
    }
    if let Some(keys) = r.keys("keys")? {
        p.keys = keys;
    }

    let unknown = r.leftovers();
    if !unknown.is_empty() {
        let names: Vec<String> = unknown.iter().map(|k| format!("\"{k}\"")).collect();
        return Err(format!(
            "unknown {}: {}. A key with no field behind it would be a run that reports this host's \
             default while the file says otherwise; start a key with `_` to keep a note in the file",
            if names.len() == 1 { "key" } else { "keys" },
            names.join(", ")
        ));
    }
    check(&p)?;
    Ok(p)
}

/// Refuse a device that is not a device: arithmetic the host would have to answer for, not a choice
/// about a handset.
fn check(p: &DeviceProfile) -> Result<(), String> {
    if p.width == 0 || p.height == 0 {
        return Err(format!(
            "\"width\" and \"height\" have to be at least 1, and this document gives {}x{}",
            p.width, p.height
        ));
    }
    if p.color_depth == 0 {
        return Err(
            "\"color_depth\" is 0; it is the depth of the screen, so it cannot be no bits at all"
                .to_string(),
        );
    }
    if p.storage_free > p.storage_total {
        return Err(format!(
            "\"storage_free\" is {} and \"storage_total\" is {}: more free space than the filesystem \
             has is an answer a pack divides by",
            p.storage_free, p.storage_total
        ));
    }
    // `AEE_DEVICEITEM_IMEI` is documented as fifteen digits, and the item is written as ASCII with a
    // terminator, so a longer string is one the caller's fifteen-digit buffer cannot hold.
    if p.imei.len() > 15 || !p.imei.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "\"imei\" has to be at most fifteen ASCII digits, and this is {:?}",
            p.imei
        ));
    }
    // Every identity string leaves this host as a NUL-terminated buffer — UTF-16 for the four that
    // `utf16` answers, ASCII for `firmware_version` — so an embedded NUL would end the guest's copy
    // early and the file would be saying something the guest never sees. Refusing it keeps "what the
    // file says" and "what the pack reads" the same sentence.
    for (key, text) in [
        ("chip_id", &p.chip_id),
        ("model_name", &p.model_name),
        ("manufacturer", &p.manufacturer),
        ("friendly_name", &p.friendly_name),
        ("firmware_version", &p.firmware_version),
    ] {
        if text.contains('\0') {
            return Err(format!(
                "\"{key}\" contains U+0000; it is answered as a terminated string, so the guest \
                 would stop reading there and never see the rest of {text:?}"
            ));
        }
    }
    Ok(())
}

/// What [`load`] found.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub profile: DeviceProfile,
    /// Whether the file was absent and has just been written with the standard handset. The caller
    /// says so out loud: a run that silently invented a file its operator did not ask for is a
    /// surprise waiting for the first person who edits the wrong one.
    pub created: bool,
}

/// Read `path`, or write the standard profile there and use it.
///
/// Creating the file is the whole discovery story: there is no search path, no environment variable
/// and no default location, because a device description that a run picked up from somewhere is
/// exactly the kind of thing that makes two runs of the same command disagree. Ask for the file and
/// you get it; do not ask and the built-in handset is used.
pub fn load(path: &Path) -> Result<Loaded, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let value = json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile = from_json(&value).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Loaded {
                profile,
                created: false,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let profile = DeviceProfile::standard();
            save(path, &profile).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Loaded {
                profile,
                created: true,
            })
        }
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Write `profile` to `path`, replacing whatever was there.
///
/// The `_`-prefixed notes already in the file are carried over. They are the one part of the document
/// this module does not own — a person wrote them — and a re-save that dropped them would delete an
/// explanation at the moment someone was rewriting the values it explained.
///
/// That is also why a file that cannot be *read* is refused instead of overwritten: "no notes found"
/// and "the file did not parse" are the same thing to a writer that shrugs, and only one of them is
/// the truth. A file that is not there is the one case with nothing to lose.
///
/// What comes out is the notes first, then every field in the order [`DeviceProfile`] declares them —
/// so a file that has notes does not start with `schema`, and that is on purpose: the note is the
/// reason the file exists.
pub fn save(path: &Path, profile: &DeviceProfile) -> Result<(), String> {
    let mut document = to_json(profile);
    let notes = match std::fs::read_to_string(path) {
        Ok(text) => {
            let value = json::parse(&text).map_err(|e| {
                format!(
                    "{}: {e} — not overwriting it, because the `_` notes in it would be lost",
                    path.display()
                )
            })?;
            match value {
                Value::Object(object) => object
                    .iter()
                    .filter(|(k, _)| k.starts_with('_'))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if !notes.is_empty() {
        let mut merged = Map::new();
        for (k, v) in notes {
            merged.insert(k, v);
        }
        if let Some(object) = document.as_object() {
            for (k, v) in object.iter() {
                merged.insert(k.clone(), v.clone());
            }
        }
        document = Value::Object(merged);
    }
    // Through a temporary file: `fs::write` truncates first, so a run that dies mid-write leaves a
    // half document that the *next* run then refuses to parse — the file is the answer to "what did
    // I describe", and an answer cut in half is worse than the previous one left alone.
    let text = json::write(&document);
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text).map_err(|e| format!("{}: {e}", temp.display()))?;
    std::fs::rename(&temp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The name the ABI and the keypad table give a key code, if either knows one.
///
/// The keypad table first: it is this host's own list of the keys a player can press, so its names
/// are the ones the rest of the tree already prints. `const_name` is the fallback for the keys a
/// handset advertises but no key draws (`AVK_SEND`, `AVK_MENU`, `AVK_END` are in [`DeviceProfile`]'s
/// default list and not on the drawn phone).
fn key_name(code: u16) -> Option<&'static str> {
    if let Some(key) = crate::keymap::KEYS.iter().find(|k| k.avk() as u16 == code) {
        return Some(key.name);
    }
    abi::const_name("AVK_", u32::from(code))
}

/// A key code from an `AVK_*` name. Only names under that prefix: `ABI` has nine thousand constants
/// and a keypad entry that happens to be `AEE_EBADPARM` is a file that means something else.
///
/// Any name under the prefix is taken, including the five the SDK uses as markers rather than keys
/// (`AVK_FIRST` `0x0000e020`, `AVK_UNDEFINED` `0x0000e010`, `AVK_OEM_RANGE_START`/`_END`
/// `0x0000f800`/`0x0000f8ff`, `AVK_MAX_OVERLOAD` `0x00000005`). Telling those apart would take a
/// hardcoded list that goes stale with the next SDK, and the cost of getting it wrong is a handset
/// that claims support for a code no pack will ever press — cheaper than refusing a key this host
/// has not heard of, which would be the file saying something true and the host calling it a typo.
fn key_code(name: &str) -> Option<u16> {
    if !name.starts_with("AVK_") {
        return None;
    }
    let value = abi::const_value(name)?;
    if value > u32::from(u16::MAX) {
        return None;
    }
    Some(value as u16)
}

/// One object being read, remembering which keys were asked for so the rest can be named as errors.
struct Reader<'a> {
    object: &'a Map,
    seen: Vec<&'static str>,
}

impl<'a> Reader<'a> {
    fn new(object: &'a Map) -> Self {
        Reader {
            object,
            seen: Vec::new(),
        }
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        let value = self.object.get(key);
        if value.is_some() {
            self.seen.push(key);
        }
        value
    }

    fn u64(&mut self, key: &'static str) -> Result<Option<u64>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_u64() {
            Some(n) => Ok(Some(n)),
            None => Err(mismatch(key, "a non-negative whole number", value)),
        }
    }

    fn u32(&mut self, key: &'static str) -> Result<Option<u32>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u32::MAX) => Ok(Some(n as u32)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 32 bits")),
        }
    }

    fn u16(&mut self, key: &'static str) -> Result<Option<u16>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u16::MAX) => Ok(Some(n as u16)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 16 bits")),
        }
    }

    fn u8(&mut self, key: &'static str) -> Result<Option<u8>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u8::MAX) => Ok(Some(n as u8)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 8 bits")),
        }
    }

    fn text(&mut self, key: &'static str) -> Result<Option<String>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_str() {
            Some(s) => Ok(Some(s.to_string())),
            None => Err(mismatch(key, "a string", value)),
        }
    }

    fn bool(&mut self, key: &'static str) -> Result<Option<bool>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_bool() {
            Some(b) => Ok(Some(b)),
            None => Err(mismatch(key, "true or false", value)),
        }
    }

    /// The flags object, each bit a key, applied over `base` so that naming one bit leaves the rest.
    fn flags(
        &mut self,
        key: &'static str,
        base: DeviceFlags,
    ) -> Result<Option<DeviceFlags>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(object) = value.as_object() else {
            return Err(mismatch(key, "an object of flags", value));
        };
        let mut inner = Reader::new(object);
        let mut flags = base;
        if let Some(b) = inner.bool("alt_display")? {
            flags.alt_display = b;
        }
        if let Some(b) = inner.bool("flip")? {
            flags.flip = b;
        }
        if let Some(b) = inner.bool("vibrator")? {
            flags.vibrator = b;
        }
        if let Some(b) = inner.bool("ext_speaker")? {
            flags.ext_speaker = b;
        }
        if let Some(b) = inner.bool("voice_recognition")? {
            flags.voice_recognition = b;
        }
        if let Some(b) = inner.bool("position_location")? {
            flags.position_location = b;
        }
        if let Some(b) = inner.bool("midi")? {
            flags.midi = b;
        }
        if let Some(b) = inner.bool("cmx")? {
            flags.cmx = b;
        }
        if let Some(b) = inner.bool("pen")? {
            flags.pen = b;
        }
        let unknown = inner.leftovers();
        if !unknown.is_empty() {
            let names: Vec<String> = unknown.iter().map(|k| format!("\"{k}\"")).collect();
            return Err(format!("\"{key}\": unknown flag {}", names.join(", ")));
        }
        Ok(Some(flags))
    }

    /// The keypad: `AVK_*` names, or raw 16-bit codes for the ones no name covers.
    fn keys(&mut self, key: &'static str) -> Result<Option<Vec<u16>>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(items) = value.as_array() else {
            return Err(mismatch(key, "an array of key names or codes", value));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Value::String(name) => match key_code(name) {
                    Some(code) => out.push(code),
                    None => {
                        return Err(format!(
                            "\"{key}\": \"{name}\" is not an `AVK_*` name in the ABI tables"
                        ))
                    }
                },
                other => match other.as_u64() {
                    Some(n) if n <= u64::from(u16::MAX) => out.push(n as u16),
                    Some(n) => {
                        return Err(format!("\"{key}\": {n} does not fit a 16-bit key code"))
                    }
                    None => {
                        return Err(format!(
                            "\"{key}\": a key is a name or a 16-bit code, and this is {}",
                            other.kind()
                        ))
                    }
                },
            }
        }
        Ok(Some(out))
    }

    /// Every key nobody asked for. An underscore is how a person writes a note in a format that has
    /// no comments, so those are skipped.
    fn leftovers(&self) -> Vec<&'a str> {
        self.object
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !k.starts_with('_'))
            .filter(|k| !self.seen.iter().any(|seen| seen == k))
            .collect()
    }
}

fn mismatch(key: &str, wanted: &str, found: &Value) -> String {
    format!("\"{key}\": expected {wanted}, found {}", found.kind())
}

#[cfg(test)]
#[path = "test/device_file.rs"]
mod tests;
