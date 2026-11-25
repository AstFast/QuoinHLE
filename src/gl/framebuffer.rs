//! Implementation blocks for the software pipeline's supporting types.

use super::*;

impl Framebuffer {
    pub(super) fn empty() -> Self {
        Framebuffer {
            color: 0,
            color_level: 0,
            color_target: GL_TEXTURE_2D,
            surface: Surface::new(1, 1, SurfaceKind::Pbuffer),
            depth: None,
            stencil: None,
            depth_texture: None,
            stencil_texture: None,
            depth_target: GL_TEXTURE_2D,
            stencil_target: GL_TEXTURE_2D,
        }
    }
}

impl DrawTarget {
    pub(super) fn depth_tested(&self, depth_test: bool) -> bool {
        depth_test
            && (self.framebuffer == 0 || self.depth.is_some() || self.depth_texture.is_some())
    }

    /// Whether the stencil test runs for this target.
    ///
    /// A framebuffer tests the stencil only when it has a plane attached. The default surface
    /// depends on the profile instead: a handset that reports no stencil bits has nothing to test
    /// against, even though this host keeps a plane for it.
    pub(super) fn stencil_tested(&self, stencil_test: bool, screen_has_stencil: bool) -> bool {
        if !stencil_test {
            return false;
        }
        if self.framebuffer == 0 {
            screen_has_stencil
        } else {
            self.stencil.is_some() || self.stencil_texture.is_some()
        }
    }
}
