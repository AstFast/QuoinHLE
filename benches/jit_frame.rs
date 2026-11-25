//! Zero-sample set JIT frame costs: `cargo bench --features jit --bench jit_frame`.
//!
//! Two things a throughput number cannot see, both of them JIT-specific and both of them capable of
//! eating a whole frame without retiring a single expensive guest instruction:
//!
//! 1. **Cold compilation.** Rynarmic translates a basic block the first time the PC reaches it, on
//!    the caller's thread. A scene change that jumps into a lot of fresh code pays for all of those
//!    translations inside one `pump`. Worse, the fork's own `rynarmic::cpu::JIT_ARENA_LIMIT` is 16384
//!    blocks: once the arena fills, `lookup_or_translate` calls `invalidate_code_cache()` and throws
//!    away *every* block, so a working set larger than the arena recompiles forever instead of ever
//!    getting warm. The host no longer lives with that — `src/cpu/jit.rs` raises the limit to
//!    [`ARENA_BLOCKS`] (65536) and caps compilation per turn — and this bench takes both from the
//!    crate so it measures the arrangement a run actually uses, not the fork's default.
//! 2. **SVC trap round-trip.** Every HLE call leaves the backend, runs `Runtime::dispatch`, and
//!    re-enters `run_jit` (halt trap check, window generation, memory rebind, backend entry). A fast
//!    guest can reach the host 16x more often per second than the interpreter ever could.
//!
//! Everything here is hand-written ARM words mapped into a fresh `Mem`: no `.mod`, no window, no HLE
//! service, no sample set. Same category as `benches/interp.rs`: self-contained, with no guest pack.
//!
//! The numbers are not a game's frame time. They are the price of one unit of work, so that a trace
//! saying "N host calls this frame" can be turned into milliseconds.

use std::env;
use std::time::Instant;

use quoin::cpu::jit::{ARENA_BLOCKS, COMPILE_BUDGET_PER_TURN};
use quoin::cpu::{Cpu, Stop};
use quoin::mem::Mem;

const CODE: u32 = 0x1000_0000;

/// `add r0, r0, #1` / `b next`. The unconditional branch ends the translated block, so every pair
/// becomes one cached JitBlock. The last branch loops back to `CODE`.
fn chained_blocks(n: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        out.push(0xe280_0001);
        let at = CODE + (out.len() as u32) * 4;
        out.push(if i + 1 == n {
            let delta = (CODE as i64 - (at as i64 + 8)) / 4;
            0xea00_0000 | ((delta as u32) & 0x00ff_ffff)
        } else {
            // offset 0 -> branch to the instruction right after this one
            0xea00_0000
        });
    }
    out
}

/// The one-block loop from `benches/jit.rs`, so "N instructions with no compilation" has a number.
fn one_block_loop() -> Vec<u32> {
    let mut code = vec![
        0xe592_1000, // ldr r1, [r2]
        0xe281_1001, // add r1, r1, #1
        0xe582_1000, // str r1, [r2]
        0xe280_0001, // add r0, r0, #1
        0xe350_0000, // cmp r0, #0
    ];
    let off = -(code.len() as i32 + 2);
    code.push(0x1a00_0000 | (off as u32 & 0x00ff_ffff));
    code
}

fn machine(words: &[u32]) -> Cpu {
    let mut c = Cpu::new(Mem::new());
    // The generated program decides the mapping: a work set larger than the arena needs more
    // pages than any fixed default could give it. The trailing page holds the loop's data word
    // and stack pointer *outside* the generated code — a fixed DATA address inside the code area
    // would silently overwrite instruction 1024 and change the program being measured.
    let len = ((words.len() * 4) as u32 + 0x1000).max(0x3000);
    c.mem.map(CODE, len);
    for (i, w) in words.iter().enumerate() {
        c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
    }
    let data = CODE + len - 0x1000;
    c.pc = CODE;
    c.r[0] = 1;
    c.r[2] = data;
    c.r[13] = CODE + len - 0x100;
    c.r[14] = CODE;
    c.mem.write_u32(data, 0).unwrap();
    c.enable_jit();
    c
}

