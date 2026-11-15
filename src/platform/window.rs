//! Window capability by platform.

/// Whether a running game can be given a window of its own. Mobile and Web embed the view in the
/// host surface instead.
pub const SEPARATE_WINDOW_POSSIBLE: bool = !cfg!(any(
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
));
