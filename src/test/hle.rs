// Moved out of `src/hle.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn a_call_into_the_module_is_still_counted_after_the_history_dropped_it() {
    // What this is for: `downcalls` keeps the most recent half, so a run long enough for the pack's
    // own per-frame callback to fill it has nothing left of the key somebody pressed minutes earlier —
    // and that is the only question the history is read for. These counts do not expire.
    let mut hle = Hle::new();
    let pressed = (0x2002_e721u32, 0x0000_e032, 0x0000_0100); // addr, evt, wp — AVK_DOWN
    hle.record_downcall(Downcall {
        addr: pressed.0,
        args: [0x2006_2850, pressed.1, pressed.2, 0],
        retval: 1,
        steps: 0,
    });
    for i in 0..(Hle::HISTORY as u64 + 16) {
        hle.record_downcall(Downcall {
            addr: 0x2002_e721,
            args: [0x2006_2850, 0x0000_0001, 0x0000_0000, 0],
            retval: 1,
            steps: i,
        });
    }

    assert!(
        !hle.downcalls()
            .iter()
            .any(|d| d.addr == pressed.0 && d.args[1] == pressed.1),
        "the history really has dropped the one key press"
    );
    let folded = hle.folded_downcalls();
    assert_eq!(
        folded.iter().find(|(k, _)| *k == pressed).map(|(_, n)| *n),
        Some(1),
        "the key press must still be in the folded counts: {folded:?}"
    );
    assert_eq!(
        folded
            .iter()
            .find(|(k, _)| *k == (0x2002_e721, 0x0000_0001, 0x0000_0000))
            .map(|(_, n)| *n),
        Some(Hle::HISTORY as u64 + 16),
        "and the heartbeat's own count is exact rather than capped: {folded:?}"
    );
    assert!(!hle.folded_full(), "two keys do not fill the table");
}

#[test]
fn a_folded_table_that_hit_its_ceiling_says_so() {
    // `FOLDED` is a ceiling on *kinds*, and the one shape that can grow without bound is a callee
    // handed a fresh pointer every time. Past it the counts already held keep accumulating, no new key
    // is taken, and this flag is what stops a report from reading like the whole story.
    let mut hle = Hle::new();
    for i in 0..(Hle::FOLDED as u32 + 32) {
        hle.record_downcall(Downcall {
            addr: 0x2005_9415,
            args: [0x2029_8a40, 0x1000_0000 + i * 4, 0, 0],
            retval: 0,
            steps: 0,
        });
    }
    assert!(hle.folded_full());
    assert_eq!(hle.folded_downcalls().len(), Hle::FOLDED);
}

/// Every way this file can say "no" has to carry a Chinese line, because the report is what a
/// person playing a game reads and the English above is written for someone with the source open.
/// The cases are built from the variants rather than typed out as strings, so rewording a
/// sentence without adding its `advice` entry fails here and names the variant that lost its line.
#[test]
fn every_failure_sentence_has_an_advice() {
    let cases = [
        (
            "Unimplemented",
            Error::Unimplemented {
                id: 0x0570_07,
                label: "IGraphics::GetFillMode".into(),
                this: 0x0400_0374,
            },
        ),
        (
            "Wanted",
            Error::Wanted {
                id: 0x0570_2a,
                label: "IGraphics::DrawEllipticalArc".into(),
                want: "DrawEllipticalArc".into(),
                this: 0x0400_0354,
            },
        ),
        ("NotAnObject", Error::NotAnObject { addr: 0x1234 }),
        (
            "NotAnApplet",
            Error::NotAnApplet {
                addr: 0x2000_00f0,
                cls: 0x0106_dee9,
                seen: [0x2000_0030, 0, 0, 0],
                module: 0,
            },
        ),
        (
            "Unsupported",
            Error::Unsupported {
                what: "an 8bpp bitmap".into(),
            },
        ),
        ("NoApplet", Error::NoApplet),
        (
            "NoReturn per function",
            Error::NoReturn {
                addr: 0x1000,
                steps: 5,
                spun: false,
                session: false,
            },
        ),
        (
            "NoReturn spinning",
            Error::NoReturn {
                addr: 0x1000,
                steps: 5,
                spun: true,
                session: false,
            },
        ),
        (
            "NoReturn session",
            Error::NoReturn {
                addr: 0x1000,
                steps: 5,
                spun: true,
                session: true,
            },
        ),
        (
            "TooDeep",
            Error::TooDeep {
                depth: 64,
                addr: 0x100,
            },
        ),
        ("Unmapped", Error::Mem(Fault::Unmapped { addr: 0x10 })),
        (
            "Fault::HeapExhausted",
            Error::Mem(Fault::HeapExhausted { want: 0xffff_ffff }),
        ),
    ];
    for (variant, e) in cases {
        let text = e.to_string();
        assert!(
            advice(&text).is_some(),
            "{variant} has no advice line for: {text}"
        );
    }
    // The refused-method line is the one someone playing a game reads, so it has to name the
    // thing that stopped working and not the symbol: the verb, the part of the game it belongs
    // to, and the flag that lists the rest.
    let arc = advice(
        &Error::Wanted {
            id: 0x0570_2a,
            label: "IGraphics::DrawEllipticalArc".into(),
            want: "DrawEllipticalArc".into(),
            this: 0x0400_0354,
        }
        .to_string(),
    )
    .expect("the refused-method line");
    for said in ["elliptical", "drawing", "--coverage", "--break"] {
        assert!(arc.contains(said), "{arc} never says {said:?}");
    }
}

