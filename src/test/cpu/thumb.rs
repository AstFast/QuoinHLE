// Moved out of `src/cpu/thumb.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::cpu::{CpuFault, FaultKind, FLAG_C, FLAG_Z};
use crate::mem::Mem;

const CODE: u32 = 0x0001_0000;
const STACK: u32 = 0x0002_0000;

/// Execute `budget` halfwords from a mapped page, starting in Thumb state.
///
/// As in the ARM tests, the encodings are not invented: they are either bytes taken from
/// `helloworld.mod`'s handler or halfwords confirmed against an independent decoding this session. A decoder
/// that misplaces one field still runs, so the assertions check *which* register and *which*
/// address, not just that execution continued.
fn run_with(halfwords: &[u16], budget: u64, setup: impl FnOnce(&mut Cpu)) -> Cpu {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.map(STACK - 0x1000, 0x2000);
    for (i, h) in halfwords.iter().enumerate() {
        cpu.mem
            .write_u16(CODE + i as u32 * 2, u32::from(*h))
            .unwrap();
    }
    cpu.pc = CODE;
    cpu.set_thumb(true);
    cpu.r[13] = STACK;
    setup(&mut cpu);
    match cpu.run(budget) {
        Stop::Budget { .. } => {}
        other => panic!("unexpected stop: {other:?}"),
    }
    cpu
}

fn run(halfwords: &[u16], setup: impl FnOnce(&mut Cpu)) -> Cpu {
    run_with(halfwords, halfwords.len() as u64, setup)
}

/// The group name a halfword refuses to execute, with the address it stopped at.
fn refusal(halfword: u16) -> (&'static str, u32) {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u16(CODE, u32::from(halfword)).unwrap();
    cpu.pc = CODE;
    cpu.set_thumb(true);
    match cpu.run(1) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Unimplemented { group, .. },
            pc,
            ..
        }) => (group, pc),
        other => panic!("expected a named unimplemented for {halfword:#06x}, got {other:?}"),
    }
}

#[test]
fn the_real_handler_prologue_builds_its_frame() {
    // `push {r4-r7,lr}` / `sub sp,#0x2c` / `ldr r1,[pc,#0x4c]` / `add r5,sp,#0x14` /
    // `add r1,pc`, the first five halfwords of `HelloModule_HandleEvent`.
    let setup = |cpu: &mut Cpu| {
        cpu.mem.write_u32(CODE + 0x54, 0x2000_abc0).unwrap();
        cpu.r[4] = 0x11;
        cpu.r[5] = 0x22;
        cpu.r[6] = 0x33;
        cpu.r[7] = 0x44;
        cpu.r[14] = CODE + 0x801;
    };
    // The load on its own, because the `add r1, pc` below moves the same register. It sits at
    // CODE+4, so the literal comes from `(instruction + 4) & ~3 + 0x4c`.
    let cpu = run(&[0xb5f0, 0xb08b, 0x4913], setup);
    assert_eq!(cpu.r[1], 0x2000_abc0);
    // Five words for the push, then the 0x2c-byte local frame.
    assert_eq!(cpu.r[13], STACK - 0x14 - 0x2c);
    assert_eq!(cpu.mem.read_u32(STACK - 0x14).unwrap(), 0x11, "r4 lowest");
    assert_eq!(cpu.mem.read_u32(STACK - 0x10).unwrap(), 0x22);
    assert_eq!(cpu.mem.read_u32(STACK - 0x4).unwrap(), CODE + 0x801);

    let cpu = run(&[0xb5f0, 0xb08b, 0x4913, 0xad05, 0x4479], setup);
    assert_eq!(
        cpu.r[5],
        cpu.r[13] + 0x14,
        "`add r5, sp, #0x14` after the frame"
    );
    // Hi-register `add` is the *unaligned* bias, and the destination is also the source: this
    // one ran at CODE+8, so r1 ends up the literal plus CODE+0xc.
    assert_eq!(cpu.r[1], 0x2000_abc0 + CODE + 0xc);
}

#[test]
fn a_literal_base_is_aligned_however_the_instruction_is_offset() {
    let cpu = run(&[0x0000, 0x4805], |cpu| {
        cpu.mem.write_u32(CODE + 0x18, 0x1111).unwrap();
        cpu.mem.write_u32(CODE + 0x1c, 0x2222).unwrap();
    });
    assert_eq!(cpu.r[0], 0x1111, "ldr literal from the aligned base");

    // `adr r0, #0x14` (0xa005) uses the same aligned base and sets no flags.
    let cpu = run(&[0x0000, 0xa005], |_| {});
    assert_eq!(cpu.r[0], CODE + 4 + 0x14);
}

