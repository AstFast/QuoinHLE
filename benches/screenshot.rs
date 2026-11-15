//! One presentation's host-side cost: reading the device framebuffer out of guest memory.
//!
//! `cargo bench --bench screenshot`. Hand-written framebuffer bytes, no pack, no window, no guest
//! execution — the no-guest-run rule is about running a customer's game, and nothing here loads
//! one. The number says how much of `session`'s 16 ms turn is spent in `Display::screenshot`
//! itself rather than in the guest, so a claim about speeding it up has something to stand on.
//!
//! `harness = false`: a plain `main`, stable toolchain, no dependency. Run *this target only*;
//! bare `cargo bench` also builds the corpus-gated integration tests under `tests/`.

use std::hint::black_box;
use std::time::Instant;

use quoin::runtime::Runtime;

const FRAMES: u32 = 2000;
const WARMUP: u32 = 50;

fn main() {
    let mut rt = Runtime::new();
    let (wide, high) = (
        usize::from(rt.display.device.width),
        usize::from(rt.display.device.height),
    );
    // A recognisable first row, so the read cannot collapse into an all-zero buffer and a change
    // to the pixel format shows up in the checksum instead of in a silent pass.
    let base = rt.display.device.px;
    for i in 0..16u32 {
        rt.cpu
            .mem
            .write_u16(base + i * 2, u32::from(0xf81f ^ (i as u16)))
            .expect("the booted device framebuffer is mapped");
    }

    for _ in 0..WARMUP {
        black_box(rt.display.screenshot(&rt.cpu.mem).unwrap());
    }
    let began = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..FRAMES {
        let rgb = rt.display.screenshot(&rt.cpu.mem).unwrap();
        checksum = checksum.wrapping_add(u64::from(rgb[0]) + u64::from(rgb[rgb.len() - 1]));
    }
    let elapsed = began.elapsed();
    println!(
        "screenshot {wide}x{high}: {FRAMES} frames in {elapsed:?} = {:.2} us/frame \
         ({:.1} Mpixel/s), checksum {checksum}",
        elapsed.as_secs_f64() * 1e6 / f64::from(FRAMES),
        (wide * high) as f64 * f64::from(FRAMES) / elapsed.as_secs_f64() / 1e6,
    );
}
