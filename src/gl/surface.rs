//! Implementation blocks for the software pipeline's supporting types.

use super::*;

impl Surface {
    pub(super) fn new(width: u32, height: u32, kind: SurfaceKind) -> Self {
        let n = (width.max(1) * height.max(1)) as usize;
        Surface {
            width: width.max(1),
            height: height.max(1),
            rgba: vec![0; n * 4],
            depth: vec![1.0; n],
            kind,
            target: None,
            stencil: vec![0; n],
            swap_behavior: EGL_BUFFER_DESTROYED,
            texture_format: EGL_TEXTURE_RGBA,
            texture_target: EGL_TEXTURE_2D,
            color_buffer: None,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y.min(self.height - 1) * self.width + x.min(self.width - 1)) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    pub(super) fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as u32 * self.width + x as u32) as usize)
    }
}
