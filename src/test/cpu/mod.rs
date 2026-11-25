// Moved out of `src/cpu/mod.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

fn cpu() -> Cpu {
    Cpu::new(Mem::new())
}

#[test]
fn condition_codes_read_the_flags() {
    let mut c = cpu();
    c.cpsr = 0;
    c.set_flag(FLAG_Z, true);
    assert!(arm::condition_met(&c, 0)); // eq
    assert!(!arm::condition_met(&c, 1)); // ne
    c.cpsr = 0;
    assert!(arm::condition_met(&c, 14)); // always
    assert!(!arm::condition_met(&c, 15)); // the reserved field never executes
}

/// `--break ADDR,N` waits for the N-th arrival, because the call that explains a pack is often the
/// third one through a function that runs once per record in a table. The two assertions together
/// are the point: a stop that left `r0` at 0 would have been the first arrival, not the second.
#[test]
fn a_breakpoint_can_wait_for_the_arrival_that_matters() {
    const CODE: u32 = 0x1000_0000;
    let mut c = cpu();
    c.mem.map(CODE, 0x1000);
    // add r0,r0,#1 ; cmp r0,#2 ; bne back ; then a self-loop the run should never reach.
    for (i, w) in [0xe280_0001u32, 0xe350_0002, 0x1aff_fffc, 0xeaff_fffe]
        .iter()
        .enumerate()
    {
        c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
    }
    c.pc = CODE;
    c.break_at.push(Break {
        addr: CODE,
        after: 2,
        seen: 0,
        when: None,
    });
    match c.run(100) {
        Stop::Stopped {
            why: StopWhy::Break(addr),
        } => {
            assert_eq!(addr, CODE);
            assert_eq!(c.r[0], 1, "the first arrival was passed");
            assert_eq!(c.break_at[0].seen, 2, "and the tally says so");
        }
        other => panic!("expected the breakpoint, got {other:?}"),
    }
}

/// `--break ADDR,r0=2` stops at the arrival that *holds* the value, not at the N-th one. The point
/// is that this address is visited three times before `r0` reaches 2 and the value, not a count,
/// is what picked the third: the run stops with `r0` at 2 while the tally reads 3.
#[test]
fn a_breakpoint_can_wait_for_a_register_value() {
    const CODE: u32 = 0x1000_0000;
    let mut c = cpu();
    c.mem.map(CODE, 0x1000);
    // add r0,r0,#1 ; cmp r0,#3 ; bne back ; then a self-loop the run should never reach.
    for (i, w) in [0xe280_0001u32, 0xe350_0003, 0x1aff_fffc, 0xeaff_fffe]
        .iter()
        .enumerate()
    {
        c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
    }
    c.pc = CODE;
    c.break_at.push(Break {
        addr: CODE,
        after: 1,
        seen: 0,
        when: Some(RegTest { reg: 0, val: 2 }),
    });
    match c.run(100) {
        Stop::Stopped {
            why: StopWhy::Break(addr),
        } => {
            assert_eq!(addr, CODE);
            assert_eq!(c.r[0], 2, "the stop happened where the value held");
            assert_eq!(
                c.break_at[0].seen, 3,
                "which was the third arrival there, not the first"
            );
        }
        other => panic!("expected the breakpoint, got {other:?}"),
    }
}

#[test]
fn barrel_shifter_matches_the_architecture() {
    assert_eq!(
        shift_imm(0x8000_0001, Shift::Lsl, 1, false),
        (0x0000_0002, true)
    );
    assert_eq!(shift_imm(1, Shift::Lsl, 32, true), (0, true));
    assert_eq!(shift_imm(0x8000_0000, Shift::Lsr, 0, false), (0, true));
    assert_eq!(
        shift_imm(0xffff_ffff, Shift::Asr, 31, false),
        (0xffff_ffff, true)
    );
    // The two cases that separate "carry out is the last bit shifted out" from "carry out is
    // the sign": a positive word shifted by 15, and a negative word shifted by 1 whose bit 0
    // is clear. Both were indistinguishable under the all-ones value above.
    assert_eq!(shift_imm(0x0000_ffff, Shift::Asr, 15, false), (1, true));
    assert_eq!(
        shift_imm(0x8000_0000, Shift::Asr, 1, false),
        (0xc000_0000, false),
        "bit 0 leaves, not the sign"
    );
    // RRX pulls the carry in at bit 31 and pushes the source's bit 0 out.
    assert_eq!(
        shift_imm(0x0000_0003, Shift::Ror, 0, true),
        (0x8000_0001, true)
    );
    assert_eq!(shift_reg(1, Shift::Lsl, 33, false), (0, false));
    // The register form has to leave the carry the immediate form does, because the rule is
    // about the shift rather than about the encoding: the last bit shifted out, which for a
    // positive word shifted right by 15 is bit 14. Pinned separately because the two forms are
    // separate code paths, and the immediate one's fix did not reach this one.
    assert_eq!(shift_reg(0x0000_ffff, Shift::Asr, 15, false), (1, true));
    // The amounts an immediate cannot encode, measured against unicorn rather than reasoned out.
    // Past a whole word an LSR has no bit left to have left, while exactly 32 leaves bit 31; and
    // a ROR by a whole word changes no bit and still replaces the carry with bit 31 instead of
    // leaving the caller's carry-in there.
    assert_eq!(shift_reg(0x8000_0000, Shift::Lsr, 32, false), (0, true));
    assert_eq!(shift_reg(0x8000_0000, Shift::Lsr, 33, false), (0, false));
    assert_eq!(
        shift_reg(0x8000_0000, Shift::Ror, 32, false),
        (0x8000_0000, true)
    );
    // This one used to expect `true`, and so could not tell the two readings apart: the value it
    // rotates has bit 31 clear, and the carry-in it passes is 1.
    assert_eq!(
        shift_reg(0x1234_5678, Shift::Ror, 32, true),
        (0x1234_5678, false)
    );
}

