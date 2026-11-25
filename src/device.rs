//! The device profile: what this emulated handset says about itself.
//!
//! A BREW module never reads a config file. It learns the screen size, the colour depth, the
//! close-key code and the heap budget from `IShell`, so this module is where those numbers live
//! and the only place that knows how to put them into a guest `AEEDeviceInfo`. Every byte position
//! comes from [`crate::abi`]'s generated tables rather than from a literal, which is what keeps a
//! 64-byte struct ending in a nine-bit bitfield run honest after a header change.
//!
//! The default profile uses the values the a reference front end came with (240x320, RGB565, the
//! `AVK_*` keypad, a 64 MiB advertised heap) because those are the numbers the sample applets were
//! tuned against. None of them are ABI: they are choices, and a real target would read its own.

use crate::abi::{self, generated as g, StructInfo};
use crate::encoding::TextEncoding;
use crate::hle::HResult;
use crate::mem::Mem;

/// `#define RGB_NONE (0xffffffffL)` from the header. It has two documented meanings:
/// `IDisplay_SetColor` reads it as "give me the current value and change nothing", and
/// `IDisplay_DrawRect` reads it as "use the active colour for this part" — which is how
/// `IDisplay_ClearScreen` can pass it as the fill colour and still clear the screen.
/// It can never collide with a real colour, because `MAKE_RGB` leaves the alpha byte zero.
pub const RGB_NONE: u32 = 0xffff_ffff;

/// `AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO`: the handset's own number, as The carrier's
/// `ISHELL_GetDeviceInfoEx` answers it — the "EZ number" the handset is subscribed under.
///
/// It lives here rather than in [`crate::abi::generated`] because that table is built from the
/// public interface declarations and this item is a carrier extension; and not in [`crate::abi::jp`] because
/// that file is generated from the au private headers, where a hand-added value would be dropped by
/// the next regeneration. The one record is a reference front end's `a carrier extension header`, which
/// contains this constant and nothing else.
///
/// A pack asks for it early. `w62ca_new-ex-mahjong` calls
/// `IShell_GetDeviceInfoEx(0x58f1, NULL, &n)` — the null-buffer form that asks *how many bytes* —
/// while initialising its Carrier system, and gives up on the whole applet when the answer is
/// `EUNSUPPORTED`: it printed `INITKDDISYSTEM` and never reached `EVT_APP_START`.
pub const AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO: u32 = 0x58f1;

/// The first item in the carrier-defined range: the device declarations puts
/// `AEE_DEVICEITEM_CARRIER_BEGIN` at `0x58ef`, so `0x58f0` is a carrier's own numbering and no
/// published header names it or sizes its payload. Nothing in this tree records a name for it
/// either — the one third-party host that answers `0x58f1` answers only that one, and the handset
/// images carry the id inside a run of ascending ids with no name and no payload beside it.
///
/// What the id *does* record is that a pack asks whether the item is there at all, with the
/// null-buffer size query, and treats `EUNSUPPORTED` as "this handset cannot run me": two packs
/// from different publishers query it while initialising and abandon every one of their class ids
/// when the answer is a refusal. So this host answers the item as **present and empty** — one
/// NUL byte, which is what a size query then reports as a one-byte item.
///
/// Answered empty rather than filled with a plausible-looking value: the payload is unrecorded,
/// and a made-up body would be read as fact by the caller. The two packs measured here are
/// indifferent to the body — an empty answer and a ten-digit numeric string produce the same
/// applet, the same retired-instruction count and the same number of drawing calls — so presence
/// is what is actually being asked for. If a pack is later found to read this item for a purpose,
/// that purpose is the evidence this constant is waiting for, and the answer belongs here rather
/// than in a per-pack branch.
pub const AEE_DEVICEITEM_CARRIER_FIRST: u32 = 0x58f0;

/// `MAKE_RGB(r, g, b)` from the same header. BREW's `RGBVAL` is *not* the usual `0x00RRGGBB`:
/// red occupies bits 8..15, green 16..23, blue 24..31, and the low byte is the alpha slot
/// `MAKE_RGBA` fills. Reading a captured screen back as if it were `0xRRGGBB` rotates the
/// channels, so the packing lives here and nowhere else.
pub const fn make_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 8) | ((g as u32) << 16) | ((b as u32) << 24)
}

/// Inverse of [`make_rgb`]. The low byte is dropped: it is alpha, and the display has none.
pub const fn split_rgb(v: u32) -> (u8, u8, u8) {
    (
        ((v >> 8) & 0xff) as u8,
        ((v >> 16) & 0xff) as u8,
        ((v >> 24) & 0xff) as u8,
    )
}

pub const RGB_BLACK: u32 = 0;
pub const RGB_WHITE: u32 = make_rgb(255, 255, 255);
/// `RGB_MASK_COLOR`: magenta, the transparent colour of a colour display's bitmaps. Monochrome
/// and greyscale devices use white instead.
pub const RGB_MASK_COLOR: u32 = make_rgb(255, 0, 255);

/// The nine `flg b*:1` members of `AEEDeviceInfo`, held as named booleans because which bit each
/// one occupies is table data, not something to hand-transcribe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceFlags {
    /// An alternate display (a pager), which is what makes `cxAltScreen`/`cyAltScreen` mean
    /// anything.
    pub alt_display: bool,
    /// Sends `EVT_FLIP` when the clamshell opens or closes.
    pub flip: bool,
    pub vibrator: bool,
    pub ext_speaker: bool,
    pub voice_recognition: bool,
    pub position_location: bool,
    pub midi: bool,
    pub cmx: bool,
    pub pen: bool,
}

/// Reads one `b*:1` flag out of a [`DeviceFlags`].
type Flag = fn(&DeviceFlags) -> bool;

impl DeviceFlags {
    /// `(AEEDeviceInfo member, is it set)`, in the order the struct declares them, so the
    /// packing run is walked the way the compiler laid it out.
    const MEMBERS: &'static [(&'static str, Flag)] = &[
        ("bAltDisplay", |f| f.alt_display),
        ("bFlip", |f| f.flip),
        ("bVibrator", |f| f.vibrator),
        ("bExtSpeaker", |f| f.ext_speaker),
        ("bVR", |f| f.voice_recognition),
        ("bPosLoc", |f| f.position_location),
        ("bMIDI", |f| f.midi),
        ("bCMX", |f| f.cmx),
        ("bPen", |f| f.pen),
    ];
}

/// The Q3D EGL `IEGLSurfaceManip` capabilities the display controller reports.
///
/// These are hardware/board facts, not GLES versions: the One handset's
/// `eglGetSurfaceOverlayCapsQUALCOMM` writes 1 overlay, 1 underlay, 16 surfaces per layer and 33
/// total; older firmware such as the W64S does not expose the extension at all. A profile that
/// names an older handset therefore sets [`OverlayCaps::none`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayCaps {
    /// Whether the `IEGLSurfaceManip` extension exists on this device.
    pub present: bool,
    /// Number of overlay layers above the base (`layer = 1..=max_overlay`).
    pub max_overlay: u8,
    /// Number of underlay layers below the base (`layer = -1..=-max_underlay`).
    pub max_underlay: u8,
    pub max_surface_per_layer: u16,
    pub max_total_surfaces: u16,
    pub pbuffer: bool,
    pub pixmap: bool,
}

