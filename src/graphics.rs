//! `IGraphics`: the drawing state a handset keeps on top of its display.
//!
//! `IDisplay` draws with a colour handed to it per call; `IGraphics` draws with a colour it holds,
//! which is why an app can set a pen once and then issue a hundred rectangles. So this module is
//! mostly state — and the state has to *stay* right, because an app reads it back (`GetColor`,
//! `GetFillMode`) as often as a screensaver does, and a getter that returns the default after a
//! setter ran is a wrong answer nothing else would catch.
//!
//! Geometry lives here too. Lines and circles are computed from their own definitions rather than by
//! dressing up an `IDisplay` call, because `IDisplay::DrawRect` takes two guest `AEECOLOR` structs
//! and there is no reason to allocate guest memory per drawn shape.
//!
//! Every loop that walks a shape is clamped to the drawing window before it runs. A shape's extents
//! are the guest's own `int16`s, and a rectangle of `0x7FFF` pixels would otherwise mean a billion
//! calls that each decided, one pixel at a time, that they were off-screen.
//!
//! What is not here: nothing that the published interface names is refused any more. Every shape form
//! is drawn — arcs, wedges, ellipses, their arcs, triangles, polygons, and a stretched picture — and
//! where a curve is drawn by a method rather than by a handset's exact rasteriser, the method's own
//! comment says how.

use std::collections::HashMap;

use crate::abi::generated as g;
use crate::display::{rgb_to_native, Display, IndexMemo, Rect};
use crate::hle::HResult;
use crate::mem::Mem;

/// A colour as `IGraphics` holds it: three components, packed `(r << 8) + (g << 16) + (b << 24)`.
///
/// The fourth byte is part of the *prototype* and not part of the state — the shipped setters drop
/// the `alpha` argument on the floor and `GetColor` answers it from the packed word's low byte, which
/// is always `0`. So `a` is stored rather than guessed at, and it is `0` on every path the runtime
/// itself writes. AEEGraphics.h is explicit that it is "just a placeholder that has no effect".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba { r, g, b, a }
    }

    /// The `RGBVAL` the colour setters answer with, and the layout `display::rgb_to_native` reads.
    pub const fn rgbval(&self) -> u32 {
        ((self.r as u32) << 8) | ((self.g as u32) << 16) | ((self.b as u32) << 24)
    }

    /// The value the display's own primitives take.
    pub const fn native(&self) -> u32 {
        rgb_to_native(self.rgbval())
    }
}

/// One `IGraphics` object's state.
#[derive(Debug, Clone, Copy)]
pub struct State {
    /// The pen. `SetColor` names it, and every outline uses it.
    pub fg: Rgba,
    /// The background, which `ClearRect` and `ClearViewport` paint with.
    pub bg: Rgba,
    /// The fill colour. `SetFillColor` owns it, and a filled shape uses it instead of the pen, which
    /// is why the interface carries both.
    pub fill: Rgba,
    /// `SetFillMode`: closed shapes are filled rather than outlined.
    pub filled: bool,
    /// `SetPointSize`, in pixels square.
    pub point: u8,
    /// `AEEStrokeStyle`. Anything but `STROKE_SOLID` draws every other pixel, which is all it can
    /// mean when the interface offers no dash pattern to consult.
    pub stroke: u32,
    /// `AEEAlgorithmHint`, whose documented default is `ALGORITHM_FAST`.
    pub hint: u32,
    /// `AEEPaintMode`, stored: the transparency modes it names need a blend this host does not do.
    pub paint: u32,
    /// The drawing window. Shape coordinates are relative to its origin and clipped to it, which is
    /// what a viewport means.
    pub viewport: Rect,
    /// The effective clip, the body's `[+88]`/`[+90]`/`[+92]`/`[+94]` box: device coordinates, already
    /// run through `transformRectToScreen`, which is why nothing here transforms it a second time. A
    /// clip a pack names is viewport-relative *input*; what drawing is tested against is where that
    /// shape landed on the screen, and `Translate` moves the input mapping without moving this box.
    pub clip: Option<Region>,
    /// The box `valid` tests — `[+88]`/`[+90]`/`[+92]`/`[+94]`, the same device corners, kept
    /// separately because a circle or an ellipse is clamped *into* the window here
    /// (`initShapeClipInfo` narrows `[+88]..[+94]` to what the window already allows) while the figure
    /// the stepper walks keeps its full extent. For a rectangular clip the two are one value.
    pub clip_box: Option<Rect>,
    /// The caller's own `AEEClip` bytes, sixteen of them, kept exactly as
    /// `aee_memmove(a1 + 72, a2, 16)` keeps them. `GetClip` hands these back verbatim instead of
    /// rebuilding the struct, so a pack's padding bytes round trip — and a shape the body refused on a
    /// radius test still left them stored.
    pub clip_raw: [u8; 16],
    /// `SetClip`: the shipped `byte[+102]`, remembered because `GetViewport` reports it and
    /// `ClearViewport` shrinks by one pixel on each side when it is set.
    pub framed: bool,
    /// The `byte[+50]`/`byte[+52]` world origin that `Translate` accumulates into and `Pan` recentres.
    /// Drawing lands at `viewport origin − translate + point`, which is the body's `[+58] = [+54] -
    /// [+50]`; the reported viewport (`GetViewport`) is not touched by either call.
    pub translate: (i32, i32),
    /// The `IBitmap` being drawn into, or `0` for the screen itself.
    pub dest: u32,
    /// The depth of that destination, which `GetColorDepth` answers and `SetDestination` re-reads.
    /// The shipped object caches it at `byte[+105]` from the same `IBitmap::GetInfo` that sized the
    /// window, so it changes only when the destination does.
    pub depth: u32,
    /// The display this state was made from, so `Update` reaches something. Resolved again at the
    /// call if it was never set.
    pub display: u32,
}

/// Two endpoints, as `AEELine` holds them.
#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub a: (i32, i32),
    pub b: (i32, i32),
}

/// What `SetClip` handed over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clip {
    /// No clip: the viewport is the whole window.
    None,
    /// `CLIPPING_RECT`, in viewport-relative coordinates.
    Rect(Rect),
    /// `CLIPPING_CIRCLE` or `CLIPPING_ELLIPSE`, already turned into the bounding box the body builds.
    Ellipse(Rect),
    /// Any other `AEEClipShape`, carried by value so the run can name the one it wanted.
    Shape(u32),
}

