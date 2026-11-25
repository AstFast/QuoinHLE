//! `--gl-log`: one line per draw, naming the state that decided the fragment colour.
//!
//! This exists because the HLE call ring keeps only its most recent few thousand entries
//! ([`crate::hle::Hle`]), so a texture upload early in a load is long gone by the time a frame is
//! drawn — and "why is this quad white" is answered by the upload as much as by the draw. The log
//! is a read-only observer: nothing here changes a pixel, and with the flag absent no probe is
//! ever armed and each fragment costs one `Option` test.
//!
//! The rows are formatted here and printed by the binary, so a test can read them without
//! capturing stdout.

use std::cell::Cell;

use super::*;

/// The first fragment a draw wrote, with the pixel it replaced.
#[derive(Clone, Copy)]
pub(crate) struct Frag {
    pub x: i32,
    pub y: i32,
    pub dst: [u8; 4],
    /// The shaded colour before blending, so a blend that hides everything is visible as such.
    pub pre: [f32; 4],
    pub out: [u8; 4],
}

/// What one draw was told, captured when it was issued, plus what it produced.
pub(crate) struct DrawProbe {
    /// Everything up to the fragment line.
    head: String,
    /// Screen bounds of the projected vertices and the eye-space depth they came from, filled in
    /// by [`Gles::submit`] once the vertices are transformed.
    geometry: Cell<Option<([f32; 4], (f32, f32))>>,
    frag: Cell<Option<Frag>>,
    /// The first sampled texel of unit 0 — the value behind "the quad is white".
    texel: Cell<Option<[f32; 4]>>,
    written: Cell<u64>,
    opaque: Cell<u64>,
}

impl DrawProbe {
    pub(super) fn note_geometry(&self, bbox: [f32; 4], eye: (f32, f32)) {
        self.geometry.set(Some((bbox, eye)));
    }

    /// Count one fragment that passed depth, stencil and the alpha test. `opaque` says its alpha
    /// reached one half, which is what tells "this draw painted nothing visible" from "this draw
    /// painted nothing at all".
    pub(super) fn note_written(&self, opaque: bool) {
        self.written.set(self.written.get() + 1);
        if opaque {
            self.opaque.set(self.opaque.get() + 1);
        }
    }

    pub(super) fn note_fragment(&self, x: i32, y: i32, dst: [u8; 4], pre: [f32; 4], out: [u8; 4]) {
        if self.frag.get().is_none() {
            self.frag.set(Some(Frag {
                x,
                y,
                dst,
                pre,
                out,
            }));
        }
    }

    pub(super) fn note_texel(&self, t: [f32; 4]) {
        if self.texel.get().is_none() {
            self.texel.set(Some(t));
        }
    }

    /// Close the row: the geometry, the first written fragment, and how much the draw painted.
    pub(super) fn finish(self) -> String {
        let mut row = self.head;
        if let Some((bbox, eye)) = self.geometry.get() {
            row.push_str(&format!(
                " bbox={:.0},{:.0}..{:.0},{:.0} eyez={:.2}..{:.2}",
                bbox[0], bbox[1], bbox[2], bbox[3], eye.0, eye.1
            ));
        }
        match self.frag.get() {
            Some(f) => row.push_str(&format!(
                "\n  | frag@({},{}) dst=({},{},{},{}) pre=({:.2},{:.2},{:.2},{:.2}) out=({},{},{},{})",
                f.x,
                f.y,
                f.dst[0],
                f.dst[1],
                f.dst[2],
                f.dst[3],
                f.pre[0],
                f.pre[1],
                f.pre[2],
                f.pre[3],
                f.out[0],
                f.out[1],
                f.out[2],
                f.out[3]
            )),
            None => row.push_str("\n  | frag=none (no fragment reached the colour write)"),
        }
        row.push_str(&format!(
            " written={} opaque={}",
            self.written.get(),
            self.opaque.get()
        ));
        if let Some(t) = self.texel.get() {
            row.push_str(&format!(
                " texel0=({:.2},{:.2},{:.2},{:.2})",
                t[0], t[1], t[2], t[3]
            ));
        }
        row
    }
}

/// The rows `--gl-log` collected, oldest first, with the count of everything asked for so a
/// truncated list says so.
pub struct GlDrawLog {
    rows: Vec<String>,
    limit: usize,
    total: u64,
}

impl GlDrawLog {
    pub fn new(limit: usize) -> Self {
        Self {
            rows: Vec::new(),
            limit,
            total: 0,
        }
    }

    pub(super) fn push(&mut self, row: String) {
        self.total += 1;
        if self.rows.len() < self.limit {
            self.rows.push(row);
        }
    }

    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    /// How many rows were asked for, including the ones past [`GlDrawLog::rows`]'s end.
    pub fn total(&self) -> u64 {
        self.total
    }
}

