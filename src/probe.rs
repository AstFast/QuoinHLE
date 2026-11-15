//! What a pack says about itself **without running it**.
//!
//! `--device` is a human answer to a question the pack can partly answer for itself, and getting it
//! wrong is silent: a pack reads `AEEDeviceInfo`, lays its art out for that canvas, and draws a cropped
//! or misaligned picture without failing, without a note, and with the same presentation count
//! and nothing else changes). So this module asks the pack's own
//! bytes instead: its module header, its containers, and above all the pictures it carries, because a
//! game's art is made for the screen it will be shown on.
//!
//! Nothing here starts a guest. There is no `Runtime`, no instruction is executed and no address is
//! trusted — which is what makes it safe to run over a whole directory of packs, and what bounds what it
//! may claim: a number read out of a container is the pack's *art*, and only a run shows what the pack
//! does with it.

use std::collections::BTreeMap;
use std::path::Path;

use crate::abi;
use crate::audio;
use crate::device::DeviceProfile;
use crate::disasm;
use crate::library;
use crate::loader;
use crate::par;
use crate::resource;
use crate::store::{self, Record, Store};

/// Below this a picture is a sprite or an icon, not evidence about a screen.
///
/// A chosen bound, not a measured one, and the measurement says why it cannot be tighter: the picture
/// the library shows as a pack's icon is always the **smallest** one in its container (16x16 in all 35
/// builds measured), but the containers hold other small pictures too — 26x26,
/// 50x50 and 65x42 all occur — and 65 is already wider than `MIN_ART`. So 32 keeps the true icons out and
/// cannot keep the icon-sized ones out; what does that job is [`THIN_ART`], which is why the report
/// carries a warning rather than trusting a width.
const MIN_ART: usize = 32;

/// How many of the decoded pictures the report lists, biggest first.
const SHOW_ART: usize = 5;

/// The mark on the report's own warning line.
const CAUTION_MARK: char = '⚠';

/// Whether a line of the report is that report's own warning.
///
/// The library's pane colours by this instead of carrying its own copy of the rule: the warning is
/// printed once by `Probe::lines`, and a second site deciding "is this the warning line" is how a line
/// ends up on screen twice - which this one already did once.
pub fn is_caution(line: &str) -> bool {
    line.contains(CAUTION_MARK)
}

/// The first word of the fixup routine, on all 14 `BREW`-headered modules in this corpus and on the
/// store sample. `src/loader.rs` clears the range that ends there, so a module whose word differs is
/// either not this format or something the loader has never been measured against.
const FIXUP_PUSH: u32 = 0xe92d_00f0;

/// One picture the host can actually decode out of the pack's containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub from: String,
    pub id: u32,
    pub width: usize,
    pub height: usize,
}

/// What the pack's own art says about the canvas it was drawn for.
///
/// Two facts, in this order:
///
/// - **Width**: a status bar, a soft-key bar, a full-width divider — a picture much wider than it is
///   tall spans the screen, so nothing in the pack is wider than the screen and the widest of those
///   bars is the best width the art gives. With no bar at all the rule falls back on the height the
///   pictures that are big in **both** directions agree on, and the narrowest width carried at that
///   height; the report says which of the two it used.
/// - **Height**: the tallest picture drawn *at that width*. A background is one screenful, so this is
///   the height the pack actually needs — and it is at most the device height, because the chrome above
///   and below the canvas is not part of the picture — a 17-pixel status bar is why one pack needs 223
///   rows on a 240-row handset. **That 320x223 is not this module's measurement**: it is the hand-count
///   from a hand count of one pack, and `--info` cannot see it, because such a
///   background lives in `i0..i42` slices rather than in any record this host mounts.
///   It is an example of the shape, not a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub width: usize,
    pub height: usize,
    /// The width the bar set, when a bar set it. `None` means the pack has no span and the fallback
    /// rule answered instead — which is a different strength of evidence, so the report says which.
    pub bar: Option<usize>,
    /// How many pictures are drawn at that width.
    pub at_that_width: usize,
    /// The largest picture by area in the evidence the screen answer used: mounted records first,
    /// and the anonymous `.par` leaves this host can decode but cannot mount.
    pub largest: Option<(usize, usize)>,
    /// How many pictures were wide enough to count at all.
    pub considered: usize,
}

impl Screen {
    /// The noun phrase naming which of the two rules set this width, without the "no bar" preface.
    fn source(&self) -> String {
        match self.bar {
            Some(bw) => format!("a {bw}-pixel-wide bar"),
            // Not "the widest picture": the fallback groups the pictures that are big in both
            // directions by height and takes the **narrowest** width at the height most of them share,
            // which is what keeps a 531-pixel scroll strip from becoming a 531-pixel handset
            // (`the_scroll_strip_is_not_the_screen` pins it).
            None => "the narrowest width at the shared height of the large pictures".to_string(),
        }
    }

    /// The same phrase as the report's parenthetical, which has to say outright that there was no bar.
    pub fn width_source(&self) -> String {
        match self.bar {
            Some(_) => self.source(),
            None => format!("no bar; falling back to {}", self.source()),
        }
    }

    /// The one sentence that says what this canvas claim is backed by.
    ///
    /// It is a method rather than a clause inside [`Probe::lines`] because the library's size picker
    /// shows this sentence beside the number it explains — and a second wording of one fact is the drift
    /// this repository keeps being bitten by (a second wording of one fact drifts
    /// away from the first).
    pub fn evidence(&self) -> String {
        format!(
            "width from {}; {} picture(s) at that width, the tallest gives {}",
            self.source(),
            self.at_that_width,
            self.height
        )
    }
}

