//! The HLE seam: how a guest vtable call becomes a Rust function call.
//!
//! A BREW module never imports a host symbol. It loads a numeric class id or an `IId`, gets
//! back a pointer to a guest-visible object, reads the word at offset 0 to find a vtable, and
//! branches to `vtable[slot]`. Every one of those addresses is ours to choose, so the host
//! synthesizes them:
//!
//! - a *stub page* holds one `SVC #method_id` per dispatchable method, four bytes each, at
//!   [`crate::layout::STUB_BASE`] + `id * 4`;
//! - a *vtable region* holds arrays of those stub addresses, one array per interface, laid out
//!   from [`crate::abi`]'s generated slot order;
//! - *objects* are guest memory whose first word is such a vtable address.
//!
//! So the guest's `ldr r1,[r0]; ldr r2,[r1,#slot*4]; blx r2` lands on an `SVC`, the run loop
//! returns [`crate::cpu::Stop::Hle`], and the id in the immediate says exactly which interface
//! method was called — no symbol table, no guessing, and no ambiguity about which vtable slot a
//! raw offset meant.
//!
//! The same mechanism covers the C-library surface, which in classic BREW is not a link-time
//! dependency either: a module reads `*(AEEHelperFuncs **)(AEEMod_Load - 4)` and calls through
//! that struct. See [`Hle::helper_table`].
//!
//! The seam runs both ways. A handset shell also *calls the module* — `IModule::CreateInstance`,
//! `IApplet::HandleEvent` — through a vtable the module wrote for itself, and from the host's side
//! such a downcall is an address like any other, so [`crate::runtime`] branches to it the same way
//! and logs a [`Downcall`] for it.

use std::collections::HashMap;
use std::fmt;

use crate::abi::{
    self, helper_field, helper_id, iface_at, iface_index, is_helper_id, HELPER_STRUCT,
};
use crate::gl::qegl;
use crate::layout;
use crate::mem::{Fault, MResult, Mem};

/// ARM `SVC #imm24`. In Thumb state the same trap is `0xDF00 | imm8`, which cannot carry a
/// 12-bit interface index — another reason [`crate::cpu::thumb`] is not a bolt-on.
pub const SVC_ARM: u32 = 0xef00_0000;

pub type HResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Mem(Fault),
    /// The guest called through an offset the host has no object at, or a vtable that is not
    /// one we synthesized. Almost always an ABI mistake rather than a guest bug.
    NotAnObject {
        addr: u32,
    },
    /// A module answered `AEEClsCreateInstance` with an object, but that object is not laid out like
    /// any `AEEApplet` this host knows: none of the `m_pIShell` offsets it tries holds the shell
    /// pointer, or the module pointer beside it does not name the module that built it. BREW 4.0 /
    /// Brew MP actors, extensions and service factories hand back exactly this shape and are not
    /// applets to start — calling it "not a host object" sent readers looking for a released
    /// interface instead.
    NotAnApplet {
        addr: u32,
        cls: u32,
        /// The words read at `m_pIShell` for each shape in [`crate::runtime::AppletLayout::ALL`],
        /// in that order: the SDK's, the widened and Brew MP's.
        seen: [u32; 3],
        /// The word read at `m_pIModule` for a shape whose shell matched but whose module did not,
        /// or 0 when no shape's shell matched.
        module: u32,
    },
    /// Reached a dispatch the runtime has not implemented. Named, so the next one to add is
    /// obvious from a failure message.
    Unimplemented {
        id: u32,
        label: String,
        this: u32,
    },
    /// A call the host has no answer for *inside* an interface it does serve, where the interface's
    /// own dispatch has a catch-all. Carries the bare method name as well as the label because the
    /// person reading the report is not looking up an ABI — they want to know what the game stopped
    /// doing, and `want` is what turns `DrawEllipticalArc` into "it wanted to draw an ellipse".
    Wanted {
        id: u32,
        label: String,
        want: String,
        this: u32,
    },
    /// A request the host's chosen *representation* cannot carry, as opposed to a slot with no
    /// behaviour yet: [`crate::display`] writes RGB565 and nothing else, so an 8-bpp bitmap is
    /// refused rather than silently drawn at the wrong stride.
    Unsupported {
        what: String,
    },
    /// An applet-scoped shell call — `CloseApplet`, `SendEvent` — with nothing running. A handset
    /// falls back to its idle applet; this host has exactly one slot, and inventing a resident
    /// one would hide the fact that a start failed.
    NoApplet,
    /// A host→guest call that ran out of instructions instead of returning. Named with the entry
    /// address and the instruction count, because a guest loop that never terminates is otherwise
    /// indistinguishable from the emulator hanging.
    ///
    /// `spun` says *which* of the two budgets ran out, and that is the whole diagnosis: a guest that
    /// retired its instructions **without reaching the host once** is a loop that is waiting for
    /// something that will never arrive, while one that kept calling the host and still did not
    /// return is a loop that is working and simply long. The first is a bug to find; the second is a
    /// game, and killing it and calling that a fault is what this flag exists to stop.
    ///
    /// `session` refines it once more: the whole run's own ceiling (`--session-ceiling`) is also an
    /// absolute figure, so a session that never finishes reports through the same `spun` shape and
    /// would otherwise send the reader off to raise a per-function number that was never the limit.
    NoReturn {
        addr: u32,
        steps: u64,
        spun: bool,
        session: bool,
    },
    /// A host→guest call nested past the runtime's ceiling. The shell recognises a module handing the
    /// same class back as that module's refusal, so what lands here is a guest that keeps re-entering
    /// host code on requests the registry cannot short-circuit. Without a ceiling every level costs
    /// host stack until the emulator process dies of it. The call log names the bouncing call, so this
    /// only has to say where the nesting was.
    TooDeep {
        depth: u32,
        addr: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Mem(e) => write!(f, "{}", e),
            Error::NotAnObject { addr } => {
                write!(
                    f,
                    "{:#010x} is not a host object (no vtable we synthesized)",
                    addr
                )
            }
            Error::NotAnApplet { addr, cls, seen, module } => {
                write!(
                    f,
                    "{addr:#010x} (class {cls:#010x}) is not an AEEApplet object this host knows: \
                     m_pIShell read {:#010x}/{:#010x}/{:#010x} at the SDK's, the widened and Brew \
                     MP's layouts, m_pIModule {:#010x}",
                    seen[0], seen[1], seen[2], module
                )
            }
            Error::Unimplemented { id, label, this } => write!(
                f,
                "unimplemented HLE call {} (id {:#08x}) on this={:#010x}",
                label, id, this,
            ),
            Error::Wanted { id, label, want, this } => write!(
                f,
                "unimplemented HLE call {} (id {:#08x}) on this={:#010x}: it wanted to {}",
                label, id, this, want,
            ),
            Error::Unsupported { what } => write!(f, "unsupported: {}", what),
            Error::NoApplet => write!(f, "no applet is running"),
            Error::NoReturn { session: true, addr, steps, .. } => write!(
                f,
                "the run spent its whole --session-ceiling and stopped inside guest function at \
                 {:#010x}, which had retired {} instructions of its own; it kept reaching the host, \
                 so this is a pack that never finishes its task rather than a loop that is stuck — \
                 raise --session-ceiling to let it run further",
                addr, steps,
            ),
            Error::NoReturn { addr, steps, spun, .. } if *spun => write!(
                f,
                "guest function at {:#010x} retired {} instructions and still has not returned; \
                 that is the per-function ceiling, not the wait-for-the-host one, so the loop may \
                 well be working — raise --steps-ceiling, or --session-ceiling if it is the whole \
                 run that never finishes, or play it with --window",
                addr, steps,
            ),
            Error::NoReturn { addr, steps, .. } => write!(
                f,
                "guest function at {:#010x} ran {} instructions without reaching the host once \
                 and did not return",
                addr, steps,
            ),
            Error::TooDeep { depth, addr } => write!(
                f,
                "downcall nesting hit its ceiling at {} levels, re-entering {:#010x}",
                depth, addr,
            ),
        }
    }
}

