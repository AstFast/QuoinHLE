//! Host settings a mobile or web entry installs before the manager starts.
//!
//! Desktop builds leave these unset: [`crate::library::program_data_dir`] falls back to
//! `QUOIN_DATA_DIR` and then `data/` beside the executable, the language comes from the OS or the
//! environment, and the CJK font comes from the desktop font paths. A mobile bundle has no
//! `current_exe` layout it can rely on, so its entry calls [`set_data_dir`] (and usually
//! [`set_ui_lang`] and [`set_font_file`]) before `manager::run`.
//!
//! Each setter keeps the first value. An entry that has already chosen a directory must not have it
//! replaced by a later library call.

pub mod console;
pub mod data;
pub mod fonts;
pub mod lang;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod window;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static UI_LANG: OnceLock<&'static str> = OnceLock::new();
static FONT_FILE: Mutex<Option<String>> = Mutex::new(None);

/// Name the app's persistent data directory (the `data/` root). Paths are host-native.
pub fn set_data_dir(dir: impl Into<PathBuf>) {
    let _ = DATA_DIR.set(dir.into());
}

/// The entry's data directory, if it installed one.
pub fn data_dir() -> Option<&'static Path> {
    DATA_DIR.get().map(PathBuf::as_path)
}

/// Set the interface language explicitly: `"en"` or `"zh"`.
pub fn set_ui_lang(tag: &'static str) {
    let _ = UI_LANG.set(tag);
}

/// The entry's language tag, if it installed one.
pub fn ui_lang() -> Option<&'static str> {
    UI_LANG.get().copied()
}

/// Set the CJK font file the interface should load. Later calls replace an earlier choice, which
/// is what the manager's font page needs; mobile entries call it once before `manager::run`.
pub fn set_font_file(path: &str) {
    if let Ok(mut slot) = FONT_FILE.lock() {
        *slot = Some(path.to_owned());
    }
}

/// Forget the explicit font choice and go back to the first path this host finds.
pub fn clear_font_file() {
    if let Ok(mut slot) = FONT_FILE.lock() {
        *slot = None;
    }
}

/// The selected font file, if the entry or the manager installed one.
pub fn font_file() -> Option<String> {
    FONT_FILE.lock().ok().and_then(|slot| slot.clone())
}
