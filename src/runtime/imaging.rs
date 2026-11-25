//! `IGraphics`, canvas faces, the image decoders and the QEGL family.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

fn qegl_face_enabled(caps: &crate::device::GlCaps, face: crate::gl::qegl::Face) -> bool {
    use crate::gl::qegl::Face;
    if !caps.qegl {
        return false;
    }
    match face {
        Face::Gl => caps.gles10,
        Face::Gl11 => caps.gles11,
        Face::Gl10Ext => caps.gles10_ext,
        Face::Gl11Ext => caps.gles11_ext,
        Face::Gl11ExtPak => caps.gles11_ext_pak,
        Face::ImageOn => caps.imageon_ext,
        Face::ImageOnV1 => caps.imageon_ext_v1,
        Face::Egl | Face::Egl11 | Face::ColorBuffer | Face::ColorBufferInfo => true,
    }
}

impl Runtime {
    /// `QueryInterface` for any face of a QEGL family.
    ///
    /// The arm that reaches here is chosen by the interface name — the vtable the pack read the
    /// slot from — but the answer is a property of the *object*: `IGLES11ExtPak` answering
    /// `AEEIID_GLES10` is the same fact as `IGLES11` answering it. So the table is looked up by
    /// `iface`, and a navigation only happens when `this` really is that face.
    ///
    /// `this` need not be a member at all. A pack that calls `IGLES10::QueryInterface` on some
    /// other pointer lands here, and the arms this replaced answered it with the pointer it was
    /// given (`unwrap_or(this)`) — [`crate::gl::qegl::Miss`] keeps that shape, because quietly
    /// answering `AEE_CLASSNOTSUPPORT` instead would tell the pack its own object is not a QEGL.
    pub(crate) fn qegl_qi(&mut self, iface: &str, this: u32) -> Result<u32, Error> {
        let [iid, ppv] = self.args()?;
        if ppv == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        // The match arm enumerates exactly the family, so this is never `None`.
        let Some(face) = crate::gl::qegl::Face::of_iface(iface) else {
            return Ok(g::AEE_ECLASSNOTSUPPORT);
        };
        if !qegl_face_enabled(&self.device.gl, face) {
            self.cpu.mem.write_u32(ppv, 0)?;
            return Ok(g::AEE_ECLASSNOTSUPPORT);
        }
        let member = match self.hle.payload(this) {
            Some(crate::hle::Payload::Qegl(member)) if member.face == face => Some(*member),
            _ => None,
        };
        if let Some(member) = member {
            // Every face is registered at creation, so an edge that exists always resolves —
            // unless this profile's firmware never exposed the destination face.
            if let Some((to, _)) = crate::gl::qegl::answer(face, iid) {
                if !qegl_face_enabled(&self.device.gl, to) {
                    self.cpu.mem.write_u32(ppv, 0)?;
                    return Ok(g::AEE_ECLASSNOTSUPPORT);
                }
                return Ok(self.return_object(member.faces.addr(to), ppv)?);
            }
        } else if crate::gl::qegl::answer(face, iid).map(|(_, miss)| miss)
            == Some(crate::gl::qegl::Miss::AnswerSelf)
        {
            return Ok(self.return_object(this, ppv)?);
        }
        Ok(self.answer_qi(this, iid, ppv, face.own_iids())?)
    }

    /// `IShell::DetectType(cpBuf, pdwSize, cpszName, pcpszMIME)`: what format are these bytes?
    ///
    /// **The answer comes out of the code that serves the format.** That is the whole design: a
    /// picture is named by [`resource::sniff`], which is the same code that decodes it, and a score
    /// by [`audio::sniff`], which runs the parse the front end itself will run. A caller of this
    /// slot takes the MIME string and goes and loads the payload with it — through
    /// `IShell::GetHandler` and a viewer class, or straight back into [`resource::image`] — so a name
    /// produced by a cheaper test than the decoder is a claim the host then contradicts:
    /// `image/png` here, "not a picture this host decodes" there, and the pack is holding a value it
    /// has no way to disbelieve. Two answers to the same question is the shape this host refuses.
    /// It follows that this host names **four** formats and no more — PNG, BMP, JPEG and GIF, because
    /// [`resource::image`] has an arm for each — plus `audio/mid`, because the synthesizer renders it.
    /// GIF is the one that is *not* in the delivered set: no extracted member decodes it
    /// ([`resource::gif`] says what that means, and the control that the search works is
    /// the delivered body), so a build of those members would refuse `image/gif` here while this
    /// host names and then honours it. `audio/mp3` is refused in the other direction: that class is
    /// served (see [`HOST_SERVICES`]) because refusing it is a null a pack walks into, but nothing in
    /// this crate decodes the stream. The viewer-class door is a third answer again: only PNG has a
    /// picture row in [`BUILTIN_HANDLERS`], so `IShell::GetHandler` + `CreateInstance` stays refused for
    /// BMP, JPEG and GIF alike — as a handset with no such viewer module refuses it — while
    /// `IShell::LoadResObject` and `ISHELL_LoadResImage`, the doors a pack actually brings its art
    /// through, build the image.
    ///
    /// the header gives the rest of the contract, and each clause is taken literally below:
    ///
    /// * `pdwSize` is in/out — the caller's byte count on input, *how many more bytes* the answer
    ///   needs on output. Detection here is decided from the bytes handed over, so a success writes
    ///   zero. That is the same one-door shape [`device::put_sized_item`] has: read the word, answer
    ///   it, never leave a caller's size word holding its own capacity after a call that succeeded.
    ///   What follows from taking the contract literally is worth stating, because it is the one way
    ///   this slot can bite a pack that seems to be calling it correctly: a caller that sniffs twice
    ///   through **one** length word must put its length back between calls, or the second call
    ///   reads our answered zero as the size of its own buffer and is refused. The alternative —
    ///   leaving the word alone on success — would be the quieter bug and the worse one, because the
    ///   word then reads "N more bytes needed" from a call that just answered the question.
    /// * `cpBuf` takes precedence over `cpszName`, and the name is tried when the bytes said nothing.
    /// * `pcpszMIME` is an *out* pointer, and `AEEMedia`'s own worked example calls this slot with
    ///   every pointer null to ask one bare question, so a null here is a legal probe and must not
    ///   fault. Nothing is written through it on a refusal either: `ENOTYPE` leaves the caller's word
    ///   exactly as it was handed over, which is how this file answers "I could not" everywhere else
    ///   (`GetPrefs`, `QueryClass`) and the opposite of dangling it in front of a caller that reads
    ///   the pointer without reading the return.
    /// * The return set is SUCCESS / `ENOTYPE` / `EBADPARM` / `ENEEDMORE` / `EUNSUPPORTED`.
    ///   `ENEEDMORE` is answered for the header's **probe**: a null buffer with a null name and a
    ///   size word asks "how many bytes do you want?", and the delivered
    ///   `AEEMediaUtil_FindClass` opens with exactly that call (`DetectType(ps, 0, &size, 0, 0)
    ///   != 35 || !size ⇒ EFAILED`). The window is this host's own (see below), and it is a promise
    ///   this host keeps: with that many bytes the answer is `SUCCESS` or `ENOTYPE`, never a third
    ///   "ask again". A short *given* buffer is still answered from what is there — the host does
    ///   not ask a caller that already offered bytes for more.
    pub(crate) fn detect_type(&mut self) -> Result<u32, Error> {
        let [buf, psize, name, pmime] = self.args()?;
        // Every format either sniffer names is decided inside its first 32 bytes (the widest headers
        // are RIFF's 12 and ID3's 10), so 32 is a number this host can stand behind rather than a
        // page guessed at. The media utility measures against it and then hands exactly this much.
        const WINDOW: u32 = TYPE_DETECT_WINDOW;
        if buf == 0 && name == 0 {
            // Nowhere to report the number, so the probe cannot be answered.
            if psize == 0 {
                return Ok(g::AEE_EBADPARM);
            }
            self.cpu.mem.write_u32(psize, WINDOW)?;
            return Ok(g::AEE_ENEEDMORE);
        }
        // A buffer with no length *pointer* is a question this host cannot ask its own decoder: the
        // pointer is the only thing carrying the bytes, and `pdwSize` is what says where they stop.
        // Guessing a page's worth is how a sniffer ends up naming whatever sat beside the payload,
        // so this is the header's `EBADPARM` — wrong input data — and nothing is written. A zero
        // *inside* a valid word is the other case, and the honest one: the caller says it has no
        // bytes, so it gets the answer a length of nothing deserves, which is `ENOTYPE` below.
        if buf != 0 && psize == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        // The name is read up front rather than where it is used, because both halves below want it:
        // the suffix answer if the bytes said nothing, and the line that says which of the two was
        // refused.
        let named = if name == 0 {
            None
        } else {
            Some(self.cstring(name)?)
        };
        let mut found = None;
        let mut seen = 0usize;
        if buf != 0 {
            // Three bounds, each one the caller's or the host's own rather than a number invented
            // here: the length the caller declared, at most `resource::MAX_IMAGE_BYTES` of it — the
            // ceiling `resource::file_size` already puts on a bitmap that asks to be sized — and
            // never past the last mapped page, which is how `SetupNativeImage` reads a caller's
            // pointer for the same reason. The name says *image* and it now bounds scores too; a
            // MIDI over 8 MiB is a million-minute composition, so what that costs is a refusal, not
            // a wrong answer. What the two paths genuinely do not share is the window: this one is
            // the caller's declared length, while a play gets whatever the run resolved into
            // [`Runtime::audio`], so a sniff and a play of one clip can be handed different amounts
            // of it and can disagree. That is the header's `ENEEDMORE` case, which
            // this host refuses to promise (see the refusal below) rather than answer with a number
            // it cannot keep.
            let declared = self.cpu.mem.read_u32(psize)? as usize;
            let want = declared.min(resource::MAX_IMAGE_BYTES) as u32;
            let room = self.cpu.mem.mapped_run(buf, want) as usize;
            let bytes = self.cpu.mem.read_bytes(buf, room)?;
            seen = bytes.len();
            // Picture first, then score, and each authority asked to *read* the bytes rather than
            // match a prefix: `resource::sniff` only names what `resource::image` will decode and
            // `audio::sniff` only names what the front end will synthesise, so neither can hand
            // this caller a string its own next load fails on.
            found = resource::sniff(&bytes).or_else(|| audio::sniff(&bytes));
        }
        if found.is_none() {
            // Reached when there was no buffer, and when there was one whose bytes said nothing: the
            // header's precedence is the buffer *first*, then the name.
            let asked = named.as_deref().unwrap_or("");
            found = resource::sniff_name(asked).or_else(|| audio::sniff_name(asked));
        }
        let Some(mime) = found else {
            // Said out loud, because a loader that asked and was refused has to be able to tell
            // "the host has no decoder for this" apart from "the host never looked". `Display::note`
            // deduplicates and bounds its own list, so a pack that sniffs a whole container leaves
            // one line per distinct ask rather than one per call. The caller's own words are left
            // untouched either way, which is what makes this a refusal and not an answer.
            let offered = match (buf, named.as_deref()) {
                (0, Some(text)) => format!("the name {text:?}"),
                (_, Some(text)) => format!("{seen} byte(s) named {text:?}"),
                _ => format!("{seen} byte(s) of buffer"),
            };
            self.display.note(format!(
                "IShell::DetectType: nothing this host decodes for {offered}"
            ));
            return Ok(g::AEE_ENOTYPE);
        };
        if pmime != 0 {
            let at = self.guest_mime(mime)?;
            self.cpu.mem.write_u32(pmime, at)?;
        }
        // `pdwSize` is [in/out] with the out half promised **only on ENEEDMORE** ("*pdwSize contains
        // the required number of additional bytes"); on success the header says nothing, its own
        // sample never reads the word again, and the one caller in the reference pk source that does —
        // the ringer create, whose `&m_nBytes` must survive the call for its first write to mean
        // anything — depends on it holding the value it put in. So a successful detect leaves the
        // caller's word alone; writing 0 "additional" here used to clear exactly that.
        Ok(g::AEE_SUCCESS)
    }

