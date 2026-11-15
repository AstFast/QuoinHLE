//! `Runtime`'s side of the Web interfaces: `IWeb`, `IWebResp`, `IWebUtil` and `IHtmlViewer`.
//!
//! Each interface's slots arrive here from the dispatch table in [`crate::runtime`], which is the
//! only place that knows an interface id from a method name; what happens afterwards is this file's
//! business and nothing else's. The four are one family: `IWeb::GetResponse` produces the `IWebResp`
//! that `IWebResp` answers for, `IWebUtil` is the string arithmetic those two need, and
//! `IHtmlViewer` is the control that renders what came back.
//!
//! Two of them were once `pub`-nothing here and the rest of the runtime had to reach in:
//! `build_response` is called from `poll_net` and `html_notify` from the same place, so those two
//! are `pub(crate)` and the rest are private to the module.
//!
//! The document model is [`crate::web::html`], the option ids [`crate::web::options`], and the
//! control's own record [`crate::web::widget`]. What is *not* here is the transport: a request goes
//! through [`crate::net`], which is also what `--net-proxy` stands for outside the browser.

use crate::abi::{self, generated as g};
use crate::hle::{Error as HleError, HResult};
use crate::net;
use crate::runtime::{html_bytes, Body, Error, Response, Runtime, Timer, Waiting, TEXT_CAP};
use crate::web;

/// One `<img src>` a page has asked the network for.
///
/// A document's own bytes are synchronous — the caller hands them over — but a picture it names by
/// URL is a request, and a request is the one thing a redraw must never wait for. This is the record
/// that ties a reply back to the page that wants it and to the attribute text the decode cache is
/// keyed by, so an answer arriving turns can later still lands in the right place.
pub(crate) struct HtmlFetch {
    /// The `IHtmlViewer` control that asked.
    pub(crate) ctl: u32,
    /// The `src` attribute exactly as the document wrote it: `image.src` is the cache key, and
    /// resolving it against `HVOPT_SZ_URL` must not change which picture is being remembered. The
    /// absolute address that was actually requested travels back on the reply, so it is not kept
    /// here as a second copy that could disagree with it.
    pub(crate) src: String,
}

