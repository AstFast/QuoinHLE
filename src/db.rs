//! The database store: the file `IDBMgr`, `IDatabase` and `IDBRecord` are a front for.
//!
//! A handset's database *is* a file in the app's own directory, and those three interfaces are a
//! thin shell over the platform layer the firmware supplied: `OEM_DBOpenEx`/`OEM_DBCreate` on the way in,
//! then `OEM_DBRecord{Add,Update,Get,Next,Delete,Count,Free}` for the records. What *is* observable
//! through the API is the record body the reference `AEEDBRecord_Update` assembles before it hands
//! the bytes down: the `AEEDBFileBasicField`/`AEEDBFileField` array from the header with its padding
//! and its zero terminator. This module stores that verbatim.
//!
//! The container around those bodies is no longer a guess either. BREW 3.1.2 ships the platform layer's
//! source, and real database files in the sample set match it byte for byte: an
//! eight-byte `oemdb_header`, a run of `oemdb_rec_header` + body records, and an index at the end
//! (`index_header` plus `index_entry` per live record, sorted by id). [`encode_file`] writes that
//! shape, so a file this host saves is one a handset would read. [`decode_file`] reads it, falls
//! back to scanning the data area when the index is missing or short — which is exactly what the
//! reference `LoadIndex` does — and still accepts the older `QBDB` container
//! this host used before the OEM source turned up, so existing saves do not disappear.
//!
//! Deleted records stay in the data area with `wRecID = AEE_DB_RECID_NULL`; the index is what says
//! which ones are live (`DB_PopulateIdx`, the delivered body). Because this host rewrites the
//! whole file on every save, its own output has no deleted records and no slack.
//!
//! # The serialized record
//!
//! From `AEEDBRecord_Update` read instruction by instruction: each field is a header followed by
//! its payload, every field padded so the next one starts even.
//!
//! * `AEEDB_FT_BYTE`/`WORD`/`DWORD` — `[type][name]` then 1, 2 or 4 bytes. No length in the header:
//!   the type says how many.
//! * every other type — `[type][name][len:u16]` then `len` bytes.
//! * `AEEDB_FT_STRING` is the one type whose payload has its own marker, because the body stores it
//!   in whichever of two forms is smaller: `0x03` and one byte per character when every unit is
//!   below `0x100` (`IsUNICODE`'s high-byte test), otherwise `0xFF` and two bytes per unit. The
//!   length in the header counts the marker, which is why a string's reported length is always its
//!   wide size plus two.
//!
//! The array ends with a zero type byte, which is also why `AEEDB_FT_NONE` cannot be stored: reading
//! stops there.
//!
//! # Where the eight-byte field comes from
//!
//! `AEEDBField` is **eight** bytes on the target and sixteen in this host's generated tables. The
//! header makes the difference itself: `AEEDBFieldType` and `AEEDBFieldName` are `int8` off
//! `_WIN32` and `unsigned` on it, so the target gets `fType@0, fName@1, wDataLen@2, pBuffer@4` while
//! the simulator's reading pads both enums to four. The documented behaviour walks the caller's array in
//! steps of eight (`AEEDBRecord_Update`, `ADD r5, #8`), so a pack compiled for a handset hands over
//! eight-byte records and that is what the runtime reads. [`FIELD_STRIDE`] is that number, and the
//! offsets beside it are the ones the body uses.

use std::collections::{BTreeMap, HashMap};

/// `AEE_DB_RECID_NULL` — the id `GetRecordByID` refuses and the one enumeration parks on when the
/// records run out.
pub const RECID_NULL: u16 = 0xffff;

/// How far apart two `AEEDBField` records sit in the array a caller passes, on the target. See the
/// module docs: the ABI tables answer sixteen, which is the simulator's layout.
pub const FIELD_STRIDE: u32 = 8;
/// `AEEDBField::fType`, the `AEEDBFieldType` the caller states.
pub const FIELD_TYPE: u32 = 0;
/// `AEEDBField::fName`, the `AEEDBFieldName` the caller states.
pub const FIELD_NAME: u32 = 1;
/// `AEEDBField::wDataLen`, read only for the types whose length is not implied.
pub const FIELD_LEN: u32 = 2;
/// `AEEDBField::pBuffer`, the contents.
pub const FIELD_DATA: u32 = 4;

