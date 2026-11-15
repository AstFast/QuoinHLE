//! `ISprite`: four sprite buffers, four tile buffers, one destination, and the two walks that turn a
//! caller's command list or map array into rectangle copies.
//!
//! The shipped body contains no drawing of its own: every
//! pixel leaves through a blit on the destination the caller handed to `SetDestination` —
//! `IBITMAP_BltIn` for the simple bodies and
//! `ITRANSFORM_TransformBlt{Simple,Complex}` for the other pair. That is why
//! the geometry here is arithmetic over integers rather than drawing code — the object's job is to
//! decide *which* rectangles, and the host already knows how to blit one.
//!
//! The two bodies the shipped object ships for each draw slot are picked by `SetDestination`: it asks
//! the destination for an `ITransform` and installs the transform-backed bodies over its own
//! vtable if it gets one or the simple ones if it does not. The two walks are the
//! same arithmetic, so only the last step differs — and the difference is visible: the transform bodies
//! read `unTransform` and a tile's bits 15-13, where those bits are the very same
//! `TRANSFORM_FLIP_`/`TRANSFORM_ROTATE_`/`TRANSFORM_SCALE_` packing `IBitmap::TransformBltSimple` is
//! called with elsewhere (the header spells its flip, rotation and scale codes out of those
//! names, and `SPRITE_MATRIX_TRANSFORM` is bit 15, the header), and a command that names a matrix
//! while no table is installed draws nothing and sets `EBADPARM`, which the simple bodies
//! do neither.

use std::collections::HashMap;

use crate::abi::generated as g;

/// `aSpriteSize` and `aTileSize`: the four size codes are edges in pixels.
pub const SIZE_PX: [u32; 4] = [8, 16, 32, 64];

/// `aTileSizeMask` — the low bits of the same four edges.
pub const SIZE_MASK: [u32; 4] = [0x07, 0x0F, 0x1F, 0x3F];

/// `aTileSizeBits` — how far to shift a scroll offset to count whole tiles.
pub const SIZE_BITS: [u32; 4] = [3, 4, 5, 6];

/// `aMapSizeMask`, indexed by the `MAP_SIZE_*` log2 codes of `AEETileMap::w` and `h`.
pub const MAP_MASK: [u32; 11] = [
    0x0000, 0x0001, 0x0003, 0x0007, 0x000F, 0x001F, 0x003F, 0x007F, 0x00FF, 0x01FF, 0x03FF,
];

/// `sizeof AEETransformMatrix` — four `int16` factors, which is what
/// turning `unMatrixTransform` into an address costs.
pub const MATRIX_BYTES: u32 = 8;

/// One `AEESpriteCmd`, the fields that carry meaning.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Cmd {
    /// The screen position of the sprite's upper-left corner.
    pub x: i32,
    pub y: i32,
    /// Bits 15, 5-3, 2, 1-0: matrix table in use, scale, flip-x, rotation. The simple body reads none
    /// of them; the transform body hands the whole field to `ITRANSFORM_TransformBltSimple`,
    /// which is why it travels with the rectangle instead of being decoded here.
    pub transform: u16,
    /// Index into the caller's matrix table, used only with bit 15.
    pub matrix: u8,
    /// Which row of the buffer this sprite is in: the source y is `edge * index`.
    pub index: u8,
    /// One of the four size codes; `SPRITE_SIZE_END` (0xFF) is what stops the walk.
    pub size: u8,
    /// `COMPOSITE_KEYCOLOR` (0) is transparent, anything else copies (the delivered body).
    pub composite: u8,
    /// 0-3, drawn in that order; `SPRITE_LAYER_HIDDEN` (0xFF) matches no pass.
    pub layer: u8,
}

/// One `AEETileMap`, the fields the walk uses.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Map {
    /// The caller's array of tile words. A null one ends the array of maps.
    pub array: u32,
    pub flags: u32,
    pub x: i32,
    pub y: i32,
    /// `MAP_SIZE_*` codes, not edges.
    pub w: u32,
    pub h: u32,
    pub size: u8,
}

