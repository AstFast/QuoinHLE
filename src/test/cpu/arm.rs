// Moved out of `src/cpu/arm.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::cpu::{CpuFault, FaultKind};
use crate::mem::Mem;

const CODE: u32 = 0x0001_0000;
const STACK: u32 = 0x0002_0000;

/// Execute a whole table of ARM words from a mapped page, with `setup` applied first.
///
/// The words below are not invented: each is either a real word from `helloworld.mod` or one
/// assembled by hand and confirmed against the encoding, so a decoder regression shows up
/// as a wrong address or a wrong value rather than as a plausible-looking run.
fn run(words: &[u32], setup: impl FnOnce(&mut Cpu)) -> Cpu {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.map(STACK - 0x1000, 0x2000);
    for (i, w) in words.iter().enumerate() {
        cpu.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
    }
    cpu.pc = CODE;
    cpu.r[13] = STACK;
    setup(&mut cpu);
    match cpu.run(words.len() as u64) {
        Stop::Budget { .. } => {}
        other => panic!("unexpected stop: {other:?}"),
    }
    cpu
}

/// The first nine instructions of `AEEMod_Load`, straight out of the module.
const PROLOGUE: &[u32] = &[
    0xe52d_e004, // str lr, [sp, #-4]!
    0xe24d_d00c, // sub sp, sp, #0xc
    0xe3a0_3000, // mov r3, #0
    0xe58d_3000, // str r3, [sp]
    0xe58d_3004, // str r3, [sp, #4]
    0xe1a0_3002, // mov r3, r2
    0xe1a0_2001, // mov r2, r1
    0xe1a0_1000, // mov r1, r0
    0xe3a0_0014, // mov r0, #0x14
];

#[test]
fn the_two_halfword_offset_forms_read_their_offset_from_the_right_bits() {
    // strh r0, [r1, r4]  — register offset: Rm is bits[3:0], bits[11:8] are should-be-zero.
    let cpu = run(&[0xe181_00b4], |cpu| {
        cpu.r[0] = 0xdead_beef;
        cpu.r[1] = STACK;
        cpu.r[4] = 8;
    });
    assert_eq!(cpu.mem.read_u16(STACK + 8).unwrap(), 0xbeef);
    assert_eq!(
        cpu.mem.read_u16(STACK + 4).unwrap(),
        0,
        "not the immediate reading"
    );

    // strh r0, [r4, #0x50]  — immediate offset split across bits[11:8] and bits[3:0].
    let cpu = run(&[0xe1c4_05b0], |cpu| {
        cpu.r[0] = 0x1122_3344;
        cpu.r[4] = STACK;
    });
    assert_eq!(cpu.mem.read_u16(STACK + 0x50).unwrap(), 0x3344);

    // strh ip, [r7], -r6  — post-indexed with a register offset: the access is at the old base
    // and the base comes back decremented.
    let cpu = run(&[0xe007_c0b6], |cpu| {
        cpu.r[7] = STACK + 0x40;
        cpu.r[6] = 0x10;
        cpu.r[12] = 0x55aa_00bb;
    });
    assert_eq!(cpu.mem.read_u16(STACK + 0x40).unwrap(), 0x00bb);
    assert_eq!(cpu.r[7], STACK + 0x30);
}

#[test]
fn a_byte_transfer_can_take_its_offset_from_a_shifted_register() {
    let cpu = run(&[0xb7dc_c7c1], |cpu| {
        cpu.cpsr = FLAG_N; // N set, V clear: the LT condition this word carries is true.
        cpu.r[1] = 0x0000_ffff;
        cpu.r[12] = STACK;
        cpu.mem.write_u8(STACK + 1, 0x5a).unwrap();
        cpu.mem.write_u8(STACK + 2, 0xa5).unwrap();
    });
    // 0xffff >>a 15 = 1, so the byte came from ip+1 and not ip+2, and `LDRB` zero-extends.
    assert_eq!(cpu.r[12], 0x5a);
    // The flags are left alone: N is still set as the fixture put it, and C is still clear even
    // though bit 14 — what ASR #15 shifts out last — is set. A pack's own inflate is what decided
    // it; see the comment at the decode.
    assert_ne!(
        cpu.cpsr & FLAG_N,
        0,
        "the condition's own N survives the address shift"
    );
    assert_eq!(
        cpu.cpsr & FLAG_C,
        0,
        "and the shift does not leave a carry behind"
    );
}

#[test]
fn the_real_prologue_builds_its_frame() {
    let cpu = run(PROLOGUE, |cpu| {
        cpu.r[0] = crate::layout::OBJECT_BASE;
        cpu.r[1] = 0x2000_1000;
        cpu.r[2] = 0x2000_2000;
        cpu.r[14] = 0xdead_0001;
    });
    assert_eq!(cpu.pc, CODE + 9 * 4);
    // Pre-index with writeback: one word for LR, then the 0xc-byte frame.
    assert_eq!(cpu.r[13], STACK - 4 - 0xc);
    assert_eq!(cpu.mem.read_u32(STACK - 4).unwrap(), 0xdead_0001);
    assert_eq!(cpu.mem.read_u32(STACK - 0x10).unwrap(), 0);
    assert_eq!(cpu.mem.read_u32(STACK - 0xc).unwrap(), 0);
    // The three moves shift the arguments up one register, and the last one is an immediate.
    assert_eq!(cpu.r[1], crate::layout::OBJECT_BASE);
    assert_eq!(cpu.r[2], 0x2000_1000);
    assert_eq!(cpu.r[3], 0x2000_2000);
    assert_eq!(cpu.r[0], 0x14);
}

