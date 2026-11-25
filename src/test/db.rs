//! The database store's own tests: the bytes [`encode`] builds and the walk [`step`] takes.
//!
//! Every expectation here is derived from `AEEDBRecord_Update` and `AEEDBRecord_NextField` read
//! instruction by instruction, so a failure means the codec drifted from the documented behaviour rather
//! than that a format was chosen differently. The two are worth telling apart, which is why the
//! arithmetic appears in the comments rather than only in the assertions.

use crate::db::*;

/// A `Value::Text` from a `&str`, which is how the tests spell a string field.
fn text(s: &str) -> Value {
    Value::Text(s.encode_utf16().collect())
}

fn field(kind: u8, name: u8, value: Value) -> Field {
    Field { kind, name, value }
}

#[test]
fn a_dword_field_is_two_header_bytes_and_four_of_data() {
    let bytes = encode(&[field(
        3,
        1,
        Value::Bytes(0x1122_3344u32.to_le_bytes().to_vec()),
    )]);
    assert_eq!(
        bytes,
        vec![3, 1, 0x44, 0x33, 0x22, 0x11, 0],
        "header (type, name), the four bytes, no padding (already even), then the terminator"
    );
}

#[test]
fn a_byte_field_pads_before_the_next_one() {
    // 2 header + 1 data = 3, so the writer pads one byte before the second field starts at 4.
    let bytes = encode(&[
        field(1, 1, Value::Bytes(vec![0x7f])),
        field(2, 2, Value::Bytes(0xabcd_u16.to_le_bytes().to_vec())),
    ]);
    assert_eq!(bytes, vec![1, 1, 0x7f, 0x00, 2, 2, 0xcd, 0xab, 0]);
}

#[test]
fn a_narrow_string_stores_one_byte_per_character() {
    // "Hi": wstrsize is 2*(2+1) = 6; the narrow branch halves it to 3 and adds one for the length
    // field's "marker included" reading, so the header says 4 and the payload is `03 48 69 00`.
    let bytes = encode(&[field(4, 1, text("Hi"))]);
    assert_eq!(bytes, vec![4, 1, 4, 0, 0x03, b'H', b'i', 0x00, 0]);
}

#[test]
fn a_wide_string_stores_two_bytes_per_unit_and_the_marker_is_counted() {
    // The unit 0x4e2d (中) has a high byte, so the string is wide: marker 2 + 2*(1+1) = 6, which is
    // what the length field says, and the payload carries the terminator the body copies with it.
    let bytes = encode(&[field(4, 1, text("\u{4e2d}"))]);
    assert_eq!(bytes, vec![4, 1, 6, 0, 0xff, 0x00, 0x2d, 0x4e, 0, 0, 0]);
}

#[test]
fn a_string_is_always_walked_as_a_wide_pointer() {
    // `NextField` reports the string's stored length (marker included), not its character count.
    let bytes = encode(&[field(4, 1, text("Hi"))]);
    let (at, seen) = step(&bytes, None).expect("the first field");
    assert_eq!(
        (at, seen.kind, seen.name, seen.len, seen.hdr),
        (0, 4, 1, 4, 4)
    );
    assert_eq!(
        read_string(&bytes[seen.data..seen.data + seen.len as usize]),
        vec![b'H' as u16, b'i' as u16]
    );
}

#[test]
fn the_walk_reads_every_field_and_then_stops_at_the_terminator() {
    let bytes = encode(&[
        field(3, 1, Value::Bytes(7u32.to_le_bytes().to_vec())),
        field(4, 2, text("ab")),
        field(1, 3, Value::Bytes(vec![9])),
    ]);
    let mut at = None;
    let mut seen = Vec::new();
    while let Some((next, view)) = step(&bytes, at) {
        seen.push((view.kind, view.name));
        at = Some(next);
        assert!(seen.len() < 16, "the walk must terminate");
    }
    assert_eq!(seen, vec![(3, 1), (4, 2), (1, 3)]);
}