#[test]
fn writing_r15_in_arm_state_ignores_the_state_bit() {
    let mut c = cpu();
    c.set_reg(15, 0x1001);
    assert_eq!(c.pc, 0x1000);
    assert!(!c.thumb());
    c.branch_to(0x1001, true);
    assert_eq!(c.pc, 0x1000);
    assert!(c.thumb());
}

#[test]
fn r15_operand_carries_the_pipeline_bias() {
    let mut c = cpu();
    c.pc = 0x1004; // points at the next instruction, so the current one is at 0x1000
    assert_eq!(c.reg(15), 0x1008);
    c.set_thumb(true);
    c.pc = 0x1002;
    assert_eq!(c.reg(15), 0x1004);
}

/// A pack that spins inside one function is the case the cancel flag exists for, and it is the case a
/// session loop cannot serve: a caller that only looks *between* visits to the host is waiting on a
/// function that never comes back to be looked at. So the ask is answered here, in the run loop, and it
/// is answered at [`CANCEL_LOOK`] granularity rather than per instruction.
#[test]
fn a_cancel_lands_inside_the_function_that_never_returns() {
    const CODE: u32 = 0x1000_0000;
    let mut c = cpu();
    c.mem.map(CODE, 0x1000);
    // `b .` — one instruction, forever. From inside the emulator this is what a latch that never
    // changes looks like, and it is also what an applet stuck in its own message loop looks like.
    c.mem.write_u32(CODE, 0xeaff_fffe).unwrap();
    c.pc = CODE;
    let flag = Arc::new(AtomicBool::new(false));
    c.arm_cancel(flag.clone());
    flag.store(true, Ordering::Relaxed);
    // A few looks' worth of budget, on purpose. An "unbounded" budget here would not make the test
    // stronger; it would make a broken cadence unkillable, since the only thing that ends the loop is
    // the check being tested. With a bound, the same mutation fails in milliseconds instead of chasing
    // the arithmetic ceiling.
    match c.run(CANCEL_LOOK * 4) {
        Stop::Cancelled { pc } => {
            assert_eq!(pc, CODE, "a cancel says where the guest had reached");
            assert_eq!(
                c.steps, CANCEL_LOOK,
                "exactly one look, not one per instruction: the flag was already up when the run began"
            );
        }
        other => panic!("expected the cancel to land, got {other:?}"),
    }
}

/// Positive control for the test above, and the reason it is not self-proving: an armed machine whose
/// flag is never set must still end on its budget. A check that always fires would pass that test too,
/// and the freeze would come back with the suite green.
#[test]
fn an_armed_cancel_flag_that_is_never_set_stops_nothing() {
    const CODE: u32 = 0x1000_0000;
    let mut c = cpu();
    c.mem.map(CODE, 0x1000);
    c.mem.write_u32(CODE, 0xeaff_fffe).unwrap();
    c.pc = CODE;
    c.arm_cancel(Arc::new(AtomicBool::new(false)));
    // Short enough to stay inside the first look, which is the other half of the claim: nobody who did
    // not ask for a cancel pays even the load.
    match c.run(CANCEL_LOOK / 4) {
        Stop::Budget { steps } => assert_eq!(steps, CANCEL_LOOK / 4),
        other => panic!("nothing asked this run to stop, got {other:?}"),
    }
}