#[test]
fn a_push_and_pop_round_trip_restores_the_frame_exactly() {
    // The module's own pair, confirmed against the encoding: eight registers each way, so
    // `pop` has to leave SP where `push` found it. Ending 4 bytes low is what made the outer
    // `pop {pc}` read the word *below* the saved LR and fetch from 0.
    let cpu = run(
        &[0xe92d_43f8, 0xe3a0_3099, 0xe3a0_7077, 0xe8bd_83f8],
        |cpu| {
            cpu.r[3] = 0x11;
            cpu.r[7] = 0x33;
            cpu.r[9] = 0x44;
            cpu.r[14] = CODE + 0x800;
        },
    );
    assert_eq!(cpu.r[13], STACK, "the frame is restored whole");
    assert_eq!(cpu.r[3], 0x11, "`mov r3, #0x99` must not survive the pop");
    assert_eq!(cpu.r[7], 0x33);
    assert_eq!(cpu.r[9], 0x44);
    assert_eq!(cpu.pc, CODE + 0x800, "pc came back out of the saved LR");
}

#[test]
fn a_compare_sets_flags_and_no_destination() {
    // r0 starts at 0xe7, `cmp r5, #4` must not touch it, and the `add r0, r0, #1` afterwards
    // proves the compare really did leave the destination alone. Storing the difference into
    // r0 is the classic interpreter bug, and the module hit it at once.
    let cpu = run(&[0xe3a0_00e7, 0xe355_0004, 0xe280_0001], |cpu| {
        cpu.r[5] = 0x10;
    });
    assert_eq!(cpu.r[0], 0xe7 + 1, "r0 = 0xe7, then add 1 with no carry");
    assert!(cpu.carry(), "0x10 - 4 produces no borrow");
    assert!(!cpu.flag(FLAG_Z));

    let cpu = run(&[0xe3a0_00e7, 0xe375_0004], |cpu| {
        cpu.r[5] = 0xffff_fffc;
    });
    assert_eq!(cpu.r[0], 0xe7, "`cmn r5, #4` must leave r0 alone too");
    assert!(cpu.flag(FLAG_Z), "-4 + 4 = 0");
}

#[test]
fn arithmetic_opcodes_are_the_architecture_s_and_not_a_rotated_copy() {
    // ADD must not pick up the carry, which is what the table used to do; ADC and SBC must.
    // R0 = 2, R1 = 1, so SBC is 2 - 1 - !carry.
    let cases: &[(&str, u32, bool, u32)] = &[
        ("add", 0xe080_2001, true, 3),
        ("add", 0xe080_2001, false, 3),
        ("adc", 0xe0a0_2001, true, 4),
        ("adc", 0xe0a0_2001, false, 3),
        ("sbc", 0xe0c0_2001, true, 1),
        ("sbc", 0xe0c0_2001, false, 0),
        ("rsb", 0xe060_2001, true, 0xffff_ffff),
    ];
    for (name, insn, carry, want) in cases {
        let cpu = run(&[*insn], |cpu| {
            cpu.r[0] = 2;
            cpu.r[1] = 1;
            cpu.set_flag(FLAG_C, *carry);
        });
        assert_eq!(cpu.r[2], *want, "{name} {insn:#010x} carry={carry}");
    }
}

#[test]
fn an_immediate_with_bit4_set_stays_data_processing() {
    // `mov r0, #0xf0` is e3a000f0: bit4 belongs to imm8 here, and is the multiply/BX hint
    // only in group 0b000.
    let cpu = run(&[0xe3a0_00f0], |_| {});
    assert_eq!(cpu.r[0], 0xf0);
}

/// In this group only `W` moves the base, and a base that is also a destination takes the
/// loaded value — `P` says nothing about it, whatever Thumb does. `0xe8960070` is the word a
/// guest module runs at `0x10003dce0` to pull its own stacked arguments, and reading it as
/// "always move the base" handed the module a stack address where it wanted the small index the
/// same slot held.
#[test]
fn a_block_transfer_moves_its_base_only_when_the_instruction_says_so() {
    // ldm r6, {r4, r5, r6}  — no writeback, and Rn is in the list.
    let cpu = run(&[0xe896_0070], |cpu| {
        for (i, v) in [0x140u32, 0xd9, 2].iter().enumerate() {
            cpu.mem.write_u32(STACK + i as u32 * 4, *v).unwrap();
        }
        cpu.r[6] = STACK;
    });
    assert_eq!(cpu.r[4], 0x140);
    assert_eq!(cpu.r[5], 0xd9);
    assert_eq!(cpu.r[6], 2, "the loaded value, not the address one past it");

    // ldmia sp!, {r4, r5, r6, lr}  — the same list with writeback does move the base.
    let cpu = run(&[0xe8bd_4070], |cpu| {
        for (i, v) in [1u32, 2, 3, 4].iter().enumerate() {
            cpu.mem.write_u32(STACK + i as u32 * 4, *v).unwrap();
        }
    });
    assert_eq!(cpu.r[4], 1);
    assert_eq!(cpu.r[13], STACK + 16, "four registers, written back");
}