#[test]
fn the_end_of_a_record_parks_the_walk_where_it_started() {
    // The body stores zero in `m_pCurField` when a field read fails, and zero is also "nothing read
    // yet" — so a record enumerated to its end starts over. This is that, and it is the reference
    // shape rather than a convenience.
    let bytes = encode(&[field(3, 1, Value::Bytes(1u32.to_le_bytes().to_vec()))]);
    let (first, _) = step(&bytes, None).expect("the only field");
    assert_eq!(step(&bytes, Some(first)), None, "past the last field");
    let (again, view) = step(&bytes, None).expect("the walk restarts");
    assert_eq!((again, view.name), (0, 1));
}

#[test]
fn a_record_needs_only_its_terminator() {
    assert_eq!(encode(&[]), vec![0]);
    assert_eq!(step(&[0], None), None);
}

#[test]
fn a_header_that_runs_past_its_bytes_is_not_a_field() {
    // A four-byte header that claims 40 bytes of payload in a 6-byte record. The documented behaviour read
    // this through a platform layer that had validated it; here it has to answer "no field".
    assert_eq!(field_at(&[5, 1, 40, 0], 0), None);
}

#[test]
fn a_file_round_trips_with_ids_and_bodies_intact() {
    let records = vec![
        Record {
            id: 0,
            bytes: encode(&[field(3, 1, Value::Bytes(1u32.to_le_bytes().to_vec()))]),
        },
        Record {
            id: 7,
            bytes: encode(&[field(4, 2, text("game"))]),
        },
    ];
    assert_eq!(decode_file(&encode_file(&records)), Some(records));
}

#[test]
fn the_container_is_the_shipped_oem_layout() {
    // `oemdb_header` / `oemdb_rec_header` / `index_header` / `index_entry` from the delivered database layer.
    // One three-byte record: 8 header + (4 + 3) data + 4 index header + 8 index entry.
    let f = encode_file(&[Record {
        id: 7,
        bytes: vec![1, 2, 3],
    }]);
    assert_eq!(f.len(), 27);
    assert_eq!(&f[0..2], &1u16.to_le_bytes(), "wCurRecCount");
    assert_eq!(&f[2..4], &7u16.to_le_bytes(), "wLastAssignedRecID");
    assert_eq!(
        &f[4..8],
        &15u32.to_le_bytes(),
        "dwIndexOffset, past the last record"
    );
    assert_eq!(&f[8..10], &7u16.to_le_bytes(), "wRecID");
    assert_eq!(&f[10..12], &3u16.to_le_bytes(), "wRecSize");
    assert_eq!(&f[12..15], &[1, 2, 3]);
    assert_eq!(&f[15..17], &1u16.to_le_bytes(), "index recCount");
    assert_eq!(
        &f[17..19],
        &0u16.to_le_bytes(),
        "index delCount: a clean rewrite"
    );
    assert_eq!(&f[19..23], &7u32.to_le_bytes(), "index entry recID");
    assert_eq!(&f[23..27], &8u32.to_le_bytes(), "index entry dwOffset");
}

#[test]
fn a_record_body_from_a_handset_file_walks_as_fields() {
    // `Straces web/MobileIM/mobileim/ins_aol`, the record at file offset 3702: a narrow STRING
    // ("Dummy") followed by a DWORD. These bytes came off a handset rather than out of `encode`,
    // so they check the codec against real saved data as well as against itself.
    let body = [
        0x04, 0x00, 0x07, 0x00, 0x03, b'D', b'u', b'm', b'm', b'y', 0x00, 0x00, // + pad
        0x03, 0x00, 0x2a, 0x00, 0x00, 0x00, // DWORD = 42
        0x00, // the record's terminator
    ];
    let (at, first) = step(&body, None).unwrap();
    assert_eq!(at, 0);
    assert_eq!((first.kind, first.name, first.hdr, first.len), (4, 0, 4, 7));
    assert_eq!(
        read_string(&body[first.data..first.data + first.len as usize]),
        "Dummy".encode_utf16().collect::<Vec<_>>()
    );
    let (at, second) = step(&body, Some(at)).unwrap();
    assert_eq!(
        at, 12,
        "four header bytes plus seven of payload, padded to even"
    );
    assert_eq!((second.kind, second.hdr, second.len), (3, 2, 4));
    assert_eq!(&body[second.data..second.data + 4], &42u32.to_le_bytes());
    assert!(
        step(&body, Some(at)).is_none(),
        "then the terminator ends it"
    );
}

