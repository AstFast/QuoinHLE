//! `IHtmlViewer` option ids and their error flags.
//!
//! These are the one deliberately hand-written part of the ABI surface. The generator
//! (`tools/gen_abi.py`) reads the published headers through `spec/abi_extract.py`, which harvests
//! `enum {... }` members and plain integer `#define`s. The header declares the
//! `HVOPT_*` ids as **aliases of `WEBOPT_*` arithmetic** (`#define HVOPT_N_INDENT
//! (WEBOPT_32BIT | 0x1001)` and friends), so the extractor sees no integer literal to
//! harvest: `src/abi/generated/consts.rs` carries `WEBOPT_32BIT`, `WEBOPT_SZ` and
//! `WEBOPT_INTERFACE`, but not a single concrete `HVOPT_*`. The headers themselves are
//! proprietary and are not vendored here, so re-running the generator without them cannot
//! recover these either.
//!
//! Keeping the aliases in one hand-written module (rather than in the renderer, where
//! they used to sit) gives them a single authority and lets a test check them against the
//! generated `WEBOPT_*` bases. The three bases below come from `consts.rs`; the low halves
//! are the ids the header gives. Every constant is `i32` because `HVOpt` is
//! signed `int32` on the guest side and the option table is keyed by that word.
//!
//! `HVERR_*` is **not** defined here. The header's error flags are not in the generated
//! tables either, and no copy of the header is in the tree to check the bit values against;
//! inventing them would put a wrong number behind a get-only ABI slot. Until the values can
//! be sourced, `HVOPT_N_ERROR` honestly reports 0 (see `Runtime::ihtmlviewer`).

use crate::abi::generated as g;

/// `WEBOPT_32BIT` ids: numeric options.
pub const HVOPT_N_INDENT: i32 = (g::WEBOPT_32BIT | 0x1001) as i32;
pub const HVOPT_N_VSPACE: i32 = (g::WEBOPT_32BIT | 0x1002) as i32;
pub const HVOPT_N_SBWIDTH: i32 = (g::WEBOPT_32BIT | 0x1003) as i32;
/// Get-only: the `HVERR_*` flags of the last load.
pub const HVOPT_N_ERROR: i32 = (g::WEBOPT_32BIT | 0x1004) as i32;
pub const HVOPT_RGB_TEXT: i32 = (g::WEBOPT_32BIT | 0x1101) as i32;
pub const HVOPT_RGB_LINK: i32 = (g::WEBOPT_32BIT | 0x1102) as i32;

/// `WEBOPT_SZ` ids: the value is a `char *` in guest memory, not a number.
pub const HVOPT_SZ_TYPE: i32 = (g::WEBOPT_SZ | 0x1001) as i32;
pub const HVOPT_SZ_URL: i32 = (g::WEBOPT_SZ | 0x1002) as i32;

/// `WEBOPT_INTERFACE` ids: the value is an interface pointer.
pub const HVOPT_PTR_IWEB: i32 = (g::WEBOPT_INTERFACE | 0x1003) as i32;

/// All nine ids, for the table check below and for callers that want to enumerate them.
pub const HVOPT_IDS: &[(&str, i32)] = &[
    ("HVOPT_N_INDENT", HVOPT_N_INDENT),
    ("HVOPT_N_VSPACE", HVOPT_N_VSPACE),
    ("HVOPT_N_SBWIDTH", HVOPT_N_SBWIDTH),
    ("HVOPT_N_ERROR", HVOPT_N_ERROR),
    ("HVOPT_RGB_TEXT", HVOPT_RGB_TEXT),
    ("HVOPT_RGB_LINK", HVOPT_RGB_LINK),
    ("HVOPT_SZ_TYPE", HVOPT_SZ_TYPE),
    ("HVOPT_SZ_URL", HVOPT_SZ_URL),
    ("HVOPT_PTR_IWEB", HVOPT_PTR_IWEB),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_match_the_generated_webopt_bases() {
        // The generated table is the authority for the three ranges. If someone edits a
        // base there and forgets these aliases, this is where the drift shows up.
        assert_eq!(g::WEBOPT_32BIT, 0x0002_0000);
        assert_eq!(g::WEBOPT_SZ, 0x0001_0000);
        assert_eq!(g::WEBOPT_INTERFACE, 0x0003_0000);
        assert_eq!(HVOPT_N_INDENT, 0x0002_1001);
        assert_eq!(HVOPT_N_VSPACE, 0x0002_1002);
        assert_eq!(HVOPT_N_SBWIDTH, 0x0002_1003);
        assert_eq!(HVOPT_N_ERROR, 0x0002_1004);
        assert_eq!(HVOPT_RGB_TEXT, 0x0002_1101);
        assert_eq!(HVOPT_RGB_LINK, 0x0002_1102);
        assert_eq!(HVOPT_SZ_TYPE, 0x0001_1001);
        assert_eq!(HVOPT_SZ_URL, 0x0001_1002);
        assert_eq!(HVOPT_PTR_IWEB, 0x0003_1003);
    }

    #[test]
    fn generated_table_has_no_conflicting_hvopt() {
        // If a future generator run does harvest the header aliases, this fires and the
        // hand-written module can be retired in favour of the generated constants.
        assert!(!g::CONST_TABLE.iter().any(|c| c.name.starts_with("HVOPT_")));
    }
}
