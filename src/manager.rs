//! The game manager: every pack on this machine, and everything needed to pick one, read it and
//! start it.
//!
//! The split from [`crate::egui_window`] is deliberate. That file is the screen and keyboard of
//! **one game** — it carries three rules that are already verified (the simulation runs on the guest
//! thread in [`crate::session`], this side calls no `Runtime` method except reading `device` width and
//! height once before handing it to the worker to size the window; keys enter only through
//! `post_key_press`/`post_key_release`; pixels are always NEAREST and an integer scale), so it should
//! care about one pack. This file is how a **wall of games** is organised: the left column is the
//! library, and the right column is either the selected pack's page or the game that is running.
//!
//! The lower half of this file (from here down to `saves`) is pure data: search, filter, grouping,
//! icon sizes, save directories. No egui type appears there, so it can be asserted row by row
//! headless — with "never run a guest" as a rule, that is the only way to prove the interface has not
//! drifted, and it is the same idea as `egui_window`'s `painted_text`.
//!
//! **Compute once, not every frame**: filtering and grouping produce a [`Plan`], rebuilt by
//! [`Plan::of`] when "the rows or the query changed". Lower-casing 35 row titles sixty times a second
//! is wasted work, and 35 rows hold 1,280 details; recomputing them is not a move to ignore.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect as PointRect, Stroke, TextureOptions, Vec2,
};

use crate::egui_window::{self, Frontend};
use crate::handset::{self, Skin};
use crate::library::{self, Pick, Row};
use crate::manager_data::{self, Data};

/// The search text and filter on the list.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Query {
    /// Case-insensitive substring; it hits a name, a build or a disk path. An empty string means "no filter".
    pub(crate) text: String,
    /// Keep only packs that satisfy this entry.
    pub(crate) filter: Filter,
}

/// A filter. **It lists only what the host can actually see**.
///
/// There is deliberately no "playable" entry: the host does not know whether a pack can run (only a
/// small part of the corpus has been confirmed by a person), and a "playable" mark guessed from a
/// name or a warning is exactly the kind of silent success this repository fears most; what can be
/// filtered is these facts read from the bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    /// Everything, no filter.
    #[default]
    All,
    /// With a warning: the evidence the host read is too thin, or something else a player should know.
    Caution,
    /// Without a warning.
    Clean,
    /// With an icon (the small picture in the pack's own `.mif`, which the host can read).
    WithArt,
    /// Without an icon: the host cannot read its picture, so the list shows a blank.
    WithoutArt,
    /// With at least one size witness (the directory name, or its own art).
    Claimed,
    /// With no size witness.
    Unclaimed,
}

/// The language the manager's own chrome is drawn in. A pack's own text is never translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiLang {
    En,
    Zh,
}

impl UiLang {
    /// The language this machine is set to; English when neither the environment nor the system
    /// answers Chinese.
    pub(crate) fn from_system() -> UiLang {
        if let Some(tag) = crate::platform::ui_lang() {
            if let Some(lang) = UiLang::from_tag(tag) {
                return lang;
            }
        }
        if let Some(tag) = crate::platform::lang::system_tag() {
            return if tag.starts_with("zh") {
                UiLang::Zh
            } else {
                UiLang::En
            };
        }
        UiLang::En
    }

    fn next(self) -> UiLang {
        match self {
            UiLang::En => UiLang::Zh,
            UiLang::Zh => UiLang::En,
        }
    }

    fn label(self) -> &'static str {
        match self {
            UiLang::En => "English",
            UiLang::Zh => "中文",
        }
    }

    fn pick(self, en: &'static str, zh: &'static str) -> &'static str {
        match self {
            UiLang::En => en,
            UiLang::Zh => zh,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            UiLang::En => "en",
            UiLang::Zh => "zh",
        }
    }

    fn from_tag(tag: &str) -> Option<UiLang> {
        match tag {
            "en" => Some(UiLang::En),
            "zh" => Some(UiLang::Zh),
            _ => None,
        }
    }
}

impl Filter {
    /// Laid out in this order in the interface.
    pub const ALL: &'static [Filter] = &[
        Filter::All,
        Filter::Caution,
        Filter::Clean,
        Filter::WithArt,
        Filter::WithoutArt,
        Filter::Claimed,
        Filter::Unclaimed,
    ];

    /// The name a player reads.
    pub fn name(self, lang: UiLang) -> &'static str {
        match self {
            Filter::All => lang.pick("All", "全部"),
            Filter::Caution => lang.pick("With warnings", "有警告"),
            Filter::Clean => lang.pick("No warnings", "没警告"),
            Filter::WithArt => lang.pick("With art", "有图"),
            Filter::WithoutArt => lang.pick("No art", "没图"),
            Filter::Claimed => lang.pick("Size witness", "有尺寸证人"),
            Filter::Unclaimed => lang.pick("No size witness", "没尺寸证人"),
        }
    }

    /// Does `row` survive this filter?
    fn keeps(self, row: &Row) -> bool {
        match self {
            Filter::All => true,
            Filter::Caution => row.caution.is_some(),
            Filter::Clean => row.caution.is_none(),
            Filter::WithArt => row.entry.icon.is_some(),
            Filter::WithoutArt => row.entry.icon.is_none(),
            Filter::Claimed => !row.sizes.is_empty(),
            Filter::Unclaimed => row.sizes.is_empty(),
        }
    }
}

impl Query {
    /// Does this row belong on the list under this query?
    pub(crate) fn matches(&self, row: &Row) -> bool {
        self.filter.keeps(row) && self.hits_text(row)
    }

    /// The substring test. Lowercased on both sides, which is what makes an upper-case query match a
    /// lower-case name; a case-less script like Chinese is unaffected either way.
    ///
    /// The path is searched as well as the name, because the path is where the build and the handset it
    /// was filed under live: "240x320" is not part of any pack's name, and a person looking for the
    /// build for their own phone types exactly that.
    fn hits_text(&self, row: &Row) -> bool {
        let needle = self.text.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let entry = &row.entry;
        let path = entry.mod_path.to_string_lossy();
        [entry.title.as_str(), entry.build.as_str(), &*path]
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
    }
}

/// What the list is showing: the rows that survived, in display order, and how they are gathered into
/// games. Rebuilt when the query changes, not per frame.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    /// Indices into the row vector this was built from, in the order they should be drawn.
    pub(crate) hits: Vec<usize>,
    /// The same hits gathered by game: the pack's own name, and the hits that carry it. A game filed
    /// for four handsets is one entry in this list, which is the whole point of grouping.
    pub(crate) groups: Vec<Group>,
}

/// One game's worth of hits.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    /// The game's own name — the `.mod` file stem, which every build of it shares.
    pub(crate) title: String,
    pub(crate) rows: Vec<usize>,
}

impl Plan {
    /// Read `rows` through `query`.
    ///
    /// O(hits × groups) with a linear scan for the group a hit belongs to rather than a map: this corpus
    /// is 35 rows and the number of games is smaller still, so the scan costs less than the hashing and
    /// the groups come out in the order the rows are in, which is the order a person reading a list
    /// expects. A corpus in the thousands is the point at which that trade flips, and it is not this one.
    pub fn of(rows: &[Row], query: &Query) -> Plan {
        let hits: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| query.matches(row))
            .map(|(i, _)| i)
            .collect();
        let mut groups: Vec<Group> = Vec::new();
        for &i in &hits {
            let title = &rows[i].entry.title;
            match groups.iter_mut().find(|g| &g.title == title) {
                Some(g) => g.rows.push(i),
                None => groups.push(Group {
                    title: title.clone(),
                    rows: vec![i],
                }),
            }
        }
        Plan { hits, groups }
    }

    /// How many games the list is showing.
    pub fn games(&self) -> usize {
        self.groups.len()
    }

    /// How many packs the list is showing.
    pub fn packs(&self) -> usize {
        self.hits.len()
    }
}

// ────────────────────────────── the window ──────────────────────────────

/// The manager before anything is chosen: wide enough for a nav, a list of build names beside a phone,
/// and the report's longest line. Wider than it was before there was a nav — and the nav can be collapsed
/// to hand that width back to the two columns that matter.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const MANAGER_SIZE: Vec2 = Vec2::new(1240.0, 720.0);

/// The nav with words beside its icons, and the nav as a rail of icons alone. The narrow one is sized for
/// one Chinese character plus the button's own padding.
const NAV_WIDE: f32 = 132.0;
const NAV_NARROW: f32 = 42.0;

/// The box a pack's own icon is drawn in. A multiple of the 16x16 pictures every build in this corpus
/// carries, because the scale below is an integer one and 2.5 would put the pixels back where
/// nearest-neighbour cannot help them.
const ICON_BOX: f32 = 48.0;

/// The size an icon of `wide`x`high` is drawn at: the largest **whole** multiple of itself that fits
/// [`ICON_BOX`], never less than one. An icon bigger than the box is drawn at its own size and the cell
/// gives way, because shrinking 64 to 48 is a 0.75 scale and that is the thing this file promises not
/// to do to the pack's own pixels.
/// Rows narrower than this are not drawn, only skipped.
///
/// A drawing cut, not a layout one, and the difference matters: `start` and `share` were computed
/// before this filter runs, so a row that is skipped does not pull the rows after it to the left —
/// which is the mistake that would make the picture say something untrue about where things are.
const MIN_ROW_SHARE: f32 = 0.002;

/// Draw a tree as bands, one band per depth, and answer which row the pointer is over.
///
/// An icicle rather than a row of separate bars: a band begins where the band before it at the same
/// depth ended, so the eye can follow a call from its caller down into its callee.
fn draw_flame(ui: &mut egui::Ui, flame: &crate::profile::Flame) -> Option<usize> {
    const BAND: f32 = 16.0;
    const GAP: f32 = 3.0;
    let bands = flame.depth + 1;
    let height = bands as f32 * (BAND + GAP);
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let area = response.rect;
    let at = response.hover_pos();
    let mut hovered = None;
    for (i, row) in flame.rows.iter().enumerate() {
        if (row.share as f32) < MIN_ROW_SHARE {
            continue;
        }
        let left = area.left() + row.start as f32 * area.width();
        let width = ((row.share as f32) * area.width()).max(1.0);
        let top = area.top() + row.depth as f32 * (BAND + GAP);
        let band = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, BAND));
        painter.rect_filled(band, 2.0, flame_colour(row.kind, row.self_share as f32));
        if band.width() > 52.0 {
            painter.text(
                band.left_center() + egui::vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                &row.name,
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(24, 24, 28),
            );
        }
        if at.is_some_and(|p| band.contains(p)) {
            hovered = Some(i);
        }
    }
    hovered
}

/// Write a tree out as text below `data/flame/`, and answer where it went.
///
/// The program data directory is where this host keeps everything a later run can see: saves,
/// manager.json, script trust, and these reports.
fn save_flame(
    flame: &crate::profile::Flame,
    pack: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    let dir = crate::library::program_data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("data"))
        .join("flame");
    let _ = std::fs::create_dir_all(&dir);
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(flame_file_name(pack, secs));
    // The whole tree, cut by nothing — `0.0` is this renderer's "print every row", and that is the half
    // a saved file has over a screenshot. The weight is the one the page draws by.
    let text = flame.report(0.0, crate::profile::Weight::Nanos);
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// What a saved tree is called: `flame-<pack>-<seconds>.txt`.
///
/// The characters a Windows file name cannot hold are dropped rather than replaced: a pack's title is
/// Chinese more often than not, and a title padded out with `-` says less than a shorter one that still
/// reads. Seconds since the epoch rather than a formatted date because this host has no calendar code and
/// the file's own modification time is the readable half anyway. Pack first, so a directory of these
/// sorts by game and then by when.
fn flame_file_name(pack: Option<&str>, secs: u64) -> String {
    let stem: String = pack
        .unwrap_or("")
        .chars()
        .filter(|c| !r#"\/:*?"<>|"#.contains(*c) && !c.is_control())
        .collect();
    let stem: String = stem.trim().trim_matches('.').chars().take(32).collect();
    let stem = stem.trim();
    if stem.is_empty() {
        format!("flame-{secs}.txt")
    } else {
        format!("flame-{stem}-{secs}.txt")
    }
}

/// A band's colour: the three kinds of frame apart, and dimmer the less of the row was its own work —
/// so a band that is all its own reads solid and one that is mostly holding other calls reads pale.
fn flame_colour(kind: &str, self_share: f32) -> egui::Color32 {
    let (r, g, b) = match kind {
        // A branch into the module: the pack's own code, under the host's call.
        "down" => (0.30, 0.55, 0.85),
        // A guest-to-host dispatch: the host answering the pack.
        "call" => (0.85, 0.52, 0.24),
        // The host's own named spans (the frame loop, the upload).
        _ => (0.42, 0.46, 0.55),
    };
    let dim = 0.45 + 0.55 * self_share.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        (r * dim * 255.0) as u8,
        (g * dim * 255.0) as u8,
        (b * dim * 255.0) as u8,
    )
}