/// Word 0 of the image, read both ways.
///
/// There is no cheap bit test that tells an ARM entry from a Thumb one: a Thumb halfword may have zero
/// in its low two bits (`blx r7` is `0x4778`), and an arbitrary pair of halfwords still decodes as
/// *some* ARM word. So this asks the host's own decoder what each reading gives and prints both. The
/// one firm statement is the loader's — a word whose condition field is `0xf…` is no instruction since
/// ARMv4, so ARM entry is refused, and that is when Thumb is the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub word: u32,
    /// The first instruction read as ARM, or `None` when the decoder refused it.
    pub arm: Option<String>,
    /// The same bytes read as Thumb.
    pub thumb: Option<String>,
    /// Whether `load()` would refuse this image told to enter in ARM.
    pub refuses_arm: bool,
    /// What this host enters with: `ARM`, `Thumb`, or `unknown`.
    pub suggests: &'static str,
}

/// One container's inventory.
#[derive(Debug, Default)]
pub struct Census {
    pub records: usize,
    /// `RESTYPE_*` tallies.
    pub kinds: BTreeMap<String, usize>,
    /// Declared-MIME tallies. `(no declaration)` is a record whose property block names nothing, which is
    /// every `.par` leaf.
    pub mimes: BTreeMap<String, usize>,
    pub pictures: Vec<Picture>,
    /// Records filed under the image kind that no reader here can decode — MIDI, text, and the private
    /// formats a pack decodes for itself.
    pub undecodable: usize,
}

/// One line of the audio census. Its **two** counts answer two different questions a person asks of a
/// pack, which used to be collapsed into one hand-written
/// `starts_with("audio/")` that could only answer the weaker of the two.
/// [`AudioCensus::recognised`] then qualifies the second of them.
///
/// * [`AudioCensus::playable`] — the payload was recognised by [`crate::audio::sniff`], which
///   *parses* it. This is the count that means "this host could make a sound out of it", and it is
///   the only one that can see a clip whose property block names nothing (every `.par` leaf) or names
///   something else (the corpus files MIDI under the image kind).
/// * [`AudioCensus::declared`] — the property block named a sound (either SDK sound base, per
///   [`crate::audio::names_a_sound`]) and the authority did not recognise the bytes. That is an
///   inventory fact, not a capability: an AMR or an MP3 clip, or a file whose extension lies. The name
///   is stored trimmed, so `"audio/mid "` joins `audio/mid` rather than starting a line of its own.
/// * [`AudioCensus::recognised`] — within those, the records whose bytes the same module's byte
///   authority places ([`crate::audio::recognises`]).
///   Still an inventory count — the row says so on the line it prints — but a *witnessed* one, which is
///   the difference between "205 records are called `audio/mpeg`" and "205 records are MP3".
///
/// A payload is counted once and once only, under the authority's name when it has one: the full
/// declared-MIME tally is already [`Census::mimes`], so preferring the byte answer here loses nothing
/// that the report needs.
///
/// **The unit is a resource record, not a distinct byte span.** [`crate::resource::Container::all`]
/// yields one record per id, and a container can hang two ids off the same span — two map entries
/// sharing an `offset_index`, or one entry's `extra_ids` run landing on a table slot another entry
/// already used (`hmx.bar`: entry 0 covers ids 5001..5003 at index 0, entry 1 covers 5004 at the same
/// index, so 5001 and 5004 read the same bytes). Measured: five packs do it, one clip each. That
/// matches what `IShell::LoadResource(id)` would agree to serve, but it means the byte figures are
/// per-request too, and the count is not a census of distinct songs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AudioCensus {
    pub mime: String,
    pub playable: usize,
    pub playable_bytes: usize,
    pub declared: usize,
    pub declared_bytes: usize,
    /// Of the records filed **by name** in this row, how many the byte authority still recognises as
    /// a specific format, by the name it gave them. See [`crate::audio::recognises`]: that is the
    /// byte answer ("what are these"), which was wider than the playable set until `symphonia` landed,
    /// and this count is what tells a person whether writing a decoder for the family the container
    /// named would actually reach audio — without this row claiming the host can play a single one.
    ///
    /// The keys are the authority's names, which are not this row's key: a pack that declares
    /// `audio/mpeg` gets its bytes recognised as `audio/mp3`, because that is what BREW calls it.
    pub recognised: BTreeMap<String, usize>,
}

impl AudioCensus {
    fn add(&mut self, count: usize, bytes: usize, by_bytes: bool) {
        if by_bytes {
            self.playable += count;
            self.playable_bytes += bytes;
        } else {
            self.declared += count;
            self.declared_bytes += bytes;
        }
    }
}

/// The `BREW` header a shipped module reads for itself at load, and the one fact about it that is
/// checkable without running anything. The offsets and the "does this header describe the file" test are
/// the loader's (`loader::relocation`, `loader::relocation_describes`) — read here, never re-derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Brew {
    pub image: u32,
    pub ref_off: u32,
    pub fixup_off: u32,
    pub ref_size: u32,
    /// Whether the range the header names actually lands in the bytes there are. `false` is the loader's
    /// own verdict that the header is not describing this file.
    pub describes: bool,
    /// The word at `image + ref_off + fixup_off + ref_size`, or `None` when that lands past the end —
    /// which is itself an answer, because the header then describes a bigger image than there are bytes.
    pub fixup_word: Option<u32>,
}

