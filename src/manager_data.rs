//! What the manager remembers between runs.
//!
//! This is the only file of this host's own shape. A pack's saves belong to the pack's volume
//! ([`crate::library::default_save_dir`]); nothing written here ever goes near a `.mod`. What a person
//! changes in the manager — the order their games sit in, what they renamed one to, which picture they
//! gave it, what screen they want it started at — is the manager's business and stops at the manager.
//! The pack keeps its own bytes and its own idea of what it is; the manager keeps a note of what the
//! person wants to see.
//!
//! JSON, because it is meant to be readable and editable by hand: whoever dislikes the order the manager
//! worked out can open this file and type another one. That is also why the reader is forgiving — an
//! unknown field is ignored, a missing one takes its default, and a file this host cannot read at all is
//! **moved aside rather than deleted** (see [`Data::load`]): a note about a game's name is not worth
//! losing somebody's ordering over, and it is certainly not worth losing silently.
//!
//! **Hand-written, no `serde`.** The shape is three keys of strings, numbers and two levels of object, and
//! this repository's rule is that a dependency has to buy something that cannot reasonably be written.
//! A hundred and fifty lines of parser buys not having a second ecosystem in the build — and the reader
//! that ships with the data is the one that knows exactly what this file may contain.
//!
//! Keys are **relative to the pack root** wherever a path is used (a pack's directory under the root),
//! not absolute: a library moved to another drive should keep its notes, and an absolute path would spend
//! the rest of its life pointing at where the packs used to be.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::audio::AudioBackend;

/// Everything the manager remembers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Data {
    /// Every game's `.mod` file stem, in the order a person put them in. A game that is not in here is
    /// one the manager has not seen before, and it goes **last** — a new pack arriving at the top of the
    /// list would undo the ordering every time a pack is added.
    pub order: Vec<String>,
    /// What a person changed about a game, by its own name.
    pub games: BTreeMap<String, Game>,
    /// What they changed about one build of one game, by its path relative to the pack root.
    pub packs: BTreeMap<String, Pack>,
    /// The SoundFonts in the sound library, in the order a person put them in.
    ///
    /// Absolute paths, not relative ones: an `.sf2` usually lives somewhere else entirely (a download
    /// folder, an unpacked Debian package), and filing it below the pack root would tie the library to a
    /// directory it has nothing to do with.
    pub sounds: Vec<String>,
    /// Which of them a launch plays through. `None` is silence, and silence is also the opening state: a
    /// host that made a noise nobody asked for would be worse than one that made none.
    pub sound_in_use: Option<String>,
    /// Which compressed-audio decoder the next launch uses: `rust` (default) or `ffmpeg`.
    ///
    /// `None` is the state of every file written before the setting existed, and reads as the default
    /// Rust backend. The value is the backend's own name, so a hand-edited file is readable.
    pub audio_backend: Option<String>,
    /// Whether a started game gets a window of its own. `None` is "this file has never been asked",
    /// which is the state of every file written before the setting existed — and it reads as **yes**,
    /// because a person who starts a game expects to keep the list they started it from. The file only
    /// carries a `true`/`false` once somebody has used the checkbox in the settings page. Only a desktop
    /// build can honour it, which is `manager.rs`'s business, not this file's.
    pub separate_window: Option<bool>,
    /// Whether the game window draws the guest presentation rate. `None` is "never asked", which
    /// reads as **yes**: a player looking for a frame rate should not have to discover a hidden file
    /// setting first. A stored `false` is the explicit choice to hide it.
    pub show_fps: Option<bool>,
    /// The language the manager's own chrome is drawn in: `"en"`, `"zh"`, or `None` for the system
    /// language. A pack's own text is never translated.
    pub ui_lang: Option<String>,
    /// The font file the manager should draw with. `None` is "use the first font this host finds".
    pub font: Option<String>,
    /// Handset `AVK_*` names mapped to the egui key name a player pressed. Missing names use the
    /// defaults in `crate::keymap`.
    pub keys: BTreeMap<String, String>,
}

/// A game's overrides: what to call it, and which picture to draw for it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Game {
    /// The name to show instead of the `.mod` file stem.
    pub name: Option<String>,
    /// A picture to draw instead of the one in the pack's own `.mif`, as a path relative to the pack root.
    pub icon: Option<String>,
}

