//! A software OpenGL ES 1.0 core and the EGL 1.0 surface layer.
//!
//! The guest talks to `IGLES10` and `IEGL` through the same SVC seam as every other interface:
//! each slot arrives as a method name and a handful of words. There is no GPU behind this module
//! — a vertex is transformed by the fixed-function matrices, clipped to the viewport, rasterized
//! into a host RGBA buffer, and `eglSwapBuffers` copies that buffer into the device bitmap the
//! guest already owns. Keeping the pixels in host memory is what makes the path testable without a
//! window, a driver or a sample set pack.
//!
//! What is implemented is the part a 3D pack actually reaches: EGL display/config/context/surface,
//! the matrix stacks, immediate mode, vertex arrays, `DrawArrays`/`DrawElements`, depth, blending,
//! back-face culling and 2D textures. Slots that are not here answer `AEE_EUNSUPPORTED` by name
//! rather than a plausible zero, so a pack that needs one stops where a reader can see it.

use std::collections::HashMap;

use glam::{Mat4, Vec3, Vec4};

use crate::display::{pack565, Display};
use crate::hle::Error as HleError;
use crate::mem::Mem;

type HResult<T> = Result<T, HleError>;

mod diag;
mod dispatch;
mod egl;
mod enums;
mod framebuffer;
mod pipeline;
/// The QEGL object family: which face answers which iid.
pub(crate) mod qegl;
mod state;
mod surface;
mod texture;
pub use egl::Egl;
pub use diag::GlDrawLog;
pub(crate) use egl::PROC_NAMES;
pub(crate) use enums::*;

// ---- surfaces and textures ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceKind {
    #[default]
    Pbuffer,
    Window,
}

/// One drawable. Pixels are host `RGBA8`, so a test can read them and `SwapBuffers` can copy them
/// into the guest's RGB565 device bitmap without a second format in between.
#[derive(Default)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub depth: Vec<f32>,
    pub kind: SurfaceKind,
    /// `IDIB*` this surface copies back to on `eglSwapBuffers`/`eglCopyBuffers`, or `None` for a
    /// pbuffer that only exists in host memory. The reference layer writes the guest's own DIB, not
    /// the device the display happens to default to.
    pub target: Option<u32>,
    /// One byte per pixel, like the depth plane: the fixed-function stencil test reads and writes
    /// it before a fragment is allowed to reach the colour buffer.
    pub stencil: Vec<u8>,
    /// `eglSurfaceAttrib(EGL_SWAP_BEHAVIOR, ...)`; `QuerySurface` returns this value.
    pub swap_behavior: u32,
    /// `EGL_TEXTURE_FORMAT` used by `BindTexImage` (RGB or RGBA).
    pub texture_format: u32,
    /// `EGL_TEXTURE_TARGET` used by `BindTexImage` (currently only `EGL_TEXTURE_2D`).
    pub texture_target: u32,
    /// Guest allocation lazily created for `IEGLGetColorBuffer` on a pbuffer. Window/pixmap
    /// surfaces use `target` directly instead.
    pub color_buffer: Option<u32>,
}

fn cube_face(target: u32) -> Option<usize> {
    if (GL_TEXTURE_CUBE_MAP_POSITIVE_X..=GL_TEXTURE_CUBE_MAP_NEGATIVE_Z).contains(&target) {
        Some((target - GL_TEXTURE_CUBE_MAP_POSITIVE_X) as usize)
    } else {
        None
    }
}

