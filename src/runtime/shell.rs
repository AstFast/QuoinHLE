//! The shell's own bookkeeping: starting and closing an applet, adopting the
//! object a module built, posting events and the auto-start search.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// Record that the module took a class this host forwarded it: what it answered, and how much of
    /// the interface the object it published can actually serve. See [`AcceptedClass`].
    ///
    /// The evidence that it *took* the class, rather than refusing it, is one of three things: it
    /// published an object, it stopped inside its own code, or it reached the host while deciding.
    /// The last of those is the one no return value can settle. A module ends
    /// `AEEModule_CreateInstance` by handing any class it does not own back to `IShell`, so a
    /// constructor that ran and *then* failed leaves the same answer as a class that was never the
    /// module's; and a constructor that got that far said something on the way — to the shell, to
    /// `AEEHelperFuncs`. What separates them is exactly that: reaching the host past the one dispatch
    /// a hand-back costs means the module's own code ran for this id.
    ///
    /// A refusal that asked for nothing is therefore not recorded, and that is not caution for its own
    /// sake. `--start auto` offers every id-shaped word in the image, and in a classic ER_RO that
    /// includes the module's own instructions — one store binary scans 852 of them, `0x012fff1e`
    /// among them, which is `bx lr` — and answers every stranger with `AEE_EFAILED`. Recording those
    /// would have the report say the module took 852 classes, which is the same lie in the other
    /// direction. What it *did* take, its own applet class, is in the list: its constructor
    /// reached `malloc` and `dbgprintf` before it failed.
    pub(crate) fn note_accepted_class(
        &mut self,
        cls: u32,
        ppv: u32,
        outcome: &Result<u32, Error>,
        reached_host: bool,
    ) {
        let took = match outcome {
            Err(_) => true,
            Ok(rv) if *rv == g::AEE_SUCCESS => true,
            Ok(_) => reached_host,
        };
        // Recorded before the early return: "did the module take this class" is a decision
        // `discover_applet_class` makes, and a refusal is the answer it has to be able to see.
        self.last_class_taken = took;
        if !took {
            return;
        }
        if let Some(seen) = self.accepted_classes.iter_mut().find(|a| a.cls == cls) {
            seen.times += 1;
            return;
        }
        if self.accepted_classes.len() >= ACCEPTED_CLASS_CAP {
            self.accepted_classes_dropped += 1;
            return;
        }
        // Only a success is read for an object. A refusal's slot is cleared by the caller that asked
        // the module, so anything its constructor wrote there is gone by the time this is reached, and
        // a half-built applet's vtable is not a fact about the class.
        let object = match outcome {
            Ok(rv) if *rv == g::AEE_SUCCESS => self.cpu.mem.read_u32(ppv).unwrap_or(0),
            _ => 0,
        };
        let (rv, fault) = match outcome {
            Ok(rv) => (Some(*rv), None),
            Err(e) => (None, Some(e.to_string())),
        };
        self.accepted_classes.push(AcceptedClass {
            cls,
            name: abi::clsid_name(cls),
            rv,
            fault,
            object,
            vtable_words: self.vtable_words(object),
            published: published_iface(cls),
            times: 1,
        });
    }

    /// The accepted classes the report may accuse: an id the platform publishes an interface for, whose
    /// object's vtable is shorter than that interface. Nothing else belongs here — see
    /// [`Runtime::accepted_unjudged_classes`], and see [`AcceptedClass::too_short`] for why the two
    /// are separable by this one test.
    pub fn accepted_short_classes(&self) -> Vec<&AcceptedClass> {
        self.accepted_classes
            .iter()
            .filter(|a| a.too_short())
            .collect()
    }

    /// The accepted classes the report cannot judge, which is the complement of
    /// [`Runtime::accepted_short_classes`] and the majority of any real run: a module's own class ids
    /// are in no header, so there is nothing for its object's word count to be measured against, and
    /// a working applet lands here. Together the two lists are `accepted_classes` in full, so a
    /// heading count over either one is a count over that group alone.
    pub fn accepted_unjudged_classes(&self) -> Vec<&AcceptedClass> {
        self.accepted_classes
            .iter()
            .filter(|a| !a.too_short())
            .collect()
    }

    /// Claim a module-built object as an instance of the interface the running slot was addressed
    /// through, so the slots after this one can be served.
    ///
    /// A module that fabricates a class answers it with its own object, whose vtable is the module's
    /// code and whose fields are the module's layout — so its `IHIDDevice` slots have to be answered
    /// out of *its* words, which is what [`crate::display::Display::display_for`] and the rest of the
    /// object methods do once `Hle` knows the address. `adopt_applet` refuses to register exactly
    /// that kind of object, and for the opposite reason: an applet owns its event loop and its own
    /// `Release`, while a device, a stream or a control is nothing but the fields its header names.
    ///
    /// It registers only on the same evidence `adopt_applet` would accept, and that is the whole of
    /// the safety: the object has to name this shell in `m_pIShell` and this module in `m_pIModule`,
    /// which a null, a vtable pointer or a word that was never an object cannot do. Those keep
    /// answering `NotAnObject`, which is what they answered before, and what the crash report reads
    /// as the null it is.
    pub(crate) fn adopt_module_object(
        &mut self,
        this: u32,
        iface: Option<&'static str>,
    ) -> Result<(), Error> {
        let Some(iface) = iface.filter(|i| is_an_abi_name(i)) else {
            return Ok(());
        };
        let mut hit = None;
        for layout in AppletLayout::ALL {
            if self.cpu.mem.read_u32(this + layout.shell)? != self.shell {
                continue;
            }
            let module = self.cpu.mem.read_u32(this + layout.module)?;
            if self.owns_module(module) {
                hit = Some(layout);
                break;
            }
        }
        let Some(layout) = hit else {
            return Ok(());
        };
        self.hle.adopt_object(this, iface, 0, layout.refs);
        Ok(())
    }

    /// Record a resource the pack asked for and this host could not find, naming the file it asked
    /// for and the kind it asked for it under.
    ///
    /// The kind belongs on the line because the id alone cannot be checked against a container: the
    /// map files one id under whichever kind the packer chose, and a font table may sit under the
    /// *image* kind. Without the number a reader has to go and re-run the pack with `--args` to find
    /// out what was asked.
    pub(crate) fn want_resource(&mut self, file: u32, id: u32, asked_kind: u32) -> String {
        let name = self
            .res_named(file)
            .unwrap_or_else(|| String::from("<applet>"));
        let kind = match asked_kind as u16 {
            resource::RESTYPE_STRING => "string",
            resource::RESTYPE_IMAGE => "image",
            resource::RESTYPE_BINARY => "binary",
            _ => "",
        };
        let asked = if kind.is_empty() {
            format!("{name}#{id:#06x} (type {asked_kind:#06x})")
        } else {
            format!("{name}#{id:#06x} ({kind})")
        };
        if !self.missing_resources.contains(&asked) {
            self.missing_resources.push(asked);
        }
        name
    }

    /// A run that moves its drawing window gets one line in the report. `Translate` is taken as an
    /// absolute origin and `Pan` as a relative one, which is what distinguishes the two words, and a
    /// pack that meant it the other way is visible here rather than silently drawing in the wrong
    /// place.
    pub(crate) fn note_viewport(&mut self, method: &str, r: &Rect) {
        let whole = Rect::whole(self.device.width, self.device.height);
        if *r != whole {
            self.display.note(format!(
                "IGraphics::{method} viewport to ({},{}) {}x{}",
                r.x, r.y, r.dx, r.dy
            ));
        }
    }

    /// Claim a module-created object as *the* applet, after checking that it is one.
    ///
    /// The check is not distrust of the guest in general: `_AEEApplet`'s shell and module pointers
    /// are what `ISHELL_GETAPP`-style code and a later `Release` depend on, and an object that does
    /// not name *this* shell would be handed an event loop it has no business running. Deliberately
    /// absent is any [`crate::hle`] registration: the vtable, the reference count and the
    /// destructor are all guest code, and a host record claiming them would let the runtime answer
    /// `Release` for an object it does not own.
    pub(crate) fn adopt_applet(&mut self, applet: u32, cls: u32) -> Result<(), Error> {
        // A started applet is not the shell's idle state any more, even when the previous one
        // returned there.
        self.idle = false;
        // Which shape this module built is answered by the applet's own contents: `m_pIShell` is
        // the one field every shape holds *this* shell in, and `m_pIModule` has to name a module
        // this run loaded. Trying the layouts in turn instead of assuming one is what keeps a
        // working MP applet from being reported as "not an AEEApplet": its `m_pIShell` sits at 24
        // with `m_pIModule` *before* it at 20, which is not a shift of the platform's offsets and so was
        // invisible to the two-candidate test that preceded this table.
        //
        // Each candidate's `m_pIShell` word is read as the walk goes, so a refusal can report all
        // three rather than only that the object was rejected — that word is usually the whole
        // diagnosis. A match ends the walk early: the answer is known, and the offsets are only
        // meaningful up to the size of the object being tried, so reading on would quote whatever
        // the heap put next door as if it were a field.
        let mut seen = [0u32; 4];
        let mut claimed = 0u32;
        let mut hit: Option<(AppletLayout, u32, bool)> = None;
        for (i, layout) in AppletLayout::ALL.iter().enumerate() {
            seen[i] = self.cpu.mem.read_u32(applet + layout.shell)?;
            if seen[i] != self.shell {
                continue;
            }
            let module = self.cpu.mem.read_u32(applet + layout.module)?;
            if self.owns_module(module) {
                hit = Some((*layout, module, true));
                break;
            }
            // A module this host never adopted, but shaped like one, is what a self-unpacking
            // pack answers with — see `module_shaped`. Keep walking: a later layout whose module
            // *is* held is a better answer than this one.
            if self.module_shaped(module) {
                hit.get_or_insert((*layout, module, false));
                continue;
            }
            if claimed == 0 {
                claimed = module;
            }
        }
        // A module that publishes actors, extensions or service factories instead of an applet
        // answers a class with an object of a different shape. Saying "not a host object" here named
        // the wrong suspect — see `hle::Error::NotAnApplet` for the words the report now uses.
        let Some((layout, module, known)) = hit else {
            return Err(Error::Hle(HleError::NotAnApplet {
                addr: applet,
                cls,
                seen,
                module: claimed,
            }));
        };
        self.applet_layout = layout;
        // The applet's own `m_pIModule` is what "the running module" means from here: a class asked
        // for after this start is resolved against the module that built the applet, not the one the
        // run was launched with.
        //
        // With one exception, and it is the self-unpacking pack: the `.mod` the loader mapped is
        // only an unpacker that inflates the real image and jumps into it, so the applet names an
        // `IModule` the guest built afterwards rather than one this host adopted. That word is a
        // good enough record that the object *is* an applet (`module_shaped` is what reads it), but
        // it cannot be the pointer a later class question goes through — the registry has no entry
        // for it. The unpacker answers in its place: its own `CreateInstance` is what built this
        // object, and on the handset the payload is not a module the shell ever knew either.
        self.run_module = if known {
            module
        } else {
            self.created_module().unwrap_or(0)
        };
        self.applet = applet;
        self.app_cls = cls;
        // From here until `EVT_APP_START` has been answered the applet is starting, not yet the
        // top visible one: a pack that asks about itself from its own `EVT_APP_START` handler sees
        // `APPSTATE_STARTING` on a handset, and the shell records the same uptime it hands back
        // later as the start time.

        Ok(())
    }

    /// `IShell::StartApplet(cls, options, pszArgs)`.
    ///
    /// Four documented details fix the order of this function. The module is asked for the applet
    /// *before* the screen is cleared, because its constructor is what creates the display and thus
    /// the colour table the clear reads. The clear itself is skipped for `AEE_AFLAG_POPUP`. The
    /// applet is told *why* it started through an `AEEAppStart`, whose `pDisplay` is the display the
    /// applet now holds. And the real call is *asynchronous*: its `AEEResult` says whether the
    /// applet could be created at all, never what it drew. This host runs the start event to
    /// completion before answering, which is the one deliberate simplification — and the reason a
    /// headless run's screenshot is reproducible.
    pub fn start_applet(&mut self, cls: u32, options: u32, psz_args: u32) -> Result<u32, Error> {
        let ppv = self.cpu.mem.alloc_zeroed(4)?;
        // Every construction below — the running module's and each library's — is an *applet* being
        // built, which `get_app_instance` has to be able to tell from an ordinary object.
        self.starting_applets += 1;
        self.constructing_applets += 1;
        let created = self.create_applet_object(cls, ppv);
        self.starting_applets -= 1;
        self.constructing_applets -= 1;
        let rv = created?;
        if rv != g::AEE_SUCCESS {
            return Ok(rv);
        }
        let applet = self.cpu.mem.read_u32(ppv)?;
        if applet == 0 {
            return Ok(g::AEE_EFAILED);
        }
        self.start_created(applet, cls, options, psz_args)
    }

    /// The half of `IShell::StartApplet` that happens once the module has produced an object.
    pub(crate) fn start_created(
        &mut self,
        applet: u32,
        cls: u32,
        options: u32,
        psz_args: u32,
    ) -> Result<u32, Error> {
        self.adopt_applet(applet, cls)?;
        // The constructor has finished; a `constructor`-scoped legacy answer stops here, before
        // the applet's own `EVT_APP_START` loads any more images.
        if self.oem_until_applet {
            self.oem_native_images = false;
            self.oem_until_applet = false;
        }
        // The shell links a history entry for every applet it starts, at the head of the chain —
        // which is what `gBREW[238]` is, and therefore what a history object's cursor is born
        // pointing at (`AEEAppHistory_New`'s `v5[2] = gBREW[238]`). The launch string is the one this
        // call was given, copied the way `AHistory_New` copies it.
        let args = if psz_args == 0 {
            None
        } else {
            let text = self.cstring(psz_args)?;
            (!text.is_empty()).then_some(text)
        };
        self.history.push_head(cls, args);
        // `AHistory_New` ends by waking every registered model listener (`for i = bss$2 …
        // AEE_ResumeCallback`), so a start is a model change the way `Insert` and `Remove` are.
        self.notify_history_listeners()?;
        self.attach_scripts();
        if options & g::AEE_AFLAG_POPUP == 0 {
            self.clear_screen()?;
        }
        let start = self.write_app_start(cls, psz_args)?;
        // `EVT_APP_START` first, because that is the event the ABI fixed and the one every classic
        // applet answers. Brew MP's *window* applications are the exception: they are started with
        // `EVT_APP_START_WINDOW` ("Start event for window application") and treat
        // the classic event as one they were not written for. Both MP packs in the sample set say so in
        // their own words — `settingsapp` returns `FALSE` to it from a handler that logs nothing and
        // asks this host for nothing, and `contactapp` prints
        // `************EVT_APP_START_WINDOW************` the moment it is
        // handed the windowed one — so the applet is the record, and a refusal from a MP-shaped
        // applet is retried once with the windowed event before it is taken as a refusal.
        //
        // Nothing else is retried. A classic applet's `FALSE` is a real answer from a handler that
        // was handed exactly the event it was compiled for, and re-sending a second event would run
        // its start path twice.
        let mut sent = self.send_event(g::EVT_APP_START, 0, start);
        if matches!(sent, Ok(0)) && self.applet_layout == AppletLayout::MP {
            sent = self.send_event(g::EVT_APP_START_WINDOW, 0, start);
        }
        // `CShell_WakeStart` releases the request the moment the event has been delivered:
        // `AppStart_Free` (76 B) frees the struct — and the `pszArgs` copy inside
        // it, which the delivered `AEE_StartAppletEx` had `AEEHeap_StrDup`'d — unless the struct
        // was carved from the shell's static pool. Ours is always a heap block, so it always goes
        // back; leaving it would hand every start a permanent leak on the guest's heap. The free
        // comes before the `?` for the same reason: a delivery that faulted still spent the block.
        self.cpu.mem.free(start);
        let sent = sent?;
        if sent == 0 {
            // A handler that refuses its own start event gets closed, before it is ever drawn to.
            self.close_applet(false)?;
            return Ok(g::AEE_EFAILED);
        }
        if self.app_cls != 0 {
            self.app_state = g::APPSTATE_TOP_VISIBLE;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// One field of the app's own `AEEAppInfo` record, read out of its manifest at the width the ABI
    /// gives that field. `None` when the pack ships no manifest, or no record of that shape — which
    /// is the ordinary case for a module built to be run straight out of a `.mod`.
    pub(crate) fn applet_field(&self, name: &str) -> Option<u32> {
        let record = self.resources.applet_info()?;
        let info = abi::struct_info("AEEAppInfo")?;
        let field = info.fields.iter().find(|f| f.name == name)?;
        let bytes = record
            .get(usize::try_from(field.off).ok()?..usize::try_from(field.off + field.size).ok()?)?;
        Some(match bytes {
            [b] => u32::from(*b),
            &[lo, hi] => u32::from(u16::from_le_bytes([lo, hi])),
            &[a, b, c, d] => u32::from_le_bytes([a, b, c, d]),
            _ => return None,
        })
    }

    /// The name the pack's own manifest gives the applet, which is what a handset shows in its app list
    /// and in the title bar of the window it draws into.
    ///
    /// It is a resource id, not a field: `AEEAppInfo::wIDBase` is the base of the applet's own id range
    /// and `IDR_NAME_OFFSET` — which is 0 — is where the title sits above it. So the name is looked up
    /// the same way the pack looks up its own icon, out of the manifest mounted beside it.
    ///
    /// `None` when no manifest is mounted, or it carries no name at that id. A caller is then left with
    /// the file on disk, which is the state four builds of one game are in and the reason the manifest
    /// is asked for first.
    pub fn applet_name(&self) -> Option<String> {
        let base = self.applet_field("wIDBase")?;
        let units = self
            .resources
            .string(None, (base + g::IDR_NAME_OFFSET) as u16)?;
        // `Store::string` guarantees a terminator, and a manifest that pads its name with spaces is
        // showing them in a title bar if this does not trim.
        // `Store::string` answers the text the container holds, already Unicode; an applet's name
        // is shown by the host, not stored back into the guest, so no device encoding is involved.
        let text = TextEncoding::Unicode.units_to_string(&units);
        let text = text
            .trim_matches(['\0', ' ', '\r', '\n'].as_ref())
            .to_string();
        (!text.is_empty()).then_some(text)
    }

    /// The class ids in this module's own image that could be one, in the order they appear, with
    /// duplicates and out-of-range words dropped.
    ///
    /// A classic `.mod` publishes no list of the classes it owns *in its code*: `IModuleInfo` and the
    /// `AEEAppInfo.gcls` it would report through are a Brew MP idea and absent here, and the flat
    /// ER_RO image carries no header to read them from. What is in the image, though, is the literal
    /// the module compares the asked-for class against — so the ids worth asking about are exactly
    /// the words shaped like one.
    ///
    /// The manifest is the other half, and for one pack the only half: `1073744582`'s `.mod` is 568
    /// bytes of loader for an image it inflates into the heap at run time, and its class id appears
    /// nowhere in those 568 bytes. It is in the `<appid>.mif` beside it, as the first word of the
    /// `RESTYPE_BINARY` record of id 2 — the same record every other manifest in the sample set carries,
    /// and whose id matches the one this scan finds by other means for all of them.
    /// A class id the app's own manifest spells as text. insa carries `"0x0103de2b"` in a string
    /// record of its `.bar`; the id appears nowhere in the 142040-byte image, so the word scan
    /// cannot find it and the manifest's `AEEAppInfo` record is absent. The text is the pack's own
    /// declaration, and starting the pack by it works.
    fn declared_text_class(&self) -> Option<u32> {
        for units in self.resources.own_strings() {
            let text = TextEncoding::Unicode.units_to_string(&units);
            let text = text.trim_matches(['\0', ' ', '\t', '\r', '\n'].as_ref());
            let Some(hex) = text
                .strip_prefix("0x")
                .or_else(|| text.strip_prefix("0X"))
            else {
                continue;
            };
            if hex.len() != 8 {
                continue;
            }
            let Ok(cls) = u32::from_str_radix(hex, 16) else {
                continue;
            };
            let family = cls >> 16;
            if (0x0100..=0x0107).contains(&family)
                || family == 0x0180
                || family == 0x0a00
            {
                return Some(cls);
            }
        }
        None
    }

    pub(crate) fn candidate_classes(&self) -> Result<Vec<u32>, Error> {
        let Some(loaded) = self.module.as_ref() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        let mut at = loaded.base;

        while at + 4 <= loaded.base + loaded.len {
            let word = self.cpu.mem.read_u32(at)?;
            // Published classes live in the 0x0100–0x0107 families, plus The carrier's 0x0180 private
            // range and the 0x0a00 AppGen range. The wider 0x01xx–0x0fxx sweep the first
            // version used collected ARM instructions — `bx lr` is 0x012fff1e, `pop {r4,pc}`
            // is 0x08bd4010 — and probing one of those first faulted guest2's generic
            // constructor before the image's real class (`0x0102cd2e`) was ever asked about.
            let family = word >> 16;
            if ((0x0100..=0x0107).contains(&family)
                || family == 0x0180
                || family == 0x0a00)
                && !self.host_answers_itself(word)
                && !out.contains(&word)
            {
                out.push(word);
            }
            at += 4;
        }
        // The AppGen default goes first whether or not the image happens to hold it: it is the id a
        // handset would start this module with, and the one a module that compares nothing accepts.
        out.retain(|w| *w != APPGEN_CLSID);
        out.insert(0, APPGEN_CLSID);
        // The manifest's declaration goes before even that: it is the id the pack was *installed*
        // under, not a guess from a word that happens to be shaped like one. A pack that names
        // its class only as text still counts as declaring it.
        if let Some(cls) = self
            .applet_field("cls")
            .or_else(|| self.declared_text_class())
        {
            out.retain(|w| *w != cls);
            out.insert(0, cls);
        }
        Ok(out)
    }

    /// Every other word in `0x0100..=0x01ff` that could be a private class id.
    ///
    /// The narrow sweep stops at `0x0107` because the wider range collects ARM instructions
    /// (`bx lr` is `0x012fff1e`). A private family — au's `0x01a0` and `0x015x` — lives above that
    /// line, and a module whose only class sits there answers `AEE_EFAILED` to every narrow id.
    /// This pass hands the remainder over, ordered so that ids whose low half looks like an index
    /// (`…0001`, `…0002`) come before words whose low half is code, which is the cheapest way to
    /// ask the real candidates first. The caller tolerates faults here; it does not read them as
    /// ownership.
    pub(crate) fn wide_candidate_classes(&self, narrow: &[u32]) -> Result<Vec<u32>, Error> {
        let Some(loaded) = self.module.as_ref() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        let mut at = loaded.base;
        while at + 4 <= loaded.base + loaded.len {
            let word = self.cpu.mem.read_u32(at)?;
            let family = word >> 16;
            if (0x0100..=0x01ff).contains(&family)
                && !narrow.contains(&word)
                && !self.host_answers_itself(word)
                && !out.contains(&word)
            {
                out.push(word);
            }
            at += 4;
        }
        out.sort_by_key(|w| w & 0xffff > 0x00ff);
        Ok(out)
    }

    /// `--start auto`, step one: which of the class ids in the image does this module own?
    ///
    /// Asking is the only way to know. A classic module publishes no class list, and
    /// `IModule::CreateInstance` answers a class it does not own by handing it straight back to
    /// `IShell::CreateInstance` — which this runtime recognises as a decline, and which records
    /// nothing. So an id counts as owned when the module produced an object for it, *or* when its
    /// answer came from the module's own code rather than from that hand-back. The second half is
    /// [`Runtime::last_class_taken`], and it is the one test that separates a class whose constructor
    /// then failed on a service this host has not written from a class that was never its own.
    ///
    /// The test this used to run — whether the probe added to the missing-resource or missing-class
    /// ledgers — could not make that separation. A refusal is refused by the shell too, and the shell
    /// records the id in `missing_classes`, so *every* stranger looked like a class the module had
    /// taken. A sample whose manifest names a class the module does not answer for made that visible:
    /// with its `.mif` readable, `helloworld` offered `0x01009ff0` first and discovery stopped there.
    ///
    /// Nothing is adopted and no event is sent here: an owned id whose constructor then faults has
    /// still answered the ownership question, and that answer is the useful half.
    pub fn discover_applet_class(&mut self) -> Result<AutoStart, Error> {
        // Stage one: the families a published class id actually lives in. Everything the sample set
        // declares sits here, and this is the sweep the guest2 fix narrowed the search to.
        let narrow = self.candidate_classes()?;
        let found = self.discover_in(&narrow, true, 0)?;
        if found.cls != 0 || found.note.is_some() {
            return Ok(found);
        }
        // Stage two: every narrow id was turned down. A private class can still sit outside those
        // families — au's `0x01a0`/`0x015x` ids do, and the wide sweep is what found them before the
        // narrowing. Sweep the rest of `0x0100..=0x01ff` once, tolerating faults: a word-shaped
        // instruction there (`bx lr` is `0x012fff1e`) faults the generic constructor, and that says
        // nothing about the class. Only reachable when nothing in the image's own families claimed
        // the applet, so the guest2 false positive — a probe that stops on an instruction *before*
        // the real class is asked — cannot happen here.
        let wide = self.wide_candidate_classes(&narrow)?;
        if wide.is_empty() {
            return Ok(found);
        }
        let again = self.discover_in(&wide, false, narrow.len())?;
        if again.cls != 0 || again.note.is_some() {
            return Ok(again);
        }
        Ok(AutoStart {
            tried: narrow.len() + wide.len(),
            ..found
        })
    }

    /// One pass over `candidates`. `strict` is the narrow sweep: a fault there means the module got
    /// past its own comparison, so the class is owned and the reason is reported. A non-strict pass
    /// treats a fault as "not a class" and keeps going, because its candidate list also contains
    /// instruction words.
    fn discover_in(
        &mut self,
        candidates: &[u32],
        strict: bool,
        base: usize,
    ) -> Result<AutoStart, Error> {
        let recorded = self.accepted_classes.len();
        for (n, cls) in candidates.iter().enumerate() {
            let ppv = self.cpu.mem.alloc_zeroed(4)?;
            let classes = self.missing_classes.len();
            let resources = self.missing_resources.len();
            self.last_class_taken = false;
            // The probe is the other door an applet is built through, and it is a start in every way
            // that matters to a timer: the module's constructor runs here, and it may arm one.
            self.constructing_applets += 1;
            let mut outcome = self.create_via_module(*cls, ppv);
            self.constructing_applets -= 1;
            // `APPGEN_CLSID` is offered before anything the image holds, because a module that
            // compares nothing builds its applet for it. A module that *does* compare may still be
            // handed it first and fault inside the generic constructor — guest2 reads a null the
            // real class would have filled. That fault is not an answer about any class in the
            // image, so the probe keeps going instead of ending on the synthetic default. The same
            // holds for every 0x0a00 word the image scan finds: that range is also ARM's `beq`
            // encoding, so a false candidate there can fault the generic path just as the default
            // does.
            let appgen = (*cls >> 16) == 0x0a00;
            // Two facts, and the class is owned only when both hold: the probe asked for something
            // this host owes, **and** the module's own code ran to ask it. The ledger alone cannot
            // answer the second — a class the module refuses is refused by the shell too, and that
            // refusal is recorded — while `last_class_taken` alone is too wide for this decision (it
            // is also true of a constructor that failed without asking for anything, which the test
            // below pins as "not chosen").
            let owed =
                self.missing_classes.len() > classes || self.missing_resources.len() > resources;
            let asked = owed && self.last_class_taken;
            let applet = match outcome {
                Ok(rv) if rv == g::AEE_SUCCESS => self.cpu.mem.read_u32(ppv)?,
                Ok(_) => 0,
                // A decline is a clean answer: the module hands the class back and
                // `create_via_module` returns the refusal. Only a module that got past its own
                // comparison reaches code this host cannot serve, so *any* fault here means the class
                // was owned — with the reason kept, because owned-and-unstartable is still answered.
                // The synthetic default is the exception: a fault there says the module is not the
                // generic applet, not that it owns the name.
                Err(e) => {
                    if appgen || !strict {
                        self.crash = None;
                        self.last_fault = None;
                        continue;
                    }
                    return Ok(AutoStart {
                        cls: *cls,
                        tried: base + n + 1,
                        applet: 0,
                        note: Some(e.to_string()),
                    })
                }
            };
            // A probe that returned a failure value is the ambiguous case: that is both a decline and
            // an applet whose constructor failed because the shell would not give it a service. Both
            // halves of `asked` are needed to tell them apart — see where it is computed.
            if applet != 0 || asked {
                // The module took the default and built nothing: keep looking for a class it can
                // actually construct. A real class from the image is still reported as owned.
                if appgen && applet == 0 {
                    continue;
                }
                return Ok(AutoStart {
                    cls: *cls,
                    tried: base + n + 1,
                    applet,
                    note: None,
                });
            }
        }
        Ok(AutoStart {
            cls: 0,
            tried: base + candidates.len(),
            applet: 0,
            note: self.took_without_an_object(recorded),
        })
    }

    /// `--start auto`, step two: start what [`Runtime::discover_applet_class`] found, using the
    /// object the winning probe already created rather than asking the module a second time.
    pub fn start_auto(&mut self) -> Result<AutoStart, Error> {
        let mut found = self.discover_applet_class()?;
        // Owned but no object: the module took the class and its own constructor then stopped. There
        // is nothing to start, and the ownership answer is the one worth carrying back.
        if found.applet == 0 {
            return Ok(found);
        }
        match self.start_created(found.applet, found.cls, 0, 0) {
            Ok(g::AEE_SUCCESS) => {}
            Ok(rv) => found.note = Some(format!("StartApplet -> {rv:#010x}")),
            Err(e) => found.note = Some(e.to_string()),
        }
        Ok(found)
    }

    /// Queue an event for the dispatch loop rather than delivering it now — the difference between
    /// `ISHELL_PostEvent` and `ISHELL_SendEvent`, and what the CLI's scripted input uses so that a
    /// session goes through the same one door a real applet's events go through.
    pub fn post_event(&mut self, evt: u32, wp: u32, dwp: u32) {
        self.queue.push_back(Job::Event {
            evt,
            wp,
            dwp,
            owner: self.applet,
        });
    }

    /// Queue one pointer event — `EVT_POINTER_DOWN`, `_MOVE` or `_UP` — at a screen position.
    ///
    /// Unlike every other event on this queue, its `dwParam` is not a value. The handset writes a
    /// NUL-terminated string into guest memory, `x=…,y=…,time=…,clkcnt=…` with the coordinates as
    /// *signed* 32-bit hex, and passes that string's address as `dwParam` with its length — NUL
    /// included — as `wParam`. This matters for anyone reaching for `--event`
    /// instead: hand the applet a packed x/y in `dwParam` and the module parses a small integer as
    /// text, walking its own hex decoder off the end of it. That reads as a fault inside the pack
    /// and is not one.
    pub fn post_pointer(&mut self, evt: u32, x: i32, y: i32) -> Result<(), crate::mem::Fault> {
        let text = pointer_payload(self.clock_ms, x, y);
        let at = self.cpu.mem.alloc_zeroed(text.len() as u32 + 1)?;
        self.cpu.mem.write_bytes(at, text.as_bytes())?;
        self.post_event(evt, text.len() as u32 + 1, at);
        Ok(())
    }

    /// Queue one key going down: `EVT_KEY_PRESS`, then the first `EVT_KEY` repeat, and start the
    /// device's autorepeat clock for as long as it stays down.
    pub fn post_key_press(&mut self, code: u32) {
        self.post_event(g::EVT_KEY_PRESS, code, 0);
        self.post_event(g::EVT_KEY, code, 0);
        let next = self.clock_ms.saturating_add(self.device.key_repeat_start);
        let owner = self.applet;
        if let Some(held) = self.held.iter_mut().find(|k| k.code == code) {
            held.next = next;
        } else {
            self.held.push(HeldKey { code, next, owner });
        }
    }

    /// Queue one key coming up, which ends its repeats from this moment.
    ///
    /// A release never overtakes its own press. If the `EVT_KEY_PRESS` for this code is still sitting
    /// in the queue, the applet has not been told the key went down yet, and handing it down-and-up
    /// in one breath is a thing no handset produces: the header documents the initial `EVT_KEY` and
    /// then repeats every `KB_AUTOREPEAT_RATE` (100) ms for as long as the key is held, starting
    /// `KB_AUTOREPEAT_START` (1500) ms in — a schedule that only exists if press and release are
    /// separate moments. An app that reads which keys are *currently* down rather than counting the
    /// events sees nothing at all when both land in the same turn, and that app is not reachable from
    /// a script either, because every scripted route already pumps between the two. So the release
    /// waits for the next turn.
    pub fn post_key_release(&mut self, code: u32) {
        let running = self.applet;
        if self.queue.iter().any(|job| {
            matches!(job, Job::Event { evt, wp, owner, .. }
                if *evt == g::EVT_KEY_PRESS && *wp == code && owned(*owner, running))
        }) {
            // One entry per release, not one per code: the window can hand over two whole taps of one
            // key in a single batch, and a host that keeps one release per code then tells the applet
            // it went down twice and up once — which leaves a key stuck down in exactly the app this
            // deferral exists for.
            let owner = self.applet;
            self.pending_release.push((code, owner));
        } else {
            self.post_event(g::EVT_KEY_RELEASE, code, 0);
        }
        // The keyboard is one device, so a key coming up stops its repeats whoever was holding the
        // applet end: the entry leaves `held` whatever its owner. Attribution still matters, and it
        // lives on the repeat job that `held` schedules — see the `owner` carried into `due_repeat`.
        self.held.retain(|k| k.code != code);
    }

    /// Queue a whole key: press, repeat and release, with the release held back to the turn after
    /// the press rather than landing beside it — see [`Runtime::post_key_release`].
    ///
    /// That is what a handset does *not* do: a key is held for tens of milliseconds, and an app
    /// that keeps a bitmask of the keys currently down rather than reading the events sees nothing
    /// at all when the press and the release land in the same turn. Some packs are built that way,
    /// session that expects an answer wants [`Runtime::post_key_press`], a pump, and then
    /// [`Runtime::post_key_release`] — which is what `--key` does explicitly, and what this route
    /// now makes every caller get whether it remembered to pump or not.
    pub fn post_key(&mut self, code: u32) {
        self.post_key_press(code);
        self.post_key_release(code);
    }

    /// Run the notify pair stored inside an `AEECallback`, which is both what `IShell::Resume` is
    /// for and what a timer armed through `ISHELL_SetTimerEx` has to do when it comes due.
    ///
    /// The struct is the app's own memory, so it is read back every time rather than cached: an app
    /// legitimately rearms by writing `pfnNotify`/`pNotifyData` again, and a host-side copy would
    /// fire a stale pair. A callback whose notify pointer was never filled in is not an error —
    /// `CALLBACK_Init` is a separate step from scheduling, and `CALLBACK_IsQueued` keys off the
    /// cancel pointer instead — so this says nothing and does nothing.
    pub(crate) fn resume(&mut self, pcb: u32) -> Result<(), Error> {
        // A suspended thread's resume callback is not a function the app wrote. On a handset the pair
        // inside that struct is the AEE's own context switch, so "resuming" here means putting that
        // thread's registers back and handing it the machine until it blocks again. The struct is
        // recognised by the address `GetResumeCBK` handed out — its `pfnNotify` is deliberately zero
        // rather than a pointer the guest could call, because the code behind it is not in the module.
        if let Some(idx) = self
            .threads
            .iter()
            .position(|t| t.resume_cbk != 0 && t.resume_cbk == pcb)
        {
            if self.threads[idx].state == ThreadState::Suspended {
                self.threads[idx].state = ThreadState::Running;
                return self.run_thread(idx);
            }
            return Ok(());
        }
        // The resource controller's wake is not a callback the app wrote either: it is this host's stand
        // for the `AEECallback` the reference record embeds, whose notify is the runtime's own
        // `IResourceCtl_ObjNotify`. Reading the pair happens inside that hook, not here.
        if self.res_ctl.wake != 0 && pcb == self.res_ctl.wake {
            return self.resource_obj_notify();
        }
        let at = |name: &str| {
            abi::offset_of("_AEECallback", name).expect("_AEECallback is in the tables")
        };
        let pfn = self.cpu.mem.read_u32(pcb + at("pfnNotify"))?;
        let user = self.cpu.mem.read_u32(pcb + at("pNotifyData"))?;
        if pfn != 0 {
            self.call_guest(pfn, &[user])?;
        }
        Ok(())
    }

    /// Start an applet from the host side with arguments that are Rust text.
    ///
    /// The CLI's entry point. It writes the string into the guest's heap so that there is one start
    /// path — the ABI one — rather than a host-shaped shortcut next to it.
    pub fn start_app(&mut self, cls: u32, args: Option<&str>) -> Result<u32, Error> {
        let psz = match args {
            None => 0,
            Some(s) => {
                let addr = self.cpu.mem.alloc(s.len() as u32 + 1, 4)?;
                self.cpu.mem.write_bytes(addr, s.as_bytes())?;
                self.cpu.mem.write_u8(addr + s.len() as u32, 0)?;
                addr
            }
        };
        self.start_applet(cls, 0, psz)
    }

    pub fn send_event(&mut self, evt: u32, wp: u32, dwp: u32) -> Result<u32, Error> {
        let applet = self.applet;
        if applet == 0 {
            return Err(Error::Hle(HleError::NoApplet));
        }
        self.script_hook(crate::script::Hook::Event { evt, wp, dwp });
        let handler = self.guest_slot(applet, "IApplet", "HandleEvent")?;
        let ms = self.clock_ms;
        let result = self.call_guest(handler, &[applet, evt, wp, dwp])?;
        if let Some(trace) = self.event_trace.as_mut() {
            use std::io::Write;
            // Per-row flush: a run that faults in the next event still leaves this one on disk.
            let _ = writeln!(trace, "{ms}\t{evt:#x}\t{wp:#x}\t{dwp:#x}\t{result:#x}");
            let _ = trace.flush();
        }
        Ok(result)
    }

    /// Load the package for the applet class the module actually started, and run `on_start`.
    ///
    /// The manifest's `declare` block is read before any script is executed. A package that asks for
    /// write access must either be explicitly allowed with `--script-writes` or be accepted once in
    /// a native popup; that acceptance is written to `scripts/.trust/` and is bound to the manifest
    /// digest, so editing the package asks again.
    pub(crate) fn attach_scripts(&mut self) {
        if self.scripts.is_some() || self.app_cls == 0 {
            return;
        }
        let declaration = match crate::script::Declaration::inspect(&self.script_root, self.app_cls)
        {
            Ok(Some(declaration)) => declaration,
            Ok(None) => return,
            Err(error) => {
                self.display.note(format!("scripts: {error}"));
                return;
            }
        };
        let trusted =
            crate::script::is_trusted(&self.script_root, self.app_cls, declaration.digest);
        let mut allow_writes = self.script_writes || (declaration.requests_writes && trusted);
        if declaration.requests_writes && !allow_writes {
            if self.script_consent_ui {
                if crate::script::ask_consent(&declaration) {
                    if let Err(error) = crate::script::record_trust(
                        &self.script_root,
                        self.app_cls,
                        declaration.digest,
                    ) {
                        self.display.note(format!("scripts: {error}"));
                    }
                    allow_writes = true;
                } else {
                    self.display.note(format!(
                        "scripts: write access declined for {}; package not loaded",
                        declaration.name
                    ));
                    return;
                }
            } else {
                self.display.note(format!(
                    "scripts: {} requests write access; use --script-writes or run with a window",
                    declaration.name
                ));
                return;
            }
        }
        self.display
            .note(format!("script package: {}", declaration.summary()));
        if trusted {
            self.display
                .note("script package: accepted by an existing trust token");
        }
        match crate::script::ScriptHost::load(&self.script_root, self.app_cls, allow_writes) {
            Ok(Some(host)) => {
                self.scripts = Some(host);
                self.script_hook(crate::script::Hook::Start);
            }
            Ok(None) => {}
            Err(error) => self.display.note(format!("scripts: {error}")),
        }
    }

    /// One key, reported the way a handset reports it: `EVT_KEY_PRESS`, then `EVT_KEY`, then
    /// `EVT_KEY_RELEASE`, all three with the same `AVK_*` in `wParam`.
    ///
    /// Three events rather than two because the shell puts an `EVT_KEY` between the press and the
    /// release when the device supports key repeat, and an applet written against the older
    /// contract switches on `EVT_KEY` alone — send only the pair and such an applet never sees the
    /// key at all, which is exactly how a guest game looked dead to input here.
    pub fn press_key(&mut self, code: u32) -> Result<u32, Error> {
        self.dispatch_event(g::EVT_KEY_PRESS, code, 0)?;
        self.dispatch_event(g::EVT_KEY, code, 0)?;
        self.dispatch_event(g::EVT_KEY_RELEASE, code, 0)
    }

    pub fn close_applet(&mut self, return_to_idle: bool) -> Result<u32, Error> {
        self.close_applet_inner(return_to_idle)
    }

    pub(crate) fn close_applet_inner(&mut self, return_to_idle: bool) -> Result<u32, Error> {
        let applet = self.applet;
        if applet == 0 {
            self.display
                .note("CloseApplet: no applet is running, so nothing was closed");
            return Ok(g::AEE_SUCCESS);
        }
        self.script_hook(crate::script::Hook::Stop);
        // The answer lives on the shell's own frame, not the applet's heap: `App_Stop` (
        // 492 B) opens with `sub sp, #0xc` / `movs r0, #1` / `strb r0, [sp]` and hands `sp` to
        // `App_SendEvent`. Allocating it here instead is a **4-byte request against the guest heap**,
        // which takes (and splits) the lowest free node that fits — and a pack that closes itself and
        // keeps allocating is then handed a different block than a handset gives it. The slot is one
        // word below the caller's frame, which is where a callee's first push lands, so a handler that
        // takes the address as a `boolean *` still reads and writes what it would on a handset.
        let sp = self.cpu.r[13];
        let slot = sp.wrapping_sub(8) & !7;
        // A close that is not running on a guest frame (the host itself asking, or a test with no
        // machine behind it) has no stack to borrow, and then the heap is the only guest memory left
        // that can hold the answer.
        let on_stack = sp >= layout::STACK_TOP - layout::STACK_SIZE && sp <= layout::STACK_TOP;
        let borrowed = on_stack && self.cpu.mem.write_u32(slot, g::TRUE).is_ok();
        let allow = if borrowed {
            slot
        } else {
            let at = self.cpu.mem.alloc_zeroed(4)?;
            self.cpu.mem.write_u32(at, g::TRUE)?;
            at
        };
        // The stop handler runs while the applet is closing: the handset sets `APPSTATE_CLOSING`
        // before it asks, so a query from inside that handler no longer lists the applet as running.
        let was_running = self.app_state;
        self.app_state = g::APPSTATE_CLOSING;
        if borrowed {
            self.cpu.r[13] = slot;
        }
        let sent = self.send_event(g::EVT_APP_STOP, 0, allow);
        if borrowed {
            self.cpu.r[13] = sp;
        }
        sent?;
        if self.cpu.mem.read_u32(allow)? == 0 {
            // The header's `dwParam` is a `boolean *`: FALSE means the applet declined to close and
            // is still running. The `int` return tells the caller, but a pack that ignores it (as the
            // sample set's exit paths do) would otherwise leave a failed exit entirely silent. Keep the
            // fact in the report and put it on stderr, where a headless run can see it.
            let said = "CloseApplet: the applet refused EVT_APP_STOP, so it is still running";
            self.display.note(said);
            eprintln!("{said}");
            self.app_state = was_running;
            return Ok(g::AEE_EFAILED);
        }
        if return_to_idle {
            self.display
                .note("CloseApplet(TRUE): no idle applet exists here, so nothing was started");
        }
        if !return_to_idle {
            // `App_Stop`'s reference question, after the stop event and only on the ordinary
            // (`a2 == 0`) close: add one reference, release it again, and read the count that is
            // left. More than one means another holder (the app's own global, a thread, a callback)
            // still needs the applet; the delivered body answers 0 there and `App_Close` puts the
            // app in the background instead of freeing it.
            let addref = self.guest_slot(applet, "IApplet", "AddRef")?;
            self.call_guest(addref, &[applet])?;
            let release = self.guest_slot(applet, "IApplet", "Release")?;
            let refs = self.call_guest(release, &[applet])?;
            if refs > 1 {
                self.app_state = g::APPSTATE_BACKGROUND;
                self.display.note(
                    "CloseApplet: the applet still has references, so it went to the background",
                );
                return Ok(g::AEE_SUCCESS);
            }
        }
        // `App_Stop` (492 B) tells the applet it closed, then drops what it left
        // behind — and the event is **`AEE_SCB_APP_CLOSED` (4)**, not `AEE_SCB_AEE_EXIT`: the body
        // issues `AEE_IssueSystemCallback(4)` at `0x156`, whose `AEE_USR_IssueSystemCallback` runs each
        // of that event's callbacks before it returns. `AEE_SCB_AEE_EXIT` is the AEE going away, not
        // one applet, so this host no longer fires it here (a runtime-exit path would be its issuer,
        // and none is modelled). Fired **before** the applet is cleared and released: the callbacks
        // are the applet's own code and have to run while it is still there.
        self.sys_cb_issue(g::AEE_SCB_APP_CLOSED, applet)?;
        // Then `AEE_CancelCallbacks(appctx)`: everything else this applet registered is cancelled,
        // so a kind-2/3/5/6/7 registration does not outlive the app that made it.
        self.sys_cb_cancel_all(applet)?;
        // And the blocks this applet armed on the dispatcher. `App_CancelDispatchPair` filters on the
        // node's owner word; this host keeps that word beside the queue, so the close takes exactly
        // its own entries. Blocks armed by an applet still running underneath (a popup closing over
        // it) keep theirs, which is the same rule `close_applet`'s event/timer cleanup already uses.
        let queued = self.dispatcher.owned_by(applet);
        for pcb in &queued {
            self.dispatcher_unstamp(*pcb)?;
        }
        let ctl = self.resource_ctl;
        // The delivered drain deletes every object this app linked; the controller is one record this
        // host can tell apart by owner, so a popup closing over its parent leaves the parent's.
        if ctl != 0 && self.res_ctl.owner == applet {
            if let Some(refs_off) = self.hle.object(ctl).map(|o| o.refs_off) {
                let release = self.guest_slot(ctl, "IBase", "Release")?;
                while self.cpu.mem.read_u32(ctl + refs_off)? != 0 {
                    self.call_guest(release, &[ctl])?;
                }
            } else {
                // An address the table no longer knows: clear the six words without pretending to
                // release anything.
                self.resource_ctl_died(ctl)?;
            }
        }
        // Cleared before the release: `AEEApplet_Release` runs guest code that may call back into
        // the shell, and an applet that is already on its way out must not be handed another event.
        self.applet = 0;
        // Nothing is running, so no module is "the running one" until the next start names an applet.
        self.run_module = 0;
        self.app_cls = 0;
        self.app_state = g::APPSTATE_STOPPED;
        self.app_started_ms = 0;
        // A copyright notice the applet put up goes with it: `ADialog_Release` reports the end to the
        // applet's own door, and an applet that is closing has nobody left to tell. Dropping the
        // deadline here is what keeps a later turn from releasing an object the applet's own teardown
        // has already let go of.
        self.copyright = None;
        // Its own work goes with it. Every queued event and every armed timer names code inside
        // this applet, and running either after its release would be executing freed memory — on a
        // handset the shell cancels an applet's timers for exactly this reason.
        //
        // **Only its own.** This used to clear the whole lot, which is wrong the moment a popup is
        // closed on top of a running applet: the events, timers and held keys below it belong to an
        // applet that is still alive, and cancelling them silently ends its animation and its
        // autorepeat. Anything with no owner (`0`, posted while nothing was running) goes with it,
        // because there is nobody left who could claim it.
        let mine = |owner: u32| owner == applet || owner == 0;
        let left = self.timers.iter().filter(|t| mine(t.owner)).count();
        let jobs = self.queue.iter().filter(|j| mine(j.owner())).count();
        self.queue.retain(|j| !mine(j.owner()));
        self.timers.retain(|t| !mine(t.owner));
        // And the keys the device is holding, which are the third thing that turns into an event for
        // this applet: a repeat is `EVT_KEY` with `KB_AUTOREPEAT` set, posted on a clock of its own.
        // A handset stops repeating a key when the applet it was repeating it to goes, and a host
        // that does not finds the next turn posting a repeat to an applet that has closed — which
        // ends the run with "no applet is running" for a key nobody is waiting on.
        self.held.retain(|k| !mine(k.owner));
        // A release held back a turn is the fourth thing of this kind, and it is the one this runtime
        // creates itself: the queue was cleared above, so without this the *next* pump finds this list
        // still holding the key, queues its release, and hands that to the applet that is gone.
        self.pending_release.retain(|(_, owner)| !mine(*owner));
        // The pad gets the same treatment as the keyboard, and for the same reason: a control held
        // across the close would still be held for whoever runs next.
        for pad in self.pads.iter_mut() {
            pad.rest();
        }
        if left != 0 || jobs != 0 {
            self.display.note(format!(
                "CloseApplet: cancelled {left} timer(s) and {jobs} queued job(s) of this applet"
            ));
        }
        // `aee_forcereleaseobj(&a1[3])`: release once, then keep releasing while the answer is
        // positive — the delivered helper stops at zero or when a call no longer moves the count.
        // A single Release would leave behind any reference the applet took for itself.
        self.force_release_applet(applet)?;
        // The shell has no applet to run now. On a handset its idle applet would take over — and
        // that applet is always there, so both close shapes end here; `bReturnToIdle` only says
        // whether the shell starts it, not whether it exists. This host has none, so the flag is
        // what keeps the empty shell from reading as a failed start.
        self.idle = true;
        Ok(g::AEE_SUCCESS)
    }

    /// `aee_forcereleaseobj` (72 B): one `Release`, then repeat while the answer
    /// is positive, stopping at zero or when a call no longer moves the count.
    pub(crate) fn force_release_applet(&mut self, applet: u32) -> Result<(), Error> {
        let release = self.guest_slot(applet, "IApplet", "Release")?;
        let mut last = self.call_guest(release, &[applet])? as i32;
        while last > 0 {
            let now = self.call_guest(release, &[applet])? as i32;
            if now == last || now <= 0 {
                break;
            }
            last = now;
        }
        Ok(())
    }

    /// Follow one `ISoundPlayer`'s naming to the bytes behind it, and record them.
    ///
    /// The vtable *names* a source rather than carrying one: `Set` and `SetInfo` hand over which of
    /// `SDT_*` the pointer is and the pointer itself — a NUL-terminated file name for `SDT_FILE`,
    /// an address the caller filled for `SDT_BUFFER`. That state was always kept; what was missing
    /// is this step, which is the half of the interface a decoder would need and the whole of it
    /// that needs no decoder.
    ///
    /// It never fails, and that is the point. `6e88388` made every sound slot answer so that sound
    /// cannot end a run, and a host that aborted on a clip it could not read would end one anyway,
    /// with a worse answer than the silence it was avoiding. An unmapped buffer or a name the
    /// volume does not have becomes [`AudioSource::problem`] and the run carries on.
    ///
    /// One entry per clip, because a game names the same clip on every play and the report wants the
    /// clips a pack carries rather than the calls that named them. For a buffer that key is
    /// `(input, data, size)`; for a file name it is the name, because the record holds a fresh copy
    /// of it on every naming and the address only tells one call from the next.
    pub(crate) fn note_audio_source(&mut self, player: &Player) {
        // The most this host will read for one clip. A file name carries no length in the record
        // (the delivered `IntSetInfo` writes `dwSize = 0` for exactly that reason), and a stream
        // the caller means to feed is not this host's to bound, so a name with no size gets a
        // ceiling rather than `u32::MAX`, which would be a request for four gigabytes of file.
        const READ_CAP: u32 = 8 << 20;
        let (input, data, size) = (player.input, player.data, player.size);
        if data == 0 || !matches!(input, g::SDT_FILE | g::SDT_BUFFER) {
            return;
        }
        // A buffer's identity is its **bytes**, not its address: the pack frees the block and the
        // allocator hands the same address to the next clip, so an address-keyed entry would keep
        // the first clip's length alive under the second clip's name. Read it once here and let the
        // comparison below decide whether this is the same clip named again.
        let fresh = if input == g::SDT_BUFFER && size != 0 {
            Some(self.cpu.mem.read_bytes(data, size as usize))
        } else {
            None
        };
        let file_name = if input == g::SDT_FILE {
            self.cstring(data).ok()
        } else {
            None
        };
        // Keyed by name, a file would add one record per naming — and the record the current copy
        // keys would be the newest one, whose payload the audio budget may have already dropped, so
        // the clip would stop timing and the pack would stop hearing its end.
        let known = self.audio.iter().position(|s| {
            if input == g::SDT_FILE {
                file_name.as_deref() == Some(s.name.as_str())
            } else {
                s.input == input && s.data == data && s.size == size
            }
        });
        if let Some(i) = known {
            if input == g::SDT_FILE {
                // The record naming this clip is the one in hand; a left-over address would point
                // at a block that went back with the naming that made it.
                self.audio[i].data = data;
                if !self.audio[i].bytes.is_empty() {
                    return;
                }
                // The entry exists but holds nothing, so the earlier read failed. Re-read it instead
                // of keeping that first answer for the rest of the run: `SDT_FILE` names a path in a
                // volume the pack writes into (a download finishing is the ordinary case).
                //
                // This is not only a reporting question: the media and sound notification schedules
                // time a clip out of this same table, so a stale failure would silently cost a pack
                // its `START`, its `TICK_UPDATE`, its `DONE`, and its `GetTotalTime` answer — and say
                // nothing while doing it.
                self.audio.remove(i);
            } else {
                match &fresh {
                    // Same address, same length, same bytes: the same clip named again.
                    Some(Ok(bytes)) if self.audio[i].bytes == *bytes => return,
                    // Same address holding different bytes is a different clip; the old entry's
                    // payload leaves the budget with it.
                    Some(_) => {
                        let gone = self.audio.remove(i);
                        self.audio_bytes = self.audio_bytes.saturating_sub(gone.bytes.len());
                    }
                    // No length: nothing new can be learned about this name until `SetInfo` supplies
                    // one, so that entry stays as it is.
                    None => return,
                }
            }
        }
        let mut source = AudioSource {
            input,
            data,
            size,
            name: String::new(),
            bytes: Vec::new(),
            problem: None,
        };
        if input == g::SDT_BUFFER {
            source.name = format!("{:#010x}", data);
            match fresh {
                None => {
                    // A buffer named with no length has no knowable end: a `Set` stores the pk's
                    // 16-byte ceiling, so this shape comes from the other door — `MMD_BUFFER` handed
                    // to `IMedia::SetMediaParm` with a zero length — and there is nothing here to
                    // read.
                    source.problem = Some("no length: the naming carries none".to_string());
                }
                Some(Ok(bytes)) => source.bytes = bytes,
                Some(Err(e)) => source.problem = Some(e.to_string()),
            }
        } else {
            match self.cstring(data) {
                Ok(name) => {
                    source.name = name;
                    match self.volume.open(&source.name, g::_OFM_READ) {
                        Ok(opened) => {
                            // A whole file, which is what a decoder wants. `want` is a ceiling and
                            // not a demand, so a short file answers with itself.
                            let want = if size == 0 { READ_CAP } else { size };
                            source.bytes = self.volume.read(&opened, want);
                        }
                        Err(e) => {
                            source.problem = Some(format!("{}: AEE 0x{:08x}", source.name, e));
                        }
                    }
                }
                Err(e) => source.problem = Some(e.to_string()),
            }
        }
        // The budget is spent on payloads, not on records: a source named past it still appears in
        // the report with its name and length, because those are what says what the pack asked to
        // play, and they cost nothing.
        if self.audio_bytes.saturating_add(source.bytes.len()) > AUDIO_BUDGET {
            self.audio_skipped += 1;
            source.problem.get_or_insert_with(|| {
                format!(
                    "{} byte(s) not kept: the {} MiB audio budget is spent",
                    source.bytes.len(),
                    AUDIO_BUDGET >> 20
                )
            });
            source.bytes = Vec::new();
        } else {
            self.audio_bytes += source.bytes.len();
        }
        self.audio.push(source);
    }

    /// `IShell::QueryClass` as a boolean: the applet this module started (by the id it was created
    /// for, the id its manifest declares, or the class the module itself owns) and the services the
    /// shell answers itself. `IRegistry::SetHandler` asks the same question about a handler class —
    /// the body's vtable `+12` call — so both doors read this predicate instead of keeping two
    /// answers that could drift apart.
    pub(crate) fn query_class_known(&self, cls: u32) -> bool {
        let declared = self.applet_field("cls");
        (self.module.is_some()
            && (cls == APPGEN_CLSID || cls == self.app_cls || Some(cls) == declared))
            || self.host_answers_itself(cls)
    }

    /// `IShell::CheckPrivLevel` as a boolean: the running app is in group `want`. One rule, shared
    /// with the registry's `AEECLSID_PROVISIONITEM` gate, so a future privilege model cannot move
    /// one door without the other.
    pub(crate) fn check_priv_level(&self, want: u32) -> bool {
        self.app_in_group(self.app_cls, want) == g::AEE_SUCCESS
    }

    /// Whether this host could start a class. It has one module and therefore one applet class: the
    /// one already running, or the one its constructor is building. Everything else is unsupported
    /// rather than merely unchecked — a caller told `SUCCESS` here is told it may start the class.
    pub(crate) fn can_start_class(&self, cls: u32) -> bool {
        cls != 0 && (cls == self.app_cls || self.creating.last().copied() == Some(cls))
    }

    /// The `IModule*` the shell should ask for a class right now: the running applet's module, or the
    /// app's own before an applet has been adopted.
    pub(crate) fn current_module(&self) -> HResult<u32> {
        // Inside a module's own `CreateInstance`, "the current module" is the one being asked: a
        // constructor that builds a sub-object of a class its own module owns is asking about itself.
        if let Some(&module) = self.creating_module.last() {
            return Ok(module);
        }
        if self.run_module != 0 {
            return Ok(self.run_module);
        }
        self.created_module()
    }

    /// Whether an address is an `IModule*` this machine holds — the app's own or a loaded library's.
    ///
    /// `adopt_applet` and the object-adoption path compared against the app's module alone, which
    /// rejected an applet a library built. That shape is exactly what `StartApplet` needs when the
    /// class belongs to another module.
    pub(crate) fn owns_module(&self, module: u32) -> bool {
        module != 0
            && (self.created_module().ok() == Some(module)
                || self.libraries.iter().any(|lib| lib.module == module))
    }

    /// Whether `module` is shaped like an `IModule` built by this run: a vtable pointer, a
    /// reference count, and *this* shell at offset 8.
    ///
    /// A self-unpacking pack needs this. Its mapped `.mod` inflates the real image into the guest
    /// heap and jumps into it, and the payload builds its own `IModule` — same three words as every
    /// module this host synthesizes, but a pointer the registry never saw, because nothing in the
    /// loader adopted it. The applet the payload then builds names that module, and refusing the
    /// applet over it would trade a working start for a bookkeeping detail.
    ///
    /// What this deliberately does *not* say is "any pointer will do": the shell word is the fixed
    /// address of this run's `IShell`, so an object that carries it at an `IModule`'s offset is
    /// either a module or an object built to masquerade as one — and the alternative reading, that
    /// it is an actor or a factory whose neighbouring word happens to be the shell, is what the
    /// three-layout walk already ruled out before this is asked.
    fn module_shaped(&self, module: u32) -> bool {
        module != 0 && self.cpu.mem.read_u32(module + 8).ok() == Some(self.shell)
    }

    /// Read the image at `path`, load it into this machine, and start the applet the module declares
    /// for itself.
    ///
    /// Reading the file is inside this call rather than beside it because that is where the two copies
    /// had already drifted apart: both front ends read it themselves, and a third would have written
    /// the same `path: error` shape a third time.
    ///
    /// Loading is two steps, not one: mapping the image only parks the guest at its entry point, and
    /// the module publishes the `IModule*` that every class question goes through while that entry
    /// point runs. A front end that skips the run asks every class of a module that has answered
    /// nothing yet, and the auto-start search then reports that no class was claimed.
    ///
    /// The start's own record comes back either way — see [`AutoStart::refusal`] for the two answers
    /// that mean "there is nothing to run", and why they are a judgement to be made by the front end
    /// rather than an error raised here.
    pub fn start_pack(
        &mut self,
        path: &std::path::Path,
        entry_thumb: bool,
    ) -> Result<AutoStart, String> {
        let image = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.load(&image, entry_thumb).map_err(|e| e.to_string())?;
        let outcome = self.run().map_err(|e| e.to_string())?;
        if !matches!(outcome, Outcome::Halted { .. }) {
            return Err("the module did not finish loading".to_string());
        }
        if self.created_module().unwrap_or(0) == 0 {
            return Err("the module published no IModule*".to_string());
        }
        self.start_auto().map_err(|e| e.to_string())
    }

    /// Put a loose half back, all at once.
    #[allow(dead_code)]
    pub(crate) fn adopt(&mut self, loose: Loose) {
        self.prefs = loose.prefs;
        self.handlers = loose.handlers;
        self.static_ptr = loose.static_ptr;
        self.images = loose.images;
        self.image_frames = loose.image_frames;
        self.image_display = loose.image_display;
        self.file_maps = loose.file_maps;
        self.files = loose.files;
        self.display_prefs = loose.display_prefs;
        self.system_callbacks = loose.system_callbacks;
        self.notifies = loose.notifies;
        self.event_names = loose.event_names;
        self.properties = loose.properties;
        self.decoder_bytes = loose.decoder_bytes;
        self.faces = loose.faces;
        self.rot_of = loose.rot_of;
        self.exclusive = loose.exclusive;
        self.running_thread = loose.running_thread;
        self.thread_yield = loose.thread_yield;
        self.notify_note = loose.notify_note;
        self.in_notify = loose.in_notify;
        self.applet_enum = loose.applet_enum;
        self.resource_ctl = loose.resource_ctl;
        self.next_net_id = loose.next_net_id;
        self.audio_bytes = loose.audio_bytes;
        self.audio_skipped = loose.audio_skipped;
        self.notified = loose.notified;
        self.sound_dropped = loose.sound_dropped;
        self.accepted_classes_dropped = loose.accepted_classes_dropped;
        self.handed_back = loose.handed_back;
        self.dispatcher.ready = loose.dispatch.ready;
        self.dispatcher.waiting = loose.dispatch.waiting;
        self.dispatcher.batch = loose.dispatch.batch;
        self.dispatcher.owners = loose.dispatch.owners;
        self.res_ctl = loose.res_ctl;
    }
}
