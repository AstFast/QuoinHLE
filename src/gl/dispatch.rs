//! `Gles::method`: the single guest-facing GL entry point and its name dispatch.

use super::helpers::*;
use super::*;

/// Read one enable-ish state bit by its enum. `GetBooleanv` and `IsEnabled`
/// used to carry two separate `match` blocks over the same fields, and the copies
/// disagreed: `Enable(GL_LIGHTING)` set `lighting`, `GetBooleanv(GL_LIGHTING)` answered TRUE and
/// `IsEnabled(GL_LIGHTING)` answered FALSE, while `GL_TEXTURE_GEN_S` went the other way. One
/// reader means the two queries cannot contradict each other about the host's own state.
/// `None` is "not an enum this host models"; the caller decides what that means for its API.
fn enabled_state(gl: &Gles, cap: u32) -> Option<bool> {
    Some(match cap {
        GL_DEPTH_TEST => gl.depth_test,
        GL_BLEND => gl.blend,
        GL_COLOR_LOGIC_OP => gl.logic_op_enabled,
        GL_CULL_FACE => gl.cull,
        GL_SCISSOR_TEST => gl.scissor_test,
        GL_TEXTURE_2D => gl.texture_2d[gl.active_texture],
        GL_ALPHA_TEST => gl.alpha_test,
        GL_SAMPLE_COVERAGE => gl.sample_coverage,
        GL_SAMPLE_COVERAGE_INVERT => gl.sample_coverage_invert,
        GL_LIGHTING => gl.lighting,
        GL_NORMALIZE => gl.normalize,
        GL_RESCALE_NORMAL => gl.rescale_normal,
        GL_POINT_SPRITE_OES => gl.point_sprite,
        GL_MATRIX_PALETTE_OES => gl.matrix_palette,
        GL_COLOR_MATERIAL => gl.color_material,
        GL_LIGHT_MODEL_TWO_SIDE => gl.two_sided,
        GL_LIGHT_MODEL_LOCAL_VIEWER => gl.local_viewer,
        GL_FOG => gl.fog,
        GL_STENCIL_TEST => gl.stencil_test,
        GL_POLYGON_OFFSET_FILL => gl.polygon_offset_fill,
        x if (GL_LIGHT0..GL_LIGHT0 + 8).contains(&x) => gl.lights[(x - GL_LIGHT0) as usize].enabled,
        x if (GL_CLIP_PLANE0..GL_CLIP_PLANE0 + 6).contains(&x) => {
            gl.clip_plane_enabled[(x - GL_CLIP_PLANE0) as usize]
        }
        x if (GL_TEXTURE_GEN_S..=GL_TEXTURE_GEN_Q).contains(&x) => {
            gl.tex_gen_enabled[(x - GL_TEXTURE_GEN_S) as usize]
        }
        _ => return None,
    })
}

/// `glIsEnabled` only accepts the caps `glEnable`/`glDisable` can switch (GLES 1.1 §2.4);
/// `glGetBooleanv` also reads plain state variables such as `GL_LIGHT_MODEL_TWO_SIDE`. This is
/// the positive whitelist of switchable caps — the same set the `Enable`/`Disable` arm accepts —
/// so an enum added to `enabled_state` later does not silently become an `IsEnabled` answer.
/// It is a set of names, not a second copy of the state values.
fn is_switchable_cap(cap: u32) -> bool {
    matches!(
        cap,
        GL_DEPTH_TEST
            | GL_BLEND
            | GL_COLOR_LOGIC_OP
            | GL_SAMPLE_COVERAGE
            | GL_CULL_FACE
            | GL_SCISSOR_TEST
            | GL_TEXTURE_2D
            | GL_ALPHA_TEST
            | GL_LIGHTING
            | GL_NORMALIZE
            | GL_RESCALE_NORMAL
            | GL_POINT_SPRITE_OES
            | GL_MATRIX_PALETTE_OES
            | GL_COLOR_MATERIAL
            | GL_FOG
            | GL_STENCIL_TEST
            | GL_POLYGON_OFFSET_FILL
    ) || (GL_LIGHT0..GL_LIGHT0 + 8).contains(&cap)
        || (GL_CLIP_PLANE0..GL_CLIP_PLANE0 + 6).contains(&cap)
        || (GL_TEXTURE_GEN_S..=GL_TEXTURE_GEN_Q).contains(&cap)
}

