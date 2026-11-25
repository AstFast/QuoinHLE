//! The framebuffer surface: `IDisplay`, the four `IBitmap` faces, the blit
//! and raster paths, sprites, fonts and screenshots.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// Lay a decoded picture into a fresh `IBitmap` at the depth this screen uses, and return its
    /// object address. Two callers need exactly this: `ISHELL_LoadResObject` — which hands this
    /// object straight to a pack that asked for a bitmap, and keeps it behind an `IImage` for one
    /// that asked for a viewer — and the `CONVERTBMP` helper whose result the app hands straight to
    /// `IDISPLAY_BitBlt`.
    pub(crate) fn raster(&mut self, img: &resource::Bitmap) -> HResult<u32> {
        // `IBitmap` names its extent in `u16`s and a decoded picture does not have to: a 24-bpp strip
        // 70 000 pixels wide fits the size cap and would silently truncate to 4 464 here, leaving the
        // copy below writing over a hundred thousand bytes past the buffer it was given. Refused by
        // shape, because a picture this host cannot lay out is not a picture.
        let too_big = |n: usize| HleError::Unsupported {
            what: format!("a decoded picture {n} pixels across, past the 65 535 an IBitmap names"),
        };
        let w = u16::try_from(img.width).map_err(|_| too_big(img.width))?;
        let h = u16::try_from(img.height).map_err(|_| too_big(img.height))?;
        // The picture is laid into the destination's own format, not a fixed 16 bpp: a 32-bpp frame's
        // `SetupNativeImage`/`LoadResObject` bitmap is a four-byte surface, and a pack that composes
        // into the bitmap it was handed writes at that stride.
        let depth = self.display.device.depth;
        let src = self
            .display
            .new_bitmap(&mut self.cpu.mem, &mut self.hle, w, h, depth)?;
        let dst = self
            .display
            .bitmap(src)
            .ok_or(HleError::NotAnObject { addr: src })?;
        for y in 0..img.height {
            for x in 0..img.width {
                let (r, g, b) = img.rgb(x, y);
                let at = dst.px + (y as i32 * dst.pitch) as u32 + x as u32 * u32::from(depth) / 8;
                if depth == 32 {
                    self.cpu
                        .mem
                        .write_u32(at, ((r as u32) << 16) | ((g as u32) << 8) | b as u32)?;
                } else {
                    self.cpu
                        .mem
                        .write_u16(at, rgb_to_native(device::make_rgb(r, g, b)) as u16 as u32)?;
                }
            }
        }
        // The picture's own transparency goes onto the bitmap the blitter will read. Flattening to
        // 565 above keeps the *colours* the pack provides, and this is the other half of the same
        // file: a PNG's `tRNS` entry or alpha channel says which of those pixels let the screen show
        // through, and without it a see-through tile background paints as its stored white.
        let map = if img.alpha.any_transparent() {
            let bytes = (0..img.height)
                .flat_map(|y| (0..img.width).map(move |x| img.alpha_of(x, y)))
                .collect::<Vec<u8>>();
            AlphaMap::new(bytes, img.width)
        } else {
            None
        };
        self.display.set_alpha(src, map)?;
        Ok(src)
    }

    /// The interior and frame drawing that `SetClip` and `SetViewport` are asked for by their
    /// `nFlag` bits, which is part of *those* calls rather than a separate one: the header's
    /// Comments say the frame is drawn, the interior cleared, or the interior filled "while setting"
    /// the region.
    ///
    /// `CLEAR` is the background colour, `FILL` the current fill colour, `FRAME` the foreground as an
    /// outline — the same three colours the primitive setters use, through the same canvas, so the
    /// paint mode and the clip apply exactly as they do to a `DrawRect`.
    ///
    /// `origin` is already in screen coordinates, and that is deliberate: `SetViewport`'s own
    /// paragraph is the one place in this header that says its input is in the screen system while
    /// every other `IGraphics` input is in the world system, so the two callers transform before
    /// arriving rather than sharing one assumption about the space.
    pub(crate) fn paint_region(
        &mut self,
        st: &State,
        origin: (i32, i32),
        size: (i32, i32),
        flag: u32,
    ) {
        let (x, y) = origin;
        let (w, h) = size;
        let mut cv = self.canvas(*st);
        if flag & g::AEE_GRAPHICS_CLEAR != 0 {
            let c = st.bg.native();
            cv.fill(x, y, w, h, c);
        }
        if flag & g::AEE_GRAPHICS_FILL != 0 {
            let c = st.fill.native();
            cv.fill(x, y, w, h, c);
        }
        if flag & g::AEE_GRAPHICS_FRAME != 0 {
            let c = st.fg.native();
            cv.frame(x, y, w, h, c);
        }
    }

    /// The `ITransform` slots. `TransformBltSimple` is a blit whose source rectangle is read through a
    /// reflection or a turn first — the geometry is in [`Display::transform_in`] — and the composite
    /// mode maps exactly the way a reference front end's own shortcut maps it: `COMPOSITE_KEYCOLOR` is the
    /// transparent raster op, and the header says outright that every other value is to be treated as
    /// opaque.
    ///
    /// `TransformBltComplex` is the matrix form of the same question, and it is the one a pack uses:
    /// 1356 calls in a single run, every one of them a horizontal mirror (`A=-256, D=+256` in
    /// The header sheet. The maths is in [`Display::transform_complex_in`].
    pub(crate) fn itransform(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
    ) -> HResult<u32> {
        let Some(transform) = self.transforms.get(&this).copied() else {
            return Err(HleError::NotAnObject { addr: this });
        };
        match name {
            "TransformBltSimple" => {
                let [xd, yd, src, xs, ys, cw, ch, xform, composite]: [u32; 9] = self.args()?;
                let rop = composite_rop(composite);
                Ok(self.display.transform_in(
                    &mut self.cpu.mem,
                    transform.dst,
                    xd as i32,
                    yd as i32,
                    src,
                    xs as i32,
                    ys as i32,
                    cw as i32,
                    ch as i32,
                    xform,
                    rop,
                )?)
            }
            "TransformBltComplex" => {
                let [xd, yd, src, xs, ys, cw, ch, matrix, composite]: [u32; 9] = self.args()?;
                if matrix == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(self.display.transform_complex_in(
                    &mut self.cpu.mem,
                    transform.dst,
                    xd as i32,
                    yd as i32,
                    src,
                    xs as i32,
                    ys as i32,
                    cw as i32,
                    ch as i32,
                    matrix,
                    composite_rop(composite),
                )?)
            }
            _ => Err(HleError::Unimplemented {
                id,
                label: label.to_string(),
                this,
            }),
        }
    }

    /// A whole-screen capture as a Windows BMP file image.
    ///
    /// For the debug endpoints and the crash report, which want a file rather than a frame. Same
    /// reason as [`Runtime::capture`]: the pixels are in `display`'s device bitmap and the bytes are
    /// in `cpu.mem`, and a caller should not have to know either.
    pub fn screenshot_bmp(&self) -> crate::hle::HResult<Vec<u8>> {
        self.display.to_bmp(&self.cpu.mem)
    }

    /// A whole-screen capture as 8-bit RGB, top row first.
    pub fn screenshot_rgb(&self) -> crate::hle::HResult<Vec<u8>> {
        self.display.screenshot(&self.cpu.mem)
    }

    /// The `AEEFont` a drawing call should actually resolve to, once `IDisplay_SetFont` is taken
    /// into account.
    ///
    /// An applet that installed one of this host's system faces under a designator gets that face,
    /// which is the whole point of the call. An applet that installed a font of its *own* gets
    /// the built-in face the designator already names: this host cannot ask a module to glyph its own
    /// text, and drawing the line in the system face is what a handset with a broken private font
    /// falls back to anyway.
    ///
    /// The first thing all three bodies do is fold the designator itself: `GetFontMetrics`,
    /// `MeasureTextEx` and `DrawText` each open with
    /// `if ( (unsigned)(nFont - 0x8000) >= 8 ) nFont = 0x8000;` — a designator outside the eight
    /// system slots is **not refused**, it is answered as `AEE_FONT_NORMAL`. A made-up id therefore
    /// draws and measures, in the normal face; only `SetFont` refuses one (see
    /// [`Display::set_font`]), because that body answers `0` without storing.
    pub(crate) fn designator(&self, disp: u32, nfont: u32) -> u32 {
        let nfont = if nfont < g::AEE_FONT_NORMAL || nfont >= g::AEE_FONT_TOTAL {
            g::AEE_FONT_NORMAL
        } else {
            nfont
        };
        let installed = self.display.font_installed(disp, nfont);
        if installed == 0 {
            return nfont;
        }
        self.hle
            .object(installed)
            .and_then(|o| {
                FONT_FACES
                    .iter()
                    .find(|(cls, _)| *cls == o.clsid)
                    .map(|(_, face)| *face)
            })
            .unwrap_or(nfont)
    }

    /// The `IDisplay` slots this host backs. The gaps are deliberate and named: there is a
    /// framebuffer, a clip stack and a built-in glyph face here, but no resource loader, so
    /// `DrawFrame`, the `IDIB` creators and `GetSymbol` still stop a run rather than returning a
    /// plausible zero.
    pub(crate) fn idisplay(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `AECHAR IDisplay_GetSymbol(IDisplay *po, AEESymbol sym, AEEFont fnt)` returns "the AECHAR value
            // corresponding to the specified symbol value", and its Comments name the answer
            // for a symbol the device does not have: "If the symbol does not exist on the device a space
            // character will be returned `(AECHAR)' '`". This host's font comes from the pack's
            // own resource, so it carries no device-defined symbol glyphs: the platform defines the symbol **values**
            // (`AEE_SYM_*`, 22 of them) but the mapping from a symbol to a character is a property of the
            // device's font, and there is none here to consult. So every symbol is one that "does not exist on
            // the device", which the header already says what to do with.
            "GetSymbol" => {
                let _: [u32; 3] = self.args()?;
                Ok(' ' as u32)
            }
            "DrawRect" => {
                let [prc, frame, fill, flags] = self.args()?;
                self.display
                    .draw_rect(&mut self.cpu.mem, this, prc, frame, fill, flags)?;
                Ok(0)
            }
            "DrawFrame" => {
                let [prc, ft, fill] = self.args()?;
                self.display
                    .draw_frame(&mut self.cpu.mem, this, prc, ft as u8, fill)?;
                Ok(0)
            }
            "BitBlt" => {
                let [xd, yd, cw, ch, src, xs, ys, rop] = self.args()?;
                self.display.bit_blt(
                    &mut self.cpu.mem,
                    this,
                    xd as i32,
                    yd as i32,
                    cw as i32,
                    ch as i32,
                    src,
                    xs as i32,
                    ys as i32,
                    rop,
                )?;
                Ok(0)
            }
            "Update" => {
                let [posted] = self.args()?;
                // A page that changed since the last frame is the viewer's own debt, and this is
                // the last moment before the pixels leave: see `paint_html_views`.
                self.paint_html_views(this)?;
                self.display.update(&self.cpu.mem, this, posted)?;
                Ok(0)
            }
            "CreateDIBitmap" => {
                // `IDisplay_CreateDIBitmap(pIDisplay, IDIB **ppIDIB, uint8 colorDepth, uint16 w,
                // uint16 h)`: an off-screen bitmap, handed back as an `IDIB` because that struct — with
                // its `pBmp`, its `nPitch` and its `pRGB` — is how a pack reaches the pixels it is about
                // to draw into. A pack may ask for 8 bpp and fill it before blitting it out, so 8 bpp
                // has to be a depth this host backs — see `display::check_depth`.
                //
                // `w` and `h` pass straight through: the reference 3.1.2 wrapper is exactly
                // `DIB_New(colorDepth, w, h, 0)`, and `DIB_NewEx` keys the pixel pointer on `w * h`,
                // so both zero names an empty DIB. This host used to
                // substitute the screen size here, inferred from twins asking `8, 0, 0` — withdrawn.
                // The substitution handed six packs a `cy` they never named; they then restated
                // `nDepth`/`nPitch`/`cx`, which the delivered body documents as supported and which
                // `CreateDIBitmapEx` does itself, and `FillRect` walked that row count off the end of a
                // zero-byte pixel allocation.
                let [ppv, depth, w, h]: [u32; 4] = self.args()?;
                let addr = self.display.new_bitmap(
                    &mut self.cpu.mem,
                    &mut self.hle,
                    w as u16,
                    h as u16,
                    depth as u16,
                )?;
                self.give_object(addr, ppv)
            }
            "CreateDIBitmapEx" => {
                // `IDisplay_CreateDIBitmapEx(po, IDIB **ppIDIB, int nDepth, int nHeight, int nWidth,
                // int nPaletteSize, int cbExtra)`: the same off-screen
                // bitmap as `CreateDIBitmap`, with the palette size and the extra bytes named
                // separately instead of being implied by the depth.
                //
                // This host's `display::new_bitmap` builds the palette a depth implies, so the two
                // extra numbers are **read and said** rather than assumed: a caller that asked for a
                // palette size this host would not build learns that here instead of finding out
                // from pixels that came out wrong. `w`/`h` pass through as they do for
                // `CreateDIBitmap`: the reference Ex wrapper allocates a 32-bpp DIB scaled to hold the
                // bytes it needs and then restates `cx`/`cy`/`nDepth`/`nPitch`/`pRGB`/`cntRGB` itself,
                // with no screen-size substitution anywhere in it.
                let [ppv, depth, h, w, palette, extra]: [u32; 6] = self.args()?;
                if palette != 0 || extra != 0 {
                    self.display.note(format!(
                        "IDisplay::CreateDIBitmapEx asked for nPaletteSize={palette} and \
                         cbExtra={extra} at {w}x{h} depth {depth}; this host builds the palette a \
                         depth implies"
                    ));
                }
                let addr = self.display.new_bitmap(
                    &mut self.cpu.mem,
                    &mut self.hle,
                    w as u16,
                    h as u16,
                    depth as u16,
                )?;
                self.give_object(addr, ppv)
            }
            "SetAnnunciators" => {
                let [val, mask] = self.args()?;
                self.display
                    .set_annunciators(this, val as u16, mask as u16)?;
                Ok(0)
            }
            "Backlight" => {
                let [on] = self.args()?;
                self.display.set_backlight(this, on)?;
                Ok(0)
            }
            "SetColor" => {
                let [item, rgb] = self.args()?;
                Ok(self.display.set_color(this, item, rgb)?)
            }
            "SetDestination" => {
                let [dst] = self.args()?;
                Ok(self
                    .display
                    .set_destination(&mut self.cpu.mem, &mut self.hle, this, dst)?)
            }
            "GetDestination" => {
                // Returns the bitmap by value rather than through an out-parameter, and the header
                // is explicit that it adds a reference: "The reference count of the bitmap will be
                // incremented." Handing it back bare would let the caller's eventual release eat
                // the *display's* own reference to it.
                let obj = self.display.destination(this)?;
                self.hle.change_refs(&mut self.cpu.mem, obj, 1)?;
                Ok(obj)
            }
            "GetDeviceBitmap" => {
                // The screen is a real `IBitmap*`, so this is the one place a module can reach the
                // framebuffer through the object rather than by name.
                let [ppv] = self.args()?;
                let obj = self.display.device_bitmap();
                self.return_object(obj, ppv)
            }
            "SetClipRect" => {
                let [prc] = self.args()?;
                self.display.set_clip_rect(&self.cpu.mem, this, prc)?;
                Ok(0)
            }
            "GetClipRect" => {
                let [prc] = self.args()?;
                self.display.get_clip_rect(&mut self.cpu.mem, this, prc)?;
                Ok(0)
            }
            "Clone" => {
                let [ppv] = self.args()?;
                let addr = self
                    .display
                    .clone_display(&mut self.cpu.mem, &mut self.hle, this)?;
                self.give_object(addr, ppv)
            }
            "MakeDefault" => {
                // The body ends by handing back its own new reference count, and `IDisplay_SetFont`
                // and `IDisplay_GetDestination` are the two other slots on this interface whose answer
                // is a value the `void`/`IBitmap*` headers do not make obvious.
                self.display
                    .make_default(&mut self.cpu.mem, &mut self.hle, this)
            }
            "GetFontMetrics" => {
                let [nfont, pn_ascent, pn_descent] = self.args()?;
                let nfont = self.designator(this, nfont);
                self.display
                    .get_font_metrics(&mut self.cpu.mem, this, nfont, pn_ascent, pn_descent)
            }
            "SetFont" => {
                let [nfont, pfont] = self.args()?;
                self.display.set_font(this, nfont, pfont)
            }
            "MeasureTextEx" => {
                let [nfont, text, nchars, max_width, pn_fits] = self.args()?;
                let nfont = self.designator(this, nfont);
                self.display.measure_text(
                    &mut self.cpu.mem,
                    this,
                    nfont,
                    text,
                    nchars as i32,
                    max_width as i32,
                    pn_fits,
                )
            }
            "SetPrefs" => {
                // `IDISPLAY_SetPrefs(pi, pchSettings, cSettings)` hands the list to the device
                // bitmap's own `IBitmapDev2_SetPrefs`, which is where the four names are defined:
                // `w` screen width, `h` screen height, `c` colour scheme, `a` annunciators.
                //
                // The height is the one a guest pack actually uses, and the reason this slot had
                // to be served rather than refused: a pack whose frame is laid out for a taller
                // panel asks for that height, and then clears the frame it believes it was given.
                // Refusing while a pack does not check the answer left it writing past the screen
                // the host had made. The string is still recorded, because which names a pack sends
                // is how this stays honest about the ones still unserved.
                let [settings, count] = self.args()?;
                let cap = if count as i32 <= 0 {
                    256
                } else {
                    (count as usize).min(256)
                };
                let mut bytes = Vec::with_capacity(cap);
                for i in 0..cap as u32 {
                    let b = self.cpu.mem.read_u8(settings + i)?;
                    if b == 0 {
                        break;
                    }
                    bytes.push(b);
                }
                let asked = crate::encoding::stream_text(u32::from(self.device.encoding), &bytes);
                if !self.display_prefs.contains(&asked) {
                    self.display_prefs.push(asked.clone());
                }
                self.display
                    .set_prefs(&mut self.cpu.mem, &mut self.hle, &asked)
            }
            "DrawText" => {
                let [nfont, text, nchars, x, y, prc, flags] = self.args()?;
                let nfont = self.designator(this, nfont);
                self.display.draw_text(
                    &mut self.cpu.mem,
                    this,
                    nfont,
                    text,
                    nchars as i32,
                    x as i32,
                    y as i32,
                    prc,
                    flags,
                )
            }
            "IsEnabled" => Ok(self.display.is_enabled(this)?),
            // `IDisplay::NotifyEnable(pcb)` is the same delivered body as
            // `IBitmapDev::NotifyEnable` — `IDisplay`'s vtable begins with `IBitmapDev`'s, and this is
            // slot 5 of it — so it answers the same two things and registers nothing. Read it in
            // [`Runtime::ibitmapdev`], where the documented behaviour and the inspection that shows the
            // forward carrying `pcb` in r1 are written out: `AEE_EBADPARM` for a null callback,
            // `AEE_EUNSUPPORTED` for a real one, because this host has no device object at record
            // `+124` to hand it to.
            "NotifyEnable" => {
                let [pcb] = self.args()?;
                if pcb == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(g::AEE_EUNSUPPORTED)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Where in its clip a player stands, in virtual milliseconds — the one reading of the two shapes
    /// this host keeps a position in.
    ///
    /// An armed pass knows where it is from its own window, and says nothing new in `position`; a
    /// disarmed one (paused, or seeked while stopped) has only `position` to be read from. Deriving
    /// both here rather than at each call site is what keeps a `MM_SEEK_CURRENT` seek, a `Pause`'s
    /// elapsed, and a `GetTotalTime`-adjacent report from disagreeing with each other.
    pub(crate) fn clip_position(&self, player: &Player) -> u32 {
        if player.pass_at == 0 {
            // Nothing is armed, so the stored place is the whole answer.
            player.position
        } else {
            media_position_at(player, self.clock_ms, self.media_tempo, player.rate_parts())
        }
    }

    /// `IBitmapRot` — the rotation face of a bitmap, `AEEIID_BITMAPROT` (`0x0103e8d9`).
    ///
    /// One slot: "`int IBITMAPROT_GetRotation(IBitmapRot *po)` … This function gets the current rotation of
    /// the bitmap relative to the default. Return Value: The number of degrees that the device bitmap has
    /// been rotated relative to …". This host's device bitmap is never
    /// rotated, so the number is `0` — and that is the answer the slot exists to give, not a placeholder:
    /// a caller that lays out an unrotated screen is correct here.
    pub(crate) fn ibitmaprot(
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
                self.answer_qi(this, iid, ppv, &[g::AEEIID_BITMAPROT])
            }
            // The provenance this face was given at `QueryInterface` time is read here rather than assumed,
            // so a face whose bitmap has gone is a named object rather than a silent zero.
            "GetRotation" => {
                if !self.rot_of.contains_key(&this) {
                    return Err(HleError::NotAnObject { addr: this });
                }
                Ok(0)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IBitmapDev` — the device-bitmap face of a bitmap, `AEEIID_IBitmapDev` (`0x01012719`).
    ///
    /// Three slots, all documented:
    ///
    /// * `Update` — the documented behaviour decides, and its last line is the answer here.
    ///   `OEMBitmap_Update` pushes the dirty rectangle
    ///   through `IDISPLAYDEV_Update` when the bitmap has a display device, and otherwise merges it
    ///   into a parent with `IBITMAP_Invalidate` + `IBITMAPDEV_Update(m_pParentDev)`; a bitmap with
    ///   **neither** falls to `return SUCCESS` having copied nothing. This host exports the face on
    ///   one object — the screen bitmap, which is what the display presents, so it has no device to
    ///   push to and no parent to merge into — and that fall-through is therefore this slot's whole
    ///   behaviour, not a shortcut around it. One layer out the reference wrapper agrees:
    ///   `PWBitmapDev_Update` is twelve bytes that load the wrapped
    ///   object's own slot 3 and tail-call it, keeping its return. `Update` takes **no** `bDefer`
    ///   (`spec` slot 3 with one parameter); that argument belongs to
    ///   `IDisplay::Update`, where it selects `AEE_ResumeCallback` over a synchronous wake.
    /// * `IsEnabled` — "TRUE: if this bitmap is enabled. FALSE: otherwise". The device bitmap
    ///   of a display that is always enabled is enabled.
    /// * `NotifyEnable` — the one refusal the header names, "`EBADPARM`: if `pcb` is NULL", and
    ///   otherwise a registration ("It is desirable to reregister for this notification in the callback
    ///   function …"). The bitmap neither changes state nor goes away here, so it is kept and
    ///   never fired.
    pub(crate) fn ibitmapdev(
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
                self.answer_qi(this, iid, ppv, &[g::AEEIID_IBitmapDev])
            }
            "Update" => {
                // "copies the bitmap to its associated display synchronously". The face was handed
                // out for the device bitmap, but a bitmap no display is showing has nothing to copy
                // to — that is the case the body falls through on. See `Runtime::dev_of`.
                let source = self.dev_of.get(&this).copied();
                if source == Some(self.display.device_bitmap()) {
                    self.display.present_device_bitmap();
                }
                Ok(g::AEE_SUCCESS)
            }
            "IsEnabled" => Ok(g::TRUE),
            "NotifyEnable" => {
                let [pcb] = self.args()?;
                if pcb == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(g::AEE_EUNSUPPORTED)
            }
            // `IBitmapDev2::SetPrefs(const char *pchSettings, int cSettings)` — the one slot that face
            // adds to this one, and "It is used to implement `IDisplay_SetPrefs()`",
            // so the two faces are one machine and give one answer.
            // The header publishes both the grammar (`w`/`h`/`c`/`a` as `name:value` pairs)
            // and the code this host's state deserves: "`AEE_EUNSUPPORTED` if this bitmap does not
            // support preferences". Neither face has behaviour for any of the four names —
            // the device configuration is fixed when a run starts — so the string is kept in a note
            // instead, for the reason `IDisplay::SetPrefs` gives: which names a guest pack sends is
            // how any of the four would earn real behaviour.
            "SetPrefs" => {
                let [settings, count] = self.args()?;
                let mut bytes = Vec::new();
                for i in 0..(count.max(0) as u32).min(256) {
                    let b = self.cpu.mem.read_u8(settings + i)?;
                    if b == 0 {
                        break;
                    }
                    bytes.push(b);
                }
                self.display.note(format!(
                    "IBitmapDev2::SetPrefs({:?}) - this host has no behaviour for w/h/c/a, and the \
                     header has a word for that: EUNSUPPORTED",
                    crate::encoding::stream_text(u32::from(self.device.encoding), &bytes)
                ));
                Ok(g::AEE_EUNSUPPORTED)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IBitmapScale`'s one slot. The interface is a **service** — a scaler taking a source and a
    /// destination bitmap with their own rectangles, a clip, and a transparency operation
    /// (`AEEBITMAPSCALE_OPAQUE`/`ALPHA`/`KEYCOLOR`, the header) — and this
    /// host publishes no object under that name: no class id for it exists anywhere in this platform, so
    /// `CreateInstance` has nothing to hand out and the slot is reachable by id only, the position
    /// `ICharsetConv`, `IVersion` and `IBearerTechnology` are also in.
    ///
    /// ⚠ **The refusal is about a service this host does not offer, not about something it cannot
    /// do.** Its own drawing primitives do not resample, so there is no scaler behind this name;
    /// `EUNSUPPORTED` is the true statement about that, and it is also the one that keeps a pack from
    /// ending its run at `unimplemented`. A pack that needed scaling would be the reason to write a
    /// real one — but inventing it now would answer a question no caller here has asked.
    pub(crate) fn ibitmapscale(
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
                self.answer_qi(this, iid, ppv, &[g::AEEIID_IBitmapScale])
            }
            "Scale" => Ok(g::AEE_EUNSUPPORTED),
            _ => Err(unimplemented()),
        }
    }

    /// The sprite engine's six slots. the delivered body is the shape of every one of them, and this
    /// object does hold state, so the door reads the engine out of [`Runtime::sprites`],
    /// edits a copy, and puts it back before anything that can fail runs — which is what makes the
    /// reference bookkeeping and the stored buffer agree whatever the blit does next.
    pub(crate) fn isprite(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let gap = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let mut engine = self.sprites.engine(this).ok_or_else(|| gap())?;
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                if iid == g::AEECLSID_SPRITE || iid == g::AEEIID_IQI {
                    return self.answer_qi(this, iid, ppv, &[g::AEECLSID_SPRITE]);
                }
                //: the caller's pointer is written **before** the refusal, and the code
                // this class answers with is `EUNSUPPORTED`, not the `ECLASSNOTSUPPORT` most
                // `QueryInterface` bodies use.
                if ppv != 0 {
                    self.cpu.mem.write_u32(ppv, 0)?;
                }
                Ok(g::AEE_EUNSUPPORTED)
            }
            // stores the table as given, with no test of any kind. A null table is a
            // legal call, and the header is explicit that the object neither copies it
            // nor ever frees it — which is why this is a guest address and not a host allocation.
            "SetTransformTable" => {
                let [table] = self.args()?;
                engine.matrices = table;
                self.sprites.put(this, engine);
                Ok(g::AEE_SUCCESS)
            }
            // and are the same eleven lines twice: outside the four size codes
            // nothing is touched and the answer is `EBADPARM`; inside them the buffer held for that
            // code is released, the new one is addrefed, and it is stored — **including a null one**,
            // which is a legal way to empty a slot. The reference is the half a pack can see: art it
            // handed over and then released itself stays alive for the engine's use.
            "SetSpriteBuffer" | "SetTileBuffer" => {
                let [which, buf] = self.args()?;
                // The published parameter is a `uint8`, so only the low byte of the register counts.
                let slot = usize::from((which & 0xff) as u8);
                let sprites = name == "SetSpriteBuffer";
                let slots = if sprites {
                    &mut engine.sprite_bufs[..]
                } else {
                    &mut engine.tile_bufs[..]
                };
                let Some(prev) = slots.get_mut(slot).map(|at| std::mem::replace(at, buf)) else {
                    return Ok(g::AEE_EBADPARM);
                };
                self.sprites.put(this, engine);
                self.hold(prev, -1)?;
                self.hold(buf, 1)?;
                Ok(g::AEE_SUCCESS)
            }
            // answers `SUCCESS` whatever it was given, including null. The documented behaviour
            // addrefs the new destination *before* asking it for an `ITransform` and only
            // lets the old pair go afterwards, and the query's answer decides which of
            // the object's two bodies the draw slots reach (). A destination this host
            // made answers the query the way `IBitmap`'s own `QueryInterface` answers it for a pack; a
            // bitmap a module built for itself answers nothing, which is the reference route to the
            // simple bodies rather than an error.
            "SetDestination" => {
                let [dst] = self.args()?;
                let prev = std::mem::replace(&mut engine.dst, dst);
                let face = if dst == 0 {
                    0
                } else {
                    self.transform_face(dst)?
                };
                let prev_face = std::mem::replace(&mut engine.transform, face);
                self.sprites.put(this, engine);
                self.hold(dst, 1)?;
                self.hold(prev, -1)?;
                self.hold(prev_face, -1)?;
                Ok(g::AEE_SUCCESS)
            }
            "DrawSprites" => {
                let [cmds] = self.args()?;
                if cmds == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                //, before a single command is read: no destination is `EFAILED`, not
                // `EBADPARM`, because the argument that is wrong is the engine's own state. The
                // transform body answers the same code for the same reason at.
                if engine.dst == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                let list = self.sprite_cmds(cmds)?;
                let (blits, bad) =
                    sprite::plan_sprites(&list, &engine.sprite_bufs, engine.matrices);
                for b in blits {
                    let rop = composite_rop(u32::from(b.composite));
                    if engine.transform == 0 {
                        // The simple body reads neither `unTransform` nor the table.
                        self.display.blt_in(
                            &mut self.cpu.mem,
                            engine.dst,
                            b.x,
                            b.y,
                            b.w as i32,
                            b.h as i32,
                            b.src,
                            b.xs as i32,
                            b.ys as i32,
                            rop,
                        )?;
                    } else if b.matrix != 0 {
                        self.display.transform_complex_in(
                            &mut self.cpu.mem,
                            engine.dst,
                            b.x,
                            b.y,
                            b.src,
                            b.xs as i32,
                            b.ys as i32,
                            b.w as i32,
                            b.h as i32,
                            b.matrix,
                            rop,
                        )?;
                    } else {
                        self.display.transform_in(
                            &mut self.cpu.mem,
                            engine.dst,
                            b.x,
                            b.y,
                            b.src,
                            b.xs as i32,
                            b.ys as i32,
                            b.w as i32,
                            b.h as i32,
                            u32::from(b.transform),
                            rop,
                        )?;
                    }
                }
                // The walk already happened, so the pixels are where they are: a list with one bad
                // size in it draws every other sprite and then reports `EBADPARM`.
                Ok(if bad { g::AEE_EBADPARM } else { g::AEE_SUCCESS })
            }
            "DrawTiles" => {
                let [maps] = self.args()?;
                if maps == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if engine.dst == 0 {
                    return Ok(g::AEE_EFAILED);
                }
                //: the destination's own extent, read through `IBITMAP_GetInfo`, is what bounds
                // the walk — a map larger than the screen still contributes exactly the tiles that
                // reach it.
                let (cx, cy) = self
                    .display
                    .bitmap(engine.dst)
                    .map(|b| (u32::from(b.width), u32::from(b.height)))
                    .ok_or_else(|| gap())?;
                let stride = abi::struct_info("AEETileMap").ok_or_else(|| gap())?.size;
                let mut at = maps;
                let mut bad = false;
                for _ in 0..TILE_MAP_CAP {
                    let map = self.tile_map(at)?;
                    if map.array == 0 {
                        return Ok(if bad { g::AEE_EBADPARM } else { g::AEE_SUCCESS });
                    }
                    if map.w > g::MAP_SIZE_1024
                        || map.h > g::MAP_SIZE_1024
                        || usize::from(map.size) >= engine.tile_bufs.len()
                    {
                        //: this one map is skipped, the array is not, and the result is
                        // the same sticky `EBADPARM` the sprite walk uses.
                        bad = true;
                        at = at.wrapping_add(stride);
                        continue;
                    }
                    let edge = sprite::SIZE_PX[usize::from(map.size)];
                    let buf = engine.tile_bufs[usize::from(map.size)];
                    for slot in sprite::tile_slots(&map, cx, cy) {
                        let word = self.cpu.mem.read_u16(
                            map.array
                                .wrapping_add(u32::try_from(slot.at).unwrap_or(0) * 2),
                        )?;
                        if word & g::TILE_INDEX_MASK == g::TILE_INDEX_NOTHING {
                            continue;
                        }
                        // Bits 15-13 of a tile word are the same three fields as
                        // `AEESpriteCmd::unTransform`'s bits 2-0 (and `TILE_FLIP_X`
                        // is literally `TRANSFORM_FLIP_X << TILE_TRANFORM_SHIFT` at), so the
                        // transform body just shifts them down and hands them over with the tile's own
                        // transparency turned into a composite code (). The
                        // simple body reaches for neither.
                        let rop = composite_rop(if word & g::TILE_TRANSPARENT != 0 {
                            g::COMPOSITE_KEYCOLOR
                        } else {
                            g::COMPOSITE_OPAQUE
                        });
                        let ys = (edge * (word & g::TILE_INDEX_MASK)) as i32;
                        if engine.transform == 0 {
                            self.display.blt_in(
                                &mut self.cpu.mem,
                                engine.dst,
                                slot.x,
                                slot.y,
                                edge as i32,
                                edge as i32,
                                buf,
                                0,
                                ys,
                                rop,
                            )?;
                        } else {
                            self.display.transform_in(
                                &mut self.cpu.mem,
                                engine.dst,
                                slot.x,
                                slot.y,
                                buf,
                                0,
                                ys,
                                edge as i32,
                                edge as i32,
                                (word >> g::TILE_TRANFORM_SHIFT) as u32,
                                rop,
                            )?;
                        }
                    }
                    at = at.wrapping_add(stride);
                }
                Ok(if bad { g::AEE_EBADPARM } else { g::AEE_SUCCESS })
            }
            _ => Err(gap()),
        }
    }

    /// The `ITransform` taken out of a destination bitmap, which is the same face a pack gets by
    /// asking `IBitmap::QueryInterface` for `AEECLSID_TRANSFORM` — the query the delivered body runs
    /// before it decides which pair of draw bodies to install (). A bitmap this host
    /// never made has no face to hand over, and that is the reference route to the simple bodies rather
    /// than an error.
    pub(crate) fn transform_face(&mut self, bitmap: u32) -> HResult<u32> {
        if self
            .hle
            .object(bitmap)
            .map_or(true, |o| o.iface != "IBitmap")
        {
            return Ok(0);
        }
        let addr = self.hle.new_object(
            &mut self.cpu.mem,
            "ITransform",
            g::AEECLSID_TRANSFORM,
            8,
            4,
            &[],
        )?;
        self.transforms.insert(addr, Transform { dst: bitmap });
        Ok(addr)
    }

    /// The caller's `AEESpriteCmd` list, read until its `SPRITE_SIZE_END` terminator.
    ///
    /// The cap is the one thing added: an unterminated list is a runaway walk on a handset as much
    /// as here, and a host that hangs cannot say what it was shown.
    pub(crate) fn sprite_cmds(&mut self, at: u32) -> HResult<Vec<sprite::Cmd>> {
        let off = |field: &str| -> HResult<u32> {
            abi::offset_of("AEESpriteCmd", field).ok_or_else(|| HleError::NotAnObject { addr: at })
        };
        let (o_x, o_y, o_t, o_m, o_i, o_s, o_c, o_l) = (
            off("x")?,
            off("y")?,
            off("unTransform")?,
            off("unMatrixTransform")?,
            off("unSpriteIndex")?,
            off("unSpriteSize")?,
            off("unComposite")?,
            off("unLayer")?,
        );
        let stride = abi::struct_info("AEESpriteCmd")
            .ok_or(HleError::NotAnObject { addr: at })?
            .size;
        let mut out = Vec::new();
        let mut at = at;
        for _ in 0..SPRITE_CMD_CAP {
            let size = self.cpu.mem.read_u8(at + o_s)?;
            if size == g::SPRITE_SIZE_END as u8 {
                break;
            }
            out.push(sprite::Cmd {
                x: i32::from(self.cpu.mem.read_u16(at + o_x)? as i16),
                y: i32::from(self.cpu.mem.read_u16(at + o_y)? as i16),
                transform: self.cpu.mem.read_u16(at + o_t)? as u16,
                matrix: self.cpu.mem.read_u8(at + o_m)?,
                index: self.cpu.mem.read_u8(at + o_i)?,
                size,
                composite: self.cpu.mem.read_u8(at + o_c)?,
                layer: self.cpu.mem.read_u8(at + o_l)?,
            });
            at = at.wrapping_add(stride);
        }
        Ok(out)
    }

    /// One `AEETileMap` of the caller's array of them.
    pub(crate) fn tile_map(&mut self, at: u32) -> HResult<sprite::Map> {
        let off = |field: &str| -> HResult<u32> {
            abi::offset_of("AEETileMap", field).ok_or(HleError::NotAnObject { addr: at })
        };
        let (o_array, o_flags, o_x, o_y, o_w, o_h, o_size) = (
            off("pMapArray")?,
            off("unFlags")?,
            off("x")?,
            off("y")?,
            off("w")?,
            off("h")?,
            off("unTileSize")?,
        );
        Ok(sprite::Map {
            array: self.cpu.mem.read_u32(at + o_array)?,
            flags: self.cpu.mem.read_u32(at + o_flags)?,
            x: self.cpu.mem.read_u32(at + o_x)? as i32,
            y: self.cpu.mem.read_u32(at + o_y)? as i32,
            w: u32::from(self.cpu.mem.read_u16(at + o_w)?),
            h: u32::from(self.cpu.mem.read_u16(at + o_h)?),
            size: self.cpu.mem.read_u8(at + o_size)?,
        })
    }

    /// The sixteen bytes `aee_memmove(a1 + 72, a2, 16)` takes from the caller's `AEEClip` — and the
    /// struct is only twelve, so the last four are whatever sits behind it in the guest heap. This host
    /// copies what is actually mapped and leaves the rest as it already stood, rather than faulting on
    /// a pack that allocated exactly the published size.
    pub(crate) fn clip_bytes(&self, addr: u32) -> HResult<[u8; 16]> {
        let room = (self.cpu.mem.mapped_run(addr, 16) as usize).min(16);
        let mut out = [0u8; 16];
        if room > 0 {
            out[..room].copy_from_slice(&self.cpu.mem.read_bytes(addr, room)?);
        }
        Ok(out)
    }

    /// `IBitFontFoundry::CreateFromBBF(pbyMem, cbMem, pvStorage, ppIFont)`.
    ///
    /// The delivered body hands the bytes straight to `AEEBitFont_NewFromBBF` with no allocator:
    /// `pvStorage` only says whether the object may keep the caller's pointer
    /// (`AEEBITFONT_STATICDATA`) or must copy (`AEEBITFONT_COPYDATA`). This host always copies, so
    /// both values are accepted and the caller may release the file as soon as the call returns.
    pub(crate) fn foundry_create_from_bbf(&mut self) -> HResult<u32> {
        let [pby, cb, _storage, ppif] = self.args()?;
        if ppif == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        self.cpu.mem.write_u32(ppif, 0)?;
        if pby == 0 {
            return Ok(g::AEE_EBADPARM);
        }
        if (cb as i32) < bbf::HEADER_LEN as i32 {
            return Ok(g::AEE_EINVALIDFORMAT);
        }
        // A font is loaded once, but the bound keeps a wrong length from asking the guest arena
        // for an address it never mapped.
        let len = (cb as usize).min(64 << 20);
        let bytes = match self.cpu.mem.read_bytes(pby, len) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(g::AEE_EBADPARM),
        };
        let face = match bbf::Bbf::parse(&bytes) {
            Ok(face) => Arc::new(face),
            Err(bbf::Error::UnsupportedVersion) => return Ok(g::AEE_EUNSUPPORTED),
            Err(_) => return Ok(g::AEE_EINVALIDFORMAT),
        };
        let obj = self
            .hle
            .new_object(&mut self.cpu.mem, "IFont", g::AEECLSID_FONT, 8, 4, &[])?;
        self.foundry_fonts.insert(obj, font::Face::Bbf(face));
        self.cpu.mem.write_u32(ppif, obj)?;
        Ok(g::AEE_SUCCESS)
    }

    /// Every `IFont` slot. The object is one of this host's system faces, named by the class id it
    /// was created under, and every answer comes out of the one face table the display already
    /// measures and draws with — this is a second door to it, not a second font.
    pub(crate) fn ifont(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let resolved = if let Some(face) = self.foundry_fonts.get(&this) {
            Some(face.clone())
        } else {
            let cls = self.hle.object(this).map(|object| object.clsid);
            cls.and_then(|cls| self.font_class_faces.get(&cls).cloned())
                .or_else(|| {
                    cls.and_then(|cls| {
                        FONT_FACES
                            .iter()
                            .find(|(known, _)| *known == cls)
                            .map(|(_, designator)| *designator)
                    })
                    .and_then(|designator| self.display.face(designator))
                })
        };
        let Some(font) = resolved else {
            return Err(unimplemented());
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEEIID_IFont])
            }
            // `IFont::GetInfo(pinfo, nSize)` — the two numbers a layout needs before it can place a
            // line. The header publishes the struct as two `int16`s, and both platform routines
            // compare the caller's `sizeof` for **equality**:
            // `if (sizeof(*pInfo) != nSize) return EUNSUPPORTED;` (
            // The delivered body
            // half-filled — answering SUCCESS there would hand back a record whose other fields the
            // reference member never wrote.
            "GetInfo" => {
                let [pinfo, nsize] = self.args()?;
                let info = abi::struct_info("AEEFontInfo").ok_or_else(unimplemented)?;
                if (nsize as i32) != info.size as i32 {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                if pinfo == 0 {
                    // The documented behaviour has no pointer check at all — it copies into whatever address it
                    // was handed — so this refusal is the host's own line, not the ABI's.
                    return Ok(g::AEE_EBADPARM);
                }
                let ascent = abi::offset_of("AEEFontInfo", "nAscent").ok_or_else(unimplemented)?;
                let descent =
                    abi::offset_of("AEEFontInfo", "nDescent").ok_or_else(unimplemented)?;
                self.cpu.mem.write_u16(pinfo + ascent, font.ascent())?;
                self.cpu.mem.write_u16(pinfo + descent, font.descent())?;
                Ok(g::AEE_SUCCESS)
            }
            // `IFont::DrawText(pDst, x, y, pcText, nChars, fg, bg, prcClip, dwFlags)` — ten arguments, so
            // most of them arrive on the guest's stack; `self.args()` reads them by the calling
            // convention rather than by anything this arm has to know. Note what the header says a caller
            // may pass and a host must **not** interpret: `x`/`y` are the top-left of the left-most cell
            // even when they are negative, the colours are already `NativeColor`s,
            //and `prcClip` is the caller's own rectangle.
            "DrawText" => {
                let [dst, x, y, text, nchars, fg, bg, prc, flags] = self.args()?;
                self.display.draw_text_into(
                    &mut self.cpu.mem,
                    &font,
                    dst,
                    text,
                    nchars as i32,
                    x as i32,
                    y as i32,
                    fg,
                    bg,
                    prc,
                    flags,
                )
            }
            // `IFont::MeasureText(pcText, nChars, nMaxWidth, pnFits, pnPixels)`. The header is
            // explicit that this is *not* `IDisplay_MeasureText`: every parameter must be valid and
            // a zero width means zero pixels rather than "unlimited", so only `IFONT_MAXWIDTH` is
            // taken as the whole string.
            "MeasureText" => {
                let [text, nchars, max_width, pn_fits, pn_pixels] = self.args()?;
                if pn_fits == 0 || pn_pixels == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let width = if max_width == g::IFONT_MAXWIDTH {
                    i32::MAX
                } else {
                    max_width as i32
                };
                if width < 0 {
                    // The reference measuring loop has no sign test — the delivered body accumulates
                    // a width and asks `if (wid > nMaxWidth)`, so a negative budget is exceeded by the
                    // first character, steps back off it, and stops: `pnFits = 0`, `pnPixels = 0`, and
                    // still `SUCCESS`. The header gives `0` the same reading ("zero means zero").
                    // `IDisplay::MeasureTextEx` means the **opposite** by the same word — the header
                    // is `MeasureTextEx(p, f, psz, -1, -1, NULL)` for "measure the whole string" — which is
                    // why this branch sits on this door rather than in the shared measuring loop.
                    self.cpu.mem.write_u32(pn_fits, 0)?;
                    self.cpu.mem.write_u32(pn_pixels, 0)?;
                    return Ok(g::AEE_SUCCESS);
                }
                // No display object: the documented behaviour measures out of the font's own metrics
                // (the delivered body never read a display), so a pack
                // that asks a font before it has asked for the display is answered, not refused.
                let enc = self.text_encoding();
                let pixels = Display::measure_face(
                    &mut self.cpu.mem,
                    &font,
                    text,
                    nchars as i32,
                    width,
                    pn_fits,
                    enc,
                )?;
                self.cpu.mem.write_u32(pn_pixels, pixels)?;
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Pull from the inner source until `wanted` bytes are buffered, the way `IGetLine_Peek`'s loop
    /// does. Answers `(available, code)` — the two numbers the body writes into the caller's
    /// parameters, with `code` one of `ISOURCE_{END,ERROR,WAIT}`/`IPEEK_{FULL,UNK}`.
    pub(crate) fn line_fill(&mut self, this: u32, wanted: u32) -> Result<(u32, i32), Error> {
        let Some(record) = self.line_sources.get(&this).cloned() else {
            return Ok((0, g::ISOURCE_END as i32));
        };
        // `size - 1` is the body's capacity field: one byte is kept out of the count so a line can
        // always be terminated in place.
        let cap = record.size.saturating_sub(1);
        let read = self.guest_slot(record.source, "ISource", "Read")?;
        let mut start = record.start;
        let mut end = record.end;
        let mut limit = record.limit;
        let mut have = end.saturating_sub(start);
        // The body starts from `IPEEK_UNK` and only overwrites it when it tries a read or runs out of
        // room, so a caller that already had enough buffered gets `-4` back rather than an end-of-file.
        let mut code = g::IPEEK_UNK as i32;
        while have < wanted && have < cap {
            // The body compacts only once the reader has moved and less than 550 bytes of room are
            // left — the constant that makes a long-lived reader keep its head instead of memmoving
            // on every refill.
            if start > 0 && end + 550 > cap {
                let bytes = self
                    .cpu
                    .mem
                    .read_bytes(record.buffer + start, have as usize)?;
                self.cpu.mem.write_bytes(record.buffer, &bytes)?;
                start = 0;
                end = have;
            }
            let mut want = cap - end;
            if limit == 0 {
                // The body reads the limit first and skips the read entirely at zero, which falls out
                // of the loop as "the source gave nothing" — `ISOURCE_END`.
                code = g::ISOURCE_END as i32;
                break;
            }
            if (limit as u32) < want {
                want = limit as u32;
            }
            let got = self.call_guest(read, &[record.source, record.buffer + end, want])? as i32;
            if limit != -1 && got > 0 {
                limit -= got;
            }
            if got <= 0 {
                code = got;
                break;
            }
            end += got as u32;
            have += got as u32;
        }
        if have >= cap {
            code = g::IPEEK_FULL as i32;
        }
        if let Some(record) = self.line_sources.get_mut(&this) {
            record.start = start;
            record.end = end;
            record.limit = limit;
        }
        Ok((have, code))
    }

    /// Every `IBitmap` slot. A host bitmap is a rectangle of guest memory plus a depth, which is
    /// the whole model, so unlike `IDisplay` this interface has no gaps.
    pub(crate) fn ibitmap(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        // Every method here works on the bitmap the app holds, and an `IDIB`'s geometry is public data
        // the app is entitled to change — so read it back before answering, rather than drawing with the
        // size and buffer this host picked when it created the object.
        self.display.sync_bitmap(&self.cpu.mem, this)?;
        match name {
            "QueryInterface" => {
                // `IBitmap` inherits `IQI`, unlike `IShell` — and its `IId` is the same number as
                // its class id, so one comparison answers both.
                let [iid, ppv] = self.args()?;
                if iid == g::AEECLSID_DIB || iid == g::AEEIID_DIB_20 {
                    // The header IBitmap, so an IDIB may be used as an IBitmap by type casting", and supplies
                    // `IDIB_to_IBitmap` to do it. The same pointer therefore answers both names —
                    // which is also how a pack gets at the `pBmp` and `nPitch` it then fills itself.
                    //
                    // The second id is the 2.x numbering of the same thing, and a pack built against
                    // an older shell asks with it: a pack takes the screen bitmap out of
                    // `IDisplay::GetDestination`, asks it for `AEEIID_DIB_20`, and stores whatever
                    // arrives into a field it later loads a vtable from without checking. Refusing
                    // the older spelling of an id this host already answers is not a conservative
                    // answer — it is the null that pack then dereferences.
                    return self.return_object(this, ppv);
                }
                if iid == g::AEECLSID_TRANSFORM {
                    // A handset's OEM bitmap carries a second base inside itself for this, so the
                    // interface the caller gets back is a different pointer that draws *into* the
                    // bitmap it came from. A pack asks for it on the screen and then calls whatever
                    // answer arrived without checking, so refusing is a jump to address zero.
                    let addr =
                        self.hle
                            .new_object(&mut self.cpu.mem, "ITransform", iid, 8, 4, &[])?;
                    self.transforms.insert(addr, Transform { dst: this });
                    return self.give_object(addr, ppv);
                }
                // `AEEIID_IBitmapDev` is answered **only for the device bitmap**, which is what that header's
                // first line requires: "This interface is used for various functions relevant to device
                // bitmaps. It is only implemented for device bitmaps and is obtained by calling a bitmap's
                // `QueryInterface` method with an interface ID of `AEEIID_IBitmapDev`".
                // This host has one — the screen bitmap, which `Display`'s own
                // field calls the device bitmap — and an offscreen `IDIB` asked for this gets the same
                // `ECLASSNOTSUPPORT` any unknown id gets. That is the point of the sentence: `Update` means
                // "copy the bitmap to its **associated display**", and an offscreen bitmap has none.
                if iid == g::AEEIID_IBitmapDev && this == self.display.device.obj {
                    let addr =
                        self.hle
                            .new_object(&mut self.cpu.mem, "IBitmapDev", iid, 8, 4, &[])?;
                    // Which bitmap this face belongs to, so `Update` can tell the screen from a
                    // bitmap no display is showing. See `Runtime::dev_of`.
                    self.dev_of.insert(addr, this);
                    return self.give_object(addr, ppv);
                }
                if iid == g::AEEIID_BITMAPROT {
                    // "This interface is used to query the rotation of bitmaps. It is **obtained by calling a
                    // bitmap's QueryInterface method with an interface ID of `AEEIID_BITMAPROT`**"
                    // — so this is a face of the bitmap, reached the same way
                    // `AEECLSID_TRANSFORM`'s is: a second object with its own vtable, whose slots answer about
                    // the bitmap it came from. For a rotation it does not have to point back at all: this host
                    // never rotates a bitmap, which is exactly what `GetRotation`'s answer says.
                    let addr =
                        self.hle
                            .new_object(&mut self.cpu.mem, "IBitmapRot", iid, 8, 4, &[])?;
                    self.rot_of.insert(addr, this);
                    return self.give_object(addr, ppv);
                }
                if iid == g::AEECLSID_AEEDIB {
                    // The delivered body
                    // out"). The documented behaviour answers it with the address of `pPaletteMap` and takes no
                    // reference — a pointer that is not the `IDIB` any caller could walk. This host
                    // hands back the bitmap itself, with the reference every other answer carries; the
                    // reference interior pointer is not something a pack can reach a record through.
                    return self.return_object(this, ppv);
                }
                if iid != g::AEEIID_IBitmap {
                    // ⚠ The zero matters, and this arm has to write it itself: a caller that branches on the
                    // result still reads the pointer it was promised, and leaving its stack value there is how
                    // a refusal turns into a call through somebody's old local. `answer_qi` says exactly this
                    // — the same sentence, for the same reason — and it cannot be used here because its
                    // argument is one list of ids and the ids above are not one list.
                    //
                    // The code is `EUNSUPPORTED` because the delivered body says so: this body returns
                    // `EUNSUPPORTED` for every id it does not answer, not the `ECLASSNOTSUPPORT` most
                    // other `QueryInterface` bodies use.
                    if ppv != 0 {
                        self.cpu.mem.write_u32(ppv, 0)?;
                    }
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                self.return_object(this, ppv)
            }
            "RGBToNative" => {
                let [rgb] = self.args()?;
                Ok(self.display.pixel_value(&self.cpu.mem, this, rgb)?)
            }
            "NativeToRGB" => {
                let [native] = self.args()?;
                Ok(self.display.pixel_rgb(&self.cpu.mem, this, native)?)
            }
            "DrawPixel" => {
                let [x, y, color, rop] = self.args()?;
                Ok(self
                    .display
                    .draw_pixel(&mut self.cpu.mem, this, x, y, color, rop)?)
            }
            "GetPixel" => {
                let [x, y, pcolor] = self.args()?;
                Ok(self
                    .display
                    .get_pixel(&mut self.cpu.mem, this, x, y, pcolor)?)
            }
            "SetPixels" => {
                let [cnt, points, color, rop] = self.args()?;
                Ok(self
                    .display
                    .set_pixels(&mut self.cpu.mem, this, cnt, points, color, rop)?)
            }
            "DrawHScanline" => {
                let [y, xmin, xmax, color, rop] = self.args()?;
                Ok(self
                    .display
                    .h_scanline(&mut self.cpu.mem, this, y, xmin, xmax, color, rop)?)
            }
            "FillRect" => {
                let [prc, color, rop] = self.args()?;
                Ok(self
                    .display
                    .fill_rect(&mut self.cpu.mem, this, prc, color, rop)?)
            }
            "BltIn" => {
                // `this` is the destination.
                let [xd, yd, cw, ch, src, xs, ys, rop] = self.args()?;
                Ok(self.display.blt_in(
                    &mut self.cpu.mem,
                    this,
                    xd as i32,
                    yd as i32,
                    cw as i32,
                    ch as i32,
                    src,
                    xs as i32,
                    ys as i32,
                    rop,
                )?)
            }
            "BltOut" => {
                // `this` is the source, and the eight arguments keep their `BltIn` meaning.
                let [xd, yd, cw, ch, dst, xs, ys, rop] = self.args()?;
                Ok(self.display.blt_out(
                    &mut self.cpu.mem,
                    this,
                    xd as i32,
                    yd as i32,
                    cw as i32,
                    ch as i32,
                    dst,
                    xs as i32,
                    ys as i32,
                    rop,
                )?)
            }
            "GetInfo" => {
                let [pinfo, nsize] = self.args()?;
                Ok(self
                    .display
                    .get_info(&mut self.cpu.mem, this, pinfo, nsize)?)
            }
            "CreateCompatibleBitmap" => {
                let [ppv, w, h] = self.args()?;
                let addr = self.display.create_compatible(
                    &mut self.cpu.mem,
                    &mut self.hle,
                    this,
                    w as u16,
                    h as u16,
                )?;
                self.give_object(addr, ppv)
            }
            "SetTransparencyColor" => {
                let [color] = self.args()?;
                Ok(self.display.transparency(&mut self.cpu.mem, this, color)?)
            }
            "GetTransparencyColor" => {
                let [pcolor] = self.args()?;
                Ok(self
                    .display
                    .get_transparency(&mut self.cpu.mem, this, pcolor)?)
            }
            _ => Err(unimplemented()),
        }
    }

    /// Which raster an animated image draws now. An explicit `DrawFrame` index wins; otherwise a
    /// started image advances on the virtual clock using `IPARM_RATE` when the caller set one, and
    /// the file's own frame delays otherwise.
    fn image_frame_index(&self, frame: i32, state: &ImageFrames, parm: &ImageParm) -> usize {
        let count = state.rasters.len();
        if count == 0 {
            return 0;
        }
        if frame >= 0 {
            return (frame as usize).min(count - 1);
        }
        if let Some(start) = state.started_ms {
            let elapsed = self.clock_ms.saturating_sub(start);
            if parm.rate > 0 {
                return ((elapsed / parm.rate) as usize) % count;
            }
            let total: u32 = state.delays.iter().map(|d| u32::from(*d) * 10).sum();
            if total > 0 {
                let mut at = elapsed % total;
                for (i, &delay) in state.delays.iter().enumerate() {
                    let span = u32::from(delay) * 10;
                    if at < span {
                        return i;
                    }
                    at -= span;
                }
            }
        }
        state.current.min(count - 1)
    }

    /// Copy an image onto the applet's screen, which is all `IImage::Draw` asks for.
    ///
    /// `frame` is the `nFrame` of `IImage::DrawFrame`; plain `Draw` is frame 0. The frame is picked
    /// out of the strip by `IPARM_CXFRAME`, which is the parameter the header says to use for "formats
    /// not normally supporting animation" — a PNG among them — and a caller counting frames
    /// 0..`IPARM_NFRAMES` means steps of exactly that width.
    pub(crate) fn blit_image(&mut self, obj: u32, frame: i32, x: i32, y: i32) -> HResult<u32> {
        let animated = self.image_frames.get(&obj);
        let is_animated = animated.is_some();
        let mut parm = self.image_parm.get(&obj).copied().unwrap_or_default();
        let src = if let Some(state) = animated {
            let index = self
                .image_frame_index(frame, state, &parm)
                .min(state.rasters.len().saturating_sub(1));
            let src = state.rasters[index];
            if frame < 0 {
                // Remember the frame the clock landed on, so a later `Stop` freezes the picture
                // that is actually on screen rather than jumping back to the first frame.
                if let Some(state) = self.image_frames.get_mut(&obj) {
                    state.current = index;
                }
            }
            // A decoded frame is a whole picture; the strip width the caller may have named is a
            // different feature, so it must not slide a window inside this raster.
            parm.cx_frame = None;
            parm.frames = None;
            src
        } else {
            self.images
                .get(&obj)
                .copied()
                .ok_or(HleError::NotAnObject { addr: obj })?
        };
        let b = self
            .display
            .bitmap(src)
            .ok_or(HleError::NotAnObject { addr: src })?;
        let down = |e: Error| match e {
            Error::Hle(h) => h,
            other => HleError::Unsupported {
                what: other.to_string(),
            },
        };
        // What the caller asked to see, not the whole decoded picture: `IPARM_SIZE` and
        // `IPARM_OFFSET` name a portion of a sprite sheet, and `IPARM_ROP` names how it is laid
        // down. Both paths below carry the same three, because a pack that composited onto its own
        // display was asking for the same portion of the same image.
        //
        // The frame slides the source window along the strip; whatever portion the caller named
        // still applies inside it, and the clamping is the same rule with the shifted origin. An
        // animated picture already picked its raster, so there is no strip to slide.
        let (sx, sy) = if is_animated {
            (parm.off.0, parm.off.1)
        } else {
            let stride = parm.frame_width(i32::from(b.width), i32::from(b.height));
            (parm.off.0 + frame.max(0) * stride, parm.off.1)
        };
        let (cx, cy) = {
            let mut window = parm;
            window.off = (sx, sy);
            window.crop(i32::from(b.width), i32::from(b.height))
        };
        if cx <= 0 || cy <= 0 {
            // The fourth way a picture paints nothing and says nothing. `Display::copy` already names
            // the two that blame the destination's clip and the third that blames the source; this one
            // is reached before either, when the portion the caller *named* leaves no pixels once it is
            // put inside the picture — a size or offset of its own that walks off the bitmap, or a
            // frame index past the end of a strip. Returning `0` here without saying so makes "the host
            // drew my sprite" and "the host drew nothing at all" the same answer, which is the shape
            // this repository keeps having to hunt.
            self.display.note(format!(
                "IImage::Draw of {obj:#010x} paints nothing: {cx}x{cy} taken from a {}x{} picture \
                 at offset ({sx},{sy})",
                b.width, b.height
            ));
            return Ok(0);
        }
        // The caller may have named a display of its own with `IPARM_DISPLAY`. When it did, the
        // picture has to land there and nowhere else: the object behind that pointer is guest code,
        // so this is a call down through its vtable rather than a write into this host's framebuffer.
        if let Some(disp) = self.image_display.get(&obj).copied() {
            let bit_blt = self.guest_slot(disp, "IDisplay", "BitBlt").map_err(down)?;
            return self
                .call_guest(
                    bit_blt,
                    &[
                        disp, x as u32, y as u32, cx as u32, cy as u32, src, sx as u32, sy as u32,
                        parm.rop,
                    ],
                )
                .map_err(down);
        }
        let disp = self.display.default_display();
        self.display
            .bit_blt(&mut self.cpu.mem, disp, x, y, cx, cy, src, sx, sy, parm.rop)?;
        Ok(0)
    }
}