/// `None` leaves the host's default (`ARENA_BLOCKS`); `Some(n)` reproduces an older/smaller budget.
/// The override clears the (still empty) block cache, so it has to happen before the first run.
fn machine_with(words: &[u32], arena: Option<usize>) -> Cpu {
    let mut c = machine(words);
    if let Some(n) = arena {
        c.set_jit_arena_limit(n);
    }
    c
}

/// One `Cpu::run` of `budget` instructions, with the shape asserted so a number is never printed
/// for a run that stopped early.
fn timed(c: &mut Cpu, budget: u64) -> u128 {
    let began = Instant::now();
    let stop = c.run(budget);
    let nanos = began.elapsed().as_nanos();
    assert!(
        matches!(stop, Stop::Budget { .. }),
        "the JIT stopped early: {stop:?}"
    );
    nanos
}

/// What one 60-byte register copy costs: the size of `Cpu::r` and of Rynarmic's `state.r[..15]`.
///
/// A round-trip makes two of them — into `core.state` before the block runs, back out after — so
/// this is the floor on the "execute + state sync" arm of [`trap_decomposition`]. Without it there
/// is no way to tell "our synchronisation is expensive" from "Rynarmic's call path is".
///
/// `black_box` on both ends: without it LLVM sees a loop copying a constant array back and forth
/// and folds the whole thing away.
fn register_copy_cost(rounds: u32) -> f64 {
    let (slice, elementwise, whole) = register_copy_variants(rounds);
    println!("    register copy, 15 u32: copy_from_slice {slice:.2} ns, element-wise {elementwise:.2} ns, array assignment {whole:.2} ns");
    slice
}

/// The same 60 bytes moved three ways, because the round-trip uses the first one and 5.5 ns for
/// 60 bytes is far too slow to be the hardware's answer.
///
/// `copy_from_slice` on a `[u32]` of unknown length lowers to a `memcpy` call, not to inline moves:
/// the length is a parameter, so LLVM cannot unroll it. An array of known size can be moved with
/// four 16-byte moves. This exists to find out which one the synchronisation is actually paying for
/// before anyone redesigns the register file around it.
fn register_copy_variants(rounds: u32) -> (f64, f64, f64) {
    let mut a = [0u32; 15];
    let mut b = [0u32; 15];
    let mut c = [0u32; 16];
    let mut d = [0u32; 16];
    std::hint::black_box((&mut a, &mut c));

    let began = Instant::now();
    for _ in 0..rounds {
        b.copy_from_slice(std::hint::black_box(&a));
        a.copy_from_slice(std::hint::black_box(&b));
    }
    let slice = began.elapsed().as_nanos() as f64 / (rounds as f64 * 2.0);

    let began = Instant::now();
    for _ in 0..rounds {
        for i in 0..15 {
            b[i] = std::hint::black_box(a[i]);
        }
        for i in 0..15 {
            a[i] = std::hint::black_box(b[i]);
        }
    }
    let elementwise = began.elapsed().as_nanos() as f64 / (rounds as f64 * 2.0);

    let began = Instant::now();
    for _ in 0..rounds {
        d = std::hint::black_box(c);
        c = std::hint::black_box(d);
    }
    let whole = began.elapsed().as_nanos() as f64 / (rounds as f64 * 2.0);

    std::hint::black_box((&a, &b, &c, &d));
    (slice, elementwise, whole)
}

/// A single `SVC #0` whose LR points back at itself: one trap per `run(1)`.
fn trap_machine() -> Cpu {
    let mut c = machine(&[0xef00_0000]);
    c.r[14] = CODE;
    c
}

/// The same shape with nothing to trap on: `MOV r0, r0` then `B` back to it.
///
/// Two instructions so a `run(1)` alternates between them and pays an ordinary fetch/decode, rather
/// than measuring one branch over and over. The branch offset is `-3` words: `B` at `CODE + 4`
/// resolves to `(CODE + 4) + 8 - 12 = CODE`, which is the `MOV`.
fn plain_machine() -> Cpu {
    machine(&[0xE1A0_0000, 0xEAFF_FFFD])
}