/// The app's own record in its manifest, read at the offsets the ABI tables publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppInfo {
    pub cls: u32,
    pub id_base: u16,
    pub app_type: u16,
    pub flags: u16,
}

/// What a PAR container contributes to the report when the mount could not take all of it.
///
/// The counts come from the same walk as [`crate::par::read`], so they explain a silent mount
/// rather than competing with it. When the file has anonymous leaves, `census` is the one returned
/// by the inflating [`crate::par::leaves_and_census`], so `unreadable` can be more precise than
/// [`crate::par::census`] (which does not inflate those leaves). The audio and picture counts
/// describe only the leaves with no id: the ones a mount can serve are already in the container's
/// record counts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParReport {
    pub census: par::Census,
    /// Unnamed leaves whose bytes are a MIDI file this host can play.
    pub unnamed_audio: usize,
    /// Unnamed leaves that decode as a picture.
    pub unnamed_pictures: usize,
    /// The largest unnamed picture by area, when there is one.
    pub largest_picture: Option<(usize, usize)>,
    /// Every unnamed picture's dimensions, in the order the leaves appear. The screen inference
    /// uses these; `Probe::pictures` keeps only the mounted art it has always meant.
    pub picture_sizes: Vec<(usize, usize)>,
}

/// Everything one pack tells without being run.
#[derive(Debug)]
pub struct Probe {
    pub path: std::path::PathBuf,
    pub bytes: usize,
    /// Word 0, which is the first instruction of `AEEMod_Load`, read both ways.
    pub word0: Entry,
    /// What the loader would have said about this image, kept as words because `LoadError` is not
    /// something a report has to hold on to.
    pub refusal: Option<String>,
    pub brew: Option<Brew>,
    pub app_info: Option<AppInfo>,
    /// The manifest the app's own records came from, by name.
    pub manifest: Option<String>,
    pub containers: Vec<(String, Census)>,
    /// `(file, id, how many strings, the first one)`.
    pub strings: Vec<(String, u32, usize, String)>,
    /// The sounds in the pack, grouped by MIME. See [`AudioCensus`] for why one line carries two
    /// counts and what each of them is a witness of.
    pub audio: Vec<AudioCensus>,
    /// Sibling files a run cannot reach, with the size that explains it.
    pub unmounted: Vec<(String, u64)>,
    /// PAR containers the host recognised but could not mount whole, with the counts that explain
    /// which leaves it can name and which it cannot, and what the unnamed leaves turn out to be
    /// (playable MIDI, decodable pictures) when asked by their bytes.
    pub par_census: Vec<(String, ParReport)>,
    /// Every picture the host can decode out of the **mounted** containers, biggest first. Anonymous
    /// `.par` leaves are not in this list — they cannot be served by an id lookup — and are reported
    /// separately; the screen inference uses both sets.
    pub pictures: Vec<Picture>,
    pub screen: Option<Screen>,
    /// The size the pack's *directory name* claims, when somebody wrote one. A hint, not a witness:
    /// this host has been wrong about these before.
    pub hint: Option<(u16, u16)>,
    /// What the host would answer with no `--device` at all.
    pub default: (u16, u16),
}