/// `AEEDB_FT_STRING`, the one type stored with a marker byte.
const FT_STRING: u8 = 4;
/// The marker a narrow (whole-string-below-`0x100`) string is stored under.
const STRING_NARROW: u8 = 0x03;
/// The marker a wide string is stored under. The body writes only this byte and skips two, so the
/// second is left as the allocation found it; this host writes zero there, which is the one thing a
/// reader may notice and no reader looks at.
const STRING_WIDE: u8 = 0xff;

/// The signature of the container this host used before the OEM source was available. It is still
/// read so old saves keep working, but [`encode_file`] no longer writes it.
pub const MAGIC: &[u8; 4] = b"QBDB";
/// The version of that older container. A file with any other version is refused as a whole rather
/// than half-read, because a half-read database is a pack editing records that are not there.
pub const VERSION: u16 = 1;

/// `sizeof(oemdb_header)`: `wCurRecCount`, `wLastAssignedRecID`, `dwIndexOffset`.
const OEM_HEADER_LEN: usize = 8;
/// `sizeof(oemdb_rec_header)`: `wRecID`, `wRecSize`.
const OEM_REC_HEADER_LEN: usize = 4;
/// `sizeof(index_header)`: `recCount`, `delCount`.
const OEM_INDEX_HEADER_LEN: usize = 4;
/// `sizeof(index_entry)`: `recID:u32`, `dwOffset:u32`.
const OEM_INDEX_ENTRY_LEN: usize = 8;

/// One field as a caller hands it over: the two enum bytes the target header compiles, and the
/// contents in whichever shape their type implies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// `AEEDBFieldType`.
    pub kind: u8,
    /// `AEEDBFieldName`.
    pub name: u8,
    /// The contents. An `AEEDB_FT_STRING` carries the wide units it points at (up to the
    /// terminator, which the codec adds); every other type carries its bytes.
    pub value: Value,
}

/// A field's contents before they are written into the record's form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Bytes, for every type whose length the caller states.
    Bytes(Vec<u8>),
    /// The units of an `AECHAR` string, terminator excluded.
    Text(Vec<u16>),
}

impl Value {
    fn bytes(&self) -> &[u8] {
        match self {
            Value::Bytes(bytes) => bytes,
            // A string field that arrived as bytes keeps them: the codec would otherwise have to
            // guess an encoding for a caller that already chose one.
            Value::Text(_) => &[],
        }
    }
}

/// One field of a stored record, located rather than copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldView {
    /// `AEEDBFieldType`.
    pub kind: u8,
    /// `AEEDBFieldName`.
    pub name: u8,
    /// The header's size: two bytes for the fixed-width types, four for the rest.
    pub hdr: u16,
    /// The payload's size, which for a string includes its marker byte — see the module docs.
    pub len: u16,
    /// Where the payload starts.
    pub data: usize,
}

/// The field at `at`, or `None` when that offset is not a field.
///
/// `None` covers both answers the platform code gives: the zero terminator (which is how a record
/// ends) and an offset past the bytes this record has. The body does not distinguish them because
/// it read through a platform layer that had already validated the buffer; here the buffer is ours, so
/// the same two answers arrive as one.
pub fn field_at(rec: &[u8], at: usize) -> Option<FieldView> {
    let kind = *rec.get(at)?;
    if kind == 0 || kind >= 9 {
        return None;
    }
    let (hdr, len) = match kind {
        1 => (2u16, 1u16),
        2 => (2, 2),
        3 => (2, 4),
        // `GetDBFileFieldLen`'s default arm: the length comes out of the header, which is four
        // bytes wide. Note this is the *stored* length, marker included, not the string's size.
        _ => {
            let lo = *rec.get(at + 2)?;
            let hi = *rec.get(at + 3)?;
            (4, u16::from_le_bytes([lo, hi]))
        }
    };
    let name = *rec.get(at + 1)?;
    let end = at + hdr as usize + len as usize;
    if end > rec.len() {
        return None;
    }
    Some(FieldView {
        kind,
        name,
        hdr,
        len,
        data: at + hdr as usize,
    })
}

