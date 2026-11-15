// Moved out of `src/display.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::device::make_rgb;
use crate::layout;

#[test]
fn five_six_five_puts_red_in_the_top_five_bits() {
    // The one layout claim in this file that is a choice rather than a header fact, so it is
    // pinned here: R[15:11] G[10:5] B[4:0], i.e. `GL_UNSIGNED_SHORT_5_6_5`.
    assert_eq!(pack565(255, 0, 0), 0xf800);
    assert_eq!(pack565(0, 255, 0), 0x07e0);
    assert_eq!(pack565(0, 0, 255), 0x001f);
    // The low bits of each argument are dropped, not rounded: 8 red is one step, 7 is none.
    assert_eq!(pack565(7, 0, 0), 0);
    assert_eq!(pack565(8, 0, 0), 0x0800);
    // Green has an extra bit, so it is the channel that survives a grey ramp best.
    assert_eq!(pack565(0, 4, 0), 0x0020);
}

#[test]
fn the_two_conversions_are_exact_in_both_directions() {
    assert_eq!(rgb_to_native(RGB_WHITE), 0xffff);
    assert_eq!(rgb_to_native(RGB_BLACK), 0);
    assert_eq!(native_to_rgb(0xffff), RGB_WHITE);
    assert_eq!(native_to_rgb(0), RGB_BLACK);
    // Every packed pixel is its own fixed point, which is what lets a screen dump be decoded
    // without knowing how it was drawn.
    for v in 0..=u16::MAX {
        let (r, g, b) = unpack565(v);
        assert_eq!(pack565(r, g, b), v, "{v:#06x} round trip");
    }
    // And an RGBVAL loses exactly the bits 565 has no room for, no more.
    for c in [0u8, 1, 7, 8, 127, 128, 248, 249, 255] {
        let native = rgb_to_native(make_rgb(c, c, c));
        let back = native_to_rgb(native);
        let (r, _, _) = split_rgb(back);
        assert_eq!(r, unpack565(native as u16).0, "{c} red");
        assert!((r as i32 - c as i32).abs() <= 8, "{c} is not lost: {r}");
    }
}

#[test]
fn a_clip_is_intersected_with_the_bitmap_never_wrapped() {
    let screen = Rect::whole(24, 24);
    // Fully outside is nothing at all, not a rectangle with a negative extent.
    assert_eq!(
        Rect::new(30, 30, 10, 10)
            .clipped(screen)
            .unwrap_or(Rect::EMPTY),
        Rect::EMPTY
    );
    // Straddling the edges clamps to what is actually on the bitmap.
    assert_eq!(
        Rect::new(-6, -6, 12, 12).clipped(screen).unwrap(),
        Rect::new(0, 0, 6, 6)
    );
    assert_eq!(
        Rect::new(20, 20, 40, 40).clipped(screen).unwrap(),
        Rect::new(20, 20, 4, 4)
    );
    // The corners a guest can reach with arbitrary `int`s: the arithmetic is done wide and the
    // result clamped, so no subtract overflows and no `i16` field wraps.
    for (x, y, dx, dy) in [
        (i16::MIN, i16::MIN, i16::MAX, i16::MAX),
        (i16::MAX, i16::MAX, i16::MAX, i16::MAX),
        (i16::MIN, 0, i16::MAX, i16::MIN),
        (0, 0, i16::MIN, i16::MAX),
        (i16::MAX - 1, i16::MAX - 1, 4, 4),
    ] {
        let got = Rect::new(x, y, dx, dy).clipped(screen);
        if let Some(r) = got {
            assert!(r.x >= 0 && r.y >= 0, "{:?} escaped the origin", r);
            assert!(
                i32::from(r.x) + i32::from(r.dx) <= 24 && i32::from(r.y) + i32::from(r.dy) <= 24,
                "{:?} is past the bitmap",
                r
            );
        }
        // The empty rectangle is a hole in the coordinate space, so it clips everything away.
        assert_eq!(Rect::EMPTY.clipped(screen), None);
    }
}

#[test]
fn a_display_backs_every_depth_its_pixels_are_packed_at() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    // The three index depths a pack asks for when it wants a device-independent bitmap to decode
    // into, each of which the pixel accessors know how to pack.
    for depth in [8, 4, 1] {
        let d = Display::new(&mut mem, &mut hle, 24, 24, depth).unwrap();
        let b = d.bitmap(d.device_bitmap()).unwrap();
        assert_eq!(b.depth, depth);
        // A row is counted in bits: 24 pixels of 4 bpp is 12 bytes, not 24.
        assert_eq!(b.pitch, row_bytes(24, depth) as i32);
    }
    // 16 works, and the object it builds is a live `IBitmap*`.
    let d = Display::new(&mut mem, &mut hle, 24, 24, 16).unwrap();
    assert_eq!(d.device.width, 24);
    assert_eq!(d.device.pitch, 48);
    assert_eq!(d.bitmap(d.device_bitmap()).unwrap().depth, NATIVE_DEPTH);
    // A depth nothing here knows how to pack is refused by name rather than drawn as a corrupt
    // screen.
    let err = Display::new(&mut mem, &mut hle, 24, 24, 24).unwrap_err();
    assert!(matches!(err, HleError::Unsupported { .. }), "{}", err);
    assert!(format!("{}", err).contains("24 bpp"), "{}", err);
}

#[test]
fn a_zero_depth_dib_is_an_empty_bitmap_not_a_refusal() {
    // `IDISPLAY_CreateDIBitmap(..., 0, ...)` is legal on a handset: the shipped 3.1.2
    // `DIB_NewEx` rejects only a negative nPixelSize and hands back a DIB with nDepth = 0,
    // nPitch = 0 and no pixels. Six shipped packs in BREW_Collect/BREW ask for exactly that,
    // and refusing it stopped each one at its first off-screen bitmap.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 8, 4, 0).unwrap();
    let b = d.bitmap(obj).unwrap();
    assert_eq!(b.depth, 0, "the object keeps the depth the caller asked for");
    assert_eq!(b.pitch, 0, "and there is no row of pixels to stride over");
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    assert_eq!(mem.read_u32(obj + at("nDepth")).unwrap() & 0xff, 0);
    // The 16-bpp fall-through would read or write two bytes past the object here. Both
    // accessors answer "no pixels" instead, and a write leaves guest memory alone.
    assert_eq!(read_pixel(&mem, &b, 1, 1).unwrap(), Some(0));
    let before = mem.read_u32(b.px).unwrap();
    write_pixel(&mut mem, &b, 1, 1, 0xffff).unwrap();
    assert_eq!(mem.read_u32(b.px).unwrap(), before);
    // A depth this host really cannot pack is still refused by name.
    let err = d.new_bitmap(&mut mem, &mut hle, 8, 4, 24).unwrap_err();
    assert!(matches!(err, HleError::Unsupported { .. }), "{}", err);
}

#[test]
fn a_bitmap_says_where_its_pixels_are_at_the_published_offset() {
    // The struct is private to the handset, but its *prefix* is not: a shipped game takes the
    // pointer `CONVERTBMP` hands back, reads the word at +8 and writes pixels straight through
    // it. An object that ends at the refcount gives that read the next object's first word, and
    // the game then stores into `NULL[40217]`, which is what a null without this reads as.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let b = d.bitmap(obj).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    assert_eq!(
        mem.read_u32(obj + at("pBmp")).unwrap(),
        b.px,
        "+8 is the pixel base, where `IDIB::pBmp` lives"
    );
    // The words the interface itself owns are untouched by the new one.
    assert_ne!(mem.read_u32(obj).unwrap(), 0, "+0 is the vtable");
    assert_eq!(
        mem.read_u32(obj + at("pPaletteMap")).unwrap(),
        0,
        "+4 reads back as no palette map, which is what the header says a null there means"
    );
    assert_eq!(
        mem.read_u32(obj + at("nDepth")).unwrap() & 0xff,
        u32::from(b.depth),
        "the depth a pack asks for is the one it reads back"
    );
    assert_eq!(
        mem.read_u32(obj + at("cx")).unwrap() & 0xffff,
        u32::from(b.width),
        "and so are the dimensions"
    );
    assert_eq!(
        mem.read_u32(obj + hle.object(obj).unwrap().refs_off)
            .unwrap(),
        1,
        "the reference count sits where a pack is told to ignore"
    );
}

/// A pack that composes into a surface of its own says so by writing the device `IDIB`'s `pBmp`,
/// and `IDISPLAY_Update` is the only moment it is obliged to mean it. A presentation that re-reads
/// nothing then counts frames of a buffer nobody owns any more, and the capture keeps reading the
/// bytes the module stopped using. Compare [`Display::sync_bitmap`], which re-reads on demand for
/// every blit — this is the same licence, at the one point the module promises the picture is
/// finished.
#[test]
fn presenting_adopts_the_pixels_the_module_pointed_the_device_at() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let device = d.device_bitmap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");

    // The module's own surface: 8x4, top-down, and one red pixel where (0,0) is.
    let px = mem.alloc(8 * 4 * 2, 4).unwrap();
    mem.write_u16(px, u32::from(pack565(255, 0, 0))).unwrap();
    mem.write_u32(device + at("pBmp"), px).unwrap();

    // Before the present the host still believes its own buffer is the screen, so a capture reads
    // the boot background rather than the module's picture. Without this the test would pass even
    // if `update` re-read nothing.
    let before = d.screenshot(&mem).unwrap();
    assert_ne!(
        (before[0], before[1], before[2]),
        (255, 0, 0),
        "the module's surface must not already be what we read"
    );

    d.update(&mem, disp, 0).unwrap();
    assert_eq!(d.presentations, 1, "and the counter still counts");
    let shot = d.screenshot(&mem).unwrap();
    assert_eq!(
        (shot[0], shot[1], shot[2]),
        (255, 0, 0),
        "the pixel the module pointed at is what a capture now reads"
    );
    assert_eq!(
        d.screenshot(&mem).unwrap().len(),
        8 * 4 * 3,
        "the whole frame still comes out, so the geometry came with it"
    );
}

#[test]
fn a_posted_update_waits_for_the_ui_task_and_is_still_counted() {
    // With `bDefer` TRUE the update message "is posted onto the queue of the
    // UI task, thereby allowing all the drawings to be done before updating the screen" — and the
    // header's own `IDisplay_Update(p)` macro passes that TRUE, so deferring
    // is what an ordinary call means. Two things are pinned: the screen is not taken at the call, and
    // the messages are not lost or merged on the way, because `Presents` is what a report counts.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let device = d.device_bitmap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let px = mem.alloc(8 * 4 * 2, 4).unwrap();
    mem.write_u32(device + at("pBmp"), px).unwrap();
    mem.write_u16(px, u32::from(pack565(255, 0, 0))).unwrap();

    d.update(&mem, disp, 1).unwrap();
    assert_eq!(d.presentations, 0, "a posted update takes no screen yet");

    // "allowing all the drawings to be done": the module keeps drawing after the call, then posts
    // again. The flush that stands for the UI task's turn owes both messages their presentation.
    mem.write_u16(px, u32::from(pack565(0, 255, 0))).unwrap();
    d.update(&mem, disp, 1).unwrap();
    assert_eq!(d.presentations, 0, "and the second is posted too");
    d.flush_deferred(&mem).unwrap();
    assert_eq!(
        d.presentations, 2,
        "two posts are two presentations, not one coalesced frame"
    );
    let shot = d.screenshot(&mem).unwrap();
    assert_eq!(
        (shot[0], shot[1], shot[2]),
        (0, 255, 0),
        "the screen taken is the buffer as the drawing left it"
    );
    d.flush_deferred(&mem).unwrap();
    assert_eq!(
        d.presentations, 2,
        "and an empty queue spends nothing: the mark is cleared, not left to fire again"
    );
}

#[test]
fn an_update_made_while_a_dib_is_the_destination_takes_no_screen() {
    // The first clause of the same sentence: `IDisplay_Update` "updates the screen **if the destination
    // bitmap is the device bitmap**", and the header says plainly that
    // without this call the drawing is not shown. A pack composing into its own surface therefore gets
    // no refresh from a call made while that surface is the destination — and `presentations` is the
    // observable that proves we did not hand it one anyway.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let dib = d.new_bitmap(&mut mem, &mut hle, 4, 2, 16).unwrap();

    d.set_destination(&mut mem, &mut hle, disp, dib).unwrap();
    d.update(&mem, disp, 0).unwrap();
    d.update(&mem, disp, 1).unwrap();
    d.flush_deferred(&mem).unwrap();
    assert_eq!(
        d.presentations, 0,
        "neither the immediate nor the posted form refreshes onto another bitmap"
    );

    // `SetDestination(NULL)` is the device bitmap again, and the very same call then counts.
    d.set_destination(&mut mem, &mut hle, disp, 0).unwrap();
    d.update(&mem, disp, 0).unwrap();
    assert_eq!(
        d.presentations, 1,
        "with the screen as the destination the refresh is owed"
    );
}

#[test]
fn an_app_that_restates_the_dib_owns_what_the_host_draws_with() {
    // The header publishes the geometry as data, and a pack takes that licence: it points `pBmp` at
    // the art it decoded itself, restates `cx`/`cy` for the sprite it unpacked, and sets `nPitch`
    // negative because its BMP is bottom-up. A host that kept the record it made at creation time
    // blitted an empty buffer and drew solid black shapes, which is what this pack did for a frame.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    mem.write_u32(obj + at("pBmp"), layout::HEAP_BASE + 0x5000)
        .unwrap();
    mem.write_u32(obj + at("cx"), 3 | (5 << 16)).unwrap();
    mem.write_u32(obj + at("nPitch"), u32::from((-4i32) as u16))
        .unwrap();
    d.sync_bitmap(&mem, obj).unwrap();
    let b = d.bitmap(obj).unwrap();
    assert_eq!(b.px, layout::HEAP_BASE + 0x5000, "the app's own buffer");
    assert_eq!((b.width, b.height), (3, 5));
    assert_eq!(b.pitch, -4, "a bottom-up bitmap keeps its negative pitch");
}

