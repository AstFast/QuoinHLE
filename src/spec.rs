//! Which BREW generation, and which carrier's private extension set, a run is aimed at.
//!
//! The host answers from one ABI table — the classic 4.0.2 one — and that table is a superset of
//! almost every older release: an interface keeps its published slots in the order it declared them,
//! so a 1.x pack's `(interface, slot)` pair still names the same method on 4.0.2. What a pack cannot
//! tell from its own code is which answers the *firmware* it was built against gave: whether a
//! structure is 60 bytes or 64, whether an interface existed at all, whether `DrawText` takes bytes
//! or `AECHAR`. Those are properties of a generation, not of a handset, and this module names the
//! generations so that a run can say which one it is pretending to be.
//!
//! Two axes, deliberately independent:
//!
//! * [`SpecId`] — the BREW release the firmware reference (`1.1` … `4.0.2`, and Brew MP after it).
//! * [`CarrierId`] — the operator whose private classes and interfaces sit beside the public ABI,
//!   which is The carrier's MultiPack on one side and a carrier's application store on the other.
//!
//! `1.2.3` on a carrier handset and `2.0` on a carrier one are both real combinations, so a single enum
//! would have to spell out every pair; two axes keep the table linear.
//!
//! **Nothing here changes behaviour yet.** This module resolves names and reports them; the
//! generation's structure sizes, published-interface set and semantics are filled in by the stages
//! that consume them, and a run that names no generation keeps every answer it gives today.
//!
//! ```json
//! { "version": "1.2", "platform": "classic-1.2.3", "carrier": "ktf" }
//! ```

/// A BREW release whose firmware a run can pretend to be.
///
/// Ordered oldest first, which is also the order a `since` table would index them in; the order is
/// part of the API because a generated table will key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecId {
    Classic1_1,
    Classic1_2,
    Classic2_0,
    Classic3_1,
    Classic4_0,
    Mp7_12,
}

/// One generation, and the names a person may use for it.
#[derive(Debug)]
pub struct BrewSpec {
    pub id: SpecId,
    /// The canonical name, as it is printed in a report and written in a declaration.
    pub label: &'static str,
    /// Other spellings accepted on the command line and in a declaration. The canonical name is
    /// accepted too and is not repeated here.
    pub aliases: &'static [&'static str],
    /// `sizeof(AEEDeviceInfo)` as this generation's own headers declare it, or `None` where no
    /// header of that release has been read.
    ///
    /// Measured, not inferred: 1.1 and its interface declarations, 2.0, 3.1.2, 4.0.2 and MP 7.12.5 all
    /// declare 64 bytes, with `dwPlatformID` last at offset 60 — the 60-byte form belongs to the
    /// release before 1.1. `1.2.3` has no header tree anywhere in this project, so it has no
    /// opinion and a handset's own declaration (The carrier's, at 60) stands.
    pub device_info_size: Option<u16>,
    /// Link address of a `.mod` on this generation's handsets.
    ///
    /// The image is PIC, but a guest pack carries absolute data pointers fixed at its link
    /// address. The carrier's 1.2.3 image proves `0x8000` (a sample pack' MT table literal `0x27798` minus the
    /// module base is exactly the table's file offset). The 2.x/3.x/4.x value is the address this
    /// host mapped modules at before the carrier fix, under which the au sample set ran; no au firmware in
    /// this tree has been inspected far enough to quote a stronger record. A device file can
    /// override either by naming `loader.module_base`.
    pub module_base: u32,
}

/// An operator whose private extension set sits beside the public ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CarrierId {
    /// The public ABI and nothing else: the default, and what every non-carrier handset answers.
    Generic,
    /// The carrier's MultiPack: the `0x018000fe` device-information class, the DRM class, `fs:/mod/<id>/`.
    Ktf,
    /// a carrier's application store: `IKDDIPHONESYSTEM`, `IKDDIPPP`, the `0x58f1` subscriber item, Shift-JIS.
    Kddi,
    /// Zeebo: a console running Brew MP. Registered so a profile can name it; no private ABI yet.
    Zeebo,
}

