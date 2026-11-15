// Moved out of `src/abi/mod.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn vtable_order_matches_the_evidence() {
    // Slot 0 of every interface is AddRef because AEEGETPVTBL puts the vptr at offset 0
    // and IBase is the root of the INHERIT chain. Verified against helloworld.mod, whose
    // AEEMod_Create does ldr r0,[r6] / ldr r1,[r0] / blx r1 with r6 = PSHELL.
    for i in IFACES {
        assert_eq!(
            i.methods.first().map(|m| m.name),
            Some("AddRef"),
            "{}",
            i.name
        );
        assert_eq!(
            i.methods.get(1).map(|m| m.name),
            Some("Release"),
            "{}",
            i.name
        );
        for (n, m) in i.methods.iter().enumerate() {
            assert_eq!(m.slot as usize, n, "{} slot {}", i.name, n);
        }
    }
}

#[test]
fn the_host_registry_agrees_with_itself() {
    // This is the contract that used to be restated in six places: a predicate in `dispatch`, a
    // block in `label_table`, a `vtable_*` builder in `hle`, its call site, and the constants here.
    // None of those five could check the other four. Now one entry is the source, and this test is
    // what says the derived halves still point at it.
    for (pos, host) in HOST_IFACES.iter().enumerate() {
        assert!(
            host.index >= IFACE_COUNT,
            "{}: a host index inside the generated range would shadow a real interface",
            host.names[0]
        );
        assert!(
            host.index < HELPER_IFACE,
            "{}: an index at or above the helper index builds stubs outside the stub page",
            host.names[0]
        );
        assert!(
            host.index != PROC_IFACE && host.index != HOST_FN_IFACE,
            "{}: reserved index",
            host.names[0]
        );
        // `host_iface` must come back to this very entry, not merely to some entry.
        let resolved = host_iface(host.index).expect("registered index does not resolve");
        assert_eq!(
            resolved as *const HostIface, host as *const HostIface,
            "{}: resolves to another entry",
            host.names[0]
        );
        assert!(is_host_iface(host.index));
        // No two entries may claim one index: the second would silently answer the first's slots.
        assert!(
            !HOST_IFACES[..pos].iter().any(|h| h.index == host.index),
            "{}: index {:#x} is claimed twice",
            host.names[0],
            host.index
        );
        for name in host.names {
            match iface_index(name) {
                Some(index) if index == host.index => {}
                // A generated interface of the same name shadows this table, and that order is
                // deliberate: `IFACES` is checked first, because it always was. `IModel` is the live
                // case — the classic header declares one and Brew MP declares one, and a pack that
                // asks by name must keep getting the classic table it got before the registry existed.
                Some(index) => assert!(
                    IFACES.get(index).is_some_and(|i| i.name == *name),
                    "{name} resolves to an interface under another name"
                ),
                None => panic!("{name} does not resolve at all"),
            }
        }
        // The shadowing case, written down rather than left to be rediscovered: only the generated
        // `IModel` is reachable by name, and the MP vector model is reached by its index alone.
        if host.index == VECTORMODEL_IFACE {
            assert_eq!(iface_index("IModel"), IFACES.iter().position(|i| i.name == "IModel"));
            assert_eq!(iface_index("IListModel"), Some(VECTORMODEL_IFACE));
            assert_eq!(iface_index("IVectorModel"), Some(VECTORMODEL_IFACE));
        }
        // `IBase` first, for the same reason every generated interface is checked for it: a guest
        // reads slot 0 as `AddRef` before it knows anything else about the object.
        assert!(host.methods.len() >= 3, "{}", host.names[0]);
        for (slot, expected) in [(0, "AddRef"), (1, "Release"), (2, "QueryInterface")] {
            assert_eq!(host.methods[slot].1, expected, "{}", host.names[0]);
        }
        // Every slot is named for the interface the header declares it on, not for the table's
        // deepest name: `IVectorModel::Size` would point at a slot `AEEIVectorModel.h` never writes.
        for (slot, (iface, method)) in host.methods.iter().enumerate() {
            let id = method_id(host.index, slot as u32);
            assert_eq!(label(id), format!("{iface}::{method}"), "{id:#08x}");
        }
    }
    // A generated index and an unserved one must both come back empty.
    assert!(host_iface(0).is_none());
    assert!(!is_host_iface(0));
    assert!(host_iface(HELPER_IFACE).is_none());
    assert!(host_iface(HOST_FN_IFACE).is_none());
}

#[test]
fn core_clsids_are_where_the_headers_say() {
    assert_eq!(const_value("AEECLSID_SHELL"), Some(0x0100_1000));
    assert_eq!(const_value("AEECLSID_DISPLAY"), Some(0x0100_1001));
    assert_eq!(const_value("AEECLSID_FILEMGR"), Some(0x0100_1003));
    assert_eq!(const_value("EVT_KEY"), Some(0x100));
}

#[test]
fn a_word_is_named_back_under_the_prefix_it_belongs_to() {
    // The report reads a key code back for the reader instead of leaving `0xe036` on the page.
    assert_eq!(const_name("AVK_", 0xE035), Some("AVK_SELECT"));
    assert_eq!(const_name("AVK_", 0xE036), Some("AVK_SOFT1"));
    assert_eq!(const_name("EVT_", 0x100), Some("EVT_KEY"));
    // The prefix is not decoration. A word that carries no name under it says so, rather than
    // borrowing one from a different family: `0x00060000` is a file's own number (issue #53), and
    // calling it a key would be exactly the mistake that issue was chased with.
    assert_eq!(const_name("AVK_", 0x0006_0000), None);
    assert_eq!(
        const_name("EVT_", 0xE035),
        None,
        "a key code is not an event"
    );
}