#[test]
fn the_two_operand_raster_ops_combine_the_packed_words() {
    // The header states each case as an expression over the source and destination buffers,
    // so the arithmetic is on the packed halfword — the five unused low bits of red and the one
    // of blue are inverted along with the colour, which is what the hardware does.
    let (s, d) = (0xf800u32, 0x001fu32);
    assert_eq!(raster(g::AEE_RO_OR, s, d), Some(0xf81f));
    assert_eq!(raster(g::AEE_RO_XOR, s, d), Some(0xf81f));
    assert_eq!(raster(g::AEE_RO_AND, s, d), Some(0));
    assert_eq!(raster(g::AEE_RO_COPY, s, d), Some(0xf800));
    assert_eq!(raster(g::AEE_RO_NOT, s, d), Some(0x07ff));
    // `MERGENOT` is destination OR *not* source, `ANDNOT` is destination AND not source: the two
    // that a pack uses to lay a mask down and to punch a hole through one.
    assert_eq!(raster(g::AEE_RO_MERGENOT, s, d), Some(0x07ff));
    assert_eq!(raster(g::AEE_RO_ANDNOT, s, d), Some(0x001f));
    assert_eq!(raster(g::AEE_RO_ANDNOT, 0x001f, d), Some(0));
    // Every answer is a 565 word, whatever the operands were.
    assert_eq!(raster(g::AEE_RO_MERGENOT, 0, 0), Some(0xffff));
    // Refused rather than guessed, for the two this host cannot carry out.
    assert_eq!(raster(g::AEE_RO_OLDMASK, s, d), None);
    assert_eq!(raster(g::AEE_RO_BLEND, s, d), None);
    assert!(!rop_supported(g::AEE_RO_OLDMASK));
    assert!(!rop_supported(g::AEE_RO_BLEND));
    // One list decides it in both places, which is the point of `rop_supported` being a
    // projection of `raster` rather than a second `matches!`.
    for rop in 0..g::AEE_RO_TOTAL {
        assert_eq!(rop_supported(rop), raster(rop, 0, 0).is_some(), "{rop}");
    }
}

