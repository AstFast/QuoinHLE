//! Software rasteriser, projection, texture combine and state helpers.

use super::helpers::*;
use super::*;

impl Gles {
    /// Record the first error a call raises. GL reports the earliest unread one, so a later
    /// call must not overwrite a value the guest has not read yet.
    pub(super) fn set_error(&mut self, code: u32) {
        if self.error == GL_NO_ERROR {
            self.error = code;
        }
    }

    /// The texture unit an `ActiveTexture`/`ClientActiveTexture` enum names, or `None` when the
    /// profile's `GL_MAX_TEXTURE_UNITS` does not reach it. The software pipeline stores four.
    pub(super) fn texture_unit(&self, texture: u32) -> Option<usize> {
        let units = self.texture_units();
        texture
            .checked_sub(GL_TEXTURE0)
            .map(|n| n as usize)
            .filter(|n| *n < units)
    }

    /// How many texture units this profile has, capped by the four the pipeline stores.
    pub(super) fn texture_units(&self) -> usize {
        (self.caps.max_texture_units as usize).clamp(1, 4)
    }

    /// Reject a count that would make the host allocate or loop without bound. The guest's
    /// `GLsizei` arrives as an unsigned word, so a negative count is a very large one.
    pub(super) fn check_count(&mut self, count: u32, max: u32) -> bool {
        if count > max {
            self.set_error(GL_INVALID_VALUE);
            return false;
        }
        true
    }

    /// Apply one `glTexParameter*` value. The scalar and vector entry points differ only in
    /// how they read the value; the state they write is the same four enums.
    pub(super) fn set_tex_parameter(&mut self, pname: u32, value: u32) {
        let valid = match pname {
            // The anisotropic extension makes a ratio outside the supported range a value error,
            // not a name the enum does not contain.
            GL_TEXTURE_MAX_ANISOTROPY_EXT => {
                if value < 1 || value > self.caps.max_texture_max_anisotropy {
                    self.set_error(GL_INVALID_VALUE);
                    return;
                }
                true
            }
            GL_TEXTURE_MAG_FILTER | GL_TEXTURE_MIN_FILTER => is_filter(value),
            GL_TEXTURE_WRAP_S | GL_TEXTURE_WRAP_T => is_wrap(value),
            _ => false,
        };
        if !valid {
            self.set_error(GL_INVALID_ENUM);
            return;
        }
        let unit = self.active_texture;
        if let Some(t) = self.textures.get_mut(&self.bound_texture[unit]) {
            match pname {
                GL_TEXTURE_MAG_FILTER => t.mag = value,
                GL_TEXTURE_MIN_FILTER => t.min = value,
                GL_TEXTURE_WRAP_S => t.wrap_s = value,
                GL_TEXTURE_WRAP_T => t.wrap_t = value,
                GL_TEXTURE_MAX_ANISOTROPY_EXT => t.max_anisotropy = value as f32,
                _ => {}
            }
        }
    }

    /// One `IGLES10`/`IGLES11` slot. `egl` is the surface state the draw calls land in.
    pub(super) fn clear_client(&mut self, cap: u32) {
        match cap {
            GL_VERTEX_ARRAY => self.client.vertex = None,
            GL_COLOR_ARRAY => self.client.color = None,
            GL_NORMAL_ARRAY => self.client.normal = None,
            GL_TEXTURE_COORD_ARRAY => self.client_tex[self.client_active_texture] = None,
            GL_POINT_SIZE_ARRAY_OES => self.client.point_size = None,
            GL_MATRIX_INDEX_ARRAY_OES => self.client.matrix_index = None,
            GL_WEIGHT_ARRAY_OES => self.client.weight = None,
            _ => {}
        }
    }

    pub(super) fn read_matrix(&self, ptr: u32, fixed: bool, mem: &Mem) -> HResult<Mat4> {
        let mut m = [0.0f32; 16];
        for (i, v) in m.iter_mut().enumerate() {
            let w = mem.read_u32(ptr + i as u32 * 4)?;
            *v = if fixed {
                (w as i32) as f32 / 65536.0
            } else {
                f32::from_bits(w)
            };
        }
        Ok(Mat4::from_cols_array(&m))
    }

    pub(super) fn push_vertex(&mut self, pos: Vec3, w: f32) {
        if self.vertices.len() >= 1 << 16 {
            return;
        }
        self.vertices.push(Vertex {
            pos,
            w,
            color: self.current.color,
            tex: self.current.tex,
            tex_r: self.current.tex_r,
            tex_q: self.current.tex_q,
            normal: self.current.normal,
            point_size: self.point_size,
            matrix_indices: self.current.matrix_indices,
            weights: self.current.weights,
            matrix_count: self.current.matrix_count,
        });
    }

