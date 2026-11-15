// Moved out of `src/graphics.rs` so that file is implementation only. The `#[path]` attribute on the
// `mod tests;` declaration in it is what keeps this a child module of that file, and that is what
// lets these tests reach its private items with `use super::*`.

use super::*;
use crate::device::make_rgb;
use crate::hle::Hle;
use crate::layout;

/// A four-by-two 8-bpp `IBitmap` with this host's own 256-entry default table, every pixel on index
/// 0, and a `State` whose destination is that bitmap.
///
/// The table is not written by hand: `new_bitmap` already publishes a full `pRGB`, and the colours
/// these tests ask for are ones it names exactly — so the fixture needs no byte order of its own,
/// which is the thing `test/display.rs` goes out of its way not to guess twice either.
fn indexed_dib(mem: &mut Mem, hle: &mut Hle, d: &mut Display) -> (u32, State) {
    let obj = d.new_bitmap(mem, hle, 4, 2, 8).unwrap();
    let b = d.bitmap(obj).unwrap();
    for i in 0..8u32 {
        mem.write_u8(b.px + i, 0).unwrap();
    }
    let mut g = Graphics::default();
    g.add(1, 0, Rect::new(0, 0, 4, 2), Rgba::new(0, 0, 0, 0));
    let mut st = g.get(1).unwrap();
    st.dest = obj;
    (obj, st)
}

#[test]
fn a_shape_draws_the_entry_that_names_its_colour() {
    // Every `IGraphics` shape hands the blitter a *colour* — `SetColor` resolved an `RGBVAL` — and
    // the destination is whatever `SetDestination` last named. On a palettized destination that
    // colour is not what gets stored; the entry that names it is. Nothing about the picture says
    // which question was asked, because a fill of one colour lands on one index either way: what
    // this pins is *which* index.
    //
    // The expected index comes from `IBitmap::RGBToNative`, which is the same question a pack asks
    // before it stores a key — so what is asserted is that drawing agrees with the ABI's own answer
    // rather than with a number this test made up.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let (obj, st) = indexed_dib(&mut mem, &mut hle, &mut d);
    let green = make_rgb(0, 255, 0);
    let want = d.pixel_value(&mem, obj, green).unwrap();
    let b = d.bitmap(obj).unwrap();

    let mut ctx = Ctx::new(&mut mem, &mut d, st);
    ctx.fill(0, 0, 4, 2, rgb_to_native(green));

    let mut got = Vec::new();
    for i in 0..8u32 {
        got.push(mem.read_u8(b.px + i).unwrap());
    }
    assert_eq!(
        got,
        vec![want as u8; 8],
        "the entry `RGBToNative` names, not `colour & 0xff`"
    );
    assert_ne!(want, 0, "and not the entry the pixels started on");
}

#[test]
fn a_second_call_asks_the_palette_again() {
    // The colour-to-index memo lives inside one `Ctx`, and that is the only reason it may exist: a
    // pack rewrites `pRGB` whenever it likes — palette animation is a real thing on this hardware —
    // and nothing during a drawing call can stop it, while nothing *between* two calls protects a
    // memo either.
    //
    // So the same colour, drawn in two calls with the table changed in between, has to land on two
    // different entries. A memo hung anywhere that outlived the call would keep answering the first
    // entry, and every frame of an animated palette would come out in the first frame's colours —
    // no error, no note, and nothing on screen to compare against.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let (obj, st) = indexed_dib(&mut mem, &mut hle, &mut d);
    let green = make_rgb(0, 255, 0);
    let first = d.pixel_value(&mem, obj, green).unwrap();
    let red = d.pixel_value(&mem, obj, make_rgb(255, 0, 0)).unwrap();
    assert_ne!(first, red, "the default table keeps these apart");

    let mut ctx = Ctx::new(&mut mem, &mut d, st);
    ctx.fill(0, 0, 4, 2, rgb_to_native(green));
    let b = d.bitmap(obj).unwrap();
    assert_eq!(mem.read_u8(b.px).unwrap(), first as u8);

    // The pack rewrites the table, which is the licence the header grants: `pRGB` is public data,
    // and the pointer and the count are untouched here — only what they point at. Swapping the two
    // words it already holds makes green the entry red had, without this test inventing a byte order.
    let green_word = mem.read_u32(b.rgb + first * 4).unwrap();
    let red_word = mem.read_u32(b.rgb + red * 4).unwrap();
    mem.write_u32(b.rgb + first * 4, red_word).unwrap();
    mem.write_u32(b.rgb + red * 4, green_word).unwrap();
    let now = d.pixel_value(&mem, obj, green).unwrap();
    assert_eq!(now, red, "green moved to where red was");

    let mut ctx = Ctx::new(&mut mem, &mut d, st);
    ctx.fill(0, 0, 4, 2, rgb_to_native(green));
    assert_eq!(
        mem.read_u8(b.px).unwrap(),
        now as u8,
        "the second call reads the table it was given, not the one before it"
    );
}

