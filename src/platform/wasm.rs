//! Browser-only storage plumbing.

/// The browser's `localStorage`, if this page is allowed to use it. Private mode and some embedded
/// webviews answer `None`; callers must then say that changes will not be kept.
pub fn web_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
