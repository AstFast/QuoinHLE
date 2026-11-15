//! The handset's controls: `IControl` and the classes that inherit it.
//!
//! A classic control is a rectangle on the applet's screen that takes the keyboard. All of them
//! answer the same nine `IControl` slots — a rect, an active flag, a property word, a reset, a
//! repaint and an event — and differ only in what they paint and which keys they claim, so one
//! [`Control`] record holds it all and [`Widgets::redraw`] branches on the class the object was
//! handed out for.
//!
//! The text a control is given is copied into the guest's own heap rather than kept as a pointer
//! into the caller's buffer: that is what lets `GetTextPtr` answer later, and
//! [`Display::draw_text`] reads its text out of guest memory anyway.

use std::collections::HashMap;

use crate::abi::{self, generated as g};
use crate::device::{make_rgb, RGB_NONE};
use crate::display::{Display, Rect};
use crate::font::Font;
use crate::hle::{Error as HleError, HResult};
use crate::locale::{self, Language};
use crate::mem::Mem;
use crate::runtime::{civil_from_days, days_from_civil};
use crate::web::html::Html;

/// The interfaces one [`Control`] answers. A shipped app may call an inherited slot through
/// whichever name it holds — `IMENUCTL_SetProperties` reaches the dispatch as an `IMenuCtl` id —
/// so these are one family, not seven unrelated tables.
pub const WIDGET_IFACES: &[&str] = &[
    "IControl",
    "IStatic",
    "IMenuCtl",
    "IDateCtl",
    "ITimeCtl",
    "ITextCtl",
    "IImageCtl",
    "IAClockCtl",
    // `IHtmlViewer` inherits `IControl` (`INHERIT_IControl`), so it shares
    // slots 0-10 with every other control here; its own slots 11-23 are answered by
    // `Runtime::iwidget`'s `IHtmlViewer` arm.
    "IHtmlViewer",
];

/// Does `iface` name one of the control classes?
pub fn is_widget(iface: &str) -> bool {
    WIDGET_IFACES.contains(&iface)
}

/// Leading between the rows of a control, on top of the font's own cell.
const ROW_LEADING: i16 = 2;

/// The colour of the bar behind a selected item. A handset takes this from its system colour
/// table, which this host does not serve, so the classic highlight is carried outright.
const SEL_BAR: u32 = make_rgb(0x33, 0x66, 0xcc);

/// The header
const MP_ALLOW_DUPE_IDS: u32 = 0x0004_0000;

/// Where a date control's day number counts from.
///
/// The header
/// of days since **the first Sunday before January 1, 1970**" — 28 December 1969. The host's own
/// clock counts from 1 January 1970, so the two are four days apart, and the conversion between them
/// lives here rather than in either of the two callers.
const JULIAN_EPOCH_DAYS_BEFORE_1970: i64 = 4;
/// The day number `DateToJDay` answers for 28 December 1969 — the Sunday the shipped control puts at
/// zero (with `JDAY_OFFSET 719_465` from the header). It is the one triple
/// that converts to `0` and is still a legal date, which is why `SetDate` has to name it.
const EPOCH_YEAR: i32 = 1969;
const EPOCH_MONTH: i32 = 12;
const EPOCH_DAY: i32 = 28;
/// `JDayToDate`'s upper gate: `jday >= ((uint32)0xffffffff - (JDAY_OFFSET * 4)) / 4` is refused.
///It keeps the conversion inside the range its `long` arithmetic can name; the
/// year gate at binds long before it for any date a control would show.
const JULIAN_DAY_LIMIT: i64 = (0xffff_ffff_u32 as i64 - 719_465 * 4) / 4;

/// The bound `ITimeCtl_SetTime` puts on a **clock** only: `SECS_PER_HOUR * 1000 * 24`,
///an `>` test so exactly one day is accepted. The define of the same name at
/// is never referenced anywhere else in that file.
const MSECS_PER_DAY: i32 = 24 * 60 * 60 * 1000;

/// `DateToJDay`: the day number the date control names a triple by, and `0`
/// for a triple it refuses — `year < 1`, a month outside 1..12, a day outside 1..31.
/// A day past the end of its own month is **not** refused: the March-based formula simply adds it, so
/// 31 April becomes 1 May. The two formulas agree on the two dates worth checking: 28 December 1969
/// gives `0` and 1 January 1970 gives `4`.
fn date_to_julian(year: i32, month: i32, day: i32) -> i32 {
    if year < 1 || month < 1 || month > 12 || day < 1 || day > 31 {
        return 0;
    }
    (days_from_civil(i64::from(year), month as u16, day as u16) + JULIAN_EPOCH_DAYS_BEFORE_1970)
        as i32
}

/// How far a string taken out of guest memory as a sort key may run.
///
/// A bound and not a length: the pointer comes from the pack, and one that does not point at text
/// would otherwise walk the arena looking for a terminator that is not there.
const MAX_KEY_CHARS: u32 = 4096;

#[derive(Debug)]
struct Item {
    id: u16,
    text: u32,
    data: u32,
    /// The pair `SetItemTime` hands in and `GetItemTime` hands back. The published signature is two
    /// words in and one out plus an `int` result, so the two travel together as one value here.
    time: (u16, u16),
}

/// What a control's own key door decided.
///
/// `Post` carries the `wParam` of the `EVT_COMMAND` the shipped member hands to
/// `ISHELL_HandleEvent`, and the control's answer *is* the applet's answer to that event
/// (the delivered body, the delivered body) — so only the runtime, which can call the
/// guest, can finish the answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    NotHandled,
    Handled,
    /// Claimed, and the shipped body repaints from inside the handler. `Menu_MoveSelect`
    /// (`0x23d8`) is the one that does: it ends on `Menu_Draw` or
    /// `Menu_DrawNewSelectItem`, both of which paint and ask for the frame, and it skips them when
    /// the selection did not actually move. The runtime needs the cursor to follow the key the way
    /// a handset shows it, not a frame later.
    Redraw,
    Post(u32),
}

/// One control: its rect, its text, its items, and which of them is selected.
#[derive(Debug)]
pub struct Control {
    pub(crate) class: &'static str,
    /// Where the control is on the applet's screen. `pub(crate)` for the same reason as `class`:
    /// the Web half of the widget lives in `crate::web::widget` and lays the page out in this rect.
    pub(crate) rect: Rect,
    active: bool,
    props: u32,
    title: u32,
    text: u32,
    font_title: u32,
    font_text: u32,
    items: Vec<Item>,
    sel: usize,
    /// Where [`Widgets::next_sel`] is in its walk of the items.
    enum_sel: usize,
    enabled: bool,
    max_chars: u16,
    caret: u32,
    sel_bits: u32,
    /// The line `IStatic::GoToLine` has scrolled to, 0-based. Its own field because the header's
    /// return value is the *previous* index, so this is state a caller reads back through a write.
    scroll_line: i32,
    /// What `SetOwnerDrawCB` was handed: the pack's callback, its context, and the rows it named.
    /// Kept because a caller that installs one and then reads nothing back still deserves for the
    /// host to have taken it rather than dropped it; the paint path does not consult it yet.
    owner_draw: u32,
    owner_draw_data: u32,
    owner_draw_mask: u16,
    /// The selection of a date control, as the year/month/day triple the shipped object keeps. The
    /// day number and the weekday are **re-derived** from it rather than stored: `select_julian` is the
    /// only way the triple is written, and it stores the triple `JDayToDate` answers for that number,
    /// so the round trip is exact — including a refused set, where the shipped member leaves the triple
    /// at zeros and `DateToJDay` answers zero for zeros. the delivered body/ hand back those
    /// two stored numbers, and this answers the same value.
    date: (i32, u32, u32),
    /// `SetActiveDayMask`: which days of the month a date control lets the user land on.
    day_mask: u32,
    /// `EnableCommand`: the delivered body keeps two things, and stores the id **only when
    /// enabling**, so a disable leaves the last id in place for the next enable.
    send_cmd: bool,
    cmd_id: u16,
    /// `ITextCtl::SetSoftKeyMenu`'s menu object. Taken rather than used: the host draws no soft keys,
    /// and a caller that installs one and reads nothing back is better served by the host having it.
    softkey_menu: u32,
    /// `ITextCtl::SetInputMode`'s current mode, and where `EnumNextMode` is in its walk of the modes
    /// the host offers. Both are `AEETextInputMode` words the table publishes.
    input_mode: u32,
    enum_mode: u32,
    /// `ITextCtl::SetPropertiesEx`'s word, handed back unchanged by `GetPropertiesEx`.
    props_ex: u32,
    /// `IImageCtl::SetImage`'s picture. The shipped body does not just remember the pointer: it
    /// releases the one it held and takes a reference on the new one, so
    /// the control keeps the image alive; [`Widgets::set_image`] hands the previous one back for the
    /// arm to release.
    image: u32,
    /// `IImageCtl::SetRedraw`'s notify and its context, stored **unconditionally** — a zero pointer is
    /// a real answer here, because it is what un-installs the callback.
    redraw_cb: u32,
    redraw_user: u32,
    /// The time control's own state: the number `GetTime` hands back, whether
    /// `SetTime` last accepted one, the minute step (`TimeCtl_New` sets it to 1 at), and the
    /// field being edited — which `SetEditField` stores as `(uint16)field * 3`, not as the
    /// enum it was given.
    ms: i32,
    ms_valid: bool,
    increment: u16,
    edit_field: u16,
    /// The control's own two item-style records — the shipped object's `+356` and `+364`, **eight
    /// bytes apiece**. `MenuCtl_New` fills them from `ISHELL_GetItemStyle`, `IMenuCtl_SetStyle`
    /// memmoves eight bytes in from each non-null argument, and `IMenuCtl_GetStyle` memmoves eight
    /// back out; the header's `AEEItemStyle` is twelve bytes counting `roImage` at `+8`, and this
    /// control never reads or writes those last four.
    style: [[u8; 8]; 2],
    /// The `AEEMenuColors` the control carries, as ten words: the mask halfword at `+0` and the
    /// padding beside it, then the nine colours at `+4..+36`. They are **system-colour indices**, not
    /// pixels — the shipped member turns them into colours at draw time through three
    /// `IDisplay::SetColor(CLR_USER_*, value)` calls (`Menu_InitColors`), and no slot of this
    /// interface ever writes the first word.
    colors: [u32; 10],
    /// The `IHtmlViewer` state, for the one class that has one. `None` for every other control.
    pub(crate) html: Option<Html>,
}

/// What `IMenuCtl_SetColors(NULL)` writes, which is also what `MenuCtl_New` calls it with,
///as the ten words of [`Control::colors`]: the delivered body's `else` arm stores
/// `6, 7, 8, 9, 12, 15, 16, 4, 5` into `+444..+476` — that is `CLR_SYS_ITEM`, `CLR_SYS_ITEM_TEXT`,
/// `CLR_SYS_ITEM_SEL`, `CLR_SYS_ITEM_SEL_TEXT`, `CLR_SYS_FRAME_LO`, `CLR_SYS_SCROLLBAR`,
/// `CLR_SYS_SCROLLBAR_FILL`, `CLR_SYS_TITLE`, `CLR_SYS_TITLE_TEXT`. The mask word stays at the zero
/// `AEE_NewClass` left there, because nothing in the member writes it.
const MENU_COLORS: [u32; 10] = [0, 6, 7, 8, 9, 12, 15, 16, 4, 5];

/// `IMenuCtl_SetColors`' mask table, read off the body: `MC_BACK`(1), `MC_TEXT`(2),
/// `MC_SEL_BACK`(4), `MC_SEL_TEXT`(8) and `MC_FRAME`(0x10) come first, then the scrollbar pair at
/// `0x40`/`0x80` and the title pair at `0x100`/`0x200` — each named bit copies the caller's word at
/// `+4 + 4 * index` into the same word of the control. **`0x20` names no colour and is not tested**,
/// so a caller that sets it changes nothing; the header defines the mask with the same gap.
const MENU_COLOR_BITS: [(u32, usize); 9] = [
    (0x001, 1),
    (0x002, 2),
    (0x004, 3),
    (0x008, 4),
    (0x010, 5),
    (0x040, 6),
    (0x080, 7),
    (0x100, 8),
    (0x200, 9),
];