/// One carrier, and the names a person may use for it.
#[derive(Debug)]
pub struct Carrier {
    pub id: CarrierId,
    pub label: &'static str,
    pub aliases: &'static [&'static str],
    /// The text-side answers this operator's firmware gives a pack.
    ///
    /// These are carrier facts rather than generation facts: a Korean 1.x handset reads EUC-KR and a
    /// Japanese one Shift-JIS, on the same BREW release. A handset that names its own value keeps
    /// it — see [`crate::device::Declared`].
    pub text: TextDefaults,
    /// Spans of writable memory this carrier's firmware has, which its packs' saves point into.
    ///
    /// Empty means the public ABI's own layout and nothing else. The carrier's 1.2.3 handsets keep their
    /// per-app heaps and fixed buffers in `0x0180_0000..0x0200_0000`, and their packs ship saves
    /// that hold absolute pointers into it — see [`crate::device::RamWindow`] and
    /// the project research notes for the evidence behind the numbers.
    pub ram: &'static [crate::device::RamWindow],
    /// Bytes of the guest heap's own span this firmware treats as RAM from the start. `0` keeps
    /// the host's lazy mapping. See [`crate::device::DeviceProfile::heap_premap`].
    pub heap_premap: u32,
    /// Bytes of the host-object region this firmware has as RAM from the start. `0` keeps the
    /// host's per-object mapping. See [`crate::device::DeviceProfile::object_premap`].
    pub object_premap: u32,
}

/// The character set and the two face sizes a carrier's firmware ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDefaults {
    /// `AEE_ENC_*`.
    pub encoding: u16,
    /// Cell height for `AEE_FONT_NORMAL`.
    pub font_normal: u16,
    /// Cell height for `AEE_FONT_LARGE`.
    pub font_large: u16,
    /// Whether `IDisplay::DrawText`'s `pcText` is the device's own 8-bit string.
    ///
    /// This is an operator fact and not a release fact, which the headers settled: every Qualcomm
    /// tree from 1.1 up declares `const AECHAR *pcText` with `AECHAR` a `uint16`, so no BREW release
    /// hands `DrawText` bytes. The carrier's 1.2.3 firmware does — its packs pass `char *` and the sample set
    /// only runs with this true — so the OEM must have kept the pre-`AECHAR` text ABI.
    pub text_bytes: bool,
}

impl TextDefaults {
    /// What this host answers when nothing names a carrier: the built-in handset's own numbers, so
    /// that the generic carrier is not a change to any existing run.
    const HOST: TextDefaults = TextDefaults {
        encoding: crate::abi::generated::AEE_ENC_S_JIS as u16,
        font_normal: 16,
        font_large: 32,
        text_bytes: false,
    };
}

/// Every generation this build knows, oldest first.
pub const SPECS: &[BrewSpec] = &[
    BrewSpec {
        id: SpecId::Classic1_1,
        label: "classic-1.1",
        aliases: &["1.1", "brew-1.1"],
        device_info_size: Some(64),
        module_base: 0x0000_8000,
    },
    BrewSpec {
        id: SpecId::Classic1_2,
        label: "classic-1.2.3",
        aliases: &["1.2", "1.2.3", "brew-1.2.3"],
        device_info_size: None,
        module_base: 0x0000_8000,
    },
    BrewSpec {
        id: SpecId::Classic2_0,
        label: "classic-2.0",
        aliases: &["2.0", "brew-2.0"],
        device_info_size: Some(64),
        module_base: 0x1000_0000,
    },
    BrewSpec {
        id: SpecId::Classic3_1,
        label: "classic-3.1.2",
        aliases: &["3.1", "3.1.2", "brew-3.1.2"],
        device_info_size: Some(64),
        module_base: 0x1000_0000,
    },
    BrewSpec {
        id: SpecId::Classic4_0,
        label: "classic-4.0.2",
        aliases: &["4.0", "4.0.2", "brew-4.0.2"],
        device_info_size: Some(64),
        module_base: 0x1000_0000,
    },
    BrewSpec {
        id: SpecId::Mp7_12,
        label: "mp-7.12.5",
        aliases: &["mp", "brew-mp", "7.12.5"],
        device_info_size: Some(64),
        module_base: 0x1000_0000,
    },
];

