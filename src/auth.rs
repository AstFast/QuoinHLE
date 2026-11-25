//! Authorization profiles for paid/premium content, and the one decision point a virtual
//! `iPremium` provider consults.
//!
//! The sample set is EZ-appli: A carrier's paid store. Some titles check a subscription through the
//! preinstalled iPremium middleware, and a handset without that subscription refuses to start them.
//! A profile says, locally and explicitly, which titles this run may open and under what identity.
//! It is deliberately **not** a hidden bypass: no profile means the reference behavior (a real library
//! answers the class, or the shell refuses it), the default is `deny`, and every decision names the
//! app and the simulated subscriber.
//!
//! Scope: use it for content you are entitled to run (a defunct service, your own purchases). The
//! host ships no keys, signatures or license data, and this is not a way to reach a live store.

use std::path::Path;

use crate::json::{self, Value};

/// The carrier iPremium's class, as the guest packs ask for it. The private middleware lives in a
/// preinstalled library; when a run has no image for it, the authorization provider answers instead.
pub const AEECLSID_KDDIPREMIUM: u32 = 0x01088b1d;

/// au's paid-content (`collect`) class, the one a Konami/Sony title asks for before it will start.
///
/// Present in the carrier build (a class-id table beside the
/// `fs:/mod/` path table) and absent from UBJQ40405 — a platform class of that au generation. The
/// packs that ask carry no image of it: `a guest module`'s own `AEEClsCreateInstance` answers every
/// class but its applet with `AEE_EFAILED` and never hands one back to the shell, so on a handset the
/// shell has to answer before the module is asked at all.
pub const AEECLSID_AU_COLLECT: u32 = 0x0102f14b;

/// What a run may do with one title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Refused: the pack sees the same failure a handset without a subscription produces.
    Deny,
    /// Reserved for a trial answer once one is known. It does not grant access today.
    Trial,
    /// Allowed: the authorization slot answers success.
    Authorized,
}

impl Decision {
    pub fn name(self) -> &'static str {
        match self {
            Decision::Deny => "deny",
            Decision::Trial => "trial",
            Decision::Authorized => "authorized",
        }
    }

    fn parse(text: &str) -> Option<Decision> {
        match text {
            "deny" => Some(Decision::Deny),
            "trial" => Some(Decision::Trial),
            "authorized" | "authorised" => Some(Decision::Authorized),
            _ => None,
        }
    }

    /// Whether the authorization slot reports success.
    pub fn allows(self) -> bool {
        matches!(self, Decision::Authorized)
    }
}

/// One title in the profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppRule {
    pub id: String,
    pub mode: Decision,
    pub note: Option<String>,
}

/// A run's authorization profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The simulated The carrier subscriber number, also fed to `AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO`.
    pub subscriber_number: Option<String>,
    /// The simulated subscriber user id, kept for the audit line.
    pub user_id: Option<String>,
    /// What an unlisted title gets. `deny` unless the file says otherwise.
    pub default: Decision,
    pub apps: Vec<AppRule>,
}

impl Profile {
    pub fn read(path: &Path) -> Result<Profile, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&text, path)
    }

    pub fn from_json(text: &str, source: &Path) -> Result<Profile, String> {
        let value = json::parse(text).map_err(|e| format!("{}: {e}", source.display()))?;
        let object = value.as_object().ok_or_else(|| {
            format!(
                "{}: an authorization profile must be a JSON object, found {}",
                source.display(),
                value.kind()
            )
        })?;
        let mut profile = Profile {
            subscriber_number: None,
            user_id: None,
            default: Decision::Deny,
            apps: Vec::new(),
        };
        for (key, value) in object.iter() {
            match key.as_str() {
                "subscriber_number" => {
                    profile.subscriber_number = Some(text_field(value, source, key)?.to_string());
                }
                "user_id" => {
                    profile.user_id = Some(text_field(value, source, key)?.to_string());
                }
                "default" => {
                    profile.default = decision_field(value, source, key)?;
                }
                "apps" => {
                    let items = value.as_array().ok_or_else(|| {
                        format!(
                            "{}: apps must be an array, found {}",
                            source.display(),
                            value.kind()
                        )
                    })?;
                    for (index, item) in items.iter().enumerate() {
                        let fields = item.as_object().ok_or_else(|| {
                            format!(
                                "{}: apps[{index}] must be an object, found {}",
                                source.display(),
                                item.kind()
                            )
                        })?;
                        let id = fields
                            .get("id")
                            .ok_or_else(|| {
                                format!("{}: apps[{index}]: id is required", source.display())
                            })
                            .and_then(|v| text_field(v, source, &format!("apps[{index}].id")))?;
                        let mode = match fields.get("mode") {
                            Some(value) => {
                                decision_field(value, source, &format!("apps[{index}].mode"))?
                            }
                            None => Decision::Authorized,
                        };
                        let note = match fields.get("note") {
                            Some(value) => Some(
                                text_field(value, source, &format!("apps[{index}].note"))?
                                    .to_string(),
                            ),
                            None => None,
                        };
                        if let Some(extra) = fields
                            .iter()
                            .map(|(k, _)| k.as_str())
                            .find(|k| !matches!(*k, "id" | "mode" | "note"))
                        {
                            return Err(format!(
                                "{}: apps[{index}]: unknown field {extra:?}",
                                source.display()
                            ));
                        }
                        if profile.apps.iter().any(|rule| rule.id == id) {
                            return Err(format!(
                                "{}: apps[{index}]: {id:?} appears twice",
                                source.display()
                            ));
                        }
                        profile.apps.push(AppRule {
                            id: id.to_string(),
                            mode,
                            note,
                        });
                    }
                }
                other => {
                    return Err(format!(
                        "{}: unknown field {other:?} in an authorization profile",
                        source.display()
                    ));
                }
            }
        }
        Ok(profile)
    }

    /// The decision for one title: its own rule, else the profile's default.
    pub fn decision_for(&self, app_id: &str) -> Decision {
        self.apps
            .iter()
            .find(|rule| rule.id == app_id)
            .map(|rule| rule.mode)
            .unwrap_or(self.default)
    }
}

fn text_field<'a>(value: &'a Value, source: &Path, field: &str) -> Result<&'a str, String> {
    value.as_str().ok_or_else(|| {
        format!(
            "{}: {field} must be a string, found {}",
            source.display(),
            value.kind()
        )
    })
}

fn decision_field(value: &Value, source: &Path, field: &str) -> Result<Decision, String> {
    let text = text_field(value, source, field)?;
    Decision::parse(text).ok_or_else(|| {
        format!(
            "{}: {field} is deny, trial or authorized, not {text:?}",
            source.display()
        )
    })
}

/// The vtable slots the virtual premium object answers, in the order the reference object lays them.
pub mod premium_slots {
    pub const ADD_REF: u32 = 0;
    pub const RELEASE: u32 = 1;
    pub const QUERY_INTERFACE: u32 = 2;
    pub const AUTH: u32 = 3;
}

/// The slots of [`AEECLSID_AU_COLLECT`] this host answers. See [`crate::abi::COLLECT_METHODS`] for
/// which of them any pack has been seen to reach, and why the rest are named by position.
pub mod collect_slots {
    pub const ADD_REF: u32 = 0;
    pub const RELEASE: u32 = 1;
    pub const QUERY_INTERFACE: u32 = 2;
    pub const COMMAND: u32 = 9;
}

#[cfg(test)]
#[path = "test/auth.rs"]
mod tests;