/// The **store** direction of this group has never been asserted through memory: the tests above read
/// registers, and a walk that wrote the list in the other order — or started at the wrong end of the
/// frame — would still leave every register and `sp` correct and pass. Full-descending means the lowest
/// listed register lands at the lowest of the `count` words the frame occupies.
#[test]
fn a_store_block_transfer_lands_each_register_on_its_own_address() {
    // stmdb sp!, {r4, r5}
    let cpu = run(&[0xe92d_0030], |cpu| {
        cpu.r[4] = 0xaaaa_0004;
        cpu.r[5] = 0xaaaa_0005;
    });
    assert_eq!(cpu.r[13], STACK - 8, "two words below where sp started");
    assert_eq!(
        cpu.mem.read_u32(STACK - 8).unwrap(),
        0xaaaa_0004,
        "r4 goes to the low end, not the high"
    );
    assert_eq!(
        cpu.mem.read_u32(STACK - 4).unwrap(),
        0xaaaa_0005,
        "and r5 one word above it"
    );

    // stmdb sp!, {r4, pc} — a stored R15 is the address of the *next* instruction plus four, which is
    // the instruction address plus eight; this is the arm no test reached before.
    let cpu = run(&[0xe92d_8010], |cpu| cpu.r[4] = 0x5a5a);
    assert_eq!(cpu.mem.read_u32(STACK - 8).unwrap(), 0x5a5a);
    assert_eq!(
        cpu.mem.read_u32(STACK - 4).unwrap(),
        CODE + 8,
        "the pc the architecture says is stored, not the pc the executor is holding"
    );
}

#[test]
fn every_offset_form_of_a_transfer_uses_the_address_it_encodes() {
    let cpu = run(
        &[
            0xe781_3002, // str r3, [r1, r2]
            0xe7c1_3004, // strb r3, [r1, r4]
            0xe791_2102, // ldr r2, [r1, r2, lsl #2]
            0xe591_0008, // ldr r0, [r1, #8]
            0xe4d1_3001, // ldrb r3, [r1], #1
        ],
        |cpu| {
            cpu.r[1] = STACK - 0x800;
            cpu.r[2] = 0x40;
            cpu.r[3] = 0xaabb_ccdd;
            cpu.r[4] = 4;
            cpu.mem.write_u32(STACK - 0x700, 0xbeef).unwrap();
            cpu.mem.write_u32(STACK - 0x800 + 8, 0xcafe).unwrap();
            cpu.mem.write_u8(STACK - 0x800, 0x5a).unwrap();
        },
    );
    assert_eq!(cpu.mem.read_u32(STACK - 0x800 + 0x40).unwrap(), 0xaabb_ccdd);
    assert_eq!(cpu.mem.read_u8(STACK - 0x800 + 4).unwrap(), 0xdd);
    assert_eq!(
        cpu.r[2], 0xbeef,
        "the shift belongs to the offset, not the result"
    );
    assert_eq!(cpu.r[0], 0xcafe);
    // Post-index: the load takes the old base, and only then does the base step.
    assert_eq!(cpu.r[3], 0x5a);
    assert_eq!(cpu.r[1], STACK - 0x800 + 1);
}

#[test]
fn branches_sign_extend_and_bl_links() {
    let cpu = run(&[0xeaff_fff0], |_| {}); // b #-64, as a reference decoder reads it
    assert_eq!(cpu.pc, CODE + 8 - 64);
    assert_eq!(cpu.r[14], 0, "a plain B must not touch LR");

    let cpu = run(&[0xeb00_00d4], |_| {}); // bl #+0x350, the module's own call
    assert_eq!(cpu.pc, CODE + 8 + 0x350);
    assert_eq!(cpu.r[14], CODE + 4);

    let cpu = run(&[0xe12f_ff1e], |cpu| cpu.r[14] = CODE + 0x100); // bx lr
    assert_eq!(cpu.pc, CODE + 0x100);
    assert!(!cpu.thumb());
}

