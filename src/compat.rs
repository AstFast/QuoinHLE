//! Per-package compatibility declarations, read from a sidecar beside the `.mod`.
//!
//! A guest pack can be built against a firmware generation whose ABI differs from the modern
//! one in ways no image scan can detect: the same `CONVERTBMP` call is answered with a pixel run
//! by an old AppGen and with an `IBitmap` by a newer one. The declaration is data, not a host
//! special case: any pack can carry `<stem>.compat.json`, and a run without one keeps every
//! modern answer.
//!
//! Layout 1.2:
//!
//! ```json
//! {
//!   "version": "1.2",
//!   "platform": "classic-1.2.3",
//!   "carrier": "ktf",
//!   "convertbmp": "pixels"
//! }
//! ```
//!
//! Layout 1.1:
//!
//! ```json
//! {
//!   "version": "1.1",
//!   "convertbmp": "pixels",
//!   "convertbmp_scope": "constructor",
//!   "slack_fill": "newline"
//! }
//! ```
//!
//! `convertbmp` is `bitmap` (default) or `pixels` (the pre-1.2 OEM answer). `convertbmp_scope`
//! is `always` (default) or `constructor`: a pack that reads the answer as pixels while it is
//! being built but as an `IBitmap` once `EVT_APP_START` arrives declares `constructor`.
//!
//! `slack_fill` is `keep` (default) or `newline`. It names what the unused bytes at the end of a
//! heap payload read as. Some 1.x line readers walk past the last byte of a text block looking
//! for `0x0A` instead of checking a length; `newline` makes that walk stop at the block's own
//! boundary. A pack without the key keeps the bytes exactly as before.
//!
//! `platform` and `carrier` (layout 1.2) name the generation and the operator's private ABI a pack
//! was built against — `"classic-1.2.3"` and `"ktf"`, in the spellings `--list-platforms` prints.
//! They are kept as text here on purpose: this file's job is to read a declaration, and whether a
//! name exists is decided by [`crate::spec`], where the error can say which names do. A declaration
//! naming nothing keeps the default generation, which is the machine this host already is.

use std::path::Path;

use crate::json::{self, Map, Value};

pub const VERSION_MAJOR: u64 = 1;
pub const VERSION_MINOR: u64 = 2;
pub const VERSION: &str = "1.2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertBmp {
    Bitmap,
    Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Always,
    Constructor,
}

/// What the spare bytes at the end of an allocation read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlackFill {
    /// Leave them as the heap found them, which is what a pack without a declaration gets.
    Keep,
    /// Fill them with `0x0A`, so a line reader that walks past its buffer stops at the boundary.
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compat {
    pub convertbmp: ConvertBmp,
    pub scope: Scope,
    pub slack_fill: SlackFill,
    /// The generation the pack was built against, as written. `None` keeps the default.
    pub platform: Option<String>,
    /// The operator extension set the pack expects, as written. `None` keeps the default.
    pub carrier: Option<String>,
}

impl Default for Compat {
    fn default() -> Self {
        Self {
            convertbmp: ConvertBmp::Bitmap,
            scope: Scope::Always,
            slack_fill: SlackFill::Keep,
            platform: None,
            carrier: None,
        }
    }
}

/// The sidecar a pack may carry: `foo.mod` → `foo.compat.json`.
pub fn path_for(mod_path: &Path) -> std::path::PathBuf {
    mod_path.with_extension("compat.json")
}