/// Every `IGraphics` object the host has handed out.
#[derive(Default)]
pub struct Graphics {
    objects: HashMap<u32, State>,
}

impl Graphics {
    pub fn add(&mut self, addr: u32, disp: u32, screen: Rect, bg: Rgba) {
        let mut state = State {
            fg: Rgba::new(0, 0, 0, 0),
            bg,
            fill: Rgba::new(0, 0, 0, 0),
            filled: false,
            point: 1,
            stroke: g::STROKE_SOLID,
            hint: g::ALGORITHM_FAST,
            paint: 0,
            viewport: screen,
            clip: None,
            clip_box: None,
            clip_raw: [0; 16],
            framed: false,
            translate: (0, 0),
            dest: 0,
            depth: u32::from(crate::display::NATIVE_DEPTH),
            display: disp,
        };
        Self::reset_clip(&mut state);
        self.objects.insert(addr, state);
    }

    pub fn forget(&mut self, addr: u32) {
        self.objects.remove(&addr);
    }

    pub fn get(&self, addr: u32) -> Option<State> {
        self.objects.get(&addr).copied()
    }

    pub fn set(&mut self, addr: u32, state: State) {
        self.objects.insert(addr, state);
    }

    /// The rectangle drawing is actually clipped to: with the frame flag the body keeps the requested
    /// rect at `[+62]..[+68]` (which is what `GetViewport` answers) and stores the rect **inset by one
    /// pixel on each side** at `[+54]/[+56]` and `[+76]..[+82]`. Every draw uses the inset one, so
    /// `at` and [`Self::window`] do too, and `ClearViewport` starts from it rather than shrinking twice.
    pub(crate) fn drawn(st: &State) -> Rect {
        if !st.framed {
            return st.viewport;
        }
        let (x, y) = (i32::from(st.viewport.x), i32::from(st.viewport.y));
        let (w, h) = (i32::from(st.viewport.dx), i32::from(st.viewport.dy));
        if w < 3 || h < 3 {
            return st.viewport;
        }
        Rect::new(
            (x + 1) as i16,
            (y + 1) as i16,
            (w - 2) as i16,
            (h - 2) as i16,
        )
    }