fn icon_size(wide: usize, high: usize) -> Vec2 {
    let (wide, high) = (wide.max(1) as f32, high.max(1) as f32);
    let scale = (ICON_BOX / wide).min(ICON_BOX / high).floor().max(1.0);
    Vec2::new(wide * scale, high * scale)
}

/// What was asked for at start-up that cannot be honoured, in words a person can read.
///
/// Both of these are things someone can get wrong and nothing else would tell them about: the manager is a
/// window with no console behind it and no file picker in front of it, and a double-clicked `quoin.exe`
/// cannot be given a flag. So each is said where it came in.
///
/// Neither is a reason to refuse to start — the run works, it just does not do what was asked.
fn complaints(skin: &str, soundfont: Option<&Path>) -> Option<String> {
    let mut said = Vec::new();
    if handset::skin(skin).is_none() && !skin.is_empty() {
        said.push(format!(
            "unknown skin name {skin}; using {}",
            handset::SKINS[0].name
        ));
    }
    if let Some(path) = soundfont.filter(|p| !p.is_file()) {
        said.push(format!(
            "the file QUOIN_SOUNDFONT points at does not exist: {} (this run will be silent)",
            path.display()
        ));
    }
    (!said.is_empty()).then(|| said.join("；"))
}

/// One nav entry's text: its character alone when the nav is collapsed, character and words when it is
/// not.
fn nav_label(icon: &str, name: &str, wide: bool) -> String {
    if wide {
        if icon.is_empty() {
            name.to_owned()
        } else {
            format!("{icon}  {name}")
        }
    } else {
        icon.to_owned()
    }
}

/// What the list says about the pack that just stopped: its name and why, or nothing when the run
/// simply ended.
fn ending_note(title: Option<String>, error: Option<String>) -> Option<String> {
    let error = error?;
    Some(match title {
        Some(title) => format!("{title}: {error}"),
        None => error,
    })
}

/// The two things the manager can do to a pack's saves.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Forgets {
    /// One file, named the way the volume names it.
    One(String),
    /// The whole directory, which is "this game starts over".
    All,
}

/// One page of the manager: what the nav asks for, and what the main area then shows.
///
/// The window is laid out as pages rather than as a pile of toggles, and that is what keeps the nav
/// honest: an entry that cannot be described as a page does not belong in it. Three of them are settings,
/// and they are pages too — a setting is something a person has to be able to look at, and a switch whose
/// current value nobody can see is the "silent success" this repository keeps being bitten by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    /// The library: one card per game. This is the page the window opens on.
    Games,
    /// What a game's music and sound effects come out of.
    Sounds,
    /// The fonts the interface and a game's own text are drawn with.
    Fonts,
    /// Which PC key is which handset key.
    Keys,
    /// What the games have saved.
    Saves,
    /// This host itself: what it is, which directory it reads, what it is set to.
    Settings,
    /// What the game on screen is spending itself on: its call tree, folded while this page asks for
    /// it and published by the guest thread.
    Dev,
}

impl Page {
    /// The nav's first group: what a person came here for.
    const MAIN: &'static [Page] = &[Page::Games, Page::Sounds, Page::Fonts, Page::Keys];

    /// The nav's second group: the tools, then this host's own settings.
    const MENU: &'static [Page] = &[Page::Saves, Page::Settings, Page::Dev];

    /// The one character the nav used to show when collapsed. Hidden for now: the nav shows words
    /// only, and this stays for a future icon rail.
    fn icon(self) -> &'static str {
        match self {
            Page::Games => "游",
            Page::Sounds => "音",
            Page::Fonts => "字",
            Page::Keys => "键",
            Page::Saves => "存",
            Page::Settings => "设",
            Page::Dev => "焰",
        }
    }

    /// What the nav calls it when there is room for words.
    fn name(self, lang: UiLang) -> &'static str {
        match self {
            Page::Games => lang.pick("Games", "游戏库"),
            Page::Sounds => lang.pick("Sounds", "音效库"),
            Page::Fonts => lang.pick("Fonts", "字体库"),
            Page::Keys => lang.pick("Keys", "按键映射"),
            Page::Saves => lang.pick("Saves", "存档管理"),
            Page::Settings => lang.pick("Settings", "设置"),
            Page::Dev => lang.pick("Developer", "开发者"),
        }
    }

    /// A short label for the collapsed rail. Two ASCII letters in English; one character in Chinese.
    fn short(self, lang: UiLang) -> &'static str {
        match self {
            Page::Games => lang.pick("Ga", "游"),
            Page::Sounds => lang.pick("So", "音"),
            Page::Fonts => lang.pick("Fo", "字"),
            Page::Keys => lang.pick("Ke", "键"),
            Page::Saves => lang.pick("Sv", "存"),
            Page::Settings => lang.pick("St", "设"),
            Page::Dev => lang.pick("De", "焰"),
        }
    }
}

/// What the window is painted in.
///
/// Two, and not a colour picker: the manager draws text and one wall of the packs' own pixel art, and
/// what a person needs is for the text to be readable in the room they are sitting in. The packs' own
/// colours are not touched by this either way — a game's screen is drawn by [`Frontend`], which knows
/// nothing about this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn name(self, lang: UiLang) -> &'static str {
        match self {
            Theme::Dark => lang.pick("Dark", "深色"),
            Theme::Light => lang.pick("Light", "浅色"),
        }
    }

    /// The other one. The nav carries a single button rather than a list, so all it can say is "next".
    fn next(self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    fn visuals(self) -> egui::Visuals {
        match self {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        }
    }
}

/// Build the manager without opening a window. Native `run` and the Web/mobile adapters share this.
pub fn build(root: &Path, skin: &str, soundfont: Option<&str>) -> Result<Manager, String> {
    // Every pack is read once, here, before the window exists: 35 of them take 0.59 s through a release
    // build and 1.52 s in a debug one, which is one pause at open rather than a stutter per frame.
    let rows = library::rows(root);
    // What a person changed last time. `Data::load` never fails: a file it cannot read is moved aside and
    // said out loud, because an order that was quietly dropped looks exactly like one that was respected.
    let data_path = manager_data::path();
    let (data, data_note) = match &data_path {
        Some(path) => Data::load(path),
        None => (
            Data::default(),
            Some(UiLang::from_system().pick("This host cannot say where its own file is, so this run's changes will not be kept", "找不到这个宿主自己的位置，所以这次的改动不会留下").to_string()),
        ),
    };
    let query = Query::default();
    let plan = Plan::of(&rows, &query);
    // A size the file remembers is where that build starts from. It is applied as a *pick*, not as a
    // decision: `library::decide` is still the one place that says what a launch will use, so a person who
    // chose a claim in the card window this session still wins over what the file says.
    let mut picks = vec![None; rows.len()];
    for (i, row) in rows.iter().enumerate() {
        let key = manager_data::key_of(root, &row.entry.mod_path);
        if let Some((width, height)) = data.screen_of(&key) {
            picks[i] = Some(Pick::Manual { width, height });
        }
    }
    // Everything that can go wrong at start-up is said in one place, in words a person can read: the
    // manager has no console behind it and a double-clicked exe cannot be given a flag.
    let chosen = handset::skin(skin);
    let soundfont = soundfont.map(PathBuf::from);
    // A SoundFont named on the way in joins the library and becomes the one in use, exactly as if it had been
    // dropped on the window. One path for both, so the two cannot end up disagreeing about what is playing.
    let mut data = data;
    if let Some(font) = data.font.as_deref() {
        crate::platform::set_font_file(font);
    }
    let lang = data
        .ui_lang
        .as_deref()
        .and_then(UiLang::from_tag)
        .unwrap_or_else(UiLang::from_system);
    if let Some(text) = soundfont
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
    {
        data.add_sound(&text);
        data.use_sound(Some(&text));
    }
    let note = [complaints(skin, soundfont.as_deref()), data_note]
        .into_iter()
        .flatten()
        .reduce(|said, more| format!("{said}；{more}"));
    // Taken once, here: this is a directory walk over the system paths and the host's own `fonts`
    // directory, and the Fonts page would otherwise repeat it on every repaint.
    let fonts = crate::platform::fonts::cjk_font_files();
    let manager = Manager {
        root: root.to_path_buf(),
        data,
        data_path,
        rows,
        query,
        plan,
        picks,
        typed: (String::new(), String::new()),
        icons: Vec::new(),
        selected: None,
        editing: None,
        capturing: None,
        dragging: None,
        drag_over: None,
        note,
        page: Page::Games,
        nav_open: true,
        fonts,
        theme: Theme::Dark,
        lang,
        saves_for: None,
        saves: Vec::new(),
        save_dir: None,
        playing: None,
        loading: None,
        current: None,
        skin: chosen.unwrap_or(&handset::SKINS[0]),
        sound_checked: BTreeSet::new(),
        sound_add: String::new(),
        sound_drag: None,
        sound_over: None,
        dev_saved: None,
        dirty: false,
        left: false,
    };
    Ok(manager)
}

/// Install the manager's fonts and dark theme on a fresh egui context.
pub fn prepare_context(cc: &eframe::CreationContext<'_>) {
    egui_window::load_cjk_fonts(&cc.egui_ctx);
    // The list is text on a dark background and nothing else, but the phone drawn beside it has
    // to read as an object against something darker than its own body.
    cc.egui_ctx.set_visuals(Theme::Dark.visuals());
}

/// Open the manager in a native window.
#[cfg(target_arch = "wasm32")]
pub fn run(_root: &Path, _skin: &str, _soundfont: Option<&str>) -> Result<(), String> {
    Err("the manager's native window is not available in this build".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run(root: &Path, skin: &str, soundfont: Option<&str>) -> Result<(), String> {
    let manager = build(root, skin, soundfont)?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(MANAGER_SIZE)
            .with_icon(crate::egui_window::app_icon())
            .with_title(crate::APP_NAME),
        ..Default::default()
    };
    eframe::run_native(
        crate::APP_NAME,
        options,
        Box::new(move |cc| {
            prepare_context(cc);
            Ok(Box::new(manager))
        }),
    )
    .map_err(|e| format!("could not open a window: {e}"))
}

/// The size of one card in the library, and the numbers the gestures are measured in.
const CARD: Vec2 = Vec2::new(136.0, 152.0);
/// How wide a card's window is.
const EDITOR_W: f32 = 520.0;

pub use crate::platform::window::SEPARATE_WINDOW_POSSIBLE;

/// What one card in the library draws.
///
/// Collected before the drawing rather than read inside it: the drawing happens in a closure that needs
/// `self` mutably for the card's gestures, and a card's picture is not something a gesture should be
/// borrowing through.
#[derive(Clone)]
struct CardFace {
    /// The game's own name — what the file orders, and what a drag moves.
    title: String,
    /// What to call it: the name a person gave it, or the pack's own.
    name: String,
    /// The picture, already decoded and uploaded at a whole multiple of its own pixels.
    icon: Option<(egui::TextureHandle, Vec2)>,
    /// The row of the game's first build — the one a double click starts.
    first_build: Option<usize>,
    /// How many builds of it there are, so a card can say so rather than hide it.
    builds: usize,
    /// Whether this is the card a person last chose.
    chosen: bool,
}

/// One card: the picture on top, the name under it.
///
/// The icon is drawn at the whole multiple [`icon_size`] worked out and through a nearest-neighbour
/// filter, which are the same two rules the game's own screen is drawn under — a pack's picture is pixel
/// art wherever it appears, and the library is where it appears most.
fn paint_card(ui: &egui::Ui, rect: PointRect, face: &CardFace, dragging: bool) {
    let visuals = ui.visuals();
    let (fill, edge) = if face.chosen {
        (visuals.selection.bg_fill, visuals.selection.stroke.color)
    } else {
        (
            visuals.faint_bg_color,
            visuals.widgets.noninteractive.bg_stroke.color,
        )
    };
    let fill = if dragging {
        fill.gamma_multiply(0.7)
    } else {
        fill
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(10), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, edge),
        egui::StrokeKind::Inside,
    );
    let picture = PointRect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + CARD.y * 0.40),
        Vec2::splat(ICON_BOX),
    );
    if let Some((texture, size)) = &face.icon {
        painter.image(
            texture.id(),
            PointRect::from_center_size(picture.center(), *size),
            PointRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 46.0),
        Align2::CENTER_TOP,
        elide(&face.name, 9),
        FontId::proportional(14.0),
        visuals.strong_text_color(),
    );
    if face.builds > 1 {
        painter.text(
            Pos2::new(rect.center().x, rect.bottom() - 24.0),
            Align2::CENTER_TOP,
            format!("{} build(s)", face.builds),
            FontId::proportional(11.0),
            visuals.weak_text_color(),
        );
    }
}

