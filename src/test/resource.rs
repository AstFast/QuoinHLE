// Moved out of `src/resource.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.
use super::*;

/// `image/jpeg` and the `image/jpg` spelling both have to reach the JPEG arm, and the sniffed answer is
/// the one word no payload can distinguish: a stream says nothing about whether its author wrote `jpeg`
/// or `jpg` in a property block.
#[test]
fn both_spellings_of_jpeg_decode_and_the_sniffer_picks_one() {
    assert!(image(b"image/jpeg\0", JPEG_GRAY).is_ok());
    assert!(image(b"image/jpg\0", JPEG_GRAY).is_ok());
    assert_eq!(sniff(JPEG_GRAY), Some(MIME_JPEG));
    assert_eq!(sniff_name("art.jpeg"), Some(MIME_JPEG));
    assert_eq!(sniff_name("cover.jpg"), Some(MIME_JPEG));
}

/// The three component counts the shipped `JPEGDecoder_Write` branches on, and the one it does not.
#[test]
fn a_jpeg_becomes_a_picture_of_the_depth_the_stream_declares() {
    let one = image(b"image/jpeg\0", JPEG_GRAY).expect("a one-component JPEG");
    assert_eq!(
        (one.width, one.height),
        (3, 2),
        "the frame the SOF declares"
    );
    // The luminance bytes are the picture; `Bitmap::rgb` reads them through the grey ramp beside them.
    assert_eq!(one.palette.as_ref().map(|p| p.len()), Some(256 * 3));
    assert_eq!(one.rgb(0, 0), (40, 40, 40), "index 40 is grey 40");
    assert_eq!(one.rgb(2, 1), (240, 240, 240));
    assert_eq!(
        one.alpha,
        Alpha::Opaque,
        "a baseline JPEG names no transparent pixel"
    );
    assert!(one.alpha_of(0, 0) == u8::MAX);

    let three = image(b"image/jpeg\0", JPEG_RGB).expect("a three-component JPEG");
    assert_eq!((three.width, three.height), (3, 2));
    assert_eq!(three.palette, None, "24 bpp has no table to read through");
    // Colour is decoded, not round-tripped exactly: YCbCr is lossy at every quality, so the claim to
    // test is that a red-ish pixel stays red-ish rather than that a byte came back the same.
    let (r, g, b) = three.rgb(2, 1);
    assert!(
        r > g && r > b,
        "the pixel authored red-dominant reads red-dominant: {r},{g},{b}"
    );
    let (l0, l1, l2) = three.rgb(0, 0);
    assert!(
        l2 > l0 && l2 > l1,
        "the pixel authored blue-dominant reads blue-dominant: {l0},{l1},{l2}"
    );

    // A stream that is not a JPEG is refused by name and by bytes, and a truncated one is refused too.
    assert!(image(b"image/jpeg\0", b"not a jpeg at all").is_err());
    assert!(image(b"image/jpeg\0", &JPEG_GRAY[..40]).is_err());
    assert_eq!(sniff(b"not a jpeg at all"), None);
}

fn png_of(color: png::ColorType, depth: png::BitDepth, w: u32, h: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(color);
        enc.set_depth(depth);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(data).unwrap();
    }
    out
}

#[test]
fn a_png_resource_decodes_to_the_colours_it_was_written_with() {
    // Two pixels of RGBA, which is the shape the pack's own art is in.
    let px = [10u8, 20, 30, 255, 40, 50, 60, 255];
    let img = png(&png_of(
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        2,
        1,
        &px,
    ))
    .unwrap();
    assert_eq!((img.width, img.height), (2, 1));
    assert_eq!(img.rgb(0, 0), (10, 20, 30));
    assert_eq!(img.rgb(1, 0), (40, 50, 60));
}

#[test]
fn an_indexed_png_keeps_a_palette_this_hosts_pixel_reader_can_use() {
    // One index per pixel into a two-colour table. The palette has to come back in the order
    // `Bitmap::rgb` indexes with, which is the one the BMP reader writes - not the PNG's own.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        // The first entry is the colour an index of 0 must resolve to, in PNG's RGB order.
        enc.set_palette(vec![1, 2, 3, 4, 5, 6]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0, 1]).unwrap();
    }

    let img = png(&out).unwrap();
    let palette = img.palette.as_ref().expect("the table came through");
    assert_eq!(
        &palette[..3],
        &[3, 2, 1],
        "entry 0, stored the way rgb() reads it"
    );
    assert_eq!(img.rgb(0, 0), (1, 2, 3));
    assert_eq!(img.rgb(1, 0), (4, 5, 6));
}