/// Split one round-trip into "entry + state synchronisation" and "the trap itself".
///
/// Both arms retire exactly one instruction per `run(1)` and both pay the same entry work — the halt
/// trap check, the window generation compare, the memory rebind, and the backend box — and the same
/// two register copies in and out of `core.state`. The difference is what Rynarmic charges to
/// detect an `SVC`, unwind out of the translated block, and hand back `SoftwareInterrupt(id)`.
///
/// That difference is the part **this crate cannot change**: it lives behind `run_cached_counted`.
/// Knowing its size is what decides whether the round-trip is worth another look at all, so it is
/// measured rather than assumed. Alternating blocks and medians, for the reason `benches/interp.rs`
/// gives: this box's background load moves a single measurement by more than the effect.
///
/// A third arm, `run(0)`, splits the fixed cost off: `limit` is already reached on the first check,
/// so the loop returns before `run_cached_counted` is called. What remains is `Cpu::run`'s gate and
/// box, `Jit::run`'s halt-trap confirmation, the window generation compare, and the memory rebind —
/// everything a turn pays whether or not it executes anything.
///
/// Returns `(trap, plain, entry_only)` ns/turn per block.
fn trap_decomposition(n_per_block: u32, blocks: u32) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut trap = trap_machine();
    let mut plain = plain_machine();
    let mut idle = plain_machine();
    // Warm both before any pair is timed, so the first block is not paying for a cold arena.
    for _ in 0..64 {
        assert!(matches!(trap.run(1), Stop::Hle { .. }));
        assert!(matches!(plain.run(1), Stop::Budget { .. }));
        assert!(matches!(idle.run(0), Stop::Budget { .. }));
    }

    let mut traps = Vec::with_capacity(blocks as usize);
    let mut plains = Vec::with_capacity(blocks as usize);
    let mut entries = Vec::with_capacity(blocks as usize);
    for _ in 0..blocks {
        let began = Instant::now();
        for _ in 0..n_per_block {
            match trap.run(1) {
                Stop::Hle { .. } => {}
                other => panic!("expected one SVC trap per run, got {other:?}"),
            }
        }
        traps.push(began.elapsed().as_nanos() as f64 / n_per_block as f64);

        let began = Instant::now();
        for _ in 0..n_per_block {
            match plain.run(1) {
                Stop::Budget { .. } => {}
                other => panic!("the plain loop stopped early: {other:?}"),
            }
        }
        plains.push(began.elapsed().as_nanos() as f64 / n_per_block as f64);

        let began = Instant::now();
        for _ in 0..n_per_block {
            match idle.run(0) {
                Stop::Budget { .. } => {}
                other => panic!("a zero budget executed something: {other:?}"),
            }
        }
        entries.push(began.elapsed().as_nanos() as f64 / n_per_block as f64);
    }
    (traps, plains, entries)
}

fn trap_roundtrip(n: u32) -> u128 {
    let mut c = trap_machine();
    // Compile the block before the clock starts. `enable_jit` hands over a cold arena, and one
    // translation is hundreds of microseconds — spread over `n` turns that is tens of nanoseconds
    // per turn, which is the same size as the thing being measured. Every figure this function
    // reported before this warm-up (85 / 95 / 99 ns) included it; the round-trip itself is under 30.
    for _ in 0..64 {
        assert!(matches!(c.run(1), Stop::Hle { .. }));
    }
    let began = Instant::now();
    for _ in 0..n {
        match c.run(1) {
            Stop::Hle { .. } => {}
            other => panic!("expected one SVC trap per run, got {other:?}"),
        }
    }
    began.elapsed().as_nanos()
}

