//! What the handset is: `IHID`/`IPad`, threads, clocks, alarms, signals and
//! the window manager the guest reaches for.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// Where a thread's record lives, by the object address the pack holds.
    pub(crate) fn thread_index(&self, obj: u32) -> Option<usize> {
        self.threads.iter().position(|t| t.obj == obj)
    }

    /// Where every cooperative thread is parked: `(object, pc, lr)` in list order. A report line
    /// wants to say which guest function a run went to sleep inside, and the context itself stays
    /// private to this module.
    pub fn thread_park_sites(&self) -> Vec<(u32, u32, u32)> {
        self.threads
            .iter()
            .map(|t| (t.obj, t.ctx.pc, t.ctx.r[14]))
            .collect()
    }

    /// A thread is over: release what the header says lives as long as it does, and resume whoever
    /// asked to hear about it.
    pub(crate) fn thread_finished(&mut self, idx: usize, rv: i32) -> Result<(), Error> {
        self.threads[idx].state = ThreadState::Ended(rv);
        self.egl.forget_thread(self.threads[idx].obj);
        let stack = self.threads[idx].stack;
        self.threads[idx].stack = 0;
        if stack != 0 {
            self.hle.dealloc(&mut self.cpu.mem, stack);
        }
        // "`IThread::Malloc` allocates memory and associates it with the lifetime of an IThread."
        let allocs = std::mem::take(&mut self.threads[idx].allocs);
        for block in allocs {
            self.hle.dealloc(&mut self.cpu.mem, block);
        }
        // The return value is collected here, and the header's `PFNTHREAD` comment is the same promise:
        // "When this function returns, any callback waiting for the thread to finish is resumed."
        let joins = std::mem::take(&mut self.threads[idx].joins);
        for (pcb, prc) in joins {
            if prc != 0 {
                self.cpu.mem.write_u32(prc, rv as u32)?;
            }
            if pcb != 0 {
                self.queue.push_back(Job::Callback { pcb });
            }
        }
        Ok(())
    }

    /// The `AEECallback` that stands for a thread's resume hook, made on first use.
    ///
    /// The struct is real guest memory laid out from the ABI tables, because the pack holds it and
    /// hands it to whatever it is blocking on (a timer, `IDispatcher`, `IShell::Resume`).
    /// `pfnNotify` stays 0 on purpose: a handset keeps the AEE's own context switch there rather
    /// than app code, which is why the header warns "Do not call `CALLBACK_Init` on the resulting
    /// AEECallback". [`Runtime::resume`] therefore finds the thread by the struct's address instead
    /// of calling through a pointer that was never the app's to make. `Start` hands this same
    /// record to the dispatcher for the thread's first turn, exactly as the delivered
    /// `IThread_Start` calls `IShell::Resume` on it.
    fn thread_resume_cbk(&mut self, idx: usize) -> Result<u32, Error> {
        let known = self.threads[idx].resume_cbk;
        if known != 0 {
            return Ok(known);
        }
        let size = abi::struct_info("_AEECallback")
            .map(|s| s.size)
            .unwrap_or(28);
        let pcb = self.heap_alloc(size)?;
        if pcb == 0 {
            // A pointer-shaped answer has to signal failure with the null pointer. The body that
            // would say which is not in this set (`CThread_GetResumeCBK`: 0 definitions, 0
            // references), so answering an error code here — as this arm did until the
            // partial-answer audit read it — would hand the caller `2` as an address.
            return Ok(0);
        }
        for off in 0..size {
            self.cpu.mem.write_u8(pcb + off, 0)?;
        }
        self.threads[idx].resume_cbk = pcb;
        Ok(pcb)
    }

    /// Every `IThread` slot. A thread on this host is a saved [`Context`] plus a stack the pack sized
    /// itself. The switch happens only where the guest hands the machine over, and a thread is
    /// entered only by a dispatch of its resume callback — the delivered `IThread_Start` queues that
    /// callback instead of calling the body (see the `Start` arm). A host OS thread would cost a
    /// scripted run the reproducibility it exists for.
    pub(crate) fn ithread(
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
        let Some(idx) = self.thread_index(this) else {
            // The class is served, so this is a thread the shell never made. Say which call, since a
            // pack that built its own `IThread` shape would otherwise be reported as a missing method.
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            // `IThread` inherits `IRscPool`, so these four arrive namespaced as IThread's own and have
            // to be answered here. Two of them are the pool's contract verbatim.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                Ok(self.answer_qi(this, iid, ppv, &[g::AEEIID_IRscPool])?)
            }
            "HoldRsc" | "ReleaseRsc" => self.irscpool(id, this, label, name),
            "Malloc" => {
                let [size] = self.args()?;
                let block = self.heap_alloc(size.saturating_add(4))?;
                if block == 0 {
                    return Ok(0);
                }
                self.cpu
                    .mem
                    .write_u32(block, this.wrapping_add(0x1F4).wrapping_add(0x28))?;
                self.threads[idx].allocs.push(block);
                Ok(block + 4)
            }
            "Free" => {
                // `IThread_Free` is `RscPool_Free(pool, p)`: the caller's pointer is four bytes past
                // the block, and a pointer this thread never handed out is the pool's own `-1`.
                let [block] = self.args()?;
                if block == 0 {
                    return Ok(0);
                }
                let raw = block.wrapping_sub(4);
                let Some(at) = self.threads[idx].allocs.iter().position(|b| *b == raw) else {
                    return Ok(u32::MAX);
                };
                self.threads[idx].allocs.remove(at);
                self.heap_free(raw);
                Ok(0)
            }
            "Start" => {
                // `(nStackSz, pfStart, pvStart)`. A stack size the pack named, an entry point, and the
                // one argument it wants; the header allows exactly one start per thread.
                let [stack_sz, pfn, pv] = self.args()?;
                if self.threads[idx].state != ThreadState::Idle {
                    return Ok(g::AEE_EALREADY);
                }
                if pfn == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                // The resume record is made before the stack so a failed stack allocation cannot
                // leave a started thread with no way to be entered. It is the same struct
                // `GetResumeCBK` hands out, embedded in the delivered object.
                let pcb = self.thread_resume_cbk(idx)?;
                if pcb == 0 {
                    return Ok(g::AEE_ENOMEMORY);
                }
                // The delivered body's literal pool says `0x803`: two kilobytes of system-call room
                // and three bytes so the rounding does not lose any of the caller's own.
                let need = stack_sz
                    .saturating_add(THREAD_STACK_SLACK)
                    .saturating_add(3)
                    & !3;
                let stack = self.heap_alloc(need)?;
                if stack == 0 {
                    return Ok(g::AEE_ENOMEMORY);
                }
                // Built the way [`Runtime::enter_guest`] builds any guest call, on a register file of
                // our own rather than the live one: `lr` on the halt sentinel, which is what turns the
                // start function returning into the thread ending, and the two arguments in the order
                // `PFNTHREAD`'s own typedef names them — the thread first, its context second. A pack
                // that reads its argument out of `r1`, which another title does, is reading that typedef.
                let caller = Context {
                    r: self.cpu.r,
                    cpsr: self.cpu.cpsr,
                    pc: self.cpu.pc,
                };
                let mut r = [0u32; 15];
                r[0] = this;
                r[1] = pv;
                r[13] = (stack + need) & !7;
                r[14] = layout::HALT;
                self.cpu.r = r;
                self.cpu.branch_to(pfn, true);
                let ctx = Context {
                    r: self.cpu.r,
                    cpsr: self.cpu.cpsr,
                    pc: self.cpu.pc,
                };
                self.cpu.r = caller.r;
                self.cpu.cpsr = caller.cpsr;
                self.cpu.pc = caller.pc;
                {
                    let t = &mut self.threads[idx];
                    t.ctx = ctx;
                    t.stack = stack;
                    // Started, not running: the delivered thread begins parked in the same state a
                    // `Suspend` leaves it in. The first turn is a dispatch of its resume callback.
                    t.state = ThreadState::Suspended;
                }
                // The delivered `IThread_Start` ends by handing the thread's resume callback to the
                // shell — `IShell::Resume(pShell, GetResumeCBK(me))` — and answering SUCCESS. It
                // never calls `pfStart` itself. The dispatcher's dispatch of that callback is the
                // only door a thread is entered through, which is what keeps the running thread and
                // the guest's idea of the current thread from drifting apart. `Runtime::resume`
                // is the other end of this queue.
                self.dispatcher_resume(pcb, false)?;
                Ok(g::AEE_SUCCESS)
            }
            "Exit" => {
                // ``. `IThread_Stop` is this with -1.
                let [rv] = self.args()?;
                match self.threads[idx].state {
                    // "EFAILED: if the IThread's never been _Started", "EALREADY: if the IThread is
                    // already stopped" — the two are different questions and the pack branches on them.
                    ThreadState::Idle => return Ok(g::AEE_EFAILED),
                    ThreadState::Ended(_) => return Ok(g::AEE_EALREADY),
                    ThreadState::Running | ThreadState::Suspended => {}
                }
                self.thread_finished(idx, rv as i32)?;
                if self.running_thread == this {
                    // "If this method is called from within the thread, it doesn't return and execution
                    // of the thread stops." Nothing further runs from this context, so the registers
                    // here are simply not saved back anywhere.
                    self.thread_yield = Some(this);
                }
                Ok(g::AEE_SUCCESS)
            }
            "Suspend" => {
                // The delivered body is one gate: `if (!me->bCurThread) return;` — and that flag is
                // set only while the dispatcher has this thread's own frame on the machine. A call
                // from another thread (or from the primordial) changes nothing, not even the state,
                // which is why it may keep polling without parking someone else's thread.
                if self.running_thread != this {
                    return Ok(0);
                }
                if self.threads[idx].state != ThreadState::Running {
                    return Ok(0);
                }
                self.threads[idx].state = ThreadState::Suspended;
                self.thread_yield = Some(this);
                Ok(0)
            }
            "Join" => {
                // `(pcb, pnRv)`: whoever calls this wants the thread's return value and a resume when
                // the thread ends. A thread that has already ended is not a mistake — the pack simply
                // asks late, and gets an answer rather than a callback that never fires.
                let [pcb, prc] = self.args()?;
                match self.threads[idx].state {
                    ThreadState::Ended(rv) => {
                        if prc != 0 {
                            self.cpu.mem.write_u32(prc, rv as u32)?;
                        }
                        if pcb != 0 {
                            self.queue.push_back(Job::Callback { pcb });
                        }
                    }
                    _ => self.threads[idx].joins.push((pcb, prc)),
                }
                Ok(0)
            }
            "GetResumeCBK" => {
                // The struct is real guest memory laid out from the ABI tables, because the pack holds
                // it and hands it to whatever it is blocking on. See [`Runtime::thread_resume_cbk`]
                // for why `pfnNotify` stays zero and how the state change behind it is recognised.
                self.thread_resume_cbk(idx)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// Which [`Signal`] an object address names — either of the two names `CreateSignal` hands back.
    pub(crate) fn signal_index(&self, obj: u32) -> Option<usize> {
        self.signals
            .iter()
            .position(|s| (s.sig != 0 && s.sig == obj) || (s.ctl != 0 && s.ctl == obj))
    }

    /// `ISignal::Set` and `ISignalCtl::Set`, which are the same call on the same state.
    ///
    /// The header's contract is a small machine, not a function call: "New Signals are enabled when
    /// created. Once a Signal has been set and dispatched, the Signal is disabled until the owner calls
    /// `ISignalCtl_Enable`. When disabled, calls to `ISignal_Set` are recorded, but not dispatched
    /// until the next call to `ISignalCtl_Enable`." A host that fired the callback every time would
    /// hand the pack notifications it had explicitly not armed for, and a host that only called when
    /// enabled would drop the recorded one.
    pub(crate) fn signal_set(&mut self, obj: u32) -> Result<u32, Error> {
        let Some(idx) = self.signal_index(obj) else {
            // Not a signal this host ever made. `EFAILED` is the answer the caller can test; ending the
            // run over it would make a pack's bookkeeping mistake look like a missing service.
            return Ok(g::AEE_EFAILED);
        };
        if !self.signals[idx].enabled {
            self.signals[idx].pending = true;
            return Ok(g::AEE_SUCCESS);
        }
        let (pfn, cx) = (self.signals[idx].pfn, self.signals[idx].cx);
        self.signals[idx].enabled = false;
        if pfn != 0 {
            self.call_guest(pfn, &[cx])?;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// `ISignalCtl::Enable`: arm it again, and deliver whatever `Set` recorded meanwhile.
    pub(crate) fn signal_enable(&mut self, obj: u32) -> Result<u32, Error> {
        let Some(idx) = self.signal_index(obj) else {
            return Ok(g::AEE_EFAILED);
        };
        self.signals[idx].enabled = true;
        if self.signals[idx].pending {
            self.signals[idx].pending = false;
            return self.signal_set(obj);
        }
        Ok(g::AEE_SUCCESS)
    }

    /// Hand a signal the notification, wherever it came from.
    ///
    /// The bus can be handed a signal this host made (`ISignalCBFactory`, the usual case) or one a pack
    /// implemented itself. The header's first stated service for this interface is "verification that an
    /// `ISignal` instance is compatible with an implementation of `ISignalBus`",
    /// and the honest way to keep it is to ask the object — through its
    /// own `Set` slot — rather than assume which side owns it. One this host made is answered directly,
    /// and the enable/pending machine in `signal_set` is deliberately **not** duplicated here: going
    /// through it is what makes "Any signals in the bus are set, and will require `ISignal_Enable`
    /// before being able to be set again" true without the bus tracking enables at all.
    pub(crate) fn signal_notify(&mut self, sig: u32) -> Result<u32, Error> {
        if self.signal_index(sig).is_some() {
            return self.signal_set(sig);
        }
        // Not a signal this host made: ask it, and treat "there is no such object" as the incompatible
        // signal the header says `Add` may be given — `EFAILED` is an answer the caller can test, while
        // ending the run over a pack's bookkeeping mistake would look like a missing service.
        let Ok(set) = self.guest_slot(sig, "ISignal", "Set") else {
            return Ok(g::AEE_EFAILED);
        };
        self.call_guest(set, &[sig])
    }

    /// `Set`: the bus, and every signal registered on it.
    ///
    /// The list is copied out before anything is called. A signal's `Set` reaches guest code
    /// (`ISignalCBFactory`'s notify function), and that code may `Add` or `Remove` on this same bus —
    /// The header
    /// would call a signal it had just been told to drop, or miss one it had just been told to add.
    pub(crate) fn signal_bus_set(&mut self) -> Result<u32, Error> {
        self.bus.set = true;
        let list = self.bus.signals.clone();
        for sig in list {
            self.signal_notify(sig)?;
        }
        Ok(g::AEE_SUCCESS)
    }

    /// `IBattery`'s one own slot.
    ///
    /// `AEEBattery_GetBatteryItem(me, nItemID, pdwData)` is three lines, in this order: a null
    /// out-pointer is `EBADPARM` **first**, an item id of four or more is `EUNSUPPORTED`, and
    /// the four that are defined go to `bridle_SystemCall(<the device at [+12]>, nItemID, pdwData,
    /// 0)` — the OEM battery reader. This host has no battery, so that branch answers the same
    /// `EUNSUPPORTED` instead of a device value; it is the one place this arm differs from the body.
    pub(crate) fn ibattery(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_BATTERY])
            }
            "GetBatteryItem" => {
                let [item, out] = self.args()?;
                if out == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                if item >= 4 {
                    return Ok(g::AEE_EUNSUPPORTED);
                }
                self.display.note(format!(
                    "{label}: battery item {item} is read through the OEM battery, which this \
                     machine does not have"
                ));
                Ok(g::AEE_EUNSUPPORTED)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IWindowMgr0` — a display region seen as a window manager, `AEECLSID_WindowMgr0` /
    /// `AEEIID_IWindowMgr0`.
    ///
    /// Five slots and the header documents each. Two things about it decide
    /// how it is answered here: it *is* "a single display region", and a window "is created from
    /// an **application-owned widget**" — the applet's own object, not one this host makes.
    ///
    /// ⚠ `CreateWindow`'s own page says a window can answer `AEE_SUCCESS` "yet **fail later** in the
    /// process. Such asynchronous errors are reported to the widget via `IWIDGET_HandleEvent` and
    /// `EVT_WDG_WINDOWSTATUS`". Nothing in this host fails a window after the fact — there is no
    /// compositor to lose it — so no such event is ever sent, and that is a fact about the host rather than a
    /// slot left unhandled.
    pub(crate) fn iwindowmgr0(
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
                self.answer_qi(
                    this,
                    iid,
                    ppv,
                    &[g::AEEIID_IWindowMgr0, g::AEECLSID_WindowMgr0],
                )
            }
            // `int GetExtent(AEEExtent *pExtent)`: "Get the canvas size of the window manager". The
            // canvas of this host's one display region is the screen.
            "GetExtent" => {
                let [extent] = self.args()?;
                if extent == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let cx = abi::offset_of("AEEExtent", "cx").ok_or_else(unimplemented)?;
                let cy = abi::offset_of("AEEExtent", "cy").ok_or_else(unimplemented)?;
                self.cpu
                    .mem
                    .write_u32(extent + cx, self.device.width as u32)?;
                self.cpu
                    .mem
                    .write_u32(extent + cy, self.device.height as u32)?;
                Ok(g::AEE_SUCCESS)
            }
            // `int CreateWindow(IQI *pWidget, uint32 uFlags)`: the widget **is** the window, so a null one is
            // the one thing that cannot become one. The flags are recorded, because the z-order question
            // `ActivateWindow` asks is about the list this call builds.
            "CreateWindow" => {
                let [widget, flags] = self.args()?;
                if widget == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                self.winmgr.windows.retain(|(held, _)| *held != widget);
                self.winmgr.windows.push((widget, flags));
                Ok(g::AEE_SUCCESS)
            }
            // `int SetWindowProp(IQI *pWidget, AEECLSID id, byte *pby, int nSize)`: "window information is
            // identified by a unique key (a class ID)". Setting the same key twice replaces it,
            // which is what "a key" means; a zero-length value is a value.
            "SetWindowProp" => {
                let [widget, key, data, size] = self.args()?;
                if widget == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let bytes = if data == 0 || size == 0 {
                    Vec::new()
                } else {
                    self.cpu.mem.read_bytes(data, size as usize)?
                };
                self.winmgr
                    .props
                    .retain(|(held, k, _)| !(*held == widget && *k == key));
                self.winmgr.props.push((widget, key, bytes));
                Ok(g::AEE_SUCCESS)
            }
            // `int ActivateWindow(IQI *pWidget)`: "bring it to the **top of the z-order** and setting the
            // current focus to it". The list is ordered bottom-to-top, so going on top is going
            // last.
            "ActivateWindow" => {
                let [widget] = self.args()?;
                let Some(idx) = self.winmgr.windows.iter().position(|(w, _)| *w == widget) else {
                    // A widget this manager does not hold is not a window it can raise. The header gives the
                    // return value as "Error code" and nothing more specific, so this is the general one.
                    return Ok(g::AEE_EFAILED);
                };
                let entry = self.winmgr.windows.remove(idx);
                self.winmgr.windows.push(entry);
                Ok(g::AEE_SUCCESS)
            }
            // `void CloseWindow(IQI *pWidget)`: "removing it from the window manager". `void`,
            // so there is no answer; its properties go with it, because a key belongs to a window.
            "CloseWindow" => {
                let [widget] = self.args()?;
                self.winmgr.windows.retain(|(held, _)| *held != widget);
                self.winmgr.props.retain(|(held, _, _)| *held != widget);
                Ok(0)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `IAlarmMgr` — the alarm manager, under `AEECLSID_ALARMMGR` and `AEEIID_ALARMMGR`.
    ///
    /// Five slots, all documented in the header — including the two differences from
    /// `IShell`'s alarm slots that a host would otherwise paper over: `IAlarmMgr` counts **seconds** where
    /// `IShell::SetAlarm` counts **minutes** (,), and `IAlarmMgr::SetAlarm` **requires
    /// `AEEGROUPID_ALARM_ALL`** and refuses an invalid class id with `EBADPARM` where the `IShell` one
    /// does neither.
    ///
    /// ⚠ The class id on these slots means "whose alarms": zero is the current running app
    /// (,), and anything else needs a privilege this host does not model. Since the one
    /// applet here is the only class it knows, an id that is neither zero nor that applet is the
    /// "invalid class ID" the header answers `EBADPARM` for.
    pub(crate) fn ialarmmgr(
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
                self.answer_qi(this, iid, ppv, &[g::AEEIID_ALARMMGR, g::AEECLSID_ALARMMGR])
            }
            // `int AlarmsActive(me, AEECLSID cls)`. ⚠ The answers are **not** `TRUE`/`FALSE`:
            // "SUCCESS: if there are pending alarms. ENOSUCH: if there are no pending alarms.
            // EPRIVLEVEL: user does not have privilege level for the operation. EBADPARM: invalid class
            // ID.". Answering a boolean here is the mistake this slot invites, and a caller
            // testing `!= TRUE` would read `ENOSUCH` as a failure either way — but one that a host which
            // answered `SUCCESS` for "nothing pending" would invert.
            "AlarmsActive" => {
                let [cls] = self.args()?;
                let cls = if cls == 0 { self.app_cls } else { cls };
                if cls != self.app_cls {
                    return Ok(g::AEE_EBADPARM);
                }
                Ok(
                    if self.alarms.alarms.iter().any(|(held, _, _)| *held == cls) {
                        g::AEE_SUCCESS
                    } else {
                        g::AEE_ENOSUCH
                    },
                )
            }
            // `int SetAlarm(me, AEECLSID cls, uint32 *pnUserCode, uint32 nSecs)`. ⚠ **Seconds**, where
            // `IShell::SetAlarm` takes **minutes** (/ against the header) — the one
            // difference between the two slots a host cannot see from the signatures.
            //
            // `pnUserCode` is in/out and carries two things: "the first two bytes
            // specify the 16-bit user code", and "the next 2 bytes carry 16 bit flags" — bit 1
            // set means the caller supplied the code, bit 2 set means the seconds are an **absolute**
            // time rather than a delay from now.
            "SetAlarm" => {
                let [cls, code_at, secs] = self.args()?;
                let cls = if cls == 0 { self.app_cls } else { cls };
                if cls != self.app_cls {
                    return Ok(g::AEE_EBADPARM);
                }
                if code_at == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let word = self.cpu.mem.read_u32(code_at)?;
                let (supplied, absolute) = (word & 0x1 != 0, word & 0x2 != 0);
                let code = (word >> 16) as u16;
                let due = if absolute {
                    // An absolute moment on the same millisecond count `IDispatcher::GetTime` answers
                    // with, so the two clocks a pack can see cannot disagree about when "now" was.
                    secs.saturating_mul(1000)
                } else {
                    self.clock_ms.saturating_add(secs.saturating_mul(1000))
                };
                let code = if supplied {
                    // "If user specifies a user code for the alarm and there is a alarm already active
                    // with that user code, the existing alarm will be cancelled and a new one started".
                    self.alarms
                        .alarms
                        .retain(|(held, c, _)| !(*held == cls && *c == code));
                    code
                } else {
                    // A generated code is one this class has neither armed nor reserved. Bounded by the
                    // width of a code, so a class that has used every one of them stops rather than
                    // spinning; `ENORESOURCE` is the header's word for that.
                    let mut free = None;
                    for candidate in 0..=u16::MAX {
                        let taken = self
                            .alarms
                            .alarms
                            .iter()
                            .any(|(h, c, _)| *h == cls && *c == candidate)
                            || self
                                .alarms
                                .reserved
                                .iter()
                                .any(|(h, c)| *h == cls && *c == candidate);
                        if !taken {
                            free = Some(candidate);
                            break;
                        }
                    }
                    let Some(free) = free else {
                        // ⚠ The header's word for this is `ENORESOURCE` ("no free user code
                        // available",), and it is **not in this repo's constant table** —
                        // `AEE_ERESOURCENOTFOUND` is a different code about a resource that is not
                        // there, so it would be a wrong answer rather than a near one. `ENOMEMORY` is
                        // listed in the same paragraph and is the faithful one for "the host
                        // has nothing left to hand out". Reached only if a class has used all 65 536
                        // codes.
                        return Ok(g::AEE_ENOMEMORY);
                    };
                    // "If the user code is generated by BREW it is returned in the first two bytes of
                    // `pnUserCode`"; the flag half the caller wrote stays where it was.
                    self.cpu
                        .mem
                        .write_u32(code_at, u32::from(free) << 16 | (word & 0xffff))?;
                    free
                };
                self.alarms.alarms.push((cls, code, due));
                Ok(g::AEE_SUCCESS)
            }
            // `int CancelAlarm(me, cls, uint16 nUserCode)`: cancel "an alarm set through
            // `IALARMMGR_SetAlarm`/`ISHELL_SetAlarm`" with "the proper class and 16-bit user code".
            //⚠ "SUCCESS: if an alarm is found and canceled. **EFAILED**: if an alarm
            // cannot be found or canceled." — `EFAILED`, not the `ENOSUCH` the other slots
            // on this interface say for "nothing there".
            "CancelAlarm" => {
                let [cls, code] = self.args()?;
                let cls = if cls == 0 { self.app_cls } else { cls };
                if cls != self.app_cls {
                    return Ok(g::AEE_EBADPARM);
                }
                let code = code as u16;
                let before = self.alarms.alarms.len();
                self.alarms
                    .alarms
                    .retain(|(held, c, _)| !(*held == cls && *c == code));
                Ok(if self.alarms.alarms.len() < before {
                    g::AEE_SUCCESS
                } else {
                    g::AEE_EFAILED
                })
            }
            // `int ListAlarmCodes(me, cls, uint16 *pnUserCodes, uint32 *pnCount)`. The header gives it
            // **three** calling shapes and they are not variants of one:
            //
            // * `pnUserCodes == NULL` ⇒ `pnCount` comes back as the **number** of reserved codes
            //   ("Null on IN: pnCount contains number of reserved alarm user codes on out",).
            // * `pnUserCodes` valid **and** `pnCount` valid ⇒ up to `pnCount` codes are written out.
            // * `pnUserCodes` valid and `pnCount == NULL` ⇒ the pointed-at code *is* the question, "the
            //   alarm user code is used to query if it is reserved. Value unchanged on OUT"
            //   — and that is exactly how the header's own `IALARMMGR_IsCodeReserved` calls it
            //   (: `ListAlarmCodes(p, 0, &nUserCode, 0)`, `SUCCESS` meaning reserved).
            "ListAlarmCodes" => {
                let [cls, codes_at, count_at] = self.args()?;
                let cls = if cls == 0 { self.app_cls } else { cls };
                if cls != self.app_cls {
                    return Ok(g::AEE_EBADPARM);
                }
                let codes: Vec<u16> = self
                    .alarms
                    .reserved
                    .iter()
                    .filter(|(held, _)| *held == cls)
                    .map(|(_, code)| *code)
                    .collect();
                if codes_at == 0 {
                    if count_at != 0 {
                        self.cpu.mem.write_u32(count_at, codes.len() as u32)?;
                    }
                    return Ok(g::AEE_SUCCESS);
                }
                if count_at == 0 {
                    let asked = self.cpu.mem.read_u16(codes_at)? as u16;
                    return Ok(if codes.contains(&asked) {
                        g::AEE_SUCCESS
                    } else {
                        g::AEE_EFAILED
                    });
                }
                let room = self.cpu.mem.read_u32(count_at)? as usize;
                for (i, code) in codes.iter().take(room).enumerate() {
                    self.cpu
                        .mem
                        .write_u16(codes_at + i as u32 * 2, *code as u32)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            // `int ReserveUserCodes(me, cls, uint16 *pnUserCodes, uint32 *pnCount)` — "reserves the
            // specified number of alarm codes for the application class ID specified **without needing
            // the application to actually set alarms**". `pnUserCodes` is `[out]` and the
            // caller allocates it; `pnCount` is `[in/out]`: "On IN specifies the number of user codes
            // needed. On OUT specifies the number of user codes generated and returned".
            //
            // ⚠ "If an app wants to reserve more than 30 user codes it needs to have a privilege level of
            // `AEEGROUPID_RESERVECODE_LARGE`", and the answer for asking without it is
            // `ENOTALLOWED` — not `EFAILED`. This host carries no privilege scheme, so 30 is
            // the ceiling and the word said about exceeding it is the one the header names.
            "ReserveUserCodes" => {
                let [cls, codes_at, count_at] = self.args()?;
                let cls = if cls == 0 { self.app_cls } else { cls };
                if cls != self.app_cls {
                    return Ok(g::AEE_EBADPARM);
                }
                if codes_at == 0 || count_at == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let asked = self.cpu.mem.read_u32(count_at)?;
                if asked > 30 {
                    return Ok(g::AEE_ENOTALLOWED);
                }
                // The codes are generated, so the host picks them: the next unused code for this class,
                // which is what makes a reserved code recognisable by `ListAlarmCodes` afterwards.
                let mut next = self
                    .alarms
                    .reserved
                    .iter()
                    .filter(|(held, _)| *held == cls)
                    .map(|(_, code)| *code as u32 + 1)
                    .max()
                    .unwrap_or(0);
                let mut granted = 0u32;
                let mut generated: Vec<u16> = Vec::new();
                for _ in 0..asked {
                    self.alarms.reserved.push((cls, next as u16));
                    generated.push(next as u16);
                    next += 1;
                    granted += 1;
                }
                for (i, code) in generated.iter().enumerate() {
                    self.cpu
                        .mem
                        .write_u16(codes_at + i as u32 * 2, *code as u32)?;
                }
                self.cpu.mem.write_u32(count_at, granted)?;
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(unimplemented()),
        }
    }

    /// `ISignal` — the notification side, four slots of which one is this host's to answer.
    pub(crate) fn isignal(
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
                Ok(self.answer_qi(this, iid, ppv, &[g::AEEIID_ISignal])?)
            }
            "Set" => self.signal_set(this),
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The pad the pack is talking to. One console, one pad: the first `IHID` created is the hardware,
    /// and a second one is another name for it.
    pub(crate) fn pad_index(&self, obj: u32) -> Option<usize> {
        self.pads
            .iter()
            .position(|p| p.obj == obj || (p.dev != 0 && p.dev == obj))
    }

    /// Say that the pad changed, and hand the notification to the dispatch loop.
    pub(crate) fn pad_notify(&mut self, obj: u32, which: PadSignal) {
        if self.pads.iter().any(|p| p.obj == obj) {
            self.queue.push_back(Job::Pad { pad: obj, which });
        }
    }

    /// Move one button, and queue the event `GetNextButtonEvent` will hand out.
    ///
    /// This is the entry point for anything that produces pad input — the `--pad` script route and the
    /// window's key mapping both come through here — so there is one place where a change becomes a
    /// notification, and one order in which that notification reaches the guest.
    pub fn pad_press(&mut self, button: u32, down: bool) -> Result<(), Error> {
        let Some(at) = self.pads.first().map(|_| 0) else {
            // No pad yet: nothing is wired to hear about it, and a press before the pack has asked for
            // the device is not a state the console can be in.
            return Ok(());
        };
        let pad = &mut self.pads[at];
        let bit = 1u32 << button.min(31);
        let was_down = pad.pressed & bit != 0;
        if was_down == down {
            // A handset reports a change, not a repeat; forwarding the same state again would let a pack
            // count one physical press as several.
            return Ok(());
        }
        pad.pressed = if down {
            pad.pressed | bit
        } else {
            pad.pressed & !bit
        };
        pad.events.push((button, down, self.clock_ms));
        let obj = pad.obj;
        self.pad_notify(obj, PadSignal::Button);
        Ok(())
    }

    /// Move one axis, in the pad's own reported range.
    pub fn pad_axis(&mut self, axis: usize, value: i32) -> Result<(), Error> {
        if axis >= 4 {
            return Ok(());
        }
        let Some(pad) = self.pads.first_mut() else {
            return Ok(());
        };
        let value = value.clamp(PAD_AXIS_MIN, PAD_AXIS_MAX);
        if pad.axes[axis] == value {
            return Ok(());
        }
        pad.axes[axis] = value;
        let obj = pad.obj;
        self.pad_notify(obj, PadSignal::Position);
        Ok(())
    }

    /// Every `IHID` slot: the console's human-interface devices, which for this host is one joystick.
    pub(crate) fn ihid(
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
        let Some(at) = self.pad_index(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                Ok(self.answer_qi(this, iid, ppv, &[g::AEEIID_IHID])?)
            }
            "GetConnectedDevices" => {
                // `(nDeviceType, pnDevHandles, nLen, pnDevHandlesLenReq)`. A caller sizes its own array
                // first — `GetConnectedDevices(type, NULL, 0, &n)` — and the platform's gamepad layer does
                // exactly that, then branches on the count rather than on the code. Sizing a list the
                // host has is not a failure, so the count is answered and `AEE_SUCCESS` returned; and a
                // type this console has no device for is answered with a count of zero, which is the
                // true thing, not with an error the caller cannot tell from a broken host.
                let [kind, handles, len, preq] = self.args()?;
                let one = kind == g::AEEUID_HID_Joystick_Device;
                let count = if one { 1 } else { 0 };
                if preq != 0 {
                    self.cpu.mem.write_u32(preq, count)?;
                }
                if one && handles != 0 && len > 0 {
                    self.cpu.mem.write_u32(handles, PAD_HANDLE)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "CreateDevice" => {
                // `(nDevHandle, ppIHIDDevice)`. The handle is the whole identity of the device, so a
                // handle this host never reported is a parameter error rather than a device to make.
                let [handle, ppdev] = self.args()?;
                if handle != PAD_HANDLE || ppdev == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let dev = self.pads[at].dev;
                let dev = if dev != 0 {
                    dev
                } else {
                    let created = self.hle.new_object(
                        &mut self.cpu.mem,
                        "IHIDDevice",
                        g::AEECLSID_HID,
                        8,
                        4,
                        &[],
                    )?;
                    self.pads[at].dev = created;
                    created
                };
                Ok(self.give_object(dev, ppdev)?)
            }
            "GetDeviceInfo" => {
                let [handle, info] = self.args()?;
                if handle != PAD_HANDLE || info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                self.write_device_info(info)?;
                Ok(g::AEE_SUCCESS)
            }
            "GetNextConnectEvent" => {
                // `(pnDevHandle, pnStatus, pbDroppedEvents)`, and every one of the three may be NULL:
                // the gamepad layer's own drain call is `GetNextConnectEvent(NULL, NULL, NULL)`.
                // `nStatus` is 0 for "this device is here" — the platform's sample compares it against
                // `AEE_SUCCESS` before creating the device, so a non-zero here reads as a removal.
                let [phandle, pstatus, pdropped] = self.args()?;
                if self.pads[at].announced {
                    return Ok(g::AEE_ENOMORE);
                }
                self.pads[at].announced = true;
                if phandle != 0 {
                    self.cpu.mem.write_u32(phandle, PAD_HANDLE)?;
                }
                if pstatus != 0 {
                    self.cpu.mem.write_u32(pstatus, 0)?;
                }
                if pdropped != 0 {
                    self.cpu.mem.write_u8(pdropped, 0)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "RegisterForConnectEvents" => {
                // An `ISignal` (or the `ISignalCtl` half of one — the sample registers its control
                // pointers here), remembered and fired by the host whenever the pad appears.
                let [signal] = self.args()?;
                let obj = self.pads[at].obj;
                self.pads[at].on[PadSignal::slot(PadSignal::Connect)] = signal;
                // The pad is already plugged in, and that is a fact the caller registered to hear. A
                // host that only notified on a hotplug would leave this pack waiting for an event that
                // its own console has already missed, which is exactly how one title stalls.
                if signal != 0 {
                    self.pad_notify(obj, PadSignal::Connect);
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// Every `IHIDDevice` slot for the virtual pad.
    pub(crate) fn ihiddevice(
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
        let Some(at) = self.pad_index(this) else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            // `IHIDDevice` chains to `IQI` directly, not through `ISignal`.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                Ok(self.answer_qi(this, iid, ppv, &[g::AEEIID_IHIDDevice])?)
            }
            "GetDeviceInfo" => {
                let [info] = self.args()?;
                if info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                self.write_device_info(info)?;
                Ok(g::AEE_SUCCESS)
            }
            "GetDeviceStatus" => {
                // 0 means present, the same reading `GetNextConnectEvent`'s status carries.
                let [pstatus] = self.args()?;
                if pstatus != 0 {
                    self.cpu.mem.write_u32(pstatus, 0)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetNumberOfButtons" => {
                let [pn] = self.args()?;
                if pn != 0 {
                    self.cpu.mem.write_u32(pn, PAD_BUTTON_COUNT)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetButtonInfo" => {
                let [button, info] = self.args()?;
                if info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let Some((_, uid)) = PAD_BUTTONS.into_iter().find(|(id2, _)| *id2 == button) else {
                    return Ok(g::AEE_EBADPARM);
                };
                self.clear_struct("AEEHIDButtonInfo", info)?;
                self.put_int("AEEHIDButtonInfo", info, "nButtonID", button as i32)?;
                self.put_int(
                    "AEEHIDButtonInfo",
                    info,
                    "nState",
                    if self.pads[at].pressed & (1 << button.min(31)) != 0 {
                        1
                    } else {
                        0
                    },
                )?;
                self.put_int("AEEHIDButtonInfo", info, "nButtonUID", uid as i32)?;
                // Digital: the header says min and max matter for analog buttons, and every one of
                // these sixteen is a switch.
                self.put_int("AEEHIDButtonInfo", info, "nButtonMin", 0)?;
                self.put_int("AEEHIDButtonInfo", info, "nButtonMax", 1)?;
                Ok(g::AEE_SUCCESS)
            }
            "GetNextButtonEvent" => {
                // `(pButtonInfo, pdwTimestamp, pbDroppedEvents)`, drained oldest first, and
                // `AEE_ENOMORE` when the queue is empty — the sample loops on exactly that.
                let [info, stamp, dropped] = self.args()?;
                if self.pads[at].events.is_empty() {
                    return Ok(g::AEE_ENOMORE);
                }
                let (button, down, at_ms) = self.pads[at].events.remove(0);
                let uid = PAD_BUTTONS
                    .into_iter()
                    .find(|(id2, _)| *id2 == button)
                    .map(|(_, u)| u)
                    .unwrap_or(0);
                if info != 0 {
                    self.clear_struct("AEEHIDButtonInfo", info)?;
                    self.put_int("AEEHIDButtonInfo", info, "nButtonID", button as i32)?;
                    // `nState` is compared `== TRUE` and `== FALSE` by the platform's own handler, so it
                    // carries 1 for down and 0 for up and nothing else.
                    self.put_int("AEEHIDButtonInfo", info, "nState", i32::from(down))?;
                    self.put_int("AEEHIDButtonInfo", info, "nButtonUID", uid as i32)?;
                    self.put_int("AEEHIDButtonInfo", info, "nButtonMin", 0)?;
                    self.put_int("AEEHIDButtonInfo", info, "nButtonMax", 1)?;
                }
                if stamp != 0 {
                    self.cpu.mem.write_u32(stamp, at_ms)?;
                }
                if dropped != 0 {
                    self.cpu.mem.write_u8(dropped, 0)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetPositionState" => {
                let [info] = self.args()?;
                if info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let axes = self.pads[at].axes;
                self.put_position(info, axes)?;
                Ok(g::AEE_SUCCESS)
            }
            "GetMinPositionInfo" | "GetMaxPositionInfo" => {
                let [info] = self.args()?;
                if info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                let edge = if name == "GetMinPositionInfo" {
                    PAD_AXIS_MIN
                } else {
                    PAD_AXIS_MAX
                };
                self.put_position(info, [edge; 4])?;
                Ok(g::AEE_SUCCESS)
            }
            "RegisterForButtonEvent" | "RegisterForPositionChange" | "RegisterForStatusChange" => {
                let [signal] = self.args()?;
                let which = match name {
                    "RegisterForButtonEvent" => PadSignal::Button,
                    "RegisterForPositionChange" => PadSignal::Position,
                    _ => PadSignal::Status,
                };
                let obj = self.pads[at].obj;
                self.pads[at].on[PadSignal::slot(which)] = signal;
                // A registration is not a request to be told now: unlike the connect event, the pad has
                // not changed since the caller last looked, and it drains what it missed with
                // `GetNextButtonEvent`.
                let _ = obj;
                Ok(g::AEE_SUCCESS)
            }
            "SetExclusiveLevel" => {
                // ``. There is one app on this host and nothing else can hold the pad, so
                // whatever level the caller asks for is the level it has.
                let [level] = self.args()?;
                self.exclusive = level;
                Ok(g::AEE_SUCCESS)
            }
            "GetExclusiveLevel" => {
                let [plevel] = self.args()?;
                if plevel != 0 {
                    self.cpu.mem.write_u32(plevel, self.exclusive)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "GetAxesInfo" => {
                // Not a range question but an identity one, and the header answers it in one sentence:
                // "If the UID is 0 then the system has no specific information on what the axis
                // represents. If the UID is -1 that indicates that the axis is not supported." So each
                // axis field comes back holding the uid of what lives there, and an axis this pad has
                // not got says so rather than reading as an axis with no range anybody declared.
                let [info] = self.args()?;
                if info == 0 {
                    return Ok(g::AEE_EBADPARM);
                }
                self.clear_struct("AEEHIDPositionInfo", info)?;
                self.put_int("AEEHIDPositionInfo", info, "bRelativeAxes", 0)?;
                for (field, uid) in PAD_AXES {
                    self.put_int("AEEHIDPositionInfo", info, field, uid)?;
                }
                Ok(g::AEE_SUCCESS)
            }
            "Rumble" | "GetRumbleStatus" => {
                // The header
                // the device does not support rumble" — on both slots. This host has no motor, so that
                // is the code; and `GetRumbleStatus`'s two out-parameters are left alone, because there
                // is no status to report. An `Hle` error here — which is what this arm used to answer —
                // ends the run instead, for a pack that only asked whether it may shake.
                Ok(g::AEE_EUNSUPPORTED)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The ten own slots of `IAClockCtl`. The object's arithmetic is in
    /// [`crate::aclock`]; what needs the runtime is the virtual clock, the face picture's reference
    /// and the display the face is drawn on.
    pub(crate) fn iaclock(&mut self, id: u32, this: u32, label: &str, name: &str) -> HResult<u32> {
        let gap = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let mut clock = self.clocks.get(this).ok_or_else(|| gap())?;
        match name {
            "Start" => {
                // reads the wall clock; this host gives the run its own virtual seconds, so a
                // replay draws the same hands. re-arms with a shell timer — here the hands are
                // computed from `time` whenever the face is drawn, which is the same picture.
                clock.time = self.clock_ms / 1000;
                clock.started = true;
                self.clocks.put(this, clock);
                self.aclock_redraw(this)?;
                Ok(0)
            }
            "Stop" => {
                // cancels the timer and touches nothing else — the hands stay.
                clock.started = false;
                self.clocks.put(this, clock);
                Ok(0)
            }
            "GetProperties" => {
                // `AClock_GetProperties` is `ldr r0, [r0, #0x14]; bx lr` — `m_dwProps` and nothing
                // else. The value a fresh clock reports is the one its constructor stored there:
                // `AClockCtl_New` ends with `movs r0, #3; str r0, [r4, #0x14]`, the two hands.
                Ok(clock.props)
            }
            "SetProperties" => {
                // `AClock_SetProperties` is `str r1, [r0, #0x14]; bx lr`: a plain store, no read of
                // the old value and no redraw, and the answer is whatever `r0` was left holding — the
                // object itself.
                let [props] = self.args()?;
                clock.props = props;
                self.clocks.put(this, clock);
                Ok(this)
            }
            "SetTime" => {
                let [t] = self.args()?;
                let was = clock.time;
                //: the auto-update is cancelled first, so a `SetTime` wins over `Start`,
                // and the answer is the time it replaced.
                clock.started = false;
                clock.time = t;
                self.clocks.put(this, clock);
                self.aclock_redraw(this)?;
                Ok(was)
            }
            "SetFaceColor" | "SetHandFrameColor" | "SetSecColor" | "SetMinColor"
            | "SetHourColor" => {
                let [rgb] = self.args()?;
                let parts = aclock::rgb_parts(rgb);
                match name {
                    "SetFaceColor" => clock.face = parts,
                    "SetHandFrameColor" => clock.frame = parts,
                    "SetSecColor" => clock.sec = parts,
                    "SetMinColor" => clock.min = parts,
                    _ => clock.hour = parts,
                }
                self.clocks.put(this, clock);
                Ok(0)
            }
            "SetFaceImage" => {
                let [pi, prc] = self.args()?;
                let prev = clock.face_image;
                clock.face_image = pi;
                if pi != 0 {
                    //: the rect is the caller's (or the face's own when none is given),
                    // and then the picture's frame size wins over both.
                    let mut rect = if prc != 0 {
                        Rect::read(&self.cpu.mem, prc)?
                    } else {
                        clock.face_rect
                    };
                    if let Some((w, h)) = self.aclock_image_size(pi) {
                        rect.dx = w;
                        rect.dy = h;
                        if clock.face_rect.dx == 0 || clock.face_rect.dy == 0 {
                            // → `AClock_CalcOffset`: a square of the shorter side, anchored
                            // where the face already was.
                            clock.face_rect = aclock::square(Rect::new(
                                clock.face_rect.x,
                                clock.face_rect.y,
                                w,
                                h,
                            ));
                        }
                    }
                    clock.image_rect = rect;
                }
                self.clocks.put(this, clock);
                self.hold(pi, 1)?;
                self.hold(prev, -1)?;
                Ok(0)
            }
            "GetFaceImageRect" => {
                let [prc] = self.args()?;
                if prc != 0 {
                    clock.image_rect.write(&mut self.cpu.mem, prc)?;
                }
                Ok(0)
            }
            // `IAClockCtl`'s table is `IControl` plus the eight clock slots above, so every other
            // name on this object is a control method: hand it to the control dispatcher, which has
            // its own `class == "IAClockCtl"` branches for exactly these (`AClock_SetRect`,
            // `AClock_HandleEvent`, `AClock_SetActive`, `AClock_IsActive`, `AClock_Redraw`). Stopping
            // the run here instead meant those branches were unreachable and a pack that sized its
            // clock died on the call.
            _ => self.iwidget(id, this, label, name, "IAClockCtl"),
        }
    }

    /// An `IImage`'s frame size — the `ii.cxFrame`/`ii.cy` that `AClock_SetFaceImage` copies into the
    /// face rect.
    pub(crate) fn aclock_image_size(&self, image: u32) -> Option<(i16, i16)> {
        let src = self.images.get(&image).copied()?;
        let b = self.display.bitmap(src)?;
        Some((b.width as i16, b.height as i16))
    }

    /// `AClock_Redraw`, drawn with the host's own `IGraphics` primitives —
    /// the same three shapes the documented behaviour draws: a disc for the face (when no picture covers it),
    /// the picture itself, then a line for the seconds and two quads for the other hands.
    pub(crate) fn aclock_redraw(&mut self, this: u32) -> HResult<u32> {
        let clock = self
            .clocks
            .get(this)
            .ok_or(HleError::NotAnObject { addr: this })?;
        let face = clock.face_rect;
        if face.dx == 0 {
            //: no face, nothing drawn, FALSE.
            return Ok(0);
        }
        let disp = self.display.default_display();
        let (w, h) = (self.display.device.width, self.display.device.height);
        let viewport = Rect::new(0, 0, w as i16, h as i16);
        let rgba = |c: [u8; 3]| Rgba::new(c[0], c[1], c[2], 255);
        let state = |fg: [u8; 3], fill: [u8; 3]| State {
            fg: rgba(fg),
            bg: rgba([0, 0, 0]),
            fill: rgba(fill),
            filled: true,
            // `SEC_HAND_WIDTH` and `FACE_WIDTH` are both 2 in the body.
            point: 2,
            stroke: g::STROKE_SOLID,
            hint: g::ALGORITHM_FAST,
            paint: 0,
            viewport,
            clip: None,
            clip_box: None,
            clip_raw: [0; 16],
            framed: false,
            translate: (0, 0),
            dest: 0,
            depth: u32::from(crate::display::NATIVE_DEPTH),
            display: disp,
        };
        let (cx, cy) = (
            i32::from(face.x) + i32::from(face.dx) / 2,
            i32::from(face.y) + i32::from(face.dy) / 2,
        );
        if clock.face_image == 0 {
            //: no face picture means the disc in the face colour.
            let mut ctx = Ctx::new(
                &mut self.cpu.mem,
                &mut self.display,
                state(clock.face, clock.face),
            );
            ctx.disc(cx, cy, i32::from(face.dx) / 2, rgba(clock.face).native());
        } else {
            //: the picture draws itself where `SetFaceImage` put it.
            let (x, y) = (i32::from(clock.image_rect.x), i32::from(clock.image_rect.y));
            self.blit_image(clock.face_image, 0, x, y)?;
        }
        let sec = clock.time % 60;
        let minute = f64::from((clock.time / 60) % 60) + f64::from(sec) / 60.0;
        let hour = f64::from((clock.time / 3600) % 24) + minute / 60.0;
        if clock.props & g::ACPROP_SECS != 0 {
            let end = aclock::hand_point(face, 60, f64::from(sec), 2.1);
            let mut ctx = Ctx::new(
                &mut self.cpu.mem,
                &mut self.display,
                state(clock.sec, clock.sec),
            );
            ctx.line(Line {
                a: (cx, cy),
                b: end,
            });
        }
        // and: each hand is a four-point quad — two corners behind the centre,
        // and a tip at diameter/2.3 or /3.
        let hands = [
            (
                clock.props & g::ACPROP_MINS != 0,
                60u32,
                minute,
                [(30.0, 12.0), (-15.0, 20.0), (0.0, 2.3), (15.0, 20.0)],
                clock.min,
            ),
            (
                clock.props & g::ACPROP_HOURS != 0,
                12u32,
                hour,
                [(6.0, 10.0), (-3.0, 18.0), (0.0, 3.0), (3.0, 18.0)],
                clock.hour,
            ),
        ];
        for (on, ticks, value, corners, colour) in hands {
            if !on {
                continue;
            }
            let pts: Vec<(i32, i32)> = corners
                .iter()
                .map(|(d, f)| aclock::hand_point(face, ticks, value + d, *f))
                .collect();
            let mut ctx = Ctx::new(
                &mut self.cpu.mem,
                &mut self.display,
                state(clock.frame, colour),
            );
            ctx.polygon(&pts, rgba(colour).native(), true, true);
        }
        // fires `EVT_CTL_CHANGING` with a `CtlValChange` when `ACPROP_NOTIFY` is set; this
        // host has no guest record for that payload, so the notification is the one part of the body
        // not sent — the delivered body does not send it.
        Ok(1)
    }

    pub(crate) fn register_system_callback(&mut self, pcb: u32, kind: u32) -> Result<(), Error> {
        if kind > g::AEE_SCB_LOW_RAM_CRITICAL || pcb == 0 {
            return Ok(());
        }
        let at = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        if self.cpu.mem.read_u32(pcb + at("pfnNotify"))? == 0 {
            return Ok(());
        }
        let cancel = self.cpu.mem.read_u32(pcb + at("pfnCancel"))?;
        if cancel != 0 {
            self.call_guest(cancel, &[pcb])?;
        }
        self.system_callbacks.retain(|(held, _)| *held != pcb);
        self.system_callbacks.push((pcb, kind));
        let owner = self.applet;
        self.sys_cb_owners.insert(pcb, owner);
        let leg = self.host_fn(HostFn::SysCbCancel)?;
        self.cpu.mem.write_u32(pcb + at("pfnCancel"), leg)?;
        Ok(self.cpu.mem.write_u32(pcb + at("pCancelData"), 0)?)
    }

    /// `AEE_CancelResume` (the delivered body, 68 bytes): the unregister leg stamped on a
    /// registered system callback. It acts only while the block's `pfnCancel` still names this leg —
    /// a block that has since been armed by another queue belongs to that one, and unlinking it here
    /// would take it out of the wrong list. Declining answers with the word it just compared.
    ///
    /// The delivered leg relinks a list the block itself carries and answers with the critical
    /// section's leave call; this host's list is host-side, so it drops the entry by address, clears
    /// the two stamped words and answers `0`. `pmc` (`+4`) and `pReserved` (`+24`) are the
    /// dispatcher's own fields and are left alone.
    pub(crate) fn sys_cb_cancel_stub(&mut self) -> Result<u32, Error> {
        let cb = self.cpu.r[0];
        let at = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        let leg = self.host_fn(HostFn::SysCbCancel)?;
        let cancel = self.cpu.mem.read_u32(cb + at("pfnCancel"))?;
        if cancel != leg {
            // Not ours to unlink. The delivered leg answers with the word it just compared, which is
            // whatever that other queue stamped — somebody else's handle, not a code.
            return Ok(cancel);
        }
        self.system_callbacks.retain(|(held, _)| *held != cb);
        self.sys_cb_owners.remove(&cb);
        self.dispatcher.forget(cb);
        self.sys_cb_unlink(cb)?;
        Ok(0)
    }

    /// Drop the two words a registration stamped on a block. [`Runtime::sys_cb_cancel_stub`] does
    /// this as the unlink the pack can ask for; the fire path does it as the unlink the dispatcher
    /// does on its own. `pmc` (`+4`) and `pReserved` (`+24`) belong to the delivered list links and
    /// the app-context word, neither of which this host keeps in the block, so they are left alone.
    /// `AEE_IssueSystemCallback` — its `AEE_USR_IssueSystemCallback` body (the delivered body, 162 B),
    /// for the events this host can produce. That body walks the event's registration list and, for
    /// each node it keeps, runs `CALLBACK_Cancel` and hands the block to the event's own
    /// dispatch list, and then calls `AEE_DispatchReady` on that list **inside the same call** — so
    /// the callbacks run before the issuer returns, in list order. `AEE_SCB_APP_CLOSED` (`evt == 4`)
    /// is the one event whose body filters: a node whose owner word is neither the current app
    /// context nor zero is skipped. This host runs one applet, so "this app or zero" is every
    /// registration it holds; the owner word is the gap named in the plan.
    pub(crate) fn sys_cb_issue(&mut self, kind: u32, owner: u32) -> Result<usize, Error> {
        let fired: Vec<u32> = self
            .system_callbacks
            .iter()
            .filter(|(pcb, held)| *held == kind && self.sys_cb_belongs(*pcb, owner))
            .map(|(pcb, _)| *pcb)
            .collect();
        for pcb in &fired {
            self.system_callbacks.retain(|(held, _)| held != pcb);
            self.sys_cb_owners.remove(pcb);
        }
        for pcb in &fired {
            self.sys_cb_unlink(*pcb)?;
        }
        for pcb in &fired {
            self.resume(*pcb)?;
        }
        Ok(fired.len())
    }

    pub(crate) fn sys_cb_cancel_all(&mut self, owner: u32) -> Result<usize, Error> {
        let held: Vec<u32> = self
            .system_callbacks
            .iter()
            .filter(|(pcb, _)| self.sys_cb_belongs(*pcb, owner))
            .map(|(pcb, _)| *pcb)
            .collect();
        for pcb in &held {
            self.system_callbacks.retain(|(held, _)| held != pcb);
            self.sys_cb_owners.remove(pcb);
        }
        for pcb in &held {
            self.sys_cb_unlink(*pcb)?;
        }
        Ok(held.len())
    }

    /// Whether a registration belongs to `owner`. A block the owner table has never heard of counts
    /// as `owner`'s: the only way to get one is a snapshot restore, and the applet whose registration
    /// survived a reload is the one running.
    pub(crate) fn sys_cb_belongs(&self, pcb: u32, owner: u32) -> bool {
        self.sys_cb_owners.get(&pcb).copied().unwrap_or(owner) == owner
    }

    pub(crate) fn sys_cb_unlink(&mut self, cb: u32) -> Result<(), Error> {
        let at = |name: &str| {
            abi::offset_of("_AEECallback", name)
                .unwrap_or_else(|| panic!("_AEECallback::{name} is not in the ABI tables"))
        };
        self.cpu.mem.write_u32(cb + at("pfnCancel"), 0)?;
        Ok(self.cpu.mem.write_u32(cb + at("pCancelData"), 0)?)
    }
}