/// One build's overrides: the screen to start it at.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pack {
    pub width: Option<u16>,
    pub height: Option<u16>,
}

/// The manager's file inside the program data directory: `manager.json`.
pub fn path() -> Option<PathBuf> {
    crate::library::program_data_dir().map(|dir| dir.join("manager.json"))
}

/// The key a pack is filed under: its path below the pack root, with `/` separators.
///
/// A path that is not below the root is filed under its own name, which is the best that can be said
/// about it — `--info` can be pointed at a single `.mod` anywhere, and a note about it still has to have
/// somewhere to live.
pub fn key_of(root: &Path, mod_path: &Path) -> String {
    let relative = mod_path.strip_prefix(root).unwrap_or(mod_path);
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

impl Data {
    /// Read the file at `path`.
    ///
    /// The second value is what went wrong, if anything did, in words for a player — the caller is a
    /// window that can put it on the list, and silence here would be the worst kind of bug this
    /// repository knows about: a lost ordering that looks exactly like an ordering that was respected.
    ///
    /// A file that cannot be read is **moved to `<name>.bad`** rather than deleted, and the defaults are
    /// used, so the next save starts from a clean note without the old one being thrown away.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &Path) -> (Data, Option<String>) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            // No file at all is not a problem: it is the first run.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Data::default(), None),
            // Anything else — a file that is not UTF-8, one this process may not read — is **not** the first
            // run, and pretending it is would have the next save write over whatever is in there.
            Err(e) => {
                let aside = path.with_extension("bad");
                let moved = std::fs::rename(path, &aside);
                let where_to = if moved.is_ok() {
                    format!("{} (renamed to {})", path.display(), aside.display())
                } else {
                    path.display().to_string()
                };
                return (
                    Data::default(),
                    Some(format!(
                        "the manager record cannot be read: {where_to} — {e}"
                    )),
                );
            }
        };
        match parse(&text).and_then(|json| Data::from_json(&json)) {
            Ok(data) => (data, None),
            Err(why) => {
                let aside = path.with_extension("bad");
                let moved = std::fs::rename(path, &aside);
                let where_to = if moved.is_ok() {
                    format!("{} (renamed to {})", path.display(), aside.display())
                } else {
                    path.display().to_string()
                };
                (
                    Data::default(),
                    Some(format!(
                        "the manager record cannot be read: {where_to} — {why}"
                    )),
                )
            }
        }
    }

    /// Write it back, as JSON a person can read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(path, self.to_json()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Read the browser's `localStorage` entry. `std::fs` has no meaning on wasm, and a browser
    /// profile is the closest thing to the per-user file the native build keeps beside the data dir.
    #[cfg(target_arch = "wasm32")]
    pub fn load(path: &Path) -> (Data, Option<String>) {
        let Some(storage) = crate::platform::wasm::web_storage() else {
            return (
                Data::default(),
                Some(
                    "browser storage is not available; this run's changes will not be kept"
                        .to_string(),
                ),
            );
        };
        let key = path.to_string_lossy();
        let text = match storage.get_item(&key) {
            Ok(Some(text)) => text,
            Ok(None) => return (Data::default(), None),
            Err(e) => {
                return (
                    Data::default(),
                    Some(format!("the manager record cannot be read: {key} — {e:?}")),
                )
            }
        };
        match parse(&text).and_then(|json| Data::from_json(&json)) {
            Ok(data) => (data, None),
            Err(why) => {
                let _ = storage.remove_item(&key);
                (
                    Data::default(),
                    Some(format!("the manager record cannot be read: {key} — {why}")),
                )
            }
        }
    }

    /// Write it back through the browser's `localStorage`.
    #[cfg(target_arch = "wasm32")]
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let storage = crate::platform::wasm::web_storage()
            .ok_or_else(|| "browser storage is not available".to_string())?;
        let key = path.to_string_lossy();
        storage
            .set_item(&key, &self.to_json())
            .map_err(|e| format!("{key}: {e:?}"))
    }

    /// Whether a started game gets a window of its own. A file that has never been asked says yes.
    pub fn uses_separate_window(&self) -> bool {
        self.separate_window.unwrap_or(true)
    }

    /// Remember the answer to that question.
    ///
    /// `Some(false)` is written as `false` rather than dropped back to `null`: "a person turned this
    /// off" and "nobody has ever looked at it" are different states, and only the second one is allowed
    /// to change its mind on its own.
    pub fn set_separate_window(&mut self, on: bool) {
        self.separate_window = Some(on);
    }

    /// Whether the game window should draw the FPS overlay. A file written before the key existed
    /// says yes.
    pub fn uses_fps(&self) -> bool {
        self.show_fps.unwrap_or(true)
    }

    pub fn set_show_fps(&mut self, on: bool) {
        self.show_fps = Some(on);
    }

    /// The decoder backend the next launch should use. An absent or unknown value is the default Rust
    /// backend, not a failed file.
    pub fn audio_backend(&self) -> AudioBackend {
        self.audio_backend
            .as_deref()
            .and_then(AudioBackend::parse)
            .unwrap_or_default()
    }

    pub fn set_audio_backend(&mut self, backend: AudioBackend) {
        self.audio_backend = Some(backend.name().to_string());
    }

    /// Remember which language the manager's own chrome is drawn in. `None` is "follow the system",
    /// which is also what a file written before this key existed means.
    pub fn set_ui_lang(&mut self, tag: &str) {
        self.ui_lang = Some(tag.to_string());
    }

    /// Choose the font file the manager draws with; `None` means the first one the host finds.
    pub fn set_font(&mut self, path: Option<String>) {
        self.font = path;
    }

    /// The custom binding for one handset key, if the person changed it.
    pub fn key_binding(&self, name: &str) -> Option<&str> {
        self.keys.get(name).map(String::as_str)
    }

    pub fn set_key(&mut self, name: &str, key: &str) {
        self.keys.insert(name.to_string(), key.to_string());
    }

    pub fn clear_key(&mut self, name: &str) {
        self.keys.remove(name);
    }

    /// Where `title` sits in the library. Games the file has never heard of sort after every game it has.
    pub fn rank(&self, title: &str) -> usize {
        self.order
            .iter()
            .position(|t| t == title)
            .unwrap_or(usize::MAX)
    }

    /// Reorder so that the game at `from` sits at `to`.
    ///
    /// Both are positions in the **list as the window draws it**, which is what a drag is expressed in; the
    /// result is written back as the file wants it, which is the whole ordering including the games this
    /// call was not about.
    pub fn move_game(&mut self, drawn: &[String], from: usize, to: usize) {
        if from >= drawn.len() || to >= drawn.len() || from == to {
            return;
        }
        let mut next = drawn.to_vec();
        let moved = next.remove(from);
        next.insert(to.min(next.len()), moved);
        // Whatever the window was not showing keeps its place at the end: a filter is not a statement about
        // where the hidden games belong.
        for title in &self.order {
            if !next.contains(title) {
                next.push(title.clone());
            }
        }
        self.order = next;
    }

    /// The name to show for a game: what a person called it, or nothing if they never said.
    pub fn name_of(&self, title: &str) -> Option<&str> {
        self.games.get(title)?.name.as_deref()
    }

    /// The picture a person chose for a game, if they chose one.
    pub fn icon_of(&self, title: &str) -> Option<&str> {
        self.games.get(title)?.icon.as_deref()
    }

    /// The screen a person wants a build started at, if they said.
    pub fn screen_of(&self, key: &str) -> Option<(u16, u16)> {
        let pack = self.packs.get(key)?;
        Some((pack.width?, pack.height?))
    }

    /// Say what a game is called. An empty name is "no name of my own", not a game called "".
    pub fn set_name(&mut self, title: &str, name: &str) {
        let name = name.trim();
        let game = self.games.entry(title.to_string()).or_default();
        game.name = (!name.is_empty()).then(|| name.to_string());
    }

    /// Say which picture a game is drawn with. Empty is "use the pack's own".
    pub fn set_icon(&mut self, title: &str, icon: &str) {
        let icon = icon.trim();
        let game = self.games.entry(title.to_string()).or_default();
        game.icon = (!icon.is_empty()).then(|| icon.to_string());
    }

    /// Say which screen a build starts at, or forget it.
    pub fn set_screen(&mut self, key: &str, screen: Option<(u16, u16)>) {
        let pack = self.packs.entry(key.to_string()).or_default();
        let (width, height) = match screen {
            Some((w, h)) => (Some(w), Some(h)),
            None => (None, None),
        };
        pack.width = width;
        pack.height = height;
    }

    /// Put an `.sf2` in the library, once. Says whether it went in.
    ///
    /// The same path twice is the same SoundFont twice: somebody who drops a file on the window again means
    /// to have one of it, not to have it listed twice and then have to work out which entry is in use.
    pub fn add_sound(&mut self, path: &str) -> bool {
        let path = path.trim();
        if path.is_empty() || self.sounds.iter().any(|held| held == path) {
            return false;
        }
        self.sounds.push(path.to_string());
        // The first one in becomes the one in use: dropping a sound bank on the window and hearing nothing
        // would read as the drop having failed.
        if self.sound_in_use.is_none() {
            self.sound_in_use = Some(path.to_string());
        }
        true
    }

    /// Take every path in `gone` out of the library, and say how many went.
    pub fn drop_sounds(&mut self, gone: &BTreeSet<String>) -> usize {
        let before = self.sounds.len();
        self.sounds.retain(|path| !gone.contains(path));
        // If the one in use was among them, the library goes back to silence: a name pointing at a file that
        // is no longer listed is a state in which "in use" and "not in use" cannot be told apart.
        if self
            .sound_in_use
            .as_ref()
            .is_some_and(|path| gone.contains(path))
        {
            self.sound_in_use = None;
        }
        before - self.sounds.len()
    }

    /// Move the entry at `from` to `to`, as the rows on screen are numbered.
    pub fn move_sound(&mut self, from: usize, to: usize) {
        if from >= self.sounds.len() || to >= self.sounds.len() || from == to {
            return;
        }
        let moved = self.sounds.remove(from);
        self.sounds.insert(to.min(self.sounds.len()), moved);
    }

    /// Say which one a launch plays through, or none of them.
    pub fn use_sound(&mut self, path: Option<&str>) {
        self.sound_in_use = path.map(str::to_string);
    }

    /// The file's contents, as text.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n  \"version\": 1,\n");
        out.push_str("  \"order\": [");
        for (i, title) in self.order.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&quote(title));
        }
        out.push_str("],\n  \"games\": {");
        for (i, (title, game)) in self.games.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            out.push_str(&quote(title));
            out.push_str(": {");
            let mut fields = Vec::new();
            if let Some(name) = &game.name {
                fields.push(format!("\"name\": {}", quote(name)));
            }
            if let Some(icon) = &game.icon {
                fields.push(format!("\"icon\": {}", quote(icon)));
            }
            out.push_str(&fields.join(", "));
            out.push('}');
        }
        // An empty object reads better on one line than as `{\n  }`, and this file is meant to be read.
        out.push_str(if self.games.is_empty() {
            "},\n"
        } else {
            "\n  },\n"
        });
        out.push_str("  \"packs\": {");
        for (i, (key, pack)) in self.packs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            out.push_str(&quote(key));
            let mut fields = Vec::new();
            if let (Some(w), Some(h)) = (pack.width, pack.height) {
                fields.push(format!("\"width\": {w}, \"height\": {h}"));
            }
            out.push_str(&format!(": {{{}}}", fields.join(", ")));
        }
        out.push_str(if self.packs.is_empty() {
            "},\n"
        } else {
            "\n  },\n"
        });
        out.push_str("  \"sounds\": [");
        for (i, path) in self.sounds.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&quote(path));
        }
        out.push_str("],\n  \"sound_in_use\": ");
        match &self.sound_in_use {
            Some(path) => out.push_str(&quote(path)),
            // Silence is "none", not an empty path: a person editing this by hand should not be able to turn
            // it into "go and look for a file called ''".
            None => out.push_str("null"),
        }
        // Always written, `null` and all: this file is meant to be read by a person, and one who wants to
        // know what this host is set to should find it written down. `null` reads as the default, which is
        // exactly what a file predating this key gets.
        out.push_str(",\n  \"separate_window\": ");
        match self.separate_window {
            Some(true) => out.push_str("true"),
            Some(false) => out.push_str("false"),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"show_fps\": ");
        match self.show_fps {
            Some(true) => out.push_str("true"),
            Some(false) => out.push_str("false"),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"audio_backend\": ");
        match &self.audio_backend {
            Some(name) => out.push_str(&quote(name)),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"ui_lang\": ");
        match &self.ui_lang {
            Some(tag) => out.push_str(&quote(tag)),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"font\": ");
        match &self.font {
            Some(path) => out.push_str(&quote(path)),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"keys\": {");
        for (i, (name, key)) in self.keys.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            out.push_str(&quote(name));
            out.push_str(": ");
            out.push_str(&quote(key));
        }
        out.push('}');
        out.push('\n');
        out.push_str("}\n");
        out
    }

    /// Read the shape [`Data::to_json`] writes, ignoring anything it does not know.
    fn from_json(json: &Json) -> Result<Data, String> {
        let object = json.object().ok_or("the top level is not an object")?;
        let mut data = Data::default();
        for (key, value) in object {
            match key.as_str() {
                "order" => {
                    let items = value.array().ok_or("\"order\" is not an array")?;
                    for item in items {
                        let title = item
                            .str()
                            .ok_or("\"order\" holds something other than names")?;
                        data.order.push(title.to_string());
                    }
                }
                "games" => {
                    for (title, value) in value.object().unwrap_or(&[]) {
                        let fields = value.object().unwrap_or(&[]);
                        let game = data.games.entry(title.clone()).or_default();
                        for (field, value) in fields {
                            match (field.as_str(), value.str()) {
                                ("name", Some(text)) => game.name = Some(text.to_string()),
                                ("icon", Some(text)) => game.icon = Some(text.to_string()),
                                // A field this version does not know is not an error: a file written by a
                                // later build should still load its order here.
                                _ => {}
                            }
                        }
                    }
                }
                "packs" => {
                    for (key, value) in value.object().unwrap_or(&[]) {
                        let fields = value.object().unwrap_or(&[]);
                        let pack = data.packs.entry(key.clone()).or_default();
                        for (field, value) in fields {
                            let size = value.number().and_then(|n| u16::try_from(n as i64).ok());
                            match (field.as_str(), size) {
                                ("width", Some(w)) => pack.width = Some(w),
                                ("height", Some(h)) => pack.height = Some(h),
                                _ => {}
                            }
                        }
                    }
                }
                "sounds" => {
                    let items = value.array().ok_or("\"sounds\" is not an array")?;
                    for item in items {
                        let path = item
                            .str()
                            .ok_or("\"sounds\" holds something other than paths")?;
                        data.sounds.push(path.to_string());
                    }
                }
                "sound_in_use" => {
                    // Only a string counts as "in use"; `null` — or anything this build does not understand —
                    // is silence, which is a state and not a failure.
                    data.sound_in_use = value.str().map(str::to_string);
                }
                "audio_backend" => {
                    // Unknown names fall back to the Rust backend in `Data::audio_backend`; keep the text
                    // so a later build that knows it can still report what the person asked for.
                    data.audio_backend = value.str().map(str::to_string);
                }
                "separate_window" => {
                    // A boolean is somebody's choice; `null` — or anything this build does not understand —
                    // is the same answer as the key being absent, and that answer is "yes" (see the field's
                    // own comment).
                    data.separate_window = value.bool();
                }
                "show_fps" => data.show_fps = value.bool(),
                "ui_lang" => {
                    // Unknown tags fall back to the system language in `UiLang::from_tag`; keep the text
                    // so a later build that knows it can still report what the person asked for.
                    data.ui_lang = value.str().map(str::to_string);
                }
                "font" => data.font = value.str().map(str::to_string),
                "keys" => {
                    for (name, value) in value.object().unwrap_or(&[]) {
                        if let Some(key) = value.str() {
                            data.keys.insert(name.clone(), key.to_string());
                        }
                    }
                }
                // `version` is written for a person reading the file; nothing branches on it yet, and a
                // file with no version at all loads the same way.
                _ => {}
            }
        }
        Ok(data)
    }
}