#[test]
fn format_four_alu_updates_the_register_it_names_first() {
    // `ands r6, r2` is r6 = r6 & r2, not a three-operand form: reading the second operand
    // from bits[8:6] makes `mvns r6, r2` come out as `mvns r6, r7`.
    let cases: &[(&str, u16, u32, u32, u32)] = &[
        ("ands", 0x4016, 0xf0, 0x3c, 0x30),
        ("eors", 0x4056, 0xf0, 0x3c, 0xcc),
        ("lsls", 0x4096, 1, 3, 8),
        ("lsrs", 0x40d6, 8, 3, 1),
        ("orrs", 0x4316, 0xf0, 0x3c, 0xfc),
        ("bics", 0x4396, 0xf0, 0x3c, 0xc0),
        ("muls", 0x4356, 0x1_0000, 0x1_0000, 0),
        ("rsbs", 0x4256, 0xaaaa, 5, (-5i32) as u32),
        ("mvns", 0x43d6, 0xaaaa, 0x0f, 0xffff_fff0),
    ];
    for (name, insn, rd, rm, want) in cases {
        let cpu = run(&[*insn], |cpu| {
            cpu.r[6] = *rd;
            cpu.r[2] = *rm;
        });
        assert_eq!(cpu.r[6], *want, "{name} ({insn:#06x})");
        assert_eq!(cpu.r[7], 0, "{name} must not touch a third register");
    }
}

#[test]
fn compare_and_test_leave_the_destination_alone() {
    // `cmp`/`tst` are the flag-only forms of format 4 (0x4296, 0x4216). Storing the result is
    // the classic mistake, and the ARM tests catch it on the ARM side; this is the Thumb side.
    let cpu = run(&[0x4296], |cpu| {
        cpu.r[6] = 0xaa;
        cpu.r[2] = 0xaa;
    });
    assert_eq!(cpu.r[6], 0xaa);
    assert!(cpu.flag(FLAG_Z), "cmp r6, r2 with equal values");
    assert!(cpu.carry(), "an equal compare borrows nothing");

    let cpu = run(&[0x4216], |cpu| {
        cpu.r[6] = 0xf0;
        cpu.r[2] = 0x0f;
        cpu.set_flag(FLAG_C, true);
    });
    assert_eq!(cpu.r[6], 0xf0, "tst never writes");
    assert!(cpu.flag(FLAG_Z), "0xf0 & 0x0f is zero");
    assert!(cpu.carry(), "tst leaves the carry alone");
}

#[test]
fn a_zero_shift_amount_means_thirty_two() {
    // Confirmed against the independent decoding: `lsrs r6,r0,#0` and `asrs r6,r0,#0` both print as
    // a shift by 0x20, while `lsls #0` is the encoding of `movs`.
    let cpu = run(&[0x1006], |cpu| cpu.r[0] = 0x8000_0000);
    assert_eq!(cpu.r[6], 0xffff_ffff, "asr #32 fills with the sign");
    assert!(cpu.carry());

    let cpu = run(&[0x0806], |cpu| cpu.r[0] = 0x8000_0001);
    assert_eq!(cpu.r[6], 0, "lsr #32 leaves nothing");
    assert!(cpu.carry(), "the last bit out is the sign");

    let cpu = run(&[0x0006], |cpu| {
        cpu.r[0] = 7;
        cpu.set_flag(FLAG_C, true);
    });
    assert_eq!(cpu.r[6], 7, "lsl #0 is movs");
    assert!(cpu.carry(), "movs leaves the carry alone");
}

#[test]
fn a_signed_load_extends_and_a_sp_relative_rd_sits_above_the_immediate() {
    // `ldrsb r6,[r2,r4]` (0x5716), `ldrsh r5,[r2,r3]` (0x5ed5), then the frame forms
    // `str r2,[sp,#0x18]` (0x9206) and `ldr r3,[sp,#0x18]` (0x9b06).
    let cpu = run(&[0x5716, 0x5ed5, 0x9206, 0x9b06], |cpu| {
        cpu.r[2] = STACK - 0x800;
        cpu.r[3] = 4;
        cpu.mem.write_u8(STACK - 0x800, 0x80).unwrap();
        cpu.mem.write_u16(STACK - 0x800 + 4, 0x8000).unwrap();
        cpu.mem.write_u32(STACK + 0x18, 0x1234).unwrap();
        cpu.r[6] = 0xdead;
    });
    assert_eq!(cpu.r[6], 0xffff_ff80, "ldrsb sign-extends the byte");
    assert_eq!(cpu.r[5], 0xffff_8000, "ldrsh sign-extends the halfword");
    // The two SP forms name `r2` and `r3`; a decoder reading Rd from bits[2:0] would have
    // written over the value `ldrsb` just produced instead.
    assert_eq!(cpu.mem.read_u32(STACK + 0x18).unwrap(), STACK - 0x800);
    assert_eq!(cpu.r[3], STACK - 0x800, "`ldr r3, [sp, #0x18]`");
}