impl std::error::Error for Error {}

/// What a method was for, in the words a tester uses. Only the names a real pack has asked for are
/// glossed by hand; anything else is quoted as it is, because guessing at a verb sends someone looking
/// in the wrong place.
fn verb(method: &str) -> Option<&'static str> {
    Some(match method {
        "DrawEllipticalArc" => {
            "draw an elliptical arc around a rectangle (filled, it becomes an elliptical sector)"
        }
        "DrawArc" => "draw an arc",
        "DrawEllipse" => "draw an ellipse",
        "DrawTriangle" => "draw a triangle",
        "DrawPie" => "draw a pie",
        "DrawRect" => "draw a rectangle",
        "DrawLine" => "draw a line",
        "DrawPoint" => "draw a point",
        "StretchBlt" => "scale a picture and blit it to a position",
        "SetClip" => "set the clip rectangle",
        "GetClip" => "read the clip rectangle",
        "SetViewport" => "set the viewport",
        "GetViewport" => "read the viewport",
        "ClearViewport" => "clear the viewport",
        "ClearRect" => "clear a rectangle",
        "SetDestination" => "move the pen to another bitmap",
        "GetDestination" => "ask which bitmap the pen draws into",
        "EnableDoubleBuffer" => "enable double buffering",
        "Update" => "flush what was drawn to the screen",
        "GetColorDepth" => "ask the colour depth of the screen",
        "Translate" => "translate later drawing",
        "Pan" => "pan later drawing",
        _ => return None,
    })
}

/// Which part of the game a method belongs to, so the report can say "this part of drawing is
/// missing" instead of naming an interface. `None` leaves the sentence without a domain rather than
/// inventing one.
fn domain(iface: &str) -> Option<&'static str> {
    Some(match iface {
        "IGraphics" => "drawing",
        "IDisplay" | "IBitmap" => "the screen and bitmaps",
        "IImage" | "IImage2" | "IPNG" | "IGIF" | "IJPEG" => "image decoding",
        "ISound" | "ISoundPlayer" | "IMedia" | "IMediaControl" => "sound",
        "IFile" | "IFileMgr" | "IAStream" | "IMemAStream" => {
            "file reading (levels, saves and settings live here)"
        }
        "INetMgr" | "ISocket" | "IPort" | "IDNS" | "INetwork" => "networking",
        "ITransform" => "transforms (rotation and scaling)",
        "IFont" => "fonts",
        "IWidget" | "IMenuCtl" | "IStatic" | "ITextField" | "IProgressCtl" | "ICheckBox"
        | "IRadioButton" | "IList" => "controls and menus",
        "IGL" | "IEGL" | "IGLES10" | "IGLES11" => "3D graphics",
        "ITelephone" | "ISMS" | "IPhoneCtl" => "telephony and SMS",
        "IDatabase" | "IDBMgr" | "IRecordStore" => "databases",
        "IWeb" => "web pages",
        _ => return None,
    })
}