/// A pick's numbers, when it is a pick that has some.
///
/// A claim without a screen is not a thing `library::decide` can produce, but the type allows it, and a
/// function that promises two numbers should not be the one that panics on the day it happens.
fn pick_size(pick: Pick) -> Option<(u16, u16)> {
    match pick {
        Pick::Claim { width, height, .. } => Some((width, height)),
        Pick::Manual { width, height } => Some((width, height)),
    }
}

/// Files dropped on the window this frame: their paths and the names to call them by.
///
/// `DroppedFile::path` has a value on the desktop. The ones without one (byte streams dragged from a
/// browser) are refused — the manager has nowhere to put such a thing, and pretending to take it and
/// then dropping it is the worst kind.
fn dropped_files(ctx: &egui::Context) -> Vec<(PathBuf, String)> {
    ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|file| {
                let path = file.path.clone()?;
                let name = if file.name.is_empty() {
                    path.file_name()?.to_string_lossy().into_owned()
                } else {
                    file.name.clone()
                };
                Some((path, name))
            })
            .collect()
    })
}

/// The last segment of a path, used as a row's title in the list.
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// A name short enough to sit on a card.
///
/// By characters and not by points: egui cannot be asked how wide a string will be until it has laid it
/// out, and a Chinese name is two of these per character anyway. A name that is cut says so, rather than
/// trailing off with no sign that there is more of it.
fn elide(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_string();
    }
    let mut out: String = text.chars().take(most.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The window: a nav on the left, and whichever page it asked for taking the rest.
pub struct Manager {
    root: PathBuf,
    /// What a person changed, and the order they put their games in. Read at open, written when it changes.
    data: Data,
    /// Where `data` came from, so it can be written back. `None` when this host cannot say where its own
    /// executable is — the manager then runs on the defaults and says so where a person can read it.
    data_path: Option<PathBuf>,
    /// Every pack found, each with the report its own bytes give. Built once, at open.
    rows: Vec<Row>,
    query: Query,
    /// What the query keeps, rebuilt when the query changes rather than every frame.
    plan: Plan,
    /// Set while the query is ahead of `plan`.
    dirty: bool,
    /// The **game** whose card is lit, by its own name. A card is a game, not a build: which of the builds
    /// to start is what the card's window is for.
    selected: Option<String>,
    /// One size pick per row, and every one of them starts empty unless the file remembers one: a wrong
    /// canvas is silent, so the honest opening state is "nobody has chosen".
    picks: Vec<Option<Pick>>,
    /// The two boxes the hand-typed claim is being filled in, as text.
    typed: (String, String),
    /// One icon per row, with the size it is drawn at, built on the first frame that draws them.
    icons: Vec<Option<(egui::TextureHandle, Vec2)>>,
    /// The card window, while one is open.
    editing: Option<Editing>,
    /// The handset key waiting for the next PC key on the Keys page.
    capturing: Option<String>,
    /// The network page's two controls: whether guest sockets leave the process, and the address they
    /// leave through. Text rather than a parsed address because a half-typed one must not be applied.
    /// The card being dragged, by game name.
    dragging: Option<String>,
    /// The card a drag is currently over, remembered **on the frames the pointer is moving**: the frame a
    /// drag ends is already too late to ask, because egui has forgotten what was being held by then.
    drag_over: Option<String>,
    /// What the last thing that went wrong had to say, kept until something replaces it.
    note: Option<String>,
    /// Which page the main area is showing, and which entry the nav has lit.
    page: Page,
    /// Whether the nav is showing words or only its one-character icons.
    nav_open: bool,
    /// The font files the Fonts page lists, taken once at open. The list is a directory walk (the
    /// system paths plus whatever a person put in the host's own `fonts` directory), and doing
    /// that per frame would walk the disk sixty times a second for an answer that has not moved.
    fonts: Vec<(String, bool)>,
    /// What the window is painted in. Applied when it changes rather than every frame: `set_visuals`
    /// rebuilds egui's style, and doing that sixty times a second to arrive at the same answer is work
    /// for nothing.
    theme: Theme,
    /// The language the manager's own chrome is drawn in.
    lang: UiLang,
    /// The saves page: which pack it is about, and what that pack has saved. Read when the page is opened
    /// rather than every frame — it is directory IO.
    saves_for: Option<String>,
    saves: Vec<library::Save>,
    save_dir: Option<PathBuf>,
    /// The game, once one is started. Its screen takes this window's main area — or, when it has one of
    /// its own ([`Manager::game_window`]), a second window this one asked `eframe` for. Either way there
    /// is no second event loop: the guest runs on its own thread and the front end draws what it publishes.
    playing: Option<Playing>,
    /// A pack being loaded on a worker thread. The window keeps drawing while this is present;
    /// booting on the UI thread froze it and made the FPS badge look like a collapse.
    loading: Option<Loading>,
    /// The pack that is playing, so that a game which ends has a name to be reported under.
    current: Option<String>,
    /// The handset the game is drawn as, and the colours it is painted in.
    skin: &'static Skin,
    /// Which SoundFonts are ticked in the sound library. Kept here rather than in the note: it is a thing about
    /// what is on screen now, not about what a person wants next time.
    sound_checked: BTreeSet<String>,
    /// What is being typed into the Add box.
    sound_add: String,
    /// The sound row being dragged, and the row a drag is over — the same pair the cards keep, for the same
    /// reason: by the frame a drag ends, egui has forgotten what was being held.
    sound_drag: Option<usize>,
    sound_over: Option<usize>,
    left: bool,
    /// Where the last saved call tree went, or what went wrong. Cleared when a new page is opened, so it
    /// reads as an answer to the click rather than as a standing claim about the disk.
    ///
    /// Kept here rather than in [`Manager::note`] because that one is drawn by the *list*: a person who
    /// just pressed a button on the developer page is looking at the developer page.
    dev_saved: Option<String>,
}

/// A game being played: the window's drawing state, and the thread that owns the machine it is drawing.
///
/// One struct rather than two fields because they live and die together. A `Guest` with no front end
/// would run a game nobody is looking at; a front end with no `Guest` could never hand back the run's
/// report, which is the reason the manager ends a game the way it does.
/// A pack that is being booted off the UI thread.
struct Loading {
    title: String,
    own_window: bool,
    previous: Option<String>,
    result: std::sync::mpsc::Receiver<Result<crate::runtime::Runtime, String>>,
}

/// A game being played: the window's drawing state, and the thread that owns the machine it is drawing.
struct Playing {
    frontend: Frontend,
    guest: crate::session::Guest,
    /// Whether *this* run draws in a window of its own — decided when it was started, not asked again.
    ///
    /// A snapshot rather than a question asked every frame: the switch says "next run", and a game that
    /// jumped between the page and its own window the moment somebody flipped it would be that promise
    /// broken. It is also what keeps `main` and `game_window` in agreement — they read one field, so they
    /// cannot disagree about which of the two arrangements this run is in.
    own_window: bool,
}

/// An open card window: which game it is about, what is being typed into it, and which build of that game
/// its size picker is pointed at.
///
/// The typed text lives here rather than in [`Manager`] because it is about one game and one build, and it
/// is thrown away when the window closes — which is what makes closing the window mean "forget what I was
/// typing" rather than "keep it for the next card, which is a different game".
#[derive(Debug, Clone, PartialEq, Eq)]
struct Editing {
    /// The game, by the name the library knows it under.
    title: String,
    /// The name box: what this game is to be called.
    name: String,
    /// The icon box: a picture to draw instead of the pack's own, as a path below the pack root.
    icon: String,
    /// Which build's reading page is showing inside the window — a row index, or `None` when the game is
    /// not in the current filter's list at all.
    build: Option<usize>,
}

impl Manager {
    /// How many packs the library found.
    ///
    /// The Web entry asks this once, right after `build`, to say out loud that an empty list is
    /// a browser-storage limitation rather than a broken page. `library::scan` walks `std::fs`,
    /// which has no backend on wasm32, so until packs can come from a browser file source this
    /// is the honest difference between "no packs here" and "the page failed to start".
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Two columns — the nav, then the page — and the card window over the top of both when one is open.
    /// Kept apart from [`eframe::App::update`] because a headless test can call exactly this with no
    /// window, no frame and no guest, which is how these panes get checked at all while nobody is allowed
    /// to boot a pack.
    fn panels(&mut self, ctx: &egui::Context) {
        self.poll_loading(ctx);
        self.nav(ctx);
        let launch = self.main(ctx);
        self.editor(ctx);
        if let Some(at) = launch {
            self.start(ctx, at);
        }
        // The game's own window, when it has one. Asked for after `start` so the frame a game is started
        // is also the frame its window appears — a click that looks like nothing happened is the shape
        // this repository keeps being bitten by.
        if self.game_window(ctx) {
            self.finish(ctx);
        }
    }

    /// The nav: one column, two groups, and the switch that collapses it to a rail of icons.
    ///
    /// It is the outermost panel, so collapsing it hands the width back to the list and the page. The
    /// collapsed form is not a hidden form: every entry is still there and still clickable, each with its
    /// own character on it — a nav that vanishes entirely is a nav a person has to remember.
    fn nav(&mut self, ctx: &egui::Context) {
        let wide = self.nav_open;
        egui::SidePanel::left("manager-nav")
            .exact_width(if wide { NAV_WIDE } else { NAV_NARROW })
            .resizable(false)
            .show(ctx, |ui| {
                // The switch says which way it goes, rather than what the state is: a "≡" would leave a
                // person guessing which of the two they are looking at.
                let arrow = if wide { "«" } else { "»" };
                if ui
                    .button(arrow)
                    .on_hover_text(if wide { self.lang.pick("Collapse the nav", "收起导航") } else { self.lang.pick("Expand the nav", "展开导航") })
                    .clicked()
                {
                    self.nav_open = !wide;
                }
                ui.separator();
                if wide {
                    ui.label(egui::RichText::new(self.lang.pick("Main", "主菜单")).weak().size(11.0));
                }
                for &page in Page::MAIN {
                    self.nav_entry(ui, page, wide);
                }
                ui.separator();
                if wide {
                    ui.label(egui::RichText::new(self.lang.pick("Menu", "菜单")).weak().size(11.0));
                }
                for &page in Page::MENU {
                    self.nav_entry(ui, page, wide);
                }
                // A run that fills this window hides every page while the nav keeps taking clicks, so a
                // click looks like nothing happened — the entry lights up and the page it names does not
                // draw. This is the one panel still on screen, so the explanation belongs here, with
                // both ways out of it.
                if self.playing.as_ref().is_some_and(|game| !game.own_window) {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(if wide { self.lang.pick("⚠ this run fills the window", "⚠ 这一局占满本窗") } else { "⚠" })
                            .weak()
                            .size(11.0),
                    )
                    .on_hover_text(
                        self.lang.pick(
                            "The game is drawn in the middle area, so switching pages is not visible — the nav still takes clicks, but the page is covered.\n\
                             · To switch pages: press Esc to end this run first.\n\
                             · To switch pages while playing (for example to watch the Developer page): Settings → Game window → turn on \"open in a new window\"; it takes effect next run.",
                            "游戏画在中间那块区域，所以换页看不见 —— 导航还点得动，只是页面被它盖住了。\n\
                             · 要切页：先按 Esc 结束这一局。\n\
                             · 想边玩边切页（比如边玩边看「开发者」页）：设置 → 游戏窗口 → 打开\
                             「弹出新窗口」，下一局生效。",
                        ),
                    );
                }
                ui.separator();
                // Two entries that are not pages: the theme is a value to flip in place, and a rescan is
                // an action. Both name what they will do rather than what they are, for the same reason
                // the collapse switch does.
                if ui
                    .button(nav_label(
                        "色",
                        &format!("{} {}", self.lang.pick("Theme:", "主题："), self.theme.name(self.lang)),
                        wide,
                    ))
                    .on_hover_text(self.lang.pick("Dark / light", "深色 / 浅色"))
                    .clicked()
                {
                    self.theme = self.theme.next();
                    ctx.set_visuals(self.theme.visuals());
                }
                if ui
                    .button(nav_label("扫", self.lang.pick("Rescan", "重新扫描"), wide))
                    .on_hover_text(self.lang.pick("Read the pack directory again — the cost of opening the library", "重新读一遍包目录 —— 开库那一下的代价"))
                    .clicked()
                {
                    self.rescan();
                }
            });
    }

    /// One nav entry, lit while it is the page being shown.
    fn nav_entry(&mut self, ui: &mut egui::Ui, page: Page, wide: bool) {
        // The rail shows a short label rather than the old one-character icon: `Page::icon` is hidden
        // for now, and a rail entry still has to be readable.
        let text = if wide {
            page.name(self.lang).to_owned()
        } else {
            page.short(self.lang).to_owned()
        };
        if ui.selectable_label(self.page == page, text).clicked() {
            if self.page != page {
                self.dev_saved = None;
            }
            self.page = page;
        }
    }

    /// Re-read every pack under the root.
    ///
    /// This is the cost the window pays at open (35 rows ≈ 0.6 s release / 1.5 s debug), which is why it
    /// is an action a person takes rather than something a frame does.
    fn rescan(&mut self) {
        self.rows = library::rows(&self.root);
        // The picks are per row, and the rows have just been rebuilt: keeping them would attach a size to
        // whichever pack now happens to sit at that index.
        self.picks = vec![None; self.rows.len()];
        // The icons are matched to rows by index too, and each texture is named after the pack's path —
        // dropping them means the next frame loads the ones that are actually there.
        self.icons.clear();
        self.selected = None;
        self.saves_for = None;
        // A half-typed size belongs to the pack that was chosen when it was typed, and the pack it would
        // now land on is a different one.
        self.typed = (String::new(), String::new());
        self.plan = Plan::of(&self.rows, &self.query);
        self.dirty = false;
    }

    /// Rebuild the list. Called when the query changes, not per frame: 35 rows is small, but the
    /// filtering reads every row's name, build and path, and a frame is 60 of those a second.
    fn rebuild(&mut self) {
        self.plan = Plan::of(&self.rows, &self.query);
        self.dirty = false;
    }

    /// The list's own controls: what it shows, and in what order.
    ///
    /// They sit at the top of the list's column rather than in a bar across the window, because that is
    /// what they act on: a search box drawn over the *window* reads as though it filtered the pages too.
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(self.lang.pick("Search", "搜索"));
                // While a game is running the keyboard belongs to it, so the box is not offered: a search
                // box that answers a keystroke aimed at the phone is one nobody can type in. The phone
                // side enforces the same rule (`egui_window::phone_keys` looks at who wants the
                // keyboard), because a guard in only one of the two places is a guard the next front end
                // can miss.
                let running = self.playing.is_some();
                let typed = ui
                    .add_enabled(
                        !running,
                        egui::TextEdit::singleline(&mut self.query.text)
                            .desired_width(240.0)
                            .hint_text(if running {
                                self.lang.pick(
                                    "A game is running; the keyboard belongs to it",
                                    "游戏在跑，按键归游戏",
                                )
                            } else {
                                self.lang
                                    .pick("Name, build or directory", "名字、build 或目录")
                            }),
                    )
                    .changed();
                if typed {
                    self.dirty = true;
                }
                if !self.query.text.is_empty()
                    && ui.button(self.lang.pick("Clear", "清空")).clicked()
                {
                    self.query.text.clear();
                    self.dirty = true;
                }
                ui.separator();
                egui::ComboBox::from_id_salt("manager-filter")
                    .selected_text(self.query.filter.name(self.lang))
                    .show_ui(ui, |ui| {
                        for filter in Filter::ALL {
                            if ui
                                .selectable_label(
                                    self.query.filter == *filter,
                                    filter.name(self.lang),
                                )
                                .clicked()
                            {
                                self.query.filter = *filter;
                                self.dirty = true;
                            }
                        }
                    });
                ui.separator();
                ui.separator();
                ui.label(if self.lang == UiLang::Zh {
                    format!("{} 个包 · {} 个游戏", self.plan.packs(), self.plan.games())
                } else {
                    format!(
                        "{} pack(s) · {} game(s)",
                        self.plan.packs(),
                        self.plan.games()
                    )
                });
            });
        });
    }

    /// The page, and whatever it asked to start.
    ///
    /// A running game takes the whole page — the library is the main page and the game replaces it —
    /// **unless** that game has a window of its own (see `game_window`), in which case this page never
    /// stops drawing the list. The in-page arrangement is not the old one kept alive for a phone's sake:
    /// it *is* the phone's arrangement, and a desktop falls back to it when the setting is off.
    fn main(&mut self, ctx: &egui::Context) -> Option<usize> {
        let mut launch = None;
        let mut ended = false;
        // Decided before the panel borrows `self.playing`: which arrangement is in force is a question
        // about this run and one setting, and asking it inside the borrow would be asking two questions.
        // This run's arrangement, not the setting's: see `Playing::own_window`.
        let own_window = self.playing.as_ref().is_some_and(|game| game.own_window);
        let current = self.current.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(title) = self.loading.as_ref().map(|loading| loading.title.clone()) {
                ui.heading(title);
                ui.add(egui::Spinner::new());
                ui.label(self.lang.pick("Loading the pack…", "正在加载游戏包…"));
                ui.label(
                    egui::RichText::new(self.lang.pick(
                        "The window stays responsive while the image, resources and applet are prepared.",
                        "准备镜像、资源与 applet 时，窗口仍然保持响应。",
                    ))
                    .weak()
                    .size(11.0),
                );
                return;
            }
            if own_window {
                // The library stays, and the two things a person needs when the game's window is behind
                // something are added: where it went, and a way to stop it.
                //
                // **No `return` here, and that is the point of the arrangement.** With the game drawing
                // in a window of its own this one is free, so the pages below are what a person
                // navigates — the developer page most of all, which is only any use *while* a game runs.
                // The contract is that the manager keeps drawing the list: that is what makes the
                // "start a second game" path clickable, and what keeps a click on the nav from naming a
                // page that never draws.
                ui.heading(current.clone().unwrap_or_default());
                ui.label(self.lang.pick("This game runs in its own window — closing that window ends it.", "这个游戏在它自己的窗口里运行 —— 关掉那个窗口就等于结束它。"));
                if ui.button(self.lang.pick("End this run", "结束这次运行")).clicked() {
                    ended = true;
                }
                ui.separator();
            } else if let Some(game) = self.playing.as_mut() {
                // The other arrangement, which is the phone's: the game takes this page, and a person on
                // a desktop gets it by turning the switch off. Nothing below draws until the run ends,
                // so the nav cannot change what is on screen — `nav()` says so, in the one panel that is
                // still visible.
                let scale = egui_window::fit(ui.available_size(), game.frontend.body());
                if !game.frontend.view(ui, scale) {
                    ended = true;
                }
                return;
            }
            match self.page {
                Page::Games => launch = self.games(ui),
                Page::Sounds => self.sounds_page(ui),
                Page::Fonts => self.fonts_page(ui),
                Page::Keys => self.keys_page(ui),
                Page::Saves => self.saves_page(ui),
                Page::Settings => self.settings_page(ui),
                Page::Dev => self.dev_page(ui),
            }
        });
        if ended {
            self.finish(ctx);
        }
        launch
    }

    /// The game library: the list's own controls over a wall of cards.
    fn games(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        self.controls(ui);
        ui.separator();
        if self.rows.is_empty() {
            ui.label(
                self.lang.pick("There is no .mod in this directory. Point BREW_GAME_DIR at the games, or put the packs in Game/.", "这个目录下没有 .mod。把 BREW_GAME_DIR 指向游戏所在的目录，或把包放进 Game/。"),
            );
            return None;
        }
        if self.plan.groups.is_empty() {
            ui.label(
                self.lang
                    .pick("No pack matches this query.", "没有包满足这个查询。"),
            );
            return None;
        }
        self.cards(ui)
    }

    /// Every card the library is to draw, in the order a person put them in.
    ///
    /// The order is the **file's**, not the scan's: somebody who dragged a game to the front meant it, and a
    /// game the file has never heard of goes last rather than jumping the queue. Ties — which is every game
    /// until somebody drags one — are broken by name, so the wall does not reshuffle itself between runs.
    fn card_faces(&self) -> Vec<CardFace> {
        let mut groups: Vec<&Group> = self.plan.groups.iter().collect();
        groups.sort_by(|a, b| {
            self.data
                .rank(&a.title)
                .cmp(&self.data.rank(&b.title))
                .then_with(|| a.title.cmp(&b.title))
        });
        groups
            .into_iter()
            .map(|group| {
                let first = group.rows.first().copied();
                CardFace {
                    name: self
                        .data
                        .name_of(&group.title)
                        .map(str::to_owned)
                        .unwrap_or_else(|| group.title.clone()),
                    icon: first.and_then(|at| self.icons.get(at).and_then(|held| held.clone())),
                    first_build: first,
                    builds: group.rows.len(),
                    chosen: self.selected.as_deref() == Some(group.title.as_str()),
                    title: group.title.clone(),
                }
            })
            .collect()
    }

    /// The cards, and the four things a card answers: a click chooses it, a double click plays it,
    /// the gear button opens its window, and a drag moves it.
    ///
    /// The drag is deliberately not egui's `dnd_*`: that hangs `Sense::drag()` on the card along with the
    /// payload machinery, and a card that senses drags is a card that has given up its click — which here is
    /// how a game is chosen.
    fn cards(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut launch = None;
        let mut settings = None;
        let faces = self.card_faces();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for face in &faces {
                    let (rect, response) =
                        ui.allocate_exact_size(CARD, egui::Sense::click_and_drag());
                    let dragged = self.dragging.as_deref() == Some(face.title.as_str());
                    paint_card(ui, rect, face, dragged);

                    // A small settings control on the card itself. It is deliberately separate from
                    // the card's click gesture: the card chooses a game, this button edits that
                    // game's name, icon and launch size.
                    let settings_rect = PointRect::from_min_size(
                        Pos2::new(rect.right() - 24.0, rect.top() + 4.0),
                        Vec2::new(20.0, 18.0),
                    );
                    let gear = egui::Button::new(egui::RichText::new("⚙").size(13.0)).frame(false);
                    let settings_clicked = ui
                        .put(settings_rect, gear)
                        .on_hover_text(self.lang.pick("Game settings", "游戏设置"))
                        .clicked();
                    if settings_clicked {
                        settings = Some(face.title.clone());
                    }

                    if response.clicked() && !settings_clicked {
                        self.selected = Some(face.title.clone());
                    }
                    if response.double_clicked() {
                        launch = face.first_build.or(launch);
                    }
                    if response.drag_started() {
                        self.dragging = Some(face.title.clone());
                    }
                    if response.dragged() {
                        // Recorded *while* the pointer is moving: on the frame a drag ends, egui has
                        // already forgotten what was being held, so asking then is asking too late — and
                        // a drop that lands nowhere is a reorder that silently does not happen.
                        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                            if rect.contains(pointer) {
                                self.drag_over = Some(face.title.clone());
                            }
                        }
                    }
                    if response.drag_stopped() {
                        let from = self.dragging.take();
                        let to = self.drag_over.take();
                        self.moved(from, to);
                    }
                }
            });
        });

        if let Some(title) = settings {
            self.open_card(&title);
        }
        launch
    }

    /// A drag that ended: put the game where it was dropped, and write the order down.
    fn moved(&mut self, from: Option<String>, to: Option<String>) {
        let (Some(from), Some(to)) = (from, to) else {
            return;
        };
        if from == to {
            return;
        }
        let titles: Vec<String> = self
            .card_faces()
            .into_iter()
            .map(|face| face.title)
            .collect();
        let (Some(from), Some(to)) = (
            titles.iter().position(|t| t == &from),
            titles.iter().position(|t| t == &to),
        ) else {
            return;
        };
        self.data.move_game(&titles, from, to);
        self.save_data();
    }

    /// Open a card's window: what a person may change about a game, and nothing that changes the game.
    fn open_card(&mut self, title: &str) {
        let build = self
            .plan
            .groups
            .iter()
            .find(|group| group.title == title)
            .and_then(|group| group.rows.first().copied());
        self.editing = Some(Editing {
            title: title.to_string(),
            name: self.data.name_of(title).unwrap_or(title).to_string(),
            icon: self.data.icon_of(title).unwrap_or_default().to_string(),
            build,
        });
    }

    /// Write the note back to disk, saying so if it will not go.
    ///
    /// A save that failed silently is the worst shape this repository knows: the order looks kept and the
    /// next run reads the old one.
    fn save_data(&mut self) {
        let Some(path) = self.data_path.clone() else {
            return;
        };
        if let Err(why) = self.data.save(&path) {
            self.note = Some(if self.lang == UiLang::Zh {
                format!("管理器的记录写不进去：{why}")
            } else {
                format!("the manager record cannot be written: {why}")
            });
        }
    }

    /// The open card's window.
    ///
    /// A layer over the main window (`egui::Modal`), not a second operating-system window. That is the
    /// point of it: a phone has no second window, and holding a card is the gesture a phone uses to open
    /// this. Esc, "Close", or a click on the backdrop closes it, as every modal ever built does.
    ///
    /// Everything in here is about what a **person** sees: a name, a picture, and the screen a launch would
    /// use. None of it goes near a pack — the pack's own bytes are the pack's.
    fn editor(&mut self, ctx: &egui::Context) {
        let Some(mut editing) = self.editing.clone() else {
            return;
        };
        let mut close = false;
        let mut launch = None;
        let mut open = true;
        egui::Window::new(elide(&editing.name, 18))
            .id(egui::Id::new("manager-card"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(EDITOR_W)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(EDITOR_W);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.lang.pick("Close", "关闭")).clicked() {
                            close = true;
                        }
                    });
                });
                ui.label(
                    egui::RichText::new(self.lang.pick(
                        "These only change the manager's note: not one byte of the pack changes, and the game does not know.",
                        "这些只改管理器的记录：包自己的字节一个都不动，游戏也不知道。",
                    ))
                    .weak()
                    .size(11.0),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(self.lang.pick("Name", "名称"));
                    ui.add(
                        egui::TextEdit::singleline(&mut editing.name)
                            .desired_width(300.0)
                            .hint_text(self.lang.pick("Empty = use the pack's own name", "留空 = 用包自己的名字")),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(self.lang.pick("Icon", "图标"));
                    ui.add(
                        egui::TextEdit::singleline(&mut editing.icon)
                            .desired_width(300.0)
                            .hint_text(self.lang.pick("Empty = use the pack's own picture (png / bmp)", "留空 = 用包自己的图（png / bmp）")),
                    );
                });
                // Written into the note in memory as it is typed, and to the file when the window closes.
                // Only when it differs from what is already there: `set_name` creates an entry for a game
                // that had none, so writing every frame would leave a note in the file for every game whose
                // window was ever opened — and "never changed" would stop being distinguishable from
                // "changed and cleared".
                let wanted_name = (!editing.name.trim().is_empty()).then(|| editing.name.trim());
                if self.data.name_of(&editing.title) != wanted_name {
                    self.data.set_name(&editing.title, &editing.name);
                }
                let wanted_icon = (!editing.icon.trim().is_empty()).then(|| editing.icon.trim());
                if self.data.icon_of(&editing.title) != wanted_icon {
                    self.data.set_icon(&editing.title, &editing.icon);
                }

                let builds: Vec<usize> = self
                    .plan
                    .groups
                    .iter()
                    .find(|group| group.title == editing.title)
                    .map(|group| group.rows.clone())
                    .unwrap_or_default();
                if builds.len() > 1 {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(self.lang.pick("Build", "版本"));
                        for &at in &builds {
                            let label = self.rows[at].entry.build.clone();
                            if ui
                                .selectable_label(editing.build == Some(at), label)
                                .clicked()
                            {
                                editing.build = Some(at);
                            }
                        }
                    });
                }
                ui.separator();
                let Some(at) = editing.build.filter(|at| *at < self.rows.len()) else {
                    ui.label(
                        self.lang.pick("This game is not in the current filter, so it has no readable build. Clear the search to see it.", "这个游戏不在当前的筛选里，所以它没有可读的版本。清掉搜索就能看到它。"),
                    );
                    return;
                };
                // The screen is the one thing here that changes what a *launch* gets, and it still goes through
                // the single decision it always did (`library::decide`) — this window does not grow a second
                // opinion about which canvas a pack boots at; it only remembers what a person chose.
                picker(
                    ui,
                    &self.rows[at],
                    &mut self.picks[at],
                    &mut self.typed.0,
                    &mut self.typed.1,
                    self.lang,
                );
                if let Some((width, height)) = self.picks[at].and_then(pick_size) {
                    let key = manager_data::key_of(&self.root, &self.rows[at].entry.mod_path);
                    self.data.set_screen(&key, Some((width, height)));
                }
                ui.separator();
                let going = library::decide(&self.rows[at], self.picks[at]);
                if ui.button(if self.lang == UiLang::Zh {
                    format!("开始游戏（{}）", going.words)
                } else {
                    format!("Start game ({})", going.words)
                }).clicked() {
                    launch = Some(at);
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(170.0)
                    .show(ui, |ui| {
                        for line in &self.rows[at].detail {
                            let text = egui::RichText::new(line).monospace().size(11.0);
                            if crate::probe::is_caution(line) {
                                ui.colored_label(Color32::LIGHT_RED, text);
                            } else {
                                ui.label(text);
                            }
                        }
                    });
            });
        if !open {
            close = true;
        }
        if close {
            self.editing = None;
            self.save_data();
        } else {
            self.editing = Some(editing);
        }
        if let Some(at) = launch {
            self.editing = None;
            self.save_data();
            self.start(ctx, at);
        }
    }

    /// The picture a person gave this game, decoded and uploaded.
    ///
    /// If it cannot be decoded, **say why** (into `note`) rather than drawing the pack's own picture and
    /// staying quiet: to a person both look like "I changed it and nothing happened", but one is the
    /// host's fault and the other is not.
    fn custom_icon(
        &mut self,
        title: &str,
        ctx: &egui::Context,
    ) -> Option<(egui::TextureHandle, Vec2)> {
        let given = self.data.icon_of(title)?.to_string();
        let path = self.root.join(&given);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.note = Some(if self.lang == UiLang::Zh {
                    format!("图标 {given} 读不了：{e}")
                } else {
                    format!("icon {given} cannot be read: {e}")
                });
                return None;
            }
        };
        // The manager has no container property block to read a MIME type out of — it has a file name
        // and the bytes that name was chosen for. Both halves go through the one authority that
        // decodes them, in the order `IShell::DetectType` uses: what the bytes say, then what the
        // suffix says. A hand-written extension table here is how the library lost PNG icons once
        // already, because one decoder entry has to serve every caller.
        let Some(mime) =
            crate::resource::sniff(&bytes).or_else(|| crate::resource::sniff_name(&given))
        else {
            self.note = Some(if self.lang == UiLang::Zh {
                format!("图标 {given}：本宿主没有能读它的解码器")
            } else {
                format!("icon {given}: this host has no decoder for it")
            });
            return None;
        };
        let mut props = mime.as_bytes().to_vec();
        props.push(0);
        let Ok(image) = crate::resource::image(&props, &bytes) else {
            self.note = Some(if self.lang == UiLang::Zh {
                format!("图标 {given} 解不开")
            } else {
                format!("icon {given} cannot be decoded")
            });
            return None;
        };
        let mut rgb = Vec::with_capacity(image.width * image.height * 3);
        for y in 0..image.height {
            for x in 0..image.width {
                let (r, g, b) = image.rgb(x, y);
                rgb.extend_from_slice(&[r, g, b]);
            }
        }
        let texture = ctx.load_texture(
            format!("custom-icon:{given}"),
            egui::ColorImage::from_rgb([image.width, image.height], &rgb),
            TextureOptions::NEAREST,
        );
        Some((texture, icon_size(image.width, image.height)))
    }

    /// The sound library: every bank this host can use, one per row.
    ///
    /// This host is a MIDI synthesiser (across the two corpora of 827 packs, every sound payload that
    /// passes `audio::sniff` is MIDI; the other 224 records the containers call sound are
    /// mpeg/qcelp/`audio/midi`, and this host has no decoder for any of them — the numbers and the
    /// method come from the pack census), so what can make a sound is a
    /// **SoundFont (`.sf2`)**, not wav/mp3. **It still sounds without an `.sf2`**: not choosing a bank
    /// uses the built-in table it generates itself. That is why the first row is "built-in wave tables",
    /// not "none (silent)".
    fn sounds_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.lang.pick("Sound library", "音效库"));
        ui.label(
            egui::RichText::new(if self.lang == UiLang::Zh {
                format!(
                    "{app} 不支持独立播放 MIDI，也不能播 wav/mp3 —— 游戏里的音乐由它自己的合成器放出来，\
                     音色默认用**内置的波形表**。这一页是用来**换掉**它的：把一个真正的 General MIDI 库\
                     （.sf2）放进来，下一局启动的游戏就用它。把 .sf2 拖进窗口就能加，也可以点最下面的 Add。\
                     换完**下一局**启动的游戏生效。",
                    app = crate::APP_NAME
                )
            } else {
                format!(
                    "{app} does not play MIDI on its own, and cannot play wav/mp3 — a game's music comes from \
                     its own synthesiser, using the built-in wave tables by default. This page replaces them: \
                     add a real General MIDI bank (.sf2) and the next run uses it. Drop an .sf2 on the window, \
                     or use Add at the bottom. It takes effect on the next run.",
                    app = crate::APP_NAME
                )
            })
            .weak(),
        );
        ui.separator();

        // What was dropped on the window. What is taken in is taken in; what is not is **said** rather than
        // swallowed — a `.wav` is not a SoundFont and this host cannot play one.
        for (path, name) in dropped_files(ui.ctx()) {
            let is_sf2 = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("sf2"));
            if !is_sf2 {
                self.note = Some(if self.lang == UiLang::Zh {
                    format!("{name} 不是 SoundFont —— 这个宿主只合成 MIDI，要的是 .sf2")
                } else {
                    format!("{name} is not a SoundFont — this host only synthesises MIDI and needs an .sf2")
                });
                continue;
            }
            let text = path.display().to_string();
            if self.data.add_sound(&text) {
                self.data.use_sound(Some(&text));
            }
        }

        let in_use = self.data.sound_in_use.clone();
        let mut play_through: Option<Option<String>> = None;
        let mut picked = None;
        let mut deleting = false;

        // The first row is not a SoundFont at all: it is the state of having none, which is why it has no
        // selectbox and cannot be deleted. It is not silence — the host's own tables answer there.
        if ui
            .selectable_label(
                in_use.is_none(),
                self.lang.pick("Built-in wave tables", "内置音色表"),
            )
            .on_hover_text(self.lang.pick(
                "Default: the eight wave tables the host generates at start-up; no file needed",
                "默认：宿主自己启动时生成的八张波形表，不需要任何文件",
            ))
            .clicked()
        {
            play_through = Some(None);
        }
        ui.separator();

        for (at, path) in self.data.sounds.clone().iter().enumerate() {
            let mut checked = self.sound_checked.contains(path);
            let using = in_use.as_deref() == Some(path.as_str());
            let dragging = self.sound_drag == Some(at);
            let row = ui.horizontal(|ui| {
                if ui.add(egui::Checkbox::without_text(&mut checked)).changed() {
                    picked = Some((path.clone(), checked));
                }
                let title = egui::RichText::new(elide(&file_name(path), 30)).size(15.0);
                let title = if using {
                    title.strong().color(Color32::LIGHT_GREEN)
                } else {
                    title
                };
                if ui.selectable_label(using, title).clicked() {
                    play_through = Some(Some(path.clone()));
                }
                ui.label(egui::RichText::new(path).weak().size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if using {
                        ui.label(egui::RichText::new(self.lang.pick("In use", "在用")).strong());
                    } else if !std::path::Path::new(path).is_file() {
                        // Listed but not there: said out loud rather than left to do nothing quietly.
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            self.lang.pick("File is gone", "文件不在了"),
                        );
                    }
                });
                ui.set_min_width(ui.available_width());
            });
            let response = row.response.interact(egui::Sense::click_and_drag());
            if dragging {
                ui.painter().rect_stroke(
                    response.rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0_f32, Color32::LIGHT_YELLOW),
                    egui::StrokeKind::Inside,
                );
            }
            // The same hand-rolled drag the cards use, for the same reason: a row that senses drags must still
            // answer clicks, and egui's `dnd_*` takes the click away with the payload machinery.
            if response.drag_started() {
                self.sound_drag = Some(at);
            }
            if response.dragged() {
                // Recorded while the pointer moves: by the frame a drag ends, egui has forgotten what was
                // being held, and a drop that lands nowhere is a reorder that silently does not happen.
                if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                    if response.rect.contains(pointer) {
                        self.sound_over = Some(at);
                    }
                }
            }
            if response.drag_stopped() {
                let from = self.sound_drag.take();
                let to = self.sound_over.take();
                if let (Some(from), Some(to)) = (from, to) {
                    self.data.move_sound(from, to);
                    self.save_data();
                }
            }
        }

        if let Some((path, checked)) = picked {
            if checked {
                self.sound_checked.insert(path);
            } else {
                self.sound_checked.remove(&path);
            }
        }
        if let Some(choice) = play_through {
            self.data.use_sound(choice.as_deref());
            self.save_data();
        }

        ui.separator();
        if self.sound_checked.is_empty() {
            ui.label(
                egui::RichText::new(self.lang.pick("Tick the box at the left of a row to select several, then delete. Dragging a row reorders it.", "勾一行最左边的框可以多选，然后删。拖行本身可以改顺序。"))
                    .weak()
                    .size(11.0),
            );
        } else {
            ui.horizontal(|ui| {
                ui.label(if self.lang == UiLang::Zh { format!("选中 {}", self.sound_checked.len()) } else { format!("Selected {}", self.sound_checked.len()) });
                if ui
                    .button(self.lang.pick("Delete selected", "删除选中的"))
                    .on_hover_text(self.lang.pick("If the one being deleted is in use, the library falls back to the built-in wave tables", "被删的那个如果正在用，音效库就回到内置音色表"))
                    .clicked()
                {
                    deleting = true;
                }
                if ui.button(self.lang.pick("Clear selection", "取消选择")).clicked() {
                    self.sound_checked.clear();
                }
            });
        }
        if deleting {
            let gone = std::mem::take(&mut self.sound_checked);
            self.data.drop_sounds(&gone);
            self.save_data();
        }

        ui.separator();
        ui.label(
            egui::RichText::new(self.lang.pick("Adding one means typing a path: this host has no file picker (that would be a new dependency).", "加一个要手填路径：这个宿主没有文件选择器（那会是一个新依赖）。"))
                .weak()
                .size(11.0),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.sound_add)
                    .desired_width(460.0)
                    .hint_text("…\\TimGM6mb.sf2"),
            );
            if ui.button("Add").clicked() {
                let typed = self.sound_add.trim().to_string();
                if self.data.add_sound(&typed) {
                    self.data.use_sound(Some(&typed));
                    self.save_data();
                }
                self.sound_add.clear();
            }
        });
    }

    /// The font page: where the window's Chinese comes from.
    fn fonts_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.lang.pick("Fonts", "字体库"));
        ui.label(
            egui::RichText::new(self.lang.pick(
                "Choose the font. It draws the manager and window chrome, and supplies the whole face behind a game's ABI `AEEFont` designators, Latin included.",
                "选择字体：管理器与窗口用它绘制，游戏的 ABI `AEEFont` 面整体也用它（含拉丁字母）。",
            ))
            .weak(),
        );
        ui.separator();
        let current = self.data.font.clone();
        for (path, found) in self.fonts.clone() {
            let selected = current.as_deref() == Some(path.as_str());
            ui.horizontal(|ui| {
                ui.label(if found {
                    self.lang.pick("Yes", "有")
                } else {
                    self.lang.pick("No", "无")
                });
                let button = egui::Button::new(if selected {
                    self.lang.pick("In use", "使用中")
                } else {
                    self.lang.pick("Use", "使用")
                });
                if ui.add_enabled(found, button).clicked() {
                    self.data.set_font(Some(path.clone()));
                    crate::platform::set_font_file(&path);
                    crate::egui_window::load_cjk_fonts(ui.ctx());
                    self.save_data();
                }
                ui.label(egui::RichText::new(&path).monospace().size(11.0));
            });
        }
        if ui
            .button(
                self.lang
                    .pick("Use the first available font", "使用找到的第一款字体"),
            )
            .clicked()
        {
            self.data.set_font(None);
            crate::platform::clear_font_file();
            crate::egui_window::load_cjk_fonts(ui.ctx());
            self.save_data();
        }
        if let Some(path) = current {
            ui.label(format!(
                "{} {path}",
                self.lang.pick("Saved choice:", "已保存的选择：")
            ));
        } else {
            ui.label(self.lang.pick(
                "No explicit choice: the first font found is used.",
                "未显式选择：使用找到的第一款字体。",
            ));
        }
        // The page lists what can be chosen, and the host's own `fonts` directory is where both
        // the generated faces and any font a person copied there live. Saying which is which is
        // the difference between "my font is missing" and "that file is a cache".
        if let Some(dir) = crate::platform::data::program_data_dir() {
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "{} {}",
                    self.lang.pick(
                        "Fonts dropped in this directory are listed above:",
                        "放进这个目录的 .ttf/.ttc 会出现在上面：",
                    ),
                    dir.join("fonts").display()
                ))
                .weak(),
            );
            ui.label(
                egui::RichText::new(self.lang.pick(
                    "The cjk-*.bbf files there are generated caches, not choices.",
                    "其中的 cjk-*.bbf 是生成的缓存，不在此列出。",
                ))
                .weak(),
            );
        }
    }

    /// Key mapping: PC keys and phone keys.
    fn keys_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.lang.pick("Keys", "按键映射"));
        ui.label(
            egui::RichText::new(self.lang.pick(
                "Each handset key can be bound to any PC key. Press Rebind, then the key you want; the choice is stored in manager.json. Escape cancels capture.",
                "每个手机键都可以绑定任意 PC 键。按“改键”，再按想用的键；选择会写入 manager.json。捕获时按 Escape 取消。",
            ))
            .weak(),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("manager-keys")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    for heading in if self.lang == UiLang::Zh {
                        ["手机键", "默认键", "当前绑定", "操作"]
                    } else {
                        ["Phone key", "Default", "Bound", "Action"]
                    } {
                        ui.label(egui::RichText::new(heading).strong());
                    }
                    ui.end_row();
                    for key in crate::keymap::KEYS {
                        let custom = self.data.key_binding(key.name).map(str::to_string);
                        let defaults = crate::keymap::default_bindings(key.name).join(" / ");
                        ui.label(key.name);
                        ui.label(if defaults.is_empty() {
                            "—".to_string()
                        } else {
                            defaults
                        });
                        ui.label(
                            custom
                                .clone()
                                .unwrap_or_else(|| self.lang.pick("default", "默认").to_string()),
                        );
                        ui.horizontal(|ui| {
                            if ui.button(self.lang.pick("Rebind", "改键")).clicked() {
                                self.capturing = Some(key.name.to_string());
                            }
                            if custom.is_some()
                                && ui.button(self.lang.pick("Reset", "重置")).clicked()
                            {
                                self.data.clear_key(key.name);
                                self.save_data();
                            }
                        });
                        ui.end_row();
                    }
                });
        });
        if let Some(waiting) = self.capturing.clone() {
            ui.separator();
            ui.label(format!(
                "{} {} · {}",
                self.lang
                    .pick("Press any PC key for", "请为以下手机键按任意 PC 键："),
                waiting,
                self.lang.pick("Escape cancels", "Escape 取消")
            ));
            let captured = ui.ctx().input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Key {
                        key, pressed: true, ..
                    } = event
                    {
                        Some(*key)
                    } else {
                        None
                    }
                })
            });
            if let Some(key) = captured {
                if key == egui::Key::Escape {
                    self.capturing = None;
                } else {
                    let name = format!("{key:?}");
                    self.data.set_key(&waiting, &name);
                    self.capturing = None;
                    self.save_data();
                }
            }
        }
    }

    /// The developer page: what the game that is running is spending itself on.
    ///
    /// Everything here comes from [`crate::session::Shared`], because this page is on the window's
    /// thread and the machine is not: the switch it flips is an atomic bit the worker reads, and the
    /// rows it draws were published by the worker. Nothing on this page borrows a `Runtime`, which is
    /// what keeps the rule this module is built on — one owner for the machine.
    ///
    /// A drawing rather than the command line's table because the question someone brings here is
    /// "where is the width", and that is the one question a band answers faster than a number.
    fn dev_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("开发者");
        let shared = match self.playing.as_ref() {
            Some(playing) => std::sync::Arc::clone(&playing.guest.shared),
            None => {
                ui.label(
                    "这一页看的是**正在跑的那一局**：先启动一个游戏，再回来看它把时间花在哪。",
                );
                return;
            }
        };
        let flame = shared.flame();
        let mut on = shared.profiling();
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut on, "记录调用树")
                .on_hover_text("打开后由客户机线程逐帧记录；关掉即停，已经记下的还留着")
                .changed()
            {
                shared.want_profile(on);
            }
            if ui
                .button("清空")
                .on_hover_text("丢掉已经折叠的树，从现在开始记")
                .clicked()
            {
                shared.reset_profile();
            }
            if ui
                .add_enabled(flame.is_some(), egui::Button::new("保存到文件"))
                .on_hover_text(
                    "把现在这张树写成文本，放在 quoin.exe 旁边。文件名带包名与时间，不覆盖旧的；\
                     存下去的是**全部行**、不按宽度裁 —— 这正是它比屏幕上那张图多的东西。",
                )
                .clicked()
            {
                let said = match flame.as_ref() {
                    Some(tree) => match save_flame(tree, self.current.as_deref()) {
                        Ok(path) => format!("已保存 {} 行：{}", tree.rows.len(), path.display()),
                        Err(e) => format!("保存失败：{e}"),
                    },
                    None => "还没有树可以保存".to_string(),
                };
                self.dev_saved = Some(said);
            }
        });
        if let Some(said) = self.dev_saved.clone() {
            ui.label(egui::RichText::new(said).weak().size(11.0));
        }
        ui.label(
            egui::RichText::new(
                "记录要读时钟：每个宿主调用两次 `Instant::now()`，所以这一局会略慢，时间列也含这点开销\
                 —— 它**按调用次数摊**，调用越密的方法被抬得越多；**指令数那一列完全不受影响**（那是 \
                 guest 真退休的条数，可以跨趟比较）。要看绝对时间就做 A/B：在同一个画面上把开关关掉、再\
                 打开，看画面右上角 `QUOIN_STATS=1` 那行里的「模拟 ms」。",
            )
            .weak()
            .size(11.0),
        );
        ui.separator();
        // A tree that was folded while the switch was on stays readable after it is turned off — that
        // is deliberate, because what the page shows is "since the last reset". What is not
        // deliberate is leaving a reader to guess which of the two they are looking at, so say it.
        if !shared.profiling() {
            ui.label(
                egui::RichText::new("开关是关的：下面这张画的是**关掉之前**记下的最后一份。")
                    .weak()
                    .size(11.0),
            );
        }
        let Some(flame) = flame else {
            ui.label("还没有发布过任何一帧的树。");
            return;
        };
        if flame.folded == 0 {
            ui.label("这一局还没有记录到任何宿主调用。");
            return;
        }
        ui.label(format!(
            "{} 行 · {} 个节点 · 折叠 {} 次 · {:.2} ms · {} 条指令{}",
            flame.rows.len(),
            flame.nodes,
            flame.folded,
            flame.total_ms,
            flame.total_steps,
            if flame.dropped > 0 {
                format!(" · {} 个帧被上限丢掉", flame.dropped)
            } else {
                String::new()
            }
        ));
        if flame.elided > 0 {
            ui.label(
                egui::RichText::new(format!(
                    "图里少了 {} 行，都在比这张图更深的层上：条的宽度和位置仍然按真实数据算。",
                    flame.elided
                ))
                .weak()
                .size(11.0),
            );
        }
        let hovered = draw_flame(ui, &flame);
        if let Some(i) = hovered {
            let row = &flame.rows[i];
            ui.label(
                egui::RichText::new(format!(
                    "{}  ·  {:.2} ms  ·  {} 条指令  ·  调用 {} 次  ·  自身 {:.0}%",
                    row.name,
                    row.ms,
                    row.steps,
                    row.calls,
                    row.self_share * 100.0,
                ))
                .monospace()
                .size(11.0),
            );
        }
        ui.separator();
        let cut = MIN_ROW_SHARE as f64;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in flame.rows.iter().filter(|r| r.share >= cut) {
                ui.label(
                    egui::RichText::new(format!(
                        "{}{:>5.1}%  {:>8.2} ms  {:>10} i  {:>6}x  {}",
                        "  ".repeat(row.depth),
                        row.share * 100.0,
                        row.ms,
                        row.steps,
                        row.calls,
                        row.name,
                    ))
                    .monospace()
                    .size(11.0),
                );
            }
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.lang.pick("Settings", "设置"));
        ui.horizontal(|ui| {
            ui.label(self.lang.pick("Interface language", "界面语言"));
            if ui.button(self.lang.label()).clicked() {
                self.lang = self.lang.next();
                self.data.set_ui_lang(self.lang.tag());
                self.save_data();
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(self.lang.pick("Theme", "主题"));
            if ui.button(self.theme.name(self.lang)).clicked() {
                self.theme = self.theme.next();
                ui.ctx().set_visuals(self.theme.visuals());
            }
        });
        ui.separator();
        ui.label(self.lang.pick("Skin (next run)", "皮肤（下一局生效）"));
        for skin in handset::SKINS {
            if ui
                .selectable_label(self.skin.name == skin.name, skin.name)
                .clicked()
            {
                self.skin = skin;
            }
        }
        ui.separator();
        ui.label(
            self.lang
                .pick("Audio backend (next run)", "音频后端（下一局生效）"),
        );
        let current = self.data.audio_backend();
        let mut chosen = None;
        egui::ComboBox::from_id_salt("manager-audio-backend")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for backend in crate::audio::AudioBackend::ALL {
                    let enabled = backend.available();
                    let label = if enabled {
                        backend.label().to_string()
                    } else {
                        format!(
                            "{} {}",
                            backend.label(),
                            self.lang.pick("(not built)", "（未编入）")
                        )
                    };
                    ui.add_enabled_ui(enabled, |ui| {
                        if ui.selectable_label(current == backend, label).clicked() {
                            chosen = Some(backend);
                        }
                    });
                }
            });
        if let Some(backend) = chosen {
            self.data.set_audio_backend(backend);
            self.save_data();
        }
        if !current.available() {
            ui.colored_label(
                Color32::YELLOW,
                self.lang.pick(
                    "The recorded backend is not built into this binary; choose Rust.",
                    "当前记录选中的后端没有编入这个二进制；请选择 Rust。",
                ),
            );
        }
        ui.separator();
        ui.label(
            self.lang
                .pick("Game window (next run)", "游戏窗口（下一局生效）"),
        );
        let mut separate = self.data.uses_separate_window();
        let flipped = ui
            .add_enabled_ui(SEPARATE_WINDOW_POSSIBLE, |ui| {
                ui.checkbox(
                    &mut separate,
                    self.lang.pick(
                        "Open a new window: a started game gets its own window",
                        "弹出新窗口：启动的游戏开在它自己的窗口里",
                    ),
                )
                .on_hover_text(self.lang.pick(
                    "When off, the game is drawn inside this manager window, the only arrangement a phone has.",
                    "关掉时游戏画在管理器这一窗里（同一个窗口），也就是手机上唯一的那种做法。",
                ))
                .changed()
            })
            .inner;
        if flipped {
            self.data.set_separate_window(separate);
            self.save_data();
        }
        if !SEPARATE_WINDOW_POSSIBLE {
            ui.label(
                egui::RichText::new(self.lang.pick(
                    "This platform has only one window, so this setting has no effect here: the game is drawn in the manager window.",
                    "这个平台只有一个窗口，所以这一项在这里不生效：游戏画在管理器这一窗里。",
                ))
                .weak(),
            );
        }
        ui.separator();
        let mut show_fps = self.data.uses_fps();
        if ui
            .checkbox(
                &mut show_fps,
                self.lang
                    .pick("Show FPS overlay (next run)", "显示帧率（下一局生效）"),
            )
            .on_hover_text(self.lang.pick(
                "Draws the guest presentation rate in the game window's top-left corner.",
                "在游戏窗口左上角显示 guest 的呈现帧率。",
            ))
            .changed()
        {
            self.data.set_show_fps(show_fps);
            self.save_data();
        }
        ui.separator();
        ui.label(
            egui::RichText::new(format!("{} {}", crate::APP_NAME, env!("CARGO_PKG_VERSION")))
                .strong(),
        );
        ui.label(self.lang.pick("Pack root", "包根目录"));
        ui.label(
            egui::RichText::new(self.root.display().to_string())
                .monospace()
                .size(11.0),
        );
        ui.label(if self.lang == UiLang::Zh {
            format!("{} 个包 · {} 个游戏", self.plan.packs(), self.plan.games())
        } else {
            format!(
                "{} pack(s) · {} game(s)",
                self.plan.packs(),
                self.plan.games()
            )
        });
        ui.separator();
        // Every `.mod` this host found, as the manager names it in its own record — the key is the
        // path below the pack root, and that is the only form that survives the root moving from
        // one machine to another. The absolute path is one click away because that is the form a
        // person pastes into a report, and asking them to join two strings by hand is exactly the
        // step that gets a bug filed against the wrong pack.
        let packs = self.rows.len();
        egui::CollapsingHeader::new(if self.lang == UiLang::Zh {
            format!("包路径（{packs} 个，相对上面的包根目录）")
        } else {
            format!("Pack paths ({packs}, relative to the root above)")
        })
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(self.lang.pick(
                    "Click one to copy its full path; that is the form to paste into a report.",
                    "点一行就复制它的完整路径 —— 报问题时贴这一行。",
                ))
                .weak()
                .size(11.0),
            );
            if ui
                .button(self.lang.pick("Copy all full paths", "复制全部完整路径"))
                .clicked()
            {
                let all: Vec<String> = self
                    .rows
                    .iter()
                    .map(|row| row.entry.mod_path.display().to_string())
                    .collect();
                ui.ctx().copy_text(all.join("\n"));
            }
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for row in &self.rows {
                        let full = row.entry.mod_path.display().to_string();
                        let key = manager_data::key_of(&self.root, &row.entry.mod_path);
                        // A label rather than a button: a deep path is wider than this pane, and
                        // truncating it keeps the column readable while the tooltip and the click
                        // still carry the whole thing.
                        let line = ui.add(
                            egui::Label::new(egui::RichText::new(&key).monospace().size(11.0))
                                .truncate()
                                .sense(egui::Sense::click()),
                        );
                        if line.clicked() {
                            ui.ctx().copy_text(full.clone());
                        }
                        line.on_hover_text(&full);
                    }
                });
        });
        ui.separator();
        ui.label(self.lang.pick(
            "The manager record (order, names, icons and sizes live here)",
            "管理器的记录（顺序、名字、图标、分辨率都记在这里）",
        ));
        match &self.data_path {
            Some(path) => {
                ui.label(
                    egui::RichText::new(path.display().to_string())
                        .monospace()
                        .size(11.0),
                );
            }
            None => {
                ui.colored_label(
                    Color32::LIGHT_RED,
                    self.lang.pick(
                        "This host cannot say where its own file is, so these changes will not be kept",
                        "找不到这个宿主自己的位置，所以这次的改动不会留下",
                    ),
                );
            }
        }
        if ui
            .button(self.lang.pick("Write it now", "现在写一遍"))
            .on_hover_text(self.lang.pick(
                "Order changes and card edits save themselves; this button is for when you want to be sure.",
                "拖过顺序或改过卡片之后会自己写；这个按钮是给不放心的",
            ))
            .clicked()
        {
            self.save_data();
        }
        ui.separator();
        ui.label(
            egui::RichText::new(self.lang.pick(
                "This host starts no pack by itself: every line above comes from the pack's own bytes or from this host's record.",
                "这个宿主不启动任何包：上面每一行都是它从包自己的字节、或从它自己的记录里读出来的。",
            ))
            .weak(),
        );
    }

    /// Saves: choose a pack and see what it has saved.
    fn saves_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.lang.pick("Saves", "存档管理"));
        ui.label(
            egui::RichText::new(self.lang.pick(
                "Saves belong to each pack: the host gives every pack a directory, and one pack cannot read another's saves. This page touches those files on disk; the pack itself is not affected.",
                "存档是每个包自己的：宿主给每个包一个目录，一个包的存档别的包读不到。这一页动的是磁盘上的\
                 那些文件，包本身不受影响。",
            ))
            .weak(),
        );
        ui.separator();
        let mut pick = None;
        let current = self.saves_for.clone();
        egui::ComboBox::from_id_salt("manager-saves")
            .width(360.0)
            .selected_text(
                current
                    .clone()
                    .unwrap_or_else(|| self.lang.pick("Choose a pack", "选一个包").to_string()),
            )
            .show_ui(ui, |ui| {
                for (at, row) in self.rows.iter().enumerate() {
                    let key = manager_data::key_of(&self.root, &row.entry.mod_path);
                    let label = format!("{} · {}", row.entry.title, row.entry.build);
                    if ui
                        .selectable_label(current.as_deref() == Some(key.as_str()), label)
                        .clicked()
                    {
                        pick = Some(at);
                    }
                }
            });
        // Choosing a pack reads the new one: **read first, then record**; the other order would make
        // `saves_panel`'s check think "already read", and the page would show the previous pack's directory.
        if let Some(at) = pick {
            self.read_saves(at);
        }
        ui.separator();
        let Some(key) = self.saves_for.clone() else {
            ui.label(self.lang.pick(
                "Choose a pack to see what it has saved.",
                "选一个包，看它存了什么。",
            ));
            return;
        };
        let Some(at) = self
            .rows
            .iter()
            .position(|row| manager_data::key_of(&self.root, &row.entry.mod_path) == key)
        else {
            ui.label(self.lang.pick(
                "This pack is not in the current list.",
                "这个包不在当前的列表里。",
            ));
            return;
        };
        ui.label(
            egui::RichText::new(format!(
                "{} · {}",
                self.rows[at].entry.title, self.rows[at].entry.build
            ))
            .strong(),
        );
        self.saves_panel(ui, at);
    }

    /// Read one pack's saves: the directory, what is in it, and which pack this page is now about.
    ///
    /// One place, so that "which directory" and "what is in it" can never be about two different packs —
    /// which is exactly what happened when choosing a pack only set the name and left the reading to a
    /// guard that the name itself had just satisfied.
    fn read_saves(&mut self, at: usize) {
        let mod_path = self.rows[at].entry.mod_path.clone();
        let key = manager_data::key_of(&self.root, &mod_path);
        self.save_dir = library::default_save_dir(&mod_path);
        self.saves = self
            .save_dir
            .as_deref()
            .map(library::saves_in)
            .unwrap_or_default();
        self.saves_for = Some(key);
    }

    /// What a pack has saved, and the two ways to get rid of it.
    ///
    /// There is no "not while a game is running" guard here, because this pane cannot be reached while
    /// one is: when a game runs, the right-hand pane *is* the game. The buttons are simply not drawn in
    /// that state, so a guard would be a branch that never runs — a rule that only looks enforced.
    fn saves_panel(&mut self, ui: &mut egui::Ui, at: usize) {
        // Read once per pack chosen, not per frame: this is directory IO.
        let key = manager_data::key_of(&self.root, &self.rows[at].entry.mod_path);
        if self.saves_for.as_deref() != Some(key.as_str()) {
            self.read_saves(at);
        }
        let Some(dir) = self.save_dir.clone() else {
            ui.label(self.lang.pick("This host cannot say where its own file is, so it cannot name this pack's save directory.", "这个宿主不知道自己被放在哪，所以说不出这个包的存档位置。"));
            return;
        };
        ui.label(
            egui::RichText::new(dir.display().to_string())
                .weak()
                .monospace()
                .size(11.0),
        );
        if self.saves.is_empty() {
            ui.label(self.lang.pick(
                "This pack has saved nothing yet — play a round and something will appear here.",
                "这个包什么都还没存 —— 玩一局之后这里会有东西。",
            ));
            return;
        }
        let mut delete = None;
        let mut wipe = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for save in &self.saves {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&save.name).monospace());
                    ui.label(
                        egui::RichText::new(if self.lang == UiLang::Zh {
                            format!("{} 字节", save.bytes)
                        } else {
                            format!("{} bytes", save.bytes)
                        })
                        .weak()
                        .size(11.0),
                    );
                    if ui.button(self.lang.pick("Delete", "删除")).clicked() {
                        delete = Some(save.name.clone());
                    }
                });
            }
        });
        ui.separator();
        if ui
            .button(
                self.lang
                    .pick("Clear this pack's saves", "清空这个包的存档"),
            )
            .on_hover_text(self.lang.pick(
                "After this the game starts from zero, as if it had never been played",
                "删掉之后这个游戏会从零开始，就像从没玩过。",
            ))
            .clicked()
        {
            wipe = true;
        }
        match (delete, wipe) {
            (Some(name), _) => self.forget(&dir, Forgets::One(name)),
            (None, true) => self.forget(&dir, Forgets::All),
            (None, false) => {}
        }
    }

    /// Delete one saved file, or the whole directory, and say how it went.
    ///
    /// Kept apart from the buttons so that "delete a save" is a thing a test can do without a window, a
    /// pointer and a layout pass. This is the path that removes a player's files, and the one thing that
    /// should not be checkable only by clicking.
    fn forget(&mut self, dir: &Path, what: Forgets) {
        let failure = match &what {
            Forgets::One(name) => library::delete_save(dir, name).err(),
            Forgets::All => library::wipe_saves(dir).err(),
        };
        match failure {
            Some(why) => self.note = Some(why),
            // A success clears the last thing that went wrong: the note is how a previous failure is
            // still being read, and leaving it up after a working delete is a lie.
            None => self.note = None,
        }
        // Read the directory again now rather than guessing what it looks like: this page is showing what
        // was just deleted, and a listing one frame out of date is a listing that is wrong.
        self.saves = library::saves_in(dir);
    }

    /// Whether a game started now gets a window of its own.
    ///
    /// Three inputs and no fourth: this build can open a window ([`SEPARATE_WINDOW_POSSIBLE`]), this
    /// backend can ([`egui::Context::embed_viewports`] answering "I cannot"), and the person has not
    /// turned it off.
    ///
    /// Asked **once, at launch**, and the answer is kept in [`Playing::own_window`]. That is what makes the
    /// switch's own promise ("next run") true: asking this every frame would move the running game between
    /// the page and its own window the moment somebody flipped it — in the same frame the old window is
    /// being closed.
    fn game_window_wanted(&self, ctx: &egui::Context) -> bool {
        // `embed_viewports` is the backend answering "I cannot do this one". An embedded viewport would
        // be drawn *inside* this window's page — on top of the arrangement `main` already makes for
        // exactly that case — so the question is asked once, here, rather than twice.
        SEPARATE_WINDOW_POSSIBLE && !ctx.embed_viewports() && self.data.uses_separate_window()
    }

    /// The id of the game's window.
    ///
    /// One id rather than one per game: the manager runs one game at a time — starting another ends the
    /// one that is on — so a second id would only be a way for two windows to exist at once.
    /// `from_hash_of` is deterministic (a fixed-key hasher over the string), so the same string is the
    /// same window on every frame of every run.
    fn game_viewport() -> egui::ViewportId {
        egui::ViewportId::from_hash_of("manager-game")
    }

    /// The game's own window, while it has one. Returns whether the run ended on this frame.
    ///
    /// A second window is opened by asking `egui` for a *viewport*: `show_viewport_immediate` renders the
    /// child in its own operating-system window ("This call will pause the current viewport and render the
    /// child viewport in its own window"), and `eframe`'s glow backend keeps a `ViewportIdMap<Viewport>`
    /// beside the root's to do it. What this is *not* is a second event loop or a second machine: the
    /// guest is still on the thread `session.rs` gave it, and this only draws what that thread publishes.
    ///
    /// Immediate and not deferred, and the reason is a bound rather than a preference: the deferred form
    /// wants a `Send + Sync + 'static` callback, which would mean putting this window's `Frontend` behind
    /// an `Arc<Mutex<_>>`. The immediate form borrows it. What it costs is stated by egui too — parent and
    /// child repaint each other — and a run that is presenting frames asks for a repaint every frame
    /// anyway, which is the one case where that cost is already being paid.
    ///
    /// Three ways out, and all three end the run: the game comes back on its own (`view` answers false),
    /// the person closes this window, or they press the stop button on the manager's page. The button
    /// exists for the same reason the "where it went" line does — a second window can end up behind the
    /// first one, and a run nobody can find their way out of is a trap.
    ///
    /// What it cannot do is outlive the manager: the root viewport is the one `eframe` runs the event
    /// loop for, so closing the manager ends the program and this window with it. That is the whole of
    /// the "close the manager, close everything" rule.
    fn game_window(&mut self, ctx: &egui::Context) -> bool {
        let Some(game) = self.playing.as_mut() else {
            return false;
        };
        // This run's arrangement, settled when it was started — not the setting's, which promises "next
        // run". See [`Playing::own_window`].
        if !game.own_window {
            return false;
        }
        let body = game.frontend.body();
        let (wide, high) = game.frontend.device();
        let title = format!(
            "{} {wide}x{high}",
            self.current.as_deref().unwrap_or(crate::APP_NAME)
        );
        // The window opens at a whole multiple of the body that fits the desktop — the same question the
        // solo window asks of the same `fit`, with the same `PAD`, so the two windows cannot drift apart.
        // After this the size is the window's own, and the picture below is re-fitted to whatever room it
        // has every frame: a person who drags an edge gets another integer multiple, not a blur.
        let scale = egui_window::fit(ctx.screen_rect().size(), body);
        let builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(body * scale + Vec2::splat(2.0 * egui_window::PAD))
            .with_min_inner_size(body + Vec2::splat(2.0 * egui_window::PAD));
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.with_icon(crate::egui_window::app_icon());
        let mut ended = false;
        ctx.show_viewport_immediate(Self::game_viewport(), builder, |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let scale = egui_window::fit(ui.available_size(), body);
                if !game.frontend.view(ui, scale) {
                    ended = true;
                }
            });
            // The window's own close box. `egui` reports the request rather than acting on it, so the
            // close is *this* line saying the run is over — and `finish` is what takes the window away.
            if ctx.input(|i| i.viewport().close_requested()) {
                ended = true;
            }
        });
        // This window is what draws that one — an immediate viewport is drawn from this pass and from
        // nowhere else — so asking for the next repaint here is what keeps the game moving.
        ctx.request_repaint();
        ended
    }

    /// Start a pack and show it — in its own window when one is wanted, in this one otherwise.
    ///
    /// Failing is not fatal here: the list comes back with the reason written on it, which is what a
    /// manager can do and a plain run cannot.
    fn start(&mut self, ctx: &egui::Context, at: usize) {
        // One game at a time, so a second launch ends the first. Ended *here* rather than overwritten:
        // `Guest`'s `Drop` asks its thread to stop but cannot wait for it, so the report only ever comes
        // back through `finish`. This used to be unreachable — a running game covered the page, so
        // nothing on it could be clicked — and giving a game a window of its own is what makes it reachable.
        let mut previous = None;
        if self.playing.is_some() {
            self.finish(ctx);
            previous = Some(match self.note.take() {
                Some(why) => {
                    if self.lang == UiLang::Zh {
                        format!("上一局已经结束：{why}")
                    } else {
                        format!("the previous run has ended: {why}")
                    }
                }
                None => self
                    .lang
                    .pick(
                        "the previous run has ended (only one game runs at a time)",
                        "上一局已经结束（同时只运行一个游戏）",
                    )
                    .to_string(),
            });
        }
        if self.loading.is_some() {
            return;
        }
        let entry = self.rows[at].entry.clone();
        let going = library::decide(&self.rows[at], self.picks[at]);
        let own_window = self.game_window_wanted(ctx);
        // Reading the image, mounting its resources, running the module and starting the applet are
        // the expensive part. Keep them off the UI thread so the library keeps painting and the FPS
        // badge does not mistake a loading freeze for a frame-rate collapse.
        let (send, result) = std::sync::mpsc::channel();
        let thread_entry = entry.clone();
        let screen = going.screen;
        std::thread::spawn(move || {
            let _ = send.send(library::boot_at(&thread_entry, screen));
        });
        self.loading = Some(Loading {
            title: entry.title,
            own_window,
            previous,
            result,
        });
        ctx.request_repaint();
    }

    /// Collect a background boot result, or keep the loading frame alive until it arrives.
    fn poll_loading(&mut self, ctx: &egui::Context) {
        let Some(loading) = self.loading.as_ref() else {
            return;
        };
        let outcome = match loading.result.try_recv() {
            Ok(outcome) => outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("the loader stopped without an answer".to_string())
            }
        };
        let loading = self.loading.take().expect("checked above");
        match outcome {
            Ok(rt) => self.show_loaded(ctx, rt, loading),
            Err(error) => {
                let said = match loading.previous {
                    Some(previous) => format!("{previous}；{}: {error}", loading.title),
                    None => format!("{}: {error}", loading.title),
                };
                self.note = Some(said);
                ctx.request_repaint();
            }
        }
    }

    /// The worker finished successfully; build the front end and hand the machine to the session.
    fn show_loaded(&mut self, ctx: &egui::Context, rt: crate::runtime::Runtime, loading: Loading) {
        let Loading {
            title,
            own_window,
            previous,
            ..
        } = loading;
        let (wide, high) = (u32::from(rt.device.width), u32::from(rt.device.height));
        let guest = crate::session::Guest::start_with_backend(
            rt,
            crate::session::Sound::named(self.data.sound_in_use.as_deref()),
            self.data.audio_backend(),
            None,
            crate::screen::Format::Rgba8888,
        );
        let frontend = Frontend::new(
            Arc::clone(&guest.shared),
            self.skin,
            self.data.keys.clone(),
            self.data.uses_fps(),
        );
        if !own_window {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "{} {wide}x{high}",
                title
            )));
        }
        self.current = Some(title);
        self.note = previous;
        self.playing = Some(Playing {
            frontend,
            guest,
            own_window,
        });
        ctx.request_repaint();
    }

    /// The game is over: say why where the list can be read, and take its window with it.
    fn finish(&mut self, ctx: &egui::Context) {
        // Joining the guest thread is what gets the reason. It is waited for here rather than in a
        // repaint that a stuck pack could sit on, and the wait is bounded: this sets the cancel bit,
        // and the interpreter reads it inside a guest function.
        // Read before the `take`: which of the two ways this ends belongs to the run that is ending, not
        // to what the switch says now — see [`Playing::own_window`].
        let own_window = self.playing.as_ref().is_some_and(|game| game.own_window);
        let why = self
            .playing
            .take()
            .map(|game| game.guest.finish())
            .and_then(|finished| finished.error);
        self.note = ending_note(self.current.take(), why);
        if own_window {
            // The game's window goes with the game. Sent to *that* viewport and not to the root one:
            // `ViewportCommand::Close` addressed to the root is the whole program going away.
            ctx.send_viewport_cmd_to(Self::game_viewport(), egui::ViewportCommand::Close);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(crate::APP_NAME.to_owned()));
        }
        // A game that has just ended is news the list has to draw.
        ctx.request_repaint();
    }
}

