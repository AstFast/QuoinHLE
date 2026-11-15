//! Free functions shared by the GL dispatch, rasteriser and pixel readers.

use super::enums::*;
use super::{ClipVertex, HResult, TxVertex};
use crate::mem::Mem;
use glam::Mat4;

pub(crate) fn lod_axes(
    a: &TxVertex,
    b: &TxVertex,
    c: &TxVertex,
    width: u32,
    height: u32,
    unit: usize,
) -> (f32, f32, f32, f32) {
    let (x0, y0) = (a.screen.x, a.screen.y);
    let (x1, y1) = (b.screen.x, b.screen.y);
    let (x2, y2) = (c.screen.x, c.screen.y);
    let det = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
    if det.abs() < 1e-6 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let du_dx = ((b.tex[unit][0] - a.tex[unit][0]) * (y2 - y0)
        - (c.tex[unit][0] - a.tex[unit][0]) * (y1 - y0))
        / det;
    let du_dy = ((c.tex[unit][0] - a.tex[unit][0]) * (x1 - x0)
        - (b.tex[unit][0] - a.tex[unit][0]) * (x2 - x0))
        / det;
    let dv_dx = ((b.tex[unit][1] - a.tex[unit][1]) * (y2 - y0)
        - (c.tex[unit][1] - a.tex[unit][1]) * (y1 - y0))
        / det;
    let dv_dy = ((c.tex[unit][1] - a.tex[unit][1]) * (x1 - x0)
        - (b.tex[unit][1] - a.tex[unit][1]) * (x2 - x0))
        / det;
    let fx = (du_dx.abs() * width as f32).max(dv_dx.abs() * height as f32);
    let fy = (du_dy.abs() * width as f32).max(dv_dy.abs() * height as f32);
    let (major, minor, mx, my) = if fx >= fy {
        (fx, fy, du_dx, dv_dx)
    } else {
        (fy, fx, du_dy, dv_dy)
    };
    let len = (mx * mx + my * my).sqrt().max(1e-6);
    (
        major.max(1.0).log2(),
        minor.max(1.0).log2(),
        mx / len,
        my / len,
    )
}

pub(crate) fn lod_of(
    a: &TxVertex,
    b: &TxVertex,
    c: &TxVertex,
    width: u32,
    height: u32,
    unit: usize,
) -> f32 {
    let (x0, y0) = (a.screen.x, a.screen.y);
    let (x1, y1) = (b.screen.x, b.screen.y);
    let (x2, y2) = (c.screen.x, c.screen.y);
    let det = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
    if det.abs() < 1e-6 {
        return 0.0;
    }
    let du_dx = ((b.tex[unit][0] - a.tex[unit][0]) * (y2 - y0)
        - (c.tex[unit][0] - a.tex[unit][0]) * (y1 - y0))
        / det;
    let du_dy = ((c.tex[unit][0] - a.tex[unit][0]) * (x1 - x0)
        - (b.tex[unit][0] - a.tex[unit][0]) * (x2 - x0))
        / det;
    let dv_dx = ((b.tex[unit][1] - a.tex[unit][1]) * (y2 - y0)
        - (c.tex[unit][1] - a.tex[unit][1]) * (y1 - y0))
        / det;
    let dv_dy = ((c.tex[unit][1] - a.tex[unit][1]) * (x1 - x0)
        - (b.tex[unit][1] - a.tex[unit][1]) * (x2 - x0))
        / det;
    let texel = (du_dx.abs() * width as f32)
        .max(du_dy.abs() * width as f32)
        .max(dv_dx.abs() * height as f32)
        .max(dv_dy.abs() * height as f32);
    texel.max(1.0).log2()
}

pub(crate) fn mix_clip(a: &ClipVertex, b: &ClipVertex, t: f32) -> ClipVertex {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    ClipVertex {
        clip: a.clip + (b.clip - a.clip) * t,
        eye: a.eye + (b.eye - a.eye) * t,
        color: [
            mix(a.color[0], b.color[0]),
            mix(a.color[1], b.color[1]),
            mix(a.color[2], b.color[2]),
            mix(a.color[3], b.color[3]),
        ],
        tex: [
            [mix(a.tex[0][0], b.tex[0][0]), mix(a.tex[0][1], b.tex[0][1])],
            [mix(a.tex[1][0], b.tex[1][0]), mix(a.tex[1][1], b.tex[1][1])],
            [mix(a.tex[2][0], b.tex[2][0]), mix(a.tex[2][1], b.tex[2][1])],
            [mix(a.tex[3][0], b.tex[3][0]), mix(a.tex[3][1], b.tex[3][1])],
        ],
        tex_r: [
            mix(a.tex_r[0], b.tex_r[0]),
            mix(a.tex_r[1], b.tex_r[1]),
            mix(a.tex_r[2], b.tex_r[2]),
            mix(a.tex_r[3], b.tex_r[3]),
        ],
        fog: mix(a.fog, b.fog),
        point_size: mix(a.point_size, b.point_size),
    }
}

