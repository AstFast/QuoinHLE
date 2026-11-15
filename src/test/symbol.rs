// Moved out of `src/symbol.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// A helper table and a shell object, built the way the loader builds them, so the addresses
/// these tests symbolize are the real ones rather than invented ones.
fn fixture() -> (Hle, crate::Mem, u32) {
    let mut mem = crate::Mem::new();
    mem.set_heap(layout::HEAP_BASE, layout::HEAP_LIMIT);
    let mut hle = Hle::new();
    hle.build_helper_table(&mut mem).unwrap();
    let shell = hle
        .new_object(
            &mut mem,
            "IShell",
            abi::const_value("AEECLSID_SHELL").unwrap(),
            8,
            4,
            &[],
        )
        .unwrap();
    (hle, mem, shell)
}

#[test]
fn a_stub_address_reads_as_the_method_it_dispatches() {
    let (hle, _mem, _shell) = fixture();
    let id = abi::method_of("IShell", "CreateInstance").unwrap();
    let stub = layout::STUB_BASE + id * 4;
    assert_eq!(symbolize(&hle, None, stub), "IShell::CreateInstance");
    let malloc = abi::helper_id(abi::offset_of("AEEHelperFuncs", "malloc").unwrap());
    assert_eq!(
        symbolize(&hle, None, layout::STUB_BASE + malloc * 4),
        "AEEHelperFuncs::malloc"
    );
    // Two bytes into a stub is inside one, and the answer says so rather than naming a method.
    assert!(symbolize(&hle, None, stub + 2).contains("middle of a stub"));
}

#[test]
fn the_null_and_the_halt_sentinel_are_named_rather_than_described() {
    let (hle, _mem, _shell) = fixture();
    assert_eq!(symbolize(&hle, None, 0), "NULL");
    assert_eq!(symbolize(&hle, None, layout::HALT), "HALT");
}

#[test]
fn each_region_answers_in_its_own_words() {
    let (hle, _mem, shell) = fixture();
    // The helper table is not an object, but it is the one address in the vtable region with a
    // name of its own.
    assert_eq!(
        symbolize(&hle, None, hle.helper_table_addr()),
        "<AEEHelperFuncs>"
    );
    assert!(symbolize(&hle, None, hle.helper_table_addr() + 4).starts_with("<vtable+"));
    assert_eq!(symbolize(&hle, None, shell), "<IShell object>");
    assert_eq!(
        this_name(&hle, shell),
        format!("IShell object @ {shell:#010x}")
    );
    assert_eq!(this_name(&hle, 0), "NULL");
    assert_eq!(symbolize(&hle, None, layout::HEAP_BASE + 0x2c), "heap+0x2c");
    assert_eq!(
        symbolize(&hle, None, layout::STACK_TOP - 0x10),
        "stack-0x10"
    );
    assert!(symbolize(&hle, None, 0x8000_0000).contains("outside every region"));
}

#[test]
fn a_module_address_is_an_offset_into_the_image_the_loader_mapped() {
    let (hle, _mem, _shell) = fixture();
    let loaded = Loaded {
        base: layout::MODULE_BASE,
        len: 0x1000,
        out: 0,
        writable: None,
    };
    assert_eq!(
        symbolize(&hle, Some(&loaded), layout::MODULE_BASE + 0x174),
        "module+0x174"
    );
    assert_eq!(
        symbolize(&hle, Some(&loaded), layout::MODULE_BASE - 4),
        "<module+helper>"
    );
    assert_eq!(
        symbolize(&hle, Some(&loaded), layout::MODULE_BASE - 8),
        "<module+stdver>"
    );
    // Past the end of the image the module stops being the answer, even though the address is
    // where the loader put it: a lookup that ran off the end would name the wrong thing.
    assert!(symbolize(&hle, Some(&loaded), layout::MODULE_BASE + 0x1000).contains("outside"));
    // Without a module loaded the same address is only a region this host does not map.
    assert!(symbolize(&hle, None, layout::MODULE_BASE + 4).contains("outside every region"));
}
