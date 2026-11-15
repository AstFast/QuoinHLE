// Moved out of `src/par.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// Deflate `payload` the way a pack stores a leaf, so a test container can hold something real.
fn deflate(payload: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::new();
    let mut enc = flate2::write::GzEncoder::new(&mut out, flate2::Compression::fast());
    enc.write_all(payload).unwrap();
    enc.finish().unwrap();
    out
}

/// A header holding `children`, with the table the format demands. Each child is already
/// framed - a nested header, an indexed payload, or a stored one.
fn pack(children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::from(*SIGNATURE);
    out.extend_from_slice(&(children.len() as u32).to_le_bytes());
    let mut at = HEADER_BYTES + 4 * children.len();
    for child in children {
        out.extend_from_slice(&(at as u32).to_le_bytes());
        at += child.len();
    }
    for child in children {
        out.extend_from_slice(child);
    }
    out
}

/// An indexed payload: the ids it serves, then its payload.
fn indexed(ids: &[u32], payload: &[u8]) -> Vec<u8> {
    let mut out = (ids.len() as u16).to_le_bytes().to_vec();
    for id in ids {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}

const BMP: &[u8] = b"BM\x00\x00\x00\x00\x00\x00\x00\x00";

#[test]
fn a_picture_stored_with_no_index_is_named_by_the_table_above_it() {
    // What `ui.par` actually looks like: the sub-files are the gzip streams themselves, so
    // their first two bytes are the gzip magic and there is no table of their own to read.
    let inner = pack(&[deflate(BMP), deflate(BMP)]);
    let outer = pack(&[indexed(&[5, 6], &inner)]);

    let found = read(&outer).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(
        found.iter().map(|r| r.id).collect::<Vec<_>>(),
        [5, 6],
        "the ids come from the table one level up"
    );
    assert!(found.iter().all(|r| r.kind == RESTYPE_IMAGE));
}

#[test]
fn an_indexed_leaf_is_filed_under_its_parents_id_not_its_own() {
    // The same file one level deeper, where every leaf does carry a table - and a table that
    // names something unrelated. Reading the leaf's own is what filed a whole pack under one id.
    let inner = pack(&[
        indexed(&[9], &deflate(BMP)),
        indexed(&[17], &deflate(BMP)),
        indexed(&[23], &deflate(b"not a picture")),
    ]);
    let outer = pack(&[indexed(&[0, 1, 2], &inner)]);

    let found = read(&outer).unwrap();
    assert_eq!(found.len(), 3, "every picture the index names comes back");
    assert_eq!(found.iter().map(|r| r.id).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(found[0].kind, RESTYPE_IMAGE);
    assert_eq!(found[1].kind, RESTYPE_IMAGE);
    // A payload that is neither a picture nor text is still a resource the app may want.
    assert_eq!(found[2].kind, RESTYPE_BINARY);
    assert_eq!(
        found[0].payload, BMP,
        "and it is decompressed on the way out"
    );
}

#[test]
fn a_leaf_with_no_table_above_it_is_filed_under_every_id_it_names() {
    // `table.par` is this shape: payloads at the top level, each naming the ids it serves. A
    // lookup for any of those ids must land; filing only the first is how a whole pack goes
    // silently half-mounted.
    let pack = pack(&[indexed(&[4, 5, 6], &deflate(b"payload"))]);
    let found = read(&pack).unwrap();
    assert_eq!(found.len(), 3);
    assert_eq!(found.iter().map(|r| r.id).collect::<Vec<_>>(), [4, 5, 6]);
    assert!(found.iter().all(|r| r.payload == b"payload"));
    assert_eq!(census(&pack).unwrap().emitted, 3, "one resource per id");
}

#[test]
fn a_payload_named_from_above_keeps_the_parents_id_even_if_its_own_table_differs() {
    // The parent's table is what `LoadResData` will ask with; the leaf's own ids are unrelated
    // and must not steal the name.
    let inner = pack(&[indexed(&[9], &deflate(BMP))]);
    let outer = pack(&[indexed(&[0, 1], &inner)]);
    let found = read(&outer).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, 0);
}

#[test]
fn text_comes_back_under_the_string_kind() {
    // What `ISHELL_LoadResString` needs: UTF-16, NUL-terminated.
    let text: Vec<u8> = "hi\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let pack = pack(&[indexed(&[0], &deflate(&text))]);
    let found = read(&pack).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, RESTYPE_STRING);
}