pub(crate) fn stencil_pass(func: u32, value: u32, reference: u32) -> bool {
    match func {
        GL_NEVER => false,
        GL_LESS => value < reference,
        GL_EQUAL => value == reference,
        GL_LEQUAL => value <= reference,
        GL_GREATER => value > reference,
        GL_NOTEQUAL => value != reference,
        GL_GEQUAL => value >= reference,
        GL_ALWAYS => true,
        _ => false,
    }
}

pub(crate) fn stencil_op(value: u8, op: u32, reference: u32, mask: u32) -> u8 {
    let next = match op {
        GL_KEEP => value,
        GL_ZERO => 0,
        GL_REPLACE => reference as u8,
        GL_INCR => value.saturating_add(1),
        GL_DECR => value.saturating_sub(1),
        GL_INVERT => !value,
        _ => value,
    };
    (next & mask as u8) | (value & !(mask as u8))
}

pub(crate) fn alpha_pass(func: u32, value: f32, reference: f32) -> bool {
    match func {
        GL_NEVER => false,
        GL_LESS => value < reference,
        GL_EQUAL => (value - reference).abs() < 1e-6,
        GL_LEQUAL => value <= reference,
        GL_GREATER => value > reference,
        GL_NOTEQUAL => (value - reference).abs() >= 1e-6,
        GL_GEQUAL => value >= reference,
        GL_ALWAYS => true,
        _ => true,
    }
}

pub(crate) fn write_values(
    mem: &mut Mem,
    ptr: u32,
    values: &[f32],
    count: u32,
    fixed: bool,
) -> HResult<()> {
    for i in 0..count {
        let v = values[i as usize];
        let word = if fixed {
            ((v * 65536.0) as i32) as u32
        } else {
            v.to_bits()
        };
        mem.write_u32(ptr + i * 4, word)?;
    }
    Ok(())
}

/// One enumerated value written as `GLint`, `GLfloat` or `GLfixed`, matching which of the three
/// Get*Info faces the caller used.
pub(crate) fn write_enum(
    mem: &mut Mem,
    ptr: u32,
    v: u32,
    int_out: bool,
    fixed: bool,
) -> HResult<()> {
    let word = if int_out {
        v
    } else if fixed {
        (v as i32 * 65536) as u32
    } else {
        (v as f32).to_bits()
    };
    mem.write_u32(ptr, word)?;
    Ok(())
}

