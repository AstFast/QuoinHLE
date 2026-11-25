//! Which key a player pressed, in the handset's own vocabulary.
//!
//! Three things have to agree here: a physical keyboard, a virtual one drawn from data, and a
//! `--key AVK_SOFT1` in a log line. So the table is written as **published names** and the numbers come
//! out of the generated constants — a code cannot drift from what the ABI says, and a name that
//! stops existing fails loudly instead of silently sending a different key.
//!
//! What is deliberately *not* here is where the keys sit. That is a property of the handset, so
//! [`crate::handset`] lays [`KEYS`] out on a body, and a test on each side keeps the two tables
//! agreeing: a key that exists here and not there is a key with no place to be pressed.

use crate::abi;

/// Win32 virtual-key codes, which are the only thing a PC keyboard reports.
const VK_BACK: usize = 0x08;
const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const VK_SPACE: usize = 0x20;
const VK_PRIOR: usize = 0x21;
const VK_NEXT: usize = 0x22;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;
/// The endpoints are spelled out because a range pattern may only name constants, not add to them.
const VK_0: usize = 0x30;
const VK_1: usize = VK_0 + 1;
const VK_9: usize = VK_0 + 9;
const VK_NUMPAD0: usize = 0x60;
const VK_NUMPAD1: usize = VK_NUMPAD0 + 1;
const VK_NUMPAD9: usize = VK_NUMPAD0 + 9;
const VK_Z: usize = 0x5A;
const VK_X: usize = 0x58;

/// A key the emulated handset has.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    /// Its `AVK_*` name in the platform.
    pub name: &'static str,
    /// What to paint on it. `AVK_SOFT1` is not a label a player reads, and the glyphs stay ASCII
    /// because a UI font is not promised to carry anything else.
    pub glyph: &'static str,
}

/// The code this key sends, from the ABI's own table.
pub fn avk(name: &str) -> u32 {
    abi::const_value(name).unwrap_or_else(|| panic!("{name} is in the ABI tables"))
}

impl Key {
    pub fn avk(&self) -> u32 {
        avk(self.name)
    }
}

/// Every key the guest packs were seen to take, in reading order.
pub const KEYS: &[Key] = &[
    Key {
        name: "AVK_SOFT1",
        glyph: "SOFT 1",
    },
    Key {
        name: "AVK_SOFT2",
        glyph: "SOFT 2",
    },
    Key {
        name: "AVK_CLR",
        glyph: "CLR",
    },
    Key {
        name: "AVK_UP",
        glyph: "UP",
    },
    Key {
        name: "AVK_LEFT",
        glyph: "LEFT",
    },
    Key {
        name: "AVK_SELECT",
        glyph: "SELECT",
    },
    Key {
        name: "AVK_RIGHT",
        glyph: "RIGHT",
    },
    Key {
        name: "AVK_DOWN",
        glyph: "DOWN",
    },
    Key {
        name: "AVK_1",
        glyph: "1",
    },
    Key {
        name: "AVK_2",
        glyph: "2",
    },
    Key {
        name: "AVK_3",
        glyph: "3",
    },
    Key {
        name: "AVK_4",
        glyph: "4",
    },
    Key {
        name: "AVK_5",
        glyph: "5",
    },
    Key {
        name: "AVK_6",
        glyph: "6",
    },
    Key {
        name: "AVK_7",
        glyph: "7",
    },
    Key {
        name: "AVK_8",
        glyph: "8",
    },
    Key {
        name: "AVK_9",
        glyph: "9",
    },
    Key {
        name: "AVK_STAR",
        glyph: "*",
    },
    Key {
        name: "AVK_0",
        glyph: "0",
    },
    Key {
        name: "AVK_POUND",
        glyph: "#",
    },
];

/// A handset key for a PC one, and only for the keys the guest packs actually take.
///
/// The soft keys are the load-bearing part: a pack's own confirmation question answers `AVK_SOFT1`
/// and declines `AVK_SELECT`, so a keyboard that cannot send a soft key cannot get past the first
/// question a real game asks.
pub fn physical(vk: usize) -> Option<u32> {
    let name = match vk {
        VK_LEFT => "AVK_LEFT",
        VK_UP => "AVK_UP",
        VK_RIGHT => "AVK_RIGHT",
        VK_DOWN => "AVK_DOWN",
        VK_RETURN | VK_SPACE => "AVK_SELECT",
        VK_Z => "AVK_SOFT1",
        VK_X => "AVK_SOFT2",
        VK_BACK => "AVK_CLR",
        // Page Up/Down stand in for the d-pad when a laptop keyboard has no arrows to spare.
        VK_PRIOR => "AVK_UP",
        VK_NEXT => "AVK_DOWN",
        VK_0 | VK_NUMPAD0 => "AVK_0",
        // `AVK_1..=AVK_9` are consecutive and `AVK_0` is not one further along, so the digits are
        // named one by one rather than computed from a code.
        d @ VK_1..=VK_9 => {
            return Some(avk(&format!("AVK_{}", d - VK_1 + 1)));
        }
        d @ VK_NUMPAD1..=VK_NUMPAD9 => {
            return Some(avk(&format!("AVK_{}", d - VK_NUMPAD1 + 1)));
        }
        _ => return None,
    };
    Some(avk(name))
}

/// Esc is not a handset key; it is the player leaving.
pub const QUIT: usize = VK_ESCAPE;

/// The PC keys a handset key starts bound to. A custom entry in `manager.json` replaces this list
/// for that key; an empty list means the manager has to bind it before it can be pressed.
pub fn default_bindings(name: &str) -> &'static [&'static str] {
    match name {
        "AVK_SOFT1" => &["Z"],
        "AVK_SOFT2" => &["X"],
        "AVK_CLR" => &["Backspace"],
        "AVK_LEFT" => &["ArrowLeft"],
        "AVK_UP" => &["ArrowUp", "Home"],
        "AVK_SELECT" => &["Enter", "Space"],
        "AVK_RIGHT" => &["ArrowRight"],
        "AVK_DOWN" => &["ArrowDown", "End"],
        "AVK_1" => &["Num1"],
        "AVK_2" => &["Num2"],
        "AVK_3" => &["Num3"],
        "AVK_4" => &["Num4"],
        "AVK_5" => &["Num5"],
        "AVK_6" => &["Num6"],
        "AVK_7" => &["Num7"],
        "AVK_8" => &["Num8"],
        "AVK_9" => &["Num9"],
        "AVK_0" => &["Num0"],
        _ => &[],
    }
}

/// What to tell a player in the window's title bar. Both front ends show it, because both implement
/// exactly this mapping — the raw window from PC virtual-key codes, egui from its own key enum —
/// and each resolves to the same `AVK_*` names this lists.
pub const HINT: &str =
    "   [arrows]  [Enter=SELECT]  [Z/X=soft keys]  [Backspace=CLR]  [0-9=digits]  [Esc=quit]";

#[cfg(test)]
#[path = "test/keymap.rs"]
mod tests;