/// What a failure means, in the words the person running a game reads.
///
/// The text above is written for someone who has the source open; this is for someone who has only
/// the window in front of them. It is keyed on the sentences this file's own `Display` emits, and
/// `every_failure_sentence_has_an_advice` below builds each variant and feeds it back, so the two
/// cannot drift apart: add a failure with no advice and that test says which one.
///
/// Each line answers three questions in order — whose fault is it, what happened, what to type next —
/// because a report that only restates the error is the one thing that still sends a reader into the
/// code.
pub fn advice(text: &str) -> Option<String> {
    // A refused method is the one case that gets a sentence built rather than looked up: the reader
    // wants to know what the game was trying to *do*, and `IGraphics::DrawEllipticalArc` does not say
    // that in terms anyone playing would recognise.
    if let Some(rest) = text
        .find("unimplemented HLE call ")
        .map(|at| &text[at + "unimplemented HLE call ".len()..])
    {
        return Some(refused(rest));
    }
    let hits = |key: &str| text.contains(key);
    let line: &'static str = if hits("is not a host object") {
        "This pointer is not an object this host handed out: the pack is calling another address as an interface object, or it already released that object. The this= field in the report is that pointer; --watch it to see when it changed."
    } else if hits("is not an AEEApplet object this host knows") {
        "The module did build an object for this class, but it is not laid out like any applet shape this host knows — the SDK's, the widened one some modules build, or Brew MP's — because none of those offsets holds the shell pointer, or the module pointer beside it names a different module. BREW 4.0 / Brew MP actors, extensions and service factories look exactly like this — they are libraries, not applets to start. `--start auto` can only pick a class from this module's own image; look in the pack's .mif for its real applet class, or run the game .mod that loads this one."
    } else if hits("spent its whole --session-ceiling") {
        "The run reached its instruction ceiling without finishing, and it kept asking the host for something — it did not finish, it is not stuck. Use --window to keep playing, or raise --session-ceiling to let it run further."
    } else if hits("still has not returned") {
        "This guest function used up its instruction budget without returning, but it is calling the host, so the run is probably just long. Raise --steps-ceiling, or play with --window."
    } else if hits("without reaching the host once") {
        "This guest function is spinning without reaching the host once — it is waiting for something that will not arrive (an event, a timeout, or a file that never opened). Look up its call log; the gap is usually a few lines above."
    } else if hits("no applet is running") {
        "An event is being delivered to an applet, but no applet is running. Two cases: its class was never created (run with `--start auto` and `--args CreateInstance` to see which one it wants); or the session has already called `CloseApplet` while events still arrive (a `CloseApplet: cancelled … timer(s) and … queued job(s)` line in the log means this — the pack quit, it did not crash)."
    } else if hits("guest heap exhausted") {
        "The pack asked for more memory than this emulated handset has. Almost every time it computed a length wrong: look at the previous value it computed rather than treating this as a missing-memory problem."
    } else if hits("unmapped guest address") {
        "The guest touched an unmapped address, usually a null or freed pointer. r0-r3 hold this call's arguments and lr is the caller; --break at that address to see the scene."
    } else if hits("instruction fetch from") {
        "The guest jumped into memory that is not code: a function pointer was never set, or it was overwritten. Whoever wrote the value at pc= is the next thing to find (--watch that location)."
    } else if hits("not a resource container") {
        "This file is not a resource container this host recognises (.bar/.mif). Either it is not a container, or its header is one this host has not seen."
    } else if hits("unsupported resource") {
        "The container is fine, but this host cannot decode this resource: the missing piece is a decoder for this format."
    } else if hits("downcall nesting hit its ceiling") {
        "Host-to-guest nesting hit the stack ceiling: the pack answered one call by asking the host for the same thing again. The address in the report is the entry point being re-entered."
    } else if hits("load failed") || hits("module image is empty") {
        "This .mod was not loaded: the file or its header is wrong, and it has nothing to do with the code inside."
    } else if hits("holds no NUL byte in") {
        "The first argument holds no zero byte in 4096, so it does not name a NUL-terminated string. Read what the call was handed rather than the host: `--args strlen` prints the argument words, and the report's this= field is the first of them (r0), with the next three after `then`."
    } else if hits("unsupported:") {
        "This approach is one the host explicitly does not implement (not one it has not implemented yet): the representation we chose cannot carry what it asks for, and the report names which part."
    } else {
        return None;
    };
    Some(line.to_string())
}

/// The line for a method this host cannot answer, from the label `Interface::Method (id …) on this=…`.
///
/// Three parts, in the order a tester needs them: what the pack wanted, which part of the game that
/// is, and what to type. An unknown method is quoted rather than glossed, and an unknown interface
/// leaves the domain out, because a wrong "it wanted to draw a rectangle" is worse to read than a plain one.
fn refused(rest: &str) -> String {
    let name = rest.split(" (id ").next().unwrap_or(rest);
    let method = name.rsplit("::").next().unwrap_or(name);
    let iface = name.rsplit_once("::").map(|(i, _)| i).unwrap_or("");
    let mut line = String::from("the game stopped here: ");
    match verb(method) {
        Some(doing) => line.push_str(&format!("it wanted to {doing}, ")),
        None => line.push_str(&format!("it called {name}, ")),
    }
    match domain(iface) {
        Some(scope) => line.push_str(&format!(
            "but this host has no such {scope} item yet. That is a host gap, not a broken game."
        )),
        None => line.push_str(
            "but this host has no such feature yet. That is a host gap, not a broken game.",
        ),
    }
    line.push_str("see what else this run lacks: add --coverage. see who called it: add --break ");
    line.push_str(name);
    line.push('.');
    line
}

impl From<Fault> for Error {
    fn from(e: Fault) -> Self {
        Error::Mem(e)
    }
}

/// What a pointer argument was pointing at, read at the trap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgText {
    /// It read as a printable NUL-terminated name.
    Name(String),
    /// It did not, so the bytes themselves were copied — which is how a structure argument
    /// (`AERect`, `AEEDeviceInfo`, `AEETransformMatrix`) is made readable at all.
    Bytes(String),
}

impl ArgText {
    /// The way this report's lines spell it: a name in quotes, a block of bytes in brackets.
    pub fn render(&self) -> String {
        match self {
            ArgText::Name(s) => format!("={s:?}"),
            ArgText::Bytes(s) => format!("=[{s}]"),
        }
    }
}

/// One taken trap, kept so a test can assert the guest really reached the seam.
#[derive(Debug, Clone)]
pub struct Call {
    pub id: u32,
    /// A process-lifetime interned name from [`crate::abi::label`], not an owned `String`: this
    /// record is made on every trap, and two allocations per trap is a price a call log nobody
    /// reads should not set.
    pub label: &'static str,
    /// The `this` pointer, i.e. the first argument.
    pub this: u32,
    /// Every argument word as it stood when the trap was taken, `this` first, including the words
    /// AAPCS puts on the stack past the fourth. Only `--args` asks for them, but recording them is
    /// what lets a run say *how* a method was called rather than only that it was.
    pub args: Vec<u32>,
    /// The same words read as text, for the calls `--args` names and nothing else.
    ///
    /// A pointer argument is only meaningful while the bytes it aims at still exist. Names a caller
    /// assembles at run time — every `s%d.save` a game builds on its own stack — live in a slot that
    /// the next function call reuses, so a report written after the run cannot read them and prints a
    /// bare address for the one argument worth seeing. This is captured at the trap for exactly that
    /// reason, and it is empty for every other call because copying strings out of the guest on all
    /// seventy-odd thousand blits of a run is not a price worth paying for a trace nobody asked for.
    pub text: Vec<Option<ArgText>>,
    pub retval: u32,
    /// Guest instruction count when the trap was taken.
    pub steps: u64,
}