#[test]
fn a_halfword_store_writes_two_bytes_and_nothing_more() {
    let field = CODE + 0x800;
    let cpu = run(
        &[
            0xe1c4_05b0, // strh r0, [r4, #0x50]
            0xe1d4_15b0, // ldrh r1, [r4, #0x50]
            0xe1d4_55f0, // ldrsh r5, [r4, #0x50]
            0xe1d4_65d0, // ldrsb r6, [r4, #0x50]
        ],
        |cpu| {
            cpu.r[0] = 0x1234_abcd;
            cpu.r[4] = field;
            // A pattern either side of the field, so an over-wide store cannot hide.
            cpu.mem.write_u32(field + 0x48, 0x1122_3344).unwrap();
            cpu.mem.write_u32(field + 0x54, 0x5566_7788).unwrap();
        },
    );
    // The offset is split across the instruction, so this is #0x50 and not #0x5.
    assert_eq!(cpu.mem.read_u32(field + 0x48).unwrap(), 0x1122_3344);
    assert_eq!(cpu.mem.read_u32(field + 0x54).unwrap(), 0x5566_7788);
    assert_eq!(cpu.mem.read_u16(field + 0x50).unwrap(), 0xabcd);
    assert_eq!(cpu.r[1], 0xabcd, "ldrh keeps the halfword unsigned");
    assert_eq!(cpu.r[5], 0xffff_abcd, "ldrsh sign-extends it");
    assert_eq!(cpu.r[6], 0xffff_ffcd, "ldrsb sign-extends the low byte");
}

#[test]
fn a_halfword_transfer_with_writeback_updates_the_base() {
    let field = CODE + 0x800;
    let cpu = run(
        &[
            0xe1c4_05b0, // strh r0, [r4, #0x50]
            0xe1f4_15b0, // ldrh r1, [r4, #0x50]! — the bang is the same W bit as the word group
        ],
        |cpu| {
            cpu.r[0] = 0x0000_1234;
            cpu.r[4] = field;
        },
    );
    assert_eq!(
        cpu.r[4],
        field + 0x50,
        "the base moved by the offset it used"
    );
    assert_eq!(cpu.r[1], 0x1234);
}

#[test]
fn a_shift_by_register_is_data_processing_not_a_special_form() {
    // `lsl r1, r0, r3` — what a guest module died on at image offset 0xe4. `bit4` marks the
    // multiply and halfword families as well, so the special path has to give the word back
    // rather than refuse what it does not own.
    let cpu = run(
        &[
            0xe1a0_1310, // lsl r1, r0, r3 — shift amount from a register
            0xe1a0_1480, // lsl r1, r0, #9 — the same shift by an immediate
        ],
        |cpu| {
            cpu.r[0] = 1;
            cpu.r[3] = 9;
        },
    );
    assert_eq!(
        cpu.r[1],
        1 << 9,
        "the register-specified shift moved it nine"
    );
    assert_eq!(cpu.r[1], 1 << 9, "and the immediate-specified one agrees");
}

#[test]
fn condition_fifteen_is_an_undefined_instruction_not_a_nop() {
    // `0xffffffff` is what the padding of a resource blob is made of, so this is the difference
    // between a fault naming where control flow left the code and a stray abort later.
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    for i in 0..4 {
        cpu.mem.write_u32(CODE + i * 4, 0xffff_ffff).unwrap();
    }
    cpu.pc = CODE;
    match cpu.run(16) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Illegal { word },
            pc,
            ..
        }) => {
            assert_eq!(word, 0xffff_ffff);
            assert_eq!(
                pc, CODE,
                "it faults on the first word, not after sliding through them"
            );
        }
        other => panic!("expected an undefined-instruction fault, got {other:?}"),
    }
}

#[test]
fn a_doubleword_transfer_moves_a_register_pair_and_bit5_says_which_way() {
    let cpu = run(&[0xe1cd_82d0], |cpu| {
        cpu.mem.write_u32(STACK + 0x20, 0x1111_2222).unwrap();
        cpu.mem.write_u32(STACK + 0x24, 0x3333_4444).unwrap();
    });
    assert_eq!(cpu.r[8], 0x1111_2222, "Rt takes the addressed word");
    assert_eq!(cpu.r[9], 0x3333_4444, "and Rt+1 the one after it");

    // The same word with bit5 set is `strd r8, sb, [sp, #0x20]`. It is not the same nibble: the
    // store arrives as `0xf`, which bit20 clear has to pull out of `LDRSH`'s encoding.
    let cpu = run(&[0xe1cd_82f0], |cpu| {
        cpu.r[8] = 0xaaaa_bbbb;
        cpu.r[9] = 0xcccc_dddd;
        cpu.mem.write_u32(STACK + 0x20, 0).unwrap();
        cpu.mem.write_u32(STACK + 0x24, 0).unwrap();
    });
    assert_eq!(cpu.mem.read_u32(STACK + 0x20).unwrap(), 0xaaaa_bbbb);
    assert_eq!(cpu.mem.read_u32(STACK + 0x24).unwrap(), 0xcccc_dddd);
}

/// `P = 0` makes the pair transfer post-indexed: the memory operand is the base as it stands and
/// the base moves afterwards, which is the same rule the word group follows.
#[test]
fn a_post_indexed_doubleword_load_reads_the_base_and_then_advances_it() {
    let cpu = run(&[0xe0cd_82d0], |cpu| {
        cpu.mem.write_u32(STACK, 0x5555_6666).unwrap();
        cpu.mem.write_u32(STACK + 4, 0x7777_8888).unwrap();
    });
    assert_eq!(cpu.r[8], 0x5555_6666, "the addressed word is the base");
    assert_eq!(cpu.r[13], STACK + 0x20, "and the base moved by the offset");
}