impl OverlayCaps {
    pub const fn none() -> Self {
        OverlayCaps {
            present: false,
            max_overlay: 0,
            max_underlay: 0,
            max_surface_per_layer: 0,
            max_total_surfaces: 0,
            pbuffer: false,
            pixmap: false,
        }
    }

    /// The values One handset's `eglGetSurfaceOverlayCapsQUALCOMM` writes.
    pub const fn w64sa() -> Self {
        OverlayCaps {
            present: true,
            max_overlay: 1,
            max_underlay: 1,
            max_surface_per_layer: 16,
            max_total_surfaces: 33,
            pbuffer: true,
            pixmap: true,
        }
    }
}

impl Default for OverlayCaps {
    fn default() -> Self {
        Self::w64sa()
    }
}

/// The GLES/EGL extension surface a device profile advertises.
///
/// This is the generic-route counterpart of [`OverlayCaps`]: the software backend can implement
/// every face, but a profile may say that the handset's firmware never exposed a face. QEGL
/// `QueryInterface`, `IGL_QueryInterface(AEEIID_GLES10)` and `GL_EXTENSIONS` all read these flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlCaps {
    /// The plain `AEECLSID_GL` class exists (the `IGL` Common-Lite face).
    pub igl: bool,
    /// The `AEECLSID_QEGL` family exists (its IEGL/GLES faces are reachable by `QueryInterface`).
    pub qegl: bool,
    /// `IGLES10` (the Common 1.0 face) is reachable from `IGL`.
    pub gles10: bool,
    /// `IGLES11`.
    pub gles11: bool,
    /// `IGLES10Ext` (`GL_OES_query_matrix`).
    pub gles10_ext: bool,
    /// `IGLES11Ext` (draw texture, point sprite, matrix palette, ...).
    pub gles11_ext: bool,
    /// `IGLES11ExtPak` (framebuffer object, blend equations, texgen, ...).
    pub gles11_ext_pak: bool,
    /// `IGLESImageonExt` / `IGLESImageonExt_V1`.
    pub imageon_ext: bool,
    pub imageon_ext_v1: bool,
    /// `GL_VENDOR`, `GL_RENDERER`, `GL_VERSION` and `GL_EXTENSIONS` as the firmware reports them.
    pub vendor: String,
    pub renderer: String,
    pub version: String,
    pub extensions: String,
    /// The implementation limits `GetIntegerv` reports and the texture entry points enforce.
    ///
    /// [`GlCaps::all`] describes the software backend. [`GlCaps::w64sa`] replaces them with the
    /// values read out of that firmware's own Q3D driver, so a pack sees the same answers it saw
    /// on the handset.
    pub max_texture_size: u32,
    /// `GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT`. Sixteen is what both presets can show; a profile that
    /// reads a smaller number out of its own driver narrows this and the setter follows.
    pub max_texture_max_anisotropy: u32,
    pub max_lights: u32,
    pub max_texture_units: u32,
    pub max_palette_matrices: u32,
    pub max_vertex_units: u32,
    /// `GL_MAX_MODELVIEW_STACK_DEPTH`, `GL_MAX_PROJECTION_STACK_DEPTH` and
    /// `GL_MAX_TEXTURE_STACK_DEPTH`.
    pub max_modelview_stack_depth: u32,
    pub max_projection_stack_depth: u32,
    pub max_texture_stack_depth: u32,
    /// `GL_ALIASED_POINT_SIZE_RANGE` / `GL_POINT_SIZE_RANGE` endpoints.
    pub point_size_min: u32,
    pub point_size_max: u32,
    /// `GL_ALIASED_LINE_WIDTH_RANGE` / `GL_LINE_WIDTH_RANGE` endpoints.
    pub line_width_min: u32,
    pub line_width_max: u32,
    /// `GL_COMPRESSED_TEXTURE_FORMATS`, in the order the profile reports them.
    pub compressed_texture_formats: Vec<u32>,
    /// `GL_SUBPIXEL_BITS`.
    pub subpixel_bits: u32,
    /// `GL_RED_BITS`, `GL_GREEN_BITS`, `GL_BLUE_BITS`, `GL_ALPHA_BITS` for the current surface.
    pub red_bits: u32,
    pub green_bits: u32,
    pub blue_bits: u32,
    pub alpha_bits: u32,
    /// `GL_DEPTH_BITS` and `GL_STENCIL_BITS`.
    pub depth_bits: u32,
    pub stencil_bits: u32,
    /// `GL_MAX_ELEMENTS_VERTICES` / `GL_MAX_ELEMENTS_INDICES`.
    pub max_elements_vertices: u32,
    pub max_elements_indices: u32,
    /// `GL_IMPLEMENTATION_COLOR_READ_FORMAT_OES` and `_TYPE_OES`.
    pub read_format: u32,
    pub read_type: u32,
}

const HOST_EXTENSIONS: &str = "GL_ATI_texture_compression_atitc GL_IMG_texture_compression_pvrtc GL_OES_read_format GL_OES_compressed_paletted_texture GL_OES_query_matrix GL_OES_vertex_buffer_object GL_OES_draw_texture GL_OES_compressed_ETC1_RGB8_texture GL_OES_framebuffer_object GL_OES_blend_subtract GL_OES_blend_func_separate GL_OES_blend_equation_separate GL_OES_point_size_array GL_OES_texture_npot GL_OES_point_sprite GL_OES_matrix_palette GL_OES_texture_mirrored_repeat GL_OES_texture_cube_map GL_EXT_texture_filter_anisotropic";

// a carrier Q3D string block.
const W64SA_EXTENSIONS: &str = "GL_ARB_texture_env_combine GL_ARB_texture_env_crossbar GL_ARB_texture_env_dot3 GL_ARB_texture_mirrored_repeat GL_ARB_vertex_buffer_object GL_ATI_extended_texture_coordinate_data_formats GL_ATI_imageon_misc GL_ATI_texture_compression_atitc GL_EXT_blend_equation_separate GL_EXT_blend_func_separate GL_EXT_blend_minmax GL_EXT_blend_subtract GL_EXT_stencil_wrap GL_OES_byte_coordinates GL_OES_compressed_paletted_texture GL_OES_draw_texture GL_OES_fixed_point GL_OES_matrix_palette GL_OES_point_size_array GL_OES_point_sprite GL_OES_read_format GL_OES_single_precision GL_OES_vertex_buffer_object GL_QUALCOMM_vertex_buffer_object";

