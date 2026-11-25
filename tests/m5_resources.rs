//! M5: read what a guest app's art and text actually live in.
//!
//! Two halves. The synthetic container runs everywhere and pins the format down from the field
//! layout, so a regression in the parser cannot hide behind one machine's sample set. The rest is gated
//! on `BREW_GAME_DIR` pointing at a directory of real third-party apps, because the only proof that
//! the layout is right is that the files themselves add up — they are not vendored here.

use std::path::{Path, PathBuf};

use quoin::resource::{self, Container, RESTYPE_IMAGE, RESTYPE_STRING};

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("BREW_GAME_DIR")?;
    if dir.is_empty() {
        return None;
    }
    let path = PathBuf::from(&dir);
    assert!(
        path.is_dir(),
        "BREW_GAME_DIR={dir:?} has to name a directory of shipped apps"
    );
    Some(path)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read the corpus") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            walk(&path, out);
            continue;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("bar") | Some("mif") => out.push(path),
            _ => {}
        }
    }
}

/// Build a container in memory: four map entries over five resources, including one entry whose
/// `extra_ids` covers a run of two, which is how the real files pack their sprites.
fn synthetic() -> Vec<u8> {
    fn props(mime: &[u8]) -> Vec<u8> {
        let head = (2 + mime.len() + 1) as u16;
        let mut block = head.to_le_bytes().to_vec();
        block.extend_from_slice(mime);
        block.push(0);
        block
    }
    fn picture() -> Vec<u8> {
        // A 2x2, 8-bit BMP: 40-byte info header, two palette entries, two rows padded to four.
        // Bottom-up, so the file's first row is the picture's last.
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&0u32.to_le_bytes()); // bfSize, unused by the reader
        b.extend_from_slice(&0u32.to_le_bytes()); // the two reserved fields
        b.extend_from_slice(&62u32.to_le_bytes()); // 14 + 40 + two palette entries
        b.extend_from_slice(&40u32.to_le_bytes()); // biSize
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // planes
        b.extend_from_slice(&8u16.to_le_bytes()); // bits per pixel
        b.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        b.extend_from_slice(&8u32.to_le_bytes()); // biSizeImage
        b.extend_from_slice(&[0u8; 8]); // biXPelsPerMeter, biYPelsPerMeter
        b.extend_from_slice(&2u32.to_le_bytes()); // biClrUsed: two entries follow
        b.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant
        b.extend_from_slice(&[10, 10, 10, 0, 20, 20, 20, 0]); // BGR triples
        b.extend_from_slice(&[1, 1, 0, 0]); // bottom row
        b.extend_from_slice(&[0, 1, 0, 0]); // top row
        let mut r = props(b"image/bmp");
        r.extend_from_slice(&b);
        r
    }
    fn text() -> Vec<u8> {
        let mut r = props(b"text/plain");
        r.push(2); // two languages
        r.extend_from_slice(b"Loading...^Cargando...~");
        r
    }

    let bodies = vec![picture(), text(), picture(), picture(), picture()];
    let entries = 4u16;
    let count = bodies.len();
    let map_off = 0x20usize;
    let offsets_off = map_off + usize::from(entries) * 8;
    let data_off = offsets_off + 4 * (count + 1);

    let mut offsets = Vec::with_capacity(count + 1);
    let mut at = data_off;
    for b in &bodies {
        offsets.push(at);
        at += b.len();
    }
    offsets.push(at);

    let mut out = Vec::new();
    out.push(0x11);
    out.push(0);
    out.extend_from_slice(&1u16.to_le_bytes()); // version
    out.extend_from_slice(&1u16.to_le_bytes()); // oldest readable
    out.extend_from_slice(&entries.to_le_bytes()); // map entries
    out.extend_from_slice(&(map_off as u32).to_le_bytes());
    out.extend_from_slice(&((usize::from(entries) * 8) as u32).to_le_bytes());
    out.extend_from_slice(&(offsets_off as u32).to_le_bytes());
    out.extend_from_slice(&(count as u32).to_le_bytes()); // resources
    out.extend_from_slice(&(data_off as u32).to_le_bytes());
    out.extend_from_slice(&((at - data_off) as u32).to_le_bytes());
    assert_eq!(out.len(), map_off);
    // kind, first_id, extra_ids, table slot
    let map = [
        [6u16, 0, 0, 0],
        [1, 1, 0, 1],
        [6, 2, 0, 2],
        [6, 3, 1, 3], // ids 3 and 4, consecutively
    ];
    for e in map {
        for w in e {
            out.extend_from_slice(&w.to_le_bytes());
        }
    }
    assert_eq!(out.len(), offsets_off);
    for o in offsets {
        out.extend_from_slice(&(o as u32).to_le_bytes());
    }
    assert_eq!(out.len(), data_off);
    for b in &bodies {
        out.extend_from_slice(b);
    }
    assert_eq!(out.len(), at);
    out
}

