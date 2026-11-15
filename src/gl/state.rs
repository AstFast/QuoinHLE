//! Implementation blocks for the software pipeline's supporting types.

use super::*;

impl MatrixStack {
    pub(super) fn new() -> Self {
        MatrixStack {
            stack: Vec::new(),
            current: Mat4::IDENTITY,
        }
    }
}

impl Light {
    pub(super) fn off() -> Self {
        Light {
            ambient: [0.0, 0.0, 0.0, 1.0],
            diffuse: [0.0, 0.0, 0.0, 1.0],
            specular: [0.0, 0.0, 0.0, 1.0],
            position: [0.0, 0.0, 1.0, 0.0],
            spot_direction: [0.0, 0.0, -1.0],
            spot_exponent: 0.0,
            spot_cutoff: 180.0,
            constant_attenuation: 1.0,
            linear_attenuation: 0.0,
            quadratic_attenuation: 0.0,
            enabled: false,
        }
    }

    pub(super) fn light0() -> Self {
        Light {
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [1.0, 1.0, 1.0, 1.0],
            ..Light::off()
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Material {
            ambient: [0.2, 0.2, 0.2, 1.0],
            diffuse: [0.8, 0.8, 0.8, 1.0],
            specular: [0.0, 0.0, 0.0, 1.0],
            emission: [0.0, 0.0, 0.0, 1.0],
            shininess: 0.0,
        }
    }
}

impl Default for Gles {
    fn default() -> Self {
        Self::new()
    }
}

impl Gles {
    pub fn new() -> Self {
        Gles {
            clear_color: [0.0, 0.0, 0.0, 1.0],
            clear_depth: 1.0,
            viewport: [0, 0, 0, 0],
            scissor: [0, 0, 0, 0],
            scissor_test: false,
            depth_range: [0.0, 1.0],
            depth_test: false,
            depth_func: GL_LESS,
            depth_mask: true,
            blend: false,
            logic_op: GL_COPY,
            logic_op_enabled: false,
            sample_coverage: false,
            sample_coverage_value: 1.0,
            sample_coverage_invert: false,
            blend_src: GL_ONE,
            blend_dst: GL_ZERO,
            blend_src_alpha: GL_ONE,
            blend_dst_alpha: GL_ZERO,
            blend_equation: GL_FUNC_ADD,
            blend_equation_alpha: GL_FUNC_ADD,
            cull: false,
            cull_face: GL_BACK,
            front_face: GL_CCW,
            texture_2d: [false; 4],
            alpha_test: false,
            alpha_func: GL_ALWAYS,
            alpha_ref: 0.0,
            color_mask: [true; 4],
            stencil_test: false,
            stencil_func: GL_ALWAYS,
            stencil_ref: 0,
            stencil_value_mask: u32::MAX,
            stencil_write_mask: u32::MAX,
            stencil_fail: GL_KEEP,
            stencil_pass_depth_fail: GL_KEEP,
            stencil_pass_depth_pass: GL_KEEP,
            clear_stencil: 0,
            lighting: false,
            normalize: false,
            rescale_normal: false,
            two_sided: false,
            color_material: false,
            color_material_face: GL_FRONT_AND_BACK,
            color_material_mode: GL_AMBIENT_AND_DIFFUSE,
            lights: [
                Light::light0(),
                Light::off(),
                Light::off(),
                Light::off(),
                Light::off(),
                Light::off(),
                Light::off(),
                Light::off(),
            ],
            material: Material::default(),
            back_material: Material::default(),
            light_model_ambient: [0.2, 0.2, 0.2, 1.0],
            fog: false,
            fog_mode: GL_EXP,
            fog_density: 1.0,
            fog_start: 0.0,
            fog_end: 1.0,
            fog_color: [0.0, 0.0, 0.0, 1.0],
            point_size: 1.0,
            point_size_min: 0.0,
            point_size_max: 64.0,
            point_fade_threshold: 1.0,
            point_distance_attenuation: [1.0, 0.0, 0.0],
            point_sprite: false,
            point_sprite_coord_replace: false,
            local_viewer: false,
            matrix_palette: false,
            current_palette_matrix: 0,
            palette_matrices: [Mat4::IDENTITY; 32],
            line_width: 1.0,
            polygon_offset_factor: 0.0,
            polygon_offset_units: 0.0,
            polygon_offset_fill: false,
            shade_model: GL_SMOOTH,
            tex_gen_enabled: [false; 4],
            tex_gen_mode: [GL_EYE_LINEAR; 4],
            tex_gen_plane: [[[0.0; 4]; 2]; 4],
            pack_alignment: 4,
            unpack_alignment: 4,
            matrices: [MatrixStack::new(), MatrixStack::new(), MatrixStack::new()],
            mode: 0,
            current: Vertex {
                pos: Vec3::ZERO,
                w: 1.0,
                color: [1.0; 4],
                tex: [[0.0; 2]; 4],
                tex_r: [0.0; 4],
                normal: Vec3::new(0.0, 0.0, 1.0),
                point_size: 1.0,
                matrix_indices: [0; 4],
                weights: [0.0; 4],
                matrix_count: 0,
            },
            vertices: Vec::new(),
            primitive: 0,
            textures: HashMap::new(),
            next_texture: 1,
            bound_texture: [0; 4],
            bound_target: [GL_TEXTURE_2D; 4],
            tex_env_mode: [GL_MODULATE; 4],
            tex_env_color: [[0.0; 4]; 4],
            combine_rgb: [GL_MODULATE; 4],
            combine_alpha: [GL_MODULATE; 4],
            src_rgb: [[GL_TEXTURE, GL_PREVIOUS, GL_CONSTANT]; 4],
            src_alpha: [[GL_TEXTURE, GL_PREVIOUS, GL_CONSTANT]; 4],
            operand_rgb: [[GL_SRC_COLOR; 3]; 4],
            operand_alpha: [[GL_SRC_COLOR; 3]; 4],
            rgb_scale: [1.0; 4],
            alpha_scale: [1.0; 4],
            active_texture: 0,
            client_active_texture: 0,
            client_tex: [None; 4],
            client: ClientArrays::default(),
            clip_planes: [[0.0; 4]; 6],
            clip_plane_enabled: [false; 6],
            buffers: HashMap::new(),
            bound_buffer: [0; 2],
            bound_buffer_id: [0; 2],
            ati_mesh: AtiMesh::default(),
            next_buffer: 0x1000,
            framebuffers: HashMap::new(),
            bound_framebuffer: 0,
            next_framebuffer: 0x2000,
            renderbuffers: HashMap::new(),
            bound_renderbuffer: 0,
            next_renderbuffer: 0x3000,
            error: GL_NO_ERROR,
        }
    }
}
