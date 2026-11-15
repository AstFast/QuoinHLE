//! A high-level emulation (HLE) of the runtime BREW hands to application modules —
//! the COM-style `IShell` line that runs from classic BREW 1.x through 4.x, and the Brew MP
//! generation after it.
//!
//! **The target is classic BREW 1.x–4.x and Brew MP; the surface answered today is narrower.** What
//! this host serves is the classic 4.0.2 table. The 1.x–3.x releases have no table of their own, so
//! a version being in scope is never the same claim as a version already running. Brew MP's extra
//! interfaces are recorded and counted but not served.
//!
//! The guest is untouched ARM machine code: a real `.mod` flat image, executed by [`cpu`].
//! Everything it talks to is emulated at the level of *meaning* rather than of silicon: the
//! module never imports a host symbol, it only walks `PSHELL->pVtbl->[slot]`, and each slot is
//! a stub address we hand it. Jumping into a stub executes an `SVC` whose immediate is a
//! method id, which is where [`hle`] takes over and Rust implements the call.
//!
//! That seam is the whole design: it is engine-independent (the same stub page and the same
//! images work under a dynarmic `CallSpecial` backend), and it means the ABI surface the guest
//! sees is declared in one place — [`abi`], generated from the published BREW headers rather
//! than transcribed by hand.

pub mod abi;
pub mod aclock;
pub mod audio;
pub mod bbf;
pub mod bmp;
pub mod canvas;
pub mod cipher;
pub mod control;
pub mod cpu;
pub mod db;
pub mod device;
pub mod device_file;
pub mod disasm;
pub mod display;
#[cfg(feature = "ui")]
pub mod egui_window;
pub mod file;
pub mod font;
pub mod gl;
pub mod gles;
pub mod graphics;
pub mod handset;
pub mod hash;
pub mod heap;
pub mod hle;
pub mod jpegdecoder;
pub mod json;
pub mod keymap;
pub mod library;
pub mod loader;
pub mod locale;
pub mod makebbf;
#[cfg(feature = "ui")]
pub mod manager;
#[cfg(feature = "ui")]
pub mod manager_data;
pub mod mem;
#[cfg_attr(target_arch = "wasm32", path = "net_stub.rs")]
pub mod net;
pub mod par;
pub mod platform;
pub mod probe;
pub mod profile;
pub mod randomsource;
pub mod resource;
pub mod rsa;
pub mod screen;
pub mod runtime;
#[cfg_attr(
    any(target_arch = "wasm32", not(feature = "script")),
    path = "script_stub.rs"
)]
pub mod script;
pub mod seed;
pub mod seed_tables;
pub mod session;
pub mod sha1;
pub mod socket;
pub mod sockport;
pub mod sprite;
pub mod store;
pub mod symbol;
pub mod systemfont;
pub mod tls;
pub mod web;
pub mod widget;
#[cfg_attr(not(windows), path = "window_stub.rs")]
pub mod window;
pub use mem::Mem;

/// What this host calls itself, in the one place a player sees it: the window's title bar. The name is
/// deliberately neither of the two words the platform it emulates was sold under — those are marks, and
/// a title bar is the least defensible place to borrow one.
pub const APP_NAME: &str = "QuoinHLE";

/// Guest address map. One flat blob per module, PIC everywhere, page 0 left unmapped so a null
/// `PSHELL` faults instead of reading zeros.
pub mod layout {
    /// HLE stubs: one `SVC #method_id` per vtable method, 4 bytes each.
    pub const STUB_BASE: u32 = 0x0100_0000;
    /// Synthesized vtables (arrays of stub addresses) handed to the guest as read-only data.
    pub const VTABLE_BASE: u32 = 0x0200_0000;
    /// Objects the host allocates on behalf of the runtime.
    pub const OBJECT_BASE: u32 = 0x0400_0000;
    /// Where a `.mod` image is mapped. ROPI code reaches its data PC-relatively, so the base
    /// is free to choose; real handsets picked one per applet at load time.
    pub const MODULE_BASE: u32 = 0x1000_0000;
    /// Bump arena behind the module image: `ISHELL_MALLOC`, module heap, string scratch.
    ///
    /// Sized well past what a pack allocates, because how much is the pack's answer to a host
    /// question: a pack reads the reported colour depth literally and lays its art out at that many
    /// bytes a pixel, so what it asks for can be far larger than the art itself. Its null then goes
    /// straight into `memset`. `DeviceProfile::color_depth` reports the depth the host really backs,
    /// but nothing is reserved up front, so a larger reachable range costs nothing and a pack that
    /// over-allocates for any other reason should still not be the one that fails.
    pub const HEAP_BASE: u32 = 0x2000_0000;
    pub const HEAP_LIMIT: u32 = 0x2F00_0000;
    /// Full-descending stack; the top of it is the initial `SP`.
    pub const STACK_TOP: u32 = 0x3000_0000;
    pub const STACK_SIZE: u32 = 0x0008_0000;
    /// Returned to the dispatcher when the guest branches here instead of a real stub.
    pub const HALT: u32 = 0x0fff_fff0;
}