    /// The 16 bytes `resetClip` leaves behind: the kind byte becomes `SHAPE_RECT` and the union's rect
    /// becomes the window the clip was just widened to. `GetClip` copies these out verbatim, so a pack
    /// that reset the clip and reads it back sees a rectangle over the whole window — not the shape it
    /// set earlier, and not zeroes.
    pub fn reset_clip_bytes(st: &mut State) {
        let window = Self::drawn(st);
        st.clip_raw = [0u8; 16];
        st.clip_raw[0] = g::CLIPPING_RECT as u8;
        for (index, value) in [window.x, window.y, window.dx, window.dy]
            .iter()
            .enumerate()
        {
            let at = 4 + index * 2;
            st.clip_raw[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// The two boxes a clip is made of, stored together: the figure the caller named, run through
    /// `transformRectToScreen`, and the window `valid` tests, which is that figure clamped into the
    /// drawn window the way `drawRect` and `initShapeClipInfo` clamp `[+88]..[+94]`. Clamping happens
    /// **here**, at the call that sets the clip, and never at draw time: the box the shipped code tests
    /// is whatever the last reset or narrowing left behind, and a `Translate` afterwards moves the
    /// figure's mapping without moving it.
    pub fn set_clip(st: &mut State, figure: Rect, kind: u32) {
        st.clip = Some(Region { rect: figure, kind });
        st.clip_box = Some(figure.clipped(Self::drawn(st)).unwrap_or(Rect::EMPTY));
    }

    /// `resetClip`: the box goes back to the whole window and the region is reported as a rectangle
    /// over it. Every `SetViewport`, every `SetDestination` and the constructor run this.
    pub fn reset_clip(st: &mut State) {
        let window = Self::drawn(st);
        st.clip = Some(Region {
            rect: window,
            kind: g::CLIPPING_RECT,
        });
        st.clip_box = Some(window);
        Self::reset_clip_bytes(st);
    }

    fn window(st: &State) -> Rect {
        let Some(clip) = st.clip_box else {
            return Self::drawn(st);
        };
        clip
    }

    /// Where a shape coordinate lands. The body keeps one halfword pair for it: `[+58] = [+54] - [+50]`
    /// (viewport origin minus the world origin `Translate` accumulates into and `Pan` recentres), and
    /// both the subtraction and the later addition are 16-bit, so the offset wraps like the halfword
    /// does. `GetViewport` reports `[+62]`-and friends, which neither `Translate` nor `Pan` touches.
    pub fn at(st: &State, x: i32, y: i32) -> (i32, i32) {
        let window = Self::drawn(st);
        let offset = |left: i32, origin: i32| (left - origin) as u16 as i16 as i32;
        (
            x + offset(i32::from(window.x), st.translate.0),
            y + offset(i32::from(window.y), st.translate.1),
        )
    }

    /// A signed `int16` out of the guest's heap at a byte offset.
    fn half(mem: &Mem, addr: u32, off: u32) -> HResult<i32> {
        Ok(i16::from_le_bytes([mem.read_u8(addr + off)?, mem.read_u8(addr + off + 1)?]) as i32)
    }

    /// A shape member's byte offset. These are compile-time facts about the ABI, not about the
    /// guest, so a miss is a bug in this file and has to say which member.
    fn field(struct_name: &str, f: &str) -> u32 {
        crate::abi::offset_of(struct_name, f)
            .unwrap_or_else(|| panic!("{struct_name}::{f} is not in the ABI tables"))
    }

    /// `AEERect` in the guest's heap. A null pointer is the caller's documented `AEE_EBADPARM`, which
    /// the arm answers before calling this; a pointer into unmapped space is a data abort.
    pub fn rect(mem: &Mem, addr: u32) -> HResult<Rect> {
        Rect::read(mem, addr)
    }

    /// `AEELine`.
    pub fn line_of(mem: &Mem, addr: u32) -> HResult<Line> {
        let at = |f: &str| Self::field("AEELine", f);
        Ok(Line {
            a: (
                Self::half(mem, addr, at("sx"))?,
                Self::half(mem, addr, at("sy"))?,
            ),
            b: (
                Self::half(mem, addr, at("ex"))?,
                Self::half(mem, addr, at("ey"))?,
            ),
        })
    }

    /// `AEEPie`: the circle a wedge belongs to, and the two angles that bound it. The header notes the
    /// struct is identical to `AEEArc` so a caller may cast between the two, which is why the angles
    /// come back with the shape rather than being read a second time.
    pub fn pie_of(mem: &Mem, addr: u32) -> HResult<(i32, i32, i32, i16, i16)> {
        let at = |f: &str| Self::field("AEEPie", f);
        Ok((
            Self::half(mem, addr, at("cx"))?,
            Self::half(mem, addr, at("cy"))?,
            Self::half(mem, addr, at("r"))?,
            Self::half(mem, addr, at("startAngle"))? as i16,
            Self::half(mem, addr, at("arcAngle"))? as i16,
        ))
    }

    /// `AEECircle`: centre and radius.
    pub fn circle_of(mem: &Mem, addr: u32) -> HResult<(i32, i32, i32)> {
        let at = |f: &str| Self::field("AEECircle", f);
        Ok((
            Self::half(mem, addr, at("cx"))?,
            Self::half(mem, addr, at("cy"))?,
            Self::half(mem, addr, at("r"))?,
        ))
    }

    /// `AEEEllipse`: centre and the two semi-axes, which is what the header's own member list says
    /// `wx` and `wy` are — "semi-axis length of the ellipse along x-axis", and the figure is
    /// axis-aligned.
    pub fn ellipse_of(mem: &Mem, addr: u32) -> HResult<(i32, i32, i32, i32)> {
        let at = |f: &str| Self::field("AEEEllipse", f);
        Ok((
            Self::half(mem, addr, at("cx"))?,
            Self::half(mem, addr, at("cy"))?,
            Self::half(mem, addr, at("wx"))?,
            Self::half(mem, addr, at("wy"))?,
        ))
    }

    /// `AEETriangle`: the three vertices, in the order the caller named them. A triangle is a closed
    /// ring of three, which is why it goes through the polygon's fill rather than having its own.
    pub fn triangle_of(mem: &Mem, addr: u32) -> HResult<[(i32, i32); 3]> {
        let at = |f: &str| Self::field("AEETriangle", f);
        let corner = |n: &str| -> HResult<(i32, i32)> {
            Ok((
                Self::half(mem, addr, at(&format!("x{n}")))?,
                Self::half(mem, addr, at(&format!("y{n}")))?,
            ))
        };
        Ok([corner("0")?, corner("1")?, corner("2")?])
    }

    /// `AEEPoint`.
    pub fn point_of(mem: &Mem, addr: u32) -> HResult<(i32, i32)> {
        let at = |f: &str| Self::field("AEEPoint", f);
        Ok((
            Self::half(mem, addr, at("x"))?,
            Self::half(mem, addr, at("y"))?,
        ))
    }

    /// `AEEPolygon` and `AEEPolyline`, which the header gives the same shape: a count and a pointer
    /// to that many `AEEPoint`s.
    ///
    /// The count is the caller's, so it is read rather than assumed — and then bounded. A ring of a
    /// few points is what these calls are for; a corrupt or hostile word here would otherwise make
    /// the fill walk rows for a polygon nobody drew, and the run would look hung rather than wrong.
    /// A null array or a count below two answers empty, which the arm draws as nothing.
    pub fn polygon_of(mem: &Mem, addr: u32) -> HResult<Vec<(i32, i32)>> {
        let at = |f: &str| Self::field("AEEPolygon", f);
        let step = crate::abi::struct_info("AEEPoint")
            .unwrap_or_else(|| panic!("AEEPoint is not in the ABI tables"))
            .size;
        let points = mem.read_u32(addr + at("points"))?;
        let len = Self::half(mem, addr, at("len"))?.max(0) as usize;
        if points == 0 || len < 2 {
            return Ok(Vec::new());
        }
        let n = len.min(4096);
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            pts.push(Self::point_of(mem, points + (i as u32) * step)?);
        }
        Ok(pts)
    }

    /// The two words a caller's `AEEPolygon` / `AEEPolyline` starts with: the count and the array
    /// pointer. The bodies read these *before* anything else, and both slots refuse a null array or a
    /// count below their own minimum, so the refusal is the header's `AEE_EBADPARM` rather than a
    /// silent "drew nothing".
    pub fn polygon_head(mem: &Mem, addr: u32) -> HResult<(i32, u32)> {
        let at = |f: &str| Self::field("AEEPolygon", f);
        Ok((
            Self::half(mem, addr, at("len"))?,
            mem.read_u32(addr + at("points"))?,
        ))
    }

    /// `AEEClip`. A null `pShape` is "no clip". The three shapes the runtime's own body accepts are
    /// mapped here — `SHAPE_RECT` as the rectangle, `SHAPE_CIRCLE` and `SHAPE_ELLIPSE` through the
    /// same box the body builds for them (`convertCircleToRect`), which is what its ellipse stepper and
    /// `valid` end up testing. Anything else is carried by value so the run can name the one it
    /// wanted, because `boolean`-returning slots answer FALSE for a refusal and the state stays.
    ///
    /// `AEEClipShape` is **one byte** — the header gives `unsigned` only under
    /// `_WIN32` and `int8` everywhere a pack is actually built — and the union after it is 4-aligned,
    /// so a real `AEEClip` leaves three **uninitialised padding bytes** between the type and the rect.
    /// Reading the type as a word therefore returns `0x00380001` where the pack wrote `1`, and this
    /// slot answers FALSE with the clip unapplied: over four seconds of virtual time at 320x240,
    /// **112 of 171 `SetClip` calls come back FALSE** and only three distinct shape
    /// values were ever noted, because [`Display::note`] folds identical text. The rect was there the
    /// whole time at `+4`; only the width of the read was wrong.
    pub fn clip_of(mem: &Mem, addr: u32) -> Option<Clip> {
        if addr == 0 {
            return Some(Clip::None);
        }
        let kind = u32::from(mem.read_u8(addr).ok()?);
        let half = |off: u32| -> Option<i32> { Self::half(mem, addr, off).ok() };
        match kind {
            g::CLIPPING_NONE => Some(Clip::None),
            g::CLIPPING_RECT => Some(Clip::Rect(Rect::read(mem, addr + 4).ok()?)),
            g::CLIPPING_CIRCLE | g::CLIPPING_ELLIPSE => {
                let rx = half(8)?;
                let ry = if kind == g::CLIPPING_CIRCLE {
                    rx
                } else {
                    half(10)?
                };
                let (cx, cy) = (half(4)?, half(6)?);
                let (dx, dy) = if rx == 0 || ry == 0 {
                    (0, 0)
                } else {
                    (2 * rx + 1, 2 * ry + 1)
                };
                Some(Clip::Ellipse(Rect::new(
                    (cx - rx) as i16,
                    (cy - ry) as i16,
                    dx as i16,
                    dy as i16,
                )))
            }
            other => Some(Clip::Shape(other)),
        }
    }

    /// Write `Rgba` back into the guest bytes the caller passed pointers to, which is how `GetColor`
    /// and its siblings report. A null component is a caller that did not want that one, not an
    /// error, so it is skipped.
    pub fn put_rgba(mem: &mut Mem, at: [u32; 4], c: Rgba) -> HResult<()> {
        for (addr, v) in at.into_iter().zip([c.r, c.g, c.b, c.a]) {
            if addr != 0 {
                mem.write_u8(addr, v)?;
            }
        }
        Ok(())
    }

    /// Three-byte form: `GetBackground` has no alpha to report.
    pub fn put_rgb(mem: &mut Mem, at: [u32; 3], c: Rgba) -> HResult<()> {
        Self::put_rgba(mem, [at[0], at[1], at[2], 0], c)
    }
}

/// A clip region, in device coordinates — the box the caller's shape lands on after
/// `transformRectToScreen` — plus the shape it was declared as.
///
/// `SHAPE_CIRCLE` and `SHAPE_ELLIPSE` are both stored as the bounding box the body builds for them
/// (`2r + 1` or `2rx + 1` by `2ry + 1`, with the centre recovered as `box + (extent - 1) / 2`) and a
/// flag saying the figure is the ellipse inscribed in that box. The runtime keeps a per-scanline span
/// for those two (`byte *[+96]`, four bytes a row, written by the same ellipse stepper that draws the
/// outline) and `valid` tests *that* instead of the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub rect: Rect,
    /// `AEEClipShape`, carried so `GetClip` can name back what was set.
    pub kind: u32,
}

/// The three things every shape needs: where the pixels go, the state that says which colour and
/// window to use, and the memory a draw may have to read. The window's span is resolved once, here,
/// because every loop below has to be clamped by it before it starts.
pub struct Ctx<'a> {
    mem: &'a mut Mem,
    display: &'a mut Display,
    st: State,
    /// The inclusive span the window allows, or `None` when it allows nothing.
    win: Option<(i32, i32, i32, i32)>,
    /// A clip declared as a circle or an ellipse, in device coordinates: centre and the two semi-axes.
    /// `None` for a rectangular clip, which the span above already covers.
    ellipse: Option<(i32, i32, i32, i32)>,
    /// The colour-to-index answers this call has already worked out for its destination.
    ///
    /// One per call, and that is the whole reason it can be kept at all: `st` is copied in and never
    /// written here, so the destination cannot change under it, and a drawing call cannot be
    /// interrupted by the guest — which is the only thing that can rewrite a palette. It is three
    /// words until a pixel lands on a bitmap that keeps an index, and most `IGraphics` calls never
    /// get that far: their destination is the screen, which stores colours.
    cache: IndexMemo,
}