/// The same contract the indexed-PNG test pins, for the other way a picture arrives. A pack is
/// free to build its own BMP in memory and hand it to `AEEHelperFuncs::SetupNativeImage` —
/// with 8-bit, 9-colour portraits, say — and reading that table in the wrong order exchanges red
/// and blue in every indexed picture while leaving truecolour alone, which is how a face comes out
/// cyan.
/// A BMP of the shape `bmp` reads: 40-byte info header, `quads` of B,G,R,reserved palette, then
/// `rows` of already-packed pixel data with no padding added on its behalf.
#[test]
fn an_index_past_the_table_a_file_declared_reads_black_rather_than_taking_the_host_down() {
    // `biClrUsed` of 2 on an 8-bpp picture is a real shape in the shipped corpus, and every pixel
    // in it can still name one of 256 colours. The lookup has to answer for the ones the file
    // never carried instead of indexing past the table.
    let bytes = bmp_of(8, 2, 1, &[0, 0, 255, 0, 0, 255, 0, 0], &[200, 1, 0, 0]);
    let img = bmp(&bytes).expect("the picture decodes");
    assert_eq!(
        img.palette.as_ref().unwrap().len(),
        6,
        "two entries, as declared"
    );
    assert_eq!(
        img.rgb(0, 0),
        (0, 0, 0),
        "index 200 names nothing the file carried"
    );
    assert_eq!(
        img.rgb(1, 0),
        (0, 255, 0),
        "and the entry that is there still reads"
    );
}

/// A GIF written by the encoder of the same crate this host reads with, so a fixture cannot agree
/// with a wrong reader the way hand-laid bytes can: `pixels` are indices into the global `palette`
/// (RGB triples), and `transparent` is the index the graphic control extension names.
fn gif_of(
    pixels: &[u8],
    width: u16,
    height: u16,
    palette: &[(u8, u8, u8)],
    transparent: Option<u8>,
) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(palette.len() * 3);
    for (r, g, b) in palette {
        rgb.extend_from_slice(&[*r, *g, *b]);
    }
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, width, height, &rgb)
            .expect("a GIF screen descriptor is writable");
        enc.write_frame(&gif::Frame::from_indexed_pixels(
            width,
            height,
            pixels.to_vec(),
            transparent,
        ))
        .expect("one indexed frame");
    }
    out
}

fn bmp_of(bits: u16, w: u32, h: i32, quads: &[u8], rows: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + quads.len() + rows.len()) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(54 + quads.len() as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(quads.len() as u32 / 4).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(quads);
    out.extend_from_slice(rows);
    out
}

/// Four bits a pixel is the depth `tank-1073743380` stores its tiles in, and the reason
/// `AEEHelperFuncs::SetupNativeImage` was answering a valid picture with `NULL`: the reader only
/// accepted 8, 24 and 32, and a sub-byte depth has to be *unpacked* — which is also the one place
/// a row's four-byte padding cannot be inferred from the pixel count.
#[test]
fn a_sub_byte_bmp_unpacks_index_to_index_past_its_row_padding() {
    // Three pixels wide, so a row is two bytes of pixels and two of padding. Bottom-up: the row
    // written first is the bottom of the picture.
    let quads = [0x30u8, 0x20, 0x10, 0x00, 0x48, 0xe0, 0xf8, 0x00];
    let rows: Vec<u8> = vec![0x10, 0x10, 0, 0, 0x01, 0x00, 0, 0];
    let bytes = bmp_of(4, 3, 2, &quads, &rows);

    let img = bmp(&bytes).unwrap();
    assert_eq!((img.width, img.height), (3, 2));
    assert_eq!(
        img.pixels,
        vec![0, 1, 0, 1, 0, 1],
        "indices, most significant first"
    );
    assert_eq!(
        img.palette
            .as_ref()
            .expect("a 4-bit picture has a table")
            .len(),
        6,
        "biClrUsed said two entries, and `nColors` is a uint8"
    );
    assert_eq!(img.rgb(0, 0), (0x10, 0x20, 0x30), "top row, first pixel");
    assert_eq!(img.rgb(1, 0), (0xf8, 0xe0, 0x48), "top row, second pixel");
    assert_eq!(img.rgb(2, 0), (0x10, 0x20, 0x30), "top row, third pixel");
    assert_eq!(
        img.rgb(0, 1),
        (0xf8, 0xe0, 0x48),
        "the row written first is the bottom"
    );
    // The sizing half has to agree, or `SetupNativeImage` allocates a buffer it then refuses to
    // fill — which is the same refusal wearing a different hat.
    assert!(file_size(&bytes).is_some_and(|n| n >= bytes.len()));
}