/// The writable span a carrier BREW 1.2.3 handset has, as its packs' saves describe it.
///
/// The evidence is in the project research notes §3: `yongman2`'s `save.dat` holds
/// `0x01FA_2B68` and `0x01FA_2C58`, 영웅서기's four versions all hold `0x01FF_0010`, 월드오브드래곤
/// holds `0x01B2_BA18` — pointers that repeat across releases of the same pack can only be
/// "fixed base + small offset". The base is `0x0180_0000`, the top is the observed ceiling
/// `0x0200_0000`, and neither falls on anything this host owns: the stub space ends at
/// `0x0100_0968` for the largest dispatch id the ABI can encode, the vtable table starts exactly at
/// `0x0200_0000`, and the guest heap at `0x2000_0000`.
const KTF_RAM: &[crate::device::RamWindow] = &[
    crate::device::RamWindow {
        base: 0x0180_0000,
        len: 0x0080_0000,
    },
    // The scroll buffer `yongman2` indexes starts 0x18000 *below* its heap context and can walk
    // down as well as up (the game's direction decides). The context lives at the bottom of the
    // host's heap (0x2000_0000), so these are handset RAM pages the host would otherwise leave as
    // a hole. 16 MiB covers the pack's 12 MiB wrap in either direction. The window ends exactly
    // where the heap begins; it never takes a page the allocator owns.
    crate::device::RamWindow {
        base: 0x1f00_0000,
        len: 0x0100_0000,
    },
];

/// How much of the heap's own span The carrier's firmware brings in as RAM. `yongman2` steps a tile-map
/// pointer by 0x18000 each frame through a 0xC00000-byte span (`sub_181D4`/`sub_14C60` in its
/// module), reading it without ever allocating it. 16 MiB covers the whole span and the wrap at
/// its far end; a lazily grown heap faults at the first unallocated page.
const KTF_HEAP_PREM: u32 = 0x0100_0000;

/// Every carrier this build knows.
pub const CARRIERS: &[Carrier] = &[
    Carrier {
        id: CarrierId::Generic,
        label: "generic",
        aliases: &["none", "public"],
        text: TextDefaults::HOST,
        ram: &[],
        heap_premap: 0,
        object_premap: 0,
    },
    Carrier {
        id: CarrierId::Ktf,
        label: "ktf",
        aliases: &["multipack", "ktf-multipack"],
        // EUC-KR and the built-in face sizes: what the sample set's own device file declares, so a
        // A carrier pack run without one now gets the right encoding instead of the host's Shift-JIS.
        text: TextDefaults {
            encoding: crate::abi::generated::AEE_ENC_EUC_KR as u16,
            font_normal: 16,
            font_large: 32,
            text_bytes: true,
        },
        ram: KTF_RAM,
        heap_premap: KTF_HEAP_PREM,
        object_premap: 0,
    },
    Carrier {
        id: CarrierId::Kddi,
        label: "kddi",
        aliases: &["ez", "ez-appli", "au"],
        // au's EZ Appli firmware is Japanese: Shift-JIS, and 1xWIN packs are laid out for 20/24 px
        // faces rather than this host's 16/32.
        text: TextDefaults {
            encoding: crate::abi::generated::AEE_ENC_S_JIS as u16,
            font_normal: 20,
            font_large: 24,
            text_bytes: false,
        },
        ram: &[],
        heap_premap: 0,
        object_premap: 0,
    },
    Carrier {
        id: CarrierId::Zeebo,
        label: "zeebo",
        aliases: &[],
        text: TextDefaults::HOST,
        ram: &[],
        heap_premap: 0,
        object_premap: 0,
    },
];

/// The generation a run is served as when nothing names one.
///
/// It is the table this host answers from, so "no declaration" and "declare 4.0.2" mean the same
/// machine — which is what makes an undeclared pack's behaviour unchanged by this module existing.
pub const DEFAULT_SPEC: SpecId = SpecId::Classic4_0;
pub const DEFAULT_CARRIER: CarrierId = CarrierId::Generic;

/// Every generation, oldest first.
pub fn specs() -> impl Iterator<Item = &'static BrewSpec> {
    SPECS.iter()
}

/// Every carrier.
pub fn carriers() -> impl Iterator<Item = &'static Carrier> {
    CARRIERS.iter()
}

