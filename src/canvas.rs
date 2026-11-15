//! The Brew MP canvas: `ICanvas` is an `IBitmap` a widget may draw on, plus a clip rectangle;
//! `IDisplayCanvas` adds the display that bitmap belongs to.
//!
//! The header interface methods"*. So this file owns no pixels and no drawing code — it is a pointer, a
//! rectangle, and the accessors the header declares. Everything a widget paints goes through
//! [`crate::display`]'s `IBitmap`, which this host already serves in full (17 slots, `DIB_NewEx`
//! blocks included).
//!
//! # Why the state lives in the guest object
//!
//! There is no `HashMap` of canvases. The three fields are written into the object block, the same
//! way `IShell`'s and `IDisplay`'s state records keep the guest's view and the host's view in one
//! place, so a `Release` that reaches zero needs no cleanup at all — forgetting the object is
//! forgetting the canvas. It also means there is exactly one copy of the clip rectangle, which is
//! the value `ICanvas_SetClipRect` last wrote.
//!
//! # The startup sequence this has to survive
//!
//! The header `AEECLSID_DisplayCanvas` exists as a class at all:
//!
//! ```text
//! ISHELL_CreateInstance(piShell, AEECLSID_DisplayCanvas, (void**)&piCanvas);
//! IDisplayCanvas_SetDisplay(piCanvas, piDisplay);
//! IRootContainer_SetCanvas(picRoot, (ICanvas*)(void*)piCanvas, &rcContainer);
//! ```
//!
//! The first two lines are this file's job. The third is `IRootContainer`'s, and it is where the
//! widget work starts, and that layer is not part of this file.

/// `IBitmap*` the canvas draws into, as a borrowed interface — the header documents
/// `ICanvas_GetBitmap` and `ICanvas_SetBitmap` as accessors with no reference-count language, and
/// `IDisplayCanvas`'s own `SetDisplay` neither, so none of the three move a count.
pub const BITMAP_OFF: u32 = 8;

/// The clip rectangle, an `AEERect` — `int16 x, y, dx, dy` (`spec/brew_mp_abi.json`, 8 bytes).
pub const CLIP_OFF: u32 = 12;

/// `IDisplay*` the canvas belongs to.
pub const DISPLAY_OFF: u32 = 20;

/// Bytes one canvas object occupies: vptr, refs, the three fields above.
pub const SIZE: u32 = 24;

/// Where the reference count sits, for [`crate::hle::Hle::new_object_with_vtable`].
pub const REFS_OFF: u32 = 4;

/// `AEEIID_ICanvas`.
pub const IID_ICANVAS: u32 = 0x0101_e3f2;

/// `AEEIID_IDisplayCanvas`.
pub const IID_IDISPLAYCANVAS: u32 = 0x0101_e443;

/// `AEECLSID_DisplayCanvas` / `AEECLSID_CDisplayCanvas` (`spec/brew_mp_abi.json`; the two names
/// carry one number, which is what tells this host the class and its implementation are the same
/// object and not two).
pub const CLSID_DISPLAYCANVAS: u32 = 0x0105_51c8;

/// An `AEERect` in the order the struct lays it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Clip {
    pub x: i16,
    pub y: i16,
    pub dx: i16,
    pub dy: i16,
}

impl Clip {
    /// The rectangle covering a bitmap of `w`×`h`, which is what a fresh canvas clips to.
    pub fn whole(w: u32, h: u32) -> Clip {
        Clip {
            x: 0,
            y: 0,
            dx: w.min(i16::MAX as u32) as i16,
            dy: h.min(i16::MAX as u32) as i16,
        }
    }

    /// True when nothing can be painted through this rectangle.
    pub fn is_empty(&self) -> bool {
        self.dx <= 0 || self.dy <= 0
    }

    /// The two words this host writes at [`CLIP_OFF`]: `x | y<<16`, then `dx | dy<<16`.
    pub fn words(self) -> [u32; 2] {
        [
            (self.x as u16 as u32) | ((self.y as u16 as u32) << 16),
            (self.dx as u16 as u32) | ((self.dy as u16 as u32) << 16),
        ]
    }

    /// The inverse of [`Clip::words`].
    pub fn from_words(words: [u32; 2]) -> Clip {
        Clip {
            x: words[0] as u16 as i16,
            y: (words[0] >> 16) as u16 as i16,
            dx: words[1] as u16 as i16,
            dy: (words[1] >> 16) as u16 as i16,
        }
    }
}

#[cfg(test)]
#[path = "test/canvas.rs"]
mod tests;
