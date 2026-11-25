//! Files, volumes and resources: `IFileMgr`/`IFile`, the app history,
//! FIFOs, unzip streams and the guest heap.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// What a module wanted from its resources, recorded so a run names the files it would need
    /// The resource file a shell call named, or `None` when it named none. A null `pszResFile` is not
    /// a missing argument — it means "the app's own", and [`Store`] is where that ordering lives. The
    /// text is returned as the caller spelled it, because the report is read by a person and
    /// `/logo1.png` says something `logo1.png` does not.
    ///
    /// An address that cannot be read answers the same way as a null one. A pack that has no file to
    /// name sometimes says so with `-1` rather than `NULL`, and this is a discriminator on a lookup,
    /// not an argument whose value the caller needs: faulting here would
    /// report the host as having walked off memory over a question that has an obvious answer.
    pub(crate) fn res_named(&self, addr: u32) -> Option<String> {
        if addr == 0 {
            return None;
        }
        let text = self.cstring(addr).ok()?;
        (!text.is_empty()).then_some(text)
    }

    /// `IFileSystem` — the whole-filesystem view of the *same* volume `IFileMgr` hands out, under
    /// class `AEECLSID_FileSystem` (`0x01020725`).
    ///
    /// `Access`/`MkDir`/`Rename`/`RmDir`/`Truncate`/`Unlink` are jobs `IFileMgr` does under other
    /// names, and they are answered from this one volume rather than from a second copy of it: a run
    /// in which the two disagreed about what exists would be a run with two filesystems. `Stat` and
    /// `StatVFS` are the new surface, and both structures they fill are already in
    /// `src/abi/generated/structs.rs` (`FSStat` from the header, `FSStatVFS` from
    /// the header).
    ///
    /// ⚠ Two pairs of bit families meet here and are **not** interchangeable:
    /// `Access`'s `nMode` is `FS_R_OK`/`FS_W_OK` where
    /// `IFileMgr::CheckPathAccess` speaks `AEEFP_*`; and `Stat`'s `dwMode` is `FS_S_IF*` where
    /// `AEEFileInfo::attrib` is `FA_*`. Each arm below says which one it is translating, because
    /// getting either wrong hands a pack a value from the wrong table that still looks like a mode.
    pub(crate) fn ifilesystem(
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
            "Access" => {
                let [f, mode] = self.args()?;
                let asked = self.cstring(f)?;
                // `FS_R_OK`/`FS_W_OK` are the two bits the header names, and they are *not* the
                // `AEEFP_*` values: `FS_R_OK` is 4 and `AEEFP_READ` is 1. Translating here keeps
                // `Volume::access`'s rule in one place.
                let wanted = (if mode & g::FS_R_OK != 0 {
                    g::AEEFP_READ
                } else {
                    0
                }) | (if mode & g::FS_W_OK != 0 {
                    g::AEEFP_WRITE
                } else {
                    0
                });
                Ok(self.volume.access(&asked, wanted).1)
            }
            "MkDir" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.mkdir(&asked))
            }
            "Rename" => {
                let [from, to] = self.args()?;
                let (from, to) = (self.cstring(from)?, self.cstring(to)?);
                Ok(self.volume.rename(&from, &to))
            }
            "RmDir" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.rmdir(&asked))
            }
            "Truncate" => {
                let [f, len] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.truncate_at(&asked, len))
            }
            "Unlink" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.remove(&asked))
            }
            "Stat" => {
                let [f, buf, size] = self.args()?;
                let asked = self.cstring(f)?;
                // The header points at `Stat` for the existence test `Access` does not have,
                // so a name that is not here says so by name.
                let Some(mode) = self.volume.mode_bits(&asked) else {
                    return Ok(g::AEE_ENOSUCH);
                };
                let Some(info) = self.volume.info(&asked) else {
                    return Ok(g::AEE_ENOSUCH);
                };
                if buf == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // `struct FSStat { uint32 dwMode; uint32 dwSize; uint32 dwCreationDate; }`,
                // with the caller's `nBufSize` as the bound — the rule
                // `GetInfoEx` follows, so a caller built against a shorter struct is written only as
                // far as it has room for.
                let at =
                    |field: &str| abi::offset_of("FSStat", field).expect("FSStat is in the tables");
                let room = size as usize;
                let fits = |off: u32| (off as usize) + 4 <= room;
                if fits(at("dwMode")) {
                    self.cpu.mem.write_u32(buf + at("dwMode"), mode)?;
                }
                if fits(at("dwSize")) {
                    self.cpu.mem.write_u32(buf + at("dwSize"), info.size)?;
                }
                if fits(at("dwCreationDate")) {
                    // No timestamps in this volume, and the field's own comment in the header says it
                    // is not a POSIX one; zero is what `GetInfoEx` already answers for it.
                    self.cpu.mem.write_u32(buf + at("dwCreationDate"), 0)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "StatVFS" => {
                let [f, buf, size] = self.args()?;
                let asked = self.cstring(f)?;
                if self.volume.test(&asked) != g::AEE_SUCCESS {
                    return Ok(g::AEE_ENOSUCH);
                }
                if buf == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // `FSStatVFS` is ten `uint32`s. A block size of 1 is what
                // makes every count below mean bytes, which is what this volume has: there is no
                // block layer under it to report a real one for.
                let total = file::room(self.device.storage_total);
                let free = file::room(self.device.storage_free);
                let (files, used) = self.volume.use_info();
                let fields = [
                    ("dwBlockSize", 1),
                    ("dwFragmentSize", 1),
                    ("dwBlocks", total),
                    ("dwBlocksFree", free),
                    ("dwBlocksAvail", free),
                    ("dwNameMax", g::AEE_MAX_FILE_NAME),
                    ("dwFilesUsed", files),
                    ("dwFSSpaceUsed", file::room(used)),
                    ("dwMaxFiles", u16::MAX as u32),
                    ("dwMaxFSSpace", total),
                ];
                let at = |field: &str| {
                    abi::offset_of("FSStatVFS", field).expect("FSStatVFS is in the tables")
                };
                let room = size as usize;
                for (field, value) in fields {
                    let off = at(field);
                    if (off as usize) + 4 <= room {
                        self.cpu.mem.write_u32(buf + off, value)?;
                    }
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The `IFileMgr` slots this host answers over [`Runtime::volume`].
    ///
    /// What is left refused is named rather than guessed: `Use`, `GetFileUseInfo`,
    /// `SetDescription`, `ResolvePath` and `CheckPathAccess` are per-handset volume services with no
    /// model here, and `GetFreeSpaceEx` asks about a path that is not this one volume.
    /// The ceiling on a description read out of guest memory: `SetDescription` is handed a
    /// terminator and no length, so a bad pointer must not be able to walk the run to the end of the
    /// address space. 1024 units is far past any description a handset's UI shows.
    pub(crate) const MAX_DESCRIPTION_UNITS: usize = 1024;

    pub(crate) fn ifilemgr(
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
            "Test" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                // `CFileMgr_Test`: resolve, let a const/ROM entry answer without the file
                // system, ask the FS otherwise — and the code lands in the file manager's error cell
                // while the **answer** is the boolean `code != 0`. Zero means the thing is there
                // (TRUE = failure, the shape `GetInfo` answers); the pk's own ringer gate reads it as
                // `!Test ⇒ EFILEEXISTS`, and the other caller in the same source reads it as
                // `Test == SUCCESS ⇒ the language file is there`. Both are right only if the boolean
                // is the answer and the code is the cell's — which is what this arm now gives, and
                // what answering the code itself used to get wrong.
                let code = self.volume.test(&asked);
                if code == 0 {
                    // The body writes `v3` unconditionally, so a hit clears the cell.
                    self.volume.accept();
                }
                Ok((code != 0) as u32)
            }
            "GetInfo" => {
                let [f, dst] = self.args()?;
                if dst == 0 || self.cpu.mem.mapped_run(dst, file::INFO_SIZE) < file::INFO_SIZE {
                    self.volume.refuse(g::AEE_EBADPARM);
                    return Ok(g::TRUE);
                }
                let asked = self.cstring(f)?;
                let Some(name) = self.volume.resolved(&asked) else {
                    return Ok(g::TRUE);
                };
                let sz_name =
                    abi::offset_of("AEEFileInfo", "szName").expect("AEEFileInfo is in the tables");
                let Some(info) = self.volume.info(&name) else {
                    // ⚠ The name is written even here. What the handset's file system answered is the code
                    // in the cell; this volume's own name for "no such file" is the one `OpenFile` refuses
                    // with, put there by the lookup above.
                    self.put_file_name(dst + sz_name, &name)?;
                    return Ok(g::TRUE);
                };
                self.put_file_info(dst, &info)?;
                self.volume.accept();
                Ok(g::FALSE)
            }
            "OpenFile" => {
                // The handset answers with an `IFile*` or `NULL`, and there is no out-parameter for
                // a reason, so a failure here is zero and `GetLastError` is what says why.
                let [f, mode] = self.args()?;
                let asked = self.cstring(f)?;
                let opened = match self.volume.open(&asked, mode) {
                    Ok(opened) => opened,
                    Err(code) => {
                        // And a pack that never asks `GetLastError` gets a refusal that is
                        // indistinguishable from a file it did not want. Say which name, in which
                        // mode, and for which of the three reasons — in the list that survives the
                        // note cap, because the runs where this matters are too long for the notes.
                        self.display.refuse(format!(
                            "OpenFile {asked:?} mode={mode:#x}: {}",
                            Volume::refusal(code)
                        ));
                        return Ok(0);
                    }
                };
                let addr = self
                    .hle
                    .new_object(&mut self.cpu.mem, "IFile", 0, 8, 4, &[])?;
                // A file that opens but holds nothing is the shape a missing resource takes when the
                // *file* is there and the payload is not: the pack's own loader reads zero bytes,
                // builds an empty table out of them, and the null-pointer crash it then hits is two
                // indirections away from this open — invisible without a line at the open itself. A
                // create-mode open is the caller manufacturing the file, so emptiness is expected
                // there and says nothing.
                if (mode & g::_OFM_CREATE) == 0
                    && self
                        .volume
                        .info(&opened.name)
                        .is_some_and(|info| info.size == 0)
                {
                    self.display.refuse(format!(
                        "OpenFile {:?}: opened a file that holds 0 bytes",
                        opened.name
                    ));
                }
                self.files.insert(addr, opened);
                Ok(addr)
            }
            "Remove" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.remove(&asked))
            }
            "MkDir" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.mkdir(&asked))
            }
            "RmDir" => {
                let [f] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.rmdir(&asked))
            }
            "Rename" => {
                let [from, to] = self.args()?;
                let (from, to) = (self.cstring(from)?, self.cstring(to)?);
                Ok(self.volume.rename(&from, &to))
            }
            "GetFreeSpace" => {
                // `uint32 GetFreeSpace(IFileMgr*, uint32* pdwTotal)` — free out of the call, total
                // into the parameter, and the header says a caller tells a real zero from a failed
                // call by looking at the total. Both come from the device profile, which is the same
                // answer `AEEDeviceInfo` gives.
                let [dst] = self.args()?;
                if dst != 0 {
                    self.cpu
                        .mem
                        .write_u32(dst, file::room(self.device.storage_total))?;
                }
                Ok(file::room(self.device.storage_free))
            }
            "EnumNextEx" => {
                let [dst] = self.args()?;
                if dst == 0 || self.cpu.mem.mapped_run(dst, 4) < 4 {
                    return Ok(g::FALSE);
                }
                if self.cpu.mem.read_u32(dst)? != 40 {
                    // The gate's own refusal writes no reason anywhere; the cell belongs to the file
                    // manager and this body never touches it.
                    return Ok(g::FALSE);
                }
                let Some(info) = self.volume.enum_next() else {
                    return Ok(g::FALSE);
                };
                self.put_info_facts_ex(dst, &info)?;
                let at = |field: &str| {
                    abi::offset_of("AEEFileInfoEx", field).expect("AEEFileInfoEx is in the tables")
                };
                let buf = self.cpu.mem.read_u32(dst + at("pszFile"))?;
                let max = self.cpu.mem.read_u32(dst + at("nMaxFile"))? as usize;
                self.put_ex_name(buf, max, &info.name, "EnumNextEx")?;
                // ⚠ This copy's guards are **not** `CFileMgr_FillInfoEx`'s: the description branch is
                // entered on `nDescriptionSize != 0` — the *size*, not the buffer — and the copy runs
                // through `pszDescription` whatever it holds (a null buffer with a declared size is a guest
                // fault there), while the class branch is entered on `pClasses != 0`. The two are not
                // interchangeable, which is why this arm does not call `fill_info_ex`.
                let desc = self.cpu.mem.read_u32(dst + at("pszDescription"))?;
                let desc_bytes = self.cpu.mem.read_u32(dst + at("nDescriptionSize"))? as usize;
                let classes = self.cpu.mem.read_u32(dst + at("pClasses"))?;
                let class_bytes = self.cpu.mem.read_u32(dst + at("nClassesSize"))? as usize;
                if desc == 0 && desc_bytes != 0 {
                    self.display.note(format!(
                        "EnumNextEx declared {desc_bytes} bytes of description but passed \
                         pszDescription = 0: the shipped copy runs on the size alone and would fault \
                         there, this host writes nothing"
                    ));
                }
                if desc_bytes != 0 || classes != 0 {
                    let described = if desc_bytes != 0 {
                        self.volume
                            .description(&info.name)
                            .map(<[u16]>::to_vec)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    self.put_ex_description(desc, desc_bytes, &described, "EnumNextEx")?;
                    self.put_ex_classes(classes, class_bytes, "EnumNextEx")?;
                }
                Ok(g::TRUE)
            }
            // `IFILEMGR_GetInfoEx(IFileMgr*, const char *pszName, AEEFileInfoEx *pi)` — the same
            // three facts as `GetInfo`, into the extended record.
            "GetInfoEx" => {
                let [f, dst] = self.args()?;
                let declared = if dst == 0 {
                    0
                } else {
                    self.cpu.mem.read_u32(dst)?
                };
                if f == 0
                    || dst == 0
                    || declared == 0
                    || self.cpu.mem.mapped_run(dst, declared) < declared
                {
                    self.volume.refuse(g::AEE_EBADPARM);
                    return Ok(g::TRUE);
                }
                let asked = self.cstring(f)?;
                // Unlike `CFileMgr_GetInfo`, which resolves the name **into the caller's `szName`** first,
                // this one builds its own path (`BuildAndCheckPath`) and reaches the struct only through
                // `CFileMgr_FillInfoEx` — which runs only after the file system answered 0. So a refusal
                // here leaves the record exactly as the caller passed it, name included.
                let Some(info) = self.volume.info(&asked) else {
                    return Ok(g::TRUE);
                };
                self.put_info_facts_ex(dst, &info)?;
                self.fill_info_ex(dst, &info)?;
                self.volume.accept();
                Ok(g::FALSE)
            }
            // `IFILEMGR_GetFreeSpaceEx(IFileMgr*, const char *cpszPath, uint32 *pdwTotal,
            // uint32 *pdwFree)` — `GetFreeSpace` with the path named and both numbers in
            // parameters. This host has one volume, so the path decides
            // only whether the question is about something it has at all.
            "GetFreeSpaceEx" => {
                // ⚠ **This slot has no delivered body in this box**: `who_defines.py
                // CFileMgr_GetFreeSpaceEx` scans 235 members and finds 0 definitions and 0 references
                // (positive controls in the same run: `aee_splitpath` defined 1 / referenced 7,
                // `AEEFile_IsMIF` defined 1). So the shape below is the header's and this host's, not
                // something taken from an object: free and total in the two parameters, and the path
                // deciding only whether the question is about something this volume has at all.
                let [f, total, free] = self.args()?;
                let asked = self.cstring(f)?;
                if self.volume.test(&asked) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EFAILED);
                }
                if total != 0 {
                    self.cpu
                        .mem
                        .write_u32(total, file::room(self.device.storage_total))?;
                }
                if free != 0 {
                    self.cpu
                        .mem
                        .write_u32(free, file::room(self.device.storage_free))?;
                }
                Ok(g::AEE_SUCCESS)
            }
            // `IFILEMGR_CheckPathAccess(IFileMgr*, const char *cpszIn, uint32 dwDesiredRights,
            // uint32 *pdwActualRights)`. What is actually granted goes
            // into the parameter either way — a caller that only reads that learns what it may do —
            // and the answer is the header's own refusal for a request beyond it.
            "CheckPathAccess" => {
                let [f, desired, actual] = self.args()?;
                let asked = self.cstring(f)?;
                let (rights, result) = self.volume.access(&asked, desired);
                if actual != 0 {
                    self.cpu.mem.write_u32(actual, rights)?;
                }
                Ok(result)
            }
            // `IFILEMGR_GetFileUseInfo(IFileMgr*, AEEFileUseInfo*)`.
            // Two of the four numbers are readings this volume has; the two ceilings are the device
            // profile's, which is what `GetFreeSpace` answers from as well.
            "GetFileUseInfo" => {
                let [dst] = self.args()?;
                if dst == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (files, bytes) = self.volume.use_info();
                let at = |field: &str| {
                    abi::offset_of("AEEFileUseInfo", field)
                        .expect("AEEFileUseInfo is in the tables")
                };
                // ⚠ Ours, not the body's: this volume enforces exactly one ceiling — `file::VOLUME_LIMIT`,
                // the number `write` and `seek` charge against — and no limit at all on how many files an
                // app may create. The delivered code spells "there is no quota behind this answer" with
                // `0xFFFF` and `0xFFFFFFFF` (its no-module branch), which is the nearest honest thing to
                // report for a count this host does not bound; the byte ceiling it does bound, so that
                // one carries the real number.
                self.cpu
                    .mem
                    .write_u16(dst + at("wMaxFiles"), u16::MAX as u32)?;
                self.cpu
                    .mem
                    .write_u16(dst + at("wFilesUsed"), files.min(u16::MAX as u32))?;
                self.cpu
                    .mem
                    .write_u32(dst + at("dwMaxSpace"), file::VOLUME_LIMIT as u32)?;
                self.cpu
                    .mem
                    .write_u32(dst + at("dwSpaceUsed"), file::room(bytes))?;
                Ok(g::AEE_SUCCESS)
            }
            // `IFILEMGR_SetDescription(IFileMgr*, const char *pszName, AECHAR *pszDesc)`.
            // The description is a wide string and a null one means
            // "forget it". Unlike `GetInfoEx`'s `nDescriptionSize` this call is given no length, so
            // the units are read to their terminator — with a ceiling, because a bad pointer must not
            // be able to walk the run off the end of memory, and a run that hits the ceiling says so
            // rather than truncating quietly.
            "SetDescription" => {
                let [f, desc] = self.args()?;
                let asked = self.cstring(f)?;
                let units = if desc == 0 {
                    None
                } else {
                    let mut units = Vec::new();
                    let mut at = desc;
                    while units.len() < Self::MAX_DESCRIPTION_UNITS {
                        let unit = self.cpu.mem.read_u16(at)? as u16;
                        if unit == 0 {
                            break;
                        }
                        units.push(unit);
                        at += 2;
                    }
                    if units.len() == Self::MAX_DESCRIPTION_UNITS {
                        self.display.note(format!(
                            "IFileMgr::SetDescription read {} unit(s) at {desc:#x} without meeting \
                             a terminator; keeping that many",
                            Self::MAX_DESCRIPTION_UNITS
                        ));
                    }
                    Some(units)
                };
                Ok(self.volume.set_description(&asked, units))
            }
            // `IFILEMGR_Use(pfm, const char *pszName, boolean bUse)`, with `IFILEMGR_UnUse`
            // being the same call with `FALSE`. The third parameter is a published macro
            // argument, so the rule is the one the header writes: non-zero means "using it".
            "Use" => {
                let [f, on] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.use_file(&asked, on != 0))
            }
            // `IFILEMGR_ResolvePath(IFileMgr*, const char *cpszIn, char *pszOut, int *pnOutLen)`.
            //The input goes through the same rule every other name here does, and the
            // output is a caller-owned buffer whose length is both in and out: the bytes written
            // (terminator included) are reported back, so a caller that sized it wrong can tell a
            // short buffer from a path that would not resolve.
            "ResolvePath" => {
                let [inp, out, out_len] = self.args()?;
                let asked = self.cstring(inp)?;
                let room = if out == 0 || out_len == 0 {
                    0
                } else {
                    self.cpu.mem.read_u32(out_len)? as usize
                };
                let n = match file::normalize(&asked) {
                    // ⚠ The body folds `.`/`..`/`//` out of the string it *built* (`LABEL_93` walks
                    // the output buffer past `fs:/`), so on a handset a path that climbs out of the
                    // module directory comes back *inside* it rather than as an error. This host
                    // keeps its refusal — every name-taking slot shares it, and a resolver that
                    // answered here would be naming a path the next `OpenFile` refuses — but it
                    // names it with the resolver's own code, which is `EBADFILENAME` (259), not
                    // `AEE_EBADPARM`. A foreign volume has no delivered answer in this box at all,
                    // and gets the same number for the same stated reason.
                    file::Resolved::Here(n) if !n.is_empty() => n,
                    _ => return Ok(file::AEE_EBADFILENAME),
                };
                let bytes = n.as_bytes();
                let need = bytes.len() as u32 + 1;
                if out == 0 {
                    if out_len != 0 {
                        self.cpu.mem.write_u32(out_len, need)?;
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                if (room as u32) >= need {
                    self.cpu.mem.write_bytes(out, bytes)?;
                    self.cpu.mem.write_u8(out + bytes.len() as u32, 0)?;
                    if out_len != 0 {
                        self.cpu.mem.write_u32(out_len, need)?;
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                // Too small: the bytes that fit with the terminator forced inside them, and 38 —
                // `aee_makepath` (which is what actually answers the `fs:/~/` forms) writes back
                // **min(wanted, room)**, never the wanted length; only the null-buffer size query
                // reports that.
                if room > 0 {
                    let keep = (room - 1).min(bytes.len());
                    self.cpu.mem.write_bytes(out, &bytes[..keep])?;
                    self.cpu.mem.write_u8(out + keep as u32, 0)?;
                }
                if out_len != 0 {
                    self.cpu.mem.write_u32(out_len, (room as u32).min(need))?;
                }
                Ok(g::AEE_EBUFFERTOOSMALL)
            }
            "GetLastError" => Ok(self.volume.last_error()),
            "EnumInit" => {
                let [f, dirs] = self.args()?;
                let asked = self.cstring(f)?;
                Ok(self.volume.enum_init(&asked, dirs != 0))
            }
            "EnumNext" => {
                let [dst] = self.args()?;
                // `boolean`, TRUE when a record was filled: answering an `AEEResult` here would read as
                // FALSE on the very first file, and running out would read as TRUE forever — a loop over a
                // record that is no longer the next one.
                let Some(info) = self.volume.enum_next() else {
                    return Ok(g::FALSE);
                };
                if dst == 0 {
                    self.display.note(
                        "IFileMgr::EnumNext was handed a null record after taking the entry: the shipped \
                         body copies into it and would fault there, this host answered FALSE"
                    );
                    return Ok(g::FALSE);
                }
                self.put_file_info(dst, &info)?;
                Ok(g::TRUE)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The `IFile` slots, for a handle `IFileMgr::OpenFile` handed out.
    ///
    /// `IFILE_Release` is what closes a file, so the record goes with the object in the `Release`
    /// arm; a handle the app never released simply dies with the run.
    pub(crate) fn ifile(
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
        if !self.files.contains_key(&this) {
            // Not a handle this host gave out. Falling back to `AEE_EBADHANDLE` would be a guess
            // about who the caller thought they were talking to.
            return Err(Error::Hle(HleError::NotAnObject { addr: this }));
        }
        match name {
            "Read" => {
                let [dst, want] = self.args()?;
                // `CFile_Read` opens with `if ( (a2 | a3) == 0 )`: a null buffer *and* a zero count is
                // the file layer's own question, and the body has two answers for it — the count
                // cached at `+84` when its at-end bit is set, or a 40-byte `AEEFileInfoEx` filled
                // through `CFile_GetInfoEx` with `v18[3]` returned; the `v18[0] = 40` in front of that
                // call is what names the struct and its `+12`: `dwSize`. In this delivery the at-end
                // half is unreachable: nothing in the 52 `AEEFile` members sets that bit, and
                // `CFile_Seek` **clears** it (`a1[21] = 0; a1[24] = 2 * (a1[24] >> 1);`). So the
                // answer is the whole file's length at any position — `size - pos`, which this host
                // answered before, agrees only at a position of zero.
                if dst == 0 && want == 0 {
                    let name = self.files[&this].name.clone();
                    return Ok(self.volume.info(&name).map(|info| info.size).unwrap_or(0));
                }
                let bytes = self.volume.read(&self.files[&this], want);
                if bytes.is_empty() {
                    // `IFILE_Read` returns the count and the header gives 0 for "unsuccessful", so a
                    // read at the end of the file and a failed one are the same answer here. A read
                    // that answers 0 is ordinary at the end of a file with content — it is how a
                    // copy loop terminates. The one shape worth naming is a file that never had
                    // anything in it: this is the read the empty-resource crash starts from, and
                    // with the size on the line the report shows the whole chain.
                    if want != 0 {
                        let name = self.files[&this].name.clone();
                        if self.volume.info(&name).is_some_and(|info| info.size == 0) {
                            self.display.refuse(format!(
                                "IFile::Read {name:?}: asked {want}, the file holds 0 bytes, \
                                 answered 0"
                            ));
                        }
                    }
                    return Ok(0);
                }
                // The volume bounded the read by the file; nothing bounded it by the buffer. A block
                // this host handed out has a known width, and a request wider than that is the pack
                // writing over whatever was allocated next to it. A clamp cannot save a pack that is
                // reading into space it freed and still points at (the space is not on the record any
                // more, so there is nothing to clamp to), but a host that overruns its own live
                // objects in silence is not answering the question either.
                //
                // What it bounds by is the *block*, which is the only extent the host was told about.
                // A pack that takes two allocations and treats the pair as one region is relying on
                // adjacency that nothing published guarantees — the address two `MALLOC`s land on is
                // not part of any contract — and the short count it gets back is the answer it can
                // see, with the reason in the notes.
                let bytes = match self.hle.block_size(dst) {
                    Some(held) if bytes.len() > held as usize => {
                        self.display.note(format!(
                            "IFile::Read asked for {} bytes at {dst:#010x}, which this host \
                             allocated {held} bytes wide: the read stops at the end of the block",
                            bytes.len()
                        ));
                        let mut inside = bytes;
                        inside.truncate(held as usize);
                        inside
                    }
                    _ => bytes,
                };
                // The other half of `if (AEE_CheckPtr(0, a2, a3, 1) == 0) return 0;`: a buffer the
                // guest cannot write — a null one with a real count included — reads back as 0 and the
                // run goes on. A short count is a fact the caller can test; a host fault is not.
                if self.cpu.mem.write_bytes(dst, &bytes).is_err() {
                    self.display.note(format!(
                        "IFile::Read cannot write {dst:#010x}: the handset's pointer check answers 0"
                    ));
                    return Ok(0);
                }
                let Some(open) = self.files.get_mut(&this) else {
                    return Ok(0);
                };
                open.pos += bytes.len() as u32;
                Ok(bytes.len() as u32)
            }
            "Write" => {
                let [src, count] = self.args()?;
                // `CFile_Write`'s second gate is on the *pair*, not on validity:
                // `if ( a2 == 0 || a3 == 0 ) { CFile_FlushCache( a1 ); return 0; }`. A null buffer is
                // therefore not a parameter error in this slot — it is a flush, answered 0, and the
                // previous reason is left standing in the error cell (the volume's empty-write path is
                // what does that, and the const view is still tested first, as the body tests it).
                if src == 0 || count == 0 {
                    if let Some(open) = self.files.get_mut(&this) {
                        let _ = self.volume.write(open, &[]);
                    }
                    return Ok(0);
                }
                // The length is the guest's own, so it is bounded by the volume's size before a
                // single byte of it is asked for. Reading it unbounded would have the host walk the
                // address space looking for four gigabytes of buffer.
                let want = (count as usize).min(file::VOLUME_LIMIT);
                let Ok(bytes) = self.cpu.mem.read_bytes(src, want) else {
                    // `IFILE_Write` answers a count, and zero *is* its unsuccessful answer, so a
                    // request that cannot be honoured is refused here rather than propagating: a
                    // bad length in the app should not end the run.
                    self.volume.refuse(g::AEE_EBADPARM);
                    return Ok(0);
                };
                let Some(open) = self.files.get_mut(&this) else {
                    return Ok(0);
                };
                // A write the volume has no room for answers the same way, with the reason left for
                // `GetLastError`.
                Ok(self.volume.write(open, &bytes).unwrap_or(0))
            }
            "Seek" => {
                let [kind, distance] = self.args()?;
                // `moveDistance` is an `int32`, so a value past `i32::MAX` is a negative seek the
                // handset would answer as a bad position rather than a huge forward one.
                //
                // The three answer shapes belong to [`file::Volume::seek`] — `_SEEK_CURRENT` with a
                // zero distance returns the position, everything else returns 0 or 1, and the reason
                // for a refusal is left in the error cell for `IFileMgr::GetLastError`. A handle over
                // the pack's own bytes takes the branch that never asks the file system, which bounds
                // the cursor to the file's size instead of letting it run past the end.
                let distance = distance as i32;
                let Some(open) = self.files.get_mut(&this) else {
                    return Ok(g::AEE_EFAILED);
                };
                Ok(self.volume.seek(open, kind, distance))
            }
            "Truncate" => {
                let [len] = self.args()?;
                // `CFile_Truncate` returns a **`bool` in which TRUE means failure**, and the
                // file-system result is written into the record's last-error cell rather than
                // returned. `IFile` inherits that polarity from the file layer, so an `AEE_E*` code
                // here is the wrong shape of answer even when its value happens to read the same:
                // `AEE_SUCCESS` is 0 and `AEE_EFAILED` is 1, but the reason this store refused is
                // `AEE_EREADONLY` = 49, which is neither FALSE nor TRUE. [`file::Volume::truncate`]
                // keeps that reason where the documented behaviour keeps its own and answers the boolean.
                // (`IFileSystem::Truncate`, which takes a path, is a different slot with a different
                // answer and is not this one.)
                let Some(open) = self.files.get_mut(&this) else {
                    self.volume.refuse(g::AEE_EFAILED);
                    return Ok(g::TRUE);
                };
                Ok(if self.volume.truncate(open, len) {
                    g::TRUE
                } else {
                    g::FALSE
                })
            }
            "GetInfo" => {
                let [dst] = self.args()?;
                if dst == 0 || self.cpu.mem.mapped_run(dst, file::INFO_SIZE) < file::INFO_SIZE {
                    self.volume.refuse(g::AEE_EBADPARM);
                    return Ok(g::TRUE);
                }
                // The three facts come from the handle's own view of the file: `FillConstInfo` for a
                // reference entry (`FA_READONLY|FA_CONST`, and the view's own size), the file system for a
                // written one. A name that has left the volume is the body's `CFile_ReOpen(a1, 1) != 0` —
                // TRUE, **nothing written**, and the error cell still holding somebody else's reason.
                let Some(name) = self.files.get(&this).map(|open| open.name.clone()) else {
                    return Ok(g::TRUE);
                };
                let Some(info) = self.volume.handle_info(&name) else {
                    return Ok(g::TRUE);
                };
                self.put_file_info(dst, &info)?;
                // ⚠ One ordering detail worth keeping: the body copies the name from the record's own field
                // (`aee_strlcpy(a2 + 12, a1 + 356, 64)`) **after** the branch and regardless of the file
                // system's answer — a failed fill still hands back a name. [`Volume::handle_info`] builds
                // `Info::name` from that same stored name, so the one write here covers both.
                //
                // What the guest is being told about the file it just opened, at the offsets
                // `put_file_info` writes them. A pack that reads this struct through a different layout is
                // what a wrong answer here would look like from the outside, and the two container formats
                // in the sample set are the ones whose sizes a run can check against the disk.
                //
                // Folded into the notes rather than printed as it happens: a pack that asks for the same
                // container picture thirty-odd times in one screen buries everything else the run has to say.
                if name.ends_with(".par") || name.ends_with(".idx") {
                    self.display.note(format!(
                        "GetInfo {name}: attrib={:#x} size={} -> dst={dst:#010x}",
                        info.attrib, info.size
                    ));
                }
                // `*(a1[3] + 12) = v6` closes the body, and `v6` is 0 on this path — so a *successful*
                // GetInfo clears an older refusal out of the cell as well.
                self.volume.accept();
                Ok(g::FALSE)
            }
            // A cache-size hint for a handset that reads through a buffer. This volume is already the
            // bytes, so there is no cache to size and the answer is 0 — which the header names as the
            // return for "no cache" ("Size of the cache. If the input size == 0, the
            // cache is freed"), and which `CFile_SetCacheSize` also returns on every failure path.
            // Refusing with a status code was wrong for a slot whose answer is a size.
            "SetCacheSize" => Ok(0),
            // `IFile` inherits these two from `IAStream`, and both are `void`.
            //`CFile_Readable` parks the pair in the object and asks the
            // manager's vtable to wake the caller later; this volume's reads never block, so there is
            // nothing to park, and `CFile_Cancel`'s `CALLBACK_Cancel` has nothing to cancel. Answering
            // the void slots is the honest shape — raising `Unimplemented` ended the whole run on a
            // call whose return value no caller can read.
            "Readable" | "Cancel" => Ok(0),
            "GetInfoEx" => {
                let [dst] = self.args()?;
                // `a2 == nullptr || *a2 == 0` — both land in the same refusal, so read the word first and
                // test the pair once. A null `dst` is not read from, so it short-circuits to 0.
                let declared = if dst == 0 {
                    0
                } else {
                    self.cpu.mem.read_u32(dst)?
                };
                if declared == 0 || self.cpu.mem.mapped_run(dst, declared) < declared {
                    self.volume.refuse(g::AEE_EBADPARM);
                    return Ok(g::TRUE);
                }
                // The three facts, then the caller's own buffers. A name that has left the volume is the
                // body's `CFile_ReOpen(a1, 1) != 0`: TRUE with **nothing written** and the cell untouched.
                let Some(name) = self.files.get(&this).map(|open| open.name.clone()) else {
                    return Ok(g::TRUE);
                };
                let Some(info) = self.volume.handle_info(&name) else {
                    return Ok(g::TRUE);
                };
                self.put_info_facts_ex(dst, &info)?;
                // Then the extended record's second half — the same `CFileMgr_FillInfoEx` the manager
                // runs, name first and then **both** clears before any lookup. It reads the room from the
                // record's own `nStructSize`, which is why it takes the address rather than the facts.
                self.fill_info_ex(dst, &info)?;
                self.volume.accept();
                Ok(g::FALSE)
            }
            "Map" => {
                // `IFILE_Map(pIFile, pStart, dwSize, protections, flags, dwOffset)` answers with an
                // address, or NULL (introduced in BREW 3.1). The parameter
                // order is not the one a reader expects — **the offset comes last**, after the
                // protections and the flags — so reading it as `(pStart, dwOffset, dwSize, …)` yields
                // a mapping of the wrong bytes rather than an error, which is why it is written out
                // here in the comment and bound in that order below.
                let [pstart, size, protections, flags, offset] = self.args()?;
                // Five conditions are the header's to enforce. Each says so rather than answering the
                // bare zero the header prescribes: NULL is the right return for a failed mapping and
                // still the worst thing to read off a run — "no mapping" and "this host has never
                // heard of `Map`" are the same value to the caller.
                if pstart != 0 {
                    self.display.note(format!(
                        "IFile::Map named a base address ({pstart:#x}) although pStart should be 0, \
                         and this host has no address space of its own to place a mapping in — NULL"
                    ));
                    return Ok(0);
                }
                if protections & g::AEE_FMAP_PROT_WRITE != 0 {
                    self.display.note(format!(
                        "IFile::Map asked for protections {protections:#x}, which include \
                         AEE_FMAP_PROT_WRITE — the header says write is not permitted — NULL"
                    ));
                    return Ok(0);
                }
                if flags & (g::AEE_FMAP_FIXED | g::AEE_FMAP_PRIVATE) != 0 {
                    // The reference gate is one mask, not two: `if ((a5 & 5) != 0) return 0` — `5` is
                    // `AEE_FMAP_FIXED | AEE_FMAP_PRIVATE`, so either bit on its own and both together
                    // are refused the same way, and a note that named `FIXED` for a caller who asked
                    // for `PRIVATE` (the values are `1` and `4`) would be pointing at a bit that was
                    // never set. `FIXED` is the one with a *reason* beyond the mask: its own line in
                    // the header says "only input void * supported, otherwise fails"
                    // — while `pStart` has to be 0 two checks up, so the two requests
                    // cannot both be honoured. Without this arm `AEE_FMAP_FIXED | AEE_FMAP_SHARED`
                    // would sail through and "succeed" at an address this host chose.
                    let mut named: Vec<&str> = Vec::new();
                    for (bit, name) in [
                        (g::AEE_FMAP_FIXED, "AEE_FMAP_FIXED"),
                        (g::AEE_FMAP_PRIVATE, "AEE_FMAP_PRIVATE"),
                    ] {
                        if flags & bit != 0 {
                            named.push(name);
                        }
                    }
                    self.display.note(format!(
                        "IFile::Map passed flags {flags:#x}, which the body's own mask \
                         `(flags & 5)` refuses as {} — AEE_FMAP_SHARED ({:#x}) is the only flag that \
                         maps — NULL",
                        named.join(" and "),
                        g::AEE_FMAP_SHARED
                    ));
                    return Ok(0);
                }
                // There is no fifth gate. `CFile_Map` tests protections once — `if (a4 & 4) return 0`
                // — so a caller that asks for `0`, for `AEE_FMAP_PROT_NONE` or for execute alone is
                // answered like one that asked for read: the reference member has no opinion about which
                // of the non-writing bits are set, only that the writing one is not.
                if flags & g::AEE_FMAP_SHARED == 0 {
                    self.display.note(format!(
                        "IFile::Map passed flags {flags:#x}; only AEE_FMAP_SHARED is supported — NULL"
                    ));
                    return Ok(0);
                }
                // What is left is the volume's: the file has to be a const one and the range has to
                // fit inside it. Asked once for the length and again for the bytes, because
                // `heap_alloc` takes the arena mutably and the volume's own buffer cannot be borrowed
                // across that call.
                let name = self.files[&this].name.clone();
                let Some(view) = self.volume.const_bytes_at(&name, offset, size) else {
                    self.display.note(format!(
                        "IFile::Map cannot serve this handle: only a const file can be mapped, and \
                         {size} byte(s) at offset {offset} have to fit inside it — NULL"
                    ));
                    return Ok(0);
                };
                let addr = self.heap_alloc(view.len() as u32)?;
                if addr == 0 {
                    self.display
                        .note("IFile::Map: the arena is exhausted, so the mapping is NULL");
                    return Ok(0);
                }
                let view = self
                    .volume
                    .const_bytes_at(&name, offset, size)
                    .expect("the same range, checked above");
                self.cpu.mem.write_bytes(addr, view)?;
                // The view lives until the handle does: see [`Runtime::file_maps`].
                self.file_maps.entry(this).or_default().push(addr);
                Ok(addr)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The `IUnzipAStream` slots: a compressed stream read back as uncompressed bytes.
    ///
    /// the header publishes the interface as one that reads "zlib compressed data streams
    /// as uncompressed streams", and leaves the caller the choice of what to feed it — "a
    /// compressed file (such as a.gzip file), memory buffer, or a socket" — so `SetStream` names an
    /// `IAStream*` rather than a file name. The only streams this host hands out are `IFile`
    /// handles, so the bytes behind the naming are the volume's, and an `IAStream` the host never
    /// gave out is [`HleError::NotAnObject`] rather than an empty read: a stream at its end and a
    /// stream nobody can read are different answers.
    ///
    /// Nothing here raises. A pack asks for this class before it asks for anything else, so a run
    /// that ended on the first stream it could not inflate would report the
    /// class instead of the stream; the reason goes to [`Unzip::problem`] and to the screen's notes,
    /// which is where the file that did not inflate can be read by name.
    pub(crate) fn iunzipastream(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> HResult<u32> {
        if !self.unzip.contains_key(&this) {
            // Not an object this host handed out, so not one whose stream is knowable.
            return Err(HleError::NotAnObject { addr: this });
        }
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let retval = match name {
            "SetStream" => {
                let [stream] = self.args()?;
                if stream == 0 {
                    // `CUnzipStream_SetStream` wraps its whole body in `if (a2 != nullptr)`: a null
                    // source is ignored rather than unbound, so the stream keeps reading what it
                    // already holds.
                    return Ok(0);
                }
                let mut unzip = self.unzip.remove(&this).unwrap_or_default();
                if unzip.stream == stream && !unzip.inflate.finished {
                    // The same source object, refilled. A caller that feeds a window at a time does
                    // this between reads — it puts the next window in the same stream and names it
                    // again — so the inflater continues where it stopped rather than starting over.
                    // Once the compressed stream has ended, naming it again is a restart instead,
                    // which is what a caller reading the same source twice means.
                    unzip.inflate.source_ended = false;
                } else {
                    unzip = Unzip {
                        stream,
                        ..Default::default()
                    };
                }
                self.unzip.insert(this, unzip);
                0
            }
            "Read" => {
                let [dst, want] = self.args()?;
                if dst == 0 || want == 0 {
                    // `IASTREAM_Read` answers a count, and a stream at its end answers zero — the
                    // same answer a failed read gets, exactly as `IFile` does it.
                    return Ok(0);
                }
                self.unzip_pump(this, want as usize)?;
                let (bytes, first) = match self.unzip.get_mut(&this) {
                    Some(unzip) => {
                        let n = unzip.bytes.len().min(want as usize);
                        let first = unzip.delivered == 0;
                        unzip.delivered += n;
                        (unzip.bytes.drain(..n).collect::<Vec<u8>>(), first)
                    }
                    None => return Ok(0),
                };
                if bytes.is_empty() {
                    return Ok(0);
                }
                self.cpu.mem.write_bytes(dst, &bytes)?;
                // A pack that inflates its own `program.bin` is loading a second BREW-headered
                // module behind this host's back. Its reference area and writable range are facts
                // the host can report without pretending to relocate it: the pack's entry point
                // still owns the decision, but a run that never applies the fixup now says so.
                if first && bytes.len() >= 0x2c && bytes.get(8..12) == Some(&b"BREW"[..]) {
                    if let Some(r) = crate::loader::relocation(&bytes) {
                        let refdata = dst.wrapping_add(r.image).wrapping_add(r.ref_off);
                        let writable = refdata.wrapping_add(r.fixup_off);
                        self.display.note(format!(
                            "{label}: inflated BREW image at {dst:#010x}: refdata {refdata:#010x}, fixup_off {:#x}, ref_size {:#x}, writable {writable:#010x}..{:#010x}",
                            r.fixup_off,
                            r.ref_size,
                            writable.wrapping_add(r.ref_size)
                        ));
                    }
                }
                bytes.len() as u32
            }
            // `CUnzipStream_Readable` stores the pair and registers it with the source's own
            // registrar (`+52`, slot 36). The slot is `void`, so a caller that registers is a
            // caller that has run out of bytes and has nothing to do until it hears back — and a
            // source this host already holds is readable the moment it is asked, so the answer is
            // raised on the next turn instead of never. One guest pack inflates its whole data
            // set this way: it names a memory stream, registers here, and re-registers every frame
            // while it waits, so a registration that is never answered leaves it polling a clock
            // forever with nothing drawn.
            //
            // Re-registering replaces the pending notification rather than adding to it: the pair
            // *is* the registration, and a caller that asks again before the turn that would have
            // fired it is asking once. That also keeps a pack's per-frame re-registration from
            // growing the queue without bound.
            "Readable" => {
                let [pfn, user] = self.args()?;
                let mut raise = false;
                if let Some(unzip) = self.unzip.get_mut(&this) {
                    unzip.notify = pfn;
                    unzip.notify_user = user;
                    // Nothing left to inflate and nothing buffered means there is nothing to be
                    // readable *for*; a stream at its end answers a `Read` with zero and has no
                    // notification to raise, which is how a caller learns it is done.
                    raise = pfn != 0 && (!unzip.inflate.finished || !unzip.bytes.is_empty());
                }
                if raise {
                    self.dequeue_notify(pfn, user);
                    self.queue.push_back(Job::Notify { pfn, user });
                }
                0
            }
            // `CUnzipStream_Cancel` clears the same pair, then calls the stream's own cancel
            // callback (`+32`). This host has none to call, and clearing is the whole effect the
            // caller can observe — a `void` slot that used to end the run instead. A notification
            // already queued for that pair goes with it, because the caller has just said it is no
            // longer waiting.
            "Cancel" => {
                if let Some(unzip) = self.unzip.get_mut(&this) {
                    let (pfn, user) = (unzip.notify, unzip.notify_user);
                    unzip.notify = 0;
                    unzip.notify_user = 0;
                    if pfn != 0 {
                        self.dequeue_notify(pfn, user);
                    }
                }
                0
            }
            _ => return Err(unimplemented().into()),
        };
        Ok(retval)
    }

    /// Drop a `Readable` notification that has not been delivered yet, so a pair can be registered
    /// once rather than piling up a queue entry per re-registration.
    ///
    /// The pair is the registration — `PFNNOTIFY` and its context are the whole of what the caller
    /// named — so two entries for one pair would be one callback run twice.
    fn dequeue_notify(&mut self, pfn: u32, user: u32) {
        self.queue.retain(|job| {
            !matches!(job, Job::Notify { pfn: p, user: u } if *p == pfn && *u == user)
        });
    }

    /// Inflate until there is something to hand out, pulling the source as it goes.
    ///
    /// A caller that feeds a window at a time refills the source between reads, so "the source
    /// answered zero" ends this *attempt* and not the stream: the next `SetStream` on the same
    /// object clears [`Inflate::source_ended`] and this resumes with the inflater's own state, its
    /// window and its bit position, still in place.
    fn unzip_pump(&mut self, this: u32, want: usize) -> HResult<()> {
        /// How much one pull asks the source for.
        const CHUNK: u32 = 16 * 1024;
        loop {
            self.unzip_step(this)?;
            let Some(unzip) = self.unzip.get(&this) else {
                return Ok(());
            };
            let (have, finished, ended, stream) = (
                unzip.bytes.len(),
                unzip.inflate.finished,
                unzip.inflate.source_ended,
                unzip.stream,
            );
            if have >= want || finished || ended {
                let note = match (unzip.problem.clone(), unzip.noted) {
                    (Some(problem), false) => Some(problem),
                    _ => None,
                };
                if let Some(problem) = note {
                    if let Some(unzip) = self.unzip.get_mut(&this) {
                        unzip.noted = true;
                    }
                    // The note is the only place a failing stream is named: the object the caller
                    // reads it through carries no name of its own.
                    self.display.note(format!("IUnzipAStream: {problem}"));
                }
                return Ok(());
            }
            let chunk = self.read_stream_chunk(stream, CHUNK)?;
            let Some(unzip) = self.unzip.get_mut(&this) else {
                return Ok(());
            };
            if chunk.is_empty() {
                unzip.inflate.source_ended = true;
            } else if unzip.inflate.dec.is_none() {
                // Still deciding the wrapper: a gzip header can be longer than one window.
                unzip.inflate.head.extend_from_slice(&chunk);
            } else {
                unzip.inflate.input.extend_from_slice(&chunk);
            }
        }
    }

    /// One step of the inflater: build it once the wrapper is finally known, then run it over the
    /// input on hand.
    fn unzip_step(&mut self, this: u32) -> HResult<()> {
        let Some(unzip) = self.unzip.get_mut(&this) else {
            return Ok(());
        };
        let Unzip {
            bytes,
            inflate,
            problem,
            ..
        } = unzip;
        let Inflate {
            dec,
            wrap,
            input,
            input_pos,
            head,
            finished,
            retried,
            produced,
            ..
        } = inflate;
        if *finished {
            return Ok(());
        }
        if dec.is_none() {
            let Some((kind, skip)) = wrapping(head) else {
                return Ok(());
            };
            *wrap = kind;
            *input = head.split_off(skip);
            *input_pos = 0;
            *dec = Some(Box::new(flate2::Decompress::new(kind == Wrap::Zlib)));
        }
        let dec = dec.as_mut().expect("built above when there was none");
        let mut out = vec![0u8; 64 * 1024];
        loop {
            let before_in = dec.total_in();
            let before_out = dec.total_out();
            let status = dec.decompress(
                &input[*input_pos..],
                &mut out,
                flate2::FlushDecompress::None,
            );
            let used = (dec.total_in() - before_in) as usize;
            let made = (dec.total_out() - before_out) as usize;
            *input_pos += used;
            if made > 0 {
                *produced += made;
                bytes.extend_from_slice(&out[..made]);
            }
            match status {
                Ok(flate2::Status::StreamEnd) => {
                    *finished = true;
                    return Ok(());
                }
                Ok(_) => {
                    if used == 0 && made == 0 {
                        // The input on hand is spent; whoever called this pulls more.
                        return Ok(());
                    }
                    if *produced >= INFLATE_CAP {
                        *problem = Some(format!(
                            "the stream inflates to {INFLATE_CAP} bytes or more"
                        ));
                        *finished = true;
                        return Ok(());
                    }
                }
                Err(e) => {
                    // A raw deflate stream that happens to begin with a zlib header is the one case
                    // this decides wrongly, and the alternative gets its turn once — the same
                    // fallback the one-shot reader this replaced had.
                    if !*retried && *wrap == Wrap::Zlib && bytes.is_empty() {
                        *retried = true;
                        *wrap = Wrap::Raw;
                        *input_pos = 0;
                        *dec = Box::new(flate2::Decompress::new(false));
                        continue;
                    }
                    *problem = Some(format!("not a deflate stream: {e}"));
                    *finished = true;
                    return Ok(());
                }
            }
        }
    }

    /// `AEESoundPlayer_IntSetInfo`'s `SDT_FILE` branch: refuse a blank name, and store an owned copy
    /// of any other.
    ///
    /// The reference branch is three statements — `if ( STRCMP((char *)pInfo->pData, "") == 0 ) return
    /// EBADPARM; pme->m_spInfo.pData = STRDUP((char *)pInfo->pData); pme->m_spInfo.dwSize = 0;` —
    /// and the blank-name operand is the empty string, which the published description spells out even
    /// though the delivered object's literal pool could not be read for it. `false` is that
    /// `EBADPARM`, and the copy the record then holds is what `GetInfo` reports and what
    /// `FreeData` frees.
    pub(crate) fn install_file_name(&mut self, player: &mut Player, data: u32) -> HResult<bool> {
        let Ok(name) = self.cstring(data) else {
            // A name this host cannot read is no name at all; the reference `STRCMP` would walk off
            // the same edge, and there is no answer it could give that a pack could use.
            return Ok(false);
        };
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return Ok(false);
        }
        let copy = self.hle.alloc(&mut self.cpu.mem, bytes.len() as u32 + 1)?;
        self.cpu.mem.write_bytes(copy, bytes)?;
        self.cpu.mem.write_u8(copy + bytes.len() as u32, 0)?;
        player.name_buffer = copy;
        player.input = g::SDT_FILE;
        player.data = copy;
        player.size = 0;
        Ok(true)
    }

    /// The text a resource id names, as a pointer a control can read. `ISHELL_LoadResString` writes
    /// into a buffer its caller owns; here the caller is a host door, so the bytes land in a block
    /// this host allocated and the control copies them out of it the way it copies out of a pack's.
    pub(crate) fn res_string(&mut self, file: Option<&str>, id: u16) -> HResult<u32> {
        let Some(units) = self.resources.string(file, id) else {
            return Ok(0);
        };
        // The store's text is Unicode; the control reads it out of guest memory as `AECHAR`, so it
        // has to be in the device's units like every other string the pack hands it.
        let units = self
            .text_encoding()
            .string_to_units(&TextEncoding::Unicode.units_to_string(&units));
        let room = (units.len() as u32 + 1) * 2;
        let at = self.heap_alloc(room)?;
        if at == 0 {
            return Ok(0);
        }
        for (i, u) in units.iter().enumerate() {
            self.cpu.mem.write_u16(at + i as u32 * 2, u32::from(*u))?;
        }
        self.cpu.mem.write_u16(at + units.len() as u32 * 2, 0)?;
        Ok(at)
    }

    /// Every `IHeap` slot. A handset gives an app a heap of its own; this host forwards to the one it
    /// already has, so a module that manages memory through `IHeap` and one that goes through
    /// `AEEHelperFuncs::malloc` draw on the same arena — see `HOST_SERVICES`.
    pub(crate) fn iheap(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "Malloc" => {
                let [size] = self.args()?;
                if alloc_request(size).0 == 0 {
                    return Ok(0);
                }
                self.heap_alloc(size)
            }
            "Realloc" => {
                let [block, size] = self.args()?;
                if alloc_request(size).0 == 0 {
                    if block != 0 {
                        self.heap_free(block);
                    }
                    return Ok(0);
                }
                self.heap_realloc(block, size)
            }
            "Free" => {
                let [block] = self.args()?;
                // `FREE(NULL)` is `aee_realloc(NULL, 0)`: nothing to give back, and no note to
                // raise about an address the caller never named.
                if block != 0 {
                    self.heap_free(block);
                }
                Ok(0)
            }
            "StrDup" => {
                let [src] = self.args()?;
                if src == 0 {
                    return Ok(0);
                }
                let units = self.cpu.mem.read_u16str(src, 4096)?;
                let keep = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                let dst = self.heap_alloc((keep as u32 + 1) * 2)?;
                if dst == 0 {
                    return Ok(0);
                }
                for (i, u) in units.iter().take(keep).enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(dst)
            }
            // The two accounting slots, answered from what the arena actually holds. `CheckAvail` says
            // yes and the allocation that follows is the real answer; the stats are the numbers below.
            //
            // They used to be zeroes. `pCurHeapUsed` and the `GetMemStats` return value are
            // "currently in use", for which 0 is never legitimate on a run that has allocated
            // anything — that half is the `answered-zero` rule.
            // `pMaxHeap` is the exception the header spells out and I did not read first: for
            // `clsid == 0` it "is not provided and is set to 0", so a zero there was the right answer
            // and is not evidence about why any pack quit.
            //
            // Records, counted rather than remembered: a pack can print `[mod] report: maxmem 0` /
            // `curmem 0` and close its own applet; the pack that asks the accounting slots most has
            // hundreds of `IHeap::GetMemStats` entries on one route and none on another, so the figure
            // belongs to a route and a run, not to the pack.
            // `IHeap_CheckAvail(n)` is not a guess: `AEEHeap_CheckMemAvail` allocates `n` with
            // `ALLOC_NO_ZMEM`, frees it, and answers whether the allocation succeeded — and a
            // zero-length question is TRUE without touching the heap. This host asks its own
            // arena the same way rather than promising bytes it may not have.
            "CheckAvail" => {
                let [size] = self.args()?;
                let (bytes, _) = alloc_request(size);
                if bytes == 0 {
                    return Ok(g::TRUE);
                }
                let probe = self.heap_alloc(size)?;
                let ok = probe != 0;
                if probe != 0 {
                    self.hle.dealloc(&mut self.cpu.mem, probe);
                }
                Ok(if ok { g::TRUE } else { g::FALSE })
            }
            "GetMemStats" => Ok(self.heap_live_bytes()),
            "GetModuleMemStats" => {
                let [cls, pmax, pcur] = self.args()?;
                self.module_mem_stats(cls, pmax, pcur)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Bytes the arena has handed out and not got back, which is what `IHeap::GetMemStats` asks for.
    ///
    /// `Mem::heap_high` is how far the heap has ever grown and `Mem::heap_spare` is the sum of its
    /// free nodes, which is where the handed-back space sits. Subtracting one from the other is the
    /// honest "in use now", and both numbers already drive the report line
    /// (`heap: N KiB in use of the 8 MiB arena`), so this is not a figure invented for an answer.
    /// The heap itself never grows here — its whole span is one free node from `set_heap` — so the
    /// high-water is the span end and this figure counts what is *charged*, headers included.
    ///
    /// A helper, not inline, because the `match` above is the down-call recursion frame and
    /// [`MAX_DOWNCALL_DEPTH`] is a stack budget measured in bytes of that frame — 63 264 of them a
    /// level on the debug profile when the ceiling was set, so a local added to the match costs
    /// [`MAX_DOWNCALL_DEPTH`] of them.
    pub(crate) fn heap_live_bytes(&self) -> u32 {
        let reached = self.cpu.mem.heap_high().saturating_sub(layout::HEAP_BASE);
        reached.saturating_sub(self.cpu.mem.heap_spare())
    }

    pub(crate) fn module_mem_stats(&mut self, cls: u32, pmax: u32, pcur: u32) -> HResult<u32> {
        // A class the host never adopted has no stats to report under that name.
        if cls != 0 && self.app_cls != 0 && cls != self.app_cls {
            return Ok(g::AEE_ENOSUCH);
        }
        // The header's two null-pointer rules, which are not one rule: for `clsid == 0` only
        // `pCurHeapUsed` is required, and for a non-zero class `EBADPARM` needs **both** pointers
        // null. Refusing a legal call is how a fix breaks a pack that was never broken, and writing
        // through a pointer the caller was allowed to leave out faults the guest at address 0 — my
        // first version of this function did each of those once, and the test caught both.
        // `pcur == 0` alone is the whole rule once both clauses are read together: the header's
        // "both null" case is a subset of it, so writing them as two conditions only invites someone
        // to "fix" the redundant one and introduce the bug back.
        if pcur == 0 {
            self.display
                .note("GetModuleMemStats: nowhere to write the heap figures, so EBADPARM");
            return Ok(g::AEE_EBADPARM);
        }
        if pmax != 0 {
            // For system stats (`clsid == 0`) the maximum "is not provided and is set to 0": a
            // non-zero figure there would be the host inventing what the interface promises is
            // absent. The allowance is reported only for a module, and this host has one arena, so
            // it is the global figure — see the note on the doc comment about what is *not* modelled.
            let allowance = if cls == 0 {
                0
            } else {
                // The whole span this host lets the heap grow into, not what it has grown to: the
                // run's own ceiling (`--heap-mb`, 8 MiB by default) of lazily-paged guest space.
                self.cpu.mem.heap_limit().saturating_sub(layout::HEAP_BASE)
            };
            self.cpu.mem.write_u32(pmax, allowance)?;
        }
        self.cpu.mem.write_u32(pcur, self.heap_live_bytes())?;
        Ok(g::AEE_SUCCESS)
    }

    /// Every `IRscPool` slot: the one heap this host has, plus the list of resource pointers the
    /// class exists to keep.
    ///
    /// The list is the interface, not bookkeeping. `HoldRsc` must *not* `AddRef` what it is given —
    /// The header run the pointer's own `Release`, which for an object the module built is module code. So that
    /// call goes back out through the guest's vtable rather than moving a reference count here. That
    /// is also why this arm returns the runtime's own error type rather than an `HleError`: a
    /// downcall can fail as a guest fault or as a stop the run was asked for, and neither of those is
    /// a gap in the host.
    pub(crate) fn irscpool(
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
            // `IRscPool` inherits `IQI`, which `IHeap` does not, so this slot is real.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                Ok(self.answer_qi(this, iid, ppv, &[g::AEEIID_IRscPool])?)
            }
            "Malloc" => {
                let [size] = self.args()?;
                Ok(self.heap_alloc(size)?)
            }
            "Free" => {
                // The block the pool allocated with `Malloc` goes back to the arena, which is the same
                // reclaim `IHeap::Free` and the `free` helper do.
                let [block] = self.args()?;
                self.heap_free(block);
                Ok(0)
            }
            "HoldRsc" => {
                let [rsc] = self.args()?;
                if rsc == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                self.rscpool.entry(this).or_default().push(rsc);
                Ok(g::AEE_SUCCESS)
            }
            "ReleaseRsc" => {
                let [rsc] = self.args()?;
                let at = self
                    .rscpool
                    .get(&this)
                    .and_then(|held| held.iter().position(|p| *p == rsc));
                let Some(at) = at else {
                    // `-1` is what the header has the pool answer for a pointer it never held.
                    return Ok(u32::MAX);
                };
                if let Some(held) = self.rscpool.get_mut(&this) {
                    held.remove(at);
                }
                // `Release` is slot 1 of every BREW vtable, whichever interface it was laid out for.
                // A pointer with no readable vtable behind it is the header's own "never held" answer,
                // not a fault: `HoldRsc` accepts any nonzero word, so a pack that held a stale one has
                // to get `-1` rather than have its own cleanup end the run. A fault from inside the
                // released object's `Release` is a different thing and still travels.
                let Ok(release) = self.guest_slot(rsc, "IQI", "Release") else {
                    return Ok(u32::MAX);
                };
                Ok(self.call_guest(release, &[rsc])?)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    pub(crate) fn resource_ctl_cancel_stub(&mut self) -> Result<u32, Error> {
        Ok(0)
    }

    pub(crate) fn resource_status_notify(&mut self) -> Result<(), Error> {
        let (mask, pcb) = self.res_ctl.notify;
        if pcb == 0 {
            return Ok(());
        }
        let status = if self.res_ctl.held {
            g::RESCTL_STATUS_ACQUIRED
        } else {
            g::RESCTL_STATUS_FREE
        };
        if mask & status == 0 {
            return Ok(());
        }
        if self.res_ctl.wake == 0 {
            self.res_ctl.wake = self.heap_alloc(4)?;
        }
        self.dispatcher.resume_from(self.res_ctl.wake, self.applet);
        Ok(())
    }

    pub(crate) fn resource_obj_notify(&mut self) -> Result<(), Error> {
        let pcb = self.res_ctl.notify.1;
        self.res_ctl.notify = (0, 0);
        if pcb == 0 {
            return Ok(());
        }
        let at = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        let pfn = self.cpu.mem.read_u32(pcb + at("pfnNotify"))?;
        if pfn == 0 {
            return Ok(());
        }
        let user = self.cpu.mem.read_u32(pcb + at("pNotifyData"))?;
        self.call_guest(pfn, &[user])?;
        Ok(())
    }

    pub(crate) fn resource_ctl_died(&mut self, addr: u32) -> Result<(), Error> {
        // An OEM controller keeps its state beside the object rather than in `res_ctl`; the last
        // release is where that entry goes, or a rebuilt controller of the same class would find the
        // previous one's status waiting for it.
        if let Some(idx) = self.oem_res_ctl.iter().position(|(obj, _)| *obj == addr) {
            self.oem_res_ctl.remove(idx);
            return Ok(());
        }
        if self.resource_ctl == 0 || addr != self.resource_ctl {
            return Ok(());
        }
        self.resource_ctl = 0;
        self.res_ctl.notify = (0, 0);
        self.res_ctl.held = false;
        self.res_ctl.reason = g::RESCTL_REASON_BASE + 1;
        self.res_ctl.relinquish.clear();
        if self.res_ctl.wake != 0 {
            self.dispatcher.forget(self.res_ctl.wake);
        }
        Ok(())
    }

    /// The two gates `IResourceCtl_Request` and `IResourceCtl_CanAcquire` share: reasons 1..0xFFF are
    /// bad parameters, and a reason at or above 0x1000000 names a group the calling app must belong to
    /// (`AEE_AppIsInGroup(reason)` in the body).
    pub(crate) fn resource_reason_ok(&self, reason: u32) -> bool {
        if reason != 0 && reason < g::RESCTL_REASON_BASE {
            return false;
        }
        if reason >= 0x0100_0000 && self.app_in_group(self.app_cls, reason) != g::AEE_SUCCESS {
            return false;
        }
        true
    }

    pub(crate) fn iapphistory(
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
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if iid == g::AEEIID_IModel {
                    // The body answers this id with the `IModel` vtable it embeds at `[+40]`, then
                    // `AddRef`s **that pointer** through the shared `AEEBase_AddRef` — which increments
                    // `[+44]`, where the constructor stored a copy of `this`, rather than the count at
                    // `[+4]`. That alias follows from reusing the base entry points on a pointer that is
                    // not the object; the companion face here gets the slot numbers right and leaves one
                    // honest counter, which is the part a caller can actually depend on.
                    let Some(model) = self.history_objects.get(&this).map(|o| o.model) else {
                        return Err(Error::Hle(unimplemented()));
                    };
                    return self.return_object(model, ppv).map_err(Error::Hle);
                }
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_APPHISTORY])
                    .map_err(Error::Hle)
            }
            // `Forward` and `Back` share their gates in the same order, then move in opposite
            // directions: no entry at all is `EFAILED`, an unlinked entry is `AEE_EINVALIDITEM`(42)
            // rather than a move, and running off either end is `AEE_ENOSUCH`(39) — a code the body
            // reaches on purpose, so it is what a caller sitting at the bottom of the list gets.
            "Forward" | "Back" => {
                let Some(cur) = self.history_cursor(this) else {
                    return Err(Error::Hle(unimplemented()));
                };
                if self.history.entry(cur).is_none() {
                    return Ok(g::AEE_EFAILED);
                }
                if self.history.entry(cur).map(|e| e.unlinked).unwrap_or(true) {
                    return Ok(g::AEE_EINVALIDITEM);
                }
                let step = if name == "Forward" { 1 } else { -1 };
                let Some(moved) = self.history.neighbour(cur, step) else {
                    return Ok(g::AEE_ENOSUCH);
                };
                if let Some(obj) = self.history_objects.get_mut(&this) {
                    obj.current = moved;
                }
                Ok(g::AEE_SUCCESS)
            }
            // `Top` is the same shape with the head as its target, and its one refusal is an empty
            // chain (`gBREW[238] == 0`), which the body answers `EFAILED` rather than success-zero.
            "Top" => {
                let Some(first) = self.history.linked().first().copied() else {
                    return Ok(g::AEE_EFAILED);
                };
                let head = self.history.entries[first].id;
                if let Some(obj) = self.history_objects.get_mut(&this) {
                    obj.current = head;
                }
                Ok(g::AEE_SUCCESS)
            }
            // `Current(pIndex)` numbers positions **from the head**, and an unlinked entry is not
            // "position zero by accident": the body writes `-1`, which is how a pack learns the
            // applet it is standing on has been taken out of the list.
            "Current" => {
                let [pindex] = self.args()?;
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                let cur = entry.id;
                let at = if entry.unlinked {
                    -1i32
                } else {
                    self.history.position(cur).unwrap_or(0) as i32
                };
                self.cpu.mem.write_u32(pindex, at as u32)?;
                Ok(g::AEE_SUCCESS)
            }
            // `GetClass` checks the out-pointer **before** it looks at the chain, unlike the four
            // slots below it — so a caller with no cursor still gets `EBADPARM` from this one.
            "GetClass" => {
                let [pcls] = self.args()?;
                if pcls == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                self.cpu.mem.write_u32(pcls, entry.cls)?;
                Ok(g::AEE_SUCCESS)
            }
            // `GetArgs` is the other double-duty out-parameter in this member: `pcb` arrives as the
            // capacity the copy may write and leaves as **the length needed, terminator included**, and
            // a caller that only wants the size passes no buffer and reads the number back. An entry
            // with no launch string answers **`*pcb = 0`** (`node[+20] == 0`), not one.
            "GetArgs" => {
                let [buf, pcb] = self.args()?;
                if pcb == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                match entry.args {
                    None => {
                        self.cpu.mem.write_u32(pcb, 0)?;
                    }
                    Some(args) => {
                        if buf != 0 {
                            let have = self.cpu.mem.read_u32(pcb)?.saturating_sub(1) as usize;
                            if have > 0 {
                                let keep = have.min(args.len());
                                self.cpu.mem.write_bytes(buf, &args.as_bytes()[..keep])?;
                                self.cpu.mem.write_u8(buf + keep as u32, 0)?;
                            }
                        }
                        let need = args.len() as u32 + 1;
                        self.cpu.mem.write_u32(pcb, need)?;
                    }
                }
                Ok(g::AEE_SUCCESS)
            }
            // `GetData` and `SetData` both run the group test first and the parameter tests after it,
            // and a key that is not there is a **success with `*pcb = 0`** — the caller is asking what
            // this applet stored, and "nothing" is an answer, not a failure.
            "GetData" => {
                let [key_at, data, pcb] = self.args()?;
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                if pcb == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let key = self.cstring_limited(key_at, HISTORY_MAX_BYTES + 1)?;
                let found = entry
                    .data
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.clone());
                match found {
                    Some(bytes) => {
                        let need = bytes.len() as u32;
                        if data != 0 {
                            let have = self.cpu.mem.read_u32(pcb)? as usize;
                            let keep = have.min(bytes.len());
                            self.cpu.mem.write_bytes(data, &bytes[..keep])?;
                        }
                        self.cpu.mem.write_u32(pcb, need)?;
                    }
                    None => {
                        self.cpu.mem.write_u32(pcb, 0)?;
                    }
                }
                Ok(g::AEE_SUCCESS)
            }
            "SetData" => {
                let [key_at, data, cb] = self.args()?;
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                let key = self.cstring_limited(key_at, HISTORY_MAX_BYTES + 1)?;
                let id = entry.id;
                let at = entry.data.iter().position(|(k, _)| *k == key);
                if at.is_some() {
                    let same = entry.data[at.unwrap()].1.len() == cb as usize;
                    if same {
                        let bytes = self.cpu.mem.read_bytes(data, cb as usize)?;
                        if let Some(entry) = self.history.entry_mut(id) {
                            entry.data[at.unwrap()].1 = bytes;
                        }
                        return Ok(g::AEE_SUCCESS);
                    }
                    if let Some(entry) = self.history.entry_mut(id) {
                        entry.data.remove(at.unwrap());
                    }
                    if cb == 0 {
                        // A different length with nothing to put in it is the body's delete path: it
                        // unlinks, frees, and answers **success**, which is why `SetData(key, NULL, 0)`
                        // is how a pack takes an item back out.
                        return Ok(g::AEE_SUCCESS);
                    }
                }
                if (cb as i32) <= 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let bytes = self.cpu.mem.read_bytes(data, cb as usize)?;
                if let Some(entry) = self.history.entry_mut(id) {
                    if entry.data.len() >= HISTORY_MAX_ITEMS {
                        return Ok(g::AEE_ENOMEMORY);
                    }
                    if key.len() + bytes.len() > HISTORY_MAX_BYTES {
                        return Ok(g::AEE_EBADPARM);
                    }
                    // Newest first: the documented behaviour links the fresh node at the head and `GetData`
                    // walks from there.
                    entry.data.insert(0, (key, bytes));
                }
                Ok(g::AEE_SUCCESS)
            }
            // `Insert` puts a new entry **after the cursor** and does not start an applet — which is
            // why its group test runs against the class the caller names, not the one the cursor
            // stands on. Three shapes the body gives that a host easily "improves" away:
            // ① the entry is born with reason [`HISTORY_NEW_REASON`], because `AHistory_New`'s fourth
            // argument lands at `node[+32]` and both call sites (`Insert`, and the shell's app-start)
            // pass that same literal; ② an **empty** argument string is stored as "none"
            // (`if (a3 && *a3)`), which is what `GetArgs` then answers `*pcb = 0` for; ③ a cursor left
            // on an unlinked entry is **not** a refusal — `AHistory_InsertUtil` splices the new node
            // after that tombstone, so the call succeeds and the entry never reaches the chain.
            "Insert" => {
                let [cls, pargs] = self.args()?;
                let Some(cur) = self.history_cursor(this) else {
                    return Err(Error::Hle(unimplemented()));
                };
                if self.history.entry(cur).is_none() {
                    return Ok(g::AEE_EFAILED);
                }
                if self.app_in_group(cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                let args = if pargs == 0 {
                    None
                } else {
                    let text = self.cstring_limited(pargs, HISTORY_ARGS_READ_CAP)?;
                    (!text.is_empty()).then_some(text)
                };
                let id = self.history.next_id;
                self.history.next_id += 1;
                // `AHistory_New` returning NULL is the body's `AEE_ENOMEMORY`; this host's allocator
                // has no failing path, so that refusal is unreachable here rather than unimplemented.
                let after = self
                    .history
                    .linked()
                    .iter()
                    .position(|&i| self.history.entries[i].id == cur);
                let fresh = HistoryEntry {
                    id,
                    cls,
                    args,
                    reason: HISTORY_NEW_REASON,
                    unlinked: after.is_none(),
                    ..Default::default()
                };
                match after {
                    Some(at) => {
                        let idx = self.history.linked()[at];
                        self.history.entries.insert(idx + 1, fresh);
                    }
                    None => self.history.entries.push(fresh),
                }
                self.notify_history_listeners()?;
                Ok(g::AEE_SUCCESS)
            }
            // `Remove` carries three refusals and one non-obvious success. Bit 3 is `AEE_EALREADY`,
            // a class outside the group is `AEE_EPRIVLEVEL`, and the head of the chain is **not
            // unlinked**: the body sets bit 4 and resumes the shell's own close-this-applet callback,
            // because taking the running applet out of the history list is not how a handset ends it.
            "Remove" => {
                let Some(cur) = self.history_cursor(this) else {
                    return Err(Error::Hle(unimplemented()));
                };
                let Some(entry) = self.history.entry(cur) else {
                    return Ok(g::AEE_EFAILED);
                };
                if entry.unlinked {
                    return Ok(g::AEE_EALREADY);
                }
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                let is_head = self.history.position(cur) == Some(0);
                if is_head {
                    if let Some(entry) = self.history.entry_mut(cur) {
                        entry.exiting = true;
                    }
                    self.display.note(format!(
                        "{label}::Remove: the cursor is on the head of the chain, so the shipped \
                         body flags that applet and resumes the shell's close-applet callback \
                         instead of unlinking it; this host records the request to exit and stops"
                    ));
                    return Ok(g::AEE_SUCCESS);
                }
                // Out of the chain, but the record stays: the reference node lives while its applet
                // does, which is why a cursor left on one answers 42 and `-1` rather than falling
                // into the "there is no such entry" path. `AHistory_Unlink` then wakes every model
                // listener, and its last act is to `App_Close` the applet if no other entry names
                // that class — this host has no second applet to close (the head entry is the one
                // it is running), so that half is the same boundary as the head branch above.
                if let Some(entry) = self.history.entry_mut(cur) {
                    entry.unlinked = true;
                }
                self.notify_history_listeners()?;
                Ok(g::AEE_SUCCESS)
            }
            // `Move(nIndex)` re-positions the entry the cursor sits on. `-1` is the end and anything
            // `Move(nIndex)` re-positions the entry the cursor sits on. The gates come in the body's
            // order, and two of them are easy to lose: `n < -1 ⇒ 14` and the group test `⇒ 21` come
            // **before** the "my node is not in the chain" refusal, and a target past the current
            // position is **bumped by one** because the mover is taken out before it is put back.
            // Running off the end is `AEE_ENOSUCH`(39) — the body reaches that code on purpose — with
            // two exceptions: `n == 0` means "to the head", and `n == -1` means "to the end" (and is
            // a silent no-op returning 0 when the mover is already the tail).
            "Move" => {
                let [nindex] = self.args()?;
                let n = nindex as i32;
                let Some(cur) = self.history_cursor(this) else {
                    return Err(Error::Hle(unimplemented()));
                };
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                if n < -1 {
                    return Ok(g::AEE_EBADPARM);
                }
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                if entry.unlinked {
                    return Ok(g::AEE_EFAILED);
                }
                let chain = self.history.linked();
                let at = chain
                    .iter()
                    .position(|&i| self.history.entries[i].id == cur)
                    .unwrap_or(0);
                let len = chain.len();
                if n as usize == at {
                    return Ok(g::AEE_SUCCESS);
                }
                if n == -1 && at == len - 1 {
                    return Ok(g::AEE_SUCCESS);
                }
                if n > at as i32 && (n as usize) >= len {
                    return Ok(g::AEE_ENOSUCH);
                }
                // Rebuild the chain without the mover and put it back at the place asked for. The
                // unlinked entries keep their records but take no position, which is what keeps a
                // cursor left on one answering `42` and `-1` instead of "no such entry".
                let id = entry.id;
                let mut rest: Vec<HistoryEntry> = Vec::with_capacity(self.history.entries.len());
                let mut aside: Vec<HistoryEntry> = Vec::new();
                for record in std::mem::take(&mut self.history.entries) {
                    if record.unlinked {
                        aside.push(record);
                    } else {
                        rest.push(record);
                    }
                }
                let at2 = rest.iter().position(|e| e.id == id).unwrap_or(0);
                let mover = rest.remove(at2);
                let to = if n < 0 || (n as usize) > rest.len() {
                    rest.len()
                } else {
                    n as usize
                };
                rest.insert(to, mover);
                rest.extend(aside);
                self.history.entries = rest;
                if at == 0 || n == 0 {
                    self.display.note(format!(
                        "{label}::Move: moving the head entry ends the platform routine with \
                         AEE_SuspendApps and a resume of every listener; this host has no \
                         suspension to run, so it reordered the chain and wakes the listeners here"
                    ));
                }
                self.notify_history_listeners()?;
                Ok(g::AEE_SUCCESS)
            }
            // `GetReason` validates its pointer; `SetReason` validates nothing at all, not even a
            // reason it has heard of — the word is the applet's message to whoever closes it.
            "GetReason" => {
                let [preason] = self.args()?;
                if preason == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                self.cpu.mem.write_u32(preason, entry.reason)?;
                Ok(g::AEE_SUCCESS)
            }
            "SetReason" => {
                let [reason] = self.args()?;
                let Some(entry) = self.history_entry(this)? else {
                    return Ok(g::AEE_EFAILED);
                };
                if self.app_in_group(entry.cls, 0) != g::AEE_SUCCESS {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                let id = entry.id;
                if let Some(entry) = self.history.entry_mut(id) {
                    entry.reason = reason;
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The cursor of one `IAppHistory` object, or `None` when this address never was one.
    pub(crate) fn history_cursor(&self, obj: u32) -> Option<u32> {
        self.history_objects.get(&obj).map(|o| o.current)
    }

    /// The entry a history object's cursor sits on, cloned so a caller can hold the class id and the
    /// key while it reaches back into the chain. `None` is the body's "no entry", which every one of
    /// these slots answers `AEE_EFAILED` for — except `Top`, whose refusal is an empty chain.
    pub(crate) fn history_entry(&self, obj: u32) -> Result<Option<HistoryEntry>, Error> {
        let Some(cur) = self.history_cursor(obj) else {
            return Ok(None);
        };
        Ok(self.history.entry(cur).cloned())
    }

    /// `IFIFO` — the named byte pipe, `AEECLSID_FIFO` (`0x01001046`), thirteen slots read out of
    /// the delivered body. Both halves of the reference object are in that one member: the `_IFIFO_*` proxy
    /// *and* the service it hands its call to (`unpackbridle_AEEFIFO_Read`/`_Write`, `AEEFIFO_Close`), so
    /// the ring, its counts and its wake conditions are measured, not guessed. The bridge hop between
    /// them moved data inside the box; here it does not need to be crossed.
    ///
    ///
    /// Three shapes a caller can see and a host can easily smooth over: an **empty** pipe answers `-2`
    /// to a read while some writer still holds it and `0` once nobody does; `IOCtl` with `0x202` only ever
    /// grows the ring (shrinking is `AEE_EBADPARM`); and the ring — its bytes included — lives until the
    /// last reader *and* the last writer have closed, because the counts are on the pipe, not on the
    /// handle.
    pub(crate) fn ififo(
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
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                // `AEE_CheckPtr(0, ppv, 4, 1)` is the body's first line, and what it returns is
                // `AEE_EMEMPTR`(29) — not the `AEE_EBADPARM` most out-pointer gates answer.
                if ppv == 0 {
                    return Ok(g::AEE_EMEMPTR);
                }
                // The list is four numbers and no others: `IQI`, the two interfaces this class inherits,
                // and its own class id. A hit AddRefs the same object; a miss writes the out-pointer and
                // answers `AEE_ECLASSNOTSUPPORT`.
                self.answer_qi(
                    this,
                    iid,
                    ppv,
                    &[g::AEEIID_ISource, g::AEEIID_IPort, g::AEECLSID_FIFO],
                )
                .map_err(Error::Hle)
            }
            // `int GetLastError(IPort *po)` is one line — `return *(this + 44)` — and that word is what
            // every other slot in this table writes when it refuses. A refused call's *return* is often
            // `-1`; the code is only in the error word, so `GetLastError` is the whole of the answer.
            "GetLastError" => {
                let Some(record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                Ok(record.last_error)
            }
            // `IFIFO_Open` is a URL. The documented behaviour gets an `IWebUtil` from the shell
            // (`QueryInterface(AEECLSID_WEBUTIL)`), calls its `ParseUrl`, and releases it again; this host
            // reaches the same parser directly, because making and dropping an object it already owns
            // would be a detour with no answer at the end of it. What is copied is the rule: the search
            // part must be exactly `?mode=` plus `rw`, `r` or `w`, and everything up to it is the name
            // handed to `OpenEx`.
            "Open" => {
                let [paddr] = self.args()?;
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                let text = self.cstring_limited(paddr, 1024)?;
                let answer = self.ififo_open_url(&mut record, text.as_bytes());
                self.fifo_objects.insert(this, record);
                Ok(answer)
            }
            "OpenEx" => {
                let [paddr, mode] = self.args()?;
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // The body's very first gate is `AEE_CheckPtr(0, pszPath, aee_strlen(pszPath), 0)`, so a
                // name the runtime cannot reach is `AEE_EMEMPTR` — before the empty-name, bad-mode and
                // already-open tests, and without touching the error word.
                if paddr == 0 {
                    self.fifo_objects.insert(this, record);
                    return Ok(g::AEE_EMEMPTR);
                }
                let text = self.cstring_limited(paddr, 1024)?;
                let answer = self.ififo_open(&mut record, &text, mode);
                self.fifo_objects.insert(this, record);
                Ok(answer)
            }
            // `Read` and `Write` share their three proxy gates and differ only in which mode bit the
            // pipe demands, so they share one arm.
            "Read" | "Write" => {
                let [buf, cb] = self.args()?;
                let write = name == "Write";
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // `AEE_CheckPtr(0, psz, cb, …)` — a null buffer is `AEE_EMEMPTR` in the error word and
                // `-1` in the return, and the caller is expected to ask `GetLastError` which of its
                // three refusals it got.
                if buf == 0 {
                    record.last_error = g::AEE_EMEMPTR;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                if (cb as i32) < 0 {
                    record.last_error = g::AEE_EBADPARM;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                let Some(pipe) = record.pipe.clone() else {
                    record.last_error = g::AEE_EBADSTATE;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                };
                if self.fifo_mode(&record, write).is_err() {
                    record.last_error = g::AEE_EPRIVLEVEL;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                let want = cb as usize;
                let answer = if write {
                    let bytes = self.cpu.mem.read_bytes(buf, want)?;
                    self.fifo_put(&pipe, &bytes)?
                } else {
                    let (bytes, was_full, was_empty) = self.fifo_take(&pipe, want)?;
                    if was_full {
                        self.fifo_wake(&pipe, false)?;
                    }
                    let n = bytes.len() as u32;
                    if n > 0 {
                        self.cpu.mem.write_bytes(buf, &bytes)?;
                    }
                    // The `-2`/`0` pair sits inside `if (v8[9] == 0)` — the byte count *before* the read,
                    // not the number that came out. So a read of zero bytes from a pipe that does hold
                    // bytes takes the copying branch and answers `0`, and only a genuinely empty pipe
                    // gets to say whether a writer is still holding it open.
                    if was_empty {
                        self.fifo_empty_answer(&pipe)
                    } else {
                        n
                    }
                };
                // The clone above was taken before any of that, and a wake in this same call empties
                // the live record's two waiter lists — so both lists come back from the live record
                // here, or writing this clone would put a writer it just gave another chance back on
                // the wait list.
                if let Some(live) = self.fifo_objects.get(&this) {
                    let (read, writeable) = (live.readable.clone(), live.writeable.clone());
                    record.readable = read;
                    record.writeable = writeable;
                }
                self.fifo_objects.insert(this, record);
                Ok(answer)
            }
            // `Flush` is the read op with no destination: the body packs the *same* call with the buffer
            // pointer at zero, and `unpackbridle_AEEFIFO_Read` skips its `memcpy` on that. So it discards
            // and counts, and it is the one data slot that never checks the pointer.
            "Flush" => {
                let [cb] = self.args()?;
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                if (cb as i32) < 0 {
                    record.last_error = g::AEE_EBADPARM;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                if record.pipe.is_none() {
                    record.last_error = g::AEE_EBADSTATE;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                let pipe = record.pipe.clone().unwrap();
                if self.fifo_mode(&record, false).is_err() {
                    record.last_error = g::AEE_EPRIVLEVEL;
                    self.fifo_objects.insert(this, record);
                    return Ok(u32::MAX);
                }
                let (bytes, full, was_empty) = self.fifo_take(&pipe, cb as usize)?;
                let n = bytes.len() as u32;
                if full {
                    self.fifo_wake(&pipe, false)?;
                }
                // `Flush` is the read op with `dst = 0`: the body still clamps to the pending count,
                // still advances the head, and still answers the number of bytes it threw away. The
                // `-2`/`0` pair is again the empty pipe's, not "nothing came out".
                let answer = if was_empty {
                    self.fifo_empty_answer(&pipe)
                } else {
                    n
                };
                self.fifo_objects.insert(this, record);
                Ok(answer)
            }
            // `Readable`/`Writeable` register a callback and then ask the pipe whether to hold it. The
            // two differ only in the condition and in which chain the node goes on, so they share an arm.
            "Readable" | "Writeable" => {
                let [pcb] = self.args()?;
                let read = name == "Readable";
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // `AEE_CheckPtr(0, pcb, 28, 1)`; then nothing happens at all if there is no handle or if
                // the caller's notify pointer is empty. Those three refusals leave `0` in `r0`, because
                // each one sets `result` from a value that is zero by definition — the failed check, the
                // null handle at `[+56]`, the null notify at `+16`.
                if pcb == 0 || record.pipe.is_none() {
                    self.fifo_objects.insert(this, record);
                    return Ok(0);
                }
                let notify = self.cpu.mem.read_u32(pcb + 16)?;
                if notify == 0 {
                    self.fifo_objects.insert(this, record);
                    return Ok(0);
                }
                // The body cancels whatever the node was already registered for, then head-inserts it —
                // and the value it hands back is the head that was there (`result = a1[12]; *a2 = result;
                // a1[12] = a2;`), so a second registration on the same object reports the first one's
                // address rather than zero. The slot is `void`, so a pack that reads `r0` is reading a
                // leftover; this host leaves the same leftover.
                let chain = if read {
                    record.readable.retain(|one| *one != pcb);
                    &mut record.readable
                } else {
                    record.writeable.retain(|one| *one != pcb);
                    &mut record.writeable
                };
                let previous = chain.first().copied().unwrap_or(0);
                chain.insert(0, pcb);
                let pipe_name = record.pipe.clone().unwrap();
                let hold = self.fifo_holds(&pipe_name, read);
                if hold {
                    // Park it: the wake comes from the other side's `Read`/`Write`, or from the last
                    // writer going away. The reference cancel pointer (`IFIFO_CancelReadable`) is a
                    // function of the runtime's own, which this host has no address space to hand a
                    // pack — so a `CALLBACK_Cancel` here cannot unlink the node.
                    self.fifo_park(&pipe_name, this, read);
                } else {
                    if read {
                        record.readable.retain(|one| *one != pcb);
                    } else {
                        record.writeable.retain(|one| *one != pcb);
                    }
                    let owner = self.applet;
                    if self.dispatcher_prepare(pcb)? {
                        self.dispatcher.resume_from(pcb, owner);
                    }
                    self.fifo_unpark(&pipe_name, this, read);
                }
                self.fifo_objects.insert(this, record);
                Ok(previous)
            }
            // `IOCtl` answers from the pipe, so its first refusal is the state, and — unlike the data
            // slots — that one is returned **without** touching the error word.
            "IOCtl" => {
                let [cmd, data] = self.args()?;
                let Some(record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                let Some(pipe) = record.pipe.clone() else {
                    return Ok(g::AEE_EBADSTATE);
                };
                if !(cmd == FIFO_IOCTL_GET_SIZE
                    || cmd == FIFO_IOCTL_GET_COUNT
                    || cmd == FIFO_IOCTL_SET_SIZE)
                {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                if cmd == FIFO_IOCTL_SET_SIZE {
                    // `0x202`: the size comes **by value**, the pipe only ever grows, and growing needs
                    // the write bit — a read-only handle gets `AEE_EPRIVLEVEL` here, not `EBADPARM`.
                    if self.fifo_mode(&record, true).is_err() {
                        return Ok(g::AEE_EPRIVLEVEL);
                    }
                    let size = self.fifos.get(&pipe).map(|one| one.data.len()).unwrap_or(0);
                    if (data as usize) <= size {
                        return Ok(g::AEE_EBADPARM);
                    }
                    if let Some(one) = self.fifos.get_mut(&pipe) {
                        one.data.resize(data as usize, 0);
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                let want_count = cmd == FIFO_IOCTL_GET_COUNT;
                let Some(one) = self.fifos.get(&pipe) else {
                    return Ok(g::AEE_EBADSTATE);
                };
                let answer = if want_count {
                    one.count as u32
                } else {
                    one.data.len() as u32
                };
                // The other two codes take a pointer and write through it; a null one is not checked,
                // because the body does not check it either.
                self.cpu.mem.write_u32(data, answer)?;
                Ok(g::AEE_SUCCESS)
            }
            // `Close` is the slot the destructor calls too, and its refusals are the plainest in the
            // table: no handle is `AEE_EBADSTATE` and everything else answers `0`.
            "Close" => {
                let Some(mut record) = self.fifo_objects.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                if record.pipe.is_none() {
                    self.fifo_objects.insert(this, record);
                    return Ok(g::AEE_EBADSTATE);
                }
                // `while (this[+48]) CALLBACK_Cancel` then the same for `[+52]`: the parked nodes are
                // taken off, not rung — closing does not tell a caller its data went away.
                record.readable.clear();
                record.writeable.clear();
                self.fifo_shutdown(this, &mut record)?;
                self.fifo_objects.insert(this, record);
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// `IFIFO_Open`, which is a URL: the body asks the shell for `AEECLSID_WEBUTIL`, calls its
    /// `ParseUrl` into a 40-byte `UrlParts`, gives the object back, and requires the search part to be
    /// `?mode=` plus exactly `rw`, `r` or `w`. Everything from the start of the text to the `?` is then
    /// handed to `IFIFO_OpenEx`, so the two refusals this slot can give are `EBADPARM` (no search, or a
    /// search that is not a mode) and whatever `OpenEx` says about the name.
    pub(crate) fn ififo_open_url(&mut self, record: &mut FifoObject, text: &[u8]) -> u32 {
        let parts = web::parse_url(text);
        let search = parts[7];
        let name = parts[8];
        if search == name
            || text.len() - search < 6
            || !text[search..search + 6].eq_ignore_ascii_case(b"?mode=")
        {
            return g::AEE_EBADPARM;
        }
        // `aee_stricmp` runs to the terminator, so a fragment after the mode makes the comparison fail
        // rather than trim to the fragment — hence the whole remainder, not `search + 6..name`.
        let value = &text[search + 6..];
        let mode = if value.eq_ignore_ascii_case(b"rw") {
            3
        } else if value.eq_ignore_ascii_case(b"r") {
            g::AEEFIFO_MODE_R
        } else if value.eq_ignore_ascii_case(b"w") {
            g::AEEFIFO_MODE_W
        } else {
            return g::AEE_EBADPARM;
        };
        let base = String::from_utf8_lossy(&text[..search]).into_owned();
        self.ififo_open(record, &base, mode)
    }

    /// `IFIFO_OpenEx`. The gates are the body's, in its order: a bad pointer is `AEE_EMEMPTR`, an empty
    /// name or a mode outside `1..=3` is `EBADPARM`, and opening a second time is `AEE_EBADSTATE` — the
    /// one refusal in this table that is not about the caller's arguments.
    pub(crate) fn ififo_open(&mut self, record: &mut FifoObject, text: &str, mode: u32) -> u32 {
        if text.is_empty() || mode == 0 || mode >> 2 != 0 {
            return g::AEE_EBADPARM;
        }
        if record.pipe.is_some() {
            return g::AEE_EBADSTATE;
        }
        // The name has to be `fifo:/…` and must not end in `/`; the rewrite (`fifo:/x` → `fs:/x`) is
        // what the resolved path is, and `AEEFile_IsResolvedPath` says a plain `fs:/x` already is one.
        let Some(rest) = text.strip_prefix("fifo:/") else {
            return g::AEE_EBADPARM;
        };
        if text.ends_with('/') {
            return g::AEE_EBADPARM;
        }
        let resolved = format!("fs:/{rest}");
        // The permission block: the group test, then the app's own module directory, then the owning
        // module's ACL, then a default ACL literal. Only the second of those is decidable here, and the
        // host keeps no ACL record to consult — see the note and the entry. The reference block can answer
        // "no bits at all", which would refuse every open with `AEE_EPRIVLEVEL`; this host grants both
        // directions and says so, because a refusal it can only reach through a record it does not have
        // is not a fact about FIFOs.
        if self.display.quiet.iter().any(|note| note == "IFIFO acl") {
            // already said
        } else {
            self.display.note(
                "IFIFO::OpenEx: the shipped permission block consults AEE_AppIsInGroup, the module's own \
                 directory and AEEACL_CheckPath over an ACL record this host does not keep, so it grants \
                 both directions and AEE_EPRIVLEVEL from that block is unreachable here"
            );
            self.display.quiet.push("IFIFO acl".to_string());
        }
        let pipe = self
            .fifos
            .entry(resolved.clone())
            .or_insert_with(|| FifoPipe {
                data: vec![0; FIFO_DEFAULT_SIZE],
                head: 0,
                count: 0,
                readers: 0,
                writers: 0,
                parked_read: Vec::new(),
                parked_write: Vec::new(),
            });
        // `if (mode & 1) ++kf[+8]; if (mode & 2) ++kf[+12];` — the counts are on the pipe, which is why
        // a second opener of the same name shares the bytes instead of starting a fresh ring.
        if mode & g::AEEFIFO_MODE_R != 0 {
            pipe.readers += 1;
        }
        if mode & g::AEEFIFO_MODE_W != 0 {
            pipe.writers += 1;
        }
        record.pipe = Some(resolved);
        record.mode = mode;
        record.last_error = 0;
        g::AEE_SUCCESS
    }

    /// Which mode bit the reference service side tests for the direction asked: read wants `[+0] & 1`,
    /// write wants `[+0] & 2`, and a handle opened the other way gets `AEE_EPRIVLEVEL` and `-1`.
    pub(crate) fn fifo_mode(&self, record: &FifoObject, write: bool) -> Result<(), ()> {
        let bit = if write {
            g::AEEFIFO_MODE_W
        } else {
            g::AEEFIFO_MODE_R
        };
        if record.mode & bit == 0 {
            return Err(());
        }
        Ok(())
    }

    /// `unpackbridle_AEEFIFO_Read`, copy included: the bytes taken, whether the pipe was exactly full
    /// beforehand — the body's own test for handing the waiting writers another chance — and whether it
    /// was empty beforehand, which is the only case in which its `-2`/`0` pair is the answer at all.
    pub(crate) fn fifo_take(
        &mut self,
        pipe: &str,
        want: usize,
    ) -> Result<(Vec<u8>, bool, bool), Error> {
        let Some(one) = self.fifos.get_mut(pipe) else {
            return Ok((Vec::new(), false, true));
        };
        if one.count == 0 {
            return Ok((Vec::new(), false, true));
        }
        let n = want.min(one.count);
        let size = one.data.len();
        let was_full = one.count == size;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(one.data[(one.head + i) % size]);
        }
        one.head = (one.head + n) % size;
        one.count -= n;
        Ok((out, was_full, false))
    }

    /// `unpackbridle_AEEFIFO_Write`: fill at the far end of what is pending, wrap once, and say whether
    /// the pipe has just gone from empty to non-empty, which is when the waiting readers are rung.
    pub(crate) fn fifo_put(&mut self, pipe: &str, bytes: &[u8]) -> Result<u32, Error> {
        let Some(one) = self.fifos.get_mut(pipe) else {
            return Ok(u32::MAX);
        };
        let size = one.data.len();
        if size == one.count {
            // Full: `-2` is the body's answer, and it is not an error code — no `AEE_E*` name covers it,
            // which is why the caller has to read the count rather than `GetLastError`.
            return Ok(0u32.wrapping_sub(2));
        }
        let n = (size - one.count).min(bytes.len());
        let start = (one.head + one.count) % size;
        let was_empty = one.count == 0;
        for i in 0..n {
            one.data[(start + i) % size] = bytes[i];
        }
        one.count += n;
        if was_empty {
            self.fifo_wake(pipe, true)?;
        }
        Ok(n as u32)
    }

    /// `-2` while a writer still holds the pipe, `0` once none does — the empty-read pair.
    pub(crate) fn fifo_empty_answer(&self, pipe: &str) -> u32 {
        match self.fifos.get(pipe) {
            Some(one) if one.writers != 0 => 0u32.wrapping_sub(2),
            _ => 0,
        }
    }

    /// Whether the callback should be held: a reader waits only while the pipe is empty **and** somebody
    /// can still write it; a writer waits only while it is full. Anything else is "already satisfied",
    /// and the body resumes it on the spot.
    pub(crate) fn fifo_holds(&self, pipe: &str, read: bool) -> bool {
        let Some(one) = self.fifos.get(pipe) else {
            return false;
        };
        if read {
            one.count == 0 && one.writers != 0
        } else {
            one.data.len() <= one.count
        }
    }

    pub(crate) fn fifo_park(&mut self, pipe: &str, object: u32, read: bool) {
        let Some(one) = self.fifos.get_mut(pipe) else {
            return;
        };
        let list = if read {
            &mut one.parked_read
        } else {
            &mut one.parked_write
        };
        if !list.contains(&object) {
            list.push(object);
        }
    }

    pub(crate) fn fifo_unpark(&mut self, pipe: &str, object: u32, read: bool) {
        let Some(one) = self.fifos.get_mut(pipe) else {
            return;
        };
        let list = if read {
            &mut one.parked_read
        } else {
            &mut one.parked_write
        };
        list.retain(|held| *held != object);
    }

    /// The wake. `IFIFO_ReadCallback` walks the parked object's own chain and resumes every node in it,
    /// so the object's whole list drains — and the parked entry goes with it.
    pub(crate) fn fifo_wake(&mut self, pipe: &str, read: bool) -> Result<(), Error> {
        let parked = match self.fifos.get_mut(pipe) {
            Some(one) => {
                if read {
                    std::mem::take(&mut one.parked_read)
                } else {
                    std::mem::take(&mut one.parked_write)
                }
            }
            None => return Ok(()),
        };
        for object in parked {
            let chain = match self.fifo_objects.get_mut(&object) {
                Some(record) => {
                    if read {
                        std::mem::take(&mut record.readable)
                    } else {
                        std::mem::take(&mut record.writeable)
                    }
                }
                None => Vec::new(),
            };
            for node in chain {
                let owner = self.applet;
                if self.dispatcher_prepare(node)? {
                    self.dispatcher.resume_from(node, owner);
                }
            }
        }
        Ok(())
    }

    /// `AEEFIFO_Close`'s service half: give back the reader or writer this handle bought, and free the
    /// pipe only when **both** counts have gone. When the last writer leaves, the readers still parked are
    /// woken — they have nothing left to wait for, and their next read answers `0` instead of `-2`.
    /// The parked nodes are dropped rather than resumed, which is the proxy's own `while (this[+48])
    /// CALLBACK_Cancel` done on this side of the call.
    pub(crate) fn fifo_shutdown(
        &mut self,
        object: u32,
        record: &mut FifoObject,
    ) -> Result<(), Error> {
        let Some(pipe) = record.pipe.clone() else {
            return Ok(());
        };
        let mut gone = false;
        let mut wake_read = false;
        if let Some(one) = self.fifos.get_mut(&pipe) {
            if record.mode & g::AEEFIFO_MODE_R != 0 {
                one.readers = one.readers.saturating_sub(1);
            }
            if record.mode & g::AEEFIFO_MODE_W != 0 {
                one.writers = one.writers.saturating_sub(1);
            }
            one.parked_read.retain(|held| *held != object);
            one.parked_write.retain(|held| *held != object);
            wake_read = one.writers == 0;
            gone = one.readers == 0 && one.writers == 0;
        }
        record.pipe = None;
        record.mode = 0;
        if wake_read {
            if let Some(one) = self.fifos.get_mut(&pipe) {
                let held = std::mem::take(&mut one.parked_read);
                for held_object in held {
                    if let Some(victim) = self.fifo_objects.get_mut(&held_object) {
                        for node in std::mem::take(&mut victim.readable) {
                            let owner = self.applet;
                            if self.dispatcher_prepare(node)? {
                                self.dispatcher.resume_from(node, owner);
                            }
                        }
                    }
                }
            }
        }
        if gone {
            self.fifos.remove(&pipe);
        }
        Ok(())
    }

    /// `realloc`, which is `AEEkHeap_Realloc(pHeap, p, len)`: in place when the node can take the
    /// size, otherwise a block elsewhere with the payload carried over and the old node handed back.
    /// `Hle::realloc` keeps the size map level, and both the `realloc` helper and `IHeap::Realloc`
    /// come through here so the two cannot drift apart.
    ///
    /// A block this host never handed out is treated as a fresh allocation with nothing to carry —
    /// the delivery would break on it, and a pack that passes a pointer into its own statics has to
    /// keep running.
    pub(crate) fn heap_realloc(&mut self, block: u32, request: u32) -> HResult<u32> {
        let (size, no_zmem) = alloc_request(request);
        if block == 0 {
            return self.heap_alloc(request);
        }
        if size == 0 {
            self.heap_free(block);
            return Ok(0);
        }
        match self.hle.realloc(&mut self.cpu.mem, block, size, !no_zmem) {
            Some(dst) => {
                // A move leaves the old address back in the arena, and a clip named there must not stay
                // behind to be read as the new occupant's bytes. Growing in place keeps the record. The
                // old extent is not asked for: the reclaim above already dropped it, and the only
                // interior key this host files is a stream's, whose block a caller cannot `realloc`.
                if dst != block {
                    self.withdraw_audio(block, 0);
                }
                Ok(dst)
            }
            // Out of arena: the old block is still the caller's, which is what `realloc` promises.
            None => Ok(0),
        }
    }

    /// `FREE(p)`: the block goes back to the arena, and any clip this host was reading out of it goes with
    /// it.
    ///
    /// A record is keyed by the address the caller named, and this heap reuses an address for the next
    /// request that fits — so a record left behind outlives the bytes it describes. Its payload stays
    /// charged against the audio budget (and past the budget's ceiling a later clip is left with no
    /// payload at all, which costs that pack its `START`, its `TICK_UPDATE` and its `GetTotalTime` answer
    /// while the host says nothing), and its name is the same `0x…` a *new* clip at that address would be
    /// given, so two records answer to one name.
    ///
    /// Both namings are withdrawn, because both can carry the caller's own pointer: a `SDT_BUFFER` naming
    /// always does, and `IMedia::SetMediaParm(MMD_FILE_NAME, …)` stores the caller's string verbatim — so
    /// a file record left behind is a pointer this host goes on reading as a name, which can resolve to
    /// whatever new occupant lives there. The one spelling where the pointer is the host's own copy is
    /// `ISoundPlayer`'s file naming, and the host releases that copy through this same funnel when it lets
    /// go of the naming, so the record goes exactly when the name does. A naming can also point *into* a
    /// block (a media stream files `buf + pos`), so the whole extent is asked for before the reclaim.
    pub(crate) fn heap_free(&mut self, block: u32) {
        // The span is read before the block goes back: a record can name a *place inside* the block
        // rather than its first byte, and after the reclaim there is nothing left to ask.
        let span = self.hle.block_size(block).unwrap_or(0) as u32;
        self.hle.dealloc(&mut self.cpu.mem, block);
        self.withdraw_audio(block, span);
    }

    /// `AEEHelperFuncs::malloc`'s answer for an `IHeap` request: a block, or null when the arena is out.
    /// Out of memory is a case a module has to cope with rather than a fault, which is the reading the
    /// `malloc` helper already takes.
    ///
    /// The request also carries a flag. The header documents the size parameter of both `Malloc` and
    /// `Realloc` as "optionally ORed with `ALLOC_NO_ZMEM`", and the header's own `MALLOCREC_EX`
    /// masks the same bit with `& ~ALLOC_NO_ZMEM` to get a length out of it — so bit 31 says *where*
    /// to allocate, not *how much*. Reading the whole word as a byte count is what turned one pack's
    /// 60-byte request into 2 147 483 708 and killed its applet constructor on the refusal.
    ///
    /// The flag is honoured, and that is the delivery's own rule rather than this host's: the
    /// top bit is `AEEkHeap_Realloc`'s "do not clear it" ("the memory returned is
    /// zero-initialized, unless the high bit of uSize is set to 1"). Every other allocation is
    /// zeroed, as a handset's is.
    pub(crate) fn heap_alloc(&mut self, request: u32) -> HResult<u32> {
        let (size, no_zmem) = alloc_request(request);
        match self.hle.alloc_with(&mut self.cpu.mem, size, !no_zmem) {
            Ok(addr) => Ok(addr),
            Err(Fault::HeapExhausted { .. }) => Ok(0),
            Err(other) => Err(HleError::Mem(other)),
        }
    }
}

/// Which wrapper the first bytes name, and how many of them belong to the wrapper rather than to
/// the deflate stream inside it.
///
/// `None` means the bytes on hand are not yet enough to tell — a gzip header can be longer than the
/// first window a caller hands over — so the pump pulls another window and asks again.
fn wrapping(head: &[u8]) -> Option<(Wrap, usize)> {
    if head.len() >= 2 && head[0] == 0x1f && head[1] == 0x8b {
        return gzip_head_len(head).map(|n| (Wrap::Gzip, n));
    }
    if head.len() < 2 {
        return None;
    }
    // A zlib header: CMF's low nibble is the compression method (8 = deflate), and the two bytes
    // together are a multiple of 31.
    if head[0] & 0x0f == 8 && (u16::from(head[0]) << 8 | u16::from(head[1])) % 31 == 0 {
        return Some((Wrap::Zlib, 0));
    }
    Some((Wrap::Raw, 0))
}

/// The length of an RFC 1952 gzip header, or `None` while the bytes on hand do not yet cover it.
///
/// Parsed here rather than handed to a `GzDecoder` because the deflate that follows goes to a raw
/// inflater: a decoder that owns the header cannot be stopped mid-stream and handed a second window.
fn gzip_head_len(b: &[u8]) -> Option<usize> {
    const FEXTRA: u8 = 0x04;
    const FNAME: u8 = 0x08;
    const FCOMMENT: u8 = 0x10;
    const FHCRC: u8 = 0x02;
    if b.len() < 10 || b[2] != 8 {
        return None;
    }
    let flags = b[3];
    let mut at = 10usize;
    if flags & FEXTRA != 0 {
        if b.len() < at + 2 {
            return None;
        }
        let xlen = usize::from(u16::from_le_bytes([b[at], b[at + 1]]));
        at += 2 + xlen;
    }
    for bit in [FNAME, FCOMMENT] {
        if flags & bit != 0 {
            while at < b.len() && b[at] != 0 {
                at += 1;
            }
            if at >= b.len() {
                return None;
            }
            at += 1;
        }
    }
    if flags & FHCRC != 0 {
        at += 2;
    }
    (b.len() >= at).then_some(at)
}
