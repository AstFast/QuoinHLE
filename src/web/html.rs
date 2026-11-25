//! A small HTML reader and painter for `IHtmlViewer`.
//!
//! The reference viewer supports a subset — hyperlinks, simple forms, inline images, left/center/right
//! alignment, bold and large text, and text colour — and says outright
//! that it "is not intended to support for arbitrary content". This module reads exactly that
//! subset into a flat document: tags in document order for the three DOM slots, and text runs with
//! the font, colour and alignment they inherit for `Redraw`.
//!
//! Two things that list does not spell out but a real document always uses: character references
//! (`&amp;` is one character, not five) and inline elements — a `<b>` inside a sentence changes the
//! face of the words around it, not the line they sit on. A run therefore carries `inline`, and
//! [`layout`] groups consecutive inline runs into one visual row. What is *not* body text —
//! `<script>`, `<style>`, `<title>`, a `<textarea>`'s initial value, an `<option>`'s label — is read
//! into its element and never painted.
//!
//! A form control is the third thing: `<input>`, `<select>` and `<textarea>` are boxes on the page
//! with a value in them, not words in a row. They are runs too (a row can hold a label and two
//! checkboxes), and [`render`] draws the box before the value. The last mark it draws is the cursor:
//! whatever element the viewer's focus names is inverted, because a page the user can act on has to
//! show where they are — the reference viewer's own cursor is the same inversion.

use std::collections::HashMap;

use crate::abi::generated as g;
use crate::device::make_rgb;
use crate::display::{Display, Rect};
use crate::encoding::TextEncoding;
use crate::font::{Face, Faces};
use crate::hle::HResult;
use crate::mem::Mem;

/// The state an `IHtmlViewer` keeps.
///
/// `HVOpt` is `void *`, so one table holds every `HVOPT_*` (widths, colours, `IWeb*`); `data` is the
/// document bytes the three loaders handed over, and `notify`/`user` are the callback
/// `SetNotifyFn` installed.
#[derive(Debug, Default)]
pub struct Html {
    pub options: HashMap<i32, u32>,
    pub data: Vec<u8>,
    pub notify: u32,
    pub user: u32,
    pub view_state: String,
    pub busy: bool,
    /// The page no longer matches what is on the screen.
    ///
    /// The repaint is the viewer's own debt, not the pack's: `IHtmlViewer::Redraw` says "The
    /// application does not normally have to call this function to ensure that the screen has been
    /// updated after changes have been made to the content or options of the viewer. The viewer
    /// takes care of ... ensuring that changes are properly reflected on the screen." A handset
    /// collects that debt in its screen-invalidate pass; this host has no such pass, so the debt is
    /// paid in `IDisplay::Update`, immediately before the frame is presented — which is also the
    /// only point late enough to survive a pack that draws its own frame *after* the page was set,
    /// as the dialog-shaped packs do.
    pub dirty: bool,
    /// `HVOPT_SZ_URL` as text: the address the document was loaded from. It is what a relative
    /// `href` is resolved against before `HVN_JUMP` hands it to the pack, and it is kept as an
    /// owned string because the caller's `char *` need only live for the `SetOption` call.
    pub base_url: String,
    /// `HVOPT_SZ_TYPE` as text: the document's media type. `text/plain` is read as bare lines and
    /// everything else as markup, which is the one decision the type makes.
    pub doc_type: String,
    /// Decoded pictures, keyed by the `src` that named them. A `None` is a `src` that could not be
    /// had or decoded — remembered so a redraw does not try it again on every frame.
    ///
    /// It lives here rather than on [`Document`] because the document is rebuilt by every redraw
    /// and a picture does not change between them.
    pub images: HashMap<String, Option<std::sync::Arc<crate::resource::Bitmap>>>,
    /// The parsed document, kept because `Redraw` asks for it every frame and parsing is the
    /// expensive half of a frame. Cleared whenever the bytes or the media type change, which is
    /// what `Widgets::html_set_data`, `html_set_bytes` and `html_set_doc_type` do.
    ///
    /// It does *not* hold the pictures: those come from [`pictures`] beside it, so a document
    /// stays immutable from the moment it is parsed.
    pub parsed: Option<std::sync::Arc<Document>>,
    /// The focus position in [`Document::focus_items`], not an element index. A document that
    /// shrinks under it is clamped on the next key rather than stored back here.
    pub focus: usize,
    /// How far the page is scrolled down, in pixels; 0 is the top.
    ///
    /// Only a key press moves it — [`Widgets::html_reveal_focus`] scrolls just far enough to show
    /// the element the focus landed on, which is the one scroll motion the interface has. A redraw
    /// reads it and nothing else, so a frame never accumulates its own drift.
    ///
    /// [`Widgets::html_reveal_focus`]: crate::web::widget::Widgets::html_reveal_focus
    pub scroll: i16,
    /// How tall the document is at the width it was last measured at, or 0 for "not measured".
    /// The scrollbar's thumb is one reader; the other is the scroll limit, which is this height
    /// less the rectangle — so a stale value here hides the bottom of the page rather than merely
    /// drawing a short thumb.
    pub content: i16,
    /// What [`Html::content`] was measured against: the parse it came from, the width, the
    /// paragraph spacing, how many pictures had settled, and how many faces the display had
    /// installed. Any of them changing makes the height a different number, and a stale height
    /// draws a thumb of the wrong length and hides the bottom of the page.
    pub content_key: Option<(usize, i16, u32, u32, u32, u32)>,
    /// Bumped every time a picture is stored, so [`Html::content_key`] can notice.
    pub pictures: u32,
    /// What the user has typed, ticked or chosen. The parse is immutable, so an edit lives beside
    /// it rather than inside it — see [`Values`] for why that is the only shape that works with a
    /// cached document.
    pub values: Values,
    /// The pen while it is down, or `None` when nobody is touching the page.
    pub drag: Option<Drag>,
    /// The runtime clock reading at the first `Redraw` of this page, and how far the clock has
    /// advanced since. A `<marquee>` turns the difference into timer steps (see
    /// [`marquee_pose`]); both are reset whenever the document bytes change, so a new page starts
    /// its scroll at the beginning rather than inheriting the old one's pose.
    pub marquee_epoch: Option<u32>,
    pub marquee_elapsed: u32,
}

/// A pen that is currently on the page.
///
/// A press means nothing on its own: it becomes a click if it is released where it started, and a
/// drag — scrolling the page — once it has moved further than [`DRAG_SLOP`]. Both need the place
/// and the offset it started at, because a drag scrolls from where the page *was*, not by the
/// distance the pen has travelled since the last event ever arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drag {
    pub x0: i16,
    pub y0: i16,
    /// The scroll offset the page had when the pen went down.
    pub scroll: i16,
    /// Whether the pen has moved far enough that this is a drag rather than a click. A drag never
    /// activates what it started on: scrolling a page by dragging a link must not follow the link.
    pub moved: bool,
}

/// How far a pen has to travel before it is dragging rather than clicking, in pixels. A handset
/// stylus jitters by a pixel or two on the way down, and a link that needs a perfectly still hand
/// is a link nobody can follow.
pub const DRAG_SLOP: i16 = 4;

/// Leading between two lines of body text, on top of the font's own cell.
const ROW_LEADING: i16 = 2;

// The `HVOPT_*` ids live in [`crate::web::options`], the one hand-written part of the ABI tables
// (the header declares them as `WEBOPT_*` aliases, which the extractor cannot harvest).
// Re-exported here so callers that already reach for `web::html::HVOPT_*` keep working.
pub use super::options::*;

/// One tag the viewer keeps. `attrs` is in the document's own order, which is what `GetAttrValue`'s
/// element index is counted against.
#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    /// The text the element encloses, which `GetElemText` hands back.
    pub text: String,
    /// Whether the element encloses nothing at all — `<p></p>` rather than `<p>x</p>`.
    ///
    /// The reference layout reads this off the node stream (`node[node_length] == 2`, the close tag
    /// being the next node) and skips both the frame push and the margin for such an element, so an
    /// empty paragraph is invisible: no gap, no line, nothing. The close tag of an empty element has
    /// no frame of its own to read a margin out of either, which is what keeps a page full of
    /// `<p></p>` from opening two gaps for every one of them.
    pub empty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Line {
    /// No line of its own: the row exists only for its margin, if it has one.
    None,
    /// One line when the line it broke was empty (`<br>`, a newline in `<pre>`). That is what makes
    /// `<p>a<br>b</p>` two adjacent lines while `<p>a<br><br>b</p>` leaves a blank one between them.
    IfEmpty,
    /// One line whatever was on the line it broke (`<hr>`).
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Rule,
    Bullet,
}

/// The square an `<li>` leaves beside the first thing its item draws.
///
/// The flag is a `&mut bool` rather than document state because it belongs to the parse and not to
/// the page: exactly one run claims it, and `</li>` throws it away if none did.
fn take_bullet(bullet: &mut bool) -> Option<Mark> {
    std::mem::take(bullet).then_some(Mark::Bullet)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// Nothing wrote a colour, so `HVOPT_RGB_TEXT` ([`Options::text_color`]) applies.
    Unset,
    /// An `<a>` wrote `HVOPT_RGB_LINK` into the frame.
    Link,
    /// A `<font color>` wrote this one. Black is `Own(0)` and means black, not "unset" — the
    /// distinction is the whole reason this is not an `Option<u32>`.
    Own(u32),
}

/// One painted line, with what it inherits from its ancestors.
#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub font: u32,
    /// The colour the innermost element that wrote one left in the frame. [`Ink::Link`] rather than
    /// a number because `HVOPT_RGB_LINK` is not known until the page is laid out.
    pub color: Ink,
    pub align: u32,
    /// Continues the row before it instead of starting one. `Hello <b>world</b>` is one row of two
    /// spans; the `<b>` changes the face, not the line.
    pub inline: bool,
    /// An index into [`Document::images`] when this run is a picture rather than a line of text.
    /// Both the row's height and what gets painted come from the image.
    pub image: Option<usize>,
    /// The element this span belongs to. Two uses: a text node belongs to the innermost element it
    /// sits in, and a control belongs to *itself* — which is what lets the focus ring find the
    /// rows of the element [`Document::focus_items`] named.
    pub elem: Option<usize>,
    /// Set when this span is a form control rather than words: the box the painter draws and the
    /// value it draws inside it. `text` still carries what is drawn, so a label and a chosen option
    /// need no second path.
    pub field: Option<Field>,
    pub marquee: Option<usize>,
    /// How many enclosing elements add the indent (`layout & 0x10`). <blockquote><dd>x</dd></blockquote>
    /// is two, so the word sits two `HVOPT_N_INDENT` steps in from the left. It is a *count* and not
    /// a number of pixels because the pixel amount is an option the applet can change after the
    /// page was parsed — the same reason `gap` is a flag.
    pub indent: u8,
    pub inset: u8,
    /// This row opens a block that takes the paragraph gap (`layout & 0x08`): the line is flushed
    /// and the next one starts `HVOPT_N_VSPACE` pixels lower. `<p>`, the headings, `<dl>`, the
    /// lists and `<form>` carry that bit; `<div>`, `<center>`, `<table>`, `<li>` and `</table>`'s
    /// cells do not. `<br>` and `<hr>` carry it in the table but never reach it — see
    /// [`Run::line`].
    pub gap: bool,
    /// The line this empty row takes of its own, if any. `<br>`, `<hr>` and a bare newline inside a
    /// preformatted block are not blocks: the layout loop handles those ids itself and an *empty
    /// element* never reaches the frame push that carries the margin, so none of the three moves
    /// the cursor by `HVOPT_N_VSPACE`.
    pub line: Line,
    /// White space inside is significant (`layout & 0x80`): `<pre>` and `<xmp>`. The parse keeps
    /// the spaces and the newlines, which every other element collapses.
    pub pre: bool,
    /// A rule is drawn under the span (`style & 0x04`): `<u>`, and an `<a>` with an `href`.
    pub underline: bool,
    /// A mark the redraw pass strokes rather than something the layout places: the rule of an
    /// `<hr>` (on the row that follows it) and the square of the first word of an `<li>`.
    pub mark: Option<Mark>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarqueeBehavior {
    Scroll,
    Slide,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marquee {
    pub behavior: MarqueeBehavior,
    /// Attribute 16 (`direction`) compared against `"left"` by `aee_stricmp`. Any other word —
    /// `right`, a typo — means the other way; the *default* is `"left"` and therefore `true`.
    pub left: bool,
    /// Attribute 30 (`loop`). `-1` is forever. `slide` defaults to one pass, the other two to -1.
    pub loops: i32,
    /// Attribute 40 (`scrollamount`), 2 by default: pixels added per timer step.
    pub amount: i32,
    /// Attribute 41 (`scrolldelay`), 36 by default: milliseconds between steps.
    pub delay: u32,
    /// Attribute 7 (`bgcolor`), or `None` for the reference default of 10 = `CLR_SYS_WIN`.
    pub bg: Option<u32>,
}

impl Marquee {
    /// Read one out of an attribute list. `Attrs` is the parser's own `Vec<(String, String)>`.
    fn of(attrs: &[(String, String)]) -> Marquee {
        let attr = |name: &str| {
            attrs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        let behavior = if attr("behavior").eq_ignore_ascii_case("slide") {
            MarqueeBehavior::Slide
        } else if attr("behavior").eq_ignore_ascii_case("alternate") {
            MarqueeBehavior::Alternate
        } else {
            MarqueeBehavior::Scroll
        };
        let default_loops = if behavior == MarqueeBehavior::Slide {
            1
        } else {
            -1
        };
        let number = |value: &str, default: i32| -> i32 {
            let digits: String = value
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
                .collect();
            digits.parse().unwrap_or(default)
        };
        Marquee {
            behavior,
            left: attrs
                .iter()
                .find(|(k, _)| k == "direction")
                .is_none_or(|(_, v)| v.eq_ignore_ascii_case("left")),
            loops: number(attr("loop"), default_loops),
            amount: number(attr("scrollamount"), 2),
            // A zero delay would make `ISHELL_SetTimer` fire on every turn; one millisecond is
            // the closest a host that divides by the delay can get without dividing by zero.
            delay: number(attr("scrolldelay"), 36).max(1) as u32,
            bg: parse_color(attr("bgcolor")),
        }
    }
}

/// The face, colour and alignment the open elements hand down to the text inside them.
#[derive(Debug, Clone, Copy)]
struct Style {
    bold: bool,
    large: bool,
    /// One value, overwritten by whoever writes last: `<a>` writes [`Ink::Link`], `<font color>`
    /// writes [`Ink::Own`]. Nothing accumulates here, so the innermost element always wins.
    color: Ink,
    align: u32,
    indent: u8,
    inset: u8,
    pre: bool,
    underline: bool,
}

/// One form control, as the page draws it.
///
/// The *value* is not here: it is the run's `text`, because that is what a painter draws and what a
/// test can read in one place. What is here is the part that decides the box: its kind, whether a
/// box or a radio is chosen, and how many characters wide a text field was asked to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub kind: FieldKind,
    pub checked: bool,
    /// The `size` (text field) or `cols` (textarea) attribute in characters; 0 when unnamed.
    pub cols: u16,
    /// The `rows` attribute of a `<textarea>` in lines; 0 when unnamed. A one-line control leaves
    /// it 0 — a text field is one cell tall however many rows somebody writes on it.
    pub rows: u16,
}

/// The six controls the viewer draws, plus the two that are just buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Password,
    Checkbox,
    Radio,
    Submit,
    Reset,
    Button,
    Select,
    Textarea,
}

impl FieldKind {
    /// Whether the control holds text the user can type into or walk a caret through.
    pub fn is_editor(self) -> bool {
        matches!(
            self,
            FieldKind::Text | FieldKind::Password | FieldKind::Textarea
        )
    }

    /// Whether the control's text is a *choice* made from a list rather than characters.
    pub fn is_list(self) -> bool {
        matches!(self, FieldKind::Select)
    }

    /// Whether the control carries an on/off state of its own.
    pub fn is_toggle(self) -> bool {
        matches!(self, FieldKind::Checkbox | FieldKind::Radio)
    }
}

/// What the user has changed about one control since the page was parsed.
///
/// `None` in a slot means "nothing changed here": the document's own `value` attribute, its
/// `checked` attribute, or the option it marked `selected`. That distinction matters because a
/// field the user never touched must still follow the page — a `<select>` whose options were
/// re-parsed is not the same thing as one the user picked from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edited {
    /// The current text: what a text field or textarea holds, or the label a closed `<select>`
    /// shows. Kept in the clear even for a password — the masking happens where the box is drawn,
    /// because the same string is what the form posts.
    pub text: Option<String>,
    /// Where the caret sits in [`Edited::text`], counted in characters from the start.
    pub caret: usize,
    /// For a checkbox or a radio: whether it is on now.
    pub checked: Option<bool>,
    /// For a list: which `<option>` is chosen, as an index into [`Document::options_of`].
    ///
    /// The label alone would not do: two options may read the same and post different values, and
    /// matching a chosen label back to an option would have to pick one of them.
    pub choice: Option<usize>,
}