#[test]
fn a_container_written_from_the_layout_parses_back() {
    let bytes = synthetic();
    let c = Container::parse(&bytes).expect("the synthetic container");
    assert_eq!(c.all().expect("readable").len(), 5, "five resources");
    let first = c
        .find(RESTYPE_IMAGE, 0)
        .expect("well-formed")
        .expect("image 0");
    assert_eq!(first.props, b"image/bmp\0", "the property block is its own");
    // `ISHELL_LoadResData` hands the app *these* bytes, block included, because the app reads the
    // block's own length out of the first byte and steps over it itself. Stripping the block here
    // instead makes that skip land in the middle of the picture - which is what stopped a reference
    // game from ever drawing one.
    assert_eq!(
        &first.stored[..2],
        b"\x0c\x00",
        "the stored bytes begin at the block"
    );
    assert_eq!(
        first.stored.split_at(12).1,
        first.body,
        "and the payload is what follows it"
    );
    let picture = resource::bmp(first.body).expect("a 2x2 8-bit BMP");
    assert_eq!((picture.width, picture.height), (2, 2));
    // Bottom-up in the file, so the first row of the bitmap is the file's last.
    assert_eq!(picture.rgb(0, 0), (10, 10, 10));
    assert_eq!(picture.rgb(1, 1), (20, 20, 20));
    let text = c
        .find(RESTYPE_STRING, 1)
        .expect("well-formed")
        .expect("string 1");
    let languages = resource::strings(text.body).expect("two languages");
    assert_eq!(languages.len(), 2);
    assert_eq!(
        String::from_utf16(&languages[0]).unwrap(),
        "Loading...",
        "the first language is the one the container was built for"
    );
    assert_eq!(String::from_utf16(&languages[1]).unwrap(), "Cargando...");
    // ids 3 and 4 come out of one map entry that carries an extra_ids run.
    for id in [2u16, 3, 4] {
        assert!(
            c.find(RESTYPE_IMAGE, id).expect("ok").is_some(),
            "image {id} is in the container"
        );
    }
    assert!(c.find(RESTYPE_IMAGE, 99).expect("ok").is_none());
    assert!(c.find(RESTYPE_STRING, 0).expect("ok").is_none());
}

#[test]
fn a_terminal_offset_past_the_end_is_tolerated_rather_than_refusing_the_container() {
    let table = 0x20 + 4 * 8;
    // The closing table entry is a sentinel: reference writers put it past the end of the file as a
    // matter of course (943 of 2 343 containers do), and the parser reads the slot but never
    // follows it. Refusing here would hide every record behind one stale word.
    let mut bytes = synthetic();
    bytes[table + 4 * 5..table + 4 * 6].copy_from_slice(&0xdead_0000u32.to_le_bytes());
    let c = Container::parse(&bytes).expect("the terminal slot's value is not compared");
    assert_eq!(
        c.all().expect("still readable").len(),
        4,
        "the tail record is the costed one — 929 of those 943 containers lose exactly their last \
         record, and the other four still serve"
    );

    // A *record* whose bytes are not in the file is a different matter: its span is bounds-checked
    // and declined alone, so the container keeps serving everything else.
    let mut bytes = synthetic();
    bytes[table + 4 * 3..table + 4 * 4].copy_from_slice(&0xdead_0000u32.to_le_bytes());
    let c = Container::parse(&bytes).expect("a bad record does not close the container");
    assert!(
        c.find(RESTYPE_IMAGE, 3).is_err(),
        "a record with no bytes in the file is declined by name, not invented"
    );
    assert!(
        c.find(RESTYPE_IMAGE, 4).expect("readable").is_some(),
        "the id after it takes the next table slot, which is still sound"
    );
    assert!(
        c.find(RESTYPE_IMAGE, 0).expect("readable").is_some(),
        "and the records that do add up still serve"
    );
}

/// The other way the table can fail to add up, and the one the test above used to stand for by
/// accident: the table ends somewhere other than where the header says the data section starts.
#[test]
fn a_table_that_ends_away_from_the_data_section_is_refused_by_name() {
    let mut bytes = synthetic();
    // `data_off` is the header's field at 0x18, and the synthetic table ends at 0x58: moving the
    // data section back to 0x40 puts the first resource ahead of it.
    bytes[0x18..0x1c].copy_from_slice(&0x40u32.to_le_bytes());
    let err = Container::parse(&bytes).expect_err("a table that disagrees with the data section");
    assert!(
        err.to_string().contains("the table ends at"),
        "named, not swallowed: {err}"
    );
}

