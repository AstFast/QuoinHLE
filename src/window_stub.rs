//! The raw front end, on a target that has no raw window.
//!
//! [`crate::window`] is a Win32 window: it links `user32` and `gdi32` directly and carries no
//! conditional compilation of its own, so on any other target that file cannot even be built — not
//! because of the code in it, which is portable, but because of the two `#[link]` attributes at its
//! top, which ask the linker for libraries only Windows has. `src/lib.rs` therefore picks between
//! the two modules by target, and this is the one a non-Windows build gets.
//!
//! It exists as a module rather than the module being absent for two reasons, and both are about
//! what the rest of the tree has to say:
//!
//! - `main.rs` shapes a run around `window::show` unconditionally. Absent, every one of those lines
//!   would need a `cfg`, and `--window raw` would become a hole in the command line rather than an
//!   answer to it.
//! - `crate::window` is linked from other modules' documentation (`egui_window`, `handset`), and a
//!   doc link to a module that does not exist on this target is a broken link.
//!
//! So the door stays where it is and what is behind it says what is true. **This is the shape a
//! second non-egui front end should copy**: a backend that hands the runtime straight back with a
//! reason, rather than one the caller has to find through `cfg`.

use crate::control::Control;
use crate::runtime::{Dispatched, Runtime};

/// Show `rt` in a window and drive it from the keyboard, until the window is closed.
///
/// There is no window here: the raw front end is the Win32 one and this build has not got it. The
/// runtime comes straight back with the reason, because the caller reads its report out of what it
/// is handed — the contract the real front end keeps on every path out of itself, `Err` included.
pub fn show(
    rt: Runtime,
    _title: &str,
    _soundfont: Option<&str>,
    _audio_backend: crate::audio::AudioBackend,
    _control: Option<Control>,
) -> (Result<Dispatched, String>, Runtime) {
    (
        Err("--window raw needs Windows: this build has the egui front end".into()),
        rt,
    )
}
