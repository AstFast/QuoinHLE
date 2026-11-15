use crate::abi::generated::AEESTDLIB_VER;
use crate::cpu::Cpu;
use crate::hle::Hle;
use crate::layout;
use crate::mem::Fault;
use crate::mem::MResult;

#[derive(Debug, Clone, Copy)]
pub struct Loaded {
    /// Guest address of `AEEMod_Load`, i.e. the first byte of the image.
    pub base: u32,
    pub len: u32,
    /// Guest address of the `IModule**` out-parameter we pass as the third argument.
    pub out: u32,
    /// For a `BREW`-headered image, the guest range that held the relocation blob and now reads
    /// as the zeroed writable area the module was linked for. `None` for a flat image.
    pub writable: Option<(u32, u32)>,
}

#[derive(Debug)]
pub enum LoadError {
    Empty,
    /// An ELF or PE has been handed to a flat-image loader.
    NotAFlatImage {
        first4: [u8; 4],
    },
    /// Word 0 is not plausibly an ARM instruction, so this is not an ARM-state `.mod`.
    EntryNotArm {
        word: u32,
    },
    Mem(Fault),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Empty => write!(f, "module image is empty"),
            LoadError::NotAFlatImage { first4 } => write!(
                f,
                "expected a flat .mod blob, found {:02x} {:02x} {:02x} {:02x} — a container, \
                 not the linker's ER_RO segment",
                first4[0], first4[1], first4[2], first4[3]
            ),
            LoadError::EntryNotArm { word } => write!(
                f,
                "entry word {:#010x} is not a decodable ARM instruction; is this module \
                 Thumb-encoded?",
                word
            ),
            LoadError::Mem(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<Fault> for LoadError {
    fn from(e: Fault) -> Self {
        LoadError::Mem(e)
    }
}

/// `0xFFFF...` in the condition field has no encoding in any ARM architecture after ARMv3, so
/// a word starting with those bits is not the head of an ARM function.
pub fn plausibly_arm(word: u32) -> bool {
    word >> 28 != 0xf
}

/// What [`load`] would answer about these bytes **before it touches an address**, or `None` when it
/// would go ahead.
///
/// This is the load-time refusals pulled out into one public function so a caller that has to *describe*
/// an image — `crate::probe`, which never builds a machine — cannot grow its own copy of the rules and
/// quietly drift from them. `load` below calls it, so the two answers are the same answer by
/// construction.
///
/// What it deliberately does **not** cover: the refusals that need a machine to discover. `load` can
/// still fail with `LoadError::Mem` when the helper table cannot be built, the arena cannot give it the
/// out-parameter, or a page write lands outside what is mapped — none of which is a fact about the file.
pub fn precheck(image: &[u8], entry_thumb: bool) -> Option<LoadError> {
    if image.len() < 4 {
        return Some(LoadError::Empty);
    }
    let mut first4 = [0u8; 4];
    first4.copy_from_slice(&image[..4]);
    if &first4 == b"\x7fELF" || &first4 == b"MZ\x90\x00" {
        return Some(LoadError::NotAFlatImage { first4 });
    }
    let word = u32::from_le_bytes(first4);
    if !entry_thumb && !plausibly_arm(word) {
        return Some(LoadError::EntryNotArm { word });
    }
    None
}

/// Map the image, write the two words a module expects below its entry point, park the CPU at
/// offset 0 with the arguments an `AEEMod_Load` call takes, and leave `LR` pointing at the halt
/// sentinel so the entry function returning ends the run.
pub fn load(
    cpu: &mut Cpu,
    hle: &mut Hle,
    image: &[u8],
    shell: u32,
    entry_thumb: bool,
) -> Result<Loaded, LoadError> {
    load_at(cpu, hle, image, shell, entry_thumb, layout::MODULE_BASE)
}

/// The same loader at a caller-chosen base, for a library module that has to live beside the
/// applet's own image instead of replacing it.
pub fn load_at(
    cpu: &mut Cpu,
    hle: &mut Hle,
    image: &[u8],
    shell: u32,
    entry_thumb: bool,
    base: u32,
) -> Result<Loaded, LoadError> {
    if let Some(e) = precheck(image, entry_thumb) {
        return Err(e);
    }

    let len = image.len() as u32;
    hle.build_helper_table(&mut cpu.mem)
        .map_err(LoadError::Mem)?;
    let helper = hle.helper_table_addr();

    // The page below the image is where the loader's two words live.
    cpu.mem.map(base - 8, 8 + len);
    cpu.mem.write_u32(base - 8, AEESTDLIB_VER)?;
    cpu.mem.write_u32(base - 4, helper)?;
    cpu.mem.write_bytes(base, image)?;
    let writable = writable_area(base, image);

    let out = cpu.mem.alloc(4, 4)?;
    cpu.mem.write_u32(out, 0)?;

    cpu.mem
        .map(layout::STACK_TOP - layout::STACK_SIZE, layout::STACK_SIZE);
    cpu.r[13] = layout::STACK_TOP - 8;
    cpu.r[14] = layout::HALT;
    cpu.set_thumb(entry_thumb);
    cpu.pc = base;
    cpu.r[0] = shell;
    // `ph`, the second argument of `AEEMod_Load`. A pack that inflates and runs its own module reads
    // the helper table out of it: it calls `[ph + 0x68]`, which is `AEEHelperFuncs::malloc`, and
    // copies `[base - 4]` into the new image's `entry - 4`, which is
    // this table again by the other convention. Zeroes there are a branch to address zero.
    cpu.r[1] = helper;
    cpu.r[2] = out;
    Ok(Loaded {
        base,
        len,
        out,
        writable,
    })
}

const IMAGE: usize = 0x18;
const REFOFF: usize = 0x1c;
/// How far the fixup stream sits inside the reference area. Everything before it is the table of
/// entries themselves, and everything from it for [`REFSIZE`] bytes is what the module consumes and
/// does not re-zero — which is the range that has to go back cleared.
const FIXUPOFF: usize = 0x20;
const REFSIZE: usize = 0x28;

/// Where a `BREW` header says the relocation blob is, as the four words this module reads.
///
/// `pub` for one reason: `crate::probe` reports these offsets for an image it will never load, and the
/// last thing this repository needs is a second copy of the layout printed out by a second reader. The
/// numbers themselves stay owned here, where the machine-code evidence for them is written down.
pub struct Relocation {
    pub image: u32,
    pub ref_off: u32,
    pub fixup_off: u32,
    pub ref_size: u32,
}

/// The header's four words, or `None` when the image carries no `BREW` magic.
pub fn relocation(image: &[u8]) -> Option<Relocation> {
    if image.get(0x08..0x0c) != Some(b"BREW") {
        return None;
    }
    let word = |off: usize| {
        image
            .get(off..off + 4)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
    };
    Some(Relocation {
        image: word(IMAGE),
        ref_off: word(REFOFF),
        fixup_off: word(FIXUPOFF),
        ref_size: word(REFSIZE),
    })
}

/// Whether the header's own range lands inside the bytes there are — **the same test `writable_area`
/// applies**, including the part of it that ignores `image`: the bound is on the reference area's own
/// offsets, not on the base-relative address. `false` is the loader's verdict that the header is not
/// describing this file.
pub fn relocation_describes(r: &Relocation, len: usize) -> bool {
    r.ref_size != 0
        && r.ref_off
            .checked_add(r.fixup_off)
            .and_then(|a| a.checked_add(r.ref_size))
            .is_some_and(|end| usize::try_from(end).is_ok_and(|end| end <= len))
}

/// Where the spent part of a `BREW`-headered module's relocation data sits, which is also where its
/// writable data has to be.
///
/// The blob is not extra memory the loader may or may not map: the linker parked it inside the
/// module's own image, because that way the file stays one contiguous copy. The module reads it only
/// while `AEEMod_Load` runs, and afterwards the range should hold its statics — zeroed, since a
/// position-independent module keeps no initialised static data *in the range it relocates through*.
/// Left as mapped, every pointer the app resolves into the range comes back as a relocation entry,
/// so a `this` dereference aborts a few hundred thousand instructions into what is otherwise a
/// healthy start.
///
/// Only the fixup stream may be cleared, and that distinction is measured on a store binary rather
/// than reasoned out: the reference area *begins* with data the module keeps reading after
/// `AEEMod_Load` — it may be the module's own zlib tables (most of the words at `refdata` are
/// non-zero in the file) — so clearing from `refdata` hands its inflate a
/// zeroed code-length table, and the app spent a whole start-up deciding its language file was
/// corrupt. The stream starts at `refdata + FIXUPOFF` and runs for `REFSIZE`, and the end of that
/// range is the fixup routine itself: on all 14 `BREW`-headered modules in the corpus plus the store
/// sample, the eight bytes at `refdata + FIXUPOFF + REFSIZE` open with the same instruction word
/// (`0xe92d00f0`, the `push` that starts the fixup routine), which is what makes the range a fact about
/// the format rather than a fitted constant.
///
/// The blob cannot be moved out to a block of its own, though: the module reaches its fixup routine
/// at `refdata + size`, so a copy elsewhere makes the module branch into the copy — measured as a
/// fetch from the heap. What is left is to clear the range in place, and that cannot happen here
/// either, because the module is still reading the blob at that point. So this only reports the
/// range, and [`crate::Runtime::load`] clears it once the entry point has returned.

fn writable_area(base: u32, image: &[u8]) -> Option<(u32, u32)> {
    let r = relocation(image)?;
    if !relocation_describes(&r, image.len()) {
        // The header is not describing this file.
        return None;
    }
    Some((
        base.wrapping_add(r.image)
            .wrapping_add(r.ref_off)
            .wrapping_add(r.fixup_off),
        r.ref_size,
    ))
}

/// The `IModule*` the entry point wrote back, if it wrote one.
pub fn created_module(cpu: &Cpu, loaded: &Loaded) -> MResult<u32> {
    cpu.mem.read_u32(loaded.out)
}

/// Hand the module the zeroed statics it was linked for, now that its entry point has read the last
/// of the relocation blob sitting in that range.
pub fn clear_writable_area(mem: &mut crate::Mem, loaded: &Loaded) -> MResult<()> {
    match loaded.writable {
        Some((lo, size)) => mem.fill(lo, size, 0),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "test/loader.rs"]
mod tests;
