// Moved out of `src/loader.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::abi;

/// A two-instruction ARM "module": `mov r0, #0` then `bx lr`, so it returns AEESUCCESS.
fn trivial_module() -> Vec<u8> {
    let mut v = Vec::new();
    for w in [0xe3a0_0000u32, 0xe12fff1e] {
        v.extend_from_slice(&w.to_le_bytes());
    }
    v
}

fn runtime() -> (Cpu, Hle, u32) {
    let mut cpu = Cpu::new(crate::Mem::new());
    cpu.mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let shell = hle
        .new_object(
            &mut cpu.mem,
            "IShell",
            abi::const_value("AEECLSID_SHELL").unwrap(),
            8,
            4,
            &[],
        )
        .unwrap();
    (cpu, hle, shell)
}

#[test]
fn the_loader_stuffs_the_helper_pointer_below_the_entry() {
    let (mut cpu, mut hle, shell) = runtime();
    let image = trivial_module();
    let loaded = load(&mut cpu, &mut hle, &image, shell, false).unwrap();
    assert_eq!(loaded.base, layout::MODULE_BASE);
    // Exactly what GET_HELPER() dereferences.
    assert_eq!(
        cpu.mem.read_u32(loaded.base - 4).unwrap(),
        hle.helper_table_addr()
    );
    assert_eq!(cpu.mem.read_u32(loaded.base - 8).unwrap(), AEESTDLIB_VER);
    // ...and the malloc slot it then indexes is a stub carrying the helper id.
    let malloc = abi::offset_of("AEEHelperFuncs", "malloc").unwrap();
    let tbl = hle.helper_table_addr();
    let stub = cpu.mem.read_u32(tbl + malloc).unwrap();
    let id = (cpu.mem.read_u32(stub).unwrap() & 0x00ff_ffff) as usize;
    assert_eq!(abi::helper_field(id as u32).map(|f| f.name), Some("malloc"));
}

#[test]
fn the_second_argument_is_the_helper_table() {
    // The SDK spells it `void *ph`, and a pack that inflates and runs its own module reads the
    // helper table out of it: it calls `[ph + 0x68]` — which is `AEEHelperFuncs::malloc` — and
    // copies `[base - 4]` into the new image's `entry - 4`. That
    // block used to be zeroes, and `bx 0` is where the pack stopped.
    let (mut cpu, mut hle, shell) = runtime();
    let image = trivial_module();
    let loaded = load(&mut cpu, &mut hle, &image, shell, false).unwrap();
    assert_eq!(cpu.r[0], shell);
    assert_eq!(cpu.r[1], hle.helper_table_addr(), "ph is the helper table");
    assert_eq!(cpu.r[2], loaded.out);
    // The slot the pack reaches through it is a stub rather than a zero, which is the difference
    // between a call and a branch to address zero.
    let malloc = abi::offset_of("AEEHelperFuncs", "malloc").unwrap();
    assert_ne!(cpu.mem.read_u32(cpu.r[1] + malloc).unwrap(), 0);
}

#[test]
fn a_real_entry_point_returns_and_we_read_the_out_parameter() {
    let (mut cpu, mut hle, shell) = runtime();
    let loaded = load(&mut cpu, &mut hle, &trivial_module(), shell, false).unwrap();
    assert_eq!(cpu.pc, loaded.base);
    assert_eq!(cpu.r[14], layout::HALT);
    let stop = cpu.run(100);
    assert!(
        matches!(stop, crate::cpu::Stop::Halted { .. }),
        "{:?}",
        stop
    );
    assert_eq!(cpu.r[0], 0);
    assert_eq!(created_module(&cpu, &loaded).unwrap(), 0);
}

/// A flat SDK sample carries no header, so nothing is taken from it.
#[test]
fn a_flat_image_keeps_every_byte_it_ships() {
    let (mut cpu, mut hle, shell) = runtime();
    let loaded = load(&mut cpu, &mut hle, &trivial_module(), shell, false).unwrap();
    assert_eq!(loaded.writable, None);
    // And asking it to clear must leave the image alone rather than faulting.
    assert!(clear_writable_area(&mut cpu.mem, &loaded).is_ok());
    assert_eq!(cpu.mem.read_u32(loaded.base).unwrap(), 0xe3a0_0000);
}

/// A `BREW`-headered image puts its relocation data where the module's writable area is, and the
/// blob has to stay readable in place until the entry point has finished with it — the module
/// reaches its own fixup routine at `refdata + size`. Only the fixup stream may be cleared: what
/// sits before it belongs to the module for the rest of its life.
#[test]
fn the_writable_area_is_reported_and_cleared_only_after_the_module_is_done_with_it() {
    let mut image = vec![0u8; 0x400];
    for (i, w) in [0xe3a0_0000u32, 0xe12f_ff1e].iter().enumerate() {
        image[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    image[0x08..0x0c].copy_from_slice(b"BREW");
    for (off, v) in [
        (IMAGE, 0x200u32),
        (REFOFF, 0x100),
        (FIXUPOFF, 0x40),
        (REFSIZE, 0x40),
    ] {
        image[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    // The reference area, in the shape a shipped module lays it out: initialised tables the
    // module keeps reading, then the fixup stream it consumes once. A store binary keeps its
    // zlib code-length tables in exactly that first part, which is why clearing from `refdata`
    // gave its inflate an all-zero table and cost the app its whole start-up.
    for i in 0..0x80 {
        image[0x200 + 0x100 + i] = 0xa5;
    }

    let (mut cpu, mut hle, shell) = runtime();
    let loaded = load(&mut cpu, &mut hle, &image, shell, false).unwrap();
    assert_eq!(
        loaded.writable,
        Some((loaded.base + 0x200 + 0x100 + 0x40, 0x40)),
        "the range the module's own arithmetic reaches"
    );
    let (lo, size) = loaded.writable.unwrap();
    assert_eq!(
        cpu.mem.read_u8(lo).unwrap(),
        0xa5,
        "mapping alone must not disturb the blob"
    );

    clear_writable_area(&mut cpu.mem, &loaded).unwrap();
    for i in 0..0x40 {
        assert_eq!(
            cpu.mem.read_u8(loaded.base + 0x200 + 0x100 + i).unwrap(),
            0xa5,
            "the tables ahead of the stream are the module's, not ours to spend"
        );
    }
    for i in 0..size {
        assert_eq!(cpu.mem.read_u8(lo + i).unwrap(), 0, "cleared at {i:#x}");
    }
    assert_eq!(
        cpu.mem.read_u8(lo + size).unwrap(),
        0x00,
        "and nothing past the end was touched"
    );
    assert_eq!(
        cpu.mem.read_u32(loaded.base + IMAGE as u32).unwrap(),
        0x200,
        "the header the module reads is left as the file had it"
    );
}

#[test]
fn containers_are_rejected_with_an_explanation() {
    let (mut cpu, mut hle, shell) = runtime();
    let mut elf = b"\x7fELF".to_vec();
    elf.extend_from_slice(&[0u8; 60]);
    assert!(matches!(
        load(&mut cpu, &mut hle, &elf, shell, false),
        Err(LoadError::NotAFlatImage { .. })
    ));
    // Bits 31:28 = 0b1111 is the encoding no ARM architecture after ARMv3 defines, so a word
    // starting with it cannot be the head of `AEEMod_Load`.
    let reserved_condition = vec![0x00, 0x00, 0xf0, 0xf7];
    assert!(matches!(
        load(&mut cpu, &mut hle, &reserved_condition, shell, false),
        Err(LoadError::EntryNotArm { .. })
    ));
}