impl eframe::App for Manager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.icons.len() != self.rows.len() {
            self.icons = self
                .rows
                .iter()
                .map(|r| {
                    r.entry.icon.as_ref().map(|icon| {
                        let texture = ctx.load_texture(
                            format!("icon:{}", r.entry.mod_path.display()),
                            egui::ColorImage::from_rgb([icon.width, icon.height], &icon.rgb),
                            // The rule the device screen is drawn under, and for the same reason: a
                            // pack's icon is pixel art, and a linear filter through a fractional scale
                            // is what turned it to mush.
                            TextureOptions::NEAREST,
                        );
                        (texture, icon_size(icon.width, icon.height))
                    })
                })
                .collect();
            // The picture a person chose goes over the pack's own. **Something has to consume it**: a path
            // written into the note with nothing changing on screen is the silent success this repository
            // keeps being bitten by.
            for at in 0..self.rows.len() {
                let title = self.rows[at].entry.title.clone();
                if self.data.icon_of(&title).is_some() {
                    if let Some(icon) = self.custom_icon(&title, ctx) {
                        self.icons[at] = Some(icon);
                    }
                }
            }
        }
        if self.dirty {
            self.rebuild();
        }
        // Esc leaves the manager. While a game is running Esc is that game's — it ends the session and comes
        // back to the library, the same key and a smaller step — and while a card's window is open, Esc is
        // that window's, which is what `Modal` reads it for.
        if self.playing.is_none()
            && self.editing.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.left = true;
        }
        self.panels(ctx);
        // The in-page arrangement has no separate eframe window to keep asking for frames. `Solo`
        // asks for the next one at the end of its own draw; this is the same request for the manager
        // page, otherwise a still screen only repaints when the mouse moves.
        if self.playing.as_ref().is_some_and(|game| !game.own_window) {
            ctx.request_repaint_after(crate::session::FRAME);
        }
        if self.left {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// One pack's screen, side by side: each witness says its own number and none of them is marked until a
/// person marks one.
///
/// It is why the picker carries each claim's own evidence sentence rather than three bare numbers —
/// `Row::sizes` is not a vetted list (a marquee bar clears the area bound and looks exactly like a
/// background), and the one thing that tells those two apart is the line under the number.
///
/// The hand-typed claim is a pair of boxes plus its own row, and it only becomes a `Pick` once both
/// parse: a half-typed size is not a screen, and offering it as one would be the host inventing a
/// number.
fn picker(
    ui: &mut egui::Ui,
    row: &Row,
    pick: &mut Option<Pick>,
    wide: &mut String,
    high: &mut String,
    lang: UiLang,
) {
    ui.label(
        egui::RichText::new(lang.pick(
            "Screen size: each witness says its own number and none is selected. Without a pick, the button below starts at the number it names.",
            "屏幕尺寸：每个证人各说各的，没人被选中。不选就按下面按钮上写的那个启动。",
        ))
        .strong(),
    );
    if row.sizes.is_empty() {
        ui.label(
            egui::RichText::new(lang.pick(
                "Neither witness spoke for this pack: the directory names no size, and the host can read too few pictures from its containers.",
                "这个包两个证人都没说话：目录里没写尺寸，宿主能从它容器里读出的图又太少。",
            ))
            .weak(),
        );
    }
    for c in &row.sizes {
        ui.selectable_value(
            pick,
            Some(c.pick()),
            format!("{} {}x{}", c.witness.name(), c.width, c.height),
        );
        // The evidence rides in the same block as its number, so a claim cannot be read without it.
        ui.label(
            egui::RichText::new(format!("  {}", c.evidence))
                .weak()
                .size(11.0),
        );
    }
    ui.horizontal(|ui| {
        ui.label(lang.pick("Typed by hand", "手填"));
        ui.add(
            egui::TextEdit::singleline(wide)
                .desired_width(46.0)
                .hint_text(lang.pick("W", "宽")),
        );
        ui.label("x");
        ui.add(
            egui::TextEdit::singleline(high)
                .desired_width(46.0)
                .hint_text(lang.pick("H", "高")),
        );
        match library::manual_size(wide, high) {
            Some((w, h)) => {
                ui.selectable_value(
                    pick,
                    Some(Pick::Manual {
                        width: w,
                        height: h,
                    }),
                    format!("{w}x{h}"),
                );
            }
            None => {
                ui.label(
                    egui::RichText::new(lang.pick(
                        "Both width and height must be integers from 40 to 1024.",
                        "宽、高都要是 40 到 1024 的整数才能选。",
                    ))
                    .weak(),
                );
            }
        }
    });
}

#[cfg(test)]
#[path = "test/manager.rs"]
mod tests;
