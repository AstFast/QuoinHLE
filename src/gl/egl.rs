//! EGL display/config/context state and the two EGL faces.

use super::*;
use crate::device::{GlCaps, OverlayCaps};

pub(crate) struct Config {
    id: u32,
    red: u32,
    green: u32,
    blue: u32,
    alpha: u32,
    depth: u32,
    stencil: u32,
    /// The surface kinds this config can bind, and the client API it serves.
    surface_type: u32,
    renderable_type: u32,
    /// The largest pbuffer edge, which follows the profile's texture limit rather than a constant.
    max_pbuffer: u32,
}

impl Config {
    /// The one config this display offers, quoting the bits the device profile reports.
    ///
    /// These numbers used to be written by hand here while the GL caps carried a second set, so a
    /// pack could be told `GL_STENCIL_BITS` was 8 and `EGL_STENCIL_SIZE` was 0 in the same run.
    /// Deriving both from the profile removes the second answer.
    fn from_caps(caps: &GlCaps) -> Self {
        Config {
            id: 1,
            red: caps.red_bits,
            green: caps.green_bits,
            blue: caps.blue_bits,
            alpha: caps.alpha_bits,
            depth: caps.depth_bits,
            stencil: caps.stencil_bits,
            surface_type: EGL_WINDOW_BIT | EGL_PIXMAP_BIT | EGL_PBUFFER_BIT,
            renderable_type: EGL_OPENGL_ES_BIT,
            max_pbuffer: caps.max_texture_size,
        }
    }
}

fn egl_read_attribs(mem: &Mem, ptr: u32) -> HResult<Vec<(u32, u32)>> {
    let mut out = Vec::new();
    if ptr == 0 {
        return Ok(out);
    }
    for i in 0..32 {
        let at = ptr + i * 8;
        let key = mem.read_u32(at)?;
        if key == EGL_NONE {
            break;
        }
        out.push((key, mem.read_u32(at + 4)?));
    }
    Ok(out)
}

fn egl_config_matches(config: &Config, attribs: &[(u32, u32)]) -> bool {
    attribs.iter().all(|(key, value)| {
        if *value == EGL_DONT_CARE {
            return true;
        }
        match *key {
            EGL_RED_SIZE => config.red >= *value,
            EGL_GREEN_SIZE => config.green >= *value,
            EGL_BLUE_SIZE => config.blue >= *value,
            EGL_ALPHA_SIZE => config.alpha >= *value,
            EGL_DEPTH_SIZE => config.depth >= *value,
            EGL_STENCIL_SIZE => config.stencil >= *value,
            EGL_BUFFER_SIZE => config.red + config.green + config.blue + config.alpha >= *value,
            EGL_SURFACE_TYPE => config.surface_type & *value == *value,
            EGL_RENDERABLE_TYPE => config.renderable_type & *value == *value,
            EGL_CONFIG_CAVEAT => *value == EGL_NONE,
            EGL_SAMPLES | EGL_SAMPLE_BUFFERS | EGL_LEVEL => *value == 0,
            EGL_TRANSPARENT_TYPE => *value == EGL_NONE,
            _ => false,
        }
    })
}

fn egl_config_attrib(config: &Config, attr: u32) -> Option<u32> {
    Some(match attr {
        EGL_BUFFER_SIZE => config.red + config.green + config.blue + config.alpha,
        EGL_RED_SIZE => config.red,
        EGL_GREEN_SIZE => config.green,
        EGL_BLUE_SIZE => config.blue,
        EGL_ALPHA_SIZE => config.alpha,
        EGL_DEPTH_SIZE => config.depth,
        EGL_STENCIL_SIZE => config.stencil,
        EGL_CONFIG_CAVEAT => EGL_NONE,
        EGL_CONFIG_ID => config.id,
        EGL_LEVEL => 0,
        EGL_MAX_PBUFFER_PIXELS => config.max_pbuffer * config.max_pbuffer,
        EGL_MAX_PBUFFER_WIDTH | EGL_MAX_PBUFFER_HEIGHT => config.max_pbuffer,
        EGL_NATIVE_RENDERABLE => EGL_FALSE,
        EGL_SAMPLES | EGL_SAMPLE_BUFFERS => 0,
        EGL_SURFACE_TYPE => config.surface_type,
        EGL_TRANSPARENT_TYPE => EGL_NONE,
        EGL_TRANSPARENT_RED_VALUE | EGL_TRANSPARENT_GREEN_VALUE | EGL_TRANSPARENT_BLUE_VALUE => 0,
        EGL_RENDERABLE_TYPE => config.renderable_type,
        _ => return None,
    })
}

/// Entries reachable through `eglGetProcAddress`. The first four are EGL functions and
/// stay on the EGL side; `gl*` names are routed to the GL dispatcher by the runtime.
pub(crate) const PROC_NAMES: &[&str] = &[
    "eglBindTexImage",
    "eglReleaseTexImage",
    "eglSurfaceAttrib",
    "eglSwapInterval",
    "glPointSizePointerOES",
    "glTexGenfOES",
    "glTexGenfvOES",
    "glGenerateMipmapOES",
    "glCurrentPaletteMatrixOES",
    "glLoadPaletteFromModelViewMatrixOES",
    "glMatrixIndexPointerOES",
    "glWeightPointerOES",
    "glBlendEquationOES",
    "glBlendEquationSeparateOES",
    "glBlendFuncSeparateOES",
    "glGenFramebuffersOES",
    "glBindFramebufferOES",
    "glDeleteFramebuffersOES",
    "glIsFramebufferOES",
    "glGenRenderbuffersOES",
    "glBindRenderbufferOES",
    "glDeleteRenderbuffersOES",
    "glIsRenderbufferOES",
    "glRenderbufferStorageOES",
    "glFramebufferRenderbufferOES",
    "glCheckFramebufferStatusOES",
    "glGetFramebufferAttachmentParameterivOES",
    "glFramebufferTexture2DOES",
    "glDrawTexfOES",
    "glDrawTexfvOES",
    "glDrawTexiOES",
    "glDrawTexivOES",
    "glDrawTexxOES",
    "glDrawTexxvOES",
    "glDrawTexsOES",
    "glDrawTexsvOES",
    "glTexGeniOES",
    "glTexGenivOES",
    "glTexGenxOES",
    "glTexGenxvOES",
    "glBindBuffer",
    "glBufferData",
    "glBufferSubData",
    "glGenBuffers",
    "glDeleteBuffers",
    "glIsBuffer",
    "glGetBufferParameteriv",
    "glQueryMatrixxOES",
    "glGetRenderbufferParameterivOES",
    "glBindBufferQUALCOMM",
    "glBufferDataQUALCOMM",
    "glBufferSubDataQUALCOMM",
    "glGenBuffersQUALCOMM",
    "glDeleteBuffersQUALCOMM",
    "glIsBufferQUALCOMM",
    "glBlendEquationEXT",
    "glBlendEquationSeparateEXT",
    "glBlendFuncSeparateEXT",
];

/// The `IEGLSurfaceManip*`, `IEGLOESSwapInterval` and `IEGLGetPowerLevel` state.
///
/// These are presentation-side properties of one drawable, not GL state: they survive a context
/// switch the way the surface does. `blit` reads the rotate/scale/colour-key/transparency fields
/// when it presents the surface; the overlay bindings are kept for the query calls.
#[derive(Debug, Clone, Default)]
pub(crate) struct SurfaceManip {
    scale: bool,
    scale_src: [i32; 4],
    scale_dst: [i32; 4],
    rotate: bool,
    rot90: bool,
    hmirror: bool,
    vmirror: bool,
    transparency: bool,
    alpha: i32,
    alpha_surface: u32,
    color_key: bool,
    key: (i32, i32, i32),
    /// Whether `SurfaceOverlayEnable` turned this surface on at all.
    overlay: bool,
}