/// The register-offset form of the pair transfer (bit22 clear), where the same bit that means
/// "byte" in the word group means "which offset form" here.
#[test]
fn a_doubleword_transfer_can_take_its_offset_from_a_register() {
    let cpu = run(&[0xe18d_82d0], |cpu| {
        // `ldrd r8, sb, [sp, r0]`
        cpu.r[0] = 0x30;
        cpu.mem.write_u32(STACK + 0x30, 0xdead_beef).unwrap();
        cpu.mem.write_u32(STACK + 0x34, 0xcafe_f00d).unwrap();
    });
    assert_eq!(cpu.r[8], 0xdead_beef);
    assert_eq!(cpu.r[9], 0xcafe_f00d);
}

/// An odd first register is UNPREDICTABLE in a pair transfer. No reference compiler emits one, so
/// the answer is to say so rather than to invent where the halves went.
#[test]
fn a_doubleword_pair_on_an_odd_register_is_refused_by_name() {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    // `ldrd r3, r4, [sp, #0x20]` — the same shape as the reference word, with an odd Rt.
    cpu.mem.write_u32(CODE, 0xe1cd_32d0).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Unimplemented { group, word },
            ..
        }) => {
            assert_eq!(group, "unpredictable doubleword register pair");
            assert_eq!(word, 0xe1cd_32d0);
        }
        other => panic!("expected a named unimplemented, got {other:?}"),
    }
}

#[test]
fn a_multiply_family_word_naming_r15_is_refused_by_name_not_by_indexing_past_the_file() {
    // Any word with `1001` in bits[7:4] goes through the multiply gate, and the gate reads the
    // two operand fields before it decides which form the word is. Both are four bits wide, so
    // `Rm = 15` is one of the values that arrives — and the register file is fifteen long,
    // because R15 lives beside it. The word below is not a real instruction at all; a reference
    // module reached it by branching through a pointer it had never filled, and the run ended in
    // a Rust panic that named no address and no word, which is the one failure mode a guest
    // fault is not allowed to have.
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xe1a0_f09f).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Unimplemented { group, word },
            pc,
            ..
        }) => {
            assert_eq!(group, "unclassified multiply");
            assert_eq!(word, 0xe1a0_f09f);
            assert_eq!(pc, CODE);
        }
        other => panic!("expected a named unimplemented, got {other:?}"),
    }
}

#[test]
fn a_multiply_writes_its_destination_not_the_field_data_processing_calls_rd() {
    // `0xe002059a` is a real module's own word at file offset 0x1cae8, and reading it the other way
    // round stops that game: `Rd` is bits[19:16], while bits[15:12] — which `MUL`
    // reserves as zero, and which this word duly holds — is where `MLA` keeps its accumulator.
    let cpu = run(&[0xe002_059a], |cpu| {
        cpu.r[0] = 0x2002_60b0;
        cpu.r[2] = 0x2002_6010;
        cpu.r[5] = 7;
        cpu.r[10] = 6;
    });
    assert_eq!(cpu.r[2], 42, "mul r2, r5, r10");
    assert_eq!(cpu.r[0], 0x2002_60b0, "and nothing else was touched");

    // `0xe021_c192`, likewise straight from a module: `mla r1, r2, r1, r12`.
    let cpu = run(&[0xe021_c192], |cpu| {
        cpu.r[1] = 3;
        cpu.r[2] = 100;
        cpu.r[12] = 5;
    });
    assert_eq!(cpu.r[1], 305, "the product plus the accumulator");
}

#[test]
fn a_long_multiply_fills_both_halves_and_signs_only_when_told_to() {
    // Every value asserted below was taken from the architecture's documented encodings, not from guesswork. An
    // earlier version of this test labelled `0xe0810392` "smull" and
    // `0xe0410392` "umull" — the two long forms were swapped — and asserted the pair the *signed*
    // reading gives, so the only tests that could have caught the signedness bug pinned it instead.
    let cpu = run(&[0xe081_0392], |cpu| {
        // `umull r0, r1, r2, r3`: 0xffffffff * 0xffffffff = 0xfffffffe_00000001.
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 0xffff_ffff;
    });
    assert_eq!(
        (cpu.r[0], cpu.r[1]),
        (1, 0xffff_fffe),
        "unsigned: the high half is the product, not a sign extension"
    );
    let cpu = run(&[0xe081_0392], |cpu| {
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 2;
    });
    assert_eq!(
        (cpu.r[0], cpu.r[1]),
        (0xffff_fffe, 1),
        "unsigned 2^33-2 pairs as lo/hi, not a sign-extended negative"
    );

    let cpu = run(&[0xe0c1_0392], |cpu| {
        // `smull r0, r1, r2, r3` with the same operands: -1 * -1.
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 0xffff_ffff;
    });
    assert_eq!((cpu.r[0], cpu.r[1]), (1, 0), "signed: -1 * -1 = 1");
    let cpu = run(&[0xe0c1_0392], |cpu| {
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 2;
    });
    assert_eq!(
        (cpu.r[0], cpu.r[1]),
        (0xffff_fffe, 0xffff_ffff),
        "signed: -2, which is the pair the unsigned form above must not report"
    );

    let cpu = run(&[0xe0a1_0392], |cpu| {
        // `umlal r0, r1, r2, r3`: 0xfffffffe_00000001 + 0x00000002_00000001 carries out of 64 bits.
        cpu.r[0] = 1;
        cpu.r[1] = 2;
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 0xffff_ffff;
    });
    assert_eq!((cpu.r[0], cpu.r[1]), (2, 0), "umlal");
    let cpu = run(&[0xe0e1_0392], |cpu| {
        // `smlal`: 1 + the same accumulator.
        cpu.r[0] = 1;
        cpu.r[1] = 2;
        cpu.r[2] = 0xffff_ffff;
        cpu.r[3] = 0xffff_ffff;
    });
    assert_eq!((cpu.r[0], cpu.r[1]), (2, 2), "smlal");

    // `umaal` is not in this family: bit23 clear, bit22 set is ARMv6E's `umaal`, which folds *both*
    // destination halves into the low result (measured: `r0=0x10,r1=0x20` and the reverse both give
    // `r0=0x30,r1=2`). Running it as a plain long multiply drops that sum silently, so it stops here.
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xe041_0392).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Unimplemented { group, word },
            ..
        }) => {
            assert_eq!(group, "umaal");
            assert_eq!(word, 0xe041_0392);
        }
        other => panic!("expected umaal to be refused by name, got {other:?}"),
    }
}