/// One host→guest call, i.e. a function pointer the runtime itself branched to.
///
/// The mirror image of [`Call`]: an upcall is named by a dispatch id, a downcall only by the
/// address it entered at, which is a vtable slot the module wrote. `args` is the register half of
/// the AAPCS arguments; anything passed on the stack is the guest's own business.
#[derive(Debug, Clone, Copy)]
pub struct Downcall {
    pub addr: u32,
    pub args: [u32; 4],
    pub retval: u32,
    /// Guest instruction count once the callee had returned.
    pub steps: u64,
}

/// A host object the guest holds a pointer to.
#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub addr: u32,
    pub iface: &'static str,
    /// `AEECLSID_...` the object answers to, or 0 when it is not class-addressable.
    pub clsid: u32,
    /// Where its reference count lives. `IShell` has no published struct so the host picks
    /// offset 4; an applet must use `m_nRefs` inside `_AEEApplet` at offset 8.
    pub refs_off: u32,
    /// Bytes this record owns from the object arena, or 0 for an object the guest built itself
    /// (`adopt_object`) and for one restored from a snapshot — neither may be handed back out.
    pub len: u32,
}

/// Per-object state that is not part of the two header words every object has.
///
/// One variant per subsystem that needs to hang something on an object. `Runtime` keeps its
/// singletons (`Cpu`, `Display`, `Heap`, the GL pipeline itself) as fields, and only what is
/// genuinely per-object comes here — the rule that a handler must not gain
/// a lookup it did not already need.
#[derive(Debug)]
pub enum Payload {
    /// One face of a QEGL family: which face this object is, plus the addresses of the other ten.
    ///
    /// The copy is deliberate — each of the eleven objects carries the family it belongs to, so a
    /// slot handed nothing but `this` can find its siblings without a second table, and so the
    /// eleven die one at a time with the objects that own them.
    Qegl(qegl::Occupant),
}

#[derive(Debug)]
pub struct Hle {
    /// iface index -> synthesized vtable address
    vtables: HashMap<usize, u32>,
    vtable_cursor: u32,
    helper_table: u32,
    objects: HashMap<u32, Object>,
    /// What an object carries beyond its two header words, keyed by the same address as `objects`
    /// and dropped by the same call.
    ///
    /// This exists because per-object state used to live in `Runtime` as one `HashMap` per
    /// subsystem, and `Release`-to-zero had to walk a list of twenty `remove`s by hand — a list
    /// that had already missed one entry (see the note beside `image_display`). Two facts settle
    /// it: **the lifetime of that state is the object's**, and **one object can die without the
    /// caller knowing anything else**. A parallel table cannot express either, because the one
    /// thing a release is handed is the address.
    ///
    /// An enum rather than `Box<dyn Any>`: downcasting would move "which subsystem owns this
    /// state" out of the type system, and the exhaustive `match` in [`Hle::forget`] is what tells
    /// the next person who adds a subsystem that they have one more place to look.
    payloads: HashMap<u32, Payload>,
    object_cursor: u32,
    /// Object blocks `forget` handed back, as `(addr, len)`, reused before the cursor moves. A pack
    /// that makes and drops a surface every frame (UnlimitedWar) would otherwise walk the object
    /// arena at frame rate — and since a bitmap's pixels now live inside its block, that walk is
    /// kilobytes a frame rather than a few dozen bytes.
    freed_objects: Vec<(u32, u32)>,
    heap_next: u32,
    /// Bytes handed out per block, by address: what `IHeap::Realloc` needs to copy a block it cannot
    /// grow in place. A bump arena can only move a block, so the copy is the whole of `realloc` here.
    sizes: HashMap<u32, u32>,
    calls: Vec<Call>,
    downcalls: Vec<Downcall>,
    /// Dispatches taken, per stub id, folded as they happen. The history below is capped, and this is
    /// what keeps `--coverage` and the per-method counts exact once it has started dropping: a game
    /// blits the same rectangle more often than anyone reads a log, and a report that quietly went
    /// short is worse than one that says where it stopped.
    counts: HashMap<u32, u64>,
    /// Downcalls per entry address, for the same reason. `Runtime` asks "how many times has this
    /// vtable slot run" once per frame, which on a scanned history is a quadratic way to count.
    down_counts: HashMap<u32, u64>,
    /// Dispatches taken, per *label* — `IShell::CreateInstance`, `AEEHelperFuncs::malloc`. This is
    /// what `--coverage` folds, and it is kept apart from the counts above because the fold groups by
    /// name and because a capped history would otherwise make the one number in that report that
    /// someone actually relies on go quietly short. Interned labels make the key a pointer, so
    /// counting a trap no longer clones a string just to hash it.
    tally: HashMap<&'static str, u64>,
    /// Every call into the module, folded by what it was handed: `(addr, a1, a2)` → how many times.
    ///
    /// This exists because [`Hle::downcalls`] cannot answer the one question a person at a keyboard
    /// asks — "did my key reach the pack at all" — once a run has gone on long enough for the pack's
    /// own per-frame callback to fill that history: it keeps the most recent half, so after a few
    /// minutes the only rows left are the heartbeat and the key pressed ten minutes ago is gone.
    /// A run of a few minutes accumulates thousands of callbacks and a couple of hundred events, and a
    /// report that kept only recent rows would show the callbacks and not one of the events. These counts are exact and do not
    /// expire — the same trade `counts`/`tally` already make for dispatches.
    ///
    /// The key is the two argument words *after* the object, because that is the shape an event
    /// delivery has (`(applet, evt, wp, dwp)` through [`Runtime::send_event`](crate::Runtime), so
    /// `(evt, wp)` is exactly "which key"). A callee handed a pointer folds one row per address, which
    /// is noisy but bounded by [`Hle::FOLDED`] — and hitting that ceiling is recorded rather than
    /// hidden.
    folded_down: HashMap<(u32, u32, u32), u64>,
    /// Whether the folded table ever refused a key. Non-zero means a report of it is a floor.
    folded_full: bool,
    /// How many records the two histories have dropped, oldest first.
    pub dropped: (u64, u64),
}