/// The same guest loop with its data word in a **windowed** span, against one in a span of its own.
///
/// `Mem::fast_spans` publishes only spans of two or more contiguous pages, and the backend publishes
/// at most four of them, so a one-page region is always the callback path: every load and store
/// there leaves the generated code, calls into Rust, and comes back. One program, one host, so the
/// difference is exactly what an inlined access buys.
fn window_vs_callback(steps: u64) -> ((u128, u32), (u128, u32)) {
    const LONE_DATA: u32 = 0x1001_0000;
    let words = one_block_loop();
    let run = |lone: bool| {
        let mut c = Cpu::new(Mem::new());
        // The code always gets two pages: a one-page code span would put instruction fetch on the
        // callback path too, and then this would be an A/B of code access rather than data access.
        c.mem.map(CODE, 0x2000);
        for (i, w) in words.iter().enumerate() {
            c.mem.write_u32(CODE + i as u32 * 4, *w).unwrap();
        }
        let data = if lone {
            // Exactly one page, and not adjacent to the code: no window can cover it.
            c.mem.map(LONE_DATA, 0x1000);
            LONE_DATA
        } else {
            CODE + 0x1000 // second page of the code span, so the code's window covers it
        };
        c.mem.write_u32(data, 0).unwrap();
        c.pc = CODE;
        c.r[0] = 1;
        c.r[2] = data;
        c.r[13] = CODE + 0x1f00;
        c.r[14] = CODE;
        c.enable_jit();
        let began = Instant::now();
        let stop = c.run(steps);
        let nanos = began.elapsed().as_nanos();
        assert!(
            matches!(stop, Stop::Budget { .. }),
            "the JIT stopped early: {stop:?}"
        );
        (nanos, c.r[0])
    };
    (run(false), run(true))
}

fn best<F: FnMut() -> u128>(reps: u32, mut f: F) -> u128 {
    let mut out = u128::MAX;
    for _ in 0..reps {
        out = out.min(f());
    }
    out
}

/// A running game's address space: 344 KiB image, a 5 MiB heap, a 512 KiB stack.
///
/// That is seven spans at 1 MiB boundaries — image (1), heap (one per MiB, 5), stack (1) — which is
/// more than the four windows the backend used to accept. The guest loop stays inside one linked
/// block and does two stack accesses per iteration, so a stack span that never got a window shows
/// up here as the callback path (`window_vs_callback` puts that at 2.4x).
fn realistic_layout(steps: u64) -> (usize, u128) {
    realistic_layout_with(steps, 0x50_0000, 128)
}

/// The same loop and the same stack, with the heap's size and the stack's size as the knobs.
///
/// The heap decides **where in the window list the stack lands** (windows are ordered longest
/// first, so a 5 MiB heap pushes the stack to sixth place), and an eight-page stack — or a smaller
/// one — decides whether it gets a window at all. That is the whole question: a window two compares
/// away, five compares away, or a callback.
fn realistic_layout_with(steps: u64, heap_bytes: u32, stack_pages: u32) -> (usize, u128) {
    const IMAGE: u32 = 0x1000_0000;
    const HEAP: u32 = 0x2000_0000;
    const STACK_LOW: u32 = 0x2FF8_0000;
    let words = [
        0xe52d_1004u32, // str r1, [sp, #-4]!
        0xe49d_1004,    // ldr r1, [sp], #4
        0xe280_0001,    // add r0, r0, #1
        0xeaff_fffb,    // b back to the first instruction
    ];
    let mut c = Cpu::new(Mem::new());
    c.mem.map(IMAGE, 0x5_7000);
    if heap_bytes > 0 {
        c.mem.map(HEAP, heap_bytes);
    }
    c.mem.map(STACK_LOW, stack_pages.max(1) * 4096);
    for (i, w) in words.iter().enumerate() {
        c.mem.write_u32(IMAGE + i as u32 * 4, *w).unwrap();
    }
    c.pc = IMAGE;
    c.r[0] = 1;
    c.r[1] = 0;
    c.r[13] = STACK_LOW + (stack_pages.max(1) - 1) * 4096 + 0x800;
    c.r[14] = IMAGE;
    c.enable_jit();
    // A short run first: it compiles the loop's block and publishes the window set.
    let _ = c.run(64);
    let windows = c.jit_windows().unwrap_or(0);
    let began = Instant::now();
    let stop = c.run(steps);
    let nanos = began.elapsed().as_nanos();
    assert!(
        matches!(stop, Stop::Budget { .. }),
        "the JIT stopped early: {stop:?}"
    );
    (windows, nanos)
}