#[test]
fn a_blit_with_andnot_punches_the_source_out_of_the_destination() {
    // The end-to-end case, and the one a pack actually reaches: a blit with raster op 6.
    // A host that only knows COPY would either refuse the whole blit or, worse, paint the source
    // over the destination and call it done.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 4, 16).unwrap();
    let dst = d.new_bitmap(&mut mem, &mut hle, 4, 4, 16).unwrap();
    let src = d.new_bitmap(&mut mem, &mut hle, 4, 4, 16).unwrap();
    let (db, sb) = (d.bitmap(dst).unwrap(), d.bitmap(src).unwrap());
    for i in 0..4 {
        mem.write_u16(db.at(i, 0), 0x07ff).unwrap();
        mem.write_u16(sb.at(i, 0), if i == 1 { 0xf800 } else { 0 })
            .unwrap();
    }
    d.blt_in(&mut mem, dst, 0, 0, 4, 1, src, 0, 0, g::AEE_RO_ANDNOT)
        .unwrap();
    assert_eq!(
        mem.read_u16(db.at(0, 0)).unwrap(),
        0x07ff,
        "src 0 clears nothing"
    );
    assert_eq!(
        mem.read_u16(db.at(1, 0)).unwrap(),
        0x07ff & !0xf800,
        "the source's bits are punched out of the destination"
    );
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn a_four_bit_byte_carries_its_left_pixel_in_the_high_nibble() {
    // Which half of the byte is pixel 0 is not something the header says, so it is pinned against
    // two packs that both draw text this way. Dragon's status bar is a 4 bpp digit sheet: replaying
    // all 1584 of its font blits against a real-device screenshot left 51.5 % of the destination
    // wrong with the halves swapped and zero shape disagreements in this order. Twins' copyright
    // line is the same size of object, and read the other way it came out as "QIUV A-IIESI" where
    // the phone shows "(C)2011  A-onesoft".
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 1, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 2, 1, 4).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let bits = mem.alloc(1, 1).unwrap();
    mem.write_u8(bits, 0x12).unwrap();
    let palette = mem.alloc_zeroed(16 * 4).unwrap();
    // Steps of 16, because the pixel goes through RGB565 on the way and five bits of red cannot
    // tell 1 from 2.
    for i in 0..16u32 {
        mem.write_u32(
            palette + i * 4,
            rgbval_to_palette(make_rgb((i * 16) as u8, 0, 0)),
        )
        .unwrap();
    }
    mem.write_u32(obj + at("pBmp"), bits).unwrap();
    mem.write_u32(obj + at("pRGB"), palette).unwrap();
    mem.write_u32(obj + at("cx"), 2 | (1 << 16)).unwrap();
    mem.write_u32(obj + at("nPitch"), 1 | (16 << 16)).unwrap();
    d.bit_blt(&mut mem, disp, 0, 0, 2, 1, obj, 0, 0, g::AEE_RO_COPY)
        .unwrap();
    let left = mem.read_u16(d.device.at(0, 0)).unwrap();
    let right = mem.read_u16(d.device.at(1, 0)).unwrap();
    assert_eq!(
        left,
        rgb_to_native(make_rgb(16, 0, 0)),
        "the even pixel is the high nibble"
    );
    assert_eq!(
        right,
        rgb_to_native(make_rgb(32, 0, 0)),
        "and its neighbour is the low one"
    );
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn a_palette_word_names_its_colour_the_way_the_header_does() {
    // Which of the two three-byte rotations `IDIB::pRGB` is written in is the difference between
    // a pack's screen and its mirror image, so it is pinned against bytes a shipped pack really
    // puts in memory. Dragon's HUD sheet holds `f1 d2 a1 00` at one index — the word
    // `0x00a1d2f1`. Read as `0x00RRGGBB` that is `(161,210,241)`, and the handset at those 94
    // pixels shows `(164,210,238)`; read as the bytes `R,G,B` it is the warm `(241,210,161)` that
    // leaves 22328 pixels of a scene exchanged. The other reading agrees: 174 of a 255-entry
    // character sheet's colours turn up in its reference photographs this way, and 90 the other.
    assert_eq!(split_rgb(palette_to_rgbval(0x00a1_d2f1)), (161, 210, 241));
    assert_eq!(palette_to_rgbval(0x00a1_d2f1), make_rgb(161, 210, 241));
    // The two directions round-trip, which is what lets the host publish its own default table
    // through the same pair and have a pack read it back unchanged.
    for rgb in [0u32, RGB_BLACK, RGB_WHITE, make_rgb(161, 210, 241)] {
        assert_eq!(palette_to_rgbval(rgbval_to_palette(rgb)), rgb);
    }
}

#[test]
fn a_colour_asked_of_a_palettized_bitmap_is_answered_in_its_own_table() {
    // `IBitmap::RGBToNative` and `IDIB::ncTransparent` live in the bitmap's own pixel space, so for
    // an 8 bpp picture the answer is an index *of that bitmap's palette*. Inverting the host's fixed
    // 3-3-2 ramp is a different question, and a pack asks this one: it queries a sprite sheet once,
    // stores the answer in `ncTransparent`, and tests every pixel against it in guest code before
    // writing. The ramp answers magenta as 227 while the sheet's own table names it 3, the test
    // never fired, and the mask colour went down as a solid block over the title art and the
    // overworld. Dragon's title screen failed the same way, one palette entry at a time.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 6, 6, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 4, 4, 8).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let palette = mem.alloc_zeroed(5 * 4).unwrap();
    let table = [
        make_rgb(0, 0, 0),
        make_rgb(241, 184, 104),
        make_rgb(20, 30, 40),
        // A table can hold a colour one step off the key it means, in either byte order, and both
        // pack to the same RGB565 word `0xf81f`. A search that compares quantised values then ties
        // on whichever it walks first and answers the wrong index — which is why that sprite kept
        // a magenta rectangle around it after the ramp was fixed. The table is bytes, so the match
        // is made in bytes.
        make_rgb(254, 0, 255),
        crate::device::RGB_MASK_COLOR,
    ];
    for (i, rgb) in table.iter().enumerate() {
        mem.write_u32(palette + i as u32 * 4, rgbval_to_palette(*rgb))
            .unwrap();
    }
    mem.write_u32(obj + at("pRGB"), palette).unwrap();
    mem.write_u32(obj + at("nPitch"), 4 | (5 << 16)).unwrap();
    d.sync_bitmap(&mem, obj).unwrap();
    assert_eq!(
        d.pixel_value(&mem, obj, crate::device::RGB_MASK_COLOR)
            .unwrap(),
        4,
        "the index this bitmap's own table names exactly, not the ramp's and not a 565 tie"
    );
    assert_eq!(
        d.pixel_rgb(&mem, obj, 4).unwrap(),
        crate::device::RGB_MASK_COLOR,
        "and the two directions agree"
    );
    assert_eq!(
        d.pixel_value(&mem, obj, make_rgb(254, 0, 255)).unwrap(),
        3,
        "the near-miss is its own index, so the two are never confused"
    );
    // A colour the table does not hold is still answered inside it: the nearest entry, never an
    // index past the end of the array the caller then subscript.
    assert_eq!(
        d.pixel_value(&mem, obj, make_rgb(240, 180, 100)).unwrap(),
        1
    );
    assert_eq!(
        d.pixel_value(&mem, obj, make_rgb(6, 4, 6)).unwrap(),
        0,
        "a near-black asked of a table that holds black lands on black, not on the ramp"
    );
    // A bitmap that never got its own colours keeps the default mapping, which is every other pack
    // in the corpus. Asserted as the round trip rather than the index, because what the host owes
    // the caller is the entry that *means* the colour asked for.
    let plain = d.new_bitmap(&mut mem, &mut hle, 4, 4, 8).unwrap();
    let answered = d
        .pixel_value(&mem, plain, crate::device::RGB_MASK_COLOR)
        .unwrap();
    assert_eq!(
        d.pixel_rgb(&mem, plain, answered).unwrap(),
        crate::device::RGB_MASK_COLOR
    );
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn a_compatible_bitmap_carries_the_palette_it_was_asked_from() {
    // The delivered body builds the new surface at the source's depth and palette size, and
    // the palette entries are copied across before anything is drawn into it. The same body stores
    // the transparency colour it was handed; re-deriving it through the palette answers a
    // *different* index for a palettized bitmap, so the public field is what pins the reading.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 6, 6, 16).unwrap();
    let src = d.new_bitmap(&mut mem, &mut hle, 4, 4, 8).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let palette = mem.alloc_zeroed(4 * 4).unwrap();
    let table = [
        make_rgb(1, 2, 3),
        make_rgb(4, 5, 6),
        make_rgb(7, 8, 9),
        crate::device::RGB_MASK_COLOR,
    ];
    for (i, rgb) in table.iter().enumerate() {
        mem.write_u32(palette + i as u32 * 4, rgbval_to_palette(*rgb))
            .unwrap();
    }
    mem.write_u32(src + at("pRGB"), palette).unwrap();
    mem.write_u32(src + at("nPitch"), 4 | (4 << 16)).unwrap();
    d.sync_bitmap(&mem, src).unwrap();

    let copy = d.create_compatible(&mut mem, &mut hle, src, 2, 2).unwrap();
    let made = d.bitmap(copy).unwrap();
    assert_eq!(made.depth, 8, "the source's depth");
    assert_eq!(
        made.cnt_rgb, 4,
        "the source's palette size, not the 256 this host seeds by default"
    );
    for i in 0..4u32 {
        assert_eq!(
            mem.read_u32(made.rgb + i * 4).unwrap(),
            mem.read_u32(palette + i * 4).unwrap(),
            "entry {i}"
        );
    }

    // A key that is an index stays that index in the public field.
    assert_eq!(d.transparency(&mut mem, src, 2).unwrap(), g::AEE_SUCCESS);
    assert_eq!(mem.read_u32(src + at("ncTransparent")).unwrap(), 2);
    let out = mem.alloc_zeroed(4).unwrap();
    assert_eq!(
        d.get_transparency(&mut mem, src, out).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(mem.read_u32(out).unwrap(), 2);
}

#[test]
fn a_decoded_eight_bit_atlas_blits_in_its_own_colours() {
    // How a pack builds the frame around its play field: an 8 bpp `IDIB` it asks the host for,
    // then points at a buffer it unzipped out of a BMP. The file is bottom-up, so `pBmp` names
    // the *last* row in memory and `nPitch` goes negative to match, and the five colours come
    // straight from the file's own table rather than the host's default one. A tile of that
    // atlas is what paints the screen's border, so a host that read any of this geometry from
    // its own creation-time record would draw the tile's black interior and lose the frame.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 6, 6, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 4, 4, 8).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");

    let bits = mem.alloc_zeroed(16).unwrap();
    // Image row 0 sits at `pBmp`, and every row below it at a *lower* address, which is what a
    // negative pitch buys. The top row is the frame's colour and the rows under it are the
    // panel's black interior.
    for (row, byte) in [(0u32, 1u8), (1, 4), (2, 4), (3, 4)] {
        let row_at = bits + (4 - 1 - row) * 4;
        for x in 0..4 {
            mem.write_u8(row_at + x, byte).unwrap();
        }
    }
    let palette = mem.alloc_zeroed(20).unwrap();
    // Stored the way `IDIB::pRGB` holds a colour — the bytes R, G, B and a pad — rather than as
    // the `RGBVAL` word the header's `uint32*` might suggest.
    let wanted = [
        make_rgb(196, 5, 21),
        make_rgb(247, 22, 40),
        make_rgb(205, 174, 117),
        make_rgb(148, 107, 71),
        make_rgb(0, 0, 0),
    ];
    for (i, rgb) in wanted.iter().enumerate() {
        mem.write_u32(palette + i as u32 * 4, rgbval_to_palette(*rgb))
            .unwrap();
    }
    mem.write_u32(obj + at("pBmp"), bits + 12).unwrap();
    mem.write_u32(obj + at("pRGB"), palette).unwrap();
    mem.write_u32(obj + at("ncTransparent"), 4).unwrap();
    mem.write_u32(obj + at("cx"), 4 | (4 << 16)).unwrap();
    mem.write_u32(obj + at("nPitch"), u32::from((-4i32) as u16) | (5 << 16))
        .unwrap();

    d.bit_blt(&mut mem, disp, 0, 0, 4, 4, obj, 0, 0, g::AEE_RO_COPY)
        .unwrap();
    assert_eq!(
        mem.read_u16(d.device.at(0, 0)).unwrap(),
        rgb_to_native(wanted[1]),
        "the top row is the atlas's own red, not the black the host allocated"
    );
    assert_eq!(
        mem.read_u16(d.device.at(3, 0)).unwrap(),
        rgb_to_native(wanted[1])
    );
    assert_eq!(
        mem.read_u16(d.device.at(0, 1)).unwrap(),
        rgb_to_native(wanted[4]),
        "and the interior row under it is still the file's index 4"
    );
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn a_bitmap_whose_size_would_wrap_is_refused_rather_than_allocated_short() {
    // Both dimensions are the guest's, and `stride * height` in `u32` wraps for the largest pair
    // it can be handed. A wrapped product is a small buffer that every later walk treats as the
    // whole picture, so this has to be an answer rather than a corruption that surfaces as
    // somebody else's fault a hundred thousand instructions on.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 6, 6, 16).unwrap();
    let err = d
        .new_bitmap(&mut mem, &mut hle, 65535, 65535, 16)
        .expect_err("131070 * 65535 does not fit a u32");
    assert!(
        matches!(&err, HleError::Unsupported { what } if what.contains("65535x65535")),
        "named by its own size, not as a bare failure: {err}"
    );
    // And a size that fits is still served, so this is a bound and not a refusal of the feature.
    assert!(d.new_bitmap(&mut mem, &mut hle, 4000, 4000, 16).is_ok());
}

#[test]
fn a_released_bitmap_hands_its_pixels_and_its_palette_back() {
    // The pixels live inside the object block now — `DIB_NewEx` allocates `pixel_bytes + 0x6B`, so
    // the rows sit behind the `IDIB` fields. Releasing a surface therefore has to return that
    // block, or a pack that builds one per frame (UnlimitedWar) walks the object arena at frame
    // rate. The palette is still a separate heap allocation and goes back with it.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 8, 16).unwrap();
    // 8 bpp is the depth with a palette, so the second half of this test has something to observe.
    let b = d.new_bitmap(&mut mem, &mut hle, 16, 16, 8).unwrap();
    let spare_before = mem.heap_spare();
    d.forget(&mut mem, &mut hle, b).unwrap();
    assert!(
        mem.heap_spare() > spare_before,
        "the palette came back: {} spare against {spare_before}",
        mem.heap_spare()
    );
    let again = d.new_bitmap(&mut mem, &mut hle, 16, 16, 8).unwrap();
    assert_eq!(
        again, b,
        "the released object block came back for the next surface"
    );
}

#[test]
fn a_note_is_still_said_once_after_the_list_has_bounded() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 8, 16).unwrap();
    for i in 0..Display::NOTES * 2 {
        d.note(format!("a rect at {i:#x}"));
    }
    assert!(
        d.quiet.len() <= Display::NOTES,
        "the list stopped growing: {}",
        d.quiet.len()
    );
    assert!(d.notes_dropped > 0, "and the report can say it did");
    d.note("the same thing twice");
    d.note("the same thing twice");
    assert_eq!(
        d.quiet
            .iter()
            .filter(|n| **n == "the same thing twice")
            .count(),
        1,
        "dedup is by content, not by position in the list"
    );
}

#[test]
fn blt_in_reads_the_source_the_app_restated() {
    // `IBitmap::BltIn` is called *on* the destination, so that side's record is the one the
    // call's own `this` gets re-read for; the source is only a number in an argument register.
    // Twins' sprite pipeline is `BltIn` out of DIBs whose `pBmp` it rewrote after asking for
    // them, so a source taken on faith copies the empty buffer the host allocated at creation
    // and the pack draws a black sprite while every call says it succeeded.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 6, 6, 16).unwrap();
    let dst = d.new_bitmap(&mut mem, &mut hle, 4, 4, 16).unwrap();
    let src = d.new_bitmap(&mut mem, &mut hle, 4, 4, 16).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    let bits = mem.alloc_zeroed(8).unwrap();
    for x in 0..4 {
        mem.write_u16(bits + x * 2, 0xf800).unwrap();
    }
    mem.write_u32(src + at("pBmp"), bits).unwrap();
    mem.write_u32(src + at("nPitch"), 8).unwrap();
    d.blt_in(&mut mem, dst, 0, 0, 4, 1, src, 0, 0, g::AEE_RO_COPY)
        .unwrap();
    let b = d.bitmap(dst).unwrap();
    assert_eq!(
        mem.read_u16(b.at(0, 0)).unwrap(),
        0xf800,
        "the pixel the app decoded, not the zero the host allocated"
    );
    assert_eq!(mem.read_u16(b.at(3, 0)).unwrap(), 0xf800);
}

/// Nine distinct 565 words, spaced so that any two differ in all three channels and none of them
/// is the black the framebuffer starts as. `Mem` moves pixels as `u32` even when they are two
/// bytes wide, so that is what this returns.
fn shade(i: u32) -> u32 {
    0x0001 + i * 0x0111
}

/// A `dx`-by-`dy` source bitmap whose pixel at `(x, y)` is `shade(y * dx + x)`.
fn shaded_source(d: &mut Display, mem: &mut Mem, hle: &mut Hle, dx: u16, dy: u16) -> HResult<u32> {
    let src = d.new_bitmap(mem, hle, dx, dy, 16)?;
    let b = d.bitmap(src).expect("the bitmap this call just made");
    for y in 0..u32::from(dy) {
        for x in 0..u32::from(dx) {
            mem.write_u16(b.at(x, y), shade(y * u32::from(dx) + x))?;
        }
    }
    Ok(src)
}

#[test]
fn scale_2_doubles_the_rectangle_about_its_own_centre() {
    // `TRANSFORM_SCALE_2` is the flag that made this whole family worth having: a pack that ships
    // one small sprite and magnifies it on screen is asking for four copies of each pixel *and*
    // for the picture to stay where it was pointed, since `xDst`/`yDst` name the untransformed
    // top-left and the header puts the origin at the centre.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 8, 16).unwrap();
    let screen = d.device.obj;
    let src = shaded_source(&mut d, &mut mem, &mut hle, 2, 2).unwrap();
    d.transform_in(
        &mut mem,
        screen,
        3,
        3,
        src,
        0,
        0,
        2,
        2,
        g::TRANSFORM_SCALE_2,
        g::AEE_RO_COPY,
    )
    .unwrap();
    // 2x2 at (3,3) grows to 4x4 at (2,2): the same centre, `2 + 2 == 3 + 1`. Source pixel
    // `(x, y)` is `shade(y * 2 + x)`, and each of them now covers a 2x2 block.
    assert_eq!(
        mem.read_u16(d.device.at(0, 0)).unwrap(),
        0,
        "nothing may be painted outside the grown rectangle"
    );
    assert_eq!(mem.read_u16(d.device.at(6, 6)).unwrap(), 0);
    for (x, y, at) in [
        (2u32, 2u32, 0),
        (3, 3, 0),
        (4, 2, 1),
        (5, 3, 1),
        (2, 4, 2),
        (3, 5, 2),
        (4, 4, 3),
        (5, 5, 3),
    ] {
        assert_eq!(
            mem.read_u16(d.device.at(x, y)).unwrap(),
            shade(at),
            "({x},{y})"
        );
    }
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn scale_half_takes_every_other_source_pixel() {
    // The shrink is the same pair of rules read backwards: half the destination, and the source
    // cursor stepping two pixels per destination pixel instead of holding. Melange advances its
    // source pointer by `den` and never looks at the pixels it skipped, so a downscale drops the
    // odd columns rather than averaging them away.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 8, 16).unwrap();
    let screen = d.device.obj;
    let src = shaded_source(&mut d, &mut mem, &mut hle, 4, 4).unwrap();
    d.transform_in(
        &mut mem,
        screen,
        0,
        0,
        src,
        0,
        0,
        4,
        4,
        g::TRANSFORM_SCALE_HALF,
        g::AEE_RO_COPY,
    )
    .unwrap();
    // 4x4 at (0,0) shrinks to 2x2 at (1,1) — centred, so the top-left pixel it keeps is (0,0)'s.
    assert_eq!(mem.read_u16(d.device.at(0, 0)).unwrap(), 0);
    assert_eq!(mem.read_u16(d.device.at(3, 3)).unwrap(), 0);
    assert_eq!(mem.read_u16(d.device.at(1, 1)).unwrap(), shade(0));
    assert_eq!(
        mem.read_u16(d.device.at(2, 1)).unwrap(),
        shade(2),
        "the second source column, not the first one under it"
    );
    assert_eq!(mem.read_u16(d.device.at(1, 2)).unwrap(), shade(8));
    assert_eq!(mem.read_u16(d.device.at(2, 2)).unwrap(), shade(10));
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn an_odd_extent_at_4x_lands_where_the_oem_arithmetic_puts_it() {
    // A 3-wide source has no exact centre, and the reference routine's two directions round
    // differently: going up it slides by `(dx/2) * (num-1)` on the floored half, so 3 wide at 4X
    // moves three pixels left, not the four or five a tidy `(new - old)/2` would give. Pinning the
    // off-centre case is the point — a pack that placed magnified art by eye on a handset was
    // placed against this number.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 12, 12, 16).unwrap();
    let screen = d.device.obj;
    let src = shaded_source(&mut d, &mut mem, &mut hle, 3, 3).unwrap();
    d.transform_in(
        &mut mem,
        screen,
        4,
        4,
        src,
        0,
        0,
        3,
        3,
        g::TRANSFORM_SCALE_4,
        g::AEE_RO_COPY,
    )
    .unwrap();
    assert_eq!(
        mem.read_u16(d.device.at(0, 0)).unwrap(),
        0,
        "the first column is still clear: the rectangle starts at x=1, not x=-1"
    );
    assert_eq!(mem.read_u16(d.device.at(1, 1)).unwrap(), shade(0));
    assert_eq!(
        mem.read_u16(d.device.at(4, 1)).unwrap(),
        shade(0),
        "one source pixel covers four destination pixels"
    );
    assert_eq!(mem.read_u16(d.device.at(5, 1)).unwrap(), shade(1));
    assert_eq!(d.quiet.len(), 0, "with nothing to explain: {:?}", d.quiet);
}

#[test]
fn a_shrink_to_nothing_says_so() {
    // The one way this family can paint nothing at all: 4 source pixels at one eighth is a
    // zero-wide rectangle, and a silent no-op is what this file keeps having to rule out.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 8, 16).unwrap();
    let screen = d.device.obj;
    let src = shaded_source(&mut d, &mut mem, &mut hle, 4, 4).unwrap();
    d.transform_in(
        &mut mem,
        screen,
        0,
        0,
        src,
        0,
        0,
        4,
        4,
        g::TRANSFORM_SCALE_EIGHTH,
        g::AEE_RO_COPY,
    )
    .unwrap();
    assert_eq!(mem.read_u16(d.device.at(0, 0)).unwrap(), 0);
    assert_eq!(d.quiet.len(), 1, "{:?}", d.quiet);
}

/// A coverage map is sampled only while it still matches the picture it was made for.
///
/// `Display::sync_bitmap` re-reads `cx`/`cy` from guest memory before every blit, because shipped
/// packs do restate their own `IDIB` headers. A map indexed by the width it was built for would then
/// put row `y` of one picture on row `y` of another, and the pixels that lost coverage would be the
/// ones nothing painted — the unreadable kind of wrong. Dropping the map is the answer that cannot
/// do that, and it is said out loud so "this file has no transparency" and "the host stopped trusting
/// it" are not the same sight. Dropping it for one blit does not destroy it: the next ask with the
/// width the map really has is answered from the map again.
#[test]
fn a_restated_bitmap_width_drops_the_coverage_map_and_says_so() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 8, 4, 16).unwrap();
    d.set_alpha(obj, AlphaMap::new(vec![0u8; 8 * 4], 8))
        .unwrap();
    assert!(
        d.picture_alpha(obj, 8).is_some(),
        "matched width: the map is used"
    );
    assert!(
        d.picture_alpha(obj, 9).is_none(),
        "restated width: the map is dropped rather than mis-sampled"
    );
    assert!(
        d.quiet
            .iter()
            .any(|n| n.contains("ignoring the transparency")),
        "and the drop is said: {:?}",
        d.quiet
    );
    assert!(
        d.picture_alpha(obj, 8).is_some(),
        "the map survives the mismatch; only that blit ignored it"
    );
}

/// The decision the batched row path is priced on: full coverage everywhere is *not* a map.
///
/// A picture can carry a `tRNS` chunk whose every value is 255, or an alpha channel that is all-one —
/// either would hand the blitter a per-pixel lookup for no change in the pixels, and that lookup
/// costs measurably more than the batched row path. One value short of full is enough to keep it,
/// because that value is a pixel the destination shows through.
#[test]
fn a_map_that_names_nothing_transparent_is_not_a_map_at_all() {
    assert!(
        AlphaMap::new(vec![u8::MAX; 8 * 4], 8).is_none(),
        "every pixel fully covered: the picture stays on the row path"
    );
    let one_short = {
        let mut v = vec![u8::MAX; 8 * 4];
        v[17] = u8::MAX - 1;
        AlphaMap::new(v, 8)
    };
    let map = one_short.expect("one partly-covered pixel is a map");
    assert!(map.partial(), "and it is the kind that composites");
    assert_eq!(map.at(1, 2), u8::MAX - 1, "found where the file put it");
    assert_eq!(map.at(0, 0), u8::MAX);
}

/// A rasterized face plots its own cell, and a large designator is a face of its own rather than a
/// magnified one — a BBF cell has no scale.
#[test]
fn a_rasterized_face_plots_its_cell_and_large_is_its_own_size() {
    use crate::makebbf::Cell;
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 64, 64, 16).unwrap();
    let b = d.bitmap(d.device_bitmap()).unwrap();
    let color = b.pixel_value(&mem, 0x00FF00).unwrap();
    let area = (0, 0, 63, 63);

    // Ink in the middle of the cell, so the assertion does not depend on which way the bitmap's
    // rows run.
    let face_at = |size: u16| {
        let side = usize::from(size) * usize::from(size);
        let mut ink = vec![false; side];
        ink[usize::from(size / 2) * usize::from(size) + usize::from(size / 2)] = true;
        let blank = vec![false; side];
        let cells = vec![
            Cell {
                ch: 0x20,
                width: size,
                ink: blank.clone(),
            },
            Cell {
                ch: 0x4E2D,
                width: size,
                ink,
            },
        ];
        let bytes = crate::makebbf::encode(&cells, size, size / 4, 0x80);
        std::sync::Arc::new(crate::bbf::Bbf::parse(&bytes).unwrap())
    };

    let normal = crate::font::Face::Bbf(face_at(crate::systemfont::SIZE));
    d.paint_glyph(
        &mut mem,
        &b,
        &normal,
        0x4E2D,
        (0, 0),
        &mut Ink::colour(color),
        area,
        &mut IndexMemo::new(),
    )
    .unwrap();
    assert_eq!(
        read_pixel(&mem, &b, 8, 8).unwrap(),
        Some(color),
        "the cell's ink, at the origin the face steps from"
    );
    assert_ne!(
        read_pixel(&mem, &b, 9, 9).unwrap(),
        Some(color),
        "one pixel, not a block: this face is 16 pixels and says so"
    );

    let large = crate::font::Face::Bbf(face_at(crate::systemfont::LARGE_SIZE));
    d.paint_glyph(
        &mut mem,
        &b,
        &large,
        0x4E2D,
        (0, 20),
        &mut Ink::colour(color),
        area,
        &mut IndexMemo::new(),
    )
    .unwrap();
    assert_eq!(
        read_pixel(&mem, &b, 16, 36).unwrap(),
        Some(color),
        "the 32-pixel face's own cell is twice as far across"
    );
    assert_ne!(
        read_pixel(&mem, &b, 17, 37).unwrap(),
        Some(color),
        "and still one pixel of ink, because nothing magnified it"
    );
}

/// The row-batched screenshot is byte-identical to the per-pixel oracle for every row geometry the
/// header allows. `pitch` may be negative (bottom-up) or wider than the image, and a row may start
/// close enough to a page end that `Mem::read_into` has to split it.
#[test]
fn row_batched_screenshot_matches_the_per_pixel_path() {
    const BASE: u32 = 0x5000_0000;
    const SPAN: u32 = 0x8000;

    let compare = |wide: u16, high: u16, pitch: i32, px: u32, watched: bool| {
        let mut mem = Mem::new();
        mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
        mem.map(BASE, SPAN);
        // A non-repeating ramp: a reader that swapped byte order, used the wrong stride or read
        // one row twice would not produce the same picture.
        for i in 0..(SPAN / 2) {
            mem.write_u16(BASE + i * 2, (i * 26_519 + 7) & 0xffff)
                .unwrap();
        }
        let mut hle = Hle::new();
        let mut d = Display::new(&mut mem, &mut hle, wide, high, 16).unwrap();
        d.device.px = px;
        d.device.pitch = pitch;
        if watched {
            // Both readers fall back to byte-wise access here, and both must still agree.
            mem.watch_span(px, 2);
        }
        let pixels = d.screenshot_pixels(&mem).unwrap();
        let rows = d.screenshot_rows(&mem).unwrap();
        assert_eq!(
            rows, pixels,
            "{wide}x{high} pitch {pitch} px {px:#x} watched {watched}"
        );
        assert_eq!(
            d.screenshot(&mem).unwrap(),
            pixels,
            "and the public entry point picks the batched path for a native 565 device"
        );
    };

    compare(64, 8, 64 * 2, BASE + 0x2000, false);
    compare(64, 8, 64 * 2 + 32, BASE + 0x2000, false); // pitch wider than the row
    compare(64, 8, -(64 * 2), BASE + 0x3000, false); // bottom-up
    compare(64, 8, 64 * 2, BASE + 0x0ffe, false); // row straddles a page end
    compare(64, 8, 64 * 2, BASE + 0x2000, true); // watchpoint armed
}

/// A device bitmap that is not native RGB565 must not be read as though it were: the public entry
/// point falls back to the per-pixel path rather than fabricating colours from row bytes.
#[test]
fn a_non_native_device_bitmap_keeps_the_per_pixel_path() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    d.device.depth = 8;
    assert_eq!(
        d.screenshot(&mem).unwrap(),
        d.screenshot_pixels(&mem).unwrap()
    );
}