impl<'a> Ctx<'a> {
    pub fn new(mem: &'a mut Mem, display: &'a mut Display, st: State) -> Self {
        let win = Graphics::window(&st).span();
        // The elliptical clip's per-row span, worked out from the same box the body hands its stepper:
        // semi-axes `(extent - 1) / 2`, centre the box corner plus them. The box is already in device
        // coordinates, like every other clip.
        let ellipse = match st.clip {
            Some(region)
                if region.kind == g::CLIPPING_CIRCLE || region.kind == g::CLIPPING_ELLIPSE =>
            {
                let (x, y) = (i32::from(region.rect.x), i32::from(region.rect.y));
                let (rx, ry) = (
                    (i32::from(region.rect.dx) - 1) / 2,
                    (i32::from(region.rect.dy) - 1) / 2,
                );
                Some((x + rx, y + ry, rx, ry))
            }
            _ => None,
        };
        Ctx {
            mem,
            display,
            st,
            win,
            ellipse,
            cache: IndexMemo::new(),
        }
    }

    /// The columns this row admits. `None` means a rectangular clip, which the window's own span
    /// already covers; an empty span (a row outside the figure) comes back as the pair `1, 0`.
    fn row_span(&self, y: i32) -> Option<(i32, i32)> {
        let (cx, cy, rx, ry) = self.ellipse?;
        let dy = y - cy;
        if dy.abs() > ry {
            return Some((1, 0));
        }
        if ry == 0 {
            return Some((cx - rx, cx + rx));
        }
        let t = dy as f64 / ry as f64;
        let half = ((rx as f64) * (1.0 - t * t).max(0.0).sqrt()).floor() as i32;
        Some((cx - half, cx + half))
    }

    /// Whether one pixel is inside an elliptical clip, for the callers that plot rather than run.
    fn inside(&self, x: i32, y: i32) -> bool {
        match self.row_span(y) {
            Some((lo, hi)) => x >= lo && x <= hi,
            None => true,
        }
    }

    /// The column range clipped to the window, or `None` when the run cannot land at all.
    fn columns(&self, from: i32, to: i32) -> Option<(i32, i32)> {
        let (l, _, r, _) = self.win?;
        let (a, b) = (from.min(to), from.max(to));
        let (a, b) = (a.max(l), b.min(r));
        (a <= b).then_some((a, b))
    }

    /// One inclusive horizontal run, clipped.
    fn run(&mut self, y: i32, from: i32, to: i32, c: u32) {
        let Some((_, t, _, b)) = self.win else {
            return;
        };
        if y < t || y > b {
            return;
        }
        let Some((a, z)) = self.columns(from, to) else {
            return;
        };
        let (a, z) = match self.row_span(y) {
            Some((lo, hi)) => (a.max(lo), z.min(hi)),
            None => (a, z),
        };
        if a > z {
            return;
        }
        for x in a..=z {
            self.paint(x, y, c);
        }
    }