#[test]
fn the_module_word_loads_the_display_slot() {
    // `ldr r2, [r1, #0x1c]` — 0x69ca, the handler's `IDisplay::Update` fetch. imm5 is in
    // bits[10:6] and scales by four, so the address is the base plus 28.
    let cpu = run(&[0x69ca], |cpu| {
        cpu.r[1] = STACK - 0x800;
        cpu.mem
            .write_u32(STACK - 0x800 + 0x1c, 0x0100_00a0)
            .unwrap();
        cpu.mem
            .write_u32(STACK - 0x800 + 0x70, 0xdead_beef)
            .unwrap();
    });
    assert_eq!(cpu.r[2], 0x0100_00a0);
}

#[test]
fn a_pair_branch_links_and_reaches_the_fitted_target() {
    // The handler's own call: 0xf7ff:0xede4 is -1080 from the pair, and bit12 of the second
    // halfword is clear, so it is a `blx` into the module's ARM code. The link value keeps
    // bit 0, which is what brings the caller back in Thumb state.
    let cpu = run_with(&[0xf7ff, 0xede4], 1, |_| {});
    assert_eq!(cpu.pc, (CODE + 4 - 1080) & !3);
    assert_eq!(cpu.r[14], (CODE + 4) | 1);
    assert!(!cpu.thumb(), "bit12 clear is a `blx` to ARM state");

    // Set bit12 and the same pair is a `bl` that stays Thumb; the target keeps its bit 1.
    let cpu = run_with(&[0xf000, 0xf800], 1, |_| {});
    assert!(cpu.thumb());
    assert_eq!(cpu.pc, CODE + 4);
    assert_eq!(cpu.r[14], (CODE + 4) | 1);

    // A second halfword that is not one is not a pair at all.
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u16(CODE, 0xf000).unwrap();
    cpu.mem.write_u16(CODE + 2, 0x0000).unwrap();
    cpu.pc = CODE;
    cpu.set_thumb(true);
    assert!(matches!(
        cpu.run(1),
        Stop::Fault(CpuFault {
            kind: FaultKind::Illegal { .. },
            ..
        })
    ));
}

#[test]
fn branch_and_link_register_select_the_state_of_the_target() {
    // `bx r2` (0x4710) and `blx r2` (0x4790); bit 0 of the target is the state selector,
    // which is how a Thumb handler reaches a host stub and how an ARM function reaches Thumb.
    let cpu = run(&[0x4790], |cpu| cpu.r[2] = CODE + 0x100);
    assert_eq!(cpu.pc, CODE + 0x100);
    assert!(!cpu.thumb());
    assert_eq!(cpu.r[14], (CODE + 2) | 1, "returns to the next halfword");

    let cpu = run(&[0x4710], |cpu| cpu.r[2] = CODE + 0x101);
    assert!(cpu.thumb(), "bit 0 keeps Thumb state");
    assert_eq!(cpu.pc, CODE + 0x100);
}

#[test]
fn a_pop_of_pc_returns_in_the_state_the_word_says() {
    // The handler's own epilogue: `push {r4-r7,lr}` ... `pop {r4-r7,pc}` (0xb5f0, 0xbdf0).
    let cpu = run(&[0xb5f0, 0x2401, 0x2702, 0xbdf0], |cpu| {
        cpu.r[4] = 0x11;
        cpu.r[7] = 0x44;
        cpu.r[14] = CODE + 0x801;
    });
    assert_eq!(cpu.r[13], STACK, "the frame comes back whole");
    assert_eq!(cpu.r[4], 0x11, "`movs r4, #1` must not survive the pop");
    assert_eq!(cpu.r[7], 0x44);
    assert_eq!(cpu.pc, CODE + 0x800);
    assert!(cpu.thumb());
}

#[test]
fn a_conditional_branch_only_moves_the_pc_when_the_condition_holds() {
    // `beq #+6` at CODE: the offset is relative to `instruction + 4`, and the sign comes from
    // bit 7 of the immediate.
    let cpu = run(&[0xd003], |cpu| cpu.set_flag(FLAG_Z, true));
    assert_eq!(cpu.pc, CODE + 4 + 6);

    let cpu = run(&[0xd003], |cpu| cpu.set_flag(FLAG_Z, false));
    assert_eq!(cpu.pc, CODE + 2, "not taken, so just the next halfword");

    let cpu = run(&[0xd1fd], |cpu| cpu.set_flag(FLAG_Z, false));
    assert_eq!(cpu.pc, CODE + 4 - 6, "the immediate is signed");

    let cpu = run(&[0xe7fd], |cpu| cpu.r[14] = 0x1234);
    assert_eq!(cpu.pc, CODE + 4 - 6);
    assert_eq!(cpu.r[14], 0x1234, "a plain `b` must not link");
}