impl Runtime {
    /// Every `IWebUtil` slot. The object holds no state, so each arm is the text work in
    /// [`crate::web`] plus the guest addresses it was handed and writes its answer back to.
    pub(crate) fn iwebutil(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `IWebUtil` inherits `IQI`, which `IShell` and `IHeap` do not, so the slot is real.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_WEBUTIL])
            }
            // The ten parts are offsets into the caller's own text, which is the point of the call:
            // the contract is that the string is untouched and outlives the struct, so a copy this
            // host owns would be the one thing that could not be promised.
            "ParseUrl" => {
                let [url, dst] = self.args()?;
                if url == 0 || dst == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                let parts = abi::struct_info("UrlParts").ok_or_else(unimplemented)?;
                let text = self.cpu.mem.read_cstr(url, TEXT_CAP)?;
                for (field, off) in parts.fields.iter().zip(web::parse_url(&text)) {
                    self.cpu.mem.write_u32(dst + field.off, url + off as u32)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "MakeUrl" => {
                let [base, rel, dst, len] = self.args()?;
                let base = self.bytes_at(base)?;
                let rel = self.bytes_at(rel)?;
                let (text, needed) = web::make_url(&base, &rel);
                if dst != 0 && len as i32 > 0 && needed <= len as usize {
                    self.cpu.mem.write_bytes(dst, &text)?;
                    self.cpu.mem.write_u8(dst + text.len() as u32, 0)?;
                    // What was written, which is the result's own length and not the larger number
                    // the sizing call reported: the dots that were taken out were never written.
                    return Ok(text.len() as u32 + 1);
                }
                if dst != 0 && len as i32 > 0 {
                    self.cpu.mem.write_u8(dst, 0)?;
                }
                Ok(needed as u32)
            }
            "UrlEncode" => self.transcode(false),
            "UrlDecode" => self.transcode(true),
            "EncodeBase64" => {
                let [src, len, dst] = self.args()?;
                let text = self.bytes_len(src, len)?;
                let out = web::b64_encode(&text);
                if dst == 0 {
                    return Ok(0);
                }
                self.cpu.mem.write_bytes(dst, &out)?;
                self.cpu.mem.write_u8(dst + out.len() as u32, 0)?;
                Ok(dst)
            }
            "DecodeBase64" => {
                let [src, dst] = self.args()?;
                let text = self.cpu.mem.read_cstr(src, TEXT_CAP)?;
                let (out, ok) = web::b64_decode(&text);
                if dst != 0 {
                    // Written first, whatever the answer is: the shipped member decodes as it walks
                    // and its `-1` has no rollback behind it (the delivered body precedes the checks
                    // at).
                    self.cpu.mem.write_bytes(dst, &out)?;
                }
                if !ok {
                    // The `-1` of a bogus string, in the same word the caller reads a length out of.
                    return Ok(u32::MAX);
                }
                // No terminator after the data: the documented answer is the length of the data
                // itself, so a caller that allocated exactly that would be written one past by here.
                Ok(out.len() as u32)
            }
            "ParseFormFields" => {
                let [ppin, awff, nmax, flags] = self.args()?;
                if ppin == 0 {
                    return Ok(0);
                }
                let start = self.cpu.mem.read_u32(ppin)?;
                if start == 0 {
                    return Ok(0);
                }
                let mut buf = self.cpu.mem.read_cstr(start, TEXT_CAP)?;
                // A text that ran to the cap has no terminator to walk back to, so it gets the one
                // answer that cannot write anywhere: point at the raw bytes and leave them alone.
                let terminated = buf.len() < TEXT_CAP;
                buf.push(0);
                // Two independent bits, read the way the body reads them: terminating
                // is off only for `WEBUTIL_PFFF_NOTERMINATE`, and decoding is off only for
                // `NODECODE`. `NOTERMINATE` is not in the published header — it is defined as
                // `0x02`, and the header's `NOMUTATE 0x03` is the two of them together. Testing
                // `NOMUTATE` alone was wrong in one direction that shows on screen: a caller that
                // asked only for `NODECODE` still gets its separators poked out, because on a
                // handset it does.
                let decode = terminated && flags & g::WEBUTIL_PFFF_NODECODE == 0;
                let mutate = terminated && flags & 0x02 == 0;
                let (fields, next) =
                    web::parse_form_fields(&mut buf, 0, nmax as usize, mutate, decode);
                if mutate {
                    self.cpu.mem.write_bytes(start, &buf)?;
                }
                if awff != 0 {
                    let form = abi::struct_info("WebFormField").ok_or_else(unimplemented)?;
                    let name_at =
                        abi::offset_of("WebFormField", "pcName").ok_or_else(unimplemented)?;
                    let value_at =
                        abi::offset_of("WebFormField", "pcValue").ok_or_else(unimplemented)?;
                    for (at, (name, value)) in fields.iter().enumerate() {
                        let slot = awff + at as u32 * form.size;
                        self.cpu
                            .mem
                            .write_u32(slot + name_at, start + *name as u32)?;
                        self.cpu
                            .mem
                            .write_u32(slot + value_at, start + *value as u32)?;
                    }
                }
                self.cpu.mem.write_u32(ppin, start + next as u32)?;
                Ok(fields.len() as u32)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IWEBUTIL_UrlEncode` and `UrlDecode`, which are one shape: an input with a length pointer,
    /// an output with a capacity pointer, and `pcOut` handed back.
    ///
    /// Both length words count **data bytes**, not the terminator. `nOutLen = MIN(needed,
    /// *pnOutLen)` (and for the decode half) spends the caller's whole
    /// `sizeof(pcOut)` on the text and then writes the text with no terminator after it (
    ///); the length handed back through `pnOutLen` is that data length.
    /// A caller that wants a C string counts its own terminator into the size it passes, exactly as
    /// it must on a handset — and one that reserved the size this reports and got a byte written
    /// past it is the failure this host used to be able to cause by being one byte tidier than the
    /// shipped member.
    ///
    /// The two halves differ in counting mode: `UrlEncode` jumps to `done` at, past the
    /// write at, so a sizing call leaves `*pnInLen` exactly as the caller set it, while
    /// `UrlDecode` has no such branch and reports how far it walked even with a null output
    ///.
    fn transcode(&mut self, decode: bool) -> HResult<u32> {
        let [src, pin_len, dst, pout_len] = self.args()?;
        let declared = (pin_len != 0)
            .then(|| self.cpu.mem.read_u32(pin_len))
            .transpose()?
            .unwrap_or(u32::MAX);
        let text = self.bytes_len(src, declared)?;
        let capacity = (pout_len != 0)
            .then(|| self.cpu.mem.read_u32(pout_len))
            .transpose()?
            .map_or(usize::MAX, |n| (n as i32).max(0) as usize);
        let limit = if dst == 0 { usize::MAX } else { capacity };
        let (out, used) = if decode {
            web::url_decode(&text, limit)
        } else {
            web::url_encode(&text, limit)
        };
        if dst != 0 {
            self.cpu.mem.write_bytes(dst, &out)?;
        }
        if pin_len != 0 && (dst != 0 || decode) {
            self.cpu.mem.write_u32(pin_len, used as u32)?;
        }
        if pout_len != 0 {
            self.cpu.mem.write_u32(pout_len, out.len() as u32)?;
        }
        Ok(dst)
    }

    /// Every `IWeb` slot: the request object, and the `IxOpts` option list it inherits.
    ///
    /// The pack that asks for it builds a full URL with `IWebUtil`, creates this to carry the
    /// request, and stores the pointer. What never arrives is a response — the shape of a handset with
    /// no connection — and the half that used to be missing is *how* the pack is told: its own
    /// `AEECallback`, which `GetResponse` now queues instead of leaving uncalled.
    pub(crate) fn iweb(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if matches!(iid, g::AEEIID_IQI | g::AEECLSID_WEB | g::AEECLSID_WEBOPTS) {
                    return self.return_object(this, ppv);
                }
                Ok(g::AEE_ECLASSNOTSUPPORT)
            }
            "AddOpt" => {
                let [items] = self.args()?;
                self.ixopts_add_list(this, items)
            }
            "RemoveOpt" => {
                let [nopt, index] = self.args()?;
                self.ixopts_remove(this, nopt as i32, index as i32)
            }
            "GetOpt" => {
                let [nopt, index, pitem] = self.args()?;
                self.ixopts_get(this, nopt as i32, index as i32, pitem)
            }
            "GetResponse" | "GetResponseV" => {
                let [ppresp, pcb, url] = self.args()?;
                // `cpszUrl` "must stay valid until the above AEECallback fires, cpszUrl is not
                // copied": the caller owns that string, so there is nothing to do
                // with it here and nothing to free when the callback runs.
                //
                // The options this call carries — varargs for `GetResponse`, an array for
                // `GetResponseV` — go to "an IWebOpts object that is then handed to the protocol
                // handler", and they are "*overrides* to any options that are already set in the
                // IWeb object". They belong to the transaction, and this host
                // has no protocol handler to hand them to, so there is nowhere for them to go: they
                // are deliberately NOT pushed onto this object's own list, because "overrides" is
                // exactly the distinction that says a later `GetOpt` on the object must not find
                // them — and on a handset it would not.
                // No response object: this host has no network, so there is nothing to hand back.
                if ppresp != 0 {
                    self.cpu.mem.write_u32(ppresp, 0)?;
                }
                // "pcb: where to call when answer is ready". The answer, on a handset
                // with no connection, is a failure — and it has to arrive *through this callback*,
                // because the prototype's own return is `None`. A host that wrote the
                // null response and never called back would leave a pack waiting for an answer that is
                // not coming, which is the one shape this project refuses to ship.
                //
                // Queued rather than called inline: an `AEECallback` comes due later (it is the same
                // struct `ISHELL_SetTimerEx` takes — see [`Timer`]), and a pack that is still inside
                // `GetResponse` must not have its callback re-entered from under it.
                //
                // With `--net-proxy` the request really goes out, and this is where the two endings
                // part. The callback is then queued *when the answer arrives*
                // ([`Runtime::poll_net`]) rather than here, because "answer is ready" is precisely
                // what a real request has not said yet. A request the proxy refuses to take is the
                // failure path and still queues it right here, so that no caller is left waiting on a
                // reply that is never coming.
                let asked = if url == 0 {
                    String::new()
                } else {
                    String::from_utf8_lossy(&self.cpu.mem.read_cstr(url, TEXT_CAP)?).into_owned()
                };
                if let Some(answer) = self.script_net(&asked) {
                    let at = u64::from(self.clock_ms);
                    self.net_log.push((at, format!("-> GET {asked} (script)")));
                    if ppresp != 0 {
                        match self.build_response(&answer) {
                            Ok(Some(response)) => self.cpu.mem.write_u32(ppresp, response)?,
                            Ok(None) | Err(_) => self.net_log.push((
                                at,
                                format!("<- {asked} answered, but no response object was laid out"),
                            )),
                        }
                    }
                    self.net_log.push((
                        at,
                        format!(
                            "<- {} {} ({} bytes, script)",
                            answer.status,
                            asked,
                            answer.body.len()
                        ),
                    ));
                    if pcb != 0 {
                        self.timers.push(Timer {
                            due: self.clock_ms,
                            pfn: pcb,
                            user: pcb,
                            owner: self.applet,
                        });
                    }
                    return Ok(0);
                }
                let mut waiting_for_reply = false;
                if let Some(proxy) = self.net.as_ref() {
                    // `read_cstr` answers bytes rather than text, because a guest string *is* bytes:
                    // a URL that is not UTF-8 cannot be sent anywhere, and the lossy conversion is
                    // what keeps that visible — the proxy receives the replacement characters and
                    // answers, instead of this host quietly inventing a different URL to fetch.
                    let asked = if url == 0 {
                        String::new()
                    } else {
                        String::from_utf8_lossy(&self.cpu.mem.read_cstr(url, TEXT_CAP)?)
                            .into_owned()
                    };
                    let id = self.next_net_id;
                    self.next_net_id += 1;
                    let at = u64::from(self.clock_ms);
                    match proxy.ask(id, asked.clone()) {
                        Ok(()) => {
                            waiting_for_reply = true;
                            if pcb != 0 || ppresp != 0 {
                                self.net_waiting.insert(id, Waiting { ppresp, pcb });
                            }
                            self.net_log.push((at, format!("-> GET {asked} (id {id})")));
                        }
                        Err(why) => {
                            self.net_log
                                .push((at, format!("-> GET {asked} (id {id}) not sent: {why}")));
                        }
                    }
                }
                if !waiting_for_reply && pcb != 0 {
                    self.timers.push(Timer {
                        due: self.clock_ms,
                        pfn: pcb,
                        user: pcb,
                        owner: self.applet,
                    });
                }
                Ok(0)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Lay out the two guest objects one answered request is handed back as: an `IWebResp` whose
    /// `GetInfo` names a `WebRespInfo`, and the `ISource` in that structure that carries the bytes.
    ///
    /// Two objects and not one, because `WebRespInfo` has no member for the body: the bytes are
    /// reachable only through `pisMessage`, which is an `ISource` the guest reads. A host that filled
    /// the structure and left that field null would be telling a pack its answer had a length and no
    /// content — the silently-wrong shape this project refuses to ship.
    ///
    /// What gets filled in is what arrived: the code, the length, and the content type when the
    /// server named one. `tExpires` and `tModified` stay zero, which is what a pack reads as "not
    /// given": they are optional headers this host would have to parse dates out of, and a wrong date
    /// is worse than an absent one.
    /// Copy one host string into the **guest heap**, NUL-terminated, answering 0 on exhaustion.
    ///
    /// `heap_alloc` rather than `Mem::alloc`: these blocks are named by a `char *` in a structure the
    /// guest owns and are given back by `IShell_Free` semantics when the response is released, so
    /// they belong to the same arena the guest's own `MALLOC` draws from.
    fn guest_heap_text(&mut self, value: &str) -> HResult<u32> {
        let bytes = value.as_bytes();
        let addr = self.heap_alloc(bytes.len() as u32 + 1)?;
        if addr == 0 {
            return Ok(0);
        }
        self.cpu.mem.write_bytes(addr, bytes)?;
        self.cpu.mem.write_u8(addr + bytes.len() as u32, 0)?;
        Ok(addr)
    }

    pub(crate) fn build_response(&mut self, answer: &net::Answer) -> HResult<Option<u32>> {
        let info_size = abi::struct_info("WebRespInfo")
            .expect("WebRespInfo is in the ABI tables")
            .size;
        let at = |name: &str| abi::offset_of("WebRespInfo", name).expect("in the ABI tables");
        // One header, two ABI fields: `text/html; charset=utf-8` becomes `text/html` in
        // `cpszContentType` and `utf-8` in `cpszCharset`. A header that names no media type keeps
        // its whole text in the first field rather than publishing an empty MIME token.
        let (content_type, charset) = match answer.content_type.as_deref() {
            Some(value) => {
                let (media, charset) = crate::web::split_content_type(value);
                let media = if media.is_empty() {
                    value.to_string()
                } else {
                    media
                };
                (Some(media), charset)
            }
            None => (None, None),
        };
        // Everything the guest heap is asked for comes first, and `0` is what it says when it cannot
        // — `heap_alloc` has `malloc`'s shape, not an error channel. Laying the objects out first
        // would register them and then fail, leaving two objects nobody can reach and nobody frees.
        let info = self.heap_alloc(info_size)?;
        if info == 0 {
            return Ok(None);
        }
        let text = match content_type.as_deref() {
            Some(kind) => {
                let addr = self.guest_heap_text(kind)?;
                if addr == 0 {
                    self.hle.dealloc(&mut self.cpu.mem, info);
                    return Ok(None);
                }
                addr
            }
            None => 0,
        };
        let charset_at = match charset.as_deref() {
            Some(set) => {
                let addr = self.guest_heap_text(set)?;
                if addr == 0 {
                    self.hle.dealloc(&mut self.cpu.mem, info);
                    if text != 0 {
                        self.hle.dealloc(&mut self.cpu.mem, text);
                    }
                    return Ok(None);
                }
                addr
            }
            None => 0,
        };
        let source =
            self.hle
                .new_object(&mut self.cpu.mem, "ISource", g::AEECLSID_SOURCE, 8, 4, &[])?;
        let resp = self.hle.new_object(
            &mut self.cpu.mem,
            "IWebResp",
            g::AEECLSID_WEBRESP,
            8,
            4,
            &[],
        )?;
        self.cpu
            .mem
            .write_bytes(info, &vec![0u8; info_size as usize])?;
        self.cpu
            .mem
            .write_u32(info + at("nCode"), u32::from(answer.status))?;
        self.cpu
            .mem
            .write_u32(info + at("lContentLength"), answer.body.len() as u32)?;
        self.cpu.mem.write_u32(info + at("pisMessage"), source)?;
        if text != 0 {
            self.cpu.mem.write_u32(info + at("cpszContentType"), text)?;
        }
        if charset_at != 0 {
            self.cpu
                .mem
                .write_u32(info + at("cpszCharset"), charset_at)?;
        }
        self.web_responses.insert(
            resp,
            Response {
                info,
                source,
                text,
                charset: charset_at,
                body: answer.body.clone(),
            },
        );
        self.bodies.insert(source, Body { resp, pos: 0 });
        Ok(Some(resp))
    }

    /// Every `IWebResp` slot: the answered request, and the `IxOpts` list it inherits.
    ///
    /// `GetInfo` is the whole of the interface's own surface. The structure it hands back is where
    /// the caller reads the code, the length and the content type, and `pisMessage` is where it finds
    /// the bytes.
    pub(crate) fn iwebresp(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_WEBRESP])
            }
            "GetInfo" => match self.web_responses.get(&this) {
                Some(response) => Ok(response.info),
                None => Err(unimplemented()),
            },
            "AddOpt" => {
                let [items] = self.args()?;
                self.ixopts_add_list(this, items)
            }
            "RemoveOpt" => {
                let [nopt, index] = self.args()?;
                self.ixopts_remove(this, nopt as i32, index as i32)
            }
            "GetOpt" => {
                let [nopt, index, pitem] = self.args()?;
                self.ixopts_get(this, nopt as i32, index as i32, pitem)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IHtmlViewer`'s own slots (slots 11-23).
    ///
    /// The host keeps the document, the `HVOPT_*` table and the view state on the control record;
    /// the three DOM queries answer `IHTMLVIEWER_NOTFOUND` (-1) until the document is parsed, which
    /// is the header's own "no such element" and what every caller already tests for.
    pub(crate) fn ihtmlviewer(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> HResult<u32> {
        let gap = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "SetOption" => {
                let [option, value] = self.args()?;
                let id = option as i32;
                // The shipped setter carries a whitelist, not a table write:
                //the two strings, three counters clamped into
                // 0..=100, two colours and the `IWeb` pointer. Anything else answers
                // `EUNSUPPORTED` and stores nothing, which is why this host cannot be talked into
                // remembering an id it would otherwise answer 0 for.
                const WHITE: &[i32] = &[
                    crate::web::html::HVOPT_SZ_TYPE,
                    crate::web::html::HVOPT_SZ_URL,
                    crate::web::html::HVOPT_N_INDENT,
                    crate::web::html::HVOPT_N_VSPACE,
                    crate::web::html::HVOPT_N_SBWIDTH,
                    crate::web::html::HVOPT_RGB_TEXT,
                    crate::web::html::HVOPT_RGB_LINK,
                    crate::web::html::HVOPT_PTR_IWEB,
                ];
                if !WHITE.contains(&id) {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                // `DoxConstrain(x, 0, 100)` on the three counters,
                // and `a3` is that function's **signed** argument: a pack writing -1 gets 0,
                // the lower bound, not 100. Clamping the raw `u32` word would send every negative
                // count to the upper bound instead — the one direction `DoxConstrain` cannot go.
                if matches!(
                    id,
                    crate::web::html::HVOPT_N_INDENT
                        | crate::web::html::HVOPT_N_VSPACE
                        | crate::web::html::HVOPT_N_SBWIDTH
                ) {
                    self.widgets
                        .html_set_option(this, id, (value as i32).clamp(0, 100) as u32)?;
                } else {
                    self.widgets.html_set_option(this, id, value)?;
                }
                // A scrollbar column the page did not have is a different layout: the shipped
                // setter runs its whole `SetRect` again for exactly this option,
                //and this host's `html_fit` is that re-run.
                if id == crate::web::html::HVOPT_N_SBWIDTH {
                    self.html_fit(this)?;
                }
                // The two string options are kept as text as well as in the table, because the
                // pointer the caller passed is only promised for the length of this call. A later
                // `HVN_JUMP` cannot resolve a relative `href` against `HVOPT_SZ_URL` once the pack
                // has freed its copy, and the next parse cannot know the document is `text/plain`
                // unless `HVOPT_SZ_TYPE` was copied here. Every other id is a number and is left in
                // the table alone — reading a small integer as a string would be a wild pointer.
                let id = option as i32;
                if id == crate::web::html::HVOPT_SZ_URL || id == crate::web::html::HVOPT_SZ_TYPE {
                    let text = if value == 0 {
                        String::new()
                    } else {
                        String::from_utf8_lossy(&self.cpu.mem.read_cstr(value, TEXT_CAP)?)
                            .into_owned()
                    };
                    if id == crate::web::html::HVOPT_SZ_URL {
                        self.widgets.html_set_base_url(this, text)?;
                    } else {
                        self.widgets.html_set_doc_type(this, text)?;
                    }
                }
                // `HVOPT_PTR_IWEB` is a pointer, so it is never read as text — it only says whether
                // the page has a transport. All three of these can make a `src` resolvable that was
                // not before, so the picture scan runs again after any of them.
                // `HVOPT_PTR_IWEB` arrives as an object the pack created. The shipped setter
                // gates it on a `QueryInterface` for `AEECLSID_CONTROL` and answers `EBADPARM`
                // when the object refuses; this host has no
                // machine code to run the guest's `QueryInterface`, so the gate it can honestly
                // keep is "a pointer the host knows": anything else is refused on the same
                // error, because dereferencing it is exactly what the handset would crash on.
                if id == crate::web::html::HVOPT_PTR_IWEB && value != 0 {
                    if self.hle.object(value).is_none() {
                        return Ok(g::AEE_EBADPARM);
                    }
                }
                if id == crate::web::html::HVOPT_PTR_IWEB
                    || id == crate::web::html::HVOPT_SZ_URL
                    || id == crate::web::html::HVOPT_SZ_TYPE
                {
                    self.html_fetch_images(this)?;
                }
                Ok(0)
            }
            "GetOption" => {
                let [option, pv] = self.args()?;
                let id = option as i32;
                // The shipped getter answers the six numeric options it stores and nothing else:
                //the string options answer `EUNSUPPORTED`
                // without a write, and so does any id outside the pool. `HVOPT_N_ERROR` is one of
                // the six — get-only, zeroed at birth, never written by a setter this host can
                // name — so 0 ("no error") is what it answers.
                const READS: &[i32] = &[
                    crate::web::html::HVOPT_N_INDENT,
                    crate::web::html::HVOPT_N_VSPACE,
                    crate::web::html::HVOPT_N_SBWIDTH,
                    crate::web::html::HVOPT_N_ERROR,
                    crate::web::html::HVOPT_RGB_TEXT,
                    crate::web::html::HVOPT_RGB_LINK,
                ];
                let value = self.widgets.html_option(this, id)?;
                if READS.contains(&id) {
                    if pv != 0 {
                        self.cpu.mem.write_u32(pv, value)?;
                    }
                    return Ok(0);
                }
                if id == crate::web::html::HVOPT_PTR_IWEB {
                    // A quirk worth keeping because a caller that checks the return value sees it:
                    // the shipped getter writes the stored pointer and *then* answers
                    // `EUNSUPPORTED` (the delivered body falls through to the same
                    // `return 20` every unknown id takes).
                    if pv != 0 {
                        self.cpu.mem.write_u32(pv, value)?;
                    }
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                Ok(g::AEE_EUNSUPPORTED)
            }
            // `LoadStream` takes an `IAStream*` and `LoadSource` an `ISource*`. Both are drained
            // synchronously into the same `Html::data` that `SetData` fills, so the three DOM
            // queries and `Redraw` see the bytes the caller actually handed over. The old shape
            // only remembered the pointer and sent `HVN_DONE` for an empty document.
            "LoadStream" | "LoadSource" => {
                let [stream] = self.args()?;
                let iface = if name == "LoadStream" {
                    "IAStream"
                } else {
                    "ISource"
                };
                let (bytes, ok) = self.html_drain_source(stream, iface);
                self.widgets.html_set_bytes(this, bytes)?;
                self.html_load_finish(this, ok)?;
                Ok(0)
            }
            "SetNotifyFn" => {
                let [notify, user] = self.args()?;
                self.widgets.html_set_notify(this, notify, user)?;
                Ok(0)
            }
            "SetData" => {
                let [psz, len] = self.args()?;
                // The shipped body cancels whatever load is in flight before it parses, and the
                // cancel is what tells the caller the previous load is over.
                self.html_cancelled_notify(this)?;
                self.widgets
                    .html_set_data(&self.cpu.mem, this, psz, len as i32)?;
                // `SetData` is the caller's own bytes: there is no read that could fail. A null
                // pointer is an empty document, which is a page of nothing rather than an error.
                self.html_load_finish(this, true)?;
                Ok(0)
            }
            "IsBusy" => Ok(u32::from(self.widgets.html_busy(this)?)),
            "Stop" => {
                self.html_cancelled_notify(this)?;
                Ok(0)
            }
            "FindElem" => {
                let [psz, index] = self.args()?;
                let tag = self.cpu.mem.read_cstr(psz, 1 << 8).map_err(HleError::Mem)?;
                let tag = String::from_utf8_lossy(&tag).to_ascii_lowercase();
                let doc = self.widgets.html_document(this)?;
                Ok(doc
                    .find(&tag, index as i32)
                    .map(|at| at as u32)
                    .unwrap_or(g::IHTMLVIEWER_NOTFOUND))
            }
            "GetElemText" => {
                let [elem, buf, cb] = self.args()?;
                let doc = self.widgets.html_document(this)?;
                if elem == g::IHTMLVIEWER_NOTFOUND || elem as usize >= doc.elements.len() {
                    return Ok(g::IHTMLVIEWER_BADELEMENT);
                }
                let text = doc.text(elem as usize).to_string();
                // A null buffer is the "how many bytes do I need" call, which is the shape a caller
                // has before it knows the length. Address 0 is not a buffer, so nothing is written
                // and the answer is still the length — `html_get_state` was the only one of these
                // that already told the two apart. A capacity of 0 is the same answer: `html_bytes`
                // would still write the terminator, one byte past what the caller offered.
                if buf != 0 && cb != 0 {
                    self.cpu
                        .mem
                        .write_bytes(buf, &html_bytes(&text, cb as i32))
                        .map_err(HleError::Mem)?;
                }
                Ok(text.len() as u32)
            }
            "GetAttrValue" => {
                let [elem, attr, buf, cb] = self.args()?;
                let doc = self.widgets.html_document(this)?;
                if elem == g::IHTMLVIEWER_NOTFOUND || elem as usize >= doc.elements.len() {
                    return Ok(g::IHTMLVIEWER_BADELEMENT);
                }
                let name = self
                    .cpu
                    .mem
                    .read_cstr(attr, 1 << 8)
                    .map_err(HleError::Mem)?;
                let name = String::from_utf8_lossy(&name).to_ascii_lowercase();
                let Some(value) = doc.attr(elem as usize, &name) else {
                    return Ok(g::IHTMLVIEWER_BADATTR);
                };
                let value = value.to_string();
                // Same split as `GetElemText`: a null `buf` asks for the length and gets no write,
                // and so does a zero capacity. An attribute that is present and empty answers 0
                // rather than `BADATTR`, because the attribute *is* there — `<a href>` has an
                // `href`, it just names nothing.
                if buf != 0 && cb != 0 {
                    self.cpu
                        .mem
                        .write_bytes(buf, &html_bytes(&value, cb as i32))
                        .map_err(HleError::Mem)?;
                }
                Ok(value.len() as u32)
            }
            "SetViewState" => {
                let [psz] = self.args()?;
                let state = if psz == 0 {
                    String::new()
                } else {
                    String::from_utf8_lossy(
                        &self
                            .cpu
                            .mem
                            .read_cstr(psz, 1 << 16)
                            .map_err(HleError::Mem)?,
                    )
                    .into_owned()
                };
                let doc = self.widgets.html_document(this)?;
                self.widgets
                    .html_set_state(this, &doc, &state, self.display.faces())?;
                // The shipped reader clamps the offset it was given into
                // `[0, content - view height]` before it applies anything:
                //a state saved on a taller control is a
                // position this page may not have.
                let limit = self.html_scroll_limit(this)?.max(0);
                let html = self.widgets.html(this)?;
                html.scroll = html.scroll.clamp(0, limit);
                Ok(0)
            }
            "GetViewState" => {
                let [buf, cb] = self.args()?;
                self.widgets
                    .html_get_state(&mut self.cpu.mem, this, buf, cb as i32)
            }
            _ => Err(gap()),
        }
    }

    /// One `IAStream::Read` / `ISource::Read`, on whichever side of the seam owns `stream`.
    ///
    /// A host-made object answers through its own slot (the `IAStream` face forwards to the record
    /// the class was created under); an object the pack built is called through its vtable. This is
    /// the same split [`Runtime::stream_read`] uses for the media sniffer.
    fn html_read(&mut self, stream: u32, iface: &str, buf: u32, len: u32) -> Result<u32, Error> {
        // A host-made object is dispatched under the class it was created with, not the
        // interface the caller named: `ISourceUtil_SourceFromMemory` hands back an `IPeek`
        // through an `ISource*` parameter, and `Read` is slot 3 of either. Asking for the
        // object's own face first is what makes `LoadSource` work on that object too.
        if let Some(face) = self.hle.object(stream).map(|o| o.iface) {
            if let Some(id) = abi::method_of(face, "Read") {
                return self.nested_method(id, stream, face, &[buf, len], 0);
            }
        }
        let Some(id) = abi::method_of(iface, "Read") else {
            return Ok(0);
        };
        if self.hle.object(stream).is_some() {
            return self.nested_method(id, stream, iface, &[buf, len], 0);
        }
        let slot = self.guest_slot(stream, iface, "Read")?;
        self.call_guest(slot, &[stream, buf, len])
    }

    /// Drain an `IAStream*` / `ISource*` into host bytes, synchronously.
    ///
    /// `Read` answers 0 at EOF and a negative `AEE_NET_*` code (seen here as a large `u32`) when it
    /// would block; neither is a byte count. A synchronous load cannot wait for a callback, so both
    /// end the drain with what has arrived so far. The limit is the same 1 MiB `SetData` accepts,
    /// so a stream that never ends cannot grow the document without bound.
    fn html_drain_source(&mut self, stream: u32, iface: &str) -> (Vec<u8>, bool) {
        const CHUNK: u32 = 4096;
        const LIMIT: usize = 1 << 20;
        if stream == 0 {
            return (Vec::new(), false);
        }
        let Ok(buf) = self.cpu.mem.alloc(CHUNK, 1) else {
            return (Vec::new(), false);
        };
        let mut out = Vec::new();
        // `Read` answering 0 is EOF and a negative code is "would block"; neither is a failure. A
        // read that could not be *made* — a null vtable slot, a guest that faulted — is, and that is
        // what the caller's `HVN_DONE` reports as ERROR.
        let mut ok = true;
        while out.len() < LIMIT {
            let Ok(got) = self.html_read(stream, iface, buf, CHUNK) else {
                ok = false;
                break;
            };
            if got == 0 || got > CHUNK {
                break;
            }
            let Ok(bytes) = self.cpu.mem.read_bytes(buf, got as usize) else {
                ok = false;
                break;
            };
            out.extend_from_slice(&bytes);
        }
        self.cpu.mem.free(buf);
        (out, ok)
    }

    /// The three notifications the header documents as sent synchronously from a
    /// load: all content arrived, the page was parsed, and the load is done. They are sent in that
    /// order, and each goes through the guest callback `SetNotifyFn` installed rather than being
    /// queued. `HVN_DONE` is last so a caller that only acts on the final code sees what it used to.
    fn html_load_finish(&mut self, this: u32, ok: bool) -> HResult<()> {
        // A new document is a new set of pictures: whatever the last one was waiting for is no
        // longer wanted, and its replies will arrive to no entry here.
        self.html_cancel_fetches(this);
        self.widgets.html_set_busy(this, false)?;
        self.html_notify(this, g::HVN_CONTENTDONE, 0)?;
        self.html_notify(this, g::HVN_PAGEDONE, 0)?;
        // `HVN_DONE`'s `nDone` is "SUCCESS or ERROR", so a stream that
        // could not be read says so instead of claiming a page it never had. Every other code in
        // this family carries no payload and keeps its zero.
        let done = if ok { g::AEE_SUCCESS } else { g::AEE_EFAILED };
        self.html_notify(this, g::HVN_DONE, done)?;
        // The bytes are in; the pictures they name by URL are not. This is the whole of what
        // `IsBusy` reports after a load: true until the last one arrives, and false at once for a
        // page that names none — which is the common case, and stays as synchronous as it was.
        self.html_fetch_images(this)?;
        Ok(())
    }

    /// Ask the network for the pictures a page names by URL, and nothing else.
    ///
    /// Called after every load and again whenever `HVOPT_PTR_IWEB` or `HVOPT_SZ_URL` is set: the
    /// first is what says the page has a transport at all, the second is what a relative `src` is
    /// resolved against, so a pack that sets either of them after `SetData` still gets its pictures.
    /// It is idempotent — a `src` already decoded, already refused, or already in flight is skipped
    /// — so a second call costs a walk of `doc.images`.
    ///
    /// A `data:` URI is skipped because those bytes are in the document already, and `Redraw`
    /// decodes them where it finds them. Everything else has to be fetched, and a run with no
    /// `--net-proxy`/`--net-script` has nowhere to fetch from — which is the answer a handset with
    /// no data connection gives, and is not an error.
    fn html_fetch_images(&mut self, this: u32) -> HResult<()> {
        if self.net.is_none()
            || self
                .widgets
                .html_option(this, crate::web::html::HVOPT_PTR_IWEB)?
                == 0
        {
            return Ok(());
        }
        let doc = self.widgets.html_document(this)?;
        if doc.images.is_empty() {
            return Ok(());
        }
        // "By default, only images with WIDTH and HEIGHT attributes are loaded";
        //`HVP_LOADALLIMAGES` is the flag that lifts it — "images
        // will be loaded even if width and height are not specified or if images are large".
        //The control's own rect stands in for "the screen": a viewer that fills the
        // screen cannot tell the two apart, and one that does not is the case the flag is for.
        let load_all = self.widgets.properties(this)? & g::HVP_LOADALLIMAGES != 0;
        let view = self.widgets.peek(this)?.rect;
        // Resolve first, ask second: the walk needs the page's record and the request needs
        // `self.net`, and holding both at once is what the borrow checker is there to stop.
        let mut wanted: Vec<(String, String)> = Vec::new();
        for image in &doc.images {
            if image.background {
                continue;
            }
            let src = image.src.as_str();
            if src.is_empty() || src.starts_with("data:") {
                continue;
            }
            if !load_all {
                let unnamed = image.width == 0 || image.height == 0;
                let large = view.dx > 0
                    && view.dy > 0
                    && (i32::from(image.width) > i32::from(view.dx)
                        || i32::from(image.height) > i32::from(view.dy));
                if unnamed || large {
                    continue;
                }
            }
            if self.widgets.html_picture_settled(this, src)? {
                continue;
            }
            if self
                .html_fetches
                .values()
                .any(|f| f.ctl == this && f.src == src)
            {
                continue;
            }
            let url = self.resolve_html_url(this, src)?;
            // A relative `src` with no `HVOPT_SZ_URL` to sit against names nothing this host can
            // request. It is left unresolved rather than guessed at against the working directory,
            // which would be a different address than the document meant.
            if !url.contains("://") {
                continue;
            }
            wanted.push((src.to_string(), url));
        }
        let mut started = 0;
        for (src, url) in wanted {
            let id = self.next_net_id;
            self.next_net_id += 1;
            let at = u64::from(self.clock_ms);
            let sent = match self.net.as_ref() {
                Some(proxy) => proxy.ask(id, url.clone()),
                None => break,
            };
            match sent {
                Ok(()) => {
                    self.net_log
                        .push((at, format!("-> GET {url} (picture for {src})")));
                    self.html_fetches.insert(id, HtmlFetch { ctl: this, src });
                    started += 1;
                }
                Err(why) => self.net_log.push((
                    at,
                    format!("-> GET {url} (picture for {src}) not sent: {why}"),
                )),
            }
        }
        if started > 0 {
            self.widgets.html_set_busy(this, true)?;
        }
        Ok(())
    }

    pub(crate) fn html_load_background(&mut self, this: u32) -> HResult<()> {
        let doc = self.widgets.html_document(this)?;
        let Some(index) = doc.bg_image else {
            return Ok(());
        };
        let src = match doc.images.get(index) {
            Some(image) => image.src.clone(),
            None => return Ok(()),
        };
        if self.widgets.html_picture_settled(this, &src)? {
            return Ok(());
        }
        let found: Option<(Vec<u8>, Vec<u8>)> =
            if let Some(bytes) = crate::web::html::image_bytes(&src) {
                Some((Vec::new(), bytes))
            } else if let Some(f) =
                self.resources
                    .find(Some(&src), crate::resource::RESTYPE_IMAGE, 0)
            {
                Some((f.props, f.body))
            } else {
                self.volume
                    .open(&src, g::_OFM_READ)
                    .ok()
                    .map(|open| self.volume.read(&open, u32::MAX))
                    .filter(|bytes| !bytes.is_empty())
                    .map(|bytes| (Vec::new(), bytes))
            };
        let picture = found
            .as_ref()
            .and_then(|(props, body)| crate::resource::image(props, body).ok())
            .map(std::sync::Arc::new);
        self.widgets.html_store_picture(this, &src, picture)?;
        Ok(())
    }

    /// Forget the pictures a page is waiting for. The replies still arrive — a request already sent
    /// cannot be recalled — and one that finds no entry here is logged as an ordinary reply and
    /// otherwise dropped, which is what a cancelled load should do with it.
    pub(crate) fn html_cancel_fetches(&mut self, this: u32) {
        self.html_fetches.retain(|_, fetch| fetch.ctl != this);
    }

    fn html_cancelled_notify(&mut self, this: u32) -> HResult<()> {
        let had_fetches = !self.html_fetches.is_empty()
            && self.html_fetches.values().any(|fetch| fetch.ctl == this);
        self.html_cancel_fetches(this);
        let busy = self.widgets.html_busy(this)?;
        self.widgets.html_set_busy(this, false)?;
        if had_fetches || busy {
            self.html_notify(this, g::HVN_DONE, g::AEE_EINCOMPLETEITEM)?;
        }
        Ok(())
    }

    /// Post a `HViewNotify` to the callback `SetNotifyFn` installed. The header documents `HVN_DONE`
    /// and its siblings as sent synchronously from the load, so this calls the guest directly rather
    /// than queueing anything.
    ///
    /// The record is 24 bytes (`structs.rs`: `code` 0, `phv` 4, `u` 8), and only the union member the
    /// code names is written — `nDone` at `+8` for `HVN_DONE`.
    pub(crate) fn html_notify(&mut self, this: u32, code: u32, done: u32) -> HResult<()> {
        let (notify, _) = self.widgets.html_notify_target(this)?;
        if notify == 0 {
            return Ok(());
        }
        let rec = self.cpu.mem.alloc(24, 4).map_err(HleError::Mem)?;
        self.cpu.mem.write_u32(rec, code)?;
        self.cpu.mem.write_u32(rec + 4, this)?;
        self.cpu.mem.write_u32(rec + 8, done)?;
        self.html_notify_record(this, rec)
    }

    /// `HVN_INVALIDATE`, for a pack that set `HVP_INVALIDATION`.
    ///
    /// The flag is the header's "expose invalidation behaviour", and the
    /// property documentation says it plainly: "HVP_INVALIDATION enables HVN_INVALIDATE
    /// notification". A host that sent the code anyway would be handing a pack a notification it
    /// has no case for — and a pack that switches on `HViewNotify.code` without a default arm would
    /// take it for something else.
    pub(crate) fn html_notify_invalidate(&mut self, this: u32) -> HResult<()> {
        if self.widgets.properties(this)? & g::HVP_INVALIDATION == 0 {
            return Ok(());
        }
        self.html_notify(this, g::HVN_INVALIDATE, 0)
    }

    /// The `HViewFocus` report: `posElem@0`, `nType@4`, `nState@8`, at the union's offset (`+8`).
    fn html_notify_focus(&mut self, this: u32, elem: usize, kind: u32, state: i32) -> HResult<()> {
        let (notify, _) = self.widgets.html_notify_target(this)?;
        if notify == 0 {
            return Ok(());
        }
        let rec = self.cpu.mem.alloc(24, 4).map_err(HleError::Mem)?;
        self.cpu.mem.write_u32(rec, g::HVN_FOCUS)?;
        self.cpu.mem.write_u32(rec + 4, this)?;
        self.cpu.mem.write_u32(rec + 8, elem as u32)?;
        self.cpu.mem.write_u32(rec + 12, kind)?;
        // `HViewFocus.nState` is "state, dependent upon type": 1 is focus
        // gained for a link or an editor, but a box reports whether it is ticked and a list reports
        // which option it shows. Leaving it at 0 read as "lost" to a caller that branches on it.
        self.cpu.mem.write_u32(rec + 16, state as u32)?;
        self.html_notify_record(this, rec)
    }

    /// The `HViewJump` report (`HVN_JUMP` and `HVN_SUBMIT` share the record): `posElem@0`,
    /// `pszURL@4`, `pszMethod@8`, `pszData@12`, at the union's offset (`+8`). The URL lives in guest
    /// memory until the callback returns, which is what the header's `const char *` means.
    fn html_notify_jump(
        &mut self,
        this: u32,
        doc: &crate::web::html::Document,
        elem: usize,
        code: u32,
        point: (i16, i16),
    ) -> HResult<()> {
        let (notify, _) = self.widgets.html_notify_target(this)?;
        if notify == 0 {
            return Ok(());
        }
        // A link carries its URL in `href`; a submit carries the form's `action`, its `method`
        // and the `name=value` pairs of everything the form encloses. `HViewJump` has a slot for
        // all four, and `HVN_SUBMIT` used to leave the last two at zero.
        let (url, method, data) = if code == g::HVN_SUBMIT {
            // The form posts what its controls hold *now*, not what the page shipped with: that is
            // the whole point of typing in one. The clone is because the notification below needs
            // `self` mutably while the values live on the control.
            let values = self.widgets.html(this)?.values.clone();
            // `point` is what an `<input type=image>` reports as `name.x`/`name.y`; every other
            // control in the form ignores it.
            crate::web::html::form_submission(doc, elem, &values, point)
        } else {
            (
                doc.attr(elem, "href").unwrap_or("").to_string(),
                String::new(),
                String::new(),
            )
        };
        // `HViewJump.pszURL` is the address the pack will fetch, not the text between the quotes:
        // `HVOPT_SZ_URL` is the document's own URL, and `IWebUtil::MakeUrl` — the same algorithm the
        // pack can call — is what turns `next.html` into the page beside this one.
        let url = self.resolve_html_url(this, &url)?;
        let url_at = self.guest_string(&url)?;
        let method_at = if method.is_empty() {
            0
        } else {
            self.guest_string(&method)?
        };
        let data_at = if data.is_empty() {
            0
        } else {
            self.guest_string(&data)?
        };
        let rec = self.cpu.mem.alloc(24, 4).map_err(HleError::Mem)?;
        self.cpu.mem.write_u32(rec, code)?;
        self.cpu.mem.write_u32(rec + 4, this)?;
        self.cpu.mem.write_u32(rec + 8, elem as u32)?;
        self.cpu.mem.write_u32(rec + 12, url_at)?;
        self.cpu.mem.write_u32(rec + 16, method_at)?;
        self.cpu.mem.write_u32(rec + 20, data_at)?;
        let answer = self.html_notify_record(this, rec);
        // The strings are `const char *` for the duration of the callback only.
        self.cpu.mem.free(url_at);
        if method_at != 0 {
            self.cpu.mem.free(method_at);
        }
        if data_at != 0 {
            self.cpu.mem.free(data_at);
        }
        answer
    }

    /// Resolve a link's `href` or a form's `action` against the document's `HVOPT_SZ_URL`.
    ///
    /// An empty reference or an empty base is answered as-is, and a reference that already names a
    /// scheme comes back unchanged from [`crate::web::make_url`], which is the delivered
    /// `IWebUtil::MakeUrl` body. That matters for a pack that compares the reported URL against a
    /// prefix: a viewer that handed back the literal `next.html` would give it nothing to compare.
    fn resolve_html_url(&mut self, this: u32, reference: &str) -> HResult<String> {
        if reference.is_empty() {
            return Ok(String::new());
        }
        let base = self.widgets.html_base_url(this)?;
        if base.is_empty() {
            return Ok(reference.to_string());
        }
        let (resolved, _) = crate::web::make_url(base.as_bytes(), reference.as_bytes());
        Ok(String::from_utf8_lossy(&resolved).into_owned())
    }

    /// Copy a host string into guest memory, NUL-terminated, for a record that names it by address.
    fn guest_string(&mut self, text: &str) -> HResult<u32> {
        let at = self
            .cpu
            .mem
            .alloc(text.len() as u32 + 1, 1)
            .map_err(HleError::Mem)?;
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        self.cpu.mem.write_bytes(at, &bytes)?;
        Ok(at)
    }

    /// Hand a prepared 24-byte `HViewNotify` to the callback and free it. The header documents these
    /// as sent synchronously, so this calls the guest directly rather than queueing anything.
    fn html_notify_record(&mut self, this: u32, rec: u32) -> HResult<()> {
        let (notify, user) = self.widgets.html_notify_target(this)?;
        let answer = self.call_guest(notify, &[user, rec]);
        self.cpu.mem.free(rec);
        match answer {
            Ok(_) => Ok(()),
            Err(Error::Hle(h)) => Err(h),
            // The host's own stop/cancel reasons travel back as themselves; a notification is the
            // one place they have no `hle::Error` shape, so they become a "wanted" gap rather than
            // being swallowed.
            Err(other) => Err(HleError::Wanted {
                id: 0,
                label: "IHtmlViewer::SetNotifyFn".to_string(),
                want: format!("the notify callback to return: {other:?}"),
                this,
            }),
        }
    }

    /// `IHtmlViewer`'s door for the user: keys and the pen.
    ///
    /// Two things a viewer does with a key: move the focus and act on what has it. Once a page has
    /// a form in it there is a third — edit the control the focus is on — and that one is stateful,
    /// so it lives in [`crate::web::html::Values`] beside the parse.
    pub(crate) fn html_handle_event(
        &mut self,
        ctl: u32,
        evt: u32,
        code: u32,
        dwp: u32,
    ) -> HResult<bool> {
        // The shipped door refuses *everything* while the control is inactive — its first line is
        // `if (Dox_IsActive(...) == 0) return 0`, the same shape
        // `ITimeCtl` and `IDateCtl` have in this host's event door. Keys, characters and pen all
        // wait for `SetActive(TRUE)`.
        if self.widgets.is_active(ctl)? == 0 {
            return Ok(false);
        }
        // The pen names a place, not a step, so it has its own door: `EVT_PEN_DOWN`/`EVT_PEN_UP`
        // carry the position packed into `dwParam`. This SDK has no
        // `EVT_STYLUS_*`; those belong to later BREW versions.
        if matches!(evt, g::EVT_PEN_DOWN | g::EVT_PEN_MOVE | g::EVT_PEN_UP) {
            return self.html_pointer(ctl, evt, dwp);
        }
        // A character is its own event: `EVT_CHAR`'s `wParam` is an `AECHAR`, while `EVT_KEY`'s is a
        // key *code*. Typing a letter therefore only works through this one, and the letters
        // `A`..`Z` have no key codes at all in the header.
        if evt == g::EVT_CHAR {
            return self.html_type(ctl, code);
        }
        // Both key events carry a key code — `EVT_KEY` is the app key, `EVT_KEY_PRESS` its keydown —
        // so a pack that sends either gets the same behaviour.
        if evt != g::EVT_KEY && evt != g::EVT_KEY_PRESS {
            return Ok(false);
        }
        let doc = self.widgets.html_document(ctl)?;
        match code {
            // Backwards and forwards. The content is laid out in one column, so a horizontal arrow
            // walks the same order the vertical one does rather than doing nothing at all.
            g::AVK_UP | g::AVK_DOWN => {
                let back = code == g::AVK_UP;
                // In a textarea the up and down arrows walk the value's lines before they walk the
                // page, exactly as left and right walk the caret before they walk the focus. A
                // value with no line breaks answers `false`, so a one-line field keeps the old
                // behaviour without being asked about.
                if let Some((elem, kind)) = self.widgets.html_focus(ctl, &doc)? {
                    if kind == g::HVFOCUS_EDIT && self.widgets.html_inline_edit(ctl)? {
                        let base = self.widgets.html_control_text(ctl, &doc, elem)?;
                        if self.widgets.html_move_caret_line(ctl, elem, &base, back)? {
                            return Ok(true);
                        }
                    }
                }
                let Some((elem, kind)) = self.widgets.html_move_focus(ctl, &doc, back)? else {
                    return Ok(false);
                };
                // The page follows the keyboard: `HVOPT_*` names no scroll position and the viewer
                // documents no scrolling key, so this is the one motion there is. A page that fits
                // does nothing here — the adjustment is only ever what is hidden.
                self.widgets
                    .html_reveal_focus(ctl, &doc, self.display.faces())?;
                self.html_focus_landed(ctl, &doc, elem, kind)?;
                Ok(true)
            }
            g::AVK_LEFT | g::AVK_RIGHT => {
                let right = code == g::AVK_RIGHT;
                // Inside a field the arrow walks the caret, and inside a list it walks the choice.
                // Only at the edge of one — or outside both — does it move the focus, which is how
                // a user gets out of a field they have typed to the end of.
                if let Some((elem, kind)) = self.widgets.html_focus(ctl, &doc)? {
                    if kind == g::HVFOCUS_EDIT && self.widgets.html_inline_edit(ctl)? {
                        let base = self.widgets.html_control_text(ctl, &doc, elem)?;
                        if self.widgets.html_move_caret(ctl, elem, &base, right)? {
                            return Ok(true);
                        }
                    } else if kind == g::HVFOCUS_LIST {
                        if self
                            .widgets
                            .html_cycle(ctl, &doc, elem, if right { 1 } else { -1 })?
                        {
                            let state = self.widgets.html_choice(ctl, &doc, elem)? as i32;
                            self.html_notify_focus(ctl, elem, kind, state)?;
                            return Ok(true);
                        }
                    }
                }
                let Some((elem, kind)) = self.widgets.html_move_focus(ctl, &doc, !right)? else {
                    return Ok(false);
                };
                self.widgets
                    .html_reveal_focus(ctl, &doc, self.display.faces())?;
                self.html_focus_landed(ctl, &doc, elem, kind)?;
                Ok(true)
            }
            // The key a handset puts on the side of the case: delete the character before the caret.
            // It only answers while an editor is focused, so a page without one keeps the key.
            g::AVK_CLR => {
                if !self.widgets.html_inline_edit(ctl)? {
                    return Ok(false);
                }
                let Some((elem, kind)) = self.widgets.html_focus(ctl, &doc)? else {
                    return Ok(false);
                };
                if kind != g::HVFOCUS_EDIT {
                    return Ok(false);
                }
                let base = self.widgets.html_control_text(ctl, &doc, elem)?;
                if !self.widgets.html_backspace(ctl, elem, &base)? {
                    return Ok(false);
                }
                self.html_notify_invalidate(ctl)?;
                Ok(true)
            }
            g::AVK_SELECT => {
                let Some((elem, kind)) = self.widgets.html_focus(ctl, &doc)? else {
                    return Ok(false);
                };
                // A key press has no place on the page to report; an image button activated this
                // way posts the corner of its picture, which is what the form algorithm says.
                self.html_activate(ctl, &doc, elem, kind, (0, 0))
            }
            // The digits and the two keys beside them are the only characters with key codes in
            // this SDK, so a pack that sends keys and no `EVT_CHAR` can still fill a
            // number in.
            _ => match key_char(code) {
                Some(ch) => self.html_insert_at_focus(ctl, &doc, ch),
                None => Ok(false),
            },
        }
    }

    /// Act on whatever holds the focus: follow a link, submit or reset a form, tick a box, walk a
    /// list. Shared by `AVK_SELECT` and the pen's release, because they are the same gesture.
    fn html_activate(
        &mut self,
        ctl: u32,
        doc: &std::sync::Arc<crate::web::html::Document>,
        elem: usize,
        kind: u32,
        point: (i16, i16),
    ) -> HResult<bool> {
        match kind {
            g::HVFOCUS_LINK => {
                self.html_notify_jump(ctl, doc, elem, g::HVN_JUMP, point)?;
                Ok(true)
            }
            g::HVFOCUS_SUBMIT => {
                self.html_notify_jump(ctl, doc, elem, g::HVN_SUBMIT, point)?;
                Ok(true)
            }
            // A reset is not a report: nothing leaves the control, the page just goes back to what
            // it started as. The pack hears about it the same way it hears about a tick — through
            // the focus notification below.
            g::HVFOCUS_RESET => {
                self.widgets.html_reset_form(ctl, doc, elem)?;
                self.html_notify_focus(ctl, elem, kind, 1)?;
                self.html_notify_invalidate(ctl)?;
                Ok(true)
            }
            g::HVFOCUS_CHECKBOX | g::HVFOCUS_RADIO => {
                let radio = kind == g::HVFOCUS_RADIO;
                self.widgets.html_toggle(ctl, doc, elem, radio)?;
                // `HViewFocus.nState` is the state "dependent upon type", and for a box that is
                // checked/unchecked — the one thing the pack needs to know about a tick.
                let state = i32::from(self.widgets.html_checked(ctl, doc, elem)?);
                self.html_notify_focus(ctl, elem, kind, state)?;
                self.html_notify_invalidate(ctl)?;
                Ok(true)
            }
            g::HVFOCUS_LIST => {
                if !self.widgets.html_cycle(ctl, doc, elem, 1)? {
                    return Ok(false);
                }
                let state = self.widgets.html_choice(ctl, doc, elem)? as i32;
                self.html_notify_focus(ctl, elem, kind, state)?;
                self.html_notify_invalidate(ctl)?;
                Ok(true)
            }
            // An editor has nothing to "activate": the caret is already in it, and a handset picks
            // its characters with the keypad.
            _ => Ok(false),
        }
    }

    /// The focus has arrived somewhere: tell the pack, and put a caret in a field that takes typing.
    fn html_focus_landed(
        &mut self,
        ctl: u32,
        doc: &std::sync::Arc<crate::web::html::Document>,
        elem: usize,
        kind: u32,
    ) -> HResult<()> {
        if kind == g::HVFOCUS_EDIT && self.widgets.html_inline_edit(ctl)? {
            // A field that has never been typed into has no caret to draw; starting an edit record
            // at the end of its text is what makes one appear, and it is where the next character
            // will go.
            let base = self.widgets.html_control_text(ctl, doc, elem)?;
            self.widgets.html_begin_edit(ctl, elem, &base)?;
        }
        self.html_notify_focus(ctl, elem, kind, 1)
    }

    /// `EVT_CHAR`: one character arrived. It only lands in an editor that has the focus and only
    /// when `HVP_INLINE_EDIT` asked for in-place editing — that property's whole documented meaning
    /// is "text input fields are editable in place in the document when they get focus".
    fn html_type(&mut self, ctl: u32, code: u32) -> HResult<bool> {
        let Some(ch) = char::from_u32(code & 0xffff) else {
            return Ok(false);
        };
        // A carriage return and a line feed are the same character to a textarea; `html_insert`
        // decides whether the field takes it at all.
        let ch = if ch == '\r' { '\n' } else { ch };
        if ch != '\n' && ch.is_control() {
            return Ok(false);
        }
        let doc = self.widgets.html_document(ctl)?;
        self.html_insert_at_focus(ctl, &doc, ch)
    }

    /// Type one character into whatever editor holds the focus.
    fn html_insert_at_focus(
        &mut self,
        ctl: u32,
        doc: &std::sync::Arc<crate::web::html::Document>,
        ch: char,
    ) -> HResult<bool> {
        if !self.widgets.html_inline_edit(ctl)? {
            return Ok(false);
        }
        let Some((elem, kind)) = self.widgets.html_focus(ctl, doc)? else {
            return Ok(false);
        };
        if kind != g::HVFOCUS_EDIT {
            return Ok(false);
        }
        // Only a textarea has lines to break: in a one-line field a return is not a character the
        // control can hold, and swallowing the key would swallow the page's own confirm.
        if ch == '\n' && doc.field_kind(elem) != Some(crate::web::html::FieldKind::Textarea) {
            return Ok(false);
        }
        let base = self.widgets.html_control_text(ctl, doc, elem)?;
        self.widgets.html_insert(ctl, elem, &base, ch)?;
        // What the page shows has changed without the pack having asked for a frame, and a pack that
        // registered `HVP_INVALIDATION` is exactly the one that wants to know.
        self.html_notify_invalidate(ctl)?;
        Ok(true)
    }

    /// Put the page back inside its own rectangle after the rectangle changed.
    ///
    /// `SetRect` is the one message that can make a legal scroll offset illegal: an offset that put
    /// the last row on the bottom edge starts the page past its own end once the control grows. So
    /// the offset is clamped here, and the focused row is brought back into view — a control that
    /// grew is exactly the case where the thing the user was looking at is no longer where the
    /// keyboard can find it.
    ///
    /// This belongs to the message rather than to the frame: a redraw that moved the page would be
    /// adjusting its own input, and the clamp alone (which `Redraw` does) cannot reveal anything.
    pub(crate) fn html_fit(&mut self, ctl: u32) -> HResult<()> {
        let doc = self.widgets.html_document(ctl)?;
        if doc.runs.is_empty() {
            return Ok(());
        }
        let limit = self.html_scroll_limit(ctl)?;
        {
            let html = self.widgets.html(ctl)?;
            html.scroll = html.scroll.min(limit);
        }
        if limit == 0 {
            // The page fits whole, so no row can be hidden and there is nothing to reveal. Skipping
            // it is also what keeps `SetRect` cheap for the pages that are about to be laid out in
            // the new rectangle anyway.
            return Ok(());
        }
        self.widgets
            .html_reveal_focus(ctl, &doc, self.display.faces())?;
        Ok(())
    }

    /// The furthest the page can be scrolled at the size it has now: its height less the rectangle
    /// it is shown in, or 0 when it fits.
    ///
    /// The height is a full layout pass, which [`Widgets::html_content_height`] caches against the
    /// parse, the width, the paragraph spacing and how many pictures have settled — so a drag that
    /// sends thirty moves a second measures the page once, not thirty times.
    fn html_scroll_limit(&mut self, ctl: u32) -> HResult<i16> {
        let rect = self.widgets.peek(ctl)?.rect;
        if rect.is_empty() {
            return Ok(0);
        }
        let doc = self.widgets.html_document(ctl)?;
        if doc.runs.is_empty() {
            return Ok(0);
        }
        let props = self.widgets.properties(ctl)?;
        let options = {
            let html = self.widgets.html(ctl)?;
            let mut options = crate::web::html::Options::from_table(&html.options);
            options.scrollbar = props & g::HVP_SCROLLBAR != 0;
            options
        };
        let pictures = self.widgets.decode_images(ctl, &doc)?;
        let content = self.widgets.html_content_height(
            ctl,
            &doc,
            &pictures,
            &options,
            rect,
            &self.display,
        )?;
        Ok(content.saturating_sub(rect.dy).max(0))
    }

    /// Scroll the page to `target`, clamped to what it has. Answers whether the offset moved.
    fn html_scroll_to(&mut self, ctl: u32, target: i16) -> HResult<bool> {
        let limit = self.html_scroll_limit(ctl)?;
        let next = target.clamp(0, limit);
        let html = self.widgets.html(ctl)?;
        if html.scroll == next {
            return Ok(false);
        }
        html.scroll = next;
        Ok(true)
    }

    /// The pen moved while it was down: drag the page with it.
    ///
    /// A drag scrolls from the offset the press started at rather than from the distance since the
    /// last move — events are not guaranteed, and a page that added up the ones it happened to
    /// receive would drift away from the hand holding the pen. Downwards shows what is above the
    /// window, which is the direction the content travels.
    fn html_drag(&mut self, ctl: u32, y: i16) -> HResult<bool> {
        let Some(mut drag) = self.widgets.html(ctl)?.drag else {
            return Ok(false);
        };
        let dy = y.saturating_sub(drag.y0);
        if !drag.moved && dy.abs() < crate::web::html::DRAG_SLOP {
            return Ok(false);
        }
        // The gesture is a drag from here on, even if it is against an edge and the page does not
        // move: releasing it must not also activate what it started on.
        drag.moved = true;
        self.widgets.html(ctl)?.drag = Some(drag);
        if !self.html_scroll_to(ctl, drag.scroll.saturating_sub(dy))? {
            return Ok(true);
        }
        // The page moved without the pack having asked for a frame, which is what `HVN_INVALIDATE`
        // is for. It is still the drag's event either way: the pen has travelled, so the release
        // must not be taken for a press.
        self.html_notify_invalidate(ctl)?;
        Ok(true)
    }

    /// `EVT_PEN_DOWN` / `EVT_PEN_MOVE` / `EVT_PEN_UP`: the user touched the page at a pixel.
    ///
    /// The hit test is the layout's own output, so what the finger finds is exactly what the frame
    /// drew — the scroll offset included. Pressing focuses what is under the point; releasing acts
    /// on it, which is the same pair the keyboard performs with the arrows and `AVK_SELECT`.
    ///
    /// A pen that travels while it is down is dragging the page instead: a press is only a click
    /// once it ends where it began, and this SDK's three pen events are what says whether it did.
    fn html_pointer(&mut self, ctl: u32, evt: u32, dwp: u32) -> HResult<bool> {
        // Signed x in the upper sixteen bits, signed y in the lower.
        let x = ((dwp >> 16) & 0xffff) as u16 as i16;
        let y = (dwp & 0xffff) as u16 as i16;
        // A press is only the start of a gesture: it is a click if the pen comes up where it went
        // down, and a drag the moment it travels far enough. Both are decided from the place and
        // the offset the press found, so that is recorded before anything is done with the pixel —
        // including when it lands on nothing, because dragging the prose scrolls the page exactly
        // as dragging a link does.
        match evt {
            g::EVT_PEN_MOVE => return self.html_drag(ctl, y),
            g::EVT_PEN_DOWN => {
                let scroll = self.widgets.html(ctl)?.scroll;
                self.widgets.html(ctl)?.drag = Some(crate::web::html::Drag {
                    x0: x,
                    y0: y,
                    scroll,
                    moved: false,
                });
            }
            _ => {
                // A release that ends a drag has done its work already: the page moved under the
                // pen. It must not also press what the pen went down on, or dragging a page by one
                // of its links would follow the link.
                let drag = self.widgets.html(ctl)?.drag.take();
                if drag.map_or(false, |drag| drag.moved) {
                    return Ok(true);
                }
            }
        }
        let doc = self.widgets.html_document(ctl)?;
        if doc.runs.is_empty() {
            return Ok(false);
        }
        let rect = self.widgets.peek(ctl)?.rect;
        if rect.is_empty() {
            return Ok(false);
        }
        let props = self.widgets.properties(ctl)?;
        let (options, scroll, values) = {
            let html = self.widgets.html(ctl)?;
            let mut options = crate::web::html::Options::from_table(&html.options);
            options.scrollbar = props & g::HVP_SCROLLBAR != 0;
            (options, html.scroll, html.values.clone())
        };
        let pictures = self.widgets.decode_images(ctl, &doc)?;
        let rows = crate::web::html::layout(
            &doc,
            rect,
            &options,
            &pictures,
            Some(&values),
            scroll,
            self.display.faces(),
        );
        let hit = rows.iter().find(|line| {
            x >= line.x
                && x < line.x.saturating_add(line.w.max(1))
                && y >= line.y
                && y < line.y.saturating_add(line.row.max(1))
        });
        let Some(line) = hit else {
            return Ok(false);
        };
        let Some(elem) = line.elem else {
            return Ok(false);
        };
        let Some(kind) = self.widgets.html_focus_on(ctl, &doc, elem)? else {
            // The point landed on something a user cannot land on — prose, a rule, a picture that
            // is not a link. Nothing moves, and the pack is not told anything moved.
            return Ok(false);
        };
        if evt == g::EVT_PEN_DOWN {
            self.html_focus_landed(ctl, &doc, elem, kind)?;
            return Ok(true);
        }
        // The place inside the span, not on the screen: `<input type=image>` posts where its
        // picture was pressed, and that is counted from the picture's own corner.
        let point = (x.saturating_sub(line.x), y.saturating_sub(line.top));
        self.html_activate(ctl, &doc, elem, kind, point)
    }
}
/// The character a key code stands for, when it stands for one.
///
/// Only the digits and the two keys after them have key codes in this SDK's the header; the
/// letters arrive as `EVT_CHAR` because a phone keypad has no letter keys. That asymmetry is the
/// interface's, not this host's.
fn key_char(code: u32) -> Option<char> {
    match code {
        g::AVK_0..=g::AVK_9 => char::from_u32(u32::from(b'0') + (code - g::AVK_0)),
        g::AVK_STAR => Some('*'),
        g::AVK_POUND => Some('#'),
        _ => None,
    }
}