/// Every control the user has touched, keyed by the element's index in [`Document::elements`].
///
/// The document is parsed once and shared through an `Arc`, so an edit cannot live in it. It lives
/// here instead, beside the parse, and [`layout`] folds the two together — which is also what keeps
/// a frame from having to clone a document to show one typed character.
///
/// The key is an element index, and that is safe because indexes are *stable* for a given byte
/// string: `Html::values` is cleared wherever the parse is (`SetData`, `LoadStream`, a new
/// `HVOPT_SZ_TYPE`), exactly like `focus` and `scroll`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Values {
    entries: HashMap<usize, Edited>,
}

impl Values {
    /// The record for one control, if the user has touched it.
    pub fn get(&self, elem: usize) -> Option<&Edited> {
        self.entries.get(&elem)
    }

    /// What this control shows now, or `None` when the document's own value still stands.
    pub fn text(&self, elem: usize) -> Option<&str> {
        self.entries.get(&elem).and_then(|e| e.text.as_deref())
    }

    /// Whether this box is ticked now, or `None` when the document's own attribute still stands.
    pub fn checked(&self, elem: usize) -> Option<bool> {
        self.entries.get(&elem).and_then(|e| e.checked)
    }

    /// The record for one control, made if this is the first edit.
    pub fn edit(&mut self, elem: usize) -> &mut Edited {
        self.entries.entry(elem).or_default()
    }

    /// Start editing from `text` with the caret at the end, unless the control is already being
    /// edited — a key press must not throw away what the last one typed.
    pub fn begin(&mut self, elem: usize, text: &str) -> &mut Edited {
        let entry = self.entries.entry(elem).or_insert_with(|| Edited {
            text: Some(text.to_string()),
            caret: text.chars().count(),
            ..Edited::default()
        });
        if entry.text.is_none() {
            entry.text = Some(text.to_string());
            entry.caret = text.chars().count();
        }
        entry
    }

    /// Tick or untick a box.
    pub fn set_checked(&mut self, elem: usize, on: bool) {
        self.edit(elem).checked = Some(on);
    }

    /// Choose one option: the label the closed list shows is the text, the caret goes to its end.
    pub fn choose(&mut self, elem: usize, index: usize, label: &str) {
        let entry = self.edit(elem);
        entry.text = Some(label.to_string());
        entry.caret = label.chars().count();
        entry.choice = Some(index);
    }

    /// Forget every edit for the elements in `range` — what a `<input type=reset>` does to its form.
    pub fn reset(&mut self, range: std::ops::Range<usize>) {
        self.entries.retain(|elem, _| !range.contains(elem));
    }

    /// Forget everything, which is what a new document means.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every edited control, in element order, for the view state and for tests.
    pub fn entries(&self) -> impl Iterator<Item = (usize, &Edited)> {
        let mut all: Vec<(&usize, &Edited)> = self.entries.iter().collect();
        all.sort_by_key(|(elem, _)| **elem);
        all.into_iter().map(|(elem, edited)| (*elem, edited))
    }
}

/// The numeric `HVOPT_*` words that actually change what [`render`] paints.
///
/// The string and interface options live on [`Html`] instead: `HVOPT_SZ_URL` becomes
/// [`Html::base_url`], `HVOPT_SZ_TYPE` becomes [`Html::doc_type`]. `HVOPT_PTR_IWEB` is still
/// only stored by `SetOption` — a viewer that drove its own `IWeb` transaction would need the
/// load to stop being synchronous, which is a different shape than this host has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// One indent step, in pixels (`HVOPT_N_INDENT`). Added once per enclosing element whose record
    /// has the indent bit — `<blockquote>`, `<dd>`, the lists — and not to the page as a whole: a
    /// document with no such element is laid out from the left edge of its rectangle whatever this
    /// says.
    pub indent: u32,
    /// The paragraph gap, in pixels (`HVOPT_N_VSPACE`). A block whose record has the margin bit
    /// (`<p>`, the headings, `<dl>`, `<ul>`, `<br>`, `<form>`) moves the cursor down by this much
    /// when it starts; every other row is spaced by the face's own cell plus [`ROW_LEADING`].
    pub vspace: u32,
    /// Width reserved on the right for a scrollbar (`HVOPT_N_SBWIDTH`).
    pub sbwidth: u32,
    /// Default text colour (`HVOPT_RGB_TEXT`), in the host's own RGB word.
    pub text_color: u32,
    /// Link colour (`HVOPT_RGB_LINK`).
    pub link_color: u32,
    /// `HVP_SCROLLBAR`: draw the bar when the document does not fit. The column it occupies is
    /// always reserved — `HVOPT_N_SBWIDTH` is subtracted whether the bar is drawn or not, which is
    /// the header's "if the document contents do fit, a scrollbar is not visible, but space for the
    /// scrollbar is still reserved".
    ///
    /// Not a `HVOPT_*`: it comes from `IControl::SetProperties`, so it is filled in beside the
    /// table rather than out of it.
    pub scrollbar: bool,
}

impl Options {
    /// Read the five painter options out of the `HVOpt` table `SetOption` keeps.
    ///
    /// An option nobody set is 0, which is exactly what a zeroed option table on the
    /// handset reads as. Values are clamped by [`render`], so a pack that writes a
    /// nonsense width cannot overflow a coordinate or starve the text of all room.
    pub fn from_table(table: &HashMap<i32, u32>) -> Options {
        let get = |id: i32| table.get(&id).copied().unwrap_or(0);
        Options {
            indent: get(HVOPT_N_INDENT),
            vspace: get(HVOPT_N_VSPACE),
            sbwidth: get(HVOPT_N_SBWIDTH),
            text_color: get(HVOPT_RGB_TEXT),
            link_color: get(HVOPT_RGB_LINK),
            // The property word is not in this table; a caller that has it sets this afterwards.
            scrollbar: false,
        }
    }
}

/// A parsed document: the tags in document order and the lines `Redraw` paints.
#[derive(Debug, Default)]
pub struct Document {
    pub elements: Vec<Element>,
    pub runs: Vec<Run>,
    /// The pictures the document names, in document order. A run reaches one through
    /// [`Run::image`]; the page's own width and height come from the `<img>` attributes, or from
    /// the picture itself once it has been decoded.
    pub images: Vec<Image>,
    pub bg: Option<u32>,
    pub bg_image: Option<usize>,
    /// The `<marquee>` objects, in document order. A run reaches one through [`Run::marquee`].
    pub marquees: Vec<Marquee>,
}

/// One `<img>` the document names.
///
/// The bytes are *not* held here: `Html::images` is the cache, keyed by `src`, because the document
/// is re-parsed on every redraw and a decode belongs to the page rather than to one frame.
#[derive(Debug, Clone, Default)]
pub struct Image {
    /// `src` as it arrived, which is what the cache is keyed by.
    pub src: String,
    /// `alt`, painted when there is no picture to paint.
    pub alt: String,
    /// The `width` attribute in pixels, or 0 when the document named none.
    pub width: u16,
    /// The `height` attribute in pixels, or 0 when the document named none.
    pub height: u16,
    /// `<body background>` rather than `<img src>`. Such an entry is loaded by the shell from a
    /// pack file at layout time, so the `IWeb` fetch that every other `src` gets must not be
    /// started for it — including when a later `<body>` has replaced this one.
    pub background: bool,
}

impl Document {
    /// Parse the subset the viewer documents. Malformed input is read as far as it goes rather than
    /// refused: a pack that ships a truncated string still gets the lines before the break.
    pub fn parse(bytes: &[u8]) -> Document {
        let mut doc = Document::default();
        let mut i = 0;
        // The elements still open, outermost first. A text node belongs to all of them, because the
        // header says `GetElemText` ignores contained *elements* and concatenates "separate sections
        // of text content (including that of sub-elements)" — so a paragraph that wraps one word in
        // `<b>` still answers the whole sentence. The stack is also what a close tag pops back to,
        // which is what keeps ` c` in `<p>a <b>b</b> c</p>` inside the paragraph instead of
        // orphaning it, and what makes `<font color>` and `align` end where their element does.
        let mut open: Vec<usize> = Vec::new();
        // Whether the row being built already has text on it. A block opens a new row; an inline
        // element does not, which is what lets `Hello <b>world</b>` come out as one line.
        let mut flow = false;
        // Whether a `<li>` has been opened whose square has not been claimed yet. The square is
        // drawn from the tag walk beside the first entry the item produces, so it survives the
        // empty rows a block inside the item may open — and dies with `</li>` if nothing ever does.
        let mut bullet = false;
        // Set while inside `<script>`, `<style>`, `<title>`, `<textarea>` or an `<option>`. What
        // those enclose is not body text: painting a script's source puts `var x = 1;` on the
        // screen, and painting an option's label puts every choice in the list on the screen. The
        // bytes still go into the element's own text, so `GetElemText` on a `title` answers the
        // title — the opaque stretch decides what is *painted*, not what is read.
        let mut opaque: Option<String> = None;
        // The reference loop's `v41`: how many frames are open. It is a counter, not `open.len` —
        // objects, void tags and empty elements never reach its `++`. Past `FRAME_LIMIT` this
        // number keeps rising (and falling again), but no frame is written or popped; see the two
        // `capped` gates below and [`Document::frame_len`].
        let mut frames: usize = 0;
        while i < bytes.len() {
            if let Some(tag) = opaque.clone() {
                // Inside an opaque element a `<` is content, not markup — a script may well contain
                // `if (a < b)`. Only the matching close tag ends it, so the search is for that.
                let needle = format!("</{tag}");
                let (end, done) = match find_ci(&bytes[i..], needle.as_bytes()) {
                    Some(at) => (i + at, true),
                    None => (bytes.len(), false),
                };
                let raw = decode_entities(&String::from_utf8_lossy(&bytes[i..end]));
                doc.push_opaque_text(&open, &tag, &raw);
                if !done {
                    break;
                }
                i = end;
                opaque = None;
                continue;
            }
            if bytes[i] != b'<' {
                let end = bytes[i..]
                    .iter()
                    .position(|b| *b == b'<')
                    .map_or(bytes.len(), |n| i + n);
                let raw = String::from_utf8_lossy(&bytes[i..end]);
                doc.push_text(&raw, &open, &mut flow, &mut bullet);
                i = end;
                continue;
            }
            let Some(end) = bytes[i..].iter().position(|b| *b == b'>') else {
                break;
            };
            let inner = String::from_utf8_lossy(&bytes[i + 1..i + end])
                .trim()
                .to_string();
            i += end + 1;
            if inner.starts_with('!') || inner.starts_with('?') {
                continue;
            }
            if let Some(name) = inner.strip_prefix('/') {
                let name = split_name(name);
                if name == "li" {
                    bullet = false;
                }
                let close_capped = frames > FRAME_LIMIT;
                let at = open.iter().rposition(|&e| doc.elements[e].tag == name);
                // `rposition` answers a position *in `open`*, not an element index. They coincide
                // only while every tag has been pushed; a void tag (`<img>`, `<br>`) or a
                // `<marquee>` (whose whole element is consumed at its open tag) shifts every
                // later index by one. Keep the two apart: `at` is for `truncate`, `elem` is for
                // the document. Mixing them made `</p>` after an `<img>` read the `empty` byte of
                // some earlier `<p>` and drop its margin.
                let elem = at.map(|at| open[at]);
                if let Some(at) = at {
                    let popped = open[at..]
                        .iter()
                        .filter(|&&e| {
                            doc.elements
                                .get(e)
                                .is_some_and(|el| pushes_frame(&el.tag, el.empty))
                        })
                        .count();
                    frames = frames.saturating_sub(popped);
                    open.truncate(at);
                }
                if is_block(name) {
                    let empty = elem.is_none_or(|e| doc.elements[e].empty);
                    if !empty && !close_capped {
                        let style = doc.style(&open);
                        let gap = tag_of(name).is_some_and(|tag| tag.layout & TAG_MARGIN != 0);
                        doc.push_space(&style, elem, gap, Line::None, style.pre, None);
                    }
                    if !close_capped {
                        flow = false;
                    }
                }
                continue;
            }
            let (name, attrs) = split_attrs(&inner);
            if name == "marquee" {
                let index = doc.elements.len();
                let text_end = bytes[i..]
                    .iter()
                    .position(|b| *b == b'<')
                    .map_or(bytes.len(), |n| i + n);
                let text = collapse_ws(&decode_entities(&String::from_utf8_lossy(
                    &bytes[i..text_end],
                )))
                .trim()
                .to_string();
                // An unterminated element consumes the rest of the stream, the same answer every
                // other unclosed tag gets here.
                let after = match find_ci(&bytes[i..], b"</marquee") {
                    Some(at) => bytes[i + at..]
                        .iter()
                        .position(|b| *b == b'>')
                        .map_or(bytes.len(), |n| i + at + n + 1),
                    None => bytes.len(),
                };
                i = after;
                doc.elements.push(Element {
                    tag: name.clone(),
                    attrs: attrs.clone(),
                    text: text.clone(),
                    empty: text.is_empty(),
                });
                let at = doc.marquees.len();
                doc.marquees.push(Marquee::of(&attrs));
                doc.runs.push(Run {
                    text,
                    font: g::AEE_FONT_NORMAL,
                    color: Ink::Unset,
                    align: g::IDF_ALIGN_LEFT,
                    inline: false,
                    image: None,
                    elem: Some(index),
                    field: None,
                    marquee: Some(at),
                    indent: 0,
                    inset: 0,
                    gap: false,
                    line: Line::None,
                    pre: false,
                    underline: false,
                    mark: take_bullet(&mut bullet),
                });
                flow = false;
                continue;
            }
            // The style the tag inherits, read *before* the tag itself is pushed: a `<p align>` sets
            // the alignment for what it encloses, not for the blank row it opens.
            let style = doc.style(&open);
            let index = doc.elements.len();
            // Whether this element encloses nothing, which is a byte the reference layout reads: the
            // node stream is `[open, id][…contents…][close, id]`, so an element whose contents are
            // empty is one whose next node is the close tag.
            let empty = bytes[i..].starts_with(b"</");
            // Only elements that actually join `open` can cost a frame: void tags have no close
            // node to balance the counter with. `frame_len` walks `open`, so the two views of the
            // depth have to count the same set.
            let will_frame = !is_void(&name) && pushes_frame(&name, empty);
            let capped = will_frame && frames >= FRAME_LIMIT;
            doc.open_tag(
                &name,
                &attrs,
                index,
                &style,
                &open,
                &mut flow,
                &mut bullet,
                empty,
                capped,
            );
            // Opening one of these starts the opaque stretch above. The element is still recorded:
            // `FindElem("title")` is a reasonable thing for a pack to ask for, and its
            // `GetElemText` should not answer "".
            if matches!(
                name.as_str(),
                "script" | "style" | "title" | "textarea" | "option"
            ) {
                opaque = Some(name.clone());
            }
            doc.elements.push(Element {
                tag: name.clone(),
                attrs,
                text: String::new(),
                empty,
            });
            if !is_void(&name) {
                open.push(index);
                if will_frame {
                    frames += 1;
                }
            }
        }
        doc.resolve_fields();
        doc.trim_row_ends();
        doc
    }

    fn push_space(
        &mut self,
        style: &Style,
        elem: Option<usize>,
        gap: bool,
        line: Line,
        pre: bool,
        mark: Option<Mark>,
    ) {
        self.runs.push(Run {
            text: String::new(),
            font: font(style.bold, style.large),
            color: style.color,
            align: style.align,
            // A space row never joins the row before it: it *is* the break.
            inline: false,
            image: None,
            elem,
            field: None,
            marquee: None,
            indent: style.indent,
            inset: style.inset,
            gap,
            line,
            pre,
            underline: false,
            mark,
        });
    }

