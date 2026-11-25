//! Implementation blocks for the software pipeline's supporting types.

use super::*;

impl Texture {
    pub(super) fn white() -> Self {
        Texture {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
            uploaded: false,
            internal: GL_RGBA,
            levels: Vec::new(),
            depth: Vec::new(),
            stencil: Vec::new(),
            faces: Vec::new(),
            cube_levels: Vec::new(),
            cube_depth: Vec::new(),
            cube_stencil: Vec::new(),
            mag: GL_LINEAR,
            min: GL_NEAREST_MIPMAP_LINEAR,
            wrap_s: GL_REPEAT,
            wrap_t: GL_REPEAT,
            max_anisotropy: 1.0,
        }
    }

    /// Rebuild the mip chain from level zero with a 2x2 box filter. Odd sizes clamp the
    /// second sample to the last row/column, which keeps the chain complete down to 1x1.
    pub(super) fn generate_mipmaps(&mut self) {
        let mut w = self.width;
        let mut h = self.height;
        let mut src = self.rgba.clone();
        self.levels.clear();
        while w > 1 || h > 1 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let mut dst = vec![0u8; (nw * nh * 4) as usize];
            for y in 0..nh {
                for x in 0..nw {
                    let x0 = (x * 2).min(w - 1);
                    let x1 = (x * 2 + 1).min(w - 1);
                    let y0 = (y * 2).min(h - 1);
                    let y1 = (y * 2 + 1).min(h - 1);
                    let o = ((y * nw + x) * 4) as usize;
                    let mut sum = [0u32; 4];
                    for (sx, sy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                        let i = ((sy * w + sx) * 4) as usize;
                        for c in 0..4 {
                            sum[c] += src[i + c] as u32;
                        }
                    }
                    for c in 0..4 {
                        dst[o + c] = (sum[c] / 4) as u8;
                    }
                }
            }
            self.levels.push(dst.clone());
            src = dst;
            w = nw;
            h = nh;
        }
    }

    /// Build the six-face mip chain from level zero with the same 2x2 box filter used for 2D
    /// textures.
    pub(super) fn generate_cube_mipmaps(&mut self) {
        if self.faces.len() < 6 {
            return;
        }
        self.cube_levels.clear();
        let mut w = self.width;
        let mut h = self.height;
        let mut src = self.faces.clone();
        while w > 1 || h > 1 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let mut next = Vec::with_capacity(6);
            for face in 0..6 {
                let mut dst = vec![0u8; (nw * nh * 4) as usize];
                for y in 0..nh {
                    for x in 0..nw {
                        let x0 = (x * 2).min(w - 1);
                        let x1 = (x * 2 + 1).min(w - 1);
                        let y0 = (y * 2).min(h - 1);
                        let y1 = (y * 2 + 1).min(h - 1);
                        let o = ((y * nw + x) * 4) as usize;
                        let mut sum = [0u32; 4];
                        for (sx, sy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                            let i = ((sy * w + sx) * 4) as usize;
                            for c in 0..4 {
                                sum[c] += src[face][i + c] as u32;
                            }
                        }
                        for c in 0..4 {
                            dst[o + c] = (sum[c] / 4) as u8;
                        }
                    }
                }
                next.push(dst);
            }
            self.cube_levels.push(next.clone());
            src = next;
            w = nw;
            h = nh;
        }
    }

    /// Anisotropic sampling along the minor axis. The caller passes the two LOD axes and the
    /// normalized minor-axis direction in texture space.
    pub(super) fn sample_aniso(
        &self,
        u: f32,
        v: f32,
        major: f32,
        minor: f32,
        du: f32,
        dv: f32,
    ) -> [f32; 4] {
        let max_ratio = self.max_anisotropy.max(1.0);
        let ratio = (major - minor).clamp(0.0, max_ratio.log2());
        if ratio <= 0.01 || max_ratio <= 1.0 {
            return self.sample(u, v, major);
        }
        let taps = ratio.exp2().ceil().min(max_ratio).max(1.0) as i32;
        if taps <= 1 {
            return self.sample(u, v, major);
        }
        let span = (2.0f32.powf(minor) / self.width.max(self.height).max(1) as f32).max(1e-5);
        let mut out = [0.0f32; 4];
        for i in 0..taps {
            let t = (i as f32 + 0.5) / taps as f32 - 0.5;
            let sample = self.sample(u + du * t * span, v + dv * t * span, major);
            for ch in 0..4 {
                out[ch] += sample[ch];
            }
        }
        for ch in out.iter_mut() {
            *ch /= taps as f32;
        }
        out
    }

    /// The pixel size of one mip level, or `None` when that level was never uploaded.
    pub(super) fn level_size(&self, level: u32) -> Option<(u32, u32)> {
        if level == 0 {
            Some((self.width, self.height))
        } else if !self.faces.is_empty()
            || !self.cube_levels.is_empty()
            || !self.cube_depth.is_empty()
            || !self.cube_stencil.is_empty()
        {
            let cube_levels = if !self.cube_depth.is_empty() {
                Some(self.cube_depth.len())
            } else if !self.cube_stencil.is_empty() {
                Some(self.cube_stencil.len())
            } else {
                Some(self.cube_levels.len() + 1)
            };
            cube_levels
                .filter(|&n| level as usize <= n)
                .map(|_| ((self.width >> level).max(1), (self.height >> level).max(1)))
        } else {
            self.levels
                .get(level as usize - 1)
                .map(|_| ((self.width >> level).max(1), (self.height >> level).max(1)))
        }
    }

    /// The RGBA bytes of one mip level. Level zero lives in `rgba`; the rest are the chain
    /// `TexImage2D` filled.
    pub(super) fn level_rgba(&self, level: u32) -> Option<Vec<u8>> {
        if level == 0 {
            Some(self.rgba.clone())
        } else {
            self.levels.get(level as usize - 1).cloned()
        }
    }

    /// RGBA bytes for a 2D texture or one cube-map face at `level`.
    pub(super) fn face_rgba(&self, target: u32, level: u32) -> Option<Vec<u8>> {
        if let Some(face) = cube_face(target) {
            if level == 0 {
                self.faces.get(face).cloned()
            } else {
                self.cube_levels
                    .get(level as usize - 1)
                    .and_then(|faces| faces.get(face))
                    .cloned()
            }
        } else {
            self.level_rgba(level)
        }
    }

    pub(super) fn level_depth(&self, level: u32) -> Option<&[f32]> {
        self.depth.get(level as usize).map(|v| v.as_slice())
    }

    pub(super) fn level_stencil(&self, level: u32) -> Option<&[u8]> {
        self.stencil.get(level as usize).map(|v| v.as_slice())
    }

    pub(super) fn level_depth_mut(&mut self, level: u32) -> Option<&mut Vec<f32>> {
        self.depth.get_mut(level as usize)
    }

    pub(super) fn level_stencil_mut(&mut self, level: u32) -> Option<&mut Vec<u8>> {
        self.stencil.get_mut(level as usize)
    }

    pub(super) fn face_depth(&self, target: u32, level: u32) -> Option<&[f32]> {
        if let Some(face) = cube_face(target) {
            self.cube_depth
                .get(level as usize)
                .and_then(|faces| faces.get(face))
                .map(|v| v.as_slice())
        } else {
            self.level_depth(level)
        }
    }

    pub(super) fn face_stencil(&self, target: u32, level: u32) -> Option<&[u8]> {
        if let Some(face) = cube_face(target) {
            self.cube_stencil
                .get(level as usize)
                .and_then(|faces| faces.get(face))
                .map(|v| v.as_slice())
        } else {
            self.level_stencil(level)
        }
    }

    pub(super) fn face_depth_mut(&mut self, target: u32, level: u32) -> Option<&mut Vec<f32>> {
        if let Some(face) = cube_face(target) {
            self.cube_depth
                .get_mut(level as usize)
                .and_then(|faces| faces.get_mut(face))
        } else {
            self.level_depth_mut(level)
        }
    }

    pub(super) fn face_stencil_mut(&mut self, target: u32, level: u32) -> Option<&mut Vec<u8>> {
        if let Some(face) = cube_face(target) {
            self.cube_stencil
                .get_mut(level as usize)
                .and_then(|faces| faces.get_mut(face))
        } else {
            self.level_stencil_mut(level)
        }
    }

    /// Writable storage for a 2D level or one cube-map face at `level`.
    pub(super) fn face_storage(
        &mut self,
        target: u32,
        level: u32,
    ) -> Option<(u32, u32, &mut Vec<u8>)> {
        if let Some(face) = cube_face(target) {
            let w = (self.width >> level).max(1);
            let h = (self.height >> level).max(1);
            let data = if level == 0 {
                self.faces.get_mut(face)?
            } else {
                self.cube_levels
                    .get_mut(level as usize - 1)?
                    .get_mut(face)?
            };
            Some((w, h, data))
        } else {
            self.level_storage(level)
        }
    }

    /// The writable storage of one mip level: its size and the RGBA bytes to update.
    pub(super) fn level_storage(&mut self, level: u32) -> Option<(u32, u32, &mut Vec<u8>)> {
        if level > 16 {
            return None;
        }
        let (width, height) = if level == 0 {
            (self.width, self.height)
        } else {
            ((self.width >> level).max(1), (self.height >> level).max(1))
        };
        let data = if level == 0 {
            &mut self.rgba
        } else {
            self.levels.get_mut(level as usize - 1)?
        };
        Some((width, height, data))
    }

    pub(super) fn sample(&self, u: f32, v: f32, lod: f32) -> [f32; 4] {
        let mipmap = matches!(
            self.min,
            GL_NEAREST_MIPMAP_NEAREST
                | GL_LINEAR_MIPMAP_NEAREST
                | GL_NEAREST_MIPMAP_LINEAR
                | GL_LINEAR_MIPMAP_LINEAR
        );
        if !mipmap {
            return self.sample_level(0, u, v, self.mag == GL_LINEAR);
        }
        let max_level = self.levels.len() as f32;
        let lod = lod.clamp(0.0, max_level);
        let l0 = lod.floor() as usize;
        let frac = lod - l0 as f32;
        let linear = matches!(self.min, GL_LINEAR_MIPMAP_NEAREST | GL_LINEAR_MIPMAP_LINEAR);
        let a = self.sample_level(l0, u, v, linear);
        if frac <= 0.0 || l0 >= self.levels.len() {
            return a;
        }
        let b = self.sample_level(l0 + 1, u, v, linear);
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = a[i] * (1.0 - frac) + b[i] * frac;
        }
        out
    }

    pub(super) fn sample_level(&self, level: usize, u: f32, v: f32, linear: bool) -> [f32; 4] {
        let (w, h, data) = if level == 0 {
            (self.width, self.height, &self.rgba)
        } else {
            match self.levels.get(level - 1) {
                Some(d) => (
                    (self.width >> level).max(1),
                    (self.height >> level).max(1),
                    d,
                ),
                None => return [1.0; 4],
            }
        };
        let wrap = |x: f32, mode: u32| -> f32 {
            if mode == GL_CLAMP_TO_EDGE {
                x.clamp(0.0, 1.0)
            } else if mode == GL_MIRRORED_REPEAT_OES {
                let f = x - x.floor();
                if (x.floor() as i64) & 1 == 0 {
                    f
                } else {
                    1.0 - f
                }
            } else {
                x - x.floor()
            }
        };
        let u = wrap(u, self.wrap_s);
        let v = wrap(v, self.wrap_t);
        let x = u * w as f32 - 0.5;
        let y = v * h as f32 - 0.5;
        let fetch = |x: i32, y: i32| -> [f32; 4] {
            let x = x.clamp(0, w as i32 - 1) as usize;
            let y = y.clamp(0, h as i32 - 1) as usize;
            let i = (y * w as usize + x) * 4;
            [
                data[i] as f32 / 255.0,
                data[i + 1] as f32 / 255.0,
                data[i + 2] as f32 / 255.0,
                data[i + 3] as f32 / 255.0,
            ]
        };
        if !linear {
            return fetch(x.round() as i32, y.round() as i32);
        }
        let (x0, y0) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let a = fetch(x0, y0);
        let b = fetch(x0 + 1, y0);
        let c = fetch(x0, y0 + 1);
        let d = fetch(x0 + 1, y0 + 1);
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = a[i] * (1.0 - fx) + b[i] * fx;
            let bottom = c[i] * (1.0 - fx) + d[i] * fx;
            out[i] = top * (1.0 - fy) + bottom * fy;
        }
        out
    }

    /// Sample one cube-map face at `lod`, interpolating between adjacent levels the same way
    /// `sample` does for a 2D texture. Face selection uses the major-axis rules.
    pub(super) fn sample_cube(&self, s: f32, t: f32, r: f32, lod: f32, linear: bool) -> [f32; 4] {
        let max_level = self.cube_levels.len() as f32;
        let lod = lod.clamp(0.0, max_level);
        let l0 = lod.floor() as usize;
        let frac = lod - l0 as f32;
        let a = self.sample_cube_level(l0, s, t, r, linear);
        if frac <= 0.0 || l0 >= self.cube_levels.len() {
            return a;
        }
        let b = self.sample_cube_level(l0 + 1, s, t, r, linear);
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = a[i] * (1.0 - frac) + b[i] * frac;
        }
        out
    }

    fn sample_cube_level(&self, level: usize, s: f32, t: f32, r: f32, linear: bool) -> [f32; 4] {
        let (face, u, v) = cube_uv(s, t, r);
        let (w, h, data) = if level == 0 {
            (self.width, self.height, self.faces.get(face))
        } else {
            (
                (self.width >> level).max(1),
                (self.height >> level).max(1),
                self.cube_levels
                    .get(level - 1)
                    .and_then(|faces| faces.get(face)),
            )
        };
        let Some(data) = data else {
            return [1.0; 4];
        };
        if data.len() < (w * h * 4) as usize {
            return [1.0; 4];
        }
        sample_rgba(data, w, h, u, v, linear, self.wrap_s, self.wrap_t)
    }
}

