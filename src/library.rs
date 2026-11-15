//! The library: the packs found on this machine, and the one entry point a front end needs to start one.
//!
//! A person who double-clicks an emulator expects a list of games, not a usage message, so this is what
//! an argument-less run gets. Everything here is deliberately front-end-free: it finds packs, reads the
//! icon a pack carries for itself, and brings one up far enough to be looked at. The window is somebody
//! else's problem, which is also why this module can be tested without one.

use std::path::{Path, PathBuf};

use crate::device::DeviceProfile;
use crate::resource;
use crate::runtime::Runtime;

/// One pack the library found.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The pack's own name: the directory the `.mod` sits in, which is what a person recognises.
    pub title: String,
    /// The build above it — `motQA30-320x240`, `283491`. The same game for two handsets differs only
    /// here, so it belongs in the name rather than being thrown away.
    pub build: String,
    pub mod_path: PathBuf,
    /// The screen the pack was built for, when the directory it is filed under says so.
    pub screen: Option<(u16, u16)>,
    /// The picture the pack carries for itself, out of its own `.mif`, or `None` when there is none this
    /// host can read. A pack with no icon still belongs in the list — it is the picture that is optional,
    /// not the game.
    pub icon: Option<Icon>,
}

/// A pack's own icon, as 24-bit RGB.
#[derive(Debug, Clone)]
pub struct Icon {
    pub width: usize,
    pub height: usize,
    /// `width * height` triples, top row first.
    pub rgb: Vec<u8>,
}

/// Which witness claims a screen. Naming one by hand is not a witness, so it is not in this list: the
/// player has that option too, and it is neither of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Witness {
    /// The `WxH` typed into the directory the pack is filed under.
    Folder,
    /// The canvas the pack's own art argues for (`crate::probe::Screen`).
    Art,
}

impl Witness {
    /// What the picker calls this witness — the two words a person reads before choosing it.
    pub fn name(&self) -> &'static str {
        match self {
            Witness::Folder => "directory",
            Witness::Art => "art",
        }
    }
}

/// One claimed screen, as data rather than as a line of text: the library has to *offer* these and then
/// boot the pack with whichever one a player picks, and a formatted string cannot do that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub width: u16,
    pub height: u16,
    pub witness: Witness,
    /// What backs this claim, in the witness's own words. The picker shows it beside the number because
    /// `sizes` is **not** a vetted list: a marquee bar can sit in it looking exactly like a background
    /// and the one thing that tells the two apart is this sentence.
    pub evidence: String,
}

impl Candidate {
    /// The same claim as a choice, minus the sentence — so a pick cannot go stale when the row it came
    /// from is rebuilt, and so `decide` never has to look this index up and fail to find it.
    pub fn pick(&self) -> Pick {
        Pick::Claim {
            width: self.width,
            height: self.height,
            witness: self.witness,
        }
    }
}

/// What the size picker holds for one row. There is no default: `None` is the state a row is in until a
/// person picks, and [`decide`] says out loud what will be used then, because "nothing chosen" that
/// quietly boots the folder's claim is the silent fallback this repository keeps finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// One of the row's own candidates, carried by value.
    Claim {
        width: u16,
        height: u16,
        witness: Witness,
    },
    /// Typed by hand, and labelled as such wherever it is shown.
    Manual { width: u16, height: u16 },
}

/// The screen a launch will use, and the sentence that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// `None` is "the host's default screen", which is what [`boot_at`] does with it — not a size the
    /// pack claimed.
    pub screen: Option<(u16, u16)>,
    pub words: String,
}

