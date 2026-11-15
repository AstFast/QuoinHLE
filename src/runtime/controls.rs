//! The control tree: `IWidget`, dialogs, menus and the screen controls that
//! paint into the display.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// The pair of item-style records this host hands out, as **eight bytes each**.
    /// `IShell::GetItemStyle` validates both pointers with `AEE_CheckPtr(0, p, 8, 1)` before it writes
    /// anything, and `IMenuCtl::GetStyle` in the delivered body memmoves exactly eight bytes from each
    /// of the two copies the control keeps (`+356` and `+364`), so the record ends at `+7` on this
    /// platform — and the 12-byte `AEEItemStyle` in the vendored header, whose fourth field sits at
    /// `+8`, is a later shape that would overrun it.
    ///
    /// ⚠ The names are published and the widths are not. The header declares
    /// `AEEItemStyle` as `AEEFrameType ft; uint16 xOffset; uint16 yOffset; AEERasterOp roImage;` — four
    /// fields and **twelve** bytes, `+8` for `roImage` — and this table copies it. But both bodies that
    /// move a record of this type move **eight** bytes, and the way `Menu_GetStyleOffsets` reads them
    /// (`+0` into `IDisplay::DrawFrame`, then halfwords at `+2` and `+4`) puts its three used members at
    /// a pitch the header's types cannot produce. So the shipped runtime's `AEEItemStyle` is a different
    /// shape than the header's, and what is known per byte is only its *role* — which is why the records
    /// are reproduced as bytes rather than filled through the header's field names.
    ///
    /// What the values are is witnessed: they come from the three `AEEDeviceInfo` fields the device reads
    /// (`cxScreen`, `cyScreen`, `nColorDepth`), which is why this answers from the host's own screen
    /// rather than a theme. Bytes `+1` and `+7` are zero because the device clears the record before
    /// choosing four of its bytes; a caller that logs the record sees the same thing on both.
    pub(crate) fn item_styles(&self) -> ([u8; 8], [u8; 8]) {
        let wide = self.device.width >= 0x78;
        let tall = self.device.height >= 0x78;
        // The device branches on `nColorDepth` first and only reaches for the screen size when the
        // depth is neither of the two low ones; `+6` is the byte that changes with it.
        let (normal_base, sel_base, normal_edge, sel_edge) = match self.device.color_depth {
            1 => (0, 0, 2, 3),
            2 => (1, 5, 2, 2),
            _ => (
                if wide && tall { 2 } else { 1 },
                if wide && tall { 3 } else { 5 },
                7,
                7,
            ),
        };
        let mut rec = [0u8; 8];
        let frame: u16 = if wide { 3 } else { 1 };
        let stack: u16 = if tall { 3 } else { 1 };
        rec[2..4].copy_from_slice(&frame.to_le_bytes());
        rec[4..6].copy_from_slice(&stack.to_le_bytes());
        let mut pair = [rec; 2];
        pair[0][0] = normal_base;
        pair[0][6] = normal_edge;
        pair[1][0] = sel_base;
        pair[1][6] = sel_edge;
        (pair[0], pair[1])
    }

    /// The `IDialog` slots: the seven the delivered body's vtable names beyond `AddRef`/`Release`, which
    /// the object machinery answers for every host class.
    pub(crate) fn idialog(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let mut dialog = self
            .dialogs
            .get(&this)
            .cloned()
            .ok_or(Error::Hle(HleError::NotAnObject { addr: this }))?;
        match name {
            "SetFocus" => {
                let [want] = self.args()?;
                self.dialog_set_focus(this, want as u16).map(u32::from)
            }
            // `ADialog_GetControl` walks the list and answers with the control it holds. It does not
            // `AddRef` it: the dialog holds the only reference, which is why releasing the dialog
            // releases the controls, and why a pack that keeps this pointer past `ISHELL::EndDialog`
            // is holding what the shipped object has already freed.
            "GetControl" => {
                let [want] = self.args()?;
                Ok(dialog.control(want as u16).unwrap_or(0))
            }
            "SetEventHandler" => {
                let [pfn, user] = self.args()?;
                dialog.pfn = pfn;
                dialog.user = user;
                self.dialogs.insert(this, dialog);
                // The slot is `void`; the shipped body leaves the object in `r0`, and a caller that
                // reads `r0` from a void call is reading whatever the body happened to return.
                Ok(this)
            }
            "GetID" => Ok(u32::from(dialog.id)),
            "SetProperties" => {
                let [props] = self.args()?;
                dialog.props |= props;
                self.dialogs.insert(this, dialog);
                Ok(this)
            }
            "GetProperties" => Ok(dialog.props),
            "Redraw" => {
                self.dialog_redraw(this)?;
                Ok(0)
            }
            _ => {
                return Err(Error::Hle(HleError::Unimplemented {
                    id,
                    label: label.to_string(),
                    this,
                }))
            }
        }
    }

    /// The class an object of this host was made from. The shipped controls keep the same fact in
    /// their own first word — `TimeCtl.m_cls`, tested by the `IsAlarmClock` / `IsCountDown` /
    /// `IsStopWatch` macros at the delivered body — and it is the class and not the interface that
    /// decides several of their answers, because three classes share the one `ITimeCtl` vtable.
    pub(crate) fn ctl_clsid(&self, addr: u32) -> u32 {
        self.hle.object(addr).map_or(0, |o| o.clsid)
    }

    /// What `ADialog_Redraw` does with the controls it owns: give each one the active state the
    /// dialog's own focus says it should have, render it, and end with one `IDisplay_Update(1)`.
    /// The shipped member draws the frame and the copyright line around them first, out of system
    /// colours this host has no witness for; the contents are the half that carries the app's own
    /// pixels, so that is the half that lands here.
    pub(crate) fn dialog_redraw(&mut self, obj: u32) -> Result<(), Error> {
        let Some(dialog) = self.dialogs.get(&obj).cloned() else {
            return Ok(());
        };
        let disp = self.display.default_display();
        for ctl in dialog.controls.iter() {
            let on = dialog.focus == Some(ctl.id);
            self.widgets
                .set_active(ctl.ctl, u32::from(on), self.is_stopwatch(ctl.ctl))?;
            self.widgets
                .redraw(&mut self.cpu.mem, &mut self.display, disp, ctl.ctl)?;
        }
        self.display.update(&self.cpu.mem, disp, 1)?;
        Ok(())
    }

    /// `ADialog_SendEvent`: the handler the dialog was given answers first, and with no handler the
    /// news goes to the owner applet. With neither, the answer is FALSE — which is also the answer
    /// that makes `ISHELL::CreateDialog` fail when there is no applet running to accept its dialog.
    ///
    /// The shipped version enters the owner's *app context* around the call, so a dialog owned by a
    /// suspended applet runs its handler with that applet as the current one. This host routes the
    /// news to whichever applet is running now, which is the same applet whenever a dialog is driven
    /// by the code that made it.
    pub(crate) fn dialog_send_event(
        &mut self,
        dialog: &Dialog,
        evt: u32,
        wp: u32,
        dwp: u32,
    ) -> Result<bool, Error> {
        if dialog.pfn != 0 {
            return Ok(self.call_guest(dialog.pfn, &[dialog.user, evt, wp, dwp])? != 0);
        }
        if dialog.owner != 0 {
            return Ok(self.send_event(evt, wp, dwp)? != 0);
        }
        Ok(false)
    }

    pub(crate) fn dialog_set_focus(&mut self, obj: u32, want: u16) -> Result<u16, Error> {
        let mut dialog = self
            .dialogs
            .get(&obj)
            .cloned()
            .ok_or(Error::Hle(HleError::NotAnObject { addr: obj }))?;
        let last = dialog.focus.unwrap_or(0);
        let prev = dialog.focus.and_then(|f| dialog.control(f));
        if let Some(next) = dialog.control(want) {
            // The change test is `v5 != a2`, and `v5` is the old node's *id*, or 0 when there is no
            // old node — so a dialog focused on a control whose id is 0 and a dialog with no focus at
            // all answer `SetFocus(0)` the same way, with nothing moving.
            if last != want {
                // `if ( v5 != 0 ) SetFocus(old, FALSE)`: the gate is the old **id**, while the
                // rollback three lines down is gated on the old *node*. So a control that held focus
                // under id 0 keeps its active state after the dialog has moved on, and the shipped
                // member leaves it lit — nothing here gets to tidy that up.
                if last != 0 {
                    if let Some(old) = prev {
                        self.widgets.set_active(old, 0, self.is_stopwatch(old))?;
                    }
                }
                self.widgets.set_active(next, 1, self.is_stopwatch(next))?;
                if self.widgets.is_active(next)? != 0 {
                    dialog.focus = Some(want);
                } else if let Some(old) = prev {
                    self.widgets.set_active(old, 1, self.is_stopwatch(old))?;
                }
                self.dialogs.insert(obj, dialog);
            }
        }
        Ok(last)
    }

    /// `IControl::HandleEvent` for a control this host made: the three codes a dialog fills a control
    /// with share the door, and everything else goes down the key path.
    pub(crate) fn widget_handle_event(&mut self, ctl: u32, evt: u32, code: u32, dwp: u32) -> HResult<bool> {
        if matches!(
            evt,
            g::EVT_CTL_ADD_ITEM | g::EVT_CTL_SET_TITLE | g::EVT_CTL_SET_TEXT
        ) {
            return self.ctl_fill(ctl, evt, code, dwp);
        }
        // `IHtmlViewer` takes keys of its own: arrows move the focus and `AVK_SELECT` follows a link
        // or submits a form, both of which are `HViewNotify` reports rather than commands.
        if self.widgets.class(ctl)? == "IHtmlViewer" {
            return self.html_handle_event(ctl, evt, code, dwp);
        }
        Ok(match self.widgets.handle_key(ctl, evt, code)? {
            widget::Key::NotHandled => false,
            widget::Key::Handled => true,
            // The control repaints from inside the handler, so the cursor moves on this key and not
            // on some later frame — `Menu_MoveSelect` ends on `Menu_Draw`, which paints and calls
            // `IDisplay::Update(po, TRUE)`.
            widget::Key::Redraw => {
                let disp = self.display.default_display();
                self.widgets
                    .redraw(&mut self.cpu.mem, &mut self.display, disp, ctl)?;
                self.display.update(&self.cpu.mem, disp, 1)?;
                true
            }
            // The control's answer is the applet's answer to the event it is about to post, so the
            // post itself waits for a caller that can carry a guest fault — see
            // [`Runtime::pending_command`]. Leaving `Some` here without making the call would be the
            // bug: the answer would silently become "handled".
            widget::Key::Post(cmd) => {
                self.pending_command = Some((g::EVT_COMMAND, cmd, ctl));
                true
            }
        })
    }

    /// The same door a pack uses, from host code. A control this host made answers out of the widget
    /// store; one a module made has no record there, and the only door it has is the slot in its own
    /// object — which is also why the hit test below asks for its rectangle through the vtable.
    pub(crate) fn control_handle_event(
        &mut self,
        ctl: u32,
        evt: u32,
        wp: u32,
        dwp: u32,
    ) -> Result<bool, Error> {
        if self.widgets.holds(ctl) {
            let answered = self.widget_handle_event(ctl, evt, wp, dwp)?;
            return Ok(match self.pending_command.take() {
                Some((command, id, control)) => self.send_event(command, id, control)? != 0,
                None => answered,
            });
        }
        let slot = self.guest_slot(ctl, "IControl", "HandleEvent")?;
        Ok(self.call_guest(slot, &[ctl, evt, wp, dwp])? != 0)
    }

    /// `ICONTROL_GetRect` for a control, read back as a rectangle rather than written for a caller.
    /// The shipped hit test keeps one `AEERect` on its own stack and reuses it per row; this host has
    /// no stack to borrow, so a guest module's control is asked through its vtable with a block this
    /// host allocated for the length of the question.
    pub(crate) fn control_rect(&mut self, ctl: u32) -> Result<Rect, Error> {
        if self.widgets.holds(ctl) {
            return Ok(self.widgets.rect_at(ctl)?);
        }
        let scratch = self.heap_alloc(8)?;
        if scratch == 0 {
            // The arena declined the eight bytes, so there is nowhere to receive the answer. A control
            // this host cannot ask is not a control the point is inside, and the reason is said out
            // loud rather than left as a silent miss.
            self.display.note(format!(
                "IControl::GetRect for {ctl:#010x} needs a scratch block the arena will not give"
            ));
            return Ok(Rect::EMPTY);
        }
        let slot = self.guest_slot(ctl, "IControl", "GetRect")?;
        let result = self.call_guest(slot, &[ctl, scratch]);
        let rect = match result {
            Ok(_) => Rect::read(&self.cpu.mem, scratch),
            Err(error) => {
                self.hle.dealloc(&mut self.cpu.mem, scratch);
                return Err(error);
            }
        };
        self.hle.dealloc(&mut self.cpu.mem, scratch);
        Ok(rect?)
    }

    /// The `DLG_HANDLE_ALL_EVENTS` step both branches end on. The record is read afresh, because a
    /// control that took the event may have moved the dialog's properties or its handler in the time
    /// since — the shipped body re-reads `m_dwProps` and `m_pfnEvent` at that point too.
    pub(crate) fn dialog_all_events(&mut self, obj: u32, evt: u32, wp: u32, dwp: u32) -> Result<bool, Error> {
        let Some(dialog) = self.dialogs.get(&obj).cloned() else {
            return Ok(false);
        };
        if dialog.props & g::DLG_HANDLE_ALL_EVENTS == 0 || dialog.pfn == 0 {
            return Ok(false);
        }
        Ok(self.call_guest(dialog.pfn, &[dialog.user, evt, wp, dwp])? != 0)
    }

    /// `ADialog_HandleEvent`, in the shipped order and with the shipped gates.
    ///
    /// The prompt branch is not here: `m_bPrompt` is raised only by `ADialog_Prompt`, and the three
    /// returns it owns (`EVT_COMMAND`, the close key, the copyright key) all end in
    /// `ADialog_Dismiss`, which is not this door yet. What is here is everything a dialog a pack built
    /// for itself actually does: the `m_bReady` gate, the focus gate that comes **before** any
    /// broadcast, `EVT_CTL_TAB`'s two directions with their wrapping, the pen hit test, the ordered
    /// walk that stops at the first control that takes the event, and the handler that is offered
    /// whatever no control took.
    pub(crate) fn dialog_handle_event(
        &mut self,
        obj: u32,
        evt: u32,
        wp: u32,
        dwp: u32,
    ) -> Result<bool, Error> {
        let dialog = self
            .dialogs
            .get(&obj)
            .cloned()
            .ok_or(Error::Hle(HleError::NotAnObject { addr: obj }))?;
        // `if ( (v5 & 1) == 0 ) return nullptr;` — everything the router accepts is behind `m_bReady`,
        // which is why the loader raises it only after `EVT_DIALOG_START` was accepted.
        if !dialog.ready {
            return Ok(false);
        }
        let pen = matches!(
            evt,
            g::EVT_PEN_DOWN | g::EVT_PEN_MOVE | g::EVT_PEN_UP | g::EVT_PEN_STALE_MOVE
        );
        if !pen {
            if dialog.focus.is_none() {
                return Ok(false);
            }
            if evt == g::EVT_CTL_TAB {
                // Forward is the focus node's `pNext`, wrapping to the head. Backward walks from the
                // head for the node *before* the focus one, and when focus was already the head the
                // shipped's second loop walks to the **last** node — so a dialog with one control
                // keeps that control, and TAB never leaves the list empty-handed.
                let at = dialog
                    .focus
                    .and_then(|f| dialog.controls.iter().position(|c| c.id == f));
                let Some(i) = at else {
                    return Ok(false);
                };
                let next = if wp != 0 {
                    if i + 1 < dialog.controls.len() {
                        i + 1
                    } else {
                        0
                    }
                } else if i == 0 {
                    dialog.controls.len() - 1
                } else {
                    i - 1
                };
                let id = dialog.controls[next].id;
                self.dialog_set_focus(obj, id)?;
                // `ADialog_SetFocus(a1, i->wID); return TRUE;` — the answer is TRUE whether or not the
                // control took the focus, and this is the one branch with no handler fallback.
                return Ok(true);
            }
            // `for (pdc = m_pControls; pdc; pdc = pdc->pNext) if (ICONTROL_HandleEvent(...)) return
            // TRUE;` — the list is in the order it was linked, which is the record's order reversed.
            for ctl in dialog.controls.iter() {
                if self.control_handle_event(ctl.ctl, evt, wp, dwp)? {
                    return Ok(true);
                }
            }
            return self.dialog_all_events(obj, evt, wp, dwp);
        }
        // `AEE_GET_X(dwParm)` is `(int16)AEE_HI(dwParm)` and `AEE_GET_Y` the low half,
        //so the position travels with **x in the high 16 bits**.
        let x = (dwp >> 16) as i16 as i32;
        let y = (dwp as u16) as i16 as i32;
        let inside = |r: &Rect| {
            let right = r.x as i32 + r.dx as i32;
            let bottom = r.y as i32 + r.dy as i32;
            x >= r.x as i32 && x <= right && y >= r.y as i32 && y <= bottom
        };
        if evt != g::EVT_PEN_DOWN {
            // MOVE, UP and STALE_MOVE go to the focused control and nowhere else.
            let Some(focus) = dialog.focus.and_then(|f| dialog.control(f)) else {
                return Ok(false);
            };
            if self.control_handle_event(focus, evt, wp, dwp)? {
                // The delivered body:
                //the handler is **notified** and its answer is
                // dropped, because the dialog already took the event. Handing the notification's own
                // boolean back would let a dialog with no handler, or one whose handler says FALSE,
                // fall through and show a consumed tap to the applet.
                self.dialog_all_events(obj, evt, wp, dwp)?;
                return Ok(true);
            }
            return Ok(false);
        }
        // DOWN first asks whether the point is in the dialog's own frame at all.
        if !inside(&dialog.rect) {
            return Ok(false);
        }
        let mut handler = 0u32;
        if let Some(focus) = dialog.focus.and_then(|f| dialog.control(f)) {
            let rect = self.control_rect(focus)?;
            if inside(&rect) {
                handler = focus;
            }
        }
        if handler == 0 {
            // The overlap rule is the loop's, not its comment's: the shipped walk does **not** stop at
            // the first control whose rectangle holds the point — it keeps going, re-focusing and
            // re-answering `pHandler` for every later one that also holds it, so the last hit wins,
            // even though the delivered body says "This gives the first control ID precedence".
            // The delivered body
            for node in dialog.controls.iter() {
                // The focus control already had its turn above, and the shipped skips the focus *node*
                // by pointer, which is what re-reading `m_pFocus` per step reproduces.
                let current = self.dialogs.get(&obj).and_then(|d| d.focus);
                if current == Some(node.id) {
                    continue;
                }
                let rect = self.control_rect(node.ctl)?;
                if !inside(&rect) {
                    continue;
                }
                self.dialog_set_focus(obj, node.id)?;
                // `if (m_pFocus && m_pFocus->wID == pdc->wID) pHandler = m_pFocus->pc;` — a control
                // that refused the focus is not the one the event goes to.
                let now = self.dialogs.get(&obj).cloned();
                if let Some(d) = now {
                    if d.focus == Some(node.id) {
                        handler = d.control(node.id).unwrap_or(0);
                    }
                }
            }
        }
        if handler == 0 {
            return Ok(false);
        }
        if self.control_handle_event(handler, evt, wp, dwp)? {
            // The delivered body
            self.dialog_all_events(obj, evt, wp, dwp)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// `IShell::CreateInstance` for one control of a dialog record: the classes this host serves get
    /// their faces and their per-class records exactly as the shell's own door hands them out, and
    /// anything else is asked of the applet's module, which is where the shell sends a class it does
    /// not know. `None` is the shipped "this control could not be made", and the caller leaves it out
    /// of the dialog rather than failing the whole thing.
    pub(crate) fn dialog_control(&mut self, cls: u32) -> Result<Option<u32>, Error> {
        if let Some((iface, _)) = HOST_SERVICES.iter().find(|(_, c)| *c == cls) {
            let mut faces: Vec<(&'static str, u32)> = Vec::new();
            for &(face, _c) in HOST_SERVICES.iter().filter(|(_, c)| *c == cls) {
                let made = self
                    .hle
                    .new_object(&mut self.cpu.mem, face, cls, 8, 4, &[])?;
                faces.push((face, made));
            }
            let addr = faces[0].1;
            self.faces.insert(cls, faces);
            let class = abi::resolve_alias(iface);
            if widget::is_widget(class) {
                self.widgets.add(addr, class);
                self.seed_widget_control(addr, class)?;
            }
            if class == "ISprite" {
                self.sprites.add(addr);
            }
            if class == "IAClockCtl" {
                self.clocks
                    .add(addr, aclock::Clock::new(self.display.device.depth));
            }
            return Ok(Some(addr));
        }
        if cls == 0 {
            // A null class is refused before anyone is asked, in this host as in the shell —
            // see [`Runtime::create_via_module`] for what a module does with one.
            return Ok(None);
        }
        let slot = self.heap_alloc(4)?;
        if slot == 0 {
            return Ok(None);
        }
        // The shipped shell clears the caller's slot before it asks, and the answer the pack must be
        // able to test is a null pointer rather than whatever its memory held.
        self.cpu.mem.write_u32(slot, 0)?;
        let code = self.create_via_module(cls, slot)?;
        let addr = self.cpu.mem.read_u32(slot)?;
        self.hle.dealloc(&mut self.cpu.mem, slot);
        if code != g::AEE_SUCCESS || addr == 0 {
            self.display.note(format!(
                "IDialog: the record names class {cls:#010x}, which neither this host nor the running \
                 module's own class table made"
            ));
            return Ok(None);
        }
        Ok(Some(addr))
    }

    /// The loader's `abort:`: release the half-built dialog the way the shipped loader does, which
    /// takes its controls, its place on the applet's dialog chain and its own memory with it.
    pub(crate) fn abandon_dialog(&mut self, obj: u32) -> Result<(), Error> {
        let release =
            abi::method_of("IDialog", "Release").expect("IDialog::Release is in the tables");
        self.object_method(release, obj, "IDialog")?;
        Ok(())
    }

    /// `ISHELL_ShowCopyright(p)` — the `Prompt(p, NULL)` case, and not a prompt at all.
    ///
    /// The dialog's `Prompt` checks the null struct before it builds anything and hands the
    /// whole call to `ADialog_ShowCopyright`, which reads four strings out of the module's MIF
    /// (application name, icon, `IDS_MIF_COPYRIGHT`, company and version), measures them against the
    /// screen, paints a frame, and arms **its own** timer: `SetTimer(2000, ADialog_Timeout, pme)`.
    /// Nothing the caller passed is read, which is why the call answers TRUE for a caller that named
    /// no buttons, and why it is a different door from the prompt in [`Runtime::iface_method`].
    ///
    /// What the notice owes the applet is not the prompt's `EVT_COMMAND` but its close:
    /// `ADialog_Timeout` is `ADialog_Dismiss(pme, pme->m_wDefCommand)` with that member still zero, so
    /// `ADialog_Dismiss` sends no command at all, and the news arrives from `ADialog_Release` —
    /// `SendEvent(pme, m_bCopyright ? EVT_COPYRIGHT_END: EVT_DIALOG_END, m_wDialogID, pme)`. One
    /// event, addressed to the applet, carrying the dialog's own id and pointer.
    ///
    /// So this host makes the dialog and lets it close. The *picture* is not drawn — the MIF's strings
    /// are in a resource encoding this host has no font for, and a notice nobody can read is not worth
    /// a frame — but the record is made, stacked where `ADialog_New` puts one, reported by
    /// `IShell::GetActiveDialog` while it is up, and closed through [`Runtime::finish_dialog`], which is
    /// the door that owns the end-of-dialog event. That is what a pack which waits for
    /// `EVT_COPYRIGHT_END` before it builds its own screen needs, and one shipped pack does exactly
    /// that: without it the applet sat idle for the whole run, having drawn nothing.
    ///
    /// Two members of the record are worth naming because they look alike and are not. `ready` is
    /// raised here: the shipped `ADialog_ShowCopyright` raises it along with the modal and copyright
    /// bits in one store to `m_dwProps`, and `ADialog_Release` reports only to a ready dialog.
    /// `prompt` stays down, because in this host that flag means "a `Prompt` already sent its command
    /// from `ADialog_Dismiss`" and a notice never sends one — raising it here would silence the one
    /// event the call exists to deliver.
    pub(crate) fn show_copyright(&mut self) -> Result<u32, Error> {
        let obj =
            self.hle
                .new_object(&mut self.cpu.mem, "IDialog", g::AEECLSID_DIALOG, 8, 4, &[])?;
        let dialog = Dialog {
            // A fresh `ADialog` has no id: `m_wDialogID` comes from the resource a load reads, and
            // this dialog was not loaded from one.
            id: 0,
            props: 0,
            owner: self.applet,
            next: self.active_dialog,
            screen: (self.device.width, self.device.height),
            ready: true,
            copyright: true,
            ..Dialog::default()
        };
        self.active_dialog = obj;
        self.dialogs.insert(obj, dialog);
        self.copyright = Some(Copyright {
            dialog: obj,
            due: self.clock_ms.saturating_add(g::COPYRIGHT_TIMEOUT),
        });
        self.display.note(format!(
            "IShell::Prompt(NULL): the copyright notice is up as dialog {obj:#x} for \
             {} ms — no frame is drawn for it, and EVT_COPYRIGHT_END comes from its close",
            g::COPYRIGHT_TIMEOUT
        ));
        Ok(g::TRUE)
    }

    /// Close the copyright notice, which is what both of its endings do: the timer, and the key that
    /// `ADialog_HandleEvent` lets dismiss it (`evt == EVT_KEY && (m_dwProps & AEE_DIALOG_…copyright)`).
    ///
    /// Either way the shipped body is `ADialog_Dismiss(pme, m_wDefCommand)` with `m_wDefCommand` zero,
    /// so the close carries no `EVT_COMMAND` and the release carries the `EVT_COPYRIGHT_END`. The
    /// release is reached through `IDialog::Release`, not by dropping the record here: that path is
    /// where the chain re-link, the controls (there are none) and the end event already live, and a
    /// second copy of it would be a second set of rules about who hears the news.
    pub(crate) fn dismiss_copyright(&mut self) -> Result<(), Error> {
        let Some(notice) = self.copyright.take() else {
            return Ok(());
        };
        let release =
            abi::method_of("IDialog", "Release").expect("IDialog::Release is in the tables");
        self.object_method(release, notice.dialog, "IDialog")?;
        Ok(())
    }

    /// `ADialog_Release` once its count reaches zero, which `ISHELL::EndDialog` reaches by calling the
    /// dialog's own `Release` and the loader's abort path reaches the same way: unstack the dialog,
    /// tell the owner it is over, release every control it made, and repaint whatever is now on top.
    ///
    /// The shipped body also cancels the dialog's timers through `ISHELL_CancelTimer(NULL, po)`. This
    /// door does not arm any yet — `ADialog_Timeout` is what sets one, and that is not this path — so
    /// there is nothing to take back, and the timer list is keyed by applet rather than by dialog.
    pub(crate) fn finish_dialog(&mut self, obj: u32, dialog: Dialog) -> Result<(), Error> {
        if self.active_dialog == obj {
            self.active_dialog = dialog.next;
        } else {
            // Otherwise walk the chain for the dialog this one hangs off, and let it point past the
            // one going away — the shipped loop does exactly that walk before it re-links.
            let mut at = self.active_dialog;
            while at != 0 {
                let Some(rec) = self.dialogs.get(&at).cloned() else {
                    break;
                };
                if rec.next == obj {
                    if let Some(front) = self.dialogs.get_mut(&at) {
                        front.next = dialog.next;
                    }
                    break;
                }
                at = rec.next;
            }
        }
        // `if(!pme->m_bPrompt) { if(pme->m_bReady) SendEvent(…EVT_DIALOG_END…) }` — a prompt dialog is
        // silent because its own `ADialog_Dismiss` has already carried the news, and a dialog that
        // never reached `m_bReady` never got to be an event source either. The event is what a pack
        // that opened a dialog and never saw a command waits for.
        let event = if dialog.copyright {
            g::EVT_COPYRIGHT_END
        } else {
            g::EVT_DIALOG_END
        };
        let mut paint = dialog.next;
        if !dialog.prompt && dialog.ready {
            self.dialog_send_event(&dialog, event, u32::from(dialog.id), obj)?;
        }
        if !dialog.prompt {
            // "In case the user created another during the callback!" — the head is read *after*
            // `ADialog_SendEvent`, so a dialog the pack opened from its own end handler is what gets
            // the one repaint, and the dialog that was on top when this one started dying does not.
            paint = self.active_dialog;
        }
        let release =
            abi::method_of("IControl", "Release").expect("IControl::Release is in the tables");
        for ctl in dialog.controls.iter() {
            // The one reference the dialog held. A control the pack reached through
            // `IDIALOG_GetControl` and kept a reference of survives this, which is the handset's
            // arithmetic too.
            self.object_method(release, ctl.ctl, "IControl")?;
        }
        if paint != 0 {
            self.discarded_repaint(paint, "IDialog::Release");
        }
        Ok(())
    }

    /// `ICONTROL_HandleEvent` for the three codes a dialog uses to fill a control, which is also what
    /// a pack can send a control itself. the delivered body states the rule the shipped controls
    /// all follow: "Text ID is wParam, resource file is dwParam if coming from resource, otherwise
    /// wID is 0, and pText is actual text to set" — a non-zero `w` is an id to load out of the file
    /// `dw` names, and a zero `w` means `dw` already points at the text.
    ///
    /// `EVT_CTL_ADD_ITEM` is the third: there `dwParam` is the caller's `CtlAddItem`, read through the
    /// tables the same way `IMenuCtl::AddItemEx` reads it. The answer is the control's own: whether it
    /// took the news.
    pub(crate) fn ctl_fill(&mut self, ctl: u32, evt: u32, w: u32, dw: u32) -> HResult<bool> {
        if evt == g::EVT_CTL_ADD_ITEM {
            // `dwParam` is the caller's `CtlAddItem`, read through the tables like `AddItemEx` reads it.
            return Ok(self.widgets.add_item_ex(&mut self.cpu.mem, ctl, dw)? != 0);
        }
        if evt != g::EVT_CTL_SET_TITLE && evt != g::EVT_CTL_SET_TEXT {
            return Ok(false);
        }
        // `res_named` is how the shell's own resource slots read a file argument: it takes the pointer
        // and answers the container mounted beside this module, so a control told `dwParam =
        // pszResFile` reaches the same text the dialog itself was read from. A zero `w` is the other
        // reading the shipped controls share — `dw` *is* the text — and that one is served from here
        // because only a guest pointer can carry it.
        if w == 0 {
            let text = dw;
            if evt == g::EVT_CTL_SET_TITLE {
                return Ok(self.widgets.set_title(&mut self.cpu.mem, ctl, text)? != 0);
            }
            return Ok(self.widgets.set_text(&mut self.cpu.mem, ctl, text)? != 0);
        }
        let file = self.res_named(dw);
        self.ctl_fill_at(ctl, evt, w, file.as_deref())
    }

    /// The same door with the resource file already named, for host code that holds the name rather
    /// than a guest pointer to it — which is what `AEEDialog_Load` has in hand when it sends
    /// `EVT_CTL_SET_TITLE` with `pszResFile` as its fourth argument.
    pub(crate) fn ctl_fill_at(&mut self, ctl: u32, evt: u32, w: u32, file: Option<&str>) -> HResult<bool> {
        if evt == g::EVT_CTL_ADD_ITEM || w == 0 {
            // An item arrives as a block, and a caller with no id to look up has no reason to come
            // this way: the dialog's fill step knows which of the two it holds.
            return Ok(false);
        }
        let text = match self.res_string(file, w as u16)? {
            0 => {
                // The shipped control tries an image next and, failing that, answers FALSE. A text id
                // that names neither is the control's problem to report, not this door's to invent an
                // empty label for.
                return Ok(false);
            }
            made => made,
        };
        if evt == g::EVT_CTL_SET_TITLE {
            return Ok(self.widgets.set_title(&mut self.cpu.mem, ctl, text)? != 0);
        }
        Ok(self.widgets.set_text(&mut self.cpu.mem, ctl, text)? != 0)
    }

    /// The dialog record a pack passes as `DialogInfo *`, walked the way `ADialog_Load` walks it: a
    /// 20-byte head, then `nControls` items of `24 + 12 * nItems` bytes. That stride is the shipped
    /// pointer arithmetic, not a guess — `sizeof(DialogItem) + sizeof(DListItem) * (nItems - 1)`, and
    /// one `DListItem` *less* when the count is zero, which is what makes a control with no items 24
    /// bytes and not 36.
    pub(crate) fn dialog_spec_at(&mut self, info: u32) -> Result<DialogSpec, Error> {
        let head = |field: &str| {
            abi::offset_of("DialogInfoHead", field).expect("DialogInfoHead is in the tables")
        };
        let id_head = |field: &str| {
            abi::offset_of("DialogItemHead", field).expect("DialogItemHead is in the tables")
        };
        let item_stride = |n: u32| {
            let one = abi::struct_info("DListItem").map(|s| s.size).unwrap_or(12);
            let base = abi::struct_info("DialogItem").map(|s| s.size).unwrap_or(36);
            if n == 0 {
                base - one
            } else {
                base + one * (n - 1)
            }
        };
        let mut spec = DialogSpec {
            id: self.cpu.mem.read_u16(info + head("wID"))? as u16,
            props: self.cpu.mem.read_u32(info + head("dwProps"))?,
            rect: Rect::read(&self.cpu.mem, info + head("rc"))?,
            focus: self.cpu.mem.read_u16(info + head("wFocusID"))? as u16,
            controls: Vec::new(),
        };
        let count = self.cpu.mem.read_u16(info + head("nControls"))? as u32;
        let head_size = abi::struct_info("DialogItemHead")
            .map(|s| s.size)
            .unwrap_or(24);
        let mut at = info
            + abi::struct_info("DialogInfoHead")
                .map(|s| s.size)
                .unwrap_or(20);
        for _ in 0..count {
            let cls = self.cpu.mem.read_u32(at + id_head("cls"))?;
            let n = self.cpu.mem.read_u16(at + id_head("nItems"))? as u32;
            let mut ctl = CtlSpec {
                cls,
                id: self.cpu.mem.read_u16(at + id_head("wID"))? as u16,
                props: self.cpu.mem.read_u32(at + id_head("dwProps"))?,
                text: self.cpu.mem.read_u16(at + id_head("wTextID"))? as u16,
                title: self.cpu.mem.read_u16(at + id_head("wTitleID"))? as u16,
                rect: Rect::read(&self.cpu.mem, at + id_head("rc"))?,
                items: Vec::new(),
            };
            let first = at + head_size;
            for i in 0..n {
                let one = abi::struct_info("DListItem").map(|s| s.size).unwrap_or(12);
                let f = |field: &str| {
                    abi::offset_of("DListItem", field).expect("DListItem is in the tables")
                };
                let here = first + i * one;
                ctl.items.push(ItemSpec {
                    id: self.cpu.mem.read_u16(here + f("wID"))? as u16,
                    text: self.cpu.mem.read_u16(here + f("wTextID"))? as u16,
                    icon: self.cpu.mem.read_u16(here + f("wIconID"))? as u16,
                    data: self.cpu.mem.read_u32(here + f("dwData"))?,
                });
            }
            spec.controls.push(ctl);
            at += item_stride(n);
        }
        Ok(spec)
    }

    /// The same record as it is stored: a `RESTYPE_DIALOG` head whose tail is `nControls` ids, each
    /// naming a `RESTYPE_CONTROL` record, each of those with `nItems` ids naming `RESTYPE_LISTITEM`
    /// records of twelve bytes. The bytes are the ones `ISHELL::LoadResData` hands a pack for the same
    /// kind and id, so a dialog read through this door and one a module read for itself come from one
    /// answer, whatever the container's own property block turns out to be.
    pub(crate) fn dialog_spec_res(&mut self, file: &str, id: u16) -> Result<Option<DialogSpec>, Error> {
        let head = |field: &str| {
            abi::offset_of("DialogInfoHead", field).expect("DialogInfoHead is in the tables")
        };
        let id_head = |field: &str| {
            abi::offset_of("DialogItemHead", field).expect("DialogItemHead is in the tables")
        };
        let Some(dialog) = self
            .resources
            .find(Some(file), g::RESTYPE_DIALOG as u16, id)
            .map(|f| f.stored)
        else {
            return Ok(None);
        };
        let bytes = |src: &Vec<u8>, at: u32, n: u32| -> Option<Vec<u8>> {
            let from = at as usize;
            let to = from + n as usize;
            (to <= src.len()).then(|| src[from..to].to_vec())
        };
        let rd16 =
            |src: &Vec<u8>, at: u32| bytes(src, at, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let rd32 = |src: &Vec<u8>, at: u32| {
            bytes(src, at, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let mut spec = DialogSpec {
            id: rd16(&dialog, head("wID")).unwrap_or(0),
            props: rd32(&dialog, head("dwProps")).unwrap_or(0),
            rect: Rect::new(
                rd16(&dialog, head("rc")).unwrap_or(0xffff) as i16,
                rd16(&dialog, head("rc") + 2).unwrap_or(0xffff) as i16,
                rd16(&dialog, head("rc") + 4).unwrap_or(0xffff) as i16,
                rd16(&dialog, head("rc") + 6).unwrap_or(0xffff) as i16,
            ),
            focus: rd16(&dialog, head("wFocusID")).unwrap_or(0),
            controls: Vec::new(),
        };
        let count = rd16(&dialog, head("nControls")).unwrap_or(0) as usize;
        let tail = abi::struct_info("DialogInfoHead")
            .map(|s| s.size)
            .unwrap_or(20);
        for i in 0..count {
            let Some(control_id) = rd16(&dialog, tail + i as u32 * 2) else {
                break;
            };
            let Some(rec) = self
                .resources
                .find(Some(file), g::RESTYPE_CONTROL as u16, control_id)
                .map(|f| f.stored)
            else {
                // `if(!pdih) continue;` — a control id the container does not have costs that control
                // and nothing else, in this loader exactly as in the shipped one.
                continue;
            };
            let mut ctl = CtlSpec {
                cls: rd32(&rec, id_head("cls")).unwrap_or(0),
                id: rd16(&rec, id_head("wID")).unwrap_or(0),
                props: rd32(&rec, id_head("dwProps")).unwrap_or(0),
                text: rd16(&rec, id_head("wTextID")).unwrap_or(0xffff),
                title: rd16(&rec, id_head("wTitleID")).unwrap_or(0xffff),
                rect: Rect::new(
                    rd16(&rec, id_head("rc")).unwrap_or(0xffff) as i16,
                    rd16(&rec, id_head("rc") + 2).unwrap_or(0xffff) as i16,
                    rd16(&rec, id_head("rc") + 4).unwrap_or(0xffff) as i16,
                    rd16(&rec, id_head("rc") + 6).unwrap_or(0xffff) as i16,
                ),
                items: Vec::new(),
            };
            let n = rd16(&rec, id_head("nItems")).unwrap_or(0) as usize;
            let item_tail = abi::struct_info("DialogItemHead")
                .map(|s| s.size)
                .unwrap_or(24);
            for k in 0..n {
                let Some(item_id) = rd16(&rec, item_tail + k as u32 * 2) else {
                    break;
                };
                let f = |field: &str| {
                    abi::offset_of("DListItem", field).expect("DListItem is in the tables")
                };
                // `AEE_GetResData(pszResFile, wItemIDs[n], RESTYPE_LISTITEM, &p, &dw)` — the item is a
                // record of its own, twelve bytes, fetched by id.
                let Some(one) = self
                    .resources
                    .find(Some(file), g::RESTYPE_LISTITEM as u16, item_id)
                    .map(|found| found.stored)
                else {
                    continue;
                };
                ctl.items.push(ItemSpec {
                    id: rd16(&one, f("wID")).unwrap_or(0),
                    text: rd16(&one, f("wTextID")).unwrap_or(0),
                    icon: rd16(&one, f("wIconID")).unwrap_or(0),
                    data: rd32(&one, f("dwData")).unwrap_or(0),
                });
            }
            spec.controls.push(ctl);
        }
        Ok(Some(spec))
    }

    /// `IControl` and the classes that inherit it.
    ///
    /// The nine inherited slots mean the same thing in every control, so they are answered once
    /// here; what differs is what a class holds and what it paints, which is `class` — the interface
    /// the object was handed out for, not the one the call came in through. `Redraw` is the only
    /// method that needs the screen, and it takes the applet's default display, because a control
    /// draws where the applet that made it draws.
    pub(crate) fn iwidget(
        &mut self,
        id: u32,
        this: u32,
        label: &str,
        name: &str,
        class: &'static str,
    ) -> HResult<u32> {
        let gap = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let disp = self.display.default_display();
        match name {
            "HandleEvent" => {
                // `AClock_HandleEvent` returns FALSE for every event: the
                // clock's own door takes nothing, not even the `EVT_CTL_SET_*` the other controls
                // answer, so the class is checked before any of that runs.
                if class == "IAClockCtl" {
                    return Ok(0);
                }
                if matches!(class, "ITimeCtl" | "IDateCtl") && self.widgets.is_active(this)? == 0 {
                    return Ok(0);
                }
                // `(evt, wParam, dwParam)`; on a control's key the code is in `wParam`. The three
                // codes a dialog uses to fill a control are answered on the same door, because a pack
                // sends them to a control it made itself: `EVT_CTL_SET_TITLE`, `EVT_CTL_SET_TEXT` and
                // `EVT_CTL_ADD_ITEM`, which is what the shipped controls' own event doors take.
                let [evt, code, dwp] = self.args()?;
                Ok(u32::from(self.widget_handle_event(this, evt, code, dwp)?))
            }
            "Redraw" => {
                if class == "IAClockCtl" {
                    return self.aclock_redraw(this);
                }
                // `HVP_TRANSPARENT`: the shipped `Redraw` opens with
                //
                //     if (properties & 0x10) { record.code = 10; notify(...); }
                //
                // so the request to erase what is *under* the page goes out before the page paints,
                // and the painter then draws no paper of its own. Sent from this door rather than
                // from `Widgets::redraw` because the callback is guest code and only the runtime
                // can run it; the property is read here for the same reason the fill is read there
                // — a pack may set it between frames.
                if class == "IHtmlViewer" {
                    if self.widgets.properties(this)? & g::HVP_TRANSPARENT != 0 {
                        self.html_notify(this, g::HVN_ERASEBKGND, 0)?;
                    }
                    // `case 9` of the shipped layout loop loads `<body background>` through the
                    // shell, not through the network. It has to be settled before the redraw walks
                    // the picture cache. The transparent branch above does not skip it: the load
                    // happens at layout time, and only the *painting* of it is replaced.
                    self.html_load_background(this)?;
                    let now = self.clock_ms;
                    let html = self.widgets.html(this)?;
                    let epoch = *html.marquee_epoch.get_or_insert(now);
                    html.marquee_elapsed = now.saturating_sub(epoch);
                }
                Ok(self
                    .widgets
                    .redraw(&mut self.cpu.mem, &mut self.display, disp, this)?)
            }
            "SetActive" => {
                let [on] = self.args()?;
                // `AClock_SetActive` is empty.
                if class == "IAClockCtl" {
                    return Ok(0);
                }
                // A menu and a static repaint from inside this call (`IMenuCtl_SetActive` →
                // `IMenuCtl_Redraw` → `Menu_Draw`; `AStatic_SetActive` → `AStatic_Redraw`), and the
                // two differ in what the repaint means: `Menu_Draw` ends with
                // `IDisplay::Update(po, TRUE)`, so a menu paints *and asks for the frame*, while
                // `AStatic_Redraw` only writes pixels and leaves the screen to whoever owns it.
                //
                // This is the whole of how a pack that never calls `IDisplay_Update` itself still
                // gets a picture: `chess.mod` builds six menu items, calls `SetActive` last, and
                // then goes idle — the six items are all it ever asks for. See the note on
                // `Widgets::set_active` for why only those two classes answer TRUE.
                if self.widgets.set_active(this, on, self.is_stopwatch(this))? {
                    let disp = self.display.default_display();
                    self.widgets
                        .redraw(&mut self.cpu.mem, &mut self.display, disp, this)?;
                    if class == "IMenuCtl" {
                        self.display.update(&self.cpu.mem, disp, 1)?;
                    }
                }
                Ok(0)
            }
            "IsActive" => {
                // `AClock_IsActive` answers FALSE whatever happened.
                if class == "IAClockCtl" {
                    return Ok(0);
                }
                self.widgets.is_active(this)
            }
            "SetRect" => {
                let [prc] = self.args()?;
                if class == "IAClockCtl" {
                    // `AClock_SetRect` keeps the caller's origin and squares the size
                    // through `CalcOffset`; it dereferences `prc` unguarded, so a null one is ignored
                    // rather than followed.
                    if prc != 0 {
                        let rc = Rect::read(&self.cpu.mem, prc)?;
                        let mut clock = self.clocks.get(this).ok_or_else(|| gap())?;
                        clock.face_rect = aclock::square(rc);
                        self.clocks.put(this, clock);
                        // The generic record carries the same square, so a reader that asks the
                        // control's rect (the dialog's layout does) and the clock's own `GetRect`
                        // cannot disagree about where it is.
                        self.widgets.place(this, clock.face_rect)?;
                    }
                    return Ok(0);
                }
                self.widgets.set_rect(&self.cpu.mem, this, prc)?;
                // A page is laid out in this rectangle, so a new one changes what fits: the
                // offset it was scrolled to may name a place that no longer exists, and the
                // focused row may be outside it. Both are fixed where the rectangle changed
                // rather than on the next frame, which must not move the page by itself.
                if class == "IHtmlViewer" {
                    self.html_fit(this)?;
                }
                Ok(0)
            }
            "GetRect" => {
                let [prc] = self.args()?;
                if class == "IAClockCtl" {
                    // The face rect, not the rect the caller set: `AClock_GetRect` copies
                    // `m_rcFace`, which is the square.
                    if prc != 0 {
                        let clock = self.clocks.get(this).ok_or_else(|| gap())?;
                        clock.face_rect.write(&mut self.cpu.mem, prc)?;
                    }
                    return Ok(0);
                }
                self.widgets.get_rect(&mut self.cpu.mem, this, prc)?;
                Ok(0)
            }
            "SetProperties" => {
                let [props] = self.args()?;
                if class == "IAClockCtl" {
                    // The delivered body
                    // static's, and no reset on any bit.
                    let mut clock = self.clocks.get(this).ok_or_else(|| gap())?;
                    clock.props = props;
                    self.clocks.put(this, clock);
                    // One word in the shipped object (`m_dwProps`); the generic record keeps a copy
                    // for the readers that only know about controls.
                    self.widgets.set_properties(this, props)?;
                    return Ok(0);
                }
                // The shipped viewer re-runs its whole `SetRect` when the scrollbar bit flips —
                // that bit decides how wide the page is laid out, so the change is a layout
                // change, not a paint hint.
                let old_props = self.widgets.properties(this)?;
                self.widgets.set_properties(this, props)?;
                if class == "IHtmlViewer" && (old_props ^ props) & g::HVP_SCROLLBAR != 0 {
                    self.html_fit(this)?;
                }
                Ok(0)
            }
            "GetProperties" => {
                if class == "IAClockCtl" {
                    // The word the clock was born with until a caller replaces it.
                    return Ok(self.clocks.get(this).ok_or_else(|| gap())?.props);
                }
                self.widgets.properties(this)
            }
            "Reset" => {
                // The shipped `IHtmlViewer_Reset` is its own body, not the generic one:
                // `Dox_Reset` frees the document, `Dox_SetUrl(dox, 0)` clears the base URL the
                // next relative `href` would resolve against, the scroll timer is cancelled and
                // the pen state is zeroed. The header adds that
                // the control "immediately" leaves active/focus mode.
                if class == "IHtmlViewer" {
                    self.html_cancel_fetches(this);
                    self.widgets.html_reset_viewer(this)?;
                    return Ok(0);
                }
                self.widgets.reset(this)?;
                if let Some(mut clock) = self.clocks.get(this) {
                    // `AClock_Reset` lets the face picture go and cancels the timer;
                    // nothing else — the time and the colours stay.
                    let prev = clock.face_image;
                    clock.face_image = 0;
                    clock.started = false;
                    self.clocks.put(this, clock);
                    self.hold(prev, -1)?;
                }
                Ok(0)
            }
            "EnableCommand" => {
                // Two published shapes for one slot name: `IMenuCtl::EnableCommand` is
                // `(boolean bEnable)` alone, while the date, text and time controls publish
                // `(boolean bEnable, uint16 nCmdId)` — and the delivered body shows what the second
                // one is for: the flag gates a command the control *posts*, and the id is kept only
                // while enabling. Taking one argument from the two-argument form dropped the id.
                //
                // The menu's one argument gates the same post (the delivered body stores only
                // `m_bSendCmd`) — it is **not** an enable of the control, and treating it as one
                // switched the cursor keys off, which is a thing no line of that body does.
                let on = self.arg(1)?;
                if class == "IMenuCtl" {
                    self.widgets.set_send_cmd(this, on)?;
                } else if class == "ITextCtl" {
                    // `CTextCtl_EnableCommand`'s whole body is inside `#if 0 // Not going to be
                    // supported`: the slot exists and does nothing, so
                    // storing an id here would be this host inventing a command the body never sends.
                } else {
                    let cmd = self.arg(2)? as u16;
                    self.widgets.set_command(this, on, cmd)?;
                }
                Ok(0)
            }
            "SetTitle" => {
                let [_resfile, _resid, text] = self.args()?;
                self.widgets.set_title(&mut self.cpu.mem, this, text)
            }
            "SetFont" => {
                let [text, title] = self.args()?;
                self.widgets.set_font(this, text, title)?;
                Ok(0)
            }
            "GetFont" => {
                let [ptext, ptitle] = self.args()?;
                // All-or-nothing, like the delivered body and the delivered body: either pointer
                // missing is FALSE and **neither** is written. Filling the one the caller did give
                // would answer a half-question. Only the date and time controls publish this slot.
                if ptext == 0 || ptitle == 0 {
                    return Ok(0);
                }
                let (text, title) = self.widgets.fonts(this)?;
                for (at, font) in [(ptext, text), (ptitle, title)] {
                    self.cpu.mem.write_u32(at, font)?;
                }
                Ok(1)
            }
            // `IStatic::SetText` carries a title, a body and two fonts; `ITextCtl::SetText` carries
            // a string and a length. Both arrive as a `SetText` on an `IControl`.
            "SetText" if class == "IStatic" => {
                let [title, text, font_title, font_text] = self.args()?;
                self.widgets.static_set_text(
                    &mut self.cpu.mem,
                    this,
                    title,
                    text,
                    font_title,
                    font_text,
                )
            }
            "SetText" if class == "ITextCtl" => {
                let [text, cch] = self.args()?;
                self.widgets
                    .edit_text(&mut self.cpu.mem, this, text, cch as i32)
            }
            "SetText" => {
                let [text] = self.args()?;
                self.widgets.set_text(&mut self.cpu.mem, this, text)
            }
            "SetTextEx" => {
                // `(pbText, pStream, bAppend)`. A stream would have to be read through the module that
                // owns it, which this host cannot do synchronously; the delivered body answers
                // FALSE when it has neither text nor stream, and a code the pack survives is the right
                // shape for that gap — a host error here would end the run instead.
                let [text, stream, append] = self.args()?;
                if stream != 0 {
                    return Ok(0);
                }
                //: without the append flag the old text is reset first; with it the new
                // text is added to what the control already holds.
                if append != 0 {
                    return self.widgets.append_text(&mut self.cpu.mem, this, text);
                }
                self.widgets.set_text(&mut self.cpu.mem, this, text)
            }
            "GetText" => {
                let [buffer, max] = self.args()?;
                self.widgets
                    .copy_text(&mut self.cpu.mem, this, buffer, max as i32)
            }
            "GetTextPtr" => self.widgets.text(this),
            "SetMaxSize" => {
                let [max] = self.args()?;
                self.widgets.set_max_chars(this, max)?;
                Ok(0)
            }
            "GetCursorPos" => self.widgets.caret(this),
            "SetCursorPos" => {
                let [at] = self.args()?;
                self.widgets.set_caret(this, at)?;
                Ok(0)
            }
            "SetSelection" => {
                let [bits] = self.args()?;
                self.widgets.set_selection(this, bits)?;
                Ok(0)
            }
            "GetSelection" => self.widgets.selection(this),
            // `IStatic::GoToLine(int nLineIdx)`. Kept next to `SizeToFit`, the other `IStatic` slot
            // that is about the control's own layout rather than its text. The header makes the answer
            // the *previous* line, so the state lives in the control and
            // this arm is a pass-through.
            //
            // ⚠ No `if class == "IStatic"` guard on this arm, and that is deliberate rather than lazy:
            // only `IStatic` declares `GoToLine` (`src/abi/generated/ifaces.rs:11751`), so the guard
            // would test nothing — and `tools/abi_coverage.py`'s arm pattern reads the `if` condition
            // as `if[^\n=]{0,120}`, so a `==` in it makes the arm invisible to the coverage tool. An
            // arm that is correct and uncounted is worse than one that is simply correct.
            "GoToLine" => {
                let [line] = self.args()?;
                self.widgets.goto_line(
                    &mut self.cpu.mem,
                    &mut self.display,
                    disp,
                    this,
                    line as i32,
                )
            }
            "SizeToFit" => {
                let [prc] = self.args()?;
                // The date and static controls fit from their title and body; the time control fits
                // the string it would draw, whose flags depend on which of its three classes this is.
                //The branch is inside the arm on purpose: `if class == …` in
                // the arm pattern makes the arm invisible to `tools/abi_coverage.py`.
                if class == "ITimeCtl" {
                    let clsid = self.ctl_clsid(this);
                    self.widgets.time_size_to_fit(
                        &mut self.cpu.mem,
                        this,
                        prc,
                        clsid == g::AEECLSID_CLOCKCTL,
                        clsid == g::AEECLSID_STOPWATCHCTL,
                    )?;
                } else {
                    self.widgets.size_to_fit(
                        &mut self.cpu.mem,
                        &mut self.display,
                        disp,
                        this,
                        prc,
                    )?;
                }
                Ok(0)
            }
            "IsScrollable" => {
                self.widgets
                    .is_scrollable(&mut self.cpu.mem, &mut self.display, disp, this)
            }
            "AddItem" => {
                // `IMenuCtl_AddItem(p, pszResText, wText, wItemID, pText, dwData)` carries both ways
                // to name a label, and a pack may use either: `pText` is the string itself, while
                // `pszResText` + `wText` name a record in a resource file. `IMenuCtl_AddItemEx`
                // (`0x60c`) resolves the pair through `IShell::LoadResString` when
                // `pText` is null and `wText` is neither 0 nor `0xFFFF`, which is the *only* shape
                // `chess.mod` uses: it passes six resource ids out of `chess_comb.bar` and no
                // `pText` at all, so dropping the pair dropped every one of its menu items.
                let [res_text, w_text, item_id, text, data] = self.args()?;
                let text = if text == 0 && res_text != 0 && w_text != 0 && w_text != 0xffff {
                    let named = self.res_named(res_text);
                    self.res_string(named.as_deref(), w_text as u16)?
                } else {
                    text
                };
                self.widgets
                    .add_item(&mut self.cpu.mem, this, item_id as u16, text, data)
            }
            "AddItemEx" => {
                let [pai] = self.args()?;
                self.widgets.add_item_ex(&mut self.cpu.mem, this, pai)
            }
            "GetItemData" => {
                let [item_id, out] = self.args()?;
                self.widgets
                    .item_data(&mut self.cpu.mem, this, item_id as u16, out)
            }
            "SetItemText" => {
                let [item_id, _resfile, _resid, text] = self.args()?;
                self.widgets
                    .set_item_text(&mut self.cpu.mem, this, item_id as u16, text)?;
                Ok(0)
            }
            "DeleteItem" => {
                let [item_id] = self.args()?;
                self.widgets.delete_item(this, item_id as u16)
            }
            "DeleteAll" => self.widgets.delete_all(this),
            "SetSel" => {
                let [item_id] = self.args()?;
                self.widgets.set_sel(this, item_id as u16)?;
                Ok(0)
            }
            "SetSelEx" => {
                let [item_id, selected] = self.args()?;
                if selected != 0 {
                    self.widgets.set_sel(this, item_id as u16)?;
                }
                Ok(0)
            }
            "GetSel" => self.widgets.sel(this),
            "GetItemCount" => self.widgets.item_count(this),
            "GetItemID" => {
                let [index] = self.args()?;
                self.widgets.item_id(this, index as i32)
            }
            "GetItemRect" => {
                let [index, prc] = self.args()?;
                self.widgets
                    .item_rect(&mut self.cpu.mem, this, index as i32, prc)?;
                Ok(0)
            }
            "EnumSelInit" => self.widgets.enum_sel_init(this),
            "EnumNextSel" => self.widgets.next_sel(this),
            // The item record round-trips through `CtlAddItem` in both directions, so a caller that
            // reads an item out and writes it back gets the same three words it handed in.
            "GetItem" => {
                let [item_id, pitem] = self.args()?;
                self.widgets
                    .get_item(&mut self.cpu.mem, this, item_id as u16, pitem)
            }
            "SetItem" => {
                let [item_id, flags, pitem] = self.args()?;
                self.widgets
                    .set_item(&mut self.cpu.mem, this, item_id as u16, flags as u16, pitem)
            }
            "MoveItem" => {
                let [item_id, dir] = self.args()?;
                self.widgets.move_item(this, item_id as u16, dir as i32)
            }
            "Sort" => {
                let [kind, _pfn, _puser] = self.args()?;
                if kind == g::MCS_CUSTOM {
                    // `MCS_CUSTOM` asks for the pack's own comparison callback. Calling guest code from
                    // inside a sort is not something this host does yet, and ordering by a key the caller
                    // did not name would be a wrong list rather than a missing one.
                    return Err(gap());
                }
                self.widgets.sort_items(&self.cpu.mem, this, kind)?;
                Ok(0)
            }
            "GetStyle" | "SetStyle" => {
                // The control's two style records, in the eight-byte shape the shipped body moves —
                // the pair `MenuCtl_New` seeded from `IShell::GetItemStyle` and the caller has since
                // been able to overwrite, half at a time. Each pointer is handled on its own in
                // [`crate::widget`], which is what both bodies do.
                let [normal, selected] = self.args()?;
                if name == "SetStyle" {
                    self.widgets
                        .set_style(this, &self.cpu.mem, normal, selected)?;
                } else {
                    self.widgets
                        .get_style(this, &mut self.cpu.mem, normal, selected)?;
                }
                Ok(0)
            }
            "SetColors" => {
                let [colors] = self.args()?;
                self.widgets.set_colors(this, &self.cpu.mem, colors)?;
                Ok(0)
            }
            "GetColors" => {
                let [colors] = self.args()?;
                self.widgets.menu_colors(this, &mut self.cpu.mem, colors)?;
                Ok(0)
            }
            "SetItemTime" => {
                let [item_id, t0, t1] = self.args()?;
                self.widgets
                    .set_item_time(this, item_id as u16, t0 as u16, t1 as u16)?;
                Ok(0)
            }
            "GetItemTime" => {
                let [item_id, out] = self.args()?;
                self.widgets
                    .item_time(&mut self.cpu.mem, this, item_id as u16, out)
            }
            "SetOwnerDrawCB" => {
                let [cb, data, mask] = self.args()?;
                self.widgets.set_owner_draw(this, cb, data, mask as u16)
            }
            "SetJulianDay" => {
                let [day] = self.args()?;
                self.widgets.set_julian_day(this, day as i32)
            }
            "GetJulianDay" => self.widgets.julian_day(this),
            "SetDate" => {
                let [year, month, day] = self.args()?;
                self.widgets
                    .set_date(this, year as i32, month as i32, day as i32)
            }
            "GetDate" => {
                let [py, pm, pd] = self.args()?;
                self.widgets.date(&mut self.cpu.mem, this, py, pm, pd)
            }
            "GetDayOfWeek" => self.widgets.day_of_week(this),
            "GetDayString" => {
                let [buf, max, pchars] = self.args()?;
                self.widgets
                    .day_string(&mut self.cpu.mem, this, buf, max as i32, pchars)
            }
            "GetMonthString" => {
                let [buf, max, pchars] = self.args()?;
                self.widgets
                    .month_string(&mut self.cpu.mem, this, buf, max as i32, pchars)
            }
            "GetDateString" => {
                let [buf, max, pchars, format] = self.args()?;
                self.widgets
                    .date_string(&mut self.cpu.mem, this, buf, max as i32, pchars, format)
            }
            "SetActiveDayMask" => {
                let [mask] = self.args()?;
                self.widgets.set_day_mask(this, mask)?;
                Ok(0)
            }
            // `ITimeCtl::SetTime(dwMSecs, bIncUpdate)` — the delivered body. The second word decides
            // only the shipped member's drawn-string cache and whether its tail `TimeCtl_Redraw`
            // repaints in place, so it is not a state this host keeps; the first is bounded by the
            // class: only the clock refuses anything past one day, and all three refuse a negative.
            "SetTime" => {
                let tod = self.arg(1)?;
                let clsid = self.ctl_clsid(this);
                self.widgets
                    .set_time(this, tod as i32, clsid == g::AEECLSID_CLOCKCTL)
            }
            "GetTime" => self.widgets.time(this),
            // `GetTimeString(dwMSecs, pDest, nSize, wFlags)` is `void`: it writes the
            // caller's buffer and answers nothing. The number it formats is the **argument**, not the
            // control's stored time — the delivered body takes `dwSecs` and never reads `m_dwTime` for it.
            "GetTimeString" => {
                let [ms, dest, size, flags] = self.args()?;
                let active = self.widgets.is_active(this)? != 0;
                self.widgets.time_string(
                    &mut self.cpu.mem,
                    dest,
                    size,
                    i64::from(ms),
                    flags,
                    active,
                )?;
                Ok(0)
            }
            "SetIncrement" => {
                let mins = self.arg(1)?;
                self.widgets.set_increment(this, mins as u16)?;
                Ok(0)
            }
            "SetEditField" => {
                let field = self.arg(1)?;
                self.widgets.set_edit_field(this, field)?;
                Ok(0)
            }
            // `IImageCtl::SetImage(pi)`: the shipped body releases what it held and takes a reference
            // on the new picture, so an image a pack handed to a control
            // and then dropped its own reference to stays alive. `pi == 0` is a normal call.
            "SetImage" => {
                let [img] = self.args()?;
                let prev = self.widgets.set_image(this, img)?;
                for (at, delta) in [(prev, -1), (img, 1)] {
                    if at != 0 {
                        if let Some(object) = self.hle.object(at).copied() {
                            self.change_refs(&object, delta)?;
                        }
                    }
                }
                Ok(0)
            }
            "SetRedraw" => {
                let [cb, user] = self.args()?;
                self.widgets.set_redraw(this, cb, user)?;
                Ok(0)
            }
            "SetSoftKeyMenu" => {
                let [menu] = self.args()?;
                // The delivered body
                // though this host paints no soft keys. The mode row the body also adds
                // needs a localised mode string this host has none of, so it
                // leaves that field alone.
                let prev = self.widgets.set_softkey_menu(this, menu)?;
                self.hold(menu, 1)?;
                self.hold(prev, -1)?;
                Ok(0)
            }
            "SetInputMode" => {
                let [mode] = self.args()?;
                self.widgets.set_input_mode(this, mode)
            }
            "GetInputMode" => {
                let [info] = self.args()?;
                self.widgets.input_mode(&mut self.cpu.mem, this, info)
            }
            "EnumModeInit" => {
                self.widgets.enum_mode_init(this)?;
                Ok(0)
            }
            "EnumNextMode" => {
                let [info] = self.args()?;
                self.widgets.enum_next_mode(&mut self.cpu.mem, this, info)
            }
            "SetPropertiesEx" => {
                let [props] = self.args()?;
                self.widgets.set_props_ex(this, props)?;
                Ok(0)
            }
            "GetPropertiesEx" => self.widgets.props_ex(this),
            // `IHtmlViewer`'s own slots 11-23. `CreateInstance` recorded the class as
            // `IHtmlViewer`, so these only arrive on a control that class made.
            "SetOption" | "GetOption" | "LoadStream" | "LoadSource" | "SetNotifyFn" | "SetData"
            | "IsBusy" | "Stop" | "FindElem" | "GetElemText" | "GetAttrValue" | "SetViewState"
            | "GetViewState" => self.ihtmlviewer(id, this, label, name),
            _ => Err(gap()),
        }
    }

}
