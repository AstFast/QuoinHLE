//! The persistent data root and the one-time migration from the old beside-exe layout.

use std::path::PathBuf;

/// The one directory this program owns for persistent state.
///
/// It is `data/` beside the executable on every platform, so the program directory is the portable
/// unit: copy or move it and the saves, manager record, trusted scripts and developer reports move
/// with it. `QUOIN_DATA_DIR` overrides the location; a platform whose executable has no writable
/// directory (Android, Web, or an iOS bundle) must set it, and `None` means this run has no place to
/// write rather than a path invented from the environment.
pub fn program_data_dir() -> Option<PathBuf> {
    if let Some(dir) = crate::platform::data_dir() {
        return Some(dir.to_path_buf());
    }
    if let Some(dir) = std::env::var_os("QUOIN_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("data"))
}

/// Move state written by older builds beside the executable into [`program_data_dir`].
///
/// The old layout put `saves/`, `scripts/` and `manager.json` directly beside the executable; the
/// portable layout keeps them under `data/`. This is a rename, not a copy: a second copy would make
/// two runs disagree about which one is current, and a failure leaves the old file alone rather than
/// deleting anything.
pub fn migrate_legacy_data() {
    let Some(data) = program_data_dir() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe.parent() else {
        return;
    };
    if std::fs::create_dir_all(&data).is_err() {
        return;
    }
    for name in ["saves", "scripts", "manager.json"] {
        let old = exe_dir.join(name);
        let new = data.join(name);
        if old != new && old.exists() && !new.exists() {
            let _ = std::fs::rename(&old, &new);
        }
    }
}