// ---- naming ---------------------------------------------------------------------------------
//
// The enums below are only ever printed, so a name per value is worth more than the number. An
// unknown value keeps its hex rather than being guessed at.

pub(super) fn prim(mode: u32) -> String {
    let name = match mode {
        GL_POINTS => "POINTS",
        GL_LINES => "LINES",
        GL_LINE_LOOP => "LINE_LOOP",
        GL_LINE_STRIP => "LINE_STRIP",
        GL_TRIANGLES => "TRIANGLES",
        GL_TRIANGLE_STRIP => "TRIANGLE_STRIP",
        GL_TRIANGLE_FAN => "TRIANGLE_FAN",
        _ => return format!("{mode:#x}"),
    };
    name.to_string()
}

pub(super) fn filter(m: u32) -> String {
    let name = match m {
        GL_NEAREST => "NEAREST",
        GL_LINEAR => "LINEAR",
        GL_NEAREST_MIPMAP_NEAREST => "NEAREST_MIPMAP_NEAREST",
        GL_LINEAR_MIPMAP_NEAREST => "LINEAR_MIPMAP_NEAREST",
        GL_NEAREST_MIPMAP_LINEAR => "NEAREST_MIPMAP_LINEAR",
        GL_LINEAR_MIPMAP_LINEAR => "LINEAR_MIPMAP_LINEAR",
        _ => return format!("{m:#x}"),
    };
    name.to_string()
}

pub(super) fn tex_env(m: u32) -> String {
    let name = match m {
        GL_MODULATE => "MODULATE",
        GL_DECAL => "DECAL",
        GL_BLEND => "BLEND",
        GL_REPLACE => "REPLACE",
        GL_ADD => "ADD",
        _ => return format!("{m:#x}"),
    };
    name.to_string()
}

pub(super) fn compare(f: u32) -> String {
    let name = match f {
        GL_NEVER => "NEVER",
        GL_LESS => "LESS",
        GL_EQUAL => "EQUAL",
        GL_LEQUAL => "LEQUAL",
        GL_GREATER => "GREATER",
        GL_NOTEQUAL => "NOTEQUAL",
        GL_GEQUAL => "GEQUAL",
        GL_ALWAYS => "ALWAYS",
        _ => return format!("{f:#x}"),
    };
    name.to_string()
}

pub(super) fn fog_mode(m: u32) -> String {
    let name = match m {
        GL_LINEAR => "LINEAR",
        GL_EXP => "EXP",
        GL_EXP2 => "EXP2",
        _ => return format!("{m:#x}"),
    };
    name.to_string()
}

/// A colour as the log spells it: three decimals, because the question being asked is usually
/// "is this 1.0 or is it 0.8".
pub(super) fn colour(c: [f32; 4]) -> String {
    format!("({:.3},{:.3},{:.3},{:.3})", c[0], c[1], c[2], c[3])
}

/// The same for a byte pixel.
pub(super) fn pixel(p: [u8; 4]) -> String {
    format!("({},{},{},{})", p[0], p[1], p[2], p[3])
}

// ---- capturing ------------------------------------------------------------------------------