#[derive(Clone)]
struct Texture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    /// Whether `rgba` came from the pack. A name that `GenTextures` handed out and
    /// `BindTexture` named holds a placeholder until an image arrives; an incomplete texture
    /// samples as opaque black rather than as that placeholder. `--gl-log`'s `img=` field makes
    /// the two cases distinguishable.
    uploaded: bool,
    /// Internal format named when level zero was uploaded; used when binding an EGL surface.
    internal: u32,
    /// Levels 1..n of the mip chain. Level 0 lives in `rgba`; each entry is RGBA8 for
    /// `max(1, width >> level) × max(1, height >> level)`.
    levels: Vec<Vec<u8>>,
    /// Depth and stencil planes for textures created with a depth internal format.
    /// A colour texture leaves both empty.
    depth: Vec<Vec<f32>>,
    stencil: Vec<Vec<u8>>,
    /// Six RGBA faces for a cube map; empty for a 2D texture. Only level zero is stored here;
    /// higher cube levels live in `cube_levels`.
    faces: Vec<Vec<u8>>,
    /// Cube-map levels 1..n, each holding six faces.
    cube_levels: Vec<Vec<Vec<u8>>>,
    /// Cube-map depth and stencil planes, indexed by level then face.
    cube_depth: Vec<Vec<Vec<f32>>>,
    cube_stencil: Vec<Vec<Vec<u8>>>,
    mag: u32,
    min: u32,
    wrap_s: u32,
    wrap_t: u32,
    max_anisotropy: f32,
}

// ---- EGL ------------------------------------------------------------------------------------

// ---- GLES ------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Vertex {
    pos: Vec3,
    /// The fourth position component. A `VertexPointer` of size four carries it; the
    /// immediate-mode and smaller array forms leave it at one, which is what the fixed
    /// function pipeline assumes when it widens a position to clip space.
    w: f32,
    color: [f32; 4],
    tex: [[f32; 2]; 4],
    /// Third texture coordinate per unit, used by cube-map sampling.
    tex_r: [f32; 4],
    /// Fourth texture coordinate per unit. The lookup divides s, t and r by it, so a pack that
    /// never sets it keeps the coordinates it wrote.
    tex_q: [f32; 4],
    normal: Vec3,
    point_size: f32,
    /// `GL_OES_matrix_palette`: up to four matrix indices and their weights.
    matrix_indices: [u16; 4],
    weights: [f32; 4],
    matrix_count: u32,
}

#[derive(Clone)]
struct MatrixStack {
    stack: Vec<Mat4>,
    current: Mat4,
}

#[derive(Clone, Copy)]
struct Array {
    ptr: u32,
    size: i32,
    ty: u32,
    stride: i32,
}

#[derive(Default, Clone, Copy)]
struct ClientArrays {
    vertex: Option<Array>,
    color: Option<Array>,
    normal: Option<Array>,
    point_size: Option<Array>,
    matrix_index: Option<Array>,
    weight: Option<Array>,
}

/// One fixed-function light, in eye space. The defaults are the ones `glLight` starts with:
/// light 0 is white and points down -z from the viewer, the other seven are off and black.
#[derive(Clone, Copy)]
struct Light {
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
    position: [f32; 4],
    spot_direction: [f32; 3],
    spot_exponent: f32,
    spot_cutoff: f32,
    constant_attenuation: f32,
    linear_attenuation: f32,
    quadratic_attenuation: f32,
    enabled: bool,
}

#[derive(Clone, Copy)]
struct Material {
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
    emission: [f32; 4],
    shininess: f32,
}