#[test]
fn a_long_multiply_takes_its_signedness_from_bit22_not_from_the_family_marker() {
    let cpu = run(&[0xe08c_0e92], |cpu| {
        cpu.r[2] = 0x8000_0000;
        cpu.r[14] = 2;
    });
    assert_eq!(
        (cpu.r[0], cpu.r[12]),
        (0, 1),
        "umull: 2^32 lands in the high half"
    );
    let cpu = run(&[0xe0cc_0e92], |cpu| {
        cpu.r[2] = 0x8000_0000;
        cpu.r[14] = 2;
    });
    assert_eq!(
        (cpu.r[0], cpu.r[12]),
        (0, 0xffff_ffff),
        "smull: the same operands are -2^31 * 2, so the high half is all ones"
    );
}

#[test]
fn the_rest_of_the_multiply_group_is_refused_rather_than_run_as_data_processing() {
    // A `1001` in bits[7:4] cannot be confused with a data-processing shift-by-register,
    // because that form is required to leave bit7 clear — so anything the group does not name
    // has to stop, since running it as `AND` or `EOR` corrupts a register and carries on.
    // `0xe120_0090` is such a word: no guest module in the sample set executes one, so this core
    // has no evidence to identify it with, and what the test pins is that it is refused.
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xe120_0090).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Fault(CpuFault {
            kind: FaultKind::Unimplemented { group, word },
            ..
        }) => {
            assert_eq!(group, "unclassified multiply");
            assert_eq!(word, 0xe120_0090);
        }
        other => panic!("expected a named unimplemented, got {other:?}"),
    }
}

#[test]
fn a_long_multiply_takes_its_signedness_from_bit22_alongside_the_family_marker() {
    let cpu = run(&[0xe0c1_0094], |cpu| {
        cpu.r[4] = 0xffff_ffff; // -1
        cpu.r[0] = 2;
    });
    assert_eq!(cpu.r[0], 0xffff_fffe, "RdLo of -2");
    assert_eq!(cpu.r[1], 0xffff_ffff, "RdHi of -2, not an unsigned product");
}