#[test]
fn a_file_that_is_not_a_pack_is_refused_by_its_first_four_bytes() {
    assert!(!is_par(b"\x11\x00\x00\x00"));
    let err = read(b"\x11\x00\x00\x00rest").unwrap_err();
    assert!(format!("{err}").contains("signature"), "{err}");
}

/// The shape the corpus has in a group of files: a header holding a bare header, whose leaves are
/// gzip members and nothing above them names an id.
///
/// `read` must still return nothing — inventing an id would be a name no pack ever asks for — but
/// the census has to say that the leaves were recognised and merely unnamed, because otherwise the
/// caller cannot tell this file apart from one that is not a container at all.
#[test]
fn a_nested_pack_with_no_ids_names_nothing_and_says_so() {
    let inner = pack(&[deflate(BMP), deflate(BMP)]);
    let outer = pack(&[inner]);
    assert!(read(&outer).unwrap().is_empty());
    let all = leaves(&outer).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|leaf| leaf.id.is_none()));
    assert_eq!(
        census(&outer).unwrap(),
        Census {
            emitted: 0,
            unnamed: 2,
            unreadable: 0
        }
    );
}

/// The two answers come from one walk, so a file the mount can use must count the same way.
#[test]
fn census_agrees_with_read_about_what_came_out() {
    let inner = pack(&[deflate(BMP), deflate(BMP)]);
    let outer = pack(&[indexed(&[5, 6], &inner)]);
    assert_eq!(census(&outer).unwrap().emitted, read(&outer).unwrap().len());
}

#[test]
fn a_corrupt_anonymous_leaf_is_unreadable_when_the_bytes_are_asked_for() {
    let mut bad = Vec::from(*GZIP);
    bad.extend_from_slice(b"bad");
    let pack = pack(&[bad]);
    // The cheap census counts the anonymous stream as unnamed because it does not inflate it.
    assert_eq!(census(&pack).unwrap().unnamed, 1);
    let (all, counted) = leaves_and_census(&pack).unwrap();
    assert!(all.is_empty());
    assert_eq!(
        counted,
        Census {
            emitted: 0,
            unnamed: 0,
            unreadable: 1
        },
        "the inflating walk must not call a corrupt stream merely unnamed"
    );
}

#[test]
fn a_header_whose_table_does_not_add_up_is_refused() {
    // The count says two sub-files, so the first must start at 16 - and this one says 8, which
    // is what a misread of a different format looks like.
    let mut bad = Vec::from(*SIGNATURE);
    bad.extend_from_slice(&2u32.to_le_bytes());
    bad.extend_from_slice(&8u32.to_le_bytes());
    bad.extend_from_slice(&0u32.to_le_bytes());
    bad.extend_from_slice(&[0; 16]);
    assert!(read(&bad).is_err());
}

#[test]
fn a_pack_deeper_than_the_cap_is_cut_off_rather_than_recursed_to() {
    // Each header holds the next one under an index, so the reader has to follow the nested
    // `PAR ` to reach the bottom. The work bound in `walk` cannot see this: a chain of headers
    // emits nothing, so `out` never grows, and a constructed file would otherwise recurse once
    // per twelve bytes until the host's stack is gone - an abort with no report.
    fn nest(depth: usize) -> Vec<u8> {
        let mut inner = pack(&[indexed(&[7], &deflate(BMP))]);
        for _ in 1..depth {
            inner = pack(&[indexed(&[7], &inner)]);
        }
        inner
    }

    let shallow = read(&nest(MAX_NEST as usize)).expect("a plausible pack still reads");
    assert_eq!(
        shallow.len(),
        1,
        "the cap leaves room well past the corpus's two levels"
    );
    assert_eq!(shallow[0].id, 7);

    let deep = read(&nest(MAX_NEST as usize + 2)).expect("an over-deep pack is cut, not fatal");
    assert!(deep.is_empty(), "{deep:?}");
}
