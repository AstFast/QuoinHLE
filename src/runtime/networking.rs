//! Networking: `INetMgr` and its friends, the socket model, `ISockPort`,
//! `IPort1` and the TLS/SSL record layer.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// `INetUtils` — the network-utilities object, `AEECLSID_NETUTILS` / `AEEIID_INETUTILS`.
    ///
    /// Four slots and three are about **Packet Data Profiles**: "the configured Packet
    /// Data Profiles on the device", with `GetPDPCount` answering "the number of Packet Data
    /// Profiles available on the device". This host has no radio and no configured profiles, so
    /// the count is **zero** — a number, not a refusal: a caller that enumerates from it gets exactly the
    /// empty enumeration such a device would give, and `GetPDPInfo` then has no valid profile number to take.
    ///
    /// ⚠ `GetDefaultNetwork` answers non-zero, and its own page names both its returns — "`AEE_NET_SUCCESS`:
    /// the default data network was filled in", "`AEE_EFAILED`: operation failed". The data
    /// network of a device with no data network is `AEE_NETWORK_DEFAULT`, which the platform defines as zero and
    /// which is what "whatever the device defaults to" means; filling that in is filling in the truth.
    ///
    /// `ExtFunc` is the one slot with no page at all — the vtable comment introduces it as an "Extension
    /// Function to allow future enhancement to the interface without breaking its binary compatibility"
    /// — so the honest answer is that this host has no extension to offer.
    pub(crate) fn inetutils(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEEIID_INETUTILS, g::AEECLSID_NETUTILS])
            }
            // "returns the default data network … `pnNetwork`: filled with the default data network
            // (`AEE_NETWORK_*`)".
            "GetDefaultNetwork" => {
                let [family, out] = self.args()?;
                if out == 0 {
                    // The header names no null-pointer refusal; a filled-in slot is what `AEE_NET_SUCCESS`
                    // asserts, so there is nothing to assert without one.
                    return Ok(g::AEE_EFAILED);
                }
                let _ = family;
                self.cpu.mem.write_u32(out, g::AEE_NETWORK_DEFAULT)?;
                Ok(g::AEE_NET_SUCCESS)
            }
            // "filled in with the PDP count"; none are configured on this host.
            "GetPDPCount" => {
                let [out] = self.args()?;
                if out == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                self.cpu.mem.write_u16(out, 0)?;
                Ok(g::AEE_NET_SUCCESS)
            }
            // "`AEE_NET_SUCCESS`: the information was filled in / `AEE_NET_EINVAL`: the PDP number is not
            // valid / `AEE_EFAILED`: operation failed". With no profiles at all, every number is
            // invalid — and "the available profiles are numbered consecutively from **one** to this count",
            //so zero is not a number a caller may ask for either.
            "GetPDPInfo" => {
                let [profile, info] = self.args()?;
                if info == 0 {
                    // Nothing to fill in, which is a different complaint from "no such profile".
                    return Ok(g::AEE_EFAILED);
                }
                let _ = profile;
                Ok(g::NET_ERROR_BASE + 0x1a)
            }
            "ExtFunc" => Ok(g::AEE_EUNSUPPORTED),
            _ => Err(unimplemented()),
        }
    }

    pub(crate) fn net_select(&mut self, id: u32) -> u32 {
        if self.net_selected != 0 {
            return socket::EOPNOTSUPP;
        }
        match id & 0xffff {
            1 | 2 | 8 => {
                self.net_selected = id;
                g::AEE_SUCCESS
            }
            0 if id == 0 => g::AEE_EFAILED,
            _ => socket::EINVAL,
        }
    }

    pub(crate) fn net_my_ip_addr(&mut self) -> u32 {
        if self.net_selected == 0 {
            return g::AEE_EFAILED;
        }
        self.net_last_error = socket::ENETNONET;
        socket::ENETNONET
    }

    /// Every `INetMgr` slot. An emulator has no radio, and this host does not pretend otherwise: what
    /// it gives instead of a refusal is the answer a handset gives when it has no data connection, in
    /// the shape each published signature asks for — a state, a null, an error code the caller already
    /// has a branch for. That distinction is the whole reason this class is served: a caller that
    /// reached this object through `IShell::CreateInstance` is one that stores what it gets, and a
    /// class-level refusal hands it nothing to store.
    ///
    /// ⚠ `OpenSocket` is the one door that is **not** answered that way, because there is no "no radio"
    /// shape for it to copy: this host has a transport to put behind a socket — the loopback machine, and
    /// a real one whenever the run names `--net-proxy` — so the manager hands out the same `ISocket` face
    /// an `ISockPort` would, with the same gates and the same provider.
    pub(crate) fn inetmgr(
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
            "SetMask" => {
                let [masks] = self.args()?;
                if self.net_selected == 0 {
                    return Ok(0);
                }
                self.net_mask = 0;
                let mut at = masks;
                while at != 0 {
                    let word = self.cpu.mem.read_u32(at)?;
                    if word == 0 {
                        break;
                    }
                    self.net_mask |= word;
                    at = at.wrapping_add(4);
                }
                Ok(0)
            }
            // The documented behaviour writes the failure into the record and then resumes the caller's
            // callback through `IShell::Resume`; a caller that waits on it must not be left waiting.
            "GetHostByName" => {
                let [result, _host, cb] = self.args()?;
                let code = g::AEE_EFAILED;
                if result != 0 {
                    let at = abi::offset_of("AEEDNSResult", "nResult").ok_or_else(unimplemented)?;
                    self.cpu.mem.write_u32(result + at, code)?;
                }
                if cb != 0 {
                    // The documented behaviour calls `IShell::Resume` — the same door every other arming
                    // route takes, so the block is gated, cancelled out of wherever it was, and
                    // stamped with the queue's leg before it is queued.
                    self.dispatcher_resume(cb, false)?;
                }
                Ok(code)
            }
            "GetLastError" => Ok((self.net_last_error as u16 as i16 as i32) as u32),
            "OpenSocket" => {
                let [stype] = self.args()?;
                // No type gate: the body passes the `NetSocket` word straight to the port as its
                // protocol (`R1 = #2` for the family, `R2 = a2` at `0x79c-0x7a0`) and translates whatever
                // the port answers. That is why `AEE_SOCK_STREAM`/`AEE_SOCK_DGRAM` need no mapping here,
                // and why an unknown type comes back as the port's own refusal rather than a new code.
                let Ok(base) = self.port_new_object() else {
                    // `AEE_NET_GENERAL_FAILURE  An ISocket could not be allocated` (the header,
                    // The header
                    self.net_last_error = socket::GENERAL_FAILURE;
                    return Ok(0);
                };
                if self.port_open_inet(base, stype as u16)? != g::AEE_SUCCESS {
                    let code = self
                        .ports
                        .get(&base)
                        .map(|port| port.error as u32)
                        .unwrap_or(socket::GENERAL_FAILURE);
                    self.net_last_error = match code {
                        socket::EINVAL => socket::ESOCKNOSUPPORT,
                        socket::ENOMEM => socket::GENERAL_FAILURE,
                        _ => g::AEE_EFAILED,
                    };
                    // The caller never received this port, so nothing will ever release it: the door
                    // that made it is the door that lets it go — the provider and the OEM record too.
                    // ⚠ Reachable and asserted: asking for a type the port refuses walks this branch.
                    self.port_died(base);
                    self.hle.forget(base);
                    return Ok(0);
                }
                // No `hold` here, unlike the `QueryInterface` door: this port was never handed to anybody
                // as an `ISockPort`, so its one reference **is** the face the caller now owns. Releasing
                // the face is what frees the record — and a reference taken for this door's own list
                // would outlive the manager, which is exactly what the published app does
                // (it opens, then releases the manager, keeping only the socket).
                Ok(self.port_face(base)?)
            }
            "NetStatus" => {
                let [stats] = self.args()?;
                if self.net_selected != 0 {
                    self.net_status_fill(stats)?;
                }
                Ok(g::AEE_NET_STATUS_INVALID_STATE)
            }
            "GetMyIPAddr" => Ok(self.net_my_ip_addr()),
            // `lsls r0,#0x10; lsrs r0,#0x10` on the way out: the caller gets the gate's code in a
            // `uint16`, and past the gate the previous linger in seconds instead.
            "SetLinger" => {
                let [secs] = self.args()?;
                if self.net_selected == 0 {
                    return Ok(g::AEE_EFAILED & 0xffff);
                }
                let previous = (self.net_linger_ms / 1000) as u16;
                if previous == 0xDEAD && (secs as u16) == 0xBEEF {
                    self.display.note(
                        "INetMgr::SetLinger: the 0xDEAD/0xBEEF pair asks the net object's \
                         dormancy interface, which this host does not model",
                    );
                }
                self.net_linger_ms = u32::from(secs as u16) * 1000;
                Ok(u32::from(previous))
            }
            // Three arguments, all forwarded: `(pfn, pUser, bRegister)`; the fourth word the net
            // object's constructor wants is `bRegister` again, pushed on the stack.
            "OnEvent" => {
                let [pfn, user, register] = self.args()?;
                if self.net_selected == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                // `AEENetOnEvent_New` matches a record by `user`, and by `pfn` too unless the
                // caller passed no `pfn` — which is the same predicate a cancel uses.
                let found = self
                    .net_events
                    .iter()
                    .position(|held| held.1 == user && (pfn == 0 || held.0 == pfn));
                if register == 0 {
                    if let Some(at) = found {
                        self.net_events.remove(at);
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                if found.is_some() {
                    return Ok(g::EALREADY);
                }
                self.net_events.insert(0, (pfn, user));
                Ok(g::AEE_SUCCESS)
            }
            // `INetMgr` options ask `AEENetMgr_SelectNetwork` before the switch, with two
            // exceptions the body checks first: `INET_OPT_SELECT_NET` and the two UMTS
            // count/info options (GetOpt, whose name is truncated to 16 bits first). Past the gate
            // the option runs off a jump table (`a2 < 0xC` for SetOpt, `a2 < 0xB` for GetOpt) and
            // everything outside it answers `AEE_EUNSUPPORTED`; the entries themselves are per-option
            // handlers whose values come from the net module, which is why this host cannot answer
            // them and says so rather than inventing a success.
            "SetOpt" => {
                let [opt, val, size] = self.args()?;
                if opt == g::INET_OPT_SELECT_NET {
                    // A 4-byte network id, then the same selection call the body makes.
                    if size != 4 {
                        return Ok(g::AEE_EBADPARM);
                    }
                    if val == 0 || self.cpu.mem.read_bytes(val, 4).is_err() {
                        return Ok(g::AEE_EMEMPTR);
                    }
                    let id = self.cpu.mem.read_u32(val)?;
                    return Ok(self.net_select(id));
                }
                if self.net_selected == 0 {
                    // No network: the selection is what fails, before any option's own checks.
                    return Ok(g::AEE_EFAILED);
                }
                if opt >= 0xC {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                self.display.note(
                    "INetMgr::SetOpt: this option's branch is the net module's, which this host cannot call",
                );
                Ok(g::AEE_EFAILED)
            }
            "GetOpt" => {
                let [opt, val, pnsize] = self.args()?;
                let opt = opt & 0xffff;
                if opt == g::INET_OPT_UMTS_COUNT || opt == 10 {
                    let need: u32 = if opt == g::INET_OPT_UMTS_COUNT {
                        2
                    } else {
                        0x44
                    };
                    // `AEE_CheckPtr(0, pnOptSize, 4, 1)`: a bad out-size pointer is `AEE_EMEMPTR`.
                    if pnsize == 0 || self.cpu.mem.read_bytes(pnsize, 4).is_err() {
                        return Ok(g::AEE_EMEMPTR);
                    }
                    let have = self.cpu.mem.read_u32(pnsize)?;
                    if val != 0 && have < need {
                        return Ok(g::AEE_EBADPARM);
                    }
                    self.cpu.mem.write_u32(pnsize, need)?;
                    if val == 0 {
                        return Ok(0);
                    }
                    if self.cpu.mem.read_bytes(val, need as usize).is_err() {
                        return Ok(g::AEE_EMEMPTR);
                    }
                    // The value itself is filled by a bridle call this box cannot make; the size
                    // half above is exact, the data half has no record.
                    self.display.note(
                        "INetMgr::GetOpt: the UMTS value comes from a bridle call this host cannot make",
                    );
                    return Ok(g::AEE_EFAILED);
                }
                if self.net_selected == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                // Past the gate the size pointer is checked before the switch, exactly as the two
                // bypassing options above check it.
                if pnsize == 0 || self.cpu.mem.read_bytes(pnsize, 4).is_err() {
                    return Ok(g::AEE_EMEMPTR);
                }
                if opt >= 0xB {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                self.display.note(
                    "INetMgr::GetOpt: this option's branch is the net module's, which this host cannot call",
                );
                Ok(g::AEE_EFAILED)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    pub(crate) fn net_status_fill(&mut self, stats: u32) -> Result<(), Error> {
        if stats == 0 {
            return Ok(());
        }
        let up = self.clock_ms;
        let b = [0u32; 7];
        let elapsed = |since: u32| {
            if since == 0 {
                0
            } else {
                up.wrapping_sub(since)
            }
        };
        let mut words = [0u32; 8];
        words[0] = elapsed(b[0]) / 1000;
        let awake_ms = b[2].wrapping_add(elapsed(b[1]));
        words[3] = (1000u64 * u64::from(b[3]) / u64::from(awake_ms.max(1))) as u32;
        words[1] = awake_ms / 1000;
        words[2] = b[3];
        words[4] = b[4].wrapping_add(words[0]);
        words[5] = b[5].wrapping_add(words[1]);
        words[6] = b[6].wrapping_add(b[3]);
        words[7] = words[6] / words[5].max(1);
        let mut raw = [0u8; 32];
        for (i, word) in words.iter().enumerate() {
            raw[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        self.cpu.mem.write_bytes(stats, &raw)?;
        Ok(())
    }

    pub(crate) fn idnsconfig(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `int SelectNetwork(me, int nNetwork)`. The body stores one selection and answers
            // `522` (already selected) to a second call; a stored zero still counts as unset.
            "SelectNetwork" => {
                let [network] = self.args()?;
                let current = self.cpu.mem.read_u32(this + 8)?;
                if current != 0 {
                    return Ok(522);
                }
                self.cpu.mem.write_u32(this + 8, network)?;
                Ok(0)
            }
            // `int SetServers(me, INSockAddr *pServers, uint16 wCount)`: the body validates the
            // caller's whole `wCount * 8` bytes first (`AEE_EMEMPTR` on failure), then clamps the
            // count to three before it stores the records.
            "SetServers" => {
                let [servers, count] = self.args()?;
                let count = count & 0xffff;
                if count != 0
                    && self
                        .cpu
                        .mem
                        .read_bytes(servers, (count as usize) * 8)
                        .is_err()
                {
                    return Ok(g::AEE_EMEMPTR);
                }
                let kept = count.min(3);
                self.cpu.mem.write_u16(this + 0x24, kept)?;
                if kept != 0 {
                    let bytes = self.cpu.mem.read_bytes(servers, (kept as usize) * 8)?;
                    self.cpu.mem.write_bytes(this + 0xc, &bytes)?;
                }
                self.cpu.mem.write_u16(this + 0x26, 0)?;
                Ok(0)
            }
            // `int GetServers(me, INSockAddr *pServers, uint16 *pwCount)`: both pointers are
            // checked; the answer is `min(*pwCount, stored)`, and the caller's count is then
            // overwritten with that. The body's cursor wraps by `% stored`; nothing here advances
            // the cursor, so the first `n` records are the answer.
            "GetServers" => {
                let [servers, pwcount] = self.args()?;
                if pwcount == 0 || self.cpu.mem.read_bytes(pwcount, 2).is_err() {
                    return Ok(g::AEE_EMEMPTR);
                }
                let cap = self.cpu.mem.read_u16(pwcount)?;
                if cap != 0
                    && self
                        .cpu
                        .mem
                        .read_bytes(servers, (cap as usize) * 8)
                        .is_err()
                {
                    return Ok(g::AEE_EMEMPTR);
                }
                let stored = self.cpu.mem.read_u16(this + 0x24)?;
                let n = cap.min(stored);
                self.cpu.mem.write_u16(pwcount, n)?;
                if n != 0 {
                    let bytes = self.cpu.mem.read_bytes(this + 0xc, (n as usize) * 8)?;
                    self.cpu.mem.write_bytes(servers, &bytes)?;
                }
                Ok(0)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Collect whatever the network thread has finished since the last turn, hand the guest what it
    /// asked for, and queue the call that was promised.
    ///
    /// Called once at the top of every [`Runtime::pump`]. A reply that has arrived does three things
    /// in order: it is logged, it becomes the response objects the caller's out-parameter was left
    /// waiting for, and the caller's `AEECallback` is queued — the contract's "where to call when
    /// answer is ready". A reply that has not arrived yet does nothing at all,
    /// which leaves the guest in exactly the state a slow connection leaves a handset in.
    ///
    /// The order of the last two is not decoration: the callback is what reads that out-parameter, so
    /// a host that queued the call first would be handing the pack a word that is still the null
    /// `GetResponse` wrote.
    pub(crate) fn poll_net(&mut self) -> Result<(), Error> {
        let replies = match self.net.as_ref() {
            Some(proxy) => proxy.take_replies(),
            None => return Ok(()),
        };
        for reply in replies {
            let at = u64::from(self.clock_ms);
            // A reply this host asked for on a page's behalf is a picture, not a guest's answer: it
            // settles into the page's decode cache instead of into a `WebRespInfo`, and the page is
            // told its content changed. `HVN_INVALIDATE` is the notification the header names for
            // exactly that; it goes out per picture so a long page fills in as the pictures land.
            if let Some(fetch) = self.html_fetches.remove(&reply.id) {
                let picture = match &reply.result {
                    Ok(answer) => {
                        self.net_log.push((
                            at,
                            format!(
                                "<- {} ({} bytes, picture for {})",
                                reply.url,
                                answer.body.len(),
                                fetch.src
                            ),
                        ));
                        crate::resource::image(&[], &answer.body)
                            .ok()
                            .map(std::sync::Arc::new)
                    }
                    Err(why) => {
                        self.net_log.push((
                            at,
                            format!("<- {} failed: {why} (picture for {})", reply.url, fetch.src),
                        ));
                        None
                    }
                };
                // A control that has been released while the request was out has nowhere to put the
                // answer and nothing to notify. Probing it first matters more than it looks: every
                // step below would *fail* on a gone control, and `html_notify` in particular would
                // carry that failure out of `poll_net` and end a run over a page the pack has
                // already closed. A page that is gone is not an error — it is a page that is gone.
                if self.widgets.html_busy(fetch.ctl).is_ok() {
                    // A failed decode is recorded as `None` too: a `src` that names nothing must
                    // not be retried on every frame it is painted.
                    let _ = self
                        .widgets
                        .html_store_picture(fetch.ctl, &fetch.src, picture);
                    if !self.html_fetches.values().any(|f| f.ctl == fetch.ctl) {
                        let _ = self.widgets.html_set_busy(fetch.ctl, false);
                    }
                    // The page has to be told its content changed, or the picture lands in a cache
                    // nobody looks at — but only a page whose pack asked for `HVN_INVALIDATE`:
                    // `HVP_INVALIDATION` is what enables it, and `html_notify_invalidate` is the
                    // one place that reads the flag. A failure here is the guest's own, and travels
                    // as itself.
                    self.html_notify_invalidate(fetch.ctl)?;
                }
                continue;
            }
            let waiting = self.net_waiting.remove(&reply.id);
            match &reply.result {
                Ok(answer) => {
                    let kind = answer.content_type.as_deref().unwrap_or("no content-type");
                    self.net_log.push((
                        at,
                        format!(
                            "<- {} {} ({} bytes, {kind}{})",
                            answer.status,
                            reply.url,
                            answer.body.len(),
                            if answer.truncated { ", truncated" } else { "" }
                        ),
                    ));
                    if let Some(waiting) = waiting.as_ref() {
                        if waiting.ppresp != 0 {
                            match self.build_response(answer) {
                                Ok(Some(response)) => {
                                    self.cpu.mem.write_u32(waiting.ppresp, response)?
                                }
                                // The guest arena is the only thing that can refuse here, and a run
                                // that has spent it has worse problems than a lost response — but it
                                // still gets said out loud, rather than reading as an empty answer.
                                Ok(None) | Err(_) => self.net_log.push((
                                    at,
                                    format!(
                                        "<- {} answered, but no response object was laid out",
                                        reply.url
                                    ),
                                )),
                            }
                        }
                    }
                }
                Err(why) => self
                    .net_log
                    .push((at, format!("<- {} failed: {why}", reply.url))),
            }
            if let Some(waiting) = waiting {
                if waiting.pcb != 0 {
                    self.timers.push(Timer {
                        due: self.clock_ms,
                        pfn: waiting.pcb,
                        user: waiting.pcb,
                        owner: self.applet,
                    });
                }
            }
        }
        Ok(())
    }

    pub(crate) const SOCKPORT_LAYOVER: [u32; 26] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 11, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25,
    ];

    /// The provider's table, built once per runtime.
    #[inline(never)]
    pub(crate) fn sockport_table(&mut self) -> Result<u32, Error> {
        if self.sock_table != 0 {
            return Ok(self.sock_table);
        }
        let table =
            self.hle
                .vtable_ordered(&mut self.cpu.mem, "ISockPort", &Self::SOCKPORT_LAYOVER)?;
        self.sock_table = table;
        Ok(table)
    }

    /// The guest address a model slot is handed out under, built on first use. `Connect` and `Accept`
    /// reach it for the socket on the other side of a loopback connection, which is why it is a lookup
    /// and not part of the birth path.
    #[inline(never)]
    pub(crate) fn sockport_object(&mut self, slot: usize) -> Result<u32, Error> {
        if let Some(addr) = self.sock_slots.get(&slot).copied() {
            return Ok(addr);
        }
        let table = self.sockport_table()?;
        let addr = self.hle.new_object(
            &mut self.cpu.mem,
            "ISockPort",
            socket::CLS_HOSTSOCKET,
            200,
            4,
            &[],
        )?;
        self.cpu.mem.write_u32(addr, table)?;
        self.sock_objects.insert(addr, slot);
        self.sock_slots.insert(slot, addr);
        Ok(addr)
    }

    /// `AEEHostSocket_New` reached the way the reference runtime reaches it — the `gICList` row for
    /// `0x01012d96`, whose `pVoid` is the shell and whose class test is against the row's own id.
    ///
    /// The class check comes before the allocation and there is **no** `AEE_CheckPtr` on `ppv`, unlike
    /// `AEESockPort_New`; the object pointer is written last, only on success. `AEE_CreateInstanceEx`
    /// has already put 0 in the slot, so a refusal still leaves the caller a null.
    #[inline(never)]
    pub(crate) fn sockport_born(&mut self, ppv: u32) -> Result<u32, Error> {
        let Ok(slot) = self.host_sockets.born(self.shell, socket::CLS_HOSTSOCKET) else {
            return Ok(socket::AEE_ECLASSNOTSUPPORT);
        };
        let addr = self.sockport_object(slot)?;
        if ppv == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        // Not [`Runtime::return_object`]: that adds a reference on top of the one the host keeps, and
        // this constructor keeps none — it writes 1 into `[+4]` of the record and hands the pointer
        // out, so the caller holds exactly that one.
        self.cpu.mem.write_u32(ppv, addr)?;
        Ok(g::AEE_SUCCESS)
    }

    #[inline(never)]
    pub(crate) fn sock_died(&mut self, this: u32) {
        let Some(slot) = self.sock_objects.remove(&this) else {
            return;
        };
        self.sock_slots.remove(&slot);
        if !self.host_sockets.is_live(slot) {
            return;
        }
        let done = self.host_sockets.close(slot);
        if let Err(err) = self.answer_sock(done) {
            self.sock_pending = Some(Box::new(err));
        }
        self.host_sockets.forget(slot);
    }

    #[inline(never)]
    pub(crate) fn fire_sock(&mut self, slot: usize, event: u32) -> Result<(), Error> {
        for read in [true, false] {
            let bit = if read {
                socket::EVENT_READ
            } else {
                socket::EVENT_WRITE
            };
            if event & bit == 0 {
                continue;
            }
            let chain = self.host_sockets.drain_chain(slot, read);
            for pcb in chain {
                self.wake_chain_entry(pcb)?;
            }
        }
        Ok(())
    }

    /// The caller's range question that `AEE_CheckPtr` answers, in the two shapes the member uses: a
    /// required pointer must also be non-null. Nothing here caps the length: the bodies hand the caller's
    /// own `cbBuf` to `AEE_CheckPtr` and to `aee_malloc` (,), so a length that is not
    /// actually mapped is refused by the range test itself, and a length that is mapped is one the guest
    /// really has.
    pub(crate) fn sock_range(&self, addr: u32, len: usize) -> bool {
        if addr == 0 || len > u32::MAX as usize {
            return false;
        }
        self.cpu.mem.mapped_run(addr, len as u32) as usize >= len
    }

    /// Every `ISockPort` slot, as the loopback provider delivers them — 26 of them. The name in
    /// `abi::method` is the header's, and [`Runtime::SOCKPORT_LAYOVER`] is what makes the byte offsets
    /// agree with the reference object, so this match can be keyed by name like every other machine.
    ///
    /// What is *not* here is the `AEESockPort` port object: its own 26 slots reach `AEENet_*`,
    /// `OEM_GetConfig` and `bridle_SystemCall` on every path that is not a refusal, and that record is
    /// absent on this box. The ledger row for the delivered body names the missing record per slot.
    #[inline(never)]
    pub(crate) fn isockport(
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
        // Two objects answer this one 26-slot table, and the address says which: the app-facing
        // `AEECLSID_SockPort` record goes to its own machine, everything else is the loopback provider
        // whose arms are below. The question is asked here rather than in the dispatch arm because an
        // arm's `Result` temporary costs the recursive down-call frame 224 bytes it does not have.
        if self.port_base(this).is_some() {
            return self.port_slot(id, this, label, name);
        }
        // A fault deferred out of a release comes out here, before this call is answered, so nothing
        // that happened on the way to it is presented as clean.
        if let Some(err) = self.sock_pending.take() {
            return Err(*err);
        }
        let Some(slot) = self.sock_objects.get(&this).copied() else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                // `AEE_CheckPtr(0, ppv, 4, 1)` is the body's first line and answers `AEE_EMEMPTR` —
                // and unlike the default arm it does **not** clear the caller's slot.
                if ppv == 0 {
                    return Ok(g::AEE_EMEMPTR);
                }
                // Five ids, one object: `IQI`, `ISource`, `IPort`, and the two socket class numbers.
                // The hit takes a reference and the miss writes 0 first, both of which is what
                // [`Runtime::answer_qi`] does; this object has no second face, so the pointer never
                // moves — which is what separates it from `AEESockPort`'s own `QueryInterface`.
                self.answer_qi(
                    this,
                    iid,
                    ppv,
                    &[
                        socket::IID_IQI,
                        socket::CLS_SOURCE,
                        socket::CLS_PORT,
                        socket::CLS_HOSTSOCKET,
                        socket::CLS_SOCKPORT,
                    ],
                )
                .map_err(Error::Hle)
            }
            // One load of `[+180]`. Every refusal in this table lands there, so this is the slot that
            // carries most of the member's real answers.
            "GetLastError" => Ok(self.host_sockets.last_error(slot)),
            // Four-byte bodies: `return 20`, no argument read, and nothing written to `[+180]`.
            "Open" | "SelectNetwork" => Ok(socket::Manager::unnamed().ret as u32),
            "OpenEx" => {
                let [family, protocol] = self.args()?;
                let done = self.host_sockets.open_ex(slot, family as u16, protocol);
                self.answer_sock(done)
            }
            "Bind" => {
                let [paddr] = self.args()?;
                let (family, port, addr) = self.read_sock_name(paddr)?;
                let ok = self.sock_range(paddr, 8);
                let done = self.host_sockets.bind(slot, family, port, addr, ok);
                self.answer_sock(done)
            }
            "Listen" => {
                let [backlog] = self.args()?;
                let done = self.host_sockets.listen(slot, backlog as i32);
                self.answer_sock(done)
            }
            "Connect" => {
                let [paddr] = self.args()?;
                let (family, port, addr) = self.read_sock_name(paddr)?;
                let ok = self.sock_range(paddr, 8);
                let result = self.host_sockets.connect(slot, family, port, addr, ok);
                match result {
                    Ok(connected) => {
                        // The server-side socket exists in the model from here; its guest address is
                        // built only when something is handed a pointer to it, which is `Accept`.
                        self.answer_sock(connected.done)
                    }
                    Err(done) => self.answer_sock(done),
                }
            }
            "Accept" => {
                let [ppsock] = self.args()?;
                let ok = self.sock_range(ppsock, 4);
                // The body writes 0 into the caller's slot **before** the state gates, so every refusal
                // but the pointer one leaves a null behind.
                if ok {
                    self.cpu.mem.write_u32(ppsock, 0)?;
                }
                match self.host_sockets.accept(slot, ok) {
                    socket::Accept2::Taken(server) => {
                        // `*a2 = queue[0]` and the slot is cleared — the queued reference *moves* to
                        // the caller, so this is a plain write, not `return_object`'s extra AddRef.
                        let addr = self.sockport_object(server)?;
                        self.cpu.mem.write_u32(ppsock, addr)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    socket::Accept2::Blocked(done) | socket::Accept2::Refused(done) => {
                        self.answer_sock(done)
                    }
                }
            }
            // `Read`/`Write` pack one `{ptr, len}` entry with the length clamped to 16 bits — `0xffff`
            // for anything larger, not zero — and hand it to the vector form; `ReadV`/`WriteV` fill or
            // drain the same ring entry by entry, stopping at the first entry they cannot complete.
            "Read" | "ReadV" | "Write" | "WriteV" => {
                let [first, second] = self.args()?;
                let read = name == "Read" || name == "ReadV";
                let vector = name == "ReadV" || name == "WriteV";
                let entries = if vector { second as usize } else { 1 };
                let mut plan: Vec<(u32, usize)> = Vec::new();
                if vector {
                    // `AEE_CheckPtr(0, iov, 8*n, …)` over the block first, then over each entry.
                    let block_ok = self.sock_range(first, entries.saturating_mul(8));
                    let mut all_ok = block_ok;
                    for n in 0..entries {
                        let at = first + 8 * n as u32;
                        let ptr = self.cpu.mem.read_u32(at).unwrap_or(0);
                        let len = self.cpu.mem.read_u16(at + 4).unwrap_or(0) as usize;
                        if !self.sock_range(ptr, len) {
                            all_ok = false;
                        }
                        plan.push((ptr, len));
                    }
                    if !all_ok {
                        let done = if read {
                            self.host_sockets.read_into(slot, 0, false).0
                        } else {
                            self.host_sockets.write_from(slot, &[], false)
                        };
                        return self.answer_sock(done);
                    }
                } else {
                    let want = if second <= 0xffff {
                        second as usize
                    } else {
                        0xffff
                    };
                    plan.push((first, want));
                }
                let total: usize = plan.iter().map(|(_, len)| *len).sum();
                if read {
                    let buffer_ok = plan.iter().all(|(ptr, len)| self.sock_range(*ptr, *len));
                    let (done, bytes) = self.host_sockets.read_into(slot, total, buffer_ok);
                    let mut sent = 0usize;
                    for (ptr, len) in plan {
                        let take = len.min(bytes.len().saturating_sub(sent));
                        if take == 0 {
                            break;
                        }
                        self.cpu.mem.write_bytes(ptr, &bytes[sent..sent + take])?;
                        sent += take;
                    }
                    // Only a delivered answer is a byte count: a refusal has already put -1 in `r0`,
                    // and overwriting it with the zero bytes copied would turn `EBADF` into success.
                    let mut done = done;
                    if done.ret >= 0 {
                        done.ret = sent as i32;
                    }
                    return self.answer_sock(done);
                }
                let mut data = Vec::new();
                let buffer_ok = plan.iter().all(|(ptr, len)| self.sock_range(*ptr, *len));
                for (ptr, len) in plan {
                    data.extend_from_slice(&self.cpu.mem.read_bytes(ptr, len)?);
                }
                let written = self.host_sockets.write_from(slot, &data, buffer_ok);
                self.answer_sock(written)
            }
            "SendTo" => {
                let [buf, cb, _flags, paddr] = self.args()?;
                // The body hands the caller's own `cbBuf` to `AEE_CheckPtr` and to `aee_malloc`, so there is
                // no 16-bit clamp here the way `Read`/`Write` have one; a length above what is mapped is
                // refused by the range test, and a negative one arrives as a very large unsigned.
                let len = cb as usize;
                let ok = self.sock_range(buf, len);
                let payload = if ok {
                    self.cpu.mem.read_bytes(buf, len)?
                } else {
                    Vec::new()
                };
                let (family, port, addr) = self.read_sock_name(paddr)?;
                let sent = self
                    .host_sockets
                    .send_to(slot, &payload, ok, family, port, addr);
                match sent {
                    socket::Send::Delivered(done)
                    | socket::Send::Dropped { done }
                    | socket::Send::Answer(done) => self.answer_sock(done),
                }
            }
            "RecvFrom" => {
                let [buf, cb, _flags, paddr, pnsize] = self.args()?;
                // The same reading as `SendTo`: the body compares the queue record's own length against
                // the caller's `cbBuf` and only then truncates, so the value is passed through unclamped.
                // A negative `cbBuf` is undefined in the body (it memmoves a negative length); here the
                // message is delivered whole, which is the nearest thing to what the guest would get.
                let want = cb as usize;
                let ok = self.sock_range(buf, want);
                let size_ok = self.sock_range(pnsize, 4);
                let size = if size_ok {
                    self.cpu.mem.read_u32(pnsize)?
                } else {
                    0
                };
                let addr_ok = self.sock_range(paddr, 8);
                let (done, message) =
                    self.host_sockets
                        .recv_from(slot, want, ok, size >= 8, addr_ok);
                if let Some(message) = message {
                    self.cpu.mem.write_bytes(buf, &message.payload)?;
                    // The from-address block is written only by the success branch; the 515 refusal
                    // happens after the payload has already been copied out of the queue.
                    if done.ret >= 0 && message.from_family != socket::FAMILY_LOCAL {
                        self.cpu.mem.write_u32(
                            paddr,
                            (message.from_family as u32) | ((message.from_port as u32) << 16),
                        )?;
                        self.cpu.mem.write_u32(paddr + 4, message.from_addr)?;
                        self.cpu.mem.write_u32(pnsize, 8)?;
                    }
                }
                self.answer_sock(done)
            }
            "GetSockName" | "GetPeerName" => {
                let [paddr, pnsize] = self.args()?;
                let size_ok = self.sock_range(pnsize, 4);
                let size = if size_ok {
                    self.cpu.mem.read_u32(pnsize)?
                } else {
                    0
                };
                let addr_ok = self.sock_range(paddr, 8);
                let (done, name) = if name == "GetSockName" {
                    self.host_sockets.get_sock_name(slot, size, addr_ok)
                } else {
                    self.host_sockets.get_peer_name(slot, size, addr_ok)
                };
                if let Some(name) = name {
                    self.cpu
                        .mem
                        .write_u32(paddr, (name.family as u32) | ((name.port as u32) << 16))?;
                    self.cpu.mem.write_u32(paddr + 4, name.addr)?;
                    self.cpu.mem.write_u32(pnsize, 8)?;
                }
                self.answer_sock(done)
            }
            "GetOpt" | "SetOpt" => {
                // Both are twelve-byte bodies: `*(me + 180) = 546; return -1`. No argument is read at
                // all, so a caller that passed a null option buffer is refused exactly like one that
                // passed a good one.
                let done = if name == "GetOpt" {
                    self.host_sockets.get_opt(slot)
                } else {
                    self.host_sockets.set_opt(slot)
                };
                self.answer_sock(done)
            }
            "IOCtl" => {
                // `*(me + 180) = 20; return -1` — the option number is never looked at.
                let done = self.host_sockets.ioctl(slot);
                self.answer_sock(done)
            }
            "Shutdown" => {
                let [how] = self.args()?;
                let done = self.host_sockets.shutdown(slot, how as i32);
                self.answer_sock(done)
            }
            "Close" => {
                let done = self.host_sockets.close(slot);
                self.answer_sock(done)
            }
            // `Readable`/`Writeable` read `pcb[+16]` and stop if it is zero; otherwise they resume the
            // callback at once when the predicate already says yes, and park it on the chain when it
            // does not. The ABI declares both `void`, so only the chain and the error word are contract.
            "Readable" | "Writeable" => {
                let [pcb] = self.args()?;
                let read = name == "Readable";
                if pcb == 0 || !self.sock_range(pcb, 28) {
                    // `AEE_CheckPtr(0, pcb, 28, 1)`: the body refuses with the **errno in r0**, not -1.
                    self.host_sockets.touch_error(slot, socket::EFAULT);
                    return Ok(socket::EFAULT);
                }
                let pfn = self.cpu.mem.read_u32(pcb + 16)?;
                let arm = if read {
                    self.host_sockets.readable(slot, pcb as usize, pfn)
                } else {
                    self.host_sockets.writeable(slot, pcb as usize, pfn)
                };
                match arm {
                    socket::Arm::Ignored => Ok(0),
                    socket::Arm::Parked => Ok(0),
                    socket::Arm::Resume => {
                        // The block is the caller's own, and the delivered `ISocket_Readable` ends in
                        // the same `AEE_ResumeCallback` every other arming route does — so it is
                        // gated, cancelled out of wherever it was, and stamped like one, rather than
                        // pushed straight onto the list.
                        self.dispatcher_resume(pcb, false)?;
                        Ok(0)
                    }
                    socket::Arm::Refused(error) => Ok(error),
                }
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The record a `this` names, whichever face it came in on.
    #[inline(never)]
    /// The INET half of `ISockPort::OpenEx` (the delivered body), on its own because
    /// `INetMgr::OpenSocket` opens the very same socket through a different door: the header says
    /// it "creates a socket and returns a pointer to the ISocket Interface", and the two types it names
    /// — `AEE_SOCK_STREAM = 0`, `AEE_SOCK_DGRAM = 1` — are the protocol word this
    /// branch already takes, down to the same `>= 2` refusal.
    ///
    /// One body for both doors is the point: a socket obtained from the manager and one obtained by
    /// calling `OpenEx` on a port the pack made itself must be the same object with the same gates
    /// (`[+176]`'s datagram answers, `IOCtl` 4097..4105, and the provider behind them).
    pub(crate) fn port_open_inet(&mut self, base: u32, protocol: u16) -> Result<u32, Error> {
        if let Some(slot) = self.ports.get_mut(&base) {
            slot.family = socket::FAMILY_INET;
        }
        // `if (a3 >= 2) { errno = EINVAL; return -1; }`.
        if protocol >= 2 {
            return self.port_refuse(base, socket::EINVAL);
        }
        // The OEM record is still built — `is_inet` and `IOCtl` 4097..4105 read it — but the provider
        // behind it is this host's own, so the OEM entry points become real ones instead of dead ends.
        let host_addr = self.port_provider(base, socket::FAMILY_INET, protocol as u32)?;
        if host_addr == 0 {
            return Ok(u32::MAX);
        }
        let Ok(record) = self.hle.alloc(&mut self.cpu.mem, 112) else {
            self.sock_died(host_addr);
            if let Some(slot) = self.ports.get_mut(&base) {
                slot.host = None;
            }
            return self.port_refuse(base, socket::ENOMEM);
        };
        self.cpu.mem.write_u32(record, u32::MAX)?;
        self.cpu.mem.write_u32(record + 4, 0)?;
        self.cpu.mem.write_u32(record + 28, 30_000)?;
        self.cpu.mem.write_u32(base + 172, record)?;
        self.cpu.mem.write_u8(base + 176, protocol as u8)?;
        if let Some(slot) = self.ports.get_mut(&base) {
            // `[+176]` is the protocol byte, and the datagram gates read it (`GetPeerName`, `Accept`
            // and `Connect` answer `EOPNOTSUPP` when it is non-zero). Only this branch writes it — a
            // local record's byte stays whatever it was ( against the local path above).
            slot.kind = protocol as u8;
            slot.net = Some(record);
        }
        Ok(g::AEE_SUCCESS)
    }

    /// The `ISocket` face of an INET port: `base + 4`, adopted once so its `AddRef`/`Release` land on the
    /// record (`AEEBase_*` reads `[me + 4]`, which for the face is `[+12]` of the record — so the face's
    /// count is the record's count four bytes earlier). Two doors hand it out: `QueryInterface`
    /// (the delivered body) and `INetMgr::OpenSocket`, and each takes
    /// the reference the caller then owns.
    pub(crate) fn port_face(&mut self, base: u32) -> HResult<u32> {
        let face = base + 4;
        if !self.port_faces.contains_key(&face) {
            self.hle
                .adopt_object(face, "ISocket", socket::CLS_SOCKET, 144);
            self.port_faces.insert(face, base);
        }
        Ok(face)
    }

    /// Give this port a provider and open it in `family`.
    ///
    /// The local family asks the shell for one in the body — `QI(shell, AEECLSID_HostSocket, &[+172])`
    /// then `OpenEx` on it — and an INET record asks the OEM net module instead. This
    /// host substitutes the same provider for that second family, which is the only way
    /// [`socket::Manager`]'s real datagram (`policy.real && family == INET`) is ever built. `0` is the
    /// refusal, already written to the port's error cell with `-1` as the answer.
    pub(crate) fn port_provider(
        &mut self,
        base: u32,
        family: u16,
        protocol: u32,
    ) -> Result<u32, Error> {
        let Ok(host_slot) = self.host_sockets.born(self.shell, socket::CLS_HOSTSOCKET) else {
            return self.port_refuse(base, socket::AEE_ENOMEMORY).map(|_| 0);
        };
        let host_addr = self.sockport_object(host_slot)?;
        let done = self.host_sockets.open_ex(host_slot, family, protocol);
        let answer = self.answer_sock(done)?;
        if answer == u32::MAX {
            // `errno = provider->GetLastError; return <its code>`.
            let errno = self.host_sockets.last_error(host_slot);
            self.sock_died(host_addr);
            self.port_refuse(base, errno)?;
            return Ok(0);
        }
        if let Some(slot) = self.ports.get_mut(&base) {
            slot.host = Some(host_slot);
        }
        Ok(host_addr)
    }

    /// Answer through the port's provider when it has one.
    ///
    /// The local family's body calls the provider directly; the INET body calls the OEM net module at
    /// the same point, and a port with a provider answers from it instead. `None` means there is no
    /// provider, so the caller keeps its own refusal.
    pub(crate) fn provider_slot(
        &mut self,
        id: u32,
        base: u32,
        label: &str,
        name: &str,
    ) -> Option<Result<u32, Error>> {
        let (host_slot, host) = self.port_host(base)?;
        let answer = self.isockport(id, host, label, name);
        // The body copies the provider's answer into the record's error cell on a refusal
        // (`errno = provider->GetLastError; return -1`), and `GetLastError` for an INET record
        // reads that cell.
        if matches!(&answer, Ok(a) if *a == u32::MAX) {
            let errno = self.host_sockets.last_error(host_slot);
            self.port_error(base, errno);
        }
        Some(answer)
    }

    pub(crate) fn port_base(&self, this: u32) -> Option<u32> {
        self.port_faces
            .get(&this)
            .copied()
            .or_else(|| self.ports.contains_key(&this).then_some(this))
    }

    /// Write `[+178]` and answer the reference `return -1`.
    pub(crate) fn port_refuse(&mut self, base: u32, errno: u32) -> Result<u32, Error> {
        if let Some(port) = self.ports.get_mut(&base) {
            port.error = errno as u16;
        }
        Ok(u32::MAX)
    }

    pub(crate) fn port_error(&mut self, base: u32, errno: u32) {
        if let Some(port) = self.ports.get_mut(&base) {
            port.error = errno as u16;
        }
    }

    /// `(+172)`'s provider, as the guest address its 26-slot table was handed out under. The local
    /// branch of every data slot is one indirect call into that object, so the arms below re-enter
    /// [`Runtime::isockport`] with this address instead of copying the provider's bodies.
    pub(crate) fn port_host(&mut self, base: u32) -> Option<(usize, u32)> {
        let slot = self.ports.get(&base)?.host?;
        let addr = self.sock_slots.get(&slot).copied()?;
        Some((slot, addr))
    }

    /// The guest address behind `*(port + 172)` — the record the face parks a connect name into. For
    /// `AF_LOCAL` that is the provider's own `IHostSocket` object; for `AF_INET` it is the 112-byte OEM
    /// record, which `OpenEx` builds *beside* the provider rather than instead of it. Both are guest
    /// memory, which is what makes the parked words something a pack can read back and re-use.
    pub(crate) fn port_record(&mut self, base: u32) -> Option<u32> {
        let record = self.ports.get(&base)?.clone();
        if let Some(net) = record.net {
            return Some(net);
        }
        let slot = record.host?;
        self.sock_slots.get(&slot).copied()
    }

    pub(crate) fn sock_port_try_connect_stub(&mut self) -> Result<u32, Error> {
        let port = self.cpu.r[0];
        let Some(record) = self.ports.get(&port).copied() else {
            return Ok(0);
        };
        let status = match record.host {
            Some(slot) => match self.port_record(port) {
                Some(parked) => {
                    let name = self.cpu.mem.read_u32(parked + 20)?;
                    let family = name as u16;
                    let number = (name >> 16) as u16;
                    let addr = self.cpu.mem.read_u32(parked + 24)?;
                    let attempt = self.host_sockets.connect(slot, family, number, addr, true);
                    let done = match attempt {
                        Ok(connected) => connected.done,
                        Err(done) => done,
                    };
                    let refused = done.ret == -1;
                    let error = done.error;
                    self.answer_sock(done)?;
                    if refused {
                        error.unwrap_or(socket::ENOTCONN)
                    } else {
                        0
                    }
                }
                None => {
                    // The face parks its name in the provider's record; with no record there is no
                    // parked name to attempt with, and nothing is invented to stand in for one.
                    self.port_refuse(port, socket::EBADF)?;
                    socket::EBADF
                }
            },
            None => {
                // No provider behind the record: never opened, or already closed. The queue holds an
                // address rather than a reference, so a port that died between the face's call and this
                // resume lands here, and `EBADF` is what the record's own `IS_OPEN` gate answers.
                self.port_refuse(port, socket::EBADF)?;
                socket::EBADF
            }
        };
        self.cpu.mem.write_u32(port + 104, status)?;
        let leg = self.host_fn(HostFn::SockPortConnectStatus)?;
        self.cpu.mem.write_u32(port + 84, leg)?;
        self.cpu.mem.write_u32(port + 88, port)?;
        // ⚠ The delivered leg brackets its re-arm with two `CALLBACK_Cancel(port + 68)` calls, which on a
        // handset unlink the block from the net object's list. Nothing here links this block into a list —
        // it only ever sits in the resume queue — so those cancels would unlink nothing and are not
        // copied; what *is* copied is the arm-then-resume order and the single notice that follows.
        self.dispatcher.resume_from(port + 68, self.applet);
        Ok(status)
    }

    pub(crate) fn sock_port_connect_status_stub(&mut self) -> Result<u32, Error> {
        let port = self.cpu.r[0];
        let pfn = self.cpu.mem.read_u32(port + 96)?;
        if pfn == 0 {
            return Ok(port);
        }
        self.cpu.mem.write_u32(port + 96, 0)?;
        let user = self.cpu.mem.read_u32(port + 100)?;
        let status = self.cpu.mem.read_u32(port + 104)?;
        self.call_guest(pfn, &[user, status])
    }

    /// The 180-byte record and its two tables, without the reference count's first increment — used by
    /// `Accept`, which builds a port per accepted connection the way does.
    #[inline(never)]
    pub(crate) fn port_new_object(&mut self) -> Result<u32, Error> {
        let table = self.sockport_table()?;
        let face = self.hle.vtable_named(&mut self.cpu.mem, "ISocket")?;
        let addr = self.hle.new_object(
            &mut self.cpu.mem,
            "ISockPort",
            socket::CLS_SOCKPORT,
            180,
            148,
            &[],
        )?;
        self.cpu.mem.write_u32(addr, table)?;
        self.cpu.mem.write_u32(addr + 4, face)?;
        self.cpu.mem.write_u32(addr + 8, addr)?;
        self.ports.insert(addr, sockport::Port::default());
        Ok(addr)
    }

    #[inline(never)]
    pub(crate) fn port_born(&mut self, ppv: u32) -> Result<u32, Error> {
        if !self.sock_range(ppv, 4) {
            return Ok(g::AEE_EMEMPTR);
        }
        self.cpu.mem.write_u32(ppv, 0)?;
        let addr = self.port_new_object()?;
        self.cpu.mem.write_u32(ppv, addr)?;
        Ok(g::AEE_SUCCESS)
    }

    /// The last release of a port, from either face.
    ///
    /// `ISockPort_FinishRelease` cancels the callbacks, runs `Unrealize` (which releases the
    /// provider), gives back the shell reference `New` took and frees the record. Here the shell's
    /// reference is the object layer's, the provider's close is [`Runtime::sock_died`], and the
    /// 112-byte INET record is the one block this object owns outright.
    #[inline(never)]
    pub(crate) fn port_died(&mut self, addr: u32) {
        let base = self.port_faces.get(&addr).copied().unwrap_or(addr);
        self.port_faces.remove(&base);
        self.port_faces.remove(&(base + 4));
        self.hle.forget(base + 4);
        // A socket that came out of `INetMgr::OpenSocket` was never handed to anybody as an
        // `ISockPort`, so the release that ends it arrives on the **face** — and the record's address is
        // not the object the release path is forgetting. Let it go here, or the record stays registered
        // with a count of zero and nothing in the guest can ever reach it.
        if base != addr {
            self.hle.forget(base);
        }
        let Some(port) = self.ports.remove(&base) else {
            return;
        };
        // The record is going back to the heap, and its **embedded** connect callback lives inside it at
        // `+68`: the queue holds addresses, so a pending resume naming that block would run whatever
        // lands there next. This is the case that needs the queue cleared — unlike a source's chain,
        // where the parked word is the pack's own block and the host has no right to drop it.
        self.dispatcher.forget(base + 68);
        if let Some(host) = port.host {
            match self.sock_slots.get(&host).copied() {
                Some(object) => self.sock_died(object),
                None => self.host_sockets.forget(host),
            }
        }
        if let Some(record) = port.net {
            self.hle.dealloc(&mut self.cpu.mem, record);
        }
    }

    pub(crate) const SSL_LAYOVER: [u32; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 8, 11];

    #[inline(never)]
    pub(crate) fn ssl_table(&mut self) -> Result<u32, Error> {
        if self.ssl_table == 0 {
            self.ssl_table =
                self.hle
                    .vtable_ordered(&mut self.cpu.mem, "ISSL", &Self::SSL_LAYOVER)?;
        }
        Ok(self.ssl_table)
    }

    /// `me[+116]` while the handshake is running and when it has completed (,).
    pub(crate) const SSL_IN_PROGRESS: u32 = 2569;
    pub(crate) const SSL_SUCCESS: u32 = 2560;
    /// The answer `Negotiate` gives when the state is already 512.
    pub(crate) const SSL_BUSY: u32 = 2578;

    /// The `host_sockets` slot behind a guest `ISocket`, when it is one of this host's. A module's own
    /// socket implementation would need a down-call per read and write; that is the other half of this
    /// seam and is not here.
    pub(crate) fn ssl_socket_slot(&self, sock: u32) -> Option<usize> {
        if let Some(slot) = self.sock_objects.get(&sock).copied() {
            return Some(slot);
        }
        let base = self.port_base(sock)?;
        self.ports.get(&base)?.host
    }

    /// Drive the handshake as far as the socket allows, one bounded round per call.
    ///
    /// This is the `SSL_DoHandshake` the delivered body arms through `(*socket)[+56]`
    /// (`ISocket::Writeable`,): write what the session produced, read what the peer left, feed
    /// it back, and stop when a round moves nothing. Returns whether the handshake has completed.
    #[inline(never)]
    pub(crate) fn tls_pump(&mut self, base: u32) -> Result<bool, Error> {
        let Some(sock) = self.ssl.get(&base).map(|a| a.socket) else {
            return Ok(false);
        };
        let Some(slot) = self.ssl_socket_slot(sock) else {
            return Ok(false);
        };
        if self.ssl.get(&base).is_some_and(|a| a.session.is_none()) {
            let roots = self
                .ssl_roots
                .clone()
                .map(|roots| roots.store())
                .unwrap_or_else(tls::platform_roots);
            let session = tls::Session::client_chain_only("ssl", roots).map_err(|e| {
                Error::Hle(HleError::Unimplemented {
                    id: 0,
                    label: format!("tls: {e:?}"),
                    this: base,
                })
            })?;
            if let Some(record) = self.ssl.get_mut(&base) {
                record.session = Some(session);
            }
        }
        for _ in 0..64 {
            let mut moved = false;
            // Whatever the socket did not take last round goes first: a stream write may carry less
            // than it was handed, and the remainder is still the handshake's.
            let mut out = match self.ssl.get_mut(&base) {
                Some(record) => std::mem::take(&mut record.pending),
                None => return Ok(false),
            };
            match self.ssl.get_mut(&base).and_then(|a| a.session.as_mut()) {
                Some(session) => out.extend_from_slice(&session.take()),
                None => return Ok(false),
            }
            if !out.is_empty() {
                let done = self.host_sockets.write_from(slot, &out, true);
                if done.ret > 0 {
                    out.drain(..done.ret as usize);
                    moved = true;
                }
            }
            if let Some(record) = self.ssl.get_mut(&base) {
                record.pending = out;
            }
            let (_, bytes) = self.host_sockets.read_into(slot, 4096, true);
            if !bytes.is_empty() {
                if let Some(session) = self.ssl.get_mut(&base).and_then(|a| a.session.as_mut()) {
                    session.feed(&bytes).map_err(|e| {
                        Error::Hle(HleError::Unimplemented {
                            id: 0,
                            label: format!("tls handshake: {e:?}"),
                            this: base,
                        })
                    })?;
                }
                moved = true;
            }
            let done = self
                .ssl
                .get(&base)
                .and_then(|a| a.session.as_ref())
                .map(|s| !s.handshaking())
                .unwrap_or(false);
            if done {
                return Ok(true);
            }
            if !moved {
                return Ok(false);
            }
        }
        Ok(false)
    }

    /// Write the result into `[+116]`, the caller's `SSLInfo` and the pending callback.
    pub(crate) fn ssl_finish(&mut self, base: u32, code: u32) -> Result<(), Error> {
        let (info, cb) = match self.ssl.get_mut(&base) {
            Some(record) => {
                record.status = code;
                (record.info, record.cb)
            }
            None => return Ok(()),
        };
        self.cpu.mem.write_u32(base + 116, code)?;
        if code == Self::SSL_SUCCESS {
            if let Some(record) = self.ssl.get_mut(&base) {
                record.state = 768;
            }
            self.cpu.mem.write_u32(base + 176, 768)?;
            for (at, read) in [(40, true), (76, false)] {
                let pfn = self.cpu.mem.read_u32(base + at)?;
                if pfn != 0 {
                    let user = self.cpu.mem.read_u32(base + at + 4)?;
                    self.ssl_arm_socket(base, read, pfn, user)?;
                }
            }
        }
        if info != 0 && self.sock_range(info, 4) {
            self.cpu.mem.write_u32(info, code)?;
        }
        if cb != 0 {
            // `record.cb` is the caller's own block, parked through `ISSL_Connect`, so it is queued
            // through the one door every pack block goes through: a block with no notify function is
            // not queued, whatever hook it still carries is run first, and it leaves stamped — which
            // is what makes `CALLBACK_IsQueued` true and `CALLBACK_Cancel` able to withdraw it before
            // the dispatch gets there.
            if self.dispatcher_prepare(cb)? {
                let owner = self.applet;
                self.dispatcher.resume_from(cb, owner);
            }
        }
        Ok(())
    }

    #[inline(never)]
    pub(crate) fn ssl_born(&mut self, ppv: u32) -> Result<u32, Error> {
        if !self.sock_range(ppv, 4) {
            return Ok(g::AEE_EBADPARM);
        }
        self.cpu.mem.write_u32(ppv, 0)?;
        let table = self.ssl_table()?;
        let face = self.hle.vtable_named(&mut self.cpu.mem, "ISocket")?;
        let addr =
            self.hle
                .new_object(&mut self.cpu.mem, "ISSL", g::AEECLSID_SSL_313, 252, 4, &[])?;
        self.cpu.mem.write_u32(addr, table)?;
        self.cpu.mem.write_u32(addr + 8, self.shell)?;
        self.cpu.mem.write_u32(addr + 20, face)?;
        // `ISSL_Socket_addref@0x0` is `(**[face + 4])([face + 4])`: the word four bytes into the
        // face subobject is a **self** pointer back to the record, not a second count. The count is
        // at `+4` and the face shares it because its AddRef/Release reach the record's.
        self.cpu.mem.write_u32(addr + 24, addr)?;
        self.cpu.mem.write_u32(addr + 176, 256)?;
        self.ssl.insert(
            addr,
            tls::Abi {
                state: 256,
                ..Default::default()
            },
        );
        self.cpu.mem.write_u32(ppv, addr)?;
        Ok(g::AEE_SUCCESS)
    }

    /// Write the face's own error word at `record + 36` and keep the host-side mirror of it.
    pub(crate) fn ssl_face_errno(&mut self, base: u32, errno: u32) -> Result<(), Error> {
        if let Some(record) = self.ssl.get_mut(&base) {
            record.last_error = errno as i32;
        }
        Ok(self.cpu.mem.write_u16(base + 36, errno)?)
    }

    pub(crate) fn ssl_face_error(&mut self, base: u32, errno: u32) -> Result<u32, Error> {
        self.ssl_face_errno(base, errno)?;
        Ok(u32::MAX)
    }

    /// The state gate's refusal, which both bodies spell the same way as "the SSL layer wants a
    /// direction": **516** into the face's `int16` and **`-2`** back. The want word at
    /// `record + 236` is *not* touched on this path — the body stores to it only inside the
    /// `get_error` switch — so a caller can tell the two refusals apart by that word alone.
    pub(crate) fn ssl_face_not_ready(&mut self, base: u32) -> Result<u32, Error> {
        self.ssl_face_errno(base, socket::EWOULDBLOCK)?;
        Ok(u32::MAX - 1)
    }

    pub(crate) fn ssl_face_would_read(&mut self, base: u32) -> Result<u32, Error> {
        self.cpu.mem.write_u32(base + 236, 2)?;
        self.ssl_face_not_ready(base)
    }

    /// Hand a notify pair to the socket underneath, which is where both readiness slots end:
    /// `(*(**(record + 16) + 8 or 56))(**(record + 16), pfn, pv)` — the socket's own `Readable`
    /// (slot 2) or `Writeable` (slot 14), with the caller's pair and nothing else. The socket comes out
    /// of the record's own word at `[+16]`, and its table out of `[*sock]`, because that second load is
    /// what faults on a handset when `SetSocket` never ran; this host raises the same memory fault
    /// rather than answering politely.
    ///
    /// The two shapes below are the host's, not the body's, and both come from one missing thing: no
    /// address in this host is a function a pack can call. A socket the module built is asked through
    /// its own table, which is the reference call. A socket this host built is [`Runtime::host_sockets`]
    /// state, whose chains take a callback **block** and are woken by the runtime — so the record goes
    /// on the chain in the block's place and the pair waits in [`tls::Abi`] for the wake that reads it.
    /// The address parked differs by direction (the record for readable, its `ISocket` face at
    /// `record + 20` for writeable) because a chain carries one word and two directions have to be told
    /// apart when it fires; both map back to the record through [`Runtime::ssl_base`]'s two tables.
    #[inline(never)]
    pub(crate) fn ssl_arm_socket(
        &mut self,
        base: u32,
        read: bool,
        pfn: u32,
        pv: u32,
    ) -> Result<(), Error> {
        let sock = self.cpu.mem.read_u32(base + 16)?;
        self.cpu.mem.read_u32(sock)?;
        let Some(slot) = self.ssl_socket_slot(sock) else {
            let at =
                self.guest_slot(sock, "ISocket", if read { "Readable" } else { "Writeable" })?;
            self.call_guest(at, &[sock, pfn, pv])?;
            return Ok(());
        };
        if pfn == 0 {
            // `IHostSocket_Readable`/`Writeable` read the notify word of what they are given and stop
            // when it is zero, so nothing is parked — and parking the record anyway would hand a later
            // event a function pointer of zero to call through.
            return Ok(());
        }
        if let Some(record) = self.ssl.get_mut(&base) {
            if read {
                record.armed_read = Some((pfn, pv));
            } else {
                record.armed_write = Some((pfn, pv));
            }
        }
        let armed = if read {
            self.host_sockets.readable_source(slot, base as usize)
        } else {
            self.host_sockets
                .writeable_source(slot, (base + 20) as usize)
        };
        // `Arm::Resume` is the socket saying the bytes are already there. The reference provider resumes
        // on the spot; queueing here is the order the chain itself keeps — empty first, then the wake —
        // and it keeps the pack out of the frame that is still inside its own `Readable`.
        if matches!(armed, socket::Arm::Resume) {
            self.ssl_wake(base, read)?;
        }
        Ok(())
    }

    pub(crate) fn ssl_notify(&mut self, base: u32, pfn: u32, user: u32) -> Result<(), Error> {
        self.cpu.mem.write_u32(base + 64, pfn)?;
        self.cpu.mem.write_u32(base + 68, user)?;
        self.dispatcher.resume_from(base + 48, self.applet);
        Ok(())
    }

    #[inline(never)]
    pub(crate) fn ssl_wake(&mut self, base: u32, read: bool) -> Result<(), Error> {
        let Some((armed, state)) = self.ssl.get_mut(&base).map(|record| {
            let fired = if read {
                record.armed_read.take()
            } else {
                record.armed_write.take()
            };
            (fired, record.state)
        }) else {
            return Ok(());
        };
        if state != 768 && self.tls_pump(base)? {
            self.ssl_finish(base, Self::SSL_SUCCESS)?;
        }
        if let Some((pfn, user)) = armed {
            self.ssl_notify(base, pfn, user)?;
        }
        Ok(())
    }

    pub(crate) fn ssl_disarm(&mut self, base: u32, read: bool) {
        let Some(sock) = self.ssl.get(&base).map(|record| record.socket) else {
            return;
        };
        if let Some(slot) = self.ssl_socket_slot(sock) {
            self.host_sockets
                .cancel_entry(slot, if read { base } else { base + 20 } as usize);
        }
        if let Some(record) = self.ssl.get_mut(&base) {
            if read {
                record.armed_read = None;
            } else {
                record.armed_write = None;
            }
        }
    }

    /// The record a `this` names, from either face.
    pub(crate) fn ssl_base(&self, this: u32) -> Option<u32> {
        self.ssl_faces
            .get(&this)
            .copied()
            .or_else(|| self.ssl.contains_key(&this).then_some(this))
    }

    #[inline(never)]
    pub(crate) fn issl(
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
        let Some(base) = self.ssl_base(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if !self.sock_range(ppv, 4) {
                    return Err(Error::Hle(unimplemented()));
                }
                match iid {
                    g::AEEIID_IQI | g::AEECLSID_SSL_313 | g::AEECLSID_WEBOPTS => {
                        self.cpu.mem.write_u32(ppv, base)?;
                        self.hold(base, 1).map_err(Error::Hle)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    socket::CLS_SOCKET => {
                        let face = base + 20;
                        // No object of its own: the face's `[+4]` is the record's `self` word, so
                        // its reference slots have to reach the record's count. `object_method`
                        // resolves a face to the record through [`Runtime::ssl_faces`].
                        self.ssl_faces.insert(face, base);
                        self.cpu.mem.write_u32(ppv, face)?;
                        self.hold(base, 1).map_err(Error::Hle)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    _ => Ok(g::AEE_ECLASSNOTSUPPORT),
                }
            }
            "AddOpt" => {
                let [items] = self.args()?;
                self.ixopts_add_list(base, items).map_err(Error::Hle)
            }
            "RemoveOpt" => {
                let [nopt, index] = self.args()?;
                self.ixopts_remove(base, nopt as i32, index as i32)
                    .map_err(Error::Hle)
            }
            "GetOpt" => {
                let [nopt, index, pitem] = self.args()?;
                self.ixopts_get(base, nopt as i32, index as i32, pitem)
                    .map_err(Error::Hle)
            }
            "Negotiate" | "NegotiateV" | "Renegotiate" | "RenegotiateV" => {
                let [pinfo, pcerts, pcb] = self.args::<3>()?;
                if pinfo == 0 {
                    return Ok(base);
                }
                if self.ssl.get(&base).map(|a| a.state) == Some(512) {
                    if self.sock_range(pinfo, 4) {
                        self.cpu.mem.write_u32(pinfo, Self::SSL_BUSY)?;
                    }
                    if pcb != 0 {
                        self.dispatcher.resume_from(pcb, self.applet);
                    }
                    return Ok(base);
                }
                if let Some(record) = self.ssl.get_mut(&base) {
                    record.info = pinfo;
                    record.certs = pcerts;
                    record.cb = pcb;
                    record.status = Self::SSL_IN_PROGRESS;
                    record.state = 512;
                }
                self.cpu.mem.write_u32(base + 96, pcb)?;
                self.cpu.mem.write_u32(base + 108, pinfo)?;
                self.cpu.mem.write_u32(base + 112, pcerts)?;
                self.cpu.mem.write_u32(base + 116, Self::SSL_IN_PROGRESS)?;
                self.cpu.mem.write_u32(base + 176, 512)?;
                if self.tls_pump(base)? {
                    self.ssl_finish(base, Self::SSL_SUCCESS)?;
                }
                Ok(base)
            }
            "SetSocket" => {
                let [sock] = self.args()?;
                if sock == 0 {
                    return Ok(base);
                }
                self.ssl_reset(base)?;
                if self.ssl_socket_slot(sock).is_none() {
                    let addref = self.guest_slot(sock, "ISocket", "AddRef")?;
                    self.call_guest(addref, &[sock])?;
                }
                if let Some(record) = self.ssl.get_mut(&base) {
                    record.socket = sock;
                    record.state = 257;
                    record.session = None;
                }
                self.cpu.mem.write_u32(base + 16, sock)?;
                self.cpu.mem.write_u32(base + 176, 257)?;
                Ok(sock)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The `ISocket` face at `me + 20`. Its sixteen slots run the handshake over the socket
    /// `SetSocket` stored, so they are the same C as the negotiate slots until that seam exists; only
    /// `QueryInterface` is answered, and it goes through the record because the count at `+24` is
    /// shared.
    #[inline(never)]
    pub(crate) fn issl_socket(
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
        let Some(base) = self.ssl_base(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            "QueryInterface" => self.issl(id, base, label, "QueryInterface"),
            "Readable" | "Writeable" => {
                let [pfn, pv] = self.args::<2>()?;
                let read = name == "Readable";
                if self.ssl.get(&base).map(|a| a.state) != Some(768) && self.tls_pump(base)? {
                    self.ssl_finish(base, Self::SSL_SUCCESS)?;
                }
                if self.ssl.get(&base).map(|a| a.state) != Some(768) {
                    let at = if read { 40 } else { 76 };
                    self.cpu.mem.write_u32(base + at, pfn)?;
                    self.cpu.mem.write_u32(base + at + 4, pv)?;
                    return Ok(this);
                }
                if read {
                    let pending = match self.ssl.get_mut(&base) {
                        Some(record) => record
                            .session
                            .as_mut()
                            .map(|session| session.pending())
                            .unwrap_or(0),
                        None => 0,
                    };
                    if pfn != 0 && pending > 0 {
                        self.ssl_notify(base, pfn, pv)?;
                        return Ok(g::AEE_SUCCESS);
                    }
                    if pfn == 0 {
                        // `CALLBACK_Cancel(record + 48)`: the hook the block carries itself, which is
                        // nothing at all while nobody has armed it — the body's own no-op.
                        self.cancel_sock_pcb(base + 48)?;
                    }
                }
                let want = self.cpu.mem.read_u32(base + 236)?;
                let want_write = if read { want == 3 } else { want != 2 };
                self.ssl_arm_socket(base, !want_write, pfn, pv)?;
                self.cpu.mem.write_u32(base + 236, 0)?;
                Ok(g::AEE_SUCCESS)
            }
            "GetLastError" => Ok(self
                .ssl
                .get(&base)
                .map(|a| a.last_error as u32)
                .unwrap_or(0)),
            "Connect" => {
                let [_addr, _port, pfn, user] = self.args::<4>()?;
                self.call_guest(pfn, &[user, socket::EISCONN])?;
                Ok(0)
            }
            "Bind" | "SendTo" | "RecvFrom" => self.ssl_face_error(base, socket::EOPNOTSUPP),
            "GetPeerName" => {
                let [paddr, plen] = self.args::<2>()?;
                let Some(inner) = self.ssl.get(&base).map(|a| a.socket).filter(|s| *s != 0) else {
                    return Ok(socket::EBADF);
                };
                let slot = self.guest_slot(inner, "ISocket", "GetPeerName")?;
                self.call_guest(slot, &[inner, paddr, plen])
            }
            "IOCtl" => {
                let [id, pv] = self.args::<2>()?;
                let inner = self.ssl.get(&base).map(|a| a.socket).unwrap_or(0);
                match id {
                    // Copy the two words out and answer 0 — and it answers 0 **with a NULL out-parameter
                    // too**, so a caller that passed nothing still gets success without the copy.
                    4097 => {
                        if pv != 0 {
                            let a = self.cpu.mem.read_u32(base + 84)?;
                            let b = self.cpu.mem.read_u32(base + 88)?;
                            self.cpu.mem.write_u32(pv, a)?;
                            self.cpu.mem.write_u32(pv + 4, b)?;
                        }
                        Ok(0)
                    }
                    // Store the two words — or **clear** them when no pointer came, which is the whole
                    // of that branch: no arming, no forward. With a socket underneath (`record[+16]`,
                    // read as the body reads it) the rest of the branch arms the record's two close
                    // notices. First the embedded callback at `record + 208`: its notify pair is
                    // `+224/+228` (`pfnNotify`/`pNotifyData` of a seven-word `AEECallback`), filled
                    // with `HandleLowRAMEvent` and the record, then registered with the shell for
                    // `AEE_SCB_LOW_RAM`. Then `{ISSL_Socket_SockCloser, this face}` goes to the socket
                    // underneath as *its* `IOCtl` — on a handset that pair sits on the SSL
                    // caller's own stack, so it lives only for the call, and the socket is expected to
                    // take the two words out of it. The answer the caller gets is that socket's code,
                    // not `0`.
                    //
                    // ⚠ Neither trigger fires here: this host registers low-RAM and never issues it,
                    // and its socket layer has no close notice of its own to run. What is reproduced is
                    // the arming — the two parked words, the registration the record's own
                    // `CALLBACK_Cancel(record + 208)` can take back, and the notice handed down — so a
                    // pack sees the same record it would have written on a handset.
                    4098 => {
                        if pv == 0 {
                            self.cpu.mem.write_u32(base + 84, 0)?;
                            self.cpu.mem.write_u32(base + 88, 0)?;
                            return Ok(0);
                        }
                        let a = self.cpu.mem.read_u32(pv)?;
                        let b = self.cpu.mem.read_u32(pv + 4)?;
                        self.cpu.mem.write_u32(base + 84, a)?;
                        self.cpu.mem.write_u32(base + 88, b)?;
                        let sock = self.cpu.mem.read_u32(base + 16)?;
                        if sock == 0 {
                            return Ok(0);
                        }
                        let at = |name: &str| {
                            abi::offset_of("_AEECallback", name).unwrap_or_else(|| {
                                panic!("_AEECallback::{name} is not in the ABI tables")
                            })
                        };
                        let block = base + 208;
                        let low_ram = self.host_fn(HostFn::SslLowRam)?;
                        self.cpu.mem.write_u32(block + at("pfnNotify"), low_ram)?;
                        self.cpu.mem.write_u32(block + at("pNotifyData"), base)?;
                        self.register_system_callback(block, g::AEE_SCB_LOW_RAM)?;
                        let closer = self.host_fn(HostFn::SslSockCloser)?;
                        let pair = self.heap_alloc(8)?;
                        if pair == 0 {
                            // The delivered body needs no memory for this — the pair is its own stack
                            // frame. Nothing is handed down when the arena declines, and the words that
                            // were already stored stay, so the refusal is said rather than hidden
                            // behind the `0` this branch already answers.
                            self.display.note(format!(
                                "ISocket::IOCtl(4098) on record {base:#010x} cannot arm the close \
                                 notice: the arena will not give the eight bytes the pair needs"
                            ));
                            return Ok(0);
                        }
                        self.cpu.mem.write_u32(pair, closer)?;
                        self.cpu.mem.write_u32(pair + 4, this)?;
                        let slot = self.guest_slot(sock, "ISocket", "IOCtl")?;
                        let answer = self.call_guest(slot, &[sock, 4098, pair]);
                        self.hle.dealloc(&mut self.cpu.mem, pair);
                        answer
                    }
                    4102 => {
                        if pv != 0 {
                            let cls = self.cpu.mem.read_u32(pv)?;
                            if cls == g::AEECLSID_SOCKET {
                                self.cpu.mem.write_u32(pv + 4, this)?;
                                self.object_ref(base, true)?;
                                return Ok(0);
                            }
                            if cls == g::AEECLSID_SSL_313 {
                                self.cpu.mem.write_u32(pv + 4, base)?;
                                self.object_ref(base, true)?;
                                return Ok(0);
                            }
                        }
                        Ok(g::AEE_EBADPARM)
                    }
                    // A vendor word, not a class id: `0x5555FEED` hands back the boot timestamp the
                    // constructor parked at record `+248`. There is no guard on the out-parameter in the
                    // body, so a NULL one is the pack's own data abort — written through, not refused.
                    0x5555_feed => {
                        let stamp = self.cpu.mem.read_u32(base + 248)?;
                        self.cpu.mem.write_u32(pv, stamp)?;
                        Ok(0)
                    }
                    // Everything else is asked of the socket underneath — through *its* `ISocket::IOCtl`,
                    // slot 15 — and with nothing under it the body answers its own number — `14` = `AEE_EBADPARM`.
                    _ => {
                        if inner == 0 {
                            return Ok(g::AEE_EBADPARM);
                        }
                        let slot = self.guest_slot(inner, "ISocket", "IOCtl")?;
                        self.call_guest(slot, &[inner, id, pv])
                    }
                }
            }
            "Cancel" => {
                let [pfn, pv] = self.args::<2>()?;
                let clear_both = (pfn | pv) == 0;
                let sock = self.cpu.mem.read_u32(base + 16)?;
                // The socket underneath is read from the record's word and tested before use, which the
                // body does the same way: unlike the readiness slots this one never dereferences a null
                // socket, it just answers what it loaded.
                let ours = self.ssl_socket_slot(sock).is_some();
                let mut result = 0;
                if sock != 0 && !ours {
                    let at = self.guest_slot(sock, "ISocket", "Cancel")?;
                    result = self.call_guest(at, &[sock, pfn, pv])?;
                }
                // Writeable's pair first, then Readable's — the order the delivered compares run, which
                // is also the order that decides what r0 ends up holding.
                for (at, read) in [(76, false), (40, true)] {
                    if !clear_both {
                        result = self.cpu.mem.read_u32(base + at)?;
                        if result != pfn {
                            continue;
                        }
                        result = self.cpu.mem.read_u32(base + at + 4)?;
                        if result != pv {
                            continue;
                        }
                    }
                    self.cpu.mem.write_u32(base + at, 0)?;
                    self.cpu.mem.write_u32(base + at + 4, 0)?;
                    // A socket this host built holds the registration itself, and its provider has no
                    // `Cancel` slot to ask — so the pair that just came out of the record comes off the
                    // chains here too, which is what the reference forward achieves by sending the pair to
                    // the socket.
                    if ours {
                        self.ssl_disarm(base, read);
                    }
                }
                Ok(result)
            }
            "Read" | "Write" => {
                let [buf, count] = self.args::<2>()?;
                self.ssl_stream(base, name == "Write", buf, count)
            }
            "ReadV" | "WriteV" => {
                let [iov, count] = self.args::<2>()?;
                let read = name == "ReadV";
                if if read { count != 0 } else { count == 0 } {
                    return Ok(0);
                }
                let mut total = 0u32;
                let mut index = 0u16;
                loop {
                    let entry = iov + u32::from(index) * 8;
                    let buf = self.cpu.mem.read_u32(entry)?;
                    let len = self.cpu.mem.read_u16(entry + 4)?;
                    let moved = self.ssl_stream(base, !read, buf, len)?;
                    if (moved as i32) < 0 || (!read && moved == 0) {
                        return Ok(if total == 0 { moved } else { total });
                    }
                    total = total.wrapping_add(moved);
                    if len != moved {
                        return Ok(total);
                    }
                    index = index.wrapping_add(1);
                    if !read && u32::from(index) >= count {
                        return Ok(total);
                    }
                }
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    #[inline(never)]
    pub(crate) fn ssl_stream(
        &mut self,
        base: u32,
        write: bool,
        buf: u32,
        count: u32,
    ) -> Result<u32, Error> {
        if self.ssl.get(&base).map(|a| a.state) != Some(768) {
            return self.ssl_face_not_ready(base);
        }
        if self.tls_pump(base)? {
            self.ssl_finish(base, Self::SSL_SUCCESS)?;
        }
        if write {
            let bytes = self.cpu.mem.read_bytes(buf, count as usize)?;
            let wrote = match self.ssl.get_mut(&base).and_then(|a| a.session.as_mut()) {
                Some(session) => session.write(&bytes).is_ok(),
                None => false,
            };
            if !wrote {
                return self.ssl_face_error(base, socket::ECONNRESET);
            }
            // The ciphertext goes out on the next pump round, and this is one.
            self.tls_pump(base)?;
            return Ok(count);
        }
        // Bounded on purpose: `ISocket::Read(po, buf, len)` answers with the number taken, and what did
        // not fit the caller's buffer stays buffered for its next call. Draining the session here would
        // discard it — which the vector slot, one caller-sized buffer per entry, shows immediately.
        let streamed = match self.ssl.get_mut(&base).and_then(|a| a.session.as_mut()) {
            Some(session) => session.read_up_to(count as usize),
            None => Ok(tls::Streamed::WouldBlock),
        };
        let plain = match streamed {
            // `case 6` of the body: the peer ended the stream, so the answer is a plain zero — not the
            // `-2` of "nothing yet", which a caller reads as "ask again" and retries for ever.
            Ok(tls::Streamed::Closed) => return Ok(0),
            Ok(tls::Streamed::WouldBlock) => return self.ssl_face_would_read(base),
            Err(_) => return self.ssl_face_error(base, socket::ECONNRESET),
            Ok(tls::Streamed::Have(plain)) => plain,
        };
        self.cpu.mem.write_bytes(buf, &plain)?;
        Ok(plain.len() as u32)
    }

    /// The last release of an `ISSL` record, from either address.
    ///
    /// The option list is *not* touched here: it is keyed by the record's address like every other
    /// object's, and the generic release below gives its copied blocks back. Removing it here first
    /// would leave those blocks in the arena with nobody holding them — which is how this function was
    /// caught.
    pub(crate) fn ssl_died(&mut self, addr: u32) {
        let base = self.ssl_faces.get(&addr).copied().unwrap_or(addr);
        // This runs for **every** object the layer releases, so the record has to be one of ours
        // before any of its words are read: the four lines below are the only part that applies to
        // an address that never was an SSL record.
        if self.ssl.contains_key(&base) {
            // `ISSL_release` (`ea 0x4a8`, 80 bytes) at its count-to-zero branch: `ISSL_Reset` first,
            // then `CALLBACK_Cancel(record + 0xD0)`, then the record goes back to the heap.
            //The `WebOptMgr_Dtor`/shell-`Release` steps around them are the host's own
            // objects and have no counterpart to run here.
            let _ = self.ssl_reset(base);
            let _ = self.ssl_cancel_block(base + 208);
        }
        // A registration does not outlive the record it names: an address left on a socket's chain
        // would be woken as a record after the allocator has handed the block back, and whatever lands
        // there next would be read as one.
        self.ssl_disarm(base, true);
        self.ssl_disarm(base, false);
        // Nor does an *already queued* resume: the two queues hold addresses too, and a record that
        // has been freed and then dispatched is the same read over the same hole.
        self.dispatcher.forget(base);
        self.dispatcher.forget(base + 20);
        self.ssl_faces.remove(&base);
        self.ssl_faces.remove(&(base + 20));
        self.ssl.remove(&base);
    }

    /// `ISSL_Reset` (`ea 0x11b4`, 82 bytes) — what both `SetSocket` and the count-to-zero release run
    /// before the record takes on a new socket or goes away:
    ///
    /// ```text
    /// CALLBACK_Cancel(record + 48);
    /// if (*[record + 96]) CALLBACK_Cancel(*[record + 96]);
    /// if ([record + 16]) { socket->slot 4 (0, 0); socket->slot 1; [record + 16] = 0; }
    /// AEE_SSL_free([record + 28]); AEE_SSL_CTX_free([record + 32]);      // the session, dropped here
    /// [record + 176] = 256;
    /// ```
    ///
    /// The socket is the pack's own when it made one, and then both calls go through **its** table —
    /// slot 4 is `ISocket::Cancel`, slot 1 `IBase::Release`. A socket this host built has no table of
    /// its own to call, so its registration is taken off the chains instead ([`Runtime::ssl_disarm`]),
    /// which is the same effect the reference forward has.
    pub(crate) fn ssl_reset(&mut self, base: u32) -> Result<(), Error> {
        self.ssl_cancel_block(base + 48)?;
        let armed = self.cpu.mem.read_u32(base + 96)?;
        self.ssl_cancel_block(armed)?;
        let sock = self.cpu.mem.read_u32(base + 16)?;
        if sock != 0 {
            if self.ssl_socket_slot(sock).is_none() {
                let cancel = self.guest_slot(sock, "ISocket", "Cancel")?;
                self.call_guest(cancel, &[sock, 0, 0])?;
                let release = self.guest_slot(sock, "ISocket", "Release")?;
                self.call_guest(release, &[sock])?;
            } else {
                self.ssl_disarm(base, true);
                self.ssl_disarm(base, false);
            }
            self.cpu.mem.write_u32(base + 16, 0)?;
        }
        self.cpu.mem.write_u32(base + 176, 256)?;
        Ok(())
    }

    /// `CALLBACK_Cancel` for a guest block: the queue entries naming it go (see
    /// [`Dispatcher::forget`]), then the block's own `pfnCancel` at `+8` runs with the block — which is
    /// what the delivered helper does, and it does **not** clear that word.
    pub(crate) fn ssl_cancel_block(&mut self, cb: u32) -> Result<(), Error> {
        if cb == 0 {
            return Ok(());
        }
        self.dispatcher.forget(cb);
        let cancel = self.cpu.mem.read_u32(cb + 8)?;
        if cancel != 0 {
            self.call_guest(cancel, &[cb])?;
        }
        Ok(())
    }

    /// `HandleLowRAMEvent` (the delivered body, 68 bytes): run the SSL record's parked pair over
    /// the face — `record[+84](record[+88], record + 20)`. Nothing else moves: no state word, no
    /// un-register, so a record that fires twice notifies twice.
    ///
    /// ⚠ This host registers `AEE_SCB_LOW_RAM` and never issues it — the one system callback whose
    /// condition a single-pack host produces is `AEE_SCB_AEE_EXIT` — so this leg runs only when
    /// someone reaches it through the address the record was stamped with.
    pub(crate) fn ssl_low_ram_stub(&mut self) -> Result<u32, Error> {
        let record = self.cpu.r[0];
        let pfn = self.cpu.mem.read_u32(record + 84)?;
        if pfn == 0 {
            return Ok(pfn);
        }
        let user = self.cpu.mem.read_u32(record + 88)?;
        self.call_guest(pfn, &[user, record + 20])
    }

    /// `ISSL_Socket_SockCloser(face, closing)` (the delivered body, 76 bytes): the close notice a socket
    /// hands back to the record that armed it. Taken only while the record still points at the
    /// socket that closed — `SetSocket` can move it, and the old socket's notice must not close the
    /// new one's caller. The words it loads are also what it answers when it declines.
    pub(crate) fn ssl_sock_closer_stub(&mut self) -> Result<u32, Error> {
        let face = self.cpu.r[0];
        let closing = self.cpu.r[1];
        let record = self.cpu.mem.read_u32(face + 4)?;
        let sock = self.cpu.mem.read_u32(record + 16)?;
        if sock != closing {
            return Ok(sock);
        }
        let pfn = self.cpu.mem.read_u32(face + 64)?;
        if pfn == 0 {
            return Ok(pfn);
        }
        let user = self.cpu.mem.read_u32(face + 68)?;
        self.call_guest(pfn, &[user, face])
    }

    /// The `AEECLSID_SockPort` object of the delivered body: the app-facing socket, 26 `ISockPort`
    /// slots plus the 16 `ISocket` ones at `me + 4`.
    ///
    /// `AF_LOCAL` is complete here, because every local branch in the delivered object is one indirect
    /// call into the `AEECLSID_HostSocket` it keeps at `[+172]` — the loopback provider this host already
    /// models. `AF_INET` has no provider: `[+108]` (the net object `SelectNetwork` links through
    /// `AEENet_SelectNetwork`) stays 0, which makes `ISockPort_TCPReady` answer `AEE_NET_ENETNONET` for
    /// the four read/write slots, and leaves every other INET path inside `AEENet_*` /
    /// `bridle_SystemCall` / `OEM_GetConfig` — the OEM net module this box does not have.
    #[inline(never)]
    pub(crate) fn port_slot(
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
        let Some(base) = self.port_base(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        let Some(port) = self.ports.get(&base).copied() else {
            return Err(Error::Hle(unimplemented()));
        };
        let (is_local, is_open, kind) = (port.is_local(), port.is_open(), port.kind);
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                // `AEE_CheckPtr(0, ppv, 4, 1)` answers 29 and leaves the caller's slot alone.
                if !self.sock_range(ppv, 4) {
                    return Ok(g::AEE_EMEMPTR);
                }
                match iid {
                    // Four ids answered with the record itself, each taking a reference.
                    socket::IID_IQI
                    | socket::CLS_SOURCE
                    | socket::CLS_PORT
                    | socket::CLS_SOCKPORT => {
                        self.cpu.mem.write_u32(ppv, base)?;
                        self.hold(base, 1).map_err(Error::Hle)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    // The `ISocket` face — only for an INET socket. A local one answers `AEE_EBADSTATE`
                    // with the caller's slot untouched.
                    socket::CLS_SOCKET if port.is_inet() => {
                        let face = self.port_face(base)?;
                        self.cpu.mem.write_u32(ppv, face)?;
                        self.hold(base, 1).map_err(Error::Hle)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    socket::CLS_SOCKET => Ok(socket::AEE_EBADSTATE),
                    _ => {
                        self.cpu.mem.write_u32(ppv, 0)?;
                        Ok(socket::AEE_ECLASSNOTSUPPORT)
                    }
                }
            }
            //: an **open local** socket answers the provider's word; everything else reads the
            // record's own `[+178]`.:
            //an **open local** port answers the provider's word; everything else reads the
            // record's own `[+178]` — where `provider_slot` mirrors a provider refusal, which is the
            // body's `errno = provider->GetLastError` line for the INET half.
            "GetLastError" => {
                if is_open && is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, "GetLastError");
                    }
                }
                Ok(port.error as u32)
            }
            //: a four-byte body, `return 20`.
            "Open" => Ok(socket::AEE_EUNSUPPORTED),
            // `AEENet_SelectNetwork(network, shell, &me->net)` — the net module's own entry and the only
            // writer of `[+108]`. C: the record is the OEM net module behind the delivered body.
            "SelectNetwork" => Err(Error::Hle(unimplemented())),
            "OpenEx" => {
                let [family, protocol] = self.args()?;
                let family = family as u16;
                let protocol = protocol as u16;
                if is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if let Some(slot) = self.ports.get_mut(&base) {
                    slot.family = family;
                }
                match family {
                    socket::FAMILY_LOCAL => {
                        // `QI(shell, AEECLSID_HostSocket, &[+172])`, then `OpenEx` on
                        // it — in `port_provider`, which the INET branch shares.
                        let host_addr = self.port_provider(base, family, protocol as u32)?;
                        if host_addr == 0 {
                            return Ok(u32::MAX);
                        }
                        self.cpu.mem.write_u32(base + 172, host_addr)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    socket::FAMILY_INET => self.port_open_inet(base, protocol),
                    // `errno = EINVAL; return -1` for any other family.
                    _ => self.port_refuse(base, socket::EINVAL),
                }
            }
            "Close" => {
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                // `Unrealize(me, 1)`: release the provider, clear `[+172]`, answer 0
                // (,). The body's `EINVAL` arm is for a record with no realized
                // provider, which after the INET substitution is only one this host cannot be in.
                if let Some((_, host)) = self.port_host(base) {
                    self.sock_died(host);
                }
                self.cpu.mem.write_u32(base + 172, 0)?;
                let record = port.net;
                if let Some(slot) = self.ports.get_mut(&base) {
                    slot.host = None;
                    slot.net = None;
                    slot.kind = 0;
                }
                if let Some(record) = record {
                    self.hle.dealloc(&mut self.cpu.mem, record);
                }
                Ok(g::AEE_SUCCESS)
            }
            // `IOCtl(me, id, pArg)` (, 281 bytes). Two ids are the record's own eight bytes,
            // then the local branch delegates; the INET ids read and write the provider record's
            // timeout and its `[+32]` pair, which the host allocated itself — so only the *clear* arm
            // of 4100 needs `OEM_GetConfig`, and the default is the body's `return 20`.
            "IOCtl" => {
                let [op, parg] = self.args()?;
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                match op {
                    4103 => {
                        if parg != 0 && self.sock_range(parg, 8) {
                            let words = port.ioctl_words;
                            self.cpu.mem.write_u32(parg, words[0])?;
                            self.cpu.mem.write_u32(parg + 4, words[1])?;
                        }
                        Ok(g::AEE_SUCCESS)
                    }
                    4104 => {
                        if parg != 0 && self.sock_range(parg, 8) {
                            let lo = self.cpu.mem.read_u32(parg)?;
                            let hi = self.cpu.mem.read_u32(parg + 4)?;
                            if let Some(slot) = self.ports.get_mut(&base) {
                                slot.ioctl_words = [lo, hi];
                            }
                        }
                        Ok(g::AEE_SUCCESS)
                    }
                    _ if is_local => {
                        if let Some((_, host)) = self.port_host(base) {
                            return self.isockport(id, host, label, "IOCtl");
                        }
                        Err(Error::Hle(unimplemented()))
                    }
                    // The INET ids need `[+172]`'s record; without one there is nothing to read.
                    _ => {
                        let Some(record) = port.net else {
                            return Err(Error::Hle(unimplemented()));
                        };
                        match op {
                            4097 => {
                                if parg != 0 && self.sock_range(parg, 8) {
                                    let lo = self.cpu.mem.read_u32(record + 32)?;
                                    let hi = self.cpu.mem.read_u32(record + 36)?;
                                    self.cpu.mem.write_u32(parg, lo)?;
                                    self.cpu.mem.write_u32(parg + 4, hi)?;
                                }
                                Ok(g::AEE_SUCCESS)
                            }
                            4098 => {
                                if parg != 0 && self.sock_range(parg, 8) {
                                    let lo = self.cpu.mem.read_u32(parg)?;
                                    let hi = self.cpu.mem.read_u32(parg + 4)?;
                                    self.cpu.mem.write_u32(record + 32, lo)?;
                                    self.cpu.mem.write_u32(record + 36, hi)?;
                                } else {
                                    self.cpu.mem.write_u32(record + 32, 0)?;
                                    self.cpu.mem.write_u32(record + 36, 0)?;
                                }
                                Ok(g::AEE_SUCCESS)
                            }
                            4100 => {
                                if parg != 0 {
                                    self.cpu.mem.write_u32(record + 28, parg)?;
                                } else {
                                    // `OEM_GetConfig(9, &timeout, 4)`: the answer is the default this
                                    // host's own `OpenEx` filled in, so the reset lands there.
                                    self.cpu.mem.write_u32(record + 28, 30_000)?;
                                }
                                Ok(g::AEE_SUCCESS)
                            }
                            4105 => {
                                if parg != 0 && self.sock_range(parg, 4) {
                                    let ms = self.cpu.mem.read_u32(record + 28)?;
                                    self.cpu.mem.write_u32(parg, ms)?;
                                }
                                Ok(g::AEE_SUCCESS)
                            }
                            _ => Ok(socket::AEE_EUNSUPPORTED),
                        }
                    }
                }
            }
            // `Read`/`Write`/`ReadV`/`WriteV`: the pointer gate, then `IS_OPEN`, then one call into the
            // provider. The INET half asks `TCPReady` first and stops on its
            // answer, which with no net object is `AEE_NET_ENETNONET`.
            "Read" | "Write" | "ReadV" | "WriteV" => {
                let [buf, count] = self.args()?;
                let vector = name.ends_with('V');
                let len = if vector {
                    (count as usize).saturating_mul(8)
                } else {
                    count as usize
                };
                if !self.sock_range(buf, len) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // `TCPReady`'s first test is `*(+108) == 0` in the body; a port this host gave a
                // provider has a socket instead, and that is the substitution.
                let ready = port.tcp_ready_with(port.host.is_some());
                if ready != 0 {
                    return self.port_refuse(base, ready);
                }
                // — the OEM or loopback branch, which here is the provider.
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            // `Readable`/`Writeable` (/): a 28-byte callback block, and this body answers
            // the **errno in r0** rather than -1. The local branch is `provider->{Readable,Writeable}`.
            "Readable" | "Writeable" => {
                let [pcb] = self.args()?;
                if !self.sock_range(pcb, 28) {
                    self.port_error(base, socket::EFAULT);
                    return Ok(socket::EFAULT);
                }
                if !is_open {
                    self.port_error(base, socket::EBADF);
                    return Ok(socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // The INET half chains the block at the provider record's `[+44]`/`[+48]` through
                // `ISockPort_Select`, whose event source is `AEENet_SocketEventOccurred`; with this
                // host's provider the watch is the provider's own chain.
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "Bind" => {
                let [paddr] = self.args()?;
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // `AEE_CheckPtr(0, a2, 8, 0)` → 515, then a family word that is not `AF_INET` → 517.
                if !self.sock_range(paddr, 8) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                let (family, _, _) = self.read_sock_name(paddr)?;
                if family != socket::FAMILY_INET {
                    return self.port_refuse(base, socket::EAFNOSUPPORT);
                }
                // Past this point the body calls `ISockPort_TryStart`, whose first act is
                // `AEENet_NetStatus(*(+108), 0)` — the OEM net module, which here is the provider.
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "Listen" => {
                //: a datagram socket first, then the open gate.
                if kind != 0 {
                    return self.port_refuse(base, socket::EOPNOTSUPP);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // The INET half is `IS_INET_NONE` / bound / `TryListen`; the provider is
                // what stands in for the last one.
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "Accept" => {
                let [ppsock] = self.args()?;
                if kind != 0 {
                    return self.port_refuse(base, socket::EOPNOTSUPP);
                }
                if !self.sock_range(ppsock, 4) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                // reads the provider record's accept bit for an INET record; this host's
                // provider is not that record, so the substitution asks the socket instead.
                let Some((host_slot, _)) = self.port_host(base) else {
                    return Err(Error::Hle(unimplemented()));
                };
                // `*a2 = 0` is written before the provider is asked.
                self.cpu.mem.write_u32(ppsock, 0)?;
                match self.host_sockets.accept(host_slot, true) {
                    socket::Accept2::Taken(server) => {
                        // A port per accepted connection: the body asks the shell for another
                        // `AEECLSID_SockPort`, opens it local, and hands the provider's socket back
                        // through the new port's `[+172]`.
                        let child = self.port_new_object()?;
                        let provider = self.sockport_object(server)?;
                        // The accepted record is the listener's family: an INET listener hands back
                        // an INET record, not a local one.
                        let family = if is_local {
                            socket::FAMILY_LOCAL
                        } else {
                            socket::FAMILY_INET
                        };
                        self.cpu.mem.write_u32(child + 168, family as u32)?;
                        self.cpu.mem.write_u32(child + 172, provider)?;
                        if let Some(slot) = self.ports.get_mut(&child) {
                            slot.family = family;
                            slot.host = Some(server);
                        }
                        self.cpu.mem.write_u32(ppsock, child)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    socket::Accept2::Blocked(done) | socket::Accept2::Refused(done) => {
                        if let Some(errno) = done.error {
                            self.port_error(base, errno);
                        }
                        self.answer_sock(done)
                    }
                }
            }
            "Connect" => {
                let [paddr] = self.args()?;
                if kind != 0 {
                    return self.port_refuse(base, socket::EOPNOTSUPP);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // `AEE_CheckPtr(0, a2, 8, 0)` → 515, then `IS_INET_NONE` → `ETIMEDOUT`.
                if !self.sock_range(paddr, 8) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                let (family, _, _) = self.read_sock_name(paddr)?;
                if family != socket::FAMILY_INET {
                    return self.port_refuse(base, socket::EAFNOSUPPORT);
                }
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "SendTo" => {
                let [buf, count, _flags, pto] = self.args()?;
                if !self.sock_range(buf, count as usize) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                if !self.sock_range(pto, 8) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "RecvFrom" => {
                let [buf, count, _flags, pfrom, psize] = self.args()?;
                if !self.sock_range(buf, count as usize) {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // `*a6 < 8 || AEE_CheckPtr(0, a5, 8, 0)` → 515.
                if psize != 0 && self.sock_range(psize, 4) {
                    let size = self.cpu.mem.read_u32(psize)?;
                    if size < 8 || !self.sock_range(pfrom, 8) {
                        return self.port_refuse(base, socket::EFAULT);
                    }
                }
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "GetSockName" | "GetPeerName" => {
                // `GetPeerName` is the one that refuses a datagram socket first.
                if name == "GetPeerName" && kind != 0 {
                    return self.port_refuse(base, socket::EOPNOTSUPP);
                }
                let [paddr, psize] = self.args()?;
                // `GetSockName`'s `AEE_CheckPtr(0, a3, 4, 1)` writes 515; `GetPeerName`'s writes
                // **514** ('s `&loc_202` is the immediate `0x202`), so the two differ on purpose.
                if !self.sock_range(psize, 4) {
                    let errno = if name == "GetPeerName" {
                        socket::EBADF
                    } else {
                        socket::EFAULT
                    };
                    return self.port_refuse(base, errno);
                }
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                let _ = paddr;
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "GetOpt" | "SetOpt" => {
                // `GetOpt`: `AEE_CheckPtr(0, a5, 4, 1) || AEE_CheckPtr(0, a4, *a5, 1)` → 515.
                // `SetOpt`: `AEE_CheckPtr(0, a4, a5, 0)` → 515.
                let [level, opt, pval, psize] = self.args()?;
                let ok = if name == "GetOpt" {
                    self.sock_range(psize, 4) && {
                        let len = self.cpu.mem.read_u32(psize)?;
                        self.sock_range(pval, len as usize)
                    }
                } else {
                    self.sock_range(pval, psize as usize)
                };
                if !ok {
                    return self.port_refuse(base, socket::EFAULT);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, name);
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                // The INET half promotes the record to OEM and then goes straight to
                // `bridle_SystemCall`; the provider answers the same question for a
                // port this host opened itself.
                let _ = (level, opt);
                if let Some(answer) = self.provider_slot(id, base, label, name) {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            "Shutdown" => {
                let [how] = self.args()?;
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                if is_local {
                    if let Some((_, host)) = self.port_host(base) {
                        return self.isockport(id, host, label, "Shutdown");
                    }
                    return Err(Error::Hle(unimplemented()));
                }
                let _ = how;
                if let Some(answer) = self.provider_slot(id, base, label, "Shutdown") {
                    return answer;
                }
                Err(Error::Hle(unimplemented()))
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// Call one of this host's own port slots with a **staged** argument list.
    ///
    /// An `ISocket` face's body calls the port's slot rather than repeating that slot's checks, so
    /// the face's arms here do the same. `r1..r3` take the first three arguments and anything past
    /// them goes at the stack pointer a nested call reads — `frame`, a block this arm owns, never
    /// the caller's own frame. The register file and the stack pointer are put back before the
    /// answer travels; the nested call is handed the port record, which is what the face's
    /// `[me + 4]` names.
    #[inline(never)]
    pub(crate) fn port_nested(
        &mut self,
        id: u32,
        base: u32,
        label: &str,
        name: &str,
        args: &[u32],
        frame: u32,
    ) -> Result<u32, Error> {
        let saved = (self.cpu.r[1], self.cpu.r[2], self.cpu.r[3], self.cpu.r[13]);
        for (i, v) in args.iter().enumerate() {
            if i < 3 {
                self.cpu.r[1 + i] = *v;
            } else {
                self.cpu.mem.write_u32(frame + ((i - 3) * 4) as u32, *v)?;
            }
        }
        self.cpu.r[13] = frame;
        let result = self.port_slot(id, base, label, name);
        self.cpu.r[1] = saved.0;
        self.cpu.r[2] = saved.1;
        self.cpu.r[3] = saved.2;
        self.cpu.r[13] = saved.3;
        result
    }

    /// The `ISocket` face's three forwarding slots — the delivered body keeps their bodies next
    /// to the port's: `ISocket_SendTo` builds `{2, port, addr}` and calls the port's
    /// `SendTo`; `ISocket_GetPeerName` and `ISocket_RecvFrom` hand the port an
    /// eight-byte name block and a size word of 8, copy the two outputs out when the family comes
    /// back IPv4, and answer **517** when it does not (for `RecvFrom`: `-1` with the port's error
    /// cell set to 517).
    ///
    /// `scratch` is this arm's own block: eight bytes of name, the four-byte size word, then the
    /// words a fourth and fifth argument travel in.
    pub(crate) fn sock_face_slot(
        &mut self,
        id: u32,
        base: u32,
        label: &str,
        name: &str,
        scratch: u32,
    ) -> Result<u32, Error> {
        let (sa, size, frame) = (scratch, scratch + 8, scratch + 16);
        match name {
            "SendTo" => {
                let [buf, count, flags, addr, port] = self.args()?;
                self.cpu.mem.write_u16(sa, socket::FAMILY_INET as u32)?;
                self.cpu.mem.write_u16(sa + 2, port)?;
                self.cpu.mem.write_u32(sa + 4, addr)?;
                self.port_nested(id, base, label, "SendTo", &[buf, count, flags, sa], frame)
            }
            "RecvFrom" => {
                let [buf, count, flags, paddr, pport] = self.args()?;
                self.cpu.mem.write_u32(size, 8)?;
                let answer = self.port_nested(
                    id,
                    base,
                    label,
                    "RecvFrom",
                    &[buf, count, flags, sa, size],
                    frame,
                )? as i32;
                if answer >= 0 {
                    let family = self.cpu.mem.read_u16(sa)?;
                    let len = self.cpu.mem.read_u32(size)?;
                    if family == socket::FAMILY_INET as u32 && len == 8 {
                        let addr = self.cpu.mem.read_u32(sa + 4)?;
                        let port = self.cpu.mem.read_u16(sa + 2)?;
                        if paddr != 0 {
                            self.cpu.mem.write_u32(paddr, addr)?;
                        }
                        if pport != 0 {
                            self.cpu.mem.write_u16(pport, port)?;
                        }
                    } else {
                        self.port_error(base, socket::EAFNOSUPPORT);
                        return Ok(u32::MAX);
                    }
                }
                Ok(answer as u32)
            }
            "GetPeerName" => {
                let [paddr, pport] = self.args()?;
                self.cpu.mem.write_u32(size, 8)?;
                let answer =
                    self.port_nested(id, base, label, "GetPeerName", &[sa, size], frame)? as i32;
                if answer != 0 {
                    // `if (PeerName == -1) return ISockPort_GetLastError; return 0;` — a
                    // port answer that is neither is *not* passed on.
                    return if answer == -1 {
                        self.port_slot(id, base, label, "GetLastError")
                    } else {
                        Ok(0)
                    };
                }
                let family = self.cpu.mem.read_u16(sa)?;
                let len = self.cpu.mem.read_u32(size)?;
                if family != socket::FAMILY_INET as u32 || len != 8 {
                    // `return 517;` — this one is the face's own answer, not the port's cell.
                    return Ok(socket::EAFNOSUPPORT);
                }
                let addr = self.cpu.mem.read_u32(sa + 4)?;
                let port = self.cpu.mem.read_u16(sa + 2)?;
                if paddr != 0 {
                    self.cpu.mem.write_u32(paddr, addr)?;
                }
                if pport != 0 {
                    self.cpu.mem.write_u16(pport, port)?;
                }
                Ok(0)
            }
            _ => Err(Error::Hle(HleError::Unimplemented {
                id,
                label: label.to_string(),
                this: base,
            })),
        }
    }

    /// The sixteen `ISocket` slots of a port record's `me + 4` face.
    ///
    /// Most of them are a shim over the record: `Read`/`Write`/`ReadV`/`WriteV`/`GetLastError` call the
    /// port's slot with the same arguments, `Bind`/`Connect`/`SendTo`/`RecvFrom`/`GetPeerName` pack or
    /// unpack the `{family, port, addr}` block this host already reads, and `QueryInterface` is
    /// `AEEBase_QueryInterface` — a forward through `[me + 4]` to the record's own body. `Readable` and
    /// `Writeable` keep the callback pair in the face's own words and then forward the block built under
    /// them, so the record's gates still answer; `Cancel` withdraws those blocks; `IOCtl` is the one slot
    /// that never reaches the transport at all.
    pub(crate) fn port_isocket(
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
        // An `ISSL` record's face is the other `ISocket` in this host; it answers from the session
        // rather than from a socket record, so it is handed over before the port lookup below.
        if self.ssl_faces.contains_key(&this) {
            return self.issl_socket(id, this, label, name);
        }
        let Some(base) = self.port_base(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            "QueryInterface" => self.port_slot(id, base, label, "QueryInterface"),
            "GetLastError" | "Read" | "Write" | "ReadV" | "WriteV" => {
                self.port_slot(id, base, label, name)
            }
            // `ISocket_Bind(me, addr, port)` builds `{2, port, psa}` on its own stack and calls the
            // port's slot with it; the answered half of that body ends where `TryStart` begins, which is
            // the OEM net module, so there is no provider this host can ask here.
            "Bind" => {
                let is_open = self
                    .ports
                    .get(&base)
                    .map(sockport::Port::is_open)
                    .unwrap_or(false);
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                Err(Error::Hle(unimplemented()))
            }
            "Connect" => {
                let [in_addr, in_port, pfn, pv] = self.args::<4>()?;
                if pfn == 0 {
                    return self.port_refuse(base, socket::EFAULT);
                }
                let is_open = self
                    .ports
                    .get(&base)
                    .map(sockport::Port::is_open)
                    .unwrap_or(false);
                if !is_open {
                    return self.port_refuse(base, socket::EBADF);
                }
                self.cpu.mem.write_u32(this + 92, pfn)?;
                self.cpu.mem.write_u32(this + 96, pv)?;
                if let Some(record) = self.port_record(base) {
                    self.cpu.mem.write_u16(record + 20, 2)?;
                    self.cpu.mem.write_u16(record + 22, in_port)?;
                    self.cpu.mem.write_u32(record + 24, in_addr)?;
                }
                let leg = self.host_fn(HostFn::SockPortTryConnect)?;
                self.cpu.mem.write_u32(base + 84, leg)?;
                self.cpu.mem.write_u32(base + 88, base)?;
                self.dispatcher.resume_from(base + 68, self.applet);
                Ok(0)
            }
            "Readable" | "Writeable" => {
                let [pfn, pv] = self.args()?;
                let (pair, block) = if name == "Readable" {
                    (24, 8)
                } else {
                    (52, 36)
                };
                self.cpu.mem.write_u32(this + pair, pfn)?;
                self.cpu.mem.write_u32(this + pair + 4, pv)?;
                let frame = self.hle.alloc(&mut self.cpu.mem, 8)?;
                let answer = self.port_nested(id, base, label, name, &[this + block], frame);
                self.hle.dealloc(&mut self.cpu.mem, frame);
                answer
            }
            "Cancel" => {
                let [pfn, pv] = self.args()?;
                let wildcard = (pfn | pv) == 0;
                // Reference order: the connect pair, then readable, then writeable. `clear_pair` marks the
                // one write that has nothing to do with a chain.
                for (pair, block, clear_pair) in [(92, 64, true), (24, 8, false), (52, 36, false)] {
                    if !wildcard
                        && (self.cpu.mem.read_u32(this + pair)? != pfn
                            || self.cpu.mem.read_u32(this + pair + 4)? != pv)
                    {
                        continue;
                    }
                    if clear_pair {
                        self.cpu.mem.write_u32(this + pair, 0)?;
                        self.cpu.mem.write_u32(this + pair + 4, 0)?;
                    }
                    let Some((slot, _)) = self.port_host(base) else {
                        continue;
                    };
                    if !self.host_sockets.armed(slot, (this + block) as usize) {
                        continue;
                    }
                    self.cpu.mem.write_u32(this + block, 0)?;
                    self.cpu.mem.write_u32(this + block + 8, 0)?;
                    self.cpu.mem.write_u32(this + block + 12, 0)?;
                    self.host_sockets
                        .cancel_entry(slot, (this + block) as usize);
                }
                let state = self.cpu.mem.read_u8(base + 176)?;
                if state != 1 {
                    return Ok(state as u32);
                }
                Ok(self.ports.get(&base).map(|one| one.net_object).unwrap_or(0))
            }
            // `ISocket_IOCtl` switches over `4097..4114` and answers 20 for everything.
            "IOCtl" => Ok(socket::AEE_EUNSUPPORTED),
            "GetPeerName" | "SendTo" | "RecvFrom" => {
                let scratch = self.hle.alloc(&mut self.cpu.mem, 32)?;
                let answer = self.sock_face_slot(id, base, label, name, scratch);
                self.hle.dealloc(&mut self.cpu.mem, scratch);
                answer
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The socket record behind a caller's `ISocket` pointer, when this host built that socket: a host
    /// socket object, or a port whose local provider is one. `None` means the pointer names an object
    /// this host did not make, and the reference registration hands *that* object a runtime entry address
    /// — the one thing [`StreamSource`] already says this host cannot produce. The source is still built
    /// either way, because the factory asks the socket for nothing; it is `Readable` that has nowhere to
    /// park, and it says so.
    #[inline(never)]
    pub(crate) fn sock_source_slot(&mut self, socket: u32) -> Option<usize> {
        if let Some(slot) = self.sock_objects.get(&socket).copied() {
            return Some(slot);
        }
        let base = self.port_base(socket)?;
        let (slot, _host) = self.port_host(base)?;
        Some(slot)
    }

    /// `SockSource_ReadbleCancel` (the delivered body, 44 bytes), reached the way the pack reaches it:
    /// the caller's block is in `r0`.
    ///
    /// ```text
    /// rec = cb[+12]; cb[+8] = 0;
    /// walk rec[+16] for cb and unlink it;
    /// if (rec[+16] == 0) socket->slot 4 (socket, SockSource_ReadableCB);
    /// ```
    ///
    /// The socket registration goes with the last block, which is what makes a pack's early
    /// `CALLBACK_CANCEL` withdraw it instead of leaving a listener pointing at a block nobody reads.
    pub(crate) fn sock_source_cancel_stub(&mut self) -> Result<u32, Error> {
        let field = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        let cb = self.cpu.r[0];
        let rec = self.cpu.mem.read_u32(cb + field("pCancelData"))?;
        self.cpu.mem.write_u32(cb + field("pfnCancel"), 0)?;
        let mut link = rec + 16;
        loop {
            let next = self.cpu.mem.read_u32(link)?;
            if next == 0 {
                break;
            }
            if next == cb {
                let after = self.cpu.mem.read_u32(cb + field("pNext"))?;
                self.cpu.mem.write_u32(link, after)?;
                break;
            }
            link = next;
        }
        if let Some(one) = self.sock_sources.get_mut(&rec) {
            one.chain.retain(|held| *held != cb);
        }
        if self.cpu.mem.read_u32(rec + 16)? == 0 {
            let socket = self.cpu.mem.read_u32(rec + 8)?;
            match self.sock_source_slot(socket) {
                Some(slot) => self.host_sockets.cancel_entry(slot, rec as usize),
                None => {
                    let at = self.guest_slot(socket, "ISocket", "Cancel")?;
                    let ready = self.host_fn(HostFn::SockSourceReady)?;
                    self.call_guest(at, &[socket, ready, 0])?;
                }
            }
        }
        Ok(0)
    }

    /// `SockSource_ReadableCB` (36 bytes): the socket's "bytes are here" wake. Each block comes off
    /// the source's chain, loses its unregister hook, and is **resumed** — the body ends each turn
    /// with `shell->Resume(shell, block)`, so nothing runs inside this call.
    pub(crate) fn sock_source_ready_stub(&mut self) -> Result<u32, Error> {
        let field = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        let rec = self.cpu.r[0];
        loop {
            let head = self.cpu.mem.read_u32(rec + 16)?;
            if head == 0 {
                break;
            }
            let next = self.cpu.mem.read_u32(head + field("pNext"))?;
            self.cpu.mem.write_u32(rec + 16, next)?;
            self.cpu.mem.write_u32(head + field("pfnCancel"), 0)?;
            if let Some(one) = self.sock_sources.get_mut(&rec) {
                one.chain.retain(|held| *held != head);
            }
            // ⚠ This one is queued **without** a stamp on purpose: `a_pack_cancels_a_socket_source_…`
            // and `a_released_socket_source_…` encode that parking a block here leaves its `+8` alone
            // until the source drains it, and gating it through `dispatcher_prepare` moved that state
            // under them (the cancel leg unlinks the chain entry the test is watching). Until those two
            // records are re-derived against a stamped queue, this path records the owner only.
            self.dispatcher.resume_from(head, self.applet);
        }
        Ok(0)
    }

    /// `gvtSockSource_ISource`'s slots over the caller's socket. `socket` comes out of the record with
    /// the same lookup that routed the call here, because both live slots reach through it. `AddRef` and
    /// `Release` never arrive: the reference record counts at `+12`, which is where the generic layer
    /// already counts.
    #[inline(never)]
    pub(crate) fn sock_source(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
        socket: u32,
    ) -> Result<u32, Error> {
        let unimplemented = || {
            Error::Hle(HleError::Unimplemented {
                id,
                label: label.to_string(),
                this,
            })
        };
        let field = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        match name {
            // The member's one shared `ISource_QueryInterface`: two ids — the source's own class and the
            // base's — and no `AEE_CheckPtr` anywhere, so on a handset a null out-parameter is a write to
            // address zero. This host's `answer_qi` refuses `EBADPARM` instead, the way every other
            // `ISource` face here does (see [`Runtime::return_object`]); recorded rather than copied.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_SOURCE])
                    .map_err(Error::Hle)
            }
            // `return (***(me + 8) + 12)(*(me + 8))` — the socket's own slot 3 with the caller's buffer
            // and length still sitting in r1 and r2. `ISocket` inherits `IAStream`, so slot 3 of either
            // name is the same read: a count, a zero at the end, `-2` for "not yet", or an error, all of
            // which pass straight through.
            "Read" => {
                let [buf, len] = self.args()?;
                let at = self.guest_slot(socket, "ISocket", "Read")?;
                self.call_guest(at, &[socket, buf, len])
            }
            // The body opens on the caller's own notify slot and **keeps it as its answer**:
            // `result = pcb[+16]; if (result != 0) { …one registration, then push… } return result;`.
            // So a callback nobody filled in answers 0 and registers nothing — and it is said out loud,
            // because "nothing is waiting" must not read as "there is no such door". The stream bridge
            // has no such gate: its body cancels, stamps and forwards whatever the block holds.
            "Readable" => {
                let [pcb] = self.args()?;
                let notify = self.cpu.mem.read_u32(pcb + field("pfnNotify"))?;
                if notify == 0 {
                    self.display.note(format!(
                        "{label}::Readable: the AEECallback at {pcb:#010x} has no notify function, so \
                         nothing is registered with the socket"
                    ));
                    return Ok(0);
                }
                self.cancel_sock_pcb(pcb)?;
                // The documented behaviour arms the block: an unregister hook in `pfnCancel` and this source in
                // `pCancelData`. The hook is a runtime C function on a handset; here it is the address
                // [`Runtime::host_fn`] mints for [`HostFn::SockSourceCancel`], so a pack that calls
                // `CALLBACK_Cancel` on its own block withdraws the registration the way the delivery
                // does — the one door that used to be unservable.
                let hook = self.host_fn(HostFn::SockSourceCancel)?;
                self.cpu.mem.write_u32(pcb + field("pfnCancel"), hook)?;
                self.cpu.mem.write_u32(pcb + field("pCancelData"), this)?;
                // One socket registration for the whole chain, taken out when the chain empties.
                let empty = self
                    .sock_sources
                    .get(&this)
                    .expect("isource routed here by the record's own address")
                    .chain
                    .is_empty();
                let mut drained = false;
                if empty {
                    match self.sock_source_slot(socket) {
                        Some(slot) => {
                            drained = matches!(
                                self.host_sockets.readable_source(slot, this as usize),
                                socket::Arm::Resume
                            );
                        }
                        None => {
                            // A socket this host did not build: the body asks **its** slot 2 for the
                            // wake and hands it this host's `SockSource_ReadableCB` as the notify, so
                            // the ready list drains through the same leg on both sides of the seam.
                            // The socket's answer is discarded, exactly as the body does — the wake
                            // arrives later, through the callback.
                            let at = self.guest_slot(socket, "ISocket", "Readable")?;
                            let ready = self.host_fn(HostFn::SockSourceReady)?;
                            self.call_guest(at, &[socket, ready, this])?;
                        }
                    }
                }
                let one = self
                    .sock_sources
                    .get_mut(&this)
                    .expect("isource routed here by the record's own address");
                let old = one.chain.first().copied().unwrap_or(0);
                one.chain.insert(0, pcb);
                // `*pcb = old; me[+16] = pcb;` — both halves live in guest memory, where the reference
                // walk (and a pack that reads its own block) finds them; the host's `Vec` is a mirror.
                self.cpu.mem.write_u32(pcb + field("pNext"), old)?;
                self.cpu.mem.write_u32(this + 16, pcb)?;
                // `Arm::Resume` is the socket saying the bytes are already there. The reference answer
                // resumes the block it was handed, whose notify is `SockSource_ReadableCB`, which drains
                // this chain — one queue hop deeper than doing it here, and the same blocks coming out in
                // the same order, because a resume only ever queues.
                if drained {
                    self.drain_sock_source(this)?;
                }
                Ok(old)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `SockSource_ReadableCB`: pop every waiting block, clear the unregister hook each one
    /// carries on the way out, and resume the block itself through the shell — `IShell::Resume` is
    /// `gvtIShell + 144`, and its second word is the callback, which is the register the reference loop
    /// leaves holding it. Nothing is *called* here: a resume only queues, so a caller still inside
    /// `Readable` is not re-entered from under itself.
    #[inline(never)]
    pub(crate) fn drain_sock_source(&mut self, obj: u32) -> Result<(), Error> {
        let field = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        loop {
            let popped = self.sock_sources.get_mut(&obj).and_then(|one| {
                if one.chain.is_empty() {
                    None
                } else {
                    Some(one.chain.remove(0))
                }
            });
            let Some(pcb) = popped else {
                return Ok(());
            };
            // `SockSource_ReadbleCancel` is what unlinks the block from the source's own chain, and it
            // zeroes `cb[+8]` on the way out; the queue then stamps its own leg, so a
            // pack that asks `CALLBACK_IsQueued` here or cancels the block reaches the queue.
            self.cpu.mem.write_u32(pcb + field("pfnCancel"), 0)?;
            let owner = self.applet;
            if self.dispatcher_prepare(pcb)? {
                self.dispatcher.resume_from(pcb, owner);
            }
        }
    }

    /// One entry off a socket's waiting chain. A callback block goes to the dispatcher the way it always
    /// has; an entry that names a socket source is the runtime's own registration, and waking it means
    /// draining that source's list — which is the only reason `SockSource_ReadableCB` exists. An entry
    /// that names an `ISSL` record is the other registration the runtime parks, by
    /// [`Runtime::ssl_arm_socket`]: the record address for its readable direction and that record's
    /// `ISocket` face for its writeable one, since a chain carries one word and the two directions have
    /// to come back apart.
    #[inline(never)]
    pub(crate) fn wake_chain_entry(&mut self, entry: usize) -> Result<(), Error> {
        let obj = entry as u32;
        if self.sock_sources.contains_key(&obj) {
            return self.drain_sock_source(obj);
        }
        if self.ssl.contains_key(&obj) {
            return self.ssl_wake(obj, true);
        }
        if let Some(base) = self.ssl_faces.get(&obj).copied() {
            return self.ssl_wake(base, false);
        }
        self.dispatcher.resume_from(obj, self.applet);
        Ok(())
    }

    /// Every byte an `IAStream` will give up, read through the object's own vtable.
    ///
    /// Downcalled rather than lifted out of the host's own record, because the stream in this
    /// argument is very often the module's — `IAStream` over its own resource blob is the documented
    /// shape — and a host that only understands the objects it handed out answers that pack with an
    /// empty image. The host's own `IMemAStream` goes through the same door for the same reason: one
    /// path, so the two cannot drift apart.
    ///
    /// Bounded, because a stream that never reports an end would otherwise be read until the host
    /// process is as large as the pack's imagination.
    pub(crate) fn drain_stream(&mut self, stream: u32) -> HResult<Vec<u8>> {
        /// How much one `Read` is asked for, and the most the whole stream may give.
        const CHUNK: u32 = 8 * 1024;
        const LIMIT: usize = 8 * 1024 * 1024;
        let mut bytes = Vec::new();
        if stream == 0 {
            return Ok(bytes);
        }
        let down = |e: Error| match e {
            Error::Hle(h) => h,
            other => HleError::Unsupported {
                what: other.to_string(),
            },
        };
        let read = self.guest_slot(stream, "IAStream", "Read").map_err(down)?;
        let buf = self.cpu.mem.alloc_zeroed(CHUNK)?;
        loop {
            // `int32 IAStream_Read(IAStream*, void* pBuf, uint32 nSize)`. Zero or negative is the
            // stream's own way of saying there is nothing more; a *short* read is not the end, only
            // a zero-length one is, or a stream that hands back one byte at a time would be taken as
            // finished after its first byte.
            let got = self.call_guest(read, &[stream, buf, CHUNK]).map_err(down)?;
            let n = match i32::try_from(got) {
                Ok(v) if v <= 0 => 0,
                _ => got.min(CHUNK) as usize,
            };
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&self.cpu.mem.read_bytes(buf, n)?);
            if bytes.len() >= LIMIT {
                self.display.note(format!(
                    "an IAStream gave {LIMIT} byte(s) and still has not ended; the rest is not read"
                ));
                break;
            }
        }
        self.cpu.mem.free(buf);
        Ok(bytes)
    }

    /// One `Read` from an `IAStream`, which is what a caller that feeds a decompressor a window at a
    /// time needs.
    ///
    /// [`Runtime::drain_stream`] answers "everything the stream will give up", which is the wrong
    /// question for `IUnzipAStream`: there, the source is refilled *between* the caller's reads, so
    /// asking for everything once would end the stream at its first window. This asks once, and a
    /// short answer is passed on as it came.
    ///
    /// The host's own file handles are served from the volume rather than downcalled, for the same
    /// reason `IFile::Read` is: their position belongs to the volume, and a copy taken through the
    /// vtable would have to guess where to put it back.
    pub(crate) fn read_stream_chunk(&mut self, stream: u32, want: u32) -> HResult<Vec<u8>> {
        if stream == 0 || want == 0 {
            return Ok(Vec::new());
        }
        if let Some(open) = self.files.get(&stream).cloned() {
            let left = self
                .volume
                .info(&open.name)
                .map(|i| i.size.saturating_sub(open.pos))
                .unwrap_or(0);
            let take = want.min(left);
            if take == 0 {
                return Ok(Vec::new());
            }
            let bytes = self.volume.read(&open, take);
            if let Some(open) = self.files.get_mut(&stream) {
                open.pos = open.pos.saturating_add(bytes.len() as u32);
            }
            return Ok(bytes);
        }
        let down = |e: Error| match e {
            Error::Hle(h) => h,
            other => HleError::Unsupported {
                what: other.to_string(),
            },
        };
        let read = self.guest_slot(stream, "IAStream", "Read").map_err(down)?;
        let buf = self.cpu.mem.alloc_zeroed(want)?;
        let got = self.call_guest(read, &[stream, buf, want]).map_err(down)?;
        let n = match i32::try_from(got) {
            Ok(v) if v <= 0 => 0,
            _ => got.min(want) as usize,
        };
        let bytes = if n == 0 {
            Vec::new()
        } else {
            self.cpu.mem.read_bytes(buf, n)?
        };
        self.cpu.mem.free(buf);
        Ok(bytes)
    }
}
