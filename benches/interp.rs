//! Interpreter throughput **and** the cost of the backend seam, with no pack and no HLE:
//! `cargo bench --bench interp`.
//!
//! Two measurements, because they want opposite shapes and each is blind to the other's cost. The
//! throughput loops hand over 200 M instructions in one `Cpu::run`, which is the right shape for
//! "how fast is an instruction" and completely hides "how much does one call into the backend cost".
//! [`seam`] is the other shape: budget 1, and the two entry points alternated in blocks, because
//! that is how `Runtime` drives the machine — once per `SVC`.
//!
//! This exists because a number is a precondition for every optimisation, and the number the
//! interpreter reports needs a window and a real pack, which a self-contained benchmark forbids.
//! Here the guest bytes are written by this file, mapped into two pages, and executed for a bounded count,
//! so nothing outside the process is involved: it is a compile-and-run measurement, the same category as
//! `cargo test --lib`.
//!
//! Two loops, deliberately paired. Each does one load, one store, two adds and a taken conditional branch,
//! and each runs the *same* work — so the gap between them is what the two fetch paths are worth, and the
//! move planned for `Mem::read_u32`/`write_u32` (getting them inlined) should move the
//! ARM number and leave the Thumb one alone. Reading only one of the two would mistake a general speedup
//! for that one.
//!
//! The branch offset is computed (`-(index + 2)` for both states) rather than written as a hex word, so a
//! future edit that inserts an instruction into a body cannot silently retarget the loop into unmapped
//! memory and turn the measurement into a fault report.
//!
//! `#[ignore]`d CPU assertions live in `src/test/cpu/`; this file asserts nothing, it prints numbers.

use std::env;
use std::time::Instant;

use quoin::cpu::{Cpu, Stop};
use quoin::mem::Mem;

/// Where the synthetic code lives. The same base the CPU tests use, and deliberately page-aligned.
const CODE: u32 = 0x1000_0000;
/// The one word the loops load and store, on the second page.
const DATA: u32 = 0x1000_1000;
/// The stack the `PUSH`/`POP` shapes grow down from, on the third page — away from both the code and
/// the data word, so a lap never disturbs what it is measuring.
const SP: u32 = 0x1000_2800;

/// The `bne` that closes a loop whose branch sits at `index` instructions into it.
///
/// Both states land on the same formula because both add the prefetch bias this executor models: ARM
/// resolves to `address + 8 + 4*off` and Thumb to `address + 4 + 2*off`, and each address is
/// `index * width` — so `off = -(index + 2)` words, or halfwords, back to the first instruction.
fn back_offset(index: usize) -> i32 {
    -(index as i32 + 2)
}

/// The Thumb loop: `ldr r1,[r2]` / `adds r1,#1` / `str r1,[r2]` / `adds r0,#1` / `bne back`.
///
/// `r2` is the data base and `r0` the counter, which is what keeps `bne` taken: `adds r0,#1` sets Z only
/// when the result is zero, and starting from 1 it is not for the next four billion increments.
fn thumb_loop() -> Vec<u32> {
    let mut code = vec![
        0x6811, // ldr r1, [r2, #0]
        0x3101, // adds r1, #1
        0x6011, // str r1, [r2, #0]
        0x3001, // adds r0, #1
    ];
    code.push(0xd100 | (back_offset(code.len()) as u32 & 0xff));
    code
}

/// The ARM loop: the same five operations. `add`/`cmp` here are the immediate forms, and the branch is
/// `bne` with the 24-bit signed word offset, so the encoding is `0x1a00_0000 | (off & 0xff_ffff)`.
fn arm_loop() -> Vec<u32> {
    let mut code = vec![
        0xe592_1000, // ldr r1, [r2, #0]
        0xe281_1001, // add r1, r1, #1
        0xe582_1000, // str r1, [r2, #0]
        0xe280_0001, // add r0, r0, #1
        0xe350_0000, // cmp r0, #0
    ];
    code.push(0x1a00_0000 | (back_offset(code.len()) as u32 & 0x00ff_ffff));
    code
}

/// Build a machine running `code`, in the given state, with `r0` = 1, `r2` = the data word, `sp` =
/// [`SP`] and `lr` = `return_to`.
fn machine(code: &[u32], thumb: bool, return_to: u32) -> Cpu {
    let mut c = Cpu::new(Mem::new());
    c.mem.map(CODE, 0x3000);
    if thumb {
        for (i, w) in code.iter().enumerate() {
            c.mem.write_u16(CODE + i as u32 * 2, *w).unwrap();
        }
        c.set_thumb(true);
    } else {
        for (i, w) in code.iter().enumerate() {
            c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
        }
    }
    c.pc = CODE;
    c.r[0] = 1;
    c.r[2] = DATA;
    c.r[13] = SP;
    c.r[14] = return_to;
    c.mem.write_u32(DATA, 0).unwrap();
    c
}