impl Probe {
    /// Read one pack. The path is the `.mod`; everything else is found beside and above it by the same
    /// rule the host uses when it mounts a run's resources.
    pub fn read(mod_path: &Path) -> Probe {
        let bytes = std::fs::read(mod_path).unwrap_or_default();
        let word0 = entry(&bytes);
        let mut store = Store::default();
        store.mount_beside(mod_path);
        let inventory = store.inventory();

        let mut containers = Vec::new();
        let mut pictures = Vec::new();
        let mut strings = Vec::new();
        let mut audio: BTreeMap<String, AudioCensus> = BTreeMap::new();
        for (name, records) in &inventory {
            let mut census = Census {
                records: records.len(),
                ..Default::default()
            };
            for r in records {
                *census.kinds.entry(kind_name(r.kind)).or_default() += 1;
                *census
                    .mimes
                    .entry(r.mime.clone().unwrap_or_else(|| "(no declaration)".into()))
                    .or_default() += 1;
                if let Some(p) = picture(name, r) {
                    census.pictures.push(p.clone());
                    pictures.push(p);
                } else if r.kind == resource::RESTYPE_IMAGE {
                    census.undecodable += 1;
                }
                if r.kind == resource::RESTYPE_STRING {
                    let langs = resource::strings(&r.body).unwrap_or_default();
                    let first = langs.first().map(|s| text_of(s)).unwrap_or_default();
                    strings.push((name.clone(), r.id, langs.len(), first));
                }
                // The two audio counts are decided here, in this order, and never summed: the parse
                // first because it is the only answer that says what the host can do with the bytes,
                // the declaration second because it is what the container chose to say about them.
                // The declaration is filed under its **trimmed** name, because that is how the same
                // strings are matched in `IShell::GetHandler`: a container that writes `"audio/mid "`
                // (the SDK's own constants have had trailing spaces) means the one MIME, and two lines
                // for one name would read as two formats.
                // `names_a_sound` is the authority module's own reading of the SDK's four MIME bases,
                // so the 1.x `snd/` spelling is not lost the way a local `starts_with("audio/")` lost
                // it; and the payload is not counted twice when both speak, because the authority's
                // name is the one the line is filed under.
                let declared = r
                    .mime
                    .as_deref()
                    .map(str::trim)
                    .filter(|m| audio::names_a_sound(m));
                let (key, by_bytes) = match (audio::sniff(&r.body), declared) {
                    (Some(by_bytes), _) => (by_bytes, true),
                    (None, Some(declared)) => (declared, false),
                    (None, None) => continue,
                };
                let e = audio.entry(key.to_owned()).or_insert_with(|| AudioCensus {
                    mime: key.to_owned(),
                    ..Default::default()
                });
                e.add(1, r.body.len(), by_bytes);
                // A row filed by name still gets its bytes asked, because the name is the container's
                // claim and the bytes are the payload's: `audio/mpeg×205` says "this pack has MP3s"
                // only if something behind those names is MPEG audio. Only the rows the parse could
                // not place are asked, since a placed one is already counted as playable — and that is
                // also why the answer here can only ever be a format this host has no decoder for.
                if !by_bytes {
                    if let Some(what) = audio::recognises(&r.body) {
                        *e.recognised.entry(what.to_owned()).or_insert(0) += 1;
                    }
                }
            }
            containers.push((name.clone(), census));
        }

        let mut order = pictures.clone();
        order.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));
        let par_census = par_census(mod_path);
        // Mounted pictures keep their own list; anonymous leaves join only the screen evidence,
        // because no LoadResData call can reach them. `id: 0` is never displayed — `infer_screen`
        // reads dimensions only.
        let mut screen_pictures = pictures.clone();
        for (name, report) in &par_census {
            for (w, h) in &report.picture_sizes {
                screen_pictures.push(Picture {
                    from: name.clone(),
                    id: 0,
                    width: *w,
                    height: *h,
                });
            }
        }
        let screen = infer_screen(&screen_pictures);
        Probe {
            path: mod_path.to_path_buf(),
            bytes: bytes.len(),
            word0,
            refusal: loader::precheck(&bytes, false).map(|e| e.to_string()),
            brew: brew_header(&bytes),
            app_info: store.applet_info().as_deref().and_then(app_info_of),
            manifest: store.manifest(),
            containers,
            strings,
            audio: audio.into_values().collect(),
            unmounted: unmounted(mod_path, &inventory),
            par_census,
            pictures: order,
            screen,
            hint: library::screen_of_dir(mod_path),
            default: {
                let d = DeviceProfile::standard();
                (d.width, d.height)
            },
        }
    }

    /// The report as lines. Built as a list rather than printed line by line because a report nobody
    /// can assert over hides exactly the kind of defect this one had: an early version printed its
    /// `⚠` warning twice, and no test could see it.
    pub fn lines(&self, label: Option<&str>) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(label) = label {
            out.push(format!("── {label} ──"));
        }
        out.push(format!(
            "module: {} ({} bytes)",
            self.path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".into()),
            self.bytes
        ));
        out.push(match &self.refusal {
            Some(e) => format!("  the loader would say: {e}"),
            None => format!(
                "  the loader will not refuse this image (without `--thumb-entry`; only memory failures are invisible statically)"
            ),
        });
        out.push(format!("  entry word0={:#010x}", self.word0.word));
        out.push(format!(
            "    read as ARM: {} | as Thumb: {}",
            self.word0.arm.as_deref().unwrap_or("undecodable"),
            self.word0.thumb.as_deref().unwrap_or("undecodable"),
        ));
        let note = if self.word0.refuses_arm {
            if self.word0.thumb.is_some() {
                "the condition field is 0xf: no ARMv4+ instruction has it, so only Thumb reads it"
            } else {
                "neither reads; a person must decide (`--thumb-entry` is that switch)"
            }
        } else if self.word0.arm.is_some() {
            "when both read, BREW convention takes ARM: `AEEMod_Load` is in ARM state. This is a default, not proof"
        } else {
            "only Thumb reads it"
        };
        out.push(format!(
            "    the host will enter as {}.{note}",
            self.word0.suggests
        ));
        out.push(match &self.brew {
            Some(b) => format!(
                "  BREW relocation header: IMAGE={:#x} REFOFF={:#x} FIXUPOFF={:#x} REFSIZE={:#x}{}, first fixup word {}",
                b.image,
                b.ref_off,
                b.fixup_off,
                b.ref_size,
                if b.describes {
                    String::new()
                } else {
                    "(**the header's range runs past the file**, so the loader says it does not describe this file)".to_string()
                },
                match b.fixup_word {
                    Some(FIXUP_PUSH) => format!("{FIXUP_PUSH:#010x} (the signature `src/loader.rs` records)"),
                    Some(w) => format!("{w:#010x} (not that signature)"),
                    None => "unreadable: the header's offset runs past the file".to_string(),
                }
            ),
            None => "  BREW relocation header: none (flat image, ER_RO copied verbatim)".to_string(),
        });
        out.push(match &self.app_info {
            Some(a) => format!(
                "  AEEAppInfo: cls={:#010x} wIDBase={} wAppType={:#x} wFlags={:#x} (manifest {})",
                a.cls,
                a.id_base,
                a.app_type,
                a.flags,
                self.manifest.clone().unwrap_or_else(|| "?".into())
            ),
            None => "  AEEAppInfo: none (no manifest mounted, or no id 2 in it)".to_string(),
        });
        if self.containers.is_empty() {
            out.push("  resource containers: none mounted".to_string());
        }
        for (name, c) in &self.containers {
            out.push(format!(
                "  container {name}: {} record(s), {} decodable picture(s), {} picture-class record(s) that do not decode",
                c.records,
                c.pictures.len(),
                c.undecodable
            ));
            out.push(format!("    kind：{}", tally(&c.kinds)));
            out.push(format!("    MIME：{}", tally(&c.mimes)));
        }
        for (name, id, count, first) in self.strings.iter().take(6) {
            out.push(format!(
                "  text {name}#{id}: {count} string(s), first {first:?}"
            ));
        }
        if self.strings.len() > 6 {
            out.push(format!(
                "  text: {} more not listed",
                self.strings.len() - 6
            ));
        }
        for a in &self.audio {
            // Both counts on one line when both are non-zero. This line is the only place a reader
            // learns that the pack's own declaration and this host's parser disagree about a clip,
            // and saying "N string(s)" without that would let a pack whose MP3s are really silence-ware read
            // as a pack with music.
            let playable = (a.playable > 0)
                .then(|| format!("{} playable ({} bytes)", a.playable, a.playable_bytes));
            let declared = (a.declared > 0).then(|| {
                format!(
                    "{} by declaration ({} bytes, this host cannot decode)",
                    a.declared, a.declared_bytes
                )
            });
            // Named beside the declared count rather than merged into it, because the two are answers
            // to different questions: "the container called these a sound" and "these bytes are that
            // format". A row where they differ in size is the interesting one — it says how much of the
            // declaration is a label on something else.
            let recognised = (!a.recognised.is_empty()).then(|| {
                let items = a
                    .recognised
                    .iter()
                    .map(|(k, v)| format!("{k}×{v}"))
                    .collect::<Vec<_>>()
                    .join("、");
                format!("{items} recognized as that format by bytes (no decoder yet)")
            });
            let parts: Vec<String> = playable
                .into_iter()
                .chain(declared)
                .chain(recognised)
                .collect();
            out.push(format!("  audio {}: {}", a.mime, parts.join(", ")));
        }
        for (name, report) in &self.par_census {
            let c = &report.census;
            let tail = if c.emitted == 0 {
                "; the host will not mount it (a pack that inflates it itself is unaffected)"
            } else if c.unnamed > 0 {
                "; leaves without an id cannot be looked up by id, so the host mounts only those that have one"
            } else {
                "; child files that cannot be read are skipped"
            };
            let mut found = Vec::new();
            if report.unnamed_audio > 0 {
                found.push(format!("{} playable MIDI by bytes", report.unnamed_audio));
            }
            if report.unnamed_pictures > 0 {
                let largest = report
                    .largest_picture
                    .map(|(w, h)| format!(", largest {w}x{h}"))
                    .unwrap_or_default();
                found.push(format!(
                    "{} decodable picture(s){largest}",
                    report.unnamed_pictures
                ));
            }
            let found = if found.is_empty() {
                String::new()
            } else {
                format!("; among them {}", found.join(", "))
            };
            out.push(format!(
                "  PAR {name}: {} id(s) with records, {} without an id, {} unreadable{tail}{found}",
                c.emitted, c.unnamed, c.unreadable
            ));
        }
        for (name, size) in &self.unmounted {
            out.push(format!(
                "  beside it, not mounted: {name} ({size} bytes, {})",
                if *size > store::CONTAINER_LIMIT {
                    "past the mount limit, so the host never read it"
                } else {
                    "neither container type"
                }
            ));
        }
        let top: Vec<String> = self
            .pictures
            .iter()
            .take(SHOW_ART)
            .map(|p| format!("{}x{}（{}#{})", p.width, p.height, p.from, p.id))
            .collect();
        out.push(format!(
            "  {} picture(s) in mounted containers; the largest {}: {}",
            self.pictures.len(),
            SHOW_ART,
            if top.is_empty() {
                "none".to_string()
            } else {
                top.join("  ")
            }
        ));
        let anonymous: usize = self
            .par_census
            .iter()
            .map(|(_, report)| report.unnamed_pictures)
            .sum();
        let anonymous_wide = self.anonymous_wide();
        if anonymous > 0 {
            out.push(format!(
                "  anonymous .par picture(s): {anonymous}, of which {anonymous_wide} are wide enough to vote (see the PAR lines)"
            ));
        }
        out.push(match &self.screen {
            Some(s) => format!(
                "  screen inference: **canvas candidate {}x{}** — {}; largest picture {:?}; {} wide picture(s) voted (of which {anonymous_wide} anonymous .par)",
                s.width,
                s.height,
                s.evidence(),
                s.largest,
                s.considered
            ),
            None => {
                let note = if anonymous > 0 {
                    format!(
                        "; {anonymous} anonymous .par picture(s), of which {anonymous_wide} are wide enough to vote"
                    )
                } else {
                    String::new()
                };
                format!(
                    "  screen inference: **cannot be asked** — no picture wider than {} that this host can decode. No default is substituted; that would be exactly the number the host was supposed to guess.{note}",
                    MIN_ART
                )
            }
        });
        if let Some(warn) = self.caveat() {
            out.push(format!("  {CAUTION_MARK} {warn}"));
        }
        out.push(format!(
            "  compare: directory name {:?}, host default {}x{}; {}",
            self.hint,
            self.default.0,
            self.default.1,
            self.verdict()
        ));
        out
    }

    /// Print the report. `label` names the pack in a directory run; a single pack needs none.
    pub fn print(&self, label: Option<&str>) {
        for line in self.lines(label) {
            println!("{line}");
        }
    }

    /// Do the art and the directory name say the same screen? `None` when only one of the two witnesses
    /// speaks, which is a different answer from "they disagree" and must not be counted as one.
    ///
    /// "Same" is width-equal and the canvas **fitting** the claimed height, not height-equal: a pack
    /// that draws 223 rows on a 240-row handset is not disagreeing with it, the missing 17 rows are the
    /// status bar, and calling that a conflict would bury the real conflicts.
    pub fn agrees(&self) -> Option<bool> {
        let s = self.screen.as_ref()?;
        let (w, h) = self.hint?;
        Some(s.width == usize::from(w) && s.height <= usize::from(h))
    }

    /// How many anonymous `.par` pictures are wide enough to enter the screen inference.
    fn anonymous_wide(&self) -> usize {
        self.par_census
            .iter()
            .flat_map(|(_, report)| report.picture_sizes.iter())
            .filter(|(w, _)| *w >= MIN_ART)
            .count()
    }

    /// Is the art this host can see too thin for the width answer to be trusted? `Some` is the warning
    /// in the report, `None` is "the evidence is thick enough".
    ///
    /// This exists because the census found the asymmetry: **20 of the corpus's 35 builds** are flagged
    /// here, 16 of them holding nothing but their three manifest icons (widest picture 65x42), one
    /// holding only four small pictures up to 50x50 while its real 320x223 background sits in `i0..i42`
    /// slices the resource readers never see, and three with mounted pictures but no picture wide enough
    /// for a width answer (the warning used to be skipped when there was no screen). The inference
    /// answers with an icon's width and looks exactly as confident as it does for the packs whose art
    /// really is mounted. A number that cannot tell those two apart is the worst kind, so the report
    /// says which one it is looking at. The counts are measured from the records
    /// this host can actually mount.
    pub fn caveat(&self) -> Option<String> {
        // The warning is about what a *mount* can serve, so it must not be silenced by the anonymous
        // `.par` pictures the screen inference also sees: those cannot answer a `LoadResData` call.
        // A pack with no mounted picture at all still gets a warning when anonymous art answered.
        let Some(picture) = self.pictures.iter().max_by_key(|p| p.width * p.height) else {
            let screen = self.screen.as_ref()?;
            return Some(format!(
                "evidence too thin: the host mounted no pictures; the canvas candidate {}x{} above comes from anonymous .par leaves and cannot be fetched by `LoadResData`.",
                screen.width, screen.height
            ));
        };
        let (w, h) = (picture.width, picture.height);
        if w * h >= THIN_ART {
            return None;
        }
        let anonymous = self.anonymous_wide();
        Some(if anonymous > 0 {
            // Do not claim the width is an icon when anonymous art is part of the evidence.
            format!(
                "evidence too thin: the largest picture in a mountable container is only {w}x{h} (under {THIN_ART} pixels). {anonymous} anonymous .par picture(s) also voted; they cannot be fetched by id, so the canvas candidate above is not mountable art."
            )
        } else {
            format!(
                "evidence too thin: the largest picture in a mountable container is only {w}x{h} (under {THIN_ART} pixels). This pack's art **is not here** — it is in its own `.mod`, in an archive the host did not mount, or behind its own decoder. The width above describes an icon, not a screen."
            )
        })
    }

    /// How the art's answer and the directory's hint compare. Three cases, because "there is no second
    /// witness" is not the claim that "the two witnesses disagree".
    pub fn verdict(&self) -> String {
        match (&self.screen, self.hint) {
            (Some(s), Some((w, h))) => {
                let (w, h) = (usize::from(w), usize::from(h));
                if s.width == w && s.height <= h {
                    format!(
                        "agree: the art canvas {}x{} fits the directory's {w}x{h} (excess of {} row(s): status bar or margin)",
                        s.width,
                        s.height,
                        h - s.height
                    )
                } else if s.width != w {
                    format!(
                        "**width disagrees**: the directory says {w}, the art says {} ({})",
                        s.width,
                        s.width_source()
                    )
                } else {
                    format!(
                        "**does not fit**: both widths are {w}, but the art needs {} rows and the directory says {h}",
                        s.height
                    )
                }
            }
            (Some(_), None) => "the art answered; the directory named no size".to_string(),
            (None, Some((w, h))) => {
                format!("the art did not answer; only the directory says {w}x{h}")
            }
            (None, None) => "neither witness".to_string(),
        }
    }
}