    /// One opening tag: the run it paints, if any, and the row state it changes.
    /// The element itself is pushed by the caller, so `index` is where it will land — which is what
    /// lets a control name itself as the element a key press has to find, and what keeps the
    /// `FindElem` order the document's own.
    fn open_tag(
        &mut self,
        name: &str,
        attrs: &[(String, String)],
        index: usize,
        style: &Style,
        open: &[usize],
        flow: &mut bool,
        bullet: &mut bool,
        empty: bool,
        capped: bool,
    ) {
        // The element the text around this tag belongs to, which is also the link a picture or a
        // control inside an `<a>` hangs off.
        let enclosing = open.last().copied();
        let face = font(style.bold, style.large);
        let run = |text: String,
                   inline: bool,
                   field: Option<Field>,
                   elem: Option<usize>,
                   image: Option<usize>,
                   mark: Option<Mark>| Run {
            text,
            font: face,
            color: style.color,
            align: style.align,
            inline,
            image,
            elem,
            field,
            marquee: None,
            indent: style.indent,
            inset: style.inset,
            gap: false,
            line: Line::None,
            pre: style.pre,
            underline: style.underline,
            mark,
        };
        match name {
            "img" => {
                let at = self.images.len();
                self.images.push(Image {
                    src: attr_in(attrs, "src"),
                    alt: attr_in(attrs, "alt"),
                    width: length_in(attrs, "width"),
                    height: length_in(attrs, "height"),
                    background: false,
                });
                self.runs
                    .push(run(String::new(), *flow, None, enclosing, Some(at), None));
                *flow = true;
            }
            "br" => {
                self.push_space(style, enclosing, false, Line::IfEmpty, style.pre, None);
                *flow = false;
            }
            "hr" => {
                let plain = Style {
                    bold: false,
                    large: false,
                    ..*style
                };
                self.push_space(
                    &plain,
                    enclosing,
                    false,
                    Line::Always,
                    style.pre,
                    Some(Mark::Rule),
                );
                *flow = false;
            }
            name if is_block(name) => {
                if !capped && !empty && tag_of(name).is_some_and(|tag| tag.layout & TAG_MARGIN != 0)
                {
                    self.push_space(style, Some(index), true, Line::None, style.pre, None);
                }
                if name == "li" && !empty {
                    *bullet = true;
                }
                if !capped {
                    *flow = false;
                }
            }
            // A form control is a box on the page, and it is the element a key press has to find:
            // `focus_items` names the `<input>` itself, not the block around it.
            "input" => {
                let ty = attr_in(attrs, "type").to_ascii_lowercase();
                let src = attr_in(attrs, "src");
                if ty == "image" && !src.trim().is_empty() {
                    // A submit button drawn as a picture: the picture joins the line like any other
                    // `<img>`, and the element it belongs to is the `<input>` — that is what a press
                    // has to find and what `HVN_SUBMIT` names. The `alt` is what the item shows until
                    // the picture arrives (or instead of it, when it cannot), so an image button with
                    // no `alt` and no picture still reads as the button it is.
                    let at = self.images.len();
                    let alt = if attr_in(attrs, "alt").is_empty() {
                        let value = attr_in(attrs, "value");
                        if value.is_empty() {
                            "Submit".to_string()
                        } else {
                            value
                        }
                    } else {
                        attr_in(attrs, "alt")
                    };
                    self.images.push(Image {
                        src,
                        alt,
                        width: length_in(attrs, "width"),
                        height: length_in(attrs, "height"),
                        background: false,
                    });
                    let mark = take_bullet(bullet);
                    self.runs
                        .push(run(String::new(), *flow, None, Some(index), Some(at), mark));
                    *flow = true;
                } else if let Some(kind) = input_field(attrs) {
                    let field = Field {
                        kind,
                        checked: attr_has(attrs, "checked"),
                        cols: length_in(attrs, "size"),
                        rows: 0,
                    };
                    let text = field_label(kind, attrs);
                    let mark = take_bullet(bullet);
                    self.runs
                        .push(run(text, *flow, Some(field), Some(index), None, mark));
                    *flow = true;
                }
            }
            // A `<select>` paints the one option it has chosen; the options themselves are opaque,
            // so their labels do not become body text — see [`Document::resolve_fields`].
            "select" => {
                let field = Field {
                    kind: FieldKind::Select,
                    checked: false,
                    cols: 0,
                    rows: 0,
                };
                let mark = take_bullet(bullet);
                self.runs.push(run(
                    String::new(),
                    *flow,
                    Some(field),
                    Some(index),
                    None,
                    mark,
                ));
                *flow = true;
            }
            // A textarea's initial value is its own text, so the box starts empty and is filled in
            // by `resolve_fields` once the closing tag has been read.
            "textarea" => {
                let field = Field {
                    kind: FieldKind::Textarea,
                    checked: false,
                    cols: length_in(attrs, "cols"),
                    rows: length_in(attrs, "rows"),
                };
                let mark = take_bullet(bullet);
                self.runs.push(run(
                    String::new(),
                    *flow,
                    Some(field),
                    Some(index),
                    None,
                    mark,
                ));
                *flow = true;
            }
            "body" => {
                if !empty {
                    self.bg = attrs
                        .iter()
                        .find(|(k, _)| k == "bgcolor")
                        .and_then(|(_, v)| parse_color(v));
                    let src = attr_in(attrs, "background");
                    if src.is_empty() {
                        self.bg_image = None;
                    } else {
                        match self.bg_image {
                            // One slot, released and refilled by the next `<body>`.
                            Some(at) => self.images[at].src = src,
                            None => {
                                let at = self.images.len();
                                self.images.push(Image {
                                    src,
                                    background: true,
                                    ..Image::default()
                                });
                                self.bg_image = Some(at);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str, open: &[usize], flow: &mut bool, bullet: &mut bool) {
        // `Tom &amp; Jerry` is one string of fifteen characters, not twenty-four: the element's text
        // and the painted line both have to see the character the reference stands for.
        let text = decode_entities(text);
        for &at in open {
            if let Some(element) = self.elements.get_mut(at) {
                element.text.push_str(&text);
            }
        }
        let style = self.style(open);
        let elem = open.last().copied();
        let mut first = true;
        for line in lines_of(&text) {
            // A preformatted line is kept as written: `<pre>` is the one element whose spaces mean
            // something, and its newlines are the row breaks. The normaliser in the reference engine
            // makes exactly this split — a branch that copies bytes verbatim while the `0x80` bit is
            // inherited, and one that collapses runs of white space when it is not.
            let line: std::borrow::Cow<'_, str> = if style.pre {
                if line.is_empty() {
                    // An empty line inside `<pre>` is a line: the layout loop treats a newline the
                    // way it treats `<br>`, flushing and then advancing by one line height because
                    // the flush found nothing. Dropping it would run the lines above and below
                    // together, which is the one thing `<pre>` exists to prevent.
                    //
                    // It is `Always` and not `IfEmpty` because of how the rows line up: the newline
                    // that ends `a` is the row boundary before this run, so the run itself *is* the
                    // second newline — the one the device flushes nothing for.
                    self.push_space(&style, elem, false, Line::Always, true, None);
                    first = false;
                    *flow = false;
                    continue;
                }
                std::borrow::Cow::Borrowed(line)
            } else {
                if line.trim().is_empty() {
                    continue;
                }
                // HTML collapses a run of white space into one space, and a space at a text node's
                // end belongs to the row it started on: trimming it here would join
                // `Hello <b>world</b>` into one word once the spans sit side by side.
                let line = collapse_ws(line.trim_start());
                if line.is_empty() {
                    continue;
                }
                std::borrow::Cow::Owned(line)
            };
            self.runs.push(Run {
                text: line.into_owned(),
                font: font(style.bold, style.large),
                color: style.color,
                align: style.align,
                // Only the first line of a text node continues the row before it; the rest are
                // lines of their own, which is what a newline in the source means.
                inline: *flow && first,
                image: None,
                elem,
                field: None,
                marquee: None,
                indent: style.indent,
                inset: style.inset,
                gap: false,
                line: Line::None,
                pre: style.pre,
                underline: style.underline,
                // Only the first run with something in it takes the square; `take_bullet` clears
                // the flag, so the rest of the item's rows are ordinary rows.
                mark: take_bullet(bullet),
            });
            first = false;
            *flow = true;
        }
    }

    /// The text of an opaque element, which belongs to that element alone.
    ///
    /// A `<script>`'s source or an `<option>`'s label is a text node like any other as far as the
    /// DOM is concerned, but putting it on the ancestors too would put `var x = 1;` into the text of
    /// `<body>`, which is exactly what a pack asking for the page's text does not want.
    fn push_opaque_text(&mut self, open: &[usize], tag: &str, text: &str) {
        if let Some(at) = open.last() {
            if let Some(element) = self.elements.get_mut(*at) {
                element.text.push_str(text);
            }
        }
        // `GetElemText` "ignores contained elements and concatenates separate sections of text
        // content (including that of sub-elements)", and an `<option>`
        // is a sub-element of its `<select>`: a pack reading the list's text expects its labels.
        // A script's source does not belong to its block in that sense, which is why this is the
        // one opaque element whose text bubbles — and only as far as the list that owns it.
        if tag == "option" {
            if let Some(select) = open
                .iter()
                .rev()
                .find(|&&e| self.elements.get(e).is_some_and(|el| el.tag == "select"))
            {
                let select = *select;
                if let Some(element) = self.elements.get_mut(select) {
                    element.text.push_str(text);
                }
            }
        }
    }

    fn frame_len(&self, open: &[usize]) -> usize {
        let mut depth = 0;
        for (at, &element) in open.iter().enumerate() {
            if !self
                .elements
                .get(element)
                .is_some_and(|e| pushes_frame(&e.tag, e.empty))
            {
                continue;
            }
            if depth == FRAME_LIMIT {
                return at;
            }
            depth += 1;
        }
        open.len()
    }

    /// The face, colour and alignment the elements still open hand down to a run.
    ///
    /// Read from the frame stack rather than kept in counters: that is what makes `<b><b>x</b>y</b>`
    /// stay bold for `y`, and what ends `<p align=center>` at its own close tag instead of centring
    /// the rest of the page. Only the prefix that [`Self::frame_len`] says is still live is read;
    /// the full `open` is still what a text node belongs to for `GetElemText`.
    fn style(&self, open: &[usize]) -> Style {
        let open = &open[..self.frame_len(open)];
        let mut style = Style {
            bold: false,
            large: false,
            color: Ink::Unset,
            align: g::IDF_ALIGN_LEFT,
            indent: 0,
            inset: 0,
            pre: false,
            underline: false,
        };
        for &at in open {
            let Some(element) = self.elements.get(at) else {
                continue;
            };
            match element.tag.as_str() {
                // `<font>` writes two things into the frame of everything it encloses: the `color`
                // attribute (id 12) when it has one, and the `size` attribute (id 43).
                "font" => {
                    if let Some((_, value)) = element.attrs.iter().find(|(k, _)| k == "color") {
                        if let Some(color) = parse_color(value) {
                            style.color = Ink::Own(color);
                        }
                    }
                    // `size` is a number from 1 to 7, and the only thing it changes here is the
                    // large bit: `if (v9 <= 3) v7 &= ~2; else v7 |= 2`. A relative `+2`/`-1` is
                    // counted from 3, which is why `<font size=+1>` is large and `<font size=3>`
                    // is not. The write is a set *or a clear*, so `<big><font size=2>` is small.
                    if let Some((_, value)) = element.attrs.iter().find(|(k, _)| k == "size") {
                        let size = font_size(value);
                        style.large = size > 3;
                    }
                }
                "a" => {
                    style.color = Ink::Link;
                    if element.attrs.iter().any(|(k, _)| k == "href") {
                        style.underline = true;
                    }
                }
                // Bold, large and "this element is centred" all come out of the tag table, which is
                // why `<h1>` is bold and large while `<h4>` is only bold, and why `<center>` centres
                // without an attribute anyone wrote.
                name => {
                    if let Some(tag) = tag_of(name) {
                        style.bold |= tag.style & TAG_BOLD != 0;
                        style.large |= tag.style & TAG_LARGE != 0;
                        style.underline |= tag.style & TAG_UNDERLINE != 0;
                        if tag.style & TAG_CENTRED != 0 {
                            style.align = g::IDF_ALIGN_CENTER;
                        }
                        if tag.layout & TAG_INDENT != 0 {
                            style.indent = style.indent.saturating_add(1);
                            if name == "blockquote" {
                                style.inset = style.inset.saturating_add(1);
                            }
                        }
                        style.pre |= tag.layout & TAG_PRE != 0;
                    }
                }
            }
            if reads_align(&element.tag) {
                if let Some((_, value)) = element.attrs.iter().find(|(k, _)| k == "align") {
                    if value.eq_ignore_ascii_case("center") {
                        style.align = g::IDF_ALIGN_CENTER;
                    } else if value.eq_ignore_ascii_case("right") {
                        style.align = g::IDF_ALIGN_RIGHT;
                    } else if value.eq_ignore_ascii_case("left") {
                        style.align = g::IDF_ALIGN_LEFT;
                    }
                }
            }
        }
        style
    }

    /// Fill in the two controls whose text is not an attribute: the option a `<select>` shows, and
    /// the initial value of a `<textarea>`.
    ///
    /// Both are read after the whole document, because both are written *after* the tag that opens
    /// them — the options follow the select, and the textarea's value is between its tags.
    fn resolve_fields(&mut self) {
        let mut fixes: Vec<(usize, String)> = Vec::new();
        for (at, run) in self.runs.iter().enumerate() {
            let (Some(field), Some(elem)) = (run.field.as_ref(), run.elem) else {
                continue;
            };
            match field.kind {
                FieldKind::Select => fixes.push((at, self.chosen_option(elem))),
                FieldKind::Textarea => {
                    // The value keeps its own line breaks: a textarea is the one control whose box
                    // is more than one line tall, and a value flattened to spaces would be a
                    // different value. HTML drops exactly one newline right after the open tag —
                    // that one is the source's own formatting, not the author's first line.
                    let raw = self.text(elem);
                    let value = raw.strip_prefix('\n').unwrap_or(raw);
                    let value = value.replace("\r\n", "\n").replace('\r', "\n");
                    fixes.push((at, value.trim_end().to_string()));
                }
                _ => {}
            }
        }
        for (at, text) in fixes {
            self.runs[at].text = text;
        }
    }

    /// The label of the option a `<select>` shows: the first `selected` one, or the first one when
    /// none is marked, which is the browser's own rule.
    fn chosen_option(&self, elem: usize) -> String {
        self.chosen_among(elem)
            .map(|at| self.option_label(at))
            .unwrap_or_default()
    }

    /// The `<option>` elements a `<select>` encloses, in document order.
    ///
    /// Tags are kept flat, so the enclosure is "every option after this select up to the next
    /// select" — which is the same rule [`form_submission`] uses for a form's extent, and the same
    /// reason: a document with two lists must not read the second one's choices into the first.
    pub fn options_of(&self, elem: usize) -> Vec<usize> {
        let Some(rest) = self.elements.get(elem + 1..) else {
            return Vec::new();
        };
        rest.iter()
            .position(|e| e.tag == "select")
            .map_or(rest, |n| &rest[..n])
            .iter()
            .enumerate()
            .filter(|(_, e)| e.tag == "option")
            .map(|(at, _)| elem + 1 + at)
            .collect()
    }

    /// What a closed list shows for one `<option>`: its label, and its `value` only when the label
    /// is empty. `<option value=2>two</option>` reads "two" while the form posts "2".
    pub fn option_label(&self, at: usize) -> String {
        let label = self.text(at).trim();
        if label.is_empty() {
            self.attr(at, "value").unwrap_or("").trim().to_string()
        } else {
            label.to_string()
        }
    }

    /// The value one `<option>` posts: its `value` attribute, or its label when it names none.
    pub fn option_value(&self, at: usize) -> String {
        let value = self.attr(at, "value").unwrap_or("");
        if value.is_empty() {
            self.text(at).trim().to_string()
        } else {
            value.to_string()
        }
    }

    /// Which `<option>` a list shows, as an index into [`Document::options_of`]. A list with no
    /// options answers 0.
    pub fn chosen_index(&self, elem: usize) -> usize {
        let options = self.options_of(elem);
        options
            .iter()
            .position(|&at| has_attr(&self.elements[at], "selected"))
            .unwrap_or(0)
    }

    /// The option element a `<select>` shows, if it has any.
    fn chosen_among(&self, elem: usize) -> Option<usize> {
        let options = self.options_of(elem);
        let at = self.chosen_index(elem);
        options.get(at).copied()
    }

    /// Which kind of control an element draws as, if it draws one. An element that paints no box —
    /// prose, a link, an `<img>` — answers `None`.
    pub fn field_kind(&self, elem: usize) -> Option<FieldKind> {
        self.runs
            .iter()
            .find(|run| run.elem == Some(elem) && run.field.is_some())
            .and_then(|run| run.field.map(|field| field.kind))
    }

    /// The value a control holds right now, before the user has touched it: the run text the parse
    /// gave its box. This is where editing starts from, so it is the *clear* value — a password's
    /// bullets are made at paint time.
    pub fn control_text(&self, elem: usize) -> String {
        self.runs
            .iter()
            .find(|run| run.elem == Some(elem) && run.field.is_some())
            .map(|run| run.text.clone())
            .unwrap_or_else(|| self.text(elem).trim().to_string())
    }

    /// The radio buttons in the same group as `elem`: same `name`, inside the same form.
    ///
    /// A radio is a set, not a switch — choosing one clears the others with the same name, which is
    /// the behaviour every browser has and the reason `HVFOCUS_RADIO` exists separately from
    /// `HVFOCUS_CHECKBOX`.
    pub fn radio_group(&self, elem: usize) -> Vec<usize> {
        let name = self.attr(elem, "name").unwrap_or("");
        if name.is_empty() {
            return vec![elem];
        }
        let (lo, hi) = self.form_range(elem);
        (lo..hi.min(self.elements.len()))
            .filter(|&at| {
                let e = &self.elements[at];
                e.tag == "input"
                    && attr_of(e, "type").eq_ignore_ascii_case("radio")
                    && attr_of(e, "name") == name
            })
            .collect()
    }

    /// The elements a form encloses: from just after its own `<form>` tag up to the next one.
    ///
    /// A document with no enclosing form answers the whole document, which is what
    /// `HVN_SUBMIT` falls back to as well.
    /// Every `<form>`'s element stretch, in document order — the ranges [`form_pairs`] reads and
    /// the ones the reference `DoxFormIter` walks when it writes a view state.
    pub fn form_stretches(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(form) = self.elements[at..].iter().position(|e| e.tag == "form") {
            let lo = at + form;
            let hi = self.elements[lo + 1..]
                .iter()
                .position(|e| e.tag == "form")
                .map_or(self.elements.len(), |next| lo + 1 + next);
            out.push((lo, hi));
            at = hi;
        }
        out
    }

    pub fn form_range(&self, elem: usize) -> (usize, usize) {
        let upto = elem.min(self.elements.len().saturating_sub(1));
        match self
            .elements
            .iter()
            .take(upto + 1)
            .rposition(|e| e.tag == "form")
        {
            Some(at) => {
                let lo = at + 1;
                let hi = self.elements[lo..]
                    .iter()
                    .position(|e| e.tag == "form")
                    .map_or(self.elements.len(), |n| lo + n);
                (lo, hi)
            }
            None => (0, self.elements.len()),
        }
    }

    /// Trim the white space that only HTML source has: the space before a close tag that ends a
    /// row. An inline run keeps its trailing space, because the next span follows it on the row.
    fn trim_row_ends(&mut self) {
        for i in 0..self.runs.len() {
            let ends_row = self.runs.get(i + 1).map_or(true, |next| !next.inline);
            // A preformatted row keeps the spaces it ends with: `<pre>` is where white space is
            // content, and the source's own formatting is not the only thing that puts it there.
            if ends_row && !self.runs[i].pre {
                // Only spaces and tabs, which is what the reader folded: `trim_end` would also
                // take a `&nbsp;` that closed the row, and that character is content.
                let keep = self.runs[i].text.trim_end_matches([' ', '\t']).len();
                self.runs[i].text.truncate(keep);
            }
        }
    }

    /// Parse the bytes the way `HVOPT_SZ_TYPE` says they are.
    ///
    /// `text/plain` is the one type the viewer treats differently: nothing in it is markup, so the
    /// bytes are the lines they look like and a `<` is a character. Every other type — the empty one
    /// a pack that never set the option leaves included — goes through the ordinary reader, because
    /// a document that arrived without a declared type is the common case and reading it as HTML is
    /// what a viewer with no type to go on does.
    pub fn parse_as(bytes: &[u8], media_type: &str) -> Document {
        Self::parse_as_in(bytes, media_type, None)
    }

    /// Parse the bytes the way `HVOPT_SZ_TYPE` says they are, in the device's own encoding.
    ///
    /// `IHtmlViewer::SetData`: "The character encoding of the source stream is assumed to be an
    /// 8-bit encoding compatible with the native encoding of the device (ISOLATIN 1, Shift-JIS, or
    /// KSC5601)." `encoding` is that encoding — [`encoding_of`] turns the `AEE_ENC_*` the device
    /// publishes into it — and `None` is a caller with no device to ask, which keeps the bytes as
    /// UTF-8 the way every such caller has always had them.
    ///
    /// The decode happens once, here, rather than inside the reader: the reader walks bytes and
    /// turns each stretch into text with `from_utf8_lossy`, so handing it bytes that are already the
    /// device's text in UTF-8 is the whole of the change and leaves no per-token decision that
    /// could drift from the one made for the next token.
    pub fn parse_as_in(
        bytes: &[u8],
        media_type: &str,
        encoding: Option<&'static encoding_rs::Encoding>,
    ) -> Document {
        let decoded;
        let bytes = match encoding {
            Some(encoding) => {
                decoded = encoding.decode(bytes).0;
                decoded.as_bytes()
            }
            None => bytes,
        };
        if is_plain_text(media_type) {
            Document::plain(bytes)
        } else {
            Document::parse(bytes)
        }
    }

    /// The whole document as text lines, with no tags and every line kept.
    fn plain(bytes: &[u8]) -> Document {
        // A newline at the very end ends the last line; it does not open a blank one after it.
        // Without this a document that ends in `\n` — which is most of them — would measure one
        // line longer than it is.
        let text = decode_entities(&String::from_utf8_lossy(bytes)).replace("\r\n", "\n");
        let text = text.trim_end_matches(['\n', '\r']);
        let mut doc = Document::default();
        for line in lines_of(text) {
            // A blank line stays as an empty run: in plain text it *is* the paragraph break, and
            // dropping it would run two paragraphs together.
            doc.runs.push(Run {
                text: line.trim_end().to_string(),
                font: self::font(false, false),
                color: Ink::Unset,
                align: g::IDF_ALIGN_LEFT,
                // Every line of plain text is a row of its own; nothing there is inline.
                inline: false,
                image: None,
                elem: None,
                field: None,
                marquee: None,
                indent: 0,
                inset: 0,
                gap: false,
                // A blank line is a line: it takes the same height as the lines of words around
                // it, whatever `HVOPT_N_VSPACE` happens to be. The paragraph gap would make it
                // vanish at the default of 0, which for text/plain is the wrong answer — the
                // break is the content.
                line: if line.trim().is_empty() {
                    Line::Always
                } else {
                    Line::None
                },
                pre: true,
                underline: false,
                mark: None,
            });
        }
        doc
    }

    /// `IHtmlViewer::FindElem`: the `index`-th element of `tag`, 0-based, or `None`.
    pub fn find(&self, tag: &str, index: i32) -> Option<usize> {
        let want = usize::try_from(index).ok()?;
        self.elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.tag == tag)
            .nth(want)
            .map(|(at, _)| at)
    }

    /// `IHtmlViewer::GetAttrValue`: the attribute's value, if the element carries one.
    pub fn attr(&self, index: usize, name: &str) -> Option<&str> {
        self.elements
            .get(index)?
            .attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Whether a box is ticked *as the page wrote it* — `checked` with or without a value.
    pub fn is_checked(&self, index: usize) -> bool {
        self.elements
            .get(index)
            .is_some_and(|e| has_attr(e, "checked"))
    }

    /// `IHtmlViewer::GetElemText`: the text the element encloses.
    pub fn text(&self, index: usize) -> &str {
        self.elements.get(index).map_or("", |e| e.text.as_str())
    }

    /// The elements the viewer lets the user land on, in document order: the six kinds the header's
    /// `HVFOCUS_*` enum names. Hidden and plain button inputs are skipped,
    /// because they carry no focus state.
    ///
    /// A `disabled` control is skipped as well. It is inert by definition — it cannot be reached,
    /// typed into, ticked or pressed — and this is the one place that decides all four, so leaving
    /// it in the order would make every key that follows act on a control the page has switched off.
    pub fn focus_items(&self) -> Vec<Focus> {
        let mut out = Vec::new();
        for (at, e) in self.elements.iter().enumerate() {
            if matches!(e.tag.as_str(), "input" | "select" | "textarea") && has_attr(e, "disabled")
            {
                continue;
            }
            let kind = match e.tag.as_str() {
                "a" => {
                    // A link is a target only when it names something to navigate to. This host runs
                    // no script, so a `javascript:` href would become a request the pack cannot
                    // make; the link stays in the document and out of the focus order.
                    let href = attr_of(e, "href").trim_start();
                    if href.is_empty() || href.to_ascii_lowercase().starts_with("javascript:") {
                        continue;
                    }
                    g::HVFOCUS_LINK
                }
                "input" => match attr_of(e, "type").to_ascii_lowercase().as_str() {
                    // `button` is a control that does nothing in this viewer — it fires script, and
                    // there is none. `hidden` was never on the page.
                    "hidden" | "button" => continue,
                    // An image button is a submit button: the focus enum has no member of its own
                    // for it, and `HVFOCUS_SUBMIT` is what it acts as.
                    "submit" | "image" => g::HVFOCUS_SUBMIT,
                    "reset" => g::HVFOCUS_RESET,
                    "checkbox" => g::HVFOCUS_CHECKBOX,
                    "radio" => g::HVFOCUS_RADIO,
                    _ => g::HVFOCUS_EDIT,
                },
                "select" => g::HVFOCUS_LIST,
                "textarea" => g::HVFOCUS_EDIT,
                _ => continue,
            };
            out.push(Focus { elem: at, kind });
        }
        out
    }
}

/// One element the user can land on: where it is in [`Document::elements`] and which `HVFOCUS_*`
/// kind it is.
#[derive(Debug, Clone, Copy)]
pub struct Focus {
    pub elem: usize,
    pub kind: u32,
}

/// An element's attribute, or `""` when it has none.
pub(crate) fn attr_of<'a>(element: &'a Element, name: &str) -> &'a str {
    element
        .attrs
        .iter()
        .find(|(k, _)| k == name)
        .map_or("", |(_, v)| v.as_str())
}

/// Whether the element carries `name` at all. `attr_of` cannot tell `checked` from
/// `checked=""`, and both mean the same thing to a form.
pub(crate) fn has_attr(element: &Element, name: &str) -> bool {
    element.attrs.iter().any(|(k, _)| k == name)
}

/// The same question about a tag that has just been split, before it is an [`Element`].
fn attr_has(attrs: &[(String, String)], name: &str) -> bool {
    attrs.iter().any(|(k, _)| k == name)
}

/// The elements that never have a closing tag, so opening one must not push it on the stack.
///
/// A document that writes `</br>` anyway is read as if it did not: the stack is truncated only when
/// a matching tag is found, so a stray close tag costs nothing.
fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "br" | "img"
            | "input"
            | "hr"
            | "meta"
            | "link"
            | "base"
            | "area"
            | "col"
            | "embed"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// The control an `<input>` is, or `None` when it is not one the viewer draws.
///
/// `hidden` is not drawn at all, and `file` needs an upload there is nothing behind. `image` is
/// also `None` here because it is not a *box*: the caller draws it as a picture row, and without a
/// `src` as a plain submit button — see [`Document::parse`].
fn input_field(attrs: &[(String, String)]) -> Option<FieldKind> {
    Some(match attr_in(attrs, "type").to_ascii_lowercase().as_str() {
        // An image button with a `src` is a picture row, which the caller draws; without one it is
        // a submit button with nothing to show but its label, which is exactly a `Submit` box.
        "image" => {
            if attr_in(attrs, "src").trim().is_empty() {
                FieldKind::Submit
            } else {
                return None;
            }
        }
        "hidden" | "file" => return None,
        "checkbox" => FieldKind::Checkbox,
        "radio" => FieldKind::Radio,
        "submit" => FieldKind::Submit,
        "reset" => FieldKind::Reset,
        "button" => FieldKind::Button,
        "password" => FieldKind::Password,
        _ => FieldKind::Text,
    })
}

/// What a control draws inside itself: a field's value, a button's label, the bullets of a password.
///
/// A password's bullets are *not* made here: they are made where the box is drawn, because the same
/// string is what the form posts and what an edit types into. Masking at parse time would leave the
/// clear text in one place and the masked text in another.
fn field_label(kind: FieldKind, attrs: &[(String, String)]) -> String {
    let value = attr_in(attrs, "value");
    match kind {
        FieldKind::Submit if value.is_empty() => "Submit".to_string(),
        FieldKind::Reset if value.is_empty() => "Reset".to_string(),
        FieldKind::Text
        | FieldKind::Password
        | FieldKind::Submit
        | FieldKind::Reset
        | FieldKind::Button
        | FieldKind::Select
        | FieldKind::Textarea => value,
        // A box's label is the text beside it, not its `value` — which is what a form posts.
        FieldKind::Checkbox | FieldKind::Radio => String::new(),
    }
}

/// The value of one attribute of a tag that has just been split, or `""`.
fn attr_in(attrs: &[(String, String)], name: &str) -> String {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// A length attribute in pixels: `"120"` and `"120px"` are both 120.
///
/// A percentage is 0, which the painter reads as "the picture's own extent" — the alternative would
/// be inventing a container width this reader has no way to know.
fn length_in(attrs: &[(String, String)], name: &str) -> u16 {
    let text = attrs
        .iter()
        .find(|(k, _)| k == name)
        .map_or("", |(_, v)| v.as_str());
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

/// Whether this element is the picture-button form of `<input>`.
fn is_image_input(element: &Element) -> bool {
    element.tag == "input" && attr_of(element, "type").eq_ignore_ascii_case("image")
}

/// The `type` an `<input>` acts as, or `None` when it is a button rather than a field.
///
/// A missing `type` is `text`, which is the header's own default. `submit`, `reset`, `button`,
/// `image` and `file` are not fields: the clicked one is named by the element the caller passed,
/// and this host has no upload to perform.
pub(crate) fn input_kind(element: &Element) -> Option<&'static str> {
    Some(
        match attr_of(element, "type").to_ascii_lowercase().as_str() {
            "checkbox" => "checkbox",
            "radio" => "radio",
            "hidden" => "hidden",
            "password" => "password",
            "submit" | "reset" | "button" | "image" | "file" => return None,
            _ => "text",
        },
    )
}

/// The `HViewJump` a form submit names: `(pszURL, pszMethod, pszData)`.
///
/// The form is the last `<form>` at or before the submit element, and its fields are the ones
/// between its own tag and the next `<form>`: a document with two forms must not post the other
/// one's. `pszData` is every named control in that stretch, in document order, as `name=value`
/// pairs. An unnamed control and a `disabled` one contribute nothing; an unchecked box or radio
/// contributes nothing and a checked one posts its `value` or `on`; a `<select>` posts its chosen
/// `<option>`'s value, or the option's text when it names no value. A form with no `action` falls
/// back to the submit element's own `href`, so a caller still receives a usable URL.
///
/// `values` is what the user has changed: a typed-in field posts what it holds now, a box posts its
/// current tick, and a list posts the option it shows. Without it, everything the page started with
/// is what gets posted — which is exactly right for a form nobody has touched.
///
/// `point` is where the activating press landed, for `<input type=image>`: that control posts
/// `name.x` and `name.y` — the place inside the picture, in pixels from its top-left corner — as
/// well as everything else in the form. A keyboard activation has no point to report and passes
/// `(0, 0)`, which is what the HTML form submission algorithm does.
pub fn form_submission(
    doc: &Document,
    elem: usize,
    values: &Values,
    point: (i16, i16),
) -> (String, String, String) {
    let upto = elem.min(doc.elements.len().saturating_sub(1));
    // Tags are kept in document order rather than in a tree, so the enclosure is the last `<form>`
    // at or before this element and its extent is everything up to the next one.
    let form_at = doc
        .elements
        .iter()
        .take(upto + 1)
        .rposition(|e| e.tag == "form");
    let (action, method) = match form_at.map(|at| &doc.elements[at]) {
        Some(form) => (
            attr_of(form, "action"),
            if attr_of(form, "method").is_empty() {
                "GET".to_string()
            } else {
                attr_of(form, "method").to_ascii_uppercase()
            },
        ),
        None => ("", "GET".to_string()),
    };
    let url = if action.is_empty() {
        doc.attr(elem, "href").unwrap_or("").to_string()
    } else {
        action.to_string()
    };
    let (lo, hi) = doc.form_range(elem);
    // The place only travels when the submit *was* the pressed picture; every other caller — a
    // keyboard activation, a saved view state — has no place, and the image buttons then contribute
    // nothing, exactly as an unpressed button does.
    let pressed = doc
        .elements
        .get(elem)
        .is_some_and(|e| e.tag == "input" && is_image_input(e))
        .then_some((elem, point));
    (
        url,
        method,
        form_pairs(doc, values, lo, hi, pressed).join("&"),
    )
}

/// The `name=value` words one form stretch posts, in document order and already
/// `application/x-www-form-urlencoded`. This is the whole of what a submission gathers and the
/// whole of what a saved view state serializes, so both go through here and cannot drift apart.
///
/// `pressed` names an `<input type=image>` inside the stretch that was actually pressed, with the
/// place in its picture: only that one contributes `name.x`/`name.y`, and an image nobody pressed
/// contributes nothing at all — the same rule the other buttons follow.
pub fn form_pairs(
    doc: &Document,
    values: &Values,
    lo: usize,
    hi: usize,
    pressed: Option<(usize, (i16, i16))>,
) -> Vec<String> {
    let mut fields = Vec::new();
    for (at, e) in doc.elements.iter().enumerate().take(hi).skip(lo) {
        let name = match e.tag.as_str() {
            "input" | "select" | "textarea" => attr_of(e, "name"),
            _ => continue,
        };
        if name.is_empty() || has_attr(e, "disabled") {
            continue;
        }
        let value = match e.tag.as_str() {
            "input" => {
                if is_image_input(e) {
                    let Some((elem, point)) = pressed else {
                        continue;
                    };
                    if at != elem {
                        continue;
                    }
                    let (x, y) = point;
                    fields.push(format!("{}.x={}", form_encode(&name), x));
                    fields.push(format!("{}.y={}", form_encode(&name), y));
                    continue;
                }
                match input_kind(e) {
                    None => continue,
                    Some("checkbox" | "radio") => {
                        // The user's tick wins over the attribute, in both directions: a box the
                        // page ticked and the user unticked contributes nothing.
                        let on = values.checked(at).unwrap_or_else(|| has_attr(e, "checked"));
                        if !on {
                            continue;
                        }
                        let value = attr_of(e, "value");
                        if value.is_empty() {
                            "on".to_string()
                        } else {
                            value.to_string()
                        }
                    }
                    Some(_) => match values.text(at) {
                        Some(text) => text.to_string(),
                        None => attr_of(e, "value").to_string(),
                    },
                }
            }
            // An `<option>` is an element of its own here, so the chosen one is read back from the
            // stretch that follows the select rather than from the select's own text — which holds
            // every option's label.
            "select" => {
                let options = doc.options_of(at);
                let chosen = match values.get(at).and_then(|e| e.choice) {
                    Some(index) => options.get(index).copied(),
                    None => options.get(doc.chosen_index(at)).copied(),
                };
                match chosen {
                    Some(option) => doc.option_value(option),
                    None => e.text.trim().to_string(),
                }
            }
            _ => match values.text(at) {
                Some(text) => text.to_string(),
                None => e.text.trim().to_string(),
            },
        };
        fields.push(format!("{}={}", form_encode(name), form_encode(&value)));
    }
    fields
}

pub fn view_state(doc: &Document, values: &Values, scroll: i32, focus: i32) -> String {
    let mut out = format!("{scroll},{focus}");
    for (lo, hi) in doc.form_stretches() {
        let pairs = form_pairs(doc, values, lo, hi, None);
        if pairs.is_empty() {
            continue;
        }
        out.push('?');
        out.push_str(&pairs.join("&"));
    }
    out
}

/// What [`parse_view_state`] understood from a saved string.
#[derive(Debug, PartialEq, Eq)]
pub enum ViewRestore {
    /// The string was `#name`: scroll the element that answers to that `id` or `name` to the top
    /// of the view, and touch nothing else.
    Anchor(String),
    Restore {
        scroll: i32,
        /// The focus item's position; -1 is the reference "leave the focus alone".
        focus: i32,
        /// Every `name=value` word after the first `?`, decoded. A second `?` starts another
        /// form's stretch on the handset; this host restores the words of all of them by name.
        fields: Vec<(String, String)>,
    },
}

/// Read back one string [`view_state`] wrote — or, in the reference reader's other half, an
/// `#anchor`. Words this host cannot make sense of are dropped rather than guessed at:
/// `SetViewState` returns `void`, so there is no caller to tell.
pub fn parse_view_state(state: &str) -> ViewRestore {
    if let Some(anchor) = state.strip_prefix('#') {
        return ViewRestore::Anchor(anchor.trim().to_string());
    }
    let (head, rest) = match state.split_once('?') {
        Some(pair) => pair,
        None => (state, ""),
    };
    let mut numbers = head.split(',');
    let scroll = numbers
        .next()
        .and_then(|n| n.trim().parse::<i32>().ok())
        .unwrap_or(0);
    let focus = numbers
        .next()
        .and_then(|n| n.trim().parse::<i32>().ok())
        .unwrap_or(-1);
    let fields = rest
        .split(['&', '?'])
        .filter(|word| !word.is_empty())
        .map(|word| match word.split_once('=') {
            Some((name, value)) => (form_decode(name), form_decode(value)),
            None => (form_decode(word), String::new()),
        })
        .collect();
    ViewRestore::Restore {
        scroll,
        focus,
        fields,
    }
}

/// `application/x-www-form-urlencoded` for one name or value. Spaces are `+` and every byte
/// outside the unreserved set is `%XX` (UTF-8 bytes are escaped individually), so a Chinese
/// field value travels as bytes rather than as a half-decoded `String`.
///
/// The view state uses it as well: it is the same problem — a string that has to survive a
/// delimiter — and a second escaping scheme would be a second thing to get wrong.
pub fn form_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The inverse of [`form_encode`]: `+` back to a space and `%XX` back to its byte.
///
/// A `%` that does not start two hex digits is kept as itself, which is what makes a hand-written or
/// truncated state a little wrong rather than a parse failure — the interface returns `void`, so
/// there is nobody to report a failure to.
pub fn form_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Clone, Copy)]
struct Tag {
    name: &'static str,
    style: u8,
    layout: u8,
}

const TAG_BOLD: u8 = 0x01;
const TAG_LARGE: u8 = 0x02;
const TAG_UNDERLINE: u8 = 0x04;
const TAG_CENTRED: u8 = 0x10;
const TAG_BLOCK: u8 = 0x04;
const TAG_MARGIN: u8 = 0x08;
const TAG_INDENT: u8 = 0x10;
const TAG_ALIGN: u8 = 0x20;
const TAG_PRE: u8 = 0x80;

const FRAME_LIMIT: usize = 29;

/// Whether an element pushes a frame. The gate around the open-tag arm of the reference loop is
/// `(layout & 3) == 0 && node[node_length] != 2`: either low bit (`?xml`/`script`/`style`/`title`
/// for `0x01`, the five objects for `0x02`) takes the tag out of the frame path, and so does an
/// element whose next node is its own close tag.
fn pushes_frame(name: &str, empty: bool) -> bool {
    !empty && tag_of(name).is_some_and(|tag| tag.layout & 0x03 == 0)
}

/// The seventy-one records, in table order.
static TAGS: [Tag; 71] = [
    Tag {
        name: "?xml",
        style: 0x00,
        layout: 0x01,
    },
    Tag {
        name: "a",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "address",
        style: 0x00,
        layout: 0x04,
    },
    Tag {
        name: "applet",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "area",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "b",
        style: 0x01,
        layout: 0x00,
    },
    Tag {
        name: "base",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "big",
        style: 0x02,
        layout: 0x00,
    },
    Tag {
        name: "blockquote",
        style: 0x00,
        layout: 0x1c,
    },
    Tag {
        name: "body",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "br",
        style: 0x00,
        layout: 0x0c,
    },
    Tag {
        name: "caption",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "center",
        style: 0x10,
        layout: 0x04,
    },
    Tag {
        name: "cite",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "code",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "dd",
        style: 0x00,
        layout: 0x14,
    },
    Tag {
        name: "dfn",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "dir",
        style: 0x00,
        layout: 0x1c,
    },
    Tag {
        name: "div",
        style: 0x00,
        layout: 0x24,
    },
    Tag {
        name: "dl",
        style: 0x00,
        layout: 0x0c,
    },
    Tag {
        name: "dt",
        style: 0x00,
        layout: 0x04,
    },
    Tag {
        name: "em",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "font",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "form",
        style: 0x00,
        layout: 0x0c,
    },
    Tag {
        name: "h1",
        style: 0x03,
        layout: 0x2c,
    },
    Tag {
        name: "h2",
        style: 0x03,
        layout: 0x2c,
    },
    Tag {
        name: "h3",
        style: 0x03,
        layout: 0x2c,
    },
    Tag {
        name: "h4",
        style: 0x01,
        layout: 0x2c,
    },
    Tag {
        name: "h5",
        style: 0x01,
        layout: 0x2c,
    },
    Tag {
        name: "h6",
        style: 0x01,
        layout: 0x2c,
    },
    Tag {
        name: "head",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "hr",
        style: 0x00,
        layout: 0x4c,
    },
    Tag {
        name: "html",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "i",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "img",
        style: 0x00,
        layout: 0x02,
    },
    Tag {
        name: "input",
        style: 0x00,
        layout: 0x02,
    },
    Tag {
        name: "isindex",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "kbd",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "label",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "li",
        style: 0x00,
        layout: 0x44,
    },
    Tag {
        name: "link",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "map",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "marquee",
        style: 0x00,
        layout: 0x02,
    },
    Tag {
        name: "menu",
        style: 0x00,
        layout: 0x1c,
    },
    Tag {
        name: "meta",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "ol",
        style: 0x00,
        layout: 0x1c,
    },
    Tag {
        name: "option",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "p",
        style: 0x00,
        layout: 0x2c,
    },
    Tag {
        name: "param",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "pre",
        style: 0x00,
        layout: 0x8c,
    },
    Tag {
        name: "samp",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "script",
        style: 0x00,
        layout: 0x01,
    },
    Tag {
        name: "select",
        style: 0x00,
        layout: 0x02,
    },
    Tag {
        name: "small",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "strike",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "strong",
        style: 0x01,
        layout: 0x00,
    },
    Tag {
        name: "style",
        style: 0x00,
        layout: 0x01,
    },
    Tag {
        name: "sub",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "sup",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "table",
        style: 0x00,
        layout: 0x04,
    },
    Tag {
        name: "td",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "textarea",
        style: 0x00,
        layout: 0x82,
    },
    Tag {
        name: "textflow",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "th",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "title",
        style: 0x00,
        layout: 0x01,
    },
    Tag {
        name: "tr",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "tt",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "u",
        style: 0x04,
        layout: 0x00,
    },
    Tag {
        name: "ul",
        style: 0x00,
        layout: 0x1c,
    },
    Tag {
        name: "var",
        style: 0x00,
        layout: 0x00,
    },
    Tag {
        name: "xmp",
        style: 0x00,
        layout: 0x80,
    },
];

fn tag_of(name: &str) -> Option<&'static Tag> {
    TAGS.iter().find(|tag| tag.name == name)
}

/// Whether this tag ends the line it opens and the line it closes — `layout & 4`.
fn is_block(name: &str) -> bool {
    tag_of(name).is_some_and(|tag| tag.layout & TAG_BLOCK != 0)
}

/// Whether this tag's `align` attribute is read at all — `layout & 0x20`. `<td align=center>` is
/// not: the cell is transparent, and only a block that says so gets an alignment of its own.
fn reads_align(name: &str) -> bool {
    tag_of(name).is_some_and(|tag| tag.layout & TAG_ALIGN != 0)
}

/// The face a bold/large flag pair selects, as the viewer documents them: bold is `AEE_FONT_BOLD`,
/// large is `AEE_FONT_LARGE`, and the two together use the bold-italic face the shell publishes for
/// that combination.
fn font(bold: bool, large: bool) -> u32 {
    match (bold, large) {
        (false, false) => g::AEE_FONT_NORMAL,
        (true, false) => g::AEE_FONT_BOLD,
        (false, true) => g::AEE_FONT_LARGE,
        (true, true) => g::AEE_FONT_LARGE,
    }
}

/// Read `#RRGGBB` or `#RGB`, the only colour syntax the viewer documents. `None` for anything else,
/// which leaves the default text colour in place.
fn parse_color(value: &str) -> Option<u32> {
    let value = value.trim();
    let Some(hex) = value.strip_prefix('#') else {
        return named_color(value);
    };
    let n = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(make_rgb(
            ((n >> 16) & 0xff) as u8,
            ((n >> 8) & 0xff) as u8,
            (n & 0xff) as u8,
        )),
        3 => Some(make_rgb(
            (((n >> 8) & 0xf) * 0x11) as u8,
            (((n >> 4) & 0xf) * 0x11) as u8,
            ((n & 0xf) * 0x11) as u8,
        )),
        _ => None,
    }
}

