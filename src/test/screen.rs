// Moved out of `src/screen.rs` so that file stays implementation only, the arrangement every other
// module here uses. The `#[path]` declaration keeps this a child module, which is what lets these
// tests reach the private buffers — and two of them need to: proving that a row was *not* converted
// is only possible by writing a marker into the output and checking it survived.

use super::*;
use crate::abi;
use crate::display::{pack565, unpack565};
use crate::hle::Hle;
use crate::layout;

/// A display whose device bitmap is a buffer this test owns, so it can draw by writing words.
///
/// `wide` x `high` at `depth`, the same three numbers `Display::new` takes.
struct Fixture {
    mem: Mem,
    hle: Hle,
    display: Display,
    present: u32,
    device: u32,
}

impl Fixture {
    fn new(wide: u16, high: u16, depth: u16) -> Self {
        let mut mem = Mem::new();
        mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
        let mut hle = Hle::new();
        let mut display = Display::new(&mut mem, &mut hle, wide, high, depth).unwrap();
        let present = display.create_display(&mut mem, &mut hle).unwrap();
        let device = display.device_bitmap();
        // The module's own surface, the way a pack that composes off-screen does it: point the device
        // `IDIB` at a buffer we control, then present so the host adopts it.
        let px = mem.alloc(u32::from(wide) * u32::from(high) * 2, 4).unwrap();
        let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
        mem.write_u32(device + at("pBmp"), px).unwrap();
        display.update(&mem, present, 0).unwrap();
        Self {
            mem,
            hle,
            display,
            present,
            device,
        }
    }

    /// Paint one pixel of the screen, in 565.
    fn put(&mut self, x: u32, y: u32, rgb: (u8, u8, u8)) {
        let px = self.mem.read_u32(self.device_row()).unwrap();
        let wide = u32::from(self.display.device.width);
        let at = px + (y * wide + x) * 2;
        self.mem
            .write_u16(at, u32::from(pack565(rgb.0, rgb.1, rgb.2)))
            .unwrap();
    }

    fn device_row(&self) -> u32 {
        let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
        self.device + at("pBmp")
    }

    /// Paint every pixel one row at a time, so a whole row is one distinguishable band.
    fn fill_row(&mut self, y: u32, rgb: (u8, u8, u8)) {
        let wide = u32::from(self.display.device.width);
        for x in 0..wide {
            self.put(x, y, rgb);
        }
    }

    fn refresh(&mut self, screen: &mut Screen) {
        screen.refresh(&self.display, &self.mem).unwrap();
    }

    /// The reference conversion, which is the oracle every test here compares against.
    fn screenshot(&self) -> Vec<u8> {
        self.display.screenshot(&self.mem).unwrap()
    }
}

#[test]
fn the_screen_says_what_the_reference_capture_says() {
    let mut f = Fixture::new(24, 16, 16);
    for y in 0..16 {
        f.fill_row(y, (y as u8 * 15, 255 - y as u8 * 15, (y * 7) as u8));
    }
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert_eq!(screen.size(), (24, 16));
    assert_eq!(
        screen.pixels(),
        f.screenshot().as_slice(),
        "a row-at-a-time conversion has to be the same bytes as the one-shot path"
    );
    assert_eq!(
        screen.bands(),
        &[(0, 16)],
        "the first refresh is the whole screen"
    );
    assert_eq!(screen.epoch(), f.display.presentations);
}

#[test]
fn a_present_of_the_same_picture_costs_no_conversion() {
    let mut f = Fixture::new(24, 16, 16);
    f.fill_row(4, (255, 0, 0));
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert!(!screen.bands().is_empty());

    // Present again without drawing anything: the counter moves, the picture does not. A window that
    // uploaded on the counter alone would re-upload 76 800 pixels for nothing.
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert!(
        screen.bands().is_empty(),
        "the guest presented, but no row moved: {:?}",
        screen.bands()
    );
    assert_eq!(
        screen.epoch(),
        f.display.presentations,
        "the epoch still moves"
    );
}