/// A hand-typed size, on the same terms the sweep harness already uses: `tools/games.ps1:156` takes a
/// candidate only within `40..=1024` on each axis, and the picker uses that bound rather than inventing
/// a third one. The upper end is not tidiness — a canvas is allocated in the host, so `65535x65535` is
/// an eight-gigabyte frame, and this machine's failure mode is that it stops rather than that it errors.
///
/// The directory-name scan ([`screen_of`]) and the CLI's `--device` (`src/main.rs`, which takes any
/// non-zero `u16` pair) still have no upper bound, so the hazard is not closed by this function — it is
/// closed nowhere until one rule sits where the frame is allocated. That gap is recorded in
/// The measurement that makes it cheap: every size the corpus claims in a directory name is inside
/// these bounds (128x128, 176x220, 220x176, 240x320, 320x240, 480x240 — 0 of 6 outside), so tightening
/// it would not change any pack this library has ever booted.
pub fn manual_size(wide: &str, high: &str) -> Option<(u16, u16)> {
    let (w, h) = (
        wide.trim().parse::<u16>().ok()?,
        high.trim().parse::<u16>().ok()?,
    );
    (w >= MIN_HANDSET && w <= MAX_HANDSET && h >= MIN_HANDSET && h <= MAX_HANDSET).then_some((w, h))
}

/// The bounds a hand-typed screen has to fall inside; see [`manual_size`] for where they come from.
const MIN_HANDSET: u16 = 40;
const MAX_HANDSET: u16 = 1024;

/// What will happen if this row is started right now.
///
/// Every branch names its own source, including the two nobody picked: the button is the only place a
/// person sees the number before something boots, and a label that just said "start the game" would let the
/// folder's guess be blamed on the emulator when the picture came out wrong.
pub fn decide(row: &Row, pick: Option<Pick>) -> Decision {
    let (screen, words) = match pick {
        Some(Pick::Claim {
            width,
            height,
            witness,
        }) => (
            Some((width, height)),
            format!(
                "{width}x{height} from {what}'s evidence",
                what = witness.name()
            ),
        ),
        Some(Pick::Manual { width, height }) => (
            Some((width, height)),
            format!("{width}x{height} typed by hand"),
        ),
        None => match row.entry.screen {
            Some((w, h)) => (
                Some((w, h)),
                format!("{w}x{h} from the directory (not chosen)"),
            ),
            None => {
                let d = DeviceProfile::standard();
                (
                    None,
                    format!(
                        "host default {}x{} (not chosen, no witness)",
                        d.width, d.height
                    ),
                )
            }
        },
    };
    Decision { screen, words }
}

/// One row of the list: the pack, the words under its name, and the report its detail pane shows.
///
/// The model lives here rather than in [`crate::egui_window`] for the reason `handset.rs` gives for the
/// same split — wording and arithmetic that never touch egui are the parts a test can hold. The detail
/// lines are `crate::probe::Probe::lines` verbatim: one authority for those facts, so the CLI report and
/// the window cannot grow two wordings that drift apart.
#[derive(Debug)]
pub struct Row {
    pub entry: Entry,
    /// Which build this is and which screen somebody claimed for it.
    pub subtitle: String,
    pub detail: Vec<String>,
    /// What each witness claims, folder first. Empty is an honest answer, not a missing one. A claim the
    /// probe calls thin is withheld and explained by [`Row::caution`]. Each claim that *is* left carries
    /// its own evidence sentence, and the picker has to show it: a marquee bar has an area over
    /// `THIN_ART` and so reads as a plain candidate — the number alone cannot tell that
    /// apart from a background, the evidence line can.
    pub sizes: Vec<Candidate>,
    /// The art this host can reach is too thin to answer a screen with. The probe decides it, the row
    /// carries it, and the pane is where a player sees it — without that line a row of icons reads as
    /// though it were evidence about a screen.
    pub caution: Option<String>,
}