fn named_color(name: &str) -> Option<u32> {
    let (r, g, b) = match name.to_ascii_lowercase().as_str() {
        "aqua" => (0, 255, 255),
        "black" => (0, 0, 0),
        "blue" => (0, 0, 255),
        "fuchsia" => (255, 0, 255),
        "gray" => (128, 128, 128),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "maroon" => (128, 0, 0),
        "navy" => (0, 0, 128),
        "olive" => (128, 128, 0),
        "purple" => (128, 0, 128),
        "red" => (255, 0, 0),
        "silver" => (192, 192, 192),
        "teal" => (0, 128, 128),
        "white" => (255, 255, 255),
        "yellow" => (255, 255, 0),
        _ => return None,
    };
    Some(make_rgb(r, g, b))
}

fn font_size(value: &str) -> u8 {
    let value = value.trim();
    let number = |text: &str| -> i32 {
        let digits: String = text
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().unwrap_or(0)
    };
    let size = match value.as_bytes().first() {
        Some(c) if c.is_ascii_digit() => number(value),
        Some(b'+') => number(&value[1..]) + 3,
        Some(b'-') => 3 - number(&value[1..]),
        _ => 3,
    };
    size.clamp(1, 7) as u8
}

/// Split `inner` into a lowercased tag name and its attributes. A bare word (`br`, `p`) has none.
fn split_attrs(inner: &str) -> (String, Vec<(String, String)>) {
    let inner = inner.trim();
    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let name = inner[..name_end].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let rest = inner[name_end..].trim();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        if start == i {
            break;
        }
        let key = rest[start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                // `href="a&amp;b=1"` names the URL `a&b=1`; an attribute value is text, and the
                // same references apply inside one.
                value = decode_entities(&rest[start..i]);
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                // `href="a&amp;b=1"` names the URL `a&b=1`; an attribute value is text, and the
                // same references apply inside one.
                value = decode_entities(&rest[start..i]);
            }
        }
        attrs.push((key, value));
    }
    (name, attrs)
}