/// One rectangle a draw slot asked for, in the words of the call it was going to make.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Blit {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Which buffer to take it out of: the address the caller installed for that size, which may
    /// legitimately be null, since `SetSpriteBuffer` accepts a null bitmap.
    pub src: u32,
    pub xs: u32,
    pub ys: u32,
    /// The command's own `unTransform`. The simple body never reads it; the transform body hands it to
    /// `ITRANSFORM_TransformBltSimple` as scale, flip and rotation.
    pub transform: u16,
    /// The matrix this command names, already added to the caller's table — `m_pMatrices +
    /// unMatrixTransform`. Null unless bit 15 is set, and bit 15 without a table is refused
    /// before it gets here.
    pub matrix: u32,
    /// `unComposite`, as given: `COMPOSITE_KEYCOLOR` (0) is transparent, anything else copies
    /// (the header). The transform body passes the same byte on.
    pub composite: u8,
}

/// One tile position to draw, and which element of the caller's map array names it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Slot {
    pub x: i32,
    pub y: i32,
    pub at: usize,
}

/// `Sprite_DrawSpritesSimple` and `Sprite_DrawSprites`, as far as
/// the plan goes.
///
/// The list is walked once per layer, `SPRITE_LAYER_MAX` times, because the shipped `for` is outside
/// the `while`: overlap is resolved by layer, and within a layer by list order. A size
/// outside the four does not stop the walk — it sets the result and keeps going, so the sprites after
/// it still draw, which is why the second half of the answer is a flag and not a
/// refusal, and why the destination can be modified by a call that reports `EBADPARM`.
///
/// `matrices` is the engine's current table, which is the one thing that separates the two bodies:
/// bit 15 of `unTransform` sends a command to the matrix path, and with no table installed that
/// command draws nothing and sets the same sticky `EBADPARM`. The simple body reads
/// neither, which is why the flag comes out of the plan and the caller decides what to do with it.
pub fn plan_sprites(cmds: &[Cmd], bufs: &[u32; 4], matrices: u32) -> (Vec<Blit>, bool) {
    let mut out = Vec::new();
    let mut bad = false;
    for pass in 0..g::SPRITE_LAYER_MAX as u8 {
        for cmd in cmds {
            let Some(&edge) = SIZE_PX.get(usize::from(cmd.size)) else {
                bad = true;
                continue;
            };
            if cmd.layer != pass {
                continue;
            }
            let complex = cmd.transform & g::SPRITE_MATRIX_TRANSFORM as u16 != 0;
            if complex && matrices == 0 {
                bad = true;
                continue;
            }
            out.push(Blit {
                x: cmd.x,
                y: cmd.y,
                w: edge,
                h: edge,
                src: bufs[usize::from(cmd.size)],
                xs: 0,
                ys: edge * u32::from(cmd.index),
                transform: cmd.transform,
                matrix: if complex {
                    matrices.wrapping_add(u32::from(cmd.matrix) * MATRIX_BYTES)
                } else {
                    0
                },
                composite: cmd.composite,
            });
        }
    }
    (out, bad)
}