impl GlCaps {
    /// The host's current software surface: every face it can build is offered.
    pub fn all() -> Self {
        GlCaps {
            igl: true,
            qegl: true,
            gles10: true,
            gles11: true,
            gles10_ext: true,
            gles11_ext: true,
            gles11_ext_pak: true,
            imageon_ext: true,
            imageon_ext_v1: true,
            vendor: "QuoinHLE".to_string(),
            renderer: "QuoinHLE software GLES".to_string(),
            version: "OpenGL ES-CM 1.0".to_string(),
            extensions: HOST_EXTENSIONS.to_string(),
            max_texture_size: 2048,
            max_texture_max_anisotropy: 16,
            max_lights: 8,
            max_texture_units: 4,
            max_palette_matrices: 32,
            max_vertex_units: 4,
            max_modelview_stack_depth: 32,
            max_projection_stack_depth: 32,
            max_texture_stack_depth: 32,
            point_size_min: 1,
            point_size_max: 64,
            line_width_min: 1,
            line_width_max: 64,
            compressed_texture_formats: {
                // The paletted set first, then the decoders this host carries: ATC, ETC1 and
                // PVRTC. A profile that lists only the paletted formats refuses the rest.
                let mut formats: Vec<u32> = (0x8B90..=0x8B99).collect();
                formats.extend(0x8C92..=0x8C94);
                formats.push(0x8D64);
                formats.extend(0x8C00..=0x8C03);
                formats
            },
            subpixel_bits: 2,
            red_bits: 5,
            green_bits: 6,
            blue_bits: 5,
            alpha_bits: 0,
            depth_bits: 16,
            stencil_bits: 8,
            max_elements_vertices: 65536,
            max_elements_indices: 65536,
            read_format: 0x1908, // GL_RGBA
            read_type: 0x1401,   // GL_UNSIGNED_BYTE
        }
    }

    /// The strings the One handset's Q3Dimension MSM7500 GL returns. Its QEGL IIDs are all
    /// present; the extension string is narrower than the host's software surface.
    ///
    /// The numeric limits are read out of the firmware's own `glGetIntegerv`, whose entries are
    /// reached through the `gl*` name strings the tooling wrapper carries rather than through a
    /// fixed address: 1024 for `GL_MAX_TEXTURE_SIZE`, 8 for
    /// `GL_MAX_LIGHTS`, 2 for `GL_MAX_TEXTURE_UNITS`, 48 for
    /// `GL_MAX_PALETTE_MATRICES_OES`, 4 for `GL_MAX_VERTEX_UNITS_OES`, stack depths 16/2/2,
    /// point size `[1, 32]`, line width `[1, 1]`, and the ten palette compressed formats at
    /// `0x8B90..=0x8B99`. The pixel-format branches read the display object: for the 16bpp path
    /// that is 5/6/5/0 with a 16-bit depth buffer, and the stencil field is the object's zeroed
    /// default (the image never parses `EGL_STENCIL_SIZE`). Readback is RGB in 5-6-5, and
    /// `GL_MAX_ELEMENTS_VERTICES`/`_INDICES` are 65536. `GL_MAX_VIEWPORT_DIMS` is not a constant
    /// there — it reports the current display surface's width and height — and the host answers it
    /// the same way, so there is no profile field for it.
    pub fn w64sa() -> Self {
        GlCaps {
            igl: true,
            qegl: true,
            gles10: true,
            gles11: true,
            gles10_ext: true,
            gles11_ext: true,
            gles11_ext_pak: true,
            imageon_ext: true,
            imageon_ext_v1: true,
            vendor: "QUALCOMM, Inc.".to_string(),
            renderer: "Q3Dimension MSM7500 01.02.32 0 4.0.0".to_string(),
            version: "OpenGL ES 1.0-CM".to_string(),
            extensions: W64SA_EXTENSIONS.to_string(),
            max_texture_size: 1024,
            max_texture_max_anisotropy: 16,
            max_lights: 8,
            max_texture_units: 2,
            max_palette_matrices: 48,
            max_vertex_units: 4,
            max_modelview_stack_depth: 16,
            max_projection_stack_depth: 2,
            max_texture_stack_depth: 2,
            point_size_min: 1,
            point_size_max: 32,
            line_width_min: 1,
            line_width_max: 1,
            compressed_texture_formats: (0x8B90..=0x8B99).collect(),
            subpixel_bits: 2,
            red_bits: 5,
            green_bits: 6,
            blue_bits: 5,
            alpha_bits: 0,
            depth_bits: 16,
            stencil_bits: 0,
            max_elements_vertices: 65536,
            max_elements_indices: 65536,
            read_format: 0x1907, // GL_RGB
            read_type: 0x8363,   // GL_UNSIGNED_SHORT_5_6_5
        }
    }

    /// No GL or QEGL class at all, as the W64S image shows (no QEGL/GLES IID or GL strings).
    pub fn none() -> Self {
        GlCaps {
            igl: false,
            qegl: false,
            gles10: false,
            gles11: false,
            gles10_ext: false,
            gles11_ext: false,
            gles11_ext_pak: false,
            imageon_ext: false,
            imageon_ext_v1: false,
            vendor: String::new(),
            renderer: String::new(),
            version: String::new(),
            extensions: String::new(),
            max_texture_size: 0,
            max_texture_max_anisotropy: 0,
            max_lights: 0,
            max_texture_units: 0,
            max_palette_matrices: 0,
            max_vertex_units: 0,
            max_modelview_stack_depth: 0,
            max_projection_stack_depth: 0,
            max_texture_stack_depth: 0,
            point_size_min: 0,
            point_size_max: 0,
            line_width_min: 0,
            line_width_max: 0,
            compressed_texture_formats: Vec::new(),
            subpixel_bits: 0,
            red_bits: 0,
            green_bits: 0,
            blue_bits: 0,
            alpha_bits: 0,
            depth_bits: 0,
            stencil_bits: 0,
            max_elements_vertices: 0,
            max_elements_indices: 0,
            read_format: 0,
            read_type: 0,
        }
    }
}

impl Default for GlCaps {
    fn default() -> Self {
        Self::all()
    }
}

/// A span of the handset's address space the host brings in zeroed before the pack runs.
///
/// A pack that came with its own save data carries absolute pointers into the region the
/// firmware's writable memory lived in — `0x01FA2B68` in `yongman2`'s `save.dat`, `0x01FF0010`
/// across four versions of 영웅서기, `0x01B2BA18` in 월드오브드래곤. On the handset those addresses
/// are real, writable memory; on this host they fall in the hole between the stub page and the
/// frames this host owns, so the first `ldr` from one is a data abort.
///
/// Mapping the span does not put the saved objects back — they lived in another process's heap and
/// nothing here can invent them — but it makes the address *exist*. A pack that follows a stale
/// pointer, reads zeroes, and then rebuilds its own state gets a chance; a pack that genuinely
/// needs the object still fails, and it will fail at the object rather than at the save file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamWindow {
    pub base: u32,
    pub len: u32,
}