/// Split `name` the way a closing tag arrives: `b` from `/b`.
fn split_name(inner: &str) -> &str {
    let end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    &inner[..end]
}

/// `haystack`'s offset of `needle`, compared without case.
///
/// Used for the close tag of an opaque element: `<script>` may be closed by `</SCRIPT>`, and a
/// document is free to spell either. Non-ASCII bytes never match here, which is what makes this a
/// byte comparison rather than a character one.
fn find_ci(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .find(|&at| haystack[at..at + needle.len()].eq_ignore_ascii_case(needle))
}

/// The codec for an `AEE_ENC_*` device encoding, or `None` for one no document can arrive in.
///
/// These are the values `AEEDeviceInfo::wEncoding` publishes, and the set is exactly what the
/// viewer's own header allows a page to be in: "ISOLATIN 1, Shift-JIS, or KSC5601", plus UTF-8 and
/// EUC-CN, which the same published names. `AEE_ENC_GSM_7_BIT_DEFAULT` and `AEE_ENC_UNICODE` name a wire
/// format rather than an 8-bit page encoding, so they answer `None`: inventing a decode the device
/// never promised is the one answer this host must not give.
///
/// `AEE_ENC_ISOLATIN1` maps to windows-1252 because that is what the label `iso-8859-1` means to
/// every decoder that has implemented it since HTML 4 — the two disagree only in 0x80..0x9F, where
/// a handset showing "Latin-1" shows the windows-1252 characters. `AEE_ENC_OCTET` is one byte, one
/// character and nothing above it, which is the same table.
///
/// Three names share `0xFE` (`KSC5601`, `EUC_KR`, `UNICODE2`); they are one codec, so one arm.
pub(crate) fn encoding_of(aee: u32) -> Option<&'static encoding_rs::Encoding> {
    TextEncoding::stream_codec(aee)
}

/// Whether `media_type` names plain text, parameters ignored: `text/plain; charset=utf-8` does.
fn is_plain_text(media_type: &str) -> bool {
    media_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("text/plain")
}

/// The lines of one text node, with `\n`, `\r` and `\r\n` each breaking exactly once.
///
/// Splitting on both characters at the same time would count the `\r\n` a Windows-saved page is
/// full of as two breaks, which turns every line of a `<pre>` block into a line and a blank one.
fn lines_of(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').flat_map(|line| {
        let line = line.strip_suffix('\r').unwrap_or(line);
        line.split('\r')
    })
}

/// Collapse each run of spaces and tabs into one space, the way HTML treats white space in text.
fn collapse_ws(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch == ' ' || ch == '\t' {
            if !space {
                out.push(' ');
                space = true;
            }
        } else {
            out.push(ch);
            space = false;
        }
    }
    out
}