/// The two shapes the data-path loops cannot see: `PUSH`/`POP` walk the **register list**, which is a
/// loop of its own in `thumb_push`/`thumb_pop` and `arm_block_transfer`. Without them a change to that
/// walk measures as nothing at all.
///
/// The Thumb shape is `push {r4,r5,lr}` / `adds r0,#1` / `ldr r1,[r2]` / `pop {r4,r5,pc}`. The `pop`
/// loads `lr` into the program counter, which is the return every interworked Thumb function actually
/// uses — so the loop closes through the branch path too, with `lr` = `CODE | 1` keeping it in state.
fn thumb_stack_loop() -> Vec<u32> {
    vec![
        0xb530, // push {r4, r5, lr}
        0x3001, // adds r0, #1
        0x6811, // ldr r1, [r2, #0]
        0xbd30, // pop {r4, r5, pc}
    ]
}

/// The ARM equivalent: `stmdb sp!, {r4,r5,lr}` / `add r0,r0,#1` / `ldr r1,[r2]` /
/// `ldmia sp!, {r4,r5,pc}`, which walks the sixteen-bit list form in `arm_block_transfer`.
/// `lr` is `CODE` here, without the Thumb bit, so the return stays in ARM state.
fn arm_stack_loop() -> Vec<u32> {
    vec![
        0xe92d_4030, // stmdb sp!, {r4, r5, lr}
        0xe280_0001, // add r0, r0, #1
        0xe592_1000, // ldr r1, [r2, #0]
        0xe8bd_8030, // ldmia sp!, {r4, r5, pc}
    ]
}

/// Run `budget` instructions and return the wall time, with the loop's own state checked so that a
/// number is never printed for a run that stopped early. A `Stop::Budget` that is not `budget` steps
/// deep, or a counter that disagrees with the instruction count, means the measurement measured nothing.
///
/// The budget is not a whole number of laps, so the expected counter includes the partial lap's own
/// increment (`r0_at`), and the program counter is only required to still be **inside** the loop.
fn timed(code: &[u32], thumb: bool, r0_at: usize, return_to: u32, budget: u64) -> u128 {
    let mut c = machine(code, thumb, return_to);
    let started = Instant::now();
    let stop = c.run(budget);
    let nanos = started.elapsed().as_nanos();
    match stop {
        Stop::Budget { steps } => assert_eq!(steps, budget, "the budget is what ran"),
        other => panic!("the loop stopped early: {other:?}"),
    }
    let laps = budget / code.len() as u64;
    let rest = budget % code.len() as u64;
    let expected = 1 + laps + u64::from(rest > r0_at as u64);
    assert_eq!(
        u64::from(c.r[0]),
        expected,
        "the counter says {} laps, not the {laps} the instruction count gives",
        c.r[0] - 1,
    );
    // A lap that ended cleanly must leave the stack exactly where it started — otherwise the next lap
    // writes over what the last one left and the two shapes stop measuring the same work. Only checkable
    // at a lap boundary: a budget that is not a whole number of laps can cut mid-lap.
    if rest == 0 {
        assert_eq!(c.r[13], SP, "sp did not come back to where it started");
    }
    let width = if thumb { 2 } else { 4 };
    let top = CODE + code.len() as u32 * width;
    assert!(
        c.pc >= CODE && c.pc < top,
        "pc {:#x} left the loop at {CODE:#x}..{top:#x}",
        c.pc
    );
    nanos
}

/// What one entry into the backend seam costs, at the cadence the HLE loop actually calls it.
///
/// `Runtime` calls `cpu.run` once per `SVC` (`src/runtime/mod.rs:5987` and), so a busy frame
/// enters here thousands of times with a small remaining budget — the opposite shape from [`timed`],
/// which hands over 200 M instructions in a single call and therefore cannot see this cost at all.
/// That is why the throughput table above is not evidence that the seam is free.
///
/// # Why this is measured in alternating blocks
///
/// Measuring each arm once and subtracting measures this box's background load, not the seam: the
/// *same* `run_interpreter` path came out at 15.3, 23.5 and 35.3 ns in three runs of one binary.
/// So the two arms alternate in blocks of a few thousand turns — short enough that the load is the
/// same on both sides of a pair, long enough that a 30 ns clock read is not part of the number — and
/// what is reported is the median of the paired differences.
///
/// Returns one ns-per-turn difference per block, `run - run_interpreter`.
fn seam(code: &[u32], thumb: bool, return_to: u32, turns_per_block: u64, blocks: u32) -> Vec<f64> {
    let mut seamed = machine(code, thumb, return_to);
    let mut bare = machine(code, thumb, return_to);

    // Both arms warm before any pair is timed, so the first block is not paying for a cold `Mem`.
    for cpu in [&mut seamed, &mut bare] {
        match cpu.run(1) {
            Stop::Budget { .. } => {}
            other => panic!("the loop stopped early: {other:?}"),
        }
    }

    let mut diffs = Vec::with_capacity(blocks as usize);
    for _ in 0..blocks {
        let started = Instant::now();
        for _ in 0..turns_per_block {
            match seamed.run(1) {
                Stop::Budget { .. } => {}
                other => panic!("the loop stopped early: {other:?}"),
            }
        }
        let through_seam = started.elapsed().as_nanos();

        // The same turn with the box stepped over: what a call cost before the backend trait existed.
        let started = Instant::now();
        for _ in 0..turns_per_block {
            match bare.run_interpreter(1) {
                Stop::Budget { .. } => {}
                other => panic!("the loop stopped early: {other:?}"),
            }
        }
        let through_interpreter = started.elapsed().as_nanos();

        diffs.push((through_seam as f64 - through_interpreter as f64) / turns_per_block as f64);
    }

    assert_eq!(
        seamed.pc, bare.pc,
        "both arms retired the same number of instructions"
    );
    assert_eq!(seamed.r[0], bare.r[0], "and did the same work doing it");
    diffs
}