/// One step of `IDBRecord::NextField`: the field after `at`, and where it sits.
///
/// `at == None` is the body's `m_pCurField == 0`, which means "nothing read yet": the step lands on
/// the first field. `None` back means the record ended, and the caller parks its cursor there —
/// which is also why a record that has been enumerated to its end starts over on the next call: the
/// body stores the same zero it started with.
pub fn step(rec: &[u8], at: Option<usize>) -> Option<(usize, FieldView)> {
    let next = match at {
        None => 0,
        Some(at) => {
            let field = field_at(rec, at)?;
            let mut next = at + field.hdr as usize + field.len as usize;
            if next % 2 != 0 {
                next += 1;
            }
            next
        }
    };
    field_at(rec, next).map(|field| (next, field))
}

/// The bytes `AEEDBRecord_Update` would build for these fields, terminator included.
///
/// One pass where the body uses two: it sizes the buffer first because it has only one allocation,
/// and this host sizes a `Vec` as it goes. The arithmetic is the body's, padding included, because
/// those bytes are the record — a reader walking them arrives at the fields only if the writer
/// stepped exactly as far.
pub fn encode(fields: &[Field]) -> Vec<u8> {
    let mut out = Vec::new();
    for field in fields {
        // The marker and payload of a string, or the payload of anything else. A string's payload
        // begins with its marker, which is also why `len` counts it.
        let payload: Vec<u8> = if field.kind == FT_STRING {
            match &field.value {
                Value::Text(units) => write_string(units),
                // The body would hand a `char *` to `aee_wstrsize` whatever the caller meant by it,
                // so the bytes are as much a wide string as anything else. Read them as one.
                Value::Bytes(bytes) => {
                    let units: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                        .take_while(|unit| *unit != 0)
                        .collect();
                    write_string(&units)
                }
            }
        } else if field.kind >= 5 {
            // The length lives in a `uint16` in the header, so a longer payload could not be walked
            // back out; the caller's own `wDataLen` cannot express one either.
            let mut bytes = field.value.bytes().to_vec();
            bytes.truncate(u16::MAX as usize);
            bytes
        } else {
            // BYTE/WORD/DWORD: the type alone says how many bytes, and the body copies exactly that
            // many regardless of what `wDataLen` claims.
            let width = match field.kind {
                1 => 1,
                2 => 2,
                3 => 4,
                _ => 0,
            };
            let mut bytes = field.value.bytes().to_vec();
            bytes.resize(width, 0);
            bytes
        };
        out.push(field.kind);
        out.push(field.name);
        if field.kind == 1 || field.kind == 2 || field.kind == 3 {
            // The fixed-width types carry no length: the header is the type byte and the name byte,
            // and the type alone says how far to read.
            out.extend_from_slice(&payload);
        } else {
            out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
            out.extend_from_slice(&payload);
        }
        if out.len() % 2 != 0 {
            out.push(0);
        }
    }
    // The terminator is a zero type byte, and it is part of the length the body reports to the OEM
    // layer, so it belongs in the buffer rather than beside it.
    out.push(0);
    out
}

/// A string's stored payload: the marker that says which form, then the characters.
fn write_string(units: &[u16]) -> Vec<u8> {
    // `IsUNICODE`'s test, which is what the body branches on: any unit with a bit above the low
    // byte makes the string wide, and a string that is entirely Latin-1 is stored one byte per
    // character. The choice is invisible through the API — `GetField` converts back either way —
    // but it decides the bytes on disk, so it is made the same way.
    let wide = units.iter().any(|unit| *unit > 0xff);
    let mut out = Vec::with_capacity(units.len() * 2 + 2);
    if wide {
        out.push(STRING_WIDE);
        // The body writes one byte and advances two; the gap is not a character.
        out.push(0);
        for unit in units {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    } else {
        out.push(STRING_NARROW);
        out.extend(units.iter().map(|unit| *unit as u8));
        out.push(0);
    }
    out
}

/// Read a stored string payload back into wide units.
///
/// The narrow form is one byte per character; the wide form is two. Both stop at the terminator the
/// writer put there, and neither trusts a payload without one.
pub fn read_string(payload: &[u8]) -> Vec<u16> {
    let mut units = Vec::new();
    match payload.first() {
        Some(&STRING_NARROW) => {
            for byte in &payload[1..] {
                if *byte == 0 {
                    break;
                }
                units.push(*byte as u16);
            }
        }
        Some(&STRING_WIDE) => {
            for pair in payload[2..].chunks_exact(2) {
                let unit = u16::from_le_bytes([pair[0], pair[1]]);
                if unit == 0 {
                    break;
                }
                units.push(unit);
            }
        }
        _ => {}
    }
    units
}

/// One record: the id the platform layer assigned it, and the bytes it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: u16,
    /// The serialized field array, exactly as [`encode`] built it.
    pub bytes: Vec<u8>,
}