impl Control {
    /// The pair of faces a control's own constructor mounts. This is what `GetFont` answers before
    /// anybody has called `SetFont`, and for the three controls whose setter substitutes on a zero it
    /// is the same pair the setter falls back to. `DateCtl_New` puts a `AEE_FONT_BOLD` title over a
    /// `AEE_FONT_NORMAL` body, `TimeCtl_New` a `AEE_FONT_BOLD` letter over
    /// `AEE_FONT_LARGE` digits, and `AStatic_New` reaches the date control's
    /// pair by calling its own setter. `None` is a class whose body names no
    /// fonts, which is every other control: those keep `AEE_FONT_NORMAL` for both, and publish no
    /// `GetFont` for anyone to read.
    fn font_defaults(class: &str) -> Option<(u32, u32)> {
        match class {
            "IDateCtl" => Some((g::AEE_FONT_NORMAL, g::AEE_FONT_BOLD)),
            "ITimeCtl" => Some((g::AEE_FONT_LARGE, g::AEE_FONT_BOLD)),
            "IStatic" => Some((g::AEE_FONT_NORMAL, g::AEE_FONT_BOLD)),
            _ => None,
        }
    }

    fn new(class: &'static str) -> Self {
        let (font_text, font_title) =
            Self::font_defaults(class).unwrap_or((g::AEE_FONT_NORMAL, g::AEE_FONT_NORMAL));
        Control {
            class,
            rect: Rect::new(0, 0, 0, 0),
            active: false,
            props: 0,
            title: 0,
            text: 0,
            font_title,
            font_text,
            items: Vec::new(),
            sel: 0,
            enum_sel: 0,
            enabled: true,
            // `TextCtl_New` fills the text control's ceiling with `DEF_TEXT_SIZE` (
            //): a fresh text control holds 32 characters, and `SetMaxSize` is what lowers or
            // raises it. Left at zero, `CTextCtl_SetText` would store nothing.
            max_chars: if class == "ITextCtl" { 32 } else { 0 },
            caret: 0,
            sel_bits: 0,
            scroll_line: 0,
            owner_draw: 0,
            owner_draw_data: 0,
            owner_draw_mask: 0,
            // A control that has never been given a date answers zero, which is what the shipped
            // object leaves after `AEE_NewClass` zeroes it: `IDateCtl_Reset` does not clear the date
            // and `IDateCtl_GetDateString` happily formats whatever is there
            // ( has no "is the date set" test), so a fresh the delivered body control says `0 0 '00`.
            // The device's own reference time (6 January 1980) belongs to `AEEDeviceInfo`, not here.
            date: (0, 0, 0),
            day_mask: 0,
            // The delivered body
            // their own `EnableCommand` (the delivered body), so they start at off.
            send_cmd: class == "IMenuCtl",
            cmd_id: 0,
            softkey_menu: 0,
            input_mode: g::AEE_TM_CURRENT,
            enum_mode: 0,
            props_ex: 0,
            ms: 0,
            ms_valid: false,
            increment: 1,
            edit_field: 0,
            image: 0,
            redraw_cb: 0,
            redraw_user: 0,
            // Both records are born the way `MenuCtl_New` leaves them: the style pair from
            // `ISHELL_GetItemStyle`, which [`Widgets::seed_menu_style`] writes as soon as the object
            // exists, and the colours from the `IMenuCtl_SetColors(NULL)` reset.
            style: [[0; 8]; 2],
            colors: MENU_COLORS,
            // Only `IHtmlViewer` carries the document/view state; the other classes leave it `None`
            // so a call through the wrong class is a missing record rather than a silent no-op.
            html: (class == "IHtmlViewer").then(Html::default),
        }
    }

    /// Write the selection from one day number, the way `IDateCtl_SetJulianDay` does
    /// with `JDayToDate`'s two refusals (,): a negative number, a
    /// number at or past [`JULIAN_DAY_LIMIT`], or a year above 9999, leaves the triple at zeros rather
    /// than keeping the caller's.
    fn select_julian(&mut self, day: i32) -> bool {
        let l = i64::from(day);
        let in_range = l >= 0 && l < JULIAN_DAY_LIMIT;
        let (y, m, d) = if in_range {
            civil_from_days(l - JULIAN_EPOCH_DAYS_BEFORE_1970)
        } else {
            (0, 0, 0)
        };
        // `JDayToDate` tests the **March-based** year it computed and only shifts it
        // afterwards for January and February, so 1 January 10000 passes its gate while
        // 1 March 10000 does not. Testing the calendar year instead would refuse the sixty day numbers
        // between them that a handset accepts.
        let march_year = if m <= 2 { y - 1 } else { y };
        if !in_range || march_year > 9999 {
            self.date = (0, 0, 0);
            return false;
        }
        self.date = (y as i32, u32::from(m), u32::from(d));
        true
    }

    /// The day number this triple came from — the inverse of [`Control::select_julian`], and `0` for
    /// the zeros a refused set leaves behind.
    fn julian(&self) -> i32 {
        date_to_julian(self.date.0, self.date.1 as i32, self.date.2 as i32)
    }

    /// The stored weekday, `jday % 7` with the first Sunday before 1970 at zero,
    /// so 0 is Sunday.
    fn weekday(&self) -> u16 {
        i64::from(self.julian()).rem_euclid(7) as u16
    }

    /// The height of one line of `font`, cell plus leading.
    fn line(font: u32) -> i16 {
        Font::of(font)
            .map(|f| f.ascent() as i16 + f.descent() as i16)
            .unwrap_or(10)
            + ROW_LEADING
    }

    /// One row of the control's body font.
    fn row_height(&self) -> i16 {
        Self::line(self.font_text)
    }

    /// How many rows the rect shows and which item is drawn first: scrolled just enough that the
    /// selection is never off the control.
    fn window(&self) -> (usize, usize) {
        let height = self.row_height();
        if height <= 0 || self.rect.dy <= 0 || self.items.is_empty() {
            return (0, 0);
        }
        let rows = (self.rect.dy as usize / height as usize).max(1);
        let sel = self.sel.min(self.items.len() - 1);
        (rows, sel.saturating_sub(rows - 1))
    }

    fn item(&self, id: u16) -> Option<usize> {
        self.items.iter().position(|it| it.id == id)
    }
}

/// Every control the host has handed out, keyed by the address of the object the guest holds.
#[derive(Debug)]
pub struct Widgets {
    controls: HashMap<u32, Control>,
    /// One `AEERect` in guest memory, taken the first time anything paints and reused for every
    /// control and every row after that.
    ///
    /// `DrawText` and `DrawRect` name their bounds *by address*, so a control laying its rows out has
    /// to point at a place rather than pass a value. Allocating one per paint — which is what this
    /// did — is eight bytes the pack can never free, at the rate of the frame clock: a menu with six
    /// items repaints 48 a second. The value is written immediately before each call and read inside
    /// it, and no guest code runs between the two, so one slot is enough for the whole session.
    scratch: u32,
    /// The language a date control answers its weekday and month names in, chosen once from the
    /// command line: a control's own slots carry nowhere to name one, and a pack cannot supply the
    /// words because they are the platform's. See [`crate::locale`].
    language: &'static Language,
}

impl Default for Widgets {
    fn default() -> Self {
        Widgets {
            controls: HashMap::new(),
            scratch: 0,
            language: locale::find(locale::DEFAULT),
        }
    }
}

impl Widgets {
    /// The record for a control `IShell::CreateInstance` has just handed out.
    /// `IStatic::GoToLine`: move the control to a 0-based line, answering the **previous** index.
    ///
    /// The header is explicit about the odd shape: "This function allows
    /// the caller to auto-scroll to a specific line. The nLine parameter is a 0-based index value",
    /// and its return value is "Previous index value". So this is a read-modify-write and not a
    /// setter — the answer is where the control *was*, which is the only way a caller can tell what
    /// moved. A control that has never been scrolled sits at 0.
    ///
    /// The move itself is gated the way the body is: `m_nLines` non-zero and
    /// `0 <= nLine < m_nLines`, or the index stays where it was. The redraw that follows there
    /// is not needed here — this host paints on the next frame regardless.
    pub fn goto_line(
        &mut self,
        mem: &mut Mem,
        display: &mut Display,
        disp: u32,
        addr: u32,
        line: i32,
    ) -> HResult<u32> {
        let was = self.peek(addr)?.scroll_line;
        let lines = self.static_lines(mem, display, disp, addr)?;
        if lines != 0 && line >= 0 && line < lines {
            self.control(addr)?.scroll_line = line;
        }
        Ok(was as u32)
    }

    pub fn add(&mut self, addr: u32, class: &'static str) {
        self.controls.insert(addr, Control::new(class));
    }

    /// `Static_New` is born covering the destination: it reads the bitmap's own size into `m_rc`,
    ///which is why a static that is drawn before `SetRect` still has
    /// somewhere to draw.
    pub fn seed_static(&mut self, addr: u32, width: u16, height: u16) -> HResult<()> {
        let c = self.control(addr)?;
        if c.class == "IStatic" && c.rect.is_empty() {
            c.rect = Rect::new(0, 0, width as i16, height as i16);
        }
        Ok(())
    }

    /// A text control is born with its text object already made: `TextCtl_New` calls
    /// `TextCtl_CalcRects` before it returns, and that is what makes
    /// `GetTextPtr` answer a real pointer — to the empty string — before any `SetText`, and
    /// `GetText` answer TRUE for the same reason.
    pub fn seed_empty_text(&mut self, mem: &mut Mem, addr: u32) -> HResult<()> {
        let empty = self.peek(addr)?.class == "ITextCtl" && self.peek(addr)?.text == 0;
        if empty {
            let at = mem.alloc(2, 2)?;
            mem.write_u16(at, 0)?;
            self.control(addr)?.text = at;
        }
        Ok(())
    }

    /// A control at zero references is gone, and its strings went with it.
    pub fn forget(&mut self, addr: u32) {
        self.controls.remove(&addr);
    }

    /// Whether this store holds the address, which is the same question as "did this host make the
    /// control": a control that came out of a module has no record here, and its event door is the
    /// vtable slot in the object rather than anything the store can answer.
    pub fn holds(&self, addr: u32) -> bool {
        self.controls.contains_key(&addr)
    }

    pub(crate) fn control(&mut self, addr: u32) -> HResult<&mut Control> {
        self.controls
            .get_mut(&addr)
            .ok_or(HleError::NotAnObject { addr })
    }

    pub(crate) fn peek(&self, addr: u32) -> HResult<&Control> {
        self.controls
            .get(&addr)
            .ok_or(HleError::NotAnObject { addr })
    }

    // ------------------------------------------------------------- IControl, all classes

    /// `IControl::SetRect`. A null rect is what the handset gets before the applet has placed the
    /// control, and it means "nowhere", not "the whole screen".
    ///
    /// `IStatic` is the one body with a gate of its own: `AStatic_SetRect` ignores a null rect and
    /// one shorter than a line of the body font, so the control keeps the box it
    /// had — which for a fresh static is the screen.
    pub fn set_rect(&mut self, mem: &Mem, addr: u32, prc: u32) -> HResult<()> {
        let c = self.control(addr)?;
        if prc == 0 {
            if c.class != "IStatic" {
                c.rect = Rect::new(0, 0, 0, 0);
            }
            return Ok(());
        }
        let rect = Rect::read(mem, prc)?;
        if c.class == "IStatic" && rect.dy < Control::line(c.font_text) {
            return Ok(());
        }
        c.rect = rect;
        Ok(())
    }

    pub fn get_rect(&self, mem: &mut Mem, addr: u32, prc: u32) -> HResult<()> {
        if prc == 0 {
            return Ok(());
        }
        self.peek(addr)?.rect.write(mem, prc)
    }

    /// The rectangle a control holds, asked of the store rather than through the caller's memory.
    /// `ADialog_Load` needs the control's own idea of its size before it moves the control, which is
    /// the question [`Widgets::get_rect`] answers for a caller that has a buffer to put it in.
    pub fn rect_at(&self, addr: u32) -> HResult<Rect> {
        Ok(self.peek(addr)?.rect)
    }

    /// The writing half of the same pair: `ICONTROL_SetRect` stores what the caller's block says, and
    /// this stores what the dialog's arithmetic came out to.
    pub fn place(&mut self, addr: u32, rc: Rect) -> HResult<()> {
        self.control(addr)?.rect = rc;
        Ok(())
    }