/// One bit a pixel, which is the same unpacking with a different shift and the depth a mask or a
/// monochrome sprite arrives in.
#[test]
fn a_one_bit_bmp_reads_its_seven_remaining_pixels_out_of_the_same_byte() {
    let quads = [0u8, 0, 0, 0, 0xff, 0xff, 0xff, 0];
    // Eight pixels across: `1011 0001` = indices 1,0,1,1,0,0,0,1, then three bytes of padding.
    let rows: Vec<u8> = vec![0b1011_0001, 0, 0, 0];
    let img = bmp(&bmp_of(1, 8, 1, &quads, &rows)).unwrap();
    assert_eq!(img.pixels, vec![1, 0, 1, 1, 0, 0, 0, 1]);
    assert_eq!(img.rgb(0, 0), (0xff, 0xff, 0xff));
    assert_eq!(img.rgb(1, 0), (0, 0, 0));
}

#[test]
fn an_indexed_bmp_palette_comes_back_the_way_the_pixel_reader_indexes_it() {
    // Two entries, each a B,G,R,reserved quad: index 0 is a dark brown, index 1 a warm skin tone.
    let quads = [0x30u8, 0x20, 0x10, 0x00, 0x48, 0xe0, 0xf8, 0x00];
    // Four across and two down, so no row padding is involved. Bottom-up: the second row written
    // is the top of the picture.
    let rows: Vec<u8> = vec![1, 0, 0, 1, 0, 1, 1, 0];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&((62 + rows.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&62u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&4i32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&quads);
    bytes.extend_from_slice(&rows);

    let img = bmp(&bytes).unwrap();
    assert_eq!(
        img.palette.as_ref().expect("the table came through")[..3],
        [0x30, 0x20, 0x10],
        "stored as BGR, the order Bitmap::rgb indexes with"
    );
    assert_eq!(img.rgb(0, 0), (0x10, 0x20, 0x30));
    assert_eq!(
        img.rgb(1, 0),
        (0xf8, 0xe0, 0x48),
        "the skin tone stays warm"
    );
    assert_eq!(
        img.rgb(0, 1),
        (0xf8, 0xe0, 0x48),
        "the bottom row is the one written last"
    );
}

#[test]
fn the_image_reader_goes_by_the_property_block_not_the_magic() {
    let bytes = png_of(png::ColorType::Grayscale, png::BitDepth::Eight, 1, 1, &[7]);
    // The block names the format, and that is what decides: the corpus files MIDI and text
    // under the image kind, so the first bytes of a payload are not the thing to trust.
    assert!(image(b"image/png\0", &bytes).is_ok());
    assert!(image(b"image/bmp\0", &bytes).is_err());
    assert_eq!(image(b"image/png\0", &bytes).unwrap().rgb(0, 0), (7, 7, 7));
    // A resource with no block at all - which is what the `.par` reader produces - falls back
    // to the payload naming itself.
    assert!(image(b"", &bytes).is_ok());
}

#[test]
fn a_format_this_host_has_no_decoder_for_is_refused_by_its_own_name() {
    // A pack's other art can be in these, and saying so is the difference between "we cannot read
    // this yet" and "this was never a picture".
    let err = image(b"image/zbm\0", b"not a picture this host knows").unwrap_err();
    assert!(format!("{err}").contains("zbm"), "{err}");
}

/// The claim every caller of [`sniff`] rests on, checked rather than commented: the list of
/// names a sniffer may give is the list [`image`] has an arm for, in both directions.
///
/// A third format added to one side only is exactly the drift the pair exists to prevent — a
/// MIME named by a suffix test and refused by the decoder, which is the failure
/// `IShell::DetectType` would hand straight to a pack that cannot disbelieve it — so it is a
/// test rather than a note.
#[test]
fn every_name_a_sniffer_gives_is_one_the_decoder_takes() {
    let of = |mime: &str| match mime {
        MIME_PNG => png_of(png::ColorType::Grayscale, png::BitDepth::Eight, 1, 1, &[7]),
        MIME_BMP => bmp_of(8, 1, 1, &[1, 2, 3, 0], &[0, 0, 0, 0]),
        MIME_JPEG => JPEG_GRAY.to_vec(),
        // Two indices, one entry each of a two-colour table: the smallest picture that can answer
        // `image/gif` with bytes rather than with a name.
        MIME_GIF => gif_of(&[0, 1], 2, 1, &[(0, 0, 0), (255, 255, 255)], None),
        other => panic!("{other} is in DECODABLE with no payload to answer it"),
    };
    for mime in DECODABLE {
        let payload = of(mime);
        assert!(
            image(mime.as_bytes(), &payload).is_ok(),
            "{mime} names an arm this reader does not take"
        );
        assert_eq!(
            sniff(&payload),
            Some(mime),
            "the bytes answer with the same word the block does"
        );
        // The suffix is the subtype, so `image/png` is `.png` and nothing else.
        let (_, subtype) = mime.rsplit_once('/').expect("a MIME is type/subtype");
        assert_eq!(
            sniff_name(&format!("dir/art.{subtype}")),
            Some(mime),
            "and a name is answered by the same set"
        );
        assert_eq!(
            sniff_name(&format!("dir/art.{}", subtype.to_ascii_uppercase())),
            Some(mime),
            "folded, because a pack's file names are not its author's doing"
        );
    }
    // Nothing outside the list is answerable from either side, however recognisable its bytes.
    // `cover.jpg` used to be in this list: a `.jpg` is `image/jpeg` now that [`jpeg`] exists, so its case
    // moved to `both_spellings_of_jpeg_decode_and_the_sniffer_picks_one`, where the answer is asserted
    // rather than refused.
    for (name, bytes) in [("theme.mid", b"MThd\x00\x00\x00\x06".as_slice())] {
        assert_eq!(sniff_name(name), None, "{name} has no decoder here");
        assert_eq!(sniff(bytes), None, "{name} has no decoder here");
    }
    // `anim.gif` is no longer one of those: it has a decoder now, so the case that keeps this loop
    // honest is the GIF whose magic is all it has. Six bytes name a format the loader cannot open, and
    // a sniffer that answered from the prefix would be the drift this test exists to catch.
    assert_eq!(sniff_name("anim.gif"), Some(MIME_GIF));
    assert_eq!(
        sniff(b"GIF89a"),
        None,
        "magic without a decodable picture names nothing"
    );
}

/// A container laid out the way the real ones are: header, one map entry, a two-word offset
/// table, and one resource whose property block names it as a BMP.
///
/// `data_bytes` is a parameter rather than the truth, because the word that lies is the one
/// these cases are about.
/// The one-record picture container this file's tests were written against, now that the shape lives
/// in one place (`synthetic_container`) and the store tests can build the same bytes. `data_bytes` is
/// patched afterwards because the point of several tests below is exactly that this word lies.
fn bar_of(payload: &[u8], data_bytes: u32) -> Vec<u8> {
    let mut bar = synthetic_container(&[(RESTYPE_IMAGE, 7, payload)], 0);
    bar[0x1c..0x20].copy_from_slice(&data_bytes.to_le_bytes());
    bar
}

#[test]
fn a_container_whose_header_size_lies_still_answers_its_resources() {
    // An art container can have a size word 3 309 bytes larger than the bytes behind it, while its
    // own offset table adds up. Refusing on that word hides every
    // one of the 312 records in it, and the pack — which had asked for its font table by name
    // and been handed NULL — went on calling through the empty handle.
    let bar = bar_of(b"BMpay", 17 + 3309);
    let c = Container::parse(&bar).expect("a size word is not what identifies a container");
    let r = c
        .find(RESTYPE_IMAGE, 7)
        .unwrap()
        .expect("the map covers this id");
    assert_eq!(r.body, b"BMpay", "the payload, past the block");
    assert_eq!(r.props, b"image/bmp\0");
    assert_eq!(
        r.stored.len(),
        12 + 5,
        "as stored, block and payload together"
    );
}

#[test]
fn a_terminal_sentinel_past_the_end_costs_one_record_not_the_container() {
    // This reader must not refuse the whole container when the offset table's final entry points
    // past the end of the file: 943 of the two corpora's 2 343 `.mif`/`.bar` files do it — 705 by
    // exactly one byte, and not one of
    // the 62 containers in the game corpus — so the check was not catching a wrong guess about the
    // format: it was hiding every picture and string in those apps. That entry bounds the *last*
    // record and is never served, so what it can cost is the records it overshoots (929 files lose
    // one, 14 lose two to four), which is the same tolerance `all()` already extends to a payload it
    // cannot frame.
    let file = synthetic_container(
        &[
            (RESTYPE_IMAGE, 7, b"BMfirst"),
            (RESTYPE_IMAGE, 8, b"BMsecond"),
        ],
        1,
    );
    let c = Container::parse(&file).expect("a sentinel is not a format error");
    let first = c
        .find(RESTYPE_IMAGE, 7)
        .unwrap()
        .expect("the record that fits is served");
    assert_eq!(first.body, b"BMfirst");
    // Error or "not there" is this reader's choice — `Store::find` treats them alike — so the
    // assertion is about what the app receives, not about which variant carries it.
    assert!(
        !matches!(c.find(RESTYPE_IMAGE, 8), Ok(Some(_))),
        "and the record the sentinel bounds is not served as a truncated picture"
    );
    let all = c.all().expect("the container still enumerates");
    assert_eq!(
        all.len(),
        1,
        "one record comes back, not zero and not a panic"
    );
}

#[test]
fn a_one_record_container_whose_sentinel_overruns_mounts_and_serves_nothing() {
    // With a single record the sentinel *is* that record's stop, so the file mounts and answers
    // nothing. That is the shape the fix is for: refusing put every one of these apps' strings behind
    // one bad word, and serving the record short would invent bytes the file does not have.
    let file = synthetic_container(&[(RESTYPE_IMAGE, 7, b"BMpay")], 2);
    let c = Container::parse(&file).expect("the container is still a container");
    assert_eq!(c.all().expect("enumeration does not fail").len(), 0);
    assert!(
        !matches!(c.find(RESTYPE_IMAGE, 7), Ok(Some(_))),
        "its only record is not served short by those two bytes"
    );
}

#[test]
fn an_absurd_sentinel_still_leaves_the_reader_guessing_nothing() {
    // Tolerating the sentinel is not tolerating any value: what is served is still bounded by the
    // file, so a one-megabyte lie costs the record it bounds and nothing else.
    let file = synthetic_container(
        &[(RESTYPE_IMAGE, 7, b"BMpay"), (RESTYPE_IMAGE, 8, b"x")],
        1 << 20,
    );
    let c = Container::parse(&file).expect("only the table's own slots have to be inside the file");
    assert_eq!(
        c.find(RESTYPE_IMAGE, 7).unwrap().unwrap().body,
        b"BMpay",
        "the sound record is unaffected"
    );
    assert_eq!(c.all().unwrap().len(), 1);
}

#[test]
fn data_that_is_not_where_the_table_starts_is_refused() {
    // The one positive identity that replaces the equality this reader used to demand: the first
    // resource begins where the header says the data section does, on every file measured.
    let mut bar = bar_of(b"BMpay", 17);
    bar[0x18..0x1c].copy_from_slice(&0x34u32.to_le_bytes());
    assert!(Container::parse(&bar).is_err(), "header and table disagree");
}

/// A PNG's `tRNS` chunk is coverage, not a colour, and reading it must not touch the colours.
///
/// This is the shape 746 pictures in the shipped corpus use: an indexed picture whose see-through
/// background entry is *also* a stored colour — white, usually.
/// A reader that turned the transparent entry into a mask colour would hand the pack bytes it never
/// shipped, and a pack that reads its own art back would find them.
#[test]
fn a_trns_entry_becomes_coverage_without_rewriting_the_colour() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(vec![255, 255, 255, 0, 0, 255]);
        // Only the first entry is named. PNG's own rule is that an index the chunk does not reach
        // is fully opaque, so the second entry must not inherit the first one's answer.
        enc.set_trns(vec![0u8]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0, 1]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(
        img.rgb(0, 0),
        (255, 255, 255),
        "the transparent entry keeps the colour the file stored"
    );
    assert_eq!(img.rgb(1, 0), (0, 0, 255));
    assert_eq!(img.alpha_of(0, 0), 0, "index 0 is see-through");
    assert_eq!(
        img.alpha_of(1, 0),
        u8::MAX,
        "an index past the end of the chunk is opaque, not missing"
    );
    assert!(img.alpha.any_transparent());
}