/// A handset's data area after updates: a deleted record left in place, then two live ones whose
/// ids are in reverse physical order because the newer copy was appended at the end.
fn handset_data_area() -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&RECID_NULL.to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&[0xaa, 0xbb]);
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&3u16.to_le_bytes());
    f.extend_from_slice(&[2, 0, 0]);
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&3u16.to_le_bytes());
    f.extend_from_slice(&[1, 0, 0]);
    f
}

fn live_pair() -> Vec<Record> {
    vec![
        Record {
            id: 1,
            bytes: vec![1, 0, 0],
        },
        Record {
            id: 2,
            bytes: vec![2, 0, 0],
        },
    ]
}

#[test]
fn a_handset_index_decides_which_records_are_live_and_in_what_order() {
    // Header: two live records, max id 2, the index starts at 8 + 6 + 7 + 7 = 28.
    let mut f = Vec::new();
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&28u32.to_le_bytes());
    f.extend_from_slice(&handset_data_area());
    assert_eq!(f.len(), 28);
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&1u32.to_le_bytes());
    f.extend_from_slice(&21u32.to_le_bytes());
    f.extend_from_slice(&2u32.to_le_bytes());
    f.extend_from_slice(&14u32.to_le_bytes());
    assert_eq!(decode_file(&f), Some(live_pair()));
}

#[test]
fn a_file_without_an_index_is_rebuilt_from_the_data_area() {
    // `dwIndexOffset == 0` is both a fresh `DB_Create` and the state `RemoveIndexFromFile`
    // leaves. `LoadIndex` then falls through to `DB_PopulateIdx`.
    let mut f = vec![0u8; 8];
    f.extend_from_slice(&handset_data_area());
    assert_eq!(decode_file(&f), Some(live_pair()));
    assert_eq!(
        decode_file(&[0u8; 8]),
        Some(Vec::new()),
        "a fresh empty file"
    );
    assert_eq!(decode_file(&encode_file(&[])), Some(Vec::new()));
    assert_eq!(
        encode_file(&[]).len(),
        12,
        "header plus an empty index header"
    );
}

#[test]
fn a_short_index_is_discarded_and_rebuilt() {
    let mut f = Vec::new();
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&28u32.to_le_bytes());
    f.extend_from_slice(&handset_data_area());
    f.extend_from_slice(&9u16.to_le_bytes()); // claims nine entries, has none
    f.extend_from_slice(&0u16.to_le_bytes());
    assert_eq!(decode_file(&f), Some(live_pair()));
}

#[test]
fn an_old_qbdb_save_is_still_readable() {
    // The container this host wrote before the delivered body was available. Existing saves must survive
    // the format change; `decode_file` keys off the signature to pick the reader.
    let mut old = Vec::new();
    old.extend_from_slice(MAGIC);
    old.extend_from_slice(&VERSION.to_le_bytes());
    old.extend_from_slice(&0u16.to_le_bytes());
    old.extend_from_slice(&1u32.to_le_bytes());
    old.extend_from_slice(&7u16.to_le_bytes());
    old.extend_from_slice(&2u16.to_le_bytes());
    old.extend_from_slice(&[0xab, 0xcd]);
    assert_eq!(
        decode_file(&old),
        Some(vec![Record {
            id: 7,
            bytes: vec![0xab, 0xcd],
        }])
    );
}

#[test]
fn a_damaged_file_is_refused_whole() {
    let file = encode_file(&[Record {
        id: 0,
        bytes: vec![0],
    }]);
    let mut bad_offset = file.clone();
    bad_offset[4..8].copy_from_slice(&0xffff_fff0u32.to_le_bytes());
    assert_eq!(
        decode_file(&bad_offset),
        None,
        "an index that is not in the file"
    );
    // The two ways an index entry can lie about a record. `LoadIndex` does not trust it in either
    // case: it drops the index and rebuilds from the data area, so the one real record survives.
    let whole = Some(vec![Record {
        id: 0,
        bytes: vec![0],
    }]);
    let mut bad_entry = file.clone();
    bad_entry[17..21].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(decode_file(&bad_entry), whole, "an id no record has");
    let mut bad_position = file.clone();
    bad_position[21..25].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        decode_file(&bad_position),
        whole,
        "an entry pointing into the header"
    );
    // A record that claims to run into the index cannot be rebuilt around either.
    let mut too_long = file.clone();
    too_long[10..12].copy_from_slice(&0xffffu16.to_le_bytes());
    assert_eq!(
        decode_file(&too_long),
        None,
        "a record that runs into the index"
    );
    assert_eq!(decode_file(b"NOT-A-QBDB-FILE"), None, "not ours");
    // `LoadIndex` reads exactly `recCount` entries and ignores the rest, so the reference reader
    // tolerates slack at the end of a file. This host has to agree or it would delete such a file.
    let mut slack = file.clone();
    slack.push(0);
    assert_eq!(decode_file(&slack), decode_file(&file));
}