/// A picture, if this record is one the host can decode.
fn picture(from: &str, r: &Record) -> Option<Picture> {
    if r.kind != resource::RESTYPE_IMAGE {
        return None;
    }
    // The declared type travels with the record, and a `.par` leaf has none, so `image`'s own fallback
    // is what decides — the same answer a viewer request gets.
    let props = r.mime.clone().map(|m| m.into_bytes()).unwrap_or_default();
    let img = resource::image(&props, &r.body).ok()?;
    if img.width == 0 || img.height == 0 {
        return None;
    }
    Some(Picture {
        from: from.to_owned(),
        id: r.id,
        width: img.width,
        height: img.height,
    })
}

/// Word 0 both ways, through the same decoder `--disasm` and a fault listing use.
fn entry(bytes: &[u8]) -> Entry {
    let first = |thumb: bool| {
        disasm::disassemble(bytes, 0, thumb, 1)
            .into_iter()
            .next()
            .and_then(|l| l.error.is_none().then_some(l.text))
    };
    let (arm, thumb) = (first(false), first(true));
    let refuses_arm = bytes.len() >= 4 && !loader::plausibly_arm(word(bytes, 0));
    let suggests = if bytes.len() < 4 {
        // Three bytes are not a module — the loader calls it empty — even though a stray halfword in
        // them decodes as something. Saying so beats letting a fragment pick an instruction set.
        "unknown"
    } else if refuses_arm {
        if thumb.is_some() {
            "Thumb"
        } else {
            "unknown"
        }
    } else if arm.is_some() {
        "ARM"
    } else if thumb.is_some() {
        "Thumb"
    } else {
        "unknown"
    };
    Entry {
        word: word(bytes, 0),
        arm,
        thumb,
        refuses_arm,
        suggests,
    }
}