/// One layer of a composite surface. Negative indices are underlays (drawn before the composite's
/// own pixels), non-negative ones overlays (drawn after); within each side the smaller index is
/// nearer the composite's own pixels, so overlays draw in ascending order and underlays in
/// descending order.
#[derive(Debug, Clone, Default)]
pub(crate) struct Layer {
    pub(crate) enabled: bool,
    /// Draw order within the layer: later bindings cover earlier ones.
    pub(crate) surfaces: Vec<u32>,
}

/// The binding a surface has in a composite. A surface has at most one (composite, layer) pair;
/// `GetSurfaceOverlayBinding` reads it back for the guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerBinding {
    pub(crate) layer: i32,
}

/// A composite surface created by `CreateCompositeSurface`: the window it presents into and the
/// overlay/underlay layers bound to it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Composite {
    pub(crate) window: u32,
    pub(crate) layers: HashMap<i32, Layer>,
    pub(crate) bindings: HashMap<u32, LayerBinding>,
}

// Layer 0 is the composite's base layer; overlay layers are 1..=max_overlay and underlay
// layers -1..=-max_underlay. The limits come from [`crate::device::OverlayCaps`] because they
// are display-controller capabilities, not GLES constants: One handset reports 1/1/16/33, while the
// W64S firmware does not expose the extension at all.

fn read_rect(mem: &Mem, at: u32) -> HResult<[i32; 4]> {
    if at == 0 {
        return Ok([0; 4]);
    }
    let mut rect = [0i32; 4];
    for (i, value) in rect.iter_mut().enumerate() {
        *value = mem.read_u32(at + 4 * i as u32)? as i32;
    }
    Ok(rect)
}

fn put_rect(mem: &mut Mem, at: u32, rect: [i32; 4]) -> HResult<()> {
    if at != 0 {
        for (i, value) in rect.iter().enumerate() {
            mem.write_u32(at + 4 * i as u32, *value as u32)?;
        }
    }
    Ok(())
}

/// EGL display/config/context state.
///
/// `method` and `method10` are ABI adapters; the operations they share live in the helpers
/// below. `make_current` is the only context transition: the runtime applies the corresponding
/// `Gles` state switch after an EGL slot returns (see `Runtime::sync_gl_context`).
pub struct Egl {
    pub(crate) configs: Vec<Config>,
    pub(crate) surfaces: HashMap<u32, Surface>,
    pub(crate) contexts: HashMap<u32, u32>,
    pub(crate) next: u32,
    pub(crate) current: Option<u32>,
    pub(crate) read: Option<u32>,
    pub(crate) current_context: Option<u32>,
    pub(crate) swap_interval: i32,
    pub(crate) error: u32,
    pub(crate) initialized: bool,
    /// A `BindTexImage` request waiting for the runtime to copy the surface into the
    /// current GL texture. EGL cannot touch `Gles` itself.
    pending_texture_bind: Option<u32>,
    /// Share group per context. Contexts created with the same share context share their
    /// texture and buffer object namespaces.
    share_groups: HashMap<u32, u32>,
    next_share_group: u32,
    /// EGL current state is per guest thread, not per process: a cooperative `IThread` may
    /// make a context current, suspend, and another thread may run with its own current
    /// context before the first resumes.
    thread_current: HashMap<u32, (Option<u32>, Option<u32>, Option<u32>)>,
    current_thread: u32,
    /// Presentation-side state for the EGL extension interfaces, keyed by surface.
    manip: HashMap<u32, SurfaceManip>,
    /// Composite surfaces, keyed by the surface `CreateCompositeSurface` returned.
    composites: HashMap<u32, Composite>,
    /// Overlay/underlay capabilities from the device profile.
    pub(crate) overlay: OverlayCaps,
    /// `--gl-log`'s rows. It lives here rather than in [`Gles`] because a GL context switch
    /// replaces that whole struct ([`crate::runtime::Runtime::sync_gl_context`]) while the EGL
    /// state, like the run, is one.
    pub(crate) gl_log: Option<diag::GlDrawLog>,
}

impl Default for Egl {
    fn default() -> Self {
        Self::new()
    }
}

impl Egl {
    /// A display for the software backend's own profile.
    pub fn new() -> Self {
        Self::with_caps(&GlCaps::all())
    }

    /// A display whose config quotes `caps`.
    pub fn with_caps(caps: &GlCaps) -> Self {
        Self::with_caps_and_overlay(caps, OverlayCaps::default())
    }

    pub fn with_caps_and_overlay(caps: &GlCaps, overlay: OverlayCaps) -> Self {
        Egl {
            configs: vec![Config::from_caps(caps)],
            surfaces: HashMap::new(),
            contexts: HashMap::new(),
            next: 0x5000,
            current: None,
            read: None,
            current_context: None,
            swap_interval: 1,
            error: EGL_SUCCESS,
            initialized: false,
            pending_texture_bind: None,
            share_groups: HashMap::new(),
            next_share_group: 1,
            thread_current: HashMap::new(),
            current_thread: 0,
            manip: HashMap::new(),
            composites: HashMap::new(),
            overlay,
            gl_log: None,
        }
    }

    /// Build the EGL state with the overlay capabilities a device profile names.
    /// Switch the EGL current state to a guest thread, saving the state the previous thread
    /// had installed. The runtime calls this around every `run_thread` slice.
    pub(crate) fn select_thread(&mut self, thread: u32) {
        if self.current_thread == thread {
            return;
        }
        self.thread_current.insert(
            self.current_thread,
            (self.current, self.read, self.current_context),
        );
        let (current, read, context) = self
            .thread_current
            .remove(&thread)
            .unwrap_or((None, None, None));
        self.current = current;
        self.read = read;
        self.current_context = context;
        self.current_thread = thread;
    }

    /// Drop a finished thread's saved EGL current state.
    pub(crate) fn forget_thread(&mut self, thread: u32) {
        self.thread_current.remove(&thread);
    }

    pub(crate) fn alloc(&mut self) -> u32 {
        let id = self.next;
        self.next = self.next.wrapping_add(0x10);
        id
    }

    pub fn current_surface_mut(&mut self) -> Option<&mut Surface> {
        let id = self.current?;
        self.surfaces.get_mut(&id)
    }

    pub fn read_surface(&self) -> Option<&Surface> {
        let id = self.current?;
        self.surfaces.get(&id)
    }

    fn layer_in_range(&self, layer: i32) -> bool {
        self.overlay.present
            && layer >= -(self.overlay.max_underlay as i32)
            && layer <= self.overlay.max_overlay as i32
    }

    fn fail(&mut self, code: u32) -> HResult<u32> {
        self.error = code;
        Ok(EGL_FALSE)
    }

    /// A context remembers its client version; share context only has to exist, because every
    /// context in this host renders through the same software pipeline.
    pub(crate) fn create_context(&mut self, mem: &Mem, share: u32, attribs: u32) -> HResult<u32> {
        let mut version = 1;
        for (key, value) in egl_read_attribs(mem, attribs)? {
            match key {
                EGL_CONTEXT_CLIENT_VERSION if value <= 1 => version = value,
                _ => {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return Ok(EGL_NO_CONTEXT);
                }
            }
        }
        if share != EGL_NO_CONTEXT && !self.contexts.contains_key(&share) {
            self.error = EGL_BAD_CONTEXT;
            return Ok(EGL_NO_CONTEXT);
        }
        let id = self.alloc();
        let group = if share != EGL_NO_CONTEXT {
            self.share_groups.get(&share).copied().unwrap_or(0)
        } else {
            0
        };
        let group = if group == 0 {
            let group = self.next_share_group;
            self.next_share_group = self.next_share_group.wrapping_add(1);
            group
        } else {
            group
        };
        self.contexts.insert(id, version);
        self.share_groups.insert(id, group);
        Ok(id)
    }