#[test]
fn a_block_transfer_always_moves_its_base() {
    // `stm r3!, {r4,r5}` (0xc330) and `ldm r3, {r4,r5}` (0xcb30). Thumb has no non-writeback
    // form here, which is what makes `push`/`pop` share the shape — so each direction needs
    // its own base, as the store leaves the base past the words it wrote.
    let cpu = run(&[0xc330], |cpu| {
        cpu.r[3] = STACK - 0x800;
        cpu.r[4] = 0x55;
        cpu.r[5] = 0x66;
        cpu.r[6] = 0x77;
    });
    assert_eq!(cpu.r[3], STACK - 0x800 + 8);
    assert_eq!(cpu.mem.read_u32(STACK - 0x800).unwrap(), 0x55);
    assert_eq!(cpu.mem.read_u32(STACK - 0x800 + 4).unwrap(), 0x66);
    assert_eq!(cpu.r[6], 0x77, "the list holds r4 and r5 only");

    let cpu = run(&[0xcb30], |cpu| {
        cpu.r[3] = STACK - 0x800;
        cpu.mem.write_u32(STACK - 0x800, 0x55).unwrap();
        cpu.mem.write_u32(STACK - 0x800 + 4, 0x66).unwrap();
    });
    assert_eq!(cpu.r[3], STACK - 0x800 + 8, "and so does the load");
    assert_eq!(cpu.r[4], 0x55);
    assert_eq!(cpu.r[5], 0x66);
}

#[test]
fn a_load_whose_base_is_in_its_own_list_keeps_the_loaded_word() {
    let cpu = run(&[0xc807], |cpu| {
        cpu.r[0] = STACK - 0x800;
        cpu.mem.write_u32(STACK - 0x800, 0).unwrap();
        cpu.mem.write_u32(STACK - 0x800 + 4, 0).unwrap();
        cpu.mem.write_u32(STACK - 0x800 + 8, 2).unwrap();
    });
    assert_eq!(
        cpu.r[0], 0,
        "the word it loaded, not the address it loaded past"
    );
    assert_eq!(cpu.r[2], 2);

    // The store direction still moves its base, because there the transfer only reads `Rn`.
    let cpu = run(&[0xc007], |cpu| {
        cpu.r[0] = STACK - 0x800;
        cpu.r[1] = 0x1111;
        cpu.r[2] = 0x2222;
    });
    assert_eq!(cpu.r[0], STACK - 0x800 + 12);
    assert_eq!(cpu.mem.read_u32(STACK - 0x800 + 8).unwrap(), 0x2222);
}

#[test]
fn thumb_add_sub_immediate() {
    // `adds r6,#0x20` from `0x1b0` and `adds r0,#0xc` from `0x1be` of `helloworld.mod`, where
    // the ARMv4T reading of the same bytes would put a code address in both registers.
    let cpu = run(&[0x3620, 0x300c], |cpu| {
        cpu.r[0] = 0x1d + 0x20;
        cpu.r[6] = 0x1d;
    });
    assert_eq!(cpu.r[6], 0x3d, "r6, not an address derived from pc");
    assert_eq!(
        cpu.r[0], 0x49,
        "the size the malloc call at 0x1c0 is handed"
    );
    assert!(!cpu.carry());

    // `subs r0,#0x20`: the group updates the flags like every other Thumb arithmetic form.
    let cpu = run(&[0x3820], |cpu| cpu.r[0] = 0x20);
    assert_eq!(cpu.r[0], 0);
    assert!(cpu.cpsr & FLAG_Z != 0);
    assert!(cpu.cpsr & FLAG_C != 0, "no borrow");
}

#[test]
fn later_architectures_are_refused_by_name() {
    // `0xb148` really is `cbz`: both would run as something plausible if the decoder guessed.
    // `0xdf2a` cannot carry an HLE id at all.
    for (halfword, group) in [
        (0xb148u16, "ARMv6+ stack/extend"),
        (0xbf00, "ARMv6+ stack/hint/IT"),
        (0xe800, "Thumb-2 or ARMv6+"),
        (0xde00, "ARMv6+ UDF"),
        (0xdf2a, "Thumb SVC (too few id bits)"),
    ] {
        let (name, pc) = refusal(halfword);
        assert_eq!(name, group, "{halfword:#06x}");
        assert_eq!(pc, CODE, "{halfword:#06x} stops at its own address");
    }
}
