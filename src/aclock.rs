//! `IAClockCtl`: the analog clock control (published).
//!
//! The object is a handful of colours, one time and two rectangles: the face it draws in and the
//! picture it draws first. Everything that matters to a caller is either a stored byte or the
//! shipped geometry in [`hand_point`], which is why this module is arithmetic and state rather than
//! a drawing surface — the pixels go through `Display` from the runtime.

use std::collections::HashMap;

use crate::abi::generated as g;
use crate::display::Rect;

/// One clock. The five colour triples are the shipped `m_cFace[3]`-style members, kept as the three
/// bytes the setters store (`AClock_SetFaceColor`, the delivered body).
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    /// `m_dwTime`: the moment the hands point at, as the seconds `ISHELL_GetJulianDate` reads.
    pub time: u32,
    /// `m_dwProps`: `ACPROP_*`, born `MINS|HOURS`.
    pub props: u32,
    pub face: [u8; 3],
    pub frame: [u8; 3],
    pub sec: [u8; 3],
    pub min: [u8; 3],
    pub hour: [u8; 3],
    /// The `IImage` the pack installed as the face, or zero.
    pub face_image: u32,
    /// `m_rcFace`: a square of the shorter side of the rect the caller gave.
    pub face_rect: Rect,
    /// `m_rcFaceImage`: where that picture goes, its own size winning over the rect.
    pub image_rect: Rect,
    /// Whether `Start` asked for the auto-update; `Stop` is the only thing that clears it.
    pub started: bool,
}

impl Clock {
    /// `AClockCtl_New`: the colours depend on the display's depth, and the
    /// two properties the constructor turns on are the two hands it draws by default.
    pub fn new(depth: u16) -> Clock {
        let mut c = Clock {
            time: 0,
            props: g::ACPROP_MINS | g::ACPROP_HOURS,
            face: [0; 3],
            frame: [0; 3],
            sec: [0; 3],
            min: [0; 3],
            hour: [0; 3],
            face_image: 0,
            face_rect: Rect::EMPTY,
            image_rect: Rect::EMPTY,
            started: false,
        };
        if depth >= 8 {
            c.face = [128, 128, 128];
            // `OUTLINE_*`, `SEC_*`, `MIN_*`, `HOUR_*`.
            c.frame = [0, 0, 0];
            c.sec = [0, 0, 0];
            c.min = [0, 255, 255];
            c.hour = [0, 0, 255];
        } else {
            // Mono: a white frame, and the second hand shares it.
            c.frame = [255, 255, 255];
            c.sec = c.frame;
        }
        c
    }
}

/// An `RGBVAL` as the three bytes the shipped setters keep: `GET_RVAL`/`GET_GVAL`/`GET_BVAL`
/// — red at bits 8-15, then green, then blue.
pub fn rgb_parts(rgb: u32) -> [u8; 3] {
    [
        ((rgb >> 8) & 0xff) as u8,
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 24) & 0xff) as u8,
    ]
}

/// `AClock_CalcOffset`: the face is a square whose side is the shorter of
/// the two the caller gave, anchored at the caller's origin.
pub fn square(rc: Rect) -> Rect {
    let d = rc.dx.min(rc.dy);
    Rect::new(rc.x, rc.y, d, d)
}

/// `AClock_GetX`/`AClock_GetY`, as one point.
///
/// `ticks` is 12 for the hour hand and 60 for the minute and second hands, `value` is the reading
/// in those units, and `factor` shrinks the reach: the shipped bodies draw the tip of a hand at
/// `diameter / 2.1` (seconds), `2.3` (minutes) and `3` (hours), with the enclosing quad's corners at
/// 12, 20 and 10/18. The `(int)` casts truncate toward zero, and the source's own `nDiameter / 2`
/// is integer division — reproduced here rather than tidied, because the two differ by a pixel.
pub fn hand_point(rc: Rect, ticks: u32, value: f64, factor: f64) -> (i32, i32) {
    let half = i32::from(rc.dx / 2);
    let d = f64::from(rc.dx);
    let angle = std::f64::consts::PI + std::f64::consts::PI / (f64::from(ticks) / 2.0) * value;
    let x = i32::from(rc.x) + (half - (angle.sin() * d / factor) as i32);
    let y = i32::from(rc.y) + (angle.cos() * d / factor) as i32 + half;
    (x, y)
}

/// Every clock the host has handed out.
#[derive(Default)]
pub struct Clocks {
    objects: HashMap<u32, Clock>,
}

impl Clocks {
    pub fn add(&mut self, addr: u32, clock: Clock) {
        self.objects.insert(addr, clock);
    }

    pub fn forget(&mut self, addr: u32) -> Option<Clock> {
        self.objects.remove(&addr)
    }

    pub fn get(&self, addr: u32) -> Option<Clock> {
        self.objects.get(&addr).copied()
    }

    pub fn put(&mut self, addr: u32, clock: Clock) {
        self.objects.insert(addr, clock);
    }
}

#[cfg(test)]
#[path = "test/aclock.rs"]
mod tests;