/// An `IDatabase` object that is open: the name it was opened under, and where enumeration is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    pub name: String,
    /// The id the last `GetNextRecord` answered, or [`RECID_NULL`] for "nothing enumerated yet" —
    /// which is where `Reset` and the constructor leave it.
    pub cursor: u16,
    /// Whether the last walk ran off the end of the records.
    ///
    /// It is a separate flag because the documented behaviour cannot tell the two states apart: it stores
    /// the same `0xFFFF` when it resets and when `OEM_DBRecordNext` answers [`RECID_NULL`], and
    /// passes it back as the "where I got to" argument. A real platform layer kept a file position of its
    /// own, so a walk that reached the end stayed there — which is what makes the canonical
    /// `Reset; while (GetNextRecord)` loop terminate, and therefore what this store reproduces.
    /// `Reset` clears it; nothing else does.
    pub done: bool,
}

/// An `IDBRecord` object that has been handed out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    /// The `IDatabase` object this record came out of. Its address, not a copy of it, because
    /// `Update` and `Remove` write through to that database.
    pub db: u32,
    pub id: u16,
    /// This record's bytes at the moment it was loaded. `Update` replaces them here and in the
    /// store; nothing else writes them, so a record is as stale as the body's own copy.
    pub bytes: Vec<u8>,
    /// `m_pCurField` as an offset, with the body's zero spelled `None`.
    pub at: Option<usize>,
    /// Where [`Holder::bytes`] was copied for the guest to read through — the address `GetField`
    /// answers for a fixed-width or wide field is `block + offset`.
    pub block: u32,
    /// The wide copy `GetField` makes for a narrow string (the body `aee_malloc`s one per call).
    pub wide: u32,
    /// How many bytes `block` holds, so a call that needs a bigger one can tell.
    pub block_len: u32,
}

/// Everything the three DB interfaces keep between calls.
#[derive(Default)]
pub struct Databases {
    /// The databases themselves, by the name a pack opened them under. This is the store: an open
    /// handle reads through it, so two handles on one name see the same records, which is what the
    /// header promises ("if the database is already open, it returns the interface object for the
    /// already open database").
    files: BTreeMap<String, Vec<Record>>,
    open: HashMap<u32, Open>,
    records: HashMap<u32, Holder>,
}

impl Databases {
    /// Attach a fresh `IDatabase` object to a name and its records.
    ///
    /// A name that is already in the store keeps the records it has: the header promises that a
    /// second open of a database already open answers the existing one, and even a pack that got a
    /// second object would not expect the records written since to disappear.
    pub fn attach(&mut self, addr: u32, name: String, records: Vec<Record>) {
        self.files.entry(name.clone()).or_insert(records);
        self.open.insert(
            addr,
            Open {
                name,
                cursor: RECID_NULL,
                done: false,
            },
        );
    }

    /// The handle for an object, which is also how the store learns which database a call means.
    pub fn open(&self, addr: u32) -> Option<&Open> {
        self.open.get(&addr)
    }

    /// Forget an `IDatabase` object, answering what it was so the caller can write it back.
    ///
    /// The record objects that came out of it are left alone: they hold guest blocks that their own
    /// releases free, and a record of a database that is no longer open is a state the documented behaviour
    /// has too — `Update` on one answers `AEE_DB_EBADREC`, because the handle it writes through is
    /// gone.
    pub fn detach(&mut self, addr: u32) -> Option<Open> {
        self.open.remove(&addr)
    }