pub struct Gles {
    /// The extension/version surface the current device profile advertises.
    caps: crate::device::GlCaps,
    clear_color: [f32; 4],
    clear_depth: f32,
    viewport: [i32; 4],
    scissor: [i32; 4],
    scissor_test: bool,
    depth_range: [f32; 2],
    depth_test: bool,
    depth_func: u32,
    depth_mask: bool,
    blend: bool,
    logic_op: u32,
    logic_op_enabled: bool,
    sample_coverage: bool,
    sample_coverage_value: f32,
    sample_coverage_invert: bool,
    blend_src: u32,
    blend_dst: u32,
    blend_src_alpha: u32,
    blend_dst_alpha: u32,
    blend_equation: u32,
    blend_equation_alpha: u32,
    cull: bool,
    cull_face: u32,
    front_face: u32,
    texture_2d: [bool; 4],
    alpha_test: bool,
    alpha_func: u32,
    alpha_ref: f32,
    color_mask: [bool; 4],
    stencil_test: bool,
    stencil_func: u32,
    stencil_ref: i32,
    stencil_value_mask: u32,
    stencil_write_mask: u32,
    stencil_fail: u32,
    stencil_pass_depth_fail: u32,
    stencil_pass_depth_pass: u32,
    clear_stencil: i32,
    lighting: bool,
    normalize: bool,
    rescale_normal: bool,
    two_sided: bool,
    color_material: bool,
    /// `glColorMaterial`'s face and mode. Only the front face is lit by this pipeline, so a
    /// back-face choice is accepted but has nothing distinct to apply it to.
    color_material_face: u32,
    color_material_mode: u32,
    lights: [Light; 8],
    material: Material,
    back_material: Material,
    light_model_ambient: [f32; 4],
    fog: bool,
    fog_mode: u32,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    fog_color: [f32; 4],
    point_size: f32,
    point_size_min: f32,
    point_size_max: f32,
    point_fade_threshold: f32,
    point_distance_attenuation: [f32; 3],
    point_sprite: bool,
    point_sprite_coord_replace: bool,
    local_viewer: bool,
    matrix_palette: bool,
    current_palette_matrix: usize,
    palette_matrices: [Mat4; 48],
    line_width: f32,
    polygon_offset_factor: f32,
    polygon_offset_units: f32,
    polygon_offset_fill: bool,
    shade_model: u32,
    /// `TexGen` state for S/T/R/Q: whether the coordinate is generated and the object/eye
    /// plane it comes from.
    tex_gen_enabled: [bool; 4],
    tex_gen_mode: [u32; 4],
    tex_gen_plane: [[[f32; 4]; 2]; 4],
    /// Row pitch for pixel transfers: `TexImage2D` reads with unpack alignment and
    /// `ReadPixels` writes with pack alignment, exactly as `glPixelStorei` sets them.
    pack_alignment: u32,
    unpack_alignment: u32,
    /// `glHint` modes, in the order perspective correction, point smooth, line smooth, fog,
    /// generate mipmap. A hint is advice to the implementation, so nothing in the pipeline
    /// branches on these — but a pack reads them back and picks its next move from the answer,
    /// which is why the value has to be kept instead of dropped.
    hints: [u32; 5],
    /// Guest addresses of the four `glGetString` answers, filled on first use and handed out
    /// again afterwards: the string is the implementation's storage, not the caller's.
    string_cache: [u32; 4],
    matrices: [MatrixStack; 3],
    mode: usize,
    current: Vertex,
    vertices: Vec<Vertex>,
    primitive: u32,
    /// Texture objects are named globally, as in GL; each unit only remembers which name it
    /// has bound. A texture created in one unit is the same object in every other unit.
    textures: HashMap<u32, Texture>,
    next_texture: u32,
    bound_texture: [u32; 4],
    bound_target: [u32; 4],
    tex_env_mode: [u32; 4],
    tex_env_color: [[f32; 4]; 4],
    combine_rgb: [u32; 4],
    combine_alpha: [u32; 4],
    src_rgb: [[u32; 3]; 4],
    src_alpha: [[u32; 3]; 4],
    operand_rgb: [[u32; 3]; 4],
    operand_alpha: [[u32; 3]; 4],
    rgb_scale: [f32; 4],
    alpha_scale: [f32; 4],
    active_texture: usize,
    client_active_texture: usize,
    client_tex: [Option<Array>; 4],
    client: ClientArrays,
    /// User clip planes, in eye space, as `IGLES11`'s `ClipPlanef`/`ClipPlanex` leave them.
    clip_planes: [[f32; 4]; 6],
    clip_plane_enabled: [bool; 6],
    /// Buffer objects: the guest buffer id maps to the guest address `BufferData` copied the
    /// bytes to, plus its size and usage. `bound_buffer` holds those addresses for the two
    /// targets, or zero when the client arrays name guest memory directly.
    buffers: HashMap<u32, Buffer>,
    bound_buffer: [u32; 2],
    bound_buffer_id: [u32; 2],
    ati_mesh: AtiMesh,
    next_buffer: u32,
    /// `GL_OES_framebuffer_object`: an FBO renders into `surface` and writes the result back to
    /// the texture named by `color` when the draw finishes.
    framebuffers: HashMap<u32, Framebuffer>,
    bound_framebuffer: u32,
    next_framebuffer: u32,
    /// Renderbuffers are depth/stencil storage; the FBO maps its attachments to their ids.
    renderbuffers: HashMap<u32, Renderbuffer>,
    bound_renderbuffer: u32,
    next_renderbuffer: u32,
    error: u32,
    /// The `--gl-log` probe for the draw being issued: what it was told, and what it wrote. `None`
    /// unless the flag named it, and taken back by the caller as soon as the draw returns.
    probe: Option<Box<diag::DrawProbe>>,
}

