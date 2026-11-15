//! Where a handset's keys sit, and what its body is made of.
//!
//! [`crate::keymap`] says which keys exist and what code each one sends; this says **where they are
//! and what they look like**, which is a property of the phone rather than of BREW. Splitting the
//! two is what keeps them honest: a key that exists in one table and not the other is a bug a test
//! can catch, and a frontend that draws a phone instead of a list of buttons has somewhere to read
//! the shape from.
//!
//! Everything here is in **device pixels** — the same unit the screen is measured in — because the
//! frontend scales a run by one integer and nothing else. A fractional scale smears the game's art;
//! scaling the body by that same integer is what keeps the pixels square *and* the phone in
//! proportion, so the geometry has to be expressed in the unit that gets multiplied.
//!
//! No `egui` type appears in this file on purpose. The layout is arithmetic, the arithmetic is what
//! a test can check, and the raw front end in [`crate::window`] is allowed to want it too.

use crate::keymap;

/// A rectangle in device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub wide: f32,
    pub high: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, wide: f32, high: f32) -> Self {
        Rect { x, y, wide, high }
    }

    pub fn right(&self) -> f32 {
        self.x + self.wide
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.high
    }

    /// The middle of the rectangle. Public because the front end finds the rocker's centre from the
    /// navigation cells' own centres, which is how it knows one ring fits all five.
    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.wide / 2.0, self.y + self.high / 2.0)
    }

    /// The same rectangle shrunk by `by` on every side, which is how a key's face is pulled away
    /// from the cell it was laid out in so neighbouring keys read as separate.
    fn inset(&self, by: f32) -> Self {
        Rect::new(
            self.x + by,
            self.y + by,
            (self.wide - 2.0 * by).max(1.0),
            (self.high - 2.0 * by).max(1.0),
        )
    }

    /// Does `other` fit entirely inside this rectangle? How the layout says a key is on the phone.
    pub fn contains(&self, other: &Rect) -> bool {
        other.x >= self.x - EPS
            && other.right() <= self.right() + EPS
            && other.y >= self.y - EPS
            && other.bottom() <= self.bottom() + EPS
    }

    /// Do two rectangles share ground? How the layout says two keys are not the same key, and that no
    /// key sits on top of the screen.
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right() - EPS
            && other.x < self.right() - EPS
            && self.y < other.bottom() - EPS
            && other.y < self.bottom() - EPS
    }
}

/// The slack a comparison of laid-out floats needs. The layout is arithmetic on a handful of
/// constants, so this is about float addition, not about tolerance for anything user-supplied.
const EPS: f32 = 0.01;

/// What to paint on a key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Face {
    /// Text — `CLR`, a digit. The glyphs in [`keymap::KEYS`] are ASCII for the same reason they are
    /// there: a UI font is not promised anything else.
    Glyph(&'static str),
    /// One arm of the navigation rocker, drawn as a triangle pointing that way.
    Arrow(Arrow),
    /// The rocker's middle: a button of its own, because a handset's centre key is not an arrow.
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

/// One key, where it sits, and what is on it.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub key: &'static keymap::Key,
    pub rect: Rect,
    pub face: Face,
}

/// The body of a candybar handset around a screen of the size a run reported.
#[derive(Clone, Debug)]
pub struct Handset {
    pub body: Rect,
    pub screen: Rect,
    pub keys: Vec<Placed>,
    /// The earpiece slit above the screen and the brand plate below the keypad — drawn, never
    /// pressed. A body with nothing but keys on it reads as a control panel, not as a phone.
    pub ear: Rect,
    pub chin: Rect,
}

/// The gap between the screen and the first key row.
const GAP: f32 = 8.0;
/// Space between rows of keys, and between the two soft-key bars.
const ROW_GAP: f32 = 5.0;
/// The body's rail, either side of the screen.
const SIDE: f32 = 14.0;
/// The handset above the screen, and below the keypad.
const CROWN: f32 = 18.0;
const CHIN: f32 = 14.0;
/// How much taller a digit row's pitch is than the key itself, so the rows read as separate without
/// the body growing a full key height between each.
const DIGIT_PITCH: f32 = 1.2;

