//! The same loop, two execution cores: the interpreter and the Rynarmic backend.
//!
//! `cargo bench --features jit --bench jit` (the target only exists with `jit`).
//!
//! The four loops here use the **same encoding** as `benches/interp.rs`; keep them in
//! sync, since a speedup claim only means something when both sides measure the same guest code.
//!
//! Exact instruction counts are not compared: block granularity lets the backend finish
//! a whole block in the last budget slot (at most one extra block), so this only checks that the loop ran.

use std::env;
use std::time::Instant;

use quoin::cpu::{Cpu, Stop};
use quoin::mem::Mem;

const CODE: u32 = 0x1000_0000;
const DATA: u32 = 0x1000_1000;
const SP: u32 = 0x1000_2800;

fn back_offset(index: usize) -> i32 {
    -(index as i32 + 2)
}

fn thumb_loop() -> Vec<u32> {
    let mut code = vec![0x6811, 0x3101, 0x6011, 0x3001];
    code.push(0xd100 | (back_offset(code.len()) as u32 & 0xff));
    code
}

fn arm_loop() -> Vec<u32> {
    let mut code = vec![
        0xe592_1000,
        0xe281_1001,
        0xe582_1000,
        0xe280_0001,
        0xe350_0000,
    ];
    code.push(0x1a00_0000 | (back_offset(code.len()) as u32 & 0x00ff_ffff));
    code
}

fn thumb_stack_loop() -> Vec<u32> {
    vec![0xb530, 0x3001, 0x6811, 0xbd30]
}

fn arm_stack_loop() -> Vec<u32> {
    vec![0xe92d_4030, 0xe280_0001, 0xe592_1000, 0xe8bd_8030]
}

fn machine(code: &[u32], thumb: bool, return_to: u32, jit: bool) -> Cpu {
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
    if jit {
        c.enable_jit();
    }
    c
}

/// Run `budget` instructions and return nanoseconds; assert only that it ran and stayed inside the loop.
fn timed(label: &str, code: &[u32], thumb: bool, return_to: u32, jit: bool, budget: u64) -> u128 {
    let mut c = machine(code, thumb, return_to, jit);
    let started = Instant::now();
    let stop = c.run(budget);
    let nanos = started.elapsed().as_nanos();
    match stop {
        Stop::Budget { .. } => {}
        other => panic!("[{label} jit={jit}] the loop stopped early: {other:?}"),
    }
    assert!(c.r[0] > 1, "the counter did not move");
    let width = if thumb { 2 } else { 4 };
    let top = CODE + code.len() as u32 * width;
    assert!(
        c.pc >= CODE && c.pc < top,
        "[{label} jit={jit}] pc {:#x} left the loop at {CODE:#x}..{top:#x}",
        c.pc
    );
    nanos
}

fn main() {
    let budget: u64 = env::var("QUOIN_BENCH_STEPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000_000);
    let reps: u32 = env::var("QUOIN_BENCH_REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let shapes: [(&str, Vec<u32>, bool, u32); 4] = [
        ("Thumb/data", thumb_loop(), true, CODE),
        ("ARM/data", arm_loop(), false, CODE),
        ("Thumb/stack", thumb_stack_loop(), true, CODE | 1),
        ("ARM/stack", arm_stack_loop(), false, CODE),
    ];

    for (label, code, thumb, return_to) in &shapes {
        timed(label, code, *thumb, *return_to, false, budget / 20);
        timed(label, code, *thumb, *return_to, true, budget / 20);
    }

    println!("{budget} instructions per rep, {reps} reps; M instr/s (best of rep)");
    println!(
        "  {:<12} {:>10} {:>10} {:>8}",
        "shape", "interp", "rynarmic", "speedup"
    );

    let mut totals = (0.0f64, 0.0f64);
    for (label, code, thumb, return_to) in &shapes {
        let measure = |jit: bool| {
            let mut best = u128::MAX;
            for _ in 0..reps {
                best = best.min(timed(label, code, *thumb, *return_to, jit, budget));
            }
            budget as f64 / (best as f64 / 1e9) / 1e6
        };
        let interp = measure(false);
        let jit = measure(true);
        totals.0 += interp;
        totals.1 += jit;
        println!(
            "  {label:<12} {interp:>10.1} {jit:>10.1} {:>7.2}x",
            jit / interp
        );
    }
    println!(
        "  {:<12} {:>10.1} {:>10.1} {:>7.2}x",
        "mean",
        totals.0 / shapes.len() as f64,
        totals.1 / shapes.len() as f64,
        totals.1 / totals.0
    );
    println!("override with QUOIN_BENCH_STEPS / QUOIN_BENCH_REPS");
}
