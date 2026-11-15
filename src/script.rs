//! Per-application Lua 5.4 scripting.
//!
//! A package lives under `scripts/<app-class>/` and is selected by the applet class the module
//! actually started. `manifest.lua` returns a table of script entries:
//!
//! ```lua
//! return {
//!   scripts = {
//!     { file = "events.lua", hooks = { "on_start", "on_event", "on_frame", "on_stop" } },
//!     { file = "net.lua",    hooks = { "on_net_request" } },
//!   },
//! }
//! ```
//!
//! Each script file returns a table of functions. The host calls every entry that declared the
//! hook, in manifest order. A hook may read and write guest memory and CPU registers; scripts are
//! trusted local files, not a security boundary.
//!
//! The `host`-side API is deliberately small but complete enough to inspect and drive a run:
//! `mem.read8/16/32`, `mem.write8/16/32`, `mem.read/write`, `reg.get/set`, `log`, `app.cls`,
//! `app.module`, `clock.ms`. `on_net_request(url)` may return
//! `{ status = 200, content_type = "application/json", body = "..." }` to answer `IWeb` locally.

use std::path::{Path, PathBuf};

use mlua::{Function, HookTriggers, Lua, RegistryKey, Table, Value, Variadic};

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
    /// Stable FNV-1a digest of `manifest.lua`; a trust token is bound to this exact package version.
    pub digest: u64,
}