#[test]
fn a_vtable_slot_holds_the_stub_for_its_own_method() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let shell = abi::iface_index("IShell").unwrap();
    let vt = hle.vtable(&mut mem, shell).unwrap();
    let addref = abi::method_of("IShell", "AddRef").unwrap();
    let create = abi::method_of("IShell", "CreateInstance").unwrap();
    assert_eq!(mem.read_u32(vt).unwrap(), layout::STUB_BASE + addref * 4);
    assert_eq!(
        mem.read_u32(vt + 8).unwrap(),
        layout::STUB_BASE + create * 4
    );
    // Reading the stub back gives the SVC whose immediate *is* the id.
    let svc = mem.read_u32(layout::STUB_BASE + create * 4).unwrap();
    assert_eq!(svc & 0xff00_0000, SVC_ARM);
    assert_eq!(svc & 0x00ff_ffff, create);
    // Built once; a second request is the same table.
    assert_eq!(hle.vtable(&mut mem, shell).unwrap(), vt);
}

#[test]
fn a_long_run_keeps_counting_after_the_history_stops_growing() {
    // A guest game reaches the host tens of thousands of times a minute. Keeping every record
    // was how a session ended with Windows fast-failing the process, and keeping only the newest
    // ones is only honest if the numbers a report prints still describe the whole run.
    let mut hle = Hle::new();
    let id = 7;
    for _ in 0..Hle::HISTORY * 2 {
        hle.record(Call {
            id,
            label: "IShell::CreateInstance",
            this: 0,
            args: Vec::new(),
            text: Vec::new(),
            retval: 0,
            steps: 0,
        });
    }
    assert!(
        hle.calls().len() <= Hle::HISTORY,
        "the list stopped growing: {}",
        hle.calls().len()
    );
    assert_eq!(
        hle.call_count(id),
        Hle::HISTORY * 2,
        "the count did not, which is the whole point of folding it as it goes"
    );
    assert_eq!(hle.call_total(), (Hle::HISTORY * 2) as u64);
    assert_eq!(
        hle.tally(),
        vec![("IShell::CreateInstance".to_string(), Hle::HISTORY * 2)]
    );
    assert!(
        hle.dropped.0 > 0,
        "and the report can say how many it dropped"
    );

    // The same for the host's own calls into the module, which `Runtime` asks about once per
    // frame — a scanned history made that question quadratic in how long the run had gone for.
    for _ in 0..3 {
        hle.record_downcall(Downcall {
            addr: 0x1000,
            args: [0; 4],
            retval: 0,
            steps: 0,
        });
    }
    assert_eq!(hle.downcall_count(0x1000), 3);
    assert_eq!(hle.downcall_count(0x2000), 0, "a slot that never ran");
}