pub(crate) fn unpack_pixels(
    mem: &Mem,
    ptr: u32,
    width: u32,
    height: u32,
    format: u32,
    ty: u32,
    alignment: u32,
) -> HResult<Vec<u8>> {
    let mut rgba = vec![255u8; (width * height * 4) as usize];
    let step = pixel_bytes(format, ty);
    let align = alignment.max(1);
    let row_bytes = (width * step + align - 1) / align * align;
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            let at = ptr + y * row_bytes + x * step;
            match (format, ty) {
                (GL_RGBA, GL_UNSIGNED_BYTE) => {
                    let p = mem.read_bytes(at, 4)?;
                    rgba[i..i + 4].copy_from_slice(&p);
                }
                (GL_RGB, GL_UNSIGNED_BYTE) => {
                    let p = mem.read_bytes(at, 3)?;
                    rgba[i..i + 3].copy_from_slice(&p);
                }
                (GL_LUMINANCE, GL_UNSIGNED_BYTE) => {
                    let l = mem.read_u8(at)?;
                    rgba[i] = l;
                    rgba[i + 1] = l;
                    rgba[i + 2] = l;
                }
                (GL_ALPHA, GL_UNSIGNED_BYTE) => {
                    let a = mem.read_u8(at)?;
                    rgba[i] = 0;
                    rgba[i + 1] = 0;
                    rgba[i + 2] = 0;
                    rgba[i + 3] = a;
                }
                (GL_LUMINANCE_ALPHA, GL_UNSIGNED_BYTE) => {
                    let p = mem.read_bytes(at, 2)?;
                    rgba[i] = p[0];
                    rgba[i + 1] = p[0];
                    rgba[i + 2] = p[0];
                    rgba[i + 3] = p[1];
                }
                (GL_RGB, GL_UNSIGNED_SHORT_5_6_5) => {
                    let (r, g, b) = crate::display::unpack565(mem.read_u16(at)? as u16);
                    rgba[i] = r;
                    rgba[i + 1] = g;
                    rgba[i + 2] = b;
                }
                (GL_RGBA, GL_UNSIGNED_SHORT_4_4_4_4) => {
                    let v = mem.read_u16(at)? as u32;
                    rgba[i] = ((v >> 12) & 0xF) as u8 * 17;
                    rgba[i + 1] = ((v >> 8) & 0xF) as u8 * 17;
                    rgba[i + 2] = ((v >> 4) & 0xF) as u8 * 17;
                    rgba[i + 3] = (v & 0xF) as u8 * 17;
                }
                (GL_RGBA, GL_UNSIGNED_SHORT_5_5_5_1) => {
                    let v = mem.read_u16(at)? as u32;
                    rgba[i] = (((v >> 11) & 0x1F) * 255 / 31) as u8;
                    rgba[i + 1] = (((v >> 6) & 0x1F) * 255 / 31) as u8;
                    rgba[i + 2] = (((v >> 1) & 0x1F) * 255 / 31) as u8;
                    rgba[i + 3] = if v & 1 != 0 { 255 } else { 0 };
                }
                _ => {}
            }
        }
    }
    Ok(rgba)
}

/// `GL_OES_compressed_paletted_texture`: `u16` mipmap count, then a 16- or 256-entry palette
/// (3 or 4 bytes each), then 4- or 8-bit indices. Only level 0 is decoded here.
pub(crate) fn decode_paletted(
    bytes: &[u8],
    width: u32,
    height: u32,
    format: u32,
) -> Option<Vec<u8>> {
    let (bits, entry, has_alpha, packed) = match format {
        GL_PALETTE4_RGB8_OES => (4usize, 3usize, false, false),
        GL_PALETTE4_RGBA8_OES => (4usize, 4usize, true, false),
        GL_PALETTE4_R5_G6_B5_OES => (4usize, 2usize, false, true),
        GL_PALETTE4_RGBA4_OES => (4usize, 2usize, true, true),
        GL_PALETTE4_RGB5_A1_OES => (4usize, 2usize, true, true),
        GL_PALETTE8_RGB8_OES => (8usize, 3usize, false, false),
        GL_PALETTE8_RGBA8_OES => (8usize, 4usize, true, false),
        GL_PALETTE8_R5_G6_B5_OES => (8usize, 2usize, false, true),
        GL_PALETTE8_RGBA4_OES => (8usize, 2usize, true, true),
        GL_PALETTE8_RGB5_A1_OES => (8usize, 2usize, true, true),
        _ => return None,
    };
    if bytes.len() < 2 {
        return None;
    }
    let palette_len = (1usize << bits) * entry;
    let mut offset = 2;
    if bytes.len() < offset + palette_len {
        return None;
    }
    let palette = &bytes[offset..offset + palette_len];
    offset += palette_len;
    let stride = if bits == 4 {
        (width as usize + 1) / 2
    } else {
        width as usize
    };
    if bytes.len() < offset + stride * height as usize {
        return None;
    }
    let mut rgba = vec![255u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let index = if bits == 4 {
                let b = bytes[offset + y * stride + x / 2];
                if x % 2 == 0 {
                    (b >> 4) as usize
                } else {
                    (b & 0x0F) as usize
                }
            } else {
                bytes[offset + y * stride + x] as usize
            };
            let p = &palette[index * entry..index * entry + entry];
            let i = (y * width as usize + x) * 4;
            let (r, g, b, a) = if packed {
                let v = u16::from_le_bytes([p[0], p[1]]);
                match format {
                    GL_PALETTE4_R5_G6_B5_OES | GL_PALETTE8_R5_G6_B5_OES => {
                        let (r, g, b) = crate::display::unpack565(v);
                        (r, g, b, 255)
                    }
                    GL_PALETTE4_RGBA4_OES | GL_PALETTE8_RGBA4_OES => (
                        ((v >> 12) & 0xF) as u8 * 17,
                        ((v >> 8) & 0xF) as u8 * 17,
                        ((v >> 4) & 0xF) as u8 * 17,
                        (v & 0xF) as u8 * 17,
                    ),
                    _ => (
                        (((v >> 11) & 0x1F) as u32 * 255 / 31) as u8,
                        (((v >> 6) & 0x1F) as u32 * 255 / 31) as u8,
                        (((v >> 1) & 0x1F) as u32 * 255 / 31) as u8,
                        if v & 1 != 0 { 255 } else { 0 },
                    ),
                }
            } else {
                (p[0], p[1], p[2], if has_alpha { p[3] } else { 255 })
            };
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = a;
        }
    }
    Some(rgba)
}