/// `--sample` takes one entry per interval and nothing at all when it was not asked for.
///
/// The unarmed half is the one that matters for every other run in the sample set: the check sits in the
/// interpreter's inner loop, so "armed" must be the only case that writes.
#[test]
fn samples_are_taken_on_the_interval_and_only_then() {
    let mut off = Samples::default();
    for i in 1..=1000u64 {
        off.note(i, 0x1000 + i as u32, true);
    }
    assert!(
        off.ring.is_empty(),
        "a run that never asked for samples records nothing"
    );
    assert!(off.hottest().is_empty(), "and has no window to fold");

    let mut on = Samples::armed(64, 4096);
    for i in 1..=1000u64 {
        on.note(i, 0x1000 + i as u32, true);
    }
    assert_eq!(on.ring.len(), 15, "one every 64 of 1000 instructions");
    assert_eq!(
        on.ring.front().unwrap().at,
        64,
        "the first is due at the interval"
    );
    assert_eq!(
        on.ring.back().unwrap().at,
        960,
        "and the last is the highest multiple"
    );
    // The sample names the instruction that retired, in the state it retired in, because an address
    // read back as the wrong width disassembles as garbage and would be blamed on the pack.
    let s = on.ring.back().unwrap();
    assert_eq!((s.pc, s.thumb), (0x1000 + 960, true));
}

/// The ring is a window: past its capacity the oldest entries leave and the newest stay.
#[test]
fn the_sample_ring_keeps_the_tail_and_drops_the_front() {
    let mut on = Samples::armed(10, 4);
    for i in 1..=1000u64 {
        on.note(i, 0x2000 + i as u32, false);
    }
    assert_eq!(on.ring.len(), 4, "the capacity is the capacity");
    let kept: Vec<u64> = on.ring.iter().map(|x| x.at).collect();
    assert_eq!(
        kept,
        vec![970, 980, 990, 1000],
        "the last four, oldest first"
    );
    assert_eq!(
        on.ring.front().unwrap().at,
        970,
        "so the address printed as the front of the window is 970 instructions old, not 10"
    );
}

/// The folded view answers "what was it busy with", and its order is the finding.
#[test]
fn the_folded_samples_answer_what_was_busyest_first() {
    let mut on = Samples::armed(1, 64);
    // One address most of the window, another a little, a third once — a spin, a helper, a stray.
    // `note` compares against the instruction count, so the counter has to move like the machine's.
    let feed = |s: &mut Samples, pc: u32, n: u64, at: &mut u64| {
        for _ in 0..n {
            *at += 1;
            s.note(*at, pc, true);
        }
    };
    let mut at = 0u64;
    feed(&mut on, 0x4000, 30, &mut at);
    feed(&mut on, 0x3000, 7, &mut at);
    feed(&mut on, 0x5000, 1, &mut at);
    assert_eq!(
        on.ring.len(),
        38,
        "one sample per instruction, so every feed is one"
    );
    let hot = on.hottest();
    assert_eq!(hot.len(), 3, "three distinct addresses");
    assert_eq!(hot[0], ((0x4000, true), 30), "busiest first");
    assert_eq!(hot[1], ((0x3000, true), 7));
    assert_eq!(
        hot[2],
        ((0x5000, true), 1),
        "and the single sighting is last"
    );
    // The order must not come out of hash iteration: the same window folded twice answers the same,
    // or a reader comparing two runs is comparing the hasher.
    assert_eq!(hot, on.hottest(), "the fold is deterministic");
    // Ties break by address, so the smaller one leads whoever folds it.
    let mut tied = Samples::armed(1, 64);
    feed(&mut tied, 0x9000, 1, &mut at);
    feed(&mut tied, 0x8000, 1, &mut at);
    assert_eq!(
        tied.hottest(),
        vec![((0x8000, true), 1), ((0x9000, true), 1)],
        "equal counts sort by address, not by arrival"
    );
}

/// The hook is in the interpreter's own loop, so the claim worth recording is that a real run fills
/// the ring — a `note` nobody calls passes every unit test above it.
#[test]
fn a_running_machine_fills_the_sample_window() {
    const CODE: u32 = 0x1000_0000;
    let mut c = cpu();
    c.mem.map(CODE, 0x1000);
    // A branch to itself, executed forever: every sample of a spinning pack lands on the same
    // address, which is exactly the shape a hang is expected to make. (A plain NOP would not do: the
    // program counter walks off into the zeroed words after it and the samples spread out.)
    c.mem.write_u32(CODE, 0xeaff_fffe).unwrap(); // b . — the address itself is the loop
    c.pc = CODE;
    c.samples = Samples::armed(128, 64);
    match c.run(1000) {
        Stop::Budget { steps } => assert_eq!(steps, 1000),
        other => panic!("nothing asked this run to stop, got {other:?}"),
    }
    assert_eq!(
        c.samples.ring.len(),
        7,
        "one every 128 of 1000, first due at 128"
    );
    assert!(
        c.samples.ring.iter().all(|x| x.pc == CODE && !x.thumb),
        "a spin answers with the address it spun at, in the state it ran in"
    );
    assert_eq!(c.samples.hottest(), vec![((CODE, false), 7)]);
    // And a machine that was never armed keeps an empty window over the same run: the loop's check is
    // not a filter that silently fills something nobody asked for.
    let mut plain = cpu();
    plain.mem.map(CODE, 0x1000);
    plain.mem.write_u32(CODE, 0xeaff_fffe).unwrap();
    plain.pc = CODE;
    let _ = plain.run(1000);
    assert!(plain.samples.ring.is_empty(), "unarmed records nothing");
}