impl Gles {
    pub fn method(
        &mut self,
        iface: &str,
        name: &str,
        args: &[u32],
        mem: &mut Mem,
        egl: &mut Egl,
    ) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        let f = |n: usize| f32::from_bits(a(n));
        let fx = |n: usize| (a(n) as i32) as f32 / 65536.0;
        // The Imageon face spells the same entry points with QUALCOMM/EXT suffixes.
        let name = match name {
            "BlendEquationSeparateEXT" => "BlendEquationSeparate",
            "BlendFuncSeparateEXT" => "BlendFuncSeparate",
            "BlendEquationEXT" => "BlendEquation",
            "BlendEquationOES" => "BlendEquation",
            "BlendEquationSeparateOES" => "BlendEquationSeparate",
            "BlendFuncSeparateOES" => "BlendFuncSeparate",
            "TexGenfOES" => "TexGenf",
            "TexGenfvOES" => "TexGenfv",
            "TexGeniOES" => "TexGeni",
            "TexGenivOES" => "TexGeniv",
            "TexGenxOES" => "TexGenx",
            "TexGenxvOES" => "TexGenxv",
            "BindBufferQUALCOMM" => "BindBuffer",
            "DeleteBuffersQUALCOMM" => "DeleteBuffers",
            "GenBuffersQUALCOMM" => "GenBuffers",
            "BufferDataQUALCOMM" => "BufferData",
            "BufferSubDataQUALCOMM" => "BufferSubData",
            "IsBufferQUALCOMM" => "IsBuffer",
            _ => name,
        };
        match name {
            // ---- state ----
            "ClearColor" => {
                self.clear_color = [f(0), f(1), f(2), f(3)];
                Ok(0)
            }
            "ClearColorx" => {
                self.clear_color = [fx(0), fx(1), fx(2), fx(3)];
                Ok(0)
            }
            "ClearDepthf" | "ClearDepthx" => {
                self.clear_depth = if name.ends_with('x') { fx(0) } else { f(0) };
                Ok(0)
            }
            "DepthRangef" | "DepthRangex" => {
                self.depth_range = if name.ends_with('x') {
                    [fx(0), fx(1)]
                } else {
                    [f(0), f(1)]
                };
                Ok(0)
            }
            "Color4f" => {
                self.current.color = [f(0), f(1), f(2), f(3)];
                Ok(0)
            }
            "Color4x" => {
                self.current.color = [fx(0), fx(1), fx(2), fx(3)];
                Ok(0)
            }
            "Normal3f" | "Normal3x" => {
                self.current.normal = if name.ends_with('x') {
                    Vec3::new(fx(0), fx(1), fx(2))
                } else {
                    Vec3::new(f(0), f(1), f(2))
                };
                Ok(0)
            }
            "MultiTexCoord4f" | "MultiTexCoord4x" => {
                let unit = (a(0).wrapping_sub(GL_TEXTURE0) as usize).min(3);
                self.current.tex[unit] = if name.contains('x') {
                    [fx(1), fx(2)]
                } else {
                    [f(1), f(2)]
                };
                self.current.tex_r[unit] = if name.contains('x') { fx(3) } else { f(3) };
                Ok(0)
            }
            "MultiTexCoord3f" | "MultiTexCoord3x" => {
                let unit = (a(0).wrapping_sub(GL_TEXTURE0) as usize).min(3);
                self.current.tex[unit] = if name.contains('x') {
                    [fx(1), fx(2)]
                } else {
                    [f(1), f(2)]
                };
                self.current.tex_r[unit] = if name.contains('x') { fx(3) } else { f(3) };
                Ok(0)
            }
            "TexCoord4f" | "TexCoord4x" => {
                self.current.tex[0] = if name.contains('x') {
                    [fx(0), fx(1)]
                } else {
                    [f(0), f(1)]
                };
                self.current.tex_r[0] = if name.contains('x') { fx(2) } else { f(2) };
                Ok(0)
            }
            "Viewport" => {
                self.viewport = [a(0) as i32, a(1) as i32, a(2) as i32, a(3) as i32];
                Ok(0)
            }
            "Scissor" => {
                self.scissor = [a(0) as i32, a(1) as i32, a(2) as i32, a(3) as i32];
                Ok(0)
            }
            "ColorMask" => {
                self.color_mask = [a(0) != 0, a(1) != 0, a(2) != 0, a(3) != 0];
                Ok(0)
            }
            "DepthFunc" => {
                let func = a(0);
                if !is_compare_func(func) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.depth_func = func;
                Ok(0)
            }
            "DepthMask" => {
                self.depth_mask = a(0) != 0;
                Ok(0)
            }
            "BlendFunc" => {
                let (src, dst) = (a(0), a(1));
                if !is_blend_factor(src) || !is_blend_factor(dst) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.blend_src = src;
                self.blend_dst = dst;
                self.blend_src_alpha = src;
                self.blend_dst_alpha = dst;
                Ok(0)
            }
            "BlendEquation" => {
                let equation = a(0);
                if !matches!(
                    equation,
                    GL_FUNC_ADD | GL_FUNC_SUBTRACT | GL_FUNC_REVERSE_SUBTRACT
                ) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.blend_equation = equation;
                self.blend_equation_alpha = equation;
                Ok(0)
            }
            "BlendFuncSeparate" => {
                let (src, dst, src_alpha, dst_alpha) = (a(0), a(1), a(2), a(3));
                if !is_blend_factor(src)
                    || !is_blend_factor(dst)
                    || !is_blend_factor(src_alpha)
                    || !is_blend_factor(dst_alpha)
                {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.blend_src = src;
                self.blend_dst = dst;
                self.blend_src_alpha = src_alpha;
                self.blend_dst_alpha = dst_alpha;
                Ok(0)
            }
            "BlendEquationSeparate" => {
                let (rgb, alpha) = (a(0), a(1));
                let valid =
                    |v: u32| matches!(v, GL_FUNC_ADD | GL_FUNC_SUBTRACT | GL_FUNC_REVERSE_SUBTRACT);
                if !valid(rgb) || !valid(alpha) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.blend_equation = rgb;
                self.blend_equation_alpha = alpha;
                Ok(0)
            }
            "CullFace" => {
                let face = a(0);
                if !matches!(face, GL_FRONT | GL_BACK | GL_FRONT_AND_BACK) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.cull_face = face;
                Ok(0)
            }
            "FrontFace" => {
                let face = a(0);
                if !matches!(face, GL_CW | GL_CCW) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.front_face = face;
                Ok(0)
            }
            "ShadeModel" => {
                let model = a(0);
                if !matches!(model, GL_FLAT | GL_SMOOTH) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.shade_model = model;
                Ok(0)
            }
            "LineWidth" | "LineWidthx" => {
                self.line_width = if name.contains('x') { fx(0) } else { f(0) };
                Ok(0)
            }
            "PointSize" | "PointSizex" => {
                self.point_size = if name.contains('x') { fx(0) } else { f(0) };
                Ok(0)
            }
            "PolygonOffset" | "PolygonOffsetx" => {
                if name.contains('x') {
                    self.polygon_offset_factor = fx(0);
                    self.polygon_offset_units = fx(1);
                } else {
                    self.polygon_offset_factor = f(0);
                    self.polygon_offset_units = f(1);
                }
                Ok(0)
            }
            "Hint" => Ok(0),
            "PixelStorei" => {
                let (pname, value) = (a(0), a(1));
                match pname {
                    GL_PACK_ALIGNMENT | GL_UNPACK_ALIGNMENT => {
                        if !matches!(value, 1 | 2 | 4 | 8) {
                            self.set_error(GL_INVALID_VALUE);
                            return Ok(0);
                        }
                        if pname == GL_PACK_ALIGNMENT {
                            self.pack_alignment = value;
                        } else {
                            self.unpack_alignment = value;
                        }
                    }
                    _ => self.set_error(GL_INVALID_ENUM),
                }
                Ok(0)
            }
            "SampleCoverage" | "SampleCoveragex" => {
                let value = if name.ends_with('x') { fx(0) } else { f(0) };
                self.sample_coverage_value = value.clamp(0.0, 1.0);
                self.sample_coverage_invert = a(1) != 0;
                Ok(0)
            }
            "AlphaFunc" => {
                let func = a(0);
                if !is_compare_func(func) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.alpha_func = func;
                self.alpha_ref = f(1);
                Ok(0)
            }
            "AlphaFuncx" => {
                let func = a(0);
                if !is_compare_func(func) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.alpha_func = func;
                self.alpha_ref = fx(1);
                Ok(0)
            }
            "Enable" | "Disable" => {
                let on = name == "Enable";
                match a(0) {
                    GL_DEPTH_TEST => self.depth_test = on,
                    GL_BLEND => self.blend = on,
                    GL_COLOR_LOGIC_OP => self.logic_op_enabled = on,
                    GL_SAMPLE_COVERAGE => self.sample_coverage = on,
                    GL_CULL_FACE => self.cull = on,
                    GL_SCISSOR_TEST => self.scissor_test = on,
                    GL_TEXTURE_2D => self.texture_2d[self.active_texture] = on,
                    GL_ALPHA_TEST => self.alpha_test = on,
                    GL_LIGHTING => self.lighting = on,
                    GL_NORMALIZE => self.normalize = on,
                    GL_RESCALE_NORMAL => self.rescale_normal = on,
                    GL_POINT_SPRITE_OES => self.point_sprite = on,
                    GL_MATRIX_PALETTE_OES => self.matrix_palette = on,
                    GL_COLOR_MATERIAL => self.color_material = on,
                    GL_FOG => self.fog = on,
                    GL_STENCIL_TEST => self.stencil_test = on,
                    GL_POLYGON_OFFSET_FILL => self.polygon_offset_fill = on,
                    x if (GL_LIGHT0..GL_LIGHT0 + 8).contains(&x) => {
                        self.lights[(x - GL_LIGHT0) as usize].enabled = on;
                    }
                    x if (GL_CLIP_PLANE0..GL_CLIP_PLANE0 + 6).contains(&x) => {
                        self.clip_plane_enabled[(x - GL_CLIP_PLANE0) as usize] = on;
                    }
                    x if (GL_TEXTURE_GEN_S..=GL_TEXTURE_GEN_Q).contains(&x) => {
                        self.tex_gen_enabled[(x - GL_TEXTURE_GEN_S) as usize] = on;
                    }
                    GL_VERTEX_ARRAY | GL_COLOR_ARRAY | GL_NORMAL_ARRAY | GL_TEXTURE_COORD_ARRAY => {
                        if !on {
                            self.clear_client(a(0));
                        }
                    }
                    _ => self.set_error(GL_INVALID_ENUM),
                }
                Ok(0)
            }
            "EnableClientState" | "DisableClientState" => {
                let cap = a(0);
                if !matches!(
                    cap,
                    GL_VERTEX_ARRAY
                        | GL_COLOR_ARRAY
                        | GL_NORMAL_ARRAY
                        | GL_TEXTURE_COORD_ARRAY
                        | GL_POINT_SIZE_ARRAY_OES
                        | GL_MATRIX_INDEX_ARRAY_OES
                        | GL_WEIGHT_ARRAY_OES
                ) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if name == "DisableClientState" {
                    self.clear_client(cap);
                }
                Ok(0)
            }
            "VertexPointer" | "ColorPointer" | "NormalPointer" | "TexCoordPointer" => {
                let (size, ty, stride, ptr) = if name == "NormalPointer" {
                    (3, a(0), a(1) as i32, a(2))
                } else {
                    (a(0) as i32, a(1), a(2) as i32, a(3))
                };
                if !is_array_type(ty) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let valid_size = match name {
                    "VertexPointer" => (2..=4).contains(&size),
                    "NormalPointer" => size == 3,
                    _ => (1..=4).contains(&size),
                };
                if !valid_size {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let arr = Array {
                    ptr,
                    size,
                    ty,
                    stride,
                };
                match name {
                    "VertexPointer" => self.client.vertex = Some(arr),
                    "ColorPointer" => self.client.color = Some(arr),
                    "NormalPointer" => self.client.normal = Some(arr),
                    _ => self.client_tex[self.client_active_texture] = Some(arr),
                }
                Ok(0)
            }
            "GetError" => {
                let e = self.error;
                self.error = GL_NO_ERROR;
                Ok(e)
            }
            "GetString" => {
                let text: &[u8] = match a(0) {
                    0x1F00 => b"QuoinHLE\0",
                    0x1F01 => b"QuoinHLE software GLES\0",
                    0x1F02 => b"OpenGL ES-CM 1.0\0",
                    0x1F03 => b"GL_ATI_texture_compression_atitc GL_IMG_texture_compression_pvrtc GL_OES_read_format GL_OES_compressed_paletted_texture GL_OES_query_matrix GL_OES_vertex_buffer_object GL_OES_draw_texture GL_OES_compressed_ETC1_RGB8_texture GL_OES_framebuffer_object GL_OES_blend_subtract GL_OES_blend_func_separate GL_OES_blend_equation_separate GL_OES_point_size_array GL_OES_texture_npot GL_OES_point_sprite GL_OES_matrix_palette GL_OES_texture_mirrored_repeat GL_EXT_texture_filter_anisotropic\0",
                    _ => return Ok(0),
                };
                let at = mem.alloc(text.len() as u32, 1)?;
                mem.write_bytes(at, text)?;
                if iface == "IGL" {
                    // `IGL::GetString` answers with the pointer itself.
                    Ok(at)
                } else {
                    // `IGLES10::GetString(name, AEEGLubyte const **ret)` writes through `ret`.
                    let ret = a(1);
                    if ret != 0 {
                        mem.write_u32(ret, at)?;
                    }
                    Ok(0)
                }
            }
            "GetIntegerv" | "GetFixedv" | "GetFloatv" => {
                let pname = a(0);
                let dst = a(1);
                if dst == 0 {
                    return Ok(0);
                }
                let matrix = match pname {
                    GL_MODELVIEW_MATRIX => Some(self.matrices[0].current),
                    GL_PROJECTION_MATRIX => Some(self.matrices[1].current),
                    GL_TEXTURE_MATRIX => Some(self.matrices[2].current),
                    _ => None,
                };
                if let Some(m) = matrix {
                    // Integer queries do not accept matrix names; the float and fixed entry
                    // points write the 16 components in column-major order.
                    if name == "GetIntegerv" {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                    for (i, v) in m.to_cols_array().iter().enumerate() {
                        let at = dst + i as u32 * 4;
                        match name {
                            "GetFloatv" => mem.write_u32(at, v.to_bits())?,
                            _ => mem.write_u32(at, ((*v * 65536.0) as i32) as u32)?,
                        }
                    }
                    return Ok(0);
                }
                let values: [f32; 4] = match pname {
                    GL_VIEWPORT => [
                        self.viewport[0] as f32,
                        self.viewport[1] as f32,
                        self.viewport[2] as f32,
                        self.viewport[3] as f32,
                    ],
                    GL_SCISSOR_BOX => [
                        self.scissor[0] as f32,
                        self.scissor[1] as f32,
                        self.scissor[2] as f32,
                        self.scissor[3] as f32,
                    ],
                    GL_DEPTH_RANGE => [self.depth_range[0], self.depth_range[1], 0.0, 0.0],
                    GL_COLOR_CLEAR_VALUE => self.clear_color,
                    GL_DEPTH_CLEAR_VALUE => [self.clear_depth, 0.0, 0.0, 0.0],
                    GL_LINE_WIDTH => [self.line_width, 0.0, 0.0, 0.0],
                    GL_POINT_SIZE => [self.point_size, 0.0, 0.0, 0.0],
                    GL_POINT_SIZE_MIN => [self.point_size_min, 0.0, 0.0, 0.0],
                    GL_POINT_SIZE_MAX => [self.point_size_max, 0.0, 0.0, 0.0],
                    GL_POINT_FADE_THRESHOLD_SIZE => [self.point_fade_threshold, 0.0, 0.0, 0.0],
                    GL_POINT_DISTANCE_ATTENUATION => [
                        self.point_distance_attenuation[0],
                        self.point_distance_attenuation[1],
                        self.point_distance_attenuation[2],
                        0.0,
                    ],
                    GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT => [16.0, 0.0, 0.0, 0.0],
                    GL_SAMPLE_COVERAGE_VALUE => [self.sample_coverage_value, 0.0, 0.0, 0.0],
                    GL_SAMPLES | GL_SAMPLE_BUFFERS => [0.0, 0.0, 0.0, 0.0],
                    GL_LIGHT_MODEL_LOCAL_VIEWER => {
                        [if self.local_viewer { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0]
                    }
                    GL_CURRENT_COLOR => self.current.color,
                    GL_CURRENT_NORMAL => [
                        self.current.normal.x,
                        self.current.normal.y,
                        self.current.normal.z,
                        0.0,
                    ],
                    GL_TEXTURE_BINDING_2D => [
                        self.bound_texture[self.active_texture] as f32,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    GL_ARRAY_BUFFER_BINDING => [self.bound_buffer_id[0] as f32, 0.0, 0.0, 0.0],
                    GL_ELEMENT_ARRAY_BUFFER_BINDING => {
                        [self.bound_buffer_id[1] as f32, 0.0, 0.0, 0.0]
                    }
                    GL_ACTIVE_TEXTURE => [
                        (GL_TEXTURE0 + self.active_texture as u32) as f32,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    GL_CLIENT_ACTIVE_TEXTURE => [
                        (GL_TEXTURE0 + self.client_active_texture as u32) as f32,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    GL_MATRIX_MODE => [
                        match self.mode {
                            0 => GL_MODELVIEW,
                            1 => GL_PROJECTION,
                            _ => GL_TEXTURE,
                        } as f32,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    GL_DEPTH_FUNC => [self.depth_func as f32, 0.0, 0.0, 0.0],
                    GL_CULL_FACE_MODE => [self.cull_face as f32, 0.0, 0.0, 0.0],
                    GL_FRONT_FACE => [self.front_face as f32, 0.0, 0.0, 0.0],
                    GL_SHADE_MODEL => [self.shade_model as f32, 0.0, 0.0, 0.0],
                    GL_LIGHT_MODEL_TWO_SIDE => {
                        [if self.two_sided { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0]
                    }
                    GL_ALPHA_TEST_FUNC => [self.alpha_func as f32, 0.0, 0.0, 0.0],
                    GL_STENCIL_FUNC => [self.stencil_func as f32, 0.0, 0.0, 0.0],
                    GL_BLEND_SRC_RGB => [self.blend_src as f32, 0.0, 0.0, 0.0],
                    GL_LOGIC_OP_MODE => [self.logic_op as f32, 0.0, 0.0, 0.0],
                    GL_BLEND_DST_RGB => [self.blend_dst as f32, 0.0, 0.0, 0.0],
                    0x0D3A => [2048.0, 2048.0, 0.0, 0.0], // GL_MAX_VIEWPORT_DIMS
                    0x0D33 => [2048.0, 0.0, 0.0, 0.0],    // GL_MAX_TEXTURE_SIZE
                    0x0D31 => [8.0, 0.0, 0.0, 0.0],       // GL_MAX_LIGHTS
                    0x84E2 => [4.0, 0.0, 0.0, 0.0],       // GL_MAX_TEXTURE_UNITS
                    GL_MAX_PALETTE_MATRICES_OES => [32.0, 0.0, 0.0, 0.0],
                    GL_MAX_VERTEX_UNITS_OES => [4.0, 0.0, 0.0, 0.0],
                    GL_CURRENT_PALETTE_MATRIX_OES => {
                        [self.current_palette_matrix as f32, 0.0, 0.0, 0.0]
                    }
                    0x0D56 => [16.0, 0.0, 0.0, 0.0], // GL_DEPTH_BITS
                    0x0D57 => [8.0, 0.0, 0.0, 0.0],  // GL_STENCIL_BITS
                    // GL_OES_read_format: the only format/type ReadPixels accepts.
                    0x8B9B => [6408.0, 0.0, 0.0, 0.0], // GL_IMPLEMENTATION_COLOR_READ_FORMAT_OES -> GL_RGBA
                    0x8B9A => [3585.0, 0.0, 0.0, 0.0], // GL_IMPLEMENTATION_COLOR_READ_TYPE_OES -> GL_UNSIGNED_BYTE
                    GL_FRAMEBUFFER_BINDING_OES => [self.bound_framebuffer as f32, 0.0, 0.0, 0.0],
                    GL_RENDERBUFFER_BINDING_OES => [self.bound_renderbuffer as f32, 0.0, 0.0, 0.0],
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        [0.0; 4]
                    }
                };
                let count = match pname {
                    GL_VIEWPORT | GL_SCISSOR_BOX | GL_COLOR_CLEAR_VALUE | GL_CURRENT_COLOR => 4,
                    GL_DEPTH_RANGE | 0x0D3A => 2,
                    GL_CURRENT_NORMAL => 3,
                    GL_POINT_DISTANCE_ATTENUATION => 3,
                    _ => 1,
                };
                for (i, v) in values.iter().take(count).enumerate() {
                    let at = dst + i as u32 * 4;
                    match name {
                        "GetFloatv" => mem.write_u32(at, v.to_bits())?,
                        "GetFixedv" => mem.write_u32(at, ((v * 65536.0) as i32) as u32)?,
                        _ => mem.write_u32(at, *v as u32)?,
                    }
                }
                Ok(0)
            }
            "QueryMatrixx" => {
                let dst = a(0);
                if dst != 0 {
                    // Three fixed-point 4x4 matrices back to back, then a bitmask of which ones
                    // were written: GL_MODELVIEW | GL_PROJECTION | GL_TEXTURE.
                    for (i, stack) in self.matrices.iter().enumerate() {
                        for (j, v) in stack.current.to_cols_array().iter().enumerate() {
                            let fixed = (v * 65536.0) as i32;
                            mem.write_u32(dst + ((i * 16 + j) as u32) * 4, fixed as u32)?;
                        }
                    }
                }
                Ok(0x7)
            }
            "QueryMatrixxOES" => {
                let (fixed, exponents, out) = (a(0), a(1), a(2));
                for (i, stack) in self.matrices.iter().enumerate() {
                    for (j, v) in stack.current.to_cols_array().iter().enumerate() {
                        let at = ((i * 16 + j) as u32) * 4;
                        if fixed != 0 {
                            mem.write_u32(fixed + at, ((*v * 65536.0) as i32) as u32)?;
                        }
                        if exponents != 0 {
                            mem.write_u32(exponents + at, 0)?;
                        }
                    }
                }
                if out != 0 {
                    mem.write_u32(out, 0x7)?;
                }
                Ok(0)
            }
            "GetBooleanv" => {
                let (pname, dst) = (a(0), a(1));
                if dst == 0 {
                    return Ok(0);
                }
                let on = match enabled_state(self, pname) {
                    Some(on) => on,
                    None => {
                        self.set_error(GL_INVALID_ENUM);
                        false
                    }
                };
                mem.write_u8(dst, u8::from(on))?;
                Ok(0)
            }
            "GetPointerv" => {
                let (pname, dst) = (a(0), a(1));
                if dst == 0 {
                    return Ok(0);
                }
                let ptr = match pname {
                    GL_VERTEX_ARRAY_POINTER => self.client.vertex.map(|a| a.ptr),
                    GL_COLOR_ARRAY_POINTER => self.client.color.map(|a| a.ptr),
                    GL_NORMAL_ARRAY_POINTER => self.client.normal.map(|a| a.ptr),
                    GL_TEXTURE_COORD_ARRAY_POINTER => self.client_tex[0].map(|a| a.ptr),
                    GL_POINT_SIZE_ARRAY_POINTER_OES => self.client.point_size.map(|a| a.ptr),
                    _ => None,
                }
                .unwrap_or(0);
                mem.write_u32(dst, ptr)?;
                Ok(0)
            }
            "GetTexEnvfv" | "GetTexEnviv" | "GetTexEnvxv" => {
                let (target, pname, ptr) = (a(0), a(1), a(2));
                if ptr == 0 || target != GL_TEXTURE_ENV {
                    return Ok(0);
                }
                let unit = self.active_texture;
                let int_out = name == "GetTexEnviv";
                let fixed = name == "GetTexEnvxv";
                match pname {
                    GL_TEXTURE_ENV_MODE => {
                        write_enum(mem, ptr, self.tex_env_mode[unit], int_out, fixed)?
                    }
                    GL_TEXTURE_ENV_COLOR => {
                        write_values(mem, ptr, &self.tex_env_color[unit], 4, fixed)?
                    }
                    GL_COMBINE_RGB => write_enum(mem, ptr, self.combine_rgb[unit], int_out, fixed)?,
                    GL_COMBINE_ALPHA => {
                        write_enum(mem, ptr, self.combine_alpha[unit], int_out, fixed)?
                    }
                    GL_RGB_SCALE => write_values(mem, ptr, &[self.rgb_scale[unit]], 1, fixed)?,
                    GL_ALPHA_SCALE => write_values(mem, ptr, &[self.alpha_scale[unit]], 1, fixed)?,
                    GL_SRC0_RGB => write_enum(mem, ptr, self.src_rgb[unit][0], int_out, fixed)?,
                    GL_SRC1_RGB => write_enum(mem, ptr, self.src_rgb[unit][1], int_out, fixed)?,
                    GL_SRC2_RGB => write_enum(mem, ptr, self.src_rgb[unit][2], int_out, fixed)?,
                    GL_SRC0_ALPHA => write_enum(mem, ptr, self.src_alpha[unit][0], int_out, fixed)?,
                    GL_SRC1_ALPHA => write_enum(mem, ptr, self.src_alpha[unit][1], int_out, fixed)?,
                    GL_SRC2_ALPHA => write_enum(mem, ptr, self.src_alpha[unit][2], int_out, fixed)?,
                    GL_OPERAND0_RGB => {
                        write_enum(mem, ptr, self.operand_rgb[unit][0], int_out, fixed)?
                    }
                    GL_OPERAND1_RGB => {
                        write_enum(mem, ptr, self.operand_rgb[unit][1], int_out, fixed)?
                    }
                    GL_OPERAND2_RGB => {
                        write_enum(mem, ptr, self.operand_rgb[unit][2], int_out, fixed)?
                    }
                    GL_OPERAND0_ALPHA => {
                        write_enum(mem, ptr, self.operand_alpha[unit][0], int_out, fixed)?
                    }
                    GL_OPERAND1_ALPHA => {
                        write_enum(mem, ptr, self.operand_alpha[unit][1], int_out, fixed)?
                    }
                    GL_OPERAND2_ALPHA => {
                        write_enum(mem, ptr, self.operand_alpha[unit][2], int_out, fixed)?
                    }
                    _ => {}
                }
                Ok(0)
            }
            "GetTexParameterfv" | "GetTexParameteriv" | "GetTexParameterxv" => {
                let (pname, ptr) = (a(1), a(2));
                if ptr == 0 {
                    return Ok(0);
                }
                let unit = self.active_texture;
                let Some(t) = self.textures.get(&self.bound_texture[unit]) else {
                    return Ok(0);
                };
                let v = match pname {
                    GL_TEXTURE_MIN_FILTER => t.min,
                    GL_TEXTURE_MAG_FILTER => t.mag,
                    GL_TEXTURE_WRAP_S => t.wrap_s,
                    GL_TEXTURE_WRAP_T => t.wrap_t,
                    GL_TEXTURE_MAX_ANISOTROPY_EXT => t.max_anisotropy as u32,
                    _ => return Ok(0),
                };
                let int_out = name == "GetTexParameteriv";
                let fixed = name == "GetTexParameterxv";
                write_enum(mem, ptr, v, int_out, fixed)?;
                Ok(0)
            }
            "GetLightfv" | "GetLightxv" => {
                let (light, pname, ptr) = (a(0), a(1), a(2));
                let index = light.wrapping_sub(GL_LIGHT0) as usize;
                let Some(l) = self.lights.get(index) else {
                    return Ok(0);
                };
                let fixed = name.ends_with('x');
                let (values, count): (&[f32], u32) = match pname {
                    GL_AMBIENT => (&l.ambient[..], 4),
                    GL_DIFFUSE => (&l.diffuse[..], 4),
                    GL_SPECULAR => (&l.specular[..], 4),
                    GL_POSITION => (&l.position[..], 4),
                    GL_SPOT_DIRECTION => (&l.spot_direction[..], 3),
                    GL_SPOT_EXPONENT => (&[l.spot_exponent][..], 1),
                    GL_SPOT_CUTOFF => (&[l.spot_cutoff][..], 1),
                    GL_CONSTANT_ATTENUATION => (&[l.constant_attenuation][..], 1),
                    GL_LINEAR_ATTENUATION => (&[l.linear_attenuation][..], 1),
                    GL_QUADRATIC_ATTENUATION => (&[l.quadratic_attenuation][..], 1),
                    _ => return Ok(0),
                };
                write_values(mem, ptr, values, count, fixed)?;
                Ok(0)
            }
            "GetMaterialfv" | "GetMaterialxv" => {
                let (face, pname, ptr) = (a(0), a(1), a(2));
                let fixed = name.ends_with('x');
                let Some(m) = self.material_for_face(face) else {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                };
                let (values, count): (&[f32], u32) = match pname {
                    GL_AMBIENT | GL_AMBIENT_AND_DIFFUSE => (&m.ambient[..], 4),
                    GL_DIFFUSE => (&m.diffuse[..], 4),
                    GL_SPECULAR => (&m.specular[..], 4),
                    GL_EMISSION => (&m.emission[..], 4),
                    GL_SHININESS => (&[m.shininess][..], 1),
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                };
                write_values(mem, ptr, values, count, fixed)?;
                Ok(0)
            }
            "PointParameterf" | "PointParameterx" => {
                let pname = a(0);
                let value = if name.ends_with('x') { fx(1) } else { f(1) };
                match pname {
                    GL_POINT_SIZE_MIN => self.point_size_min = value.max(0.0),
                    GL_POINT_SIZE_MAX => self.point_size_max = value.max(0.0),
                    GL_POINT_FADE_THRESHOLD_SIZE => self.point_fade_threshold = value.max(0.0),
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                }
                Ok(0)
            }
            "PointParameterfv" | "PointParameterxv" => {
                let (pname, ptr) = (a(0), a(1));
                if ptr == 0 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let ty = if name.contains('x') {
                    GL_FIXED
                } else {
                    GL_FLOAT
                };
                match pname {
                    GL_POINT_SIZE_MIN => {
                        self.point_size_min = read_component(mem, ptr, ty)?.max(0.0)
                    }
                    GL_POINT_SIZE_MAX => {
                        self.point_size_max = read_component(mem, ptr, ty)?.max(0.0)
                    }
                    GL_POINT_FADE_THRESHOLD_SIZE => {
                        self.point_fade_threshold = read_component(mem, ptr, ty)?.max(0.0)
                    }
                    GL_POINT_DISTANCE_ATTENUATION => {
                        for i in 0..3 {
                            self.point_distance_attenuation[i] =
                                read_component(mem, ptr + i as u32 * 4, ty)?;
                        }
                    }
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                }
                Ok(0)
            }
            "IsEnabled" => {
                // An unknown enum stays a silent FALSE, as it was before the two tables were
                // merged: adding `set_error(GL_INVALID_ENUM)` here would make caps this host does
                // not model suddenly raise a GL error in code that checks `glGetError`.
                let cap = a(0);
                let on = if is_switchable_cap(cap) {
                    enabled_state(self, cap).unwrap_or(false)
                } else {
                    false
                };
                // `int IsEnabled(IGLES11*, AEEGLenum cap, AEEGLboolean *ret)`:
                // the answer goes out through the pointer and the return is an error code — the
                // same shape as `IMedia::GetState`, and the same mistake was here. This host
                // returned the answer, so the caller got `1` as its error code and its own `ret`
                // still held whatever the stack held. `AEEGLboolean` is `unsigned char`:
                //one byte, `write_u8`, never a word.
                let out = a(1);
                if out != 0 {
                    mem.write_u8(out, if on { 1 } else { 0 })?;
                }
                Ok(0)
            }
            "IsTexture" => {
                let out = a(1);
                if out != 0 {
                    mem.write_u8(out, u8::from(self.textures.contains_key(&a(0))))?;
                }
                Ok(0)
            }
            "Finish" | "Flush" => Ok(0),
            "ReadPixels" => {
                let (x, y, width, height, format, ty, pixels) = (
                    a(0) as i32,
                    a(1) as i32,
                    a(2) as i32,
                    a(3) as i32,
                    a(4),
                    a(5),
                    a(6),
                );
                if width < 0
                    || height < 0
                    || width > MAX_TEXTURE_SIZE as i32
                    || height > MAX_TEXTURE_SIZE as i32
                {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width == 0 || height == 0 {
                    return Ok(0);
                }
                if pixels == 0 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let Some(s) = self.read_surface(&*egl) else {
                    return Ok(0);
                };
                let align = self.pack_alignment.max(1);
                if ty == GL_UNSIGNED_SHORT_5_6_5 {
                    if format != GL_RGB {
                        return Ok(crate::abi::generated::AEE_EUNSUPPORTED);
                    }
                    let row_bytes = (width as u32 * 2 + align - 1) / align * align;
                    for row in 0..height {
                        let sy = y + row;
                        if sy < 0 || sy >= s.height as i32 {
                            continue;
                        }
                        for col in 0..width {
                            let sx = x + col;
                            if sx < 0 || sx >= s.width as i32 {
                                continue;
                            }
                            let i = ((sy as u32 * s.width + sx as u32) * 4) as usize;
                            let v = pack565(s.rgba[i], s.rgba[i + 1], s.rgba[i + 2]);
                            let dst = pixels + row as u32 * row_bytes + col as u32 * 2;
                            mem.write_u16(dst, v as u32)?;
                        }
                    }
                    return Ok(0);
                }
                if ty == GL_UNSIGNED_SHORT_4_4_4_4 || ty == GL_UNSIGNED_SHORT_5_5_5_1 {
                    if format != GL_RGBA {
                        return Ok(crate::abi::generated::AEE_EUNSUPPORTED);
                    }
                    let row_bytes = (width as u32 * 2 + align - 1) / align * align;
                    for row in 0..height {
                        let sy = y + row;
                        if sy < 0 || sy >= s.height as i32 {
                            continue;
                        }
                        for col in 0..width {
                            let sx = x + col;
                            if sx < 0 || sx >= s.width as i32 {
                                continue;
                            }
                            let i = ((sy as u32 * s.width + sx as u32) * 4) as usize;
                            let (r, g, b, a) = (
                                s.rgba[i] as u32,
                                s.rgba[i + 1] as u32,
                                s.rgba[i + 2] as u32,
                                s.rgba[i + 3] as u32,
                            );
                            let v = if ty == GL_UNSIGNED_SHORT_4_4_4_4 {
                                ((r >> 4) << 12) | ((g >> 4) << 8) | ((b >> 4) << 4) | (a >> 4)
                            } else {
                                ((r >> 3) << 11) | ((g >> 3) << 6) | ((b >> 3) << 1) | (a >> 7)
                            };
                            mem.write_u16(pixels + row as u32 * row_bytes + col as u32 * 2, v)?;
                        }
                    }
                    return Ok(0);
                }
                if ty == GL_UNSIGNED_SHORT {
                    let bpp = match format {
                        GL_RGBA => 4usize,
                        GL_RGB => 3usize,
                        GL_LUMINANCE | GL_ALPHA => 1usize,
                        GL_LUMINANCE_ALPHA => 2usize,
                        _ => return Ok(crate::abi::generated::AEE_EUNSUPPORTED),
                    };
                    let row_bytes = (width as u32 * bpp as u32 * 2 + align - 1) / align * align;
                    for row in 0..height {
                        let sy = y + row;
                        if sy < 0 || sy >= s.height as i32 {
                            continue;
                        }
                        for col in 0..width {
                            let sx = x + col;
                            if sx < 0 || sx >= s.width as i32 {
                                continue;
                            }
                            let i = ((sy as u32 * s.width + sx as u32) * 4) as usize;
                            let px = match format {
                                GL_RGBA => [s.rgba[i], s.rgba[i + 1], s.rgba[i + 2], s.rgba[i + 3]],
                                GL_RGB => [s.rgba[i], s.rgba[i + 1], s.rgba[i + 2], 0],
                                GL_LUMINANCE => [s.rgba[i], 0, 0, 0],
                                GL_ALPHA => [s.rgba[i + 3], 0, 0, 0],
                                GL_LUMINANCE_ALPHA => [s.rgba[i], s.rgba[i + 3], 0, 0],
                                _ => [0; 4],
                            };
                            let dst = pixels + row as u32 * row_bytes + col as u32 * bpp as u32 * 2;
                            for ch in 0..bpp {
                                mem.write_u16(dst + ch as u32 * 2, px[ch] as u32 * 257)?;
                            }
                        }
                    }
                    return Ok(0);
                }
                if ty != GL_UNSIGNED_BYTE || pixels == 0 {
                    return Ok(crate::abi::generated::AEE_EUNSUPPORTED);
                }
                let bpp = match format {
                    GL_RGBA => 4usize,
                    GL_RGB => 3usize,
                    GL_LUMINANCE | GL_ALPHA => 1usize,
                    GL_LUMINANCE_ALPHA => 2usize,
                    _ => return Ok(crate::abi::generated::AEE_EUNSUPPORTED),
                };
                let row_bytes = (width as u32 * bpp as u32 + align - 1) / align * align;
                for row in 0..height {
                    let sy = y + row;
                    if sy < 0 || sy >= s.height as i32 {
                        continue;
                    }
                    for col in 0..width {
                        let sx = x + col;
                        if sx < 0 || sx >= s.width as i32 {
                            continue;
                        }
                        let i = ((sy as u32 * s.width + sx as u32) * 4) as usize;
                        // `glReadPixels` writes the bottom row first, the same order the drawable
                        // is kept in; no vertical flip belongs here.
                        let dst = pixels + row as u32 * row_bytes + col as u32 * bpp as u32;
                        let px = match format {
                            GL_RGBA => [s.rgba[i], s.rgba[i + 1], s.rgba[i + 2], s.rgba[i + 3]],
                            GL_RGB => [s.rgba[i], s.rgba[i + 1], s.rgba[i + 2], 0],
                            GL_LUMINANCE => [s.rgba[i], 0, 0, 0],
                            GL_ALPHA => [s.rgba[i + 3], 0, 0, 0],
                            GL_LUMINANCE_ALPHA => [s.rgba[i], s.rgba[i + 3], 0, 0],
                            _ => [0; 4],
                        };
                        mem.write_bytes(dst, &px[..bpp])?;
                    }
                }
                Ok(0)
            }
            // Lighting, fog, stencil and multitexture are accepted where the reference layer
            // forwards them, but they do not change a value this pipeline reads yet. The slots
            // answer success so a pack's initialisation does not stop on them.
            "ActiveTexture" => {
                self.active_texture = (a(0).wrapping_sub(GL_TEXTURE0) as usize).min(3);
                Ok(0)
            }
            "ClientActiveTexture" => {
                self.client_active_texture = (a(0).wrapping_sub(GL_TEXTURE0) as usize).min(3);
                Ok(0)
            }
            "ClearStencil" => {
                self.clear_stencil = a(0) as i32;
                Ok(0)
            }
            "StencilFunc" => {
                let func = a(0);
                if !is_compare_func(func) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.stencil_func = func;
                self.stencil_ref = a(1) as i32;
                self.stencil_value_mask = a(2);
                Ok(0)
            }
            "StencilMask" => {
                self.stencil_write_mask = a(0);
                Ok(0)
            }
            "StencilOp" => {
                let (fail, pass_depth_fail, pass_depth_pass) = (a(0), a(1), a(2));
                if !is_stencil_op(fail)
                    || !is_stencil_op(pass_depth_fail)
                    || !is_stencil_op(pass_depth_pass)
                {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.stencil_fail = fail;
                self.stencil_pass_depth_fail = pass_depth_fail;
                self.stencil_pass_depth_pass = pass_depth_pass;
                Ok(0)
            }
            "LogicOp" => {
                if !(GL_CLEAR..=GL_SET).contains(&a(0)) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.logic_op = a(0);
                Ok(0)
            }
            "Fogf" | "Fogx" => {
                let pname = a(0);
                let v = if name.contains('x') { fx(1) } else { f(1) };
                match pname {
                    GL_FOG_MODE => self.fog_mode = a(1),
                    GL_FOG_DENSITY => self.fog_density = v,
                    GL_FOG_START => self.fog_start = v,
                    GL_FOG_END => self.fog_end = v,
                    _ => {}
                }
                Ok(0)
            }
            "Fogfv" | "Fogxv" => {
                let (pname, ptr) = (a(0), a(1));
                if pname == GL_FOG_COLOR && ptr != 0 {
                    let ty = if name.contains('x') {
                        GL_FIXED
                    } else {
                        GL_FLOAT
                    };
                    for i in 0..4 {
                        self.fog_color[i] = read_component(mem, ptr + i as u32 * 4, ty)?;
                    }
                }
                Ok(0)
            }
            "Lightf" | "Lightx" => {
                let (light, pname) = (a(0), a(1));
                let v = if name.contains('x') { fx(2) } else { f(2) };
                if let Some(l) = self.light_mut(light) {
                    match pname {
                        GL_SPOT_EXPONENT => l.spot_exponent = v,
                        GL_SPOT_CUTOFF => l.spot_cutoff = v,
                        GL_CONSTANT_ATTENUATION => l.constant_attenuation = v,
                        GL_LINEAR_ATTENUATION => l.linear_attenuation = v,
                        GL_QUADRATIC_ATTENUATION => l.quadratic_attenuation = v,
                        _ => {}
                    }
                }
                Ok(0)
            }
            "Lightfv" | "Lightxv" => {
                let (light, pname, ptr) = (a(0), a(1), a(2));
                if ptr == 0 {
                    return Ok(0);
                }
                let ty = if name.contains('x') {
                    GL_FIXED
                } else {
                    GL_FLOAT
                };
                let n = if pname == GL_SPOT_DIRECTION { 3 } else { 4 };
                let mut v = [0.0f32; 4];
                for (i, slot) in v.iter_mut().enumerate().take(n) {
                    *slot = read_component(mem, ptr + i as u32 * 4, ty)?;
                }
                // A light is stored in eye space: the position is a point the modelview
                // matrix moves, and the spot direction is a vector it rotates.
                let position = self.matrices[0].current * Vec4::new(v[0], v[1], v[2], v[3]);
                let spot = self.matrices[0]
                    .current
                    .transform_vector3(Vec3::new(v[0], v[1], v[2]));
                if let Some(l) = self.light_mut(light) {
                    match pname {
                        GL_AMBIENT => l.ambient = v,
                        GL_DIFFUSE => l.diffuse = v,
                        GL_SPECULAR => l.specular = v,
                        GL_POSITION => {
                            l.position = [position.x, position.y, position.z, position.w]
                        }
                        GL_SPOT_DIRECTION => l.spot_direction = [spot.x, spot.y, spot.z],
                        _ => {}
                    }
                }
                Ok(0)
            }
            "LightModelf" | "LightModelx" => {
                let pname = a(0);
                let value = if name.ends_with('x') { fx(1) } else { f(1) };
                match pname {
                    GL_LIGHT_MODEL_TWO_SIDE => self.two_sided = value != 0.0,
                    GL_LIGHT_MODEL_LOCAL_VIEWER => self.local_viewer = value != 0.0,
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                }
                Ok(0)
            }
            "LightModelfv" | "LightModelxv" => {
                let (pname, ptr) = (a(0), a(1));
                if ptr == 0 {
                    return Ok(0);
                }
                let ty = if name.contains('x') {
                    GL_FIXED
                } else {
                    GL_FLOAT
                };
                match pname {
                    GL_LIGHT_MODEL_AMBIENT => {
                        for i in 0..4 {
                            self.light_model_ambient[i] =
                                read_component(mem, ptr + i as u32 * 4, ty)?;
                        }
                    }
                    GL_LIGHT_MODEL_TWO_SIDE => {
                        let value = read_component(mem, ptr, ty)?;
                        self.two_sided = value != 0.0;
                    }
                    GL_LIGHT_MODEL_LOCAL_VIEWER => {
                        self.local_viewer = read_component(mem, ptr, ty)? != 0.0;
                    }
                    _ => {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                }
                Ok(0)
            }
            "ColorMaterial" => {
                let (face, mode) = (a(0), a(1));
                if !matches!(face, GL_FRONT | GL_BACK | GL_FRONT_AND_BACK) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !matches!(
                    mode,
                    GL_AMBIENT | GL_DIFFUSE | GL_SPECULAR | GL_EMISSION | GL_AMBIENT_AND_DIFFUSE
                ) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.color_material_face = face;
                self.color_material_mode = mode;
                Ok(0)
            }
            "Materialf" | "Materialx" => {
                let face = a(0);
                if a(1) != GL_SHININESS {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let value = if name.contains('x') { fx(2) } else { f(2) };
                if !self.set_material(face, |m| m.shininess = value) {
                    self.set_error(GL_INVALID_ENUM);
                }
                Ok(0)
            }
            "Materialfv" | "Materialxv" => {
                let (face, pname, ptr) = (a(0), a(1), a(2));
                if ptr == 0 {
                    return Ok(0);
                }
                let ty = if name.contains('x') {
                    GL_FIXED
                } else {
                    GL_FLOAT
                };
                let mut v = [0.0f32; 4];
                for (i, slot) in v.iter_mut().enumerate() {
                    *slot = read_component(mem, ptr + i as u32 * 4, ty)?;
                }
                let known = matches!(
                    pname,
                    GL_AMBIENT
                        | GL_AMBIENT_AND_DIFFUSE
                        | GL_DIFFUSE
                        | GL_SPECULAR
                        | GL_EMISSION
                        | GL_SHININESS
                );
                if !known {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let updated = self.set_material(face, |m| match pname {
                    GL_AMBIENT => m.ambient = v,
                    GL_AMBIENT_AND_DIFFUSE => {
                        m.ambient = v;
                        m.diffuse = v;
                    }
                    GL_DIFFUSE => m.diffuse = v,
                    GL_SPECULAR => m.specular = v,
                    GL_EMISSION => m.emission = v,
                    GL_SHININESS => m.shininess = v[0],
                    _ => {}
                });
                if !updated {
                    self.set_error(GL_INVALID_ENUM);
                }
                Ok(0)
            }
            "TexEnvfv" | "TexEnvxv" => {
                let (pname, ptr) = (a(1), a(2));
                if pname == GL_TEXTURE_ENV_COLOR && ptr != 0 {
                    let ty = if name.contains('x') {
                        GL_FIXED
                    } else {
                        GL_FLOAT
                    };
                    for i in 0..4 {
                        self.tex_env_color[self.active_texture][i] =
                            read_component(mem, ptr + i as u32 * 4, ty)?;
                    }
                }
                Ok(0)
            }
            "TexParameterfv" | "TexParameterxv" => {
                let (pname, ptr) = (a(1), a(2));
                if ptr == 0 {
                    return Ok(0);
                }
                // The fixed entry point carries the enum itself, not a 16.16 value: several
                // wrap/filter enums are larger than a signed 16.16 number can hold.
                let value = if name == "TexParameterxv" {
                    mem.read_u32(ptr)?
                } else {
                    let value = f32::from_bits(mem.read_u32(ptr)?);
                    if !value.is_finite() || value < 0.0 {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                    value as u32
                };
                self.set_tex_parameter(pname, value);
                Ok(0)
            }
            "CompressedTexImage2D" => {
                let level = a(1);
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let (internal, width, height) = (a(2), a(3), a(4));
                let (size, data) = (a(6), a(7));
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width == 0 || height == 0 || data == 0 {
                    return Ok(0);
                }
                let bytes = mem.read_bytes(data, size as usize)?;
                let rgba = if (GL_PALETTE4_RGB8_OES..=GL_PALETTE8_RGB5_A1_OES).contains(&internal) {
                    let Some(decoded) = decode_paletted(&bytes, width, height, internal) else {
                        return Ok(0);
                    };
                    decoded
                } else {
                    let mut pixels = vec![0u32; (width * height) as usize];
                    let decoded = match internal {
                        GL_ATC_RGB_AMD => texture2ddecoder::decode_atc_rgb4(
                            &bytes,
                            width as usize,
                            height as usize,
                            &mut pixels,
                        ),
                        GL_ATC_RGBA_EXPLICIT_ALPHA_AMD | GL_ATC_RGBA_INTERPOLATED_ALPHA_AMD => {
                            texture2ddecoder::decode_atc_rgba8(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        GL_COMPRESSED_RGB_PVRTC_4BPPV1_IMG
                        | GL_COMPRESSED_RGBA_PVRTC_4BPPV1_IMG => {
                            texture2ddecoder::decode_pvrtc_4bpp(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        GL_COMPRESSED_RGB_PVRTC_2BPPV1_IMG
                        | GL_COMPRESSED_RGBA_PVRTC_2BPPV1_IMG => {
                            texture2ddecoder::decode_pvrtc_2bpp(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        GL_ETC1_RGB8_OES => texture2ddecoder::decode_etc1(
                            &bytes,
                            width as usize,
                            height as usize,
                            &mut pixels,
                        ),
                        _ => return Ok(crate::abi::generated::AEE_EUNSUPPORTED),
                    };
                    if decoded.is_err() {
                        return Ok(0);
                    }
                    // The decoder writes `0xAARRGGBB`; the pipeline keeps RGBA8.
                    let mut rgba = vec![255u8; (width * height * 4) as usize];
                    for (i, v) in pixels.iter().enumerate() {
                        rgba[i * 4] = ((v >> 16) & 0xff) as u8;
                        rgba[i * 4 + 1] = ((v >> 8) & 0xff) as u8;
                        rgba[i * 4 + 2] = (v & 0xff) as u8;
                        rgba[i * 4 + 3] = ((v >> 24) & 0xff) as u8;
                    }
                    rgba
                };
                let unit = self.active_texture;
                let id = if self.bound_texture[unit] == 0 {
                    let id = self.next_texture;
                    self.next_texture += 1;
                    id
                } else {
                    self.bound_texture[unit]
                };
                let mut tex = self
                    .textures
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(Texture::white);
                if level == 0 {
                    tex.width = width;
                    tex.height = height;
                    tex.rgba = rgba;
                    tex.levels.clear();
                } else {
                    if tex.levels.len() < level as usize {
                        tex.levels.resize(level as usize, Vec::new());
                    }
                    tex.levels[level as usize - 1] = rgba;
                }
                self.textures.insert(id, tex);
                Ok(0)
            }
            "CompressedTexSubImage2D" => {
                let level = a(1);
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let (xoffset, yoffset) = (a(2), a(3));
                let (width, height, format) = (a(4), a(5), a(6));
                let (size, data) = (a(7), a(8));
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width == 0 || height == 0 || data == 0 {
                    return Ok(0);
                }
                let bytes = mem.read_bytes(data, size as usize)?;
                let rgba = if (GL_PALETTE4_RGB8_OES..=GL_PALETTE8_RGB5_A1_OES).contains(&format) {
                    let Some(decoded) = decode_paletted(&bytes, width, height, format) else {
                        return Ok(0);
                    };
                    decoded
                } else {
                    let mut pixels = vec![0u32; (width * height) as usize];
                    let decoded = match format {
                        GL_ATC_RGB_AMD => texture2ddecoder::decode_atc_rgb4(
                            &bytes,
                            width as usize,
                            height as usize,
                            &mut pixels,
                        ),
                        GL_ATC_RGBA_EXPLICIT_ALPHA_AMD | GL_ATC_RGBA_INTERPOLATED_ALPHA_AMD => {
                            texture2ddecoder::decode_atc_rgba8(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        GL_ETC1_RGB8_OES => texture2ddecoder::decode_etc1(
                            &bytes,
                            width as usize,
                            height as usize,
                            &mut pixels,
                        ),
                        GL_COMPRESSED_RGB_PVRTC_4BPPV1_IMG
                        | GL_COMPRESSED_RGBA_PVRTC_4BPPV1_IMG => {
                            texture2ddecoder::decode_pvrtc_4bpp(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        GL_COMPRESSED_RGB_PVRTC_2BPPV1_IMG
                        | GL_COMPRESSED_RGBA_PVRTC_2BPPV1_IMG => {
                            texture2ddecoder::decode_pvrtc_2bpp(
                                &bytes,
                                width as usize,
                                height as usize,
                                &mut pixels,
                            )
                        }
                        _ => return Ok(crate::abi::generated::AEE_EUNSUPPORTED),
                    };
                    if decoded.is_err() {
                        return Ok(0);
                    }
                    let mut rgba = vec![255u8; (width * height * 4) as usize];
                    for (i, v) in pixels.iter().enumerate() {
                        rgba[i * 4] = ((v >> 16) & 0xff) as u8;
                        rgba[i * 4 + 1] = ((v >> 8) & 0xff) as u8;
                        rgba[i * 4 + 2] = (v & 0xff) as u8;
                        rgba[i * 4 + 3] = ((v >> 24) & 0xff) as u8;
                    }
                    rgba
                };
                let unit = self.active_texture;
                let Some(tex) = self.textures.get_mut(&self.bound_texture[unit]) else {
                    return Ok(0);
                };
                let Some((target_w, target_h, target)) = tex.level_storage(level) else {
                    return Ok(0);
                };
                for row in 0..height {
                    let ty = yoffset.saturating_add(row);
                    if ty >= target_h {
                        continue;
                    }
                    for col in 0..width {
                        let tx = xoffset.saturating_add(col);
                        if tx >= target_w {
                            continue;
                        }
                        let si = ((row * width + col) * 4) as usize;
                        let di = ((ty * target_w + tx) * 4) as usize;
                        target[di..di + 4].copy_from_slice(&rgba[si..si + 4]);
                    }
                }
                Ok(0)
            }
            "CopyTexImage2D" => {
                let target = a(0);
                if target != GL_TEXTURE_2D && cube_face(target).is_none() {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let level = a(1);
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let (x, y, width, height) = (a(3) as i32, a(4) as i32, a(5), a(6));
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width == 0 || height == 0 {
                    return Ok(0);
                }
                let Some(s) = self.read_surface(&*egl) else {
                    return Ok(0);
                };
                let mut rgba = vec![255u8; (width * height * 4) as usize];
                for row in 0..height {
                    let sy = y.saturating_add(row as i32);
                    if sy < 0 || sy >= s.height as i32 {
                        continue;
                    }
                    for col in 0..width {
                        let sx = x.saturating_add(col as i32);
                        if sx < 0 || sx >= s.width as i32 {
                            continue;
                        }
                        let si = ((sy as u32 * s.width + sx as u32) * 4) as usize;
                        let di = ((row * width + col) * 4) as usize;
                        rgba[di..di + 4].copy_from_slice(&s.rgba[si..si + 4]);
                    }
                }
                let unit = self.active_texture;
                let id = if self.bound_texture[unit] == 0 {
                    let id = self.next_texture;
                    self.next_texture += 1;
                    id
                } else {
                    self.bound_texture[unit]
                };
                let mut tex = self
                    .textures
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(Texture::white);
                if let Some(face) = cube_face(target) {
                    if level == 0 {
                        tex.width = width;
                        tex.height = height;
                        let count = (width * height * 4) as usize;
                        tex.faces = vec![vec![0u8; count]; 6];
                        tex.faces[face] = rgba.clone();
                        if face == 0 {
                            tex.rgba = rgba;
                        }
                        tex.levels.clear();
                        tex.cube_levels.clear();
                    } else {
                        if tex.faces.len() < 6 {
                            self.set_error(GL_INVALID_OPERATION);
                            return Ok(0);
                        }
                        if width != (tex.width >> level).max(1)
                            || height != (tex.height >> level).max(1)
                        {
                            self.set_error(GL_INVALID_VALUE);
                            return Ok(0);
                        }
                        let index = level as usize - 1;
                        if tex.cube_levels.len() <= index {
                            tex.cube_levels.resize(index + 1, Vec::new());
                        }
                        let faces = &mut tex.cube_levels[index];
                        if faces.len() < 6 {
                            faces.resize(6, vec![0u8; (width * height * 4) as usize]);
                        }
                        faces[face] = rgba;
                    }
                } else if level == 0 {
                    tex.width = width;
                    tex.height = height;
                    tex.rgba = rgba;
                    tex.levels.clear();
                } else {
                    if tex.levels.len() < level as usize {
                        tex.levels.resize(level as usize, Vec::new());
                    }
                    tex.levels[level as usize - 1] = rgba;
                }
                self.textures.insert(id, tex);
                Ok(0)
            }
            "CopyTexSubImage2D" => {
                let target = a(0);
                if target != GL_TEXTURE_2D && cube_face(target).is_none() {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let level = a(1);
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let (xoffset, yoffset) = (a(2), a(3));
                let (x, y, width, height) = (a(4) as i32, a(5) as i32, a(6), a(7));
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let Some(s) = self.read_surface(&*egl) else {
                    return Ok(0);
                };
                let (sw, sh, src_rgba) = (s.width, s.height, s.rgba.clone());
                let unit = self.active_texture;
                let Some(tex) = self.textures.get_mut(&self.bound_texture[unit]) else {
                    return Ok(0);
                };
                let Some((target_w, target_h, target)) = tex.face_storage(target, level) else {
                    return Ok(0);
                };
                for row in 0..height {
                    let sy = y.saturating_add(row as i32);
                    if sy < 0 || sy >= sh as i32 {
                        continue;
                    }
                    let ty = yoffset.saturating_add(row);
                    if ty >= target_h {
                        continue;
                    }
                    for col in 0..width {
                        let sx = x.saturating_add(col as i32);
                        if sx < 0 || sx >= sw as i32 {
                            continue;
                        }
                        let tx = xoffset.saturating_add(col);
                        if tx >= target_w {
                            continue;
                        }
                        let si = ((sy as u32 * sw + sx as u32) * 4) as usize;
                        let di = ((ty * target_w + tx) * 4) as usize;
                        target[di..di + 4].copy_from_slice(&src_rgba[si..si + 4]);
                    }
                }
                Ok(0)
            }
            "Clear" => {
                self.clear(a(0), egl, mem)?;
                Ok(0)
            }

            // ---- matrices ----
            "MatrixMode" => {
                self.mode = match a(0) {
                    GL_MODELVIEW => 0,
                    GL_PROJECTION => 1,
                    GL_TEXTURE => 2,
                    _ => 0,
                };
                Ok(0)
            }
            "LoadIdentity" => {
                self.matrices[self.mode].current = Mat4::IDENTITY;
                Ok(0)
            }
            "LoadMatrixf" | "LoadMatrixx" => {
                let m = self.read_matrix(a(0), name.ends_with('x'), mem)?;
                self.matrices[self.mode].current = m;
                Ok(0)
            }
            "MultMatrixf" | "MultMatrixx" => {
                let m = self.read_matrix(a(0), name.ends_with('x'), mem)?;
                self.matrices[self.mode].current *= m;
                Ok(0)
            }
            "PushMatrix" => {
                if self.matrices[self.mode].stack.len() >= 32 {
                    self.set_error(GL_STACK_OVERFLOW);
                    return Ok(0);
                }
                let s = &mut self.matrices[self.mode];
                s.stack.push(s.current);
                Ok(0)
            }
            "PopMatrix" => {
                if self.matrices[self.mode].stack.is_empty() {
                    self.set_error(GL_STACK_UNDERFLOW);
                    return Ok(0);
                }
                let s = &mut self.matrices[self.mode];
                if let Some(m) = s.stack.pop() {
                    s.current = m;
                }
                Ok(0)
            }
            "Translatef" | "Translatex" => {
                let v = if name.ends_with('x') {
                    Vec3::new(fx(0), fx(1), fx(2))
                } else {
                    Vec3::new(f(0), f(1), f(2))
                };
                self.matrices[self.mode].current *= Mat4::from_translation(v);
                Ok(0)
            }
            "Scalef" | "Scalex" => {
                let v = if name.ends_with('x') {
                    Vec3::new(fx(0), fx(1), fx(2))
                } else {
                    Vec3::new(f(0), f(1), f(2))
                };
                self.matrices[self.mode].current *= Mat4::from_scale(v);
                Ok(0)
            }
            "Rotatef" | "Rotatex" => {
                let (angle, axis) = if name.ends_with('x') {
                    (fx(0).to_radians(), Vec3::new(fx(1), fx(2), fx(3)))
                } else {
                    (f(0).to_radians(), Vec3::new(f(1), f(2), f(3)))
                };
                if axis.length_squared() > 0.0 {
                    self.matrices[self.mode].current *=
                        Mat4::from_axis_angle(axis.normalize(), angle);
                }
                Ok(0)
            }
            "Frustumf" | "Frustumx" => {
                let v = if name.ends_with('x') {
                    [fx(0), fx(1), fx(2), fx(3), fx(4), fx(5)]
                } else {
                    [f(0), f(1), f(2), f(3), f(4), f(5)]
                };
                let m = frustum(v[0], v[1], v[2], v[3], v[4], v[5]);
                self.matrices[self.mode].current *= m;
                Ok(0)
            }
            "Orthof" | "Orthox" => {
                let v = if name.ends_with('x') {
                    [fx(0), fx(1), fx(2), fx(3), fx(4), fx(5)]
                } else {
                    [f(0), f(1), f(2), f(3), f(4), f(5)]
                };
                let m = ortho(v[0], v[1], v[2], v[3], v[4], v[5]);
                self.matrices[self.mode].current *= m;
                Ok(0)
            }

            // ---- immediate mode ----
            "Begin" => {
                let mode = a(0);
                if !is_primitive(mode) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                self.primitive = mode;
                self.vertices.clear();
                Ok(0)
            }
            "End" => {
                let vertices = std::mem::take(&mut self.vertices);
                self.submit(&vertices, self.primitive, egl, mem)?;
                self.primitive = 0;
                Ok(0)
            }
            "Vertex2f" | "Vertex2x" => {
                let v = if name.ends_with('x') {
                    Vec3::new(fx(0), fx(1), 0.0)
                } else {
                    Vec3::new(f(0), f(1), 0.0)
                };
                self.push_vertex(v, 1.0);
                Ok(0)
            }
            "Vertex3f" | "Vertex3x" => {
                let v = if name.ends_with('x') {
                    Vec3::new(fx(0), fx(1), fx(2))
                } else {
                    Vec3::new(f(0), f(1), f(2))
                };
                self.push_vertex(v, 1.0);
                Ok(0)
            }
            "Vertex4f" | "Vertex4x" => {
                let (v, w) = if name.ends_with('x') {
                    (Vec3::new(fx(0), fx(1), fx(2)), fx(3))
                } else {
                    (Vec3::new(f(0), f(1), f(2)), f(3))
                };
                self.push_vertex(v, w);
                Ok(0)
            }
            "TexCoord2f" | "TexCoord2x" => {
                self.current.tex[0] = if name.ends_with('x') {
                    [fx(0), fx(1)]
                } else {
                    [f(0), f(1)]
                };
                self.current.tex_r[0] = 0.0;
                Ok(0)
            }
            "TexCoord3f" | "TexCoord3x" => {
                self.current.tex[0] = if name.ends_with('x') {
                    [fx(0), fx(1)]
                } else {
                    [f(0), f(1)]
                };
                self.current.tex_r[0] = if name.ends_with('x') { fx(2) } else { f(2) };
                Ok(0)
            }

            // ---- textures ----
            "GenTextures" => {
                let (n, ptr) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = self.next_texture;
                    self.next_texture += 1;
                    self.textures.insert(id, Texture::white());
                    if ptr != 0 {
                        mem.write_u32(ptr + i * 4, id)?;
                    }
                }
                Ok(0)
            }
            "DeleteTextures" => {
                let (n, ptr) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    if ptr != 0 {
                        let id = mem.read_u32(ptr + i * 4)?;
                        self.textures.remove(&id);
                    }
                }
                Ok(0)
            }
            "BindTexture" => {
                if !matches!(a(0), GL_TEXTURE_2D | GL_TEXTURE_CUBE_MAP) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let unit = self.active_texture;
                self.bound_texture[unit] = a(1);
                self.bound_target[unit] = a(0);
                if self.bound_texture[unit] != 0
                    && !self.textures.contains_key(&self.bound_texture[unit])
                {
                    self.textures
                        .insert(self.bound_texture[unit], Texture::white());
                }
                Ok(0)
            }
            "TexParameterf" | "TexParameterx" | "TexParameteri" => {
                let (pname, param) = (a(1), a(2));
                let v = if pname == GL_TEXTURE_MAX_ANISOTROPY_EXT {
                    if name == "TexParameterf" {
                        f(2).clamp(1.0, 16.0) as u32
                    } else if name == "TexParameterx" {
                        fx(2).clamp(1.0, 16.0) as u32
                    } else {
                        param
                    }
                } else if name == "TexParameterx" {
                    a(2)
                } else {
                    param
                };
                self.set_tex_parameter(pname, v);
                Ok(0)
            }
            "TexEnvf" | "TexEnvi" | "TexEnvx" => {
                if a(0) == GL_POINT_SPRITE_OES {
                    if a(1) == GL_COORD_REPLACE_OES {
                        self.point_sprite_coord_replace = if name.ends_with('f') {
                            f(2) != 0.0
                        } else {
                            a(2) != 0
                        };
                    }
                    return Ok(0);
                }
                let (pname, value) = (a(1), a(2));
                let unit = self.active_texture;
                let scalar = if name.contains('x') { fx(2) } else { f(2) };
                match pname {
                    GL_TEXTURE_ENV_MODE => self.tex_env_mode[unit] = value,
                    GL_COMBINE_RGB => self.combine_rgb[unit] = value,
                    GL_COMBINE_ALPHA => self.combine_alpha[unit] = value,
                    GL_RGB_SCALE => self.rgb_scale[unit] = scalar,
                    GL_ALPHA_SCALE => self.alpha_scale[unit] = scalar,
                    GL_SRC0_RGB => self.src_rgb[unit][0] = value,
                    GL_SRC1_RGB => self.src_rgb[unit][1] = value,
                    GL_SRC2_RGB => self.src_rgb[unit][2] = value,
                    GL_SRC0_ALPHA => self.src_alpha[unit][0] = value,
                    GL_SRC1_ALPHA => self.src_alpha[unit][1] = value,
                    GL_SRC2_ALPHA => self.src_alpha[unit][2] = value,
                    GL_OPERAND0_RGB => self.operand_rgb[unit][0] = value,
                    GL_OPERAND1_RGB => self.operand_rgb[unit][1] = value,
                    GL_OPERAND2_RGB => self.operand_rgb[unit][2] = value,
                    GL_OPERAND0_ALPHA => self.operand_alpha[unit][0] = value,
                    GL_OPERAND1_ALPHA => self.operand_alpha[unit][1] = value,
                    GL_OPERAND2_ALPHA => self.operand_alpha[unit][2] = value,
                    _ => {}
                }
                Ok(0)
            }
            "TexImage2D" => {
                let target = a(0);
                let face = cube_face(target);
                if target != GL_TEXTURE_2D && face.is_none() {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let level = a(1);
                let (internal, width, height, border, format, ty, pixels) =
                    (a(2), a(3), a(4), a(5), a(6), a(7), a(8));
                if level > 16 || border != 0 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if matches!(
                    internal,
                    GL_DEPTH_COMPONENT | GL_DEPTH_COMPONENT16_OES | GL_DEPTH_STENCIL_OES
                ) {
                    let depth_stencil = internal == GL_DEPTH_STENCIL_OES;
                    let valid_format = if depth_stencil {
                        format == GL_DEPTH_STENCIL_OES
                    } else {
                        format == GL_DEPTH_COMPONENT
                    };
                    let valid_type = if depth_stencil {
                        ty == GL_UNSIGNED_INT_24_8_OES
                    } else {
                        matches!(
                            ty,
                            GL_UNSIGNED_BYTE | GL_UNSIGNED_SHORT | GL_UNSIGNED_INT | GL_FLOAT
                        )
                    };
                    if !valid_format || !valid_type {
                        self.set_error(GL_INVALID_ENUM);
                        return Ok(0);
                    }
                    let unit = self.active_texture;
                    let id = if self.bound_texture[unit] == 0 {
                        let id = self.next_texture;
                        self.next_texture += 1;
                        id
                    } else {
                        self.bound_texture[unit]
                    };
                    let mut tex = self
                        .textures
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(Texture::white);
                    if level > 0
                        && (width != (tex.width >> level).max(1)
                            || height != (tex.height >> level).max(1))
                    {
                        self.set_error(GL_INVALID_VALUE);
                        return Ok(0);
                    }
                    let level = level as usize;
                    if level == 0 {
                        tex.width = width;
                        tex.height = height;
                        tex.rgba = vec![0u8; (width * height * 4) as usize];
                        tex.levels.clear();
                        tex.depth.clear();
                        tex.stencil.clear();
                        if face.is_some() {
                            tex.faces.clear();
                            tex.cube_levels.clear();
                            tex.cube_depth.clear();
                            tex.cube_stencil.clear();
                        }
                    }
                    let count = (width * height) as usize;
                    let mut depth = vec![1.0f32; count];
                    let mut stencil = if depth_stencil {
                        vec![0u8; count]
                    } else {
                        Vec::new()
                    };
                    if pixels != 0 {
                        let align = self.unpack_alignment.max(1);
                        let bpp = if depth_stencil {
                            4
                        } else {
                            match ty {
                                GL_UNSIGNED_BYTE => 1,
                                GL_UNSIGNED_SHORT => 2,
                                _ => 4,
                            }
                        };
                        let row_bytes = (width * bpp + align - 1) / align * align;
                        for row in 0..height {
                            for col in 0..width {
                                let at = pixels + row * row_bytes + col * bpp;
                                let i = (row * width + col) as usize;
                                if depth_stencil {
                                    let packed = mem.read_u32(at)?;
                                    depth[i] = ((packed >> 8) & 0x00FF_FFFF) as f32 / 16_777_215.0;
                                    stencil[i] = (packed & 0xFF) as u8;
                                } else {
                                    depth[i] = match ty {
                                        GL_UNSIGNED_BYTE => mem.read_u8(at)? as f32 / 255.0,
                                        GL_UNSIGNED_SHORT => mem.read_u16(at)? as f32 / 65_535.0,
                                        GL_UNSIGNED_INT => {
                                            mem.read_u32(at)? as f32 / 4_294_967_295.0
                                        }
                                        GL_FLOAT => f32::from_bits(mem.read_u32(at)?),
                                        _ => 1.0,
                                    };
                                }
                            }
                        }
                    }
                    if let Some(face) = face {
                        if level == 0 {
                            tex.cube_depth = vec![vec![depth; 6]];
                            tex.cube_stencil = if depth_stencil {
                                vec![vec![stencil; 6]]
                            } else {
                                Vec::new()
                            };
                        } else {
                            if tex.cube_depth.is_empty() {
                                self.set_error(GL_INVALID_OPERATION);
                                return Ok(0);
                            }
                            if tex.cube_depth.len() <= level {
                                tex.cube_depth.resize(level + 1, Vec::new());
                            }
                            let faces = &mut tex.cube_depth[level];
                            if faces.len() < 6 {
                                faces.resize(6, Vec::new());
                            }
                            faces[face] = depth;
                            if depth_stencil {
                                if tex.cube_stencil.len() <= level {
                                    tex.cube_stencil.resize(level + 1, Vec::new());
                                }
                                let faces = &mut tex.cube_stencil[level];
                                if faces.len() < 6 {
                                    faces.resize(6, Vec::new());
                                }
                                faces[face] = stencil;
                            }
                        }
                        self.textures.insert(id, tex);
                        return Ok(0);
                    }
                    if tex.depth.len() <= level {
                        tex.depth.resize(level + 1, Vec::new());
                    }
                    tex.depth[level] = depth;
                    if tex.stencil.len() <= level {
                        tex.stencil.resize(level + 1, Vec::new());
                    }
                    tex.stencil[level] = stencil;
                    self.textures.insert(id, tex);
                    return Ok(0);
                }
                if !is_pixel_format(format, ty) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !matches!(
                    internal,
                    GL_RGBA | GL_RGB | GL_LUMINANCE | GL_ALPHA | GL_LUMINANCE_ALPHA
                ) {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let level = level as usize;
                if width == 0 || height == 0 {
                    return Ok(0);
                }
                let mut rgba = vec![255u8; (width * height * 4) as usize];
                if pixels != 0 {
                    rgba = unpack_pixels(
                        mem,
                        pixels,
                        width,
                        height,
                        format,
                        ty,
                        self.unpack_alignment,
                    )?;
                }
                let unit = self.active_texture;
                let id = if self.bound_texture[unit] == 0 {
                    let id = self.next_texture;
                    self.next_texture += 1;
                    id
                } else {
                    self.bound_texture[unit]
                };
                let old = self
                    .textures
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(Texture::white);
                let mut tex = old;
                tex.internal = internal;
                if let Some(face) = face {
                    if level == 0 {
                        tex.width = width;
                        tex.height = height;
                        let count = (width * height * 4) as usize;
                        tex.faces = vec![vec![0u8; count]; 6];
                        tex.faces[face] = rgba.clone();
                        if face == 0 {
                            tex.rgba = rgba;
                        }
                        tex.levels.clear();
                        tex.cube_levels.clear();
                    } else {
                        if tex.faces.len() < 6 {
                            self.set_error(GL_INVALID_OPERATION);
                            return Ok(0);
                        }
                        if width != (tex.width >> level).max(1)
                            || height != (tex.height >> level).max(1)
                        {
                            self.set_error(GL_INVALID_VALUE);
                            return Ok(0);
                        }
                        let index = level as usize - 1;
                        if tex.cube_levels.len() <= index {
                            tex.cube_levels.resize(index + 1, Vec::new());
                        }
                        let count = (width * height * 4) as usize;
                        let faces = &mut tex.cube_levels[index];
                        if faces.len() < 6 {
                            faces.resize(6, vec![0u8; count]);
                        }
                        faces[face] = rgba;
                    }
                    self.textures.insert(id, tex);
                    return Ok(0);
                }
                if level > 0
                    && (width != (tex.width >> level).max(1)
                        || height != (tex.height >> level).max(1))
                {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if level == 0 {
                    tex.width = width;
                    tex.height = height;
                    tex.rgba = rgba;
                    tex.levels.clear();
                } else if level <= 16 {
                    if tex.levels.len() < level {
                        tex.levels.resize(level, Vec::new());
                    }
                    tex.levels[level - 1] = rgba;
                }
                self.textures.insert(id, tex);
                Ok(0)
            }
            "TexSubImage2D" => {
                let target = a(0);
                if target != GL_TEXTURE_2D && cube_face(target).is_none() {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let level = a(1);
                let (xoffset, yoffset) = (a(2), a(3));
                let (width, height, format) = (a(4), a(5), a(6));
                let (ty, pixels) = (a(7), a(8));
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if !is_pixel_format(format, ty) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if width == 0 || height == 0 || pixels == 0 {
                    return Ok(0);
                }
                let sub = unpack_pixels(
                    mem,
                    pixels,
                    width,
                    height,
                    format,
                    ty,
                    self.unpack_alignment,
                )?;
                let unit = self.active_texture;
                let Some(tex) = self.textures.get_mut(&self.bound_texture[unit]) else {
                    return Ok(0);
                };
                let Some((target_w, target_h, target)) = tex.face_storage(target, level) else {
                    return Ok(0);
                };
                for row in 0..height {
                    let row_y = yoffset.saturating_add(row);
                    if row_y >= target_h {
                        continue;
                    }
                    for col in 0..width {
                        let col_x = xoffset.saturating_add(col);
                        if col_x >= target_w {
                            continue;
                        }
                        let si = ((row * width + col) * 4) as usize;
                        let di = ((row_y * target_w + col_x) * 4) as usize;
                        target[di..di + 4].copy_from_slice(&sub[si..si + 4]);
                    }
                }
                Ok(0)
            }

            // ---- draws ----
            "DrawArrays" => {
                let (mode, first, count) = (a(0), a(1), a(2));
                if !is_primitive(mode) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !self.check_count(count, MAX_DRAW_COUNT) {
                    return Ok(0);
                }
                if first.checked_add(count).is_none() {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let vertices = self.read_arrays(first, count, mem)?;
                self.submit(&vertices, mode, egl, mem)?;
                Ok(0)
            }
            "DrawElements" => {
                let (mode, count, ty, index_ptr) = (a(0), a(1), a(2), a(3));
                if !is_primitive(mode) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !matches!(ty, GL_UNSIGNED_BYTE | GL_UNSIGNED_SHORT) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !self.check_count(count, MAX_DRAW_COUNT) {
                    return Ok(0);
                }
                let indices = if self.bound_buffer[1] != 0 {
                    self.bound_buffer[1] + index_ptr
                } else {
                    index_ptr
                };
                let span = count.checked_mul(2);
                if span.is_none() || indices.checked_add(span.unwrap()).is_none() {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let mut verts = Vec::with_capacity(count as usize);
                for i in 0..count {
                    let at = if ty == GL_UNSIGNED_BYTE {
                        indices + i
                    } else {
                        indices + i * 2
                    };
                    let idx = if ty == GL_UNSIGNED_BYTE {
                        mem.read_u8(at)? as u32
                    } else {
                        mem.read_u16(at)? as u32
                    };
                    verts.extend(self.read_arrays(idx, 1, mem)?);
                }
                self.submit(&verts, mode, egl, mem)?;
                Ok(0)
            }

            // ---- IGLES11 slots (AEEGLES11.h, slots 109..147) ----
            "Color4ub" => {
                self.current.color = [
                    a(0) as f32 / 255.0,
                    a(1) as f32 / 255.0,
                    a(2) as f32 / 255.0,
                    a(3) as f32 / 255.0,
                ];
                Ok(0)
            }
            "TexEnviv" => {
                let (pname, ptr) = (a(1), a(2));
                if ptr != 0 {
                    let v = mem.read_u32(ptr)?;
                    let unit = self.active_texture;
                    match pname {
                        GL_TEXTURE_ENV_MODE => self.tex_env_mode[unit] = v,
                        GL_COMBINE_RGB => self.combine_rgb[unit] = v,
                        GL_COMBINE_ALPHA => self.combine_alpha[unit] = v,
                        GL_RGB_SCALE => self.rgb_scale[unit] = v as f32,
                        GL_ALPHA_SCALE => self.alpha_scale[unit] = v as f32,
                        _ => {}
                    }
                }
                Ok(0)
            }
            "TexParameteriv" => {
                let (pname, ptr) = (a(1), a(2));
                if ptr == 0 {
                    return Ok(0);
                }
                let value = mem.read_u32(ptr)?;
                self.set_tex_parameter(pname, value);
                Ok(0)
            }
            "PointSizePointerOES" => {
                self.client.point_size = Some(Array {
                    size: 1,
                    ty: a(0),
                    stride: a(1) as i32,
                    ptr: a(2),
                });
                Ok(0)
            }
            "ClipPlanef" | "ClipPlanex" => {
                let raw = a(0);
                if !(GL_CLIP_PLANE0..GL_CLIP_PLANE0 + 6).contains(&raw) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let plane = (raw - GL_CLIP_PLANE0) as usize;
                let ptr = a(1);
                if ptr == 0 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let ty = if name.contains('x') {
                    GL_FIXED
                } else {
                    GL_FLOAT
                };
                for i in 0..4 {
                    self.clip_planes[plane][i] = read_component(mem, ptr + i as u32 * 4, ty)?;
                }
                Ok(0)
            }
            "GetClipPlanef" | "GetClipPlanex" => {
                let raw = a(0);
                if !(GL_CLIP_PLANE0..GL_CLIP_PLANE0 + 6).contains(&raw) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let plane = (raw - GL_CLIP_PLANE0) as usize;
                let ptr = a(1);
                if ptr == 0 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let fixed = name.ends_with('x');
                for i in 0..4 {
                    let v = self.clip_planes[plane][i];
                    let word = if fixed {
                        ((v * 65536.0) as i32) as u32
                    } else {
                        v.to_bits()
                    };
                    mem.write_u32(ptr + i as u32 * 4, word)?;
                }
                Ok(0)
            }
            "BindBuffer" => {
                let (target, buffer) = (a(0), a(1));
                let slot = match target {
                    GL_ARRAY_BUFFER => 0,
                    GL_ELEMENT_ARRAY_BUFFER => 1,
                    _ => return Ok(0),
                };
                self.bound_buffer_id[slot] = buffer;
                self.bound_buffer[slot] = self.buffers.get(&buffer).map(|b| b.addr).unwrap_or(0);
                Ok(0)
            }
            "BufferData" => {
                let (target, size, data, usage) = (a(0), a(1), a(2), a(3));
                let slot = match target {
                    GL_ARRAY_BUFFER => 0,
                    GL_ELEMENT_ARRAY_BUFFER => 1,
                    _ => return Ok(0),
                };
                let id = self.bound_buffer_id[slot];
                if id == 0 {
                    return Ok(0);
                }
                if !self.check_count(size, MAX_BUFFER_BYTES) {
                    return Ok(0);
                }
                let addr = mem.alloc(size, 4)?;
                if data != 0 {
                    let bytes = mem.read_bytes(data, size as usize)?;
                    mem.write_bytes(addr, &bytes)?;
                }
                self.buffers.insert(id, Buffer { addr, size, usage });
                self.bound_buffer[slot] = addr;
                Ok(0)
            }
            "BufferSubData" => {
                let (target, offset, size, data) = (a(0), a(1), a(2), a(3));
                let slot = match target {
                    GL_ARRAY_BUFFER => 0,
                    GL_ELEMENT_ARRAY_BUFFER => 1,
                    _ => return Ok(0),
                };
                let id = self.bound_buffer_id[slot];
                let Some(b) = self.buffers.get(&id).copied() else {
                    return Ok(0);
                };
                if data == 0 || offset.checked_add(size).map_or(true, |end| end > b.size) {
                    return Ok(0);
                }
                let bytes = mem.read_bytes(data, size as usize)?;
                mem.write_bytes(b.addr + offset, &bytes)?;
                Ok(0)
            }
            "GenBuffers" => {
                let (n, out) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = self.next_buffer;
                    self.next_buffer += 1;
                    mem.write_u32(out + i * 4, id)?;
                }
                Ok(0)
            }
            "DeleteBuffers" => {
                let (n, list) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = mem.read_u32(list + i * 4)?;
                    self.buffers.remove(&id);
                    for s in 0..2 {
                        if self.bound_buffer_id[s] == id {
                            self.bound_buffer_id[s] = 0;
                            self.bound_buffer[s] = 0;
                        }
                    }
                }
                Ok(0)
            }
            // `int IsBuffer(IGLES11*, AEEGLuint buffer, AEEGLboolean *ret)`,
            // and `IsBufferQUALCOMM` reaches here through the alias table with the same shape:
            //answer through the pointer, error code as the return.
            "IsBuffer" => {
                let out = a(1);
                if out != 0 {
                    mem.write_u8(out, u8::from(self.buffers.contains_key(&a(0))))?;
                }
                Ok(0)
            }
            "GetBufferParameteriv" => {
                let (target, pname, out) = (a(0), a(1), a(2));
                let slot = match target {
                    GL_ARRAY_BUFFER => 0,
                    GL_ELEMENT_ARRAY_BUFFER => 1,
                    _ => return Ok(0),
                };
                let id = self.bound_buffer_id[slot];
                let Some(b) = self.buffers.get(&id).copied() else {
                    return Ok(0);
                };
                let v = match pname {
                    GL_BUFFER_SIZE => b.size,
                    GL_BUFFER_USAGE => b.usage,
                    _ => 0,
                };
                mem.write_u32(out, v)?;
                Ok(0)
            }

            // ---- IGLES11Ext slots ----
            "CurrentPaletteMatrixOES" => {
                let index = a(0);
                if index >= 32 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                self.current_palette_matrix = index as usize;
                Ok(0)
            }
            "LoadPaletteFromModelViewMatrixOES" => {
                self.palette_matrices[self.current_palette_matrix] = self.matrices[0].current;
                Ok(0)
            }
            "MatrixIndexPointerOES" => {
                let (size, ty, stride, ptr) = (a(0) as i32, a(1), a(2) as i32, a(3));
                if !matches!(ty, GL_UNSIGNED_BYTE | GL_UNSIGNED_SHORT) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !(1..=4).contains(&size) {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                self.client.matrix_index = Some(Array {
                    ptr,
                    size,
                    ty,
                    stride,
                });
                Ok(0)
            }
            "WeightPointerOES" => {
                let (size, ty, stride, ptr) = (a(0) as i32, a(1), a(2) as i32, a(3));
                if !is_array_type(ty) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !(1..=4).contains(&size) {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                self.client.weight = Some(Array {
                    ptr,
                    size,
                    ty,
                    stride,
                });
                Ok(0)
            }
            "BufferDataATI" => {
                // The ATI face carries numvertices/vertexelemsize; the upload itself is the
                // ordinary VBO path and the count fields do not add state.
                let (target, size, data, usage) = (a(0), a(3), a(4), a(5));
                self.method("IGL", "BufferData", &[target, size, data, usage], mem, egl)?;
                self.ati_mesh.lists.clear();
                Ok(0)
            }
            "MeshListATI" => {
                let n = a(0) as i32;
                if !(0..=1024).contains(&n) {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                let (mode_ptr, count_ptr, type_ptr, indices_ptr) = (a(1), a(2), a(3), a(4));
                if n > 0 && (mode_ptr == 0 || count_ptr == 0 || type_ptr == 0 || indices_ptr == 0) {
                    return Ok(0);
                }
                let mut lists = Vec::with_capacity(n as usize);
                for i in 0..n as u32 {
                    let at = i * 4;
                    lists.push(AtiMeshList {
                        mode: mem.read_u32(mode_ptr + at)?,
                        count: mem.read_u32(count_ptr + at)?,
                        ty: mem.read_u32(type_ptr + at)?,
                        indices: mem.read_u32(indices_ptr + at)?,
                    });
                }
                self.ati_mesh.lists = lists;
                Ok(0)
            }
            "DrawVertexBufferObjectATI" => {
                let lists = std::mem::take(&mut self.ati_mesh.lists);
                for list in lists {
                    self.method(
                        "IGL",
                        "DrawElements",
                        &[list.mode, list.count, list.ty, list.indices],
                        mem,
                        egl,
                    )?;
                }
                Ok(0)
            }
            "DrawTexfOES" | "DrawTexxOES" | "DrawTexiOES" | "DrawTexsOES" => {
                let (x, y, w, h) = match name {
                    "DrawTexfOES" => (f(0), f(1), f(3), f(4)),
                    "DrawTexxOES" => (fx(0), fx(1), fx(3), fx(4)),
                    "DrawTexiOES" => (
                        a(0) as i32 as f32,
                        a(1) as i32 as f32,
                        a(3) as i32 as f32,
                        a(4) as i32 as f32,
                    ),
                    _ => (
                        a(0) as i16 as f32,
                        a(1) as i16 as f32,
                        a(3) as i16 as f32,
                        a(4) as i16 as f32,
                    ),
                };
                self.draw_texture(x, y, w, h, egl)?;
                Ok(0)
            }
            "DrawTexfvOES" | "DrawTexxvOES" | "DrawTexivOES" | "DrawTexsvOES" => {
                let ptr = a(0);
                let ty = match name {
                    "DrawTexfvOES" => GL_FLOAT,
                    "DrawTexxvOES" => GL_FIXED,
                    "DrawTexivOES" => GL_INT,
                    _ => GL_SHORT,
                };
                let x = read_component(mem, ptr, ty)?;
                let y = read_component(mem, ptr + 4, ty)?;
                let w = read_component(mem, ptr + 12, ty)?;
                let h = read_component(mem, ptr + 16, ty)?;
                self.draw_texture(x, y, w, h, egl)?;
                Ok(0)
            }

            "GenerateMipmapOES" => {
                let unit = self.active_texture;
                if let Some(tex) = self.textures.get_mut(&self.bound_texture[unit]) {
                    if tex.faces.len() == 6 {
                        tex.generate_cube_mipmaps();
                    } else {
                        tex.generate_mipmaps();
                    }
                }
                Ok(0)
            }

            "TexGenf" | "TexGeni" | "TexGenx" => {
                let (coord, pname, param) = (a(0), a(1), a(2));
                let coord = (coord.wrapping_sub(GL_TEXTURE_GEN_S) as usize).min(3);
                if pname == GL_TEXTURE_GEN_MODE {
                    self.tex_gen_mode[coord] = param;
                }
                Ok(0)
            }
            "TexGenfv" | "TexGeniv" | "TexGenxv" => {
                let (coord, pname, ptr) = (a(0), a(1), a(2));
                let coord = (coord.wrapping_sub(GL_TEXTURE_GEN_S) as usize).min(3);
                let ty = match name {
                    "TexGenfv" => GL_FLOAT,
                    "TexGenxv" => GL_FIXED,
                    _ => GL_INT,
                };
                let slot = match pname {
                    GL_OBJECT_PLANE => 0,
                    GL_EYE_PLANE => 1,
                    _ => return Ok(0),
                };
                for i in 0..4 {
                    self.tex_gen_plane[coord][slot][i] =
                        read_component(mem, ptr + i as u32 * 4, ty)?;
                }
                Ok(0)
            }
            "GetTexGenfv" | "GetTexGeniv" | "GetTexGenxv" => {
                let (coord, pname, ptr) = (a(0), a(1), a(2));
                let coord = (coord.wrapping_sub(GL_TEXTURE_GEN_S) as usize).min(3);
                let int_out = name == "GetTexGeniv";
                let fixed = name == "GetTexGenxv";
                if pname == GL_TEXTURE_GEN_MODE {
                    write_enum(mem, ptr, self.tex_gen_mode[coord], int_out, fixed)?;
                    return Ok(0);
                }
                let slot = match pname {
                    GL_OBJECT_PLANE => 0,
                    GL_EYE_PLANE => 1,
                    _ => return Ok(0),
                };
                write_values(mem, ptr, &self.tex_gen_plane[coord][slot], 4, fixed)?;
                Ok(0)
            }

            "GenRenderbuffersOES" => {
                let (n, out) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = self.next_renderbuffer;
                    self.next_renderbuffer += 1;
                    self.renderbuffers.insert(
                        id,
                        Renderbuffer {
                            format: 0,
                            width: 0,
                            height: 0,
                            depth: Vec::new(),
                            stencil: Vec::new(),
                        },
                    );
                    mem.write_u32(out + i * 4, id)?;
                }
                Ok(0)
            }
            "BindRenderbufferOES" => {
                if a(0) != GL_RENDERBUFFER_OES {
                    return Ok(0);
                }
                let id = a(1);
                if id != 0 {
                    self.renderbuffers.entry(id).or_insert(Renderbuffer {
                        format: 0,
                        width: 0,
                        height: 0,
                        depth: Vec::new(),
                        stencil: Vec::new(),
                    });
                }
                self.bound_renderbuffer = id;
                Ok(0)
            }
            "DeleteRenderbuffersOES" => {
                let (n, list) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = mem.read_u32(list + i * 4)?;
                    self.renderbuffers.remove(&id);
                    for fb in self.framebuffers.values_mut() {
                        if fb.depth == Some(id) {
                            fb.depth = None;
                        }
                        if fb.stencil == Some(id) {
                            fb.stencil = None;
                        }
                    }
                    if self.bound_renderbuffer == id {
                        self.bound_renderbuffer = 0;
                    }
                }
                Ok(0)
            }
            "RenderbufferStorageOES" => {
                let (format, w, h) = (a(1), a(2), a(3));
                if a(0) != GL_RENDERBUFFER_OES {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if !matches!(
                    format,
                    GL_DEPTH_COMPONENT16_OES | GL_STENCIL_INDEX8_OES | GL_DEPTH24_STENCIL8_OES
                ) {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let id = self.bound_renderbuffer;
                if id != 0 {
                    let (w, h) = (w.max(1), h.max(1));
                    let n = (w * h) as usize;
                    let (depth, stencil) = match format {
                        GL_DEPTH_COMPONENT16_OES => (vec![1.0; n], Vec::new()),
                        GL_STENCIL_INDEX8_OES => (Vec::new(), vec![0; n]),
                        GL_DEPTH24_STENCIL8_OES => (vec![1.0; n], vec![0; n]),
                        _ => (vec![1.0; n], vec![0; n]),
                    };
                    self.renderbuffers.insert(
                        id,
                        Renderbuffer {
                            format,
                            width: w,
                            height: h,
                            depth,
                            stencil,
                        },
                    );
                }
                Ok(0)
            }
            "FramebufferRenderbufferOES" => {
                let (attachment, target, rb) = (a(1), a(2), a(3));
                if a(0) != GL_FRAMEBUFFER_OES
                    || target != GL_RENDERBUFFER_OES
                    || !matches!(
                        attachment,
                        GL_DEPTH_ATTACHMENT_OES | GL_STENCIL_ATTACHMENT_OES
                    )
                {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                let fb = self.bound_framebuffer;
                if fb == 0 {
                    return Ok(0);
                }
                let Some(entry) = self.framebuffers.get_mut(&fb) else {
                    return Ok(0);
                };
                match attachment {
                    GL_DEPTH_ATTACHMENT_OES => {
                        entry.depth = (rb != 0).then_some(rb);
                        entry.depth_texture = None;
                        entry.depth_target = GL_TEXTURE_2D;
                    }
                    GL_STENCIL_ATTACHMENT_OES => {
                        entry.stencil = (rb != 0).then_some(rb);
                        entry.stencil_texture = None;
                        entry.stencil_target = GL_TEXTURE_2D;
                    }
                    _ => {}
                }
                // A depth- or stencil-only framebuffer has no colour texture to take its
                // size from, so the renderbuffer just attached names the drawable instead.
                if entry.color == 0 && rb != 0 {
                    if let Some(size) = self
                        .renderbuffers
                        .get(&rb)
                        .map(|r| (r.width.max(1), r.height.max(1)))
                    {
                        if (entry.surface.width, entry.surface.height) != size {
                            entry.surface = Surface::new(size.0, size.1, SurfaceKind::Pbuffer);
                        }
                    }
                }
                Ok(0)
            }
            // `int IsRenderbufferOES(IGLES11ExtPak*, AEEGLuint renderbuffer, AEEGLboolean *ret)`
            // — answer through the pointer, error code as the return.
            "IsRenderbufferOES" => {
                let out = a(1);
                if out != 0 {
                    mem.write_u8(out, u8::from(self.renderbuffers.contains_key(&a(0))))?;
                }
                Ok(0)
            }
            "GetRenderbufferParameterivOES" => {
                let (pname, out) = (a(1), a(2));
                let Some(rb) = self.renderbuffers.get(&self.bound_renderbuffer) else {
                    return Ok(0);
                };
                let v = match pname {
                    GL_RENDERBUFFER_WIDTH_OES => rb.width,
                    GL_RENDERBUFFER_HEIGHT_OES => rb.height,
                    GL_RENDERBUFFER_INTERNAL_FORMAT_OES => rb.format,
                    _ => 0,
                };
                mem.write_u32(out, v)?;
                Ok(0)
            }

            "GenFramebuffersOES" => {
                let (n, out) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = self.next_framebuffer;
                    self.next_framebuffer += 1;
                    self.framebuffers.insert(id, Framebuffer::empty());
                    mem.write_u32(out + i * 4, id)?;
                }
                Ok(0)
            }
            "BindFramebufferOES" => {
                if a(0) != GL_FRAMEBUFFER_OES {
                    return Ok(0);
                }
                let id = a(1);
                if id != 0 {
                    self.framebuffers
                        .entry(id)
                        .or_insert_with(Framebuffer::empty);
                }
                self.bound_framebuffer = id;
                Ok(0)
            }
            "DeleteFramebuffersOES" => {
                let (n, list) = (a(0), a(1));
                if !self.check_count(n, MAX_GENERATED_OBJECTS) {
                    return Ok(0);
                }
                for i in 0..n {
                    let id = mem.read_u32(list + i * 4)?;
                    self.framebuffers.remove(&id);
                    if self.bound_framebuffer == id {
                        self.bound_framebuffer = 0;
                    }
                }
                Ok(0)
            }
            "FramebufferTexture2DOES" => {
                let (attachment, textarget, texture, level) = (a(1), a(2), a(3), a(4));
                if level > 16 {
                    self.set_error(GL_INVALID_VALUE);
                    return Ok(0);
                }
                if a(0) != GL_FRAMEBUFFER_OES {
                    return Ok(0);
                }
                let id = self.bound_framebuffer;
                if id == 0 {
                    return Ok(0);
                }
                if !matches!(
                    attachment,
                    GL_COLOR_ATTACHMENT0_OES | GL_DEPTH_ATTACHMENT_OES | GL_STENCIL_ATTACHMENT_OES
                ) || (textarget != GL_TEXTURE_2D && cube_face(textarget).is_none())
                {
                    self.set_error(GL_INVALID_ENUM);
                    return Ok(0);
                }
                if texture == 0 {
                    if let Some(fb) = self.framebuffers.get_mut(&id) {
                        match attachment {
                            GL_COLOR_ATTACHMENT0_OES => {
                                fb.color = 0;
                                fb.color_level = 0;
                                fb.color_target = GL_TEXTURE_2D;
                            }
                            GL_DEPTH_ATTACHMENT_OES => {
                                fb.depth = None;
                                fb.depth_texture = None;
                                fb.depth_target = GL_TEXTURE_2D;
                            }
                            GL_STENCIL_ATTACHMENT_OES => {
                                fb.stencil = None;
                                fb.stencil_texture = None;
                                fb.stencil_target = GL_TEXTURE_2D;
                            }
                            _ => {}
                        }
                    }
                    return Ok(0);
                }
                let Some((w, h)) = self
                    .textures
                    .get(&texture)
                    .and_then(|t| t.level_size(level))
                else {
                    self.set_error(GL_INVALID_OPERATION);
                    return Ok(0);
                };
                match attachment {
                    GL_COLOR_ATTACHMENT0_OES => {
                        let Some(rgba) = self
                            .textures
                            .get(&texture)
                            .and_then(|t| t.face_rgba(textarget, level))
                        else {
                            self.set_error(GL_INVALID_OPERATION);
                            return Ok(0);
                        };
                        let Some(fb) = self.framebuffers.get_mut(&id) else {
                            return Ok(0);
                        };
                        fb.color = texture;
                        fb.color_level = level;
                        fb.color_target = textarget;
                        fb.surface = Surface {
                            width: w,
                            height: h,
                            rgba,
                            depth: vec![1.0; (w * h) as usize],
                            kind: SurfaceKind::Pbuffer,
                            target: None,
                            stencil: vec![0u8; (w * h) as usize],
                            swap_behavior: EGL_BUFFER_DESTROYED,
                            texture_format: EGL_TEXTURE_RGBA,
                            texture_target: EGL_TEXTURE_2D,
                            color_buffer: None,
                        };
                    }
                    GL_DEPTH_ATTACHMENT_OES => {
                        if self
                            .textures
                            .get(&texture)
                            .and_then(|t| t.face_depth(textarget, level))
                            .is_none()
                        {
                            self.set_error(GL_INVALID_OPERATION);
                            return Ok(0);
                        }
                        let Some(fb) = self.framebuffers.get_mut(&id) else {
                            return Ok(0);
                        };
                        fb.depth = None;
                        fb.depth_texture = Some((texture, level));
                        fb.depth_target = textarget;
                        if fb.color == 0 && (fb.surface.width, fb.surface.height) != (w, h) {
                            fb.surface = Surface::new(w, h, SurfaceKind::Pbuffer);
                        }
                    }
                    GL_STENCIL_ATTACHMENT_OES => {
                        if self
                            .textures
                            .get(&texture)
                            .and_then(|t| t.face_stencil(textarget, level))
                            .is_none()
                        {
                            self.set_error(GL_INVALID_OPERATION);
                            return Ok(0);
                        }
                        let Some(fb) = self.framebuffers.get_mut(&id) else {
                            return Ok(0);
                        };
                        fb.stencil = None;
                        fb.stencil_texture = Some((texture, level));
                        fb.stencil_target = textarget;
                        if fb.color == 0 && (fb.surface.width, fb.surface.height) != (w, h) {
                            fb.surface = Surface::new(w, h, SurfaceKind::Pbuffer);
                        }
                    }
                    _ => {}
                }
                Ok(0)
            }
            // `int IsFramebufferOES(IGLES11ExtPak*, AEEGLuint framebuffer, AEEGLboolean *ret)`
            // — answer through the pointer, error code as the return.
            "IsFramebufferOES" => {
                let out = a(1);
                if out != 0 {
                    mem.write_u8(out, u8::from(self.framebuffers.contains_key(&a(0))))?;
                }
                Ok(0)
            }

            // ---- IGLES11ExtPak slots ----
            "GetFramebufferAttachmentParameterivOES" => {
                let (attachment, pname, out) = (a(1), a(2), a(3));
                if a(0) != GL_FRAMEBUFFER_OES {
                    return Ok(0);
                }
                let Some(fb) = self.framebuffers.get(&self.bound_framebuffer) else {
                    return Ok(0);
                };
                let (object_type, object_name, level) = match attachment {
                    GL_COLOR_ATTACHMENT0_OES => {
                        let is_texture = fb.color != 0;
                        (
                            if is_texture { GL_TEXTURE } else { 0 },
                            fb.color,
                            fb.color_level,
                        )
                    }
                    GL_DEPTH_ATTACHMENT_OES => {
                        if let Some((texture, level)) = fb.depth_texture {
                            (GL_TEXTURE, texture, level)
                        } else if let Some(rb) = fb.depth {
                            (GL_RENDERBUFFER_OES, rb, 0)
                        } else {
                            (0, 0, 0)
                        }
                    }
                    GL_STENCIL_ATTACHMENT_OES => {
                        if let Some((texture, level)) = fb.stencil_texture {
                            (GL_TEXTURE, texture, level)
                        } else if let Some(rb) = fb.stencil {
                            (GL_RENDERBUFFER_OES, rb, 0)
                        } else {
                            (0, 0, 0)
                        }
                    }
                    _ => return Ok(0),
                };
                let v = match pname {
                    GL_FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE_OES => object_type,
                    GL_FRAMEBUFFER_ATTACHMENT_OBJECT_NAME_OES if object_name != 0 => object_name,
                    GL_FRAMEBUFFER_ATTACHMENT_TEXTURE_LEVEL_OES if object_type == GL_TEXTURE => {
                        level
                    }
                    GL_FRAMEBUFFER_ATTACHMENT_TEXTURE_CUBE_MAP_FACE_OES
                        if object_type == GL_TEXTURE =>
                    {
                        0
                    }
                    _ => return Ok(0),
                };
                if out != 0 {
                    mem.write_u32(out, v)?;
                }
                Ok(0)
            }
            "CheckFramebufferStatusOES" => {
                if a(0) != GL_FRAMEBUFFER_OES {
                    return Ok(0);
                }
                if self.bound_framebuffer == 0 {
                    return Ok(GL_FRAMEBUFFER_COMPLETE_OES);
                }
                let Some(fb) = self.framebuffers.get(&self.bound_framebuffer) else {
                    return Ok(GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT_OES);
                };
                if fb.color == 0
                    && fb.depth.is_none()
                    && fb.stencil.is_none()
                    && fb.depth_texture.is_none()
                    && fb.stencil_texture.is_none()
                {
                    return Ok(GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT_OES);
                }
                let mut sizes = Vec::new();
                if fb.color != 0 {
                    let Some(t) = self.textures.get(&fb.color) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    let Some((w, h)) = t.level_size(fb.color_level) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    if t.face_rgba(fb.color_target, fb.color_level).is_none() {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    }
                    sizes.push((w, h));
                }
                if let Some((texture, level)) = fb.depth_texture {
                    let Some(t) = self.textures.get(&texture) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    let Some((w, h)) = t.level_size(level) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    if t.face_depth(fb.depth_target, level).is_none() {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    }
                    sizes.push((w, h));
                }
                if let Some((texture, level)) = fb.stencil_texture {
                    let Some(t) = self.textures.get(&texture) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    let Some((w, h)) = t.level_size(level) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    if t.face_stencil(fb.stencil_target, level).is_none() {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    }
                    sizes.push((w, h));
                }
                for rb in [fb.depth, fb.stencil].into_iter().flatten() {
                    let Some(r) = self.renderbuffers.get(&rb) else {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    };
                    if r.width == 0 || r.height == 0 {
                        return Ok(GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT_OES);
                    }
                    sizes.push((r.width, r.height));
                }
                if sizes.windows(2).any(|w| w[0] != w[1]) {
                    return Ok(GL_FRAMEBUFFER_INCOMPLETE_DIMENSIONS_OES);
                }
                Ok(GL_FRAMEBUFFER_COMPLETE_OES)
            }

            // ---- things not implemented yet, answered by name ----
            _ => Ok(crate::abi::generated::AEE_EUNSUPPORTED),
        }
    }
}