/// Give `d` a palette of `colours` and point its pixels at a fresh buffer, the way a pack that
/// decoded its own art does it: `IDIB::pRGB`, `IDIB::cntRGB` and the bytes, all in guest memory.
///
/// The record is set directly rather than through `Display::sync_bitmap`, which is what a guest does
/// by writing the header — `an_indexed_screen_shows_the_colours_its_palette_names` covers that path
/// (header fields, offsets and the shared `nPitch`/`cntRGB` word included). What this helper is for is
/// the pixel question on its own.
fn with_palette(d: &mut Display, mem: &mut Mem, colours: &[(u8, u8, u8)], pixels: &[u8]) {
    let table = mem.alloc(colours.len() as u32 * 4, 4).unwrap();
    for (i, (r, g, b)) in colours.iter().enumerate() {
        // Written the way this host publishes them (`default_palette` does the same), rather than
        // hand-rolled here: the byte order of `IDIB::pRGB` is the one thing that must not be guessed
        // twice.
        let word = rgbval_to_palette(make_rgb(*r, *g, *b));
        mem.write_u32(table + i as u32 * 4, word).unwrap();
    }
    let bytes = mem.alloc(pixels.len().max(1) as u32, 4).unwrap();
    for (i, &b) in pixels.iter().enumerate() {
        mem.write_u8(bytes + i as u32, b).unwrap();
    }
    d.device.rgb = table;
    d.device.cnt_rgb = colours.len() as u16;
    d.device.px = bytes;
}

#[test]
fn an_indexed_device_screenshot_decodes_the_bitmaps_own_palette() {
    // Before this, a capture of an 8-bpp screen read the bytes as 5-6-5 words: pairs of neighbouring
    // palette *indices* became a red, a green and a blue, so the picture had nothing to do with the
    // screen the blitter drew (which has always asked `read_pixel`, and therefore the palette).
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let colours = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)];
    with_palette(&mut d, &mut mem, &colours, &[0, 1, 2, 3, 0, 1, 2, 3]);

    let mut want = Vec::new();
    for _ in 0..2 {
        for (r, g, b) in colours {
            want.extend_from_slice(&[r, g, b]);
        }
    }
    assert_eq!(
        d.screenshot(&mem).unwrap(),
        want,
        "each pixel is the colour its own entry names, row by row"
    );

    // And the two readers of this bitmap agree pixel for pixel, which is the property that was
    // missing: the capture was one answer and the blit another.
    let shot = d.screenshot(&mem).unwrap();
    for y in 0..2 {
        for x in 0..4 {
            let native = read_pixel(&mem, &d.device, x, y).unwrap().unwrap();
            let (r, g, b) = unpack565(native as u16);
            let at = ((y * 4 + x) * 3) as usize;
            assert_eq!(
                [shot[at], shot[at + 1], shot[at + 2]],
                [r, g, b],
                "at ({x}, {y}) the capture and the blitter say the same thing"
            );
        }
    }
}

#[test]
fn an_indexed_screenshot_reads_the_nibbles_in_the_order_the_blitter_does() {
    // 4-bpp puts two pixels in a byte, high nibble first — the order a 4-bpp font's blits pinned down
    // against a real device. The capture has to read them the same way round, or one of the two is
    // showing every pair of columns swapped.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 4).unwrap();
    let mut colours = [(0u8, 0u8, 0u8); 16];
    for (i, c) in colours.iter_mut().enumerate() {
        *c = (i as u8 * 17, 255 - i as u8 * 17, i as u8 * 3);
    }
    with_palette(&mut d, &mut mem, &colours, &[0x5a]);

    let shot = d.screenshot(&mem).unwrap();
    // Through 5-6-5 and back, because that is what the device is: a palette entry carries eight bits
    // a channel and the screen it lands on has five and six, so the capture shows what a viewer
    // would see rather than the table's unrounded intent.
    let through = |c: (u8, u8, u8)| {
        let (r, g, b) = unpack565(pack565(c.0, c.1, c.2));
        [r, g, b]
    };
    let even = through(colours[5]);
    let odd = through(colours[10]);
    assert_eq!(
        shot,
        [even[0], even[1], even[2], odd[0], odd[1], odd[2]],
        "high nibble is the even pixel"
    );
    assert_ne!(even, odd, "and the two entries are not the same colour");
}

#[test]
fn a_one_bit_screenshot_reads_eight_pixels_a_byte_from_the_top() {
    // The third packing `index_at` owns: eight pixels to a byte, most significant bit first. Nothing
    // in the indexed tests above exercises it, and getting it the other way round would mirror every
    // eight-pixel group.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 1, 1).unwrap();
    with_palette(&mut d, &mut mem, &[(0, 0, 0), (255, 255, 255)], &[0b1000_0001]);

    let mut want = vec![0u8; 8 * 3];
    want[..3].copy_from_slice(&[255, 255, 255]);
    want[21..].copy_from_slice(&[255, 255, 255]);
    assert_eq!(
        d.screenshot(&mem).unwrap(),
        want,
        "bit 7 is the leftmost pixel and only the two ends are set"
    );
}

#[test]
fn a_palette_that_cannot_be_read_fails_the_capture_rather_than_inventing_colours() {
    // The pre-read touches every entry the bitmap declares, so a `pRGB` that does not resolve is an
    // error even when the screen only uses index 0. That is deliberate and it matches the blitter:
    // `read_pixel` hands the same fault to a blit. It is also a **new** failure surface — the
    // transparency key used to be resolved through this table by `sync_bitmap` before every slot, so
    // a broken `pRGB` surfaced on the preceding `Update`; since the key became a stored value
    // (§7.3d of the architecture notes) nothing reads the table until something draws or captures.
    // The alternative — painting the ramp — is a screen nobody asked for, indistinguishable from a
    // working one.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    with_palette(&mut d, &mut mem, &[(8, 8, 8)], &[1, 1]);
    d.device.rgb = layout::HEAP_LIMIT + 0x1000; // nothing is mapped up there
    d.device.cnt_rgb = 256;
    assert!(
        mem.read_u32(d.device.rgb).is_err(),
        "the premise of this test is an address that does not resolve"
    );
    assert!(
        d.screenshot(&mem).is_err(),
        "an address that does not resolve is a fault, not a colour"
    );

    // A bitmap that declares no table never follows the pointer, so the same broken `pRGB` costs it
    // nothing: the ramp is the answer, which is what a bitmap born with no palette gets.
    d.device.cnt_rgb = 0;
    let (r, g, b) = unpack565(index8_to_native(1) as u16);
    assert_eq!(d.screenshot(&mem).unwrap(), [r, g, b, r, g, b]);
}

#[test]
fn an_empty_dib_captures_black_rather_than_reading_its_own_header() {
    // A 0-bpp DIB is legal — `DIB_NewEx` hands one back with `nDepth` and `nPitch` both zero and no
    // pixels — so there is no word at `pBmp` that belongs to it. The capture is black, which is the
    // same answer `read_pixel` gives, rather than whatever the object happens to start with.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let d = Display::new(&mut mem, &mut hle, 4, 2, 0).unwrap();
    // A word that would show up as a loud colour if anything read it as a pixel.
    mem.write_u32(d.device.px, 0xf800_f800).unwrap();
    assert_eq!(d.screenshot(&mem).unwrap(), vec![0u8; 4 * 2 * 3]);
}

#[test]
fn an_index_past_the_end_of_the_palette_takes_the_default_ramp() {
    // `cntRGB` bounds the array a pack wrote. Past it there is no colour to read — only whatever the
    // guest put next — so the index decodes through the fixed 3-3-2 ramp instead, which is also what
    // a bitmap carrying no palette at all gets.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    with_palette(&mut d, &mut mem, &[(8, 8, 8)], &[0, 1]);

    let shot = d.screenshot(&mem).unwrap();
    assert_eq!(
        shot[..3],
        [8, 8, 8],
        "index 0 is the entry the pack wrote (a grey 5-6-5 carries exactly)"
    );
    let (r, g, b) = unpack565(index8_to_native(1) as u16);
    assert_eq!(
        shot[3..],
        [r, g, b],
        "index 1 has no entry, so it is the ramp's green — not the next object's memory"
    );
}

#[test]
fn a_colour_drawn_into_a_palettized_bitmap_lands_on_the_entry_that_names_it() {
    // The write side of the same asymmetry the capture had. A pixel of an 8-bpp bitmap is an index,
    // so a colour being drawn into it has to be looked up in that bitmap's own table — the fixed
    // 3-3-2 ramp answers green as 28 no matter what the table says, and a pack that keys a sprite on
    // a colour then reads back a pixel its own art does not contain.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let colours = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)];
    with_palette(&mut d, &mut mem, &colours, &[0; 8]);

    let green = rgb_to_native(make_rgb(0, 255, 0));
    let b = d.device;
    d.fill(&mut mem, &b, Rect::new(0, 0, 4, 2), green, g::AEE_RO_COPY)
        .unwrap();

    let bytes: Vec<u8> = (0..8).map(|i| mem.read_u8(b.px + i).unwrap()).collect();
    assert_eq!(bytes, [1u8; 8], "the entry that names green, not the ramp's");
    assert_ne!(
        native_to_index8(green),
        1,
        "and the two answers really are different, so this test can fail"
    );
}