/// Decode the character references in one piece of text.
///
/// `&amp;` is a bare `&`, `&#8212;` and `&#x2014;` are an em dash, and a name this table does not
/// know is left exactly as it arrived — turning an unknown reference into `?` would lose text the
/// caller can still make sense of, and the pack's own source is the only place the intent exists.
fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        // A reference ends at the first `;`, and only a short one is a reference at all: `&` in
        // prose is followed by the rest of the sentence, `;` and all.
        let decoded = after
            .find(';')
            .filter(|&end| end <= 32)
            .and_then(|end| decode_reference(&after[1..end]).map(|ch| (ch, end + 1)));
        match decoded {
            Some((ch, len)) => {
                out.push(ch);
                rest = &after[len..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// One reference's body — what sits between `&` and `;` — as the character it names.
fn decode_reference(body: &str) -> Option<char> {
    if let Some(digits) = body.strip_prefix('#') {
        let (digits, radix) = match digits.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16),
            None => (digits, 10),
        };
        let n = u32::from_str_radix(digits, radix).ok()?;
        // `&#0;` is U+FFFD by the HTML rules, and a NUL would end the C string the guest reads.
        return Some(
            char::from_u32(n)
                .filter(|&c| c != '\0')
                .unwrap_or('\u{fffd}'),
        );
    }
    Some(match body {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        // Kept as the no-break space it is. The reader folds lines now, so a `10&nbsp;kg` that
        // became a plain space could be broken between the number and the unit — which is the one
        // thing the reference exists to prevent. No face here has a glyph for U+00A0, so
        // `width_of` measures it as a space and `paint_text` draws it as one; what it is not is a
        // place the folder may cut.
        "nbsp" => '\u{a0}',
        "copy" => '\u{a9}',
        "reg" => '\u{ae}',
        "trade" => '\u{2122}',
        "hellip" => '\u{2026}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201c}',
        "rdquo" => '\u{201d}',
        "laquo" => '\u{ab}',
        "raquo" => '\u{bb}',
        "bull" => '\u{2022}',
        "middot" => '\u{b7}',
        "sect" => '\u{a7}',
        "para" => '\u{b6}',
        "deg" => '\u{b0}',
        "plusmn" => '\u{b1}',
        "times" => '\u{d7}',
        "divide" => '\u{f7}',
        "frac12" => '\u{bd}',
        "frac14" => '\u{bc}',
        "frac34" => '\u{be}',
        "sup2" => '\u{b2}',
        "sup3" => '\u{b3}',
        "micro" => '\u{b5}',
        "cent" => '\u{a2}',
        "pound" => '\u{a3}',
        "yen" => '\u{a5}',
        "euro" => '\u{20ac}',
        "permil" => '\u{2030}',
        "dagger" => '\u{2020}',
        "shy" => '\u{ad}',
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarqueeBox {
    /// Index into [`Document::marquees`].
    pub at: usize,
    /// The width of the scrolling string, as the face will draw it. `w` on the span is the box.
    pub text_w: i16,
    pub pad: i16,
}

/// One line's geometry and colour, worked out without touching guest memory or a display.
///
/// Keeping this separate is what makes the `HVOPT_*` words testable: a pack's indent, leading and
/// scrollbar width are decisions about a rectangle, not about a framebuffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub x: i16,
    pub y: i16,
    /// The top of the span's own drawing, which is the row's top for text and the top of the
    /// control's box for a field — a small box is centred on the line it sits in.
    pub top: i16,
    /// The width the row had to lay out in: the rect less the indent and the scrollbar. Kept on
    /// every span so a caller can see what the alignment was decided against.
    pub dx: i16,
    /// The width this span needs. Painting uses this rather than `dx`, because a row of several
    /// spans is placed span by span and each one ends where its text does.
    pub w: i16,
    /// The height of the span's own drawing, which is its face's cell for text and the box for a
    /// field. `row` is the row it sits in, which is the taller of the two plus the leading.
    pub h: i16,
    pub row: i16,
    /// 0 means "leave the display's own user-text colour alone".
    pub color: u32,
    pub text: String,
    pub font: u32,
    pub align: u32,
    /// Drawn with a rule under it: `<u>`, or an `<a href>`. The display draws the rule on the
    /// baseline of the cell it is painting, so it costs the line no height.
    pub underline: bool,
    /// Set when this row is a picture rather than a line of text: an index into
    /// [`Document::images`]. `text` is then empty and `w` is the picture's width.
    pub image: Option<usize>,
    /// The element this span belongs to, carried through for the focus ring.
    pub elem: Option<usize>,
    /// Set when this span is a form control: the painter draws a box for it, not just words.
    pub field: Option<Field>,
    /// Set when this span is a `<marquee>`: the painter fills its own coloured rectangle and
    /// draws the words clipped inside it, at an offset the clock decides.
    pub marquee: Option<MarqueeBox>,
    /// Where the caret sits inside an editor, in pixels from the span's own top-left corner:
    /// `(x, y)`, both already carrying the box's padding. A textarea's caret is the one that needs
    /// the second number — everywhere else it is a single line and `y` is the padding. Only a
    /// control that has been typed into carries one, and only a focused control draws it.
    pub caret: Option<(i16, i16)>,
    /// Which line of a multi-line value the box starts at. Zero for everything but a `<textarea>`
    /// whose value is taller than the box, where it is the first line the box still shows — the
    /// window that follows the caret.
    pub first_row: i16,
    /// A mark the redraw strokes instead of words: the rule of an `<hr>` and the square of an
    /// `<li>`. A mark is a span of its own — it has no text, only a rectangle — because neither is
    /// laid out on the line it decorates: the rule hangs eight pixels above the *next* row, and the
    /// square sits five pixels to the left of the row it belongs to, outside the column.
    pub mark: Option<Mark>,
}

/// How many lines a control's box shows: its `rows` attribute, or the browser's two, capped at
/// what a page can sensibly ask for.
fn rows_of(field: &Field) -> i16 {
    let rows = if field.rows == 0 {
        FIELD_ROWS
    } else {
        field.rows
    };
    i16::try_from(rows.min(FIELD_ROWS_MAX)).unwrap_or(FIELD_ROWS_MAX as i16)
}

/// The width `text` needs in `face`, in pixels.
///
/// A no-break space is measured as the space it looks like: no face here has a glyph for U+00A0,
/// and [`paint_text`] substitutes the plain space when it draws. Measuring and drawing have to say
/// the same number, or a line is folded at a different place from where it was measured and the
/// span ends short of its own text.
///
/// The face is passed in rather than looked up from the designator: the painter draws with whatever
/// the display has installed under that designator, and a layout that answered the question itself
/// would fold the line where the *built-in* grid ends rather than where the drawn face does.
fn width_of(face: Option<&Face>, text: &str) -> i16 {
    let shown: std::borrow::Cow<'_, str> = if text.contains('\u{a0}') {
        std::borrow::Cow::Owned(text.replace('\u{a0}', " "))
    } else {
        std::borrow::Cow::Borrowed(text)
    };
    let width = match face {
        Some(face) => face.measure_text(&shown, TextEncoding::Unicode),
        None => shown.encode_utf16().count() as u32 * 8,
    };
    i16::try_from(width).unwrap_or(i16::MAX)
}

/// The height of the face's cell, which is what one line of text occupies.
fn cell_of(face: Option<&Face>) -> i16 {
    face.map(|f| (f.ascent() + f.descent()) as i16)
        .unwrap_or(10)
}

/// The height one line of `font` takes: the face's own cell plus the leading every row of words
/// gets. This is the number a `<br>`, an `<hr>` or a blank line in `<pre>` moves the cursor by, and
/// it is exactly the height a row of one word in that face would have had — the reference engine reads
/// it out of the same four-word per-face table the layout uses (`a1[4 * (frame & 3) + 195]`).
fn line_height(faces: Faces<'_>, font: u32) -> i16 {
    cell_of(faces.face(font).as_ref()).saturating_add(ROW_LEADING)
}

/// How far below a segment's top edge the line's baseline sits, for words in `face`.
///
/// [`cell_of`] is this plus the descenders, and the 8/2 split of its fallback is the same split a
/// face without metrics gets here — the two have to agree or a line of that face would measure one
/// height and place itself as if it were another.
fn ascent_of(face: Option<&Face>) -> i16 {
    face.map(|f| f.ascent() as i16).unwrap_or(8)
}

const PICTURE_DESCENT: i16 = 2;

/// The ascent of a picture `h` pixels tall — all of it but the descender space.
fn picture_ascent(h: i16) -> i16 {
    h.saturating_sub(PICTURE_DESCENT).max(0)
}

/// The ascent of a form control's box: how far its top edge sits above the baseline it shares with
/// the words beside it.
fn field_ascent(face: Option<&Face>, field: &Field, h: i16) -> i16 {
    match field.kind {
        // A tick box draws no text of its own, so its bottom edge is what sits on the baseline.
        FieldKind::Checkbox | FieldKind::Radio => h,
        // Every other control draws one line of text inside a padded frame; it is that line's
        // baseline that belongs on the row's, so the frame's top is the padding plus the ascent.
        _ => FIELD_PAD.saturating_add(ascent_of(face)).min(h),
    }
}

fn picture_size(
    doc: &Document,
    pictures: &Pictures,
    index: usize,
    face: Option<&Face>,
) -> (i16, i16) {
    let image = &doc.images[index];
    let picture = pictures.get(index).and_then(|p| p.as_ref());
    let own_width = picture.map_or(0, |p| u16::try_from(p.width).unwrap_or(u16::MAX));
    let own_height = picture.map_or(0, |p| u16::try_from(p.height).unwrap_or(u16::MAX));
    if own_width != 0 && own_height != 0 {
        let (named_w, named_h) = (image.width, image.height);
        if named_w != 0 && named_h != 0 && named_w <= own_width && named_h <= own_height {
            return (squash(named_w), squash(named_h));
        }
        return (squash(own_width), squash(own_height));
    }
    let h = match (image.height, own_height) {
        (named, _) if named != 0 => squash(named),
        (0, own) if own != 0 => squash(own),
        _ => cell_of(face),
    };
    let w = match (image.width, own_width) {
        (named, _) if named != 0 => squash(named),
        (0, own) if own != 0 => squash(own),
        _ => width_of(face, &image.alt),
    };
    (w, h)
}

/// A `u16` length as the `i16` a layout works in: a bitmap wider than the type can hold becomes the
/// largest coordinate rather than a negative one.
fn squash(v: u16) -> i16 {
    i16::try_from(v).unwrap_or(i16::MAX)
}

/// Inner padding of a control's box, on each side.
const FIELD_PAD: i16 = 2;

/// How many characters wide a text field is when the document names no `size`. Eight is the
/// browser's own default for a one-line field, and a handset screen shows it whole.
const FIELD_COLS: u16 = 8;

/// How many lines a `<textarea>` is tall when the document names no `rows`. Two is the browser's
/// own default, and it is what makes the difference between a textarea and a text field visible at
/// all: a one-line box for a control that accepts newlines looks like a bug.
const FIELD_ROWS: u16 = 2;

/// The tallest a `<textarea>` may be, whatever `rows` says. A page that writes `rows=2000` is not
/// asking for a box taller than any screen — it is a page nobody looked at.
const FIELD_ROWS_MAX: u16 = 20;

/// A control's box: `(width, height)` in pixels, from the run's face and what the document asked
/// for. The value it draws is clipped into the box by the painter.
fn box_of(face: Option<&Face>, text: &str, field: &Field) -> (i16, i16) {
    let cell = cell_of(face);
    // A column is as wide as the face's zero, because that is what the value inside the box is
    // drawn with — the same face the box is measured against.
    let advance = face.map_or(6, |f| {
        f.advance_of_char('0', TextEncoding::Unicode).max(1) as i16
    });
    match field.kind {
        // A box and a gap, so the label that follows it does not touch the frame.
        FieldKind::Checkbox | FieldKind::Radio => {
            let side = (cell - 2).clamp(6, 12);
            (side + 3, side)
        }
        FieldKind::Submit | FieldKind::Reset | FieldKind::Button => (
            width_of(face, text).saturating_add(FIELD_PAD * 4),
            cell.saturating_add(FIELD_PAD * 2),
        ),
        // The chosen option, plus room for the mark that says there are more of them.
        FieldKind::Select => (
            width_of(face, text)
                .saturating_add(advance * 3)
                .max(advance * 4)
                .saturating_add(FIELD_PAD * 2),
            cell.saturating_add(FIELD_PAD * 2),
        ),
        // A textarea is the one control whose box is more than one line tall, and `rows` is what
        // says how many — two when the document does not say, which is the browser's own default.
        FieldKind::Textarea => {
            let cols = if field.cols == 0 {
                FIELD_COLS
            } else {
                field.cols.min(120)
            };
            (
                advance
                    .saturating_mul(cols as i16)
                    .saturating_add(FIELD_PAD * 2),
                cell.saturating_mul(rows_of(field))
                    .saturating_add(FIELD_PAD * 2),
            )
        }
        FieldKind::Text | FieldKind::Password => {
            let cols = if field.cols == 0 {
                FIELD_COLS
            } else {
                field.cols.min(120)
            };
            (
                advance
                    .saturating_mul(cols as i16)
                    .saturating_add(FIELD_PAD * 2),
                cell.saturating_add(FIELD_PAD * 2),
            )
        }
    }
}

/// The decoded pictures of one document, positionally matching [`Document::images`]. A `None` is a
/// `src` that could not be had or decoded, which the painter shows as the `alt`.
///
/// It is a slice beside the document rather than a field in it because the document is cached:
/// parsing is the expensive half and does not change between frames, while a picture becomes
/// available part-way through a page's life.
pub type Pictures = [Option<std::sync::Arc<crate::resource::Bitmap>>];

/// One span as the frame will paint it: the document's run folded together with the user's edits.
///
/// The document is shared and immutable, so this is where "what the page says" and "what the user
/// did" meet. A run the user never touched comes through as it was parsed; an editor comes through
/// with what it holds now, or covered in bullets if it is a password; a box comes through with its
/// current tick.
struct Piece {
    text: String,
    field: Option<Field>,
    /// Pixels from the span's top-left corner to the caret, for an editor the user has typed into.
    caret: Option<(i16, i16)>,
    /// The line of the value the box starts at, when the value is taller than the box.
    first: i16,
    /// The width and height of a picture span, in pixels, resolved by [`layout`] from the
    /// `<img width/height>` the document named and the bitmap if one arrived. `None` for words.
    image: Option<(i16, i16)>,
}

/// `run` as the next frame paints it, with `values` folded in.
fn piece_of(faces: Faces<'_>, run: &Run, values: Option<&Values>) -> Piece {
    let Some(field) = run.field else {
        return Piece {
            text: run.text.clone(),
            field: None,
            caret: None,
            first: 0,
            image: None,
        };
    };
    let mut field = field;
    let mut text = run.text.clone();
    // The caret is kept as the line it is on and the pixels into that line, until the window is
    // known: which line is on screen decides where on the box it is drawn.
    let mut caret = None;
    let mut caret_row = 0i16;
    if let (Some(values), Some(elem)) = (values, run.elem) {
        if let Some(edited) = values.get(elem) {
            if let Some(now) = &edited.text {
                text = now.clone();
            }
            if let Some(on) = edited.checked {
                field.checked = on;
            }
            if field.kind.is_editor() {
                // The caret is measured in the face the box draws with, so this is the same
                // measurement `DrawText` makes when it advances over the characters before it.
                // A textarea's value has line breaks in it, and the caret then has a row as well
                // as a column: the row is how many breaks come before it, the column is what it
                // would measure if that row were a line of its own.
                let face = faces.face(run.font);
                let upto: String = text.chars().take(edited.caret).collect();
                let column = match upto.rfind('\n') {
                    Some(at) => {
                        caret_row = upto[..at].matches('\n').count() as i16 + 1;
                        &upto[at + 1..]
                    }
                    None => upto.as_str(),
                };
                caret = Some(width_of(face.as_ref(), column).saturating_add(FIELD_PAD));
            }
        }
    }
    // A password is masked here and nowhere else: the stored text is what the form posts, and a
    // half-typed one has to be as unreadable as a whole one.
    if field.kind == FieldKind::Password {
        text = "*".repeat(text.chars().count());
    }
    // The window a `<textarea>` shows: when the value is taller than the box, the box starts at
    // whichever line keeps the caret in view — the smallest move that does it, and never past the
    // last line. Nothing but the caret moves it, so it is derived rather than stored: two places
    // that both "know" a scroll offset disagree the moment one of them is forgotten.
    let mut first = 0i16;
    if field.kind == FieldKind::Textarea {
        let rows = rows_of(&field);
        let lines = text.matches('\n').count() as i16 + 1;
        if lines > rows {
            first = (caret_row - rows + 1).clamp(0, lines - rows);
        }
    }
    Piece {
        text,
        field: Some(field),
        caret: caret.map(|x| {
            let y = caret_row
                .saturating_sub(first)
                .saturating_mul(cell_of(faces.face(run.font).as_ref()))
                .saturating_add(FIELD_PAD);
            (x, y)
        }),
        first,
        image: None,
    }
}

/// One span as it lands on one line of a folded row: which piece it came from, which bytes of that
/// piece, and how wide it measures. A piece of text split across two lines is two segments that
/// name the same piece and different byte ranges.
struct Segment {
    piece: usize,
    text: std::ops::Range<usize>,
    w: i16,
    h: i16,
    ascent: i16,
}

/// The characters that occupy a full cell and that a line may be broken between.
///
/// The blocks are the ideographic ones — Han, the kana, Hangul, the CJK punctuation and fullwidth
/// forms, and the compatibility ideographs. A line-break rule that only knew about spaces would
/// never break a Chinese paragraph at all, which on a handset is the common case.
fn is_wide(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115f
            | 0x2e80..=0x303e
            | 0x3041..=0x33ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xa000..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe30..=0xfe4f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
    )
}

/// How many bytes of `text` fit in `avail`, cut at a character boundary.
///
/// Measured through [`width_of`] one prefix at a time rather than by summing advances: the faces
/// this project installs subtract the trailing leading of the last character, so a sum of advances
/// and a direct measurement of the same run disagree by a pixel — and a fold decided by the first
/// would draw a span that overflows the second.
fn fit_prefix(text: &str, face: Option<&Face>, avail: i16) -> usize {
    let mut end = 0;
    for (at, ch) in text.char_indices() {
        let next = at + ch.len_utf8();
        if width_of(face, &text[..next]) > avail {
            break;
        }
        end = next;
    }
    end
}

fn break_at(text: &str, face: Option<&Face>, avail: i16) -> Option<(usize, i16)> {
    let fit = fit_prefix(text, face, avail);
    if fit == 0 {
        return None;
    }
    if text[fit..].starts_with(' ') {
        return Some((fit, width_of(face, &text[..fit])));
    }
    if let Some(at) = text[..fit].rfind(' ') {
        if at > 0 {
            return Some((at, width_of(face, &text[..at])));
        }
    }
    let before = text[..fit].chars().next_back();
    let after = text[fit..].chars().next();
    if before.is_some_and(is_wide) || after.is_some_and(is_wide) {
        return Some((fit, width_of(face, &text[..fit])));
    }
    None
}

fn flush(
    faces: Faces<'_>,
    row: &[Run],
    pieces: &[Piece],
    line: &mut Vec<Segment>,
    lines: &mut Vec<Vec<Segment>>,
    used: &mut i16,
) {
    if line.is_empty() {
        return;
    }
    if let Some(last) = line.last_mut() {
        // Only text is trimmed, and only text can vanish here: a control with no label is a box
        // with an empty string in it, and a picture is a box whose string is always empty — the
        // box is the whole reason the row exists in either case.
        if pieces[last.piece].field.is_none()
            && pieces[last.piece].image.is_none()
            && row[last.piece].marquee.is_none()
        {
            let kept = {
                let text = &pieces[last.piece].text[last.text.clone()];
                text.trim_end_matches(' ').len()
            };
            let before = last.text.len();
            last.text.end = last.text.start + kept;
            if kept != before {
                last.w = width_of(
                    faces.face(row[last.piece].font).as_ref(),
                    &pieces[last.piece].text[last.text.clone()],
                );
            }
            if last.text.is_empty() {
                line.pop();
            }
        }
    }
    if !line.is_empty() {
        lines.push(std::mem::take(line));
    }
    *used = 0;
}

/// The spans of one row, folded into the lines they occupy at `dx` pixels wide.
///
/// A row is what the reader called one line of the document — a paragraph, or a `<br>`-less run of
/// inline markup. Whether it is *one* line on screen is a question about the column, not about the
/// document, so it is answered here and nowhere else: at a wide rect this returns the row whole, and
/// at a narrow one it returns as many lines as the text needs.
///
/// A control's box is never split. It is one item, so it moves to the next line whole, and a box
/// wider than the whole column stays where it is and overflows — clipping a text field into two
/// halves would be a different widget.
fn fold(faces: Faces<'_>, row: &[Run], pieces: &[Piece], dx: i16, pre: bool) -> Vec<Vec<Segment>> {
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut line: Vec<Segment> = Vec::new();
    let mut used = 0i16;
    for (index, piece) in pieces.iter().enumerate() {
        let face = faces.face(row[index].font);
        let face = face.as_ref();
        if row[index].marquee.is_some() {
            let cell = cell_of(face);
            let pad = (28 * i32::from(cell) / 100) as i16;
            let w = dx;
            let h = cell.saturating_add(pad.saturating_mul(2));
            if !line.is_empty() {
                // The box owns its line: it is as wide as the column, so anything already there
                // ends first. This is the same unbreakable-item rule a picture follows.
                flush(faces, row, pieces, &mut line, &mut lines, &mut used);
            }
            line.push(Segment {
                piece: index,
                text: 0..piece.text.len(),
                w,
                h,
                ascent: pad.saturating_add(ascent_of(face)),
            });
            used = used.saturating_add(w);
            continue;
        }
        if let Some((w, h)) = piece.image {
            if !line.is_empty() && used.saturating_add(w) > dx {
                flush(faces, row, pieces, &mut line, &mut lines, &mut used);
            }
            line.push(Segment {
                piece: index,
                text: 0..0,
                w,
                h,
                ascent: picture_ascent(h),
            });
            used = used.saturating_add(w);
            continue;
        }
        if let Some(field) = &piece.field {
            let (w, h) = box_of(face, &piece.text, field);
            if !line.is_empty() && used.saturating_add(w) > dx {
                flush(faces, row, pieces, &mut line, &mut lines, &mut used);
            }
            line.push(Segment {
                piece: index,
                text: 0..piece.text.len(),
                w,
                h,
                ascent: field_ascent(face, field, h),
            });
            used = used.saturating_add(w);
            continue;
        }
        let cell = cell_of(face);
        let rise = ascent_of(face);
        let mut rest = piece.text.as_str();
        while !rest.is_empty() {
            // Leading spaces are the gap the previous span left, not part of this one: they are
            // dropped when a line begins, and kept when the line already has ink on it. Inside a
            // preformatted row they are part of the text — that is the whole point of `<pre>` — so
            // a folded continuation line starts with the spaces it was written with.
            if line.is_empty() && !pre {
                rest = rest.trim_start_matches([' ', '\t']);
                if rest.is_empty() {
                    break;
                }
            }
            let base = piece.text.len() - rest.len();
            let avail = dx.saturating_sub(used);
            let full = width_of(face, rest);
            if full <= avail {
                line.push(Segment {
                    piece: index,
                    text: base..piece.text.len(),
                    w: full,
                    h: cell,
                    ascent: rise,
                });
                used = used.saturating_add(full);
                break;
            }
            match break_at(rest, face, avail) {
                Some((take, w)) => {
                    line.push(Segment {
                        piece: index,
                        text: base..base + take,
                        w,
                        h: cell,
                        ascent: rise,
                    });
                    used = used.saturating_add(w);
                    flush(faces, row, pieces, &mut line, &mut lines, &mut used);
                    rest = rest[take..].trim_start_matches([' ', '\t']);
                }
                // Nowhere to cut and nothing else on the line: the word keeps its width and
                // overflows, which is what the reference engine does with an item that fits nowhere.
                // Only the word overflows — the rest of the text still starts a line of its own.
                None if line.is_empty() => {
                    let end = rest.find(' ').unwrap_or(rest.len());
                    let w = width_of(face, &rest[..end]);
                    line.push(Segment {
                        piece: index,
                        text: base..base + end,
                        w,
                        h: cell,
                        ascent: rise,
                    });
                    used = used.saturating_add(w);
                    flush(faces, row, pieces, &mut line, &mut lines, &mut used);
                    rest = rest[end..].trim_start_matches([' ', '\t']);
                }
                // Nowhere to cut, but the line has ink on it: the word may still fit a whole line.
                None => flush(faces, row, pieces, &mut line, &mut lines, &mut used),
            }
        }
    }
    flush(faces, row, pieces, &mut line, &mut lines, &mut used);
    lines
}

pub fn content_width(rect: Rect, opts: &Options) -> i16 {
    let span = rect.dx.max(0) as u32;
    let scrollbar = opts.sbwidth.min(span) as i16;
    rect.dx.saturating_sub(scrollbar)
}

/// Lay the document out inside `rect`. Rows that do not fit are dropped from the end.
pub fn layout(
    doc: &Document,
    rect: Rect,
    opts: &Options,
    pictures: &Pictures,
    values: Option<&Values>,
    scroll: i16,
    faces: Faces<'_>,
) -> Vec<Placed> {
    // `HVOPT_N_INDENT` is one indent step, `HVOPT_N_VSPACE` one paragraph gap, and `HVOPT_N_SBWIDTH`
    // the bar's column. All three are guest-supplied pixels, clamped to the rect before any
    // coordinate arithmetic: a pack writing 0xffffffff must not produce a negative width or walk
    // `x` out of the canvas.
    let span = rect.dx.max(0) as u32;
    let step = opts.indent.min(span) as i16;
    let vspace = opts.vspace.min(span) as i16;
    let dx = content_width(rect, opts);
    if dx <= 0 {
        return Vec::new();
    }
    let leading = ROW_LEADING;
    let top = rect.y;
    let bottom = rect.y.saturating_add(rect.dy);
    // The page starts where the document does, not where the rect does: a scrolled view is the same
    // layout with its origin moved up. Rows that end above the top edge are walked over — their
    // height still counts, so the rows below them land where they would unscrolled.
    let mut y = rect.y.saturating_sub(scroll.max(0));
    let mut out = Vec::new();
    // The `<hr>` rules in `out`, by index. Each is placed where the reference redraw would put it
    // only once the line *after* it has been laid out — see the pass at the bottom of this function.
    let mut rules: Vec<usize> = Vec::new();
    let mut at = 0;
    let mut ink = false;
    // Where the last line the cursor *placed* ended. A flush moves it; a margin does not, which is
    // how two margins at the same place come to one gap instead of two.
    let mut flushed = y;
    'rows: while at < doc.runs.len() {
        // A row is a run plus every run after it that continues it. `inline` is set by the reader,
        // which is the only place that knows whether a `<b>` started a line or decorated one.
        let mut end = at + 1;
        while end < doc.runs.len() && doc.runs[end].inline {
            end += 1;
        }
        let row = &doc.runs[at..end];
        at = end;
        // What each span shows *now*: the parse, with anything the user has typed, ticked or
        // chosen folded in. Everything below measures this rather than the run, or a box would be
        // as wide as the value the page started with while drawing a longer one.
        let mut pieces: Vec<Piece> = row.iter().map(|r| piece_of(faces, r, values)).collect();
        // A picture's size is not in the run — it is the `<img width/height>` the document named,
        // or the bitmap if one has arrived — so it is resolved here, where both are in hand.
        for (piece, run) in pieces.iter_mut().zip(row) {
            if let Some(index) = run.image {
                piece.image = Some(picture_size(
                    doc,
                    pictures,
                    index,
                    faces.face(run.font).as_ref(),
                ));
            }
        }
        if pieces
            .iter()
            .all(|p| p.text.is_empty() && p.field.is_none() && p.image.is_none())
            && row.iter().all(|run| run.marquee.is_none())
        {
            if row.iter().any(|run| run.gap) {
                y = y.max(flushed.saturating_add(vspace));
            }
            let line = row
                .iter()
                .map(|run| run.line)
                .fold(Line::None, |a, b| a.max(b));
            let takes = match line {
                Line::None => false,
                Line::IfEmpty => !ink,
                Line::Always => true,
            };
            if takes {
                y = y.saturating_add(line_height(faces, row[0].font));
                flushed = y;
                if row.iter().any(|run| run.mark == Some(Mark::Rule)) {
                    rules.push(out.len());
                    out.push(Placed {
                        x: rect.x.saturating_add(2),
                        y,
                        top: y,
                        dx,
                        // The width is the whole rect less four, not the column: the rule knows
                        // nothing about an indent or the scrollbar, it spans the client rectangle.
                        w: rect.dx.saturating_sub(4),
                        h: 1,
                        row: 0,
                        color: opts.text_color,
                        text: String::new(),
                        font: row[0].font,
                        align: g::IDF_ALIGN_LEFT,
                        underline: false,
                        image: None,
                        elem: None,
                        field: None,
                        marquee: None,
                        caret: None,
                        first_row: 0,
                        mark: Some(Mark::Rule),
                    });
                }
            }
            ink = false;
            continue;
        }
        // Everything from here down puts something on the line, so the next break is one after
        // words and not one after a break.
        ink = true;
        // `HVOPT_N_INDENT` is one step per enclosing indent block, and it takes its pixels off this
        // row on both sides: `<blockquote>` narrows the column, `<dd>` only shifts the start.
        let left = step.saturating_mul(i16::from(row[0].indent));
        let right = step.saturating_mul(i16::from(row[0].inset));
        let row_dx = dx.saturating_sub(left).saturating_sub(right);
        let x0 = rect.x.saturating_add(left);
        // The leading is the gap between lines of *words*, which is why it is added to every row
        // that has one. A row of pictures alone has none: the reference engine adds nothing to a
        // line it lays out, and the picture's own height already says where the next row starts.
        let row_leading = if row.iter().any(|run| run.image.is_none()) {
            leading
        } else {
            0
        };
        // Each span's own extent first — the row is folded into the lines it takes at this width,
        // and every line is measured on its own. A row that fits is one line, which is the only
        // case a wide rect ever asks for.
        // Whether any line of this row has been placed yet. The square of an `<li>` belongs to the
        // first entry the item draws, which is the first span of the first line it takes.
        let mut started = false;
        for line in fold(faces, row, &pieces, row_dx, row[0].pre) {
            let rise = line
                .iter()
                .map(|segment| segment.ascent)
                .max()
                .unwrap_or(10);
            let drop = line
                .iter()
                .map(|segment| segment.h.saturating_sub(segment.ascent))
                .max()
                .unwrap_or(0);
            let height = rise.saturating_add(drop).saturating_add(row_leading);
            if y.saturating_add(height) <= top {
                y = y.saturating_add(height);
                flushed = y;
                continue;
            }
            if y.saturating_add(height) > bottom {
                break 'rows;
            }
            let total: i16 = line
                .iter()
                .fold(0i16, |sum, segment| sum.saturating_add(segment.w));
            // Each line is aligned as a unit — centring the first span alone would put a mixed-font
            // line off the middle by half of what follows it, and centring the whole row would
            // centre a line that has already been folded away from it.
            let mut x = match row[0].align {
                g::IDF_ALIGN_CENTER => x0.saturating_add(row_dx.saturating_sub(total) / 2),
                g::IDF_ALIGN_RIGHT => x0.saturating_add(row_dx.saturating_sub(total)),
                _ => x0,
            };
            for segment in line {
                if !started && row[0].mark == Some(Mark::Bullet) {
                    started = true;
                    let top = y.saturating_add(height / 2).saturating_sub(1);
                    out.push(Placed {
                        x: x.saturating_sub(5),
                        y: top,
                        top,
                        dx: row_dx,
                        w: 2,
                        h: 2,
                        row: 2,
                        color: opts.text_color,
                        text: String::new(),
                        font: row[0].font,
                        align: g::IDF_ALIGN_LEFT,
                        underline: false,
                        image: None,
                        elem: None,
                        field: None,
                        marquee: None,
                        caret: None,
                        first_row: 0,
                        mark: Some(Mark::Bullet),
                    });
                }
                let run = &row[segment.piece];
                let piece = &pieces[segment.piece];
                // One colour per frame, written by whichever element opened last: `<a>` writes
                // `HVOPT_RGB_LINK`, a `<font color>` writes its own, and the later one replaces the
                // earlier. So a red word inside a link stays red, and a link inside a red block is
                // drawn in the link's colour.
                let color = match run.color {
                    Ink::Own(color) => color,
                    Ink::Link if opts.link_color != 0 => opts.link_color,
                    _ => opts.text_color,
                };
                let shown = piece.text[segment.text.clone()].to_string();
                let marquee = run.marquee.map(|at| {
                    let face = faces.face(run.font);
                    MarqueeBox {
                        at,
                        text_w: width_of(face.as_ref(), &shown),
                        pad: (28 * i32::from(cell_of(face.as_ref())) / 100) as i16,
                    }
                });
                out.push(Placed {
                    x,
                    y,
                    // Every item hangs from the line's one baseline: its top is as far above that
                    // baseline as its own ascent says. A picture's ascent is its height less the
                    // descender space, a box's is where its own text sits, and a word's is the
                    // face's — which is why they line up instead of being centred against each
                    // other.
                    top: y.saturating_add(rise.saturating_sub(segment.ascent)),
                    dx: row_dx,
                    w: segment.w,
                    h: segment.h,
                    row: height,
                    color,
                    text: piece.text[segment.text.clone()].to_string(),
                    font: run.font,
                    align: run.align,
                    underline: run.underline,
                    image: run.image,
                    elem: run.elem,
                    field: piece.field,
                    marquee,
                    caret: piece.caret,
                    first_row: piece.first,
                    mark: None,
                });
                x = x.saturating_add(segment.w);
            }
            y = y.saturating_add(height);
            flushed = y;
        }
    }
    for at in rules {
        let below = out
            .get(at + 1..)
            .and_then(|rest| rest.iter().find(|p| p.mark.is_none()))
            .map(|p| p.y);
        if let Some(below) = below {
            let y = below.saturating_sub(8);
            out[at].y = y;
            out[at].top = y;
        }
    }
    // A rule whose line is outside the window is not walked at all: the redraw's tag walk starts at
    // the byte the view is scrolled to, so an `<hr>` above the fold never reaches the fill. The
    // spans are dropped rather than drawn off-screen, which would put a black pixel above the
    // control's own rectangle — the rows themselves are already trimmed by the two bounds checks
    // above, and a mark has to obey the same edge.
    out.retain(|p| p.mark != Some(Mark::Rule) || (p.y >= top && p.y < bottom));
    out
}