    /// `IControl::SetActive`. The time control's own body is the one that does not always obey:
    /// `ITimeCtl_SetActive` wraps the whole store in `if(!IsStopWatch(pme))`, so
    /// a stopwatch has no edit mode and `ITimeCtl_IsActive` answers FALSE for the rest of
    /// its life. Dropping the call is not cosmetic — `ITimeCtl_GetTimeString` reads `m_bActive` at
    /// to choose between `"%02d:%02d"` and `"%d:%02d"`, so a stopwatch that was told to become
    /// active would draw `01:05` where a handset draws `1:05`. A deactivated control also loses its
    /// selected field, which is the time control's own `m_nSelect`.
    ///
    /// The answer is not the call's result — that is `void` — but whether the shipped body repaints
    /// from here, which a caller with a `Display` in hand has to honour. It is a per-class fact and
    /// the two classes disagree:
    ///
    /// * `AStatic_SetActive` stores the new bit and then calls
    ///   `AStatic_Redraw`, but only inside `if (m_dwProps & ST_ENABLE_HLGHT)` and only when the bit
    ///   actually moved. `AStatic_Redraw` paints and stops: it never asks for a screen.
    /// * `IMenuCtl_SetActive` (`0xb40`) moves the same bit, cancels the auto-scroll
    ///   timer and falls into `IMenuCtl_Redraw` — unless the menu was given `MP_NO_REDRAW`, which
    ///   is the property that says the pack will draw it itself. `IMenuCtl_Redraw` lands on
    ///   `Menu_Draw`, whose last statement is `IDisplay::Update(po, TRUE)`: a menu repaint *is* a
    ///   frame, which is why the caller is told which class it was.
    ///
    /// Nothing else in the family has a witness for repainting, so everything else answers FALSE and
    /// the call stays a store — which is what a pack that draws from its own event handler expects.
    pub fn set_active(&mut self, addr: u32, on: u32, is_stopwatch: bool) -> HResult<bool> {
        let c = self.control(addr)?;
        if is_stopwatch {
            return Ok(false);
        }
        // `AStatic_SetActive` is the one body that refuses to move unless the
        // control was given `ST_ENABLE_HLGHT`, which is what makes the property a promise the pack
        // can rely on rather than a paint hint.
        if c.class == "IStatic" && c.props & g::ST_ENABLE_HLGHT == 0 {
            return Ok(false);
        }
        let was = c.active;
        c.active = on != 0;
        if on == 0 && c.class == "ITimeCtl" {
            c.edit_field = 0;
        }
        // Both shipped bodies test the stored bit before painting, so a `SetActive` that asks for
        // what is already true is a no-op on a handset too. `MP_NO_REDRAW` is a menu property; the
        // same bit means `ST_TEXTALLOC` on a static, and reading it as a paint hint there would
        // silence a control that the shipped body does repaint.
        if was == c.active {
            return Ok(false);
        }
        Ok(match c.class {
            "IStatic" => true,
            "IMenuCtl" => c.props & g::MP_NO_REDRAW == 0,
            _ => false,
        })
    }

    pub fn is_active(&self, addr: u32) -> HResult<u32> {
        Ok(u32::from(self.peek(addr)?.active))
    }

    pub fn set_properties(&mut self, addr: u32, props: u32) -> HResult<()> {
        self.control(addr)?.props = props;
        Ok(())
    }