impl Gles {
    /// Arm the probe for one draw, or do nothing when `--gl-log` is off.
    ///
    /// Called before [`Gles::submit`] rather than inside it, because a draw that never reaches the
    /// rasteriser is exactly as interesting as one that does — and because the probe has to be
    /// taken back unconditionally afterwards, which only the caller can promise.
    pub(super) fn arm_probe(&mut self, egl: &Egl, kind: &str, mode: u32, count: usize) {
        let Some(log) = egl.gl_log.as_ref() else {
            return;
        };
        let mut head = format!(
            "gl#{} draw {} mode={} n={} vp={},{},{},{}",
            log.total() + 1,
            kind,
            prim(mode),
            count,
            self.viewport[0],
            self.viewport[1],
            self.viewport[2],
            self.viewport[3]
        );
        // The fixed-point word the pack wrote, recovered from the colour this host kept. Exact for
        // a `Color4x` caller (f32 carries every word up to 2^24 through /65536 and back), and the
        // right ballpark for a `Color4f` one, which had no word to begin with.
        let raw: Vec<String> = self
            .current
            .color
            .iter()
            .map(|c| format!("{:#x}", (c * 65536.0).round() as i32))
            .collect();
        head.push_str(&format!(
            "\n  | col={} raw=({}) carr={} light={} cm={} flat={}",
            colour(self.current.color),
            raw.join(","),
            if self.client.color.is_some() { "on" } else { "off" },
            if self.lighting { "on" } else { "off" },
            if self.color_material { "on" } else { "off" },
            if self.shade_model == GL_FLAT { "on" } else { "off" },
        ));
        head.push_str("\n  |");
        for unit in 0..4 {
            let id = self.bound_texture[unit];
            let object = self.textures.get(&id);
            let named = self.texture_2d[unit] || id != 0;
            if unit > 0 && !named {
                head.push_str(&format!(" tex{unit}=off |"));
                continue;
            }
            let size = match object {
                Some(t) => format!("{}x{}", t.width, t.height),
                None => "none".to_string(),
            };
            head.push_str(&format!(
                " tex{unit}={} id={:#x} {size} img={} min={} mag={} env={} |",
                if self.texture_2d[unit] { "on" } else { "off" },
                id,
                match object {
                    Some(t) if t.uploaded => "yes",
                    Some(_) => "NO",
                    None => "none",
                },
                match object {
                    Some(t) => filter(t.min),
                    None => "-".to_string(),
                },
                match object {
                    Some(t) => filter(t.mag),
                    None => "-".to_string(),
                },
                tex_env(self.tex_env_mode[unit]),
            ));
        }
        head.push_str(&format!(
            "\n  | at={}({},{:.2}) blend={} {:#x}/{:#x} logic={}({:#x}) cmask={}{}{}{}",
            if self.alpha_test { "on" } else { "off" },
            compare(self.alpha_func),
            self.alpha_ref,
            if self.blend { "on" } else { "off" },
            self.blend_src,
            self.blend_dst,
            if self.logic_op_enabled { "on" } else { "off" },
            self.logic_op,
            if self.color_mask[0] { 1 } else { 0 },
            if self.color_mask[1] { 1 } else { 0 },
            if self.color_mask[2] { 1 } else { 0 },
            if self.color_mask[3] { 1 } else { 0 },
        ));
        head.push_str(&format!(
            "\n  | fog={}({},{:.3},{:.3}..{:.3},{}) depth={}({}) mask={}",
            if self.fog { "on" } else { "off" },
            fog_mode(self.fog_mode),
            self.fog_density,
            self.fog_start,
            self.fog_end,
            colour(self.fog_color),
            if self.depth_test { "on" } else { "off" },
            compare(self.depth_func),
            if self.depth_mask { "on" } else { "off" },
        ));
        self.probe = Some(Box::new(DrawProbe {
            head,
            geometry: Cell::new(None),
            frag: Cell::new(None),
            texel: Cell::new(None),
            written: Cell::new(0),
            opaque: Cell::new(0),
        }));
    }

    /// Close the armed probe and file it, whatever the draw did — a draw that faulted still has a
    /// row, and a probe left armed would attribute the *next* draw's first pixel to this one.
    pub(super) fn finish_probe(&mut self, egl: &mut Egl) {
        let Some(probe) = self.probe.take() else {
            return;
        };
        if let Some(log) = egl.gl_log.as_mut() {
            log.push(probe.finish());
        }
    }

    /// File a row that is not a draw: a texture upload or a clear.
    pub(super) fn note_row(egl: &mut Egl, body: String) {
        if let Some(log) = egl.gl_log.as_mut() {
            log.push(format!("gl#{} {body}", log.total() + 1));
        }
    }
}

impl Gles {
    /// One `gl#` row for a texture upload: what was named, and what the image turned out to be.
    ///
    /// The statistics are the point. "The quad is white" and "the image the quad samples is white"
    /// are different bugs, and `first`/`mean`/`alpha0` tell them apart without another run.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn note_upload(
        egl: &mut Egl,
        name: &str,
        id: u32,
        level: u32,
        width: u32,
        height: u32,
        format: u32,
        ty: u32,
        pixels: u32,
        rgba: &[u8],
    ) {
        if egl.gl_log.is_none() {
            return;
        }
        let count = (rgba.len() / 4) as u64;
        let mut sum = [0u64; 3];
        let mut alpha0 = 0u64;
        for px in rgba.chunks_exact(4) {
            for (c, total) in sum.iter_mut().enumerate() {
                *total += px[c] as u64;
            }
            if px[3] == 0 {
                alpha0 += 1;
            }
        }
        let mean = if count > 0 {
            [sum[0] / count, sum[1] / count, sum[2] / count]
        } else {
            [0; 3]
        };
        let first = match rgba.get(0..4) {
            Some(p) => pixel([p[0], p[1], p[2], p[3]]),
            None => "(empty)".to_string(),
        };
        let transparent = if count > 0 {
            alpha0 as f64 * 100.0 / count as f64
        } else {
            0.0
        };
        let row = format!(
            "tex {name} id={id:#x} level={level} {width}x{height} fmt={format:#x} type={ty:#x} \
             px={pixels:#010x} first={first} mean=({},{},{}) alpha0={transparent:.0}% img=yes",
            mean[0], mean[1], mean[2]
        );
        Self::note_row(egl, row);
    }

    /// A texture that was named but never given an image still has to say something when a pack
    /// asks it to become one — this is the row for the paths whose pixels this host copies
    /// verbatim rather than decoding.
    pub(super) fn note_upload_plain(egl: &mut Egl, name: &str, id: u32, body: &str) {
        Self::note_row(egl, format!("tex {name} id={id:#x} {body}"));
    }
}