/// A truecolour `tRNS` names one colour, and only the pixels equal to it are see-through; the colours
/// stay as the file stored them, as on the indexed path.
///
/// The chunk is six bytes for an 8-bit picture because that is the shape PNG gives a non-indexed key
/// (one 16-bit sample per channel, right-justified) and the decoder narrows it - a hand-written
/// 3-byte chunk is not the shape a file arrives in.
#[test]
fn a_trns_colour_key_marks_only_the_named_truecolour_pixels() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_trns(vec![0, 1, 0, 2, 0, 3]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[1, 2, 3, 9, 8, 7]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.alpha_of(0, 0), 0, "the colour the chunk named");
    assert_eq!(img.alpha_of(1, 0), u8::MAX, "one step away is opaque");
    assert_eq!(
        img.rgb(0, 0),
        (1, 2, 3),
        "a see-through pixel keeps the colour the file stored"
    );
    assert!(img.alpha.any_transparent());
}

/// The greyscale shape of the same chunk: one sample, so two bytes in the file and one key byte here
/// after the same right-justified narrowing.
#[test]
fn a_trns_colour_key_marks_only_the_named_grey_pixel() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_trns(vec![0, 7]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[7, 9]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.alpha_of(0, 0), 0);
    assert_eq!(img.alpha_of(1, 0), u8::MAX);
    assert_eq!(
        img.rgb(0, 0),
        (7, 7, 7),
        "expanded to a triple, not rewritten"
    );
}