fn main() {
    // 200M instructions per rep ≈ 2.5 s of wall clock. The first version of this file defaulted to 20M
    // (0.25 s) and the rep-to-rep spread came out 13-33 %, which is wide enough to hide or invent a
    // 8 % effect — a round has to be long before
    // "best of N" means anything, and this box is noisy enough to need it.
    let budget: u64 = env::var("QUOIN_BENCH_STEPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200_000_000);
    let reps: u32 = env::var("QUOIN_BENCH_REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    // (label, code, state, index of the instruction that adds to r0, what `lr` is set to)
    let shapes: [(&str, Vec<u32>, bool, usize, u32); 4] = [
        ("Thumb/data", thumb_loop(), true, 3, CODE),
        ("ARM/data", arm_loop(), false, 3, CODE),
        ("Thumb/stack", thumb_stack_loop(), true, 1, CODE | 1),
        ("ARM/stack", arm_stack_loop(), false, 1, CODE),
    ];

    // The sizes are printed because `Stop` is what every step returns through, and K4 in §9.2 is the
    // proposal to shrink it. This measures the claim instead of reasoning about it from the type.
    println!(
        "sizes: Stop {} B, CpuFault {} B, Mem {} B, Cpu {} B",
        std::mem::size_of::<Stop>(),
        std::mem::size_of::<quoin::cpu::CpuFault>(),
        std::mem::size_of::<Mem>(),
        std::mem::size_of::<Cpu>(),
    );

    // One warm-up per shape, discarded: the first rep of a process pays for the page table growing to
    // the highest address it has ever seen, which is not part of what is being compared.
    for (_, code, thumb, r0_at, return_to) in &shapes {
        timed(code, *thumb, *r0_at, *return_to, budget);
    }

    println!("interpreter microbenchmark: {budget} instructions per rep, {reps} reps");
    for (label, code, thumb, r0_at, return_to) in &shapes {
        let mut best = u128::MAX;
        let mut all = Vec::new();
        for _ in 0..reps {
            let nanos = timed(code, *thumb, *r0_at, *return_to, budget);
            all.push(nanos);
            best = best.min(nanos);
        }
        let best_m = budget as f64 / (best as f64 / 1e9) / 1e6;
        let worst_m = budget as f64 / (*all.iter().max().unwrap() as f64 / 1e9) / 1e6;
        println!(
            "  {label:12} best {best_m:6.2} M instr/s  worst {worst_m:6.2}  \
             ({:.1} ns/instr best, spread {:.1} %)",
            best as f64 / budget as f64,
            100.0 * (*all.iter().max().unwrap() as f64 / best as f64 - 1.0),
        );
    }
    // The seam is per *call*, so it needs the opposite shape from the loops above: budget 1, and
    // alternating blocks (see [`seam`] for why alternating rather than once each).
    let seam_turns: u64 = env::var("QUOIN_BENCH_TURNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000);
    let seam_blocks: u32 = env::var("QUOIN_BENCH_BLOCKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let (seam_code, seam_thumb) = (thumb_loop(), true);
    let mut diffs = seam(&seam_code, seam_thumb, CODE, seam_turns, seam_blocks);
    diffs.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a timing"));
    let median = diffs[diffs.len() / 2];
    let (p25, p75) = (diffs[diffs.len() / 4], diffs[diffs.len() * 3 / 4]);
    let negatives = diffs.iter().filter(|d| **d < 0.0).count();
    println!(
        "backend seam: 1 instruction per turn, {seam_turns} turns/block, {seam_blocks} blocks"
    );
    println!(
        "  the seam itself      {median:5.1} ns/call  (median; p25 {p25:.1} .. p75 {p75:.1}, \
         {negatives} blocks measured it as free)"
    );
    // How often a frame enters the seam is not measured anywhere yet — `Stop::Hle` is the entry, so
    // the count is the frame's HLE call count. These are the bracket, not a claim.
    for calls in [1_000.0, 3_000.0, 10_000.0] {
        let micros = median * calls / 1000.0;
        println!(
            "  at {calls:>6.0} calls/frame  {micros:6.1} µs/frame  ({:.2} % of 16.7 ms)",
            micros / 16_700.0 * 100.0,
        );
    }
    println!(
        "override with QUOIN_BENCH_STEPS / QUOIN_BENCH_REPS; \
         run only this target (`cargo bench --bench interp`), not bare `cargo bench`."
    );
}