#[test]
fn only_the_rows_that_moved_are_converted() {
    let mut f = Fixture::new(24, 16, 16);
    for y in 0..16 {
        f.fill_row(y, (0, 0, 255));
    }
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert_eq!(screen.bands(), &[(0, 16)]);

    // A marker in a row nobody will touch. If that row is converted again the marker disappears, so
    // this proves the saving rather than inferring it from a clock.
    let stride = 24 * 3;
    for x in 0..24 {
        screen.pixels[15 * stride + x * 3] = 0xAB;
    }
    f.fill_row(5, (255, 0, 0));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);

    assert_eq!(
        screen.bands(),
        &[(5, 6)],
        "one row moved, one row is reported"
    );
    assert_eq!(
        &screen.pixels()[5 * stride..5 * stride + 3],
        &[255, 0, 0],
        "and the row that moved is the new colour"
    );
    assert_eq!(
        screen.pixels()[15 * stride],
        0xAB,
        "the untouched row was left alone — the marker survived"
    );
}

#[test]
fn nearby_rows_are_one_band_and_distant_ones_are_two() {
    let mut f = Fixture::new(24, 16, 16);
    let mut screen = Screen::new();
    f.refresh(&mut screen);

    // Two rows three apart: one upload is cheaper than two, so they are merged.
    f.fill_row(3, (255, 0, 0));
    f.fill_row(6, (255, 0, 0));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert_eq!(
        screen.bands(),
        &[(3, 7)],
        "a three-row gap is under the merge threshold"
    );

    // More than `MERGE_GAP` apart: two rects, because a single one would upload 30 rows of nothing.
    f.fill_row(0, (0, 255, 0));
    f.fill_row(15, (0, 255, 0));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert_eq!(screen.bands(), &[(0, 1), (15, 16)]);
}

#[test]
fn a_screen_the_guest_never_presented_is_not_read_again() {
    let mut f = Fixture::new(24, 16, 16);
    f.fill_row(2, (9, 9, 9));
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    let epoch = screen.epoch();

    // Drawing without presenting is invisible to a display host by construction: the module has not
    // said the picture is finished. A refresh in that window must neither read nor report.
    f.fill_row(9, (200, 0, 0));
    f.refresh(&mut screen);
    assert!(screen.bands().is_empty());
    assert_eq!(screen.epoch(), epoch);
    assert_eq!(
        screen.pixels()[9 * 24 * 3],
        0,
        "the unpresented pixel is not on screen yet — and must not be, or a half-drawn frame would \
         be uploaded as if it were finished"
    );
}

#[test]
fn a_wider_screen_is_adopted_whole() {
    let mut small = Fixture::new(8, 8, 16);
    let mut screen = Screen::new();
    small.refresh(&mut screen);
    assert_eq!(screen.size(), (8, 8));

    // The same `Screen` against a larger display: every row is a change, because the buffer it had
    // describes a different screen.
    let mut large = Fixture::new(16, 12, 16);
    large.fill_row(11, (1, 2, 3));
    screen.refresh(&large.display, &large.mem).unwrap();
    assert_eq!(screen.size(), (16, 12));
    assert_eq!(screen.bands(), &[(0, 12)]);
    assert_eq!(screen.pixels(), large.screenshot().as_slice());
}

#[test]
fn a_depth_the_shadow_cannot_hold_falls_back_to_the_reference_path() {
    // 8-bpp: the bytes are palette indices, not 565 words, so comparing them as words would be
    // comparing two different pictures for equality. The fallback converts and reports the whole
    // screen, which is what a consumer has to upload anyway.
    let mut f = Fixture::new(16, 8, 8);
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert_eq!(screen.bands(), &[(0, 8)]);
    assert_eq!(screen.pixels(), f.screenshot().as_slice());

    // And it stays correct on the next present, rather than comparing against a stale shadow.
    f.fill_row(0, (255, 255, 255));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert_eq!(screen.pixels(), f.screenshot().as_slice());
}