/// The same key on a picture that packs eight samples to the byte. `expand_bits` scales those samples
/// to 0 and 255, so the key has to come through the same scale: comparing the raw `1` against a buffer
/// of 0s and 255s could only ever name zero, which is what this branch first did. The second half of
/// the test checks the other edge rather than assuming it - a key of `2` is not a value a one-bit
/// picture can hold, and the scaled result must not wrap onto one that it can.
#[test]
fn a_one_bit_grey_colour_key_is_scaled_the_same_way_as_the_pixels_it_names() {
    let one_bit = |key: u8| {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 8, 1);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::One);
            // A non-indexed key is one 16-bit sample, right-justified, whatever the picture's depth.
            enc.set_trns(vec![0, key]);
            let mut writer = enc.write_header().unwrap();
            // PNG fills from the most significant end, so pixels 0..3 carry sample 0 and 4..7 carry 1.
            writer.write_image_data(&[0b0000_1111]).unwrap();
        }
        png(&out).unwrap()
    };
    let img = one_bit(1);
    assert_eq!(img.rgb(0, 0), (0, 0, 0), "sample 0 is black");
    assert_eq!(
        img.rgb(7, 0),
        (u8::MAX, u8::MAX, u8::MAX),
        "sample 1 is scaled to full white, not left as the number 1"
    );
    assert_eq!(
        img.alpha_of(0, 0),
        u8::MAX,
        "the sample the key did not name keeps standing"
    );
    assert_eq!(
        img.alpha_of(7, 0),
        0,
        "the key named sample 1, which is this picture's white end"
    );
    assert!(
        !one_bit(2).alpha.any_transparent(),
        "a key the picture's own depth cannot hold names nothing, rather than wrapping onto a real sample"
    );
}