#[test]
fn ids_are_never_reused() {
    let mut dbs = Databases::default();
    dbs.attach(0x1000, "chess".to_owned(), Vec::new());
    assert_eq!(dbs.add("chess", vec![0]), 0);
    assert_eq!(dbs.add("chess", vec![0]), 1);
    assert_eq!(dbs.add("chess", vec![0]), 2);
    assert!(dbs.remove("chess", 1));
    // A saved record id outlives the record that had it, so the next one takes 3 and a stale 1
    // cannot name a different record.
    assert_eq!(dbs.add("chess", vec![0]), 3);
    assert_eq!(dbs.count("chess"), 3);
}

#[test]
fn enumeration_walks_in_the_order_the_records_were_added() {
    let mut dbs = Databases::default();
    dbs.attach(0x1000, "chess".to_owned(), Vec::new());
    for _ in 0..3 {
        dbs.add("chess", vec![0]);
    }
    let mut seen = Vec::new();
    while let Some(record) = dbs.next(0x1000) {
        seen.push(record.id);
    }
    assert_eq!(seen, vec![0, 1, 2]);
    // The end parks the cursor, so a second pass needs a `Reset` — the header says exactly that.
    assert_eq!(dbs.next(0x1000), None);
    dbs.rewind(0x1000);
    assert_eq!(dbs.next(0x1000).map(|r| r.id), Some(0));
}

#[test]
fn a_cursor_whose_record_was_removed_ends_the_walk() {
    let mut dbs = Databases::default();
    dbs.attach(0x1000, "chess".to_owned(), Vec::new());
    dbs.add("chess", vec![0]);
    dbs.add("chess", vec![0]);
    assert_eq!(dbs.next(0x1000).map(|r| r.id), Some(0));
    dbs.remove("chess", 0);
    // The record the cursor named is gone, so the OEM walk has nothing to count from. Answering the
    // first record instead would hand a pack the same record twice.
    assert_eq!(dbs.next(0x1000), None);
}

#[test]
fn one_name_is_one_database_however_many_handles() {
    let mut dbs = Databases::default();
    dbs.attach(0x1000, "chess".to_owned(), Vec::new());
    dbs.attach(0x2000, "chess".to_owned(), Vec::new());
    // The second attach re-reads the same name, so the store is the same list: a record added
    // through one handle is found by the other.
    dbs.add("chess", vec![1]);
    assert!(dbs.is_open("chess"));
    assert_eq!(dbs.by_id(0x2000, 0).map(|r| r.bytes), Some(vec![1]));
    // Two handles on one name is a state the store allows, so "open" is a question about the name
    // and stays true until the last of them goes. The documented behaviour hands out a fresh object for
    // every open, so this is also the state a pack that opened twice is really in.
    dbs.detach(0x1000);
    assert!(dbs.is_open("chess"));
    dbs.detach(0x2000);
    assert!(!dbs.is_open("chess"));
}

#[test]
fn a_record_outlives_the_database_it_came_from() {
    let mut dbs = Databases::default();
    dbs.attach(0x1000, "chess".to_owned(), Vec::new());
    dbs.hold(
        0x3000,
        Holder {
            db: 0x1000,
            id: 0,
            bytes: vec![0],
            at: None,
            block: 0,
            wide: 0,
            block_len: 0,
        },
    );
    assert!(dbs.detach(0x1000).is_some());
    // The record keeps its bytes and its cursor — releasing the database cannot take back a pointer
    // the pack still holds, and the reference `Release` documents that it cannot even close while one
    // is out. What it loses is the database behind it.
    assert_eq!(dbs.holder(0x3000).map(|h| h.id), Some(0));
    assert!(dbs.open(0x1000).is_none());
}