    /// The one place a pixel is written. Callers have clipped to the window already, so the only
    /// refusal left is the display's own bounds check on a bitmap this host owns.
    fn paint(&mut self, x: i32, y: i32, c: u32) {
        self.paint_rop(x, y, c, g::AEE_RO_COPY);
    }

    /// The same, with the raster op a caller named instead of a plain copy — which is only ever
    /// asked for by the slots whose prototype carries an op of their own.
    fn paint_rop(&mut self, x: i32, y: i32, c: u32, rop: u32) {
        let target = if self.st.dest == 0 {
            self.display.device_bitmap()
        } else {
            self.st.dest
        };
        // A colour, not a value the target stores: `c` came from the drawing state, which
        // `IGraphics::SetColor` resolved from an `RGBVAL`. The memo is this call's, which is what
        // lets a filled rectangle ask the destination's palette once instead of once a pixel —
        // about 64 ns a pixel against 1310 on a 256-entry table that does not name the colour, and
        // `fill` alone calls this 76800 times for a full screen. `put` is the one that decides
        // whether the memo is needed at all: on the 16-bpp screen that is every shape, and it is
        // never touched there.
        self.display
            .draw_pixel_colour(
                self.mem,
                target,
                x as u32,
                y as u32,
                c,
                rop,
                &mut self.cache,
            )
            .ok();
    }

    /// `IGraphics::StretchBlt`: all of a decoded picture, scaled into the rectangle the caller named,
    /// under the raster op it named.
    ///
    /// Nearest neighbour, deliberately: the ABI offers [`crate::device`]'s one pixel per destination
    /// pixel with no filter to consult, and a pack asking for a stretched sprite wants the sprite, not
    /// a blur of it. The destination is clipped through the same window every other shape uses, so a
    /// blit that hangs off the edge of a viewport lands the same way a rectangle does.
    pub fn stretch(
        &mut self,
        img: &crate::resource::Bitmap,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        rop: u32,
    ) {
        // `clamped` is what proves `w` and `h` are above zero below, and answers nothing at all for a
        // rectangle with no pixels in it.
        let Some((x0, x1, y0, y1)) = self.clamped(x, y, w, h) else {
            return;
        };
        // Asked once for the whole blit, in the same shape `Display::copy` asks it: `paint_rop` ends
        // in `.ok` because a pixel has nowhere to report to, so without this an op this host cannot
        // carry out became ten thousand silently discarded errors — a sprite that paints nothing and
        // says nothing, which is the failure shape this file keeps having to hunt.
        if !crate::display::rop_supported(rop) {
            self.display.note(format!("unsupported raster op {rop}"));
            return;
        }
        if img.width == 0 || img.height == 0 {
            return;
        }
        let (sw, sh) = (img.width as i32, img.height as i32);
        // `AEE_RO_TRANSPARENT` names the *source* pixels — "the SRC* pixels with a certain color are
        // transparent... where SRC is the source bitmap buffer" — and this
        // source is a Windows BMP: `IGRAPHICS_StretchBlt` takes "a Windows BMP structure containing the
        // source bitmap. The data being pointed is without the AEE header",
        // so there is no `ncTransparent` on it for this host to ask. What the headers do say is which
        // colour that is when nothing named one: the header calls these the *default*
        // transparent colours, magenta on a colour device and white on a monochrome one, and
        // the header repeats it for this very op by name. So the skip happens on the source
        // pixel, and the write below is a plain copy — the shape `Display::copy` and both transform
        // paths already use.
        //
        // The mask is the *destination's*, not always magenta: a 1-bpp bitmap is the monochrome case
        // the header names, and `default_transparent` already takes that branch for that
        // reason. Hard-coding magenta would stop a 1-bpp screen from keying on white at all.
        //
        // Comparing the *destination's* own key here is a different question. `IBitmap_SetTransparencyColor`
        // says that colour is used "when this bitmap is the source bitmap of a transparent bit blit",
        // and a destination is not a source. On a colour screen whose key is still the default the two
        // agree — which is why this read as harmless for as long as it did.
        let depth = self.display.bitmap(self.st.dest).map_or(16, |b| b.depth);
        let mask_rgb = if depth == 1 {
            crate::device::RGB_WHITE
        } else {
            crate::device::RGB_MASK_COLOR
        };
        let mask = rgb_to_native(mask_rgb);
        let transparent = rop == g::AEE_RO_TRANSPARENT;
        let write_rop = if transparent { g::AEE_RO_COPY } else { rop };
        for row in y0..=y1 {
            let sy = ((row - y) * sh / h).clamp(0, sh - 1) as usize;
            for col in x0..=x1 {
                let sx = ((col - x) * sw / w).clamp(0, sw - 1) as usize;
                // What the file says about this pixel's coverage. A see-through one paints nothing,
                // which is the whole of what this entry can honour: `paint_rop` has no memory to
                // read the destination from, so a *partly* covered pixel is laid down as stored
                // rather than composed. A tile that arrives through `IImage::Draw` and then
                // `Display::copy` is not on this path — that one composes — and naming the
                // difference is what keeps this from reading as a second, quieter white box.
                if img.alpha_of(sx, sy) == 0 || !self.inside(col, row) {
                    continue;
                }
                let (r, g, b) = img.rgb(sx, sy);
                let native = Rgba::new(r, g, b, 255).native();
                if transparent && native == mask {
                    continue;
                }
                self.paint_rop(col, row, native, write_rop);
            }
        }
    }