#[test]
fn the_sixty_four_bit_shift_island_rescales_a_fixed_point_product_the_way_the_architecture_does() {
    // Below thirty-two: `rsb`, then the three register-amount shifts that split the word pair.
    const UNDER: &[u32] = &[
        0xe262_3020, // rsb r3, r2, #0x20
        0xe1a0_0230, // lsr r0, r0, r2
        0xe180_0311, // orr r0, r0, r1, lsl r3
        0xe1a0_1251, // asr r1, r1, r2
        0xe12f_ff1e, // bx lr
    ];
    // Thirty-two and above: the sign fill, then the high word flattened to the sign.
    const OVER: &[u32] = &[
        0xe1a0_0351, // asr r0, r1, r3
        0xe1a0_1fc1, // asr r1, r1, #31
        0xe12f_ff1e, // bx lr
    ];
    // What the architecture says a 64-bit arithmetic shift by `n` answers, for every word pair
    // this island is reached with — including the negative products a signed 16.16 multiply gives.
    let oracle = |hi: u32, lo: u32, n: u32| -> (u32, u32) {
        let sign = if (hi as i32) < 0 { u32::MAX } else { 0 };
        if n >= 64 {
            return (sign, sign);
        }
        let v = ((hi as i64) << 32) | u64::from(lo) as i64;
        ((v >> n) as u32, ((v >> n) >> 32) as u32)
    };
    let pairs = [
        (0x0000_0000u32, 0x0001_0000u32), // 1.0 in 16.16
        (0xffff_ffffu32, 0xffff_0000u32), // -1.0
        (0xffff_fff6u32, 0xe800_0000u32), // the product a small negative times a large positive
        (0x0000_0002u32, 0x0000_0000u32), // just over 2^32
        (0x8000_0000u32, 0x0000_0000u32), // i64::MIN's high word
        (0xc9cd_34bbu32, 0x1234_5678u32), // a value this thread actually observed
    ];
    for n in [0u32, 1, 15, 16, 17, 31] {
        for (hi, lo) in pairs {
            let cpu = run(UNDER, |cpu| {
                cpu.r[0] = lo;
                cpu.r[1] = hi;
                cpu.r[2] = n;
                cpu.r[14] = CODE + 0x100;
            });
            assert_eq!(
                (cpu.r[0], cpu.r[1]),
                oracle(hi, lo, n),
                "path under 32, shift by {n}, of {hi:#010x}_{lo:#010x}"
            );
        }
    }
    for n in [32u32, 33, 47, 48, 63, 64, 65] {
        for (hi, lo) in pairs {
            let cpu = run(OVER, |cpu| {
                cpu.r[0] = lo;
                cpu.r[1] = hi;
                cpu.r[2] = n;
                cpu.r[3] = n - 32;
                cpu.r[14] = CODE + 0x100;
            });
            assert_eq!(
                (cpu.r[0], cpu.r[1]),
                oracle(hi, lo, n),
                "path from 32 up, shift by {n}, of {hi:#010x}_{lo:#010x}"
            );
        }
    }
    // The selection itself: `bpl` walks when `r2 - 0x20` is not negative, and takes the branch at
    // exactly thirty-two, which is where the fast path would otherwise hand a rotate back.
    for (n, taken) in [
        (0u32, false),
        (31, false),
        (32, true),
        (33, true),
        (48, true),
    ] {
        let cpu = run(
            &[0xe252_3020, 0x5a00_0004], // subs r3, r2, #0x20 ; bpl +0x10
            |cpu| {
                cpu.r[2] = n;
            },
        );
        assert_eq!(
            cpu.pc == CODE + 12 + 0x10,
            taken,
            "shift by {n}: bpl {}",
            if taken { "must" } else { "must not" }
        );
    }
}

/// `blx #+0x358`: the word at module+0x24 of two reference Resident Evil builds, where it is the
/// call out of a compiler-generated ARM veneer into the pack's Thumb body. The unconditional
/// space is not all undefined — ARMv5 put BLX-immediate there for exactly this interworking.
#[test]
fn blx_immediate_links_and_switches_to_thumb() {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xfa00_00d6).unwrap();
    cpu.pc = CODE;
    cpu.set_thumb(false);
    match cpu.run(1) {
        Stop::Budget { .. } => {}
        other => panic!("unexpected stop: {other:?}"),
    }
    assert_eq!(cpu.pc, CODE + 8 + 0x358, "offset is relative to <insn>+8");
    assert!(cpu.thumb(), "BLX-immediate always enters Thumb");
    assert_eq!(cpu.r[14], CODE + 4, "LR is the instruction after the BLX");
}

/// Bit 24 is the halfword bit of the target, not part of the signed 24-bit word offset.
#[test]
fn blx_immediate_halfword_bit_reaches_an_odd_halfword() {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xfb00_00d6).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Budget { .. } => {}
        other => panic!("unexpected stop: {other:?}"),
    }
    assert_eq!(cpu.pc, CODE + 8 + 0x358 + 2);
    assert!(cpu.thumb());
}

/// `blx #-0xb10c`, the word a reference QQ build uses to call backwards into its own Thumb half.
#[test]
fn blx_immediate_sign_extends_a_backward_offset() {
    let mut cpu = Cpu::new(Mem::new());
    cpu.mem.map(CODE, 0x1000);
    cpu.mem.write_u32(CODE, 0xfaff_d3bd).unwrap();
    cpu.pc = CODE;
    match cpu.run(1) {
        Stop::Budget { .. } => {}
        other => panic!("unexpected stop: {other:?}"),
    }
    assert_eq!(cpu.pc, CODE.wrapping_add(8).wrapping_sub(0xb10c));
    assert!(cpu.thumb());
}

/// `mov r0, r1, lsl r15` = 0xe1a00f11. The shift-amount register field is four bits wide but the
/// register file holds fifteen, so naming it directly indexes past the end. R15 is UNPREDICTABLE
/// here, which means answering with the biased PC the way every other operand fetch does — not
/// panicking, and not refusing an encoding a guest pack's data can contain.
#[test]
fn register_shift_amount_naming_r15_does_not_index_past_the_register_file() {
    let cpu = run(&[0xe1a0_0f11, 0xe1a0_0000], |cpu| {
        cpu.r[1] = 1;
    });
    let amount = (CODE + 8) & 0xff;
    assert_eq!(
        cpu.r[0],
        1u32.wrapping_shl(amount),
        "R15 reads as the biased PC"
    );
}