/// The control that keeps the two tests above meaning something: a picture that names no colour stays
/// on the opaque path, so a key test cannot pass by marking everything.
#[test]
fn a_truecolour_picture_without_a_key_stays_opaque() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[1, 2, 3, 9, 8, 7]).unwrap();
    }
    let img = png(&out).unwrap();
    assert!(matches!(img.alpha, Alpha::Opaque));
    assert!(!img.alpha.any_transparent());
}

/// The documented edge, pinned rather than left to drift: samples reach us narrowed (`STRIP_16`), but
/// the *key* of a 16-bit file stays in its six-byte / two-byte form, and this reader does not guess
/// which half of a pair to keep. So the honest answer for such a file is *no transparency*.
#[test]
fn a_sixteen_bit_key_is_not_applied_because_only_the_samples_get_narrowed() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 1, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Sixteen);
        enc.set_trns(vec![0, 1, 0, 2, 0, 3]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0, 1, 0, 2, 0, 3]).unwrap();
    }
    let img = png(&out).unwrap();
    assert!(
        matches!(img.alpha, Alpha::Opaque),
        "a key this reader would have to guess at must not be applied"
    );
}

/// A 16-bit truecolour picture has to come back as one byte per channel, or every reader below the
/// decoder is off by a factor: `chunks_exact(3)` over two bytes per channel names eight pixels where
/// the file has three, and the "colour" of each is half of one sample and half of the next.
#[test]
fn a_sixteen_bit_truecolour_picture_arrives_as_one_byte_per_channel() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Sixteen);
        let mut writer = enc.write_header().unwrap();
        // PNG stores a sample big-endian, so the three channels of the first pixel are 0x1234,
        // 0x5678 and 0x9abc.
        writer
            .write_image_data(&[
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44,
            ])
            .unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(
        img.pixels.len(),
        6,
        "two pixels of three channels, not twelve bytes for two"
    );
    assert_eq!(
        img.rgb(0, 0),
        (0x12, 0x56, 0x9a),
        "the high byte of each sample"
    );
    assert_eq!(img.rgb(1, 0), (0xde, 0x11, 0x33));
}

/// The same file with an alpha channel: this one also decides *which* byte becomes the coverage, and
/// the old slice took it from the middle of a colour sample.
#[test]
fn a_sixteen_bit_rgba_picture_reads_alpha_from_its_own_channel() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Sixteen);
        let mut writer = enc.write_header().unwrap();
        writer
            .write_image_data(&[
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
                0xff, 0x00,
            ])
            .unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.pixels.len(), 6, "two pixels of three stored channels");
    assert_eq!(img.rgb(0, 0), (0x11, 0x33, 0x55));
    assert_eq!(img.rgb(1, 0), (0x99, 0xbb, 0xdd));
    assert_eq!(
        img.alpha_of(0, 0),
        0x77,
        "the alpha channel, not a colour's low byte"
    );
    assert_eq!(
        img.alpha_of(1, 0),
        0xff,
        "the pair 0xff00 narrows to 0xff, opaque"
    );
}