impl Declaration {
    /// Read only `manifest.lua`; no script file is executed. `None` means this app has no package.
    pub fn inspect(root: &Path, app_cls: u32) -> Result<Option<Self>, String> {
        let dir = root.join(format!("{app_cls:08x}"));
        let manifest = dir.join("manifest.lua");
        if !manifest.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(&manifest)
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024)
            .map_err(|e| format!("script manifest: {e}"))?;
        lua.load("io=nil; os=nil; package=nil; debug=nil; dofile=nil; loadfile=nil; require=nil")
            .set_name("quoin-manifest-safe")
            .exec()
            .map_err(|e| format!("script manifest: {e}"))?;
        let spec: Table = lua
            .load(&bytes)
            .set_name(manifest.to_string_lossy().as_ref())
            .eval()
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
        let declare: Option<Table> = spec
            .get("declare")
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
        let get = |key: &str, fallback: &str| -> String {
            declare
                .as_ref()
                .and_then(|table| table.get::<Option<String>>(key).ok().flatten())
                .unwrap_or_else(|| fallback.to_string())
        };
        let capabilities = declare
            .as_ref()
            .and_then(|table| table.get::<Option<Table>>("capabilities").ok().flatten())
            .map(|list| {
                list.sequence_values::<String>()
                    .collect::<mlua::Result<Vec<_>>>()
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let requests_writes = declare
            .as_ref()
            .and_then(|table| table.get::<Option<bool>>("writes").ok().flatten())
            .unwrap_or(false);
        Ok(Some(Self {
            name: get("name", &format!("{app_cls:08x}")),
            version: get("version", "unversioned"),
            author: get("author", "unknown"),
            description: get("description", ""),
            capabilities,
            requests_writes,
            digest: fnv1a(&bytes),
        }))
    }

    pub fn summary(&self) -> String {
        let capabilities = if self.capabilities.is_empty() {
            "none".to_string()
        } else {
            self.capabilities.join(", ")
        };
        format!(
            "{} {} by {}: {} (capabilities: {}; writes: {})",
            self.name,
            self.version,
            self.author,
            self.description,
            capabilities,
            self.requests_writes
        )
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn trust_path(app_cls: u32) -> PathBuf {
    crate::library::program_data_dir()
        .unwrap_or_else(|| PathBuf::from("data"))
        .join("scripts")
        .join("trust")
        .join(format!("{app_cls:08x}.token"))
}

fn legacy_trust_path(root: &Path, app_cls: u32) -> PathBuf {
    root.join(".trust").join(format!("{app_cls:08x}.token"))
}

/// Whether this exact manifest was accepted before. A changed manifest has a new digest and asks
/// again; deleting the token file revokes the consent. A token written by an older build under
/// `<scripts>/.trust/` is moved to the program data directory on first sight.
pub fn is_trusted(root: &Path, app_cls: u32, digest: u64) -> bool {
    let wanted = format!("{digest:016x}:");
    let path = trust_path(app_cls);
    if let Ok(text) = std::fs::read_to_string(&path) {
        return text.trim_start().starts_with(&wanted);
    }
    let legacy = legacy_trust_path(root, app_cls);
    let Ok(text) = std::fs::read_to_string(&legacy) else {
        return false;
    };
    if !text.trim_start().starts_with(&wanted) {
        return false;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::copy(&legacy, &path);
    true
}

/// Write the one-time credential after the user pressed "Yes".
pub fn record_trust(_root: &Path, app_cls: u32, digest: u64) -> Result<(), String> {
    let path = trust_path(app_cls);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("script trust {}: {e}", parent.display()))?;
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        ^ u128::from(std::process::id());
    std::fs::write(&path, format!("{digest:016x}:{nonce:032x}\n"))
        .map_err(|e| format!("script trust {}: {e}", path.display()))
}

/// Native confirmation for a package that asks for write access.
pub fn ask_consent(declaration: &Declaration) -> bool {
    let text = format!(
        "{}\n\nThis Lua package requests guest memory/register write access. Allow it for this \
         version? The answer is remembered in data/scripts/trust/; delete that file to revoke it.",
        declaration.summary()
    );
    matches!(
        rfd::MessageDialog::new()
            .set_title("Lua script requests write access")
            .set_description(text)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show(),
        rfd::MessageDialogResult::Yes
    )
}

/// The Lua `popup(title, message)` API.
pub fn show_popup(title: &str, message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// One host hook. `Net` carries the guest's URL because its return value is an answer.
pub enum Hook {
    Start,
    Event { evt: u32, wp: u32, dwp: u32 },
    Frame(u32),
    Stop,
    Net(String),
}

struct Entry {
    hooks: Vec<String>,
    table: RegistryKey,
}

/// The Lua state and the loaded tables for one applet run.
pub struct ScriptHost {
    lua: Lua,
    entries: Vec<Entry>,
    /// `--script-writes`: whether the destructive memory/register APIs are exposed.
    writes: bool,
}

/// The wrappers are installed once. `__api` is replaced for each hook so the functions below can
/// call the live runtime without keeping a Rust borrow across Lua's lifetime.
const PRELUDE: &str = r#"
local function q(...) return __api(...) end
mem = {
  read8  = function(a) return q("mem.read8", a) end,
  read16 = function(a) return q("mem.read16", a) end,
  read32 = function(a) return q("mem.read32", a) end,
  write8 = function(a, v) return q("mem.write8", a, v) end,
  write16 = function(a, v) return q("mem.write16", a, v) end,
  write32 = function(a, v) return q("mem.write32", a, v) end,
  read   = function(a, n) return q("mem.read", a, n) end,
  write  = function(a, s) return q("mem.write", a, s) end,
}
reg = {
  get = function(n) return q("reg.get", n) end,
  set = function(n, v) return q("reg.set", n, v) end,
}
log = function(s) return q("log", tostring(s)) end
popup = function(title, message) return q("popup", tostring(title), tostring(message)) end
app = {
  cls = function() return q("app.cls") end,
  module = function() return q("app.module") end,
}
clock = { ms = function() return q("clock.ms") end }
"#;

impl ScriptHost {
    /// Load `root/<app-cls>/manifest.lua`, if that package exists. A missing package is not an
    /// error: most packs have no scripts.
    pub fn load(root: &Path, app_cls: u32, allow_writes: bool) -> Result<Option<Self>, String> {
        let dir = root.join(format!("{app_cls:08x}"));
        let manifest = dir.join("manifest.lua");
        if !manifest.is_file() {
            return Ok(None);
        }

        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024)
            .map_err(|e| format!("script memory limit: {e}"))?;
        lua.load(PRELUDE)
            .set_name("quoin-prelude")
            .exec()
            .map_err(|e| format!("script prelude: {e}"))?;
        if !allow_writes {
            // The default package is a reader. Removing the standard library's process/file doors
            // is the first half; the API dispatch below refuses the destructive guest operations.
            lua.load(
                "io=nil; os=nil; package=nil; debug=nil; dofile=nil; loadfile=nil; require=nil",
            )
            .set_name("quoin-safe-mode")
            .exec()
            .map_err(|e| format!("script safe mode: {e}"))?;
        }
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(1_000_000),
            |_lua, _debug| {
                Err(mlua::Error::RuntimeError(
                    "script instruction budget exceeded".to_string(),
                ))
            },
        );

        let spec_text = std::fs::read_to_string(&manifest)
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
        let spec: Table = lua
            .load(&spec_text)
            .set_name(manifest.to_string_lossy().as_ref())
            .eval()
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
        let scripts: Table = spec
            .get("scripts")
            .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;

        let mut entries = Vec::new();
        for item in scripts.sequence_values::<Table>() {
            let item = item.map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
            let file: String = item
                .get("file")
                .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?;
            let hooks: Vec<String> = match item.get::<Option<Table>>("hooks").ok().flatten() {
                Some(list) => list
                    .sequence_values::<String>()
                    .collect::<mlua::Result<Vec<_>>>()
                    .map_err(|e| format!("script manifest {}: {e}", manifest.display()))?,
                None => vec![
                    "on_start".to_string(),
                    "on_event".to_string(),
                    "on_frame".to_string(),
                    "on_stop".to_string(),
                    "on_net_request".to_string(),
                ],
            };
            let path = dir.join(&file);
            let source = std::fs::read_to_string(&path)
                .map_err(|e| format!("script {}: {e}", path.display()))?;
            let table: Table = lua
                .load(&source)
                .set_name(path.to_string_lossy().as_ref())
                .eval()
                .map_err(|e| format!("script {}: {e}", path.display()))?;
            let table = lua
                .create_registry_value(table)
                .map_err(|e| format!("script {}: {e}", path.display()))?;
            entries.push(Entry { hooks, table });
        }
        Ok(Some(Self {
            lua,
            entries,
            writes: allow_writes,
        }))
    }

    /// Run one hook against the live runtime. A `Net` hook returns the first non-nil answer.
    pub fn run(&mut self, hook: Hook, rt: &mut Runtime) -> Result<Option<Answer>, String> {
        let name = match &hook {
            Hook::Start => "on_start",
            Hook::Event { .. } => "on_event",
            Hook::Frame(_) => "on_frame",
            Hook::Stop => "on_stop",
            Hook::Net(_) => "on_net_request",
        };
        let mut answer = None;
        let result = self.lua.scope(|scope| {
            let api = scope.create_function_mut(|lua, args: Variadic<Value>| {
                dispatch(lua, rt, args, self.writes)
            })?;
            self.lua.globals().set("__api", api)?;
            for entry in &self.entries {
                if !entry.hooks.iter().any(|h| h == name) {
                    continue;
                }
                let table: Table = self.lua.registry_value(&entry.table)?;
                let Ok(function) = table.get::<Function>(name) else {
                    continue;
                };
                match &hook {
                    Hook::Start | Hook::Stop => {
                        function.call::<()>(())?;
                    }
                    Hook::Event { evt, wp, dwp } => {
                        function.call::<()>((*evt, *wp, *dwp))?;
                    }
                    Hook::Frame(ms) => {
                        function.call::<()>(*ms)?;
                    }
                    Hook::Net(url) => {
                        let value: Value = function.call(url.as_str())?;
                        if let Some(got) = parse_answer(value)? {
                            answer = Some(got);
                            break;
                        }
                    }
                }
            }
            Ok(())
        });
        result.map_err(|e| e.to_string())?;
        Ok(answer)
    }
}

fn parse_answer(value: Value) -> mlua::Result<Option<Answer>> {
    let Value::Table(table) = value else {
        return Ok(None);
    };
    let status = table.get::<Option<u32>>("status")?.unwrap_or(200).min(999) as u16;
    let content_type = table.get::<Option<String>>("content_type")?;
    let body = match table.get::<Option<Value>>("body")? {
        Some(Value::String(text)) => text.as_bytes().to_vec(),
        Some(Value::Integer(number)) => number.to_string().into_bytes(),
        Some(Value::Number(number)) => number.to_string().into_bytes(),
        _ => Vec::new(),
    };
    Ok(Some(Answer {
        status,
        content_type,
        body,
        truncated: false,
    }))
}

fn dispatch(
    lua: &Lua,
    rt: &mut Runtime,
    args: Variadic<Value>,
    allow_writes: bool,
) -> mlua::Result<Value> {
    let mut args = args.into_iter();
    let name = args
        .next()
        .and_then(|value| value.as_str().map(|s| s.to_string()))
        .ok_or_else(|| mlua::Error::RuntimeError("host API wants a method name".into()))?;
    let rest: Vec<Value> = args.collect();
    let num = |at: usize| -> mlua::Result<u32> {
        match rest.get(at) {
            Some(Value::Integer(value)) => Ok(*value as u32),
            Some(Value::Number(value)) => Ok(*value as u32),
            _ => Err(mlua::Error::RuntimeError(format!(
                "{name}: argument {at} is not a number"
            ))),
        }
    };
    let err = |e: String| mlua::Error::RuntimeError(e);
    let require_writes = |what: &str| -> mlua::Result<()> {
        if allow_writes {
            Ok(())
        } else {
            Err(mlua::Error::RuntimeError(format!(
                "script safe mode: {what} is disabled; restart with --script-writes"
            )))
        }
    };
    match name.as_str() {
        "mem.read8" => Ok(Value::Integer(
            rt.cpu
                .mem
                .read_u8(num(0)?)
                .map_err(|e| err(e.to_string()))? as i64,
        )),
        "mem.read16" => Ok(Value::Integer(
            rt.cpu
                .mem
                .read_u16(num(0)?)
                .map_err(|e| err(e.to_string()))? as i64,
        )),
        "mem.read32" => Ok(Value::Integer(
            rt.cpu
                .mem
                .read_u32(num(0)?)
                .map_err(|e| err(e.to_string()))? as i64,
        )),
        "mem.write8" => {
            require_writes("mem.write8")?;
            rt.cpu
                .mem
                .write_u8(num(0)?, num(1)? as u8)
                .map_err(|e| err(e.to_string()))?;
            Ok(Value::Nil)
        }
        "mem.write16" => {
            require_writes("mem.write16")?;
            rt.cpu
                .mem
                .write_u16(num(0)?, num(1)?)
                .map_err(|e| err(e.to_string()))?;
            Ok(Value::Nil)
        }
        "mem.write32" => {
            require_writes("mem.write32")?;
            rt.cpu
                .mem
                .write_u32(num(0)?, num(1)?)
                .map_err(|e| err(e.to_string()))?;
            Ok(Value::Nil)
        }
        "mem.read" => {
            let addr = num(0)?;
            let len = num(1)?.min(1 << 20);
            let bytes = rt
                .cpu
                .mem
                .read_bytes(addr, len as usize)
                .map_err(|e| err(e.to_string()))?;
            Ok(Value::String(lua.create_string(&bytes)?))
        }
        "mem.write" => {
            require_writes("mem.write")?;
            let addr = num(0)?;
            let Some(Value::String(text)) = rest.get(1) else {
                return Err(mlua::Error::RuntimeError(
                    "mem.write: second argument is not a string".into(),
                ));
            };
            rt.cpu
                .mem
                .write_bytes(addr, &text.as_bytes())
                .map_err(|e| err(e.to_string()))?;
            Ok(Value::Nil)
        }
        "reg.get" => {
            let register = rest
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| err("reg.get: name is not a string".into()))?;
            let value = register_value(rt, &register).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("reg.get: unknown register {register}"))
            })?;
            Ok(Value::Integer(value as i64))
        }
        "reg.set" => {
            require_writes("reg.set")?;
            let register = rest
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| err("reg.set: name is not a string".into()))?
                .to_string();
            let value = num(1)?;
            set_register(rt, &register, value).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("reg.set: unknown register {register}"))
            })?;
            Ok(Value::Nil)
        }
        "log" => {
            let text = rest
                .get(0)
                .and_then(Value::as_str)
                .map(|text| text.to_string())
                .unwrap_or_default();
            rt.display.note(format!("[script] {text}"));
            Ok(Value::Nil)
        }
        "popup" => {
            let title = rest
                .get(0)
                .and_then(Value::as_str)
                .map(|text| text.to_string())
                .unwrap_or_else(|| "Quoin script".to_string());
            let message = rest
                .get(1)
                .and_then(Value::as_str)
                .map(|text| text.to_string())
                .unwrap_or_default();
            if rt.script_consent_ui {
                show_popup(&title, &message);
            } else {
                rt.display
                    .note(format!("[script popup] {title}: {message}"));
            }
            Ok(Value::Nil)
        }
        "app.cls" => Ok(Value::Integer(rt.app_cls as i64)),
        "app.module" => Ok(Value::Integer(rt.shell as i64)),
        "clock.ms" => Ok(Value::Integer(rt.clock_ms as i64)),
        _ => Err(mlua::Error::RuntimeError(format!(
            "unknown host API {name}"
        ))),
    }
}

fn register_index(name: &str) -> Option<usize> {
    if let Some(number) = name.strip_prefix('r') {
        if let Ok(index) = number.parse::<usize>() {
            return (index <= 12).then_some(index);
        }
    }
    match name {
        "sp" => Some(13),
        "lr" => Some(14),
        _ => None,
    }
}

fn register_value(rt: &Runtime, name: &str) -> Option<u32> {
    match name {
        "pc" => Some(rt.cpu.pc),
        "cpsr" => Some(rt.cpu.cpsr),
        _ => register_index(name).map(|index| rt.cpu.r[index]),
    }
}

fn set_register(rt: &mut Runtime, name: &str, value: u32) -> Option<()> {
    match name {
        "pc" => rt.cpu.pc = value,
        "cpsr" => rt.cpu.cpsr = value,
        _ => rt.cpu.r[register_index(name)?] = value,
    }
    Some(())
}