/// `text` as a JSON string, quotes and all.
///
/// Non-ASCII is written as-is rather than as `\uXXXX`: this file is read by people, most of the names in
/// it are Chinese, and an escaped name is one nobody can recognise.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ────────────────────────────── the reader ──────────────────────────────

/// Just enough JSON to read what this file writes, and to survive what a person types into it.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(fields) => Some(fields),
            _ => None,
        }
    }

    fn array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Text(text) => Some(text),
            _ => None,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn bool(&self) -> Option<bool> {
        match self {
            Json::Bool(on) => Some(*on),
            _ => None,
        }
    }
}

/// Read one JSON value, ignoring the whitespace around it. Trailing content that is not whitespace is an
/// error, so a file with two documents in it is refused rather than half-read.
pub fn parse(src: &str) -> Result<Json, String> {
    let mut reader = Reader {
        src,
        at: 0,
        depth: 0,
    };
    reader.skip_space();
    let value = reader.value()?;
    reader.skip_space();
    if reader.at < src.len() {
        return Err(format!("trailing data after character {}", reader.at));
    }
    Ok(value)
}

/// How deep a document may nest.
///
/// `value` and `object`/`array` are mutually recursive, and [`Data::load`] feeds this parser **whatever
/// file is at that path**: a `[[[[…` file would walk this process into a stack overflow, which is not
/// catchable. A manager's notes file should not be able to do that to the machine.
const MAX_DEPTH: usize = 64;

