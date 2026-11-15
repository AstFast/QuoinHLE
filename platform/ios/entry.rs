// iOS entry template. This file is not compiled by the desktop build.
//
// Build the Rust side for `aarch64-apple-ios`, link it into the Xcode app, and call
// `quoin_ios_start` from the app delegate. `HOME` is the app sandbox on iOS; the data directory
// must stay inside it and never be derived from a desktop executable path.

#![cfg(target_os = "ios")]

use std::path::PathBuf;

#[no_mangle]
pub extern "C" fn quoin_ios_start() {
    if let Some(home) = std::env::var_os("HOME") {
        quoin::platform::set_data_dir(
            PathBuf::from(home).join("Library/Application Support/quoin"),
        );
    }
    quoin::platform::set_ui_lang("en");

    // Put the packs under the sandbox (or copy them from the app bundle first).
    let root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Application Support/quoin/Game");
    let _ = quoin::manager::run(&root, "graphite", None);
}