impl Row {
    /// Read one pack for the list. The probe re-opens what `scan` already walked — the manifest twice,
    /// every picture twice — so a row costs more than its share of reading. All 35 rows take **1.52 s**
    /// in a debug build and carry 1 280 detail lines between them; the same work through the release
    /// binary (`quoin Game --info`, which prints what these rows carry) is **0.59 s**.
    /// Affordable at window open, not free: a row is built once and kept, not rebuilt per frame — the
    /// `icons.len() != entries.len()` guard in [`crate::egui_window`] is the pattern to copy.
    pub fn new(entry: Entry) -> Row {
        let probe = crate::probe::Probe::read(&entry.mod_path);
        let subtitle = subtitle(&entry);
        let caution = probe.caveat();
        let detail = probe.lines(None);
        let mut sizes = Vec::new();
        if let Some((width, height)) = entry.screen {
            sizes.push(Candidate {
                evidence: format!("from the directory: {}", entry.build),
                width,
                height,
                witness: Witness::Folder,
            });
        }
        // An art canvas too wide for the size a boot takes is not a screen anyone could ask for, so it is
        // offered as a `detail` line rather than as a candidate — the number itself stays readable there.
        // And when the probe says its art is too thin to judge, the number it produces is an icon's
        // width: 16 of this corpus's 35 builds mount nothing but their three icons (widest 65x42) and
        // some mount only a few small pictures (widest 50x50). Those claims stay in the report, where the
        // warning travels with them, and off the list, where a number alone would read as evidence.
        if let Some(s) = probe.screen.as_ref().filter(|_| caution.is_none()) {
            if let (Ok(width), Ok(height)) = (u16::try_from(s.width), u16::try_from(s.height)) {
                sizes.push(Candidate {
                    // `Screen::evidence` is the report's own sentence for this claim, so the picker and
                    // the report cannot say two different things about one number.
                    evidence: s.evidence(),
                    width,
                    height,
                    witness: Witness::Art,
                });
            }
        }
        Row {
            entry,
            subtitle,
            detail,
            sizes,
            caution,
        }
    }
}

/// The words under a pack's title, and the same string [`Row::subtitle`] carries: the list draws it
/// before a row exists, and two wordings of one fact is the drift this repository keeps being bitten by.
///
/// `default screen` is not a size the pack has. It is the host saying nobody wrote one down, which is a fact
/// about the folder and is printed as one.
pub fn subtitle(entry: &Entry) -> String {
    match entry.screen {
        Some((w, h)) => format!("{}  {w}x{h}", entry.build),
        None => format!("{}  default screen", entry.build),
    }
}

/// Every pack under `root`, each with its static read attached, in the order [`scan`] lists them.
pub fn rows(root: &Path) -> Vec<Row> {
    scan(root).into_iter().map(Row::new).collect()
}

/// Every `.mod` under `root`, in a stable order.
///
/// A directory that cannot be read is skipped rather than fatal: a library pointed at a tree with one
/// unreadable subdirectory should show the packs it could find, not refuse to show any.
pub fn scan(root: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort_by(|a, b| {
        a.title
            .to_ascii_lowercase()
            .cmp(&b.title.to_ascii_lowercase())
            .then_with(|| a.build.cmp(&b.build))
    });
    out
}