/// `IBITMAP_Invalidate` is `FillRect(po, prc, 0, AEE_RO_TRANSPARENT)`, and
/// the shipped OEM bitmap's `FillRect` answers `case AEE_RO_TRANSPARENT: return SUCCESS;` — nothing
/// drawn (the inline `FillRect`, which `OEMBitmap_FillRect`
/// ends by calling). That is what makes an invalidate a way to widen a
/// dirty rectangle rather than a way to paint the rectangle black.
///
/// The colour is never looked at, which `page_edge_palette` makes observable: colour 2 names an
/// unmapped entry, so a call that resolves it faults.
#[test]
fn an_invalidate_paints_nothing_and_never_resolves_the_colour() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);
    // A value the fill would replace if it painted at all.
    for x in 0..u32::from(b.width) {
        mem.write_u8(b.at(x, 0), 0xff).unwrap();
    }
    let prc = mem.alloc(8, 2).unwrap();
    for (i, v) in [0u32, 0, 4, 1].into_iter().enumerate() {
        mem.write_u16(prc + i as u32 * 2, v).unwrap();
    }

    // `IBITMAP_Invalidate(p, r)` as the header defines it: colour 0, transparent.
    assert_eq!(
        d.fill_rect(&mut mem, b.obj, prc, 0, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_SUCCESS,
        "an invalidate succeeds without painting"
    );
    for x in 0..4u32 {
        assert_eq!(
            mem.read_u8(b.at(x, 0)).unwrap(),
            0xff,
            "column {x} is left alone"
        );
    }
    // The colour really is a number the call never reads: 2 names an entry that is not mapped.
    assert_eq!(
        d.fill_rect(&mut mem, b.obj, prc, 2, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_SUCCESS,
        "a transparent fill does not resolve its colour"
    );
    assert!(
        d.fill_rect(&mut mem, b.obj, prc, 2, g::AEE_RO_COPY)
            .is_err(),
        "the trap is real: the same colour under COPY has to be resolved"
    );
}

/// The four `IBitmap` slots that take a colour do not agree about `AEE_RO_TRANSPARENT`, and the
/// shipped source is the witness: `FillRect`, `DrawPixel` and `DrawHScanline` answer `SUCCESS` and
/// paint nothing (the last by delegating to
/// `FillRect`), while `SetPixels` has no arm for it and falls into `default: return EUNSUPPORTED;`.
/// Answering `SUCCESS` there would tell a pack the points were laid down.
#[test]
fn the_colour_slots_disagree_about_transparency_the_way_the_shipped_source_does() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);
    for y in 0..2u32 {
        for x in 0..u32::from(b.width) {
            mem.write_u8(b.at(x, y), 0xff).unwrap();
        }
    }
    let prc = mem.alloc(8, 2).unwrap();
    for (i, v) in [0u32, 0, 4, 1].into_iter().enumerate() {
        mem.write_u16(prc + i as u32 * 2, v).unwrap();
    }
    let points = mem.alloc(8, 4).unwrap();
    for (i, (x, y)) in [(0i16, 0i16), (1, 1)].into_iter().enumerate() {
        mem.write_u16(points + i as u32 * 4, u32::from(x as u16))
            .unwrap();
        mem.write_u16(points + i as u32 * 4 + 2, u32::from(y as u16))
            .unwrap();
    }

    assert_eq!(
        d.fill_rect(&mut mem, b.obj, prc, 2, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_SUCCESS,
        "FillRect: nothing drawn, and the colour is never resolved"
    );
    assert_eq!(
        d.draw_pixel(&mut mem, b.obj, 0, 0, 2, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_SUCCESS,
        "DrawPixel: the same answer"
    );
    assert_eq!(
        d.h_scanline(&mut mem, b.obj, 0, 0, 3, 2, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_SUCCESS,
        "DrawHScanline: it ends in FillRect, so it inherits that answer"
    );
    assert_eq!(
        d.set_pixels(&mut mem, b.obj, 2, points, 2, g::AEE_RO_TRANSPARENT)
            .unwrap(),
        g::AEE_EUNSUPPORTED,
        "SetPixels: no arm for the op, so the switch's default refuses it"
    );
    for y in 0..2u32 {
        for x in 0..u32::from(b.width) {
            assert_eq!(
                mem.read_u8(b.at(x, y)).unwrap(),
                0xff,
                "nothing in this family painted ({x},{y})"
            );
        }
    }

    // The control: the same rectangle under COPY does paint, so "nothing happened" above is a
    // statement about the op rather than about a call that never ran. Colour 0 is the one entry of
    // this palette that is readable.
    assert_eq!(
        d.fill_rect(&mut mem, b.obj, prc, 0, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(
        mem.read_u8(b.at(0, 0)).unwrap(),
        0,
        "COPY lays the same colour down"
    );
}

#[test]
fn a_bitmap_slot_takes_the_stored_value_it_documents() {
    // The header says a `NativeColor` handed to an `IBitmap` slot is *the value stored in the pixel
    // array* — an index here — and `IBitmap_RGBToNative` is the call that produces one. A pack that
    // does the right thing must not have it read back as a 5-6-5 colour and pushed through the table
    // a second time.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours: Vec<(u8, u8, u8)> = (0..8).map(|i| (i * 32, i * 16, 255 - i * 32)).collect();
    with_palette(&mut d, &mut mem, &colours, &[0; 2]);

    let dst = d.device_bitmap();
    // The slots draw through the record `Display::bitmap` hands out, which `with_palette` (like the
    // capture tests) only rewrote on `d.device`; make the two agree about the buffer under test.
    d.bitmaps.insert(dst, d.device);
    let mut rc = [0u8; 8];
    for (i, v) in [0i16, 0, 2, 1].iter().enumerate() {
        rc[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
    }
    let at = mem.alloc(8, 4).unwrap();
    mem.write_bytes(at, &rc).unwrap();
    // `IBitmap::FillRect(po, prc, 7, AEE_RO_COPY)`: the argument is index 7, not a colour.
    let status = d.fill_rect(&mut mem, dst, at, 7, g::AEE_RO_COPY).unwrap();
    assert_eq!(status, g::AEE_SUCCESS);
    let b = d.bitmap(dst).unwrap();
    assert_eq!(
        [mem.read_u8(b.px).unwrap(), mem.read_u8(b.px + 1).unwrap()],
        [7, 7],
        "the value is stored as given"
    );
}

#[test]
fn a_colour_the_table_does_not_hold_takes_the_nearest_entry() {
    // A handset's table is fixed and the colour asked of it is not always in it. The answer has to be
    // an index the caller can subscript, and the closest colour is the one that looks right.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    with_palette(&mut d, &mut mem, &[(255, 0, 0), (0, 0, 255)], &[0, 0]);

    let b = d.device;
    d.fill(&mut mem, &b, Rect::new(0, 0, 2, 1), rgb_to_native(make_rgb(200, 20, 30)), g::AEE_RO_COPY)
        .unwrap();
    assert_eq!(
        [mem.read_u8(b.px).unwrap(), mem.read_u8(b.px + 1).unwrap()],
        [0, 0],
        "a dark red is nearer the red entry than the blue one"
    );
}

#[test]
fn a_drawn_index_survives_a_read_back() {
    // The round trip that has to hold for any of this to mean anything: what was written is what the
    // bitmap says that coordinate holds, and what it holds is the colour that was drawn.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 4, 8).unwrap();
    let colours = [
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 0),
        (0, 255, 255),
        (255, 0, 255),
        (0, 0, 0),
        (255, 255, 255),
    ];
    with_palette(&mut d, &mut mem, &colours, &[0; 16]);

    let b = d.device;
    for (i, &(r, g, b_ch)) in colours.iter().enumerate() {
        let native = rgb_to_native(make_rgb(r, g, b_ch));
        assert_eq!(
            b.stored(&mem, native).unwrap(),
            i as u32,
            "the table names this colour at {i}"
        );
        let (x, y) = ((i % 4) as i32, (i / 4) as i32);
        d.put(&mut mem, &b, x, y, native, g::AEE_RO_COPY, None).unwrap();
        assert_eq!(
            mem.read_u8(b.at(x as u32, y as u32)).unwrap(),
            i as u8,
            "which is what landed"
        );
        assert_eq!(
            read_pixel(&mem, &b, x, y).unwrap().unwrap(),
            native,
            "and reading it back gives the colour drawn"
        );
    }
}

#[test]
fn a_four_bpp_bitmap_keeps_the_index_inside_its_nibble() {
    // A 4-bpp pixel owns half a byte; an index that ran past fifteen would climb into the pixel next
    // door. The table is the authority on how many indices exist, and it never has more than sixteen.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 4).unwrap();
    let colours: Vec<(u8, u8, u8)> = (0..16u8).map(|i| (i * 16, 255 - i * 16, i * 8)).collect();
    with_palette(&mut d, &mut mem, &colours, &[0; 2]);

    let b = d.device;
    d.fill(&mut mem, &b, Rect::new(1, 0, 1, 1), rgb_to_native(make_rgb(colours[12].0, colours[12].1, colours[12].2)), g::AEE_RO_COPY)
        .unwrap();
    let byte = mem.read_u8(b.px).unwrap();
    assert_eq!(byte >> 4, 0, "the pixel at x = 0 is untouched");
    assert_eq!(byte & 0x0f, 12, "and the odd pixel holds index 12");
}


#[test]
fn an_inverted_palettized_rectangle_lands_on_the_entry_nearest_its_complement() {
    // `IDisplay::DrawRect`'s `IDF_RECT_INVERT` against an off-screen DIB: `read_pixel` answers a
    // colour at every depth, so the complement is taken there and the answer comes back through the
    // same table. An `!index` would name `0xfe` of a two-entry table — a subscript past `cntRGB`.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(255, 255, 255), (0, 0, 0), (255, 0, 0)];
    with_palette(&mut d, &mut mem, &colours, &[1, 1]);

    let b = d.device;
    d.invert_rect(&mut mem, &b, Rect::new(0, 0, 2, 1)).unwrap();
    assert_eq!(
        [mem.read_u8(b.px).unwrap(), mem.read_u8(b.px + 1).unwrap()],
        [0, 0],
        "black inverted is white, and white is the entry that names it"
    );
    d.invert_rect(&mut mem, &b, Rect::new(0, 0, 2, 1)).unwrap();
    assert_eq!(
        [mem.read_u8(b.px).unwrap(), mem.read_u8(b.px + 1).unwrap()],
        [1, 1],
        "and inverting again comes back — which a fixed 3-3-2 ramp could not do here"
    );
}

#[test]
fn a_pixel_read_through_the_bitmap_slot_is_the_value_that_was_written() {
    // `IBitmap::GetPixel` promises "the value of the specified pixel",
    // through a `NativeColor*`, and the header defines that as what the pixel array holds.
    // `DrawPixel` was brought onto that side of the line in this same change; this is the other half
    // of the pair. Read as a colour instead, the loop the header describes — `RGBToNative` →
    // `DrawPixel` → `GetPixel` — answers a different number than it wrote on every palettized bitmap.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0)];
    with_palette(&mut d, &mut mem, &colours, &[0, 0]);
    let dst = d.device_bitmap();
    d.bitmaps.insert(dst, d.device);

    // `IBitmap::DrawPixel(po, 1, 0, 3, AEE_RO_COPY)`: index 3, which is yellow here.
    d.draw_pixel(&mut mem, dst, 1, 0, 3, g::AEE_RO_COPY).unwrap();
    let out = mem.alloc(4, 4).unwrap();
    assert_eq!(
        d.get_pixel(&mut mem, dst, 1, 0, out).unwrap(),
        g::AEE_SUCCESS
    );
    let b = d.bitmap(dst).unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), 3, "the index, not the colour it names");
    assert_eq!(
        read_pixel(&mem, &b, 1, 0).unwrap().unwrap(),
        rgb_to_native(make_rgb(255, 255, 0)),
        "while the drawing side still resolves it to that colour"
    );
}

#[test]
fn the_transparency_key_a_pack_writes_is_the_number_it_reads_back() {
    // `SetTransparencyColor` / `GetTransparencyColor` are a read-back pair like `DrawPixel` /
    // `GetPixel`, and both slots are entered through `sync_bitmap`, which re-reads the public field
    // before every one of them. Answering out of the record must therefore not round an index through
    // the palette and back: write 2, read the 5-6-5 word of entry 2 — and for a table whose entries 2
    // and 3 pack to the same word, read 3. The field answers the field, and the record keeps the same
    // number in the same space so that the comparisons of `put` and `source_pixel` are on values.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    // A real `IBitmap` object, because the point of this test is the order the dispatcher uses:
    // `Hle::ibitmap` runs [`Display::sync_bitmap`] before *every* slot, so a pack's set and its get
    // are never two calls in a row on this record — there is a re-read from the header between them.
    let made = d.new_bitmap(&mut mem, &mut hle, 2, 1, 8).unwrap();

    assert_eq!(
        d.transparency(&mut mem, made, 2).unwrap(),
        g::AEE_SUCCESS
    );
    d.sync_bitmap(&mem, made).unwrap();
    let out = mem.alloc(4, 4).unwrap();
    d.get_transparency(&mut mem, made, out).unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), 2, "the index, as written");
    let b = d.bitmap(made).unwrap();
    assert_eq!(
        b.trans, 2,
        "and the record holds the index the field holds, not the colour it names"
    );

    // `CreateCompatibleBitmap` carries the same number across, not one re-derived from the record.
    let copy = d.create_compatible(&mut mem, &mut hle, made, 2, 1).unwrap();
    d.sync_bitmap(&mem, copy).unwrap();
    let out = mem.alloc(4, 4).unwrap();
    d.get_transparency(&mut mem, copy, out).unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), 2, "copied, not rounded");
}

#[test]
fn a_raster_op_on_a_palettized_bitmap_combines_indices() {
    // The header defines these operations on *pixel values* — `SRC .XOR. DST*` — and
    // the header says a bitmap with no palette maps pixel values literally. On an 8-bpp
    // bitmap the value is the index, so `1 ^ 2` is 3. Combining the two colours instead gives
    // magenta, and no entry of this table is magenta: the answer would be whichever of red, blue and
    // white the distance falls to first. That was the whole of the write side before this change —
    // `COPY` exact, everything else approximate, in one function.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(0, 0, 0), (255, 0, 0), (0, 0, 255), (255, 255, 255)];
    // The destination already holds index 1, so the second operand is a real one.
    with_palette(&mut d, &mut mem, &colours, &[1, 0]);
    let b = d.device;

    let blue = rgb_to_native(make_rgb(0, 0, 255));
    d.put(&mut mem, &b, 0, 0, blue, g::AEE_RO_XOR, None)
        .unwrap();
    assert_eq!(mem.read_u8(b.px).unwrap(), 3, "index 1 xor index 2");
    let red = rgb_to_native(make_rgb(255, 0, 0));
    assert_ne!(
        b.stored(&mem, red ^ blue).unwrap(),
        3,
        "and the colour combination cannot land on that entry, so this can fail"
    );
}

#[test]
fn a_one_bit_pixel_complements_inside_its_own_bit() {
    // `AEE_RO_NOT : DST = (!SRC)*`, and a 1-bpp pixel is one bit: the complement
    // of index 1 is index 0, not `0xfffe`. The width the stored value is kept in belongs to
    // `write_pixel`, which clears or sets exactly this pixel's bit and leaves its seven neighbours.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 1, 1).unwrap();
    with_palette(&mut d, &mut mem, &[(0, 0, 0), (255, 255, 255)], &[0b1010_0000]);
    let b = d.device;

    let white = rgb_to_native(make_rgb(255, 255, 255));
    d.put(&mut mem, &b, 0, 0, white, g::AEE_RO_NOT, None).unwrap();
    assert_eq!(
        mem.read_u8(b.px).unwrap(),
        0b0010_0000,
        "the pixel that was set is cleared, and the others are not touched"
    );
    let black = rgb_to_native(make_rgb(0, 0, 0));
    d.put(&mut mem, &b, 0, 0, black, g::AEE_RO_NOT, None).unwrap();
    assert_eq!(
        mem.read_u8(b.px).unwrap(),
        0b1010_0000,
        "and the complement of index 0 sets it again"
    );
}

#[test]
fn a_depth_neither_packed_nor_the_screens_is_not_read_through_the_palette() {
    // `IDIB` is public data a pack may restate, so `nDepth` can arrive as
    // something this host never allocates. `read_pixel` and `write_pixel` both treat anything that is
    // not 8/4/1 as a whole 16-bit word, and this has to agree with them: `depth < NATIVE_DEPTH` as
    // the test would send 9 through 15 through the palette and dress a 16-bit pixel up as an 8-bit
    // index — the same mistake in the other direction, and one no test can see at 16 bpp where the
    // two spaces coincide.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 16).unwrap();
    with_palette(&mut d, &mut mem, &[(255, 0, 0), (0, 255, 0)], &[0; 4]);
    let mut b = d.device;

    let red = rgb_to_native(make_rgb(255, 0, 0));
    assert_eq!(b.stored(&mem, red).unwrap(), red, "the screen keeps the word");
    b.depth = 12;
    assert_eq!(
        b.stored(&mem, red).unwrap(),
        red,
        "and so does a depth the guest restated"
    );
    assert_eq!(b.colour_of_pixel(&mem, red).unwrap(), red, "in both directions");
    b.depth = 8;
    assert_eq!(
        b.stored(&mem, red).unwrap(),
        0,
        "while 8-bpp is the contrast: the same call asks the table"
    );
}