/// Which fields a handset wrote down itself, out of the ones a generation or a carrier also has an
/// opinion about.
///
/// A field like [`DeviceProfile::text_bytes`] has up to three possible sources: the handset's own
/// device file, the BREW release the pack was built for, and the operator whose firmware it reference
/// on. The *value* cannot say which one won — a handset that declares exactly the generation's
/// number is indistinguishable from one that said nothing — and the two must resolve differently as
/// soon as the generation changes. So each field carries a bit, and only the fields whose bit is
/// clear are filled from the generation or carrier.
///
/// Every bit is clear in [`DeviceProfile::standard`]: an undeclared handset is the host's own
/// defaults, which is what "no device file, no flag" means. A device file sets the bits for the keys
/// it actually names, and a field written after loading (a `--gl-preset` style overlay) is expected
/// to set its own bit if it means to survive [`crate::spec::Resolved::apply_to`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Declared {
    /// `abi.device_info_size` — `sizeof(AEEDeviceInfo)` for this handset.
    pub device_info_size: bool,
    /// `text.text_bytes` — `DrawText`'s `pcText` read as the device's 8-bit string.
    pub text_bytes: bool,
    /// `text.encoding` — the `AEE_ENC_*` character set `AECHAR` values are in.
    pub encoding: bool,
    /// `text.font_normal` — the cell height the host rasterizes for `AEE_FONT_NORMAL`.
    pub font_normal: bool,
    /// `text.font_large` — the cell height the host rasterizes for `AEE_FONT_LARGE`.
    pub font_large: bool,
    /// `memory.windows` — spans of the handset's address space this pack's saves point into.
    pub ram_windows: bool,
    /// `system.heap_premap` — how much of the heap's span is handset RAM from the first instruction.
    pub heap_premap: bool,
    /// `system.object_premap` — how much of the host-object region is handset RAM from the start.
    pub object_premap: bool,
    /// `loader.module_base` — where this handset links a `.mod` image.
    pub module_base: bool,
}

impl Declared {
    /// The keys this handset named, in the order a report lists them. Empty for an undeclared one.
    pub fn named(&self) -> Vec<&'static str> {
        [
            ("device_info_size", self.device_info_size),
            ("text_bytes", self.text_bytes),
            ("encoding", self.encoding),
            ("font_normal", self.font_normal),
            ("font_large", self.font_large),
            ("ram_windows", self.ram_windows),
            ("heap_premap", self.heap_premap),
            ("object_premap", self.object_premap),
            ("module_base", self.module_base),
        ]
        .into_iter()
        .filter_map(|(name, said)| said.then_some(name))
        .collect()
    }
}