#[test]
fn a_resized_buffer_is_never_reported_as_nothing_moved() {
    // The one failure this guards is silent: a buffer that grew while `bands` stayed empty would be
    // uploaded as "same frame" and the new area would stay the colour of the allocation.
    let mut f = Fixture::new(4, 4, 16);
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert_eq!(screen.pixels().len(), 4 * 4 * 3);

    let mut taller = Fixture::new(4, 9, 16);
    taller.fill_row(8, (255, 255, 255));
    screen.refresh(&taller.display, &taller.mem).unwrap();
    assert_eq!(screen.size(), (4, 9));
    assert_eq!(screen.bands(), &[(0, 9)]);
    assert_eq!(
        screen.bands(),
        &[(0, 9)],
        "nor may the bands stop short of the rows that did not exist before"
    );
}

/// The oracle in the other direction: `RGB565` words read back as `R G B`.
///
/// Every 565 test below compares against this or against the reference capture, never against the
/// screen's own shadow — a conversion checked against its own input proves nothing.
fn unpacked(words: &[u8]) -> Vec<u8> {
    words
        .chunks_exact(2)
        .flat_map(|px| {
            let (r, g, b) = unpack565(u16::from_le_bytes([px[0], px[1]]));
            [r, g, b]
        })
        .collect()
}

#[test]
fn a_565_screen_hands_over_the_guest_words_themselves() {
    let mut f = Fixture::new(24, 16, 16);
    for y in 0..16 {
        f.fill_row(y, (y as u8 * 15, 255 - y as u8 * 15, (y * 7) as u8));
    }
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::with_format(Format::Rgb565);
    f.refresh(&mut screen);

    assert_eq!(screen.format(), Format::Rgb565);
    assert_eq!(screen.stride(), 24 * 2, "two bytes a pixel, not three");
    assert_eq!(screen.pixels().len(), 24 * 16 * 2);
    assert_eq!(
        unpacked(screen.pixels()),
        f.screenshot(),
        "a pass-through has to be the same picture the reference conversion gives, pixel for pixel"
    );
}

#[test]
fn a_565_screen_writes_only_the_rows_that_moved() {
    let mut f = Fixture::new(24, 16, 16);
    for y in 0..16 {
        f.fill_row(y, (0, 0, 255));
    }
    let mut screen = Screen::with_format(Format::Rgb565);
    f.refresh(&mut screen);
    assert_eq!(screen.bands(), &[(0, 16)]);

    // The same marker the RGB888 test uses: a row nobody touched keeps what was put there, which is
    // the only way to prove the copy is per row rather than per frame.
    let stride = 24 * 2;
    screen.pixels[stride * 15] = 0xAB;
    f.fill_row(5, (255, 0, 0));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);

    assert_eq!(screen.bands(), &[(5, 6)]);
    assert_eq!(
        &screen.pixels()[stride * 5..stride * 5 + 2],
        &pack565(255, 0, 0).to_le_bytes(),
        "the row that moved is red, in the guest's own word"
    );
    assert_eq!(
        screen.pixels()[stride * 15],
        0xAB,
        "the untouched row was left alone — the marker survived"
    );
}

#[test]
fn an_indexed_screen_still_reaches_a_565_consumer() {
    // 8-bpp cannot be passed through — the bytes are indices, not 5-6-5 words — so this is the one
    // case where a 565 screen converts, and it has to land in the consumer's format all the same.
    let mut f = Fixture::new(16, 8, 8);
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::with_format(Format::Rgb565);
    f.refresh(&mut screen);
    assert_eq!(
        screen.bands(),
        &[(0, 8)],
        "a full conversion is a full screen"
    );
    assert_eq!(screen.stride(), 16 * 2);
    assert_eq!(unpacked(screen.pixels()), f.screenshot());

    // And it stays right on the next present, rather than keeping a half-565 shadow.
    f.fill_row(0, (255, 255, 255));
    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert_eq!(unpacked(screen.pixels()), f.screenshot());
}