#[test]
fn a_blit_keys_on_the_index_not_on_the_colour_it_names() {
    // The one case where the two spaces give different answers, and the reason the comparison had to
    // move. Pixels "whose values match the transparent color (ncTransparent) are
    // treated as transparent ... and the matching is performed on pixel values, not on the
    // corresponding R-G-B intensities."
    //
    // Entries 2 and 3 of this table name the same blue. A source of `[3, 0]` keyed on 2 therefore has
    // exactly one pixel that must survive — the 3 — and a matcher that resolves both sides to a colour
    // cannot tell it from the key, so it drops it. Nothing about the picture on screen says which
    // happened: the destination keeps whatever it was standing on either way.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(0, 0, 0), (255, 0, 0), (0, 0, 255), (0, 0, 255)];
    // The destination starts on index 1, so a skipped pixel and a copied one are different bytes.
    with_palette(&mut d, &mut mem, &colours, &[1, 1]);
    let dst = d.device;

    let obj = d.new_bitmap(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let table = mem.alloc(colours.len() as u32 * 4, 4).unwrap();
    for (i, (r, g, b)) in colours.iter().enumerate() {
        let word = rgbval_to_palette(make_rgb(*r, *g, *b));
        mem.write_u32(table + i as u32 * 4, word).unwrap();
    }
    let px = mem.alloc(4, 4).unwrap();
    mem.write_u8(px, 3).unwrap();
    mem.write_u8(px + 1, 0).unwrap();
    let mut src = d.bitmap(obj).unwrap();
    src.rgb = table;
    src.cnt_rgb = colours.len() as u16;
    src.px = px;
    src.trans = 2;
    d.bitmaps.insert(obj, src);
    let src = d.bitmap(obj).unwrap();
    assert_eq!(
        src.colour_of(&mem, 2).unwrap(),
        src.colour_of(&mem, 3).unwrap(),
        "the two entries really do name one colour, so the colour comparison cannot separate them"
    );

    d.copy(
        &mut mem,
        &dst,
        0,
        0,
        2,
        1,
        &src,
        0,
        0,
        g::AEE_RO_TRANSPARENT,
        None,
    )
    .unwrap();
    assert_eq!(
        [
            mem.read_u8(dst.px).unwrap(),
            mem.read_u8(dst.px + 1).unwrap()
        ],
        [2, 0],
        "index 3 does not match the key 2, so it is copied — and it arrives as 2, the entry of \
         *this* table that names the same blue, which is what a blit between two palettes has to do"
    );

    // And the key still does its job: told to skip 3, the very same pixels lose the 3 and keep the 0.
    let mut src = d.bitmap(obj).unwrap();
    src.trans = 3;
    d.bitmaps.insert(obj, src);
    let src = d.bitmap(obj).unwrap();
    mem.write_u8(dst.px, 1).unwrap();
    mem.write_u8(dst.px + 1, 1).unwrap();
    d.copy(
        &mut mem,
        &dst,
        0,
        0,
        2,
        1,
        &src,
        0,
        0,
        g::AEE_RO_TRANSPARENT,
        None,
    )
    .unwrap();
    assert_eq!(
        [mem.read_u8(dst.px).unwrap(), mem.read_u8(dst.px + 1).unwrap()],
        [1, 0],
        "now 3 is the key and 0 is not"
    );
}

#[test]
fn a_fresh_bitmap_starts_with_the_key_the_header_names() {
    // "For monochrome and greyscale displays, the transparent color is
    // `RGBToNative(RGB_WHITE)`, and for color displays the default transparent color is
    // `RGBToNative(RGB_MASK_COLOR)`". The `RGBToNative` is what makes this a test rather than a
    // constant: the answer lives in the bitmap's own pixel space, so on a palettized surface it is an
    // index. Before this the record held the colour itself, which on an 8-bpp bitmap is a number no
    // pixel can ever hold — a default key that silently never matched anything.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    for (depth, want) in [(8u16, 227u32), (4, 14), (1, 1), (16, 0xf81f)] {
        let mut d = Display::new(&mut mem, &mut hle, 2, 1, depth).unwrap();
        assert_eq!(d.device.trans, want, "the screen at {depth} bpp");
        let made = d.new_bitmap(&mut mem, &mut hle, 2, 1, depth).unwrap();
        assert_eq!(
            d.bitmap(made).unwrap().trans,
            want,
            "and a bitmap built at {depth} bpp agrees with it"
        );
    }

    // The 1-bpp answer is the one that used to differ between the two constructors, by exactly the
    // threshold: magenta's luma is (255 + 0 + 255) / 4 = 127, and `native_to_index1` cuts at 128. So
    // a 1-bpp surface seeded off the ramp answered black while one seeded off its 16-entry default
    // table answered white — the same depth, two constructors, opposite bits. White is what the
    // header names for a monochrome display, and that is what both now give.
    let magenta = rgb_to_native(crate::device::RGB_MASK_COLOR);
    assert_eq!(native_to_index1(magenta), 0, "one short of the threshold");
    assert_eq!(
        native_to_index1(rgb_to_native(RGB_WHITE)),
        1,
        "which is why 1 bpp takes the monochrome rule and not the colour one"
    );
}

#[test]
fn a_one_colour_pass_never_allocates_the_table() {
    // `IndexMemo` promises that a fill, a line or a glyph run — one colour for the whole pass —
    // never buys the 4 KiB table. No picture can show that, and `slots` is private, so the only
    // place that can look is this module. The reason it matters is `Ctx`: every rectangle on the
    // 16-bpp screen constructs one of these, and an eager `new()` would charge all of them for a
    // table none of them ever ask.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let b = d.bitmap(obj).unwrap();
    let green = rgb_to_native(make_rgb(0, 255, 0));
    let blue = rgb_to_native(make_rgb(0, 0, 255));
    let want_green = b.stored(&mem, green).unwrap();
    let want_blue = b.stored(&mem, blue).unwrap();
    assert_ne!(want_green, want_blue);

    let mut memo = IndexMemo::new();
    assert!(memo.obj.is_none() && memo.first.is_none() && memo.slots.is_none());
    assert_eq!(memo.index(&b, &mem, green).unwrap(), want_green);
    assert_eq!(memo.first, Some((green, want_green as u8)));
    assert!(
        memo.slots.is_none(),
        "one colour lives in `first`, the table is not built"
    );
    assert_eq!(memo.index(&b, &mem, green).unwrap(), want_green);
    assert!(
        memo.slots.is_none(),
        "asking the same colour again does not build it either"
    );

    assert_eq!(memo.index(&b, &mem, blue).unwrap(), want_blue);
    assert!(
        memo.slots.is_some(),
        "the second colour is what pays for the 512 slots"
    );
    assert_eq!(
        memo.first,
        Some((green, want_green as u8)),
        "`first` survives the table, so a two-colour glyph run does not re-search its ink"
    );
    assert_eq!(memo.index(&b, &mem, green).unwrap(), want_green);
    assert_eq!(memo.index(&b, &mem, blue).unwrap(), want_blue);
}

#[test]
fn a_memo_starts_over_when_the_bitmap_changes() {
    // Two palettes can name one colour at two different entries, so an index is an answer about
    // *that* table. A memo handed a second bitmap has to forget the first one's answers; keeping
    // them writes an index that names a different colour and leaves nothing on screen to compare.
    // No single `Ctx` does this today — `st.dest` is fixed for the call — but the type should not be
    // the thing that knows it.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let one = d.new_bitmap(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let two = d.new_bitmap(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let b1 = d.bitmap(one).unwrap();
    let b2 = d.bitmap(two).unwrap();
    assert_ne!(b1.obj, b2.obj);
    let green = rgb_to_native(make_rgb(0, 255, 0));
    let red = rgb_to_native(make_rgb(255, 0, 0));
    let g = b1.stored(&mem, green).unwrap();
    let r = b2.stored(&mem, red).unwrap();
    assert_ne!(g, r);

    // Swap two words in the second palette only: green now names the entry red used to.
    let green_word = mem.read_u32(b2.rgb + g * 4).unwrap();
    let red_word = mem.read_u32(b2.rgb + r * 4).unwrap();
    mem.write_u32(b2.rgb + g * 4, red_word).unwrap();
    mem.write_u32(b2.rgb + r * 4, green_word).unwrap();
    assert_eq!(b2.stored(&mem, green).unwrap(), r);

    let mut memo = IndexMemo::new();
    assert_eq!(memo.index(&b1, &mem, green).unwrap(), g);
    assert_eq!(memo.obj, Some(b1.obj));
    assert_eq!(
        memo.index(&b2, &mem, green).unwrap(),
        r,
        "the second bitmap is a new table, not the first one's answer"
    );
    assert_eq!(memo.obj, Some(b2.obj));
    assert_eq!(memo.index(&b1, &mem, green).unwrap(), g, "and back again");
}

#[test]
fn colliding_slots_still_answer_with_the_right_index() {
    // Direct-mapped means a collision costs a re-search, not a wrong answer. §7.3c already notes
    // that 256 keys into 512 slots collide by the birthday bound; this builds the tightest version
    // — three colours that all hash to the same slot — and checks each is answered from its own
    // entry, including after the slot has been overwritten twice.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 4, 2, 8).unwrap();
    let mut b = d.bitmap(obj).unwrap();

    let mut groups = vec![Vec::new(); IndexMemo::SLOTS];
    for key in 1..=0xffffu32 {
        groups[IndexMemo::slot(key)].push(key);
    }
    let triple = groups
        .iter()
        .find(|g| g.len() >= 3)
        .expect("512 slots and 65535 keys: one slot must hold three");
    let (ka, kb, kc) = (triple[0], triple[1], triple[2]);
    assert_eq!(IndexMemo::slot(ka), IndexMemo::slot(kb));
    assert_eq!(IndexMemo::slot(ka), IndexMemo::slot(kc));

    let table = mem.alloc(12, 4).unwrap();
    for (i, key) in [ka, kb, kc].iter().enumerate() {
        mem.write_u32(table + i as u32 * 4, rgbval_to_palette(native_to_rgb(*key)))
            .unwrap();
    }
    b.rgb = table;
    b.cnt_rgb = 3;
    assert_eq!(b.stored(&mem, ka).unwrap(), 0);
    assert_eq!(b.stored(&mem, kb).unwrap(), 1);
    assert_eq!(b.stored(&mem, kc).unwrap(), 2);

    let mut memo = IndexMemo::new();
    assert_eq!(memo.index(&b, &mem, ka).unwrap(), 0);
    assert_eq!(
        memo.index(&b, &mem, kb).unwrap(),
        1,
        "kb takes over the slot"
    );
    assert_eq!(
        memo.index(&b, &mem, ka).unwrap(),
        0,
        "ka still hits `first`"
    );
    assert_eq!(memo.index(&b, &mem, kc).unwrap(), 2, "kc takes over again");
    assert_eq!(
        memo.index(&b, &mem, kb).unwrap(),
        1,
        "kb is re-searched, and the answer is still right"
    );
    assert_eq!(memo.index(&b, &mem, ka).unwrap(), 0);
}

#[test]
fn a_colour_destination_never_touches_the_memo() {
    // The gate lives in `put`, not at the door: a 16-bpp target has no index to look up, so a memo
    // handed to it must be left exactly as it was — no `obj`, no `first`, and above all no table.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let b = d.bitmap(obj).unwrap();
    let green = rgb_to_native(make_rgb(0, 255, 0));
    let mut memo = IndexMemo::new();
    d.put(&mut mem, &b, 0, 0, green, g::AEE_RO_COPY, Some(&mut memo))
        .unwrap();
    assert_eq!(read_pixel(&mem, &b, 0, 0).unwrap(), Some(green));
    assert!(
        memo.obj.is_none() && memo.first.is_none() && memo.slots.is_none(),
        "no index was looked up, so nothing was recorded or allocated"
    );
}

/// An 8-bpp bitmap whose palette ends exactly at a page boundary. Entries 0 and 1 are readable;
/// entry 2 is on the unmapped page and faults. That turns "did this call resolve a colour it never
/// paints?" into an observable question — the answer is an `Err`, not a counter nobody keeps.
fn page_edge_palette(mem: &mut Mem, d: &mut Display) -> Bitmap {
    let page = 0x0900_0000u32;
    mem.map(page, 8);
    let mut b = d.device;
    b.rgb = page + crate::mem::PAGE_SIZE as u32 - 8;
    b.cnt_rgb = 4;
    d.device = b;
    d.bitmaps.insert(b.obj, b);
    assert!(
        b.colour_of(mem, 0).is_ok(),
        "entry 0 lies inside the mapped page"
    );
    assert!(
        b.colour_of(mem, 2).is_err(),
        "entry 2 crosses into the unmapped page, so resolving it faults"
    );
    b
}

#[test]
fn a_transparent_font_run_never_resolves_its_background() {
    // With `IFONT_DRAW_TRANSPARENT` only the glyph is drawn. `bg` is then a
    // number the call must not interpret at all — on an indexed bitmap interpreting it means reading
    // `pRGB`, so a `bg` that names no mapped entry used to fault a call whose only work was the glyph.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    // One blank glyph: it plots no pixel, so the glyph loop never reaches `put` and the foreground
    // is the only palette entry this call reads. A glyph with ink would walk the whole table in
    // `stored` and fault for an unrelated reason.
    let face = font::Font::of(g::AEE_FONT_NORMAL).unwrap();
    assert!(
        face.rows(u32::from(b' ') as u16).iter().all(|r| *r == 0),
        "a space has no ink"
    );
    let text = mem.alloc(2, 2).unwrap();
    mem.write_u16(text, u32::from(b' ')).unwrap();
    let face = Face::Builtin(face);
    assert_eq!(
        d.draw_text_into(
            &mut mem,
            &face,
            b.obj,
            text,
            1,
            0,
            0,
            0,
            2,
            0,
            g::IFONT_DRAW_TRANSPARENT,
        )
        .unwrap(),
        g::AEE_SUCCESS,
        "bg = 2 names an unmapped entry, and a transparent draw must never look at it"
    );
    // The trap is real: without the flag the background *is* painted, so the same `bg` must fault.
    assert!(
        d.draw_text_into(&mut mem, &face, b.obj, text, 1, 0, 0, 0, 2, 0, 0)
            .is_err(),
        "an opaque draw has to resolve bg, which is the read the transparent one spares"
    );
}

/// A run of blank glyphs plots no pixel, so `fg` is a number the call must not interpret — the rule
/// the background already keeps, applied to the other colour. On an indexed destination interpreting
/// it is a read of `pRGB`, and it used to happen at the call's door, before the glyph loop knew it
/// would draw nothing.
#[test]
fn a_blank_glyph_run_never_resolves_its_foreground() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    let face = font::Font::of(g::AEE_FONT_NORMAL).unwrap();
    assert!(
        face.rows(u32::from(b' ') as u16).iter().all(|r| *r == 0),
        "a space has no ink"
    );
    let blank = mem.alloc(2, 2).unwrap();
    mem.write_u16(blank, u32::from(b' ')).unwrap();
    let face = Face::Builtin(face);
    assert_eq!(
        d.draw_text_into(
            &mut mem,
            &face,
            b.obj,
            blank,
            1,
            0,
            0,
            2,
            0,
            0,
            g::IFONT_DRAW_TRANSPARENT,
        )
        .unwrap(),
        g::AEE_SUCCESS,
        "fg = 2 names an unmapped entry, and a run that plots no pixel must not look at it"
    );
    // The trap is real: the same `fg` with a glyph that has ink is a read the call cannot spare.
    let inked = mem.alloc(2, 2).unwrap();
    mem.write_u16(inked, u32::from(b'A')).unwrap();
    assert!(
        d.draw_text_into(
            &mut mem,
            &face,
            b.obj,
            inked,
            1,
            0,
            0,
            2,
            0,
            0,
            g::IFONT_DRAW_TRANSPARENT,
        )
        .is_err(),
        "a glyph with ink has to resolve fg, which is the read the blank run spares"
    );
}

