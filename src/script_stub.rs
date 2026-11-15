//! No-op scripting for builds without a native Lua runtime (Web).
//!
//! The real `script.rs` is desktop-only: `mlua`'s vendored Lua cannot be built for
//! `wasm32-unknown-unknown`. This module keeps the same small API so the runtime can compile and
//! simply reports that no script package was loaded.

use std::path::Path;

use crate::net::Answer;
use crate::runtime::Runtime;

/// What a package declares before any of its code runs.
#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub requests_writes: bool,
    pub digest: u64,
}

impl Declaration {
    /// There is no script loader in this build, so no package is discovered.
    pub fn inspect(_root: &Path, _app_cls: u32) -> Result<Option<Self>, String> {
        Ok(None)
    }

    pub fn summary(&self) -> String {
        self.name.clone()
    }
}

pub fn is_trusted(_root: &Path, _app_cls: u32, _digest: u64) -> bool {
    false
}

pub fn record_trust(_root: &Path, _app_cls: u32, _digest: u64) -> Result<(), String> {
    Ok(())
}

pub fn ask_consent(_declaration: &Declaration) -> bool {
    false
}

pub fn show_popup(_title: &str, _message: &str) {}

/// One host hook. Kept identical to the desktop module so `runtime.rs` needs no cfg branches.
pub enum Hook {
    Start,
    Event { evt: u32, wp: u32, dwp: u32 },
    Frame(u32),
    Stop,
    Net(String),
}

/// A scripting host that never loads a package.
pub struct ScriptHost;

impl ScriptHost {
    pub fn load(_root: &Path, _app_cls: u32, _allow_writes: bool) -> Result<Option<Self>, String> {
        Ok(None)
    }

    pub fn run(&mut self, _hook: Hook, _rt: &mut Runtime) -> Result<Option<Answer>, String> {
        Ok(None)
    }
}