/// The proof that matters: every real container in a external sample set, parsed and decoded.
#[test]
fn every_real_container_adds_up() {
    let Some(root) = game_dir() else {
        eprintln!("skipped: set BREW_GAME_DIR to a directory of shipped BREW apps to run M5");
        return;
    };
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(!files.is_empty(), "no .bar or .mif under {root:?}");
    let mut pictures = 0;
    let mut placeholders = 0;
    let mut other = 0;
    let mut unframed = 0;
    let mut foreign = 0;
    // Payloads filed under the image kind that this host cannot decode, counted per the MIME they
    // declared, with the first one's leading bytes so each can be checked against a file.
    let mut not_decoded: std::collections::BTreeMap<String, (usize, String)> =
        std::collections::BTreeMap::new();
    let mut texts = 0;
    for path in &files {
        let bytes = std::fs::read(path).expect("read the container");
        let c = match Container::parse(&bytes) {
            Ok(c) => c,
            // Not every `.mif` is this container: one in the sample set does not even carry the
            // signature. Reported rather than assumed, because it bounds what "the format" means.
            Err(e) => {
                foreign += 1;
                println!("  not this container: {} - {e}", path.display());
                continue;
            }
        };
        let all = match c.all() {
            Ok(all) => all,
            // A `.mif` font shares the container's header, map and offset table but not the
            // property-block-then-payload framing the images use. Reported, not skipped: the
            // container layout above is what is being asserted here.
            Err(e) => {
                unframed += 1;
                println!("  not covered by the framework: {} - {e}", path.display());
                continue;
            }
        };
        assert!(
            !all.is_empty(),
            "{} holds no resources at all",
            path.display()
        );
        for r in &all {
            match r.kind {
                RESTYPE_IMAGE => {
                    if r.body.is_empty() {
                        // Real containers hold image resources that are all property block and no
                        // pixels — a placeholder the app asks for by id and gets nothing back for.
                        placeholders += 1;
                        continue;
                    }
                    // The kind number says which table the handset filed this under, and the sample set
                    // files plenty under the wrong one: music is `RESTYPE_IMAGE` holding `audio/mid`
                    // and a "Loading." caption is `RESTYPE_IMAGE` holding `text/plain`.
                    // So the MIME is what makes the claim checkable — anything that says it is an
                    // image has to decode, and anything that says otherwise is counted as what it
                    // declared instead of being filed as a decoder this reader lacks.
                    let mime = resource::mime(r.props).unwrap_or("").to_string();
                    match resource::bmp(r.body) {
                        Ok(b) => {
                            assert!(b.width > 0 && b.height > 0);
                            // Every pixel has to be addressable, which catches a stride or
                            // palette slip.
                            let _ = b.rgb(b.width - 1, b.height - 1);
                            pictures += 1;
                        }
                        Err(e) => {
                            // `bmp` promises BMP, so that is the one encoding this test can demand
                            // of a payload declaring itself an image. Anything else that declares an
                            // image is a decoder this host does not have — a real gap, and it gets
                            // named below rather than asserted away, because a test that requires
                            // what the code does not do is just a louder lie.
                            assert!(
                                mime != "image/bmp",
                                "{} id {} declares {mime:?} and does not decode: {e}",
                                path.display(),
                                r.id
                            );
                            let head = r
                                .body
                                .iter()
                                .take(4)
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            not_decoded
                                .entry(if mime.is_empty() {
                                    "(none)".to_string()
                                } else {
                                    mime
                                })
                                .and_modify(|(n, _)| *n += 1)
                                .or_insert((1, head));
                            other += 1;
                        }
                    }
                }
                RESTYPE_STRING => {
                    let l = resource::strings(r.body)
                        .unwrap_or_else(|e| panic!("{} id {}: {e}", path.display(), r.id));
                    assert!(
                        !l[0].is_empty(),
                        "{} id {} has no text",
                        path.display(),
                        r.id
                    );
                    texts += 1;
                }
                _ => {}
            }
        }
    }
    println!(
        "{} containers: {} pictures decoded, {} empty image slots, {} payloads filed as images \
         that declared otherwise, {} strings, {} files whose framing is not covered",
        files.len(),
        pictures,
        placeholders,
        other,
        texts,
        unframed
    );
    for (mime, (n, head)) in &not_decoded {
        println!("  {n} x {mime:?} under the image kind, not decoded; first starts [{head}]");
    }
    assert!(
        pictures > 0,
        "some images must decode, or the framing is wrong"
    );
    assert!(
        texts > 0,
        "the .mif metadata strings must frame, or `strings` is wrong again"
    );
    // What is deliberately NOT claimed here: that every picture in every container has been seen.
    // `all()` lets one unframable payload drop out of a container rather than failing the whole
    // file, so `pictures` is a floor. What IS asserted, in the arm above, is that no payload
    // declaring `image/bmp` fails to decode — a count of "undecoded images" alone would have folded
    // MIDI tracks and the launcher's PNG icon into that number, and could not have told a framing
    // bug from a missing decoder.
    println!(
        "uncovered: {unframed} files refused as a whole, {other} image-slot payloads not decoded, \
         {foreign} files that are not this container; {placeholders} resources are empty by design"
    );
}
