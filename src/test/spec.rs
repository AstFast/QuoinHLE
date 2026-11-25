use super::*;

/// Every variant, so a new one cannot be added without the table being checked against it.
const ALL_SPECS: &[SpecId] = &[
    SpecId::Classic1_1,
    SpecId::Classic1_2,
    SpecId::Classic2_0,
    SpecId::Classic3_1,
    SpecId::Classic4_0,
    SpecId::Mp7_12,
];

const ALL_CARRIERS: &[CarrierId] = &[
    CarrierId::Generic,
    CarrierId::Ktf,
    CarrierId::Kddi,
    CarrierId::Zeebo,
];

#[test]
fn every_id_has_an_entry() {
    for id in ALL_SPECS {
        assert_eq!(spec(*id).id, *id);
    }
    for id in ALL_CARRIERS {
        assert_eq!(carrier(*id).id, *id);
    }
    // The tables are the ones `specs()` walks, so a missing entry would otherwise only show up as
    // a panic at the first lookup.
    assert_eq!(SPECS.len(), ALL_SPECS.len());
    assert_eq!(CARRIERS.len(), ALL_CARRIERS.len());
}

#[test]
fn every_label_and_alias_resolves_to_its_own_entry() {
    for entry in specs() {
        assert_eq!(spec_by_name(entry.label).unwrap().id, entry.id);
        for alias in entry.aliases {
            assert_eq!(
                spec_by_name(alias).unwrap().id,
                entry.id,
                "alias {alias} of {}",
                entry.label
            );
        }
    }
    for entry in carriers() {
        assert_eq!(carrier_by_name(entry.label).unwrap().id, entry.id);
        for alias in entry.aliases {
            assert_eq!(
                carrier_by_name(alias).unwrap().id,
                entry.id,
                "alias {alias} of {}",
                entry.label
            );
        }
    }
}

#[test]
fn no_name_is_claimed_twice() {
    let mut seen: Vec<(String, &str)> = Vec::new();
    for entry in specs() {
        for name in std::iter::once(entry.label).chain(entry.aliases.iter().copied()) {
            let key = name.to_ascii_lowercase();
            if let Some((_, owner)) = seen.iter().find(|(k, _)| *k == key) {
                panic!("\"{name}\" is claimed by both {owner} and {}", entry.label);
            }
            seen.push((key, entry.label));
        }
    }
    let mut seen: Vec<(String, &str)> = Vec::new();
    for entry in carriers() {
        for name in std::iter::once(entry.label).chain(entry.aliases.iter().copied()) {
            let key = name.to_ascii_lowercase();
            if let Some((_, owner)) = seen.iter().find(|(k, _)| *k == key) {
                panic!("\"{name}\" is claimed by both {owner} and {}", entry.label);
            }
            seen.push((key, entry.label));
        }
    }
}

#[test]
fn names_ignore_ascii_case_and_outer_space() {
    assert_eq!(
        spec_by_name("  Classic-4.0.2 ").unwrap().id,
        SpecId::Classic4_0
    );
    assert_eq!(spec_by_name("MP").unwrap().id, SpecId::Mp7_12);
    assert_eq!(carrier_by_name("KTF").unwrap().id, CarrierId::Ktf);
    assert_eq!(carrier_by_name(" ez-appli ").unwrap().id, CarrierId::Kddi);
}

#[test]
fn an_unknown_name_lists_the_known_ones() {
    let err = spec_by_name("classic-9.9").unwrap_err();
    assert!(err.contains("classic-9.9"), "{err}");
    assert!(err.contains("classic-4.0.2"), "{err}");
    let err = carrier_by_name("docomo").unwrap_err();
    assert!(err.contains("docomo"), "{err}");
    assert!(err.contains("kddi"), "{err}");
}

#[test]
fn an_empty_name_is_refused() {
    assert!(spec_by_name("   ").is_err());
    assert!(carrier_by_name("").is_err());
}

#[test]
fn the_defaults_are_the_machine_this_host_already_is() {
    // The table under `abi::generated` is the 4.0.2 one, so an undeclared run must resolve to it:
    // that is what makes "no declaration" and "declare the default" the same machine.
    assert_eq!(DEFAULT_SPEC, SpecId::Classic4_0);
    assert_eq!(spec(DEFAULT_SPEC).label, "classic-4.0.2");
    assert_eq!(DEFAULT_CARRIER, CarrierId::Generic);
    assert_eq!(carrier(DEFAULT_CARRIER).label, "generic");
}

