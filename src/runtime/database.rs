//! The database store behind `IDBMgr`/`IDatabase`/`IDBRecord`.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// Every `IBCMCSDB` slot. Broadcast/multicast service management is a network technology this
    /// host does not have, and neither slot has a published return set to follow: the header gives the
    /// C++ declaration, the two vtable entries and a
    /// comment saying what `ExtFunc` is *for* — "Extension Function to allow future enhancement to the
    /// interface without breaking its binary compatibility" — and nothing about success
    /// codes.
    ///
    /// So both slots take the answer this repository uses for a capability the machine does not have:
    /// the named `EUNSUPPORTED`, which is also what `ExtFunc`'s twin in `INetUtils` answers. ⚠ The
    /// basis is the **signature**, not something unread: `Update` takes an `IxOpts *` record plus a
    /// mode word, `ExtFunc` an extension class id, and this host runs no BCMCS flow and defines no
    /// extension class.
    pub(crate) fn ibcmcsdb(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(
                    this,
                    iid,
                    ppv,
                    &[g::AEEIID_IBCMCSDB, g::AEECLSID_BCMCSDATABASE],
                )
            }
            "Update" | "ExtFunc" => Ok(g::AEE_EUNSUPPORTED),
            _ => Err(unimplemented()),
        }
    }

    /// The `IDBMgr` — the database manager (`AEEDBMgr_New`, the delivered body).
    ///
    /// The manager is the one DB object that needs no OEM state to build: its constructor allocates
    /// a record and sets the reference count, nothing else. `OpenDatabase` is where the documented behaviour
    /// reaches `OEM_DBOpenEx`/`OEM_DBCreate`, and that layer lived in the handset's
    /// firmware — so this host stands one up, which is what [`crate::db`] is: the record bodies the
    /// reference `AEEDBRecord_Update` builds, stored in a file in the app's own volume, behind the
    /// same three interfaces.
    ///
    /// `IDBMgr` inherits only `IBase`: the reference vtable is AddRef/Release/OpenDatabase/Remove/
    /// SetCacheSize, with **no `QueryInterface` slot**, so there is none to answer.
    pub(crate) fn idbmgr(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `IDatabase *IDBMgr_OpenDatabase(me, const char *pszFile, boolean bCreate, uint32
            // dwMinSize, uint16 wMinRecs)`. The name is not validated here because the documented behaviour
            // does not validate it either — it hands it straight to the OEM, which is
            // the volume's own lookup. `dwMinSize`/`wMinRecs` are the OEM's preallocation hints
            // (the delivered body passes them straight to `OEM_DBCreate`): they decide how much room a
            // handset's flash file reserves, and this host's file grows as records are added, so
            // there is nothing to reserve and the two are read and dropped.
            "OpenDatabase" => {
                let [file, create, _min_size, _min_recs] = self.args()?;
                let name = self.cstring(file)?;
                if name.is_empty() {
                    // `OEM_DBOpenEx` was handed a null or empty name; the documented behaviour does not
                    // check it and the platform layer answered "not found", which is the NULL this gives.
                    return Ok(0);
                }
                self.db_open(&name, create != 0)
            }
            // `int IDBMgr_Remove(me, const char *pszFile)` — the body turns **any** OEM error into
            // `EBADFILENAME`, and the header documents exactly that code for
            // "database file could not be found".
            "Remove" => {
                let [file] = self.args()?;
                let name = self.cstring(file)?;
                if name.is_empty() || self.volume.info(&name).is_none() {
                    return Ok(g::FILE_ERROR + 3);
                }
                // The file is the database, so both halves go: the volume's bytes and the records a
                // still-open handle reads through. The header asks a caller to release the database
                // first, and what a handle left open sees afterwards is what a handset leaves it —
                // a database whose file is no longer there.
                self.volume.remove(&name);
                self.dbs.forget(&name);
                Ok(0)
            }
            // `void IDBMgr_SetCacheSize(me, int nSize)`: the body stores the word `OEM_DBOpenEx`
            // reads later. This host reads each database whole and writes it
            // back, so there is no cache for the number to size — answered, and kept nowhere, which
            // is the `void` the header promises.
            "SetCacheSize" => Ok(0),
            _ => Err(unimplemented().into()),
        }
    }

    /// Open (or create) the database file `name`, answering an `IDatabase` object or zero.
    ///
    /// The order is the delivered `OpenDatabase`'s: `OEM_DBOpenEx` runs
    /// first, and only its `AEE_DB_ERR_NOT_EXIST` reaches `OEM_DBCreate`. A file that is there but
    /// cannot be read is a different failure — a short header gives `BAD_STATE`,
    /// while a broken index just makes `LoadIndex` rebuild one from the data area — and on
    /// `BAD_STATE` the documented behaviour *deletes* the file and still returns NULL: `OEM_DBDelete`
    /// writes `NO_ERR` into the same error word, so the create test below it
    /// can no longer pass. The next open is the one that finds `NOT_EXIST` and creates.
    ///
    /// `db::decode_file` now reads the reference OEM layout as well as the older container this host
    /// used to write, so a preloaded handset database opens here rather than arriving at that
    /// branch. What still fails is a file that is not a database in either format, or one whose
    /// data area is damaged. `bCreate` is permission to make a *missing* file, not to overwrite
    /// one, so an unreadable file answers NULL and is removed only when no handle still has it:
    /// `OEM_DBDelete` refuses while `FindOpenDatabase` finds one,
    /// `ALREADY_OPEN`), which is what [`db::Databases::is_open`] measures.
    pub(crate) fn db_open(&mut self, name: &str, create: bool) -> Result<u32, Error> {
        let records = match self.volume.info(name) {
            Some(_) => {
                let bytes = match self.volume.open(name, g::_OFM_READ) {
                    Ok(open) => self.volume.read(&open, u32::MAX),
                    Err(_) => Vec::new(),
                };
                match db::decode_file(&bytes) {
                    Some(records) => records,
                    None => {
                        if !self.dbs.is_open(name) {
                            self.volume.remove(name);
                            let _ = self.dbs.forget(name);
                        }
                        return Ok(0);
                    }
                }
            }
            None => {
                if !create {
                    return Ok(0);
                }
                // Create: an empty database file, written now so a second open of the same name
                // finds one. The bytes are the container's header and nothing else.
                self.db_save_file(name, &[]);
                Vec::new()
            }
        };
        // A fresh object per open, which is what the documented behaviour does: `OEM_DBOpenEx` shares its
        // OEM context (refCount++) but `AEEDatabase_New` still builds a new
        // object every time. The store is shared by name, so two handles see one set of records.
        let addr =
            self.hle
                .new_object(&mut self.cpu.mem, "IDatabase", g::AEECLSID_DBMGR, 8, 4, &[])?;
        self.dbs.attach(addr, name.to_owned(), records);
        Ok(addr)
    }

    /// Write a database's records back to its file, by name.
    pub(crate) fn db_save_file(&mut self, name: &str, records: &[db::Record]) {
        let bytes = db::encode_file(records);
        if let Ok(mut open) = self.volume.open(name, g::_OFM_CREATE) {
            // The whole file is rewritten, so what was there before must not survive past the end of
            // the new bytes.
            self.volume.truncate(&mut open, 0);
            let _ = self.volume.write(&mut open, &bytes);
        }
    }

    /// Write an open database back to its file. Called after every change, because a handset writes
    /// its flash as the record is added rather than when the app happens to exit — an app that is
    /// closed by the shell mid-game keeps what it stored.
    pub(crate) fn db_save(&mut self, addr: u32) {
        let Some(open) = self.dbs.open(addr) else {
            return;
        };
        let name = open.name.clone();
        let Some(records) = self.dbs.file(&name).cloned() else {
            return;
        };
        self.db_save_file(&name, &records);
    }

    /// The `AEEDBField` array a caller passed, read into the fields the codec stores.
    ///
    /// The stride is eight rather than the sixteen the ABI tables give: `AEEDBFieldType` and
    /// `AEEDBFieldName` are `int8` off `_WIN32`, so a pack built for a handset walks its own array in
    /// steps of eight — see [`crate::db`]'s module docs for the four offsets.
    pub(crate) fn db_fields(&mut self, pfields: u32, count: i32) -> Result<Vec<db::Field>, Error> {
        let mut fields = Vec::with_capacity(count.max(0) as usize);
        for i in 0..count.max(0) as u32 {
            let at = pfields + i * db::FIELD_STRIDE;
            let kind = self.cpu.mem.read_u8(at + db::FIELD_TYPE)?;
            let name = self.cpu.mem.read_u8(at + db::FIELD_NAME)?;
            let stated = self.cpu.mem.read_u16(at + db::FIELD_LEN)?;
            let buffer = self.cpu.mem.read_u32(at + db::FIELD_DATA)?;
            let value = match kind {
                // `AEEDB_FT_STRING`: the body calls `aee_wstrsize` on the pointer whatever `wDataLen`
                // says, so the type alone decides where the end is.
                4 => db::Value::Text(self.wide_units(buffer)?),
                1 | 2 | 3 => {
                    let width = match kind {
                        1 => 1,
                        2 => 2,
                        _ => 4,
                    };
                    self.read_or_empty(buffer, width)?
                }
                // Everything else is `wDataLen` bytes the caller named. A null pointer with a
                // non-zero length is the caller's mistake; the documented behaviour would hand it to
                // `aee_memmove`, and a fault is not something this host can be more useful about
                // than an empty field.
                _ => self.read_or_empty(buffer, stated as u32)?,
            };
            fields.push(db::Field { kind, name, value });
        }
        Ok(fields)
    }

    /// An `IDBRecord` object for one record of `database`, its bytes copied where the guest can read
    /// them.
    pub(crate) fn db_record_object(
        &mut self,
        database: u32,
        record: &db::Record,
    ) -> Result<u32, Error> {
        let addr =
            self.hle
                .new_object(&mut self.cpu.mem, "IDBRecord", g::AEECLSID_DBMGR, 8, 4, &[])?;
        let block_len = record.bytes.len() as u32 + 2;
        let block = self.heap_alloc(block_len)?;
        if block == 0 {
            return Ok(0);
        }
        self.cpu.mem.write_bytes(block, &record.bytes)?;
        self.dbs.hold(
            addr,
            db::Holder {
                db: database,
                id: record.id,
                bytes: record.bytes.clone(),
                at: None,
                block,
                wide: 0,
                block_len,
            },
        );
        Ok(addr)
    }

    /// Install a freshly encoded record as the bytes the guest reads through this object.
    ///
    /// `false` is the documented behaviour's `ENOMEMORY`: the new copy does not fit and the heap has no room
    /// for a bigger one. The object is left exactly as it was in that case, so an `Update` can answer
    /// the error without the store and the guest copy disagreeing — in particular nothing ends up
    /// pointing at address zero or at a block already back on the free list.
    pub(crate) fn db_holder_recopy(&mut self, addr: u32, bytes: &[u8]) -> Result<bool, Error> {
        let Some(holder) = self.dbs.holder(addr).cloned() else {
            return Ok(true);
        };
        let wanted = bytes.len() as u32 + 2;
        let (block, block_len) = if wanted <= holder.block_len {
            (holder.block, holder.block_len)
        } else {
            // Allocate *before* freeing the old copy: a failed allocation must leave the short
            // record intact, and freeing first would hand the allocator an address it could reuse.
            let block = self.heap_alloc(wanted)?;
            if block == 0 {
                return Ok(false);
            }
            if holder.block != 0 {
                self.heap_free(holder.block);
            }
            (block, wanted)
        };
        self.cpu.mem.write_bytes(block, bytes)?;
        if let Some(held) = self.dbs.holder_mut(addr) {
            held.bytes = bytes.to_vec();
            held.at = None;
            held.block = block;
            held.block_len = block_len;
        }
        Ok(true)
    }

    /// Give a record object's guest blocks back.
    ///
    /// `Remove` ends the object's ownership before the caller gets around to releasing it, so both
    /// paths have to reach the same disposal: merely dropping the holder would leave the record's
    /// copy — and any wide transcription of a narrow string — on the guest heap until the applet
    /// ends. A second call finds no holder and frees nothing.
    pub(crate) fn db_dispose_holder(&mut self, addr: u32) {
        if let Some(holder) = self.dbs.release(addr) {
            for block in [holder.block, holder.wide] {
                if block != 0 {
                    self.heap_free(block);
                }
            }
        }
    }

    /// `IDBRecord::GetField` and the three typed readers built on it.
    ///
    /// Answers the field's type, its name, the pointer the caller reads through, and its length —
    /// the four things the body writes out. `None` is the body's NULL: no current field, or
    /// arguments it refuses.
    ///
    /// The pointer is into the guest's copy of the record except for a *narrow* string, where the
    /// body `aee_malloc`s a wide transcription per call; that block is kept on the record object and
    /// reused, which is the same bytes and one allocation instead of one per read.
    pub(crate) fn db_current_field(
        &mut self,
        addr: u32,
    ) -> Result<Option<(u8, u8, u32, u16)>, Error> {
        let Some(holder) = self.dbs.holder(addr).cloned() else {
            return Ok(None);
        };
        let Some(at) = holder.at else {
            return Ok(None);
        };
        let Some(view) = db::field_at(&holder.bytes, at) else {
            return Ok(None);
        };
        let pointer = holder.block + view.data as u32;
        if view.kind != 4 {
            return Ok(Some((view.kind, view.name, pointer, view.len)));
        }
        let payload = &holder.bytes[view.data..view.data + view.len as usize];
        match payload.first() {
            // The narrow form: the caller gets wide characters, so the bytes are transcribed. The
            // length the body reports is twice the stored one, which is the wide size plus the
            // marker — the same number the wide path reports, which is why a caller cannot tell
            // which form the file held.
            Some(&0x03) => {
                let units = db::read_string(payload);
                let bytes = (units.len() + 2) * 2;
                let enough = holder.wide != 0
                    && self.hle.block_size(holder.wide).unwrap_or(0) >= bytes as u32;
                if !enough {
                    if holder.wide != 0 {
                        self.heap_free(holder.wide);
                    }
                    // Clear it *before* allocating: a failed allocation takes the fallback below,
                    // and leaving the old address in the holder would make `Release` free a block
                    // that is already back on the free list.
                    if let Some(held) = self.dbs.holder_mut(addr) {
                        held.wide = 0;
                    }
                    let block = self.heap_alloc(bytes as u32)?;
                    if block == 0 {
                        // The body falls back to the narrow bytes in place when its allocation
                        // fails, and so does this: the caller then reads one byte per character,
                        // which is the bug the platform code has when out of memory.
                        return Ok(Some((view.kind, view.name, pointer + 1, view.len)));
                    }
                    if let Some(held) = self.dbs.holder_mut(addr) {
                        held.wide = block;
                    }
                }
                let wide = self.dbs.holder(addr).map_or(0, |held| held.wide);
                for (i, unit) in units.iter().enumerate() {
                    self.cpu.mem.write_u16(wide + i as u32 * 2, *unit as u32)?;
                }
                self.cpu.mem.write_u16(wide + units.len() as u32 * 2, 0)?;
                Ok(Some((
                    view.kind,
                    view.name,
                    wide,
                    (view.len as u32 * 2) as u16,
                )))
            }
            // The wide form: two marker bytes then the units, already in the caller's shape.
            Some(&0xff) => Ok(Some((view.kind, view.name, pointer + 2, view.len))),
            _ => Ok(Some((view.kind, view.name, pointer, view.len))),
        }
    }

    /// The `IDatabase` face. Every slot reads through the store by the name the
    /// object was opened under, which is why the manager is the only place a record can come from.
    pub(crate) fn idbdatabase(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let Some(open) = self.dbs.open(this).cloned() else {
            // An address that is not an open database: the object was released, or it was never one.
            // Both answer zero for the slots that return a pointer and zero for a count, which is
            // what the documented behaviour does with a null handle (`AEEDatabase_GetRecordCount` returns
            // its zero for `[+12] == 0`).
            return match name {
                "Reset" => Ok(0),
                "GetRecordCount" | "GetNextRecord" | "GetRecordByID" | "CreateRecord" => Ok(0),
                _ => Err(unimplemented().into()),
            };
        };
        match name {
            // `uint16 IDatabase_GetRecordCount(me)` — `OEM_DBRecordCount`, zero on a bad handle.
            "GetRecordCount" => Ok(self.dbs.count(&open.name) as u32),
            // `void IDatabase_Reset(me)`: the body stores `0xFFFF` in its cursor, the same value the
            // constructor leaves, so `GetNextRecord` starts from the first record.
            "Reset" => {
                self.dbs.rewind(this);
                Ok(0)
            }
            // `IDBRecord *IDatabase_GetNextRecord(me)`: a record with a reference on the database,
            // or NULL at the end. The cursor moves even when the record does not come back, which is
            // why the end parks it rather than leaving it on the last record.
            "GetNextRecord" => {
                let Some(record) = self.dbs.next(this) else {
                    return Ok(0);
                };
                self.db_record_object(this, &record)
            }
            // `IDBRecord *IDatabase_GetRecordByID(me, uint16 wID)`. The body refuses
            // `AEE_DB_RECID_NULL` before it asks the platform layer for anything.
            "GetRecordByID" => {
                let [want] = self.args()?;
                let Some(record) = self.dbs.by_id(this, want as u16) else {
                    return Ok(0);
                };
                self.db_record_object(this, &record)
            }
            // `IDBRecord *IDatabase_CreateRecord(me, AEEDBField *pFields, int nFieldCount)`: the body
            // makes the record object and lets its own `Update` do the adding, so a field list it
            // refuses answers NULL rather than a record with no fields.
            "CreateRecord" => {
                let [pfields, count] = self.args()?;
                let count = count as i32;
                if pfields == 0 || count <= 0 {
                    return Ok(0);
                }
                let fields = self.db_fields(pfields, count)?;
                let bytes = db::encode(&fields);
                let id = self.dbs.add(&open.name, bytes.clone());
                self.db_save(this);
                self.db_record_object(this, &db::Record { id, bytes })
            }
            _ => Err(unimplemented().into()),
        }
    }

    /// The `IDBRecord` face.
    ///
    /// Every slot works on the record's *current field*, which `Reset` puts before the first and
    /// `NextField` walks forward — the header's own wording, and the reason `GetFieldWord` and
    /// friends do not move it.
    pub(crate) fn idbrecord(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // Error codes, all from the header: `AEE_DB_EBADREC`/`EBADSTATE`/
            // `ENULLREC`/`ENULLFIELD`/`EBADFIELDNUM` are `DB_ERROR_BASE + 0..4`, and the extractor
            // keeps the base but not the aliases, so the sum is spelled here.
            //
            // `void IDBRecord_Reset(me)`: `m_pCurField = 0`, the body's "nothing read yet".
            "Reset" => {
                if let Some(holder) = self.dbs.holder_mut(this) {
                    holder.at = None;
                }
                Ok(0)
            }
            // `int IDBRecord_Update(me, AEEDBField *pFields, int nFieldCount)` — the body's own error
            // order, which is worth keeping: a record whose database is gone answers `EBADREC` before
            // it looks at the arguments at all.
            "Update" => {
                let [pfields, count] = self.args()?;
                let count = count as i32;
                let Some(holder) = self.dbs.holder(this).cloned() else {
                    return Ok(g::DB_ERROR_BASE);
                };
                let Some(open) = self.dbs.open(holder.db).cloned() else {
                    return Ok(g::DB_ERROR_BASE);
                };
                if pfields == 0 {
                    return Ok(g::DB_ERROR_BASE + 3);
                }
                if count <= 0 {
                    return Ok(g::DB_ERROR_BASE + 4);
                }
                let fields = self.db_fields(pfields, count)?;
                let bytes = db::encode(&fields);
                // The documented behaviour mallocs the new record before it asks the platform layer to write it,
                // and answers ENOMEMORY when that malloc fails. Do it in the same order: the store
                // only changes once the guest-visible copy can actually hold the new bytes.
                if !self.db_holder_recopy(this, &bytes)? {
                    return Ok(g::ENOMEMORY);
                }
                if !self.dbs.replace(&open.name, holder.id, bytes) {
                    // The record was removed from under this object; the body's `OEM_DBRecordUpdate`
                    // on an id that is no longer there is its `ENOMEMORY`-adjacent failure, and the
                    // one honest answer is that the record is not a record any more.
                    return Ok(g::DB_ERROR_BASE);
                }
                self.db_save(holder.db);
                Ok(0)
            }
            // `int IDBRecord_Remove(me)`: the record leaves the database and the object goes with it.
            // The body returns the platform layer's error byte, which this host owns now.
            "Remove" => {
                let Some(holder) = self.dbs.holder(this).cloned() else {
                    return Ok(g::DB_ERROR_BASE);
                };
                let Some(open) = self.dbs.open(holder.db).cloned() else {
                    return Ok(g::DB_ERROR_BASE + 1);
                };
                if !self.dbs.remove(&open.name, holder.id) {
                    return Ok(g::DB_ERROR_BASE);
                }
                self.db_save(holder.db);
                self.db_dispose_holder(this);
                Ok(0)
            }
            // `uint16 IDBRecord_GetID(me)`.
            "GetID" => Ok(self
                .dbs
                .holder(this)
                .map_or(db::RECID_NULL as u32, |held| held.id as u32)),
            // `AEEDBFieldType IDBRecord_NextField(me, AEEDBFieldName *pName, uint16 *pnLen)` — the
            // type of the field it moved to, `AEEDB_FT_NONE` at the end. The length it writes is the
            // stored one, marker included.
            "NextField" => {
                let [pname, plen] = self.args()?;
                let Some(holder) = self.dbs.holder(this).cloned() else {
                    return Ok(0);
                };
                match db::step(&holder.bytes, holder.at) {
                    Some((at, view)) => {
                        if pname != 0 {
                            self.cpu.mem.write_u8(pname, view.name)?;
                        }
                        if plen != 0 {
                            self.cpu.mem.write_u16(plen, view.len as u32)?;
                        }
                        if let Some(held) = self.dbs.holder_mut(this) {
                            held.at = Some(at);
                        }
                        Ok(view.kind as u32)
                    }
                    None => {
                        // The body stores zero — "nothing read yet" — so the next call starts over.
                        if let Some(held) = self.dbs.holder_mut(this) {
                            held.at = None;
                        }
                        Ok(0)
                    }
                }
            }
            // `byte *IDBRecord_GetField(me, AEEDBFieldName *pName, AEEDBFieldType *pType, uint16
            // *pnLen)`. All three out-parameters are required; the body answers NULL if any is.
            "GetField" => {
                let [pname, ptype, plen] = self.args()?;
                if pname == 0 || ptype == 0 || plen == 0 {
                    return Ok(0);
                }
                let Some((kind, field_name, pointer, len)) = self.db_current_field(this)? else {
                    return Ok(0);
                };
                self.cpu.mem.write_u8(pname, field_name)?;
                self.cpu.mem.write_u8(ptype, kind)?;
                self.cpu.mem.write_u16(plen, len as u32)?;
                Ok(pointer)
            }
            // The three typed readers are `GetField` plus a type test and a copy, which is what the
            // the platform's own routines are (`AEEDBRecord_GetFieldWord` calls slot 7 through the vtable).
            "GetFieldWord" | "GetFieldDWord" => {
                let [out] = self.args()?;
                if out == 0 {
                    return Ok(0);
                }
                let want = if name == "GetFieldWord" { 2 } else { 3 };
                let width = if want == 2 { 2 } else { 4 };
                let Some((kind, _, pointer, _)) = self.db_current_field(this)? else {
                    return Ok(0);
                };
                if kind != want {
                    return Ok(0);
                }
                let bytes = self.cpu.mem.read_bytes(pointer, width)?;
                self.cpu.mem.write_bytes(out, &bytes)?;
                Ok(1)
            }
            // `AECHAR *IDBRecord_GetFieldString(me)`: the pointer for a string field, NULL otherwise.
            "GetFieldString" => {
                let Some((kind, _, pointer, _)) = self.db_current_field(this)? else {
                    return Ok(0);
                };
                if kind != 4 {
                    return Ok(0);
                }
                Ok(pointer)
            }
            _ => Err(unimplemented().into()),
        }
    }
}