#[test]
fn an_indexed_screen_shows_the_colours_its_palette_names() {
    // The screen of an 8-bpp device is whatever `IDIB::pRGB` says it is. A capture that read the
    // bytes as 5-6-5 words showed pairs of neighbouring indices as a red, a green and a blue instead,
    // and `Screen` carried that picture through faithfully — a wrong screen, reproduced exactly.
    let mut f = Fixture::new(4, 2, 8);
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let colours = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)];
    let table = f.mem.alloc(colours.len() as u32 * 4, 4).unwrap();
    for (i, (r, g, b)) in colours.iter().enumerate() {
        // `IDIB::pRGB` holds `0x00RRGGBB` — red in bits 16-23 — which is the order `default_palette`
        // publishes and `palette_to_rgbval` reads.
        let word = (u32::from(*r) << 16) | (u32::from(*g) << 8) | u32::from(*b);
        f.mem.write_u32(table + i as u32 * 4, word).unwrap();
    }
    f.mem.write_u32(f.device + at("pRGB"), table).unwrap();
    // `nPitch` and `cntRGB` share one word, pitch in the low half.
    let pitch = f.mem.read_u32(f.device + at("nPitch")).unwrap();
    f.mem
        .write_u32(f.device + at("nPitch"), (pitch & 0xffff) | (4 << 16))
        .unwrap();
    f.display.update(&f.mem, f.present, 0).unwrap();

    // Two rows that are not the same, so a reader that transposed them would disagree.
    let px = f.mem.read_u32(f.device_row()).unwrap();
    for (i, index) in [0u8, 1, 2, 3, 3, 2, 1, 0].into_iter().enumerate() {
        f.mem.write_u8(px + i as u32, index).unwrap();
    }

    let mut want = Vec::new();
    for index in [0usize, 1, 2, 3, 3, 2, 1, 0] {
        let (r, g, b) = colours[index];
        want.extend_from_slice(&[r, g, b]);
    }
    let mut screen = Screen::new();
    f.refresh(&mut screen);
    assert_eq!(screen.pixels(), want.as_slice());
}

#[test]
fn a_565_screen_still_says_when_nothing_moved() {
    let mut f = Fixture::new(24, 16, 16);
    f.fill_row(4, (255, 0, 0));
    let mut screen = Screen::with_format(Format::Rgb565);
    f.refresh(&mut screen);
    assert!(!screen.bands().is_empty());

    f.display.update(&f.mem, f.present, 0).unwrap();
    f.refresh(&mut screen);
    assert!(screen.bands().is_empty(), "presented, but no row moved");
    assert_eq!(screen.epoch(), f.display.presentations);
}

/// A screen's bytes as `R G B` triples, whatever the format says the alpha is.
fn rgb_of(pixels: &[u8], bpp: usize) -> Vec<u8> {
    pixels
        .chunks_exact(bpp)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect()
}

#[test]
fn an_rgba_screen_is_the_reference_capture_with_an_opaque_alpha() {
    let mut f = Fixture::new(24, 16, 16);
    for y in 0..16 {
        f.fill_row(y, (y as u8 * 15, 255 - y as u8 * 15, (y * 7) as u8));
    }
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::with_format(Format::Rgba8888);
    f.refresh(&mut screen);

    assert_eq!(screen.format(), Format::Rgba8888);
    assert_eq!(screen.stride(), 24 * 4, "four bytes a pixel");
    assert_eq!(screen.pixels().len(), 24 * 16 * 4);
    assert_eq!(
        rgb_of(screen.pixels(), 4),
        f.screenshot(),
        "the colour channels are the reference conversion"
    );
    assert!(
        screen.pixels().chunks_exact(4).all(|px| px[3] == 255),
        "a handset screen is opaque, and egui treats premultiplied as straight at 255"
    );
}