fn cube_uv(s: f32, t: f32, r: f32) -> (usize, f32, f32) {
    let (ax, ay, az) = (s.abs(), t.abs(), r.abs());
    let (face, sc, tc, major) = if ax >= ay && ax >= az {
        if s > 0.0 {
            (0, -r, -t, ax)
        } else {
            (1, r, -t, ax)
        }
    } else if ay >= az {
        if t > 0.0 {
            (2, s, r, ay)
        } else {
            (3, s, -r, ay)
        }
    } else if r > 0.0 {
        (4, s, -t, az)
    } else {
        (5, -s, -t, az)
    };
    let major = major.max(1e-9);
    (face, 0.5 * (sc / major + 1.0), 0.5 * (tc / major + 1.0))
}

fn sample_rgba(
    data: &[u8],
    w: u32,
    h: u32,
    u: f32,
    v: f32,
    linear: bool,
    wrap_s: u32,
    wrap_t: u32,
) -> [f32; 4] {
    let wrap = |x: f32, mode: u32| -> f32 {
        if mode == GL_CLAMP_TO_EDGE {
            x.clamp(0.0, 1.0)
        } else if mode == GL_MIRRORED_REPEAT_OES {
            let f = x - x.floor();
            if (x.floor() as i64) & 1 == 0 {
                f
            } else {
                1.0 - f
            }
        } else {
            x - x.floor()
        }
    };
    let u = wrap(u, wrap_s);
    let v = wrap(v, wrap_t);
    let x = u * w as f32 - 0.5;
    let y = v * h as f32 - 0.5;
    let fetch = |x: i32, y: i32| -> [f32; 4] {
        let x = x.clamp(0, w as i32 - 1) as usize;
        let y = y.clamp(0, h as i32 - 1) as usize;
        let i = (y * w as usize + x) * 4;
        [
            data[i] as f32 / 255.0,
            data[i + 1] as f32 / 255.0,
            data[i + 2] as f32 / 255.0,
            data[i + 3] as f32 / 255.0,
        ]
    };
    if !linear {
        return fetch(x.round() as i32, y.round() as i32);
    }
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let a = fetch(x0, y0);
    let b = fetch(x0 + 1, y0);
    let c = fetch(x0, y0 + 1);
    let d = fetch(x0 + 1, y0 + 1);
    let mut out = [0.0; 4];
    for i in 0..4 {
        let top = a[i] * (1.0 - fx) + b[i] * fx;
        let bottom = c[i] * (1.0 - fx) + d[i] * fx;
        out[i] = top * (1.0 - fy) + bottom * fy;
    }
    out
}