/// The `BREW` header's words, and the fixup routine's first instruction — all four offsets coming from
/// [`crate::loader`], which is where the machine-code evidence for them is written down.
fn brew_header(bytes: &[u8]) -> Option<Brew> {
    let r = loader::relocation(bytes)?;
    let at = (r.image as usize)
        .checked_add(r.ref_off as usize)?
        .checked_add(r.fixup_off as usize)?
        .checked_add(r.ref_size as usize)?;
    Some(Brew {
        image: r.image,
        ref_off: r.ref_off,
        fixup_off: r.fixup_off,
        ref_size: r.ref_size,
        describes: loader::relocation_describes(&r, bytes.len()),
        fixup_word: (at + 4 <= bytes.len()).then(|| word(bytes, at)),
    })
}

/// The app's record, read through the published ABI table rather than re-derived here.
fn app_info_of(raw: &[u8]) -> Option<AppInfo> {
    let at = |member: &str| usize::try_from(abi::offset_of("AEEAppInfo", member)?).ok();
    let half = |member: &str| {
        let off = at(member)?;
        Some(u16::from_le_bytes(raw.get(off..off + 2)?.try_into().ok()?))
    };
    Some(AppInfo {
        cls: word(raw, at("cls")?),
        id_base: half("wIDBase")?,
        app_type: half("wAppType")?,
        flags: half("wFlags")?,
    })
}