#[derive(Clone, Copy)]
struct Buffer {
    addr: u32,
    size: u32,
    usage: u32,
}

#[derive(Clone, Copy)]
struct AtiMeshList {
    mode: u32,
    count: u32,
    ty: u32,
    indices: u32,
}

#[derive(Default)]
struct AtiMesh {
    lists: Vec<AtiMeshList>,
}

/// The object namespaces shared by contexts in one EGL share group. Bindings and
/// framebuffers stay per-context; textures and buffers are the objects a share context shares.
pub(crate) struct SharedGlObjects {
    textures: HashMap<u32, Texture>,
    next_texture: u32,
    buffers: HashMap<u32, Buffer>,
    next_buffer: u32,
    renderbuffers: HashMap<u32, Renderbuffer>,
    next_renderbuffer: u32,
}

struct Framebuffer {
    /// Colour texture id, or zero when this framebuffer has no colour attachment.
    color: u32,
    /// The mip level the colour texture is attached at; zero for the base level.
    color_level: u32,
    /// The texture target the colour attachment uses: 2D or one cube-map face.
    color_target: u32,
    surface: Surface,
    depth: Option<u32>,
    stencil: Option<u32>,
    depth_texture: Option<(u32, u32)>,
    stencil_texture: Option<(u32, u32)>,
    depth_target: u32,
    stencil_target: u32,
}

#[derive(Clone)]
struct Renderbuffer {
    format: u32,
    width: u32,
    height: u32,
    /// Storage for the two attachment kinds. A depth/stencil renderbuffer carries both; a
    /// colour-only or format-less one simply leaves the other plane empty.
    depth: Vec<f32>,
    stencil: Vec<u8>,
}

/// One draw landing place, taken out of an FBO or the EGL drawable so the rasteriser can
/// borrow it. The renderbuffer ids are the attachments whose planes must be copied back when
/// the draw finishes; the screen has neither and keeps its own planes.
struct DrawTarget {
    surface: Surface,
    color: u32,
    color_level: u32,
    color_target: u32,
    framebuffer: u32,
    depth: Option<u32>,
    stencil: Option<u32>,
    depth_texture: Option<(u32, u32)>,
    stencil_texture: Option<(u32, u32)>,
    depth_target: u32,
    stencil_target: u32,
}

#[derive(Clone, Copy)]
struct TxVertex {
    screen: Vec3,
    eye: Vec3,
    inv_w: f32,
    color: [f32; 4],
    /// The same vertex shaded with the back material, chosen by triangle winding in `raster`.
    back_color: [f32; 4],
    tex: [[f32; 2]; 4],
    tex_r: [f32; 4],
    /// Fourth texture coordinate per unit. The lookup divides s, t and r by it, so a pack that
    /// never sets it keeps the coordinates it wrote.
    tex_q: [f32; 4],
    fog: f32,
    point_size: f32,
}

/// A vertex still in clip space, kept so a triangle can be cut against the near plane before the
/// perspective divide. Clipping after the divide would lose any triangle crossing `w == 0`.
#[derive(Clone, Copy)]
struct ClipVertex {
    clip: Vec4,
    /// Eye-space position, which is the space `ClipPlanef` equations live in.
    eye: Vec3,
    color: [f32; 4],
    back_color: [f32; 4],
    tex: [[f32; 2]; 4],
    tex_r: [f32; 4],
    /// Fourth texture coordinate per unit. The lookup divides s, t and r by it, so a pack that
    /// never sets it keeps the coordinates it wrote.
    tex_q: [f32; 4],
    fog: f32,
    point_size: f32,
}

/// Screen-space LOD: the largest texel footprint of the triangle's `u`/`v` derivatives,
/// expressed as log2. This is the standard finite-difference estimate, taken once per triangle.
mod helpers;

#[cfg(test)]
#[path = "../test/gles.rs"]
mod tests;
