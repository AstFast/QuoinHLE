//! Compatibility re-export for the old `crate::gles` path.
//!
//! The implementation is now split across [`crate::gl`]; this module keeps existing
//! `crate::gles::Gles` / `crate::gles::Egl` references building while callers migrate.
pub use crate::gl::*;