/// Everything this handset will answer through `IShell::GetDeviceInfo` and `GetDeviceInfoEx`.
#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub width: u16,
    pub height: u16,
    /// Bits per pixel **the handset reports** — `AEEDeviceInfo::nColorDepth`, and with it what a
    /// pack plans around. The screen it actually writes may be narrower or wider; that is
    /// [`DeviceProfile::frame_bpp`], and a real handset keeps the two equal.
    pub color_depth: u16,
    /// Bits per pixel the host's screen really stores: 16 (RGB565) or 32 (`0x00RRGGBB` words).
    ///
    /// This is the depth `IDisplay::GetDestination`'s `IDIB` carries, the stride a pack computes
    /// from it, and the format the host writes when it draws. `color_depth` is only the number the
    /// firmware answers when asked; a profile that makes them differ is a diagnostic, and a pack
    /// that composites straight into `pBmp` will lay out for this one.
    pub frame_bpp: u16,
    /// Offset from a host `IBitmap` object's base to pixel (0, 0), i.e. the size of the firmware's
    /// DIB header. `DIB_NewEx` puts the pixels after its `IBitmap` record (and palette); this host
    /// publishes [`display::DIB_PIXEL_OFFSET`] by default. It has to be at least `sizeof(IDIB)`;
    /// alignment is whatever the firmware's allocator did, not a rule this host imposes.
    pub dib_pixel_offset: u32,
    /// Deepest off-screen DIB this firmware backs: 16 or 32. 8, 4 and 1 bpp are always backed.
    ///
    /// The screen can be 16 bpp while DIBs are 32: `DIB_NewEx` takes any non-negative
    /// `nPixelSize`. A pack that asks for 32 gets a real four-byte surface and the blitter converts
    /// it to the screen rather than the host refusing the request.
    pub dib_depth: u16,
    /// Offset in an `IGraphics` object where the host mirrors its destination `IBitmap*`.
    ///
    /// This is `m_pbmDst` in the reference 3.1.2 `QGraphics`, at `+20`. A pack that walks the
    /// firmware's private record instead of calling `GetDestination` reads this word; zero turns
    /// the mirror off. The object is laid out to cover the offset, and the refcount moves if the
    /// configured offset would collide with it.
    pub graphics_canvas_offset: u32,
    pub alt_width: u16,
    pub alt_height: u16,
    /// Width of the standard scroll bar, in pixels.
    pub scroll_bar: u16,
    /// `AEE_ENC_*`: the character set `AECHAR` values are in.
    pub encoding: u16,
    /// Whether `IDisplay::DrawText`'s `pcText` is the device's own 8-bit string rather than an
    /// `AECHAR` run.
    ///
    /// This is the operator's answer, not the release's: `The classic display declarations` already
    /// declares `const AECHAR *pcText` with `AECHAR` a `uint16`, and so does every tree after it, so
    /// no reference BREW release hands `DrawText` bytes. The carrier's 1.2.3 firmware does — its packs pass
    /// `char *` — so the same slot means different things on The carrier's handset and on the public one.
    pub text_bytes: bool,
    /// Cell height in pixels the host rasterizes for `AEE_FONT_NORMAL`.
    ///
    /// The guest never sees this number; it is the size of the bitmap face the host builds from
    /// the machine's own font. A carrier profile names 20 here and 24 for large; the built-in
    /// handset keeps 16 and 32, which is what the sample set was laid out against.
    pub font_normal: u16,
    /// Cell height in pixels the host rasterizes for `AEE_FONT_LARGE`.
    pub font_large: u16,
    /// Bytes between the initial `SP` and `STACK_TOP` reserved for the loader's frame.
    ///
    /// Carrier startup code reads a 0x400-byte load context upward from its own `SP`; an initial `SP`
    /// eight bytes below the top makes that read cross into unmapped memory on the first block.
    pub stack_reserve: u32,
    /// Address a `.mod` image links at, and therefore the base this host must map it at.
    ///
    /// A module is PIC, but guest packs still carry **absolute data pointers** baked in at their
    /// link address. The carrier's 1.2.3 loader puts a module at `0x8000`; au's 3.x/4.x firmware uses
    /// `0x1000_0000`. Loading a pack at the other address runs its code but faults when it follows
    /// one of those pointers, so this is a handset fact rather than a host choice. The platform's
    /// default fills it unless the device file says otherwise.
    pub module_base: u32,
    /// Q3D EGL overlay capabilities, or [`OverlayCaps::none`] when the firmware lacks the extension.
    pub overlay: OverlayCaps,
    /// GLES/EGL extension faces this firmware exposes.
    pub gl: GlCaps,
    /// Milliseconds per step of menu text auto-scroll.
    pub menu_text_scroll: u16,
    /// Milliseconds an icon-view image is held when `MP_ICON_DELAY_IMAGE` is set.
    pub menu_image_delay: u32,
    /// `dwRAM`: the size of the BREW heap a module is told it has to live in.
    ///
    /// A device file may declare its own in `system.ram` (bytes) and that always wins — the carrier
    /// profile names the guide's 256 KiB per-app ceiling, for instance. The built-in default is
    /// [`crate::layout::HEAP_DEFAULT`], because a phone that cannot hand out more than that and says
    /// it can is telling a pack to preallocate what will fail later.
    pub ram: u32,
    /// Default prompt properties.
    pub prompt_props: u32,
    pub key_close_app: u16,
    pub key_close_all_apps: u16,
    /// `AEE_LNG_*`, which the resource loader uses to pick a language block.
    pub lang: u32,
    pub net_linger: u32,
    pub sleep_defer: u32,
    pub max_path: u16,
    pub platform_id: u32,
    /// The `AEEDeviceInfo` size this handset's platform defines. BREW 1.x/2.x use 60 bytes; 3.x adds
    /// `dwPlatformID` and is 64. A pack allocates the struct on its own stack, so writing past this
    /// is a buffer overflow, not a harmless extra field. Zero means "the host's table size".
    ///
    /// Whether this number was written down for the handset, or is only standing in for the
    /// generation's, is [`Declared::device_info_size`] — the value alone cannot say.
    pub device_info_size: u16,
    /// The fields above that this handset decided for itself, rather than inheriting from the
    /// generation or carrier the run resolved to.
    pub declared: Declared,
    /// Spans of the handset's writable memory this host brings in zeroed, in order.
    ///
    /// A carrier supplies the ones its firmware is known to have (see
    /// [`crate::spec::Carrier::ram`]); a device file may name its own, and then they are the
    /// handset's. Empty means nothing is mapped beyond what the host itself owns, which is what
    /// every emulated handset has done so far.
    pub ram_windows: Vec<RamWindow>,
    /// How many bytes of the guest heap's span are writable from the very first instruction.
    ///
    /// A handset's heap is RAM whether or not the allocator has handed a block out yet. The host
    /// normally brings in only the initial free node and maps further pages as `Sbrk` grows — but a
    /// pack that indexes a buffer directly from a pointer in its heap context reads those pages
    /// without ever allocating them. The carrier's `yongman2` scrolls a 96 KiB map across a 12 MiB span
    /// this way; with a lazily mapped heap it faults at the first page past the high-water mark.
    /// Zero keeps the lazy behaviour. This is not `ram_windows`: those name addresses the host
    /// never uses, while these pages are inside the heap's own span and the allocator still owns
    /// them — the host just brings them in early.
    pub heap_premap: u32,
    /// How many bytes of the host-object region are writable RAM from the first instruction.
    ///
    /// The host hands out objects at [`crate::layout::OBJECT_BASE`] and normally maps only the page
    /// each object lands on. A handset's RAM is contiguous there, so a pack that walks past a host
    /// object — a format probe that scans a IBitmap as a word array with no bounds check — reads
    /// the bytes after it rather than faulting. The carrier's `splintercell` does exactly that: its first
    /// probe window runs 16 KiB past a 0xac-byte IBitmap and needs those pages present so the
    /// probe can fail and fall through to the element width that actually matches. Zero keeps the
    /// host's lazy mapping; a device file names the region's size when its firmware had one. This
    /// is not `heap_premap`: those pages are inside the allocator's span, these are the separate
    /// region the host hands interface objects out of.
    pub object_premap: u32,
    /// Whether this is a carrier build handset. The carrier's firmware serves classes the public BREW ABI
    /// does not name — the `0x018000fe` device-info class first — and this selects them without
    /// changing what `AEEDeviceInfo` reports. It is a host policy, not a guest-visible field.
    pub ktf: bool,
    pub flags: DeviceFlags,
    /// Identity strings, each reported through the `AEE_DEVICEITEM_*` documented for it.
    ///
    /// Owned rather than `&'static str`, which they were until [`crate::device_file`] gave them a
    /// source other than this file. A profile loaded from JSON carries text whose lifetime is the
    /// load, and the alternative — leaking it — would make "where does this string live" a question
    /// with two answers depending on how the run was started.
    pub chip_id: String,
    pub model_name: String,
    pub manufacturer: String,
    pub friendly_name: String,
    pub firmware_version: String,
    pub carrier_id: u32,
    pub esn: u32,
    /// Reported as a null-terminated ASCII string, 15 digits plus the terminator.
    pub imei: String,
    /// The number this handset is subscribed under, which The carrier reports through
    /// [`AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO`] — the "EZ number" printed on the contract.
    ///
    /// A string and not a number, for the same reason `imei` is: the caller is handed a buffer and
    /// its length, and the handsets these packs were built for answer eleven digits. Nothing here
    /// checks that the digits are a *reachable* number, because nothing could.
    pub subscriber_number: String,
    pub softkey_count: u8,
    /// One of `AEE_MANNER_MODE_*`.
    pub manner_mode: u32,
    /// `AEEREPEAT_SCROLLBAR` defaults: first repeat delay and subsequent rate, in ms.
    pub repeat_start: u32,
    pub repeat_rate: u32,
    /// the header's `KB_AUTOREPEAT_START`: how long after the first `EVT_KEY` a key that is
    /// still down starts repeating, in ms.
    pub key_repeat_start: u32,
    /// `KB_AUTOREPEAT_RATE`: the interval of those repeats, in ms. Each one arrives as `EVT_KEY`
    /// with `KB_AUTOREPEAT` set, which is how an app tells a held key from a fresh press. Not the
    /// pair above, which is a pen held beside a scrollbar thumb.
    pub key_repeat_rate: u32,
    pub storage_total: u64,
    pub storage_free: u64,
    /// Whether the handset's `fs:/` namespace folds ASCII case when it looks a name up.
    ///
    /// The BREW header describes a case-sensitive namespace, but The carrier's packs ship names such as
    /// `Img.bar` while the extracted file is `img.bar`; the handset they ran on resolved that, so
    /// this is a device fact, not a relaxation made for one pack. Default is false: exact spelling.
    pub fs_case_insensitive: bool,
    /// The key codes `AEE_DEVICEITEM_KEY_SUPPORT` answers `true` for: this is the emulated
    /// keypad, and a module that probes before drawing softkey labels reads it.
    pub keys: Vec<u16>,
}