    pub(super) fn read_arrays(&self, first: u32, count: u32, mem: &Mem) -> HResult<Vec<Vertex>> {
        let Some(v) = self.client.vertex else {
            return Ok(Vec::new());
        };
        // When an array buffer is bound, the pointer slots name offsets into it.
        let vbase = self.bound_buffer[0];
        // A component is as wide as its type says, and an absent stride names tightly packed
        // components — not the four-byte slots a `GLfloat` array happens to use. The two are
        // the same only for `GL_FLOAT`/`GL_FIXED`; byte and short arrays are the ones this
        // distinction keeps from walking off into the next component.
        let vstep = type_size(v.ty);
        let vstride = if v.stride > 0 {
            v.stride as u32
        } else {
            v.size.max(1) as u32 * vstep
        };
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let base = vbase
                .saturating_add(v.ptr)
                .saturating_add(first.saturating_add(i).saturating_mul(vstride));
            let mut pos = Vec3::ZERO;
            let mut w = 1.0;
            match v.size.clamp(2, 4) as u32 {
                2 => {
                    pos.x = read_component(mem, base, v.ty)?;
                    pos.y = read_component(mem, base + vstep, v.ty)?;
                }
                3 => {
                    pos.x = read_component(mem, base, v.ty)?;
                    pos.y = read_component(mem, base + vstep, v.ty)?;
                    pos.z = read_component(mem, base + 2 * vstep, v.ty)?;
                }
                _ => {
                    pos.x = read_component(mem, base, v.ty)?;
                    pos.y = read_component(mem, base + vstep, v.ty)?;
                    pos.z = read_component(mem, base + 2 * vstep, v.ty)?;
                    w = read_component(mem, base + 3 * vstep, v.ty)?;
                }
            }
            let color = match self.client.color {
                Some(c) => {
                    let step = type_size(c.ty);
                    let s = if c.stride > 0 {
                        c.stride as u32
                    } else {
                        c.size.max(1) as u32 * step
                    };
                    let b = vbase
                        .saturating_add(c.ptr)
                        .saturating_add(first.saturating_add(i).saturating_mul(s));
                    // The components an omitted size leaves out default to zero, alpha to one.
                    let mut col = [0.0, 0.0, 0.0, 1.0];
                    for k in 0..c.size.clamp(1, 4) as u32 {
                        col[k as usize] = read_normalized_component(mem, b + k * step, c.ty)?;
                    }
                    col
                }
                None => self.current.color,
            };
            let mut tex = self.current.tex;
            let mut tex_r = self.current.tex_r;
            let mut tex_q = self.current.tex_q;
            for unit in 0..self.texture_units() {
                if let Some(t) = self.client_tex[unit] {
                    let step = type_size(t.ty);
                    let s = if t.stride > 0 {
                        t.stride as u32
                    } else {
                        t.size.max(1) as u32 * step
                    };
                    let b = vbase
                        .saturating_add(t.ptr)
                        .saturating_add(first.saturating_add(i).saturating_mul(s));
                    let n = t.size.clamp(1, 4) as u32;
                    let mut coord = [0.0; 2];
                    if n > 0 {
                        coord[0] = read_normalized_component(mem, b, t.ty)?;
                    }
                    if n > 1 {
                        coord[1] = read_normalized_component(mem, b + step, t.ty)?;
                    }
                    if n > 2 {
                        tex_r[unit] = read_normalized_component(mem, b + 2 * step, t.ty)?;
                    }
                    if n > 3 {
                        tex_q[unit] = read_normalized_component(mem, b + 3 * step, t.ty)?;
                    }
                    tex[unit] = coord;
                }
            }
            let normal = match self.client.normal {
                Some(n) => {
                    let step = type_size(n.ty);
                    let s = if n.stride > 0 {
                        n.stride as u32
                    } else {
                        3 * step
                    };
                    let b = vbase
                        .saturating_add(n.ptr)
                        .saturating_add(first.saturating_add(i).saturating_mul(s));
                    Vec3::new(
                        read_normalized_component(mem, b, n.ty)?,
                        read_normalized_component(mem, b + step, n.ty)?,
                        read_normalized_component(mem, b + 2 * step, n.ty)?,
                    )
                }
                None => self.current.normal,
            };
            let point_size = match self.client.point_size {
                Some(p) => {
                    let step = type_size(p.ty);
                    let s = if p.stride > 0 { p.stride as u32 } else { step };
                    read_component(
                        mem,
                        vbase
                            .saturating_add(p.ptr)
                            .saturating_add(first.saturating_add(i).saturating_mul(s)),
                        p.ty,
                    )?
                }
                None => self.point_size,
            };
            let mut matrix_indices = [0u16; 4];
            let mut weights = [0.0f32; 4];
            let mut matrix_count = 0u32;
            if self.matrix_palette {
                if let Some(m) = self.client.matrix_index {
                    let step = type_size(m.ty);
                    let s = if m.stride > 0 {
                        m.stride as u32
                    } else {
                        m.size.max(1) as u32 * step
                    };
                    let b = vbase
                        .saturating_add(m.ptr)
                        .saturating_add(first.saturating_add(i).saturating_mul(s));
                    let n = m.size.clamp(1, 4) as u32;
                    matrix_count = n;
                    for k in 0..n {
                        matrix_indices[k as usize] =
                            read_component(mem, b.saturating_add(k.saturating_mul(step)), m.ty)?
                                as u16;
                    }
                }
                if let Some(w) = self.client.weight {
                    let step = type_size(w.ty);
                    let s = if w.stride > 0 {
                        w.stride as u32
                    } else {
                        w.size.max(1) as u32 * step
                    };
                    let b = vbase
                        .saturating_add(w.ptr)
                        .saturating_add(first.saturating_add(i).saturating_mul(s));
                    let n = w.size.clamp(1, 4) as u32;
                    if matrix_count == 0 {
                        matrix_count = n;
                    }
                    for k in 0..n {
                        weights[k as usize] = read_normalized_component(
                            mem,
                            b.saturating_add(k.saturating_mul(step)),
                            w.ty,
                        )?;
                    }
                }
            }
            out.push(Vertex {
                pos,
                w,
                color,
                tex,
                tex_r,
                tex_q,
                normal,
                point_size,
                matrix_indices,
                weights,
                matrix_count,
            });
        }
        Ok(out)
    }

    /// The matrix a vertex is transformed by: the palette blend when skinning is on, else
    /// the current modelview. Weights are normalised because a pack's weights need only be
    /// proportional, and a zero total falls back to the modelview.
    pub(super) fn vertex_matrix(&self, v: &Vertex) -> Mat4 {
        if !self.matrix_palette || v.matrix_count == 0 {
            return self.matrices[0].current;
        }
        let mut out = [0.0f32; 16];
        let mut total = 0.0f32;
        for i in 0..v.matrix_count.min(4) as usize {
            let weight = v.weights[i];
            if weight <= 0.0 {
                continue;
            }
            if let Some(matrix) = self.palette_matrices.get(v.matrix_indices[i] as usize) {
                let cols = matrix.to_cols_array();
                for k in 0..16 {
                    out[k] += cols[k] * weight;
                }
                total += weight;
            }
        }
        if total <= 0.0 {
            return self.matrices[0].current;
        }
        for value in &mut out {
            *value /= total;
        }
        Mat4::from_cols_array(&out)
    }

    /// The surface a read lands in: the bound FBO's colour target when one is bound, else the
    /// EGL drawable.
    pub(super) fn read_surface<'a>(&'a self, egl: &'a Egl) -> Option<&'a Surface> {
        if self.bound_framebuffer != 0 {
            self.framebuffers
                .get(&self.bound_framebuffer)
                .map(|f| &f.surface)
        } else {
            egl.read_surface()
        }
    }

    /// Take the draw target out so `raster(&self)` can borrow it. An FBO keeps its depth and
    /// stencil in renderbuffers, so those planes are copied into the surface here and copied
    /// back by `put_draw_surface`; the screen's planes are its own.
    pub(super) fn take_draw_surface(&mut self, egl: &mut Egl) -> Option<DrawTarget> {
        if self.bound_framebuffer != 0 {
            let id = self.bound_framebuffer;
            let fb = self.framebuffers.get_mut(&id)?;
            let mut surface = std::mem::take(&mut fb.surface);
            let (depth, stencil) = (fb.depth, fb.stencil);
            let (depth_texture, stencil_texture) = (fb.depth_texture, fb.stencil_texture);
            let (depth_target, stencil_target) = (fb.depth_target, fb.stencil_target);
            let color_target = fb.color_target;
            if let Some(rb) = depth.and_then(|rb| self.renderbuffers.get(&rb)) {
                copy_depth(
                    &rb.depth,
                    rb.width,
                    rb.height,
                    &mut surface.depth,
                    surface.width,
                    surface.height,
                );
            } else if let Some((texture, level)) = depth_texture {
                if let Some(t) = self.textures.get(&texture) {
                    if let (Some((w, h)), Some(plane)) =
                        (t.level_size(level), t.face_depth(depth_target, level))
                    {
                        copy_depth(
                            plane,
                            w,
                            h,
                            &mut surface.depth,
                            surface.width,
                            surface.height,
                        );
                    }
                }
            }
            if let Some(rb) = stencil.and_then(|rb| self.renderbuffers.get(&rb)) {
                copy_stencil(
                    &rb.stencil,
                    rb.width,
                    rb.height,
                    &mut surface.stencil,
                    surface.width,
                    surface.height,
                );
            } else if let Some((texture, level)) = stencil_texture {
                if let Some(t) = self.textures.get(&texture) {
                    if let (Some((w, h)), Some(plane)) =
                        (t.level_size(level), t.face_stencil(stencil_target, level))
                    {
                        copy_stencil(
                            plane,
                            w,
                            h,
                            &mut surface.stencil,
                            surface.width,
                            surface.height,
                        );
                    }
                }
            }
            Some(DrawTarget {
                color: fb.color,
                color_level: fb.color_level,
                color_target,
                surface,
                framebuffer: id,
                depth,
                stencil,
                depth_texture,
                stencil_texture,
                depth_target,
                stencil_target,
            })
        } else {
            egl.current_surface_mut().map(|s| DrawTarget {
                surface: std::mem::take(s),
                color: 0,
                color_level: 0,
                color_target: GL_TEXTURE_2D,
                framebuffer: 0,
                depth: None,
                stencil: None,
                depth_texture: None,
                stencil_texture: None,
                depth_target: GL_TEXTURE_2D,
                stencil_target: GL_TEXTURE_2D,
            })
        }
    }

    pub(super) fn put_draw_surface(&mut self, egl: &mut Egl, target: DrawTarget) {
        let DrawTarget {
            surface,
            color,
            color_level,
            color_target,
            framebuffer,
            depth,
            stencil,
            depth_texture,
            stencil_texture,
            depth_target,
            stencil_target,
        } = target;
        if framebuffer != 0 {
            if let Some(rb) = depth.and_then(|id| self.renderbuffers.get_mut(&id)) {
                copy_depth(
                    &surface.depth,
                    surface.width,
                    surface.height,
                    &mut rb.depth,
                    rb.width,
                    rb.height,
                );
            } else if let Some((texture, level)) = depth_texture {
                if let Some(t) = self.textures.get_mut(&texture) {
                    if let Some((w, h)) = t.level_size(level) {
                        if let Some(plane) = t.face_depth_mut(depth_target, level) {
                            copy_depth(
                                &surface.depth,
                                surface.width,
                                surface.height,
                                plane.as_mut_slice(),
                                w,
                                h,
                            );
                        }
                    }
                }
            }
            if let Some(rb) = stencil.and_then(|id| self.renderbuffers.get_mut(&id)) {
                copy_stencil(
                    &surface.stencil,
                    surface.width,
                    surface.height,
                    &mut rb.stencil,
                    rb.width,
                    rb.height,
                );
            } else if let Some((texture, level)) = stencil_texture {
                if let Some(t) = self.textures.get_mut(&texture) {
                    if let Some((w, h)) = t.level_size(level) {
                        if let Some(plane) = t.face_stencil_mut(stencil_target, level) {
                            copy_stencil(
                                &surface.stencil,
                                surface.width,
                                surface.height,
                                plane.as_mut_slice(),
                                w,
                                h,
                            );
                        }
                    }
                }
            }
            if color != 0 {
                if let Some(t) = self.textures.get_mut(&color) {
                    if let Some(face) = cube_face(color_target) {
                        if color_level == 0 {
                            t.uploaded = true;
                            let count = (surface.width * surface.height * 4) as usize;
                            if t.faces.len() < 6 {
                                t.faces.resize(6, vec![0u8; count]);
                            }
                            t.faces[face] = surface.rgba.clone();
                            if face == 0 {
                                t.rgba = surface.rgba.clone();
                            }
                        } else {
                            let count = (surface.width * surface.height * 4) as usize;
                            let index = color_level as usize - 1;
                            if t.cube_levels.len() <= index {
                                t.cube_levels.resize(index + 1, Vec::new());
                            }
                            let faces = &mut t.cube_levels[index];
                            if faces.len() < 6 {
                                faces.resize(6, vec![0u8; count]);
                            }
                            faces[face] = surface.rgba.clone();
                        }
                    } else if color_level == 0 {
                        t.uploaded = true;
                        t.rgba = surface.rgba.clone();
                    } else if let Some(level) = t.levels.get_mut(color_level as usize - 1) {
                        *level = surface.rgba.clone();
                    }
                }
            }
            if let Some(fb) = self.framebuffers.get_mut(&framebuffer) {
                fb.surface = surface;
            }
        } else if let Some(s) = egl.current_surface_mut() {
            *s = surface;
        }
    }

    pub(super) fn clear(&mut self, mut mask: u32, egl: &mut Egl, _mem: &mut Mem) -> HResult<()> {
        // A profile whose surface has no stencil bits drops that part of the mask rather than
        // clearing a plane the handset does not have. The device does the same: its clear removes
        // the bit before the request reaches the command stream.
        if self.caps.stencil_bits == 0 {
            mask &= !0x0400;
        }
        let Some(mut target) = self.take_draw_surface(egl) else {
            return Ok(());
        };
        let c = [
            (self.clear_color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (self.clear_color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (self.clear_color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (self.clear_color[3].clamp(0.0, 1.0) * 255.0) as u8,
        ];
        {
            let (x0, y0, x1, y1) = self.scissor_bounds(target.surface.width, target.surface.height);
            let s = &mut target.surface;
            for y in y0..y1 {
                for x in x0..x1 {
                    let Some(idx) = s.index(x, y) else {
                        continue;
                    };
                    if mask & 0x4000 != 0 {
                        // Clear obeys the colour write mask, but not the viewport: only scissor
                        // bounds it. That asymmetry is GL's §4.2.3, and packs rely on it.
                        for ch in 0..4 {
                            if self.color_mask[ch] {
                                s.rgba[idx * 4 + ch] = c[ch];
                            }
                        }
                    }
                    if mask & 0x0100 != 0 {
                        s.depth[idx] = self.clear_depth;
                    }
                    if mask & 0x0400 != 0 {
                        s.stencil[idx] = self.clear_stencil as u8;
                    }
                }
            }
        }
        self.put_draw_surface(egl, target);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn texture_data(&self, id: u32) -> Option<(u32, u32, Vec<u8>)> {
        self.textures
            .get(&id)
            .map(|t| (t.width, t.height, t.rgba.clone()))
    }

    pub(crate) fn shareable_snapshot(&self) -> SharedGlObjects {
        SharedGlObjects {
            textures: self.textures.clone(),
            next_texture: self.next_texture,
            buffers: self.buffers.clone(),
            next_buffer: self.next_buffer,
            renderbuffers: self.renderbuffers.clone(),
            next_renderbuffer: self.next_renderbuffer,
        }
    }

    pub(crate) fn restore_shareable(&mut self, shared: &SharedGlObjects) {
        self.textures = shared.textures.clone();
        self.next_texture = shared.next_texture;
        self.buffers = shared.buffers.clone();
        self.next_buffer = shared.next_buffer;
        self.renderbuffers = shared.renderbuffers.clone();
        self.next_renderbuffer = shared.next_renderbuffer;
    }

    pub(crate) fn bind_surface_texture(
        &mut self,
        width: u32,
        height: u32,
        mut rgba: Vec<u8>,
        format: u32,
    ) {
        let unit = self.active_texture;
        let id = self.bound_texture[unit];
        if !self.texture_2d[unit] || id == 0 {
            self.set_error(GL_INVALID_OPERATION);
            return;
        }
        let Some(texture) = self.textures.get_mut(&id) else {
            self.set_error(GL_INVALID_OPERATION);
            return;
        };
        let internal = texture.internal;
        let source_alpha = format != EGL_TEXTURE_RGB;
        for px in rgba.chunks_exact_mut(4) {
            let alpha = if source_alpha { px[3] } else { 255 };
            match internal {
                GL_RGB => px[3] = 255,
                GL_LUMINANCE => {
                    let l = px[0];
                    px[1] = l;
                    px[2] = l;
                    px[3] = 255;
                }
                GL_ALPHA => {
                    px[0] = 0;
                    px[1] = 0;
                    px[2] = 0;
                    px[3] = alpha;
                }
                GL_LUMINANCE_ALPHA => {
                    px[1] = px[0];
                    px[2] = px[0];
                    px[3] = alpha;
                }
                _ => px[3] = alpha,
            }
        }
        texture.width = width.max(1);
        texture.height = height.max(1);
        texture.uploaded = true;
        texture.rgba = rgba;
        texture.generate_mipmaps();
    }

    pub(super) fn submit(
        &mut self,
        vertices: &[Vertex],
        primitive: u32,
        egl: &mut Egl,
        _mem: &mut Mem,
    ) -> HResult<()> {
        if vertices.is_empty() {
            return Ok(());
        }
        let projection = self.matrices[1].current;
        let mut cvs = Vec::with_capacity(vertices.len());
        for v in vertices {
            let modelview = self.vertex_matrix(v);
            let eye = modelview * v.pos.extend(v.w);
            let normal = modelview.transform_vector3(v.normal);
            let color = self.shade_face(eye.truncate(), normal, v.color, false);
            let back_color = if self.two_sided {
                self.shade_face(eye.truncate(), normal, v.color, true)
            } else {
                color
            };
            let mut tex = v.tex;
            let mut tex_r = v.tex_r;
            let mut tex_q = v.tex_q;
            for coord in 0..4 {
                if !self.tex_gen_enabled[coord] {
                    continue;
                }
                let mode = self.tex_gen_mode[coord];
                if mode == GL_SPHERE_MAP {
                    // Sphere mapping defines S and T only; R and Q keep what the vertex carried.
                    if coord >= 2 {
                        continue;
                    }
                    let n = normal.normalize_or_zero();
                    tex[0][coord] = if coord == 0 {
                        (n.x + 1.0) * 0.5
                    } else {
                        (n.y + 1.0) * 0.5
                    };
                    continue;
                }
                let slot = if mode == GL_EYE_LINEAR { 1 } else { 0 };
                let plane = self.tex_gen_plane[coord][slot];
                let p = if mode == GL_EYE_LINEAR {
                    eye.truncate()
                } else {
                    v.pos
                };
                let value = plane[0] * p.x + plane[1] * p.y + plane[2] * p.z + plane[3];
                match coord {
                    0 => tex[0][0] = value,
                    1 => tex[0][1] = value,
                    2 => tex_r[0] = value,
                    _ => tex_q[0] = value,
                }
            }
            cvs.push(ClipVertex {
                clip: projection * eye,
                eye: eye.truncate(),
                color,
                back_color,
                tex,
                tex_r,
                tex_q,
                fog: self.fog_factor(eye.z),
                point_size: v.point_size,
            });
        }
        let tx: Vec<Option<TxVertex>> = cvs.iter().map(|v| self.to_screen(v)).collect();
        // `--gl-log`'s geometry: where the vertices landed on screen and how deep they were. Both
        // are what tells "a full-screen quad was drawn" from "a sprite that missed the viewport",
        // which no amount of state can answer on its own.
        if let Some(probe) = self.probe.as_ref() {
            let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
            let (mut near, mut far) = (f32::MAX, f32::MIN);
            for (i, v) in cvs.iter().enumerate() {
                near = near.min(v.eye.z);
                far = far.max(v.eye.z);
                if let Some(Some(t)) = tx.get(i) {
                    bbox[0] = bbox[0].min(t.screen.x);
                    bbox[1] = bbox[1].min(t.screen.y);
                    bbox[2] = bbox[2].max(t.screen.x);
                    bbox[3] = bbox[3].max(t.screen.y);
                }
            }
            if bbox[0] > bbox[2] {
                bbox = [0.0; 4];
            }
            if near > far {
                near = 0.0;
                far = 0.0;
            }
            probe.note_geometry(bbox, (near, far));
        }
        let Some(mut target) = self.take_draw_surface(egl) else {
            return Ok(());
        };
        // A framebuffer without a depth or stencil attachment has no plane to test against;
        // the screen owns both, so its tests stay as the state says.
        let saved_depth = self.depth_test;
        let saved_stencil = self.stencil_test;
        self.depth_test = target.depth_tested(saved_depth);
        self.stencil_test = target.stencil_tested(saved_stencil, self.caps.stencil_bits != 0);
        let mut surface = std::mem::take(&mut target.surface);
        match primitive {
            GL_POINTS => {
                for v in &tx {
                    if let Some(v) = v {
                        self.raster_point(v, &mut surface);
                    }
                }
            }
            GL_LINES => {
                for pair in tx.chunks(2) {
                    if let [Some(a), Some(b)] = pair {
                        self.raster_line(a, b, &mut surface);
                    }
                }
            }
            GL_LINE_STRIP | GL_LINE_LOOP => {
                for pair in tx.windows(2) {
                    if let [Some(a), Some(b)] = pair {
                        self.raster_line(a, b, &mut surface);
                    }
                }
                if primitive == GL_LINE_LOOP {
                    if let (Some(Some(a)), Some(Some(b))) = (tx.first(), tx.last()) {
                        self.raster_line(a, b, &mut surface);
                    }
                }
            }
            _ => {
                let tris: Vec<[usize; 3]> = match primitive {
                    GL_TRIANGLES => (0..cvs.len() / 3)
                        .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                        .collect(),
                    GL_TRIANGLE_STRIP => (0..cvs.len().saturating_sub(2))
                        .map(|i| {
                            if i % 2 == 0 {
                                [i, i + 1, i + 2]
                            } else {
                                [i + 1, i, i + 2]
                            }
                        })
                        .collect(),
                    GL_TRIANGLE_FAN => (1..cvs.len().saturating_sub(1))
                        .map(|i| [0, i, i + 1])
                        .collect(),
                    _ => Vec::new(),
                };
                for [i, j, k] in tris {
                    let (Some(a), Some(b), Some(c)) = (cvs.get(i), cvs.get(j), cvs.get(k)) else {
                        continue;
                    };
                    self.clip_and_raster_triangle(a, b, c, &mut surface);
                }
            }
        }
        target.surface = surface;
        self.depth_test = saved_depth;
        self.stencil_test = saved_stencil;
        self.put_draw_surface(egl, target);
        Ok(())
    }

    pub(super) fn to_screen(&self, v: &ClipVertex) -> Option<TxVertex> {
        if v.clip.w <= 1e-6 {
            return None;
        }
        let inv_w = 1.0 / v.clip.w;
        let nx = v.clip.x * inv_w;
        let ny = v.clip.y * inv_w;
        let nz = v.clip.z * inv_w;
        let (vx, vy, vw, vh) = (
            self.viewport[0] as f32,
            self.viewport[1] as f32,
            self.viewport[2] as f32,
            self.viewport[3] as f32,
        );
        let sx = vx + (nx * 0.5 + 0.5) * vw;
        // GL's window origin is bottom-left; `blit` flips this into the DIB's top-down order.
        let sy = vy + (ny * 0.5 + 0.5) * vh;
        let sz =
            (nz * 0.5 + 0.5) * (self.depth_range[1] - self.depth_range[0]) + self.depth_range[0];
        Some(TxVertex {
            screen: Vec3::new(sx, sy, sz),
            eye: v.eye,
            inv_w,
            color: v.color,
            back_color: v.back_color,
            tex: v.tex,
            tex_r: v.tex_r,
            tex_q: v.tex_q,
            fog: v.fog,
            point_size: v.point_size,
        })
    }

    /// Cut a triangle against the six clip planes (`x ± w`, `y ± w`, `z ± w`), then fan the
    /// resulting polygon. A triangle whose vertices straddle `w == 0` or one of the frustum edges
    /// would otherwise disappear entirely, which is the common case for a camera standing inside a
    /// room or a sprite reaching past the screen edge.
    pub(super) fn clip_and_raster_triangle(
        &self,
        a: &ClipVertex,
        b: &ClipVertex,
        c: &ClipVertex,
        s: &mut Surface,
    ) {
        let mut poly = vec![*a, *b, *c];
        let planes: [fn(&ClipVertex) -> f32; 6] = [
            |v| v.clip.x + v.clip.w, // left
            |v| v.clip.w - v.clip.x, // right
            |v| v.clip.y + v.clip.w, // bottom
            |v| v.clip.w - v.clip.y, // top
            |v| v.clip.z + v.clip.w, // near
            |v| v.clip.w - v.clip.z, // far
        ];
        for plane in planes {
            let mut out: Vec<ClipVertex> = Vec::with_capacity(poly.len() + 1);
            for i in 0..poly.len() {
                let cur = poly[i];
                let prev = poly[(i + poly.len() - 1) % poly.len()];
                let dc = plane(&cur);
                let dp = plane(&prev);
                let cur_in = dc >= 0.0;
                let prev_in = dp >= 0.0;
                if cur_in != prev_in {
                    let t = dp / (dp - dc);
                    out.push(mix_clip(&prev, &cur, t));
                }
                if cur_in {
                    out.push(cur);
                }
            }
            poly = out;
            if poly.len() < 3 {
                return;
            }
        }
        // User clip planes, in eye space; a vertex is kept where the plane equation is >= 0.
        for (index, plane) in self.clip_planes.iter().enumerate() {
            if !self.clip_plane_enabled[index] {
                continue;
            }
            let distance = |v: &ClipVertex| -> f32 {
                plane[0] * v.eye.x + plane[1] * v.eye.y + plane[2] * v.eye.z + plane[3]
            };
            let mut out: Vec<ClipVertex> = Vec::with_capacity(poly.len() + 1);
            for i in 0..poly.len() {
                let cur = poly[i];
                let prev = poly[(i + poly.len() - 1) % poly.len()];
                let dc = distance(&cur);
                let dp = distance(&prev);
                let cur_in = dc >= 0.0;
                let prev_in = dp >= 0.0;
                if cur_in != prev_in {
                    let t = dp / (dp - dc);
                    out.push(mix_clip(&prev, &cur, t));
                }
                if cur_in {
                    out.push(cur);
                }
            }
            poly = out;
            if poly.len() < 3 {
                return;
            }
        }
        for i in 1..poly.len() - 1 {
            let (Some(ta), Some(tb), Some(tc)) = (
                self.to_screen(&poly[0]),
                self.to_screen(&poly[i]),
                self.to_screen(&poly[i + 1]),
            ) else {
                continue;
            };
            self.raster(&ta, &tb, &tc, s);
        }
    }

    /// `GL_OES_draw_texture`: paint the bound texture as a screen-space rectangle at `(x, y)`,
    /// sampling with the unit 0 texenv just like a textured quad.
    pub(super) fn draw_texture(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        egl: &mut Egl,
    ) -> HResult<()> {
        let unit = self.active_texture;
        let Some(mut target) = self.take_draw_surface(egl) else {
            return Ok(());
        };
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let (w, h) = (w.round() as i32, h.round() as i32);
        if w <= 0 || h <= 0 {
            self.put_draw_surface(egl, target);
            return Ok(());
        }
        let Some(t) = self.textures.get(&self.bound_texture[unit]) else {
            self.put_draw_surface(egl, target);
            return Ok(());
        };
        let saved_depth = self.depth_test;
        let saved_stencil = self.stencil_test;
        self.depth_test = target.depth_tested(saved_depth);
        self.stencil_test = target.stencil_tested(saved_stencil, self.caps.stencil_bits != 0);
        let mut s = std::mem::take(&mut target.surface);
        for row in 0..h {
            for col in 0..w {
                let u = (col as f32 + 0.5) / w as f32;
                let v = (row as f32 + 0.5) / h as f32;
                let tex = if t.uploaded {
                    t.sample(u, v, 0.0)
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                };
                let color = self.combine_one(unit, self.current.color, tex, self.current.color);
                self.plot(x0 + col, y0 + row, self.depth_range[0], color, 1.0, &mut s);
            }
        }
        target.surface = s;
        self.depth_test = saved_depth;
        self.stencil_test = saved_stencil;
        self.put_draw_surface(egl, target);
        Ok(())
    }

    pub(super) fn raster_point(&self, v: &TxVertex, s: &mut Surface) {
        let mut size = if v.point_size > 0.0 {
            v.point_size
        } else {
            self.point_size
        };
        let distance = v.eye.length();
        let [constant, linear, quadratic] = self.point_distance_attenuation;
        let attenuation = constant + linear * distance + quadratic * distance * distance;
        if attenuation > 0.0 {
            size /= attenuation.sqrt();
        }
        let fade = if self.point_fade_threshold > 0.0 && size < self.point_fade_threshold {
            (size / self.point_fade_threshold).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let size = size
            .clamp(self.point_size_min.max(1.0), self.point_size_max.max(1.0))
            .round() as i32;
        let x0 = v.screen.x as i32;
        let y0 = v.screen.y as i32;
        let unit = self.active_texture;
        let sprite = self.point_sprite
            && self.point_sprite_coord_replace
            && self.texture_2d[unit];
        for dy in 0..size {
            for dx in 0..size {
                let mut color = if sprite {
                    let u = (dx as f32 + 0.5) / size as f32;
                    let t = (dy as f32 + 0.5) / size as f32;
                    match self.textures.get(&self.bound_texture[unit]) {
                        Some(texture) if texture.uploaded => {
                            self.combine_one(unit, v.color, texture.sample(u, t, 0.0), v.color)
                        }
                        Some(_) => self.combine_one(unit, v.color, [0.0, 0.0, 0.0, 1.0], v.color),
                        None => v.color,
                    }
                } else {
                    v.color
                };
                color[3] *= fade;
                self.plot(x0 + dx, y0 + dy, v.screen.z, color, v.fog, s);
            }
        }
    }

    pub(super) fn raster_line(&self, a: &TxVertex, b: &TxVertex, s: &mut Surface) {
        let width = self.line_width.max(1.0).min(64.0);
        if width <= 1.0 {
            let dx = b.screen.x - a.screen.x;
            let dy = b.screen.y - a.screen.y;
            let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let x = a.screen.x + dx * t;
                let y = a.screen.y + dy * t;
                let z = a.screen.z + (b.screen.z - a.screen.z) * t;
                let mut color = [0.0f32; 4];
                for ch in 0..4 {
                    color[ch] = a.color[ch] + (b.color[ch] - a.color[ch]) * t;
                }
                let fog = a.fog + (b.fog - a.fog) * t;
                self.plot(x as i32, y as i32, z, color, fog, s);
            }
            return;
        }

        // For a real width, cover every pixel whose centre is within half the line width of
        // the segment. The old square brush was visibly wrong on diagonals.
        let half = width * 0.5;
        let dx = b.screen.x - a.screen.x;
        let dy = b.screen.y - a.screen.y;
        let len2 = dx * dx + dy * dy;
        let min_x = (a.screen.x.min(b.screen.x) - half - 1.0).floor() as i32;
        let max_x = (a.screen.x.max(b.screen.x) + half + 1.0).ceil() as i32;
        let min_y = (a.screen.y.min(b.screen.y) - half - 1.0).floor() as i32;
        let max_y = (a.screen.y.max(b.screen.y) + half + 1.0).ceil() as i32;
        for py in min_y..=max_y {
            let cy = py as f32;
            for px in min_x..=max_x {
                let cx = px as f32;
                let t = if len2 > 0.0 {
                    (((cx - a.screen.x) * dx + (cy - a.screen.y) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let qx = a.screen.x + dx * t;
                let qy = a.screen.y + dy * t;
                if (cx - qx) * (cx - qx) + (cy - qy) * (cy - qy) > half * half {
                    continue;
                }
                let z = a.screen.z + (b.screen.z - a.screen.z) * t;
                let mut color = [0.0f32; 4];
                for ch in 0..4 {
                    color[ch] = a.color[ch] + (b.color[ch] - a.color[ch]) * t;
                }
                let fog = a.fog + (b.fog - a.fog) * t;
                self.plot(px, py, z, color, fog, s);
            }
        }
    }

    /// The one pixel write shared by points and lines: depth test, fog, blend and the depth store
    /// in the same order the triangle path uses.
    /// The rectangle `glScissor` leaves writable, in window coordinates. Without the test the
    /// whole surface is writable; with it the box is clamped to the surface.
    pub(super) fn scissor_bounds(&self, width: u32, height: u32) -> (i32, i32, i32, i32) {
        if !self.scissor_test {
            return (0, 0, width as i32, height as i32);
        }
        let x0 = self.scissor[0].clamp(0, width as i32);
        let y0 = self.scissor[1].clamp(0, height as i32);
        let x1 = self.scissor[0]
            .saturating_add(self.scissor[2])
            .clamp(0, width as i32);
        let y1 = self.scissor[1]
            .saturating_add(self.scissor[3])
            .clamp(0, height as i32);
        (x0.min(x1), y0.min(y1), x1.max(x0), y1.max(y0))
    }

    pub(super) fn scissor_allows(&self, x: i32, y: i32) -> bool {
        if !self.scissor_test {
            return true;
        }
        x >= self.scissor[0]
            && y >= self.scissor[1]
            && x < self.scissor[0].saturating_add(self.scissor[2])
            && y < self.scissor[1].saturating_add(self.scissor[3])
    }

    /// The common tail of points/lines/DrawTex and triangles: blend or logic-op the shaded
    /// colour into the framebuffer, then apply the write masks.
    ///
    /// Logic op and blending are mutually exclusive in ES 1.x (§4.1): a logic op takes the
    /// *unblended* fragment and combines it with the framebuffer. Keeping the two in one place
    /// also keeps `--gl-log`'s first-fragment record from drifting between the two rasterisers.
    fn write_fragment(
        &self,
        x: i32,
        y: i32,
        idx: usize,
        z: f32,
        color: [f32; 4],
        s: &mut Surface,
    ) {
        let dst = [
            s.rgba[idx * 4],
            s.rgba[idx * 4 + 1],
            s.rgba[idx * 4 + 2],
            s.rgba[idx * 4 + 3],
        ];
        let mut out = [
            (color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (color[3].clamp(0.0, 1.0) * 255.0) as u8,
        ];
        if self.logic_op_enabled {
            for ch in 0..4 {
                out[ch] = logic_op_byte(self.logic_op, out[ch], dst[ch]);
            }
        } else if self.blend {
            for ch in 0..4 {
                let src = out[ch] as f32 / 255.0;
                let d = dst[ch] as f32 / 255.0;
                let (sf, df) =
                    self.blend_factor(ch, src, d, out[3] as f32 / 255.0, dst[3] as f32 / 255.0);
                let equation = if ch == 3 {
                    self.blend_equation_alpha
                } else {
                    self.blend_equation
                };
                let blended = match equation {
                    GL_FUNC_SUBTRACT => src * sf - d * df,
                    GL_FUNC_REVERSE_SUBTRACT => d * df - src * sf,
                    GL_MIN => src.min(d),
                    GL_MAX => src.max(d),
                    _ => src * sf + d * df,
                };
                out[ch] = (blended.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        if let Some(probe) = self.probe.as_ref() {
            probe.note_written(out[3] >= 128);
            probe.note_fragment(x, y, dst, color, out);
        }
        for ch in 0..4 {
            if self.color_mask[ch] {
                s.rgba[idx * 4 + ch] = out[ch];
            }
        }
        if self.depth_mask {
            s.depth[idx] = z;
        }
    }

    pub(super) fn plot(&self, x: i32, y: i32, z: f32, color: [f32; 4], fog: f32, s: &mut Surface) {
        let Some(idx) = s.index(x, y) else {
            return;
        };
        if !self.scissor_allows(x, y) {
            return;
        }
        // No polygon offset here: the state applies to filled polygons only, and a point or a line
        // is not one. Applying a constant bias to them was the symptom of the constant bias itself.
        let mut color = color;
        if self.fog {
            // Fog is applied to RGB only; alpha is a coverage value, not a colour channel.
            for ch in 0..3 {
                color[ch] = color[ch] * fog + self.fog_color[ch] * (1.0 - fog);
            }
        }
        if self.sample_coverage {
            let coverage = if self.sample_coverage_invert {
                1.0 - self.sample_coverage_value
            } else {
                self.sample_coverage_value
            };
            color[3] = (color[3] * coverage).clamp(0.0, 1.0);
        }
        // Alpha test precedes stencil/depth updates: a discarded fragment must not write stencil.
        if self.alpha_test && !alpha_pass(self.alpha_func, color[3], self.alpha_ref) {
            return;
        }
        let depth_ok = !self.depth_test || self.depth_pass(z, s.depth[idx]);
        let stencil_ok = if self.stencil_test {
            stencil_pass(
                self.stencil_func,
                s.stencil[idx] as u32 & self.stencil_value_mask,
                self.stencil_ref as u32 & self.stencil_value_mask,
            )
        } else {
            true
        };
        if self.stencil_test {
            let op = if !stencil_ok {
                self.stencil_fail
            } else if !depth_ok {
                self.stencil_pass_depth_fail
            } else {
                self.stencil_pass_depth_pass
            };
            s.stencil[idx] = stencil_op(
                s.stencil[idx],
                op,
                self.stencil_ref as u32,
                self.stencil_write_mask,
            );
        }
        if !depth_ok || !stencil_ok {
            return;
        }
        self.write_fragment(x, y, idx, z, color, s);
    }

    /// One texture unit's TexEnv stage: `previous` is the fragment colour the earlier units left
    /// behind, `tex` is this unit's sample. GLES 1.0 gives each unit its own mode.
    /// One unit's stage with no other samples to reach for, which is what `DrawTex` and a point
    /// sprite need.
    pub(super) fn combine_one(
        &self,
        unit: usize,
        previous: [f32; 4],
        tex: [f32; 4],
        primary: [f32; 4],
    ) -> [f32; 4] {
        let mut samples = [[0.0f32, 0.0, 0.0, 1.0]; 4];
        samples[unit] = tex;
        self.combine(unit, previous, &samples, primary)
    }

    pub(super) fn combine(
        &self,
        unit: usize,
        previous: [f32; 4],
        samples: &[[f32; 4]; 4],
        primary: [f32; 4],
    ) -> [f32; 4] {
        let tex = samples[unit];
        match self.tex_env_mode[unit] {
            GL_COMBINE => {
                let env = self.tex_env_color[unit];
                // `GL_TEXTURE` is this unit's own sample; a numbered name reaches another unit's,
                // which is the crossbar the handset advertises.
                let src = |which: u32| -> [f32; 4] {
                    match which {
                        GL_TEXTURE => tex,
                        GL_TEXTURE0 => samples[0],
                        GL_TEXTURE1 => samples[1],
                        GL_TEXTURE2 => samples[2],
                        GL_TEXTURE3 => samples[3],
                        GL_CONSTANT => env,
                        GL_PRIMARY_COLOR => primary,
                        _ => previous, // GL_PREVIOUS
                    }
                };
                let operand = |operand: u32, s: [f32; 4]| -> [f32; 4] {
                    match operand {
                        GL_ONE_MINUS_SRC_COLOR => [1.0 - s[0], 1.0 - s[1], 1.0 - s[2], 1.0 - s[3]],
                        GL_SRC_ALPHA => [s[3]; 4],
                        GL_ONE_MINUS_SRC_ALPHA => [1.0 - s[3]; 4],
                        _ => s,
                    }
                };
                let a0 = operand(self.operand_rgb[unit][0], src(self.src_rgb[unit][0]));
                let a1 = operand(self.operand_rgb[unit][1], src(self.src_rgb[unit][1]));
                let a2 = operand(self.operand_rgb[unit][2], src(self.src_rgb[unit][2]));
                // DOT3 treats each channel as signed, `2c - 1`, and the three products are summed.
                let dot3 = |s0: [f32; 4], s1: [f32; 4]| -> f32 {
                    (((2.0 * s0[0] - 1.0) * (2.0 * s1[0] - 1.0))
                        + ((2.0 * s0[1] - 1.0) * (2.0 * s1[1] - 1.0))
                        + ((2.0 * s0[2] - 1.0) * (2.0 * s1[2] - 1.0)))
                        .clamp(0.0, 1.0)
                };
                let rgb = match self.combine_rgb[unit] {
                    GL_REPLACE => a0,
                    GL_ADD => [a0[0] + a1[0], a0[1] + a1[1], a0[2] + a1[2], a0[3] + a1[3]],
                    GL_SUBTRACT => [a0[0] - a1[0], a0[1] - a1[1], a0[2] - a1[2], a0[3] - a1[3]],
                    GL_INTERPOLATE => [
                        a0[0] * a2[0] + a1[0] * (1.0 - a2[0]),
                        a0[1] * a2[1] + a1[1] * (1.0 - a2[1]),
                        a0[2] * a2[2] + a1[2] * (1.0 - a2[2]),
                        a0[3] * a2[3] + a1[3] * (1.0 - a2[3]),
                    ],
                    GL_DOT3_RGB | GL_DOT3_RGBA => {
                        let d = dot3(a0, a1);
                        [d, d, d, d]
                    }
                    _ => [a0[0] * a1[0], a0[1] * a1[1], a0[2] * a1[2], a0[3] * a1[3]],
                };
                let b0 = operand(self.operand_alpha[unit][0], src(self.src_alpha[unit][0]));
                let b1 = operand(self.operand_alpha[unit][1], src(self.src_alpha[unit][1]));
                let b2 = operand(self.operand_alpha[unit][2], src(self.src_alpha[unit][2]));
                let alpha = if self.combine_rgb[unit] == GL_DOT3_RGBA {
                    dot3(a0, a1)
                } else {
                    match self.combine_alpha[unit] {
                        GL_REPLACE => b0[3],
                        GL_ADD => b0[3] + b1[3],
                        GL_SUBTRACT => b0[3] - b1[3],
                        GL_INTERPOLATE => b0[3] * b2[3] + b1[3] * (1.0 - b2[3]),
                        _ => b0[3] * b1[3],
                    }
                };
                [
                    (rgb[0] * self.rgb_scale[unit]).clamp(0.0, 1.0),
                    (rgb[1] * self.rgb_scale[unit]).clamp(0.0, 1.0),
                    (rgb[2] * self.rgb_scale[unit]).clamp(0.0, 1.0),
                    (alpha * self.alpha_scale[unit]).clamp(0.0, 1.0),
                ]
            }
            GL_REPLACE => tex,
            GL_ADD => [
                (previous[0] + tex[0]).min(1.0),
                (previous[1] + tex[1]).min(1.0),
                (previous[2] + tex[2]).min(1.0),
                // ADD sums RGB but multiplies alpha (ES 1.0 §3.7.12).
                previous[3] * tex[3],
            ],
            GL_BLEND => {
                let env = self.tex_env_color[unit];
                [
                    previous[0] * (1.0 - tex[0]) + env[0] * tex[0],
                    previous[1] * (1.0 - tex[1]) + env[1] * tex[1],
                    previous[2] * (1.0 - tex[2]) + env[2] * tex[2],
                    // BLEND interpolates RGB with the env colour but multiplies alpha.
                    previous[3] * tex[3],
                ]
            }
            GL_DECAL => {
                let a = tex[3];
                [
                    previous[0] * (1.0 - a) + tex[0] * a,
                    previous[1] * (1.0 - a) + tex[1] * a,
                    previous[2] * (1.0 - a) + tex[2] * a,
                    previous[3],
                ]
            }
            _ => [
                previous[0] * tex[0],
                previous[1] * tex[1],
                previous[2] * tex[2],
                previous[3] * tex[3],
            ],
        }
    }

    pub(super) fn raster(&self, a: &TxVertex, b: &TxVertex, c: &TxVertex, s: &mut Surface) {
        let area = (b.screen.x - a.screen.x) * (c.screen.y - a.screen.y)
            - (b.screen.y - a.screen.y) * (c.screen.x - a.screen.x);
        if area.abs() < 1e-6 {
            return;
        }
        let front = (area > 0.0) == (self.front_face == GL_CCW);
        if self.cull {
            let cull = match self.cull_face {
                GL_FRONT => front,
                GL_BACK => !front,
                GL_FRONT_AND_BACK => true,
                _ => false,
            };
            if cull {
                return;
            }
        }
        // Two-sided lighting selects front/back by triangle winding, the same winding cull uses;
        // the per-vertex colours for both faces were prepared during submit().
        let use_back = self.two_sided && !front;
        let minx = a.screen.x.min(b.screen.x).min(c.screen.x).floor() as i32;
        let maxx = a.screen.x.max(b.screen.x).max(c.screen.x).ceil() as i32;
        let miny = a.screen.y.min(b.screen.y).min(c.screen.y).floor() as i32;
        let maxy = a.screen.y.max(b.screen.y).max(c.screen.y).ceil() as i32;
        let inv_area = 1.0 / area;
        // Window-space depth gradient of the plane the three vertices span. `GL_POLYGON_OFFSET_FILL`
        // pushes a polygon away by `factor * m + units * r`, where `m` is the larger of the two
        // gradients and `r` the smallest difference the depth buffer can hold. A flat polygon
        // therefore takes no offset at all, which a constant bias cannot express.
        let polygon_offset = if self.polygon_offset_fill {
            let dw0dx = (b.screen.y - c.screen.y) * inv_area;
            let dw0dy = (c.screen.x - b.screen.x) * inv_area;
            let dw1dx = (c.screen.y - a.screen.y) * inv_area;
            let dw1dy = (a.screen.x - c.screen.x) * inv_area;
            let dz_dx =
                a.screen.z * dw0dx + b.screen.z * dw1dx - c.screen.z * (dw0dx + dw1dx);
            let dz_dy =
                a.screen.z * dw0dy + b.screen.z * dw1dy - c.screen.z * (dw0dy + dw1dy);
            let slope = dz_dx.abs().max(dz_dy.abs());
            let bits = self.caps.depth_bits.clamp(1, 24);
            let resolution = 1.0 / ((1u32 << bits) as f32 - 1.0);
            self.polygon_offset_factor * slope + self.polygon_offset_units * resolution
        } else {
            0.0
        };
        for y in miny..=maxy {
            for x in minx..=maxx {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = ((b.screen.x - px) * (c.screen.y - py)
                    - (b.screen.y - py) * (c.screen.x - px))
                    * inv_area;
                let w1 = ((c.screen.x - px) * (a.screen.y - py)
                    - (c.screen.y - py) * (a.screen.x - px))
                    * inv_area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let Some(idx) = s.index(x, y) else {
                    continue;
                };
                if !self.scissor_allows(x, y) {
                    continue;
                }
                let mut z = w0 * a.screen.z + w1 * b.screen.z + w2 * c.screen.z;
                z += polygon_offset;
                let (ca, cb, cc) = if use_back {
                    (a.back_color, b.back_color, c.back_color)
                } else {
                    (a.color, b.color, c.color)
                };
                let mut color = if self.shade_model == GL_FLAT {
                    // GLES 1.0 uses the last vertex as the provoking vertex.
                    cc
                } else {
                    let mut color = [0.0f32; 4];
                    for ch in 0..4 {
                        color[ch] = w0 * ca[ch] + w1 * cb[ch] + w2 * cc[ch];
                    }
                    color
                };
                let iw = w0 * a.inv_w + w1 * b.inv_w + w2 * c.inv_w;
                // Sample every enabled unit before combining any: a combine stage may name another
                // unit's texture through the crossbar the handset advertises, so the sources have to
                // exist before the first stage runs. The number of lookups is unchanged.
                let mut samples = [[0.0f32, 0.0, 0.0, 1.0]; 4];
                let mut sampled = [false; 4];
                for unit in 0..self.texture_units() {
                    // Texture object 0 is the default object, not "no texture": an uploaded image
                    // on it must reach the screen. An incomplete one still samples as black.
                    if !self.texture_2d[unit] {
                        continue;
                    }
                    if iw.abs() <= 1e-9 {
                        continue;
                    }
                    // The lookup uses s/q, t/q and r/q. A pack that leaves q alone gets the
                    // coordinates it wrote; one that projects a texture gets the divide it asked
                    // for, and neither case needs the other to be special-cased.
                    let q = (w0 * a.tex_q[unit] * a.inv_w
                        + w1 * b.tex_q[unit] * b.inv_w
                        + w2 * c.tex_q[unit] * c.inv_w)
                        / iw;
                    let scale = if q.abs() > 1e-9 { 1.0 / q } else { 1.0 };
                    let u = (w0 * a.tex[unit][0] * a.inv_w
                        + w1 * b.tex[unit][0] * b.inv_w
                        + w2 * c.tex[unit][0] * c.inv_w)
                        / iw
                        * scale;
                    let v = (w0 * a.tex[unit][1] * a.inv_w
                        + w1 * b.tex[unit][1] * b.inv_w
                        + w2 * c.tex[unit][1] * c.inv_w)
                        / iw
                        * scale;
                    let r = (w0 * a.tex_r[unit] * a.inv_w
                        + w1 * b.tex_r[unit] * b.inv_w
                        + w2 * c.tex_r[unit] * c.inv_w)
                        / iw
                        * scale;
                    if let Some(t) = self.textures.get(&self.bound_texture[unit]) {
                        // An object with no level-zero image is incomplete: the lookup returns
                        // opaque black (OpenGL ES 1.1 §3.7.x), not the 1x1 white placeholder this
                        // host keeps until an upload arrives. That distinction is what turns a
                        // missed upload into a black surface rather than a white one.
                        let tex = if t.uploaded {
                            let lod = lod_of(a, b, c, t.width, t.height, unit);
                            if self.bound_target[unit] == GL_TEXTURE_CUBE_MAP {
                                t.sample_cube(u, v, r, lod, t.mag == GL_LINEAR)
                            } else if t.max_anisotropy > 1.0 {
                                let (major, minor, du, dv) =
                                    crate::gl::helpers::lod_axes(a, b, c, t.width, t.height, unit);
                                t.sample_aniso(u, v, major, minor, du, dv)
                            } else {
                                t.sample(u, v, lod)
                            }
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        };
                        if unit == 0 {
                            if let Some(probe) = self.probe.as_ref() {
                                probe.note_texel(tex);
                            }
                        }
                        samples[unit] = tex;
                        sampled[unit] = true;
                    }
                }
                let primary = color;
                for unit in 0..self.texture_units() {
                    if !sampled[unit] {
                        continue;
                    }
                    color = self.combine(unit, color, &samples, primary);
                }
                if self.fog {
                    let fog = w0 * a.fog + w1 * b.fog + w2 * c.fog;
                    for ch in 0..3 {
                        color[ch] = color[ch] * fog + self.fog_color[ch] * (1.0 - fog);
                    }
                }
                if self.sample_coverage {
                    let coverage = if self.sample_coverage_invert {
                        1.0 - self.sample_coverage_value
                    } else {
                        self.sample_coverage_value
                    };
                    color[3] = (color[3] * coverage).clamp(0.0, 1.0);
                }
                if self.alpha_test && !alpha_pass(self.alpha_func, color[3], self.alpha_ref) {
                    continue;
                }
                let depth_ok = !self.depth_test || self.depth_pass(z, s.depth[idx]);
                let stencil_ok = if self.stencil_test {
                    stencil_pass(
                        self.stencil_func,
                        s.stencil[idx] as u32 & self.stencil_value_mask,
                        self.stencil_ref as u32 & self.stencil_value_mask,
                    )
                } else {
                    true
                };
                if self.stencil_test {
                    let op = if !stencil_ok {
                        self.stencil_fail
                    } else if !depth_ok {
                        self.stencil_pass_depth_fail
                    } else {
                        self.stencil_pass_depth_pass
                    };
                    s.stencil[idx] = stencil_op(
                        s.stencil[idx],
                        op,
                        self.stencil_ref as u32,
                        self.stencil_write_mask,
                    );
                }
                if !depth_ok || !stencil_ok {
                    continue;
                }
                self.write_fragment(x, y, idx, z, color, s);
            }
        }
    }

    pub(super) fn light_mut(&mut self, light: u32) -> Option<&mut Light> {
        let i = light.checked_sub(GL_LIGHT0)? as usize;
        // A profile that reports fewer lights than the array holds still has slots for them, so the
        // count has to be checked here; otherwise a pack writes state no light can ever read.
        if i as u32 >= self.caps.max_lights {
            return None;
        }
        self.lights.get_mut(i)
    }

    /// Apply a material update to the face(s) named by `glMaterial*`. `GL_FRONT_AND_BACK`
    /// calls the update for both copies; an unknown face returns false.
    pub(super) fn set_material(&mut self, face: u32, set: impl Fn(&mut Material)) -> bool {
        match face {
            GL_FRONT => {
                set(&mut self.material);
                true
            }
            GL_BACK => {
                set(&mut self.back_material);
                true
            }
            GL_FRONT_AND_BACK => {
                set(&mut self.material);
                set(&mut self.back_material);
                true
            }
            _ => false,
        }
    }

    pub(super) fn material_for_face(&self, face: u32) -> Option<&Material> {
        match face {
            GL_FRONT | GL_FRONT_AND_BACK => Some(&self.material),
            GL_BACK => Some(&self.back_material),
            _ => None,
        }
    }

    /// Fixed-function per-vertex lighting for callers that have no triangle winding yet (points,
    /// lines, and the unit tests). Triangle rasterisation instead selects the winding-derived
    /// colour prepared by [`Gles::shade_face`].
    pub(super) fn shade(&self, eye: Vec3, normal: Vec3, color: [f32; 4]) -> [f32; 4] {
        let view = if eye.length_squared() > 0.0 {
            (-eye).normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let back = self.two_sided && normal.dot(view) < 0.0;
        self.shade_face(eye, normal, color, back)
    }

    /// The lighting equation for one explicitly chosen face. `GL_LIGHT_MODEL_TWO_SIDE` selects
    /// the back material by triangle winding, not by the vertex normal's relation to the eye.
    pub(super) fn shade_face(
        &self,
        eye: Vec3,
        normal: Vec3,
        color: [f32; 4],
        back: bool,
    ) -> [f32; 4] {
        if !self.lighting {
            return color;
        }
        // `glColorMaterial` substitutes the vertex colour into one material slot while the
        // light equation still runs. The face field decides which copy the substitution touches.
        let view = if eye.length_squared() > 0.0 {
            (-eye).normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let mut material = if back {
            self.back_material
        } else {
            self.material
        };
        let color_face = match self.color_material_face {
            GL_FRONT => !back,
            GL_BACK => back,
            _ => true,
        };
        if self.color_material && color_face {
            match self.color_material_mode {
                GL_AMBIENT => material.ambient = color,
                GL_DIFFUSE => material.diffuse = color,
                GL_SPECULAR => material.specular = color,
                GL_EMISSION => material.emission = color,
                _ => {
                    material.ambient = color;
                    material.diffuse = color;
                }
            }
        }
        let mut n = if (self.normalize || self.rescale_normal) && normal.length_squared() > 0.0 {
            normal.normalize()
        } else {
            normal
        };
        // Two-sided lighting evaluates a back-facing normal as if it pointed at the viewer.
        if back {
            n = -n;
        }
        let mut out = material.emission;
        for ch in 0..4 {
            out[ch] += material.ambient[ch] * self.light_model_ambient[ch];
        }
        for light in self.lights.iter().filter(|l| l.enabled) {
            let pos = Vec3::new(light.position[0], light.position[1], light.position[2]);
            let (lvec, atten) = if light.position[3] == 0.0 {
                let dir = if pos.length_squared() > 0.0 {
                    (-pos).normalize()
                } else {
                    Vec3::new(0.0, 0.0, 1.0)
                };
                (dir, 1.0)
            } else {
                let d = pos - eye;
                let dist = d.length().max(1e-6);
                let a = light.constant_attenuation
                    + light.linear_attenuation * dist
                    + light.quadratic_attenuation * dist * dist;
                (d / dist, if a > 0.0 { 1.0 / a } else { 0.0 })
            };
            let ndotl = n.dot(lvec).max(0.0);
            let spec_view = if self.local_viewer {
                view
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };
            let spec = if ndotl > 0.0 && material.shininess > 0.0 {
                // GL ES 1.x fixed-function specular uses the half-angle vector (Blinn-Phong),
                // not the reflected light vector. With an infinite viewer the half vector is
                // still taken against (0,0,1) in eye space.
                let half = lvec + spec_view;
                let h = if half.length_squared() > 0.0 {
                    half.normalize()
                } else {
                    spec_view
                };
                n.dot(h).max(0.0).powf(material.shininess)
            } else {
                0.0
            };
            // A spot light only affects diffuse and specular inside its cone; ambient is
            // not attenuated by the cone. A directional light has no cone at all.
            let spot = if light.position[3] != 0.0 && light.spot_cutoff < 180.0 {
                let direction = Vec3::new(
                    light.spot_direction[0],
                    light.spot_direction[1],
                    light.spot_direction[2],
                );
                let cosine = (-lvec).dot(direction).clamp(-1.0, 1.0);
                let cutoff = light.spot_cutoff.clamp(0.0, 180.0).to_radians().cos();
                if cosine < cutoff {
                    0.0
                } else {
                    cosine.max(0.0).powf(light.spot_exponent.max(0.0))
                }
            } else {
                1.0
            };
            for ch in 0..4 {
                out[ch] += light.ambient[ch] * material.ambient[ch];
                out[ch] += (light.diffuse[ch] * material.diffuse[ch] * ndotl
                    + light.specular[ch] * material.specular[ch] * spec)
                    * atten
                    * spot;
            }
        }
        out
    }

    /// GL's fog factor from the eye-space z distance: 1 is no fog, 0 is fully fogged.
    pub(super) fn fog_factor(&self, z: f32) -> f32 {
        if !self.fog {
            return 1.0;
        }
        let d = z.abs();
        match self.fog_mode {
            GL_LINEAR => {
                ((self.fog_end - d) / (self.fog_end - self.fog_start).max(1e-6)).clamp(0.0, 1.0)
            }
            GL_EXP => (-self.fog_density * d).exp(),
            GL_EXP2 => (-(self.fog_density * d).powi(2)).exp(),
            _ => 1.0,
        }
    }

    pub(super) fn blend_factor(
        &self,
        ch: usize,
        src: f32,
        dst: f32,
        sa: f32,
        da: f32,
    ) -> (f32, f32) {
        let (src_f, dst_f) = if ch == 3 {
            (self.blend_src_alpha, self.blend_dst_alpha)
        } else {
            (self.blend_src, self.blend_dst)
        };
        let factor = |f: u32| -> f32 {
            match f {
                GL_ZERO => 0.0,
                GL_ONE => 1.0,
                GL_SRC_COLOR => src,
                GL_ONE_MINUS_SRC_COLOR => 1.0 - src,
                GL_DST_COLOR => dst,
                GL_ONE_MINUS_DST_COLOR => 1.0 - dst,
                GL_SRC_ALPHA => sa,
                GL_ONE_MINUS_SRC_ALPHA => 1.0 - sa,
                GL_DST_ALPHA => da,
                GL_ONE_MINUS_DST_ALPHA => 1.0 - da,
                GL_SRC_ALPHA_SATURATE => sa.min(1.0 - da),
                _ => 1.0,
            }
        };
        (factor(src_f), factor(dst_f))
    }

    pub(super) fn depth_pass(&self, z: f32, stored: f32) -> bool {
        match self.depth_func {
            GL_NEVER => false,
            GL_LESS => z < stored,
            GL_EQUAL => (z - stored).abs() < 1e-6,
            GL_LEQUAL => z <= stored,
            GL_GREATER => z > stored,
            GL_NOTEQUAL => (z - stored).abs() >= 1e-6,
            GL_GEQUAL => z >= stored,
            GL_ALWAYS => true,
            _ => false,
        }
    }
}

/// One bitwise colour operation from the GL `LogicOp` table. GL applies the operation to
/// each RGBA component, with the fragment as source and the framebuffer as destination.
pub(super) fn logic_op_byte(op: u32, src: u8, dst: u8) -> u8 {
    let (s, d) = (src as u32, dst as u32);
    let out = match op {
        GL_CLEAR => 0,
        GL_AND => s & d,
        GL_AND_REVERSE => s & !d,
        GL_COPY => s,
        GL_AND_INVERTED => !s & d,
        GL_NOOP => d,
        GL_XOR => s ^ d,
        GL_OR => s | d,
        GL_NOR => !(s | d),
        GL_EQUIV => !(s ^ d),
        GL_INVERT => !d,
        GL_OR_REVERSE => s | !d,
        GL_COPY_INVERTED => !s,
        GL_OR_INVERTED => !s | d,
        GL_NAND => !(s & d),
        GL_SET => u32::MAX,
        _ => s,
    };
    (out & 0xff) as u8
}