/// Read the declaration beside `mod_path`, or the default when there is none.
pub fn load(mod_path: &Path) -> Result<Compat, String> {
    let path = path_for(mod_path);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Compat::default()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn parse(text: &str) -> Result<Compat, String> {
    let value = json::parse(text).map_err(|e| e.to_string())?;
    let Value::Object(map) = value else {
        return Err(format!("expected an object, found {}", value.kind()));
    };
    check_keys(
        &map,
        &[
            "version",
            "convertbmp",
            "convertbmp_scope",
            "slack_fill",
            "platform",
            "carrier",
            "_note",
        ],
    )?;
    let version = map
        .get("version")
        .and_then(Value::as_str)
        .ok_or("missing \"version\"")?;
    let (major, minor) = split_version(version)?;
    if major != VERSION_MAJOR {
        return Err(format!(
            "layout {version} is not one this build reads (it reads {VERSION_MAJOR}.x)"
        ));
    }
    if minor > VERSION_MINOR {
        return Err(format!(
            "layout {version} is newer than this build's {VERSION}"
        ));
    }
    let convertbmp = match map.get("convertbmp") {
        None => ConvertBmp::Bitmap,
        Some(v) => match v.as_str() {
            Some("bitmap") => ConvertBmp::Bitmap,
            Some("pixels") => ConvertBmp::Pixels,
            Some(other) => {
                return Err(format!(
                    "\"convertbmp\" is \"{other}\"; expected \"bitmap\" or \"pixels\""
                ))
            }
            None => return Err(format!("\"convertbmp\" is {}", v.kind())),
        },
    };
    let scope = match map.get("convertbmp_scope") {
        None => Scope::Always,
        Some(v) => match v.as_str() {
            Some("always") => Scope::Always,
            Some("constructor") => Scope::Constructor,
            Some(other) => {
                return Err(format!(
                    "\"convertbmp_scope\" is \"{other}\"; expected \"always\" or \"constructor\""
                ))
            }
            None => return Err(format!("\"convertbmp_scope\" is {}", v.kind())),
        },
    };
    let slack_fill = match map.get("slack_fill") {
        None => SlackFill::Keep,
        Some(v) => match v.as_str() {
            Some("keep") => SlackFill::Keep,
            Some("newline") => SlackFill::Newline,
            Some(other) => {
                return Err(format!(
                    "\"slack_fill\" is \"{other}\"; expected \"keep\" or \"newline\""
                ))
            }
            None => return Err(format!("\"slack_fill\" is {}", v.kind())),
        },
    };
    // Kept as written: `spec::spec_by_name` is the one place that decides whether a name exists, and
    // it can say which names do. An empty string is not a name, so it is refused here rather than
    // silently resolving to the default.
    let name = |key: &str| -> Result<Option<String>, String> {
        match map.get(key) {
            None => Ok(None),
            Some(v) => match v.as_str() {
                Some(s) if !s.trim().is_empty() => Ok(Some(s.to_string())),
                Some(_) => Err(format!("\"{key}\" is empty")),
                None => Err(format!("\"{key}\" is {}", v.kind())),
            },
        }
    };
    let platform = name("platform")?;
    let carrier = name("carrier")?;
    Ok(Compat {
        convertbmp,
        scope,
        slack_fill,
        platform,
        carrier,
    })
}

fn check_keys(map: &Map, allowed: &[&str]) -> Result<(), String> {
    for (key, _) in map.iter() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("\"{key}\" is not a key of this layout"));
        }
    }
    Ok(())
}

fn split_version(text: &str) -> Result<(u64, u64), String> {
    let (major, minor) = text
        .split_once('.')
        .ok_or_else(|| format!("\"version\" is \"{text}\"; expected MAJOR.MINOR"))?;
    let major = major
        .parse()
        .map_err(|_| format!("\"version\" is \"{text}\"; expected MAJOR.MINOR"))?;
    let minor = minor
        .parse()
        .map_err(|_| format!("\"version\" is \"{text}\"; expected MAJOR.MINOR"))?;
    Ok((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_declaration_names_the_two_answers() {
        assert_eq!(
            parse(r#"{"version":"1.1","convertbmp":"pixels","convertbmp_scope":"constructor"}"#)
                .unwrap(),
            Compat {
                convertbmp: ConvertBmp::Pixels,
                scope: Scope::Constructor,
                ..Compat::default()
            }
        );
        assert_eq!(parse(r#"{"version":"1.1"}"#).unwrap(), Compat::default());
        assert_eq!(
            parse(r#"{"version":"1.1","slack_fill":"newline"}"#).unwrap(),
            Compat {
                slack_fill: SlackFill::Newline,
                ..Compat::default()
            }
        );
    }

    #[test]
    fn the_platform_and_carrier_are_kept_as_written() {
        let got = parse(
            r#"{"version":"1.2","platform":"classic-1.2.3","carrier":"ktf","slack_fill":"newline"}"#,
        )
        .unwrap();
        assert_eq!(got.platform.as_deref(), Some("classic-1.2.3"));
        assert_eq!(got.carrier.as_deref(), Some("ktf"));
        assert_eq!(got.slack_fill, SlackFill::Newline);
        // A declaration that names neither keeps both defaults.
        assert_eq!(parse(r#"{"version":"1.2"}"#).unwrap(), Compat::default());
        // Resolving the names is `spec`'s job, not this file's: an unknown name is still a
        // syntactically fine declaration, and it fails where the message can list the real ones.
        assert_eq!(
            parse(r#"{"version":"1.2","platform":"classic-9.9"}"#)
                .unwrap()
                .platform
                .as_deref(),
            Some("classic-9.9")
        );
    }

    #[test]
    fn an_empty_or_non_string_platform_is_refused() {
        assert!(parse(r#"{"version":"1.2","platform":""}"#).is_err());
        assert!(parse(r#"{"version":"1.2","carrier":"  "}"#).is_err());
        assert!(parse(r#"{"version":"1.2","platform":7}"#).is_err());
    }

    #[test]
    fn an_unknown_key_or_version_is_refused() {
        assert!(parse(r#"{"version":"1.1","convertbmp":"raw"}"#).is_err());
        assert!(parse(r#"{"version":"1.1","surprise":1}"#).is_err());
        assert!(parse(r#"{"version":"1.1","slack_fill":"zero"}"#).is_err());
        assert!(parse(r#"{"version":"2.0"}"#).is_err());
        assert!(parse(r#"{"version":"1.3"}"#).is_err());
    }

    #[test]
    fn a_layout_1_0_declaration_still_reads() {
        assert_eq!(parse(r#"{"version":"1.0"}"#).unwrap(), Compat::default());
    }
}