impl Default for DeviceProfile {
    fn default() -> Self {
        DeviceProfile::standard()
    }
}

impl DeviceProfile {
    /// Apply a named GL/overlay family on top of this handset, as `--gl-preset` does.
    ///
    /// The names are firmware families, not handsets: `w64sa` advertises the Q3Dimension strings
    /// and numeric limits read out of that image, `w64s` has no QEGL/GLES classes or Q3D overlay,
    /// and `host` is the software backend's own surface.
    pub fn with_gl_preset(self, name: &str) -> Result<Self, String> {
        match name.to_ascii_lowercase().as_str() {
            "host" => Ok(DeviceProfile {
                gl: GlCaps::all(),
                ..self
            }),
            "w64sa" => Ok(DeviceProfile {
                gl: GlCaps::w64sa(),
                overlay: OverlayCaps::w64sa(),
                ..self
            }),
            "w64s" | "none" => Ok(DeviceProfile {
                gl: GlCaps::none(),
                overlay: OverlayCaps::none(),
                ..self
            }),
            _ => Err(format!("--gl-preset wants host, w64sa or w64s, got {name}")),
        }
    }

    /// A 240x320 RGB565 candybar handset.
    pub fn standard() -> Self {
        DeviceProfile {
            width: 240,
            height: 320,
            // The depth of the screen this handset actually hands out, because a module is entitled to
            // write that screen without asking again. `IDisplay::GetDestination` returns an `IDIB` that
            // says 16 and `IDIB_COLORSCHEME_565`; if this word disagrees, the two answers are not one
            // device, and a pack that composites straight into `pBmp` trusts this one instead and lays
            // three bytes where the host reads two. a reference front end's `OEM_GetDeviceInfo` answers 24, but its
            // own screen is a 32-bit Android surface, so on that host the two answers do agree.
            color_depth: 16,
            frame_bpp: 16,
            dib_pixel_offset: crate::display::DIB_PIXEL_OFFSET,
            // The built-in handset is the conservative one: the screen is 16 bpp and so is the deepest DIB
            // unless a profile says its firmware takes 32. The reference `DIB_NewEx` accepts any
            // non-negative pixel size, so the target's profile names 32.
            dib_depth: 16,
            // `QGraphics::m_pbmDst` in the delivered 3.1.2 source.
            graphics_canvas_offset: 20,
            alt_width: 0,
            alt_height: 0,
            scroll_bar: 8,
            encoding: 253,
            text_bytes: false,
            font_normal: 16,
            font_large: 32,
            stack_reserve: crate::layout::STACK_RESERVE,
            module_base: crate::layout::MODULE_BASE,
            overlay: OverlayCaps::w64sa(),
            gl: GlCaps::all(),
            menu_text_scroll: 200,
            menu_image_delay: 1000,
            // The header's own definition of `dwRAM` is "the size of the BREW heap a module is told
            // it has to live in", so the default tracks the heap span a run gets rather than naming
            // a figure of its own: answering 64 MiB (what a reference front end advertised) to a machine whose
            // heap is 8 MiB is an invitation to preallocate what only fails later. `--ram-mb` still
            // answers whatever a title really came with.
            ram: crate::layout::HEAP_DEFAULT,
            prompt_props: 0,
            key_close_app: g::AVK_END as u16,
            key_close_all_apps: g::AVK_END as u16,
            lang: 0,
            net_linger: 0,
            sleep_defer: 0,
            max_path: 255,
            platform_id: 0,
            device_info_size: 64,
            declared: Declared::default(),
            ram_windows: Vec::new(),
            heap_premap: 0,
            object_premap: 0,
            ktf: false,
            flags: DeviceFlags {
                vibrator: true,
                ext_speaker: true,
                // A pen, which the handset in a reference front end's the delivered body reports and which a pack may
                // branch its layout on: a pack may lay its About text out against the panel when this
                // is set and against the whole screen when it is not.
                pen: true,
                ..DeviceFlags::default()
            },
            chip_id: "MSM7201A".to_string(),
            model_name: "QuoinHLE".to_string(),
            manufacturer: "QuoinHLE".to_string(),
            friendly_name: "QuoinHLE".to_string(),
            firmware_version: "1.0".to_string(),
            carrier_id: 0,
            esn: 0xab2b_3c4f,
            imei: "123456781234567".to_string(),
            subscriber_number: "09012345678".to_string(),
            softkey_count: 2,
            manner_mode: g::AEE_MANNER_MODE_NORMAL,
            repeat_start: 200,
            repeat_rate: 75,
            key_repeat_start: 1500,
            key_repeat_rate: 100,
            storage_total: 512 * 1024 * 1024,
            storage_free: 256 * 1024 * 1024,
            fs_case_insensitive: false,
            keys: KEYS.to_vec(),
        }
    }