fn collect(dir: &Path, out: &mut Vec<Entry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    // Sorted here so a library listing is the same on every machine and every run: directory order is
    // the file system's business, and a front end that showed it would look like it had shuffled itself.
    let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    children.sort();
    for path in children {
        if path.is_dir() {
            collect(&path, out);
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("mod") {
            continue;
        }
        let title = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let (build, screen) = build_of(&path);
        out.push(Entry {
            title,
            build,
            screen,
            icon: icon_of(&path),
            mod_path: path,
        });
    }
}

/// The build a pack is, and the screen it was built for — read from the same directory, because they are
/// the same fact.
///
/// The corpus files these two ways. One layout puts a model-and-screen directory above an application
/// id; another has only the id. Walking up looking for the screen tells the two apart without a rule
/// per layout, and it is the screen directory that names the build — a list that used the application
/// id would be naming the app store's row, not the build.
///
/// The walk is bounded at three levels: a `.mod` filed deep under a directory that happens to be called
/// `320x240` for some unrelated reason must not reach the root, and the corpus is nowhere near that deep.
fn build_of(mod_path: &Path) -> (String, Option<(u16, u16)>) {
    let start = crate::store::dir_of(mod_path);
    let mut at = Some(start.as_path());
    for _ in 0..3 {
        let Some(dir) = at else { break };
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(size) = screen_of(name) {
            return (name.to_string(), Some(size));
        }
        at = dir.parent();
    }
    // No screen anywhere above it: the pack is named by the directory it sits in, which is its id.
    let name = start.file_name().and_then(|n| n.to_str()).unwrap_or("");
    (name.to_string(), None)
}

/// The size a pack's own directory tree claims for it, for a caller that wants the hint and not the
/// name. Read from the same walk [`build_of`] does, so the list and a probe cannot disagree about what
/// the folder said — and it stays a **hint**: a directory name is something a person typed, and this
/// host has been misled by one before.
pub fn screen_of_dir(mod_path: &Path) -> Option<(u16, u16)> {
    build_of(mod_path).1
}

/// The `WxH` a build directory is filed under, when it is filed under one.
///
/// These packs are named `<model>-<WxH>` or just `<appid>`, so the size is in the directory name when it
/// is anywhere. Reading it here is what lets the list say "320x240" without loading the module, and it is
/// a hint rather than a fact: a pack that disagrees with its own directory is loaded with what the
/// directory said, exactly as `--device` would.
///
/// The scan is a digit run, a separator, a digit run, anywhere in the name — last one wins. It used to
/// anchor on the final `-` or `_` first, which works while every directory is `<model>-<WxH>` and fails
/// on one that is bracketed, space-separated and written with a full-width multiplication sign because
/// whoever typed the name was typing Chinese. Those get **no** hint and are silently loaded at the
/// default screen.
pub fn screen_of(name: &str) -> Option<(u16, u16)> {
    // `×` is not decoration for `x`. It is what a phone keyboard produces, and a directory named by
    // hand on one gets it; reading only the ASCII pair means that half the corpus is invisible here.
    const SEP: [char; 3] = ['x', 'X', '×'];
    let chars: Vec<char> = name.chars().collect();
    let digits = |at: usize| {
        let mut end = at;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        end
    };
    let mut found = None;
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let w1 = i;
        i = digits(w1);
        if i >= chars.len() || !SEP.contains(&chars[i]) {
            continue;
        }
        let sep = i;
        i += 1;
        let h0 = i;
        i = digits(h0);
        if i == h0 {
            continue;
        }
        let span =
            |r: std::ops::Range<usize>| chars[r].iter().collect::<String>().parse::<u16>().ok();
        // A run too long for a screen is a version number or an app id that happens to sit next to an
        // `x`, not a size, and it must not end the scan — the real one may be further along.
        let (Some(w), Some(h)) = (span(w1..sep), span(h0..i)) else {
            continue;
        };
        // A zero-sized screen is a directory that happens to have an `x` in it, not a handset.
        if w != 0 && h != 0 {
            found = Some((w, h));
        }
    }
    found
}

/// The picture a pack carries for itself: an image out of the `.mif` beside its `.mod`.
///
/// That container is also where a pack keeps its full-screen art, so the smallest image is taken: an icon
/// is the picture that is small, and the alternative — the first one — is whatever the container's author
/// happened to write first.
///
/// Which formats count is [`resource::image`]'s answer, the same one a viewer request goes through, and
/// not a test invented here. The corpus's pictures are not all BMPs: ten of its thirty-five builds carry
/// three `image/png` images and no BMP at all.
///
/// Inheriting that entry point also inherits its **last** arm: a resource with no property block is
/// decided by its own bytes rather than refused. The old prefix test refused those, so this is a
/// widening beyond "PNG works now", and it is pinned by a test rather than left implied.
///
/// Two things about the corpus are worth knowing before reading this. The container is named after the
/// *directory* the pack sits in — a `.mif` beside its own directory — not after the module, so the
/// module's own name is the wrong guess; and a picture this host has no reader for is not an icon this
/// function can invent, which is why a pack whose art is refused by [`resource::image`] shows a blank
/// space rather than something the host made up.
///
/// The icon is chosen from the container's own type, not from a name prefix: ten packs carry PNG
/// only, and the host has had a PNG decoder (`resource::png`) all along, so a prefix test would
/// leave those rows blank for a decoder that exists.
fn icon_of(mod_path: &Path) -> Option<Icon> {
    let bytes = std::fs::read(mif_beside(mod_path)?).ok()?;
    let container = resource::Container::parse(&bytes).ok()?;
    let mut best: Option<Icon> = None;
    for res in container.all().ok()? {
        if res.kind != resource::RESTYPE_IMAGE {
            continue;
        }
        let Ok(img) = resource::image(res.props, res.body) else {
            continue;
        };
        if img.width == 0 || img.height == 0 {
            continue;
        }
        if best
            .as_ref()
            .is_some_and(|b| b.width * b.height <= img.width * img.height)
        {
            continue;
        }
        let mut rgb = Vec::with_capacity(img.width * img.height * 3);
        for y in 0..img.height {
            for x in 0..img.width {
                let (r, g, b) = img.rgb(x, y);
                rgb.extend_from_slice(&[r, g, b]);
            }
        }
        best = Some(Icon {
            width: img.width,
            height: img.height,
            rgb,
        });
    }
    best
}