/// What a frame needs beyond the document itself.
///
/// Three numbers that come from the control rather than from the parse: where the keyboard is, how
/// far the page has been scrolled, and how tall it is. They are one record because they are read
/// together and because passing three bare integers to a nine-parameter painter invited the wrong
/// argument in the wrong slot.
#[derive(Debug, Clone, Copy, Default)]
pub struct View<'a> {
    /// The element that has the focus, if any: its spans get the cursor.
    pub focus: Option<usize>,
    /// How far down the page the view starts, in pixels.
    pub scroll: i16,
    /// How tall the document is at this width, or 0 when nobody measured it. Only the scrollbar
    /// reads it.
    pub content: i16,
    /// What the user has changed. `None` is a page nobody has touched — and the state every caller
    /// that only wants geometry can pass.
    pub values: Option<&'a Values>,
    /// `HVP_TRANSPARENT`: the page has no paper of its own. The reference `Redraw` asks for the
    /// background to be erased instead of filling it — see [`render`].
    pub transparent: bool,
    pub elapsed_ms: u32,
}

/// Paint the document inside `rect`, one line per run, and answer how many lines were drawn.
///
/// Each line's bytes live in guest memory for the length of `draw_text`, because that call names its
/// text by address; the block is freed immediately after, so a long document does not accumulate.
pub fn render(
    doc: &Document,
    pictures: &Pictures,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
    rect: Rect,
    opts: &Options,
    view: &View<'_>,
) -> HResult<u32> {
    // The page is measured with the faces this display paints with, not with the built-in table:
    // an installed BBF answers both the folding and the drawing, and the two have to be the same
    // face or a line is cut where the grid ends and drawn wider than the row it was given.
    let lines = layout(
        doc,
        rect,
        opts,
        pictures,
        view.values,
        view.scroll,
        display.faces(),
    );
    let mut painted = 0;
    if !view.transparent {
        let tile = doc
            .bg_image
            .and_then(|at| pictures.get(at))
            .and_then(|slot| slot.as_ref());
        if let Some(picture) = tile {
            paint_background(picture, rect, view.scroll, mem, display, disp)?;
        } else {
            let page = match doc.bg {
                Some(rgb) => rgb,
                None => display
                    .color(disp, g::CLR_SYS_WIN)
                    .unwrap_or(crate::device::RGB_WHITE),
            };
            rect.write(mem, scratch)?;
            display.draw_rect(
                mem,
                disp,
                scratch,
                crate::device::RGB_NONE,
                page,
                g::IDF_RECT_FILL,
            )?;
        }
    }
    for line in lines {
        let focused = line.elem.is_some() && line.elem == view.focus;
        // A mark is not a row of the page: it is the rectangle the reference tag walk fills while it
        // is walking, so it is stroked here and counts as nothing.
        if line.mark.is_some() {
            paint_mark(&line, mem, display, disp, scratch)?;
            continue;
        }
        painted += if let Some(index) = line.image {
            let drawn = paint_image(doc, pictures, index, &line, mem, display, disp, scratch)?;
            if focused {
                frame_inside(&line, mem, display, disp, scratch)?;
            }
            drawn
        } else if let Some(field) = line.field {
            paint_field(&field, &line, focused, mem, display, disp, scratch)?
        } else if let Some(box_) = line.marquee {
            paint_marquee(
                &doc.marquees[box_.at],
                &box_,
                &line,
                view.elapsed_ms,
                mem,
                display,
                disp,
                scratch,
            )?
        } else {
            paint_text(&line, focused, mem, display, disp, scratch)?
        };
    }
    paint_scrollbar(mem, display, disp, scratch, rect, opts, view)?;
    Ok(painted)
}