#[test]
fn the_generations_are_ordered_oldest_first() {
    // A `since` table will index by this order, so it is part of the contract rather than a
    // coincidence of how the enum happens to be written today.
    let mut sorted = ALL_SPECS.to_vec();
    sorted.sort();
    assert_eq!(sorted, ALL_SPECS);
    assert_eq!(
        specs().map(|s| s.id).collect::<Vec<_>>(),
        ALL_SPECS.to_vec()
    );
}

fn compat(platform: Option<&str>, carrier: Option<&str>) -> crate::compat::Compat {
    crate::compat::Compat {
        platform: platform.map(str::to_string),
        carrier: carrier.map(str::to_string),
        ..Default::default()
    }
}

const KTF_PACK: &str = "/corpus/KTF BREW/0186_나이트 테일즈/ktales.mod";
const LOOSE_PACK: &str = "/corpus/_scratch/loose.mod";

#[test]
fn nothing_named_leaves_the_default_machine() {
    let got = resolve(
        None,
        None,
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    assert_eq!(got.spec.id, DEFAULT_SPEC);
    assert_eq!(got.carrier.id, DEFAULT_CARRIER);
    assert_eq!(got.spec_from, Source::Default);
    assert_eq!(got.carrier_from, Source::Default);
    assert_eq!(
        got.line(),
        "platform: classic-4.0.2 (carrier generic) [default]"
    );
}

#[test]
fn the_directory_names_ktf_packs() {
    let got = resolve(
        None,
        None,
        &compat(None, None),
        std::path::Path::new(KTF_PACK),
    )
    .unwrap();
    assert_eq!(got.spec.id, SpecId::Classic1_2);
    assert_eq!(got.carrier.id, CarrierId::Ktf);
    assert_eq!(got.spec_from, Source::Directory);
    assert_eq!(
        got.line(),
        "platform: classic-1.2.3 (carrier ktf) [directory]"
    );
}

#[test]
fn a_declaration_beats_the_directory_and_a_flag_beats_both() {
    let path = std::path::Path::new(KTF_PACK);
    let declared = resolve(
        None,
        None,
        &compat(Some("classic-3.1.2"), Some("kddi")),
        path,
    )
    .unwrap();
    assert_eq!(declared.spec.id, SpecId::Classic3_1);
    assert_eq!(declared.carrier.id, CarrierId::Kddi);
    assert_eq!(declared.spec_from, Source::Pack);

    let flagged = resolve(
        Some(spec_by_name("mp").unwrap()),
        Some(carrier_by_name("generic").unwrap()),
        &compat(Some("classic-3.1.2"), Some("kddi")),
        path,
    )
    .unwrap();
    assert_eq!(flagged.spec.id, SpecId::Mp7_12);
    assert_eq!(flagged.carrier.id, CarrierId::Generic);
    assert_eq!(flagged.spec_from, Source::Cli);
    assert_eq!(
        flagged.line(),
        "platform: mp-7.12.5 (carrier generic) [flag]"
    );
}

#[test]
fn the_two_axes_are_decided_separately() {
    // A flag names the generation; the directory still names the carrier.
    let got = resolve(
        Some(spec_by_name("classic-4.0.2").unwrap()),
        None,
        &compat(None, None),
        std::path::Path::new(KTF_PACK),
    )
    .unwrap();
    assert_eq!(got.spec_from, Source::Cli);
    assert_eq!(got.carrier_from, Source::Directory);
    assert_eq!(
        got.line(),
        "platform: classic-4.0.2 (carrier ktf) [flag/directory]"
    );
}

#[test]
fn an_unknown_declared_name_says_which_file_said_it() {
    let err = resolve(
        None,
        None,
        &compat(Some("classic-9.9"), None),
        std::path::Path::new(KTF_PACK),
    )
    .unwrap_err();
    assert!(err.contains("ktales.compat.json"), "{err}");
    assert!(err.contains("classic-9.9"), "{err}");
    assert!(err.contains("classic-4.0.2"), "{err}");
}

#[test]
fn a_marker_matches_a_path_component_not_a_substring() {
    let path = std::path::Path::new("/corpus/KTF BREW notes/thing.mod");
    let got = resolve(None, None, &compat(None, None), path).unwrap();
    assert_eq!(got.spec_from, Source::Default);
}

/// The size is what each release's own headers say, which is 64 bytes for every release whose
/// headers this project has read — 1.1 included, which is where the "1.x is 60" guess died:
/// the classic interface declarations ends `AEEDeviceInfo` with `dwPlatformID`, and `spec/versions/
/// brew_1_1.json` measures it at 64. The 60-byte form predates 1.1.
///
/// 1.2.3 is `None` because there is no 1.2.3 header tree anywhere in this project — not because it
/// is small. The carrier's handset declares 60 for itself and that declaration is what decides, which is the
/// whole point of `Declared`.
#[test]
fn the_generations_declare_the_size_their_own_headers_do() {
    let size = |id: SpecId| spec(id).device_info_size;
    assert_eq!(size(SpecId::Classic1_1), Some(64));
    assert_eq!(
        size(SpecId::Classic1_2),
        None,
        "no 1.2.3 headers exist here"
    );
    assert_eq!(size(SpecId::Classic2_0), Some(64));
    assert_eq!(size(SpecId::Classic3_1), Some(64));
    assert_eq!(size(SpecId::Classic4_0), Some(64));
    assert_eq!(size(SpecId::Mp7_12), Some(64));
    // Never larger than the table this host lays the struct out from, or the write would be clipped
    // by the struct rather than by the declaration and the two would disagree silently.
    let table = crate::abi::struct_info("AEEDeviceInfo").unwrap().size;
    assert!(ALL_SPECS.iter().all(|id| spec(*id)
        .device_info_size
        .map_or(true, |s| u32::from(s) <= table)));
}

#[test]
fn an_undeclared_handset_follows_the_generation_but_a_declared_one_does_not() {
    // A handset that never said how big its struct is, sitting at the pre-1.1 size: naming a
    // generation whose headers have been read fills the measured number in.
    let quiet = crate::device::DeviceProfile {
        device_info_size: 60,
        ..crate::device::DeviceProfile::standard()
    };
    let modern = resolve(
        Some(spec_by_name("classic-4.0.2").unwrap()),
        None,
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    assert_eq!(modern.apply_to(quiet).device_info_size, 64);

    // The same 60 written down: the handset wins, which is how The carrier's 1.2.3 body keeps its size even
    // under a generation that says 64.
    let said = crate::device::DeviceProfile {
        device_info_size: 60,
        declared: crate::device::Declared {
            device_info_size: true,
            ..Default::default()
        },
        ..crate::device::DeviceProfile::standard()
    };
    assert_eq!(modern.apply_to(said).device_info_size, 60);

    // And a generation with no header behind it writes nothing at all.
    let unknown = resolve(
        Some(spec_by_name("classic-1.2.3").unwrap()),
        None,
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    assert_eq!(
        unknown
            .apply_to(crate::device::DeviceProfile::standard())
            .device_info_size,
        64,
        "1.2.3 has no opinion here, so the handset's own 64 stands"
    );
}

#[test]
fn the_default_machine_is_the_machine_the_host_already_is() {
    let p = crate::device::DeviceProfile::standard();
    let generic = carrier(CarrierId::Generic).text;
    assert_eq!(generic.encoding, p.encoding);
    assert_eq!(generic.font_normal, p.font_normal);
    assert_eq!(generic.font_large, p.font_large);
    assert_eq!(generic.text_bytes, p.text_bytes);
    assert_eq!(
        spec(SpecId::Classic4_0).device_info_size,
        Some(p.device_info_size)
    );

    let got = resolve(
        None,
        None,
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap()
    .apply_to(p.clone());
    assert_eq!(got.text_bytes, p.text_bytes);
    assert_eq!(got.encoding, p.encoding);
    assert_eq!(got.font_normal, p.font_normal);
    assert_eq!(got.font_large, p.font_large);
    assert_eq!(got.device_info_size, p.device_info_size);
}

/// The text side is the operator's, all of it. The headers settled this: every Qualcomm release from
/// 1.1 up declares `DrawText(IDisplay*, AEEFont, const AECHAR *pcText, …)` with `AECHAR` a `uint16`,
/// so no *release* hands it bytes. The carrier's firmware does, which is why the bit sits on the carrier
/// next to the charset rather than on the generation.
#[test]
fn a_carrier_speaks_for_the_whole_text_side() {
    let kddi = resolve(
        Some(spec_by_name("classic-2.0").unwrap()),
        Some(carrier_by_name("kddi").unwrap()),
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap()
    .apply_to(crate::device::DeviceProfile::standard());
    assert!(!kddi.text_bytes, "au's firmware takes AECHAR");
    assert_eq!(kddi.encoding, crate::abi::generated::AEE_ENC_S_JIS as u16);
    assert_eq!((kddi.font_normal, kddi.font_large), (20, 24));

    let ktf = resolve(
        Some(spec_by_name("classic-1.1").unwrap()),
        Some(carrier_by_name("ktf").unwrap()),
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap()
    .apply_to(crate::device::DeviceProfile::standard());
    assert!(ktf.text_bytes, "The carrier firmware keeps the 8-bit text ABI");
    assert_eq!(ktf.encoding, crate::abi::generated::AEE_ENC_EUC_KR as u16);
}

/// The sample set' own handset names every field this module could fill, so running it under the
/// generation and carrier its directory implies must hand back exactly the profile that was read.
/// This is the G2 invariant in one place: if it holds, the carrier baseline cannot move.
#[test]
fn the_shipped_ktf_handset_under_its_own_generation_is_returned_unchanged() {
    let p = crate::device_file::from_json(
        &crate::json::parse(include_str!("../../assets/devices/ktf-kp2100.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        p.declared.named(),
        [
            "device_info_size",
            "text_bytes",
            "encoding",
            "font_normal",
            "font_large",
            "object_premap",
            "module_base"
        ],
        "the file names all seven, so no default may reach this handset"
    );

    let r = resolve(
        None,
        None,
        &compat(None, None),
        std::path::Path::new(KTF_PACK),
    )
    .unwrap();
    assert_eq!(r.spec.id, SpecId::Classic1_2);
    assert_eq!(r.carrier.id, CarrierId::Ktf);
    let got = r.apply_to(p.clone());
    assert_eq!(got.text_bytes, p.text_bytes);
    assert_eq!(got.encoding, p.encoding);
    assert_eq!(got.font_normal, p.font_normal);
    assert_eq!(got.font_large, p.font_large);
    assert_eq!(got.device_info_size, p.device_info_size);
}

/// The carrier's handsets have writable memory where its packs' saves point, and the public ones do not: the
/// span belongs to the carrier, exactly like the character set.
#[test]
fn a_carrier_brings_the_memory_its_packs_point_into() {
    use crate::device::{DeviceProfile, RamWindow};

    let ktf = resolve(
        None,
        Some(carrier_by_name("ktf").unwrap()),
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    let ktf_profile = ktf.apply_to(DeviceProfile::standard());
    assert_eq!(
        ktf_profile.ram_windows,
        [
            RamWindow {
                base: 0x0180_0000,
                len: 0x0080_0000
            },
            // The RAM below the heap that yongman2's scroll buffer walks through.
            RamWindow {
                base: 0x1f00_0000,
                len: 0x0100_0000,
            },
        ]
    );
    // Its scroll runs across 12 MiB of pages the allocator never hands out, so those pages have
    // to be RAM from the first instruction rather than mapped on Sbrk.
    assert_eq!(ktf_profile.heap_premap, 0x0100_0000);
    // The object region is a handset's own statement, not a carrier default: A carrier build leaves it at the
    // host's lazy mapping until a device file says otherwise.
    assert_eq!(ktf_profile.object_premap, 0);

    let public = resolve(
        None,
        Some(carrier_by_name("generic").unwrap()),
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    let public_profile = public.apply_to(DeviceProfile::standard());
    assert!(public_profile.ram_windows.is_empty());
    assert_eq!(public_profile.heap_premap, 0);
    assert_eq!(public_profile.object_premap, 0);

    // A handset that names its own spans keeps them, including when it names none: an empty list is
    // the statement "this handset has no such region", not silence.
    let mine = DeviceProfile {
        ram_windows: vec![RamWindow {
            base: 0x0100_0000,
            len: 0x1000,
        }],
        declared: crate::device::Declared {
            ram_windows: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert_eq!(ktf.apply_to(mine.clone()).ram_windows, mine.ram_windows);
    let none = DeviceProfile {
        declared: crate::device::Declared {
            ram_windows: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert!(ktf.apply_to(none).ram_windows.is_empty());

    // The same goes for the heap pages: a handset that says how many it has keeps that number,
    // even when the answer is none.
    let mine = DeviceProfile {
        heap_premap: 0x2000,
        declared: crate::device::Declared {
            heap_premap: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert_eq!(ktf.apply_to(mine.clone()).heap_premap, 0x2000);
    let none = DeviceProfile {
        declared: crate::device::Declared {
            heap_premap: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert_eq!(ktf.apply_to(none).heap_premap, 0);

    // The host-object region: a handset that names it keeps the number, including none, and the
    // carrier never supplies a default for it.
    let mine = DeviceProfile {
        object_premap: 0x20_0000,
        declared: crate::device::Declared {
            object_premap: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert_eq!(ktf.apply_to(mine.clone()).object_premap, 0x20_0000);
    let none = DeviceProfile {
        declared: crate::device::Declared {
            object_premap: true,
            ..Default::default()
        },
        ..DeviceProfile::standard()
    };
    assert_eq!(ktf.apply_to(none).object_premap, 0);
}

/// The memory a carrier brings has to be there when the machine is built, not merely described on
/// the profile: `yongman2` reads a scroll buffer that begins below the heap and walks 12 MiB up
/// through pages the allocator never hands out.
#[test]
fn the_carriers_memory_is_actually_mapped_when_the_machine_is_built() {
    let ktf = resolve(
        None,
        Some(carrier_by_name("ktf").unwrap()),
        &compat(None, None),
        std::path::Path::new(LOOSE_PACK),
    )
    .unwrap();
    let profile = ktf.apply_to(crate::device::DeviceProfile::standard());
    let rt = crate::runtime::Runtime::with_device(profile);

    // 0x1ffe8074 is where sub_181D4 points the map: the heap context minus 0x18000. The game
    // can also scroll down, to 0x1fef8074 on the second step.
    assert_eq!(rt.cpu.mem.read_u16(0x1ffe_8074), Ok(0));
    assert_eq!(rt.cpu.mem.read_u16(0x1fef_8074), Ok(0));
    // The far end of the 0xC00000-byte scroll, still inside the heap's pre-mapped span.
    assert_eq!(rt.cpu.mem.read_u16(0x20c0_0034), Ok(0));
    // The pre-map is a bounded promise, not a licence to read the whole arena.
    assert!(rt.cpu.mem.read_u16(0x2100_0000).is_err());
}

/// A handset that declares object-region RAM has it mapped when the machine is built, so a probe
/// that runs past a host object reads bytes instead of faulting. Undeclared keeps the lazy mapping.
#[test]
fn a_handsets_object_region_ram_is_mapped_when_the_machine_is_built() {
    // A page far past any object a fresh machine hands out. With the lazy mapping it is a hole;
    // with a declaration it is RAM. `0xa000` cannot serve: the first objects already cover it.
    let far = crate::layout::OBJECT_BASE + 0x18_0000;
    let declared = crate::device::DeviceProfile {
        object_premap: 0x20_0000,
        declared: crate::device::Declared {
            object_premap: true,
            ..Default::default()
        },
        ..crate::device::DeviceProfile::standard()
    };
    let rt = crate::runtime::Runtime::with_device(declared);
    assert_eq!(rt.cpu.mem.read_u16(far), Ok(0));

    // A declaration larger than the gap is clamped to the guest heap, not allowed to spill into it.
    let huge = crate::device::DeviceProfile {
        object_premap: 0x2000_0000,
        declared: crate::device::Declared {
            object_premap: true,
            ..Default::default()
        },
        ..crate::device::DeviceProfile::standard()
    };
    let rt = crate::runtime::Runtime::with_device(huge);
    assert!(rt
        .cpu
        .mem
        .read_u16(crate::layout::HEAP_BASE - 4)
        .is_ok());

    let lazy = crate::runtime::Runtime::with_device(crate::device::DeviceProfile::standard());
    assert!(lazy.cpu.mem.read_u16(far).is_err());
}

/// The windows a carrier brings must land in memory this host does not use for anything else. This is
/// the whole risk of mapping guest-chosen spans, so it is a test rather than a comment: if the
/// layout later grows a region into `0x0180_0000..0x0200_0000`, this fails here instead of as a stub
/// overwriting a save file on somebody's real run.
#[test]
fn a_carriers_memory_lands_where_the_host_owns_nothing() {
    use crate::layout;
    let last_id = ((crate::abi::IFACE_COUNT as u32) << 12) | (crate::abi::MAX_METHODS as u32 - 1);
    let stubs_end = layout::STUB_BASE + (last_id + 1) * crate::abi::VTABLE_SLOT_SIZE as u32;
    let owned = [
        ("module", layout::MODULE_BASE, 0x0040_0000),
        ("stubs", layout::STUB_BASE, stubs_end - layout::STUB_BASE),
        (
            "vtables",
            layout::VTABLE_BASE,
            layout::OBJECT_BASE - layout::VTABLE_BASE,
        ),
        (
            "heap",
            layout::HEAP_BASE,
            layout::HEAP_LIMIT - layout::HEAP_BASE,
        ),
        (
            "stack",
            layout::STACK_TOP - layout::STACK_SIZE,
            layout::STACK_SIZE,
        ),
    ];
    for carrier in carriers().filter(|c| !c.ram.is_empty()) {
        for w in carrier.ram {
            assert!(w.len > 0, "{}: a window of nothing", carrier.label);
            let end = w.base.checked_add(w.len).expect("no wrap");
            for (name, base, len) in owned {
                assert!(
                    end <= base || base + len <= w.base,
                    "{}'s window 0x{:08x}..0x{end:08x} overlaps this host's {name} region",
                    carrier.label,
                    w.base
                );
            }
        }
    }
}