fn main() {
    let reps: u32 = env::var("QUOIN_FRAME_REPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let blocks: Vec<usize> = env::var("QUOIN_FRAME_BLOCKS")
        .ok()
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![8000, 20000]);
    let traps: u32 = env::var("QUOIN_FRAME_TRAPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let arena: Option<usize> = env::var("QUOIN_FRAME_ARENA")
        .ok()
        .and_then(|v| v.parse().ok());
    let ab_blocks: usize = env::var("QUOIN_FRAME_AB_BLOCKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);

    println!("jit frame costs - hand-written ARM words, no pack, no window, no HLE service");
    println!("reps={reps} (best of), arena default = {ARENA_BLOCKS} blocks, override = {arena:?}");
    println!();
    // Measured *before* the compilation runs: translating tens of thousands of blocks leaves the
    // host busy enough to skew a nanosecond-scale loop that follows it.
    let per_trap = best(reps, || trap_roundtrip(traps)) / traps as u128;
    println!(
        "  svc trap exit/re-enter: {per_trap} ns per trap ({traps} traps, excludes Runtime::dispatch)"
    );

    // The split, right after: same guest shape, one armed with `SVC` and one with an ordinary
    // instruction, so the difference is the trap and the rest is ours.
    let split_blocks: u32 = env::var("QUOIN_FRAME_SPLIT_BLOCKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let split_turns: u32 = env::var("QUOIN_FRAME_SPLIT_TURNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    let (mut trap_ns, mut plain_ns, mut entry_ns) = trap_decomposition(split_turns, split_blocks);
    trap_ns.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    plain_ns.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    entry_ns.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    let trap_median = trap_ns[trap_ns.len() / 2];
    let plain_median = plain_ns[plain_ns.len() / 2];
    let entry_median = entry_ns[entry_ns.len() / 2];
    println!(
        "  split ({split_turns} turns/block, {split_blocks} blocks, median): \
         run(0) {entry_median:.1} ns, plain {plain_median:.1} ns, svc {trap_median:.1} ns"
    );
    println!(
        "    entry only {entry_median:.1} ns, execute + state sync {:.1} ns, \
         the trap itself {:.1} ns",
        plain_median - entry_median,
        trap_median - plain_median,
    );
    // Where the middle arm goes. Two copies of this size are the part we own; the rest is
    // `run_cached_counted` itself, which is inside the fork.
    let copy_ns = register_copy_cost(20_000_000);
    println!(
        "    of that: two register copies {:.1} ns ({:.0} %), run_cached_counted the rest ({:.1} ns)",
        copy_ns * 2.0,
        100.0 * copy_ns * 2.0 / (plain_median - entry_median).max(0.1),
        (plain_median - entry_median) - copy_ns * 2.0,
    );
    println!();
    let access_steps: u64 = env::var("QUOIN_FRAME_ACCESS_STEPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000_000);
    let ((win_ns, win_count), (cb_ns, cb_count)) = window_vs_callback(access_steps);
    println!(
        "  data access, same loop: windowed {:.2} ns/instr, one-page span {:.2} ns/instr ({:.1}x slower)",
        win_ns as f64 / access_steps as f64,
        cb_ns as f64 / access_steps as f64,
        cb_ns as f64 / win_ns.max(1) as f64
    );
    assert_eq!(win_count, cb_count, "both loops must retire the same count");
    let (windows, real_ns) = realistic_layout(access_steps);
    println!(
        "  realistic layout (image + 5 MiB heap + stack): {windows} windows, stack loop {:.2} ns/instr",
        real_ns as f64 / access_steps as f64
    );
    for (heap, pages) in [
        (0u32, 128u32),
        (0x10_0000, 128),
        (0x30_0000, 128),
        (0x50_0000, 128),
        (0x50_0000, 1),
    ] {
        let (n, ns) = realistic_layout_with(access_steps, heap, pages);
        let label = if heap == 0 {
            "no heap".to_string()
        } else {
            format!("{} KiB", heap / 1024)
        };
        let note = if pages == 1 {
            "  <- stack unwindowed: what the old four slots gave it"
        } else {
            ""
        };
        println!(
            "    heap {:<9} {n} windows, stack loop {:.2} ns/instr{note}",
            label,
            ns as f64 / access_steps as f64
        );
    }
    println!();
    println!("  cold compile: chained blocks, first pass vs second pass on the same Cpu");
    for &n in &blocks {
        let words = chained_blocks(n);
        let total = (2 * n) as u64;
        let mut cold = u128::MAX;
        let mut warm = u128::MAX;
        for _ in 0..reps {
            let mut c = machine_with(&words, arena);
            cold = cold.min(timed(&mut c, total));
            warm = warm.min(timed(&mut c, total));
        }
        println!(
            "    N={n:<6} cold {:>9.2} ms   warm {:>9.2} ms   cold/warm {:>7.1}x",
            cold as f64 / 1e6,
            warm as f64 / 1e6,
            cold as f64 / warm as f64
        );
    }
    let loop_words = one_block_loop();
    let biggest = (2 * blocks.iter().max().copied().unwrap_or(1)) as u64;
    let baseline = best(reps, || {
        timed(&mut machine_with(&loop_words, arena), biggest)
    });
    println!(
        "    one cached block, same {} instructions: {:.2} us",
        biggest,
        baseline as f64 / 1e3
    );
    println!();

    // The decisive A/B: the same program, the old 16384-block budget against this build's default.
    // A second pass that stays slow is thrashing; one that collapses to milliseconds is fixed.
    let words = chained_blocks(ab_blocks);
    let total = (2 * ab_blocks) as u64;
    let mut old = (u128::MAX, u128::MAX);
    let mut new = (u128::MAX, u128::MAX);
    for _ in 0..reps {
        let mut c = machine_with(&words, Some(16_384));
        let a = timed(&mut c, total);
        let b = timed(&mut c, total);
        old = (old.0.min(a), old.1.min(b));
        let mut c = machine_with(&words, None);
        let a = timed(&mut c, total);
        let b = timed(&mut c, total);
        new = (new.0.min(a), new.1.min(b));
    }
    println!("  arena A/B at N={ab_blocks} blocks (same guest words, same host):");
    println!(
        "    16384   cold {:>9.2} ms   warm {:>9.2} ms",
        old.0 as f64 / 1e6,
        old.1 as f64 / 1e6
    );
    println!(
        "    {ARENA_BLOCKS:<6} cold {:>9.2} ms   warm {:>9.2} ms   warm speedup {:>7.1}x",
        new.0 as f64 / 1e6,
        new.1 as f64 / 1e6,
        old.1 as f64 / new.1 as f64
    );
    println!();
    println!(
        "  A cold pass that is much slower than the warm one is compilation paid inside one turn."
    );
    println!("  A warm pass that stays slow above the arena limit is code-cache thrashing.");

    // The stutter the arena fix does not remove: a first visit to N fresh blocks, all translated on
    // the executing thread. Capping how many a single turn may compile moves that cost into the
    // following turns. Worst-turn latency is what a player feels; total time is what it costs.
    let chunk: u64 = env::var("QUOIN_FRAME_CHUNK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(65_536);
    let cap: u32 = env::var("QUOIN_FRAME_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(COMPILE_BUDGET_PER_TURN);
    println!();
    println!(
        "  per-turn compile budget at N={ab_blocks} blocks (a turn = {chunk} instructions, cap = {cap}):"
    );
    for (label, budget) in [("uncapped", None), ("capped", Some(cap))] {
        let mut c = machine_with(&words, None);
        let mut worst = 0u128;
        let mut total = 0u128;
        let mut turns = 0u32;
        loop {
            c.set_jit_compile_budget(budget);
            let began = Instant::now();
            let stop = c.run(chunk);
            let nanos = began.elapsed().as_nanos();
            worst = worst.max(nanos);
            total += nanos;
            turns += 1;
            // Every block has machine code now, or the guest stopped on its own: either way the
            // first visit is over and this turn's number is the one to report.
            if c.jit_cached_blocks() == Some(ab_blocks) || !matches!(stop, Stop::Budget { .. }) {
                break;
            }
            if turns >= 10_000 {
                break;
            }
        }
        println!(
            "    {label:<8} worst turn {:>9.2} ms   total {:>9.2} ms   turns {turns}",
            worst as f64 / 1e6,
            total as f64 / 1e6
        );
    }
    println!();
    println!(
        "  A capped run pays more wall time in total and never blocks a frame for one long stall."
    );
}