/// One line of words: the host's own text, drawn without a trip through guest memory.
fn paint_text(
    line: &Placed,
    focused: bool,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<u32> {
    // A no-break space was measured as a space and no face has a glyph for U+00A0, so it is drawn
    // as the space it looks like. It is still not a place the folder may cut, which is the whole
    // reason it survived the parse as U+00A0 rather than becoming `' '` there.
    let shown: std::borrow::Cow<'_, str> = if line.text.contains('\u{a0}') {
        std::borrow::Cow::Owned(line.text.replace('\u{a0}', " "))
    } else {
        std::borrow::Cow::Borrowed(&line.text)
    };
    // The page is host text: it was decoded from the device's byte stream when it was parsed, and
    // the face it draws with is keyed by code point. It is never turned back into `AECHAR` units.
    Rect::new(line.x, line.top, line.w.max(0), line.h.max(0)).write(mem, scratch)?;
    if line.color != 0 {
        display.set_color(disp, g::CLR_USER_TEXT, line.color)?;
    }
    let mut flags = g::IDF_ALIGN_TOP;
    if focused {
        flags |= g::IDF_TEXT_INVERTED;
    } else {
        flags |= g::IDF_TEXT_TRANSPARENT;
        if line.underline {
            flags |= g::IDF_TEXT_UNDERLINE;
        }
    }
    display.draw_text_unicode(
        mem,
        disp,
        line.font,
        &shown,
        line.x as i32,
        line.top as i32,
        scratch,
        flags,
    )?;
    Ok(1)
}

/// One form control: its box, then what the box holds, then the caret.
///
/// The frame and the fill are the display's own `CLR_USER_LINE` and background rather than the
/// page's text colour: a page that writes its prose in red still has black boxes, which is what a
/// handset's editor looks like. A password arrives already masked and a select already says which
/// option it shows, so this is the same drawing for all four of them.
fn paint_field(
    field: &Field,
    line: &Placed,
    focused: bool,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<u32> {
    let box_ = Rect::new(line.x, line.top, line.w.max(1), line.h.max(1));
    box_.write(mem, scratch)?;
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        crate::device::RGB_NONE,
        g::IDF_RECT_FILL | g::IDF_RECT_FRAME,
    )?;
    // The inside of the box: the cell the value is drawn in *and* the clip that stops it. A value
    // longer than the box shows its beginning and the frame cuts the rest, which is what a
    // one-line editor does before a caret scrolls it.
    let inner = Rect::new(
        box_.x + FIELD_PAD,
        box_.y + FIELD_PAD,
        (box_.dx - FIELD_PAD * 2).max(1),
        (box_.dy - FIELD_PAD * 2).max(1),
    );
    if matches!(field.kind, FieldKind::Checkbox | FieldKind::Radio) {
        if field.checked {
            // A tick and a dot read the same at eight pixels, and one filled square is what a
            // monochrome handset draws for both of them.
            let inset = if field.kind == FieldKind::Radio { 3 } else { 2 };
            let ink = display
                .color(disp, g::CLR_USER_LINE)
                .unwrap_or(crate::device::RGB_BLACK);
            let dot = Rect::new(
                box_.x + inset,
                box_.y + inset,
                (box_.dx - inset * 2).max(1),
                (box_.dy - inset * 2).max(1),
            );
            dot.write(mem, scratch)?;
            display.draw_rect(
                mem,
                disp,
                scratch,
                crate::device::RGB_NONE,
                ink,
                g::IDF_RECT_FILL,
            )?;
        }
    } else if !line.text.is_empty() {
        inner.write(mem, scratch)?;
        if line.color != 0 {
            display.set_color(disp, g::CLR_USER_TEXT, line.color)?;
        }
        // A textarea's value is several lines and a text field's is one; both are the same loop.
        // Each line is drawn on its own cell, and the last line is clipped by `inner` — which is
        // what a box with more text than room shows — the part the window is on, which is the
        // beginning of the value until the caret walks past the last line the box can hold. The
        // cell is the face `DrawText` will use, or the lines of the value would be spaced for the
        // built-in grid and drawn in a handset's face.
        let cell = cell_of(display.face(line.font).as_ref());
        let mut y = inner.y;
        let skip = usize::try_from(line.first_row.max(0)).unwrap_or(0);
        for text in line.text.split('\n').skip(skip) {
            if y.saturating_add(cell) > inner.y.saturating_add(inner.dy) {
                break;
            }
            if !text.is_empty() {
                display.draw_text_unicode(
                    mem,
                    disp,
                    line.font,
                    text,
                    inner.x as i32,
                    y as i32,
                    scratch,
                    g::IDF_ALIGN_TOP,
                )?;
            }
            y = y.saturating_add(cell);
        }
    }
    // The caret: an editor the user has typed into carries one, and only the focused one draws it.
    // It is a filled column inside the box, so the next frame's fill is what erases it — the same
    // rule the rest of the cursor follows.
    if focused && field.kind.is_editor() {
        if let Some(offset) = line.caret {
            let ink = if line.color != 0 {
                line.color
            } else {
                display
                    .color(disp, g::CLR_USER_TEXT)
                    .unwrap_or(crate::device::RGB_BLACK)
            };
            // `Placed::caret` is measured from the span's own corner and already carries the
            // padding, so this is `line.x + offset` and not `inner.x + offset` — one padding, not
            // two. It is clamped to the inside of the frame, which is where a real editor stops
            // drawing one, and the column is one cell tall rather than the whole box: in a
            // textarea the caret belongs to its own line.
            let cell = cell_of(display.face(line.font).as_ref());
            let x = offset
                .0
                .clamp(FIELD_PAD, (line.w - FIELD_PAD).max(FIELD_PAD));
            let y = offset
                .1
                .clamp(FIELD_PAD, (line.h - FIELD_PAD - cell).max(FIELD_PAD));
            Rect::new(line.x + x, line.top + y, 1, cell).write(mem, scratch)?;
            display.draw_rect(
                mem,
                disp,
                scratch,
                crate::device::RGB_NONE,
                ink,
                g::IDF_RECT_FILL,
            )?;
        }
    }
    // The cursor, for every kind of control — a box, a chosen radio and an empty field all have to
    // show where the user is.
    if focused {
        frame_inside(line, mem, display, disp, scratch)?;
    }
    Ok(1)
}

/// A second frame one pixel inside the span's own rectangle: how a control says "this one".
///
/// Drawn inside rather than around, so the span's own repaint — the box's fill, or the picture's
/// blit — is what erases it when the focus moves on.
fn frame_inside(
    line: &Placed,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<()> {
    if line.w <= 2 || line.h <= 2 {
        return Ok(());
    }
    Rect::new(line.x + 1, line.top + 1, line.w - 2, line.h - 2).write(mem, scratch)?;
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        crate::device::RGB_NONE,
        g::IDF_RECT_FRAME,
    )
}

struct MarqueePose {
    offset: i32,
    left: bool,
}

fn marquee_pose(mq: &Marquee, box_w: i16, text_w: i16, elapsed_ms: u32) -> MarqueePose {
    let width = i32::from(box_w);
    let text = i32::from(text_w);
    let mut offset = if mq.behavior == MarqueeBehavior::Alternate {
        text
    } else {
        0
    };
    let mut left = mq.left;
    let mut cycles: i32 = 0;
    let running = |c: i32| mq.loops < 0 || c < mq.loops;
    let mut steps = elapsed_ms / mq.delay.max(1);
    if mq.loops < 0 && mq.amount > 0 {
        // An unbounded marquee repeats the same walk for ever, so a long idle interval can be
        // reduced modulo one period instead of replaying every step. A "half" is the number of
        // steps from the reset offset to the first one past the edge; alternate needs two of
        // them because the direction comes back only after an even number of flips.
        let start = if mq.behavior == MarqueeBehavior::Alternate {
            text
        } else {
            0
        };
        let limit = if mq.behavior == MarqueeBehavior::Scroll {
            width.saturating_add(text)
        } else {
            width
        };
        let half =
            ((i64::from(limit) - i64::from(start)).max(0) / i64::from(mq.amount) + 1).max(1) as u32;
        let period = if mq.behavior == MarqueeBehavior::Alternate {
            half.saturating_mul(2)
        } else {
            half
        };
        steps %= period.max(1);
    }
    // A bounded marquee stops after a handful of passes, so replaying its steps is cheap. The cap
    // is a guard for a pathological `box` width, not a reference behaviour.
    for _ in 0..steps.min(4096) {
        if !running(cycles) {
            break;
        }
        offset = offset.saturating_add(mq.amount);
        match mq.behavior {
            MarqueeBehavior::Scroll => {
                if offset > width.saturating_add(text) {
                    cycles += 1;
                    offset = 0;
                }
            }
            MarqueeBehavior::Slide => {
                if offset > width {
                    cycles += 1;
                    offset = if running(cycles) { 0 } else { width };
                }
            }
            MarqueeBehavior::Alternate => {
                if offset > width {
                    cycles += 1;
                    if running(cycles) {
                        offset = text;
                        left = !left;
                    } else {
                        offset = width;
                    }
                }
            }
        }
    }
    MarqueePose { offset, left }
}

fn paint_marquee(
    mq: &Marquee,
    box_: &MarqueeBox,
    line: &Placed,
    elapsed_ms: u32,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<u32> {
    let rect = Rect::new(line.x, line.y, line.w.max(0), line.h.max(0));
    let bg = match mq.bg {
        Some(rgb) => rgb,
        None => display
            .color(disp, g::CLR_SYS_WIN)
            .unwrap_or(crate::device::RGB_WHITE),
    };
    rect.write(mem, scratch)?;
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        bg,
        g::IDF_RECT_FILL,
    )?;
    if line.text.is_empty() {
        return Ok(1);
    }
    let pose = marquee_pose(mq, line.w, box_.text_w, elapsed_ms);
    let x = if pose.left {
        i32::from(line.x) + i32::from(line.w) - pose.offset
    } else {
        i32::from(line.x) + pose.offset - i32::from(box_.text_w)
    };
    if line.color != 0 {
        display.set_color(disp, g::CLR_USER_TEXT, line.color)?;
    }
    // The rectangle is both the fill's shape and the text's clip. Writing it again is harmless:
    // `draw_rect` did not change it, and it keeps the two calls independent of each other.
    rect.write(mem, scratch)?;
    display.draw_text_unicode(
        mem,
        disp,
        line.font,
        &line.text,
        x,
        i32::from(line.y) + i32::from(box_.pad),
        scratch,
        g::IDF_TEXT_TRANSPARENT,
    )?;
    Ok(1)
}

fn paint_mark(
    line: &Placed,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<()> {
    Rect::new(line.x, line.y, line.w.max(0), line.h.max(0)).write(mem, scratch)?;
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        crate::device::make_rgb(0, 0, 0),
        g::IDF_RECT_FILL,
    )
}

fn paint_background(
    picture: &crate::resource::Bitmap,
    rect: Rect,
    scroll: i16,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
) -> HResult<()> {
    let (w, h) = (picture.width as i32, picture.height as i32);
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    let Some(clip) = rect.clipped(display.clip_of(disp)?) else {
        return Ok(());
    };
    let (left, top) = (rect.x as i32, rect.y as i32);
    let (right, bottom) = (left + rect.dx as i32, top + rect.dy as i32);
    // The scroll is an unsigned `DWORD` on the device; the host's `i16` never goes negative, and
    // reading one that did as no phase is the closer answer than wrapping the page upside down.
    let phase = (scroll.max(0) as i32) % h;
    let mut y = top - phase;
    while y < bottom {
        let mut x = left;
        while x < right {
            display.draw_picture(mem, disp, (x, y), picture, Some(clip))?;
            x += w;
        }
        y += h;
    }
    Ok(())
}

/// Paint one picture row, or its `alt` when there is no picture to paint.
///
/// Answers `1` when something was drawn, so a caller counting rows sees the same number whether the
/// row held a picture or a line of text.
fn paint_image(
    doc: &Document,
    pictures: &Pictures,
    index: usize,
    line: &Placed,
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
) -> HResult<u32> {
    let image = &doc.images[index];
    if let Some(Some(picture)) = pictures.get(index) {
        // `DrawPicture` clips to the destination bitmap and to the rectangle it is given, and it is
        // the one painter here that does *not* read the display's clip — a picture in a scrolled row
        // that straddles the top edge would otherwise paint over whatever the pack put above the
        // control. Handing it the display's clip is what keeps the page inside its own rect.
        let clip = display.clip_of(disp)?;
        display.draw_picture(
            mem,
            disp,
            (line.x as i32, line.top as i32),
            picture,
            Some(clip),
        )?;
        return Ok(1);
    }
    // No picture: the span exists for the `alt`. An `<img>` with neither draws nothing and still
    // took its width, which is what keeps the text after it where the document put it.
    if image.alt.is_empty() {
        return Ok(0);
    }
    // The `alt` is the span, not the line: drawn across the column it would paint over whatever
    // shares the row with it.
    Rect::new(line.x, line.top, line.w.max(0), line.h.max(0)).write(mem, scratch)?;
    display.draw_text_unicode(
        mem,
        disp,
        line.font,
        &image.alt,
        line.x as i32,
        line.top as i32,
        scratch,
        g::IDF_ALIGN_TOP,
    )?;
    Ok(1)
}

/// The vertical bar `HVP_SCROLLBAR` asks for: a frame around the reserved column, and a thumb as
/// long as the share of the page that is on screen.
///
/// Nothing is drawn when the document fits — "if the document contents do fit, a scrollbar is not
/// visible, but space for the scrollbar is still reserved" — and the
/// column is reserved either way, because [`layout`] always subtracts `HVOPT_N_SBWIDTH`.
fn paint_scrollbar(
    mem: &mut Mem,
    display: &mut Display,
    disp: u32,
    scratch: u32,
    rect: Rect,
    opts: &Options,
    view: &View,
) -> HResult<()> {
    let width = opts.sbwidth.min(rect.dx.max(0) as u32) as i16;
    if !opts.scrollbar || width <= 0 || rect.dy < 4 || view.content <= rect.dy {
        return Ok(());
    }
    let track = Rect::new(rect.x + rect.dx - width, rect.y, width, rect.dy);
    track.write(mem, scratch)?;
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        crate::device::RGB_NONE,
        g::IDF_RECT_FILL | g::IDF_RECT_FRAME,
    )?;
    // The thumb is the visible share of the page, at least three pixels so it stays a mark rather
    // than a line, and it travels the part of the track the thumb itself does not cover.
    let visible = i32::from(rect.dy);
    let total = i32::from(view.content).max(visible + 1);
    let len = (visible * visible / total).max(3).min(visible - 2);
    let travel = visible - len;
    let max_scroll = (total - visible).max(1);
    let at = (i32::from(view.scroll.max(0)) * travel / max_scroll).clamp(0, travel);
    let thumb = Rect::new(
        track.x + 1,
        track.y.saturating_add(at as i16),
        (width - 2).max(1),
        len as i16,
    );
    thumb.write(mem, scratch)?;
    let ink = display
        .color(disp, g::CLR_USER_LINE)
        .unwrap_or(crate::device::RGB_BLACK);
    display.draw_rect(
        mem,
        disp,
        scratch,
        crate::device::RGB_NONE,
        ink,
        g::IDF_RECT_FILL,
    )
}

/// The most one `data:` URI may carry. A picture big enough to need more than this is not one a
/// handset document would have inlined, and the bound keeps a malformed document from asking the
/// decoder for an allocation it cannot get.
const DATA_URI_CAP: usize = 8 * 1024 * 1024;

/// The bytes an `<img src>` carries itself, when it carries them.
///
/// A `data:` URI is the one source this host can honour without a network round trip: the document
/// already holds the picture, so decoding it is a synchronous thing to do at redraw time. A
/// document that names a URL instead is left to `alt` — fetching it would have to be asynchronous,
/// and the viewer's own `IsBusy`/`HVN_DONE` machinery for that is not built.
pub fn image_bytes(src: &str) -> Option<Vec<u8>> {
    let rest = src.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    // `;base64` is the only encoding a data URI may declare; anything else is percent-encoded text.
    if meta.rsplit(';').next() == Some("base64") {
        let (bytes, ok) = crate::web::b64_decode(payload.trim().as_bytes());
        ok.then_some(bytes)
    } else {
        Some(crate::web::url_decode(payload.as_bytes(), DATA_URI_CAP).0)
    }
}

#[cfg(test)]
#[path = "../test/web_html.rs"]
mod tests;