#[test]
fn helper_table_matches_the_measured_offsets() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let tbl = hle.build_helper_table(&mut mem).unwrap();
    let malloc = abi::offset_of(HELPER_STRUCT, "malloc").unwrap();
    assert_eq!(malloc, 0x68, "helloworld.mod loads #0x68 for MALLOC");
    assert_eq!(
        mem.read_u32(tbl + malloc).unwrap(),
        layout::STUB_BASE + helper_id(malloc) * 4
    );
}

#[test]
fn objects_answer_addref_through_their_own_offset() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let obj = hle
        .new_object(&mut mem, "IShell", 0x0100_1000, 8, 4, &[])
        .unwrap();
    assert_eq!(hle.object(obj).unwrap().iface, "IShell");
    assert_eq!(hle.refcount(&mem, hle.object(obj).unwrap()).unwrap(), 1);
    let v = hle.refcount(&mem, hle.object(obj).unwrap()).unwrap();
    hle.set_refcount(&mut mem, hle.object(obj).unwrap(), v + 1)
        .unwrap();
    assert_eq!(hle.refcount(&mem, hle.object(obj).unwrap()).unwrap(), 2);
}

#[test]
fn a_reference_count_clamps_rather_than_wrapping() {
    let mut mem = Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    let obj = hle
        .new_object(&mut mem, "IShell", 0x0100_1000, 8, 4, &[])
        .unwrap();
    assert_eq!(hle.change_refs(&mut mem, obj, 3).unwrap(), 4);
    // Wrapping would turn a double-free into a count of 0xfffffffc, i.e. an object that looks
    // alive for the rest of the run. Saturating keeps the bug visible in a state dump.
    assert_eq!(hle.change_refs(&mut mem, obj, -4).unwrap(), 0);
    assert_eq!(hle.change_refs(&mut mem, obj, -99).unwrap(), 0);
    // A count the guest drove to zero can be picked up again, which is what a handset's
    // recycled heap would do with a stale pointer.
    assert_eq!(hle.change_refs(&mut mem, obj, 1).unwrap(), 1);
    assert!(hle.forget(obj), "the record is dropped on request");
    assert!(!hle.forget(obj), "and dropping it twice is a no-op");
    assert!(matches!(
        hle.change_refs(&mut mem, obj, 1),
        Err(Error::NotAnObject { addr }) if addr == obj
    ));
}

#[test]
fn stub_page_and_vtables_do_not_collide() {
    // The largest id the encoding can produce, and the first byte of the vtable region.
    let last = layout::STUB_BASE + abi::helper_id(4095 * 4) * 4 + 4;
    assert!(
        last <= layout::VTABLE_BASE,
        "{:#x} past {:#x}",
        last,
        layout::VTABLE_BASE
    );
}

/// The object table is what makes a pointer mean something again: the vtables, the sizes and the
/// cursors all decide where the *next* allocation lands, so a restore that dropped one would move a
/// block the run has already handed out.
#[test]
fn an_hle_state_keeps_the_pointers_a_run_holds_and_where_the_next_block_goes() {
    let mut hle = Hle::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    hle.set_heap(mem.heap_high());
    let vtable = hle.vtable_named(&mut mem, "IDisplay").unwrap();
    hle.adopt_object(0x0400_0000, "IDisplay", 0, 8);
    let block = hle.alloc(&mut mem, 32).unwrap();
    let before = hle.heap_next();

    let mut out = Vec::new();
    hle.write_state(&mut out);

    let mut back = Hle::new();
    back.read_state(&mut crate::mem::State::new(&out)).unwrap();
    assert!(
        back.object(0x0400_0000).is_some(),
        "the object the run is holding is still an object"
    );
    assert_eq!(
        back.block_size(block),
        Some(32),
        "and the size of the block it was handed"
    );
    assert_eq!(back.heap_next(), before, "and where the next one will go");
    assert_eq!(
        back.vtable_named(&mut mem, "IDisplay").unwrap(),
        vtable,
        "the same interface answers with the same vtable address"
    );
}