/// `IFont::DrawText`'s clip is the caller's rectangle alone, and it need not intersect the bitmap —
/// text placed beside the bitmap paints nothing, so the ink must not be read for it either.
#[test]
fn a_text_run_beside_the_bitmap_never_resolves_its_foreground() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    let face = Face::Builtin(font::Font::of(g::AEE_FONT_NORMAL).unwrap());
    let text = mem.alloc(2, 2).unwrap();
    mem.write_u16(text, u32::from(b'A')).unwrap();
    // A clip wide enough to cover the text where it was placed — just not on the bitmap. The glyph
    // has ink; its pixels would land at x = 20.. on a 16-wide bitmap.
    let prc = mem.alloc(8, 4).unwrap();
    for (i, v) in [0u32, 0, 100, 100].into_iter().enumerate() {
        mem.write_u16(prc + i as u32 * 2, v).unwrap();
    }
    assert_eq!(
        d.draw_text_into(
            &mut mem,
            &face,
            b.obj,
            text,
            1,
            20,
            0,
            2,
            0,
            prc,
            g::IFONT_DRAW_TRANSPARENT,
        )
        .unwrap(),
        g::AEE_SUCCESS,
        "fg = 2 names an unmapped entry, and text beside the bitmap paints no pixel"
    );
}

/// The same rule one coordinate over: a run entirely to the right of the bitmap paints nothing —
/// `put` drops every pixel of it — so it must not resolve the colour on the way to being ignored.
#[test]
fn a_scanline_past_the_right_edge_never_reads_the_palette() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    assert_eq!(
        d.h_scanline(&mut mem, b.obj, 0, 200, 260, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a run that starts past the right edge is an empty pixel loop"
    );
    assert!(
        d.h_scanline(&mut mem, b.obj, 0, 0, 0, 2, g::AEE_RO_COPY)
            .is_err(),
        "the trap is real: a run that does paint has to resolve the colour and fault"
    );
}

/// `xMax` is the guest's number and may be any distance past the edge. Looping `xMin..=xMax` meant a
/// legal `u32::MAX` walked four billion coordinates to discard each one; the run is clipped instead.
#[test]
fn a_scanline_clips_its_run_to_the_bitmap() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    // Every entry of that palette names black, so a painted pixel and an untouched one would both
    // read back as zero. Prime the row with a value the run has to replace, which makes "painted to
    // the edge and no further" a question the pixels answer rather than the return code.
    mem.write_u32(b.rgb, 0).unwrap();
    for y in 0..2u32 {
        for x in 0..u32::from(b.width) {
            mem.write_u8(b.at(x, y), 0xff).unwrap();
        }
    }
    assert_eq!(
        d.h_scanline(&mut mem, b.obj, 0, 0, u32::MAX, 0, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a run that starts on the bitmap is painted to its edge and no further"
    );
    for x in 0..u32::from(b.width) {
        assert_eq!(
            read_stored(&mem, &b, x as i32, 0).unwrap(),
            Some(0),
            "column {x} is inside the clip and must be painted"
        );
    }
    assert_eq!(
        read_stored(&mem, &b, 0, 1).unwrap(),
        Some(0xff),
        "the row below is not part of the run and must be left alone"
    );
}

/// A point list whose every entry lands outside the bitmap paints nothing, and `put` drops each of
/// them. The colour is resolved after that is known, not before: on an indexed destination resolving
/// it is a read of `pRGB`, and a call that writes no pixel must not fault on an entry it never uses.
#[test]
fn a_point_list_off_the_bitmap_never_reads_the_palette() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    let points = mem.alloc(8, 4).unwrap();
    for (i, (x, y)) in [(200i16, 200i16), (201, 201)].into_iter().enumerate() {
        mem.write_u16(points + i as u32 * 4, u32::from(x as u16))
            .unwrap();
        mem.write_u16(points + i as u32 * 4 + 2, u32::from(y as u16))
            .unwrap();
    }
    assert_eq!(
        d.set_pixels(&mut mem, b.obj, 2, points, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "every point is off the bitmap, so the colour is never used"
    );
    // The trap is real: move the first point onto the bitmap and the colour is read.
    mem.write_u16(points, 0).unwrap();
    mem.write_u16(points + 2, 0).unwrap();
    assert!(
        d.set_pixels(&mut mem, b.obj, 2, points, 2, g::AEE_RO_COPY)
            .is_err(),
        "one point on the bitmap is a pixel the call does write, so it resolves the colour"
    );
}


#[test]
fn ibitmap_slots_that_paint_nothing_never_read_the_palette() {
    // `fill_rect` gets this from `Rect::clipped`; the two slots here have no rectangle to clip, so
    // their empty cases have to be ruled out at the door. A call that writes no pixel must not turn
    // into a guest-memory fault just because the colour argument names an unmapped palette entry.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 16, 8, 8).unwrap();
    let b = page_edge_palette(&mut mem, &mut d);

    assert_eq!(
        d.h_scanline(&mut mem, b.obj, 99, 0, 3, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a row past the bottom is an empty pixel loop"
    );
    assert_eq!(
        d.h_scanline(&mut mem, b.obj, 0, 3, 0, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "an inverted x range is an empty pixel loop"
    );
    let points = mem.alloc(4, 4).unwrap();
    assert_eq!(
        d.set_pixels(&mut mem, b.obj, 0, points, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "zero points is an empty pixel loop, not a bad pointer"
    );
    let prc = mem.alloc(8, 2).unwrap();
    for (i, v) in [0i16, 0, 0, 8].iter().enumerate() {
        mem.write_u16(prc + i as u32 * 2, *v as u32).unwrap();
    }
    assert_eq!(
        d.fill_rect(&mut mem, b.obj, prc, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a zero-width rectangle is clipped away before the colour is read"
    );
    assert_eq!(
        d.draw_pixel(&mut mem, b.obj, 99, 0, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a single pixel past the right edge is dropped by put"
    );
    assert_eq!(
        d.draw_pixel(&mut mem, b.obj, u32::MAX, 0, 2, g::AEE_RO_COPY)
            .unwrap(),
        g::AEE_SUCCESS,
        "a huge coordinate reaches put as a negative one and is dropped too"
    );
    assert_eq!(
        d.draw_pixel(&mut mem, b.obj, 99, 0, 2, 0xff).unwrap(),
        g::AEE_EUNSUPPORTED,
        "an unsupported op is refused before the out-of-bounds early return"
    );
    assert!(
        d.draw_pixel(&mut mem, b.obj, 0, 0, 2, g::AEE_RO_COPY)
            .is_err(),
        "an in-bounds pixel does have to resolve the colour and fault"
    );
    assert!(
        d.h_scanline(&mut mem, b.obj, 0, 0, 0, 2, g::AEE_RO_COPY)
            .is_err(),
        "the trap is real: a run that does paint a pixel has to resolve the colour and fault"
    );
}

#[test]
fn a_transparent_blit_never_filters_on_the_destinations_key() {
    // The header's `IBitmap_SetTransparencyColor`: the key "is used when this bitmap is the
    // source bitmap of a transparent bit blit operation". So a destination whose own key happens to
    // name a value present in the source must not swallow that pixel. `copy` also turns the write
    // into a plain COPY; leaving that conversion out would reintroduce a second, different filter.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(0, 0, 0), (255, 0, 0), (0, 255, 0), (0, 0, 255)];
    with_palette(&mut d, &mut mem, &colours, &[1, 1]);
    let mut dst = d.device;
    dst.trans = 2;
    d.device = dst;
    d.bitmaps.insert(dst.obj, dst);

    let obj = d.new_bitmap(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let table = mem.alloc(colours.len() as u32 * 4, 4).unwrap();
    for (i, (r, g, b)) in colours.iter().enumerate() {
        let word = rgbval_to_palette(make_rgb(*r, *g, *b));
        mem.write_u32(table + i as u32 * 4, word).unwrap();
    }
    let px = mem.alloc(4, 4).unwrap();
    mem.write_u8(px, 2).unwrap();
    mem.write_u8(px + 1, 3).unwrap();
    let mut src = d.bitmap(obj).unwrap();
    src.rgb = table;
    src.cnt_rgb = colours.len() as u16;
    src.px = px;
    src.trans = 3;
    d.bitmaps.insert(obj, src);
    let src = d.bitmap(obj).unwrap();

    d.copy(
        &mut mem,
        &dst,
        0,
        0,
        2,
        1,
        &src,
        0,
        0,
        g::AEE_RO_TRANSPARENT,
        None,
    )
    .unwrap();
    assert_eq!(
        [
            mem.read_u8(dst.px).unwrap(),
            mem.read_u8(dst.px + 1).unwrap()
        ],
        [2, 1],
        "source value 2 is not the source key 3, so it is copied even though the destination's \
         own key is 2; the keyed pixel leaves the standing 1"
    );
}

#[test]
fn a_transformed_transparent_blit_still_keys_on_the_source() {
    // §7.3d/§7.3f say the transform paths do the same two things as `copy`: keep the caller's op
    // for `source_pixel` (which asks the source's key) and hand the write a plain COPY. No
    // `transform_in` test used `AEE_RO_TRANSPARENT` before this one, so those claims had no witness.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let colours = [(0, 0, 0), (255, 0, 0), (0, 255, 0), (0, 0, 255)];
    with_palette(&mut d, &mut mem, &colours, &[1, 1]);
    let mut dst = d.device;
    dst.trans = 2;
    d.device = dst;
    d.bitmaps.insert(dst.obj, dst);

    let obj = d.new_bitmap(&mut mem, &mut hle, 2, 1, 8).unwrap();
    let table = mem.alloc(colours.len() as u32 * 4, 4).unwrap();
    for (i, (r, g, b)) in colours.iter().enumerate() {
        let word = rgbval_to_palette(make_rgb(*r, *g, *b));
        mem.write_u32(table + i as u32 * 4, word).unwrap();
    }
    let px = mem.alloc(4, 4).unwrap();
    mem.write_u8(px, 2).unwrap();
    mem.write_u8(px + 1, 3).unwrap();
    let mut src = d.bitmap(obj).unwrap();
    src.rgb = table;
    src.cnt_rgb = colours.len() as u16;
    src.px = px;
    src.trans = 3;
    d.bitmaps.insert(obj, src);
    let src = d.bitmap(obj).unwrap();

    // `transform_in` re-reads both IDIB headers through `sync_bitmap`, so the guest-visible fields
    // have to say the same thing as the host records; `cx`/`cy` and `nPitch`/`cntRGB` share words.
    let publish = |mem: &mut Mem, b: &Bitmap| {
        let at = |f: &str| abi::offset_of("IDIB", f).expect("IDIB is in the tables");
        mem.write_u32(b.obj + at("pBmp"), b.px).unwrap();
        mem.write_u32(b.obj + at("pRGB"), b.rgb).unwrap();
        mem.write_u32(
            b.obj + at("cx"),
            u32::from(b.width) | (u32::from(b.height) << 16),
        )
        .unwrap();
        mem.write_u32(
            b.obj + at("nPitch"),
            (b.pitch as u16 as u32) | (u32::from(b.cnt_rgb) << 16),
        )
        .unwrap();
        mem.write_u32(b.obj + at("nDepth"), u32::from(b.depth))
            .unwrap();
        mem.write_u32(b.obj + at("ncTransparent"), b.trans).unwrap();
    };
    publish(&mut mem, &dst);
    publish(&mut mem, &src);

    // No rotation and no scaling: the geometry is not what this test is about.
    d.transform_in(
        &mut mem,
        dst.obj,
        0,
        0,
        obj,
        0,
        0,
        2,
        1,
        0,
        g::AEE_RO_TRANSPARENT,
    )
    .unwrap();
    assert_eq!(
        [
            mem.read_u8(dst.px).unwrap(),
            mem.read_u8(dst.px + 1).unwrap()
        ],
        [2, 1],
        "source value 2 survives the source key 3 even though the destination's own key is 2"
    );
}

/// `ITransform::TransformBltComplex` had one pixel-level test before this — the ABI-level
/// `a_transform_blit_shares_its_sources_centre_with_the_bitmap_it_draws_on` in `src/test/runtime.rs`,
/// which covers an identity matrix and a reflection and reads back one pixel. What was untested is
/// the other half of the matrix: a half turn, and a singular one. The matrix is 8.8 and the mapping
/// is measured from the source rectangle's own centre, so index `i` has to land
/// on `8 - i` — a whole 3×3 read back, not one corner.
#[test]
fn a_complex_transform_copies_pixels_and_turns_them_end_for_end() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 3, 3, 16).unwrap();
    let dst = d.device;
    let obj = d.new_bitmap(&mut mem, &mut hle, 3, 3, 16).unwrap();
    let src = d.bitmap(obj).unwrap();
    // Every pixel a different number, so a sample taken one row or one column off shows up as the
    // wrong number rather than as a plausible one.
    for i in 0..9u16 {
        mem.write_u16(src.px + u32::from(i) * 2, 0x1000 + u32::from(i))
            .unwrap();
    }

    let matrix = mem.alloc(8, 2).unwrap();
    let write_matrix = |mem: &mut Mem, a: i16, b: i16, c: i16, dd: i16| {
        for (field, value) in [("A", a), ("B", b), ("C", c), ("D", dd)] {
            let off = abi::offset_of("AEETransformMatrix", field)
                .expect("AEETransformMatrix is in the tables");
            mem.write_u16(matrix + off as u32, u32::from(value as u16))
                .unwrap();
        }
    };
    let run = |d: &mut Display, mem: &mut Mem| {
        d.transform_complex_in(mem, dst.obj, 0, 0, obj, 0, 0, 3, 3, matrix, g::AEE_RO_COPY)
            .unwrap()
    };

    write_matrix(&mut mem, 0x0100, 0, 0, 0x0100);
    assert_eq!(run(&mut d, &mut mem), g::AEE_SUCCESS);
    for i in 0..9u32 {
        assert_eq!(
            mem.read_u16(dst.px + i * 2).unwrap(),
            0x1000u32 + i,
            "identity: pixel {i} is the source pixel at the same place"
        );
    }

    // Negating both axes is a half turn about the source's centre: index `i` lands on index `8 - i`
    // on a 3x3 rectangle.
    write_matrix(&mut mem, -0x0100, 0, 0, -0x0100);
    assert_eq!(run(&mut d, &mut mem), g::AEE_SUCCESS);
    for i in 0..9u32 {
        assert_eq!(
            mem.read_u16(dst.px + i * 2).unwrap(),
            0x1000u32 + (8 - i),
            "half turn: pixel {i} comes from its opposite corner"
        );
    }

    // A singular matrix maps the rectangle onto a line; the header's own answer is `EUNSUPPORTED`.
    write_matrix(&mut mem, 0, 0, 0, 0);
    assert_eq!(run(&mut d, &mut mem), g::AEE_EUNSUPPORTED);
}

#[test]
fn the_batched_row_path_keys_on_the_source_too() {
    // The 16-bpp row path is a separate implementation of the same skip, and it must answer the
    // same question: source value 0x001f is not the source key, even though the destination's key
    // says it is. Without this, a transparent `BltIn` would change its answer depending on whether
    // the two buffers happen to overlap.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 16).unwrap();
    let mut dst = d.device;
    dst.trans = 0x001f;
    mem.write_u16(dst.px, 0).unwrap();
    mem.write_u16(dst.px + 2, 0x07e0).unwrap();
    d.device = dst;
    d.bitmaps.insert(dst.obj, dst);

    let obj = d.new_bitmap(&mut mem, &mut hle, 2, 1, 16).unwrap();
    let mut src = d.bitmap(obj).unwrap();
    src.trans = 0xf81f;
    mem.write_u16(src.px, 0x001f).unwrap();
    mem.write_u16(src.px + 2, 0xf81f).unwrap();
    d.bitmaps.insert(obj, src);
    let src = d.bitmap(obj).unwrap();

    // Two independently allocated buffers, so `share_memory` is false and the row path is taken.
    assert!(!share_memory(&src, &dst));
    d.copy(
        &mut mem,
        &dst,
        0,
        0,
        2,
        1,
        &src,
        0,
        0,
        g::AEE_RO_TRANSPARENT,
        None,
    )
    .unwrap();
    assert_eq!(
        [
            mem.read_u16(dst.px).unwrap(),
            mem.read_u16(dst.px + 2).unwrap()
        ],
        [0x001f, 0x07e0],
        "the blue pixel is copied, the magenta key is skipped"
    );
}

/// The 16-bpp COPY fill fast path keeps one scratch row on `Display` instead of allocating a
/// `Vec` per call. Two ways that can go wrong: a row that is not rewritten keeps the previous
/// colour, and a row resized down still writes the old, wider length. Both are invisible in a
/// single-call test, which is why this one fills three times into the same bitmap.
#[test]
fn the_reused_fill_row_is_rewritten_for_every_colour_and_width() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 1, 16).unwrap();
    let b = d.device;
    let red = rgb_to_native(make_rgb(255, 0, 0));
    let green = rgb_to_native(make_rgb(0, 255, 0));
    let blue = rgb_to_native(make_rgb(0, 0, 255));
    assert_ne!(red, green, "the control has to be two different words");
    assert_ne!(green, blue, "and the third pass has to be different again");

    let row = |mem: &Mem| -> Vec<u32> {
        (0..8)
            .map(|i| mem.read_u16(b.px + 2 * i).unwrap())
            .collect()
    };
    d.fill(&mut mem, &b, Rect::new(0, 0, 8, 1), red, g::AEE_RO_COPY)
        .unwrap();
    d.fill(&mut mem, &b, Rect::new(0, 0, 8, 1), green, g::AEE_RO_COPY)
        .unwrap();
    assert_eq!(
        row(&mem),
        vec![green as u32; 8],
        "same width: the previous colour must not survive in the reused buffer"
    );

    d.fill(&mut mem, &b, Rect::new(0, 0, 3, 1), blue, g::AEE_RO_COPY)
        .unwrap();
    let after = row(&mem);
    assert_eq!(&after[..3], &[blue as u32; 3], "the narrower row is blue");
    assert_eq!(
        &after[3..],
        &[green as u32; 5],
        "a row resized down must not paint its old width"
    );
}

/// `Rect::read` now takes one `read_into` instead of four `read_u16`s. A rectangle whose eight
/// bytes straddle a page boundary is the case where a bulk read has to keep the same answer as
/// four separate reads; negative and >127 values make byte order observable too.
#[test]
fn a_rect_straddling_a_page_boundary_reads_the_same_four_words() {
    // The fast path indexes a fixed eight-byte buffer by the ABI offsets. If the tables ever lay
    // AEERect out differently, `Rect::read` falls back to four reads and this test says so instead
    // of silently measuring a path that is no longer the hot one.
    assert_eq!(
        [
            abi_offset("AEERect", "x"),
            abi_offset("AEERect", "y"),
            abi_offset("AEERect", "dx"),
            abi_offset("AEERect", "dy")
        ],
        [0, 2, 4, 6],
        "AEERect is four consecutive i16s; the bulk read assumes this layout"
    );
    let mut mem = Mem::new();
    let page = 0x0900_0000u32;
    let page_size = crate::mem::PAGE_SIZE as u32;
    mem.map(page, page_size + 8);
    let at = page + page_size - 4; // four words: two on each page
    let words: [i16; 4] = [-7, 300, -1, 17];
    for (i, w) in words.iter().enumerate() {
        mem.write_u16(at + 2 * i as u32, *w as u16 as u32).unwrap();
    }
    let r = Rect::read(&mem, at).unwrap();
    assert_eq!([r.x, r.y, r.dx, r.dy], words);
}

/// `draw_rect` no longer clones `DisplayState` (two `Vec`s) on every call; it borrows the live
/// record and copies out the `Copy` values it needs. `RGB_NONE` + `CLR_USER_BACKGROUND` is the
/// shape that makes "did it read the live colour table?" observable, and changing the colour
/// between two calls makes a stale snapshot visible too.
#[test]
fn a_draw_rect_reads_the_live_display_colour_table() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let disp = d.create_display(&mut mem, &mut hle).unwrap();
    let b = d.device;
    let red = make_rgb(255, 0, 0);
    let blue = make_rgb(0, 0, 255);
    assert_ne!(red, blue, "the two passes have to be different colours");

    let row = |mem: &Mem| -> Vec<u32> {
        (0..8)
            .map(|i| mem.read_u16(b.px + 2 * i).unwrap())
            .collect()
    };
    let fill = |d: &mut Display, mem: &mut Mem| {
        d.draw_rect(mem, disp, 0, 0, g::RGB_NONE, g::IDF_RECT_FILL)
            .unwrap()
    };

    d.set_color(disp, g::CLR_USER_BACKGROUND, red).unwrap();
    fill(&mut d, &mut mem);
    assert_eq!(row(&mem), vec![rgb_to_native(red) as u32; 8]);

    // A second call after the table changed must see the new colour, not a snapshot taken on
    // the first call. This is the assertion a stale cached `DisplayState` would fail.
    d.set_color(disp, g::CLR_USER_BACKGROUND, blue).unwrap();
    fill(&mut d, &mut mem);
    assert_eq!(row(&mem), vec![rgb_to_native(blue) as u32; 8]);

    // And an unknown display is still refused, even for a flag combination that draws nothing:
    // the state lookup deliberately happens before the early returns.
    assert!(d
        .draw_rect(&mut mem, 0xdead_beef, 0, 0, 0, 0)
        .is_err());
}

/// The display hot path resolves `AEERect` / `IDIB` field offsets once instead of walking the
/// 594-entry `STRUCTS` table on every `DrawRect` and `BitBlt`. The cache must still be *derived*
/// from that table — a regenerated ABI changes the layout without anybody editing this file — and
/// it must be the same instance on every call, which is what actually takes the search out of the
/// frame loop.
#[test]
fn cached_field_offsets_still_come_from_the_abi_tables() {
    let o = |s: &str, f: &str| abi::offset_of(s, f).expect("the field is in the tables");
    let l = field_layout();
    assert_eq!(
        l.rect,
        [
            o("AEERect", "x"),
            o("AEERect", "y"),
            o("AEERect", "dx"),
            o("AEERect", "dy")
        ]
    );
    assert_eq!(l.idib_pbmp, o("IDIB", "pBmp"));
    assert_eq!(l.idib_prgb, o("IDIB", "pRGB"));
    assert_eq!(l.idib_cx, o("IDIB", "cx"));
    assert_eq!(l.idib_pitch, o("IDIB", "nPitch"));
    assert_eq!(l.idib_depth, o("IDIB", "nDepth"));
    assert_eq!(l.idib_trans, o("IDIB", "ncTransparent"));
    // Six distinct words: two fields read from the same offset would silently pass every
    // "the object was read" test while `pRGB` and `pBmp` swapped places.
    assert_ne!(l.idib_pbmp, l.idib_prgb);
    assert_ne!(l.idib_cx, l.idib_pitch);
    assert!(
        std::ptr::eq(field_layout(), field_layout()),
        "one resolved layout, not a fresh table walk per call"
    );
}

/// `sync_bitmap` now reads its six offsets from the cache. Every field gets a distinguishable
/// value here, so a pair of offsets that got swapped cannot hide behind a zero or a shared word.
#[test]
fn a_synced_idib_reads_every_field_at_its_own_offset() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let obj = d.new_bitmap(&mut mem, &mut hle, 8, 4, 16).unwrap();
    let at = |field: &str| abi::offset_of("IDIB", field).expect("IDIB is in the tables");
    mem.write_u32(obj + at("pBmp"), layout::HEAP_BASE + 0x7000)
        .unwrap();
    mem.write_u32(obj + at("pRGB"), layout::HEAP_BASE + 0x7100)
        .unwrap();
    mem.write_u32(obj + at("cx"), 3 | (5 << 16)).unwrap();
    mem.write_u32(obj + at("nPitch"), u32::from((-4i16) as u16) | (7 << 16))
        .unwrap();
    // Only the low byte is nDepth; the neighbours belong to nColorScheme / reserved.
    mem.write_u32(obj + at("nDepth"), 8 | 0xab00).unwrap();
    // Only the low 16 bits are the transparency key.
    mem.write_u32(obj + at("ncTransparent"), 0x1234_5678)
        .unwrap();

    d.sync_bitmap(&mem, obj).unwrap();
    let b = d.bitmap(obj).unwrap();
    assert_eq!(b.px, layout::HEAP_BASE + 0x7000, "pBmp");
    assert_eq!(b.rgb, layout::HEAP_BASE + 0x7100, "pRGB");
    assert_eq!((b.width, b.height), (3, 5), "cx / cy");
    assert_eq!((b.pitch, b.cnt_rgb), (-4, 7), "nPitch / cntRGB");
    assert_eq!(b.depth, 8, "nDepth is one byte of that word");
    assert_eq!(b.trans, 0x5678, "ncTransparent is 16 bits of that word");
}