/// The `.mif` that belongs to a pack.
///
/// It is named after the *directory* the pack sits in — a `.mif` beside its own directory — which is
/// how this corpus files them, and the module's own name is tried after that because a pack assembled by
/// hand is as likely to have used it. Asking the file system which one exists is cheaper than deciding.
fn mif_beside(mod_path: &Path) -> Option<PathBuf> {
    let dir = crate::store::dir_of(mod_path);
    let mut candidates = Vec::new();
    if let (Some(id), Some(parent)) = (dir.file_name().and_then(|n| n.to_str()), dir.parent()) {
        candidates.push(parent.join(format!("{id}.mif")));
    }
    candidates.push(mod_path.with_extension("mif"));
    candidates.into_iter().find(|p| p.is_file())
}

/// Load the other `.mod` files that belong to the same installed bundle as `mod_path`.
///
/// A handset's class registry knows every installed module; this host's equivalent is the bundle the
/// pack came in. `Refresher.mod` is the case that made this necessary: its whole `EVT_APP_START` is
/// `CloseApplet` and then `StartApplet(0x0107f149)`, which is `monopoly.mod`'s applet class. With
/// only the launched module loaded the shell answered `AEE_EFAILED`, and the pack fell through to
/// its own cleanup, so it never showed anything.
///
/// The bundle root is the directory the app-info manifest sits in — `<appid>.mif` beside the
/// `<appid>/` that holds the `.mod`, the layout [`crate::store::Store::mount_beside`] already relies
/// on. Only when that manifest is actually there do the root's other subdirectories count: without
/// the gate, a pack filed directly in a collection directory would pull in its neighbours, and every
/// load runs that module's own entry point.
///
/// A module that will not load is reported and skipped rather than failing the run: the app itself is
/// fine, and what is missing is a class it meant to start.
pub fn load_sibling_modules(rt: &mut Runtime, mod_path: &Path) -> Vec<String> {
    let Some(dir) = mod_path.parent() else {
        return Vec::new();
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mods_in = |at: &Path, out: &mut Vec<PathBuf>| {
        let Ok(read) = std::fs::read_dir(at) else {
            return;
        };
        for path in read.flatten().map(|e| e.path()) {
            if path.extension().and_then(|x| x.to_str()) == Some("mod") && path != mod_path {
                out.push(path);
            }
        }
    };
    mods_in(dir, &mut candidates);
    if let (Some(root), Some(name)) = (dir.parent(), dir.file_name()) {
        let manifest = |extension: &str| {
            let mut named = std::ffi::OsString::from(name);
            named.push(".");
            named.push(extension);
            root.join(named)
        };
        if manifest("mif").is_file() || manifest("bar").is_file() {
            let mut subdirs: Vec<PathBuf> = std::fs::read_dir(root)
                .map(|read| read.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
                .unwrap_or_default();
            subdirs.sort();
            for sub in subdirs {
                mods_in(&sub, &mut candidates);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    let mut loaded = Vec::new();
    for path in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        // The module's own files and resources come with it. A bundle's apps each keep their data
        // beside their own image — `devices.dat`, `monopoly.bar`, `Monopoly.mif` all sit next to
        // `monopoly.mod` — and a launcher that starts one needs that namespace: without it Monopoly's
        // constructor opens `devices.dat`, fails, and answers its own `0x4009` instead of building.
        rt.volume.mount_beside(&path);
        rt.resources.mount_beside(&path);
        match rt.load_library(&bytes, false) {
            Ok(_) => loaded.push(path.display().to_string()),
            Err(e) => loaded.push(format!("{}: {e} (skipped)", path.display())),
        }
    }
    loaded
}

/// Bring a pack up far enough to be looked at: map it, run its module load, and start its applet.
///
/// The same three steps `quoin <file.mod> --start auto` takes, and for the same reason — an applet is
/// what draws, and a library that showed a module's load and stopped would show a black screen to somebody
/// who had just picked a game.
///
/// The budget is [`crate::runtime::GAME_STEP_LIMIT`] rather than the probe default, because a pack the
/// library starts is one that is about to be *played*: a single frame of a real game is one host→guest
/// call, and the default is sized for a probe that stops. See that constant for the measurement that
/// settled the number.
///
/// `screen` is the size the picker decided on — [`decide`]'s answer, not a size worked out here — and
/// `None` is the host's default profile rather than "no screen". The pack's own art and saved files are
/// mounted beside the image whichever size is used, because that part does not depend on the canvas.
pub fn boot_at(entry: &Entry, screen: Option<(u16, u16)>) -> Result<Runtime, String> {
    let image =
        std::fs::read(&entry.mod_path).map_err(|e| format!("{}: {e}", entry.mod_path.display()))?;
    let profile = match screen {
        Some((w, h)) => DeviceProfile::standard().with_screen(w, h),
        None => DeviceProfile::standard(),
    };
    let mut rt = Runtime::with_device(profile);
    // The manager has no command line to carry `--jit`: a game opened from the GUI gets
    // the same engine a CLI run gets by default when the binary was built with the backend.
    // A build without the feature keeps the interpreter unchanged. This has to happen before
    // `load` / `run`, so the module's own startup code runs through the backend too.
    #[cfg(feature = "jit")]
    rt.cpu.enable_jit();
    // The wide face comes from the machine's own font (the manager's choice first), rasterized
    // into a BBF once and cached under `data/fonts/`. A pack whose menus are Chinese carries no
    // fallback of its own, so without this every one of those code units is a notdef box.
    crate::systemfont::install(&mut rt, &[]);
    rt.step_limit = crate::runtime::GAME_STEP_LIMIT;
    rt.step_ceiling = crate::runtime::GAME_STEP_CEILING;
    // A pack's art and its own saved files live beside the image, which is what a null resource file
    // name means. The CLI does the same before it loads anything.
    rt.resources.mount_beside(&entry.mod_path);
    rt.volume.mount_beside(&entry.mod_path);
    // The manager is a second way to start a pack, so it has to give the volume the same home the
    // command line does; without this a game launched from the library writes its save into memory and
    // forgets it when the window closes.
    if let Some(dir) = default_save_dir(&entry.mod_path) {
        rt.volume.persist_to(&dir);
    }
    rt.load(&image, false).map_err(|e| e.to_string())?;
    // The rest of the bundle, so a launcher shim can start the class it names — Refresher.mod's only
    // act is starting monopoly.mod's applet. See `load_sibling_modules`.
    load_sibling_modules(&mut rt, &entry.mod_path);
    // The compiled-code ceiling, for the same reason and in the same shape as `QUOIN_SPEED`: the
    // manager has no command line, so the one reading a player can reach is the environment. Unset
    // keeps the backend's own default; `auto` sizes it from this pack's own image.
    #[cfg(feature = "jit")]
    if let Ok(text) = std::env::var("QUOIN_JIT_ARENA") {
        if !text.trim().is_empty() {
            let choice =
                crate::cpu::JitArena::parse(&text).map_err(|e| format!("QUOIN_JIT_ARENA: {e}"))?;
            if let Some(line) = rt.cpu.size_jit_arena(choice, image.len() as u64) {
                println!("{line}");
            }
        }
    }
    rt.run().map_err(|e| e.to_string())?;
    // The CLI treats a start that found a class but no applet as a failed run, and says why. The
    // manager has to make the same judgement: dropping `AutoStart` here left the window on a black
    // page with no line explaining it, which is indistinguishable from a pack that started and drew
    // nothing.
    let started = rt.start_auto().map_err(|e| e.to_string())?;
    if started.cls == 0 {
        let why = match started.note {
            Some(note) => format!(
                "the module built no applet for any of the {} class ids in its own image; {note}",
                started.tried
            ),
            None => format!(
                "the module claims none of the {} class ids in its own image",
                started.tried
            ),
        };
        return Err(why);
    }
    if let Some(why) = started.note {
        return Err(why);
    }
    if started.applet == 0 || rt.applet != started.applet {
        return Err(format!(
            "StartApplet({:#010x}) left no applet running",
            started.cls
        ));
    }
    Ok(rt)
}

pub use crate::platform::data::{migrate_legacy_data, program_data_dir};

/// Where a run's own writes go when the caller named no directory: `data/saves/<pack>-<hash>`
/// beside the emulator itself.
///
/// Per pack, because two games' files must not meet: a handset gives every app its own directory, and
/// one pack writing a file after its own name is not a name another pack can share. The hash is of the
/// pack's own path, which keeps two builds of one game apart — a game may ship several, and a save
/// written by one build is not something the others can read.
///
/// `None` when the emulator cannot say where it is, which leaves the run with no home directory: the
/// explanation for a save that does not survive is then "we could not find a place to put it", not a
/// run that quietly wrote into whatever directory happened to be current.
///
/// It lives here rather than in `main.rs`, where it was written: the manager shows a pack's saves, so
/// "which directory are they in" has two readers now, and a copy in each is exactly the drift this
/// repository keeps being bitten by.
pub fn default_save_dir(mod_path: &Path) -> Option<PathBuf> {
    let dir = program_data_dir()?.join("saves");
    // FNV-1a, spelled out rather than taken from `DefaultHasher`: the name of a save directory is
    // something a player will see on disk and go looking in, so it must not change under a toolchain
    // that decides to hash differently.
    let absolute = std::path::absolute(mod_path).ok()?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in absolute.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let stem = mod_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("app");
    // Anything Windows will not take in a file name becomes an underscore; the rest — a pack whose
    // name is Chinese, say — is left alone, because it is the name the player would look for.
    let stem: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    Some(dir.join(format!("{stem}-{:08x}", hash as u32)))
}

/// One file in a pack's save directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Save {
    /// The name the volume itself uses for it: the path below the save directory, `/`-separated.
    pub name: String,
    pub bytes: u64,
    /// When it was last written, when the file system will say.
    pub modified: Option<std::time::SystemTime>,
}

/// What a pack has saved, or an empty list — including when there is no such directory, which is the
/// honest reading of "this game has never been run".
///
/// The walk stops at [`crate::file::MOUNT_DEPTH`], the same depth a run mounts its own directory to:
/// a file deeper than that is one the game itself cannot reach, so offering it here would be offering
/// to delete something that was never a save.
pub fn saves_in(dir: &Path) -> Vec<Save> {
    let mut out = Vec::new();
    let mut todo = vec![(dir.to_path_buf(), String::new(), 0u8)];
    while let Some((at, prefix, depth)) = todo.pop() {
        let Ok(read) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let key = format!("{prefix}{name}");
            if path.is_dir() {
                if depth < crate::file::MOUNT_DEPTH {
                    todo.push((path, format!("{key}/"), depth + 1));
                }
                continue;
            }
            let Ok(meta) = path.metadata() else {
                continue;
            };
            out.push(Save {
                name: key,
                bytes: meta.len(),
                modified: meta.modified().ok(),
            });
        }
    }
    // Sorted here so the pane does not reshuffle itself between two looks at the same directory.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The file a save name points at, refusing anything that is not one plain name below `dir`.
///
/// The names come back off the disk and then get used to delete something, so this is the one place
/// where "delete a save" could become "delete somewhere else": `..`, a leading separator and a
/// backslash are all refused, and the result is checked to still be inside `dir`.
fn save_path(dir: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.contains("..") || name.starts_with('/') || name.contains('\\') {
        return Err(format!("not a save name: {name}"));
    }
    let path = dir.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
    if path.starts_with(dir) {
        Ok(path)
    } else {
        Err(format!("{name} is outside {}", dir.display()))
    }
}

/// Delete one saved file. The directory itself is left alone, so a game that is played again finds an
/// empty home rather than no home.
pub fn delete_save(dir: &Path, name: &str) -> Result<(), String> {
    let path = save_path(dir, name)?;
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Forget everything a pack saved, which is what "start this game over" means. The directory is
/// recreated empty, so the next run finds the same home it would have found the first time.
pub fn wipe_saves(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => {}
        // Nothing there is already the state this asks for.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

#[cfg(test)]
#[path = "test/library.rs"]
mod tests;