/// Greyscale widens to the triples every picture reader here speaks, so a 16-bit one must not put two
/// bytes of one sample into three consecutive pixels.
#[test]
fn a_sixteen_bit_greyscale_picture_widens_from_its_high_byte() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Sixteen);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0xab, 0xcd, 0x12, 0x34]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.pixels.len(), 6);
    assert_eq!(img.rgb(0, 0), (0xab, 0xab, 0xab));
    assert_eq!(img.rgb(1, 0), (0x12, 0x12, 0x12));
}

/// The stride, not the transparency: a file may store four bits per index, and then one byte of the
/// stream is two pixels. Both pixel readers above this one walk one byte per pixel, and since `#50`
/// the transparency walks with them, so a packed row left packed would name the *wrong* pixels
/// see-through. 15 of the corpus' 921 PNG records are indexed below eight bits.
#[test]
fn a_four_bit_indexed_png_reaches_the_pixel_readers_as_one_byte_per_pixel() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 8, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Four);
        enc.set_palette(vec![255, 255, 255, 200, 20, 20]);
        enc.set_trns(vec![0u8]);
        let mut writer = enc.write_header().unwrap();
        // Eight samples of four bits: the writer takes the packed form, which is four bytes, and the
        // last nibble of the row is a real pixel rather than padding.
        writer.write_image_data(&[0x11, 0x11, 0x11, 0x10]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.pixels.len(), 8, "one byte per pixel, not one per two");
    assert_eq!(img.rgb(0, 0), (200, 20, 20), "the first nibble is index 1");
    assert_eq!(img.rgb(7, 0), (255, 255, 255), "the last is index 0");
    assert_eq!(img.alpha_of(0, 0), u8::MAX);
    assert_eq!(
        img.alpha_of(7, 0),
        0,
        "the see-through one is the last pixel"
    );
}

/// One bit per index is the extreme case of the same rule, and it is where a wrong end-of-byte goes
/// unnoticed in the colours and shows up in the transparency: every pixel of a run shares one byte.
#[test]
fn a_one_bit_indexed_png_fills_its_pixels_from_the_most_significant_end() {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 8, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::One);
        enc.set_palette(vec![255, 255, 255, 10, 20, 200]);
        enc.set_trns(vec![0u8]);
        let mut writer = enc.write_header().unwrap();
        // Only pixel 0 is set, and PNG puts it in the top bit of the row's single byte.
        writer.write_image_data(&[0x80]).unwrap();
    }
    let img = png(&out).unwrap();
    assert_eq!(img.pixels.len(), 8);
    assert_eq!(img.rgb(0, 0), (10, 20, 200), "the set bit is index 1");
    assert_eq!(img.rgb(1, 0), (255, 255, 255), "the clear ones are index 0");
    assert_eq!(img.alpha_of(0, 0), u8::MAX);
    assert_eq!(img.alpha_of(1, 0), 0);
    assert_eq!(
        img.alpha_of(7, 0),
        0,
        "all seven remaining pixels, not just the next one"
    );
}

/// A sub-eight-bit grey is the other shape the format allows packed, and there the sample is a
/// brightness rather than a table index: four bits of `10` means two thirds of the range, not 10.
#[test]
fn a_four_bit_grey_png_scales_its_samples_into_the_range_it_meant() {
    let img = png(&png_of(
        png::ColorType::Grayscale,
        png::BitDepth::Four,
        4,
        1,
        &[0x05, 0xA5],
    ))
    .unwrap();
    assert_eq!(img.rgb(0, 0), (0, 0, 0));
    assert_eq!(img.rgb(1, 0), (85, 85, 85), "5 of 15");
    assert_eq!(img.rgb(2, 0), (170, 170, 170), "10 of 15 is two thirds");
    assert_eq!(img.rgb(3, 0), (85, 85, 85));
}

#[test]
fn a_trns_chunk_that_names_nothing_keeps_the_picture_on_the_opaque_path() {
    // The case the batched row path depends on: a file may carry a `tRNS` chunk whose every value
    // is 255. If that were treated as transparency the blitter would fall to the per-pixel loop for
    // pictures that need nothing of the kind, and the batched row path would quietly stop being
    // taken.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(vec![1, 2, 3, 4, 5, 6]);
        enc.set_trns(vec![u8::MAX, u8::MAX]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0, 1]).unwrap();
    }
    let img = png(&out).unwrap();
    assert!(
        matches!(img.alpha, Alpha::Index(_)),
        "the chunk is still recorded as the shape it is"
    );
    assert!(
        !img.alpha.any_transparent(),
        "and it says nothing, so no picture is demoted to the per-pixel path because of it"
    );
}