pub(crate) fn pixel_bytes(format: u32, ty: u32) -> u32 {
    match (format, ty) {
        (_, GL_UNSIGNED_SHORT_5_6_5)
        | (_, GL_UNSIGNED_SHORT_4_4_4_4)
        | (_, GL_UNSIGNED_SHORT_5_5_5_1) => 2,
        (GL_RGBA, _) => 4,
        (GL_RGB, _) => 3,
        (GL_LUMINANCE_ALPHA, _) => 2,
        (GL_LUMINANCE, _) | (GL_ALPHA, _) => 1,
        _ => 4,
    }
}

/// Copy the overlapping rectangle of two depth planes, which are kept at their own sizes
/// because a framebuffer's attachments may disagree until `CheckFramebufferStatus` says so.
pub(crate) fn copy_depth(src: &[f32], sw: u32, sh: u32, dst: &mut [f32], dw: u32, dh: u32) {
    let w = sw.min(dw) as usize;
    let h = sh.min(dh) as usize;
    if w == 0 || h == 0 {
        return;
    }
    for y in 0..h {
        let src_off = y * sw as usize;
        let dst_off = y * dw as usize;
        let n = w
            .min(dst.len().saturating_sub(dst_off))
            .min(src.len().saturating_sub(src_off));
        if n == 0 {
            break;
        }
        dst[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
    }
}

pub(crate) fn copy_stencil(src: &[u8], sw: u32, sh: u32, dst: &mut [u8], dw: u32, dh: u32) {
    let w = sw.min(dw) as usize;
    let h = sh.min(dh) as usize;
    if w == 0 || h == 0 {
        return;
    }
    for y in 0..h {
        let src_off = y * sw as usize;
        let dst_off = y * dw as usize;
        let n = w
            .min(dst.len().saturating_sub(dst_off))
            .min(src.len().saturating_sub(src_off));
        if n == 0 {
            break;
        }
        dst[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
    }
}

pub(crate) fn is_primitive(mode: u32) -> bool {
    mode <= GL_TRIANGLE_FAN
}

pub(crate) fn is_filter(value: u32) -> bool {
    matches!(
        value,
        GL_NEAREST
            | GL_LINEAR
            | GL_NEAREST_MIPMAP_NEAREST
            | GL_LINEAR_MIPMAP_NEAREST
            | GL_NEAREST_MIPMAP_LINEAR
            | GL_LINEAR_MIPMAP_LINEAR
    )
}

pub(crate) fn is_wrap(value: u32) -> bool {
    matches!(value, GL_REPEAT | GL_CLAMP_TO_EDGE | GL_MIRRORED_REPEAT_OES)
}

pub(crate) fn is_blend_factor(value: u32) -> bool {
    matches!(
        value,
        GL_ZERO
            | GL_ONE
            | GL_SRC_COLOR
            | GL_ONE_MINUS_SRC_COLOR
            | GL_DST_COLOR
            | GL_ONE_MINUS_DST_COLOR
            | GL_SRC_ALPHA
            | GL_ONE_MINUS_SRC_ALPHA
            | GL_DST_ALPHA
            | GL_ONE_MINUS_DST_ALPHA
            | GL_SRC_ALPHA_SATURATE
    )
}

pub(crate) fn is_compare_func(value: u32) -> bool {
    (GL_NEVER..=GL_ALWAYS).contains(&value)
}

pub(crate) fn is_stencil_op(value: u32) -> bool {
    matches!(
        value,
        GL_KEEP | GL_ZERO | GL_REPLACE | GL_INCR | GL_DECR | GL_INVERT
    )
}

/// The `(format, type)` pairs `unpack_pixels` can turn into RGBA bytes.
pub(crate) fn is_pixel_format(format: u32, ty: u32) -> bool {
    matches!(
        (format, ty),
        (GL_RGBA, GL_UNSIGNED_BYTE)
            | (GL_RGB, GL_UNSIGNED_BYTE)
            | (GL_LUMINANCE, GL_UNSIGNED_BYTE)
            | (GL_ALPHA, GL_UNSIGNED_BYTE)
            | (GL_LUMINANCE_ALPHA, GL_UNSIGNED_BYTE)
            | (GL_RGB, GL_UNSIGNED_SHORT_5_6_5)
            | (GL_RGBA, GL_UNSIGNED_SHORT_4_4_4_4)
            | (GL_RGBA, GL_UNSIGNED_SHORT_5_5_5_1)
    )
}

pub(crate) fn is_array_type(ty: u32) -> bool {
    matches!(
        ty,
        GL_BYTE | GL_UNSIGNED_BYTE | GL_SHORT | GL_UNSIGNED_SHORT | GL_FIXED | GL_FLOAT
    )
}

/// Bytes per component for an array's `type`. Both an absent stride and the offset between
/// consecutive components are measured in this unit.
pub(crate) fn type_size(ty: u32) -> u32 {
    match ty {
        GL_BYTE | GL_UNSIGNED_BYTE => 1,
        GL_SHORT | GL_UNSIGNED_SHORT => 2,
        _ => 4,
    }
}

/// A position component, converted without normalisation: an integer position array names
/// coordinates in its own range, which is what `glVertexPointer` specifies.
pub(crate) fn read_component(mem: &Mem, addr: u32, ty: u32) -> HResult<f32> {
    Ok(match ty {
        GL_FLOAT => f32::from_bits(mem.read_u32(addr)?),
        GL_FIXED => (mem.read_u32(addr)? as i32) as f32 / 65536.0,
        GL_BYTE => (mem.read_u8(addr)? as i8) as f32,
        GL_UNSIGNED_BYTE => mem.read_u8(addr)? as f32,
        GL_SHORT => (mem.read_u16(addr)? as i16) as f32,
        GL_UNSIGNED_SHORT => mem.read_u16(addr)? as f32,
        GL_INT => mem.read_u32(addr)? as i32 as f32,
        _ => 0.0,
    })
}

/// A colour, normal or texture-coordinate component. Those arrays are normalised to [0, 1]
/// or [-1, 1] according to their type; reading them raw turns every byte/short normal into
/// a value the lighting equation clamps away.
pub(crate) fn read_normalized_component(mem: &Mem, addr: u32, ty: u32) -> HResult<f32> {
    Ok(match ty {
        GL_FLOAT => f32::from_bits(mem.read_u32(addr)?),
        GL_FIXED => (mem.read_u32(addr)? as i32) as f32 / 65536.0,
        GL_BYTE => ((mem.read_u8(addr)? as i8) as f32 / 127.0).clamp(-1.0, 1.0),
        GL_UNSIGNED_BYTE => mem.read_u8(addr)? as f32 / 255.0,
        GL_SHORT => ((mem.read_u16(addr)? as i16) as f32 / 32767.0).clamp(-1.0, 1.0),
        GL_UNSIGNED_SHORT => mem.read_u16(addr)? as f32 / 65535.0,
        GL_INT => ((mem.read_u32(addr)? as i32) as f32 / 2147483647.0).clamp(-1.0, 1.0),
        _ => 0.0,
    })
}

pub(crate) fn frustum(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let rl = right - left;
    let tb = top - bottom;
    let fn_ = far - near;
    Mat4::from_cols_array(&[
        2.0 * near / rl,
        0.0,
        0.0,
        0.0,
        0.0,
        2.0 * near / tb,
        0.0,
        0.0,
        (right + left) / rl,
        (top + bottom) / tb,
        -(far + near) / fn_,
        -1.0,
        0.0,
        0.0,
        -2.0 * far * near / fn_,
        0.0,
    ])
}

pub(crate) fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let rl = right - left;
    let tb = top - bottom;
    let fn_ = far - near;
    Mat4::from_cols_array(&[
        2.0 / rl,
        0.0,
        0.0,
        0.0,
        0.0,
        2.0 / tb,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / fn_,
        0.0,
        -(right + left) / rl,
        -(top + bottom) / tb,
        -(far + near) / fn_,
        1.0,
    ])
}