    pub fn properties(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.props)
    }

    /// `IControl::Reset` empties what the control *holds*. The rect, properties and fonts stay,
    /// because those are how the applet laid the control out, not what it carries.
    pub fn reset(&mut self, addr: u32) -> HResult<()> {
        let c = self.control(addr)?;
        c.items.clear();
        c.sel = 0;
        c.enum_sel = 0;
        c.title = 0;
        c.text = 0;
        c.caret = 0;
        // `IDateCtl_Reset` also drops the active flag and the day mask, and does
        // **not** touch the selection, the fonts or the command id — a control that has been reset
        // still shows the date it was given.
        if c.class == "IDateCtl" {
            c.active = false;
            c.day_mask = 0;
        }
        // `ITimeCtl_Reset` is the same shape with its own fields: active off and
        // the edit cursor back to 0 — note that is **0**, where the date control's current field goes
        // to −1 — and the time value and the minute step stay as they were.
        if c.class == "ITimeCtl" {
            c.active = false;
            c.edit_field = 0;
        }
        Ok(())
    }

    /// `IDateCtl::EnableCommand`: the flag is `bEnable != 0`, and the id is
    /// stored **only when enabling**, so disabling keeps the last id for the next
    /// enable. What the flag is for is the payoff this host does not have yet: on `AVK_SELECT` the
    /// shipped control re-posts `EVT_COMMAND` with that id through the shell, which is
    /// the reason the slot exists. Registered as a gap rather than silently half-done.
    /// `IMenuCtl::EnableCommand(boolean bEnable)` — one parameter, no id: the delivered body stores
    /// only `m_bSendCmd`, and the item a confirm reports is the *selected* one, read when the key
    /// arrives (,). Storing the enabled flag here instead switched the control's whole
    /// key handling off, which is not what this slot is for.
    pub fn set_send_cmd(&mut self, addr: u32, on: u32) -> HResult<()> {
        self.control(addr)?.send_cmd = on != 0;
        Ok(())
    }

    pub fn set_command(&mut self, addr: u32, on: u32, cmd: u16) -> HResult<()> {
        let c = self.control(addr)?;
        c.send_cmd = on != 0;
        if c.send_cmd {
            c.cmd_id = cmd;
        }
        Ok(())
    }

    /// `ITimeCtl::SetTime`: the validity flag is cleared **first**, so a refusal
    /// is a refusal even if the number was accepted before. A negative value is always refused; the
    /// one-day bound belongs to the clock alone (`IsAlarmClock`), and it is an `>` test, so exactly
    /// 86 400 000 is accepted. `bIncUpdate` only decides what happens to the drawn-string cache, which
    /// this host has none of — and the shipped tail calls `TimeCtl_Redraw`, which the host's paint
    /// path does in its own turn.
    pub fn set_time(&mut self, addr: u32, tod: i32, is_clock: bool) -> HResult<u32> {
        let c = self.control(addr)?;
        c.ms_valid = false;
        if tod < 0 {
            return Ok(0);
        }
        if is_clock && tod > MSECS_PER_DAY {
            return Ok(0);
        }
        c.ms_valid = true;
        c.ms = tod;
        Ok(1)
    }

    /// `ITimeCtl::GetTime`: the stored number, whatever it is. No failure
    /// branch, no reference to whether `SetTime` last accepted it.
    pub fn time(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.ms as u32)
    }

    /// `ITimeCtl::SetIncrement`: stored raw. There is no range check, so a
    /// caller that passes 0 freezes the minute step and one that passes 900 is kept too — the
    /// consumption sites are the arrow keys and `TimeCtl_BumpMins`.
    pub fn set_increment(&mut self, addr: u32, mins: u16) -> HResult<()> {
        self.control(addr)?.increment = mins;
        Ok(())
    }

    /// `ITimeCtl::SetEditField`: `m_nSelect = (uint16)field * 3`, with no range
    /// check and no redraw. The three fields are 0, 3 and 6 because the editing cursor is a character
    /// offset into the drawn string, not a field number.
    pub fn set_edit_field(&mut self, addr: u32, field: u32) -> HResult<()> {
        self.control(addr)?.edit_field = (field as u16).wrapping_mul(3);
        Ok(())
    }

    /// `IImageCtl::SetImage`: drop the picture it held, clear the cached
    /// `AEEImageInfo` and the scroll offsets, take the new pointer, and **addref it when it is not
    /// null** — so the control holds a real reference, not a borrowed address. Every path through it
    /// succeeds, and `pi == 0` is an ordinary call meaning "no picture", not a refusal.
    /// Answers the previous picture so the arm can release it the way `aee_releaseobj` does.
    pub fn set_image(&mut self, addr: u32, img: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        let prev = c.image;
        c.image = img;
        Ok(prev)
    }

    /// `IImageCtl::SetRedraw`: store the notify and its context first,
    /// **whatever they are**, then — and only then — reach into the bound picture with
    /// `IIMAGE_SetParm(IPARM_REDRAW, pfn ? ImageCtl_RedrawCB: NULL, pme)` so the image calls the
    /// control instead of drawing a frame during animation. The second half is
    /// not this host's to copy yet: our `IImage` takes no `IPARM_REDRAW` and has no animation tick to
    /// call anything from, so only the control's own two words are kept.
    pub fn set_redraw(&mut self, addr: u32, cb: u32, user: u32) -> HResult<()> {
        let c = self.control(addr)?;
        c.redraw_cb = cb;
        c.redraw_user = user;
        Ok(())
    }

    /// `ITimeCtl_GetTimeString`, a `void` slot that writes into `pDest`.
    ///
    /// The layout is read line by line there: `GTS_MSECS` with a zero hour collapses to `"%02d"` of
    /// the minutes alone; otherwise hours and minutes are `"%02d:%02d"` when the control is **active**
    /// or the caller asked for seconds, and `"%d:%02d"` when it is neither — the same number printed
    /// two ways depending on a flag the pack set with `SetActive`. `GTS_SECS` or `GTS_MSECS` appends
    /// `":%02d"`, `GTS_MSECS` also appends `".%02d"` of **hundredths**, not milliseconds, and
    /// `GTS_AMPM` appends the hardcoded `" PM"`/`" AM"` — no shell, no locale, no resource, unlike the
    /// date control's words. `nSize` is bytes and 0 means "write nothing".
    ///
    /// What is *not* witnessed here: the shipped decomposition is `aee_MSECSToHM`, which has only a
    /// declaration in this drop. The 12-hour fold and the PM flag
    /// are taken from the same file's own editing rule (`TimeCtl_BumpHours`,: 1..12 with
    /// the flip at 12 up / 11 down), which is a **B**, not a copy.
    pub fn time_string(
        &self,
        mem: &mut Mem,
        dest: u32,
        size: u32,
        ms: i64,
        flags: u32,
        active: bool,
    ) -> HResult<()> {
        let text = Self::time_text(ms, flags, active);
        Self::put_words(mem, dest, size as i32, &text)?;
        Ok(())
    }

    /// The formatting half of [`Widgets::time_string`], on its own because `ITimeCtl::SizeToFit`
    /// measures this same string in a buffer of its own.
    fn time_text(ms: i64, flags: u32, active: bool) -> String {
        let hundredths = (ms / 10).rem_euclid(100) as u32;
        let secs = (ms / 1000).rem_euclid(60) as u32;
        let mins = (ms / 60_000).rem_euclid(60) as u32;
        let hours = ms / 3_600_000;
        let (hour, pm) = if flags & g::GTS_AMPM != 0 {
            let day = hours.rem_euclid(24) as u32;
            (((day + 11) % 12) + 1, day >= 12)
        } else {
            (hours as u32, false)
        };
        let mut out = String::new();
        if flags & g::GTS_MSECS != 0 && hours == 0 {
            out.push_str(&format!("{mins:02}"));
        } else if active || flags & g::GTS_SECS != 0 {
            out.push_str(&format!("{hour:02}:{mins:02}"));
        } else {
            out.push_str(&format!("{hour}:{mins:02}"));
        }
        if flags & (g::GTS_SECS | g::GTS_MSECS) != 0 {
            out.push_str(&format!(":{secs:02}"));
        }
        if flags & g::GTS_MSECS != 0 {
            out.push_str(&format!(".{hundredths:02}"));
        }
        if flags & g::GTS_AMPM != 0 {
            out.push_str(if pm { " PM" } else { " AM" });
        }
        out
    }

    /// `ITimeCtl::SizeToFit`: the flags are built from the **class** — seconds
    /// unless `TP_NO_SECONDS`, AM/PM for the clock, hundredths for the stopwatch unless
    /// `TP_NO_MSECONDS` — and the string is measured at its own width, the height is the taller of the
    /// two faces. It only ever shrinks the caller's `dx`/`dy`, never touches `x`/`y`, and then hands
    /// the rectangle to `ITimeCtl_SetRect`. The width a handset gets comes from `IDisplay`'s font
    /// metrics; this host measures its own face table, which is the part that cannot be copied.
    pub fn time_size_to_fit(
        &mut self,
        mem: &mut Mem,
        addr: u32,
        prc: u32,
        is_clock: bool,
        is_stopwatch: bool,
    ) -> HResult<()> {
        if prc == 0 {
            return Ok(());
        }
        let c = self.control(addr)?;
        let mut flags = 0u32;
        if c.props & g::TP_NO_SECONDS == 0 {
            flags |= g::GTS_SECS;
        }
        if is_clock {
            flags |= g::GTS_AMPM;
        }
        if is_stopwatch && c.props & g::TP_NO_MSECONDS == 0 {
            flags |= g::GTS_MSECS;
        }
        let text = Self::time_text(i64::from(c.ms), flags, c.active);
        let units: Vec<u16> = text.encode_utf16().collect();
        let width = |font: u32| -> u32 {
            Font::of(font)
                .map(|f| f.measure_chars(&units))
                .unwrap_or_default()
        };
        let n_width = i64::from(width(c.font_text));
        let height = |font: u32| -> i64 { i64::from(Control::line(font)) };
        let tall = height(c.font_text).max(height(c.font_title));
        let mut rect = Rect::read(mem, prc)?;
        if n_width < i64::from(rect.dx) {
            rect.dx = n_width.min(i64::from(i16::MAX)) as i16;
        }
        if tall < i64::from(rect.dy) {
            rect.dy = tall as i16;
        }
        rect.write(mem, prc)?;
        self.set_rect(mem, addr, prc)?;
        Ok(())
    }

    pub fn set_enabled(&mut self, addr: u32, on: u32) -> HResult<()> {
        self.control(addr)?.enabled = on != 0;
        Ok(())
    }

    /// The key *presses* a control claims for itself: the cursor moves the selection, and a confirm
    /// leaves it where it is for the applet to read back with `GetSel`. Everything else — the
    /// matching release included, which a control has no use for — is reported as not handled, so
    /// the applet's own handler still sees it. Acting on the key code alone would step the selection
    /// once for the press and again for the release.
    ///
    /// A confirm on a control whose command flag is on is not consumed quietly: the delivered body
    /// and the delivered body answer it with
    /// `ISHELL_HandleEvent(shell, EVT_COMMAND, (uint16)m_nCmdId, (uint32)po)` and **return the
    /// applet's answer as the control's own**, and the menu does the same with the selected item's id,
    ///returning FALSE when its flag is off. The time control refuses
    /// every event before that when it is not active. Only the runtime can make
    /// that call, so it is handed back here.
    pub fn handle_key(&mut self, addr: u32, evt: u32, code: u32) -> HResult<Key> {
        let c = self.control(addr)?;
        // Both spellings are the same key to a shipped control. `IMenuCtl_HandleEvent`
        // (`0x870`) acts on `a2 == 256`, which is the older `EVT_KEY` that carries a
        // press and a release in one event — `chess.mod` sends exactly that. `EVT_KEY_PRESS` is what
        // this host posts from its own keyboard, and a pack that uses the split pair never sends
        // `EVT_KEY`, so taking both steps the cursor once either way.
        if !c.enabled || !matches!(evt, g::EVT_KEY | g::EVT_KEY_PRESS) {
            return Ok(Key::NotHandled);
        }
        if code == g::AVK_SELECT {
            c.enum_sel = 0;
            if !c.send_cmd {
                // All three bodies hand the key back when their flag is off:
                // `Menu_HandleSelection` returns FALSE, `IDateCtl_HandleEvent`
                // opens its `AVK_SELECT` case with `if ( (v5 & 4) == 0 ) return 0;`, and
                // The delivered body
                // function's own `return FALSE`. Claiming it would swallow a key the handset passes to
                // the applet.
                return Ok(if matches!(c.class, "IMenuCtl" | "IDateCtl" | "ITimeCtl") {
                    Key::NotHandled
                } else {
                    Key::Handled
                });
            }
            return Ok(match c.class {
                "IMenuCtl" => match c.items.get(c.sel) {
                    Some(item) => Key::Post(item.id as u32),
                    None => Key::NotHandled,
                },
                "IDateCtl" => Key::Post(c.cmd_id as u32),
                "ITimeCtl" if c.active => Key::Post(c.cmd_id as u32),
                _ => Key::Handled,
            });
        }
        if c.items.is_empty() {
            return Ok(Key::NotHandled);
        }
        let was = c.sel;
        let last = c.items.len() - 1;
        match code {
            g::AVK_UP => c.sel = c.sel.saturating_sub(1),
            g::AVK_DOWN => c.sel = (c.sel + 1).min(last),
            _ => return Ok(Key::NotHandled),
        }
        // A cursor that is already at the top (or the bottom) claims the key and has nothing to
        // show, which is the `if (v13 != v6 || v5 != v7)` gate `Menu_MoveSelect` opens with.
        Ok(if c.sel != was {
            Key::Redraw
        } else {
            Key::Handled
        })
    }

    // ---------------------------------------------------------------- text and item setters

    /// Take ownership of the caller's `AECHAR` string. A null pointer clears what was there.
    fn own_str(mem: &mut Mem, text: u32) -> HResult<u32> {
        let Some(at) = Self::cut_str(mem, text)? else {
            return Ok(0);
        };
        Ok(at)
    }

    /// Read a caller's `AECHAR` string and copy it into a buffer of the host's own making.
    fn cut_str(mem: &mut Mem, text: u32) -> HResult<Option<u32>> {
        if text == 0 {
            return Ok(None);
        }
        let mut units = mem.read_u16str(text, 4096)?;
        units.truncate(units.iter().position(|c| *c == 0).unwrap_or(units.len()));
        let at = mem.alloc((units.len() as u32 + 1) * 2, 2)?;
        mem.write_u16str(at, &units)?;
        Ok(Some(at))
    }

    pub fn set_title(&mut self, mem: &mut Mem, addr: u32, text: u32) -> HResult<u32> {
        let taken = Self::own_str(mem, text)?;
        self.control(addr)?.title = taken;
        Ok(u32::from(taken != 0))
    }

    /// `IStatic::SetFont`, whose two fonts the header gives as `(fntText, fntTitle)` — the other way
    /// round from `SetText`. The date control's own body adds two rules this door did not have:
    ///a designator of **zero means that field's default** (`AEE_FONT_NORMAL`
    /// for the body, `AEE_FONT_BOLD` for the title,), and a pair that is not both inside
    /// `BETWEEN(fnt, AEE_FONT_NORMAL, AEE_FONT_TOTAL)` makes the whole call **do nothing** — no error,
    /// no partial write, so a caller that names one bad font keeps the pair it already had.
    /// `BETWEEN` is unsigned with an exclusive upper bound, which makes the legal set
    /// exactly `0x8000..=0x8007`.
    pub fn set_font(&mut self, addr: u32, text: u32, title: u32) -> HResult<()> {
        let c = self.control(addr)?;
        // The three controls that name defaults do it differently: the date control's body is
        // `AEE_FONT_NORMAL` under a `AEE_FONT_BOLD` title, the time control's
        // digits are `AEE_FONT_LARGE` under `AEE_FONT_BOLD` letters, and the
        // static's pair is the date control's.
        let defaults = Control::font_defaults(c.class);
        let Some((default_text, default_letter)) = defaults else {
            c.font_text = text;
            c.font_title = title;
            return Ok(());
        };
        let text = if text == 0 { default_text } else { text };
        let title = if title == 0 { default_letter } else { title };
        // `IDateCtl_SetFont` and `ITimeCtl_SetFont` both wrap their stores in `BETWEEN(fnt,
        // AEE_FONT_NORMAL, AEE_FONT_TOTAL)` (the delivered body), which rejects
        // the whole call. `AStatic_SetFont` has no such test — there is no such guard in it at all —
        // so it keeps whatever number it was handed.
        let within = |font: u32| {
            font.wrapping_sub(g::AEE_FONT_NORMAL) < g::AEE_FONT_TOTAL - g::AEE_FONT_NORMAL
        };
        if c.class != "IStatic" && (!within(text) || !within(title)) {
            return Ok(());
        }
        c.font_text = text;
        c.font_title = title;
        // The static's setter also scrolls back to the top, which is a state a
        // caller reads: `IStatic::GoToLine` answers the index it is replacing.
        if c.class == "IStatic" {
            c.scroll_line = 0;
        }
        Ok(())
    }

    pub fn fonts(&self, addr: u32) -> HResult<(u32, u32)> {
        let c = self.peek(addr)?;
        Ok((c.font_text, c.font_title))
    }

    /// `IStatic::SetText`, the one setter that carries its fonts with it.
    pub fn static_set_text(
        &mut self,
        mem: &mut Mem,
        addr: u32,
        title: u32,
        text: u32,
        font_title: u32,
        font_text: u32,
    ) -> HResult<u32> {
        let title = Self::own_str(mem, title)?;
        let text = Self::own_str(mem, text)?;
        let c = self.control(addr)?;
        // The delivered body
        // string, so a zero font here means the setter's own default (title BOLD, body NORMAL,
        //) and the index the control is scrolled to goes back to the top.
        let (default_text, default_title) = Control::font_defaults(c.class).unwrap_or((0, 0));
        c.font_text = if font_text == 0 {
            default_text
        } else {
            font_text
        };
        c.font_title = if font_title == 0 {
            default_title
        } else {
            font_title
        };
        c.scroll_line = 0;
        c.title = title;
        c.text = text;
        // hands back `AStatic_SetTextEx(pText, NULL, FALSE)`, whose first act with neither
        // text nor stream is `return FALSE`: a title alone is not a body.
        Ok(u32::from(text != 0))
    }

    /// `IStatic::SetTextEx(..., bAppend)`: the shipped body only resets when it is *not* appending,
    ///so the new text is added to what the control already holds.
    pub fn append_text(&mut self, mem: &mut Mem, addr: u32, text: u32) -> HResult<u32> {
        let Some(add) = Self::cut_str(mem, text)? else {
            return Ok(0);
        };
        let held = self.peek(addr)?.text;
        let mut units = if held == 0 {
            Vec::new()
        } else {
            let mut old = mem.read_u16str(held, 4096)?;
            old.truncate(old.iter().position(|c| *c == 0).unwrap_or(old.len()));
            old
        };
        let mut more = mem.read_u16str(add, 4096)?;
        more.truncate(more.iter().position(|c| *c == 0).unwrap_or(more.len()));
        units.extend_from_slice(&more);
        let at = mem.alloc((units.len() as u32 + 1) * 2, 2)?;
        mem.write_u16str(at, &units)?;
        self.control(addr)?.text = at;
        Ok(1)
    }

    pub fn set_text(&mut self, mem: &mut Mem, addr: u32, text: u32) -> HResult<u32> {
        let taken = Self::own_str(mem, text)?;
        self.control(addr)?.text = taken;
        Ok(u32::from(taken != 0))
    }

    pub fn text(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.text)
    }

    /// `ITextCtl::SetText(psz, cch)`. `cch` is a character count whose
    /// negative value means "to the end of the string", and the body clamps it twice: first to the
    /// string's own length, then to `m_nMaxChars` — **32** on a fresh
    /// control (`DEF_TEXT_SIZE`,/). A zero ceiling stores an empty string, because the
    /// else-branch sets `cch = 0` whenever `psz && m_nMaxChars` is false.
    pub fn edit_text(&mut self, mem: &mut Mem, addr: u32, text: u32, cch: i32) -> HResult<u32> {
        if text == 0 {
            return Ok(0);
        }
        let mut units = mem.read_u16str(text, 4096)?;
        units.truncate(units.iter().position(|c| *c == 0).unwrap_or(units.len()));
        let max = usize::from(self.peek(addr)?.max_chars);
        if max == 0 {
            units.clear();
        } else {
            let mut want = if cch < 0 { units.len() } else { cch as usize };
            want = want.min(units.len()).min(max);
            units.truncate(want);
        }
        let len = units.len() as u32;
        let at = mem.alloc((len + 1) * 2, 2)?;
        mem.write_u16str(at, &units)?;
        let c = self.control(addr)?;
        c.text = at;
        c.caret = len;
        Ok(1)
    }

    /// `ITextCtl::GetText(pBuffer, nMaxChars)`. The body's shape is the
    /// one thing that is certain: a short buffer is **truncated** rather than refused ( copies
    /// through `WSTRNCOPYN`, and its answer is not consulted) and the return is TRUE whenever the
    /// text object exists, not "was it all copied". The unit of `nMaxChars` is what two witnesses
    /// disagree about: the header calls it a count of characters, while the body hands it straight to
    /// a helper whose parameter is `cbDest` — a byte count. This reads it as characters with the
    /// terminator inside the count, which is the header's contract.
    pub fn copy_text(&self, mem: &mut Mem, addr: u32, buffer: u32, max_chars: i32) -> HResult<u32> {
        let src = self.peek(addr)?.text;
        if buffer == 0 || src == 0 {
            return Ok(0);
        }
        let room = (max_chars.max(0) as usize).saturating_sub(1);
        let mut units = mem.read_u16str(src, 4096)?;
        units.truncate(units.iter().position(|c| *c == 0).unwrap_or(units.len()));
        units.truncate(room);
        // `room` is everything but the terminator, so `write_u16str` fills exactly the caller's
        // `nMaxChars` units and one unit past it is not written.
        mem.write_u16str(buffer, &units)?;
        Ok(1)
    }

    pub fn set_max_chars(&mut self, addr: u32, max: u32) -> HResult<()> {
        self.control(addr)?.max_chars = max as u16;
        Ok(())
    }

    pub fn caret(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.caret)
    }

    pub fn set_caret(&mut self, addr: u32, at: u32) -> HResult<()> {
        self.control(addr)?.caret = at;
        Ok(())
    }

    pub fn set_selection(&mut self, addr: u32, bits: u32) -> HResult<()> {
        self.control(addr)?.sel_bits = bits;
        Ok(())
    }

    pub fn selection(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.sel_bits)
    }

    pub fn add_item(
        &mut self,
        mem: &mut Mem,
        addr: u32,
        id: u16,
        text: u32,
        data: u32,
    ) -> HResult<u32> {
        let taken = Self::own_str(mem, text)?;
        let c = self.control(addr)?;
        // The delivered body
        // thing an id is checked for is duplication, and that check is skipped entirely
        // while `MP_ALLOW_DUPE_IDS` is on. The shipped "we cannot show anything" gate is
        // `!pImage && !bSingleFrame && !pText && (!pszResText || !wText)` — a single-frame
        // menu takes a label-less item, and so does any menu that loads the text itself out of
        // `pszResText` + `wText`. This store keeps a label and nothing else, so an item
        // arriving with no text is the one shape those two waivers cannot be followed into.
        if taken == 0 {
            return Ok(0);
        }
        if c.item(id).is_some() && c.props & MP_ALLOW_DUPE_IDS == 0 {
            return Ok(0);
        }
        c.items.push(Item {
            id,
            text: taken,
            data,
            time: (0, 0),
        });
        Ok(1)
    }

    /// `IMenuCtl::AddItemEx`, whose argument is a `CtlAddItem` the guest laid out in its own memory.
    pub fn add_item_ex(&mut self, mem: &mut Mem, addr: u32, pai: u32) -> HResult<u32> {
        let at =
            |name: &str| abi::offset_of("CtlAddItem", name).expect("CtlAddItem is in the tables");
        let text = mem.read_u32(pai + at("pText"))?;
        let id = mem.read_u16(pai + at("wItemID"))? as u16;
        let data = mem.read_u32(pai + at("dwData"))?;
        self.add_item(mem, addr, id, text, data)
    }

    pub fn item_data(&self, mem: &mut Mem, addr: u32, id: u16, out: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        let Some(at) = c.item(id) else { return Ok(0) };
        if out != 0 {
            mem.write_u32(out, c.items[at].data)?;
        }
        Ok(1)
    }

    /// `IMenuCtl::GetItem`, the reader that goes with `AddItemEx`: the caller hands in a
    /// `CtlAddItem` and gets back the three words [`Item`] keeps. The other fields of the record
    /// (image, resource name, font) are left as the caller wrote them, because the host's items
    /// never carried any — see [`Widgets::add_item_ex`], which reads the same three.
    pub fn get_item(&self, mem: &mut Mem, addr: u32, id: u16, out: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        let Some(at) = c.item(id) else { return Ok(0) };
        if out != 0 {
            let at_field = |name: &str| {
                abi::offset_of("CtlAddItem", name).expect("CtlAddItem is in the tables")
            };
            mem.write_u32(out + at_field("pText"), c.items[at].text)?;
            mem.write_u16(out + at_field("wItemID"), u32::from(id))?;
            mem.write_u32(out + at_field("dwData"), c.items[at].data)?;
        }
        Ok(1)
    }

    /// `IMenuCtl::SetItem`: update the fields of an item that already exists, as named by `flags`.
    ///
    /// The mask is the header's own `MSIF_*`: a field it does not name keeps the value it had, which
    /// is how a caller changes one row's text without disturbing its data. `wItemID` inside the record
    /// is *not* read — the item is named by `id`, and the header's `SetItem` is not a way to renumber
    /// one. `MSIF_IMAGE` and `MSIF_FONT` name fields this host's items never carried, so those masks
    /// have nothing to update.
    pub fn set_item(
        &mut self,
        mem: &mut Mem,
        addr: u32,
        id: u16,
        flags: u16,
        item: u32,
    ) -> HResult<u32> {
        let c = self.control(addr)?;
        let Some(at) = c.item(id) else { return Ok(0) };
        if item == 0 {
            return Ok(0);
        }
        let at_field =
            |name: &str| abi::offset_of("CtlAddItem", name).expect("CtlAddItem is in the tables");
        if u32::from(flags) & g::MSIF_TEXT != 0 {
            let taken = Self::own_str(mem, mem.read_u32(item + at_field("pText"))?)?;
            c.items[at].text = taken;
        }
        if u32::from(flags) & g::MSIF_DATA != 0 {
            c.items[at].data = mem.read_u32(item + at_field("dwData"))?;
        }
        Ok(1)
    }

    /// `IMenuCtl::MoveItem`: move an item one place up or down.
    ///
    /// The third argument is a **direction**, not a position: the header names -1 as up and 1 as
    /// down, and moves the item by one place. Anything else — including zero — leaves the list alone,
    /// and an item already at the end it is being moved toward stays where it is. The header's own
    /// result is `void`, so the zero here is what the handler returns to a caller that reads nothing.
    pub fn move_item(&mut self, addr: u32, id: u16, dir: i32) -> HResult<u32> {
        let c = self.control(addr)?;
        let Some(at) = c.item(id) else { return Ok(0) };
        let to = if dir < 0 {
            at.checked_sub(1)
        } else if dir > 0 {
            (at + 1 < c.items.len()).then_some(at + 1)
        } else {
            None
        };
        if let Some(to) = to {
            c.items.swap(at, to);
        }
        Ok(0)
    }

    /// `MenuCtl_New`'s other seeding step: the device's item-style pair, copied into
    /// the control's own two records, which is where a control that is never given a style keeps what
    /// it was born with — and why `GetStyle` answers something before anybody calls `SetStyle`.
    pub fn seed_menu_style(
        &mut self,
        addr: u32,
        normal: [u8; 8],
        selected: [u8; 8],
    ) -> HResult<()> {
        let c = self.control(addr)?;
        c.style = [normal, selected];
        Ok(())
    }

    /// `IMenuCtl::SetStyle(pNormal, pSel)` — `void`, and the two halves are taken **independently**:
    /// the body is `if (pNormal) aee_memmove(+356, pNormal, 8); if (pSel) aee_memmove(+364, pSel, 8);
    /// return Menu_Recalc(this);`. So naming one record and passing null for the other changes exactly
    /// the one, and naming nothing still recalculates. Eight bytes is the width the body moves, not
    /// The header
    pub fn set_style(&mut self, addr: u32, mem: &Mem, normal: u32, selected: u32) -> HResult<()> {
        let read = |at: u32| -> HResult<Option<[u8; 8]>> {
            if at == 0 {
                return Ok(None);
            }
            let bytes = mem.read_bytes(at, 8)?;
            let mut rec = [0u8; 8];
            rec.copy_from_slice(&bytes);
            Ok(Some(rec))
        };
        // Both reads happen before the control is held: `mem.read_bytes` and the record borrow
        // different parts of the run, and the second read must not see the first one's write.
        let a = read(normal)?;
        let b = read(selected)?;
        let c = self.control(addr)?;
        if let Some(rec) = a {
            c.style[0] = rec;
        }
        if let Some(rec) = b {
            c.style[1] = rec;
        }
        Ok(())
    }

    /// `IMenuCtl::GetStyle(pNormal, pSel)` — the same independence, in the other direction: each
    /// non-null argument gets the control's eight bytes, and a null one is skipped rather than
    /// refusing the pair. This is the gate this host used to apply to **both** pointers at once, which
    /// turned a caller that asked only for the selected record into a call that wrote nothing.
    pub fn get_style(&self, addr: u32, mem: &mut Mem, normal: u32, selected: u32) -> HResult<()> {
        let c = self.peek(addr)?;
        for (at, which) in [(normal, 0usize), (selected, 1usize)] {
            if at != 0 {
                mem.write_bytes(at, &c.style[which])?;
            }
        }
        Ok(())
    }

    /// `IMenuCtl::SetColors(pc)` — `void`. With a record, each mask bit takes one word of it, and only
    /// the bits the body tests; with null, the nine words go back to the system indices the
    /// constructor left them at. The caller's mask word is never copied into the control, so the
    /// control has no record of which fields a caller named — see [`MENU_COLOR_BITS`] for the table
    /// and [`Control::colors`] for why the values are indices.
    pub fn set_colors(&mut self, addr: u32, mem: &Mem, pc: u32) -> HResult<()> {
        // The body reads a word only *inside* the `if` that names it — `if (mask & 1) *(a1+444) =
        // *(a2+4)` — so a caller that points at a short record and names one field gets that field
        // rather than a fault. Collecting all nine words first is the shape that would end a run the
        // shipped member answers, which is why the read sits down the branch, next to the write.
        let mask = if pc == 0 { 0 } else { mem.read_u16(pc)? as u32 };
        let c = self.control(addr)?;
        if pc == 0 {
            // The shipped reset writes the nine words and leaves the mask word alone — and nothing in
            // the member ever writes that word, so a caller that reads the record back finds the zero
            // the allocation left, not the mask it sent.
            for i in 1..c.colors.len() {
                c.colors[i] = MENU_COLORS[i];
            }
        } else {
            for (bit, dest) in MENU_COLOR_BITS {
                if mask & bit != 0 {
                    let word = mem.read_u32(pc + dest as u32 * 4)?;
                    c.colors[dest] = word;
                }
            }
        }
        Ok(())
    }

    /// `IMenuCtl::GetColors(pc)` — `void`, and the body is one gate and one copy:
    /// `if (pc) aee_memmove(pc, &m_c, 40)`. Every word of the control's record goes out, the mask word
    /// included, whether or not anything ever set it — so this answers the control's own state rather
    /// than a theme. The bar this host *draws* behind the selected row is still its own constant; the
    /// shipped member turns the stored index into a colour by asking the display
    /// (`Menu_InitColors`: three `IDisplay::SetColor(CLR_USER_*, value)` calls), and that resolution is
    /// recorded against the display, not invented here.
    pub fn menu_colors(&self, addr: u32, mem: &mut Mem, out: u32) -> HResult<()> {
        if out == 0 {
            return Ok(());
        }
        let c = self.peek(addr)?;
        for (i, word) in c.colors.iter().enumerate() {
            mem.write_u32(out + i as u32 * 4, *word)?;
        }
        Ok(())
    }

    /// `IMenuCtl::SetItemTime`: record the pair of words a caller attaches to an item. What the two
    /// mean is not published anywhere this tree can read — the ABI table names the type but carries
    /// none of its members — so they are stored and handed back unchanged rather than interpreted.
    pub fn set_item_time(&mut self, addr: u32, id: u16, t0: u16, t1: u16) -> HResult<()> {
        let c = self.control(addr)?;
        if let Some(at) = c.item(id) {
            c.items[at].time = (t0, t1);
        }
        Ok(())
    }

    /// `IMenuCtl::GetItemTime`: the pair [`Widgets::set_item_time`] stored.
    ///
    /// The header
    /// the result is the **start time**. An item the control does not have answers -1, not zero — a
    /// calendar menu asks this to decide whether there is a row to draw, and a start time of zero
    /// minutes is midnight, which is a real row.
    pub fn item_time(&self, mem: &mut Mem, addr: u32, id: u16, out: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        let Some(at) = c.item(id) else {
            return Ok(u32::MAX);
        };
        if out != 0 {
            mem.write_u16(out, u32::from(c.items[at].time.1))?;
        }
        Ok(u32::from(c.items[at].time.0))
    }

    /// `IMenuCtl::SetOwnerDrawCB`: take the pack's callback for the rows it names, and its context.
    /// The host paints its menus itself and does not call the callback yet, so this is recorded
    /// rather than honoured; what a caller can be told is that the host has it, which is more than
    /// dropping it on the floor says.
    pub fn set_owner_draw(&mut self, addr: u32, cb: u32, data: u32, mask: u16) -> HResult<u32> {
        let c = self.control(addr)?;
        c.owner_draw = cb;
        c.owner_draw_data = data;
        c.owner_draw_mask = mask;
        Ok(1)
    }

    /// `IDateCtl::SetJulianDay`, which is the one place the shipped control writes its selection.
    ///`JDayToDate` refuses two things and refuses them **before** writing
    /// the caller's three integers: a negative day number, and one at or past
    /// `((uint32)0xffffffff - JDAY_OFFSET*4)/4`, then a year above 9999. A refusal
    /// therefore leaves the year/month/day at zero, zeroes the stored day number and weekday, and
    /// answers FALSE — the number the caller passed is not kept.
    pub fn set_julian_day(&mut self, addr: u32, day: i32) -> HResult<u32> {
        let c = self.control(addr)?;
        Ok(u32::from(c.select_julian(day)))
    }

    /// `IDateCtl::GetJulianDay`: the stored number, which is zero after a refused set and zero for a
    /// control nobody has given a date.
    pub fn julian_day(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.julian() as u32)
    }

    /// `IDateCtl::SetDate`: convert with `DateToJDay`, hand the result to
    /// `SetJulianDay` **whatever it is**, and answer FALSE only when the conversion came out zero and
    /// the triple is not the epoch itself. `DateToJDay` refuses `year < 1`, a month outside 1..12 and
    /// a day outside 1..31 — but it does **not** refuse a day past the end of its own
    /// month: 31 April rolls into 1 May, because the March-based formula simply adds it (,
    /// and the header says the same). Clamping would be a different control.
    pub fn set_date(&mut self, addr: u32, year: i32, month: i32, day: i32) -> HResult<u32> {
        let c = self.control(addr)?;
        let julian = date_to_julian(year, month, day);
        c.select_julian(julian);
        Ok(u32::from(
            julian != 0 || (year, month, day) == (EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY),
        ))
    }

    /// `IDateCtl::GetDate`: the stored triple, one argument at a time, and
    /// **always TRUE** — a control that was never set answers 0/0/0 rather than refusing.
    pub fn date(&self, mem: &mut Mem, addr: u32, py: u32, pm: u32, pd: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        for (at, value) in [(py, c.date.0), (pm, c.date.1 as i32), (pd, c.date.2 as i32)] {
            if at != 0 {
                mem.write_u32(at, value as u32)?;
            }
        }
        Ok(1)
    }

    /// `IDateCtl::GetDayOfWeek`: the stored weekday, which is
    /// `jday % 7` with `JDAY_OFFSET` putting the Sunday before 1970-01-01 at zero — so 0 is Sunday,
    /// the `DOW_SUNDAY..DOW_SATURDAY` numbering of the header. A refused or never-done set
    /// leaves it at 0, which reads as Sunday.
    pub fn day_of_week(&self, addr: u32) -> HResult<u32> {
        Ok(u32::from(self.peek(addr)?.weekday()))
    }

    /// `IDateCtl::SetActiveDayMask`: which days of the month the control lets the user land on. The
    /// host keeps it and does not act on it, because it has no picker to restrict.
    pub fn set_day_mask(&mut self, addr: u32, mask: u32) -> HResult<()> {
        self.control(addr)?.day_mask = mask;
        Ok(())
    }

    /// Choose the language a date control answers its words in, and report which one was taken — a
    /// tag this host does not carry lands on [`locale::DEFAULT`] rather than being a failure, because
    /// the run can say which language it got.
    pub fn set_language(&mut self, tag: &str) -> &'static str {
        let language = locale::find(tag);
        self.language = language;
        language.tag
    }

    /// The language this host currently answers in.
    pub fn language(&self) -> &'static str {
        self.language.tag
    }

    /// Write one of the host's own words into the caller's `AECHAR` buffer and report how much got in.
    ///
    /// `nMaxChars` is **bytes**: `IDateCtl_GetDateString` opens with `nRemaining = nMaxSize /
    /// sizeof(AECHAR)` and the two name getters pass the same number straight to
    /// `ISHELL_LoadResString`, whose size argument is a byte count. So the budget is `max/2` units with
    /// the terminator **inside** it. Reading `max` as a character count — which is what this door did —
    /// writes twice what the caller's buffer holds, into memory the pack never reserved for us.
    ///
    /// The first number is what `pnChars` gets (the shipped getter sets it from `WSTRLEN` of the buffer
    /// afterwards, the delivered body//); the second says whether the whole word fit.
    fn put_words(mem: &mut Mem, buf: u32, max_bytes: i32, text: &str) -> HResult<(usize, bool)> {
        let room = max_bytes / 2;
        if buf == 0 || room <= 0 {
            return Ok((0, false));
        }
        let mut n = 0usize;
        for ch in text.chars() {
            if n as i32 >= room - 1 {
                break;
            }
            mem.write_u16(buf + (n as u32) * 2, u32::from(ch as u16))?;
            n += 1;
        }
        mem.write_u16(buf + (n as u32) * 2, 0)?;
        Ok((n, n == text.chars().count()))
    }

    /// `pBuffer + nChars + 1` in `AECHAR *` arithmetic — one unit **past** the
    /// terminator, which on a refusal is just `pBuffer + 2`. The header's "NULL if unsuccessful"
    /// is not what the body does, and the body is the witness.
    fn word_end(buf: u32, chars: usize) -> u32 {
        buf.wrapping_add(((chars + 1) * 2) as u32)
    }

    /// `IDateCtl::GetDayString` (the delivered body + the helper at). The helper's only
    /// gate is the index, `nDay >= 0 && nDay <= 6` — unlike the month getter it does **not** test the
    /// buffer first, and a refusal leaves the buffer exactly as it was and sets `pnChars` to zero.
    pub fn day_string(
        &self,
        mem: &mut Mem,
        addr: u32,
        buf: u32,
        max: i32,
        pchars: u32,
    ) -> HResult<u32> {
        let c = self.peek(addr)?;
        let at = c.weekday() as usize;
        let name = self.language.days.get(at);
        let (n, _) = match name {
            Some(word) => Self::put_words(mem, buf, max, word)?,
            None => (0, false),
        };
        if pchars != 0 {
            mem.write_u32(pchars, n as u32)?;
        }
        Ok(Self::word_end(buf, n))
    }

    /// `IDateCtl::GetMonthString` (the delivered body + the helper at). This one tests
    /// `pBuff && nMonth >= 1 && nMonth <= 12`, so a control that was never given a date — the stored
    /// month is zero — refuses rather than answering the twelfth month's name.
    pub fn month_string(
        &self,
        mem: &mut Mem,
        addr: u32,
        buf: u32,
        max: i32,
        pchars: u32,
    ) -> HResult<u32> {
        let c = self.peek(addr)?;
        let at = c.date.1.checked_sub(1).and_then(|m| {
            if (m as usize) < self.language.months.len() {
                Some(m as usize)
            } else {
                None
            }
        });
        let (n, _) = match (buf, at) {
            (0, _) => (0, false),
            (_, Some(i)) => Self::put_words(mem, buf, max, self.language.months[i])?,
            _ => (0, false),
        };
        if pchars != 0 {
            mem.write_u32(pchars, n as u32)?;
        }
        Ok(Self::word_end(buf, n))
    }

    /// `IDateCtl::GetDateString`. Three things about this slot are not what the
    /// name suggests: it returns a **boolean**, not a string pointer (the header's own declaration,
    /// and the body's `return(FALSE)` at/); `nMaxChars` is bytes; and of the
    /// `DFMT_*` word **only** `dwDateFormat & DFMT_FORMAT_MASK` is examined, so the
    /// `DFMT_SLASH_SEP`/`DASH_SEP`/`SPACE_SEP` bits this door used to honour are dead code in the
    /// shipped member — the delimiter is fixed per format (`'/'` at//, `'.'` for
    /// `DFMT_INT_YYYY_MM_DD`). An unnamed format falls to `default: return(FALSE)` at, and
    /// the control's own properties are **not** consulted here.
    ///
    /// The paddings come from the case bodies: the day is written with pad `0` (,), the
    /// `YY` forms write `nYear % 100` after a literal `'` (,) with pad 2, and the
    /// numeric forms put pad 2 on the **year** (,,) — which is a no-op for a
    /// four-digit year — so no month and no day is ever zero-padded. That is why the header's own
    /// examples read `10/15/2000` and `2001/6/1`. An `_MON_`/`_YY` form truncates the month name to
    /// `ABBREV_LEN` = 3 characters (,; the header).
    ///
    /// A buffer too small keeps the partial string — `WriteChar` and `WWRITELONGEX` write as far as
    /// they can and the switch then returns FALSE (,) — and `pnChars`, which was
    /// zeroed at, stays zero.
    pub fn date_string(
        &self,
        mem: &mut Mem,
        addr: u32,
        buf: u32,
        max: i32,
        pchars: u32,
        format: u32,
    ) -> HResult<u32> {
        let c = self.peek(addr)?;
        if buf == 0 {
            return Ok(0);
        }
        if pchars != 0 {
            mem.write_u32(pchars, 0)?;
        }
        let (y, m, d) = (c.date.0, c.date.1, c.date.2);
        // `ABBREV_LEN`, the header — the width the `_MON_` forms cut the month name down to.
        const ABBREV: usize = 3;
        let fmt = format & g::DFMT_FORMAT_MASK;
        // The six name forms call `IDateCtl_GetMonthString`, which refuses a month outside 1..12
        // and then has its caller stamp `psz[3] = 0` over whatever the buffer
        // already held — copying that would be copying the caller's old bytes. The four numeric forms
        // print the stored numbers and never ask for a name, so they answer for month 0
        // exactly as the shipped body does rather than refusing with it.
        let month: &str = match fmt {
            g::DFMT_DD_MONTH_YYYY..=g::DFMT_MON_DD_YY => match (m as usize).checked_sub(1) {
                Some(i) if i < self.language.months.len() => self.language.months[i],
                _ => return Ok(0),
            },
            _ => "",
        };
        let short_y = y.rem_euclid(100);
        let abbrev = |name: &str| -> String { name.chars().take(ABBREV).collect() };
        // `pad 2` on a year is a minimum width, so a year under 10 is zero-padded (,,,
        //,). For four-digit years it changes nothing, which is why the header's
        // examples never show it.
        let year = format!("{y:02}");
        let text = match fmt {
            g::DFMT_DD_MONTH_YYYY => format!("{d} {month} {year}"),
            g::DFMT_DD_MON_YYYY => format!("{d} {} {year}", abbrev(month)),
            g::DFMT_DD_MON_YY => format!("{d} {} '{short_y:02}", abbrev(month)),
            g::DFMT_MONTH_DD_YYYY => format!("{month} {d}, {year}"),
            g::DFMT_MON_DD_YYYY => format!("{} {d}, {year}", abbrev(month)),
            g::DFMT_MON_DD_YY => format!("{} {d}, '{short_y:02}", abbrev(month)),
            g::DFMT_MM_DD_YYYY => format!("{m}/{d}/{year}"),
            g::DFMT_DD_MM_YYYY => format!("{d}/{m}/{year}"),
            g::DFMT_YYYY_MM_DD => format!("{year}/{m}/{d}"),
            g::DFMT_INT_YYYY_MM_DD => format!("{year}.{m}.{d}"),
            _ => return Ok(0),
        };
        let (n, fits) = Self::put_words(mem, buf, max, &text)?;
        if !fits {
            return Ok(0);
        }
        if pchars != 0 {
            mem.write_u32(pchars, n as u32)?;
        }
        Ok(1)
    }

    /// `ITextCtl::SetSoftKeyMenu`: the old menu is released, the new one
    /// is held, and — unless `CP_STATIC` — the mode in force is added to it as a row.
    /// This host draws no soft keys, so the row is the one part not added; the references are the
    /// caller's to take and give back, which is why the object that was there comes out of here.
    pub fn set_softkey_menu(&mut self, addr: u32, menu: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        Ok(std::mem::replace(&mut c.softkey_menu, menu))
    }

    /// `ITextCtl::SetInputMode`: set the mode and answer the one that was in force. The published
    /// setter and getter return the same type, so the result is the previous mode rather than a
    /// status code.
    pub fn set_input_mode(&mut self, addr: u32, mode: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        let previous = c.input_mode;
        c.input_mode = mode;
        Ok(previous)
    }

    /// `ITextCtl::GetInputMode`: the mode in force, written into the caller's info record when it
    /// passes one. The record's name field is left as the caller wrote it — this host has no
    /// localised name for a mode, and an invented one is a string a caller would put on screen.
    pub fn input_mode(&self, mem: &mut Mem, addr: u32, info: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        if info != 0 {
            let at = |name: &str| {
                abi::offset_of("AEETextInputModeInfo", name)
                    .expect("AEETextInputModeInfo is in the tables")
            };
            mem.write_u32(info + at("tmMode"), c.input_mode)?;
        }
        Ok(c.input_mode)
    }

    /// `ITextCtl::EnumModeInit`: start the walk of the input modes again.
    pub fn enum_mode_init(&mut self, addr: u32) -> HResult<()> {
        self.control(addr)?.enum_mode = 0;
        Ok(())
    }

    /// `ITextCtl::EnumNextMode`: the modes this host offers, one per call, 0 once the walk is over.
    /// There is exactly one — a desktop keyboard has no input method to switch between — so the
    /// current mode is the whole list and the call after it ends the walk. The name field is left
    /// alone for the reason [`Widgets::input_mode`] gives.
    pub fn enum_next_mode(&mut self, mem: &mut Mem, addr: u32, info: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        let mode = if c.enum_mode == 0 { c.input_mode } else { 0 };
        c.enum_mode += 1;
        if info != 0 && mode != 0 {
            let at = |name: &str| {
                abi::offset_of("AEETextInputModeInfo", name)
                    .expect("AEETextInputModeInfo is in the tables")
            };
            mem.write_u32(info + at("tmMode"), mode)?;
        }
        Ok(mode)
    }

    /// `ITextCtl::SetPropertiesEx` / `GetPropertiesEx`: the extended property word, kept and handed
    /// back unchanged. The paint path does not consult it, the same as for `SetProperties`.
    pub fn set_props_ex(&mut self, addr: u32, props: u32) -> HResult<()> {
        self.control(addr)?.props_ex = props;
        Ok(())
    }

    pub fn props_ex(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.props_ex)
    }

    /// `IMenuCtl::Sort`: order the items by their text or by their user data.
    ///
    /// The header
    /// not separate keep the order the pack added them in. A name key has to be read out of guest
    /// memory *before* the sort: comparing through the pointer would order by address.
    ///
    /// `MCS_CUSTOM` is not handled here — it asks for the caller's own comparison callback, which is
    /// code in the pack, and the handler refuses that one by name rather than sorting by something the
    /// caller did not ask for.
    pub fn sort_items(&mut self, mem: &Mem, addr: u32, kind: u32) -> HResult<()> {
        let c = self.control(addr)?;
        match kind {
            g::MCS_NAME_ASCENDING | g::MCS_NAME_DESCENDING => {
                let mut keyed: Vec<(String, Item)> = Vec::with_capacity(c.items.len());
                for item in c.items.drain(..) {
                    let key = Self::read_words(mem, item.text)?;
                    keyed.push((key, item));
                }
                // `String`'s ordering compares UTF-8 bytes, which is code-point order — the ordering a
                // menu's own rows are shown in.
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                if kind == g::MCS_NAME_DESCENDING {
                    keyed.reverse();
                }
                c.items = keyed.into_iter().map(|(_, item)| item).collect();
            }
            g::MCS_PARAM_ASCENDING | g::MCS_PARAM_DESCENDING => {
                c.items.sort_by_key(|item| item.data);
                if kind == g::MCS_PARAM_DESCENDING {
                    c.items.reverse();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The `AECHAR` string at `at`, as a `String`.
    ///
    /// Used as a sort key, so a unit that is not a character is skipped rather than failing the whole
    /// read: one unreadable row should not stop a menu from being ordered. The length cap is there for
    /// the same reason — a text pointer that does not point at text would otherwise walk the arena.
    fn read_words(mem: &Mem, at: u32) -> HResult<String> {
        let mut out = String::new();
        if at == 0 {
            return Ok(out);
        }
        for i in 0..MAX_KEY_CHARS {
            let unit = mem.read_u16(at + i * 2)? as u16;
            if unit == 0 {
                break;
            }
            if let Some(ch) = char::from_u32(u32::from(unit)) {
                out.push(ch);
            }
        }
        Ok(out)
    }

    pub fn delete_item(&mut self, addr: u32, id: u16) -> HResult<u32> {
        let c = self.control(addr)?;
        let Some(at) = c.item(id) else { return Ok(0) };
        c.items.remove(at);
        c.sel = c.sel.min(c.items.len().saturating_sub(1));
        c.enum_sel = 0;
        Ok(1)
    }

    pub fn delete_all(&mut self, addr: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        c.items.clear();
        c.sel = 0;
        c.enum_sel = 0;
        Ok(1)
    }

    pub fn set_sel(&mut self, addr: u32, id: u16) -> HResult<()> {
        let c = self.control(addr)?;
        if let Some(at) = c.item(id) {
            c.sel = at;
            c.enum_sel = 0;
        }
        Ok(())
    }

    /// The id of the selected item, or 0 with nothing selected.
    pub fn sel(&self, addr: u32) -> HResult<u32> {
        let c = self.peek(addr)?;
        Ok(c.items.get(c.sel).map(|it| it.id as u32).unwrap_or(0))
    }

    pub fn item_count(&self, addr: u32) -> HResult<u32> {
        Ok(self.peek(addr)?.items.len() as u32)
    }

    pub fn item_id(&self, addr: u32, index: i32) -> HResult<u32> {
        let c = self.peek(addr)?;
        let id = if index < 0 {
            None
        } else {
            c.items.get(index as usize).map(|it| it.id)
        };
        Ok(id.unwrap_or(0) as u32)
    }

    pub fn enum_sel_init(&mut self, addr: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        c.enum_sel = 0;
        Ok(u32::from(!c.items.is_empty()))
    }

    pub fn next_sel(&mut self, addr: u32) -> HResult<u32> {
        let c = self.control(addr)?;
        let Some(id) = c.items.get(c.enum_sel).map(|it| it.id) else {
            return Ok(0);
        };
        c.enum_sel += 1;
        Ok(id as u32)
    }

    pub fn set_item_text(&mut self, mem: &mut Mem, addr: u32, id: u16, text: u32) -> HResult<()> {
        let taken = Self::own_str(mem, text)?;
        let c = self.control(addr)?;
        if taken != 0 {
            if let Some(at) = c.item(id) {
                c.items[at].text = taken;
            }
        }
        Ok(())
    }

    /// The rect of one row, which is how an applet that draws its own icons knows where to put them.
    pub fn item_rect(&self, mem: &mut Mem, addr: u32, index: i32, prc: u32) -> HResult<()> {
        let c = self.peek(addr)?;
        if prc == 0 || index < 0 {
            return Ok(());
        }
        let height = c.row_height();
        let (rows, first) = c.window();
        let row = (index as usize)
            .saturating_sub(first)
            .min(rows.saturating_sub(1)) as i16;
        Rect::new(c.rect.x, c.rect.y + row * height, c.rect.dx, height).write(mem, prc)
    }

    /// How many lines an `IStatic`'s body wraps to in its own width — the quantity the shipped
    /// `AStatic_Recalc` lays out as `m_nLines`. `IsScrollable` compares it with
    /// the lines its rect shows, and `GoToLine` refuses a line at or past it.
    fn static_lines(
        &self,
        mem: &mut Mem,
        display: &mut Display,
        disp: u32,
        addr: u32,
    ) -> HResult<i32> {
        let (text, font, dx) = {
            let c = self.peek(addr)?;
            (c.text, c.font_text, c.rect.dx)
        };
        if text == 0 || dx <= 0 {
            return Ok(0);
        }
        // The whole run has to be measured to learn how many boxes of `dx` it fills. Asking
        // `IDisplay::MeasureTextEx` with a budget of `dx` measures the longest prefix that *fits*,
        // which is never wider than `dx`, so the division below would answer one line for every text
        // and a long body in a short box would report "not scrollable" and refuse to move. An
        // unbounded budget is the other reading the slot supports (`nMaxWidth == -1` widens it to
        // `0x7FFFFFFF` in the body) and is the one this count needs.
        //
        // This is still a **character**-wrap count. `AStatic_Recalc` breaks paragraphs at spaces and
        // newlines and keeps a per-line table of `{offset, length, alignment}` at `[+52]`, so a text
        // with spaces can end up with more lines than this says; that table is the open gap, not the
        // measurement door.
        let width = display.measure_text(mem, disp, font, text, -1, -1, 0)?;
        Ok(i32::try_from(width.div_ceil(dx.max(1) as u32))
            .unwrap_or(i32::MAX)
            .max(1))
    }

    /// `IControl::IsScrollable` as `IStatic` implements it: `m_nLines > m_nPageLines`,
    ///where the page count is `(rect height - title height) / text line
    /// height`. A control of another class keeps the item-row rule menus have always
    /// used here.
    pub fn is_scrollable(
        &mut self,
        mem: &mut Mem,
        display: &mut Display,
        disp: u32,
        addr: u32,
    ) -> HResult<u32> {
        let (is_static, items, rows, title, font_title, font_text, dy) = {
            let c = self.peek(addr)?;
            let (rows, _) = c.window();
            (
                c.class == "IStatic",
                c.items.len(),
                rows,
                c.title,
                c.font_title,
                c.font_text,
                c.rect.dy,
            )
        };
        if !is_static {
            return Ok(u32::from(items > rows));
        }
        let lines = self.static_lines(mem, display, disp, addr)?;
        let title_h = if title != 0 {
            i32::from(Control::line(font_title)) + 2
        } else {
            0
        };
        let body_h = i32::from(Control::line(font_text)).max(1);
        let page = (i32::from(dy) - title_h) / body_h;
        Ok(u32::from(lines > page))
    }

    /// `IStatic::SizeToFit`. The caller's rect is the box to lay the text out *in* — its width is
    /// the wrap — and it comes back as the box that holds the text.
    pub fn size_to_fit(
        &mut self,
        mem: &mut Mem,
        display: &mut Display,
        disp: u32,
        addr: u32,
        prc: u32,
    ) -> HResult<()> {
        if prc == 0 {
            return Ok(());
        }
        let (class, title, text, font_title, font_text, rect) = {
            let c = self.peek(addr)?;
            (c.class, c.title, c.text, c.font_title, c.font_text, c.rect)
        };
        let wanted = Rect::read(mem, prc)?;
        let wrap = wanted.dx.max(1) as usize;
        let mut height = 0i16;
        let mut width = 0i16;
        for (at, font) in [(title, font_title), (text, font_text)] {
            if at == 0 {
                continue;
            }
            let line = Control::line(font);
            let measured =
                display.measure_text(mem, disp, font, at, -1, wanted.dx as i32, 0)? as usize;
            width = width.max(measured.min(wrap) as i16);
            height += (measured.div_ceil(wrap).max(1) as i16) * line;
        }
        if class == "IStatic" {
            // The delivered body
            // *shrinks* to it — a box taller than the text keeps its height — while the width and the
            // origin are the caller's and come back untouched. The settled rect is also what the
            // control is placed at, through `SetRect`, which ignores one shorter than a line.
            let mut settled = wanted;
            if height < settled.dy {
                settled.dy = height;
            }
            if settled.dy >= Control::line(font_text) {
                self.control(addr)?.rect = settled;
            }
            return settled.write(mem, prc);
        }
        Rect::new(rect.x, rect.y, width.max(1), height.max(1)).write(mem, prc)
    }

    // ------------------------------------------------------------------------- the pixels

    /// Paint the control inside its own rect on the applet's screen, answering with the number of
    /// rows drawn. A control with no rect has nowhere to draw, which is why `SetRect` is what makes
    /// a first `Redraw` do anything at all.
    ///
    /// A menu paints its items and everything else its title over its body, because that is the
    /// whole difference between the classes at this level: the item text of a menu is per-row, and
    /// the selection is marked with a filled bar behind its row, which is what reads as "this one"
    /// in a screenshot.
    pub fn redraw(
        &mut self,
        mem: &mut Mem,
        display: &mut Display,
        disp: u32,
        addr: u32,
    ) -> HResult<u32> {
        // `IHtmlViewer` paints its parsed document rather than the title/body pair every other
        // control carries; the two paths share only the control's rect.
        if self.peek(addr)?.class == "IHtmlViewer" {
            let rect = self.peek(addr)?.rect;
            if rect.is_empty() {
                return Ok(0);
            }
            // The `HVOPT_*` table is read here rather than at `SetOption` time: a pack may
            // set an option after loading the document, and the next `Redraw` is where it
            // has to show up. The document itself is cached on the page — a frame is not a
            // reason to parse the same bytes again. `HVP_SCROLLBAR` is not in that table at all
            // (it is an `IControl` property), so the control's own word is folded in here.
            let props = self.peek(addr)?.props;
            let options = {
                let html = self.html(addr)?;
                let mut options = crate::web::html::Options::from_table(&html.options);
                options.scrollbar = props & g::HVP_SCROLLBAR != 0;
                options
            };
            let doc = self.html_document(addr)?;
            // Pictures decode here, before the layout asks how tall they are: a `<img>` with no
            // `height` attribute gets one from the picture itself.
            let pictures = self.decode_images(addr, &doc)?;
            // The cursor is wherever `HandleEvent` last put it, and it is an index into
            // `focus_items` rather than into the elements — the layout needs the element itself.
            let focus = self.html_focus(addr, &doc)?.map(|(elem, _)| elem);
            if self.scratch == 0 {
                self.scratch = mem.alloc(8, 4).map_err(HleError::Mem)?;
            }
            let scratch = self.scratch;
            // Only the scrollbar needs to know how tall the page is — and so does a page that is
            // scrolled, because a scroll offset belongs to a rectangle. Measuring is a full layout
            // pass, cached against the parse and the width it measured.
            let mut scroll = self.html(addr)?.scroll;
            let content = if options.scrollbar || scroll > 0 {
                self.html_content_height(addr, &doc, &pictures, &options, rect, display)?
            } else {
                0
            };
            // `SetRect` is what makes a page that did not fit fit. An offset left over from the
            // smaller rectangle would start the page below its own last row and show nothing but
            // background, so the clamp is part of drawing rather than something the pack has to
            // know to ask for. It only ever moves the offset *back*, which is why it cannot drift
            // the way a frame that accumulated its own adjustments would.
            if scroll > 0 {
                let limit = content.saturating_sub(rect.dy).max(0);
                if scroll > limit {
                    scroll = limit;
                    self.html(addr)?.scroll = scroll;
                }
            }
            let transparent = props & g::HVP_TRANSPARENT != 0;
            // The page's edits travel with the frame: the layout folds them into the runs, and the
            // painter draws what it is handed, so this is the only place that knows both.
            let page = self.html(addr)?;
            let view = crate::web::html::View {
                focus,
                scroll,
                content,
                values: Some(&page.values),
                transparent,
                elapsed_ms: page.marquee_elapsed,
            };
            // The page paints inside its own rectangle and nowhere else. A scrolled row can straddle
            // the top edge, and `DrawText` fills its cell up to the *display's* clip rather than to
            // the bounds it was handed, so a clip is the one mechanism that keeps the frame inside
            // the control. Saving and restoring it is what a control does on a handset; `SetClipRect`
            // replaces rather than narrows, so the saved rect comes back exactly.
            let saved = display.clip_of(disp)?;
            Rect::new(rect.x, rect.y, rect.dx, rect.dy).write(mem, scratch)?;
            display.set_clip_rect(mem, disp, scratch)?;
            let painted = crate::web::html::render(
                &doc, &pictures, mem, display, disp, scratch, rect, &options, &view,
            );
            // Put the clip back whatever happened: a narrowed clip left behind would blank the rest
            // of the frame, and the error is on its way out anyway.
            saved.write(mem, scratch)?;
            display.set_clip_rect(mem, disp, scratch)?;
            return painted;
        }
        let c = self.control(addr)?;
        if c.rect.is_empty() {
            return Ok(0);
        }
        let row_h = c.row_height();
        let (rows, first) = c.window();
        let box_rect = c.rect;
        let menu = c.class == "IMenuCtl";
        let mut lines: Vec<(u32, u32, bool)> = Vec::new();
        if menu {
            for (i, it) in c.items.iter().enumerate().skip(first).take(rows.max(1)) {
                lines.push((it.text, c.font_text, i == c.sel));
            }
        } else {
            for (at, font) in [(c.title, c.font_title), (c.text, c.font_text)] {
                if at != 0 {
                    lines.push((at, font, false));
                }
            }
        }
        // One scratch `AEERect`, rewritten per row, is what `DrawText` aligns and clips against.
        if self.scratch == 0 {
            self.scratch = mem.alloc(8, 4)?;
        }
        let scratch = self.scratch;
        // Both shipped paints open the same way — `Menu_Draw` (`0x1acc`) and
        // `AStatic_Redraw` both begin with `IDisplay::DrawRect(po, &rect, -1, 2, 2)`, filling the
        // control's own rectangle with `CLR_USER_BACKGROUND` before a single character lands. A
        // repaint that skipped it could only ever add: a menu that replaced its six items with two
        // left the old labels showing through, which is what the second chess screen did.
        Rect::new(box_rect.x, box_rect.y, box_rect.dx, box_rect.dy).write(mem, scratch)?;
        display.draw_rect(
            mem,
            disp,
            scratch,
            RGB_NONE,
            g::CLR_USER_BACKGROUND,
            g::IDF_RECT_FILL,
        )?;
        if lines.is_empty() {
            return Ok(0);
        }
        let mut y = box_rect.y;
        let mut painted = 0;
        for (at, font, chosen) in lines {
            Rect::new(box_rect.x, y, box_rect.dx, row_h).write(mem, scratch)?;
            if chosen {
                display.draw_rect(mem, disp, scratch, SEL_BAR, SEL_BAR, g::IDF_RECT_FILL)?;
            }
            // The chosen row has a bar behind it already, so its text goes down transparent: an
            // opaque cell would paint over the bar and leave the selection looking exactly like
            // every other row — which is what the first chess run showed, a menu with six identical
            // lines and no visible cursor.
            let mut flags = g::IDF_ALIGN_LEFT | g::IDF_ALIGN_TOP;
            if chosen {
                flags |= g::IDF_TEXT_TRANSPARENT;
            }
            display.draw_text(
                mem,
                disp,
                font,
                at,
                -1,
                box_rect.x as i32 + 1,
                y as i32,
                scratch,
                flags,
            )?;
            painted += 1;
            y += row_h;
        }
        Ok(painted)
    }
}

/// The controls the host has handed out: their rectangles, the pointers to their text, their items and
/// which item is selected.
///
/// The strings themselves are not here either — a control's text is copied into the guest's own heap
/// when it is set (the module docs say why), so the state block already carries it, and what travels
/// here are the pointers plus everything a key press or a repaint consults.
impl Widgets {
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        use crate::mem::{put_bool, put_string, put_u32, sorted_u32};
        let rows = sorted_u32(&self.controls);
        put_u32(out, rows.len() as u32);
        for (addr, control) in rows {
            put_u32(out, addr);
            put_string(out, control.class);
            crate::display::put_rect(out, control.rect);
            put_bool(out, control.active);
            put_u32(out, control.props);
            put_u32(out, control.title);
            put_u32(out, control.text);
            put_u32(out, control.font_title);
            put_u32(out, control.font_text);
            put_u32(out, control.items.len() as u32);
            for item in &control.items {
                out.extend_from_slice(&item.id.to_le_bytes());
                put_u32(out, item.text);
                put_u32(out, item.data);
                out.extend_from_slice(&item.time.0.to_le_bytes());
                out.extend_from_slice(&item.time.1.to_le_bytes());
            }
            put_u32(out, control.sel as u32);
            put_u32(out, control.enum_sel as u32);
            put_bool(out, control.enabled);
            out.extend_from_slice(&control.max_chars.to_le_bytes());
            put_u32(out, control.caret);
            put_u32(out, control.sel_bits);
            put_u32(out, control.scroll_line as u32);
            put_u32(out, control.owner_draw);
            put_u32(out, control.owner_draw_data);
            out.extend_from_slice(&control.owner_draw_mask.to_le_bytes());
            put_u32(out, control.date.0 as u32);
            put_u32(out, control.date.1);
            put_u32(out, control.date.2);
            put_u32(out, control.day_mask);
            put_u32(out, control.softkey_menu);
            put_u32(out, control.input_mode);
            put_u32(out, control.enum_mode);
            put_u32(out, control.props_ex);
            // The menu's own two style records and its colour record. Both are state a pack can write
            // through a slot and read back through another, so they go into the block whole: sixteen
            // bytes of style, forty of colour, in the control's own order.
            for rec in &control.style {
                out.extend_from_slice(rec);
            }
            for word in &control.colors {
                put_u32(out, *word);
            }
        }
        put_u32(out, self.scratch);
    }

    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut crate::mem::State) -> Result<(), String> {
        let count = st.small()?;
        let mut controls = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            let name = st.string()?;
            // A class this build's table has not got is a state it cannot honour: the paint path
            // dispatches on exactly this word.
            let class = WIDGET_IFACES
                .iter()
                .copied()
                .find(|iface| *iface == name)
                .ok_or_else(|| {
                    format!("a state naming a control class this build has not got: {name}")
                })?;
            let rect = crate::display::get_rect(st)?;
            let active = st.u8()? != 0;
            let props = st.u32()?;
            let title = st.u32()?;
            let text = st.u32()?;
            let font_title = st.u32()?;
            let font_text = st.u32()?;
            let rows = st.small()?;
            let mut items = Vec::with_capacity(rows);
            for _ in 0..rows {
                let b = st.take(2)?;
                let id = u16::from_le_bytes([b[0], b[1]]);
                let text = st.u32()?;
                let data = st.u32()?;
                let b = st.take(2)?;
                let t0 = u16::from_le_bytes([b[0], b[1]]);
                let b = st.take(2)?;
                let t1 = u16::from_le_bytes([b[0], b[1]]);
                items.push(Item {
                    id,
                    text,
                    data,
                    time: (t0, t1),
                });
            }
            let sel = st.u32()? as usize;
            let enum_sel = st.u32()? as usize;
            let enabled = st.u8()? != 0;
            let b = st.take(2)?;
            controls.insert(
                addr,
                Control {
                    class,
                    rect,
                    active,
                    props,
                    title,
                    text,
                    font_title,
                    font_text,
                    items,
                    sel,
                    enum_sel,
                    enabled,
                    max_chars: u16::from_le_bytes([b[0], b[1]]),
                    caret: st.u32()?,
                    sel_bits: st.u32()?,
                    scroll_line: st.u32()? as i32,
                    owner_draw: st.u32()?,
                    owner_draw_data: st.u32()?,
                    owner_draw_mask: {
                        let b = st.take(2)?;
                        u16::from_le_bytes([b[0], b[1]])
                    },
                    date: (st.u32()? as i32, st.u32()?, st.u32()?),
                    // The day number and the weekday are re-derived from that triple by
                    // [`Control::julian`], exactly as they were before the snapshot was taken.
                    cmd_id: 0,
                    // The time control's four words are not in the snapshot's format either; see the
                    // note on `date` above for why the same reasoning does not apply to `julian`.
                    ms: 0,
                    ms_valid: false,
                    increment: 1,
                    edit_field: 0,
                    // The command flag comes back the way a new control gets it — on for the menu,
                    // off for the others — because it is not in the snapshot's format either.
                    send_cmd: class == "IMenuCtl",
                    image: 0,
                    redraw_cb: 0,
                    redraw_user: 0,
                    day_mask: st.u32()?,
                    softkey_menu: st.u32()?,
                    input_mode: st.u32()?,
                    enum_mode: st.u32()?,
                    props_ex: st.u32()?,
                    style: [[0u8; 8]; 2],
                    colors: MENU_COLORS,
                    // The HTML document, options and view state are not in the snapshot's format; a
                    // restored `IHtmlViewer` starts empty, like a newly created one.
                    html: (class == "IHtmlViewer").then(Html::default),
                },
            );
            // The tail of the block, in the order `write_state` wrote it: the menu's own style pair and
            // then its colour record. They go onto the record that was just inserted rather than into
            // the literal above, because a struct literal reads its fields in its own order and the
            // block puts these two last.
            let back = controls.get_mut(&addr).expect("the control just inserted");
            for rec in back.style.iter_mut() {
                let eight = st.take(8)?;
                rec.copy_from_slice(eight);
            }
            for word in back.colors.iter_mut() {
                *word = st.u32()?;
            }
        }
        self.controls = controls;
        self.scratch = st.u32()?;
        Ok(())
    }
}