/// The entry for an id. Panics only if the table is missing an id it declares, which is a bug in
/// this file rather than something a command line can cause.
pub fn spec(id: SpecId) -> &'static BrewSpec {
    SPECS
        .iter()
        .find(|s| s.id == id)
        .expect("every SpecId has an entry in SPECS")
}

pub fn carrier(id: CarrierId) -> &'static Carrier {
    CARRIERS
        .iter()
        .find(|c| c.id == id)
        .expect("every CarrierId has an entry in CARRIERS")
}

/// Resolve a generation by canonical name or alias, ignoring ASCII case.
pub fn spec_by_name(name: &str) -> Result<&'static BrewSpec, String> {
    let name = name.trim();
    SPECS
        .iter()
        .find(|s| {
            s.label.eq_ignore_ascii_case(name)
                || s.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| {
            format!(
                "unknown platform \"{name}\"; known: {}",
                SPECS.iter().map(|s| s.label).collect::<Vec<_>>().join(", ")
            )
        })
}

/// Resolve a carrier by canonical name or alias, ignoring ASCII case.
pub fn carrier_by_name(name: &str) -> Result<&'static Carrier, String> {
    let name = name.trim();
    CARRIERS
        .iter()
        .find(|c| {
            c.label.eq_ignore_ascii_case(name)
                || c.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| {
            format!(
                "unknown carrier \"{name}\"; known: {}",
                CARRIERS
                    .iter()
                    .map(|c| c.label)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Where a resolved value came from, so a report can say why the run is the machine it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `--platform` / `--carrier`: the person running it said so.
    Cli,
    /// The pack's own `<stem>.compat.json`.
    Pack,
    /// The sample set directory the pack sits in — The carrier's tree is the one this build knows.
    Directory,
    /// Nothing named one, so the machine stayed the one this host already is.
    Default,
}

impl Source {
    /// The word the report prints in brackets.
    pub fn tag(self) -> &'static str {
        match self {
            Source::Cli => "flag",
            Source::Pack => "pack",
            Source::Directory => "directory",
            Source::Default => "default",
        }
    }
}

/// A generation and a carrier, with where each came from.
#[derive(Debug)]
pub struct Resolved {
    pub spec: &'static BrewSpec,
    pub carrier: &'static Carrier,
    pub spec_from: Source,
    pub carrier_from: Source,
}

/// Sample directories that say which machine their packs were built for.
///
/// A marker is matched against the *components* of the pack's path rather than as a substring, so a
/// directory called `A carrier BREW notes` cannot claim a pack by accident. Only directories recorded by
/// the sample set are listed: guessing from a filename is the kind of inference that turns a host bug
/// into a "pack property" nobody can check.
const DIR_MARKERS: &[(&str, SpecId, CarrierId)] =
    &[("KTF BREW", SpecId::Classic1_2, CarrierId::Ktf)];

/// What a directory says about the packs under it.
fn directory_of(path: &std::path::Path) -> Option<(SpecId, CarrierId)> {
    path.components().find_map(|c| {
        let std::path::Component::Normal(name) = c else {
            return None;
        };
        let name = name.to_str()?;
        DIR_MARKERS
            .iter()
            .find(|(marker, _, _)| marker.eq_ignore_ascii_case(name))
            .map(|(_, spec, carrier)| (*spec, *carrier))
    })
}

/// Combine what the command line said, what the pack declares, and what its directory implies.
///
/// Precedence is `Cli > Pack > Directory > Default`, and each axis is decided on its own: a run may
/// take the generation from a flag while the carrier still comes from the directory.
///
/// The pack's names are resolved here rather than while the declaration is read, because this is the
/// only place that knows the file the names came from — and an error that says
/// `a sample pack.compat.json: unknown platform "x"` is worth more than one that says the name is unknown.
pub fn resolve(
    cli_spec: Option<&'static BrewSpec>,
    cli_carrier: Option<&'static Carrier>,
    compat: &crate::compat::Compat,
    mod_path: &std::path::Path,
) -> Result<Resolved, String> {
    // The file the names came from, so a refusal can say which declaration is wrong.
    let at = crate::compat::path_for(mod_path);
    let declared_spec = |name: &str| -> Result<&'static BrewSpec, String> {
        spec_by_name(name).map_err(|e| format!("{}: {e}", at.display()))
    };
    let declared_carrier = |name: &str| -> Result<&'static Carrier, String> {
        carrier_by_name(name).map_err(|e| format!("{}: {e}", at.display()))
    };
    let dir = directory_of(mod_path);
    let from_dir = |id: SpecId| -> &'static BrewSpec { spec(id) };
    let from_dir_carrier = |id: CarrierId| -> &'static Carrier { carrier(id) };

    let (spec, spec_from) = match (cli_spec, compat.platform.as_deref()) {
        (Some(s), _) => (s, Source::Cli),
        (None, Some(name)) => (declared_spec(name)?, Source::Pack),
        (None, None) => match dir {
            Some((id, _)) => (from_dir(id), Source::Directory),
            None => (spec(DEFAULT_SPEC), Source::Default),
        },
    };
    let (carrier, carrier_from) = match (cli_carrier, compat.carrier.as_deref()) {
        (Some(c), _) => (c, Source::Cli),
        (None, Some(name)) => (declared_carrier(name)?, Source::Pack),
        (None, None) => match dir {
            Some((_, id)) => (from_dir_carrier(id), Source::Directory),
            None => (carrier(DEFAULT_CARRIER), Source::Default),
        },
    };
    Ok(Resolved {
        spec,
        carrier,
        spec_from,
        carrier_from,
    })
}