/// ARMv6's four extends, and the reason they are here at all: read as a transfer they decode to
/// `ldr Rd, [pc, ...]` (bits[19:16] come out as r15), which *ran* and changed nothing — so a
/// guest pack's `sxth` (`a sample module`'s module-init path) was a no-op that looked like a computed
/// value, and the two engines disagreed the moment the sibling decoder answered it honestly.
///
/// The four words are the real encodings the sample set executes, not hand-rolled neighbours.
#[test]
fn armv6_extends_widen_and_do_not_decode_as_a_transfer() {
    // (word, input in r0, expected in r1, what)
    let cases: &[(u32, u32, u32, &str)] = &[
        (0xe6bf_5070, 0x0000_8001, 0xffff_8001, "sxth r5, r0"),
        (0xe6af_5070, 0x0000_0080, 0xffff_ff80, "sxtb r5, r0"),
        (0xe6ff_5070, 0x0000_8001, 0x0000_8001, "uxth r5, r0"),
        (0xe6ef_5070, 0x0000_00ff, 0x0000_00ff, "uxtb r5, r0"),
        // The word the sample set actually executes, condition included: `a sample module` runs this with CC.
        (
            0x36bf_5070,
            0x0000_8001,
            0xffff_8001,
            "sxthcc r5, r0 with carry set",
        ),
    ];

    for &(insn, input, expected, what) in cases {
        let cpu = run(&[insn], |cpu| {
            cpu.r[0] = input;
            // The last case is the sample set word `sxthcc`, and `cc` means *carry clear* — the opposite
            // of what the mnemonic reads like. With the carry left clear it runs; with it set it is
            // skipped, which is what cost an hour of chasing a decoder that was already right.
            cpu.cpsr &= !FLAG_C;
        });
        assert_eq!(cpu.r[5], expected, "{what}: r5 = {:#010x}", cpu.r[5]);
        assert_eq!(
            cpu.pc,
            CODE + 4,
            "an extend must not branch; pc moved to {:#010x}",
            cpu.pc
        );
    }
}

/// `LDR pc, [...]` is `LoadWritePC`, and from ARMv5 on that behaves as `BX`.
///
/// The instruction this is really about is `pop {pc}` (`e49df004`), because that is how ARM code
/// returns to a Thumb caller. Reading it as an ordinary write to R15 cleared no state bit but
/// *kept* the old one, so the return address's low bit was dropped and the caller's halfwords were
/// fetched as ARM words. In a sample pack that turned two Thumb `bl` halfwords (`f0 f7 a5 f9`) into
/// `illegal instruction 0xf9a5f7f0 at 0x1000fce0 (ARM)` — a report that reads like a Thumb-2
/// encoding gap and is nothing of the sort.
#[test]
fn arm_ldr_into_pc_interworks_the_way_bx_does() {
    // `pop {pc}`, exactly as the pack's own return path spells it.
    const POP_PC: u32 = 0xe49d_f004;

    let odd = run(&[POP_PC], |cpu| {
        cpu.mem.write_u32(STACK, CODE + 5).unwrap();
    });
    assert_eq!(
        odd.pc,
        CODE + 4,
        "an odd popped address must land on the halfword boundary"
    );
    assert!(odd.thumb(), "bit 0 of the popped word selects Thumb");

    let even = run(&[POP_PC], |cpu| {
        cpu.mem.write_u32(STACK, CODE + 8).unwrap();
    });
    assert_eq!(even.pc, CODE + 8);
    assert!(!even.thumb(), "bit 0 clear stays in ARM");
}

/// `clz` names its destination in bits 15..12, and the decoder used to mask that field away by
/// comparing `insn & 0x0fff_fff0` against `0x016f_1f10` — which is the encoding of `clz r1, rX`
/// only. Every other destination fell through to the multiply group and left its register alone,
/// so `clz r6, r2` read as "leading zeros = 0". A 64-bit division routine normalises its divisor
/// with exactly that instruction and then shifts by the answer, which turns one missed decode
/// into a wrong quotient rather than into an error.
///
/// `a guest module`'s libgcc `__aeabi_uldivmod` is that routine: with `clz` answering 0 it divided
/// 76800 by 10, got a quotient of `0x12c00_00000000` and a remainder of 76800, then used the
/// remainder to index the `"0123456789abcdef"` table 32 KiB past the end of the module.
#[test]
fn clz_writes_whatever_destination_the_encoding_names() {
    // (word, value in r0, destination register, expected, what)
    let cases: &[(u32, u32, usize, u32, &str)] = &[
        (0xe16f_1f10, 0x0000_000a, 1, 28, "clz r1, r0"),
        (0xe16f_6f12, 0x0000_000a, 6, 28, "clz r6, r2"),
        (0xe16f_2f10, 0x8000_0000, 2, 0, "clz r2, r0 with the top bit set"),
        (0xe16f_3f10, 0x0000_0000, 3, 32, "clz r3, r0 over zero"),
        (0xe16f_7f10, 0x0000_0001, 7, 31, "clz r7, r0 over one"),
    ];

    for &(insn, input, rd, expected, what) in cases {
        let cpu = run(&[insn], |cpu| {
            cpu.r[0] = input;
            cpu.r[2] = input;
        });
        assert_eq!(
            cpu.r[rd], expected,
            "{what}: r{rd} = {:#010x}, expected {expected}",
            cpu.r[rd]
        );
        assert_eq!(
            cpu.pc,
            CODE + 4,
            "{what} must not branch; pc moved to {:#010x}",
            cpu.pc
        );
    }
}
