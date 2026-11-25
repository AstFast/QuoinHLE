//! Audio and media: `IMedia`/`IMediaUtil`, the sound players, the tone
//! generator, the ringer and the position detector.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// The `IMedia` slots, for the media classes this host serves: `AEECLSID_MEDIAMIDI`,
    /// `AEECLSID_MEDIAMP3`, `AEECLSID_MEDIAQCP`, `AEECLSID_MEDIAAAC`, `AEECLSID_MEDIAMMF` and the
    /// The carrier SPF face `AEECLSID_MEDIAPHR`. The three Yamaha faces (`MIDI`, `MMF`, `SPF`) differ only
    /// in a format flag, exactly as `IMediaMA3` registers them (`audio/mid`, `audio/mmf`,
    /// `audio/spf`).
    ///
    /// A media controller and a sound player are the same object in two shapes. The caller names bytes
    /// — `SetMediaParm(MM_PARM_MEDIA_DATA, AEEMediaData *, 0)`, and that struct is a class id, a
    /// pointer and a length, which is an `SDT_BUFFER` under another name — arms a notify, then says
    /// `Play`. So this answers through the [`Player`] record an `ISoundPlayer` already uses, and the
    /// front end's voice-per-object polling picks the clip up with no new machinery behind it.
    ///
    /// Time is the pump's, not this object's: a `Play` that reaches a clip this host can time opens
    /// a window (`tick_at`/`pass_at`) and the dispatch loop turns those deadlines into the
    /// `MM_STATUS_*` jobs the `RegisterNotify` pair asks for. `GetTotalTime` and `MM_PARM_POS` work
    /// against that same clip, which is why a position this host cannot measure is refused by name
    /// rather than answered with a plausible number. Recording is a different device
    /// entirely and answers `AEE_EUNSUPPORTED` rather than pretending to a microphone. A parameter this
    /// record has no room for is accepted and *noted*, because a silent yes is the one answer a later
    /// reader cannot tell from a real one.
    /// `MM_PARM_MEDIA_DATA`: `AEEMediaData { clsData, pData, dwSize }`, whose first word says what the
    /// other two *are* — `MMD_FILE_NAME` names a file, `MMD_BUFFER` holds the whole score, and any
    /// `ISource`-based class id means `pData` is that interface, whose bytes the media object is
    /// expected to drain. Sets the object's naming, clears the pass the old one armed (a new clip has
    /// no position yet), and answers whether a naming was set at all.
    pub(crate) fn media_set_data(&mut self, obj: u32, pmd: u32, label: &str) -> HResult<bool> {
        let at = |field: &str| {
            abi::offset_of("AEEMediaData", field).expect("AEEMediaData is in the tables")
        };
        let cls = self.cpu.mem.read_u32(pmd + at("clsData"))?;
        let data = self.cpu.mem.read_u32(pmd + at("pData"))?;
        let size = self.cpu.mem.read_u32(pmd + at("dwSize"))?;
        let mut player = self.players.get(&obj).copied().unwrap_or_default();
        if cls == g::MMD_BUFFER {
            player.input = g::SDT_BUFFER;
            player.data = data;
            player.size = size;
        } else if cls == g::MMD_FILE_NAME {
            player.input = g::SDT_FILE;
            player.data = data;
            player.size = 0;
        } else if let Some(stream) = self.memstreams.get(&data).copied() {
            // A stream is a buffer with a reader's place in it, so what the media object is being
            // handed is the rest of the chunk — and naming it as an `SDT_BUFFER` is what lets the
            // same clip be read, dumped and played by everything downstream.
            player.input = g::SDT_BUFFER;
            player.data = stream.buf + stream.pos;
            player.size = stream.size - stream.pos.min(stream.size);
        } else {
            // The naming resolved to nothing, so the player has named nothing either — leaving
            // whatever it held before would be the one answer that reads as a clip.
            player.input = 0;
            player.data = 0;
            player.size = 0;
            self.display.note(format!(
                "{label}: media data of class {cls:#010x} at {data:#010x} is neither a buffer, a \
                 file name, nor a stream this host handed out"
            ));
        }
        let named = player.input != 0;
        if named {
            self.note_audio_source(&player);
        }
        player.tick_at = 0;
        player.pass_at = 0;
        player.position = 0;
        player.play_start = 0;
        player.play_stop = None;
        player.rate_num = 0;
        player.rate_den = 0;
        self.players.insert(obj, player);
        Ok(named)
    }

    /// `AEEMediaUtil_FindClass`: the class id one `AEEMediaData` resolves to, or the error the body
    /// answers instead (`MM_ENOTENOUGHTDATA`, `MM_ENOMEDIAMEMORY`, `MM_EBADMEDIADATA`). The order is
    /// the reference one: the `DetectType` probe, the file extension, then the bytes.
    pub(crate) fn media_util_find_class(&mut self, pmd: u32) -> HResult<u32> {
        let at = |field: &str| {
            abi::offset_of("AEEMediaData", field).expect("AEEMediaData is in the tables")
        };
        let cls_data = self.cpu.mem.read_u32(pmd + at("clsData"))?;
        let data = self.cpu.mem.read_u32(pmd + at("pData"))?;
        let size = self.cpu.mem.read_u32(pmd + at("dwSize"))?;
        let need = TYPE_DETECT_WINDOW;
        if cls_data == g::MMD_FILE_NAME {
            let Ok(path) = self.cstring(data) else {
                return Ok(MM_EBADMEDIADATA);
            };
            if let Some((_, ext)) = path.rsplit_once('.') {
                if !ext.is_empty() {
                    for base in ["audio/", "video/"] {
                        if let Some(cls) =
                            self.reg_lookup(g::AEECLSID_MEDIA, &format!("{base}{ext}"))
                        {
                            return Ok(cls);
                        }
                    }
                }
            }
            // Not named by its extension: read the bytes and sniff them. The body reads the probe's
            // window; this host's sniffers parse, so they get the whole file (bounded).
            let Ok(opened) = self.volume.open(&path, g::_OFM_READ) else {
                return Ok(MM_ENOMEDIAMEMORY);
            };
            let bytes = self.volume.read(&opened, MEDIA_UTIL_READ_CAP);
            if (bytes.len() as u32) < need {
                return Ok(MM_ENOTENOUGHTDATA);
            }
            return Ok(self.media_util_class_of(&bytes));
        }
        if cls_data == g::MMD_BUFFER {
            if size < need {
                return Ok(MM_ENOTENOUGHTDATA);
            }
            let want = size.min(MEDIA_UTIL_READ_CAP);
            let room = self.cpu.mem.mapped_run(data, want) as usize;
            let bytes = self.cpu.mem.read_bytes(data, room.min(want as usize))?;
            return Ok(self.media_util_class_of(&bytes));
        }
        if cls_data >= g::AEECLSID_SOURCE {
            // The body asks the source for `AEECLSID_PEEK`; this host hands no `IPeek` out, so every
            // source lands on the body's own `MM_EBADMEDIADATA`.
            return Ok(MM_EBADMEDIADATA);
        }
        // Any other class id: the body treats `pData` as a buffer and reads the window regardless of
        // `dwSize`. Read what is mapped, so a tight buffer is sniffed rather than faulted on.
        let room = self.cpu.mem.mapped_run(data, MEDIA_UTIL_READ_CAP) as usize;
        let bytes = self
            .cpu
            .mem
            .read_bytes(data, room.min(MEDIA_UTIL_READ_CAP as usize))?;
        Ok(self.media_util_class_of(&bytes))
    }

    /// `DetectType(buf, &size, 0, &mime)` followed by `GetHandler(AEECLSID_MEDIA, mime)`, or 0.
    pub(crate) fn media_util_class_of(&mut self, bytes: &[u8]) -> u32 {
        match crate::resource::sniff(bytes).or_else(|| crate::audio::sniff(bytes)) {
            Some(mime) => self.reg_lookup(g::AEECLSID_MEDIA, mime).unwrap_or(0),
            None => 0,
        }
    }

    /// `IMediaUtil` — `AEEMediaUtil_CreateMedia` and its `FindClass`, plus the two slots this box has
    /// no body for. The reference class is a singleton (`gsMediaUtil`, `AddRef`→1, `Release`→0); this
    /// host models it as an ordinary service object.
    pub(crate) fn imediautil(
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
                if ppv == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(self.answer_qi(this, iid, ppv, &[g::AEECLSID_MEDIAUTIL])?)
            }
            "CreateMedia" => {
                let [pmd, ppm] = self.args()?;
                let at = |field: &str| {
                    abi::offset_of("AEEMediaData", field).expect("AEEMediaData is in the tables")
                };
                if pmd == 0 || ppm == 0 || self.cpu.mem.read_u32(pmd + at("pData"))? == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // `*ppm = NULL` before anything else, which is the family's own rule.
                self.cpu.mem.write_u32(ppm, 0)?;
                let found = self.media_util_find_class(pmd)?;
                if found >= MM_EBADMEDIADATA && found < 0x0100_0000 {
                    return Ok(found);
                }
                let obj = if found == 0 {
                    0
                } else {
                    self.open_media_object(found).unwrap_or(0)
                };
                if obj == 0 {
                    // `CreateInstance` failing — including the class 0 that `FindClass` leaves when
                    // no handler answered — is `MM_ENOMEDIAMEMORY` in the body.
                    return Ok(MM_ENOMEDIAMEMORY);
                }
                match self.media_set_data(obj, pmd, label) {
                    Ok(true) => {
                        self.cpu.mem.write_u32(ppm, obj)?;
                        Ok(0)
                    }
                    Ok(false) => {
                        self.object_ref(obj, false)?;
                        Ok(MM_EBADMEDIADATA)
                    }
                    Err(e) => {
                        self.object_ref(obj, false)?;
                        Err(e.into())
                    }
                }
            }
            "EncodeMedia" => {
                let [per, cls_dest, pei, _pcb] = self.args()?;
                let at = |field: &str| {
                    abi::offset_of("AEEMediaEncodeInfo", field)
                        .expect("AEEMediaEncodeInfo is in the tables")
                };
                let bad = per == 0
                    || cls_dest == 0
                    || pei == 0
                    || self.cpu.mem.read_u32(pei + at("pmdDest"))? == 0
                    || self.cpu.mem.read_u16(pei + at("nCount"))? == 0
                    || self.cpu.mem.read_u32(pei + at("pmdList"))? == 0;
                if bad {
                    return Ok(g::AEE_EBADPARM);
                }
                self.display.note(format!(
                    "{label}: EncodeMedia has only the PMD encoder in the delivery, and that needs \
                     the CMX codec this machine does not have"
                ));
                Ok(g::AEE_EUNSUPPORTED)
            }
            "CreateMediaEx" => {
                self.display.note(format!(
                    "{label}: CreateMediaEx has no body in this box — the delivered utility has only \
                     CreateMedia and FindClass"
                ));
                Ok(g::AEE_EUNSUPPORTED)
            }
            _ => Err(unimplemented().into()),
        }
    }

    /// `AEEPosDet_New`: a 320-byte block, the class id kept at
    /// `+40`, a reference on the shell (`(**a1)(a1)`) held at `+32` and given back by `IPosDet_Delete`.
    /// The body opens with a class question — an id outside `clsSupported` is refused
    /// `AEE_EUNSUPPORTED` with the slot cleared — which this host asks one step earlier, in the shell's
    /// own table of classes it makes, so only the four ids reach here.
    ///
    /// ⚠ The body's last three acts are device calls, and this host has no bridle GPS to make them:
    /// `bridle_SystemCall(0x600000 | swi)` fills the object's default configuration, `OEM_GetConfig(65)`
    /// asks whether position determination is switched on at all, and only when it is, the two
    /// server-config reads (`32`, `33`) decide whether to open an autonomies session with
    /// `Appx5_CtorZ`. So the configuration store starts empty and no session is opened — which is the
    /// state a handset with the receiver switched off is in anyway, and the one place this constructor
    /// is thinner than the body.
    pub(crate) fn posdet_born(&mut self, clsid: u32, ppv: u32) -> Result<u32, Error> {
        if ppv == 0 {
            // `AEE_CreateInstanceEx` answers `AEE_EBADPARM` for a null slot before its class table is
            // consulted at all; the reference constructor, which the shell calls only after that, would
            // have written through address zero.
            return Ok(g::AEE_EBADPARM);
        }
        let addr = self
            .hle
            .new_object(&mut self.cpu.mem, "IPosDet", clsid, 8, 4, &[])?;
        let shell = self.shell;
        self.hold(shell, 1)?;
        // The caller owns the one reference the new object carries — `AEEPosDet_New` leaves
        // `m_nRefs` (`+36`) at zero and the shell hands the object out, so the pack's first `Release`
        // is what reaches zero and runs `IPosDet_Delete`.
        Ok(self.give_object(addr, ppv)?)
    }

    pub(crate) fn posdet_died(&mut self, iface: &str) -> Result<(), Error> {
        if iface != "IPosDet" {
            return Ok(());
        }
        let shell = self.shell;
        self.hold(shell, -1)?;
        Ok(())
    }

    /// `+40` — the class id this object was made as, which is the version number every gate in this
    /// family reads. The object layer carries it, because `AEEPosDet_New` writes the word once and
    /// nothing in the body ever changes it.
    pub(crate) fn posdet_cls(&self, this: u32) -> u32 {
        self.hle
            .object(this)
            .map(|object| object.clsid)
            .unwrap_or_default()
    }

    /// One entry of the delivered uncertainty table, by the index the body uses.
    ///
    /// The body multiplies a `uint8` by eight and adds it to the table, so an index past entry 31
    /// reads the debug strings that follow it in the guest image and calls one of those a distance
    /// in metres. There is no record for what such a number *should* be, so this host says so out
    /// loud and leaves the caller's own bytes alone instead of inventing a reading.
    pub(crate) fn posdet_unc(&mut self, index: u32, what: &str) -> Option<f64> {
        if let Some(value) = POSDET_UNC.get(index as usize) {
            return Some(*value);
        }
        self.display.note(format!(
            "IPosDet::ExtractPositionInfo: {what} names uncertainty index {index}, past the 32 \
             entries of the shipped table; this host leaves the field alone rather than read off \
             the end of it"
        ));
        None
    }

    pub(crate) fn posdet_extract(&mut self, gps: u32, out: u32) -> Result<u32, Error> {
        if gps == 0 || out == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        let position = |name: &str| {
            abi::offset_of("AEEPositionInfoEx_ALIGN_32", name)
                .expect("AEEPositionInfoEx_ALIGN_32 is in the ABI tables")
        };
        let field = |name: &str| {
            abi::offset_of("AEEGPSInfo", name).expect("AEEGPSInfo is in the ABI tables")
        };
        // ⚠ The align-4 spelling is the one the platform code uses: its `Heading` sits at `+28`, an
        // unaligned double written as two words, and `+32` is where its `HorVelocity` begins. The
        // table's `AEEPositionInfoEx` row is the align-8 layout of a *different* build.
        if self.cpu.mem.read_u32(out)? != position("AltitudeUnc") + 8 {
            return Ok(g::AEE_EBADPARM);
        }
        let valid = self.cpu.mem.read_u16(gps + field("fValid"))?;
        let mut bits = self.cpu.mem.read_u32(out + 4)?;
        let set = |bits: &mut u32, bit: u32, on: bool| {
            if on {
                *bits |= bit;
            } else {
                *bits &= !bit;
            }
        };

        if valid & g::AEEGPS_VALID_LAT != 0 {
            let raw = self.cpu.mem.read_u32(gps + field("dwLat"))? as i32;
            let degrees = f64::from(raw) / POSDET_DEGREES_PER_UNIT;
            posdet_write_double(&mut self.cpu.mem, out + position("Latitude"), degrees)?;
            bits |= 1;
        } else {
            // `*(&po->fLatitude) = 2 * (*(&po->fLatitude) >> 1)` — the body's own way of clearing bit
            // zero of a word it is not otherwise allowed to touch.
            bits = 2 * (bits as i32 >> 1) as u32;
        }
        if valid & g::AEEGPS_VALID_LON != 0 {
            let raw = self.cpu.mem.read_u32(gps + field("dwLon"))? as i32;
            let degrees = f64::from(raw) / POSDET_DEGREES_PER_UNIT;
            posdet_write_double(&mut self.cpu.mem, out + position("Longitude"), degrees)?;
            bits |= 2;
        } else {
            bits &= !2;
        }
        if valid & g::AEEGPS_VALID_ALT != 0 {
            let raw = i16::from_le_bytes(
                (self.cpu.mem.read_u16(gps + field("wAltitude"))? as u16).to_le_bytes(),
            );
            let metres = i32::from(raw) - 500;
            self.cpu
                .mem
                .write_u32(out + position("nAltitude"), metres as u32)?;
            bits |= 4;
        } else {
            bits &= !4;
        }
        if valid & g::AEEGPS_VALID_HEAD != 0 {
            let raw = self.cpu.mem.read_u16(gps + field("wHeading"))?;
            let degrees = f64::from((360 * raw) >> 10);
            posdet_write_double(&mut self.cpu.mem, out + position("Heading"), degrees)?;
            bits |= 8;
        } else {
            bits &= !8;
        }
        if valid & g::AEEGPS_VALID_HVEL != 0 {
            let raw = self.cpu.mem.read_u16(gps + field("wVelocityHor"))?;
            let metres_per_second = f64::from(25 * raw / 100);
            posdet_write_double(
                &mut self.cpu.mem,
                out + position("HorVelocity"),
                metres_per_second,
            )?;
            bits |= 0x10;
        } else {
            bits &= !0x10;
        }
        if valid & g::AEEGPS_VALID_VVEL != 0 {
            let raw = self.cpu.mem.read_u8(gps + field("bVelocityVer"))? as i8;
            let metres_per_second = f64::from(i32::from(raw) * 50 / 100);
            posdet_write_double(
                &mut self.cpu.mem,
                out + position("VerVelocity"),
                metres_per_second,
            )?;
            bits |= 0x20;
        } else {
            bits &= !0x20;
        }
        if valid & g::AEEGPS_VALID_HUNC != 0 {
            let angle = f64::from(
                5625 * u32::from(self.cpu.mem.read_u8(gps + field("bHorUncAngle"))?) / 1000,
            );
            posdet_write_double(&mut self.cpu.mem, out + position("HorUncAngle"), angle)?;
            let along = self.posdet_unc(
                u32::from(self.cpu.mem.read_u8(gps + field("bHorUnc"))?),
                "bHorUnc",
            );
            let across = self.posdet_unc(
                u32::from(self.cpu.mem.read_u8(gps + field("bHorUncPerp"))?),
                "bHorUncPerp",
            );
            if let Some(value) = along {
                posdet_write_double(&mut self.cpu.mem, out + position("HorUnc"), value)?;
            }
            if let Some(value) = across {
                posdet_write_double(&mut self.cpu.mem, out + position("HorUncPerp"), value)?;
            }
            bits |= 0x40;
        } else {
            bits &= !0x40;
        }
        set(&mut bits, 0x80, false);
        if valid & g::AEEGPS_VALID_VUNC != 0 {
            let index = self.cpu.mem.read_u16(gps + field("wVerUnc"))?;
            if index < 32 {
                // This one *is* range-checked by the body, because index 31 of the table is the
                // `-1.0` "no uncertainty available" sentinel; the flag follows the value being a
                // distance at all.
                let value = POSDET_UNC[index as usize];
                posdet_write_double(&mut self.cpu.mem, out + position("AltitudeUnc"), value)?;
                set(&mut bits, 0x80, value > 0.0);
            }
        }
        self.cpu.mem.write_u32(out + 4, bits)?;
        Ok(g::AEE_SUCCESS)
    }

    /// The one thing every request-shaped slot of this family ends with: `bridle_SystemCall` with an
    /// `OEMPosDet` software-call number folded in (`0x600000` for the configuration, `0x500000` for a
    /// GPS fix, `0x700000` for the serving sector, `0x800000` for an orientation). This host delivers
    /// no such device, so the answer is the device's own failure code — the reference slots pass that
    /// number straight out, or ignore it — with one sentence in the report naming what is missing.
    pub(crate) fn posdet_device(&mut self, what: &str) -> u32 {
        self.display.note(format!(
            "IPosDet::{what} reached the bridle GPS call; this host has no OEMPosDet device to carry \
             the request"
        ));
        g::AEE_EFAILED
    }

    /// The `IPosDet` slots, from the delivered body — the one member of this family the box holds as
    /// machine code, and it is the whole interface: ten functions, four of them the object header.
    ///
    /// What makes the family worth landing even though this host has no receiver is where its answers
    /// sit. Five of the seven slots open with `ISHELL_CheckPrivLevel` on one of the legacy `PL_*` bits
    /// (the header — `PL_SECTORINFO` for the two sector doors, `PL_POS_LOCATION` for the
    /// three GPS ones), and **this host cannot take that branch**: [`Runtime::app_in_group`] compares
    /// the applet with itself and never reads the group it was asked about, so `CheckPrivLevel`
    /// answers "member" for every id — an earlier decision, made because the declaration that would
    /// say otherwise (the module's MIF privilege word) is not something this box can read. Those
    /// questions are therefore asked here in the comments, not in code, because a branch no run can
    /// reach would be a claim about a gate that never closes. On a handset an unprivileged applet
    /// hears `AEE_EPRIVLEVEL` from `GetSectorInfoEx` and `AEE_SUCCESS` **with an untouched structure**
    /// from `GetSectorInfo`, which is the asymmetry worth knowing about this family.
    ///
    /// What stays in code is what this host *can* answer differently: the version gates on
    /// `+40`, the parameter gates, `pfnNotify`, and the whole of `ExtractPositionInfo`, which asks
    /// nothing of the platform at all.
    pub(crate) fn iposdet(
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
                if ppv == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if POSDET_CLASSES.contains(&iid) {
                    self.hold(this, 1)?;
                    self.cpu.mem.write_u32(ppv, this)?;
                    return Ok(g::AEE_SUCCESS);
                }
                self.cpu.mem.write_u32(ppv, 0)?;
                Ok(g::AEE_ECLASSNOTSUPPORT)
            }
            "GetSectorInfo" => {
                let [pi] = self.args()?;
                match self.posdet_cls(this) {
                    g::AEECLSID_POSDET_11 | g::AEECLSID_POSDET_20 => {}
                    _ => return Ok(g::AEE_EBADPARM),
                }
                // ⚠ The body's `return 0` sits **outside** the privilege test and outside the device
                // call: a caller with a null structure is told success, and a caller whose request the
                // device could not serve is told success too, with the structure left as it was. The
                // `size >= 10` test the body repeats before the call is one of the two sizes above, so
                // it can never fail here.
                if pi != 0 {
                    let _ = self.posdet_device("GetSectorInfo");
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetSectorInfoEx" => {
                let [pi, size] = self.args()?;
                if pi == 0 || size < 10 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(self.posdet_device("GetSectorInfoEx"))
            }
            "SetGPSConfig" => {
                let [pc] = self.args()?;
                if pc == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let server =
                    abi::offset_of("AEEGPSConfig", "server").expect("AEEGPSConfig is here");
                let svr_type = self.cpu.mem.read_u32(pc + server)?;
                // Naming a server that is not the platform default is a bigger ask than choosing a
                // mode: the body allows it for a `_20` or `_302` object, or for an applet in
                // `AEECLSID_LOCATIONSERVER`'s group. The group half is the question this host cannot
                // answer no to (see above), so what is left is the version test — and it is real:
                // a `_11` or current object may not name a server.
                let clsid = self.posdet_cls(this);
                if svr_type != 0
                    && clsid != g::AEECLSID_POSDET_20
                    && clsid != g::AEECLSID_POSDET_302
                {
                    return Ok(g::AEE_EPRIVLEVEL);
                }
                Ok(self.posdet_device("SetGPSConfig"))
            }
            "GetGPSConfig" => {
                let [pc] = self.args()?;
                if pc == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // The reference `aee_memmove(pc, obj + 44, 20)` — the object's own store, which on this
                // host is the zeroed block `aee_malloc` handed out (the delivered allocator clears by
                // default) and nothing has ever written. The `_20` swap-back of the mirror cannot
                // disagree with these bytes, because the only writer of that mirror is the `Set` above.
                for word in 0..5u32 {
                    self.cpu.mem.write_u32(pc + word * 4, 0)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetGPSInfo" => {
                let [_req, acc, pi, pcb] = self.args()?;
                if pi == 0 || pcb == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // Past the call the body links the caller's callback through
                // `AEE_LinkSysCallbackEx` (220 bytes, `AEE_ENOMEMORY` when that fails), stamps the
                // request into the caller's `AEEGPSInfo` and asks the platform for permission to reveal
                // a position at all. None of it is reachable without a device.
                //
                // The accuracy is named in the refusal because it is the only place this number appears:
                // `IShell::GetPosition` chooses it from `wPrecision` and hands it over here, and nothing
                // else in this host ever reads it back.
                let asked = format!("GetGPSInfo (accuracy {acc})");
                Ok(self.posdet_device(&asked))
            }
            "GetOrientation" => {
                let [_req, _pi, pcb] = self.args()?;
                let notify = abi::offset_of("_AEECallback", "pfnNotify")
                    .expect("_AEECallback::pfnNotify is in the ABI tables");
                if self.cpu.mem.read_u32(pcb + notify)? == 0 {
                    // No notify function in the caller's block and the body answers `0` — success —
                    // having registered nothing. A pack that forgets to fill the block is told the
                    // request is in hand and never hears from it.
                    return Ok(g::AEE_SUCCESS);
                }
                // The body then registers its own cancel hook in the caller's block, keeps the block at
                // `+308` for `IPosDet_Delete`, and asks the device; when the device refuses it cancels
                // the registration again before returning. Both halves of that pair are a host function
                // address written into guest memory, which is not something this host can hand out, and
                // no request survives this call to be cancelled later — so nothing is written into the
                // caller's block here, and the refusal is the whole observable.
                Ok(self.posdet_device("GetOrientation"))
            }
            "ExtractPositionInfo" => {
                let [gps, out] = self.args()?;
                self.posdet_extract(gps, out)
            }
            _ => Err(unimplemented().into()),
        }
    }

    /// The `IRingerMgr` slots. This family's body is the OEM's, and the only copy of it this box has is
    /// real source — the reference 235 members hold no ringer object — so the
    /// source is the highest record available here, and every answer below is taken from it line for
    /// line.
    ///
    /// The object behind every slot is the run's one `gMgr`: the reference `CRingerMgr_New` hands the same
    /// static to each `CreateInstance`, so a second holder sees the first holder's enumeration cursors,
    /// its category table and its `PFNRINGEREVENT` — not a fresh set of its own.
    pub(crate) fn iringermgr(
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
                if ppv == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(self.answer_qi(this, iid, ppv, &[g::AEECLSID_RINGERMGR])?)
            }
            "RegisterNotify" => {
                // Two stores and no return value: `pme->m_pfnEvent = pfn; pme->m_pEvent = pUser;`.
                // The manager's event door is one slot wide for the whole run, so the last holder to
                // register is the only one that gets the `ARE_*` reports.
                let [pfn, user] = self.args()?;
                self.ringer.pfn = pfn;
                self.ringer.user = user;
                Ok(0)
            }
            "GetNumberFormats" => Ok(RINGER_FORMATS.len() as u32),
            "GetFormats" => {
                let [dst, count] = self.args()?;
                // `if (nCount && pwFormat) { if (nCount >= gnFormats) { …every row…; return 0; } }
                // return EFAILED;` — a caller with room for **fewer** than the table holds is refused
                // rather than given a partial list, and nothing is written on that path.
                if count == 0 || dst == 0 || count < RINGER_FORMATS.len() as u32 {
                    return Ok(g::AEE_EFAILED);
                }
                for (i, (format, _, _)) in RINGER_FORMATS.iter().enumerate() {
                    self.cpu
                        .mem
                        .write_u32(dst + i as u32 * 4, ringer_format_id(*format))?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "SetRinger" => {
                let [cat, ring_id] = self.args()?;
                // The ringer is looked up **first**: a name the walk has not got cannot become the
                // active one, whatever the category says. `OEM_RINGER_SUPPORT` would hand the choice to
                // the OEM's own active-ringer table (`OEM_SetActiveRinger`, a prototype with no body in
                // this box), so this is the body's own `#else`: the per-category array.
                if self.ringer_find(0, RingerWant::Id(ring_id))?.is_none() {
                    return Ok(g::AEE_EFAILED);
                }
                // `if ( idCat <= CAT_MAX )` — on an unsigned id, so a category above the table is
                // refused rather than wrapped into the array.
                if cat as usize > RINGER_CATEGORIES.len() - 1 {
                    return Ok(g::AEE_EFAILED);
                }
                self.ringer.categories[cat as usize] = ring_id;
                Ok(g::AEE_SUCCESS)
            }
            "EnumCategoryInit" => {
                // `m_enumCat = AEE_RINGER_CATEGORY_ALL`, which is 0, and the answer is always 0.
                self.ringer.enum_cat = 0;
                Ok(g::AEE_SUCCESS)
            }
            "EnumNextCategory" => {
                let [dst] = self.args()?;
                if dst == 0 {
                    return Ok(g::FALSE);
                }
                // The clear runs before the range gate, so a caller that asks past the last category
                // still gets a zeroed record back with the FALSE.
                let size = abi::struct_info("AEERingerCat")
                    .expect("AEERingerCat is in the tables")
                    .size as usize;
                self.cpu.mem.write_bytes(dst, &vec![0u8; size])?;
                let cat = self.ringer.enum_cat;
                // `if (pme->m_enumCat > CAT_MAX) return FALSE;` — and `CRingerMgr_New` starts the cursor
                // at `AEE_RINGER_CATEGORY_NONE`, so a caller that never ran `EnumCategoryInit` gets
                // nothing but the zeroed record.
                if cat as usize > RINGER_CATEGORIES.len() - 1 {
                    return Ok(g::FALSE);
                }
                let at = |field: &str| {
                    abi::offset_of("AEERingerCat", field).expect("AEERingerCat is in the tables")
                };
                self.cpu.mem.write_u32(dst + at("id"), cat)?;
                self.put_ringer_wstr(
                    dst + at("szName"),
                    RINGER_CATEGORIES[cat as usize],
                    g::MAX_RINGER_NAME as usize,
                )?;
                self.cpu
                    .mem
                    .write_u32(dst + at("idRinger"), self.ringer.categories[cat as usize])?;
                self.ringer.enum_cat = cat + 1;
                Ok(g::TRUE)
            }
            "EnumRingerInit" => Ok(self.ringer_walk_init()),
            "EnumNextRinger" => {
                let [dst] = self.args()?;
                let size = abi::struct_info("AEERingerInfo")
                    .expect("AEERingerInfo is in the tables")
                    .size as usize;
                // `if (pi) MEMSET(pi, 0, sizeof(AEERingerInfo));` — the documented behaviour clears only when it
                // was given a record, then writes into it **without** any further gate: a null record
                // with a file in the directory is a write to address zero on a handset. This host
                // answers FALSE for that caller, and says so; the entry is still consumed, because the
                // body takes it before it touches the record.
                if dst != 0 {
                    self.cpu.mem.write_bytes(dst, &vec![0u8; size])?;
                }
                if self.ringer.enum_ringer == RINGER_ID_NONE {
                    return Ok(g::FALSE);
                }
                let Some(entry) = self.ringer_walk_next() else {
                    return Ok(g::FALSE);
                };
                if dst == 0 {
                    self.display.note(
                        "IRingerMgr::EnumNextRinger was handed a null record after taking an entry: \
                         the platform routine writes into it and would fault there; this host answered \
                         FALSE and left the entry consumed",
                    );
                    return Ok(g::FALSE);
                }
                self.put_ringer_info(dst, &entry)?;
                Ok(g::TRUE)
            }
            "GetRingerID" => {
                let [psz] = self.args()?;
                let asked = self.cstring(psz)?;
                // `pszIn = STRRCHR(pszFile, DIRECTORY_CHAR); if (pszIn) pszIn++; else pszIn = pszFile;`
                // — the query is the leaf of what the caller spelled, and the match is case-insensitive.
                let leaf = ringer_leaf(&asked).to_owned();
                Ok(match self.ringer_find(0, RingerWant::Leaf(leaf))? {
                    Some(entry) => entry.id,
                    None => RINGER_ID_NONE,
                })
            }
            "GetRingerInfo" => {
                let [ring_id, dst] = self.args()?;
                if dst == 0 {
                    self.display.note(
                        "IRingerMgr::GetRingerInfo was handed a null record: the shipped Find writes \
                         every candidate into it through EnumNextRinger and would fault there; this \
                         host ran the walk and answered the code",
                    );
                }
                Ok(
                    if self.ringer_find(dst, RingerWant::Id(ring_id))?.is_some() {
                        g::AEE_SUCCESS
                    } else {
                        g::AEE_EFAILED
                    },
                )
            }
            "Remove" => {
                let [ring_id] = self.args()?;
                // `if (!CRingerMgr_GetRingerInfo(po, id, &ri)) return IFILEMGR_Remove(pfm, ri.szFile);`
                // — so an id the walk has not got is `EFAILED`, and a real one is removed by the **path**
                // the enumeration reported, not by a name the caller invented.
                match self.ringer_find(0, RingerWant::Id(ring_id))? {
                    Some(entry) => Ok(self.volume.remove(&entry.file)),
                    None => Ok(g::AEE_EFAILED),
                }
            }
            // `CRingerMgr_Create(pme, pszName, format, pStream)` — the one slot that reads the caller's
            // stream, and the body's own chain is followed step for step: finish any create in flight,
            // refuse a null stream, take a reference on it, copy the wide name, malloc the detection
            // buffer, then either name the file straight away (`ISPREDEFINED`) or read until
            // `ISHELL_DetectType` sniffs, pick the handler class, and only then name and write.
            "Create" => {
                let [psz_name, format, pstream] = self.args()?;
                // The body's first statement, which is also why two creates never interleave: whatever
                // the last one was holding is given back, and the pack hears `ARE_CREATE` with
                // `EINCOMPLETEITEM` if anything was held.
                self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EINCOMPLETEITEM)?;
                if pstream == 0 {
                    self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EBADPARM)?;
                    return Ok(g::AEE_EBADPARM);
                }
                self.ringer.stream = pstream;
                self.hold(pstream, 1)?;
                self.ringer.format = format;
                // `m_pszName = MALLOC(sizeof(AECHAR) * (WSTRLEN(pszName) + 1)); WSTRCPY(...)` — the
                // wide copy is kept as units; `FormatFileName` is what narrows it.
                let mut name = Vec::new();
                let mut at = psz_name;
                loop {
                    let unit = self.cpu.mem.read_u16(at)? as u16;
                    if unit == 0 {
                        break;
                    }
                    name.push(unit);
                    at += 2;
                }
                self.ringer.name = name;
                // `m_pBuff = MALLOC(OEM_RING_BUFF)`, and its `ENOMEMORY` if the heap says no.
                match self.hle.alloc(&mut self.cpu.mem, RINGER_BUFF) {
                    Ok(buff) => self.ringer.buff = buff,
                    Err(_) => {
                        self.ringer_create_finish(RINGER_ID_NONE, g::AEE_ENOMEMORY)?;
                        return Ok(g::AEE_ENOMEMORY);
                    }
                }
                if ringer_predefined(format) {
                    return self.ringer_create_ringer();
                }
                self.ringer_detect_type()
            }
            "Play" => {
                let [ring_id, psz_file, pstream, dw_pause] = self.args()?;
                // `CRingerMgr_Play` opens with `CRingerMgr_Stop(po)` and throws the answer away, so a
                // pass already running is ended **first** — and its caller hears `ARE_PLAY` with
                // `AEE_EINCOMPLETEITEM` before anything new has been named.
                self.ringer_stop()?;
                // `if (id != AEE_RINGER_ID_NONE && !CRingerMgr_GetRingerInfo(po, id, &ri)) pszFile =
                // ri.szFile;` — the id only *replaces* the caller's name when the walk has it. An id it
                // has not got leaves `pszFile` exactly as it came in, and a null `pszFile` with a null
                // stream is the body's `EFAILED`.
                let mut named = if psz_file != 0 {
                    Some(self.cstring(psz_file)?)
                } else {
                    None
                };
                if ring_id != RINGER_ID_NONE {
                    if let Some(entry) = self.ringer_find(0, RingerWant::Id(ring_id))? {
                        named = Some(entry.file);
                    }
                }
                if named.is_none() && pstream == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                // `ISHELL_CreateInstance(pme->m_pShell, AEECLSID_SOUNDPLAYER, &pme->m_pPlayer)`, and the
                // manager keeps the one reference the constructor gave it — which is what `new_object`'s
                // count already is, and why there is no `return_object` here.
                let player = self.hle.new_object(
                    &mut self.cpu.mem,
                    "ISoundPlayer",
                    g::AEECLSID_SOUNDPLAYER,
                    8,
                    4,
                    &[],
                )?;
                // `CRingerMgr_SoundStatus` is registered in place of a pack's handler, so this player
                // reports to the manager (see [`Player::host_ringer`]); `noted` is set because the
                // one-time "this host only sounds MIDI" line belongs to a pack that asked for a player,
                // not to one the manager made behind its back.
                self.players.insert(
                    player,
                    Player {
                        host_ringer: this,
                        noted: true,
                        ..Default::default()
                    },
                );
                self.ringer.player = player;
                self.ringer.delay = dw_pause;
                self.ringer.playing = true;
                // The body's own order: register, name, play. `RegisterNotify` is called with the pair
                // the reference call carries — the callback's address is this host's, so only the manager's
                // own address travels as `pUser`.
                self.ringer_player_slot("RegisterNotify", player, &[0, this])?;
                if let Some(name) = &named {
                    // `ISOUNDPLAYER_Set(pPlayer, SDT_FILE, pszFile)`. The pointer the reference call hands
                    // over is the caller's own, and the player copies it; this host resolved the name out
                    // of the walk, so it goes into a block that is given back the moment that copy is
                    // made — which is the same lifetime the caller's string had on a handset.
                    let block = self.hle.alloc(&mut self.cpu.mem, name.len() as u32 + 1)?;
                    self.cpu.mem.write_bytes(block, name.as_bytes())?;
                    self.cpu.mem.write_u8(block + name.len() as u32, 0)?;
                    let answer = self.ringer_player_slot("Set", player, &[g::SDT_FILE, block]);
                    self.hle.dealloc(&mut self.cpu.mem, block);
                    answer?;
                } else {
                    // `ISOUNDPLAYER_SetStream`. This host keeps the naming and has no way to read a
                    // stream's bytes, so the player's own `Play` gate refuses it and no window follows —
                    // and with no window there is no `DONE` to report the pass over. The handset's player
                    // refuses an unusable media the same way; what it does *not* do is leave the manager
                    // holding a pass that can only be ended by `Stop`, so the fact goes in the report.
                    self.display.note(format!(
                        "{label}: Play was handed an IAStream, which this host records but cannot read; \
                         the pass has no length to time, so no ARE_PLAY follows until Stop"
                    ));
                    self.ringer_player_slot("SetStream", player, &[pstream])?;
                }
                self.ringer_player_slot("Play", player, &[])?;
                Ok(g::AEE_SUCCESS)
            }
            "Stop" => Ok(self.ringer_stop()?),
            _ => Err(unimplemented().into()),
        }
    }

    /// `CRingerMgr_Release`'s teardown branch, which is the reason the manager's player does not outlive
    /// it: the body runs `Stop` — and `Stop` releases the player and reports `ARE_PLAY` — then
    /// `FinishCreate(AEE_RINGER_ID_NONE, EINCOMPLETEITEM)`, then `MEMSET(&gMgr, 0, sizeof(gMgr))`, so what
    /// a handset keeps past the last release is the static's address and none of its state. This host
    /// forgets the address too, because its object layer drops the record at zero — the next
    /// `CreateInstance` re-runs `CRingerMgr_New` and gets fresh cursors either way.
    ///
    /// The notification inside that `Stop` is what the reference `m_bInRelease` latch exists for: the body
    /// takes a reference around the guest call and gives it back with a `Release` that reaches zero
    /// again, so without the latch the teardown would run once more per event delivered.
    pub(crate) fn ringer_died(&mut self, addr: u32) -> Result<(), Error> {
        if addr == 0 || addr != self.ringer_object || self.ringer.in_release {
            return Ok(());
        }
        self.ringer.in_release = true;
        // The object layer gives this address up at the top of the release hook — `Display::forget` is
        // the line that forgets it, and it runs before this one — but the reference static stays
        // addressable for the whole teardown: `CRingerMgr_Notify` counts references on `pme` and the
        // pack's handler may call any slot of the manager from inside the notification. So the record is
        // claimed back for the duration, and the release the notification makes gives it up again —
        // which leaves the caller's pointer dangling exactly as `MEMSET` leaves the handset's.
        if self.hle.object(addr).is_none() {
            self.hle
                .adopt_object(addr, "IRingerMgr", g::AEECLSID_RINGERMGR, 4);
        }
        // The reference order is `IFILEMGR_Release`, `Stop`, `FinishCreate`, `MEMSET`. This host has no
        // file manager of its own to give back — it reads [`Runtime::volume`] directly — so what is
        // left is the pass and anything a create left in flight, whose `ARE_CREATE` with
        // `EINCOMPLETEITEM` goes out from inside this release.
        self.ringer_stop()?;
        self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EINCOMPLETEITEM)?;
        self.ringer = Ringer::born();
        self.ringer_object = 0;
        // When the event door was never registered, the reference dance above does not run and nothing
        // else gives the address up — and a record left behind would let a pack raise the released
        // manager by calling through its old pointer, which on a handset is exactly the call that
        // faults: `MEMSET` zeroed the static's vtable. Forget it here; `Release` from inside the
        // notification already did the same for the path where the dance did run.
        self.hle.forget(addr);
        Ok(())
    }

    /// `CRingerMgr_Stop`: the delay goes and the replay timer is cancelled — both unconditionally, and
    /// both before the player is even looked at — then the player is given back and the caller is told
    /// the pass ended, with `AEE_EINCOMPLETEITEM` when the manager still believed it was playing.
    ///
    /// `m_bPlaying` is **not** cleared here, because nothing in the body clears it: the only thing that
    /// does is the device's `DONE` report. That is why a `Stop` of a manager whose pass already reported
    /// done answers `0` with no error attached, and why two `Stop`s in a row give `0` then `EFAILED`.
    pub(crate) fn ringer_stop(&mut self) -> Result<u32, Error> {
        self.ringer.delay = 0;
        self.ringer.replay_due = None;
        let player = self.ringer.player;
        if player == 0 {
            return Ok(g::AEE_EFAILED);
        }
        self.ringer.player = 0;
        let release = abi::method_of("ISoundPlayer", "Release")
            .expect("ISoundPlayer::Release is in the tables");
        self.object_method(release, player, "ISoundPlayer")?;
        let err = if self.ringer.playing {
            g::AEE_EINCOMPLETEITEM
        } else {
            0
        };
        self.ringer_notify(g::ARE_PLAY, 0, err)?;
        Ok(g::AEE_SUCCESS)
    }

    /// `CRingerMgr_Notify`: the event door, with the documented behaviour's reference dance around the call.
    /// That dance is not decoration — the return value is `Release(...) != 0`, so a handler that gave up
    /// its last reference from inside its own notification makes this answer FALSE, and the one caller
    /// that reads it ([`Runtime::ringer_sound_status`]) drops the replay timer it was about to arm. With
    /// no handler registered the body answers TRUE without calling anyone.
    pub(crate) fn ringer_notify(&mut self, evt: u32, parm: u32, err: u32) -> Result<bool, Error> {
        if self.ringer.pfn == 0 {
            return Ok(true);
        }
        let mgr = self.ringer_object;
        let addref =
            abi::method_of("IRingerMgr", "AddRef").expect("IRingerMgr::AddRef is in the tables");
        self.object_method(addref, mgr, "IRingerMgr")?;
        self.call_guest(self.ringer.pfn, &[self.ringer.user, evt, parm, err])?;
        self.notified.0 += 1;
        let release =
            abi::method_of("IRingerMgr", "Release").expect("IRingerMgr::Release is in the tables");
        Ok(self.object_method(release, mgr, "IRingerMgr")? != 0)
    }

    /// `CRingerMgr_SoundStatus` — the report the manager registers on the player it made. It acts on
    /// exactly one combination, `AEE_SOUNDPLAYER_PLAY_CB` with `AEE_SOUNDPLAYER_DONE`, and ignores every
    /// other command report and every other status, the per-second tick included. The order is what has
    /// to be kept: the flag goes down **before** the notification, so a handler that calls `Play` from
    /// inside `ARE_PLAY` starts a clean pass instead of being told a running one was swapped; and the
    /// deadline is only armed when the notification said the manager is still alive.
    pub(crate) fn ringer_sound_status(&mut self, cmd: u32, status: u32) -> Result<(), Error> {
        if cmd != g::AEE_SOUNDPLAYER_PLAY_CB || status != g::AEE_SOUNDPLAYER_DONE {
            return Ok(());
        }
        self.ringer.playing = false;
        if !self.ringer_notify(g::ARE_PLAY, 0, 0)? {
            return Ok(());
        }
        if self.ringer.delay != 0 {
            self.ringer.replay_due = Some(self.clock_ms.saturating_add(self.ringer.delay));
        }
        Ok(())
    }

    /// `CRingerMgr_Replay`: `if (pme->m_pPlayer) ISOUNDPLAYER_Play(pme->m_pPlayer);` and nothing else.
    /// The manager's `m_bPlaying` is **not** raised here — the body does not raise it — so a `Stop` after
    /// a replayed pass reports `ARE_PLAY` with no error, while a `Stop` during the first one reports
    /// `AEE_EINCOMPLETEITEM`.
    pub(crate) fn ringer_replay(&mut self) -> Result<(), Error> {
        self.ringer.replay_due = None;
        if self.ringer.player != 0 {
            let player = self.ringer.player;
            self.ringer_player_slot("Play", player, &[])?;
        }
        Ok(())
    }

    /// Call one of this host's own `ISoundPlayer` slots with a staged argument list, the way
    /// `CRingerMgr_Play` and `CRingerMgr_Release` call the player they hold.
    ///
    /// Only `RegisterNotify`, `Set`, `SetStream`, `Play` and `Release` are reached this way, and none of
    /// them takes a word on the stack, so `r1..r3` are the whole list; they go back before the answer
    /// travels. The nested call is the **manager's**, not the pack's, and that is what makes the player's
    /// own reports — `PLAY_CB` for a `Play`, `STATUS_CB` for a `Set` — arrive inside the `IRingerMgr` slot
    /// that asked for them, in the order the platform's own routines put them.
    #[inline(never)]
    pub(crate) fn ringer_player_slot(
        &mut self,
        name: &str,
        player: u32,
        args: &[u32],
    ) -> Result<u32, Error> {
        let id = abi::method_of("ISoundPlayer", name).ok_or(HleError::Unimplemented {
            id: 0,
            label: "ISoundPlayer".to_string(),
            this: player,
        })?;
        self.nested_method(id, player, "ISoundPlayer", args, 0)
    }

    /// `CRingerMgr_DetectType`: read the stream into the detection buffer until
    /// `ISHELL_DetectType` names a MIME, then let the handler table decide the format. Every branch
    /// that ends the create goes through [`Runtime::ringer_create_finish`], and the block per read is
    /// reported to the pack as `ARE_WRITE` exactly when the sniffer asks for more.
    pub(crate) fn ringer_detect_type(&mut self) -> Result<u32, Error> {
        // `&pme->m_nBytes` is the body's own field address; this host keeps the count in `Ringer` and
        // mirrors it through a word for the duration of each call, because the shell slot writes the
        // caller's length word and reads the bytes through it.
        let count = self.hle.alloc(&mut self.cpu.mem, 4)?;
        let mime_out = self.hle.alloc(&mut self.cpu.mem, 4)?;
        let frame = self.hle.alloc(&mut self.cpu.mem, 8)?;
        let detect =
            abi::method_of("IShell", "DetectType").expect("IShell::DetectType is in the tables");
        let shell = self.shell;
        let answer: Result<u32, Error> = loop {
            if self.ringer.bytes >= RINGER_BUFF {
                // The buffer is full and nothing sniffed: the body falls out of its loop with
                // `cpszMIME` as the last call left it, which is `NULL` here, and answers `EFAILED`.
                self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EFAILED)?;
                break Ok(g::AEE_EFAILED);
            }
            let at = self.ringer.buff + self.ringer.bytes;
            let want = RINGER_BUFF - self.ringer.bytes;
            let got = self.stream_read(self.ringer.stream, at, want)?;
            if got == 0 {
                // The stream is spent and the bytes never named a type: `ENEEDMORE`, and the
                // half-made file (there is none yet) with it.
                self.ringer_create_finish(RINGER_ID_NONE, g::AEE_ENEEDMORE)?;
                break Ok(g::AEE_ENEEDMORE);
            }
            if got == g::AEE_NET_WOULDBLOCK {
                // `IASTREAM_Readable(pStream, (PFNNOTIFY)CRingerMgr_DetectType, pme)` re-arms the
                // body's own static function — a host address, which no guest vector table can call.
                // The create stays open exactly as the body leaves it and resumes when the pack's
                // next `Create` or `Release` finishes it; that resume is this host's named gap.
                self.display.note(
                    "IRingerMgr::Create: the stream answered WOULDBLOCK; the shipped manager \
                     re-arms its own reader through IASTREAM_Readable, and this host holds the \
                     create open until the next Create or Release finishes it"
                        .to_string(),
                );
                break Ok(g::AEE_NET_WOULDBLOCK);
            }
            self.ringer.bytes += got;
            self.cpu.mem.write_u32(count, self.ringer.bytes)?;
            self.cpu.mem.write_u32(mime_out, 0)?;
            let code = self.nested_method(
                detect,
                shell,
                "IShell",
                &[self.ringer.buff, count, 0, mime_out],
                frame,
            )?;
            self.ringer.bytes = self.cpu.mem.read_u32(count)?;
            if code == g::AEE_SUCCESS {
                let mime = self.cpu.mem.read_u32(mime_out)?;
                break self.ringer_detect_chosen(mime);
            }
            if code == g::AEE_ENEEDMORE {
                // The sniffer wants more bytes than it has; the block just read is the pack's
                // `ARE_WRITE` news.
                self.ringer_notify(g::ARE_WRITE, got, 0)?;
                continue;
            }
            self.ringer_create_finish(RINGER_ID_NONE, code)?;
            break Ok(code);
        };
        self.hle.dealloc(&mut self.cpu.mem, count);
        self.hle.dealloc(&mut self.cpu.mem, mime_out);
        self.hle.dealloc(&mut self.cpu.mem, frame);
        answer
    }

    /// The MIME is named: `ISHELL_GetHandler(AEECLSID_MEDIA, cpszMIME)` picks the class, and the
    /// format table decides whether this device rings that class — a class outside the table leaves
    /// `m_eFormat` at `AEE_SOUNDPLAYER_FILE_UNKNOWN`, which `FormatFileName` then refuses.
    pub(crate) fn ringer_detect_chosen(&mut self, mime: u32) -> Result<u32, Error> {
        if mime == 0 {
            self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EFAILED)?;
            return Ok(g::AEE_EFAILED);
        }
        let get_handler =
            abi::method_of("IShell", "GetHandler").expect("IShell::GetHandler is in the tables");
        let cls = self.nested_method(
            get_handler,
            self.shell,
            "IShell",
            &[g::AEECLSID_MEDIA, mime],
            0,
        )?;
        if cls == 0 {
            self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EUNSUPPORTED)?;
            return Ok(g::AEE_EUNSUPPORTED);
        }
        self.ringer.format = RINGER_FORMATS
            .iter()
            .find(|(_, c, _)| *c == cls)
            .map(|(f, _, _)| *f)
            .unwrap_or(g::AEE_SOUNDPLAYER_FILE_UNKNOWN);
        self.ringer_create_ringer()
    }

    /// `CRingerMgr_CreateRinger`: name the file, refuse a taken name, create it, and hand the write
    /// loop the rest of the stream.
    pub(crate) fn ringer_create_ringer(&mut self) -> Result<u32, Error> {
        let Some(path) = ringer_format_file_name(&self.ringer.name, self.ringer.format) else {
            self.ringer_create_finish(RINGER_ID_NONE, g::AEE_EFAILED)?;
            return Ok(g::AEE_EFAILED);
        };
        // `if (!IFILEMGR_Test(pfm, szFile)) → EFILEEXISTS`: `CFileMgr_Test` answers the **boolean**
        // `code != 0` — zero means the name is taken — so the gate fires when the name is *not* free.
        if self.volume.test(&path) == 0 {
            self.ringer_create_finish(RINGER_ID_NONE, EFILEEXISTS)?;
            return Ok(EFILEEXISTS);
        }
        let opened = match self.volume.open(&path, g::_OFM_CREATE) {
            Ok(opened) => opened,
            Err(why) => {
                self.ringer_create_finish(RINGER_ID_NONE, why)?;
                return Ok(why);
            }
        };
        self.ringer.file = Some(opened);
        self.ringer.path = path;
        self.ringer_write_file()
    }

    /// `CRingerMgr_WriteFile`: the bytes detection already read go first, then the stream block by
    /// block, each full block reported as `ARE_WRITE`, until the stream is spent — and the verdict is
    /// whether the walk has the file it just closed.
    pub(crate) fn ringer_write_file(&mut self) -> Result<u32, Error> {
        let stream = self.ringer.stream;
        let buff = self.ringer.buff;
        if self.ringer.bytes != 0 {
            let bytes = self.cpu.mem.read_bytes(buff, self.ringer.bytes as usize)?;
            if !self.ringer_write_bytes(&bytes)? {
                return Ok(EFSFULL);
            }
        }
        loop {
            let got = self.stream_read(stream, buff, RINGER_BUFF)?;
            if got == 0 {
                // The stream is spent. The file closes **first**, and only then does the walk get
                // asked whether the name it just made is a ringer it would list — a created file the
                // walk does not recognise is removed by `FinishCreate`, not served.
                self.ringer.file = None;
                let leaf = ringer_leaf(&self.ringer.path).to_owned();
                let found = self.ringer_find(0, RingerWant::Leaf(leaf))?;
                let n_err = if found.is_some() { 0 } else { g::AEE_EFAILED };
                let id = found.map(|entry| entry.id).unwrap_or(RINGER_ID_NONE);
                self.ringer_create_finish(id, n_err)?;
                return Ok(n_err);
            }
            if got == g::AEE_NET_WOULDBLOCK {
                // The same named gap as in the detection loop: the resume is the body's own static
                // function, re-armed through `IASTREAM_Readable`, and this host holds the create open
                // instead.
                self.display.note(
                    "IRingerMgr::Create: the stream answered WOULDBLOCK mid-write; the create \
                     stays open until the next Create or Release finishes it"
                        .to_string(),
                );
                return Ok(g::AEE_NET_WOULDBLOCK);
            }
            let bytes = self.cpu.mem.read_bytes(buff, got as usize)?;
            if !self.ringer_write_bytes(&bytes)? {
                return Ok(EFSFULL);
            }
            self.ringer_notify(g::ARE_WRITE, got, 0)?;
        }
    }

    /// One `IFILE_Write` of the create, with the body's own verdict for a short one: `EFSFULL`, and
    /// `FinishCreate` removes the half-made file behind it. `false` is that short write.
    pub(crate) fn ringer_write_bytes(&mut self, bytes: &[u8]) -> Result<bool, Error> {
        let wrote = match self.ringer.file.as_mut() {
            Some(open) => self.volume.write(open, bytes),
            None => Err(g::AEE_EFAILED),
        };
        match wrote {
            Ok(n) if n == bytes.len() as u32 => Ok(true),
            _ => {
                self.ringer_create_finish(RINGER_ID_NONE, EFSFULL)?;
                Ok(false)
            }
        }
    }

    /// `CRingerMgr_FinishCreate`: give back everything the create was holding — the stream reference,
    /// the open file, the detection buffer — remove the half-made file on every error path, and tell
    /// the pack `ARE_CREATE` with the new id or the code, but only if something was actually held.
    /// The caller's answer is the code it came in with.
    pub(crate) fn ringer_create_finish(&mut self, id: u32, n_err: u32) -> Result<(), Error> {
        let mut held = false;
        if self.ringer.stream != 0 {
            held = true;
            let stream = self.ringer.stream;
            self.ringer.stream = 0;
            self.stream_ref(stream, false)?;
        }
        if self.ringer.file.is_some() {
            held = true;
            self.ringer.file = None;
        }
        if !self.ringer.path.is_empty() {
            held = true;
            if n_err != 0 {
                self.volume.remove(&self.ringer.path);
            }
            self.ringer.path.clear();
        }
        if self.ringer.buff != 0 {
            held = true;
            let buff = self.ringer.buff;
            self.ringer.buff = 0;
            self.ringer.bytes = 0;
            self.hle.dealloc(&mut self.cpu.mem, buff);
        }
        self.ringer.name.clear();
        if held {
            self.ringer_notify(g::ARE_CREATE, id, n_err)?;
        }
        Ok(())
    }

    /// `CRingerMgr_EnumRingerInit`: rewind the walk to the first row of [`RINGER_FORMATS`] that opens.
    /// The cursor the body leaves is the row that succeeded, and `m_enumRinger` starts at 0 so the
    /// first entry gets id 1.
    pub(crate) fn ringer_walk_init(&mut self) -> u32 {
        self.ringer.enum_ringer = 0;
        for (i, (_, _, pattern)) in RINGER_FORMATS.iter().enumerate() {
            if self.volume.enum_init(pattern, false) == g::AEE_SUCCESS {
                self.ringer.enum_format = i;
                return g::AEE_SUCCESS;
            }
        }
        // No row opened: the body's loop ends with the index past the table, and the answer is
        // `EFAILED` — which is also what stops `EnumNextRinger` from walking an old listing.
        self.ringer.enum_format = RINGER_FORMATS.len();
        g::AEE_EFAILED
    }

    /// One step of the body's ringer walk, as a [`RingerEntry`] rather than as bytes: take the current
    /// row until its listing is spent, then move to the next row that opens
    /// (`for (++idx; idx < gnFormats; idx++) if (!IFILEMGR_EnumInit(...)) break;`).
    pub(crate) fn ringer_walk_next(&mut self) -> Option<RingerEntry> {
        while self.ringer.enum_format < RINGER_FORMATS.len() {
            if let Some(info) = self.volume.enum_next() {
                let idx = self.ringer.enum_format;
                self.ringer.enum_ringer += 1;
                return Some(RingerEntry {
                    id: self.ringer.enum_ringer,
                    format: ringer_format_id(RINGER_FORMATS[idx].0),
                    file: info.name.clone(),
                    name: ringer_display_name(&info.name),
                });
            }
            self.ringer.enum_format += 1;
            while self.ringer.enum_format < RINGER_FORMATS.len() {
                if self
                    .volume
                    .enum_init(RINGER_FORMATS[self.ringer.enum_format].2, false)
                    == g::AEE_SUCCESS
                {
                    break;
                }
                self.ringer.enum_format += 1;
            }
        }
        None
    }

    /// `CRingerMgr_Find`: start the walk over and take every ringer until a predicate answers. Each
    /// candidate is written into the caller's record **before** the test, the way the body does it
    /// inside `EnumNextRinger` — so a walk that finds nothing still leaves the **last examined** entry
    /// in the caller's memory, which is what its own stack `AEERingerInfo` holds when `GetRingerInfo`
    /// answers `EFAILED`. `dst == 0` is that stack record: the same walk, nothing written.
    pub(crate) fn ringer_find(
        &mut self,
        dst: u32,
        want: RingerWant,
    ) -> Result<Option<RingerEntry>, Error> {
        if self.ringer_walk_init() != g::AEE_SUCCESS {
            return Ok(None);
        }
        while let Some(entry) = self.ringer_walk_next() {
            if dst != 0 {
                self.put_ringer_info(dst, &entry)?;
            }
            let hit = match &want {
                RingerWant::Id(id) => entry.id == *id,
                RingerWant::Leaf(leaf) => ringer_leaf(&entry.file).eq_ignore_ascii_case(leaf),
            };
            if hit {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }

    pub(crate) fn imedia(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        if !self.players.contains_key(&this) {
            // Not a media object this host handed out, so not one with anything to answer from.
            return Err(HleError::NotAnObject { addr: this });
        }
        let mut player = self.players[&this];
        // Whether this call began a pass the caller wants told about. Decided in the `Play` arm,
        // delivered only after `arm_notify` below has said whether the pass can be followed to its end.
        let mut start_news = false;
        // What the seek this call performed owes the registered handler: the `MM_STATUS_SEEK` itself,
        // and separately whether the media then *resumed*, because the header says the `RESUME` after a
        // seek is conditional on the media having been playing. Collected here and posted below, after
        // `arm_notify` has said whether the new window was accepted at all.
        let mut seek_news: Vec<(u32, u32, u32)> = Vec::new();
        let mut seek_resume: Option<u32> = None;
        // The same for a pause and a resume, which the header's table also pairs with an elapsed time:
        // posted only once the position the number speaks of has been written down.
        let mut pause_news: Option<u32> = None;
        let mut resume_news = false;
        // Set only by `MM_PARM_TEMPO` when the value is outside the shared factor's range: the
        // call did not set anything, so its answer is a bad parameter rather than success.
        let mut tempo_bad = false;
        // The same for `MM_PARM_TUNE`, whose step is the same ±12 semitones the sound player's own
        // `SetTune` accepts.
        let mut tune_bad = false;
        let retval = match name {
            "RegisterNotify" => {
                let (pfn, user) = (self.arg(1)?, self.arg(2)?);
                player.pfn = pfn;
                player.user = user;
                g::AEE_SUCCESS
            }
            "SetMediaParm" => {
                let parm = self.arg(1)?;
                let p1 = self.arg(2)?;
                let p2 = self.arg(3)?;
                if parm == g::MM_PARM_MEDIA_DATA {
                    // See [`Runtime::media_set_data`] — the same rule `IMediaUtil::CreateMedia` uses
                    // when it puts the caller's data on the object it has just made.
                    self.media_set_data(this, p1, label)?;
                    // The helper writes the record; this arm works on a copy, and the tail below
                    // re-arms from it.
                    player = self.players[&this];
                } else if parm == g::MM_PARM_VOLUME {
                    player.volume = p1;
                } else if parm == g::MM_PARM_POS {
                    let mode = p1 >> 16;
                    let which = p1 & 0xffff;
                    if which > 1 {
                        self.display.note(format!(
                            "{label}: MM_PARM_POS start-stop {which} is neither start nor stop — EBADPARM"
                        ));
                        return Ok(g::AEE_EBADPARM);
                    }
                    if mode != g::MM_SEEK_MODE_TIME {
                        self.display.note(format!(
                            "{label}: MM_PARM_POS mode {mode:#x} names frames or chapters; \
                             MM_PARM_SEEK_CAPS advertises time only — EUNSUPPORTED"
                        ));
                        return Ok(g::AEE_EUNSUPPORTED);
                    }
                    let Some(length) = self
                        .player_clip(&player)
                        .and_then(|s| crate::audio::clip_length_ms(&s.bytes))
                    else {
                        let code = if player.input == 0 {
                            g::AEE_EBADSTATE
                        } else {
                            g::AEE_EUNSUPPORTED
                        };
                        self.display.note(format!(
                            "{label}: MM_PARM_POS needs a clip this host can time; no range was set"
                        ));
                        return Ok(code);
                    };
                    let raw = p2 as i32;
                    let last = if which == 0 {
                        player.play_start
                    } else {
                        player.play_stop.unwrap_or(length)
                    };
                    let value = if raw == -1 {
                        i64::from(last)
                    } else {
                        i64::from(raw)
                    };
                    if value < 0 || value > i64::from(length) {
                        self.display.note(format!(
                            "{label}: MM_PARM_POS {value} ms is outside 0..{length} ms — EBADPARM"
                        ));
                        return Ok(g::AEE_EBADPARM);
                    }
                    let value = value as u32;
                    if which == 0 {
                        player.play_start = value;
                        player.position = value;
                    } else {
                        player.play_stop = Some(value);
                        if value < player.play_start {
                            player.position = player.play_start;
                        } else {
                            player.position = player.position.min(value);
                        }
                    }
                    // The range changed; let the tail re-arm against it.
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.pass_started = None;
                } else if parm == g::MM_PARM_RATE {
                    let numerator = p1 & 0xffff;
                    let denominator = p1 >> 16;
                    if numerator == 0 {
                        self.display.note(format!(
                            "{label}: MM_PARM_RATE numerator 0 is not a playback rate — EBADPARM"
                        ));
                        return Ok(g::AEE_EBADPARM);
                    }
                    let denominator = if denominator == 0 { 1 } else { denominator };
                    let previous = player.rate_parts();
                    player.rate_num = numerator;
                    player.rate_den = denominator;
                    let rate = player.rate_parts();
                    retime_running_pass(
                        &mut player,
                        self.clock_ms,
                        self.media_tempo,
                        self.media_tempo,
                        previous,
                        rate,
                    );
                } else if parm == g::MM_PARM_TUNE {
                    // The header player's own record keeps it centred on `0x40`; both are written here so the
                    // two ABI doors cannot disagree about the pitch a voice plays at. The bound is
                    // `ISoundPlayer::SetTune`'s, because they are the same knob.
                    let step = p1 as i32;
                    if (-12..=12).contains(&step) {
                        player.tune = (0x40 + step) as u32;
                    } else {
                        tune_bad = true;
                        self.display.note(format!(
                            "{label}: MM_PARM_TUNE {step} is outside -12..12 semitones, so the \
                             pitch in force is unchanged"
                        ));
                    }
                } else if parm == g::MM_PARM_MIDI_MSG {
                    // The header the synth that consumes them belongs to the audio thread.
                    if p2 == 0 || p2 > 1_024 {
                        self.display.note(format!(
                            "{label}: MM_PARM_MIDI_MSG size {p2} is not a MIDI packet this host reads"
                        ));
                        return Ok(g::AEE_EBADPARM);
                    }
                    let mut bytes = Vec::with_capacity(p2 as usize);
                    for offset in 0..p2 {
                        bytes.push(self.cpu.mem.read_u8(p1.saturating_add(offset))?);
                    }
                    self.midi_out.entry(this).or_default().push(bytes);
                } else if parm == g::MM_PARM_TICK_TIME {
                    // 0 is a legal period meaning "no tick events"; the `DONE` remains scheduled.
                    player.tick_ms = Some(p1);
                    if player.playing && player.pass_at != 0 {
                        player.tick_at = if p1 == 0 {
                            0
                        } else {
                            self.clock_ms.saturating_add(p1)
                        };
                    }
                } else if parm == g::MM_PARM_TEMPO {
                    // One value for the whole synthesizer; see [`Runtime::media_tempo`]. The
                    // shared factor uses the range `ISoundPlayer::SetTempo` documents; a value
                    // outside it changes nothing and the call answers EBADPARM rather than
                    // claiming a set that did not happen.
                    if let Some(percent) = crate::audio::tempo_percent(p1) {
                        let previous = self.media_tempo;
                        self.media_tempo = percent;
                        self.retime_other_players(this, previous, percent);
                        let rate = player.rate_parts();
                        retime_running_pass(
                            &mut player,
                            self.clock_ms,
                            previous,
                            percent,
                            rate,
                            rate,
                        );
                    } else {
                        tempo_bad = true;
                        self.display.note(format!(
                            "{label}: tempo {p1} is outside 1-500 %, so the factor in force is \
                             unchanged"
                        ));
                    }
                } else {
                    self.display
                        .note(format!("{label}: parameter {parm:#x} has no effect here"));
                }
                if tempo_bad || tune_bad {
                    g::AEE_EBADPARM
                } else {
                    g::AEE_SUCCESS
                }
            }
            "GetMediaParm" => {
                let parm = self.arg(1)?;
                let p1 = self.arg(2)?;
                let p2 = self.arg(3)?;
                // `MM_PARM_SEEK_CAPS` is the one get that allows a null list pointer: the caller may
                // ask for the byte count first and allocate afterwards.
                if p1 == 0 && parm != g::MM_PARM_SEEK_CAPS {
                    return Ok(g::AEE_EBADPARM);
                }
                if parm == g::MM_PARM_SEEK_CAPS {
                    if p2 == 0 {
                        return Ok(g::AEE_EBADPARM);
                    }
                    // The list is one byte per supported seek mode. This host has a millisecond clock
                    // and no frame/chapter table, so time is the only truthful entry.
                    self.cpu.mem.write_u32(p2, 1)?;
                    if p1 != 0 {
                        self.cpu.mem.write_u8(p1, g::MM_SEEK_MODE_TIME as u8)?;
                    }
                } else if parm == g::MM_PARM_MEDIA_DATA {
                    let at = |field: &str| {
                        abi::offset_of("AEEMediaData", field)
                            .expect("AEEMediaData is in the tables")
                    };
                    self.cpu.mem.write_u32(p1 + at("pData"), player.data)?;
                    self.cpu.mem.write_u32(p1 + at("dwSize"), player.size)?;
                } else if parm == g::MM_PARM_CAPS {
                    // The one question in this family that a pack answers by *branching* on: the Media
                    // Player sample reads caps and tests it against `MM_CAPS_VIDEO` before it decides
                    // to show a video area (and it discards the return
                    // value, so the bits are the whole answer). This host decodes audio and nothing
                    // else, so audio is what it claims — and claiming no video is what keeps the
                    // sample off the path it only half-implements.
                    self.cpu.mem.write_u32(p1, g::MM_CAPS_AUDIO)?;
                } else if parm == g::MM_PARM_VOLUME {
                    self.cpu.mem.write_u32(p1, player.volume)?;
                } else if parm == g::MM_PARM_TEMPO {
                    self.cpu.mem.write_u32(p1, self.media_tempo)?;
                } else if parm == g::MM_PARM_TICK_TIME {
                    self.cpu
                        .mem
                        .write_u32(p1, player.tick_ms.unwrap_or(1_000))?;
                } else if parm == g::MM_PARM_TUNE {
                    // The step, not the centred value: `SetMediaParm` takes a step and the record
                    // stores `0x40 + step`, and a getter that handed the record's own number back
                    // would be answering a different question than the setter was asked.
                    let step = crate::audio::tone::tune_step(player.tune);
                    self.cpu.mem.write_u32(p1, step as u32)?;
                } else if parm == g::MM_PARM_RATE {
                    let (numerator, denominator) = player.rate_parts();
                    let packed = (numerator & 0xffff) | ((denominator & 0xffff) << 16);
                    // The header
                    if p2 != 0 {
                        self.cpu.mem.write_u32(p2, packed)?;
                    } else {
                        self.cpu.mem.write_u32(p1, packed)?;
                    }
                } else if parm == g::MM_PARM_POS {
                    // The header the word and reads the current position in milliseconds back through the same
                    // pointer. `p2`, when supplied, carries the same current position for the seek
                    // mode. The mode itself is not distinguished by this host, so no other number is
                    // claimed for it.
                    let position = self.clip_position(&player);
                    self.cpu.mem.write_u32(p1, position)?;
                    if p2 != 0 {
                        self.cpu.mem.write_u32(p2, position)?;
                    }
                } else {
                    // An unknown parameter is answered with nothing, not with whatever the last
                    // question happened to want — the shape this arm used to have — and said out loud
                    // so a caller that reads it back sees a zero with a reason next to it.
                    self.cpu.mem.write_u32(p1, 0)?;
                    self.display
                        .note(format!("{label}: no value kept for parameter {parm:#x}"));
                }
                g::AEE_SUCCESS
            }
            "Play" => {
                // Whether this call began a pass the caller wants told about, decided here and
                // delivered only after `arm_notify` below has said whether the pass can be followed to
                // its end.
                start_news = !player.playing && player.pfn != 0 && player.media != 0;
                player.playing = true;
                g::AEE_SUCCESS
            }
            // `IMedia::Record(po)` — `int Record(iname *po)` and nothing else, with
            // no page of its own; what the header does say is in the status table, where `MM_STATUS_START`
            // and the whole `MM_STATUS_*` run are annotated "**[Play/Rec]** … Playback/**Recording** has
            // started", and where two entries are annotated `[Rec]` alone: `MM_STATUS_SPACE_WARNING`
            // "Memory available to store recording running low" and `MM_STATUS_SPACE_ERROR` "Memory
            // unavailable to store recording; **recording ended**".
            //
            // So this is `Play`'s twin on the state machine the two share: it starts a pass, and it owes the
            // registered handler the same start/tick/end reports. ⚠ What it does **not** have is
            // anywhere to put the audio: this host has an output path and no capture device, so the
            // recording is started and the data goes nowhere. That is said here rather than left as a
            // surprise, and it is also why neither of the two `[Rec]` statuses can ever be posted — there is
            // no memory being filled to run low on.
            "Record" => {
                start_news = !player.playing && player.pfn != 0 && player.media != 0;
                player.playing = true;
                g::AEE_SUCCESS
            }
            // `Resume` and `Pause` are split on the header's own distinction rather than on this
            // host's: a start is a pass from the top, a resume carries the elapsed time it picked up
            // at, and this host does keep a position to come back to. ⚠ **Contract-derived, with no
            // pack record**: across the sweep logs the method-level `IMedia` lines are `Play` 65,
            // `GetState` 64, `SetMediaParm` 7, `RegisterNotify` 7, `Stop` 1, `Release` 1, and
            // `Resume`/`Pause`/`Seek` **0** each (`Hle::HISTORY` truncates a long call log, so those
            // are "what survived the window", not "never"). No pack in the sample set has ever resumed or
            // paused through this door, which is also why the frame regression is the thing that would
            // catch a mistake here rather than a test.
            "Resume" => {
                resume_news = !player.playing && player.pfn != 0 && player.media != 0;
                player.playing = true;
                g::AEE_SUCCESS
            }
            "Stop" => {
                player.playing = false;
                player.play_gen = player.play_gen.wrapping_add(1);
                // Nothing is running, so nothing owes a tick or an end, and a stop is the one call that
                // puts the clip back at the top: what the caller asked for next is the clip, not the
                // tail of it.
                player.tick_at = 0;
                player.pass_at = 0;
                player.position = 0;
                g::AEE_SUCCESS
            }
            "Pause" => {
                player.playing = false;
                // The position is written down *before* the window is torn down, because the window is
                // where it lives while running (`clip_position` derives it from `pass_at`), and after
                // the disarm there is nothing left to derive it from.
                let held = self.clip_position(&player);
                player.position = held;
                pause_news = Some(held);
                player.tick_at = 0;
                player.pass_at = 0;
                // `MM_STATUS_PAUSE` carries "Elapsed Time in MS" in the header's own table, and the
                // reference sample reads exactly that (`if (dwSize)` then `pCmdData / 1000`). This arm
                // used to answer with silence, on the reasoning that a number would be invented —
                // see the `Resume` split above for what changed. The silence is kept for the one case
                // where it is still true: nobody registered, so there is no handler to carry it.
                g::AEE_SUCCESS
            }
            // `IMedia_Seek(IMedia*, AEEMediaSeek eSeek, int32 lTimeMS)`,
            // and `lTimeMS` is milliseconds. **Signed**, which the Seek paragraph never says out loud —
            // the record is the type (`int32`) plus its sibling `IMedia_SeekFrame`, whose own parameter
            // line reads "Number of frames. Negative indicates reverse direction".
            // So the arithmetic below goes through `i64` rather than the
            // `u32` the register hands over: read as unsigned, `MM_SEEK_END, -5 000` on a 30 000 ms clip
            // is `30 000 + 4 294 962 296 = 4 294 992 296`, which the range check refuses as "past the
            // end" instead of serving as "five seconds before the end".
            //
            // The length is measured afresh here rather than taken from `pass_ms`, for the same reason
            // `IMedia::GetTotalTime` measures it: `pass_ms` is only refreshed where a pass is being
            // armed, so on an object nobody has ever played it says nothing about the clip it is
            // pointed at. (`ISoundPlayer::GetTotalTime` does read the field, and answers 0 for such an
            // object; that is the older interface's own contract, stated where it is written.)
            "Seek" => {
                let (reference, offset) = (self.arg(1)?, self.arg(2)?);
                // Only the low byte of the register is the reference: `AEEMediaSeek` is an `int8` to an
                // ARM build (where `_WIN32` gets an `unsigned` and everything
                // else does not), so what sits above it is whatever the caller's register held before.
                // Reading the whole word would refuse a legitimate `MM_SEEK_START` whose upper bits are
                // non-zero — and this file already masks for exactly this reason at `AEESoundPlayerInput`.
                let reference = reference & 0xff;
                let length = self
                    .player_clip(&player)
                    .and_then(|s| crate::audio::clip_length_ms(&s.bytes));
                // The seek *mode* bits share the word with the seek *reference*, and two of the three
                // references then read as something else: `MM_SEEK_MODE_TIME` is 0, which is also
                // `MM_SEEK_START`, while `MM_SEEK_MODE_FRAME` (0x10) and `_CHAPTER` (0x20) turn
                // `MM_SEEK_END`/`_CURRENT` into values no arm below matches. A caller asking for a
                // frame- or chapter-based seek wants a clock this host does not have, so it is refused
                // by name instead of being answered as the bare reference its low bits happen to spell.
                if reference & (g::MM_SEEK_MODE_FRAME | g::MM_SEEK_MODE_CHAPTER) != 0 {
                    self.display.note(format!(
                        "{label}: Seek asked for mode {reference:#x}, which names frames or chapters; \
                         MM_PARM_SEEK_CAPS advertises time only — EUNSUPPORTED"
                    ));
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                let Some(length) = length else {
                    // Two different reasons land on this one branch, and they get different codes
                    // because the header offers both: Idle has no state in
                    // which to seek, while an unreadable clip is `EUNSUPPORTED` — a code the header
                    // reserves for "API not supported by the class", which this host stretches to "not
                    // supported by *this clip*", because the alternative is a bare refusal with no
                    // sentence next to it saying what was tried.
                    let code = if player.input == 0 {
                        self.display.note(format!(
                            "{label}: Seek in the Idle state — no media has been named, so there is no \
                             position to move — EBADSTATE"
                        ));
                        g::AEE_EBADSTATE
                    } else {
                        self.display.note(format!(
                            "{label}: Seek cannot be answered on a clip this host cannot time, so no \
                             position is promised — EUNSUPPORTED"
                        ));
                        g::AEE_EUNSUPPORTED
                    };
                    return Ok(code);
                };
                let base: i64 = match reference {
                    g::MM_SEEK_START => 0,
                    g::MM_SEEK_END => length as i64,
                    g::MM_SEEK_CURRENT => self.clip_position(&player) as i64,
                    other => {
                        self.display.note(format!(
                            "{label}: Seek reference {other:#x} is none of MM_SEEK_START/END/CURRENT \
                             — EBADPARM"
                        ));
                        return Ok(g::AEE_EBADPARM);
                    }
                };
                let target = base + offset as i32 as i64;
                if target < 0 || target > length as i64 {
                    // Said with the three numbers, because "out of range" alone does not tell a caller
                    // whether it added the wrong amount or asked about the wrong clip.
                    self.display.note(format!(
                        "{label}: Seek would put the clip at {target} ms, outside 0..{length} ms \
                         — EBADPARM, position left alone"
                    ));
                    return Ok(g::AEE_EBADPARM);
                }
                let target = target as u32;
                player.position = target;
                // Disarm so the `arm_notify` at the tail of this function re-places the window against
                // the new position; going through that one path is what keeps a seeked pass, a resumed
                // one and a fresh one from carrying three different ideas of where the seconds start.
                player.tick_at = 0;
                player.pass_at = 0;
                // `MM_STATUS_SEEK` carries nothing in the header's own table (unlike its neighbours
                // `MM_STATUS_PAUSE`/`RESUME`, which are annotated "Elapsed Time in MS"), so `size` says
                // so rather than shipping a position the table does not ask for.
                seek_news.push((g::MM_STATUS_SEEK, 0, 0));
                // Whether the header's follow-on is owed — "`MM_STATUS_RESUME` is sent when media
                // resumes after seeking" — is decided **below**, by whether the re-arm after this
                // actually produced a pass. Asking "was it playing when the seek came in" here would be
                // the same question one step earlier and would be wrong in the one case that matters: a
                // clip this host cannot time re-arms to nothing, and a `RESUME` for a pass that will
                // never run is the stranded-caller shape the `START` above is careful not to produce.
                seek_resume = Some(target);
                g::AEE_SUCCESS
            }
            // The two interfaces are different on purpose: `IMedia::GetTotalTime` is
            // `int (IMedia*)` returning a **result code** (`spec/brew_abi.json`, the header),
            // where `ISoundPlayer::GetTotalTime` is `void (ISoundPlayer*)` with the milliseconds in r0
            // and is answered for real in `isoundplayer`.
            //
            // This arm used to answer `AEE_EUNSUPPORTED` outright, on the reasoning that the length
            // arrives through a callback this host did not implement, which is the reason `Job::Media`
            // exists. The record counts are: across the sweep logs the method-level lines are
            // `IMedia::Play` 65, `GetState` 64, `SetMediaParm` 7, `RegisterNotify` 7, `Stop` 1,
            // `Release` 1, and `GetTotalTime` **0** (`Hle::HISTORY` truncates a long call log, so those
            // are "what survived the window", not "never"). That leaves `GetTotalTime`, `Pause`,
            // `Resume` and `Seek` at zero: nobody in the sample set has ever asked this
            // question — which means the answer below is **contract-derived, with no pack record at
            // all**, and it is the reason `IMedia::RegisterNotify`'s seven hits are load-bearing:
            // those packs did register, and this host calls them back.
            //
            // So the answer is now decided by the three things the contract requires rather than by a
            // blanket refusal: the header returns a result code with the value
            // "returned in the registered callback", says it "only works in the READY state", and lists
            // `EBADSTATE` and `EUNSUPPORTED` among its answers. A success this host cannot follow up is
            // still the one answer that must not be given, so each missing precondition is named.
            "GetTotalTime" => {
                let timed = self
                    .player_clip(&player)
                    .and_then(|s| crate::audio::clip_length_ms(&s.bytes));
                if player.input == 0 {
                    // Idle: no media data has been set, so there is no READY state to ask in.
                    self.display.note(format!(
                        "{label}: GetTotalTime in the Idle state — nothing has been named to measure"
                    ));
                    g::AEE_EBADSTATE
                } else if player.playing {
                    self.display.note(format!(
                        "{label}: GetTotalTime asks in the Play state; the header says it only works \
                         in READY"
                    ));
                    g::AEE_EBADSTATE
                } else if player.pfn == 0 {
                    self.display.note(format!(
                        "{label}: the length would come back in a callback nobody registered, so it \
                         is not promised"
                    ));
                    g::AEE_EUNSUPPORTED
                } else {
                    match timed {
                        Some(ms) => {
                            // The milliseconds travel **in** `pCmdData`, not behind it — that is how
                            // the reference sample reads it, and the header's
                            // own table calls these entries "values" while it says "points to this
                            // structure" for the one case that really is a pointer (`AEEMediaAudioSync`).
                            // ⚠ This is the one place in the channel where the two readings cannot both
                            // be honoured: a pack that dereferenced the field would read guest memory
                            // at an address equal to the length. The sample is the record this host
                            // follows, and the frame regression is what would catch the other shape.
                            self.queue.push_back(Job::Media {
                                player: this,
                                cmd: g::MM_CMD_GETTOTALTIME,
                                status: g::MM_STATUS_DONE,
                                data: ms,
                                size: 4,
                            });
                            g::AEE_SUCCESS
                        }
                        None => {
                            self.display.note(format!(
                                "{label}: this host cannot time a clip it cannot parse, so it says so"
                            ));
                            g::AEE_EUNSUPPORTED
                        }
                    }
                }
            }
            // `int GetState(iname *po, boolean *pbStateChanging)` — the header, and the
            // second word of the prototype is the whole of this arm. It is a **`boolean`**, one
            // byte, not the `AEEMediaState` a reader of the name would guess; the state itself is
            // the *return* value ("Return Value: MM_STATE_XXX",), and the flag only
            // says whether a transition is in progress.
            //
            // Both halves were wrong here — a four-byte store of `0` and a return of `AEE_SUCCESS`
            // — and the width is the half that bites. A pack's flag is one byte of its own frame,
            // so a four-byte store runs three bytes past it: `Asphalt 3D 2` keeps it at `fp-0x19`,
            // four-byte store lands on the low half of its saved `r4` and turns `0x20087734` into
            // `0x20000000`, and four instructions later the pack does `ldr r0,[r4,#4]` and takes a
            // data abort at zero. That is a crash this host caused in a caller that checked
            // nothing, from a call that read as a success.
            "GetState" => {
                let pstate = self.arg(1)?;
                if pstate != 0 {
                    // FALSE: this host moves between states inside the call that changes them, so
                    // there is no moment in which a caller could observe a transition.
                    self.cpu.mem.write_u8(pstate, 0)?;
                }
                if player.playing {
                    g::MM_STATE_PLAY
                } else if player.input != 0 {
                    g::MM_STATE_READY
                } else {
                    g::MM_STATE_IDLE
                }
            }
            _ => {
                return Err(HleError::Unimplemented {
                    id,
                    label: label.to_string(),
                    this,
                })
            }
        };
        self.players.insert(this, player);
        // The same rule the sound player arms by, from the same place: the bytes to time only exist
        // once the naming above has been resolved, and a clip re-pointed mid-run has to be timed from
        // the new one. `arm_notify` is shape-agnostic — the schedule is identical for both interfaces,
        // and only the *shape* of what the pump enqueues differs (see its `sounding` loop).
        self.arm_notify(this);
        // `MM_STATUS_START` is "[Play/Rec] Playback/Recording has started" and the reference sample
        // branches on it to move its own menu selection, so a play that never says it began leaves the
        // caller waiting for news only this host can give. It is told **only for a pass the arming
        // above accepted**: a clip this host cannot parse gets no start either, because a start is the
        // first chapter of a story that ends in a `DONE`, and opening a pass whose ending the host
        // cannot see is how a pack sits in its "now playing" state for ever. Silence about a clip it
        // cannot follow is the same rule `arm_notify` already states for the end of a pass.
        //
        // A pack that registers *after* `Play` therefore never hears the start of that pass — a start is
        // a moment, not a state, and inventing one when the callback appears would report something the
        // host did not know at the time. It does hear the ticks and the end, because this tail call runs
        // on every method and `arm_notify` needs the registration to exist before it has anyone to tell;
        // the window then runs from the registration, which is also what the `ISoundPlayer` path has
        // always done. Timing a pass from the first moment anyone could be told keeps one number for
        // both doors, where arming it at `Play` and delivering from `RegisterNotify` would need a
        // backlog of ticks to be replayed from the elapsed seconds.
        if start_news && self.players[&this].pass_at != 0 {
            self.queue.push_back(Job::Media {
                player: this,
                cmd: g::MM_CMD_PLAY,
                status: g::MM_STATUS_START,
                data: 0,
                size: 0,
            });
        }
        // A `Play` the arming above declined because the clip has no run left in it — a seek to its own
        // end, then a play — is the one success-and-silence this host must not hand back, because the
        // caller is waiting for a `START` it will never get. Silence is only honest when the reason is
        // said, so it is said here rather than left to the absence of a callback.
        let rearmed = self.players[&this];
        if start_news
            && rearmed.pass_at == 0
            && rearmed.pass_ms != 0
            && rearmed.position >= rearmed.pass_ms
        {
            self.display.note(format!(
                "{label}: Play starts at {} ms of a {} ms clip, so there is nothing left to run and \
                 this host says so instead of starting a pass of no length",
                rearmed.position, rearmed.pass_ms
            ));
        }
        // What the seek did, in the order the header gives it: the `SEEK`, then the `RESUME` it promises
        // "when media resumes after seeking". The second is posted only when the window below really
        // re-armed, because a resumed pass that never arrived is the same lie as a start with no end.
        // Both are gated on the object having been handed out as an `IMedia` and having a handler, for
        // the reason `start_news` is: `Job::Media` is the two-register shape, and an address that came
        // from `ISoundPlayer` expects four (`player.media` is 0 there).
        if player.media != 0 && player.pfn != 0 {
            for (status, data, size) in seek_news {
                self.queue.push_back(Job::Media {
                    player: this,
                    cmd: g::MM_CMD_PLAY,
                    status,
                    data,
                    size,
                });
            }
            if let Some(at) = seek_resume.filter(|_| rearmed.pass_at != 0) {
                self.queue.push_back(Job::Media {
                    player: this,
                    cmd: g::MM_CMD_PLAY,
                    status: g::MM_STATUS_RESUME,
                    data: at,
                    size: 4,
                });
            }
        }
        // A pause's own news. Posted whatever the handler does with it, since unlike `DONE` a pause
        // report cannot strand a caller: it is told that nothing further is coming, which is what just
        // happened. The elapsed is the position the pause captured above, in ms, as the table asks.
        if player.media != 0 && player.pfn != 0 {
            if let Some(elapsed) = pause_news {
                // The elapsed time is only a **measurement** where the clip has a measurable length.
                // On one this host cannot time, `clip_position` falls back to the stored place, which is
                // 0 because nothing ever moved it — and shipping that 0 with `dwSize: 4` would tell the
                // sample to divide a number it was never given by 1 000 and draw it as a position. So
                // the report is made, and says it carries nothing, exactly as the end of a pass does.
                let (data, size) = if rearmed.pass_ms == 0 {
                    (0, 0)
                } else {
                    (elapsed, 4)
                };
                self.queue.push_back(Job::Media {
                    player: this,
                    cmd: g::MM_CMD_PLAY,
                    status: g::MM_STATUS_PAUSE,
                    data,
                    size,
                });
            }
            // A resume is announced only for a pass that really re-armed, which is the same guard the
            // `START` above and the seek's `RESUME` above carry: `MM_STATUS_RESUME` reads to a pack as
            // "seconds are now advancing", and on a clip with no window there are no seconds.
            if resume_news && rearmed.pass_at != 0 {
                self.queue.push_back(Job::Media {
                    player: this,
                    cmd: g::MM_CMD_PLAY,
                    status: g::MM_STATUS_RESUME,
                    data: self.clip_position(&rearmed),
                    size: 4,
                });
            }
        }
        Ok(retval)
    }

    /// The `IMemAStream` slots: a chunk of the guest's own memory, read back as a stream.
    ///
    /// `Set` and `SetEx` name the chunk — and `SetEx` names a callback to run when the stream is
    /// finished with it, which this host keeps and never calls, because it never decides when the
    /// app is done with its own heap. `Read` answers a count and stops at the end, which is what
    /// `IASTREAM_Read` says a stream at its end answers. `Readable` and `Cancel` have nothing to do:
    /// every byte is already in memory, so there is no arrival to wait for and no callback to call
    /// off — the same answer `IUnzipAStream` gives them, for the same reason.
    pub(crate) fn imemastream(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        if !self.memstreams.contains_key(&this) {
            return Err(HleError::NotAnObject { addr: this }.into());
        }
        // `Readable` and `Cancel` are `void` in `IAStream`; a status in r0 is not an error the
        // caller can see. Both are answered below rather than raised: a refusal here used to end
        // the whole run for a call the interface says returns nothing.
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let retval = match name {
            "Set" | "SetEx" => {
                let (buf, size, offset) = (self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let (free_fn, free_user, plain) = if name == "SetEx" {
                    (self.arg(4)?, self.arg(5)?, false)
                } else {
                    (0, buf, true)
                };
                // `CMemStream_Set` gives back what the stream held **before** it looks at the new
                // buffer, then installs the new owner: `Set` installs `aee_free` and the buffer as
                // the context, `SetEx` the caller's pair. A call that names no buffer still runs
                // the freshly installed owner over it — `aee_free(0)` for the plain form.
                let old = self.memstreams.get(&this).copied().unwrap_or_default();
                self.release_memstream(old)?;
                let mut stream = MemStream {
                    free_fn,
                    free_user,
                    plain,
                    ..Default::default()
                };
                if buf != 0 && size != 0 {
                    stream.buf = buf;
                    stream.size = size;
                    stream.pos = offset;
                    self.memstreams.insert(this, stream);
                } else {
                    self.memstreams.insert(this, stream);
                    let fresh = self.memstreams[&this];
                    self.release_memstream(fresh)?;
                    // The body leaves the owner installed here, so a release would run it a second
                    // time over bytes it already gave back. This host clears it instead: one free,
                    // which is what the caller asked for.
                    self.memstreams.insert(this, MemStream::default());
                }
                0
            }
            "Read" => {
                let [dst, want] = self.args()?;
                let stream = self.memstreams[&this];
                let start = (stream.pos as usize).min(stream.size as usize);
                let stop = start
                    .saturating_add(want as usize)
                    .min(stream.size as usize);
                let n = stop - start;
                if dst == 0 || n == 0 {
                    return Ok(0);
                }
                let bytes = self.cpu.mem.read_bytes(stream.buf + start as u32, n)?;
                self.cpu.mem.write_bytes(dst, &bytes)?;
                let Some(stream) = self.memstreams.get_mut(&this) else {
                    return Ok(0);
                };
                stream.pos += n as u32;
                n as u32
            }
            "Readable" => {
                // `CMemStream_Readable(pfn, pUser)` stores the pair and registers it with the
                // object's source (`+64`) through that object's slot 36. This host hands out no
                // source that fills a stream asynchronously — a memory stream is always readable —
                // so there is nobody to register with and nothing to raise; the pair is kept so
                // that the call survives and `Cancel` has something to clear.
                let [pfn, user] = self.args()?;
                let mut stream = self.memstreams[&this];
                stream.notify = pfn;
                stream.notify_user = user;
                self.memstreams.insert(this, stream);
                0
            }
            "Cancel" => {
                // `CMemStream_Cancel` clears the same pair, then calls the stream's own cancel
                // callback (`+44`). This host has none to call, and clearing is the whole effect
                // the caller can observe.
                let mut stream = self.memstreams[&this];
                stream.notify = 0;
                stream.notify_user = 0;
                self.memstreams.insert(this, stream);
                0
            }
            _ => return Err(unimplemented().into()),
        };
        Ok(retval)
    }

    pub(crate) fn isoundplayer(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let mut player = self.players.get(&this).copied().unwrap_or_default();
        if !player.noted {
            player.noted = true;
            self.players.insert(this, player);
            self.display.note(format!(
                "{label}: the only sound this host makes is MIDI, rendered through its own built-in \
                 tone tables unless a SoundFont was given"
            ));
        }
        // What this call owes the registered handler, as `(cmd, status, data, size)`. Collected by
        // the arms and delivered after the state work below, because the delivered bodies call the
        // handler from inside the slot — after the state it describes and before the slot returns.
        let mut reports: Vec<(u32, u32, u32, u32)> = Vec::new();
        let retval = match name {
            "RegisterNotify" => {
                let [pfn, user] = self.args()?;
                player.pfn = pfn;
                player.user = user;
                0
            }
            // The older half of the same request `SetInfo` makes: `t` says which of `SDT_FILE`,
            // `SDT_BUFFER` or `SDT_VOICEPROMPT` the pointer is, and there is no length in it. The
            // delivered body, in its own order:
            //
            // 1. `if ( pme->m_bPlaying )` — the playing flag **alone**, not `IsBusy`, which also
            //    counts the streaming half this host has no counterpart for. A `Set` that lands on a
            //    running clip is a *queued* re-pointing: the new naming is recorded, the old clip
            //    keeps playing to its end, and the pack's next `Play` swaps the two. `SetInfo`
            //    answers `AEE_EITEMBUSY` in the same position instead.
            // 2. On that path the body makes the pending media first — `NewMedia(&{t, pData, 16})` —
            //    and reports a NULL one *before* it touches anything, so a refusal leaves the
            //    record, the owned buffers and the window exactly as they were.
            // 3. `AEESoundPlayer_FreeData` then runs on both paths, releasing the previous buffer
            //    (`Set(SDT_BUFFER, …)` takes ownership of it), the previous file name and the
            //    previous media.
            // 4. The tail installs the record — a buffer keeps the caller's pointer and gets the
            //    pk's 16-byte length, a file name is copied and its length written 0 — and calls
            //    `IntSetInfo`, whose answer becomes one `STATUS_CB` report.
            //
            // What is not reproduced is `NewMedia`'s own verdict past its unconditional line: the
            // media class is chosen through the device's handler table and `ISHELL_DetectType`, and
            // this host models neither in the same terms. Only `AEEMediaUtil_CreateMedia`'s
            // `if ( !pmd->pData ) return EBADPARM;` is kept, plus the fact that a blank name cannot
            // open. A named file the volume does not have, or a buffer whose first bytes are not a
            // format the device knew, is taken here as media the device would have made.
            //
            // The second-`Play` half of the machine (`if ( m_bDataSet && !m_bPlayIssued )` →
            // `Stop`) lives in the `Play` arm, because that is the slot that consumes it.
            "Set" => {
                let [input, data] = self.args()?;
                // `switch ( t )` on an `int8`: 0x80..0xFF is negative and lands on the default.
                let kind = (input as u8 as i8) as i32;
                let mut refused = false;
                if player.playing {
                    let named = match kind {
                        k if k == g::SDT_FILE as i32 => {
                            matches!(self.cstring(data), Ok(ref name) if !name.is_empty())
                        }
                        k if k == g::SDT_BUFFER as i32 => data != 0,
                        _ => false,
                    };
                    if named {
                        // `m_bDataSet = TRUE`. The window is the old clip's and is left alone,
                        // which is the whole point: a pack waiting for that clip's `DONE` still
                        // gets it.
                        player.pending = true;
                    } else {
                        refused = true;
                    }
                } else {
                    // `m_bDataSet = FALSE`: nothing is playing, so the new media replaces the old
                    // one outright and starts at its beginning.
                    player.pending = false;
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.pass_started = None;
                    player.position = 0;
                }
                if refused {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                } else {
                    self.release_player_data(&mut player);
                    let installed = match kind {
                        k if k == g::SDT_FILE as i32 => {
                            self.install_file_name(&mut player, data)?
                        }
                        k if k == g::SDT_BUFFER as i32 => {
                            // `IntSetInfo`'s `if ( !pInfo->pData ) return EBADPARM;` stands above
                            // its switch, so no kind can name nothing — `Set`'s buffer half included.
                            if data == 0 {
                                false
                            } else {
                                player.raw_buffer = data;
                                player.input = g::SDT_BUFFER;
                                player.data = data;
                                player.size = Self::SET_BUFFER_SIZE;
                                true
                            }
                        }
                        // `SDT_VOICEPROMPT` is `AEE_EUNSUPPORTED` and anything else is
                        // `AEE_EBADPARM`; both become the same `FAILURE` report, and both leave
                        // the record empty because `FreeData` has already run.
                        _ => false,
                    };
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        if installed {
                            g::AEE_SOUNDPLAYER_SUCCESS
                        } else {
                            g::AEE_SOUNDPLAYER_FAILURE
                        },
                        0,
                        0,
                    ));
                }
                0
            }
            "Play" => {
                // The tempo is deliberately not reset here. The older interface's description says a
                // player's factor starts at 100 % when playback begins, but the factor this host
                // plays is the synthesizer's and is shared: resetting it on one object's `Play`
                // would change every other stream in flight. `MM_PARM_TEMPO` is a get/set parameter
                // with no such reset, and that is the reading followed here.
                let mut refused_stop = false;
                // The swap path's own `Play` report is the queued one; the body's branch returns
                // right after the internal `Stop` and never reaches the ordinary accept answer.
                let mut swapped = false;
                if player.pending {
                    // The reference second-`Play` machine: `if ( m_bDataSet && !m_bPlayIssued ) {
                    // m_bDataSet = FALSE; m_bPlayIssued = TRUE; Stop; return; }`, and when that
                    // stop completes `MediaNotify` swaps `m_pMedia = m_pMediaPending` and calls
                    // `Play` again. What the pack hears is the interrupted clip finishing — a
                    // `PLAY_CB` `DONE` — and then the new clip's own news, so the old window ends
                    // here and the tail below arms the naming the `Set` recorded.
                    //
                    // The swap rides on a stop the body only issues while `m_bPlaying` is set, so a
                    // paused player's `Play` hears the refused stop and nothing starts. The
                    // `m_bPlayIssued` latch itself is not kept: its other two consequences — a
                    // `Set` refusing while a swap is in flight, and a swap that waits for the next
                    // clip to end — need a second naming this host does not hold.
                    player.pending = false;
                    if player.playing {
                        swapped = true;
                        player.tick_at = 0;
                        player.pass_at = 0;
                        player.pass_started = None;
                        player.position = 0;
                        // The two reports the swap's completion carries, in the order `MediaNotify`
                        // makes them: the internal `Play` reports its command accepted, and then the
                        // interrupted clip's `DONE` reaches the pack. Both ride the queue, because
                        // they belong to the device's frame rather than to this slot's.
                        if player.pfn != 0 {
                            self.queue.push_back(Job::Sound {
                                player: this,
                                cmd: g::AEE_SOUNDPLAYER_PLAY_CB,
                                status: g::AEE_SOUNDPLAYER_SUCCESS,
                                data: 0,
                                size: 0,
                            });
                            self.queue.push_back(Job::Sound {
                                player: this,
                                cmd: g::AEE_SOUNDPLAYER_PLAY_CB,
                                status: g::AEE_SOUNDPLAYER_DONE,
                                data: 0,
                                size: 0,
                            });
                        }
                        // And the internal `Stop`'s own answer, which the body delivers in the
                        // caller's frame.
                        reports.push((
                            g::AEE_SOUNDPLAYER_STATUS_CB,
                            g::AEE_SOUNDPLAYER_SUCCESS,
                            0,
                            0,
                        ));
                    } else {
                        refused_stop = true;
                        reports.push((
                            g::AEE_SOUNDPLAYER_STATUS_CB,
                            g::AEE_SOUNDPLAYER_FAILURE,
                            0,
                            0,
                        ));
                    }
                }
                if !refused_stop && !swapped {
                    // `AEESoundPlayer_Play`'s shared gate — no media, nothing loaded, or the
                    // streaming bit — answers `PLAY_CB` failure, and the accept path answers
                    // `PLAY_CB` success in the caller's own frame.
                    if player.input != 0 && player.data != 0 {
                        player.playing = true;
                        reports.push((
                            g::AEE_SOUNDPLAYER_PLAY_CB,
                            g::AEE_SOUNDPLAYER_SUCCESS,
                            0,
                            0,
                        ));
                    } else {
                        reports.push((
                            g::AEE_SOUNDPLAYER_PLAY_CB,
                            g::AEE_SOUNDPLAYER_FAILURE,
                            0,
                            0,
                        ));
                    }
                }
                0
            }
            // `Resume` is the control slot, not a second `Play`: the reference `Resume` has no
            // second-play branch of its own, so a naming queued during the clip is left for `Play`
            // to consume.
            "Resume" => {
                if player.input != 0 && player.data != 0 {
                    player.playing = true;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "Stop" => {
                // `AEESoundPlayer_CanPlayCtlCmd` — `m_pMedia && m_bPlaying && !m_bStreaming` — is
                // this slot's first branch: a stop that cannot be issued is **reported** and
                // changes nothing. `m_pMedia`'s counterpart here is a naming that resolved to
                // something (the media class itself is not modelled).
                if player.playing && player.input != 0 && player.data != 0 {
                    // The reference stop completion is what clears `m_bDataSet`, so a naming queued by
                    // a `Set` during the clip is dropped rather than carried into the next `Play`.
                    player.pending = false;
                    player.playing = false;
                    player.play_gen = player.play_gen.wrapping_add(1);
                    // Nothing is running, so nothing owes a `DONE`. A pack that stops a clip and then
                    // plays it again re-arms from the new start.
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.position = 0;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            // Split from `Stop` for the same reason `IMedia` splits them: a pause is not a rewind. This
            // door has no way to name a place of its own, so in the ordinary case the field is already
            // 0 and nothing changes — but an object that was handed an `IMedia::Seek` first has one, and
            // `Play` after a `Pause` should carry on rather than start over.
            "Pause" => {
                if player.input != 0 && player.data != 0 {
                    // A pause is not a rewind: keep the media position the live window had, scaled by
                    // the shared tempo, so a later `Play`/`Resume` starts from the right place.
                    let held = self.clip_position(&player);
                    player.playing = false;
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.position = held;
                    player.pass_started = None;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    // `AEESoundPlayer_Pause`'s shared gate; nothing moves when it refuses.
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "Rewind" => {
                let [time] = self.args()?;
                if player.input != 0 && player.data != 0 {
                    player.position = player.position.saturating_sub(time);
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.pass_started = None;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "FastForward" => {
                let [time] = self.args()?;
                if player.input != 0 && player.data != 0 {
                    let limit = if player.pass_ms != 0 {
                        player.pass_ms
                    } else {
                        u32::MAX
                    };
                    player.position = player.position.saturating_add(time).min(limit);
                    player.tick_at = 0;
                    player.pass_at = 0;
                    player.pass_started = None;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "SetTune" => {
                let [step] = self.args()?;
                let step = (step as u8 as i8) as i32;
                // `AEESoundPlayer_SetTune`'s shared gate runs before its own range test, and both
                // refusals carry the same `STATUS_CB` failure.
                if player.input == 0 || player.data == 0 || !(-12..=12).contains(&step) {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                } else {
                    player.tune = (0x40 + step) as u32;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                }
                0
            }
            "SetSoundDevice" => {
                let [device, ear, mic] = self.args()?;
                player.sound_device = device;
                player.ear_mute = ear;
                player.mic_mute = mic;
                reports.push((
                    g::AEE_SOUNDPLAYER_SOUND_CB,
                    g::AEE_SOUNDPLAYER_SUCCESS,
                    0,
                    0,
                ));
                0
            }
            "SetStream" => {
                let [stream] = self.args()?;
                player.stream = stream;
                0
            }
            "SetTempo" => {
                let [tempo] = self.args()?;
                if player.input == 0 || player.data == 0 {
                    // The shared gate, ahead of the value: `AEESoundPlayer_SetTempo` has no range
                    // test of its own (the 1-500 % rule below comes from `MM_PARM_TEMPO`).
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                } else if let Some(percent) = crate::audio::tempo_percent(tempo) {
                    let previous = self.media_tempo;
                    self.media_tempo = percent;
                    // The factor is shared; every other running object keeps its media position
                    // and gets a new wall-clock deadline too.
                    self.retime_other_players(this, previous, percent);
                    let rate = player.rate_parts();
                    retime_running_pass(&mut player, self.clock_ms, previous, percent, rate, rate);
                    // The header's side-effect table names SUCCESS as the result of an accepted
                    // set; TEMPO is the additional playback callback, and only when a pass is
                    // active. Both travel in the caller's frame, in that order.
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                    if player.playing {
                        reports.push((
                            g::AEE_SOUNDPLAYER_STATUS_CB,
                            g::AEE_SOUNDPLAYER_TEMPO,
                            percent,
                            4,
                        ));
                    }
                } else {
                    self.display.note(format!(
                        "{label}: tempo {tempo} is outside 1-500 %, so the factor in force is \
                         unchanged"
                    ));
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "SetVolume" => {
                let [volume] = self.args()?;
                // `AEESoundPlayer_SetVolume` has no shared gate: it asks the device and reports the
                // answer, and a null media is that same failure path (the body's non-null marker is
                // a function address, not the caller's value). A player with no naming is the case
                // this host can tell apart, so that is where the failure is answered.
                if player.input != 0 && player.data != 0 {
                    player.volume = volume;
                    player.volume_set = true;
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        0,
                        0,
                    ));
                } else {
                    reports.push((
                        g::AEE_SOUNDPLAYER_STATUS_CB,
                        g::AEE_SOUNDPLAYER_FAILURE,
                        0,
                        0,
                    ));
                }
                0
            }
            "GetVolume" => {
                reports.push((
                    g::AEE_SOUNDPLAYER_VOLUME_CB,
                    g::AEE_SOUNDPLAYER_SUCCESS,
                    player.volume,
                    2,
                ));
                0
            }
            // The clip's length, which this host does know: it parses the MIDI to speak it, and the
            // same parse times it. Zero is the answer for a clip it could not name a length for,
            // which is what an unstarted or untimable source is.
            //
            // `AEESoundPlayer_GetTotalTime` is `void`, and the length travels on
            // `AEE_SOUNDPLAYER_TIME_CB` — "Get time callback" — which this host had never raised
            // anywhere: a pack that asked for the length and waited for the number had nothing to
            // wait for. The reference slot raises two reports, because its device answers late:
            // `AEESoundPlayer_NotifyCmdSuccess(a1, 2)` with a null `dwParam` for the accepted query,
            // then the device's own notify with the number. This host knows the number at the moment
            // it is asked, so it raises the one report that carries it, on the same channel and with
            // the same status code. ⚠ The `r0` this slot returns is the other divergence: the reference
            // slot leaves there the callback's own result, and a pack compiled against `void` never
            // reads it — this host still answers the number, because its own tests and report read it.
            "GetTotalTime" => {
                if player.media == 0 {
                    reports.push((
                        g::AEE_SOUNDPLAYER_TIME_CB,
                        g::AEE_SOUNDPLAYER_SUCCESS,
                        player.pass_ms,
                        4,
                    ));
                }
                player.pass_ms
            }
            "SetInfo" => {
                let [pinfo] = self.args()?;
                if player.is_busy() {
                    return Ok(g::AEE_EITEMBUSY);
                }
                self.release_player_data(&mut player);
                // Step 2 has already spent its effect, so the record goes back before anything can
                // refuse; the naming cannot be left pointing at a block this host has handed back.
                self.players.insert(this, player);
                if pinfo == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                // `AEESoundPlayer_IntSetInfo`. Its first statement reads `pData`, and a null one is
                // refused *before* the input kind is looked at — so `SDT_VOICEPROMPT` with no pointer
                // answers `AEE_EBADPARM`, not `AEE_EUNSUPPORTED`.
                let data = self.cpu.mem.read_u32(pinfo + 4)?;
                if data == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // `switch ( (signed byte)pinfo[0] )`, so 0x80..0xFF is negative and falls to the
                // `default` arm with everything else that is not one of the three named kinds.
                let kind = (self.cpu.mem.read_u8(pinfo)? as i8) as i32;
                let size = self.cpu.mem.read_u32(pinfo + 8)?;
                if kind == g::SDT_VOICEPROMPT as i32 {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                if kind != g::SDT_FILE as i32 && kind != g::SDT_BUFFER as i32 {
                    return Ok(g::AEE_EBADPARM);
                }
                if kind == g::SDT_FILE as i32 {
                    // `case SDT_FILE:` opens with a comparison the delivered object could not be
                    // read for — its literal-pool word carries a relocation onto that object's own
                    // vtable, not onto a string — but the published description spells the operand out:
                    // `if ( STRCMP((char *)pInfo->pData, "") == 0 ) return EBADPARM;`. So a blank
                    // name is a refusal, and it is the first statement of the branch.
                    //
                    // The rest is `record[+32] = STRDUP(pData); record[+36] = 0; record[+28] =
                    // eInput;`: the name is **copied**, and the length is **zeroed** whatever the
                    // caller wrote into `dwSize`, because a file name has no length to report. The
                    // record is what `GetInfo` reports, so the address that comes back is the copy's,
                    // and `FreeData` is where the copy goes back.
                    if !self.install_file_name(&mut player, data)? {
                        return Ok(g::AEE_EBADPARM);
                    }
                } else {
                    // `SDT_BUFFER` is the one kind that must have a length: `if ( pinfo->dwSize == 0 )
                    // return 14;`, then `aee_memmove(record + 28, pinfo, 12)` stores the caller's
                    // struct verbatim — including the three padding bytes and a length of the
                    // caller's own.
                    if size == 0 {
                        return Ok(g::AEE_EBADPARM);
                    }
                    player.input = kind as u32;
                    player.data = data;
                    player.size = size;
                }
                // The busy gate above is why this arm may clear the window at all: nothing was in
                // flight when it got here, so `tick_at`/`pass_at` are already empty. A re-pointed
                // source also starts at its beginning, which is what the position reset says.
                player.tick_at = 0;
                player.pass_at = 0;
                player.position = 0;
                // The reference tail — `if ( (record[+80] & 2) == 0 ) record[+20] =
                // AEESoundPlayer_NewMedia(record, record + 28);` — is the step that turns the naming
                // into a media object, and on this host that is `note_audio_source` plus the
                // arming in [`Runtime::arm_notify`], both run below for every call.
                0
            }
            "GetInfo" => {
                // `AEESoundPlayer_GetInfo` has two different jobs, told apart by the pointer, and the
                // null one is not a parameter error.
                //
                // Non-null is the read of the naming `Set`/`SetInfo` installed:
                // `aee_memmove(a2, a1 + 28, 12); return 0;` — the whole twelve-byte
                // `AEESoundPlayerInfo` and nothing else.
                //
                // NULL is a **status query**: the same gate `SetInfo` opens with answers
                // `AEE_EITEMBUSY`, and a player with nothing in flight answers `AEE_EIDLE`. This
                // host used to answer `AEE_EBADPARM` for both, which is a code the documented behaviour never
                // returns from this slot.
                let [pinfo] = self.args()?;
                if pinfo == 0 {
                    return Ok(if player.is_busy() {
                        g::AEE_EITEMBUSY
                    } else {
                        g::AEE_EIDLE
                    });
                }
                self.cpu.mem.write_u8(pinfo, player.input as u8)?;
                self.cpu.mem.write_u32(pinfo + 4, player.data)?;
                self.cpu.mem.write_u32(pinfo + 8, player.size)?;
                0
            }
            _ => {
                return Err(Error::Hle(HleError::Unimplemented {
                    id,
                    label: label.to_string(),
                    this,
                }))
            }
        };
        self.players.insert(this, player);
        self.note_audio_source(&player);
        // Arming goes here rather than inside the `Play` arm, because the bytes to time only exist
        // once the naming above has been resolved — and a pack that re-points a playing voice
        // (`Stop`, `Set`, `Play` in one turn, which is how a pack switches to its jingle) has to be
        // timed from the new clip, not the one it started with.
        self.arm_notify(this);
        // Every call, not only the ones that change something: the question is what the pack *asked*
        // when its music did not come back, and a getter it read is as much an answer to that as a
        // `Set` it did not make. Repeated identical lines collapse in `log_sound`, so a pack that
        // polls `GetVolume` every frame adds one line rather than thousands.
        if let Some(player) = self.players.get(&this).copied() {
            self.log_sound(name, this, &player);
        }
        // The command's report goes out **last** and **inside this call**, which is where the
        // delivered bodies put it: after the state the report describes, and before the slot
        // returns. Nothing writes the record again after this, so a handler that called the player
        // back from inside its own notification keeps what it wrote.
        for (cmd, status, data, size) in reports {
            self.sound_report(this, cmd, status, data, size)?;
        }
        Ok(retval)
    }

    /// One command-triggered `ISoundPlayer` report, delivered the way the delivered bodies deliver
    /// it — `pfn(user, cmd, status, dwParam)` called in the pack's own frame, not queued.
    ///
    /// Device-originated reports (`TICK_UPDATE`, `DONE`) are the other kind and stay in [`Job::Sound`]
    /// where the pump plays the device's part. The record is read here and re-read afterwards, and
    /// only the field this frame owns — where the command-data block lives — is written back,
    /// because the handler may have called the player from inside its own notification.
    pub(crate) fn sound_report(
        &mut self,
        this: u32,
        cmd: u32,
        status: u32,
        data: u32,
        size: u32,
    ) -> Result<(), Error> {
        let Some(player) = self.players.get(&this).copied() else {
            return Ok(());
        };
        // The ringer manager's own player answers to a host callback rather than to a guest one; see
        // [`Player::host_ringer`]. Everything the reference `CRingerMgr_SoundStatus` does with a report it
        // does not act on is: nothing.
        if player.host_ringer != 0 {
            return self.ringer_sound_status(cmd, status);
        }
        if player.pfn == 0 {
            return Ok(());
        }
        let mut block = player.cmddata;
        let param = self.cmddata(&mut block, data, size, "AEESoundPlayerCmdData")?;
        self.call_guest(player.pfn, &[player.user, cmd, status, param])?;
        self.notified.0 += 1;
        if let Some(mut now) = self.players.get(&this).copied() {
            now.cmddata = block;
            self.players.insert(this, now);
        }
        Ok(())
    }

    /// Re-place every running pass except the one whose local copy the caller is editing.
    ///
    /// The tempo is one shared value; a change through one object moves every voice, so every
    /// armed window needs the same media-position-preserving retime.
    pub(crate) fn retime_other_players(&mut self, except: u32, old_tempo: u32, new_tempo: u32) {
        let now = self.clock_ms;
        for (address, player) in self.players.iter_mut() {
            if *address != except {
                let rate = player.rate_parts();
                retime_running_pass(player, now, old_tempo, new_tempo, rate, rate);
            }
        }
    }

    /// How many `ISoundPlayer` calls the sequence below keeps.
    pub(crate) const SOUND_LOG: usize = 512;

    /// What each `ISoundPlayer` the run holds is pointed at, for the control interface's `/status`.
    ///
    /// The report summarises the *clips* a run named; this is the other half of the question a person
    /// asks when the music is wrong, which is which object is playing which clip *now*.
    pub fn audio_state(&self) -> Vec<(u32, String, bool)> {
        let mut out: Vec<(u32, String, bool)> = self
            .players
            .iter()
            .filter(|(_, p)| p.input != 0)
            .map(|(addr, p)| {
                let clip = self
                    .player_clip(p)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "-".to_string());
                (*addr, clip, p.playing)
            })
            .collect();
        out.sort();
        out
    }

    /// The source a player named, found the same way [`crate::audio`] finds it: the three fields
    /// `Set`/`SetInfo` stored are the whole of the naming.
    ///
    /// A file name is the exception the record itself creates: the body stores a **copy**, so the
    /// same name arrives at a new address on every naming and the address half of the key matches
    /// only the latest one. The name is what identifies the clip; the buffer half keeps the address,
    /// which is the caller's own and stays put.
    pub(crate) fn player_clip(&self, player: &Player) -> Option<&AudioSource> {
        if player.input == g::SDT_FILE {
            let name = self.cstring(player.data).ok()?;
            return self
                .audio
                .iter()
                .find(|s| s.input == g::SDT_FILE && s.name == name);
        }
        self.audio
            .iter()
            .find(|s| s.input == player.input && s.data == player.data && s.size == player.size)
    }

    /// Call the status function an `ISound` was given, in the caller's own frame.
    ///
    /// This is the door `AEESound_StatusNotify` uses, and it is the one place a refusal by these
    /// slots can be heard: the platform code calls the handler before its own slot returns, so a pack
    /// that asked for a tone outside the published list is told so *during* the call. Everything the
    /// hardware would report later travels as [`Job::Tone`] instead, which is what
    /// `AEESound_StatusCB` does when it fills a record and hands it to `AEE_ResumeCallback`.
    ///
    /// The reference function brackets the handler with the interface's own `AddRef` and `Release` so
    /// the object cannot die underneath it; this host cannot, because its cleanup runs in the
    /// `Release` dispatch rather than on the count reaching zero inside a host-side reference. So
    /// [`Runtime::tone_write_back`] checks for the one case that bracket was there to prevent.
    pub(crate) fn tone_notify(
        &mut self,
        tone: &Tone,
        cmd: u32,
        status: u32,
        param: u32,
    ) -> Result<(), Error> {
        if tone.pfn == 0 {
            return Ok(());
        }
        self.call_guest(tone.pfn, &[tone.user, cmd, status, param])?;
        Ok(())
    }

    /// Queue the `AEE_SOUND_SUCCESS` that the published status lists promise for `ISOUND_StopTone` and
    /// `ISOUND_SetVolume` — the two slots whose whole job is a request the host can answer at once.
    /// The two vibration slots are deliberately not among them: their own side-effects lines say
    /// "This function does not cause any callback", and the reference
    /// bodies raise nothing. The play slots are not either: `AEESound_PlayTone`'s accepted path calls
    /// the OEM generator and counts the play, and the report that follows is the end of the play, not
    /// its start.
    ///
    /// `AEE_SOUND_SUCCESS` means "Request is accepted" in the published list of status meanings,
    /// which is exactly what this host did — it is not a claim that anything was heard. The push is
    /// not gated on a registered handler, because `AEESound_AEECallbackNotify` does its own
    /// bookkeeping whether or not there is anybody to call.
    pub(crate) fn tone_accepted(&mut self, object: u32) {
        self.queue.push_back(Job::Tone {
            object,
            cmd: g::AEE_SOUND_STATUS_CB,
            status: g::AEE_SOUND_SUCCESS,
            data: 0,
            size: 2,
        });
    }

    pub(crate) fn tone_arm(&mut self, at: u32, tone: &mut Tone) -> Result<bool, Error> {
        let item = tone.list + u32::from(tone.list_index) * 4;
        let id = u32::from(self.cpu.mem.read_u8(item)?);
        let duration = u32::from(self.cpu.mem.read_u16(item + 2)?);
        self.tone_play(at, tone, Some(id), None, duration)
    }

    /// `AEESound_PlayTone`'s own three steps: reject an id the published list does not name, count
    /// the request as pending, and arm the report that ends it. `None` for the id is
    /// `AEESound_PlayFreqTone`, which names two frequencies instead and has nothing to range-check —
    /// those arrive in `freq`, and an id-based request resolves its pair through
    /// [`crate::audio::tone::frequencies`] here so the audio thread never owns a table of ids.
    ///
    /// A duration of zero arms nothing: the header's comment on both play slots says the tone "plays
    /// until the `ISOUND_StopTone` call is issued", so zero is a request to keep going, not a tone
    /// of no length. The pending count then waits for a stop, which is what a handset does too.
    ///
    /// The answer says whether this frame reached the caller's own function, which is what the caller
    /// needs before it writes anything back: the platform code stores its state *first* and raises the
    /// status function *after* (`AEESound_PlayToneList` writes its four list fields, then calls
    /// `AEESound_PlayTone`), so a handler that answers the report by calling the interface back is the
    /// last writer. A frame that stored its own copy afterwards would undo that call.
    pub(crate) fn tone_play(
        &mut self,
        at: u32,
        tone: &mut Tone,
        id: Option<u32>,
        freq: Option<(u16, u16)>,
        duration: u32,
    ) -> Result<bool, Error> {
        if let Some(id) = id {
            // `AEESound_PlayTone` answers `(char)a2 <= 0 || (char)v4 >= 100` with
            // `AEESound_StatusNotify(a1, 3)` — a `AEE_SOUND_FAILURE` handed to the caller's function
            // before the slot returns, and the tone is never counted as playing.
            //
            // The bound is the reference one, not `AEE_TONE_LAST`: that constant is `0x68`, a reserved
            // sentinel, while the published list ends at `AEE_TONE_PTT_FLOOR_AVAILABLE` = `0x63`, so
            // the reference `100` is exactly one past the highest id a pack may name. Ids 100 to 103
            // name nothing any header lists; on a handset they are the OEM tone database's problem,
            // which is not the same thing as a valid id.
            // The reference test is written against `(char)` arguments, so its two halves are a sign
            // test and a range test. Only the range half is modelled here: the id arrives as a
            // 32-bit word, `AEE_TONE_FIRST` is zero, and the narrowing's signedness decides whether
            // ids 128-255 are the "negative" case — a reading this host has not confirmed, so it
            // rejects the one id it can name (zero) and leaves the rest to the range test rather
            // than guessing a branch. The exact id boundary remains an open question.
            if id == g::AEE_TONE_FIRST || id >= 100 {
                self.tone_write_back(at, *tone);
                self.tone_notify(tone, g::AEE_SOUND_STATUS_CB, g::AEE_SOUND_FAILURE, 0)?;
                return Ok(true);
            }
            tone.tone = id;
            // What the id sounds like, resolved here and not on the audio thread: the table is the
            // ABI's, and an id outside it is a tone this host cannot make rather than an error —
            // the request is still accepted and still reports `AEE_SOUND_PLAY_DONE`.
            let (hi, lo) = crate::audio::tone::frequencies(id).unwrap_or((0, 0));
            tone.freq_hi = hi;
            tone.freq_lo = lo;
        } else {
            let (hi, lo) = freq.unwrap_or((0, 0));
            tone.freq_hi = hi;
            tone.freq_lo = lo;
        }
        // `AEESound_PlayFreqTone` names two frequencies instead of an id, counts its play before
        // anything else it does, and has nothing here to refuse.
        tone.playing = true;
        tone.pending += 1;
        tone.due_at = if duration == 0 {
            0
        } else {
            self.clock_ms.saturating_add(duration)
        };
        Ok(false)
    }

    /// Put an `ISound` record back, unless the handler it delivered released the interface from
    /// underneath itself. `IShell`'s release path already dropped the record; writing it back would
    /// hand the next object created at this address a stranger's tone, list and volume.
    pub(crate) fn tone_write_back(&mut self, object: u32, tone: Tone) {
        if self.tones.contains_key(&object) {
            self.tones.insert(object, tone);
        }
    }

    /// The `ISound` slots — the tone and vibration player, which is a different thing from the
    /// sound player and has no `Play` at all.
    ///
    /// As on [`Runtime::isoundplayer`], every slot the table has is answered, because an app that
    /// wanted a dial tone and was told the interface does not exist stops for a reason that is not
    /// true of the pack. The commands whose only effect is audible or physical are taken and
    /// dropped; the reads report the state the caller put here, which is the honest answer for a
    /// silent device — it really does hold what it was told to hold.
    ///
    /// `Get` reads and writes the five members of `AEESoundInfo` as five bytes each. The published
    /// struct table says twenty, because each member is `#ifdef _WIN32 unsigned #else int8` and the
    /// extractor kept the host's branch; the guest is ARM code, which takes the other one. Five is
    /// the width that cannot write outside the caller's struct under either reading.
    ///
    /// `RegisterNotify` used to be recorded and never raised, on the reasoning that a tone's status
    /// callback reports a completion this host cannot reach. That reasoning is withdrawn: the header
    /// lists the callback for eight named slots, `AEESound_StatusNotify` answers a rejected request
    /// inside the caller's frame, and a pack that plays a tone and waits to be told it is over has
    /// nothing to wait for while the record sits unused. What this host sends is `AEE_SOUND_SUCCESS`
    /// — "Request is accepted", which is true of it — and `AEE_SOUND_PLAY_DONE` after the length the
    /// caller itself asked for. `AEESound_SetDevice` is the one void command the table does *not*
    /// list, so it is still the one command that answers nothing.
    ///
    /// Like [`Runtime::irscpool`], this arm returns the runtime's own error type rather than an
    /// `HleError`: refusing a tone by calling the caller's handler is a downcall, and a handler that
    /// faults or that stops the run is not a gap in the host.
    pub(crate) fn isound(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let mut tone = self.tones.get(&this).copied().unwrap_or_default();
        if !tone.noted {
            tone.noted = true;
            self.tones.insert(this, tone);
            self.display.note(format!(
                "{label}: this host makes no sound and does not vibrate"
            ));
        }
        // Whether this frame has already reached the caller's own function. Once it has, the frame has
        // finished writing: the handler may call the interface back from inside that call, and on a
        // handset the handler's stores are the last word about the record, not this frame's copy.
        let mut reached = false;
        let retval = match name {
            "RegisterNotify" => {
                let [pfn, user] = self.args()?;
                tone.pfn = pfn;
                tone.user = user;
                0
            }
            // `AEESound_SetVolume` keeps the caller's number on the 0..`AEE_MAX_VOLUME` scale and
            // does not touch the hardware from this call: it stores the value, asks the OEM for the
            // device's level count, and `AEESound_LevelCB` does the arithmetic —
            // `volume * levels / 100` — and the set once that answer comes back. This host has no
            // level count to scale against, so the number it stores is the one the caller hears back
            // from `GetVolume` unchanged, which is the only round trip it can promise.
            "SetVolume" => {
                let [volume] = self.args()?;
                tone.volume = volume;
                self.tone_accepted(this);
                0
            }
            "Set" => {
                let [pinfo] = self.args()?;
                if pinfo == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let mut info = [0u8; 5];
                for (i, member) in info.iter_mut().enumerate() {
                    *member = self.cpu.mem.read_u8(pinfo + i as u32)?;
                }
                // `AEESound_Set` range-checks all five members in the order the struct declares them
                // and stores nothing unless every one passes:
                // `if ( *a2 >= 11 || a2[1] >= 8 || a2[2] >= 4 || a2[3] >= 2 || a2[4] >= 2 ) return 14;`
                // then `aee_memmove(a1 + 100, a2, 5)`. Those four numbers are that build's own enum
                // counts; the bounds used here are the published `AEE_SOUND_*_LAST`, because a pack
                // running here names the ids from the published header, not from one chipset's list.
                if info[0] >= g::AEE_SOUND_DEVICE_LAST as u8
                    || info[1] >= g::AEE_SOUND_METHOD_LAST as u8
                    || info[2] >= g::AEE_SOUND_APATH_LAST as u8
                    || info[3] >= g::AEE_SOUND_MUTECTL_LAST as u8
                    || info[4] >= g::AEE_SOUND_MUTECTL_LAST as u8
                {
                    return Ok(g::AEE_EBADPARM);
                }
                tone.info = info;
                0
            }
            "Get" => {
                let [pinfo] = self.args()?;
                if pinfo != 0 {
                    for (i, member) in tone.info.iter().enumerate() {
                        self.cpu.mem.write_u8(pinfo + i as u32, *member)?;
                    }
                }
                0
            }
            // `AEESound_GetVolume` is `void`. It raises a flag, asks the OEM for the level count, and
            // `AEESound_VolumeCB` normalizes what the OEM then reports — `100 * a3 / levels`, the
            // `/100` being `__rt_divs_100` for `AEE_MAX_VOLUME` — into the two-byte `AEESoundCmdData`
            // inside its own record before resuming it. So the answer arrives as
            // `AEE_SOUND_VOLUME_CB` with `dwParam` pointing at a `uint16`,
            // and this slot's `r0` holds only whatever the bridle call left there.
            "GetVolume" => {
                self.queue.push_back(Job::Tone {
                    object: this,
                    cmd: g::AEE_SOUND_VOLUME_CB,
                    status: g::AEE_SOUND_SUCCESS,
                    data: tone.volume,
                    size: 2,
                });
                0
            }
            "GetResourceCtl" => {
                let [ppo] = self.args()?;
                if ppo == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if self.resource_ctl == 0 {
                    let obj = self.hle.new_object(
                        &mut self.cpu.mem,
                        "IResourceCtl",
                        g::AEEIID_IResourceCtl,
                        8,
                        4,
                        &[],
                    )?;
                    self.resource_ctl = obj;
                    self.res_ctl.owner = self.applet;
                }
                // One object, handed out again and again: there is one sound resource, so there is one
                // controller for it, and the `AddRef` is what makes the "user should Release" contract true.
                let obj = self.resource_ctl;
                self.hle.change_refs(&mut self.cpu.mem, obj, 1)?;
                self.cpu.mem.write_u32(ppo, obj)?;
                g::AEE_SUCCESS
            }
            // The commands below are `void` and, on a handset, hand the work to the OEM tone
            // generator; what this host keeps is the request and the report the caller was promised.
            // `SetDevice` takes no arguments in the 3.1 ABI: it commits the device already inside the
            // record, and the header does not list it among the commands that report back.
            "SetDevice" => 0,
            // One register for the whole `AEESoundToneData` — the delivered body shows
            // `int __fastcall AEESound_PlayTone(int a1, int a2)` and passes `&v4` down, because
            // `AEESoundTone` is `int8` off Windows, which puts the id in the
            // low byte and the duration in the halfword at offset two. Reading a second register for
            // the length was reading whatever the caller's `r2` happened to hold.
            "PlayTone" => {
                let [packed] = self.args()?;
                reached = self.tone_play(
                    this,
                    &mut tone,
                    Some(packed & 0xff),
                    None,
                    (packed >> 16) & 0xffff,
                )?;
                0
            }
            "PlayToneList" => {
                let [list, count] = self.args()?;
                if list == 0 || count == 0 {
                    self.tone_notify(&tone, g::AEE_SOUND_STATUS_CB, g::AEE_SOUND_FAILURE, 0)?;
                    return Ok(0);
                }
                // `AEESound_PlayToneList` stores the caller's pointer instead of copying the list and
                // truncates the length into a `uint16` field, so the buffer belongs to the pack for
                // as long as the list runs — each further item is read out of it when the previous
                // one reports done. The four stores come first, exactly as they do in the reference
                // body (`+120` flag, `+122` index, `+124` count, `+128` pointer), and only then does
                // it call `AEESound_PlayTone` for item zero, which may refuse and reach the handler.
                tone.list = list;
                tone.list_index = 0;
                tone.list_count = count as u16;
                tone.list_active = true;
                reached = self.tone_arm(this, &mut tone)?;
                0
            }
            "PlayFreqTone" => {
                let [hi, lo, duration] = self.args()?;
                // Two frequencies and a length, with no id to range-check: `AEESound_PlayFreqTone`
                // counts the play before anything else it does, and nothing here can refuse it. The
                // ABI declares both as `uint16` (`ifaces.rs` `ISound::PlayFreqTone`), so the upper
                // half of each register is the caller's `r` width and not part of the tone.
                self.tone_play(
                    this,
                    &mut tone,
                    None,
                    Some((hi as u16, lo as u16)),
                    duration,
                )?;
                0
            }
            "StopTone" => {
                // `AEESound_StopTone` clears the list *pointer* while leaving the list flagged
                // running, and the flag is what the next `PLAY_DONE` clears. So a stop in the middle
                // of a list makes the item then playing the last one — the rest of the list is not
                // picked up, because the record it lives in has been let go.
                if tone.list_active {
                    tone.list = 0;
                }
                // Cancelling the armed deadline is this host's own move: the documented behaviour stops the
                // OEM generator and leaves the record's play flags alone, because the report that
                // ends the play comes back from the OEM through `AEESound_AEECallbackNotify`. Here
                // there is no other party to report the end of a tone the caller has just stopped, so
                // the armed `AEE_SOUND_PLAY_DONE` is taken back rather than delivered behind the stop.
                tone.due_at = 0;
                tone.playing = false;
                self.tone_accepted(this);
                0
            }
            // The two vibration slots take the request and stay quiet about it. `AEESound_Vibrate` is
            // `AEESound_AcquireResource` plus one call of the OEM function stored at `+300`, and
            // `AEESound_StopVibrate` the same with a zero length: neither raises
            // `AEESound_StatusNotify`, neither touches the pending count at `+116`, and the header's
            // own side-effects line for both says "This function does not cause any callback"
            // — although the list under `ISOUND_RegisterNotify` names
            // both among the functions that report through `AEE_SOUND_STATUS_CB`. Where
            // The header nothing. The length is a request to a physical vibrator this host has no model of, so
            // there is no timed end to report either.
            "Vibrate" => {
                let [_duration] = self.args()?;
                tone.vibrating = true;
                0
            }
            "StopVibrate" => {
                tone.vibrating = false;
                0
            }
            _ => {
                return Err(Error::Hle(HleError::Unimplemented {
                    id,
                    label: label.to_string(),
                    this,
                }))
            }
        };
        if !reached {
            self.tone_write_back(this, tone);
        }
        Ok(retval)
    }

    /// The bridged `IAStream` — the other direction, an `IAStream` face over a source the caller owns.
    /// Its vtable has no `QueryInterface` at all (`gvtSourceAStream_IAStream` is AddRef, Release,
    /// Readable, Read, Cancel), which is what the vendored five-slot table says too.
    pub(crate) fn iastream(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let unimplemented = || {
            Error::Hle(HleError::Unimplemented {
                id,
                label: label.to_string(),
                this,
            })
        };
        // A host-made `IMemAStream` reached through its `IAStream` face is the reference shape — the
        // delivered `CMemStream` vtable **is** an `IAStream`'s — and a pack that calls `IASTREAM_Read`
        // on one ended the run here, because this arm only knew the bridged source. Forward to the
        // record the class was created under.
        if self.memstreams.contains_key(&this) {
            return self.imemastream(id, this, label, name);
        }
        let Some(bridge) = self.source_streams.get(&this).copied() else {
            return Err(unimplemented());
        };
        let at = |field: &str| {
            abi::offset_of("_AEECallback", field).expect("_AEECallback is in the ABI tables")
        };
        match name {
            // `SourceAStream_Read`: the source's slot 3 with the caller's buffer and length handed
            // over. A source that answers zero or negative says so through this object unchanged.
            "Read" => {
                let [buf, len] = self.args()?;
                let read = self.guest_slot(bridge.source, "ISource", "Read")?;
                self.call_guest(read, &[bridge.source, buf, len])
            }
            // `SourceAStream_Readable(pfn, pcx)` is the shape that needs no thunk: the caller's
            // function and context are written into the record's own embedded callback, at the offsets
            // `_AEECallback` gives its notify pair, and the *source* is told to call that callback. So
            // when the pack's source has bytes it runs the pack's own function with the pack's own
            // context, exactly as the reference object arranges. With no function named, the body only
            // cancels.
            "Readable" => {
                let [pfn, ctx] = self.args()?;
                let readable = self.guest_slot(bridge.source, "ISource", "Readable")?;
                if pfn == 0 {
                    self.cpu.mem.write_u32(bridge.cb + at("pfnNotify"), 0)?;
                    self.call_guest(readable, &[bridge.source, bridge.cb])?;
                    return Ok(g::SUCCESS);
                }
                self.cpu.mem.write_u32(bridge.cb + at("pfnNotify"), pfn)?;
                self.cpu.mem.write_u32(bridge.cb + at("pNotifyData"), ctx)?;
                self.call_guest(readable, &[bridge.source, bridge.cb])?;
                Ok(g::SUCCESS)
            }
            // `SourceAStream_Cancel` ignores both of its arguments and cancels the record's own
            // callback — `CALLBACK_Cancel`, which runs `pfnCancel` on the whole struct if one is
            // registered and does nothing if there is not.
            "Cancel" => {
                self.args::<2>()?;
                let cancel = self.cpu.mem.read_u32(bridge.cb + at("pfnCancel"))?;
                if cancel != 0 {
                    self.call_guest(cancel, &[bridge.cb])?;
                    self.cpu.mem.write_u32(bridge.cb + at("pfnCancel"), 0)?;
                }
                Ok(g::SUCCESS)
            }
            _ => Err(unimplemented()),
        }
    }
}
