//! Virtual libraries: classes a handset had preinstalled, served by the host under a library name.
//!
//! A pack asks the shell for a class, and a handset's registry knows which installed module owns it.
//! The sample set ships the app's own image; it cannot carry the system libraries that were burned into
//! the phone. `--lib` loads a real `.mod` for those, but a package may be missing one and some were
//! never dumped. A virtual library is the host's answer for the second case: a named set of classes
//! it builds itself, registered between the real libraries and the app's module.
//!
//! A virtual class names an *interface from the ABI tables*. Creating one is the same
//! `Hle::new_object` the host service table uses, so its slots answer from the host, and a slot the
//! host has no body for refuses by name — the same honest shape as every other missing slot. There
//! is deliberately no way to say "pretend this class is authorised": a library is a class provider,
//! not a policy.

use std::path::{Path, PathBuf};

use crate::abi;
use crate::json::{self, Value};

/// One class a virtual library serves, and the interface it is handed out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualClass {
    pub class: u32,
    pub iface: &'static str,
}

/// A named group of classes the host serves in place of a preinstalled library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualLibrary {
    pub name: String,
    pub classes: Vec<VirtualClass>,
}

impl VirtualLibrary {
    pub fn serves(&self, cls: u32) -> bool {
        self.classes.iter().any(|c| c.class == cls)
    }

    pub fn iface_for(&self, cls: u32) -> Option<&'static str> {
        self.classes
            .iter()
            .find(|c| c.class == cls)
            .map(|c| c.iface)
    }

    /// Add one class, refusing a second interface for a class already served: two answers for one
    /// request is the kind of ambiguity a registry exists to prevent.
    fn add(&mut self, class: u32, iface: &'static str) -> Result<(), String> {
        if let Some(existing) = self.classes.iter().find(|c| c.class == class) {
            if existing.iface == iface {
                return Ok(());
            }
            return Err(format!(
                "virtual library {:?} serves {class:#010x} as both {} and {}",
                self.name, existing.iface, iface
            ));
        }
        self.classes.push(VirtualClass { class, iface });
        Ok(())
    }
}

/// What one run was told about libraries: where to look, which real images to load, and which
/// virtual libraries to register.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryPlan {
    pub search_paths: Vec<PathBuf>,
    pub libs: Vec<String>,
    pub virtual_libs: Vec<VirtualLibrary>,
}