#[test]
fn object_headers_have_the_measured_shapes() {
    assert_eq!(struct_info("AEERect").map(|s| s.size), Some(8));
    assert_eq!(offset_of("AEERect", "dy"), Some(6));
    assert_eq!(struct_info("_AEEApplet").map(|s| s.size), Some(32));
    assert_eq!(offset_of("_AEEApplet", "m_pIShell"), Some(12));
    assert_eq!(struct_info("AEEMod").map(|s| s.size), Some(20));
}

/// A run of bitfields has to cost its storage unit, or every field after it moves.
///
/// The numbers here are measured, not derived: a compiler probe reproduces
/// `AEEDeviceInfo`'s declaration sequence with the non-`_WIN32` `EmptyEnum` and prints
/// `offsetof` per member. Nine `flg x:1` flags share one `unsigned` at byte 28, so
/// `dwPromptProps` is at 32 — a layout that dropped them would say 28.
#[test]
fn a_bitfield_run_costs_one_storage_unit() {
    assert_eq!(struct_info("AEEDeviceInfo").map(|s| s.size), Some(64));
    assert_eq!(offset_of("AEEDeviceInfo", "unused2"), Some(16));
    assert_eq!(offset_of("AEEDeviceInfo", "dwRAM"), Some(24));
    assert_eq!(offset_of("AEEDeviceInfo", "dwPromptProps"), Some(32));
    assert_eq!(offset_of("AEEDeviceInfo", "wStructSize"), Some(44));
    assert_eq!(offset_of("AEEDeviceInfo", "dwNetLinger"), Some(48));
    assert_eq!(offset_of("AEEDeviceInfo", "dwPlatformID"), Some(60));
    assert_eq!(
        bitfield_of("AEEDeviceInfo", "bAltDisplay"),
        Some((28, 0, 1))
    );
    assert_eq!(bitfield_of("AEEDeviceInfo", "bVibrator"), Some((28, 2, 1)));
    assert_eq!(bitfield_of("AEEDeviceInfo", "bPen"), Some((28, 8, 1)));
    // An ordinary member is not a bitfield, so it has no bit position to report.
    assert_eq!(bitfield_of("AEEDeviceInfo", "dwRAM"), None);
}

#[test]
fn aliases_resolve_to_a_real_interface() {
    assert_eq!(slot_of("IQueryInterface", "QueryInterface"), Some(2));
    assert_eq!(iface_index("IHashCTX"), iface_index("IHashCtx"));
    assert_eq!(iface_index("INotARealInterface"), None);
    assert_eq!(
        iface_index("IShell"),
        Some(iface_of_method_id(method_of("IShell", "AddRef").unwrap()))
    );
    assert_eq!(slot_of("IShell", "AddRef"), Some(0));
    assert!(method_of("IShell", "StartApplet").is_some());
}

#[test]
fn method_ids_round_trip() {
    // Slot 2 of IShell is CreateInstance, not a QueryInterface: `INHERIT_IShell` chains to
    // `INHERIT_IBase`, and that is only AddRef and Release.
    let id = method_of("IShell", "CreateInstance").unwrap();
    assert_eq!(slot_of_method_id(id), 2);
    assert_eq!(IFACES[iface_of_method_id(id)].name, "IShell");
    assert_eq!(method_label(id), "IShell::CreateInstance");
}

/// `label` used to build a fresh `String` on every trap. It is interned now, but the words a
/// report prints have to be byte-for-byte what the old `format!` chain produced: the call log,
/// `--coverage`, `--flame` and fault reports all read them.
#[test]
fn interned_labels_match_every_generated_slot() {
    let mut checked = 0;
    for (index, iface) in IFACES.iter().enumerate() {
        for m in iface.methods.iter() {
            let id = method_id(index, m.slot);
            assert_eq!(
                label(id),
                format!("{}::{}", iface.name, m.name),
                "id {id:#010x}"
            );
            assert_eq!(label(id), method_label(id), "id {id:#010x}");
            checked += 1;
        }
    }
    assert!(
        checked > 2000,
        "the table covers every published slot: {checked}"
    );
    // The same pointer every time means a taken trap allocates nothing to name itself.
    let id = method_of("IShell", "CreateInstance").unwrap();
    assert!(std::ptr::eq(label(id), label(id)));
    assert_eq!(label(id), "IShell::CreateInstance");
}

#[test]
fn interned_labels_cover_helpers_host_fns_and_gaps() {
    let info = struct_info(HELPER_STRUCT).unwrap();
    for f in info.fields.iter() {
        let id = helper_id(f.off);
        assert_eq!(label(id), format!("{HELPER_STRUCT}::{}", f.name));
        assert_eq!(helper_field(id).map(|g| g.name), Some(f.name));
    }
    // A helper slot past the published table keeps the numbered spelling, and the cold path
    // interns it too rather than leaking a fresh string on every lookup.
    let stray = helper_id(info.fields.len() as u32 * POINTER_SIZE as u32);
    assert_eq!(label(stray), format!("helper#{stray:#08x}"));
    assert!(std::ptr::eq(label(stray), label(stray)));
    assert!(helper_field(stray).is_none());

    for n in [0, 1, 7, 255] {
        assert_eq!(label(host_fn_id(n)), format!("host#{n}"));
    }
    // Past the reserved bucket the name is unchanged, just reached through the cold path.
    assert_eq!(label(host_fn_id(900)), "host#900");
    // EGL procs have no published name; `method_label` always fell back to the id.
    let proc = proc_id(3);
    assert_eq!(label(proc), format!("method#{proc:#08x}"));
}
