// @generated from local ABI definitions. DO NOT EDIT.
// Re-run the generator instead of patching this file.
#![allow(dead_code, non_upper_case_globals, non_snake_case)]

mod consts;
mod enums;
mod ifaces;
mod meta;
mod structs;

pub use self::{consts::*, enums::*, ifaces::*, meta::*, structs::*};

// ---- carried in from the spec's own validation ----
// The platform COMPILE_ASSERTs evaluated true: 19 of 19 (false: 0, unevaluable: 0)
const _: () = assert!(SIZE_AEE_MD2_CTX == 148);  // The platform publishes AEECLSID_MD2Ctx_SIZE for this context buffer
const _: () = assert!(SIZE_AEE_MD5_CTX == 88);  // The platform publishes AEECLSID_MD5Ctx_SIZE for this context buffer

const _: () = assert!(POINTER_SIZE == 4 && VTABLE_SLOT_SIZE == 4);
const _: () = assert!(IFACE_COUNT > 0 && VTABLE_SLOT_TOTAL > 0);
const _: () = assert!(IFACE_COUNT < (1 << 20) && MAX_METHODS < (1 << 12));
pub const MAX_METHODS: usize = 148;