/// The 16-bpp COPY row path now calls `Mem::copy_bytes` — guest memory to guest memory, page-run
/// to page-run — instead of staging each row in a host `Vec`. A row straddling a page boundary is
/// where a page-wise copy has to agree with the old read_into/write_bytes pair, and an armed
/// watchpoint is where `copy_bytes` takes its staged byte-wise route. Non-repeating pixels make a
/// short copy, a swapped stride or a stale byte visible.
#[test]
fn a_copy_blit_moves_rows_page_to_page_without_a_host_buffer() {
    const BASE: u32 = 0x6000_0000;
    const SPAN: u32 = 0x8000;
    const PAGE_END: u32 = crate::mem::PAGE_SIZE as u32;

    let run = |px_src: u32, px_dst: u32, watched: bool, label: &str| {
        let mut mem = Mem::new();
        mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
        mem.map(BASE, SPAN);
        let mut hle = Hle::new();
        let mut d = Display::new(&mut mem, &mut hle, 8, 3, 16).unwrap();
        let mut dst = d.device;
        dst.px = px_dst;
        d.device = dst;
        d.bitmaps.insert(dst.obj, dst);

        let obj = d.new_bitmap(&mut mem, &mut hle, 8, 3, 16).unwrap();
        let mut src = d.bitmap(obj).unwrap();
        src.px = px_src;
        d.bitmaps.insert(obj, src);
        let src = d.bitmap(obj).unwrap();
        assert!(
            !share_memory(&src, &dst),
            "{label}: the row gate must be open"
        );

        let ramp = |i: u32| (i * 26_519 + 11) & 0xffff;
        for i in 0..24u32 {
            mem.write_u16(px_src + i * 2, ramp(i)).unwrap();
            mem.write_u16(px_dst + i * 2, 0).unwrap();
        }
        if watched {
            mem.watch_span(px_src, 2);
        }
        d.copy(&mut mem, &dst, 0, 0, 8, 3, &src, 0, 0, g::AEE_RO_COPY, None)
            .unwrap();
        for i in 0..24u32 {
            assert_eq!(
                mem.read_u16(px_dst + i * 2).unwrap(),
                ramp(i),
                "{label}: destination pixel {i}"
            );
            assert_eq!(
                mem.read_u16(px_src + i * 2).unwrap(),
                ramp(i),
                "{label}: the source is a copy, not a move"
            );
        }
    };

    run(BASE + 0x1000, BASE + 0x3000, false, "plain");
    run(
        BASE + PAGE_END - 4,
        BASE + 0x3000,
        false,
        "source row crosses a page",
    );
    run(
        BASE + 0x2000,
        BASE + PAGE_END - 4,
        false,
        "destination row crosses a page",
    );
    run(BASE + 0x1000, BASE + 0x3000, true, "watchpoint armed");
}

/// A blit may ask for more pixels than its source bitmap owns. The pixel loop answers that by
/// skipping every coordinate `read_stored` says is outside the picture; the batched row path now
/// answers it by clipping the shown rectangle to the same intersection. Both ops have to leave
/// the rest of the destination *untouched* — painting zeroes there would be a different picture.
#[test]
fn a_blit_larger_than_its_source_paints_only_the_intersection() {
    const BASE: u32 = 0x7000_0000;
    const SPAN: u32 = 0x4000;
    const SENTINEL: u32 = 0xaaaa;
    let run = |rop: u32, key: u32| {
        let mut mem = Mem::new();
        mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
        mem.map(BASE, SPAN);
        let mut hle = Hle::new();
        let mut d = Display::new(&mut mem, &mut hle, 8, 4, 16).unwrap();
        let mut dst = d.device;
        dst.px = BASE + 0x1000;
        d.device = dst;
        d.bitmaps.insert(dst.obj, dst);

        let obj = d.new_bitmap(&mut mem, &mut hle, 3, 2, 16).unwrap();
        let mut src = d.bitmap(obj).unwrap();
        src.px = BASE + 0x2000;
        src.trans = key;
        d.bitmaps.insert(obj, src);
        let src = d.bitmap(obj).unwrap();
        assert!(!share_memory(&src, &dst), "the row gate must be open");

        // The request is 8x4 at (1,1); the source is only 3x2. Only (1,1)..(3,2) may change.
        for i in 0..(8 * 4) {
            mem.write_u16(BASE + 0x1000 + i * 2, SENTINEL).unwrap();
        }
        let source = |i: u32| 0x1000u32 + i;
        for i in 0..6u32 {
            mem.write_u16(BASE + 0x2000 + i * 2, source(i)).unwrap();
        }
        d.copy(&mut mem, &dst, 1, 1, 8, 4, &src, 0, 0, rop, None)
            .unwrap();

        let at = |x: u32, y: u32| mem.read_u16(BASE + 0x1000 + (y * 8 + x) * 2).unwrap();
        for y in 0..4u32 {
            for x in 0..8u32 {
                let inside = (1..=3).contains(&x) && (1..=2).contains(&y);
                let expected = if !inside {
                    SENTINEL
                } else {
                    let i = (y - 1) * 3 + (x - 1);
                    if rop == g::AEE_RO_TRANSPARENT && source(i) == key {
                        SENTINEL // the key keeps what the destination held
                    } else {
                        source(i)
                    }
                };
                assert_eq!(at(x, y), expected, "rop {rop} at ({x},{y})");
            }
        }
        for i in 0..6u32 {
            assert_eq!(
                mem.read_u16(BASE + 0x2000 + i * 2).unwrap(),
                source(i),
                "the source is a copy, not a move"
            );
        }
    };
    run(g::AEE_RO_COPY, 0xffff_ffff);
    run(g::AEE_RO_TRANSPARENT, 0x1001); // the second source pixel is the colour key
}