    /// The records stored under a name.
    pub fn file(&self, name: &str) -> Option<&Vec<Record>> {
        self.files.get(name)
    }

    /// The records stored under a name, for a caller that is about to change them.
    pub fn file_mut(&mut self, name: &str) -> Option<&mut Vec<Record>> {
        self.files.get_mut(name)
    }

    /// Whether a name is already open. A database file may only be removed when it is not.
    ///
    /// Asked as a yes-or-no rather than "which handle", because one name may have several: the
    /// documented behaviour answers each `OpenDatabase` with a fresh object, so naming *one* of them would be
    /// a choice this store has no reason to make (and, over a hash map, a nondeterministic one).
    pub fn is_open(&self, name: &str) -> bool {
        self.open.values().any(|open| open.name == name)
    }

    /// Forget a database by name, which is what `IDBMgr::Remove` does to the store after it deletes
    /// the file: the records are the database, so nothing may outlive the file they came from.
    pub fn forget(&mut self, name: &str) -> Option<Vec<Record>> {
        self.files.remove(name)
    }

    /// `IDatabase::GetRecordCount`.
    pub fn count(&self, name: &str) -> u16 {
        self.files
            .get(name)
            .map_or(0, |records| records.len() as u16)
    }

    /// `IDatabase::Reset`.
    pub fn rewind(&mut self, addr: u32) {
        if let Some(open) = self.open.get_mut(&addr) {
            open.cursor = RECID_NULL;
            open.done = false;
        }
    }

    /// `IDatabase::GetNextRecord`'s two halves: the id the OEM walk answers, and the record it
    /// names. `None` is the end of the records, which also parks the walk.
    pub fn next(&mut self, addr: u32) -> Option<Record> {
        let open = self.open.get(&addr)?;
        if open.done {
            return None;
        }
        let records = self.files.get(&open.name)?;
        let index = match open.cursor {
            RECID_NULL => 0,
            id => records.iter().position(|record| record.id == id)? + 1,
        };
        let found = records.get(index).cloned();
        let Some(record) = found else {
            if let Some(open) = self.open.get_mut(&addr) {
                open.cursor = RECID_NULL;
                open.done = true;
            }
            return None;
        };
        if let Some(open) = self.open.get_mut(&addr) {
            open.cursor = record.id;
        }
        Some(record)
    }

    /// `IDatabase::GetRecordByID`.
    pub fn by_id(&self, addr: u32, id: u16) -> Option<Record> {
        if id == RECID_NULL {
            return None;
        }
        let open = self.open.get(&addr)?;
        self.files
            .get(&open.name)?
            .iter()
            .find(|record| record.id == id)
            .cloned()
    }

    /// Append a record and answer the id it was given.
    ///
    /// The first record of a fresh database is id zero: `GetRecordByID` refuses only
    /// [`RECID_NULL`], so zero is a record like any other, and ids are never reused so a stale one
    /// read out of a save file cannot silently name a different record.
    pub fn add(&mut self, name: &str, bytes: Vec<u8>) -> u16 {
        let records = self.files.entry(name.to_owned()).or_default();
        let id = records
            .iter()
            .map(|record| record.id as u32 + 1)
            .max()
            .unwrap_or(0) as u16;
        records.push(Record { id, bytes });
        id
    }

    /// Replace a record's bytes. Answers whether the id was there.
    pub fn replace(&mut self, name: &str, id: u16, bytes: Vec<u8>) -> bool {
        let Some(records) = self.files.get_mut(name) else {
            return false;
        };
        match records.iter_mut().find(|record| record.id == id) {
            Some(record) => {
                record.bytes = bytes;
                true
            }
            None => false,
        }
    }

    /// Drop a record. Answers whether the id was there.
    pub fn remove(&mut self, name: &str, id: u16) -> bool {
        let Some(records) = self.files.get_mut(name) else {
            return false;
        };
        let before = records.len();
        records.retain(|record| record.id != id);
        records.len() != before
    }

    /// Take a record object under this host's wing.
    pub fn hold(&mut self, addr: u32, holder: Holder) {
        self.records.insert(addr, holder);
    }

    pub fn holder(&self, addr: u32) -> Option<&Holder> {
        self.records.get(&addr)
    }