#[test]
fn an_indexed_screen_still_reaches_an_rgba_consumer() {
    // The other fallback arm: an indexed bitmap is converted straight into four bytes, without a
    // detour through the three-byte shape nobody asked for.
    let mut f = Fixture::new(16, 8, 8);
    f.display.flush_deferred(&f.mem).unwrap();
    let mut screen = Screen::with_format(Format::Rgba8888);
    f.refresh(&mut screen);
    assert_eq!(screen.bands(), &[(0, 8)]);
    assert_eq!(rgb_of(screen.pixels(), 4), f.screenshot());
    assert!(screen.pixels().chunks_exact(4).all(|px| px[3] == 255));
}

#[test]
fn every_format_describes_the_same_picture() {
    // Three outputs from one screen, compared through the one thing they have in common: the
    // reference conversion. This is the test that would catch a `converter` arm pointing at the wrong
    // function — the failure mode is a swapped channel, which no single-format test can see.
    let mut f = Fixture::new(12, 9, 16);
    for y in 0..9 {
        f.fill_row(y, (y as u8 * 25, 40 + y as u8 * 10, 200 - y as u8 * 20));
    }
    f.display.flush_deferred(&f.mem).unwrap();
    let oracle = f.screenshot();

    let mut words = Screen::with_format(Format::Rgb565);
    f.refresh(&mut words);
    assert_eq!(unpacked(words.pixels()), oracle, "5-6-5");

    let mut bytes = Screen::with_format(Format::Rgb888);
    f.refresh(&mut bytes);
    assert_eq!(bytes.pixels(), oracle.as_slice(), "RGB888");

    let mut wide = Screen::with_format(Format::Rgba8888);
    f.refresh(&mut wide);
    assert_eq!(rgb_of(wide.pixels(), 4), oracle, "RGBA8888");

    assert_eq!(
        (words.stride(), bytes.stride(), wide.stride()),
        (12 * 2, 12 * 3, 12 * 4),
        "and each says how wide a row is, which is what an uploader needs"
    );
}

/// A 32-bpp frame reaches a consumer as colour through the same row-shadow path as 565: the shadow
/// is four bytes a pixel, and only rows that actually moved are converted.
#[test]
fn a_32_bpp_screen_reaches_a_consumer_as_colour() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut display = Display::new_with_layout(&mut mem, &mut hle, 2, 1, 32, 32, 128).unwrap();
    let present = display.create_display(&mut mem, &mut hle).unwrap();
    mem.write_u32(display.device.px, 0x0012_3456).unwrap();
    mem.write_u32(display.device.px + 4, 0x00ab_cdef).unwrap();
    display.update(&mem, present, 0).unwrap();

    let mut screen = Screen::with_format(Format::Rgb888);
    screen.refresh(&display, &mem).unwrap();
    assert_eq!(
        screen.pixels(),
        &[0x12, 0x34, 0x56, 0xab, 0xcd, 0xef][..],
        "the frame's own 888 words, untouched"
    );
    assert_eq!(screen.bands(), &[(0, 1)]);

    // A present of the same picture costs one row compare and no output work.
    display.update(&mem, present, 0).unwrap();
    screen.refresh(&display, &mem).unwrap();
    assert!(screen.bands().is_empty());

    // One changed word produces one band and leaves the neighbour alone.
    mem.write_u32(display.device.px, 0x00ff_0000).unwrap();
    display.update(&mem, present, 0).unwrap();
    screen.refresh(&display, &mem).unwrap();
    assert_eq!(screen.bands(), &[(0, 1)]);
    assert_eq!(screen.pixels(), &[0xff, 0, 0, 0xab, 0xcd, 0xef][..]);

    let mut words = Screen::with_format(Format::Rgb565);
    words.refresh(&display, &mem).unwrap();
    assert_eq!(
        words.pixels(),
        &[
            pack565(0xff, 0, 0).to_le_bytes(),
            pack565(0xab, 0xcd, 0xef).to_le_bytes()
        ]
        .concat()[..]
    );
}