    /// A filled axis-aligned box.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let Some((x0, x1, y0, y1)) = self.clamped(x, y, w, h) else {
            return;
        };
        for row in y0..=y1 {
            self.run(row, x0, x1, c);
        }
    }

    /// The outline of an axis-aligned box, drawn with the pen.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let Some((x0, x1, y0, y1)) = self.clamped(x, y, w, h) else {
            return;
        };
        self.run(y0, x0, x1, c);
        self.run(y1, x0, x1, c);
        for row in y0..=y1 {
            self.run(row, x0, x0, c);
            self.run(row, x1, x1, c);
        }
    }

    /// A box clipped to the window, as inclusive corners. A zero or negative extent is a shape with
    /// nothing in it, and the two entry points have to agree on that or a flipped run paints pixels
    /// left of where the guest pointed.
    fn clamped(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(i32, i32, i32, i32)> {
        if w <= 0 || h <= 0 {
            return None;
        }
        let (x0, x1) = self.columns(x, x + w - 1)?;
        let (_, t, _, b) = self.win?;
        let (y0, y1) = (y.max(t), (y + h - 1).min(b));
        (y0 <= y1).then_some((x0, x1, y0, y1))
    }

    /// A box with its corners cut to the ellipse the caller named — `IGraphics::DrawRoundRectangle`,
    /// in both the filled and the outlined form a handset draws for a dialog.
    ///
    /// Row by row rather than assembled from [`Ctx::disc`], because the two radii differ: an
    /// `arcWidth` of 8 against an `arcHeight` of 2 is a squashed corner, and no circle makes one. Each
    /// row's span is inset by however much the corner ellipse has eaten into it, which is also what
    /// makes the outline exact: a pixel is on the edge when the row above or below does not cover it,
    /// or when it is where this row starts or stops.
    #[allow(clippy::too_many_arguments)]
    pub fn round_rect(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        rx: i32,
        ry: i32,
        c: u32,
        filled: bool,
    ) {
        let Some((x0, x1, y0, y1)) = self.clamped(x, y, w, h) else {
            return;
        };
        // A corner radius past half the box is the box with its corners meeting in the middle, which
        // is what a caller asking for one means; clamping keeps the ellipse test monotone.
        let (rx, ry) = (rx.min(w / 2).max(0), ry.min(h / 2).max(0));
        let mut spans = Vec::new();
        for row in y0..=y1 {
            let yy = row - y;
            let oy = (ry - yy.min(h - 1 - yy)).max(0);
            let inset = if rx == 0 || oy == 0 {
                0
            } else {
                rx - Self::corner_run(rx, ry, oy)
            };
            spans.push((row, (x + inset).max(x0), (x + w - 1 - inset).min(x1)));
        }
        for (i, (row, a, b)) in spans.iter().enumerate() {
            if filled {
                self.run(*row, *a, *b, c);
                continue;
            }
            let covers = |span: Option<&(i32, i32, i32)>, px: i32| {
                span.is_some_and(|(_, sa, sb)| px >= *sa && px <= *sb)
            };
            let above = i.checked_sub(1).map(|j| &spans[j]);
            let below = spans.get(i + 1);
            for px in *a..=*b {
                if px == *a || px == *b || !covers(above, px) || !covers(below, px) {
                    self.run(*row, px, px, c);
                }
            }
        }
    }

    /// How far into the corner band a row may reach: the largest horizontal distance whose corner
    /// pixel is still inside the ellipse of half-axes `rx` and `ry`. Tested as
    /// `(ox·ry)² + (oy·rx)² ≤ (rx·ry)²` in `i64`, which is the ellipse without a square root.
    fn corner_run(rx: i32, ry: i32, oy: i32) -> i32 {
        let limit = i64::from(rx) * i64::from(ry);
        let mut ox = 0;
        while ox < rx {
            let next = i64::from(ox + 1) * i64::from(ry);
            let y = i64::from(oy) * i64::from(rx);
            if next * next + y * y > limit * limit {
                break;
            }
            ox += 1;
        }
        ox
    }

    /// Bresenham.
    pub fn line(&mut self, line: Line) {
        let (mut x, mut y) = line.a;
        let (x1, y1) = line.b;
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
        let (c, dotted) = (self.st.fg.native(), self.st.stroke != 0);
        if self.win.is_none() {
            return;
        }
        let mut err = dx + dy;
        let mut n = 0u32;
        loop {
            if !dotted || n.is_multiple_of(2) {
                self.run(y, x, x, c);
            }
            n += 1;
            if x == x1 && y == y1 {
                return;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Midpoint circle, outline only. The error term is the published form, with `x` decremented
    /// before it: verified to keep every plotted pixel within half a pixel of the true radius for
    /// every radius to 60, which the test below re-checks.
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32) {
        let c = self.st.fg.native();
        if r < 0 {
            return;
        }
        if r == 0 {
            self.run(cy, cx, cx, c);
            return;
        }
        let (mut x, mut y, mut err) = (r, 0, 1 - r);
        while x >= y {
            for (px, py) in [
                (cx + x, cy + y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx - x, cy + y),
                (cx - x, cy - y),
                (cx - y, cy - x),
                (cx + y, cy - x),
                (cx + x, cy - y),
            ] {
                self.run(py, px, px, c);
            }
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// A filled disc, for `SetFillMode` with `DrawCircle`: every scanline between the two halves of
    /// the circle is run through the same clipped row as the outline.
    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        if r < 0 {
            return;
        }
        for dy in -r..=r {
            // The integer half-width of the circle at this row, from r^2 - dy^2.
            let inside = (r * r - dy * dy) as f64;
            let half = inside.sqrt().floor() as i32;
            self.run(cy + dy, cx - half, cx + half, c);
        }
    }

    /// `IGraphics::DrawPie`: the wedge of a circle between two bearings, filled or outlined.
    ///
    /// Outlined is the arc *plus the two closing radii*, so the figure is closed at any sweep; filled
    /// is every pixel whose bearing falls inside, put through the same clipped row the circle and the
    /// polygon use.
    pub fn sector(&mut self, cx: i32, cy: i32, r: i32, start: i16, sweep: i16, filled: bool) {
        self.wedge(cx, cy, r, start, sweep, filled, true);
    }

    /// `IGraphics::DrawArc`: the curve of a circle between two bearings, and nothing else.
    ///
    /// The header gives this one no fill and no closing lines — "the color of the arc is the current
    /// foreground color" is the whole of it — which is the only difference from [`Ctx::sector`] and
    /// the reason the two are not one method with a flag at the call site.
    pub fn arc(&mut self, cx: i32, cy: i32, r: i32, start: i16, sweep: i16) {
        self.wedge(cx, cy, r, start, sweep, false, false);
    }

    /// The circle wedge behind both, with the radii optional.
    ///
    /// The angles follow the convention the header spells out where it documents the elliptical arc:
    /// 0 is the positive x axis and they increase counterclockwise. The screen's y grows downward, so
    /// the bearing is taken with `cy - y` — that is what makes a positive sweep turn the way the pack
    /// expects to see it rather than mirrored in the x axis. A sweep of a full turn or more is one
    /// full turn, because a handset draws no second lap, and a negative one is the same wedge swept
    /// the other way, which is what the signed `int16` is there for.
    #[allow(clippy::too_many_arguments)]
    fn wedge(
        &mut self,
        cx: i32,
        cy: i32,
        r: i32,
        start: i16,
        sweep: i16,
        filled: bool,
        radii: bool,
    ) {
        if r < 0 {
            return;
        }
        let sign = if sweep < 0 { -1.0 } else { 1.0 };
        let span = (sweep as i32).unsigned_abs().min(360) as f64;
        let inside = |px: i32, py: i32| -> bool {
            if span >= 360.0 || (px == cx && py == cy) {
                return true;
            }
            let bearing = ((cy - py) as f64).atan2((px - cx) as f64).to_degrees();
            ((bearing - start as f64) * sign).rem_euclid(360.0) <= span
        };
        if filled {
            let c = self.st.fill.native();
            for dy in -r..=r {
                let half = ((r * r - dy * dy) as f64).sqrt().floor() as i32;
                for x in (cx - half)..=(cx + half) {
                    if inside(x, cy + dy) {
                        self.run(cy + dy, x, x, c);
                    }
                }
            }
            return;
        }
        let c = self.st.fg.native();
        // The arc is the midpoint circle's own eight symmetric points, kept only where the bearing is
        // inside the wedge — so the curve it traces is the same pixels `circle` would have plotted.
        let (mut x, mut y, mut err) = (r, 0, 1 - r);
        while x >= y {
            for (px, py) in [
                (cx + x, cy + y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx - x, cy + y),
                (cx - x, cy - y),
                (cx - y, cy - x),
                (cx + y, cy - x),
                (cx + x, cy - y),
            ] {
                if inside(px, py) {
                    self.run(py, px, px, c);
                }
            }
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
        if radii {
            for deg in [start as f64, start as f64 + sweep as f64] {
                let (cos, sin) = deg.to_radians().sin_cos();
                self.line(Line {
                    a: (cx, cy),
                    b: (
                        cx + (r as f64 * cos).round() as i32,
                        cy - (r as f64 * sin).round() as i32,
                    ),
                });
            }
        }
    }

    /// `IGraphics::DrawEllipse` and `DrawEllipticalArc`: the axis-aligned ellipse centred on
    /// `(cx, cy)` with the two semi-axes the caller named, over the bearing range
    /// `start..= start + sweep`.
    ///
    /// The bearings are the same convention [`Ctx::wedge`] draws to — 0 on the positive x axis,
    /// increasing counterclockwise against a screen whose y grows down — with one addition an ellipse
    /// forces: a point's bearing is taken on the *normalised* offsets, `(rx·sin, ry·cos)` rather than
    /// the raw pixel offsets. That is what makes the wedge's two ends land where the caller's degrees
    /// say on a squashed figure; testing the raw offset would put the end of a 45° sweep somewhere
    /// else entirely on a 40×10 ellipse.
    ///
    /// Outlined is the curve alone, stroked as consecutive samples of the parametric form — the
    /// header gives `DrawEllipticalArc` no closing radii either, and a full turn closes on itself.
    /// Filled is every pixel inside the figure whose bearing is inside the sweep, which for a full
    /// turn is the ellipse's interior and so what `DrawEllipse` with fill mode on means.
    #[allow(clippy::too_many_arguments)]
    pub fn elliptical_wedge(
        &mut self,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        start: i16,
        sweep: i16,
        filled: bool,
    ) {
        // A semi-axis of zero is a figure with no pixels in it, and a negative one is the same
        // figure named the other way round: neither is worth stroking a degenerate path for.
        if rx <= 0 || ry <= 0 {
            return;
        }
        let (rx, ry) = (rx as f64, ry as f64);
        let sign = if sweep < 0 { -1.0 } else { 1.0 };
        let span = (sweep as i32).unsigned_abs().min(360) as f64;
        let turn = |px: i32, py: i32| -> bool {
            if span >= 360.0 || (px == cx && py == cy) {
                return true;
            }
            let bearing = ((cy - py) as f64 / ry)
                .atan2((px - cx) as f64 / rx)
                .to_degrees();
            ((bearing - start as f64) * sign).rem_euclid(360.0) <= span
        };
        if filled {
            let c = self.st.fill.native();
            for dy in -(ry.ceil() as i32)..=ry.ceil() as i32 {
                // The row's half-width from the ellipse's own equation, floored the way the disc's is.
                // The apex row — where the width is exactly zero — still paints its one pixel, so the
                // filled figure reaches the same top and bottom the outline does.
                let inside = 1.0 - (dy as f64 * dy as f64) / (ry * ry);
                if inside < 0.0 {
                    continue;
                }
                let half = (rx * inside.sqrt()).floor() as i32;
                for x in (cx - half)..=(cx + half) {
                    if turn(x, cy + dy) {
                        self.run(cy + dy, x, x, c);
                    }
                }
            }
            return;
        }
        // One sample per pixel of arc length, bounded: a 1° step is already finer than the screen on
        // anything but the largest ellipse, and a fixed small step would make a 3-pixel ellipse spend
        // thousands of clipped rows on a curve of six pixels.
        let steps = ((rx + ry) * span.to_radians()).ceil() as usize;
        let steps = steps.clamp(2, 1440);
        let point = |i: usize| {
            let deg = start as f64 + sign * span * i as f64 / steps as f64;
            let (cos, sin) = deg.to_radians().sin_cos();
            (
                cx + (rx * cos).round() as i32,
                cy - (ry * sin).round() as i32,
            )
        };
        let first = point(0);
        let mut prev = first;
        for i in 1..=steps {
            let next = if i == steps && span >= 360.0 {
                first
            } else {
                point(i)
            };
            // Every segment carries its own start pixel, so the walk paints the whole curve with the
            // pen `line` already reads from the state — and a repeated sample costs nothing back.
            if next != prev {
                self.line(Line { a: prev, b: next });
            }
            prev = next;
        }
    }

    /// `IGraphics::DrawPolygon` and `DrawPolyline`: the ring of points the caller named, filled or
    /// outlined. `close` is what separates the two — a polygon's last vertex joins back to its first,
    /// a polyline's does not — and `filled` follows the pen mode the caller set with `SetFillMode`.
    ///
    /// Filled goes row by row with an even-odd crossing test rather than as a fan from the first
    /// vertex, because nothing in the header promises the ring is convex: a four-point diamond is the
    /// shape a fan cannot fill, and a fan leaves the far side of a concave or self-crossing ring empty. Each row
    /// collects the columns where its scanline crosses an edge and fills them in pairs, which is the
    /// same rule the outline implies. The crossing is half-open (`top <= y < bottom`) so a vertex
    /// shared by two edges is counted once and a pair does not collapse.
    pub fn polygon(&mut self, pts: &[(i32, i32)], c: u32, filled: bool, close: bool) {
        if pts.len() < 2 {
            return;
        }
        let n = pts.len();
        if !filled {
            for i in 0..if close { n } else { n - 1 } {
                self.line(Line {
                    a: pts[i],
                    b: pts[(i + 1) % n],
                });
            }
            return;
        }
        let Some((_, top, _, bottom)) = self.win else {
            return;
        };
        let (ymin, ymax) = pts.iter().fold((i32::MAX, i32::MIN), |mut a, &(_, y)| {
            a.0 = a.0.min(y);
            a.1 = a.1.max(y);
            a
        });
        let stop = if close { n } else { n - 1 };
        for y in ymin.max(top)..=ymax.min(bottom) {
            let mut crossings: Vec<i32> = Vec::new();
            for i in 0..stop {
                let (mut ax, mut ay) = pts[i];
                let (mut bx, mut by) = pts[(i + 1) % n];
                if ay > by {
                    ((ax, ay), (bx, by)) = ((bx, by), (ax, ay));
                }
                if ay <= y && y < by {
                    let d = (by - ay) as i64;
                    let x = ax as i64 + ((bx - ax) as i64 * (y - ay) as i64 + d / 2) / d;
                    crossings.push(x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
                }
            }
            crossings.sort_unstable();
            for pair in crossings.chunks(2) {
                if pair.len() == 2 {
                    self.run(y, pair[0], pair[1] - 1, c);
                }
            }
        }
    }
}

/// The `IGraphics` objects the host has handed out, and the state each of them holds.
///
/// The *pixels* are not here: an `IGraphics` draws into a bitmap, and a bitmap's bits live in guest
/// memory, which the state block already carries. What has to travel is the pen, the fill mode, the
/// viewport and the destination — a restored machine that had forgotten them would paint its next
/// shape in the wrong colour, into the wrong rectangle.
impl Graphics {
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        use crate::mem::{put_bool, put_u32, put_u8, sorted_u32};
        let rows = sorted_u32(&self.objects);
        put_u32(out, rows.len() as u32);
        for (addr, state) in rows {
            put_u32(out, addr);
            put_rgba(out, &state.fg);
            put_rgba(out, &state.bg);
            put_rgba(out, &state.fill);
            put_bool(out, state.filled);
            put_u8(out, state.point);
            put_u32(out, state.stroke);
            put_u32(out, state.hint);
            put_u32(out, state.paint);
            crate::display::put_rect(out, state.viewport);
            match state.clip {
                Some(region) => {
                    put_bool(out, true);
                    crate::display::put_rect(out, region.rect);
                    put_u8(out, region.kind as u8);
                }
                None => put_bool(out, false),
            }
            match state.clip_box {
                Some(clip) => {
                    put_bool(out, true);
                    crate::display::put_rect(out, clip);
                }
                None => put_bool(out, false),
            }
            out.extend_from_slice(&state.clip_raw);
            put_u32(out, state.dest);
            put_u32(out, state.depth);
            put_u32(out, state.display);
            put_bool(out, state.framed);
            put_u32(out, state.translate.0 as u32);
            put_u32(out, state.translate.1 as u32);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut crate::mem::State) -> Result<(), String> {
        let count = st.small()?;
        let mut objects = HashMap::with_capacity(count);
        for _ in 0..count {
            let addr = st.u32()?;
            let fg = get_rgba(st)?;
            let bg = get_rgba(st)?;
            let fill = get_rgba(st)?;
            let filled = st.u8()? != 0;
            let point = st.u8()?;
            let stroke = st.u32()?;
            let hint = st.u32()?;
            let paint = st.u32()?;
            let viewport = crate::display::get_rect(st)?;
            let clip = if st.u8()? != 0 {
                Some(Region {
                    rect: crate::display::get_rect(st)?,
                    kind: st.u8()? as u32,
                })
            } else {
                None
            };
            let clip_box = if st.u8()? != 0 {
                Some(crate::display::get_rect(st)?)
            } else {
                None
            };
            let mut clip_raw = [0u8; 16];
            for byte in clip_raw.iter_mut() {
                *byte = st.u8()?;
            }
            let dest = st.u32()?;
            let depth = st.u32()?;
            let display = st.u32()?;
            let framed = st.u8()? != 0;
            let translate = (st.u32()? as i32, st.u32()? as i32);
            objects.insert(
                addr,
                State {
                    fg,
                    bg,
                    fill,
                    filled,
                    point,
                    stroke,
                    hint,
                    paint,
                    viewport,
                    clip,
                    clip_box,
                    clip_raw,
                    framed,
                    translate,
                    dest,
                    depth,
                    display,
                },
            );
        }
        self.objects = objects;
        Ok(())
    }
}

/// A colour as four bytes, in the order the struct declares them.
///
/// `display` carries the matching pair for `Rect`; each lives with the type it reads rather than in a
/// shared "state helpers" module, so a type that changes shape changes exactly one file.
#[allow(dead_code)]
pub(crate) fn put_rgba(out: &mut Vec<u8>, c: &Rgba) {
    out.extend_from_slice(&[c.r, c.g, c.b, c.a]);
}

#[allow(dead_code)]
pub(crate) fn get_rgba(st: &mut crate::mem::State) -> Result<Rgba, String> {
    let b = st.take(4)?;
    Ok(Rgba::new(b[0], b[1], b[2], b[3]))
}

#[cfg(test)]
#[path = "test/graphics.rs"]
mod tests;