/// Registers, the counters and the memory behind them come back together: a restored machine is the
/// one that was saved, not one with new registers over old bytes.
#[test]
fn a_cpu_state_round_trips_including_the_memory_behind_it() {
    let mut cpu = Cpu::new(Mem::new());
    cpu.r[0] = 0x1111_2222;
    cpu.r[13] = 0x3000_0000;
    cpu.pc = 0x1000_0010;
    cpu.cpsr = 0;
    cpu.set_thumb(true);
    cpu.steps = 42;
    cpu.hle_calls = 7;
    cpu.mem.map(0x1000_0000, 4);
    cpu.mem.write_u32(0x1000_0000, 0xfeed_face).unwrap();

    let mut block = Vec::new();
    cpu.write_state(&mut block);

    let mut back = Cpu::new(Mem::new());
    back.read_state(&mut crate::mem::State::new(&block))
        .unwrap();
    assert_eq!(back.r[0], 0x1111_2222, "a register");
    assert_eq!(back.pc, 0x1000_0010, "the program counter");
    assert!(back.thumb(), "the Thumb bit travels in `cpsr`");
    assert_eq!(back.steps, 42);
    assert_eq!(back.hle_calls, 7);
    assert_eq!(
        back.mem.read_u32(0x1000_0000).unwrap(),
        0xfeed_face,
        "and the memory came with it"
    );
}

/// `--jit-arena` and `QUOIN_JIT_ARENA` are one choice read in two places, so there is one parser:
/// `auto`, `0` for no ceiling, or a count. Everything else is an error rather than a number nobody
/// meant — the flag exists to be measured with, and a typo that quietly became a ceiling of 0 blocks
/// would look like the backend being broken.
#[test]
fn the_arena_choice_reads_auto_zero_and_a_count() {
    assert_eq!(JitArena::parse("auto"), Ok(JitArena::Auto));
    assert_eq!(JitArena::parse(" AUTO "), Ok(JitArena::Auto));
    assert_eq!(JitArena::parse("0"), Ok(JitArena::Blocks(0)));
    assert_eq!(JitArena::parse("65536"), Ok(JitArena::Blocks(65536)));
    assert!(JitArena::parse("").is_err(), "an empty value names nothing");
    assert!(JitArena::parse("till it hurts").is_err());
    assert!(
        JitArena::parse("-1").is_err(),
        "a negative ceiling is not a smaller one"
    );
    // Past this the count cannot be reached before the 32 MiB code arena is full, so accepting it
    // would be promising something the backend cannot do.
    assert!(JitArena::parse("2000000").is_err());
    assert_eq!(JitArena::parse("1048576"), Ok(JitArena::Blocks(1 << 20)));
}

/// The `auto` estimate against the packs that bound this tree: what it says for the widest image, for
/// the narrowest, and for a ceiling a caller set below both.
///
/// The guarantee being pinned is "a bound, not a prediction": `351 152` bytes is 87 788 ARM entry
/// points at most, which is past the default, so the widest pack is handed the default rather than a
/// number it could outgrow. The measured pack used 2 563 blocks at its title screen — the estimate is
/// allowed to be that loose, and may not be tight.
#[test]
fn the_arena_estimate_floors_a_small_pack_and_never_exceeds_the_default() {
    const DEFAULT: usize = 65536;
    assert_eq!(
        jit_arena_blocks(351_152, DEFAULT),
        DEFAULT,
        "351 KiB of ARM code is 87788 possible entries: past the default, so the default it is"
    );
    assert!(
        jit_arena_blocks(351_152, 1 << 20) == 87_788,
        "with room above it, the bound is one block per instruction"
    );
    assert_eq!(
        jit_arena_blocks(157_404, DEFAULT),
        39_351,
        "a 154 KiB image is under the default and is sized by its own code"
    );
    assert_eq!(
        jit_arena_blocks(4_516, DEFAULT),
        ARENA_FLOOR_BLOCKS,
        "a 4.5 KiB pack is not judged on 1129 blocks"
    );
    assert_eq!(
        jit_arena_blocks(351_152, 1000),
        1000,
        "an explicit ceiling below the floor is still the ceiling"
    );
}