impl Hle {
    pub fn new() -> Self {
        Hle {
            vtables: HashMap::new(),
            vtable_cursor: layout::VTABLE_BASE,
            helper_table: 0,
            objects: HashMap::new(),
            payloads: HashMap::new(),
            object_cursor: layout::OBJECT_BASE,
            freed_objects: Vec::new(),
            heap_next: 0,
            sizes: HashMap::new(),
            calls: Vec::new(),
            downcalls: Vec::new(),
            counts: HashMap::new(),
            down_counts: HashMap::new(),
            tally: HashMap::new(),
            folded_down: HashMap::new(),
            folded_full: false,
            dropped: (0, 0),
        }
    }

    /// Guest address of the `SVC` for a dispatch id, mapping and writing the stub.
    pub fn stub(mem: &mut Mem, id: u32) -> MResult<u32> {
        let addr = layout::STUB_BASE + id * 4;
        if addr >= layout::VTABLE_BASE {
            // Would overwrite the vtable region; the id encoding is out of range.
            return Err(Fault::Unmapped { addr });
        }
        mem.map(addr, 4);
        mem.write_u32(addr, SVC_ARM | (id & 0x00ff_ffff))?;
        Ok(addr)
    }

    fn bump(mem: &mut Mem, cursor: &mut u32, len: u32) -> MResult<u32> {
        let addr = (*cursor + 3) & !3;
        mem.map(addr, len);
        *cursor = addr + len;
        Ok(addr)
    }

    /// A block for a new object: the first freed one that fits, else the bump cursor.
    fn take_object(&mut self, mem: &mut Mem, want: u32) -> MResult<u32> {
        if let Some(i) = self.freed_objects.iter().position(|(_, len)| *len >= want) {
            let (addr, len) = self.freed_objects.remove(i);
            if len - want >= 8 {
                self.freed_objects.push((addr + want, len - want));
            }
            mem.map(addr, want);
            return Ok(addr);
        }
        Self::bump(mem, &mut self.object_cursor, want)
    }

    /// The vtable for an interface index, built once and cached. Slot `n` holds the stub for
    /// `method_id(index, n)`, so a guest `ldr`/`blx` pair lands on the right dispatch.
    ///
    /// Both halves of the table are served here: a generated interface's length comes from
    /// `IFACES`, an interface the host serves itself takes its length from [`abi::HOST_IFACES`].
    /// The loop is one loop because both halves are the same shape — N slots in declaration order —
    /// and `abi`'s slot-order test is what keeps that true for the generated side. Unifying also
    /// restored the cache to the six hand-written builders this replaced: each of those bumped a
    /// fresh page on every call, so a second object of the same class duplicated its vtable.
    pub fn vtable(&mut self, mem: &mut Mem, index: usize) -> MResult<u32> {
        if let Some(a) = self.vtables.get(&index) {
            return Ok(*a);
        }
        let slots = match iface_at(index) {
            Some(info) => info.methods.len(),
            None => abi::host_iface(index)
                .ok_or(Fault::Unmapped { addr: index as u32 })?
                .methods
                .len(),
        };
        let base = Self::bump(mem, &mut self.vtable_cursor, (slots * 4) as u32)?;
        for slot in 0..slots as u32 {
            let stub = Self::stub(mem, abi::method_id(index, slot))?;
            mem.write_u32(base + slot * 4, stub)?;
        }
        self.vtables.insert(index, base);
        Ok(base)
    }

    pub fn vtable_named(&mut self, mem: &mut Mem, name: &str) -> MResult<u32> {
        let index = iface_index(name).ok_or(Fault::Unmapped { addr: 0 })?;
        self.vtable(mem, index)
    }

    /// The same table, with the slots laid out in an order this host supplies.
    ///
    /// The tables below come from the 4.0.2 SP19 headers, and the delivered 3.1.2 object of one
    /// interface does not agree with them: the delivered body's and the delivered body's own
    /// `.constdata` both put `SelectNetwork` at slot 11 and `OpenEx` at 12, where
    /// the header has them the other way round. A guest call lands on a *byte offset*, so the
    /// order in the table this host hands out is the thing that decides which rule runs, and the
    /// shipped order is what this method exists to deliver. The dispatch side reads the slot number
    /// back out of the id, so the two ends cannot drift apart silently.
    pub fn vtable_ordered(&mut self, mem: &mut Mem, name: &str, order: &[u32]) -> MResult<u32> {
        let index = iface_index(name).ok_or(Fault::Unmapped { addr: 0 })?;
        let base = Self::bump(mem, &mut self.vtable_cursor, (order.len() as u32) * 4)?;
        for (position, slot) in order.iter().enumerate() {
            let stub = Self::stub(mem, abi::method_id(index, *slot))?;
            mem.write_u32(base + 4 * position as u32, stub)?;
        }
        Ok(base)
    }

    /// Synthesize the `AEEHelperFuncs` table the guest finds below `AEEMod_Load`.
    ///
    /// This is the emulator's entire libc contract: 117 function pointers, every one of them a
    /// stub. Field offsets come from the generated struct table, so `malloc` is where the
    /// header says it is — which `helloworld.mod` independently confirms by loading offset
    /// 0x68 out of this table before its first `blx`.
    pub fn build_helper_table(&mut self, mem: &mut Mem) -> MResult<u32> {
        if self.helper_table != 0 {
            return Ok(self.helper_table);
        }
        let info = abi::struct_info(HELPER_STRUCT).ok_or(Fault::Unmapped { addr: 0 })?;
        let base = Self::bump(mem, &mut self.vtable_cursor, info.size)?;
        for field in info.fields {
            let stub = Self::stub(mem, helper_id(field.off))?;
            mem.write_u32(base + field.off, stub)?;
        }
        self.helper_table = base;
        Ok(base)
    }