    /// The same handset at another screen size.
    ///
    /// A guest app is built for one screen and sizes its own buffers from what `IShell` reports:
    /// the `VX8300-176x220` pack of a real game indexes a structure at `176 * 128` bytes, which only
    /// its own width explains, and answering `240x320` for it takes a branch that leaves that
    /// pointer null. Only the screen changes here, so a run that selects a size is still being told
    /// the truth about everything else the handset claims.
    pub fn with_screen(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Override the **advertised** colour depth and `AECHAR` encoding, either of which a pack can
    /// read to choose a code path.
    ///
    /// This is a diagnostic and it deliberately lets the two answers disagree: it changes what
    /// `AEEDeviceInfo` says, not [`DeviceProfile::frame_bpp`], so the frame this host hands out
    /// keeps the profile's real format. Asking a pack what it does with a different answer is the
    /// point; using `--frame-bpp` is what changes the screen itself.
    pub fn with_advertised(mut self, color_depth: Option<u16>, encoding: Option<u16>) -> Self {
        if let Some(d) = color_depth {
            self.color_depth = d;
        }
        if let Some(e) = encoding {
            self.encoding = e;
        }
        self
    }

    /// Override the frame this host really backs and the deepest DIB it will allocate.
    ///
    /// `frame_bpp` is 16 or 32. `dib_depth` is 16 or 32 and cannot be shallower than the screen:
    /// a 32-bpp screen whose own `CreateCompatibleBitmap` refused 32 would not be one device.
    pub fn with_backing(mut self, frame_bpp: Option<u16>, dib_depth: Option<u16>) -> Self {
        if let Some(d) = frame_bpp {
            self.frame_bpp = d;
        }
        if let Some(d) = dib_depth {
            self.dib_depth = d;
        }
        self.dib_depth = self.dib_depth.max(self.frame_bpp);
        self
    }

    /// Answer `ISHELL_GetDeviceInfo(pShell, pInfo)`.
    ///
    /// The slot returns void, so a module cannot be told it got a short answer — which is exactly why
    /// the negotiation has to be read correctly. The header puts `wStructSize` at the end of the
    /// fields a handset always reports and marks only what follows it — `dwNetLinger`,
    /// `dwSleepDefer`, `wMaxPath`, `dwPlatformID` — as needing the caller's declared size. So the
    /// prefix through `dwNetLinger` is written unconditionally and only the tail is gated; a reference front end's
    /// OEM routine is the same, filling every field and never consulting the caller's word.
    ///
    /// Treating that word as a limit on the *whole* struct is a real defect, not a corner case:
    /// nothing obliges a module to initialise a struct whose fields are all outputs, so a stale word
    /// left in that stack slot silently shortens the write, and the caller never sees the fields past
    /// it.
    pub fn write_device_info(&self, mem: &mut Mem, pi: u32) -> HResult<()> {
        if pi == 0 {
            return Ok(());
        }
        let bytes = self.device_info_bytes();
        let st = info_struct();
        let declared = mem.read_u16(pi + field(st, "wStructSize") as u32)? as usize;
        let always = field(st, "dwNetLinger");
        let limit = if self.device_info_size == 0 {
            bytes.len()
        } else {
            usize::from(self.device_info_size).min(bytes.len())
        };
        // A caller that did not initialize `wStructSize` hands this a stale word. The device's own
        // struct size is the ceiling that keeps that word from writing past the caller's stack.
        let n = declared.max(always).min(limit);
        mem.write_bytes(pi, &bytes[..n])?;
        Ok(())
    }

    /// A complete `AEEDeviceInfo`, laid out from the generated field table.
    pub fn device_info_bytes(&self) -> Vec<u8> {
        let st = info_struct();
        let size = if self.device_info_size == 0 {
            st.size as usize
        } else {
            usize::from(self.device_info_size).min(st.size as usize)
        };
        let mut b = vec![0u8; st.size as usize];
        put16(st, &mut b, "cxScreen", self.width);
        put16(st, &mut b, "cyScreen", self.height);
        put16(st, &mut b, "cxAltScreen", self.alt_width);
        put16(st, &mut b, "cyAltScreen", self.alt_height);
        put16(st, &mut b, "cxScrollBar", self.scroll_bar);
        put16(st, &mut b, "wEncoding", self.encoding);
        put16(st, &mut b, "wMenuTextScroll", self.menu_text_scroll);
        put16(st, &mut b, "nColorDepth", self.color_depth);
        put32(st, &mut b, "wMenuImageDelay", self.menu_image_delay);
        put32(st, &mut b, "dwRAM", self.ram);
        for (name, set) in DeviceFlags::MEMBERS {
            if set(&self.flags) {
                set_bit(st, &mut b, name);
            }
        }
        put32(st, &mut b, "dwPromptProps", self.prompt_props);
        put16(st, &mut b, "wKeyCloseApp", self.key_close_app);
        put16(st, &mut b, "wKeyCloseAllApps", self.key_close_all_apps);
        put32(st, &mut b, "dwLang", self.lang);
        put16(st, &mut b, "wStructSize", size as u16);
        put32(st, &mut b, "dwNetLinger", self.net_linger);
        put32(st, &mut b, "dwSleepDefer", self.sleep_defer);
        put16(st, &mut b, "wMaxPath", self.max_path);
        put32(st, &mut b, "dwPlatformID", self.platform_id);
        b.truncate(size);
        b
    }

    /// The bytes an `AEE_DEVICEITEM_*` answer occupies, or `None` when this handset does not
    /// report that item — which becomes `AEE_EUNSUPPORTED`, the documented answer for an item the
    /// device does not support.
    ///
    /// Only items whose payload the published header documents are answered here; the rest of the
    /// `AEE_DEVICEITEM_*` list has no width in the header, and guessing one would be a silent
    /// ABI claim.
    ///
    /// `pbuff` is *read* as well as written, because key support and auto-repeat take their
    /// selector inside the structure they return.
    pub fn device_item(&self, mem: &Mem, item: u32, pbuff: u32) -> HResult<Option<Vec<u8>>> {
        let read_in = |off: usize, width: usize| -> HResult<u32> {
            if pbuff == 0 {
                return Ok(0);
            }
            let a = pbuff + off as u32;
            Ok(match width {
                1 => mem.read_u8(a)? as u32,
                2 => mem.read_u16(a)?,
                _ => mem.read_u32(a)?,
            })
        };
        let enc = TextEncoding::of(u32::from(self.encoding));
        let bytes = match item {
            g::AEE_DEVICEITEM_ESN => self.esn.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_CARRIER_ID => self.carrier_id.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_MANNER_MODE => self.manner_mode.to_le_bytes().to_vec(),
            g::AEE_DEVICEITEM_SOFTKEY_COUNT => vec![self.softkey_count],
            g::AEE_DEVICEITEM_CHIP_ID => utf16(enc, &self.chip_id),
            g::AEE_DEVICEITEM_MODEL_NAME => utf16(enc, &self.model_name),
            g::AEE_DEVICEITEM_MANUFACTURER_NAME => utf16(enc, &self.manufacturer),
            g::AEE_DEVICEITEM_FRIENDLY_NAME => utf16(enc, &self.friendly_name),
            g::AEE_DEVICEITEM_IMEI => ascii0(&self.imei),
            // A carrier extension, so its constant is this file's rather than a `g::` path — see
            // [`AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO`]. Answered as a terminated string for the
            // same reason the IMEI is: the caller passes a buffer and a capacity, and the null-buffer
            // form asks for the byte count that fills it.
            AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO => ascii0(&self.subscriber_number),
            // Present and empty; the reasoning is at [`AEE_DEVICEITEM_CARRIER_FIRST`].
            AEE_DEVICEITEM_CARRIER_FIRST => vec![0],
            g::AEE_DEVICEITEM_FIRMWARE_VERSION => ascii0(&self.firmware_version),
            g::AEE_DEVICEITEM_DISPINFO1 => self.bitmap_info(),
            g::AEE_DEVICEITEM_KEY_SUPPORT => {
                let st = table("KeySupportType");
                let key = read_in(field(st, "key"), 2)? as u16;
                let mut b = vec![0u8; st.size as usize];
                put16(st, &mut b, "key", key);
                b[field(st, "supported")] = u8::from(self.keys.contains(&key));
                b
            }
            g::AEE_DEVICEITEM_REPEAT_TIME => {
                let st = table("AEEAutoRepeat");
                let sel = read_in(field(st, "wItem"), 2)? as u16;
                let mut b = vec![0u8; st.size as usize];
                put16(st, &mut b, "wItem", sel);
                put32(st, &mut b, "dwStart", self.repeat_start);
                put32(st, &mut b, "dwRate", self.repeat_rate);
                b
            }
            g::AEE_DEVICEITEM_FSSPACE_ROOT | g::AEE_DEVICEITEM_FSSPACE_CARD0 => {
                let st = table("AEEFSFreeSpace");
                let mut b = vec![0u8; st.size as usize];
                b[..8].copy_from_slice(&self.storage_total.to_le_bytes());
                b[8..].copy_from_slice(&self.storage_free.to_le_bytes());
                b
            }
            // `0x9010` is in the user range (`AEE_DEVICEITEM_USER_BEGIN 0x7FFF .. USER_END 0x9FFF`),
            // so no header names it and no platform constant exists. The one pack that asks —
            // a carrier pack's resource name — takes its **first byte** and folds it into a six-way choice
            // of 19/39/59/79/99/100 (`ldrb r7,[buf]` / `cmp r7,#5` / `addls pc,pc,r7,lsl 2`, then
            // `strh r0,[r4,#0x30]`). Three different first bytes (0, 2 and 5 → tiers 19, 59 and 100)
            // produced byte-identical frames and identical draw counts, so what it gates lies past
            // start-up: the run only needs the item to *exist*, which is why answering it turned
            // silence into the main menu while the number itself changed nothing.
            //
            // Four zero bytes, then: present, and the tier left at its lowest rather than invented.
            // A handset's own tier is unrecorded — this is the shape `0x58F0` already has here, and
            // the shape a value nobody has seen must take.
            0x9010 => vec![0; 4],
            _ => return Ok(None),
        };
        Ok(Some(bytes))
    }

    fn bitmap_info(&self) -> Vec<u8> {
        let st = table("AEEBitmapInfo");
        let mut b = vec![0u8; st.size as usize];
        put32(st, &mut b, "cx", u32::from(self.width));
        put32(st, &mut b, "cy", u32::from(self.height));
        // `AEEBitmapInfo.nDepth` is the depth of the bitmap the item *is*, i.e. this host's own frame,
        // and `AEEDeviceInfo::nColorDepth` reports the same screen through the same number for the same
        // reason: a module that composites into `pBmp` has no way to reconcile two answers.
        put32(
            st,
            &mut b,
            "nDepth",
            u32::from(crate::display::NATIVE_DEPTH),
        );
        b
    }
}

/// The documented size negotiation of `ISHELL_GetDeviceInfoEx(pShell, nItem, pBuff, pnSize)`, which
/// `IAppletCtl_GetRunningInfo` spells the same way.
///
/// `pnSize` is in/out: on entry it is the buffer's capacity, on exit either the number of bytes
/// written or the number that *would* have been needed. A null `pBuff` is a legitimate query for
/// that size, which is how a module allocates for a variable-length item. The cell is a 32-bit `int`
/// here — unlike `IShell::GetProperty`'s `pwSize`, which is a `uint16`, so the two doors next to each
/// in the same header do not share a width and must not share an accessor.
///
/// ⚠ The reference door does not refuse a small buffer: `CShell_GetDeviceInfoEx` copies
/// `min(capacity, need)` and answers SUCCESS, and on the string items that copy is a `strncpy` of the
/// truncated length, so the caller can be handed a buffer with no terminator. This host answers
/// `EBUFFERTOOSMALL` and writes nothing — a refusal the caller can act on, rather than a value it
/// would read as complete.
pub fn put_sized_item(mem: &mut Mem, bytes: &[u8], pbuff: u32, pnsize: u32) -> HResult<u32> {
    if pnsize == 0 {
        return Ok(g::AEE_EBADPARM);
    }
    let need = bytes.len() as u32;
    let have = mem.read_u32(pnsize)?;
    mem.write_u32(pnsize, need)?;
    if pbuff == 0 {
        return Ok(g::AEE_SUCCESS);
    }
    if have < need {
        return Ok(g::AEE_EBUFFERTOOSMALL);
    }
    mem.write_bytes(pbuff, bytes)?;
    Ok(g::AEE_SUCCESS)
}

fn info_struct() -> &'static StructInfo {
    table("AEEDeviceInfo")
}