struct Reader<'a> {
    src: &'a str,
    /// A byte offset, but always on a `char` boundary: everything here steps by whole characters.
    at: usize,
    /// How many containers deep the reader is.
    depth: usize,
}

impl Reader<'_> {
    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.at += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.at..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += c.len_utf8();
        Some(c)
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        if self.bump() == Some(want) {
            Ok(())
        } else {
            Err(format!("expected {want:?} at character {}", self.at))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(format!("nesting deeper than {MAX_DEPTH}"));
        }
        let value = self.value_here();
        self.depth -= 1;
        value
    }

    /// One value, with the depth already counted by [`Reader::value`].
    fn value_here(&mut self) -> Result<Json, String> {
        self.skip_space();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Text(self.string()?)),
            Some('t') => self.literal("true", Json::Bool(true)),
            Some('f') => self.literal("false", Json::Bool(false)),
            Some('n') => self.literal("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!("found {other:?} at character {}", self.at)),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.src[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("no value at character {}", self.at))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut fields = Vec::new();
        self.skip_space();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.expect(':')?;
            let value = self.value()?;
            fields.push((key, value));
            self.skip_space();
            match self.bump() {
                Some(',') => continue,
                Some('}') => break,
                other => {
                    return Err(format!(
                        "found {other:?} in the object at character {}",
                        self.at
                    ))
                }
            }
        }
        Ok(Json::Object(fields))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_space();
            match self.bump() {
                Some(',') => continue,
                Some(']') => break,
                other => {
                    return Err(format!(
                        "found {other:?} in the array at character {}",
                        self.at
                    ))
                }
            }
        }
        Ok(Json::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated string".to_string()),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let first = self.hex4()?;
                        // A pair of surrogates is one character; half a pair is not a character at all, and
                        // a replacement is better than refusing the whole file over it.
                        let c = match first {
                            0xd800..=0xdbff => match self.peek() {
                                Some('\\') => {
                                    self.bump();
                                    self.expect('u')?;
                                    let second = self.hex4()?;
                                    let combined = 0x10000
                                        + ((first as u32 - 0xd800) << 10)
                                        + (second as u32).saturating_sub(0xdc00);
                                    char::from_u32(combined)
                                }
                                _ => None,
                            }
                            .unwrap_or('\u{fffd}'),
                            other => char::from_u32(other as u32).unwrap_or('\u{fffd}'),
                        };
                        out.push(c);
                    }
                    other => return Err(format!("unknown escape {other:?} in a string")),
                },
                Some(c) => out.push(c),
            }
        }
        Ok(out)
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut value = 0u16;
        for _ in 0..4 {
            let digit = self
                .bump()
                .and_then(|c| c.to_digit(16))
                .ok_or("\\u must be followed by four hex digits")?;
            value = value * 16 + digit as u16;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.at;
        if self.peek() == Some('-') {
            self.bump();
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-')
        {
            self.bump();
        }
        self.src[start..self.at]
            .parse::<f64>()
            .map(Json::Number)
            .map_err(|_| format!("not a number at {start}"))
    }
}

#[cfg(test)]
#[path = "test/manager_data.rs"]
mod tests;