    pub fn helper_table_addr(&self) -> u32 {
        self.helper_table
    }

    /// Allocate a guest object: a vptr, a reference count initialised to 1, then `tail` words
    /// at explicit byte offsets.
    pub fn new_object(
        &mut self,
        mem: &mut Mem,
        iface: &'static str,
        clsid: u32,
        size: u32,
        refs_off: u32,
        tail: &[(u32, u32)],
    ) -> MResult<u32> {
        let vtable = self.vtable_named(mem, iface)?;
        let want = size.max(8);
        let addr = Self::bump(mem, &mut self.object_cursor, want)?;
        mem.write_u32(addr, vtable)?;
        mem.write_u32(addr + refs_off, 1)?;
        for (off, val) in tail {
            mem.write_u32(addr + off, *val)?;
        }
        self.objects.insert(
            addr,
            Object {
                addr,
                iface: abi::resolve_alias(iface),
                clsid,
                refs_off,
                len: want,
            },
        );
        Ok(addr)
    }

    /// Like [`new_object`], but takes a block `forget` handed back before moving the cursor.
    ///
    /// Bitmaps go through this. Their block carries the pixels (`DIB_NewEx` allocates
    /// `pixel_bytes + 0x6B`), and a pack that builds a surface per frame would otherwise walk the
    /// object arena at frame rate. Ordinary objects keep [`new_object`]: their addresses being
    /// fresh is a property the rest of this host relies on, and they are small enough that the
    /// arena is not the thing that runs out.
    pub fn new_object_reusing(
        &mut self,
        mem: &mut Mem,
        iface: &'static str,
        clsid: u32,
        size: u32,
        refs_off: u32,
        tail: &[(u32, u32)],
    ) -> MResult<u32> {
        let vtable = self.vtable_named(mem, iface)?;
        let want = size.max(8);
        let addr = self.take_object(mem, want)?;
        mem.write_u32(addr, vtable)?;
        mem.write_u32(addr + refs_off, 1)?;
        for (off, val) in tail {
            mem.write_u32(addr + off, *val)?;
        }
        self.objects.insert(
            addr,
            Object {
                addr,
                iface: abi::resolve_alias(iface),
                clsid,
                refs_off,
                len: want,
            },
        );
        Ok(addr)
    }

    pub fn object(&self, addr: u32) -> Option<&Object> {
        self.objects.get(&addr)
    }

    /// [`Hle::new_object_reusing`] for an interface whose vtable the caller built, because the
    /// interface is served by the host but is not in the generated tables (`IPort1`).
    #[allow(clippy::too_many_arguments)]
    pub fn new_object_with_vtable(
        &mut self,
        mem: &mut Mem,
        iface: &'static str,
        clsid: u32,
        vtable: u32,
        size: u32,
        refs_off: u32,
    ) -> MResult<u32> {
        let want = size.max(8);
        let addr = self.take_object(mem, want)?;
        mem.write_u32(addr, vtable)?;
        mem.write_u32(addr + refs_off, 1)?;
        self.objects.insert(
            addr,
            Object {
                addr,
                iface,
                clsid,
                refs_off,
                len: want,
            },
        );
        Ok(addr)
    }

    /// Adopt a vtable pointer the guest produced itself, so `AddRef`/`Release` on it behave.
    ///
    /// Necessary because a module builds its *own* vtables at load time: `AEEMod_New` mallocs
    /// `sizeof(AEEMod) + 16`, writes four of its own function addresses into the trailing 16
    /// bytes, and points `pMod` at them. Those objects are guest code on both ends, so the host
    /// only tracks the header.
    pub fn adopt_object(&mut self, addr: u32, iface: &'static str, clsid: u32, refs_off: u32) {
        self.objects.insert(
            addr,
            Object {
                addr,
                iface: abi::resolve_alias(iface),
                clsid,
                refs_off,
                len: 0,
            },
        );
    }

    /// Stop recognising an object, so a later call through its stale address faults by name
    /// instead of dispatching against something already destroyed.
    pub fn forget(&mut self, addr: u32) -> bool {
        // Here, not in a list of `remove`s beside the call: whatever an object carried goes with
        // it, and an object that carries nothing costs one failed lookup. A subsystem that adds a
        // `Payload` variant gets this for free; `crate::gl::qegl` is the first one that does.
        self.payloads.remove(&addr);
        if let Some(o) = self.objects.remove(&addr) {
            // Only bitmap blocks come back to the arena: `new_object_reusing` is the one caller that
            // looks in `freed_objects`, and a block that carried pixels is the one worth reusing.
            if o.iface == "IBitmap" && o.len != 0 {
                self.freed_objects.push((o.addr, o.len));
            }
            true
        } else {
            false
        }
    }

    /// Attach state to an object that is already in `objects`.
    ///
    /// `None` detaches it. Nothing reads a payload for an address that is not an object, so a
    /// caller that has one is a caller that just made the object.
    pub fn set_payload(&mut self, addr: u32, payload: Payload) {
        self.payloads.insert(addr, payload);
    }

    pub fn payload(&self, addr: u32) -> Option<&Payload> {
        self.payloads.get(&addr)
    }

    pub fn refcount(&self, mem: &Mem, obj: &Object) -> MResult<u32> {
        mem.read_u32(obj.addr + obj.refs_off)
    }

    pub fn set_refcount(&self, mem: &mut Mem, obj: &Object, v: u32) -> MResult<()> {
        mem.write_u32(obj.addr + obj.refs_off, v)
    }