    pub fn holder_mut(&mut self, addr: u32) -> Option<&mut Holder> {
        self.records.get_mut(&addr)
    }

    /// Forget a record object, answering the blocks it was lent so the caller can free them.
    pub fn release(&mut self, addr: u32) -> Option<Holder> {
        self.records.remove(&addr)
    }
}

/// The bytes of a database file, in the reference OEM format.
///
/// The record bodies are opaque here — this layer only frames them — which is the same division the
/// platform layer had, and the reason a change to the field codec does not invalidate a save file.
pub fn encode_file(records: &[Record]) -> Vec<u8> {
    // `DB_WriteIndex` seeks to EOF, records that offset in `dwIndexOffset`, then writes the index
    // there. So the index starts just past the last record.
    let index_at = OEM_HEADER_LEN
        + records
            .iter()
            .map(|r| OEM_REC_HEADER_LEN + r.bytes.len())
            .sum::<usize>();
    let mut out =
        Vec::with_capacity(index_at + OEM_INDEX_HEADER_LEN + records.len() * OEM_INDEX_ENTRY_LEN);
    let last_id = records.iter().map(|r| r.id).max().unwrap_or(0);
    out.extend_from_slice(&(records.len() as u16).to_le_bytes());
    out.extend_from_slice(&last_id.to_le_bytes());
    out.extend_from_slice(&(index_at as u32).to_le_bytes());

    let mut placed: Vec<(u16, u32)> = Vec::with_capacity(records.len());
    for record in records {
        let offset = out.len() as u32;
        out.extend_from_slice(&record.id.to_le_bytes());
        // `wRecSize` is a word on the target too. A body longer than that has no representation in
        // this format; the reference allocator would have refused it long before it got here.
        out.extend_from_slice(&(record.bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&record.bytes);
        placed.push((record.id, offset));
    }
    debug_assert_eq!(out.len(), index_at);

    // Every save is a clean rewrite, so `delCount` is zero: a handset instead appends an updated
    // record and marks the old copy deleted, which is why its index is not in physical order.
    out.extend_from_slice(&(placed.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    // `DB_PopulateIdx` ends with `QSORT(..., CmpRecID)` and `LoadIndex` trusts that order.
    placed.sort_by_key(|&(id, _)| id);
    for (id, offset) in placed {
        out.extend_from_slice(&(id as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
    }
    out
}

/// Read a database file, or `None` for bytes that are not one.
///
/// All or nothing: a count that does not match the bytes or a record that runs past the end answers
/// `None`, because a database half read from a damaged file would have a pack editing records that
/// are not there. [`MAGIC`] selects the older container this host wrote before the OEM source was
/// available; anything else is read as a handset's file.
pub fn decode_file(bytes: &[u8]) -> Option<Vec<Record>> {
    if bytes.starts_with(MAGIC) {
        decode_legacy(bytes)
    } else {
        decode_oem(bytes)
    }
}

fn decode_legacy(bytes: &[u8]) -> Option<Vec<Record>> {
    fn take<'a>(at: &mut usize, n: usize, bytes: &'a [u8]) -> Option<&'a [u8]> {
        let end = at.checked_add(n)?;
        let out = bytes.get(*at..end)?;
        *at = end;
        Some(out)
    }
    let mut at = 0usize;
    if take(&mut at, 4, bytes)? != MAGIC {
        return None;
    }
    let version = u16::from_le_bytes(take(&mut at, 2, bytes)?.try_into().ok()?);
    if version != VERSION {
        return None;
    }
    take(&mut at, 2, bytes)?;
    let count = u32::from_le_bytes(take(&mut at, 4, bytes)?.try_into().ok()?);
    let mut records = Vec::new();
    for _ in 0..count {
        let id = u16::from_le_bytes(take(&mut at, 2, bytes)?.try_into().ok()?);
        let len = u16::from_le_bytes(take(&mut at, 2, bytes)?.try_into().ok()?) as usize;
        records.push(Record {
            id,
            bytes: take(&mut at, len, bytes)?.to_vec(),
        });
    }
    // Trailing bytes mean the file was written by something that disagreed with itself, and reading
    // the prefix would hide that.
    (at == bytes.len()).then_some(records)
}

fn decode_oem(bytes: &[u8]) -> Option<Vec<Record>> {
    if bytes.len() < OEM_HEADER_LEN {
        return None;
    }
    let word = |at: usize| -> Option<u16> {
        Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
    };
    let dword = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
    };
    let index_at = dword(4)? as usize;
    if index_at == 0 {
        // `DB_Create` writes an all-zero header and leaves `dwIndexOffset` at zero until the first
        // `DB_WriteIndex`; `RemoveIndexFromFile` truncates back to this shape
        // too. `LoadIndex` then rebuilds by walking the data area to EOF.
        return scan_oem_data(bytes, OEM_HEADER_LEN, bytes.len());
    }
    if index_at < OEM_HEADER_LEN || index_at + OEM_INDEX_HEADER_LEN > bytes.len() {
        return None;
    }
    let live = word(index_at)? as usize;
    let index_len = OEM_INDEX_HEADER_LEN + live * OEM_INDEX_ENTRY_LEN;
    if index_at + index_len > bytes.len() {
        // A short index is what makes `LoadIndex` fall through to `DB_PopulateIdx`: it discards the
        // index and rebuilds from the records instead of refusing the file.
        return scan_oem_data(bytes, OEM_HEADER_LEN, index_at);
    }
    // `LoadIndex` reads exactly `recCount` entries and ignores anything past them; so do we.
    let mut records = Vec::with_capacity(live);
    for i in 0..live {
        let entry = index_at + OEM_INDEX_HEADER_LEN + i * OEM_INDEX_ENTRY_LEN;
        let Some(id) = dword(entry) else {
            return scan_oem_data(bytes, OEM_HEADER_LEN, index_at);
        };
        let Some(at) = dword(entry + 4) else {
            return scan_oem_data(bytes, OEM_HEADER_LEN, index_at);
        };
        // An index entry names both the id and the offset. If the record at that offset disagrees,
        // the index is not trustworthy. `LoadIndex` answers that case by throwing the index away
        // and rebuilding from the data area, which uses the record headers' own ids.
        let found = (id <= RECID_NULL as u32)
            .then(|| read_oem_record(bytes, at as usize, index_at))
            .flatten()
            .filter(|(physical, _)| *physical as u32 == id);
        let Some((_, body)) = found else {
            return scan_oem_data(bytes, OEM_HEADER_LEN, index_at);
        };
        records.push(Record {
            id: id as u16,
            bytes: body.to_vec(),
        });
    }
    Some(records)
}

/// Walk `oemdb_rec_header` + body pairs from `at` to `end`, skipping the records a handset marked
/// deleted with `AEE_DB_RECID_NULL`, and answer them in id order like the reference `QSORT` does.
fn scan_oem_data(bytes: &[u8], at: usize, end: usize) -> Option<Vec<Record>> {
    let mut records = Vec::new();
    let mut at = at;
    while at + OEM_REC_HEADER_LEN <= end {
        let id = u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?);
        let len = u16::from_le_bytes(bytes.get(at + 2..at + 4)?.try_into().ok()?) as usize;
        let body_at = at + OEM_REC_HEADER_LEN;
        let stop = body_at.checked_add(len)?;
        if stop > end {
            return None;
        }
        if id != RECID_NULL {
            records.push(Record {
                id,
                bytes: bytes[body_at..stop].to_vec(),
            });
        }
        at = stop;
    }
    (at == end).then(|| {
        records.sort_by_key(|r| r.id);
        records
    })
}

fn read_oem_record(bytes: &[u8], at: usize, data_end: usize) -> Option<(u16, &[u8])> {
    if at + OEM_REC_HEADER_LEN > data_end {
        return None;
    }
    let id = u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?);
    let len = u16::from_le_bytes(bytes.get(at + 2..at + 4)?.try_into().ok()?) as usize;
    let body_at = at + OEM_REC_HEADER_LEN;
    let stop = body_at.checked_add(len)?;
    if id == RECID_NULL || stop > data_end {
        return None;
    }
    Some((id, bytes.get(body_at..stop)?))
}

#[cfg(test)]
#[path = "test/db.rs"]
mod tests;
