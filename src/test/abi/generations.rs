// Tests for the generated `crate::abi::generations` tables. They check the invariants the file's
// generator promises — ordering, the value domain, and that nothing claims a fact about 1.2.3 — and a
// few facts read off the header trees, so a regenerated table cannot quietly disagree with them.

use crate::abi::generations::*;
use crate::spec::SpecId;

/// The `SINCE_*` constants exist so the tables are not bare numbers; the `slot_order` lookup also
/// converts a `SpecId` with `as u8`, so the constants and the enum must agree exactly.
#[test]
fn the_since_numbers_are_the_spec_enum() {
    assert_eq!(SINCE_1_1, SpecId::Classic1_1 as u8);
    assert_eq!(SINCE_2_0, SpecId::Classic2_0 as u8);
    assert_eq!(SINCE_3_1, SpecId::Classic3_1 as u8);
    assert_eq!(SINCE_4_0, SpecId::Classic4_0 as u8);
    assert_eq!(SINCE_MP, SpecId::Mp7_12 as u8);
    assert_eq!(
        SpecId::Classic1_2 as u8,
        1,
        "1.2.3 is between 1.1 and 2.0 in the enum; nothing may use its number"
    );
}

/// No table here may claim a fact about 1.2.3: there is no 1.2.3 header tree in this project, so the
/// only honest 1.2.3 statement is "1.1 had it, so it has it too", which is derived at the call site
/// and never stored.
#[test]
fn nothing_claims_a_fact_about_1_2_3() {
    let no_record = SpecId::Classic1_2 as u8;
    assert!(IFACE_SINCE.iter().all(|(_, since)| *since != no_record));
    assert!(CLSID_SINCE.iter().all(|(_, since)| *since != no_record));
    assert!(SLOT_ORDERS.iter().all(|o| o.since != no_record));
}

/// Binary search is how the lookups work, so a regenerated table that lost its ordering would still
/// compile and start returning wrong answers.
#[test]
fn the_tables_are_sorted_and_free_of_duplicates() {
    assert!(
        IFACE_SINCE.windows(2).all(|w| w[0].0 < w[1].0),
        "IFACE_SINCE must be strictly ascending by name"
    );
    assert!(
        CLSID_SINCE.windows(2).all(|w| w[0].0 < w[1].0),
        "CLSID_SINCE must be strictly ascending by id"
    );
    assert!(
        SLOT_ORDERS
            .windows(2)
            .all(|w| (w[0].since, w[0].iface) < (w[1].since, w[1].iface)),
        "SLOT_ORDERS must be strictly ascending by (since, iface)"
    );
}

/// A slot order is written into a guest vtable by `Hle::vtable_ordered`, which resolves each value
/// through `abi::method_id`, so every value has to name a method this host actually has.
#[test]
fn every_slot_order_names_methods_this_host_has() {
    for order in SLOT_ORDERS {
        let index = crate::abi::iface_index(order.iface)
            .unwrap_or_else(|| panic!("{} is not in the host's table", order.iface));
        let count = crate::abi::IFACES[index].methods.len();
        assert!(!order.slots.is_empty(), "{}: empty order", order.iface);
        for slot in order.slots {
            assert!(
                (*slot as usize) < count,
                "{}: slot {slot} but the host table has {count} methods",
                order.iface
            );
        }
        assert!(
            crate::abi::MAX_METHODS as u32 > *order.slots.iter().max().unwrap(),
            "{}: a slot beyond MAX_METHODS cannot be encoded in a dispatch id",
            order.iface
        );
        assert_ne!(
            order.slots,
            (0..order.slots.len() as u32).collect::<Vec<_>>(),
            "{}: an identity order is the host's own and does not need an entry",
            order.iface
        );
    }
}

/// What the trees say, read off them rather than copied: `IShell` is in 1.1's `Inc/AEEShell.h`, the
/// widget family is MP-only, and `AEECLSID_SHELL` is as old as `IShell`.
#[test]
fn the_tables_answer_what_the_header_trees_show() {
    assert_eq!(iface_since("IShell"), Some(SpecId::Classic1_1));
    assert_eq!(iface_since("IDisplay"), Some(SpecId::Classic1_1));
    assert_eq!(
        iface_since("IWidget"),
        Some(SpecId::Mp7_12),
        "the widget model is a Brew MP addition"
    );
    assert_eq!(
        clsid_since(0x0100_1000),
        Some(SpecId::Classic1_1),
        "AEECLSID_SHELL"
    );
    assert_eq!(iface_since("INoSuchInterfaceAnywhere"), None);
}

/// `ICallHistory` is the one release whose order genuinely moves: 2.0's slot 6 is `Clear` where the
/// host's is `UpdateEntry`, and 2.0 has 7 slots against the host's 11. The map has to put 2.0's
/// positions on the host slots that carry the same names, and it must exist only for 2.0.
#[test]
fn the_one_reordered_vtable_is_2_0s_call_history() {
    // 2.0's slot 6 is `Clear`; the host has `Clear` at 7, because `UpdateEntry` was inserted at 6
    // in a later release. Everything below 6 is the same method in both.
    let order = slot_order("ICallHistory", SpecId::Classic2_0).expect("2.0 reorders ICallHistory");
    assert_eq!(order, &[0, 1, 2, 3, 4, 5, 7]);
    assert_eq!(slot_order("ICallHistory", SpecId::Classic1_1), None);
    assert_eq!(slot_order("ICallHistory", SpecId::Classic4_0), None);
    assert_eq!(slot_order("ICallHistory", SpecId::Mp7_12), None);
}