/// An alpha channel is a *coverage* number: the colours stay as shipped and the blitter is the one
/// that composites. 87 423 partly-covered pixels live in the corpus this way.
#[test]
fn an_rgba_png_keeps_its_alpha_as_coverage_rather_than_as_a_colour() {
    let px = [10u8, 20, 30, 0, 40, 50, 60, 128];
    let img = png(&png_of(
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        2,
        1,
        &px,
    ))
    .unwrap();
    assert_eq!(img.rgb(0, 0), (10, 20, 30));
    assert_eq!(img.rgb(1, 0), (40, 50, 60));
    assert_eq!(img.alpha_of(0, 0), 0);
    assert_eq!(
        img.alpha_of(1, 0),
        128,
        "a value between 0 and 255 is the case that composites instead of skipping"
    );
    assert!(img.alpha.any_transparent());
}

/// A GIF keeps its indices and the colour table that names them, because an 8-bit `IBitmap` is
/// asked for palette entries by index (`Bitmap::palette`, `Bitmap::rgb`) — and because the format
/// names transparency as an index, not as a colour.
#[test]
fn a_gif_frame_keeps_its_indices_and_the_table_that_names_them() {
    let bytes = gif_of(
        &[0, 1, 2, 3],
        2,
        2,
        &[(255, 0, 0), (0, 255, 0), (0, 0, 255), (10, 20, 30)],
        Some(2),
    );
    let img = gif(&bytes).expect("a GIF written by the crate this host reads with");
    assert_eq!((img.width, img.height), (2, 2));
    assert_eq!(
        img.pixels,
        vec![0, 1, 2, 3],
        "the indices are the picture, not an expansion of it"
    );
    assert_eq!(
        img.palette,
        Some(vec![0, 0, 255, 0, 255, 0, 255, 0, 0, 30, 20, 10]),
        "BGR, which is the order Bitmap::rgb indexes with"
    );
    match &img.alpha {
        Alpha::Index(table) => {
            assert_eq!(table.len(), 4, "one alpha per entry the file named");
            assert_eq!(table[2], 0, "the index the file pointed its pen at");
            assert!(
                table
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != 2)
                    .all(|(_, &a)| a == u8::MAX),
                "every other entry is painted as stored"
            );
        }
        other => panic!("a GIF that names a transparent index cannot arrive as {other:?}"),
    }
    assert!(img.alpha.any_transparent());
    assert_eq!(
        img.rgb(1, 1),
        (10, 20, 30),
        "and a colour survives the table"
    );

    // A second frame does not turn this into two pictures: animation is `IAnimation`'s contract, so a
    // caller that asked for a static image gets the first frame and nothing else.
    let rgb = [0u8, 0, 0, 255, 255, 255];
    let mut two = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut two, 2, 1, &rgb).unwrap();
        enc.write_frame(&gif::Frame::from_indexed_pixels(2, 1, vec![0, 1], None))
            .unwrap();
        enc.write_frame(&gif::Frame::from_indexed_pixels(2, 1, vec![1, 0], None))
            .unwrap();
    }
    let first = gif(&two).expect("the first frame is a picture");
    assert_eq!(
        first.pixels,
        vec![0, 1],
        "and it is the one handed over, not the one after it"
    );

    // The refusal half of the same claim: bytes that say GIF and do not decode are not a type this
    // host will hand to a caller's load.
    assert!(gif(b"GIF89a").is_err(), "six bytes are not a picture");
    assert!(gif(b"GIF8").is_err(), "not even a complete magic");
}

/// A KDDI container marks a Japanese string with `FD FE` and stores the bytes that follow in
/// Shift-JIS. Reading them one per byte turns `83 65` ("テ") into two Latin-1 code units, which is
/// exactly the mojibake a menu shows when its text is read as Latin.
#[test]
fn a_shift_jis_marked_string_comes_back_as_the_characters_it_names() {
    // "テスト"
    let body = [0xFD, 0xFE, 0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
    let langs = strings(&body).expect("a marked string");
    assert_eq!(langs, vec![vec![0x30C6, 0x30B9, 0x30C8]]);
}

/// The big-endian spelling of the UTF-16 mark, kept next to the little-endian one the packing
/// tools write so the two cannot answer in different orders.
#[test]
fn a_utf16_big_endian_mark_is_read_in_its_own_order() {
    let body = [0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42];
    assert_eq!(strings(&body).unwrap(), vec![vec![0x0041, 0x0042]]);
}