    /// One `ICanvas` / `IDisplayCanvas` call, dispatched by slot.
    ///
    /// The two interfaces share one table ([`abi::CANVAS_IFACE`]), so a slot past 6 can only have
    /// come from a caller holding the `IDisplayCanvas` face — the shape `IVectorModel` already uses.
    pub(crate) fn canvas_call(&mut self, id: u32) -> Result<u32, Error> {
        let this = self.cpu.r[0];
        match abi::slot_of_method_id(id) {
            0 => self
                .hle
                .change_refs(&mut self.cpu.mem, this, 1)
                .map_err(Error::Hle),
            1 => {
                let refs = self.hle.change_refs(&mut self.cpu.mem, this, -1)?;
                if refs == 0 {
                    self.hle.forget(this);
                }
                Ok(refs)
            }
            2 => {
                let [iid, ppi] = self.args()?;
                // `IDisplayCanvas` is `ICanvas` plus a tail, so both ids name this same object and
                // both hand back the same base — a caller that asked for the shorter face simply
                // never reaches slot 7.
                if iid == canvas::IID_ICANVAS
                    || iid == canvas::IID_IDISPLAYCANVAS
                    || iid == g::AEEIID_IQI
                {
                    self.cpu.mem.write_u32(ppi, this)?;
                    Ok(g::AEE_SUCCESS)
                } else {
                    self.cpu.mem.write_u32(ppi, 0)?;
                    Ok(g::AEE_ECLASSNOTSUPPORT)
                }
            }
            3 => {
                // `GetBitmap` is an accessor, not a factory: it hands out the pointer the canvas
                // already has and moves no reference — the header documents it with no
                // reference-count language, and `SetBitmap` with none either.
                let [ppo] = self.args()?;
                let bitmap = self.cpu.mem.read_u32(this + canvas::BITMAP_OFF)?;
                self.cpu.mem.write_u32(ppo, bitmap)?;
                Ok(if bitmap == 0 {
                    g::AEE_EFAILED
                } else {
                    g::AEE_SUCCESS
                })
            }
            4 => {
                let [pib] = self.args()?;
                if pib == 0 {
                    // A set to "nothing to draw on": the canvas keeps the call and drops the
                    // surface, and from here `GetBitmap` says so.
                    self.cpu.mem.write_u32(this + canvas::BITMAP_OFF, 0)?;
                    self.write_canvas_clip(this, canvas::Clip::default())?;
                    return Ok(g::AEE_EFAILED);
                }
                self.cpu.mem.write_u32(this + canvas::BITMAP_OFF, pib)?;
                let clip = self.canvas_bitmap_clip(pib);
                self.write_canvas_clip(this, clip)?;
                Ok(g::AEE_SUCCESS)
            }
            5 => {
                let [prc] = self.args()?;
                let clip = self.read_canvas_clip(this)?;
                if prc != 0 {
                    self.write_rect(prc, clip)?;
                }
                Ok(self.canvas_has_bitmap(this)?)
            }
            6 => {
                let [prc] = self.args()?;
                if prc == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                let clip = self.read_rect(prc)?;
                self.write_canvas_clip(this, clip)?;
                Ok(self.canvas_has_bitmap(this)?)
            }
            7 => {
                let [pp] = self.args()?;
                let display = self.cpu.mem.read_u32(this + canvas::DISPLAY_OFF)?;
                self.cpu.mem.write_u32(pp, display)?;
                Ok(if display == 0 {
                    g::AEE_EFAILED
                } else {
                    g::AEE_SUCCESS
                })
            }
            8 => {
                let [pdisp] = self.args()?;
                if pdisp == 0 {
                    self.cpu.mem.write_u32(this + canvas::DISPLAY_OFF, 0)?;
                    return Ok(g::AEE_EFAILED);
                }
                self.cpu.mem.write_u32(this + canvas::DISPLAY_OFF, pdisp)?;
                // the header never calls `SetBitmap`: the display *is* where the canvas
                // paints. So a display brings its device bitmap along, unless the caller already
                // named a surface of its own — `SetBitmap` is the more specific instruction and
                // this must not quietly undo it.
                if self.cpu.mem.read_u32(this + canvas::BITMAP_OFF)? == 0 {
                    let bitmap = self.display.device_bitmap();
                    self.cpu.mem.write_u32(this + canvas::BITMAP_OFF, bitmap)?;
                    let clip = self.canvas_bitmap_clip(bitmap);
                    self.write_canvas_clip(this, clip)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(HleError::Unimplemented {
                id,
                label: abi::label(id).to_string(),
                this,
            })),
        }
    }

    /// `AEE_SUCCESS` when the canvas has a surface to draw on, `AEE_EFAILED` when it does not —
    /// the two codes the header lists for `GetClipRect` and `SetClipRect`.
    pub(crate) fn canvas_has_bitmap(&self, canvas: u32) -> Result<u32, Error> {
        Ok(
            if self.cpu.mem.read_u32(canvas + canvas::BITMAP_OFF)? == 0 {
                g::AEE_EFAILED
            } else {
                g::AEE_SUCCESS
            },
        )
    }

    /// The clip a bitmap arrives with: all of it.
    pub(crate) fn canvas_bitmap_clip(&self, bitmap: u32) -> canvas::Clip {
        match self.display.bitmap(bitmap) {
            Some(b) => canvas::Clip::whole(b.width as u32, b.height as u32),
            // A surface this host does not own (a device bitmap it has not seen, or a null): clip
            // to nothing rather than invent an extent. A widget that then paints nothing is
            // visible in a dump; one that painted over a guessed area would not be.
            None => canvas::Clip::default(),
        }
    }

    /// The `IGraphics` slots. Per-object state lives in [`Runtime::graphics`], the geometry in
    /// `graphics.rs`, and every shape reaches the screen through the same viewport-clipped pixel, so
    /// no shape can forget to clip.
    ///
    /// Every own method the header declares has an arm here now. What a caller cannot get is said
    /// in the arm that answers it — a clip to a triangle, say, is stored as nothing and noted, because
    /// the alternative is a shape drawn where the app believed it had closed the region.
    ///
    /// The state is read into a copy and written back at the end. Each arm can then hold a mutable
    /// borrow of the display or the guest memory while it draws, which a `&mut State` inside the map
    /// would not allow.
    pub(crate) fn igranphics_dispatch(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> Result<u32, Error> {
        let Some(mut st) = self.graphics.get(this) else {
            return Err(Error::Hle(HleError::NotAnObject { addr: this }));
        };
        // What a setter answers is its own contract, not a status code: the colour setters return the
        // updated RGBVAL, `SetFillMode` the updated boolean, `SetPointSize` the updated size, and the
        // `boolean`-returning ones answer TRUE/FALSE. `void` slots answer nothing, so 0 stands for
        // them. Each arm reads only as many arguments as it has: the ABI gives a callee four
        // argument registers in all, `this` among them, so a getter that asked for four would reach
        // past the registers into a stack the caller never wrote.
        let mut out = 0u32;
        match name {
            "SetColor" => {
                // The body packs three bytes and nothing else: `(r << 8) + (g << 16) + (b << 24)`, so the
                // fifth argument the header names (`alpha`) is read into a register and dropped, and
                // `GetColor` answers it from the packed word's low byte — always `0`.
                let [r, g, b, _alpha] = self.args()?;
                st.fg = Rgba::new(r as u8, g as u8, b as u8, 0);
                out = st.fg.rgbval();
            }
            "GetColor" => {
                let [r, g, b, alpha] = self.args()?;
                Graphics::put_rgba(&mut self.cpu.mem, [r, g, b, alpha], st.fg)?;
            }
            "SetBackground" => {
                let [r, g, b] = self.args()?;
                st.bg = Rgba::new(r as u8, g as u8, b as u8, 255);
                out = st.bg.rgbval();
            }
            "GetBackground" => {
                let [r, g, b] = self.args()?;
                Graphics::put_rgb(&mut self.cpu.mem, [r, g, b], st.bg)?;
            }
            "SetFillColor" => {
                // Same packing, same dropped `alpha` as `SetColor`: the body writes three bytes at `+44`.
                let [r, g, b, _alpha] = self.args()?;
                st.fill = Rgba::new(r as u8, g as u8, b as u8, 0);
                out = st.fill.rgbval();
            }
            "GetFillColor" => {
                let [r, g, b, alpha] = self.args()?;
                Graphics::put_rgba(&mut self.cpu.mem, [r, g, b, alpha], st.fill)?;
            }
            "SetFillMode" => {
                let [mode] = self.args()?;
                // "If an invalid value (other than TRUE or FALSE) is passed as input parameter, the
                // fill mode is set to FALSE by default" — so `mode != 0` was not a harmless
                // generalisation: it turned a caller's garbage into "filled", which is the one
                // difference between an outlined rectangle and a solid block over the art below it.
                st.filled = mode == g::TRUE;
                out = u32::from(st.filled);
            }
            "GetFillMode" => out = u32::from(st.filled),
            "SetPointSize" => {
                let [size] = self.args()?;
                st.point = size as u8;
                out = size;
            }
            "GetPointSize" => out = u32::from(st.point),
            "SetStrokeStyle" => {
                let [style] = self.args()?;
                // "Returns the previous stroke style," and the body says so literally: `result` is read
                // from `byte[+100]` before the store. The store itself is a raw byte, with no
                // normalisation to the enum.
                out = (st.stroke as u8) as i8 as i32 as u32;
                st.stroke = style & 0xFF;
            }
            // `*(char *)(a1 + 100)` — a signed read, so a style byte above `0x7F` comes back negative.
            "GetStrokeStyle" => out = (st.stroke as u8) as i8 as i32 as u32,
            "SetAlgorithmHint" => {
                let [hint] = self.args()?;
                st.hint = u32::from(hint != 0);
            }
            "GetAlgorithmHint" => out = st.hint,
            // The header's own words for `SetPaintMode` are "the updated enum value", which is what a
            // caller that saves and restores a mode needs - the previous value is not retrievable
            // from the return of the call that set it.
            "SetPaintMode" => {
                let [mode] = self.args()?;
                // An enum value outside `AEE_PAINT_COPY`/`AEE_PAINT_XOR` (the only two the header
                // defines) is specified to reset the mode to COPY, not to be stored and echoed.
                st.paint = if mode <= g::AEE_PAINT_XOR {
                    mode
                } else {
                    g::AEE_PAINT_COPY
                };
                out = st.paint;
            }
            "GetPaintMode" => out = st.paint,
            "SetClip" => {
                let [shape, flag] = self.args()?;
                // The body's order, in full: a null `pShape` calls `resetClip` and answers TRUE without
                // touching the caller's stored bytes; otherwise the kind must be 1, 2 or 3 — anything
                // else, `CLIPPING_NONE` included, is a FALSE returned **before** anything is stored;
                // then the sixteen bytes are kept, the clip is reset to the window, and the shape's own
                // gate runs (a negative radius is FALSE, but the bytes are stored and the box is already
                // reset by then).
                if shape == 0 {
                    st.clip = None;
                    st.clip_box = None;
                    out = g::TRUE;
                } else {
                    match Graphics::clip_of(&self.cpu.mem, shape) {
                        None => out = g::FALSE,
                        Some(Clip::None) | Some(Clip::Shape(_)) => out = g::FALSE,
                        Some(Clip::Rect(r)) => {
                            st.clip_raw = self.clip_bytes(shape)?;
                            let at = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                            let wide = Rect::new(at.0 as i16, at.1 as i16, r.dx, r.dy);
                            // `drawRect` does not replace the box, it *narrows* it: the new corners are
                            // clamped between what `[+88]..[+94]` already said and the shape's own
                            // extent, and a box that ends up inverted draws nothing. `SetClip` ran
                            // `resetClip` first, so "already said" is the window.
                            Graphics::set_clip(&mut st, wide, g::CLIPPING_RECT);
                            if flag != 0 {
                                self.paint_region(
                                    &st,
                                    at,
                                    (i32::from(r.dx), i32::from(r.dy)),
                                    flag,
                                );
                            }
                            out = g::TRUE;
                        }
                        Some(Clip::Ellipse(r)) => {
                            let kind = u32::from(self.cpu.mem.read_u8(shape)?);
                            let radius = i32::from(i16::from_le_bytes([
                                self.cpu.mem.read_u8(shape + 8)?,
                                self.cpu.mem.read_u8(shape + 9)?,
                            ]));
                            let second = if kind == g::CLIPPING_ELLIPSE {
                                Some(i32::from(i16::from_le_bytes([
                                    self.cpu.mem.read_u8(shape + 10)?,
                                    self.cpu.mem.read_u8(shape + 11)?,
                                ])))
                            } else {
                                None
                            };
                            st.clip_raw = self.clip_bytes(shape)?;
                            // `resetClip(a1)` runs before the radius gate, so a refused circle or
                            // ellipse still leaves the bytes stored and the clip box back at the window.
                            st.clip = None;
                            st.clip_box = None;
                            if radius < 0 || second.unwrap_or(0) < 0 {
                                out = g::FALSE;
                            } else {
                                // Same two steps as the rectangle: `convertCircleToRect`, then
                                // `transformRectToScreen`, then `initShapeClipInfo` narrows
                                // `[+88]..[+94]` to the result and `drawEllipse` paints it.
                                let at = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                                let wide = Rect::new(at.0 as i16, at.1 as i16, r.dx, r.dy);
                                Graphics::set_clip(&mut st, wide, kind);
                                if flag != 0 {
                                    self.paint_region(
                                        &st,
                                        at,
                                        (i32::from(r.dx), i32::from(r.dy)),
                                        flag,
                                    );
                                }
                                out = g::TRUE;
                            }
                        }
                    }
                }
            }
            "GetClip" => {
                let [shape] = self.args()?;
                if shape == 0 {
                    out = g::FALSE;
                } else {
                    // `aee_memmove(a2, a1 + 72, 16)` — the caller's own bytes, kept by `SetClip`, are
                    // handed straight back. Rebuilding the struct here would lose the three padding
                    // bytes a pack put behind the kind, which is the field `GetClip` is read for.
                    self.put_clip_bytes(shape, &st.clip_raw)?;
                    out = g::TRUE;
                }
            }
            "SetViewport" => {
                let [rect, flag] = self.args()?;
                if rect == 0 {
                    // Documented as a reset: the default viewport is the screen, and the clip and the
                    // framed byte go back with it. Written back rather than returned straight from here,
                    // because the write-back happens at the end of the method and this arm does not
                    // reach it. The world origin survives — the body recomputes `[+58]`/`[+60]` from
                    // `[+50]`/`[+52]` rather than clearing them, so a `Translate` is not undone here.
                    st.viewport = Rect::whole(self.device.width, self.device.height);
                    st.framed = false;
                    // `resetClip(a1)` in this branch too: kind byte 1 and the box copied from
                    // `[+62]..[+68]`, so the clip becomes the whole window rather than disappearing.
                    Graphics::reset_clip(&mut st);
                    self.graphics.set(this, st);
                    return Ok(g::TRUE);
                }
                let Ok(r) = Graphics::rect(&self.cpu.mem, rect) else {
                    return Ok(g::FALSE);
                };
                let (x, y, w, h) = (
                    i32::from(r.x),
                    i32::from(r.y),
                    i32::from(r.dx),
                    i32::from(r.dy),
                );
                // `if ((v10 >= 3 && a2[3] >= 3 || (a3 & 2) == 0) && v10 >= 1)` with `v11 >= 1` inside:
                // a **framed** viewport has to be at least 3x3, because the frame costs one pixel on
                // each side and the body wants something left inside it; an unframed one only has to be
                // nonempty. These are size rules, not screen rules, and they are checked first.
                if !((w >= 3 && h >= 3 || flag & g::AEE_GRAPHICS_FRAME == 0) && w >= 1 && h >= 1) {
                    return Ok(g::FALSE);
                }
                // Then the containment, on the **inclusive** far corner: `x + w - 1 >= [+8]` or
                // `y + h - 1 >= [+10]` (the destination's own width and height) is a refusal, and so is
                // a negative origin. On a refusal the previous window is kept, which is why this returns
                // early instead of falling through to the write-back.
                if x < 0
                    || y < 0
                    || x + w - 1 >= i32::from(self.device.width)
                    || y + h - 1 >= i32::from(self.device.height)
                {
                    self.display.note(format!(
                        "IGraphics::SetViewport({x},{y}) {w}x{h} is not a part of this {}x{} screen",
                        self.device.width, self.device.height
                    ));
                    return Ok(g::FALSE);
                }
                self.note_viewport("SetViewport", &r);
                st.framed = flag & g::AEE_GRAPHICS_FRAME != 0;
                // What is *stored* is the requested rectangle: `[+62]..[+68]` is written **before** the
                // inset, and those are the fields `GetViewport` answers from. The inset goes to
                // `[+54]/[+56]` and `[+76]..[+82]` — `Graphics::drawn`, which is what drawing and
                // clipping use. So a framed (10,10,20,20) reports 20x20 while painting inside 11..28.
                st.viewport = r;
                if st.framed {
                    self.paint_region(&st, (x, y), (w, h), g::AEE_GRAPHICS_FRAME);
                }
                let window = Graphics::drawn(&st);
                Graphics::reset_clip(&mut st);
                if flag & g::AEE_GRAPHICS_CLEAR != 0 {
                    // `fillRect(a1, a1 + 76, 0, [+24], 2)` — the stored (already inset) window, so with a
                    // frame the cleared band sits one pixel inside it.
                    let (ix, iy) = (i32::from(window.x), i32::from(window.y));
                    let (iw, ih) = (i32::from(window.dx), i32::from(window.dy));
                    self.paint_region(&st, (ix, iy), (iw, ih), g::AEE_GRAPHICS_CLEAR);
                }
                out = g::TRUE;
            }
            "GetViewport" => {
                let [rect, framed] = self.args()?;
                if rect == 0 {
                    out = g::FALSE;
                } else {
                    st.viewport.write(&mut self.cpu.mem, rect)?;
                    if framed != 0 {
                        // `*a3 = byte[+102]` — the flag *this* window was set with, which is why the
                        // answer is stored rather than derived. The body writes it without checking the
                        // pointer; a null one is skipped here because this host has no fault to hand back.
                        //
                        // One byte, because the parameter is `boolean *framed`
                        // — a four-byte store here would write three bytes past
                        // a caller's own flag, the same way `IMedia::GetState`'s did (see the media
                        // arm), and a caller that keeps that flag next to its saved registers would
                        // lose them.
                        self.cpu.mem.write_u8(framed, u8::from(st.framed))?;
                    }
                    out = g::TRUE;
                }
            }
            // `Translate` moves the world origin *by* an amount and `Pan` recentres it *at* a world
            // coordinate; neither touches the rectangle `GetViewport` reports. The body writes
            // `[+50]`/`[+52]` and recomputes the drawing offsets as `[+54] - [+50]`, so the pair lives
            // with [`Graphics::at`] and the reported viewport is `[+62]`-and-friends.
            "Translate" => {
                let [x, y] = self.args()?;
                let add = |old: i32, by: u32| (old + by as u16 as i16 as i32) as u16 as i16 as i32;
                st.translate = (add(st.translate.0, x), add(st.translate.1, y));
            }
            "Pan" => {
                let [cx, cy] = self.args()?;
                // `[+50] = cx - (([+64] - [+62] + 1) >> 1)` on each axis: half the *current window's*
                // extent, and the whole thing stored into a halfword. The world point named then lands
                // at the window's centre — which is also why `Translate` moves the picture the other
                // way: what these two write is where the world origin sits, not where the content goes.
                let centre =
                    |at: u32, span: i32| (at as u16 as i16 as i32 - span / 2) as u16 as i16 as i32;
                st.translate = (
                    centre(cx, i32::from(st.viewport.dx)),
                    centre(cy, i32::from(st.viewport.dy)),
                );
            }
            "ClearViewport" => {
                // `ClearViewport` builds its rect from `[+62]..[+68]` — the **outer** rect, the one
                // `GetViewport` answers — and shrinks it once when the framed byte is set. `drawn` is
                // that same shrink, so it must not be applied twice: a window set framed at (10,10) 20x20
                // clears 11..28, not 12..27.
                let window = Graphics::drawn(&st);
                let (x, y) = (i32::from(window.x), i32::from(window.y));
                let (w, h) = (i32::from(window.dx), i32::from(window.dy));
                let c = st.bg.native();
                if w > 0 && h > 0 {
                    self.canvas(st).fill(x, y, w, h, c);
                }
            }
            "ClearRect" => {
                let [rect] = self.args()?;
                // `if (a2 == 0 || *(int16 *)(a2 + 4) < 0 || *(int16 *)(a2 + 6) < 0) return 14;` then a
                // zero extent answers `0` having painted nothing.
                if rect == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let r = Graphics::rect(&self.cpu.mem, rect)?;
                let (w, h) = (i32::from(r.dx), i32::from(r.dy));
                if w < 0 || h < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if w > 0 && h > 0 {
                    // And the inspection says the rest outright: `movs r2, #4; bl drawRect` — the rect
                    // pointer is the pack's own `r1`, never transformed. So `ClearRect` paints in
                    // **device** coordinates while `DrawRect` paints in the world system: the two differ
                    // by exactly the origin every other shape gets. Reproducing it is the point; "fixed"
                    // to share `DrawRect`, this slot would erase a sprite one viewport-width away from the
                    // pixels the sprite is standing on.
                    let c = st.bg.native();
                    self.canvas(st).fill(
                        i32::from(r.x),
                        i32::from(r.y),
                        i32::from(r.dx),
                        i32::from(r.dy),
                        c,
                    );
                }
                out = g::AEE_SUCCESS;
            }
            "DrawPoint" => {
                let [point] = self.args()?;
                if point == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (px, py) = Graphics::point_of(&self.cpu.mem, point)?;
                let (x, y) = Graphics::at(&st, px, py);
                let (c, s) = (st.fg.native(), i32::from(st.point));
                self.canvas(st).fill(x, y, s, s, c);
                out = g::AEE_SUCCESS;
            }
            "DrawLine" => {
                let [line] = self.args()?;
                if line == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let l = Graphics::line_of(&self.cpu.mem, line)?;
                let (ax, ay) = Graphics::at(&st, l.a.0, l.a.1);
                let (bx, by) = Graphics::at(&st, l.b.0, l.b.1);
                self.canvas(st).line(Line {
                    a: (ax, ay),
                    b: (bx, by),
                });
                out = g::AEE_SUCCESS;
            }
            "DrawRect" => {
                let [rect] = self.args()?;
                if rect == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let r = Graphics::rect(&self.cpu.mem, rect)?;
                // `if (a2 == 0 || *(int16 *)(a2 + 4) < 0 || *(int16 *)(a2 + 6) < 0) return 14;` and then
                // `if (*(uint16 *)(a2 + 4) != 0 && *(uint16 *)(a2 + 6) != 0)` around the drawing: a
                // negative extent is a refusal, a zero one is a **silent success that paints nothing**.
                let (w, h) = (i32::from(r.dx), i32::from(r.dy));
                if w < 0 || h < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if w > 0 && h > 0 {
                    let (x, y) = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                    let mut cv = self.canvas(st);
                    if st.filled {
                        let c = st.fill.native();
                        cv.fill(x, y, w, h, c);
                    } else {
                        let c = st.fg.native();
                        cv.frame(x, y, w, h, c);
                    }
                }
                out = g::AEE_SUCCESS;
            }
            "DrawRoundRectangle" => {
                let [rect, aw, ah] = self.args()?;
                if rect == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let r = Graphics::rect(&self.cpu.mem, rect)?;
                let (w, h) = (i32::from(r.dx), i32::from(r.dy));
                let (arc_w, arc_h) = (aw as u16 as i16 as i32, ah as u16 as i16 as i32);
                // The refusal list is five long: a negative extent, a negative arc, or an arc the box
                // cannot hold (`w < arcWidth || h < arcHeight`) all answer `AEE_EBADPARM`.
                if w < 0 || h < 0 || arc_w < 0 || arc_h < 0 || w < arc_w || h < arc_h {
                    return Ok(g::AEE_EBADPARM);
                }
                if w == 0 || h == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                if arc_w <= 2 || arc_h <= 2 {
                    // `if (a3 <= 2 || a4 <= 2) return QGraphics_DrawRect(a1, a2);` — under two pixels of
                    // corner there is no corner to draw, and the body hands the whole call to `DrawRect`
                    // (so the fill mode decides, not the caller's arc size).
                    let (x, y) = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                    let mut cv = self.canvas(st);
                    if st.filled {
                        let c = st.fill.native();
                        cv.fill(x, y, w, h, c);
                    } else {
                        let c = st.fg.native();
                        cv.frame(x, y, w, h, c);
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                // `arcWidth` and `arcHeight` name the *axes* of the corner ellipse, following the
                // `RoundRect` convention the header keeps, so half of each is what reaches in from an
                // edge. The pen or the brush is the caller's stroke style, exactly as `DrawRect` reads
                // it — the same shape in two modes rather than two shapes.
                let (x, y) = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                let (rx, ry) = (arc_w / 2, arc_h / 2);
                let c = if st.filled { st.fill } else { st.fg }.native();
                let mut cv = self.canvas(st);
                cv.round_rect(x, y, w, h, rx, ry, c, st.filled);
                out = g::AEE_SUCCESS;
            }
            "DrawCircle" => {
                let [circle] = self.args()?;
                if circle == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (cx, cy, r) = Graphics::circle_of(&self.cpu.mem, circle)?;
                if r < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if r == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                let (x, y) = Graphics::at(&st, cx, cy);
                let mut cv = self.canvas(st);
                if st.filled {
                    let c = st.fill.native();
                    cv.disc(x, y, r, c);
                } else {
                    cv.circle(x, y, r);
                }
                out = g::AEE_SUCCESS;
            }
            "DrawPie" => {
                let [pie] = self.args()?;
                if pie == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (cx, cy, r, start, sweep) = Graphics::pie_of(&self.cpu.mem, pie)?;
                // `if (a2[2] < 0) return 14; if (a2[2] == 0 || a2[4] == 0) return 0;` — a negative radius
                // is a refusal, no radius or no sweep answers success with nothing drawn.
                if r < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if r == 0 || sweep == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                let (x, y) = Graphics::at(&st, cx, cy);
                let mut cv = self.canvas(st);
                cv.sector(x, y, r, start, sweep, st.filled);
                out = g::AEE_SUCCESS;
            }
            "DrawArc" => {
                let [arc] = self.args()?;
                if arc == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // `AEEArc` and `AEEPie` are the same five fields — the header says so to justify
                // casting one to the other — and the only difference is what gets drawn: this is the
                // curve with neither radii nor fill, whatever the fill mode says.
                let (cx, cy, r, start, sweep) = Graphics::pie_of(&self.cpu.mem, arc)?;
                // The same two gates as `DrawPie`, for the same struct: `a2[2] < 0` is `AEE_EBADPARM`,
                // `a2[2] == 0 || a2[4] == 0` is a success that draws nothing.
                if r < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if r == 0 || sweep == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                let (x, y) = Graphics::at(&st, cx, cy);
                let mut cv = self.canvas(st);
                cv.arc(x, y, r, start, sweep);
                out = g::AEE_SUCCESS;
            }
            "DrawEllipse" => {
                let [ellipse] = self.args()?;
                if ellipse == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (cx, cy, rx, ry) = Graphics::ellipse_of(&self.cpu.mem, ellipse)?;
                if rx < 0 || ry < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if rx == 0 || ry == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                let (x, y) = Graphics::at(&st, cx, cy);
                let mut cv = self.canvas(st);
                cv.elliptical_wedge(x, y, rx, ry, 0, 360, st.filled);
                out = g::AEE_SUCCESS;
            }
            "DrawEllipticalArc" => {
                let [rect, start, sweep] = self.args()?;
                if rect == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // The caller names the *containing* rectangle, so the figure has to be read out of it.
                // Half of each extent is the semi-axis and the sum is the centre, rounded inward so an
                // odd box keeps its symmetry and an even one stays inside the box rather than
                // painting one column past its right edge.
                let r = Graphics::rect(&self.cpu.mem, rect)?;
                // The gates are on the **box**, before anything is derived from it: a negative extent is
                // `AEE_EBADPARM`, and a zero box or a zero sweep is a success that draws nothing.
                let (bw, bh) = (i32::from(r.dx), i32::from(r.dy));
                if bw < 0 || bh < 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if bw == 0 || bh == 0 || (sweep as u16 as i16) == 0 {
                    return Ok(g::AEE_SUCCESS);
                }
                let (x, y) = Graphics::at(&st, i32::from(r.x), i32::from(r.y));
                let (rx, ry) = ((bw - 1) / 2, (bh - 1) / 2);
                let mut cv = self.canvas(st);
                cv.elliptical_wedge(
                    x + rx,
                    y + ry,
                    rx,
                    ry,
                    start as u16 as i16,
                    sweep as u16 as i16,
                    st.filled,
                );
                out = g::AEE_SUCCESS;
            }
            "DrawTriangle" => {
                let [tri] = self.args()?;
                if tri == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let corners = Graphics::triangle_of(&self.cpu.mem, tri)?;
                let pts: Vec<(i32, i32)> = corners
                    .iter()
                    .map(|(px, py)| Graphics::at(&st, *px, *py))
                    .collect();
                let c = if st.filled { st.fill } else { st.fg }.native();
                let mut cv = self.canvas(st);
                cv.polygon(&pts, c, st.filled, true);
                out = g::AEE_SUCCESS;
            }
            "StretchBlt" => {
                let [x, y, w, h, bits, rop] = self.args()?;
                // The prototype hands over a pointer to a Windows bitmap and no length, so the
                // picture's own header is the only answer there is — sized and then checked against
                // what is actually mapped, exactly as `CONVERTBMP` has to. A pointer that is not a
                // bitmap this host can read is a note and no pixels: the slot returns `void`, so
                // there is no code to return and a silent success would be a lie.
                if bits == 0 {
                    // `if (a6 != 0) { … }` around the whole body: a null source is not an error and not a
                    // refusal, it is nothing at all — the slot is `void` and the pack gets silence. The
                    // note is kept because the run is the only place a person can see why a sprite did
                    // not arrive, but the call answers, as it does on a handset.
                    self.display
                        .note("IGraphics::StretchBlt: a null source bitmap, and nothing to do");
                    return Ok(g::AEE_SUCCESS);
                }
                // `if (a7 >= 0xA) a7 = 2;` — an op the runtime does not know is *clamped*, not refused:
                // the picture still arrives, drawn with op 2.
                let rop = if rop >= 0xA { 2 } else { rop };
                let room = self.cpu.mem.mapped_run(bits, BMP_HEADER as u32) as usize;
                let head = self.cpu.mem.read_bytes(bits, room)?;
                let Some(total) = resource::file_size(&head) else {
                    self.display.note(format!(
                        "IGraphics::StretchBlt: {bits:#010x} is not a BMP this host can size \
                         ({room} byte(s) mapped)"
                    ));
                    return Err(Error::Hle(HleError::Wanted {
                        id,
                        label: label.to_string(),
                        want: "a readable bitmap".to_string(),
                        this,
                    }));
                };
                let whole = self.cpu.mem.mapped_run(bits, total as u32) as usize;
                if whole < total {
                    self.display.note(format!(
                        "IGraphics::StretchBlt: {bits:#010x} claims a {total}-byte BMP with {whole} \
                         mapped byte(s) behind it"
                    ));
                    return Err(Error::Hle(HleError::Wanted {
                        id,
                        label: label.to_string(),
                        want: "a readable bitmap".to_string(),
                        this,
                    }));
                }
                let bytes = self.cpu.mem.read_bytes(bits, total)?;
                let Ok(img) = resource::bmp(&bytes) else {
                    self.display.note(format!(
                        "IGraphics::StretchBlt: a {total}-byte BMP in a shape this host cannot decode"
                    ));
                    return Err(Error::Hle(HleError::Wanted {
                        id,
                        label: label.to_string(),
                        want: "a readable bitmap".to_string(),
                        this,
                    }));
                };
                let (x, y) = Graphics::at(&st, x as u16 as i16 as i32, y as u16 as i16 as i32);
                let mut cv = self.canvas(st);
                cv.stretch(
                    &img,
                    x,
                    y,
                    w as u16 as i16 as i32,
                    h as u16 as i16 as i32,
                    rop,
                );
            }
            "DrawPolygon" | "DrawPolyline" => {
                let [poly] = self.args()?;
                if poly == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let (len, points) = Graphics::polygon_head(&self.cpu.mem, poly)?;
                if points == 0 || len < if name == "DrawPolygon" { 3 } else { 2 } {
                    return Ok(g::AEE_EBADPARM);
                }
                // The ring arrives in the caller's drawing window, so each vertex goes through the
                // same origin and translation every other shape does before the fill starts.
                let pts: Vec<(i32, i32)> = Graphics::polygon_of(&self.cpu.mem, poly)?
                    .into_iter()
                    .map(|(x, y)| Graphics::at(&st, x, y))
                    .collect();
                let mut cv = self.canvas(st);
                cv.polygon(
                    &pts,
                    st.fill.native(),
                    st.filled,
                    // A polygon is a ring; a polyline is the same points with the last join left open.
                    name == "DrawPolygon",
                );
                out = g::AEE_SUCCESS;
            }
            "SetDestination" => {
                let [dst] = self.args()?;
                // `if (a2 != nullptr) (**a2)(a2); else result = (**(this + 16) + 64)(this[+16], &a2);`
                // — the caller's `IBitmap` is AddRef'd, or `IDisplay::GetDeviceBitmap` (slot 16) is asked
                // for one. Then `IBitmap::GetInfo` (slot 12, twelve bytes) sizes the window and the depth,
                // the previous destination is Released, and the body resets the viewport and the clip and
                // zeroes the world origin. A refusal from `GetInfo` is handed back with the new reference
                // dropped, so nothing is stored on the way out.
                let target = if dst == 0 {
                    self.display.device_bitmap()
                } else {
                    dst
                };
                let Some(bmp) = self.display.bitmap(target) else {
                    // A pointer that is not a bitmap this host carries. The documented behaviour would call a
                    // vtable through it, which is a crash rather than an answer; this host refuses and
                    // says which pointer, and keeps the state exactly as it stood.
                    self.display.note(format!(
                        "IGraphics::SetDestination: {target:#010x} is not a bitmap this host carries"
                    ));
                    return Ok(g::AEE_EBADPARM);
                };
                if dst != 0 {
                    self.hle.change_refs(&mut self.cpu.mem, dst, 1).ok();
                }
                if st.dest != 0 && st.dest != target {
                    self.hle.change_refs(&mut self.cpu.mem, st.dest, -1).ok();
                }
                st.dest = target;
                st.depth = u32::from(bmp.depth);
                st.viewport = Rect::whole(bmp.width as u16, bmp.height as u16);
                st.framed = false;
                st.translate = (0, 0);
                // `QGraphics_SetViewport(a1, 0, 0)` then `QGraphics_SetClip(a1, nullptr, 0)` — which is
                // `resetClip`, i.e. a rectangle over the new window, not "no clip".
                Graphics::reset_clip(&mut st);
                out = g::AEE_SUCCESS;
            }
            "GetDestination" => {
                let target = if st.dest == 0 {
                    self.display.device_bitmap()
                } else {
                    st.dest
                };
                self.hle.change_refs(&mut self.cpu.mem, target, 1).ok();
                out = target;
            }
            // `byte[+105]`, written from the same `IBitmap::GetInfo` that sized the window — so this is
            // the destination's depth, and it changes only when the destination does.
            "GetColorDepth" => out = st.depth,
            "EnableDoubleBuffer" => {
                let [flag] = self.args()?;
                // The body is one line: `return a2;`. It stores nothing and asks nothing — the runtime
                // object has no double-buffer field at all, and what the header calls "returns FALSE when
                // the client tries to turn on the double buffering" is left to whatever the device does
                // when `Update` reaches it. So the answer here is the echo, and the note is what says this
                // host still presents the framebuffer as it is written.
                out = flag;
                self.display.note(format!(
                    "IGraphics::EnableDoubleBuffer({flag}) echoed {flag}: the platform routine stores no \
                     double-buffer state and `IGraphics::Update` is a tail call to IDisplay::Update(TRUE)"
                ));
            }
            "Update" => {
                // `return (*(int (**)(void *, int))(**(this + 16) + 28))(this[+16], 1);` — slot 7 of the
                // `IDisplay` the object was built from, with `TRUE`, and that call's answer is this slot's
                // answer. There is no "no effect" branch here for the host to keep itself honest with:
                // whatever double-buffering the device has is the device's business, and this host's
                // `IDisplay::Update` is what shows the pixels.
                self.display.update(&self.cpu.mem, st.display, g::TRUE)?;
            }
            // The method name rides along, because this is the one report a person playing a game
            // reads and `IGraphics::DrawEllipticalArc` is not what they are looking for — what the
            // game stopped *doing* is. See [`HleError::Wanted`].
            _ => {
                return Err(Error::Hle(HleError::Wanted {
                    id,
                    label: label.to_string(),
                    want: name.to_string(),
                    this,
                }))
            }
        }
        self.graphics.set(this, st);
        if name == "SetDestination" {
            self.mirror_graphics_canvas(this)?;
        }
        Ok(out)
    }

    /// Write the destination bitmap into the private `IGraphics` record at the offset the profile
    /// names. A pack that walks the firmware's own record reads this word rather than calling
    /// `GetDestination`; zero offset means this handset's layout is not mirrored.
    pub(crate) fn mirror_graphics_canvas(&mut self, this: u32) -> HResult<()> {
        let offset = self.graphics_canvas_offset;
        if offset == 0 {
            return Ok(());
        }
        // The caller has just installed the state, or is the constructor; a record that is not
        // there is not one this host should be writing through.
        let Some(st) = self.graphics.get(this) else {
            return Ok(());
        };
        let target = if st.dest == 0 {
            self.display.device_bitmap()
        } else {
            st.dest
        };
        self.cpu.mem.write_u32(this + offset, target)?;
        Ok(())
    }

    /// The drawing context for one call: this host's three services, borrowed together, with the
    /// state the shapes draw under.
    pub(crate) fn canvas(&mut self, st: State) -> Ctx<'_> {
        Ctx::new(&mut self.cpu.mem, &mut self.display, st)
    }

    /// Register the JPEG decoder's face pair for one created object.
    ///
    /// One stream, two `this` pointers: the reference record is 40 bytes with the second face at `me + 8`
    /// and `[+12]` pointing back at the base, so a `QueryInterface` through either lands in the same
    /// body. This host gives each face its own object, so the shared [`jpegdecoder::Feed`] is keyed by the
    /// decoder face and both addresses map to it.
    ///
    /// ⚠ `#[inline(never)]` is load-bearing, not style: this runs from the `CreateInstance` arm, which is
    /// one of the frames `the_host_stack_one_downcall_level_costs_is_measured` walks 32 times, and the
    /// map inserts here cost the level 192 bytes when they are inlined into it.
    #[inline(never)]
    pub(crate) fn jpeg_remember_faces(
        &mut self,
        cls: u32,
        addr: u32,
        faces: &[(&'static str, u32)],
    ) {
        if cls != g::AEECLSID_JPEGDECODER {
            return;
        }
        self.jpeg.insert(addr, jpegdecoder::Feed::default());
        self.jpeg_faces.insert(addr, addr);
        for (_, made) in faces.iter().skip(1) {
            self.jpeg_other.insert(addr, *made);
            self.jpeg_faces.insert(*made, addr);
        }
    }

    /// What `JPEGDecoder_Release` does at zero: `[+36]` goes with the record and the cached `[+24]`
    /// bitmap is released, so a picture cannot outlive the decoder that was fed it.
    ///
    /// It returns nothing on purpose — this runs from [`Runtime::object_method`], and the frame budget
    /// measured by `the_host_stack_one_downcall_level_costs_is_measured` is charged per down-call level
    /// (see [`Runtime::sock_died`]). The only fallible step is letting go of a reference, and the object
    /// being gone is the same outcome as the release succeeding, so it is said out loud rather than
    /// turned into a fault the caller cannot act on.
    pub(crate) fn jpeg_died(&mut self, addr: u32) {
        let Some(key) = self.jpeg_faces.remove(&addr) else {
            return;
        };
        self.jpeg_faces.remove(&key);
        self.jpeg_other.remove(&key);
        let Some(mut feed) = self.jpeg.remove(&key) else {
            return;
        };
        if let Some(dib) = feed.dib.take() {
            if let Err(err) = self.hold(dib, -1) {
                self.display.note(format!(
                    "jpegdecoder: the cached bitmap {dib:#x} was already gone: {err}"
                ));
            }
        }
    }

    #[inline(never)]
    pub(crate) fn jpeg_slot(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
        key: u32,
    ) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                match jpegdecoder::Feed::face_of(iid) {
                    // The decoder address, whatever face was called: the documented behaviour answers with the
                    // record base it reached through `[+12]`.
                    jpegdecoder::Face::Same => self.return_object(key, ppv),
                    jpegdecoder::Face::Other => {
                        let other = *self
                            .jpeg_other
                            .get(&key)
                            .ok_or(HleError::NotAnObject { addr: this })?;
                        self.return_object(other, ppv)
                    }
                    // Clear the caller's slot and answer 3 — and `AEECLSID_JPEGDECODER` itself is not in
                    // the list of three, which is where this object differs from the BMP decoder beside it.
                    // The body takes no `AEE_CheckPtr`, so a null `ppObj` is a store to address 0 there;
                    // this host skips the store and keeps the answer.
                    jpegdecoder::Face::None => {
                        if ppv != 0 {
                            self.cpu.mem.write_u32(ppv, 0)?;
                        }
                        Ok(jpegdecoder::AEE_ECLASSNOTSUPPORT)
                    }
                }
            }
            // A four-byte body, `return 2` = `AEE_RO_COPY`, reading nothing: baseline JPEG names no
            // transparent pixel, so the default raster op is a copy. Unlike the BMP decoder's `GetRop`
            // this one cannot vary, and the answer is not this host's opinion of JPEGs.
            "GetRop" => Ok(jpegdecoder::AEE_RO_COPY),
            "GetBitmap" => {
                let [ppb] = self.args()?;
                let dib = self.jpeg.get(&key).and_then(|feed| feed.bitmap());
                // The store comes first and the test second, in this order in the body: a caller that
                // asks before any flush is handed a null, not the value it brought in.
                if ppb != 0 {
                    self.cpu.mem.write_u32(ppb, dib.unwrap_or(0))?;
                }
                match dib {
                    // `IBitmap::AddRef` on the cached picture, so two calls before the next flush name one
                    // bitmap and hold two references to it.
                    Some(addr) => {
                        self.hold(addr, 1)?;
                        Ok(g::AEE_SUCCESS)
                    }
                    // `1`, not a refusal code: the body's own answer when nothing has been decoded yet.
                    None => Ok(jpegdecoder::AEE_EFAILED),
                }
            }
            "Write" => {
                let [pbuf, cb] = self.args()?;
                match jpegdecoder::Feed::write_kind(pbuf == 0, cb as i32) {
                    jpegdecoder::Write::Ignored => Ok(g::AEE_SUCCESS),
                    jpegdecoder::Write::Impossible => Ok(jpegdecoder::AEE_ENOMEMORY),
                    jpegdecoder::Write::Append => {
                        let more = self.cpu.mem.read_bytes(pbuf, cb as usize)?;
                        if let Some(feed) = self.jpeg.get_mut(&key) {
                            feed.bytes.extend_from_slice(&more);
                        }
                        Ok(g::AEE_SUCCESS)
                    }
                    jpegdecoder::Write::Flush => {
                        // `[+36] == 0` is the first question the flush asks, and its answer is the
                        // initialized `1` — no decode, no picture, and the caller's `GetBitmap` still says 1.
                        let nothing = match self.jpeg.get(&key) {
                            Some(feed) => !feed.has_bytes(),
                            None => true,
                        };
                        if nothing {
                            return Ok(jpegdecoder::AEE_EFAILED);
                        }
                        let bytes = match self.jpeg.get_mut(&key) {
                            Some(feed) => std::mem::take(&mut feed.bytes),
                            None => Vec::new(),
                        };
                        // Both decode-failure shapes answer `1`; the picture already at `[+24]` stays, which
                        // is the header-rejection shape rather than the mid-scan one.
                        let Ok(img) = resource::jpeg(&bytes) else {
                            return Ok(jpegdecoder::AEE_EFAILED);
                        };
                        let dib = self.raster(&img)?;
                        // The object's birth reference *is* the decoder's `[+24]`: `Reset` and the last
                        // `Release` are the only things that let it go, and a second flush overwrites the
                        // cache without releasing the first, which is what the body does too.
                        if let Some(feed) = self.jpeg.get_mut(&key) {
                            feed.dib = Some(dib);
                        }
                        Ok(g::AEE_SUCCESS)
                    }
                }
            }
            // `[+36]` goes, and so does the cached picture: `JPEGDecoder_Reset` releases `[+24]` and
            // nulls it, so a `GetBitmap` after a `Reset` answers 1 rather than handing back a bitmap the
            // decoder no longer holds.
            "Reset" => {
                let freed = match self.jpeg.get_mut(&key) {
                    Some(feed) => feed.reset(),
                    None => None,
                };
                if let Some(dib) = freed {
                    self.hold(dib, -1)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IImageDecoder` and `IForceFeed` — the two faces of `AEECLSID_BMPDecoder` (`0x01026e21`).
    ///
    /// "This interface is used to retrieve an `IBitmap` from an image decoder. **Typically, classes that
    /// export the `IImageDecoder` interface will also export another interface that is used to feed encoded
    /// data to the decoder. Currently, the `IForceFeed` interface exists for this purpose.**",
    /// and the header's own table names the classes: BMP, JPEG and PNG. This host already decodes
    /// BMP — `SetupNativeImage`
    /// and `LoadResObject` both go through `resource::bmp` — so this decoder is that decoder, behind two
    /// pointers instead of one.
    pub(crate) fn iimagedecoder(
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
        if let Some(&key) = self.jpeg_faces.get(&this) {
            return self.jpeg_slot(id, this, label, name, key);
        }
        match name {
            // `AEEIID_IImageDecoder` is this face, `AEEIID_IForceFeed` is the one beside it, and anything
            // else is the base, which `answer_qi` answers with this same pointer.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if iid == g::AEEIID_IForceFeed {
                    self.return_face(g::AEECLSID_BMPDecoder, "IForceFeed", this, ppv)
                } else if iid == g::AEEIID_IImageDecoder {
                    self.return_face(g::AEECLSID_BMPDecoder, "IImageDecoder", this, ppv)
                } else {
                    self.answer_qi(this, iid, ppv, &[g::AEECLSID_BMPDecoder])
                }
            }
            // `int GetBitmap(IBitmap **ppiBitmap)`: "This function is used to retrieve the decoded bitmap".
            //The bytes are whatever `IForceFeed::Write` left, decoded through the same
            // `resource::bmp` every other picture in this host goes through — so a picture that arrives by
            // this door and one that arrives by `LoadResObject` cannot disagree about what it looks like.
            "GetBitmap" => {
                let [ppi] = self.args()?;
                if ppi == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let bytes = std::mem::take(&mut self.decoder_bytes);
                let Ok(img) = resource::bmp(&bytes) else {
                    self.display.note(format!(
                        "{label}: a {}-byte feed is not a BMP this host decodes",
                        bytes.len()
                    ));
                    self.cpu.mem.write_u32(ppi, 0)?;
                    return Ok(g::AEE_EFAILED);
                };
                let obj = self.raster(&img)?;
                self.cpu.mem.write_u32(ppi, obj)?;
                Ok(g::AEE_SUCCESS)
            }
            // `int GetRop(po)`: "returns the default raster op to be used when blitting this bitmap. …
            // This will return `AEE_RO_COPY` for opaque images, and either `AEE_RO_TRANSPARENT` or
            // `AEE_RO_BLEND` for transparent images, depending on the transparency type".
            // This host plots a transparent pixel by leaving it alone, which is `AEE_RO_TRANSPARENT`, and
            // it has no alpha blending to answer `AEE_RO_BLEND` with.
            "GetRop" => {
                let bytes = self.decoder_bytes.clone();
                let rop = match resource::bmp(&bytes) {
                    Ok(img) if img.alpha.any_transparent() => g::AEE_RO_TRANSPARENT,
                    _ => g::AEE_RO_COPY,
                };
                Ok(rop)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IForceFeed` — the feeding half of the same object.
    pub(crate) fn iforcefeed(
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
        if let Some(&key) = self.jpeg_faces.get(&this) {
            return self.jpeg_slot(id, this, label, name, key);
        }
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if iid == g::AEEIID_IImageDecoder {
                    self.return_face(g::AEECLSID_BMPDecoder, "IImageDecoder", this, ppv)
                } else if iid == g::AEEIID_IForceFeed {
                    self.return_face(g::AEECLSID_BMPDecoder, "IForceFeed", this, ppv)
                } else {
                    self.answer_qi(this, iid, ppv, &[g::AEECLSID_BMPDecoder])
                }
            }
            // `int Write(void *pBuf, int cb)`: the encoded bytes, which accumulate until a `GetBitmap`.
            "Write" => {
                let [buf, cb] = self.args()?;
                if cb != 0 && buf == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let more = self.cpu.mem.read_bytes(buf, cb as usize)?;
                self.decoder_bytes.extend_from_slice(&more);
                Ok(g::AEE_SUCCESS)
            }
            // `void Reset(po)`: start over. Whatever was fed is dropped, so the next `Write` begins a fresh
            // image instead of appending to a half-read one.
            "Reset" => {
                self.decoder_bytes.clear();
                Ok(0)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Drop the raster(s) a previous `SetStream` installed on this image. An animated image owns a
    /// frame list; a static image owns the single raster in `images`. Either way the old picture is
    /// no longer reachable from this image, so its pixels and palettes go back here.
    pub(crate) fn drop_image_raster(&mut self, image: u32) -> HResult<()> {
        let still = self.images.remove(&image);
        if let Some(frames) = self.image_frames.remove(&image) {
            for raster in frames.rasters {
                self.display
                    .forget(&mut self.cpu.mem, &mut self.hle, raster)?;
            }
        } else if let Some(src) = still {
            if src != 0 {
                self.display.forget(&mut self.cpu.mem, &mut self.hle, src)?;
            }
        }
        Ok(())
    }

    /// Report a decode that finished before `SetStream` returned, once, to the callback the pack
    /// armed with `IImage_Notify`.
    fn queue_image_notify(&mut self, image: u32) {
        if let Some(parm) = self.image_parm.get(&image).copied() {
            let (pfn, user) = parm.notify;
            if pfn != 0 {
                self.queue.push_back(Job::Image { image, pfn, user });
                if let Some(parm) = self.image_parm.get_mut(&image) {
                    parm.notify = (0, 0);
                }
            }
        }
    }

    /// `IImage`: the picture a module draws without ever touching its pixels.
    ///
    /// The class is draw-shaped rather than pixel-shaped — `Draw(x, y)`, `GetInfo`, animation
    /// controls — so the host does the blitting and the module only says where. A frame index
    /// slides the source window when the caller named a frame layout (`IPARM_CXFRAME`/`IPARM_NFRAMES`);
    /// with none named it is dropped rather than turned into a stride this host invented, and
    /// `GetInfo` says so through `bAnimated`/`cxFrame`.
    pub(crate) fn iimage(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let gap = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let src = self
            .images
            .get(&this)
            .copied()
            .ok_or(HleError::NotAnObject { addr: this })?;
        // An image the shell created has no raster until its owner calls `SetStream`, and `IImage_GetInfo`
        // and `IImage_Draw` are both `void` in the header — there is no code to hand back and no
        // notification to fire, so a refusal would end a run over a call that is merely early. What the
        // two halves do differs, though, and the difference is the repo's own rule for this shape:
        // drawing nothing is what an empty picture does, but *measuring* it is a question with an answer,
        // and `Ok(0)` with the caller's struct untouched is "silence plus a success code" — the shape
        // the `answered-zero` rule forbids. So `GetInfo` reports a picture that
        // measures nothing (and says so in the note), exactly as `IPARM_GETBITMAP` writes a null pointer
        // plus the reason beside it. An audit found the first version of this block claiming in prose
        // that leaving the struct untouched "would be a guess" while doing precisely that.
        if src == 0 && matches!(name, "GetInfo" | "Draw" | "DrawFrame") {
            self.display
                .note(format!("{label}_{name} on an image with no stream set yet"));
            if name == "GetInfo" {
                let [pi] = self.args()?;
                let parm = self.image_parm.get(&this).copied().unwrap_or_default();
                self.put_image_info(pi, 0, 0, 0, &parm)?;
            }
            return Ok(0);
        }
        match name {
            "GetInfo" => {
                let [pi] = self.args()?;
                let b = self
                    .display
                    .bitmap(src)
                    .ok_or(HleError::NotAnObject { addr: src })?;
                let mut parm = self.image_parm.get(&this).copied().unwrap_or_default();
                if let Some(frames) = self.image_frames.get(&this) {
                    parm.frames = Some(frames.rasters.len() as i32);
                    parm.cx_frame = Some(i32::from(b.width));
                }
                // `cnt_rgb` is already "how many palette entries, or zero when the pixels are
                // themselves colours", which is the same reading the converter path reports.
                self.put_image_info(
                    pi,
                    b.width as usize,
                    b.height as usize,
                    b.cnt_rgb as usize,
                    &parm,
                )?;
                Ok(0)
            }
            "Draw" => {
                let [x, y] = self.args()?;
                // `AEEImage_Draw` is `AEEImage_DrawFrame(po, -1, x, y)` — the body spells the
                // whole-image index itself, so this slot does too.
                self.blit_image(this, -1, x as i32, y as i32)
            }
            "DrawFrame" => {
                let [frame, x, y] = self.args()?;
                if frame <= i32::MAX as u32 {
                    let frame = frame as i32;
                    if let Some(state) = self.image_frames.get_mut(&this) {
                        state.current =
                            (frame.max(0) as usize).min(state.rasters.len().saturating_sub(1));
                        // An explicit frame is a caller-driven selection; the clock no longer
                        // advances it.
                        state.started_ms = None;
                    }
                }
                self.blit_image(this, frame as i32, x as i32, y as i32)
            }
            // `AEEImage_Start(x, y)` stores the corner, zeroes its position word and calls its own
            // `Animate`; `Stop` is `CALLBACK_Cancel`. This host has no animation timer, so the two
            // slots keep the state the caller named — and answer zero instead of ending the run,
            // which is what a refusal on a `void` slot used to do.
            "Start" => {
                let [x, y] = self.args()?;
                self.image_parm.entry(this).or_default().anim = Some((x as i16, y as i16));
                if let Some(state) = self.image_frames.get_mut(&this) {
                    state.started_ms = Some(self.clock_ms);
                    state.current = 0;
                }
                Ok(0)
            }
            "Stop" => {
                self.image_parm.entry(this).or_default().anim = None;
                if let Some(state) = self.image_frames.get_mut(&this) {
                    state.started_ms = None;
                }
                Ok(0)
            }
            "HandleEvent" => Ok(self.no_implementation_false("IImage", "HandleEvent")),
            // `AEEImage_Notify(pfn, pUser)` — `PFNIMAGEINFO(pUser, pIImage, AEEImageInfo*, nErr)`,
            // the report the header tells a pack to wait for before it draws. `SetStream` here has
            // the picture before it returns, so an image that already carries one owes the callback
            // immediately; it is *queued* rather than called, because the pack arms it in the same
            // breath as it blocks, and a notify that ran inside this call would resume a thread that
            // had not parked yet.
            "Notify" => {
                let [pfn, user] = self.args()?;
                self.image_parm.entry(this).or_default().notify = (pfn, user);
                if pfn != 0 && self.images.get(&this).is_some_and(|src| *src != 0) {
                    self.queue.push_back(Job::Image {
                        image: this,
                        pfn,
                        user,
                    });
                    if let Some(parm) = self.image_parm.get_mut(&this) {
                        parm.notify = (0, 0);
                    }
                }
                Ok(0)
            }
            // `IImage_SetParm(nParm, p1, p2)` is one call carrying the whole `IPARM_*` protocol.
            // `IPARM_DISPLAY` is not a value but *a destination*: the header's own `IImage_SetDisplay`
            // helper hands over an IDisplay to draw on, and the first store binary run through
            // this host hands back a display it wrote itself — an off-screen surface whose `BitBlt`
            // slot is the only thing that clears the status word the pack reads to decide whether its
            // own picture loaded. `IPARM_SIZE`/`OFFSET`/`ROP` are values, and they are the three that
            // say *which pixels and how*: see [`ImageParm`]. The rest stay unanswered and said.
            "SetParm" => {
                let [parm, p1, p2] = self.args()?;
                match parm {
                    g::IPARM_DISPLAY => {
                        // `AEEImage_SetParm` case 8 wraps the whole body in `if (p1 != nullptr)`:
                        // a null display is ignored, not an unbind.
                        if p1 != 0 {
                            self.image_display.insert(this, p1);
                        }
                    }
                    g::IPARM_SIZE => {
                        self.image_parm.entry(this).or_default().size = (p1 as i32, p2 as i32);
                    }
                    g::IPARM_OFFSET => {
                        self.image_parm.entry(this).or_default().off = (p1 as i32, p2 as i32);
                    }
                    g::IPARM_ROP => {
                        // `AEERasterOp` is an `int8`, so the register's upper three bytes are not
                        // part of the value the caller meant.
                        self.image_parm.entry(this).or_default().rop = p1 & 0xff;
                    }
                    g::IPARM_CXFRAME => {
                        self.image_parm.entry(this).or_default().cx_frame = Some(p1 as i32);
                    }
                    g::IPARM_NFRAMES => {
                        self.image_parm.entry(this).or_default().frames = Some(p1 as i32);
                    }
                    g::IPARM_RATE => {
                        // Case 5: a negative period is replaced by 150 ms (`MOVS R5,#0x96`), and
                        // zero also calls the object's slot 7 — `Stop` — so "rate 0" is a stop.
                        let rate = if (p1 as i32) < 0 { 150 } else { p1 };
                        let mut parm = self.image_parm.get(&this).copied().unwrap_or_default();
                        parm.rate = rate;
                        if p1 == 0 {
                            parm.anim = None;
                            if let Some(state) = self.image_frames.get_mut(&this) {
                                state.started_ms = None;
                            }
                        }
                        self.image_parm.insert(this, parm);
                    }
                    g::IPARM_REDRAW => {
                        // Case 7 stores the pair. This host keeps it and never fires it.
                        self.image_parm.entry(this).or_default().redraw = (p1, p2);
                    }
                    g::IPARM_DECODER => {
                        // Case 11 replaces the decoder object and re-points the image at it. This
                        // host decodes with its own reader and keeps no decoder-object registry,
                        // so the answer is named rather than silently fallen through.
                        self.display.note(format!(
                            "IImage_SetParm(IPARM_DECODER, {p1:#010x}): this host decodes with its own reader"
                        ));
                    }
                    g::IPARM_GETROP => {
                        // the header — "p1 = pointer to rop; p2 = pointer to result code (or
                        // NULL)". Both words belong to the caller, so the answer goes where it was
                        // asked for: the op this host is holding (whose default the header names) and
                        // the code that says the read happened.
                        self.cpu.mem.write_u32(
                            p1,
                            self.image_parm.get(&this).map_or(g::AEE_RO_COPY, |p| p.rop),
                        )?;
                        if p2 != 0 {
                            self.cpu.mem.write_u32(p2, g::AEE_SUCCESS)?;
                        }
                    }
                    g::IPARM_GETBITMAP => {
                        // the header, the same shape with a bitmap instead of an op. This host
                        // keeps no `IBitmap` behind an `IImage` — the decoded picture is read at draw
                        // time — so the pointer comes back null and the second word says why, which is
                        // what it is there for. A caller that read only the first would take "no
                        // bitmap" for "the bitmap is object 0".
                        self.cpu.mem.write_u32(p1, 0)?;
                        if p2 != 0 {
                            self.cpu.mem.write_u32(p2, g::AEE_EUNSUPPORTED)?;
                        }
                    }
                    g::IPARM_EXTFUNC => {
                        // the header declares the struct, and the helper at `112-115` shows
                        // how it is driven: `SetParm(IPARM_EXTFUNC, (int)&parm, (int)&nReturn)` and
                        // then `return nReturn;`. So the value the caller acts on is the word at `p2`,
                        // and an extension this host implements no CLSID of has to be told there —
                        // the return word alone would read as success to that caller.
                        if p2 != 0 {
                            self.cpu.mem.write_u32(p2, g::AEE_EUNSUPPORTED)?;
                        }
                    }
                    _ => {
                        self.display.note(format!(
                            "IImage_SetParm({parm}) names nothing this host behaves on"
                        ));
                    }
                }
                Ok(0)
            }
            // `IImage_SetStream(pIImage, pIAStream)` is the *only* way bytes reach an image on this
            // interface — the header's vtable has no `Read` slot — so answering it without reading
            // the stream is what leaves a pack holding an image that was never loaded, and the
            // `GetInfo` it then asks reports whatever the object was born with rather than the truth.
            "SetStream" => {
                let [stream] = self.args()?;
                let bytes = self.drain_stream(stream)?;
                // A stream can name several frames rather than one picture. Those frames are laid
                // into rasters of their own; `DrawFrame` then selects one and `Draw` can advance
                // through them on the clock. A single-frame format stays on the static path below,
                // which keeps its indexed palette and sprite-sheet behaviour.
                if let Some(Ok(animation)) = resource::animation(&[], &bytes) {
                    if animation.frames.len() > 1 {
                        let mut rasters = Vec::with_capacity(animation.frames.len());
                        for frame in &animation.frames {
                            rasters.push(self.raster(frame)?);
                        }
                        self.drop_image_raster(this)?;
                        self.images.insert(this, rasters[0]);
                        let parm = self.image_parm.entry(this).or_default();
                        parm.rop = self.display.default_rop(rasters[0]);
                        parm.frames = Some(animation.frames.len() as i32);
                        parm.cx_frame = Some(animation.frames[0].width as i32);
                        self.image_frames.insert(
                            this,
                            ImageFrames {
                                rasters,
                                delays: animation.delays,
                                current: 0,
                                started_ms: None,
                            },
                        );
                        self.queue_image_notify(this);
                        return Ok(0);
                    }
                    if animation.frames.len() == 1
                        && (bytes.starts_with(resource::SAF_MAGIC)
                            || bytes.starts_with(resource::SIS_MAGIC))
                    {
                        // A one-frame SIS is the common case and its frame is already decoded
                        // here; reusing it keeps `SetStream` from inflating it a second time.
                        if let Some(frame) = animation.frames.into_iter().next() {
                            let src = self.raster(&frame)?;
                            self.image_parm.entry(this).or_default().rop =
                                self.display.default_rop(src);
                            self.drop_image_raster(this)?;
                            self.images.insert(this, src);
                            self.queue_image_notify(this);
                            return Ok(0);
                        }
                    }
                }
                // An empty property block, which is the case `resource::image` covers by letting the
                // payload name its own format: a stream carries bytes, not a container's MIME field.
                match resource::image(&[], &bytes) {
                    Ok(img) => {
                        let src = self.raster(&img)?;
                        // A picture arriving through a stream is loaded here rather than at object
                        // creation, so this is where its default raster op is chosen — and it is
                        // chosen the same way, from what the file named. Other parameters the caller
                        // already set (a portion of a sheet, a display) are kept.
                        self.image_parm.entry(this).or_default().rop =
                            self.display.default_rop(src);
                        self.drop_image_raster(this)?;
                        self.images.insert(this, src);
                        self.queue_image_notify(this);
                    }
                    // The slot returns `void`, so there is no code to hand back. A stream this host
                    // cannot decode is said out loud: silence would make "no decoder" indistinguishable
                    // from "the pack loaded it fine", so the refusal is recorded where the report
                    // shows it.
                    Err(e) => self.display.note(format!(
                        "IImage stream of {} byte(s) is not a picture this host decodes: {e}",
                        bytes.len()
                    )),
                }
                Ok(0)
            }
            // Refused rather than answered: an image-loaded notification is the signal a module may
            // be waiting on before it draws, and firing it wrongly is worse than naming the gap.
            _ => Err(gap()),
        }
    }
}