fn word(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

/// Below this, the art the host can see is too thin for a width answer to be trusted.
///
/// Calibrated on this corpus, not read out of a spec: the 64x64 area is above every pack whose mounted
/// art is only its icons (the widest thing three of them hold is 65x42 = 2 730 pixels, and one pack's
/// mounted pictures top out at 50x50) and below every pack whose width answer matched its own build
/// directory. A pack whose real art lives in a blob the host never mounts, or in a format it has no
/// reader for, still answers a width — it is just not that pack's width, and the report says so.
const THIN_ART: usize = 4096;

/// The screen the pack's art argues for. [`Screen`]'s doc is the rule; this is the arithmetic.
///
/// `None` when the pack holds no picture wide enough to say anything. That is answered rather than
/// filled in with the default, because the default is the number the host was going to guess anyway.
pub fn infer_screen(pictures: &[Picture]) -> Option<Screen> {
    let wide: Vec<&Picture> = pictures.iter().filter(|p| p.width >= MIN_ART).collect();
    if wide.is_empty() {
        return None;
    }
    // A bar is a picture at least three times as wide as it is tall — a status strip, a soft-key row, a
    // divider. Three is chosen, not measured: it is the shape a sprite cannot have, and it is where a
    // span across the canvas starts to look like one.
    //
    // **The widest bar wins, and that is a measured choice with a known cost.** Ranking the bars instead
    // by the tallest picture drawn under them scores worse on this corpus — 5 agreements against 7 — so
    // it is not the rule. The cost is a marquee: a pack that mounts a 693x18 ticker, or a 320x36 one
    // over a 176-wide canvas, gets the ticker's width for its answer, and both are on
    // the disagreement list the report prints. `the_widest_bar_wins_and_a_marquee_steals_the_answer`
    // pins that behaviour rather than hiding it.
    let bars: Vec<usize> = {
        let mut v: Vec<usize> = wide
            .iter()
            .filter(|p| p.width >= 3 * p.height)
            .map(|p| p.width)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let bar_width = bars.last().copied();
    let width = match bar_width {
        Some(b) => b,
        None => {
            // Nothing spans the canvas, so the next best witness is the height the pictures that are
            // big in **both** directions agree on, and the narrowest width carried at that height: a
            // pack that scrolls keeps one strip wider than its screen, and reading the widest of those
            // is how a 531-pixel canvas turns into a 531-pixel handset.
            let big: Vec<&&Picture> = wide.iter().filter(|p| p.height >= MIN_ART).collect();
            let mut by_height: BTreeMap<usize, (usize, usize, Vec<usize>)> = BTreeMap::new();
            for p in big {
                let e = by_height.entry(p.height).or_default();
                e.0 += 1;
                e.1 += p.width * p.height;
                e.2.push(p.width);
            }
            let (_, (_, _, mut widths)) = by_height
                .into_iter()
                .max_by_key(|(_, (count, area, _))| (*count, *area))?;
            widths.sort_unstable();
            widths.dedup();
            *widths.first()?
        }
    };
    let at: Vec<usize> = wide
        .iter()
        .filter(|p| p.width == width)
        .map(|p| p.height)
        .collect();
    let largest = wide
        .iter()
        .map(|p| (p.width * p.height, p.width, p.height))
        .max()
        .map(|(_, w, h)| (w, h));
    Some(Screen {
        width,
        height: *at.iter().max()?,
        bar: bar_width,
        at_that_width: at.len(),
        largest,
        considered: wide.len(),
    })
}

/// Whether `path` names the module being reported.
///
/// `read_dir(dir_of(mod))` returns paths rooted at that directory, while `mod_path` may be the bare
/// name the caller typed (or an absolute path). Comparing the two full paths is therefore false for
/// a bare name; a directory listing holds direct children, so the file name is the identity needed.
fn is_module_file(path: &Path, mod_path: &Path) -> bool {
    path.file_name() == mod_path.file_name()
}

/// A PAR container that was recognised but not mounted whole, with the counts that say why.
///
/// A leaf with no id above it cannot be looked up by `IShell::LoadResData`, so the host does not
/// mount it. That is correct, but it must not read as "this file is not a container": the report
/// names the file, the leaves it could file and the leaves it could not. The mount itself is left
/// alone — inventing an id would be a name no pack ever asked for.
fn par_census(mod_path: &Path) -> Vec<(String, ParReport)> {
    let mut out = Vec::new();
    let dir = store::dir_of(mod_path);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_file() || is_module_file(&path, mod_path) {
            continue;
        }
        // A file over the mount cap is already named by `unmounted`, with the reason that fits it.
        if path
            .metadata()
            .map(|m| m.len() > store::CONTAINER_LIMIT)
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if !par::is_par(&bytes) {
            continue;
        }
        let Ok(census) = par::census(&bytes) else {
            continue;
        };
        if census.emitted > 0 && census.unnamed == 0 && census.unreadable == 0 {
            continue;
        }
        // Only unnamed leaves need asking: the named ones are already in the per-record census.
        // The bytes decide, because a `.par` leaf carries no MIME to declare a type with. This
        // walk also inflates the anonymous leaves, so its census is the one that can say whether
        // a corrupt anonymous stream was unreadable rather than merely unnamed.
        let (census, unnamed_audio, picture_sizes, largest_picture) = if census.unnamed == 0 {
            (census, 0, Vec::new(), None)
        } else {
            match par::leaves_and_census(&bytes) {
                Ok((leaves, counted)) => {
                    let mut audio = 0;
                    let mut picture_sizes = Vec::new();
                    let mut largest = None;
                    for leaf in leaves.iter().filter(|leaf| leaf.id.is_none()) {
                        if audio::sniff(&leaf.payload).is_some() {
                            audio += 1;
                        }
                        if let Ok(img) = resource::image(&[], &leaf.payload) {
                            if img.width > 0 && img.height > 0 {
                                picture_sizes.push((img.width, img.height));
                                if largest.is_none_or(|(w, h): (usize, usize)| {
                                    img.width * img.height > w * h
                                }) {
                                    largest = Some((img.width, img.height));
                                }
                            }
                        }
                    }
                    (counted, audio, picture_sizes, largest)
                }
                Err(_) => (census, 0, Vec::new(), None),
            }
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push((
            name,
            ParReport {
                census,
                unnamed_audio,
                unnamed_pictures: picture_sizes.len(),
                largest_picture,
                picture_sizes,
            },
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The sibling files a run cannot reach.
///
/// A file over [`store::CONTAINER_LIMIT`] is never even read by the mount, and a mount that refuses one
/// leaves the pack reporting a missing resource without saying "too big to look at". Naming what was
/// passed over is what turns that into a sentence a person can act on.
fn unmounted(mod_path: &Path, mounted: &[(String, Vec<Record>)]) -> Vec<(String, u64)> {
    let have: Vec<&str> = mounted.iter().map(|(n, _)| n.as_str()).collect();
    let mut out = Vec::new();
    let dir = store::dir_of(mod_path);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() || is_module_file(&path, mod_path) {
            // The module itself is not a container and not a thing a run reaches sideways; listing it
            // would be listing the subject of the report as one of its findings.
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        if have.contains(&name.as_str()) {
            continue;
        }
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        if size > store::CONTAINER_LIMIT {
            out.push((name, size));
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        // Neither container reader takes it. Normal, and worth printing anyway: this is where a pack's
        // save files, its indexes and its own compressed code show up.
        if resource::Container::parse(&bytes).is_err() && !par::is_par(&bytes) {
            out.push((name, size));
        }
    }
    out
}

/// A printable form of a string resource's characters.
///
/// A counted resource holds one byte per `AECHAR` and a UTF-16 manifest holds real ones, and
/// [`resource::strings`] has already told those apart. Either way this prints what is sayable as ASCII
/// and leaves the rest as escapes: a pack's version line is information, and a box glyph or a
/// mis-decoded GB2312 pair is not.
fn text_of(chars: &[u16]) -> String {
    chars
        .iter()
        .take(64)
        .map(|&c| match c {
            0x20..=0x7e => char::from(c as u8).to_string(),
            other => format!("\\u{other:04x}"),
        })
        .collect()
}

fn kind_name(kind: u16) -> String {
    match kind {
        resource::RESTYPE_STRING => "STRING".to_string(),
        resource::RESTYPE_IMAGE => "IMAGE".to_string(),
        resource::RESTYPE_BINARY => "BINARY".to_string(),
        other => format!("kind{other:#06x}"),
    }
}

fn tally(map: &BTreeMap<String, usize>) -> String {
    let parts: Vec<String> = map.iter().map(|(k, n)| format!("{k}×{n}")).collect();
    if parts.is_empty() {
        "(empty)".to_string()
    } else {
        parts.join("  ")
    }
}

/// The load-time refusal this image *would* get is `loader::precheck`'s answer, not a second copy of it:
/// the three byte-only cases are the loader's to decide, and `--info` only prints what it said.

/// Every `.mod` under `root`, in the order the library lists them.
pub fn packs(root: &Path) -> Vec<std::path::PathBuf> {
    library::scan(root)
        .into_iter()
        .map(|e| e.mod_path)
        .collect()
}

#[cfg(test)]
#[path = "test/probe.rs"]
mod tests;