impl Handset {
    /// A screen at the top, two soft keys, a navigation rocker, `CLR`, then the 4x3 digit grid.
    ///
    /// The keypad is laid out on a three-column grid whose pitch comes from the screen's own width,
    /// clamped at both ends: a 176-pixel screen would otherwise get keys too small to read, and a
    /// 320-pixel one would get a body wider than it is tall and taller than any window.
    pub fn candybar(wide: f32, high: f32) -> Self {
        let pitch = (wide / 3.0).clamp(20.0, 40.0);
        let cell = pitch;
        let key_h = (pitch * 0.6).max(14.0);
        let grid = 3.0 * cell;
        let body_wide = wide.max(grid) + 2.0 * SIDE;
        // The grid is centred under the screen, so a landscape screen does not push the keypad off
        // to one side of a body that is wider than either.
        let left = (body_wide - grid) / 2.0;
        let screen = Rect::new((body_wide - wide) / 2.0, CROWN, wide, high);

        let mut y = screen.bottom() + GAP;
        let mut keys = Vec::with_capacity(keymap::KEYS.len());
        // The two soft keys, as bars rather than squares: they are the keys a dialog's answer is
        // written on, and a handset labels them with the text it draws on the screen above them.
        let bar = (grid - ROW_GAP) / 2.0;
        for (i, name) in ["AVK_SOFT1", "AVK_SOFT2"].iter().enumerate() {
            keys.push(placed(
                name,
                Rect::new(left + i as f32 * (bar + ROW_GAP), y, bar, key_h),
            ));
        }
        y += key_h + ROW_GAP;
        // The rocker: one 3x3 grid, drawn as a ring, hit-tested as nine boxes. A real handset's
        // rocker is a single piece of plastic with five switches under it, and this is that.
        let rocker = 3.0 * cell;
        let mut arm = |row: usize, col: usize, name: &'static str| {
            keys.push(placed(
                name,
                Rect::new(left + col as f32 * cell, y + row as f32 * cell, cell, cell).inset(1.0),
            ));
        };
        arm(0, 1, "AVK_UP");
        arm(1, 0, "AVK_LEFT");
        arm(1, 1, "AVK_SELECT");
        arm(1, 2, "AVK_RIGHT");
        arm(2, 1, "AVK_DOWN");
        y += rocker + ROW_GAP;
        keys.push(placed("AVK_CLR", Rect::new(left + cell, y, cell, key_h)));
        y += key_h + ROW_GAP;
        // The digit grid, in the order the SDK's own keypad diagram reads it.
        for (row, line) in [
            ["AVK_1", "AVK_2", "AVK_3"],
            ["AVK_4", "AVK_5", "AVK_6"],
            ["AVK_7", "AVK_8", "AVK_9"],
            ["AVK_STAR", "AVK_0", "AVK_POUND"],
        ]
        .iter()
        .enumerate()
        {
            for (col, name) in line.iter().enumerate() {
                keys.push(placed(
                    name,
                    Rect::new(
                        left + col as f32 * cell,
                        y + row as f32 * key_h * DIGIT_PITCH,
                        cell,
                        key_h,
                    ),
                ));
            }
        }
        y += 4.0 * key_h * DIGIT_PITCH;

        Handset {
            body: Rect::new(0.0, 0.0, body_wide, y + CHIN),
            ear: Rect::new(
                screen.centre().0 - wide / 6.0,
                CROWN / 2.0 - 2.0,
                wide / 3.0,
                4.0,
            ),
            chin: Rect::new(
                screen.centre().0 - body_wide / 8.0,
                y + CHIN / 4.0,
                body_wide / 4.0,
                4.0,
            ),
            screen,
            keys,
        }
    }

    /// The same body for a device that reported `wide`x`high`.
    pub fn for_device(wide: u32, high: u32) -> Self {
        Handset::candybar(wide as f32, high as f32)
    }

    pub fn key(&self, name: &str) -> Option<&Placed> {
        self.keys.iter().find(|p| p.key.name == name)
    }
}

/// A key by its SDK name, in the cell `rect`, with the face this file chose for it. Panics on an
/// unknown name: the list above is a claim about the handset, and a claim that stops matching
/// [`keymap::KEYS`] should stop the build rather than draw a phone with a hole in it.
fn placed(name: &'static str, rect: Rect) -> Placed {
    let key = keymap::KEYS
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("{name} is not a key the table offers"));
    let face = match name {
        "AVK_UP" => Face::Arrow(Arrow::Up),
        "AVK_DOWN" => Face::Arrow(Arrow::Down),
        "AVK_LEFT" => Face::Arrow(Arrow::Left),
        "AVK_RIGHT" => Face::Arrow(Arrow::Right),
        "AVK_SELECT" => Face::Select,
        _ => Face::Glyph(key.glyph),
    };
    Placed { key, rect, face }
}

/// A handset's colours. The model is a body and five inks; what a skin decides is which of those are
/// bright, which are dark, and whether the keys look moulded or printed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skin {
    pub name: &'static str,
    /// The body's face, and the line that separates it from whatever the window is behind it.
    pub body: [u8; 3],
    pub edge: [u8; 3],
    /// The recess the screen sits in. Always darker than the body, or the glass reads as flat.
    pub bezel: [u8; 3],
    pub key: [u8; 3],
    pub key_down: [u8; 3],
    /// Labels, and the rocker's arrows.
    pub ink: [u8; 3],
    /// The rocker's ring: the part a handset moulds as a separate piece of plastic.
    pub ring: [u8; 3],
}

/// The skins on offer, first one being what a run gets when nobody chooses.
pub const SKINS: &[Skin] = &[
    Skin {
        name: "graphite",
        body: [0x2b, 0x2e, 0x33],
        edge: [0x0d, 0x0f, 0x12],
        bezel: [0x08, 0x09, 0x0b],
        key: [0x3c, 0x41, 0x48],
        key_down: [0x6a, 0x7a, 0x8f],
        ink: [0xd8, 0xdc, 0xe2],
        ring: [0x1b, 0x1d, 0x21],
    },
    Skin {
        name: "silver",
        body: [0xc9, 0xcc, 0xd1],
        edge: [0x7a, 0x7e, 0x85],
        bezel: [0x2a, 0x2c, 0x30],
        key: [0xe6, 0xe8, 0xec],
        key_down: [0x9d, 0xb4, 0xd6],
        ink: [0x24, 0x26, 0x2a],
        ring: [0x9c, 0xa0, 0xa7],
    },
    Skin {
        name: "candy",
        body: [0x35, 0x1f, 0x4a],
        edge: [0x14, 0x0b, 0x1e],
        bezel: [0x0b, 0x07, 0x11],
        key: [0x50, 0x2f, 0x6e],
        key_down: [0xd0, 0x6b, 0x3a],
        ink: [0xf2, 0xe7, 0xff],
        ring: [0x24, 0x15, 0x33],
    },
];

/// The skin called `name`, case-insensitively, or none of them.
pub fn skin(name: &str) -> Option<&'static Skin> {
    SKINS
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .or_else(|| (name.is_empty()).then_some(&SKINS[0]))
}

/// What to tell a player who asks for a skin that is not there.
pub fn skin_names() -> String {
    SKINS.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
#[path = "test/handset.rs"]
mod tests;