    /// Move the reference count of the object at a guest address by `delta`, clamped at zero.
    ///
    /// The runtime's `AddRef`/`Release` slots go through this too, so a host object that takes a
    /// reference of its own — an `IDisplay` holds its destination `IBitmap` — cannot wrap the count
    /// behind the guest's back.
    pub fn change_refs(&mut self, mem: &mut Mem, addr: u32, delta: i32) -> HResult<u32> {
        let obj = *self.object(addr).ok_or(Error::NotAnObject { addr })?;
        let now = mem.read_u32(obj.addr + obj.refs_off)?;
        // A handset would free the object at zero and a `Release` past that would corrupt its
        // heap. Refuse to wrap quietly: the count is guest state, so clamping keeps a leak or a
        // double-free visible in a state dump instead of hiding behind 0xfffffffc.
        let next = if delta < 0 {
            now.saturating_sub(delta.unsigned_abs())
        } else {
            now.saturating_add(delta as u32)
        };
        mem.write_u32(obj.addr + obj.refs_off, next)?;
        Ok(next)
    }

    /// Bytes from the guest heap: `AEEkHeap_Realloc(pHeap, 0, len)`, with `zero` meaning "clear it",
    /// which is the absence of `ALLOC_NO_ZMEM`'s top bit.
    ///
    /// A block this host freed comes back into circulation exactly when the delivered search reaches
    /// it, which is not "never" — see [`crate::heap`]. The sizes map stays because it is what tells a
    /// `FREE` for an address this host handed out from one it did not.
    pub fn alloc(&mut self, mem: &mut Mem, len: u32) -> MResult<u32> {
        self.alloc_with(mem, len, true)
    }

    /// [`Hle::alloc`], with the caller's answer to `ALLOC_NO_ZMEM`: a handset's `malloc` clears the
    /// block, and the top bit of the size is how a caller says it would rather it did not.
    pub fn alloc_with(&mut self, mem: &mut Mem, len: u32, zero: bool) -> MResult<u32> {
        let len = if len == 0 { 4 } else { len };
        let addr = if zero {
            mem.alloc(len, 4)?
        } else {
            mem.alloc_unzeroed(len)?
        };
        self.sizes.insert(addr, len);
        Ok(addr)
    }

    /// `realloc(p, len)`: in place when the delivered allocator can, which keeps the address a pack
    /// is holding. `None` means the heap refused and the old block is still the caller's.
    pub fn realloc(&mut self, mem: &mut Mem, block: u32, len: u32, zero: bool) -> Option<u32> {
        let len = if len == 0 { 4 } else { len };
        if !self.sizes.contains_key(&block) {
            // A pointer this host never handed out: there is nothing to carry over and nothing to
            // give back, so the honest answer is a fresh block — the delivery would break on it, and
            // a pack that reallocs one of its own statics has to keep running.
            return self.alloc_with(mem, len, zero).ok();
        }
        let at = mem.realloc(block, len, zero)?;
        self.sizes.remove(&block);
        self.sizes.insert(at, len);
        Some(at)
    }

    /// How many bytes were asked for at `addr` — what a caller about to replace that block has to copy.
    /// `None` for an address this host never handed out.
    pub fn block_size(&self, addr: u32) -> Option<u32> {
        self.sizes.get(&addr).copied()
    }

    /// `FREE(p)`: the block goes back to the arena.
    ///
    /// An address this host never handed out is ignored rather than trusted — a pack that frees a
    /// pointer into its own statics, or frees the same block twice, is a pack that has to keep
    /// running, and the second free in particular must not hand the same space to two callers.
    pub fn dealloc(&mut self, mem: &mut Mem, addr: u32) {
        if let Some(_len) = self.sizes.remove(&addr) {
            mem.free(addr);
        } else {
            // Nothing to give back, but the *attempt* is the fact a watchpoint was built for: a pack
            // that releases a block twice is holding a stale pointer, which is exactly the shape
            // a watchpoint is built to catch. Refusing is right; saying nothing about it is not.
            mem.note_unowned_free(addr);
        }
    }

    /// How many call records each history keeps before it starts dropping the oldest. A shipped game
    /// reaches the host tens of thousands of times a minute, which is far more than anyone reads, and
    /// keeping all of them is what grew the process until Windows fast-failed it. The counts beside it
    /// stay exact, so a report can still say how many there were.
    pub const HISTORY: usize = 4096;

    /// How many distinct `(addr, a1, a2)` keys the folded table holds before it stops taking new
    /// ones.
    ///
    /// Unlike [`Hle::HISTORY`] this is a ceiling on *kinds* of call rather than on records: a shipped
    /// game reaches a few dozen, and the ones that could grow without bound are the callees handed a
    /// fresh pointer every time. Past it the counts already held keep accumulating, new keys are
    /// refused, and `folded_full` says so.
    pub const FOLDED: usize = 8192;

    pub fn record(&mut self, call: Call) {
        *self.counts.entry(call.id).or_insert(0) += 1;
        *self.tally.entry(call.label).or_insert(0) += 1;
        if self.calls.len() >= Self::HISTORY {
            let drop = self.calls.len() - Self::HISTORY / 2;
            self.calls.drain(..drop);
            self.dropped.0 += drop as u64;
        }
        self.calls.push(call);
    }

    /// The dispatches still held, oldest first. [`Hle::call_total`] is how many there were.
    pub fn calls(&self) -> &[Call] {
        &self.calls
    }

    /// Every dispatch the run took, including the ones the history has dropped.
    pub fn call_total(&self) -> u64 {
        self.counts.values().sum()
    }

    /// How many times each label was reached, in no particular order. [`Hle::calls`] is capped; this is
    /// not, which is what makes it the right source for a report that adds the numbers up.
    pub fn tally(&self) -> Vec<(String, usize)> {
        self.tally
            .iter()
            .map(|(label, n)| ((*label).to_string(), *n as usize))
            .collect()
    }

    pub fn call_count(&self, id: u32) -> usize {
        self.counts.get(&id).copied().unwrap_or(0) as usize
    }