impl Resolved {
    /// The handset this run should use: the profile as given, with the generation's and the
    /// carrier's answers filled in only where the handset did not name its own.
    ///
    /// This is the one place the three sources meet — handset, generation, carrier — so the tie-break
    /// lives here rather than at the ninety thousand lines that read the fields. A device file that
    /// says 60 means 60 even under a 4.0.2 run; a handset that says nothing inherits whatever the
    /// generation and carrier it resolved to declared.
    ///
    /// Two fields that look like they belong here deliberately do not:
    ///
    /// * `convertbmp` is not a generation property. Returning pixels as the 1.x OEM answer for the
    ///   whole generation was tried and withdrawn — packs of the same age disagree about it — so it
    ///   stays a per-pack [`crate::compat`] declaration.
    /// * `heap_initial` already is 256 KiB ([`crate::heap::INITIAL`]) for every generation, which is
    ///   the number the 1.x A carrier guide asks for. There is nothing to split until something is
    ///   recorded to need more, and a knob with one value on both sides is not a feature.
    pub fn apply_to(&self, profile: crate::device::DeviceProfile) -> crate::device::DeviceProfile {
        let mut profile = profile;
        let said = profile.declared;
        if !said.device_info_size {
            if let Some(size) = self.spec.device_info_size {
                profile.device_info_size = size;
            }
        }
        if !said.module_base {
            profile.module_base = self.spec.module_base;
        }
        let text = self.carrier.text;
        if !said.text_bytes {
            profile.text_bytes = text.text_bytes;
        }
        let text = self.carrier.text;
        if !said.encoding {
            profile.encoding = text.encoding;
        }
        if !said.font_normal {
            profile.font_normal = text.font_normal;
        }
        if !said.font_large {
            profile.font_large = text.font_large;
        }
        if !said.ram_windows {
            profile.ram_windows = self.carrier.ram.to_vec();
        }
        if !said.heap_premap {
            profile.heap_premap = self.carrier.heap_premap;
        }
        if !said.object_premap {
            profile.object_premap = self.carrier.object_premap;
        }
        profile
    }

    /// The one line a run prints about this, e.g.
    /// `platform: classic-1.2.3 (carrier ktf) [directory]`.
    ///
    /// Each axis names its own source, because the two are usually decided by different things: a
    /// flag names the generation while the directory names the carrier.
    pub fn line(&self) -> String {
        format!(
            "platform: {} (carrier {}) [{}{}]",
            self.spec.label,
            self.carrier.label,
            self.spec_from.tag(),
            if self.spec_from == self.carrier_from {
                String::new()
            } else {
                format!("/{}", self.carrier_from.tag())
            }
        )
    }
}

#[cfg(test)]
#[path = "test/spec.rs"]
mod tests;
