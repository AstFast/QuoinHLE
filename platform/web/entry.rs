// Web entry template for the standalone `quoin-web` crate in this directory.
//
// Build from `platform/web` with `trunk serve` (or `trunk build --release`), after
// `rustup target add wasm32-unknown-unknown` and `cargo install trunk`. The wrapper crate is a
// deliberately small `cdylib`; `quoin` itself stays an rlib, because Windows GNU ld cannot export
// the whole ABI table of the core crate as a DLL.
//
// `manager::build` is shared with the native `run`; `prepare_context` installs the manager's font
// and dark theme. Browser storage is a boundary: `manager.json` goes through `localStorage`, while
// pack files and `std::fs` scanning still have no browser backend. `pack_count` reports that
// boundary to `index.html` so an empty library is explained rather than shown as a blank page.
//
// Lifecycle: `index.html` owns `init()`, `new WebHandle()`, `start()` and `destroy()`. Trunk's
// `inject_scripts = false` keeps it the only initializer.

#![cfg(target_arch = "wasm32")]

use std::cell::Cell;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
    /// The number of rows `start` built, for the page's "no packs in a browser yet" notice.
    packs: Cell<usize>,
}

#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            runner: eframe::WebRunner::new(),
            packs: Cell::new(0),
        }
    }

    pub async fn start(&self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        quoin::platform::set_data_dir("quoin-data");
        // The manager has two languages; pick the one the browser asks for, not a hard-coded default.
        let browser_lang = web_sys::window()
            .and_then(|window| window.navigator().language())
            .unwrap_or_default()
            .to_ascii_lowercase();
        quoin::platform::set_ui_lang(if browser_lang.starts_with("zh") {
            "zh"
        } else {
            "en"
        });
        let manager = quoin::manager::build(std::path::Path::new("Game"), "graphite", None)
            .map_err(|e| JsValue::from_str(&e))?;
        self.packs.set(manager.row_count());
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    quoin::manager::prepare_context(cc);
                    Ok(Box::new(manager))
                }),
            )
            .await
    }

    /// How many packs the manager found. Zero is expected until the browser has a file source:
    /// `library::scan` and `library::boot_at` both read `std::fs`, which wasm32 has no backend for.
    pub fn pack_count(&self) -> usize {
        self.packs.get()
    }

    /// `index.html` calls this on `pagehide`, on a failed start, and before re-starting.
    pub fn destroy(&self) {
        self.runner.destroy();
    }
}