    pub fn record_downcall(&mut self, call: Downcall) {
        *self.down_counts.entry(call.addr).or_insert(0) += 1;
        // The counts that outlive the history. A hit is the common case and is one lookup; a miss
        // either adds a row or sets the flag that says the table is a floor.
        let key = (call.addr, call.args[1], call.args[2]);
        // A nested `if` rather than a guard: the scrutinee's mutable borrow lasts to the end of the
        // `match`, so asking how many keys the table holds inside a guard cannot compile.
        match self.folded_down.get_mut(&key) {
            Some(n) => *n += 1,
            None => {
                if self.folded_down.len() < Self::FOLDED {
                    self.folded_down.insert(key, 1);
                } else {
                    self.folded_full = true;
                }
            }
        }
        if self.downcalls.len() >= Self::HISTORY {
            let drop = self.downcalls.len() - Self::HISTORY / 2;
            self.downcalls.drain(..drop);
            self.dropped.1 += drop as u64;
        }
        self.downcalls.push(call);
    }

    /// Every call into the module, folded, busiest first. Unlike [`Hle::downcalls`] this is not capped
    /// by time — see [`Hle::folded_down`] for why that distinction is the whole point.
    pub fn folded_downcalls(&self) -> Vec<((u32, u32, u32), u64)> {
        let mut out: Vec<((u32, u32, u32), u64)> =
            self.folded_down.iter().map(|(k, v)| (*k, *v)).collect();
        // Ties broken by the key so two runs of the same pack print in the same order.
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// Whether the folded table ever refused a key, i.e. whether the list above is a floor.
    pub fn folded_full(&self) -> bool {
        self.folded_full
    }

    pub fn downcalls(&self) -> &[Downcall] {
        &self.downcalls
    }

    /// How many times the runtime branched to `addr`, i.e. to one particular vtable slot.
    pub fn downcall_count(&self, addr: u32) -> usize {
        self.down_counts.get(&addr).copied().unwrap_or(0) as usize
    }

    /// Did the guest dispatch through `Iface::Method` at all.
    pub fn saw(&self, iface: &str, method: &str) -> bool {
        let want = match (iface_index(iface), abi::slot_of(iface, method)) {
            (Some(i), Some(s)) => abi::method_id(i, s),
            _ => return false,
        };
        self.calls.iter().any(|c| c.id == want)
    }

    pub fn saw_helper(&self, field: &str) -> bool {
        self.calls
            .iter()
            .any(|c| is_helper_id(c.id) && helper_field(c.id).is_some_and(|f| f.name == field))
    }

    /// Name the arena cursor so the caller can report memory growth.
    pub fn set_heap(&mut self, base: u32) {
        self.heap_next = base;
    }

    pub fn heap_next(&self) -> u32 {
        self.heap_next
    }
}

impl Default for Hle {
    fn default() -> Self {
        Self::new()
    }
}

/// The dispatch id for `Iface::Method`, or the helper field of that name.
pub fn id_of(iface: &str, method: &str) -> Option<u32> {
    if iface == HELPER_STRUCT || iface == "helper" {
        let off = abi::offset_of(HELPER_STRUCT, method)?;
        return Some(helper_id(off));
    }
    abi::method_of(iface, method)
}

/// What a restored machine needs in order to recognise the pointers it is holding.
///
/// Not the histories and the folded counters (`calls`, `downcalls`, `counts`, `down_counts`, `tally`,
/// `folded_down`): those describe what a run *did*, for a person reading a report, and a machine
/// restored mid-run does not need them to go on running. `sizes` and the two cursors are a different
/// thing — they are how the next `Realloc` and the next `CreateInstance` place their blocks, so
/// leaving them out would move allocations a run has already made.
impl Hle {
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        use crate::mem::{put_string, put_u32, sorted_u32};
        // Sorted, because a map's iteration order is not state and two snapshots of one machine have
        // to be the same bytes.
        let mut vtables: Vec<(usize, u32)> = self.vtables.iter().map(|(k, v)| (*k, *v)).collect();
        vtables.sort_by_key(|(k, _)| *k);
        put_u32(out, vtables.len() as u32);
        for (index, addr) in vtables {
            put_u32(out, index as u32);
            put_u32(out, addr);
        }
        put_u32(out, self.vtable_cursor);
        put_u32(out, self.helper_table);
        let objects = sorted_u32(&self.objects);
        put_u32(out, objects.len() as u32);
        for (addr, object) in objects {
            put_u32(out, addr);
            put_string(out, object.iface);
            put_u32(out, object.clsid);
            put_u32(out, object.refs_off);
        }
        put_u32(out, self.object_cursor);
        put_u32(out, self.heap_next);
        let sizes = sorted_u32(&self.sizes);
        put_u32(out, sizes.len() as u32);
        for (addr, size) in sizes {
            put_u32(out, addr);
            put_u32(out, *size);
        }
    }

    /// Read back what [`Hle::write_state`] wrote.
    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut crate::mem::State) -> Result<(), String> {
        let count = st.small()?;
        let mut vtables = HashMap::with_capacity(count);
        for _ in 0..count {
            let index = st.u32()? as usize;
            let addr = st.u32()?;
            vtables.insert(index, addr);
        }
        let vtable_cursor = st.u32()?;
        let helper_table = st.u32()?;
        let count = st.small()?;
        let mut objects = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            let name = st.string()?;
            // A name this build's table has not got is a state it cannot honour: the vtable addresses
            // in it would point at slots nothing dispatches to.
            let iface = iface_index(&name)
                .and_then(iface_at)
                .map(|i| i.name)
                .ok_or_else(|| {
                    format!("a state naming an interface this build has not got: {name}")
                })?;
            let clsid = st.u32()?;
            let refs_off = st.u32()?;
            objects.insert(
                addr,
                Object {
                    addr,
                    iface,
                    clsid,
                    refs_off,
                    len: 0,
                },
            );
        }
        let object_cursor = st.u32()?;
        let heap_next = st.u32()?;
        let count = st.small()?;
        let mut sizes = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            let size = st.u32()?;
            sizes.insert(addr, size);
        }

        self.vtables = vtables;
        self.vtable_cursor = vtable_cursor;
        self.helper_table = helper_table;
        self.objects = objects;
        self.object_cursor = object_cursor;
        self.heap_next = heap_next;
        self.sizes = sizes;
        Ok(())
    }
}

#[cfg(test)]
#[path = "test/hle.rs"]
mod tests;