    pub(crate) fn share_group(&self, id: u32) -> Option<u32> {
        self.share_groups.get(&id).copied()
    }

    pub(crate) fn make_current(&mut self, draw: u32, read: u32, ctx: u32) -> bool {
        if ctx == EGL_NO_CONTEXT {
            self.current = None;
            self.read = None;
            self.current_context = None;
            return true;
        }
        if !self.contexts.contains_key(&ctx) {
            self.error = EGL_BAD_CONTEXT;
            return false;
        }
        if draw != EGL_NO_SURFACE && !self.surfaces.contains_key(&draw) {
            self.error = EGL_BAD_SURFACE;
            return false;
        }
        if read != EGL_NO_SURFACE && !self.surfaces.contains_key(&read) {
            self.error = EGL_BAD_SURFACE;
            return false;
        }
        self.current = (draw != EGL_NO_SURFACE).then_some(draw);
        self.read = (read != EGL_NO_SURFACE).then_some(read);
        self.current_context = Some(ctx);
        true
    }

    /// The `IEGL11` face: the `IEGL10` slots plus `SurfaceAttrib`/`BindTexImage`/
    /// `ReleaseTexImage`/`SwapInterval`.
    pub fn method11(
        &mut self,
        name: &str,
        this: u32,
        args: &[u32],
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        let put = |mem: &mut Mem, at: u32, v: u32| -> HResult<()> {
            if at != 0 {
                mem.write_u32(at, v)?;
            }
            Ok(())
        };
        match name {
            "SurfaceAttrib" => {
                let (surface, attr, value, ret) = (a(1), a(2), a(3), a(4));
                let ok = self.surface_attrib(surface, attr, value);
                put(mem, ret, if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "BindTexImage" => {
                let (surface, ret) = (a(1), a(3));
                let ok = self.bind_tex_image(surface, 0);
                put(mem, ret, if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "ReleaseTexImage" => {
                let (surface, ret) = (a(1), a(3));
                let ok = self.release_tex_image(surface);
                put(mem, ret, if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "SwapInterval" => {
                self.swap_interval = a(1) as i32;
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            _ => self.method10(name, this, args, mem, display),
        }
    }

    /// The two vendor color-buffer faces. Window/pixmap surfaces return the guest DIB; a
    /// pbuffer lazily gets a guest allocation populated from its RGBA plane.
    pub fn color_buffer(&mut self, name: &str, args: &[u32], mem: &mut Mem) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        let put = |mem: &mut Mem, at: u32, v: u32| -> HResult<()> {
            if at != 0 {
                mem.write_u32(at, v)?;
            }
            Ok(())
        };
        match name {
            "GetColorBuffer" => {
                let Some(id) = self.current else {
                    return Ok(crate::abi::generated::AEE_EUNSUPPORTED);
                };
                let Some(surface) = self.surfaces.get_mut(&id) else {
                    return Ok(crate::abi::generated::AEE_EUNSUPPORTED);
                };
                if let Some(target) = surface.target {
                    put(mem, a(0), target)?;
                    return Ok(0);
                }
                let buffer = match surface.color_buffer {
                    Some(buffer) => buffer,
                    None => {
                        let size = surface.width * surface.height * 4;
                        let buffer = mem.alloc(size, 4)?;
                        surface.color_buffer = Some(buffer);
                        buffer
                    }
                };
                mem.write_bytes(buffer, &surface.rgba)?;
                put(mem, a(0), buffer)?;
                Ok(0)
            }
            "GetColorBufferInfo" => {
                let dims = self
                    .current
                    .and_then(|id| self.surfaces.get(&id))
                    .map(|s| (s.width, s.height));
                let Some((width, height)) = dims else {
                    self.error = EGL_BAD_SURFACE;
                    put(mem, a(2), EGL_FALSE)?;
                    return Ok(0);
                };
                let value = match a(0) {
                    EGL_WIDTH => width,
                    EGL_HEIGHT => height,
                    _ => {
                        self.error = EGL_BAD_ATTRIBUTE;
                        put(mem, a(2), EGL_FALSE)?;
                        return Ok(0);
                    }
                };
                put(mem, a(1), value)?;
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            _ => Ok(crate::abi::generated::AEE_EUNSUPPORTED),
        }
    }

    /// The context `eglMakeCurrent` installed, if any.
    pub fn current_context(&self) -> Option<u32> {
        self.current_context
    }

    /// Whether a context id is still alive.
    pub fn has_context(&self, id: u32) -> bool {
        self.contexts.contains_key(&id)
    }

    /// One `IEGL` slot. `this` is the display handle the guest was handed by `eglGetDisplay`.
    fn config_ids(&self) -> Vec<u32> {
        self.configs.iter().map(|c| c.id).collect()
    }

    fn choose_config(&self, mem: &Mem, attribs: u32) -> HResult<Option<u32>> {
        let wanted = egl_read_attribs(mem, attribs)?;
        Ok(self
            .configs
            .iter()
            .find(|c| egl_config_matches(c, &wanted))
            .map(|c| c.id))
    }

    fn config_attrib(&self, config: u32, attr: u32) -> Result<u32, u32> {
        let Some(cfg) = self.configs.iter().find(|c| c.id == config) else {
            return Err(EGL_BAD_CONFIG);
        };
        egl_config_attrib(cfg, attr).ok_or(EGL_BAD_ATTRIBUTE)
    }

    fn create_surface(
        &mut self,
        kind: SurfaceKind,
        config: u32,
        native: u32,
        attribs: u32,
        mem: &Mem,
        display: &Display,
    ) -> HResult<Option<u32>> {
        let Some(max_pbuffer) = self
            .configs
            .iter()
            .find(|c| c.id == config)
            .map(|c| c.max_pbuffer)
        else {
            self.error = EGL_BAD_CONFIG;
            return Ok(None);
        };
        let (width, height, target) = match kind {
            SurfaceKind::Pbuffer => {
                let mut width = 64;
                let mut height = 64;
                let mut largest = false;
                for (key, value) in egl_read_attribs(mem, attribs)? {
                    match key {
                        EGL_WIDTH => width = value.max(1),
                        EGL_HEIGHT => height = value.max(1),
                        EGL_LARGEST_PBUFFER => largest = value != EGL_FALSE,
                        _ => {}
                    }
                }
                if largest {
                    width = width.min(max_pbuffer);
                    height = height.min(max_pbuffer);
                } else if width > max_pbuffer || height > max_pbuffer {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return Ok(None);
                }
                (width, height, None)
            }
            SurfaceKind::Window => {
                let Some(bitmap) = display.bitmap(native) else {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return Ok(None);
                };
                (bitmap.width as u32, bitmap.height as u32, Some(native))
            }
        };
        let id = self.alloc();
        let mut surface = Surface::new(width, height, kind);
        surface.target = target;
        self.surfaces.insert(id, surface);
        Ok(Some(id))
    }

    fn destroy_surface(&mut self, surface: u32) {
        self.surfaces.remove(&surface);
        for (draw, read, _) in self.thread_current.values_mut() {
            if *draw == Some(surface) {
                *draw = None;
            }
            if *read == Some(surface) {
                *read = None;
            }
        }
        if self.current == Some(surface) {
            self.current = None;
        }
        if self.read == Some(surface) {
            self.read = None;
        }
        self.manip.remove(&surface);
        // A destroyed surface loses its binding in every composite.
        for c in self.composites.values_mut() {
            if let Some(binding) = c.bindings.remove(&surface) {
                if let Some(layer) = c.layers.get_mut(&binding.layer) {
                    layer.surfaces.retain(|s| *s != surface);
                }
            }
        }
        // A composite wrapping this window cannot present into it any more; it goes with the
        // window, and so does its own drawable.
        let wrapped: Vec<u32> = self
            .composites
            .iter()
            .filter(|(_, c)| c.window == surface)
            .map(|(id, _)| *id)
            .collect();
        for id in wrapped {
            self.composites.remove(&id);
            self.surfaces.remove(&id);
            self.manip.remove(&id);
        }
        // Destroying a composite drops its layer table; the bound surfaces stay alive.
        self.composites.remove(&surface);
    }

    fn query_surface(&self, surface: u32, attr: u32) -> Result<u32, u32> {
        let Some(s) = self.surfaces.get(&surface) else {
            return Err(EGL_BAD_SURFACE);
        };
        match attr {
            EGL_WIDTH | EGL_LARGEST_PBUFFER => Ok(s.width),
            EGL_HEIGHT => Ok(s.height),
            EGL_SWAP_BEHAVIOR => Ok(s.swap_behavior),
            EGL_TEXTURE_FORMAT => Ok(s.texture_format),
            EGL_TEXTURE_TARGET => Ok(s.texture_target),
            _ => Err(EGL_BAD_ATTRIBUTE),
        }
    }

    fn destroy_context(&mut self, id: u32) {
        self.contexts.remove(&id);
        self.share_groups.remove(&id);
        for state in self.thread_current.values_mut() {
            if state.2 == Some(id) {
                *state = (None, None, None);
            }
        }
        if self.current_context == Some(id) {
            self.current_context = None;
            self.current = None;
            self.read = None;
        }
    }

    fn query_context(&self, ctx: u32, attr: u32) -> Result<u32, u32> {
        let Some(&version) = self.contexts.get(&ctx) else {
            return Err(EGL_BAD_CONTEXT);
        };
        match attr {
            EGL_CONTEXT_CLIENT_VERSION => Ok(version),
            _ => Ok(0),
        }
    }

    fn get_current_surface(&self, which: u32) -> u32 {
        let surface = if which == EGL_READ {
            self.read
        } else if which == EGL_DRAW {
            self.current
        } else {
            None
        };
        surface.unwrap_or(EGL_NO_SURFACE)
    }

    fn surface_attrib(&mut self, surface: u32, attr: u32, value: u32) -> bool {
        let Some(s) = self.surfaces.get_mut(&surface) else {
            self.error = EGL_BAD_SURFACE;
            return false;
        };
        match attr {
            EGL_SWAP_BEHAVIOR => {
                if !matches!(value, EGL_BUFFER_DESTROYED | EGL_BUFFER_PRESERVED) {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return false;
                }
                s.swap_behavior = value;
            }
            EGL_TEXTURE_FORMAT => {
                if !matches!(value, EGL_NO_TEXTURE | EGL_TEXTURE_RGB | EGL_TEXTURE_RGBA) {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return false;
                }
                s.texture_format = value;
            }
            EGL_TEXTURE_TARGET => {
                if !matches!(value, EGL_NO_TEXTURE | EGL_TEXTURE_2D) {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return false;
                }
                s.texture_target = value;
            }
            _ => {
                self.error = EGL_BAD_ATTRIBUTE;
                return false;
            }
        }
        true
    }

    fn bind_tex_image(&mut self, surface: u32, _buffer: u32) -> bool {
        let Some(s) = self.surfaces.get(&surface) else {
            self.error = EGL_BAD_SURFACE;
            return false;
        };
        if s.texture_format == EGL_NO_TEXTURE || s.texture_target != EGL_TEXTURE_2D {
            self.error = EGL_BAD_MATCH;
            return false;
        }
        self.pending_texture_bind = Some(surface);
        true
    }

    fn release_tex_image(&mut self, surface: u32) -> bool {
        if !self.surfaces.contains_key(&surface) {
            self.error = EGL_BAD_SURFACE;
            return false;
        }
        if self.pending_texture_bind == Some(surface) {
            self.pending_texture_bind = None;
        }
        true
    }

    pub(crate) fn take_texture_bind(&mut self) -> Option<u32> {
        self.pending_texture_bind.take()
    }

    pub(crate) fn surface_data(&self, surface: u32) -> Option<(u32, u32, Vec<u8>)> {
        self.surfaces
            .get(&surface)
            .map(|s| (s.width, s.height, s.rgba.clone()))
    }

    pub(crate) fn surface_texture_format(&self, surface: u32) -> u32 {
        self.surfaces
            .get(&surface)
            .map(|s| s.texture_format)
            .unwrap_or(EGL_TEXTURE_RGBA)
    }

    fn query_string(&self, which: u32) -> Option<&'static [u8]> {
        match which {
            0x3053 => Some(b"QuoinHLE\0"),
            0x3054 => Some(b"1.0\0"),
            0x3055 => Some(b"EGL_QUALCOMM_COLOR_BUFFER EGL_QUALCOMM_get_color_buffer\0"),
            0x308D => Some(b"OpenGL_ES\0"),
            _ => None,
        }
    }

    fn take_error(&mut self) -> u32 {
        let error = self.error;
        self.error = EGL_SUCCESS;
        error
    }

    pub(crate) fn proc_call(&mut self, name: &str, args: &[u32]) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        Ok(match name {
            "eglBindTexImage" => {
                if self.bind_tex_image(a(1), a(2)) {
                    EGL_TRUE
                } else {
                    EGL_FALSE
                }
            }
            "eglReleaseTexImage" => {
                if self.release_tex_image(a(1)) {
                    EGL_TRUE
                } else {
                    EGL_FALSE
                }
            }
            "eglSurfaceAttrib" => {
                if self.surface_attrib(a(1), a(2), a(3)) {
                    EGL_TRUE
                } else {
                    EGL_FALSE
                }
            }
            "eglSwapInterval" => {
                self.swap_interval = a(1) as i32;
                EGL_TRUE
            }
            _ => EGL_FALSE,
        })
    }

    pub fn method(
        &mut self,
        name: &str,
        this: u32,
        args: &[u32],
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        match name {
            "eglGetDisplay" => Ok(this),
            "eglInitialize" => {
                if a(0) != this {
                    return self.fail(EGL_BAD_DISPLAY);
                }
                self.initialized = true;
                Ok(EGL_TRUE)
            }
            "eglTerminate" => {
                self.initialized = false;
                self.current = None;
                self.read = None;
                self.current_context = None;
                self.thread_current.clear();
                self.current_thread = 0;
                self.pending_texture_bind = None;
                self.surfaces.clear();
                self.contexts.clear();
                self.share_groups.clear();
                self.manip.clear();
                self.composites.clear();
                Ok(EGL_TRUE)
            }
            "eglGetConfigs" => {
                let (configs, max, count) = (a(1), a(2), a(3));
                let ids = self.config_ids();
                if configs != 0 {
                    for (i, id) in ids.iter().take(max as usize).enumerate() {
                        mem.write_u32(configs + i as u32 * 4, *id)?;
                    }
                }
                if count != 0 {
                    mem.write_u32(count, ids.len().min(max as usize) as u32)?;
                }
                Ok(EGL_TRUE)
            }
            "eglChooseConfig" => {
                let (attribs, configs, max, count) = (a(1), a(2), a(3), a(4));
                let chosen = self.choose_config(mem, attribs)?;
                if configs != 0 && max > 0 {
                    if let Some(id) = chosen {
                        mem.write_u32(configs, id)?;
                    }
                }
                if count != 0 {
                    mem.write_u32(count, chosen.is_some().then_some(1).unwrap_or(0).min(max))?;
                }
                Ok(EGL_TRUE)
            }
            "eglGetConfigAttrib" => {
                let (config, attr, value) = (a(1), a(2), a(3));
                match self.config_attrib(config, attr) {
                    Ok(v) => {
                        if value != 0 {
                            mem.write_u32(value, v)?;
                        }
                        Ok(EGL_TRUE)
                    }
                    Err(code) => self.fail(code),
                }
            }
            "eglCreateWindowSurface" | "eglCreatePixmapSurface" => {
                let Some(id) =
                    self.create_surface(SurfaceKind::Window, a(1), a(2), a(3), mem, display)?
                else {
                    return Ok(EGL_NO_SURFACE);
                };
                Ok(id)
            }
            "eglCreatePbufferSurface" => {
                let Some(id) =
                    self.create_surface(SurfaceKind::Pbuffer, a(1), 0, a(2), mem, display)?
                else {
                    return Ok(EGL_NO_SURFACE);
                };
                Ok(id)
            }
            "eglDestroySurface" => {
                self.destroy_surface(a(1));
                Ok(EGL_TRUE)
            }
            "eglQuerySurface" => {
                let (surface, attr, value) = (a(1), a(2), a(3));
                match self.query_surface(surface, attr) {
                    Ok(v) => {
                        if value != 0 {
                            mem.write_u32(value, v)?;
                        }
                        Ok(EGL_TRUE)
                    }
                    Err(code) => self.fail(code),
                }
            }
            "eglCreateContext" => {
                let (share, attribs) = (a(2), a(3));
                Ok(self.create_context(mem, share, attribs)?)
            }
            "eglDestroyContext" => {
                self.destroy_context(a(1));
                Ok(EGL_TRUE)
            }
            "eglMakeCurrent" => {
                let (draw, read, ctx) = (a(1), a(2), a(3));
                Ok(if self.make_current(draw, read, ctx) {
                    EGL_TRUE
                } else {
                    EGL_FALSE
                })
            }
            "eglGetCurrentContext" => Ok(self.current_context.unwrap_or(EGL_NO_CONTEXT)),
            "eglGetCurrentSurface" => Ok(self.get_current_surface(a(0))),
            "eglGetCurrentDisplay" => Ok(this),
            "eglQueryContext" => {
                let (ctx, attr, value) = (a(1), a(2), a(3));
                match self.query_context(ctx, attr) {
                    Ok(v) => {
                        if value != 0 {
                            mem.write_u32(value, v)?;
                        }
                        Ok(EGL_TRUE)
                    }
                    Err(code) => self.fail(code),
                }
            }
            "eglSwapBuffers" => {
                self.present(a(1), mem, display)?;
                Ok(EGL_TRUE)
            }
            "eglCopyBuffers" => {
                let target = a(2);
                if target != 0 {
                    self.blit(a(1), target, mem, display)?;
                }
                Ok(EGL_TRUE)
            }
            "eglWaitGL" | "eglWaitNative" => Ok(EGL_TRUE),
            "eglBindTexImage" => Ok(if self.bind_tex_image(a(1), a(2)) {
                EGL_TRUE
            } else {
                EGL_FALSE
            }),
            "eglReleaseTexImage" => Ok(if self.release_tex_image(a(1)) {
                EGL_TRUE
            } else {
                EGL_FALSE
            }),
            "eglGetError" => Ok(self.take_error()),
            "eglQueryString" => {
                let Some(text) = self.query_string(a(1)) else {
                    return self.fail(EGL_BAD_ATTRIBUTE);
                };
                let at = mem.alloc(text.len() as u32, 1)?;
                mem.write_bytes(at, text)?;
                Ok(at)
            }
            "eglGetProcAddress" => {
                let Ok(bytes) = mem.read_cstr(a(0), 64) else {
                    return Ok(0);
                };
                let Ok(name) = std::str::from_utf8(&bytes) else {
                    return Ok(0);
                };
                let Some(index) = PROC_NAMES.iter().position(|n| *n == name) else {
                    return Ok(0);
                };
                Ok(crate::hle::Hle::stub(
                    mem,
                    crate::abi::proc_id(index as u32),
                )?)
            }
            _ => Ok(crate::abi::generated::AEE_EUNSUPPORTED),
        }
    }

    /// One `IEGL10` slot. Unlike `IEGL`, this face returns `SUCCESS` and writes every answer
    /// through a trailing out-pointer, which is what `CEGL10` in the reference layer does.
    pub fn method10(
        &mut self,
        name: &str,
        this: u32,
        args: &[u32],
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<u32> {
        let a = |n: usize| args.get(n).copied().unwrap_or(0);
        let put = |mem: &mut Mem, at: u32, v: u32| -> HResult<()> {
            if at != 0 {
                mem.write_u32(at, v)?;
            }
            Ok(())
        };
        match name {
            "GetError" => {
                put(mem, a(0), self.take_error())?;
                Ok(0)
            }
            "GetDisplay" => {
                let id = a(0);
                put(mem, a(1), if id != 0 { id } else { this })?;
                Ok(0)
            }
            "Initialize" => {
                self.initialized = true;
                put(mem, a(1), 1)?;
                put(mem, a(2), 0)?;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "Terminate" => {
                self.initialized = false;
                self.current = None;
                self.read = None;
                self.current_context = None;
                self.surfaces.clear();
                self.contexts.clear();
                self.share_groups.clear();
                put(mem, a(1), EGL_TRUE)?;
                Ok(0)
            }
            "QueryString" => {
                let Some(text) = self.query_string(a(1)) else {
                    self.error = EGL_BAD_ATTRIBUTE;
                    return Ok(0);
                };
                let at = mem.alloc(text.len() as u32, 1)?;
                mem.write_bytes(at, text)?;
                put(mem, a(2), at)?;
                Ok(0)
            }
            "GetConfigs" => {
                let (configs, max, count, ret) = (a(1), a(2), a(3), a(4));
                let ids = self.config_ids();
                if configs != 0 {
                    for (i, id) in ids.iter().take(max as usize).enumerate() {
                        mem.write_u32(configs + i as u32 * 4, *id)?;
                    }
                }
                put(mem, count, ids.len().min(max as usize) as u32)?;
                put(mem, ret, EGL_TRUE)?;
                Ok(0)
            }
            "ChooseConfig" => {
                let (attribs, configs, max, count, ret) = (a(1), a(2), a(3), a(4), a(5));
                let chosen = self.choose_config(mem, attribs)?;
                if configs != 0 && max > 0 {
                    if let Some(id) = chosen {
                        mem.write_u32(configs, id)?;
                    }
                }
                put(mem, count, if chosen.is_some() { 1 } else { 0 })?;
                put(mem, ret, EGL_TRUE)?;
                Ok(0)
            }
            "GetConfigAttrib" => {
                let (config, attr, value, ret) = (a(1), a(2), a(3), a(4));
                match self.config_attrib(config, attr) {
                    Ok(v) => {
                        put(mem, value, v)?;
                        put(mem, ret, EGL_TRUE)?;
                    }
                    Err(code) => {
                        self.error = code;
                        put(mem, ret, EGL_FALSE)?;
                    }
                }
                Ok(0)
            }
            "CreateWindowSurface" | "CreatePixmapSurface" => {
                let ret = a(4);
                let created =
                    self.create_surface(SurfaceKind::Window, a(1), a(2), a(3), mem, display)?;
                put(mem, ret, created.unwrap_or(EGL_NO_SURFACE))?;
                Ok(0)
            }
            "CreatePbufferSurface" => {
                let (attribs, ret) = (a(2), a(3));
                let created =
                    self.create_surface(SurfaceKind::Pbuffer, a(1), 0, attribs, mem, display)?;
                put(mem, ret, created.unwrap_or(EGL_NO_SURFACE))?;
                Ok(0)
            }
            "DestroySurface" => {
                self.destroy_surface(a(1));
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            "QuerySurface" => {
                let (surface, attr, value, ret) = (a(1), a(2), a(3), a(4));
                match self.query_surface(surface, attr) {
                    Ok(v) => {
                        put(mem, value, v)?;
                        put(mem, ret, EGL_TRUE)?;
                    }
                    Err(code) => {
                        self.error = code;
                        put(mem, ret, EGL_FALSE)?;
                    }
                }
                Ok(0)
            }
            "CreateContext" => {
                let (share, attribs, out, ret) = (a(2), a(3), a(4), a(5));
                let id = self.create_context(mem, share, attribs)?;
                put(mem, out, id)?;
                put(
                    mem,
                    ret,
                    if id != EGL_NO_CONTEXT {
                        EGL_TRUE
                    } else {
                        EGL_FALSE
                    },
                )?;
                Ok(0)
            }
            "DestroyContext" => {
                self.destroy_context(a(1));
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            "MakeCurrent" => {
                let (draw, read, ctx, ret) = (a(1), a(2), a(3), a(4));
                let ok = self.make_current(draw, read, ctx);
                put(mem, ret, if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "GetCurrentContext" => {
                put(mem, a(0), self.current_context.unwrap_or(EGL_NO_CONTEXT))?;
                Ok(0)
            }
            "GetCurrentSurface" => {
                put(mem, a(1), self.get_current_surface(a(0)))?;
                Ok(0)
            }
            "GetCurrentDisplay" => {
                put(mem, a(0), this)?;
                Ok(0)
            }
            "QueryContext" => {
                let (ctx, attr, value, ret) = (a(1), a(2), a(3), a(4));
                match self.query_context(ctx, attr) {
                    Ok(v) => {
                        put(mem, value, v)?;
                        put(mem, ret, EGL_TRUE)?;
                    }
                    Err(code) => {
                        self.error = code;
                        put(mem, ret, EGL_FALSE)?;
                    }
                }
                Ok(0)
            }
            "WaitGL" => {
                put(mem, a(0), EGL_TRUE)?;
                Ok(0)
            }
            "WaitNative" => {
                put(mem, a(1), EGL_TRUE)?;
                Ok(0)
            }
            "SwapBuffers" => {
                self.present(a(1), mem, display)?;
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            "CopyBuffers" => {
                let (surface, target, ret) = (a(1), a(2), a(3));
                if target != 0 {
                    self.blit(surface, target, mem, display)?;
                }
                put(mem, ret, EGL_TRUE)?;
                Ok(0)
            }

            // ---- EGL extension interfaces ------------------------------------------------
            // `IEGLGetPowerLevel`: the handset has no battery reading here; a fixed full level is
            // the answer the Q3Dimension layer gives when the query is not backed by hardware.
            "GetPowerLevel" => {
                put(mem, a(0), 0)?;
                Ok(0)
            }
            // `IEGLOESSwapInterval`: the interval is display state, not per-context state.
            "SwapInterval" => {
                self.swap_interval = a(1) as i32;
                put(mem, a(2), EGL_TRUE)?;
                Ok(0)
            }
            "GetSwapInterval" => {
                put(mem, a(1), self.swap_interval as u32)?;
                Ok(0)
            }
            // `IEGLSurfaceManip_V1`: scale.
            "SurfaceScaleEnable" => {
                self.manip.entry(a(1)).or_default().scale = a(2) != 0;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "SetSurfaceScale" => {
                let src = read_rect(mem, a(2))?;
                let dst = read_rect(mem, a(3))?;
                let m = self.manip.entry(a(1)).or_default();
                m.scale_src = src;
                m.scale_dst = dst;
                put(mem, a(4), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceScale" => {
                let (enabled, src, dst) = self
                    .manip
                    .get(&a(1))
                    .map(|m| (m.scale, m.scale_src, m.scale_dst))
                    .unwrap_or_default();
                put(mem, a(2), if enabled { EGL_TRUE } else { EGL_FALSE })?;
                put_rect(mem, a(3), src)?;
                put_rect(mem, a(4), dst)?;
                put(mem, a(5), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceScaleCaps" => {
                let param = a(2);
                if param != 0 {
                    // 1.0..4.0 in 16.16, 1..4096 pixels. The software blit has no smaller
                    // hardware limit to report.
                    let caps = [
                        1 << 16,
                        4 << 16,
                        1 << 16,
                        4 << 16,
                        1,
                        4096,
                        1,
                        4096,
                        1,
                        4096,
                        1,
                        4096u32,
                    ];
                    for (i, value) in caps.iter().enumerate() {
                        mem.write_u32(param + 4 * i as u32, *value)?;
                    }
                }
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            // `IEGLSurfaceManip_V1`: rotate and mirror.
            "SurfaceRotateEnable" => {
                self.manip.entry(a(1)).or_default().rotate = a(2) != 0;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "SetSurfaceRotate" => {
                let m = self.manip.entry(a(1)).or_default();
                m.rot90 = a(2) != 0;
                m.hmirror = a(3) != 0;
                m.vmirror = a(4) != 0;
                put(mem, a(5), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceRotate" => {
                let m = self.manip.get(&a(1)).cloned().unwrap_or_default();
                put(mem, a(2), if m.rotate { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(3), if m.rot90 { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(4), if m.hmirror { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(5), if m.vmirror { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(6), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceRotateCaps" => {
                put(mem, a(2), EGL_TRUE)?;
                put(mem, a(3), EGL_TRUE)?;
                put(mem, a(4), EGL_TRUE)?;
                put(mem, a(5), EGL_TRUE)?;
                Ok(0)
            }
            // `IEGLSurfaceManip_V1`: constant alpha and an alpha map.
            "SurfaceTransparencyEnable" => {
                self.manip.entry(a(1)).or_default().transparency = a(2) != 0;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "SetSurfaceTransparency" => {
                self.manip.entry(a(1)).or_default().alpha = a(2) as i32;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceTransparency" => {
                let m = self.manip.get(&a(1)).cloned().unwrap_or_default();
                put(mem, a(2), if m.transparency { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(3), m.alpha as u32)?;
                put(mem, a(4), EGL_TRUE)?;
                Ok(0)
            }
            "SetSurfaceTransparencyMap" => {
                self.manip.entry(a(1)).or_default().alpha_surface = a(2);
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceTransparencyMap" => {
                let m = self.manip.get(&a(1)).cloned().unwrap_or_default();
                put(
                    mem,
                    a(2),
                    if m.alpha_surface != 0 {
                        EGL_TRUE
                    } else {
                        EGL_FALSE
                    },
                )?;
                put(mem, a(3), m.alpha_surface)?;
                put(mem, a(4), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceTransparencyCaps" => {
                put(mem, a(2), EGL_TRUE)?;
                put(mem, a(3), EGL_TRUE)?;
                put(mem, a(4), EGL_TRUE)?;
                put(mem, a(5), EGL_TRUE)?;
                Ok(0)
            }
            // `IEGLSurfaceManip`: colour key.
            "SurfaceColorKeyEnable" => {
                self.manip.entry(a(1)).or_default().color_key = a(2) != 0;
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }
            "SetSurfaceColorKey" => {
                let surf = a(1);
                let (red, green, blue) = (a(2), a(3), a(4));
                let ok =
                    red <= 255 && green <= 255 && blue <= 255 && self.surfaces.contains_key(&surf);
                if ok {
                    self.manip.entry(surf).or_default().key =
                        (red as i32, green as i32, blue as i32);
                }
                put(mem, a(5), if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "GetSurfaceColorKey" => {
                let m = self.manip.get(&a(1)).cloned().unwrap_or_default();
                put(mem, a(2), if m.color_key { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(3), m.key.0 as u32)?;
                put(mem, a(4), m.key.1 as u32)?;
                put(mem, a(5), m.key.2 as u32)?;
                put(mem, a(6), EGL_TRUE)?;
                Ok(0)
            }
            // `IEGLSurfaceManip`: a composite surface is a pbuffer with the window's dimensions;
            // overlays are recorded as bindings and reported by the query calls.
            "CreateCompositeSurface" => {
                let win = a(1);
                let (width, height, target) = self
                    .surfaces
                    .get(&win)
                    .map(|s| (s.width, s.height, s.target))
                    .unwrap_or((64, 64, None));
                let id = self.alloc();
                let mut surface = Surface::new(width, height, SurfaceKind::Window);
                surface.target = target;
                self.surfaces.insert(id, surface);
                self.composites.insert(
                    id,
                    Composite {
                        window: win,
                        layers: HashMap::new(),
                        bindings: HashMap::new(),
                    },
                );
                put(mem, a(3), id)?;
                Ok(0)
            }
            "SurfaceOverlayEnable" => {
                let surf = a(1);
                let enable = a(2);
                let ok = enable <= 1 && self.surfaces.contains_key(&surf);
                if ok {
                    self.manip.entry(surf).or_default().overlay = enable != 0;
                }
                put(mem, a(3), if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "SurfaceOverlayLayerEnable" => {
                let comp = a(1);
                let layer = a(2) as i32;
                let enable = a(3);
                let ok = enable <= 1
                    && self.layer_in_range(layer)
                    && self
                        .composites
                        .get_mut(&comp)
                        .map(|c| {
                            c.layers.entry(layer).or_default().enabled = enable != 0;
                            true
                        })
                        .unwrap_or(false);
                put(mem, a(4), if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "SurfaceOverlayBind" => {
                let comp = a(1);
                let surf = a(2);
                let layer = a(3) as i32;
                let enable_value = a(4);
                let enable = enable_value != 0;
                let mut ok = false;
                if enable_value <= 1
                    && self.layer_in_range(layer)
                    && self.surfaces.contains_key(&surf)
                {
                    // A surface has one binding; moving it first detaches the old layer.
                    for c in self.composites.values_mut() {
                        if let Some(old) = c.bindings.remove(&surf) {
                            if let Some(l) = c.layers.get_mut(&old.layer) {
                                l.surfaces.retain(|s| *s != surf);
                            }
                        }
                    }
                    if !enable {
                        ok = true;
                    } else {
                        let total: usize = self.composites.values().map(|c| c.bindings.len()).sum();
                        let on_layer = self
                            .composites
                            .get(&comp)
                            .and_then(|c| c.layers.get(&layer))
                            .map(|l| l.surfaces.len())
                            .unwrap_or(0);
                        if on_layer < self.overlay.max_surface_per_layer as usize
                            && total < self.overlay.max_total_surfaces as usize
                        {
                            if let Some(c) = self.composites.get_mut(&comp) {
                                c.layers.entry(layer).or_default().surfaces.push(surf);
                                c.bindings.insert(surf, LayerBinding { layer });
                                ok = true;
                            }
                        }
                    }
                }
                put(mem, a(5), if ok { EGL_TRUE } else { EGL_FALSE })?;
                Ok(0)
            }
            "GetSurfaceOverlayBinding" => {
                let surf = a(1);
                let (comp, layer) = self
                    .composites
                    .iter()
                    .find_map(|(comp, c)| c.bindings.get(&surf).map(|b| (*comp, b.layer)))
                    .unwrap_or((0, 0));
                put(mem, a(2), comp)?;
                put(mem, a(3), layer as u32)?;
                put(mem, a(4), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceOverlay" => {
                let surf = a(1);
                let found = self
                    .composites
                    .iter()
                    .find_map(|(comp, c)| c.bindings.get(&surf).map(|b| (*comp, b.layer)));
                let (layer_enable, surf_enable) = match found {
                    Some((comp, layer)) => (
                        self.composites
                            .get(&comp)
                            .and_then(|c| c.layers.get(&layer))
                            .map(|l| l.enabled)
                            .unwrap_or(false),
                        self.manip.get(&surf).map(|m| m.overlay).unwrap_or(false),
                    ),
                    None => (false, false),
                };
                put(mem, a(2), if layer_enable { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(3), if surf_enable { EGL_TRUE } else { EGL_FALSE })?;
                put(mem, a(4), EGL_TRUE)?;
                Ok(0)
            }
            "GetSurfaceOverlayCaps" => {
                let param = a(2);
                if param != 0 {
                    let caps = [
                        u32::from(self.overlay.max_overlay),
                        u32::from(self.overlay.max_underlay),
                        u32::from(self.overlay.max_surface_per_layer),
                        u32::from(self.overlay.max_total_surfaces),
                        self.overlay.pbuffer as u32,
                        self.overlay.pixmap as u32,
                    ];
                    for (i, value) in caps.iter().enumerate() {
                        mem.write_u32(param + 4 * i as u32, *value)?;
                    }
                }
                put(mem, a(3), EGL_TRUE)?;
                Ok(0)
            }

            _ => Ok(crate::abi::generated::AEE_EUNSUPPORTED),
        }
    }

    fn present(&mut self, surface: u32, mem: &mut Mem, display: &mut Display) -> HResult<()> {
        self.flush_color_buffer(surface, mem);
        let target = self.surfaces.get(&surface).and_then(|s| s.target);
        if let Some(target) = target {
            self.blit(surface, target, mem, display)?;
        }
        Ok(())
    }

    /// A pbuffer's guest `IEGLGetColorBuffer` is writable. Flush it back before the surface is
    /// presented or copied, so guest rendering reaches the colour surface.
    fn flush_color_buffer(&mut self, surface: u32, mem: &mut Mem) {
        let Some(s) = self.surfaces.get_mut(&surface) else {
            return;
        };
        let Some(buffer) = s.color_buffer else {
            return;
        };
        let len = s.rgba.len();
        if len == 0 {
            return;
        }
        let Ok(bytes) = mem.read_bytes(buffer, len) else {
            return;
        };
        if bytes.len() == len {
            s.rgba.copy_from_slice(&bytes);
        }
    }

    /// Copy a drawable into the guest `IDIB*` named by `target`, converting to the DIB's own pixel
    /// depth and flipping the row order: GL's window origin is bottom-left, the DIB's first row is
    /// the top. This is the reference layer's `glReadPixels` + vertical swap + `IBitmap_BltIn`.
    fn blit(
        &mut self,
        surface: u32,
        target: u32,
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<()> {
        // A composite presents its own pixels; a window presents its pixels plus every composite
        // that wraps it. A window can have more than one, so the composite id is the tie-breaker
        // inside a layer; otherwise the HashMap order would decide what an app sees.
        let composites: Vec<Composite> = if self.composites.contains_key(&surface) {
            self.composites.get(&surface).cloned().into_iter().collect()
        } else {
            let mut ids: Vec<u32> = self
                .composites
                .iter()
                .filter(|(_, c)| c.window == surface)
                .map(|(id, _)| *id)
                .collect();
            ids.sort();
            ids.into_iter()
                .filter_map(|id| self.composites.get(&id).cloned())
                .collect()
        };
        if composites.is_empty() {
            return self.draw_surface(surface, target, mem, display);
        }
        for layer in -(self.overlay.max_underlay as i32)..=-1 {
            for composite in &composites {
                self.draw_layer(composite, layer, target, mem, display)?;
            }
        }
        // Layer 0 is the base layer on the handset: when a surface is bound there it replaces
        // the composite/window pixels for that presentation. Without one, the composite itself
        // is the base.
        let has_base = composites.iter().any(|composite| {
            composite
                .layers
                .get(&0)
                .map(|layer| {
                    layer.enabled
                        && layer.surfaces.iter().any(|surface| {
                            self.manip.get(surface).map(|m| m.overlay).unwrap_or(false)
                        })
                })
                .unwrap_or(false)
        });
        if has_base {
            for composite in &composites {
                self.draw_layer(composite, 0, target, mem, display)?;
            }
        } else {
            self.draw_surface(surface, target, mem, display)?;
        }
        for layer in 1..=self.overlay.max_overlay as i32 {
            for composite in &composites {
                self.draw_layer(composite, layer, target, mem, display)?;
            }
        }
        Ok(())
    }

    fn draw_layer(
        &mut self,
        composite: &Composite,
        layer: i32,
        target: u32,
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<()> {
        let Some(layer) = composite.layers.get(&layer) else {
            return Ok(());
        };
        if !layer.enabled {
            return Ok(());
        }
        for surface in &layer.surfaces {
            if self.manip.get(surface).map(|m| m.overlay).unwrap_or(false) {
                self.draw_surface(*surface, target, mem, display)?;
            }
        }
        Ok(())
    }

    fn draw_surface(
        &mut self,
        surface: u32,
        target: u32,
        mem: &mut Mem,
        display: &mut Display,
    ) -> HResult<()> {
        self.flush_color_buffer(surface, mem);
        let Some(s) = self.surfaces.get(&surface) else {
            self.error = EGL_BAD_SURFACE;
            return Ok(());
        };
        if s.width == 0 || s.height == 0 {
            return Ok(());
        }
        let Some(b) = display.bitmap(target) else {
            self.error = EGL_BAD_SURFACE;
            return Ok(());
        };
        let m = self.manip.get(&surface).cloned().unwrap_or_default();
        let (src_x, src_y, sw, sh) = if m.scale {
            (
                m.scale_src[0],
                m.scale_src[1],
                m.scale_src[2].max(1),
                m.scale_src[3].max(1),
            )
        } else {
            (0, 0, s.width as i32, s.height as i32)
        };
        let (mut dw, mut dh) = if m.scale && m.scale_dst[2] > 0 && m.scale_dst[3] > 0 {
            (m.scale_dst[2], m.scale_dst[3])
        } else {
            (s.width as i32, s.height as i32)
        };
        if m.rotate && m.rot90 {
            std::mem::swap(&mut dw, &mut dh);
        }
        let dw = dw.min(b.width as i32).max(0) as u32;
        let dh = dh.min(b.height as i32).max(0) as u32;
        let bytes = if b.depth >= 32 { 4 } else { 2 };
        for dy in 0..dh {
            for dx in 0..dw {
                let mut lu = if dw > 0 {
                    dx as i32 * sw / dw as i32
                } else {
                    0
                };
                let mut lv = if dh > 0 {
                    dy as i32 * sh / dh as i32
                } else {
                    0
                };
                if m.rotate {
                    if m.rot90 {
                        let t = lu;
                        lu = lv;
                        lv = sh - 1 - t;
                    }
                    if m.hmirror {
                        lu = sw - 1 - lu;
                    }
                    if m.vmirror {
                        lv = sh - 1 - lv;
                    }
                }
                let sx = (src_x + lu).clamp(0, s.width as i32 - 1) as u32;
                let sy = (src_y + lv).clamp(0, s.height as i32 - 1) as u32;
                let i = (((s.height - 1 - sy) * s.width + sx) * 4) as usize;
                let (cr, cg, cb) = (s.rgba[i], s.rgba[i + 1], s.rgba[i + 2]);
                if m.color_key && (cr as i32, cg as i32, cb as i32) == m.key {
                    continue;
                }
                // The pixel's own alpha is the default source alpha; the explicit transparency
                // state replaces it, and the alpha map multiplies either one. This is what lets a
                // layer surface be blended without every layer caller also setting a constant.
                let mut alpha = if m.transparency {
                    m.alpha.clamp(0, 255) as u32
                } else {
                    s.rgba[i + 3] as u32
                };
                if m.alpha_surface != 0 {
                    if let Some(a) = self.surfaces.get(&m.alpha_surface) {
                        if a.width > 0 && a.height > 0 {
                            let ax = sx.min(a.width - 1);
                            let ay = sy.min(a.height - 1);
                            let ai = (((a.height - 1 - ay) * a.width + ax) * 4) as usize;
                            let map = a.rgba.get(ai + 3).copied().unwrap_or(255) as u32;
                            alpha = alpha * map / 255;
                        }
                    }
                }
                if alpha == 0 {
                    continue;
                }
                let addr = (b.px as i64 + dy as i64 * b.pitch as i64 + dx as i64 * bytes) as u32;
                if bytes == 4 {
                    let (r, g, bl) = if alpha == 255 {
                        (cr as u32, cg as u32, cb as u32)
                    } else {
                        let dst = mem.read_u32(addr)?;
                        let dr = (dst >> 16) & 0xff;
                        let dg = (dst >> 8) & 0xff;
                        let db = dst & 0xff;
                        (
                            (cr as u32 * alpha + dr * (255 - alpha)) / 255,
                            (cg as u32 * alpha + dg * (255 - alpha)) / 255,
                            (cb as u32 * alpha + db * (255 - alpha)) / 255,
                        )
                    };
                    // `IDIB_COLORSCHEME_888` words are `0x00RRGGBB`.
                    mem.write_u32(addr, (r << 16) | (g << 8) | bl)?;
                } else {
                    let (r, g, bl) = if alpha == 255 {
                        (cr, cg, cb)
                    } else {
                        let dst = mem.read_u16(addr)?;
                        let dr = ((dst >> 11) & 0x1f) as u32 * 255 / 31;
                        let dg = ((dst >> 5) & 0x3f) as u32 * 255 / 63;
                        let db = (dst & 0x1f) as u32 * 255 / 31;
                        (
                            ((cr as u32 * alpha + dr * (255 - alpha)) / 255) as u8,
                            ((cg as u32 * alpha + dg * (255 - alpha)) / 255) as u8,
                            ((cb as u32 * alpha + db * (255 - alpha)) / 255) as u8,
                        )
                    };
                    mem.write_u16(addr, pack565(r, g, bl) as u32)?;
                }
            }
        }

        Ok(())
    }
}