/// The slots one map contributes, in the two shapes `MAP_FLAG_WRAP` distinguishes.
///
/// Wrapping is the scrolling case: the walk covers the destination from whichever tile its own edge
/// lands on, indices are taken modulo the map's dimensions so it tiles over infinitely, and it starts
/// one tile early when the scroll offset is inside a tile rather than on its edge —
/// `xStart = -((x & mask) != 0)` at, with the partial tile drawn at `x & mask`.
///
/// Not wrapping is placement: nothing outside the map's own rows and columns is drawn, and a
/// map pushed a whole map-height plus eight tiles off screen contributes nothing at all.
///
/// A map whose `w`/`h` are not `MAP_SIZE_*` codes, or whose tile size is not one of the four, is
/// refused by the caller before this runs; the empty answer here is a second line of
/// defence, not the refusal itself.
pub fn tile_slots(m: &Map, cx: u32, cy: u32) -> Vec<Slot> {
    let size = usize::from(m.size);
    let (Some(&px), Some(&mask), Some(&bits)) =
        (SIZE_PX.get(size), SIZE_MASK.get(size), SIZE_BITS.get(size))
    else {
        return Vec::new();
    };
    let (Some(&cmask), Some(&rmask)) = (MAP_MASK.get(m.w as usize), MAP_MASK.get(m.h as usize))
    else {
        return Vec::new();
    };
    let (cols, rows) = (u64::from(cmask) + 1, u64::from(rmask) + 1);
    let (edge, mask) = (u64::from(px), u64::from(mask));
    let mut out = Vec::new();
    if m.flags & g::MAP_FLAG_WRAP != 0 {
        // `(unsigned)pMaps->x`: a negative scroll offset becomes the huge number an `unsigned` cast
        // makes of it, and the masks below are what bring the index back inside the map.
        let (x, y) = (u64::from(m.x as u32), u64::from(m.y as u32));
        let (xm, ym) = ((x & mask) as i64, (y & mask) as i64);
        let (tx, ty) = ((x >> u64::from(bits)) as i64, (y >> u64::from(bits)) as i64);
        let first = |v: u64| if v & mask == 0 { 0i64 } else { -1i64 };
        let stop =
            |across: u32| i64::try_from((u64::from(across) + edge - 1) / edge).unwrap_or(i64::MAX);
        for j in first(y)..stop(cy) {
            for i in first(x)..stop(cx) {
                out.push(Slot {
                    x: (i * edge as i64 + xm) as i32,
                    y: (j * edge as i64 + ym) as i32,
                    // `(1 << w) * ((j - (y >> bits)) & aMapSizeMask[h]) + ((i - (x >> bits)) &
                    // aMapSizeMask[w])`: the multiplier is the column *count*, the two
                    // masks are the sizes minus one — masking by the count would keep one bit and
                    // lose every index but 0 and 2^w.
                    at: (cols * ((j - ty) as u64 & u64::from(rmask))
                        + ((i - tx) as u64 & u64::from(cmask))) as usize,
                });
            }
        }
        return out;
    }
    // `y + (1 << (h + unTileSize + 3)) > 0`, and the same along x: a map pushed a whole map-height
    // plus eight tiles off the screen contributes nothing at all.
    let reach = |dim: u32| 1i64 << (i64::from(dim) + 3 + i64::from(m.size));
    if i64::from(m.y) + reach(m.h) <= 0 || i64::from(m.x) + reach(m.w) <= 0 {
        return out;
    }
    let start = |at: i32| {
        if at > 0 {
            0
        } else {
            u64::try_from(-i64::from(at)).unwrap_or(0) / edge
        }
    };
    // `(unsigned)(bi.cx - x + mask) / edge`, against the map's own extent. A negative is the huge
    // number an `unsigned` cast makes of it, so the `MIN` then leaves the extent standing: the whole
    // row is walked, and the blit is what clips the tiles that miss the screen.
    let stop = |across: u32, at: i32, extent: u64| {
        let room = i64::from(across) - i64::from(at) + mask as i64;
        if room < 0 {
            return extent;
        }
        extent.min(u64::try_from(room).unwrap_or(0) / edge)
    };
    let (x_stop, y_stop) = (stop(cx, m.x, cols), stop(cy, m.y, rows));
    for j in start(m.y)..y_stop {
        for i in start(m.x)..x_stop {
            out.push(Slot {
                x: m.x.wrapping_add((i * edge) as i32),
                y: m.y.wrapping_add((j * edge) as i32),
                at: (cols * j + i) as usize,
            });
        }
    }
    out
}

#[cfg(test)]
#[path = "test/sprite.rs"]
mod tests;

/// The per-object state. `Sprite_New` allocates a fresh object per call, so unlike
/// `IWebUtil` this one does hold things: four sprite buffers and four tile buffers indexed by size
/// code, the destination, and the caller's matrix table.
#[derive(Copy, Clone, Default, Debug)]
pub struct Engine {
    /// Referenced, never copied, never freed, which is why it is a guest
    /// address rather than something the host allocated.
    pub matrices: u32,
    pub sprite_bufs: [u32; 4],
    pub tile_bufs: [u32; 4],
    /// The destination. The shipped object holds **two** references to it — one through `IBitmap`, and
    /// one through the `ITransform` it asked that bitmap for, which is the face the draw
    /// slots then blit through. Null is the shipped "simple bodies" answer, not an error.
    pub dst: u32,
    pub transform: u32,
}

#[derive(Default)]
pub struct Sprites {
    objects: HashMap<u32, Engine>,
}

impl Sprites {
    pub fn add(&mut self, addr: u32) {
        self.objects.insert(addr, Engine::default());
    }

    pub fn forget(&mut self, addr: u32) -> Option<Engine> {
        self.objects.remove(&addr)
    }

    pub fn engine(&self, addr: u32) -> Option<Engine> {
        self.objects.get(&addr).copied()
    }

    pub fn put(&mut self, addr: u32, engine: Engine) {
        self.objects.insert(addr, engine);
    }
}