/// A named struct from the generated tables. Panicking means this file asked for a field that
/// does not exist — a bug in the host, never a guest condition, and every name used here is
/// covered by [`every_table_field_the_profile_writes_exists`].
fn table(name: &str) -> &'static StructInfo {
    match abi::struct_info(name) {
        Some(st) => st,
        None => panic!("{name} is not in the ABI tables"),
    }
}

fn field(st: &StructInfo, name: &str) -> usize {
    for f in st.fields {
        if f.name == name {
            return f.off as usize;
        }
    }
    panic!("{} has no field {name}", st.name)
}

fn put16(st: &StructInfo, b: &mut [u8], name: &str, v: u16) {
    let o = field(st, name);
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn put32(st: &StructInfo, b: &mut [u8], name: &str, v: u32) {
    let o = field(st, name);
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// Set one bit of a bitfield run at its table position. A run packs LSB-first into a unit whose
/// size is the *declared* type's size, little-endian — the rule a compiler probe measured
/// with the compiler class that built the samples.
fn set_bit(st: &StructInfo, b: &mut [u8], name: &str) {
    for f in st.fields {
        if f.name == name && f.bits != 0 {
            let (o, unit) = (f.off as usize, f.size as usize);
            let mut u = [0u8; 4];
            u[..unit].copy_from_slice(&b[o..o + unit]);
            let mut v = u32::from_le_bytes(u);
            v |= 1 << f.bit_off;
            b[o..o + unit].copy_from_slice(&v.to_le_bytes()[..unit]);
            return;
        }
    }
    panic!("{} has no bitfield {name}", st.name);
}

/// An item documented as an `AECHAR` string: the device's units, null terminated.
fn utf16(encoding: TextEncoding, s: &str) -> Vec<u8> {
    let mut b = Vec::with_capacity(s.len() * 2 + 2);
    for u in encoding.string_to_units(s) {
        b.extend_from_slice(&u.to_le_bytes());
    }
    b.extend_from_slice(&0u16.to_le_bytes());
    b
}

/// An item documented as a null-terminated ASCII string.
fn ascii0(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    b
}

/// The keypad: digits, call keys, the pad, and the two softkeys. The header defines the codes
/// themselves, so this is a choice of *which* keys exist, not of values.
static KEYS: [u16; 23] = [
    g::AVK_0 as u16,
    g::AVK_1 as u16,
    g::AVK_2 as u16,
    g::AVK_3 as u16,
    g::AVK_4 as u16,
    g::AVK_5 as u16,
    g::AVK_6 as u16,
    g::AVK_7 as u16,
    g::AVK_8 as u16,
    g::AVK_9 as u16,
    g::AVK_STAR as u16,
    g::AVK_POUND as u16,
    g::AVK_END as u16,
    g::AVK_SEND as u16,
    g::AVK_CLR as u16,
    g::AVK_UP as u16,
    g::AVK_DOWN as u16,
    g::AVK_LEFT as u16,
    g::AVK_RIGHT as u16,
    g::AVK_SELECT as u16,
    g::AVK_SOFT1 as u16,
    g::AVK_SOFT2 as u16,
    g::AVK_MENU as u16,
];

#[cfg(test)]
#[path = "test/device.rs"]
mod tests;