#[test]
fn two_colours_through_one_call_each_land_on_their_own_entry() {
    // The other half of a colour-to-index memo: what it is keyed on. A cache that remembered the
    // last answer without remembering the colour it was for would still get a single-colour fill
    // perfectly right, and would paint every interleaved pixel of a two-colour figure in one of
    // them. Interleaving is the shape that tells the two apart — a fill of one colour, then a fill
    // of the other, is indistinguishable from a cache with no key at all.
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 4, 2, 16).unwrap();
    let (obj, st) = indexed_dib(&mut mem, &mut hle, &mut d);
    let green = make_rgb(0, 255, 0);
    let blue = make_rgb(0, 0, 255);
    let g = d.pixel_value(&mem, obj, green).unwrap();
    let b = d.pixel_value(&mem, obj, blue).unwrap();
    assert_ne!(g, b);
    let px = d.bitmap(obj).unwrap().px;

    let mut ctx = Ctx::new(&mut mem, &mut d, st);
    for i in 0..8 {
        let c = if i % 2 == 0 { green } else { blue };
        ctx.paint(i % 4, i / 4, rgb_to_native(c));
    }

    let mut got = Vec::new();
    for i in 0..8u32 {
        got.push(mem.read_u8(px + i).unwrap());
    }
    let want: Vec<u8> = (0..8)
        .map(|i| if i % 2 == 0 { g } else { b } as u8)
        .collect();
    assert_eq!(got, want, "every pixel asks about its own colour");
}

/// `IGRAPHICS_StretchBlt`'s source is a Windows BMP without the AEE header,
///so it has no `ncTransparent` for the host to ask. The colour
/// `AEE_RO_TRANSPARENT` names is the *default* one — `RGBToNative(RGB_MASK_COLOR)`, magenta, per
/// the header and the header — and the rule is about the source pixels. Keying on
/// the destination's colour instead agrees only while that destination still holds the default.
#[test]
fn a_stretched_picture_keys_on_its_own_magenta_not_the_destinations_key() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 16).unwrap();
    let dst = d.device;
    // A key that is not the default. If the blit asks the destination, nothing in the source matches
    // it and the magenta pixel is painted; it is the source's own magenta that makes it transparent.
    d.transparency(&mut mem, dst.obj, 0x1234).unwrap();
    mem.write_u16(dst.px, 0).unwrap();
    mem.write_u16(dst.px + 2, 0).unwrap();

    // Magenta and blue, as a decoded Windows BMP would arrive: no palette, no AEE header.
    let img = crate::resource::Bitmap {
        width: 2,
        height: 1,
        pixels: vec![255, 0, 255, 0, 0, 255],
        palette: None,
        alpha: crate::resource::Alpha::Opaque,
    };

    let mut g = Graphics::default();
    g.add(1, 0, Rect::new(0, 0, 2, 1), Rgba::new(0, 0, 0, 0));
    let mut st = g.get(1).unwrap();
    st.dest = dst.obj;
    let mut ctx = Ctx::new(&mut mem, &mut d, st);
    ctx.stretch(&img, 0, 0, 2, 1, g::AEE_RO_TRANSPARENT);

    assert_eq!(
        mem.read_u16(dst.px).unwrap(),
        0,
        "the source's magenta is the transparent colour, so the destination keeps what stood there"
    );
    assert_eq!(
        mem.read_u16(dst.px + 2).unwrap(),
        rgb_to_native(make_rgb(0, 0, 255)),
        "the opaque pixel is painted"
    );
}

/// A 1-bpp destination is the monochrome case the header names: its mask is white, not
/// magenta. That is why the mask is asked of the destination — hard-coding magenta would leave a
/// monochrome screen with no key at all, and `default_transparent` already takes the same branch.
#[test]
fn a_stretched_picture_on_a_one_bit_screen_keys_on_white() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let mut d = Display::new(&mut mem, &mut hle, 2, 1, 1).unwrap();
    let dst = d.device;
    // Both pixels start black, which is the only background that tells "white was keyed out" from
    // "white was painted": on a 1-bpp bitmap those two leave different bits behind. Bit 7 is x = 0
    // and bit 6 is x = 1.
    mem.write_u8(dst.px, 0).unwrap();

    let img = crate::resource::Bitmap {
        width: 2,
        height: 1,
        pixels: vec![255, 255, 255, 0, 0, 0],
        palette: None,
        alpha: crate::resource::Alpha::Opaque,
    };
    let mut g = Graphics::default();
    g.add(1, 0, Rect::new(0, 0, 2, 1), Rgba::new(0, 0, 0, 0));
    let mut st = g.get(1).unwrap();
    st.dest = dst.obj;
    {
        let mut ctx = Ctx::new(&mut mem, &mut d, st);
        ctx.stretch(&img, 0, 0, 2, 1, g::AEE_RO_TRANSPARENT);
    }
    assert_eq!(
        mem.read_u8(dst.px).unwrap(),
        0,
        "white is the monochrome mask, so it is keyed out and the black pixel paints black on black"
    );

    // The control: the same picture under COPY paints the white, which is what makes the assertion
    // above a statement about the key rather than about a blit that never ran.
    mem.write_u8(dst.px, 0).unwrap();
    {
        let mut ctx = Ctx::new(&mut mem, &mut d, st);
        ctx.stretch(&img, 0, 0, 2, 1, g::AEE_RO_COPY);
    }
    assert_eq!(
        mem.read_u8(dst.px).unwrap(),
        0b1000_0000,
        "under COPY the white pixel is painted over the black one"
    );
}