impl LibraryPlan {
    /// Read a run manifest. Relative `lib_paths` are resolved against the manifest's own directory,
    /// which is also searched first, because a manifest and the libraries beside it travel together.
    pub fn read_manifest(path: &Path) -> Result<LibraryPlan, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&text, path)
    }

    pub fn from_json(text: &str, source: &Path) -> Result<LibraryPlan, String> {
        let value = json::parse(text).map_err(|e| format!("{}: {e}", source.display()))?;
        let object = value.as_object().ok_or_else(|| {
            format!(
                "{}: a library manifest must be a JSON object, found {}",
                source.display(),
                value.kind()
            )
        })?;
        let base = source.parent().unwrap_or_else(|| Path::new("."));
        let mut plan = LibraryPlan::default();
        plan.search_paths.push(base.to_path_buf());
        for (key, value) in object.iter() {
            match key.as_str() {
                "lib_paths" => {
                    let items = value.as_array().ok_or_else(|| {
                        format!(
                            "{}: lib_paths must be an array of strings, found {}",
                            source.display(),
                            value.kind()
                        )
                    })?;
                    for item in items {
                        let text = item.as_str().ok_or_else(|| {
                            format!(
                                "{}: a lib_paths entry must be a string, found {}",
                                source.display(),
                                item.kind()
                            )
                        })?;
                        let path = PathBuf::from(text);
                        plan.search_paths.push(if path.is_relative() {
                            base.join(path)
                        } else {
                            path
                        });
                    }
                }
                "libs" => {
                    let items = value.as_array().ok_or_else(|| {
                        format!(
                            "{}: libs must be an array of strings, found {}",
                            source.display(),
                            value.kind()
                        )
                    })?;
                    for item in items {
                        let text = item.as_str().ok_or_else(|| {
                            format!(
                                "{}: a libs entry must be a string, found {}",
                                source.display(),
                                item.kind()
                            )
                        })?;
                        plan.libs.push(text.to_string());
                    }
                }
                "virtual_libs" => {
                    let libs = value.as_object().ok_or_else(|| {
                        format!(
                            "{}: virtual_libs must be an object, found {}",
                            source.display(),
                            value.kind()
                        )
                    })?;
                    for (name, entries) in libs.iter() {
                        let entries = entries.as_array().ok_or_else(|| {
                            format!(
                                "{}: virtual library {name:?} must be an array, found {}",
                                source.display(),
                                entries.kind()
                            )
                        })?;
                        let mut lib = VirtualLibrary {
                            name: name.clone(),
                            classes: Vec::new(),
                        };
                        for entry in entries {
                            let fields = entry.as_object().ok_or_else(|| {
                                format!(
                                    "{}: virtual library {name:?}: a class must be an object, found {}",
                                    source.display(),
                                    entry.kind()
                                )
                            })?;
                            let class =
                                fields.get("class").and_then(class_word).ok_or_else(|| {
                                    format!(
                                        "{}: virtual library {name:?}: class must be a number or a \
                                         \"0x…\" string",
                                        source.display()
                                    )
                                })?;
                            let iface =
                                fields.get("iface").and_then(Value::as_str).ok_or_else(|| {
                                    format!(
                                        "{}: virtual library {name:?}: iface must be a string",
                                        source.display()
                                    )
                                })?;
                            let iface =
                                abi::iface(iface).map(|info| info.name).ok_or_else(|| {
                                    format!(
                                        "{}: virtual library {name:?}: unknown interface {iface}",
                                        source.display()
                                    )
                                })?;
                            if let Some(extra) = fields
                                .iter()
                                .map(|(k, _)| k.as_str())
                                .find(|k| *k != "class" && *k != "iface")
                            {
                                return Err(format!(
                                    "{}: virtual library {name:?}: unknown field {extra:?}",
                                    source.display()
                                ));
                            }
                            lib.add(class, iface)?;
                        }
                        plan.virtual_libs.push(lib);
                    }
                }
                other => {
                    return Err(format!(
                        "{}: unknown field {other:?} in a library manifest",
                        source.display()
                    ));
                }
            }
        }
        Ok(plan)
    }

    /// Fold another plan into this one: search paths and real libraries in order, virtual libraries
    /// merged by name so a manifest and the command line can describe one library together.
    pub fn merge(&mut self, other: LibraryPlan) -> Result<(), String> {
        for path in other.search_paths {
            if !self.search_paths.contains(&path) {
                self.search_paths.push(path);
            }
        }
        for lib in other.libs {
            if !self.libs.contains(&lib) {
                self.libs.push(lib);
            }
        }
        for incoming in other.virtual_libs {
            match self
                .virtual_libs
                .iter_mut()
                .find(|lib| lib.name == incoming.name)
            {
                Some(lib) => {
                    for class in incoming.classes {
                        lib.add(class.class, class.iface)?;
                    }
                }
                None => self.virtual_libs.push(incoming),
            }
        }
        Ok(())
    }

    /// `--virtual-class NAME,CLASS,IFACE`: one class, declared on the command line.
    pub fn add_virtual_class(&mut self, spec: &str) -> Result<(), String> {
        let mut parts = spec.splitn(3, ',');
        let name = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("--virtual-class {spec}: expected NAME,CLASS,IFACE"))?;
        let class_text = parts
            .next()
            .ok_or_else(|| format!("--virtual-class {spec}: expected NAME,CLASS,IFACE"))?;
        let iface_text = parts
            .next()
            .ok_or_else(|| format!("--virtual-class {spec}: expected NAME,CLASS,IFACE"))?;
        let class = parse_class(class_text)
            .ok_or_else(|| format!("--virtual-class {spec}: {class_text:?} is not a class id"))?;
        let iface = abi::iface(iface_text)
            .map(|info| info.name)
            .ok_or_else(|| format!("--virtual-class {spec}: unknown interface {iface_text}"))?;
        match self.virtual_libs.iter_mut().find(|lib| lib.name == name) {
            Some(lib) => lib.add(class, iface),
            None => {
                let mut lib = VirtualLibrary {
                    name: name.to_string(),
                    classes: Vec::new(),
                };
                lib.add(class, iface)?;
                self.virtual_libs.push(lib);
                Ok(())
            }
        }
    }

    /// Resolve one real library: as written when that file exists, else against the search paths.
    pub fn resolve(&self, lib: &str) -> Option<PathBuf> {
        let path = PathBuf::from(lib);
        if path.is_file() {
            return Some(path);
        }
        self.search_paths
            .iter()
            .map(|dir| dir.join(lib))
            .find(|candidate| candidate.is_file())
    }

    /// The real libraries this run will load, in order, with the name that failed named when one
    /// cannot be found.
    pub fn resolved_libs(&self) -> Result<Vec<PathBuf>, String> {
        self.libs
            .iter()
            .map(|lib| {
                self.resolve(lib).ok_or_else(|| {
                    let looked: Vec<String> = self
                        .search_paths
                        .iter()
                        .map(|dir| dir.display().to_string())
                        .collect();
                    format!(
                        "--lib {lib}: not found{}",
                        if looked.is_empty() {
                            String::new()
                        } else {
                            format!("; searched {}", looked.join(", "))
                        }
                    )
                })
            })
            .collect()
    }
}

/// A class id from a JSON value: a number, or a string in decimal or `0x` hex.
fn class_word(value: &Value) -> Option<u32> {
    if let Some(number) = value.as_u64() {
        return u32::try_from(number).ok();
    }
    parse_class(value.as_str()?)
}

fn parse_class(text: &str) -> Option<u32> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

#[cfg(test)]
#[path = "test/virtual_lib.rs"]
mod tests;
