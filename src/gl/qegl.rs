//! The QEGL object family: one creation, eleven guest-visible faces.
//!
//! `ISHELL_CreateInstance(AEECLSID_QEGL)` does not make one object. It makes an EGL display and,
//! alongside it, every face a pack can reach with `QueryInterface`: the GLES rasterizer, the 1.1
//! and extension faces on top of it, the EGL 1.1 face, and the Qualcomm color-buffer and imageon
//! faces. They are allocated together and released together, so they are one thing with eleven
//! names rather than eleven things that happen to know each other.
//!
//! This module holds the two facts that used to be spread over twenty `HashMap`s and eleven
//! `QueryInterface` arms: **which face answers which iid**, and **which face is which address**.
//! Nothing here touches the guest; the caller supplies the address and writes the answer back.
//!
//! The reachability is deliberately not symmetric, because the shipped layers are not: `IGLES11`
//! cannot answer `AEEIID_EGL11`, and `IEGL10` cannot answer `AEEIID_GLES11` at all — the 1.1 faces
//! are reached from `IGLES10`, not from the EGL object. Widening it would be a compatibility change
//! that no header asks for.

use crate::abi::generated as g;

/// One face of a QEGL family.
///
/// The order is the creation order in `create_instance`; it carries no meaning beyond that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// `IEGL10` — the object `CreateInstance` hands back, and the root of the family.
    Egl,
    /// `IGLES10` — the software rasterizer.
    Gl,
    /// `IGLES11`.
    Gl11,
    /// `IGLES11Ext`.
    Gl11Ext,
    /// `IGLES11ExtPak` — the packed-extension face `openglesextension.dll` adds.
    Gl11ExtPak,
    /// `IEGL11`.
    Egl11,
    /// `IEGLGetColorBuffer`.
    ColorBuffer,
    /// `IEGLGetColorBufferInfo`.
    ColorBufferInfo,
    /// `IGLES10Ext` — one slot, `QueryMatrixxOES`.
    Gl10Ext,
    /// `IGLESImageonExt`.
    ImageOn,
    /// `IGLESImageonExt_V1`.
    ImageOnV1,
}

impl Face {
    pub const ALL: [Face; 11] = [
        Face::Egl,
        Face::Gl,
        Face::Gl11,
        Face::Gl11Ext,
        Face::Gl11ExtPak,
        Face::Egl11,
        Face::ColorBuffer,
        Face::ColorBufferInfo,
        Face::Gl10Ext,
        Face::ImageOn,
        Face::ImageOnV1,
    ];

    /// The interface name the ABI tables publish for this face.
    pub const fn iface(self) -> &'static str {
        match self {
            Face::Egl => "IEGL10",
            Face::Gl => "IGLES10",
            Face::Gl11 => "IGLES11",
            Face::Gl11Ext => "IGLES11Ext",
            Face::Gl11ExtPak => "IGLES11ExtPak",
            Face::Egl11 => "IEGL11",
            Face::ColorBuffer => "IEGLGetColorBuffer",
            Face::ColorBufferInfo => "IEGLGetColorBufferInfo",
            Face::Gl10Ext => "IGLES10Ext",
            Face::ImageOn => "IGLESImageonExt",
            Face::ImageOnV1 => "IGLESImageonExt_V1",
        }
    }

    /// Which face a `QueryInterface` slot reached through `iface` belongs to.
    ///
    /// This is the inverse of [`Face::iface`], and it is the only place the string names appear
    /// twice; [`tests::the_names_round_trip`] is what keeps the two halves together.
    pub fn of_iface(iface: &str) -> Option<Face> {
        Face::ALL.into_iter().find(|f| f.iface() == iface)
    }

    /// The iids this face answers *as itself*, which is what `answer_qi` is given.
    ///
    /// `IEGL10` answers two: the shipped runtime registers `AEECLSID_QEGL` under the same object
    /// as `AEEIID_EGL10`, and packs query both. `AEEIID_IQI` is added by `answer_qi` for every
    /// interface, so it is not repeated here.
    pub const fn own_iids(self) -> &'static [u32] {
        match self {
            Face::Egl => &[g::AEEIID_EGL10, g::AEECLSID_QEGL],
            Face::Gl => &[g::AEEIID_GLES10],
            Face::Gl11 => &[g::AEEIID_GLES11],
            Face::Gl11Ext => &[g::AEEIID_GLES11EXT],
            Face::Gl11ExtPak => &[g::AEEIID_GLES11EXTPAK],
            Face::Egl11 => &[g::AEEIID_EGL11],
            Face::ColorBuffer => &[g::AEEIID_EGLGETCOLORBUFFER],
            Face::ColorBufferInfo => &[g::AEEIID_EGLGETCOLORBUFFERINFO],
            Face::Gl10Ext => &[g::AEEIID_GLES10EXT],
            Face::ImageOn => &[g::AEEIID_GLESIMAGEONEXT],
            Face::ImageOnV1 => &[g::AEEIID_GLESIMAGEONEXT_V1],
        }
    }
}

/// Every address in one family, indexed by face.
///
/// The fields are private so a face can only be read through [`Faces::addr`]; a caller that names
/// `faces.gl11` directly would be one more place that has to learn a new face was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Faces {
    egl: u32,
    gl: u32,
    gl11: u32,
    gl11ext: u32,
    gl11extpak: u32,
    egl11: u32,
    colorbuffer: u32,
    colorbufferinfo: u32,
    gl10ext: u32,
    imageon: u32,
    imageon_v1: u32,
}

impl Faces {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        egl: u32,
        gl: u32,
        gl11: u32,
        gl11ext: u32,
        gl11extpak: u32,
        egl11: u32,
        colorbuffer: u32,
        colorbufferinfo: u32,
        gl10ext: u32,
        imageon: u32,
        imageon_v1: u32,
    ) -> Faces {
        Faces {
            egl,
            gl,
            gl11,
            gl11ext,
            gl11extpak,
            egl11,
            colorbuffer,
            colorbufferinfo,
            gl10ext,
            imageon,
            imageon_v1,
        }
    }

    pub const fn addr(self, face: Face) -> u32 {
        match face {
            Face::Egl => self.egl,
            Face::Gl => self.gl,
            Face::Gl11 => self.gl11,
            Face::Gl11Ext => self.gl11ext,
            Face::Gl11ExtPak => self.gl11extpak,
            Face::Egl11 => self.egl11,
            Face::ColorBuffer => self.colorbuffer,
            Face::ColorBufferInfo => self.colorbufferinfo,
            Face::Gl10Ext => self.gl10ext,
            Face::ImageOn => self.imageon,
            Face::ImageOnV1 => self.imageon_v1,
        }
    }

    /// `(face, address)` for each face, which is what registering the family walks.
    pub fn iter(self) -> impl Iterator<Item = (Face, u32)> {
        Face::ALL.into_iter().map(move |f| (f, self.addr(f)))
    }
}

/// One address of a family: which face it is, and the family it belongs to.
///
/// `Runtime` keys its one `qegl` map by address and stores this, so a slot that is handed nothing
/// but `this` can answer "which face is this" and "where is the one I want" in one lookup.
#[derive(Debug, Clone, Copy)]
pub struct Occupant {
    pub face: Face,
    pub faces: Faces,
}

/// What a `QueryInterface` does when the edge has no answer.
///
/// Two shapes, because the shipped layers have two. An edge that is part of the object's own
/// structure answers the caller's pointer (`unwrap_or(this)` in the arms this replaced): asking
/// `IGLES10` for `AEEIID_EGL10` on something that is not a QEGL family still hands back what it
/// was given. An edge into an optional face declines, and the caller falls through to the identity
/// list, which is how `IGLES11` ends up refusing `AEEIID_GLES11EXT` on a runtime without the
/// extension layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Miss {
    /// Hand the caller back the pointer it asked with.
    AnswerSelf,
    /// Fall through to the identity list (`own_iids`), which answers `AEE_CLASSNOTSUPPORT`
    /// for anything else.
    Decline,
}

/// `(from, iid) -> (to, what to do when the family is not there)`.
///
/// Read as: "a `from` face asked for `iid` answers with the `to` face". The eleven rows-per-face
/// are the arms this replaced, in the same order, so a diff against the old code is a diff against
/// this table. `AEEIID_IQI` and the identity iids are not rows — [`Face::own_iids`] covers them.
const ANSWERS: &[(Face, u32, Face, Miss)] = &[
    // `IEGL10`: `CEGL10_QueryInterface` hands back the embedded rasterizer, the EGL 1.1 face and
    // the two Qualcomm color-buffer faces, all of which hang directly off the EGL object.
    (Face::Egl, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::Egl, g::AEEIID_EGL11, Face::Egl11, Miss::Decline),
    (Face::Egl, g::AEEIID_EGLGETCOLORBUFFER, Face::ColorBuffer, Miss::Decline),
    (
        Face::Egl,
        g::AEEIID_EGLGETCOLORBUFFERINFO,
        Face::ColorBufferInfo,
        Miss::Decline,
    ),
    (Face::Egl, g::AEEIID_GLESIMAGEONEXT, Face::ImageOn, Miss::Decline),
    (
        Face::Egl,
        g::AEEIID_GLESIMAGEONEXT_V1,
        Face::ImageOnV1,
        Miss::Decline,
    ),
    // `IGLES10`: down to EGL, up to 1.1 and the 10 extensions, and across to imageon. Not to
    // `AEEIID_GLES11EXT` — that one is reached from `IGLES11`.
    (Face::Gl, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::Gl, g::AEEIID_GLES11, Face::Gl11, Miss::Decline),
    (Face::Gl, g::AEEIID_GLES10EXT, Face::Gl10Ext, Miss::Decline),
    (Face::Gl, g::AEEIID_GLESIMAGEONEXT, Face::ImageOn, Miss::Decline),
    (Face::Gl, g::AEEIID_GLESIMAGEONEXT_V1, Face::ImageOnV1, Miss::Decline),
    // `IGLES11`.
    (Face::Gl11, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::Gl11, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::Gl11, g::AEEIID_GLES11EXT, Face::Gl11Ext, Miss::Decline),
    // `IGLES11Ext`.
    (Face::Gl11Ext, g::AEEIID_GLES11, Face::Gl11, Miss::AnswerSelf),
    (Face::Gl11Ext, g::AEEIID_GLES11EXTPAK, Face::Gl11ExtPak, Miss::Decline),
    (Face::Gl11Ext, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::Gl11Ext, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    // `IGLES11ExtPak`. The packed face sits at the bottom of the chain, so it answers every face
    // above it; the reverse direction (ext -> pak) is the one `openglesextension.dll` adds.
    (Face::Gl11ExtPak, g::AEEIID_GLES11EXT, Face::Gl11Ext, Miss::AnswerSelf),
    (Face::Gl11ExtPak, g::AEEIID_GLES11, Face::Gl11, Miss::AnswerSelf),
    (Face::Gl11ExtPak, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::Gl11ExtPak, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    // `IEGL11` and the two color-buffer faces: back down to `IEGL10`, and out to each other.
    (Face::Egl11, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::Egl11, g::AEECLSID_QEGL, Face::Egl, Miss::AnswerSelf),
    (Face::Egl11, g::AEEIID_EGLGETCOLORBUFFER, Face::ColorBuffer, Miss::Decline),
    (
        Face::Egl11,
        g::AEEIID_EGLGETCOLORBUFFERINFO,
        Face::ColorBufferInfo,
        Miss::Decline,
    ),
    (Face::ColorBuffer, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::ColorBuffer, g::AEECLSID_QEGL, Face::Egl, Miss::AnswerSelf),
    (Face::ColorBuffer, g::AEEIID_EGL11, Face::Egl11, Miss::Decline),
    (Face::ColorBufferInfo, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::ColorBufferInfo, g::AEECLSID_QEGL, Face::Egl, Miss::AnswerSelf),
    (Face::ColorBufferInfo, g::AEEIID_EGL11, Face::Egl11, Miss::Decline),
    // `IGLES10Ext`: up to the rasterizer and its EGL object, nothing else.
    (Face::Gl10Ext, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::Gl10Ext, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    // imageon, both revisions: to the rasterizer and EGL, and to each other.
    (Face::ImageOn, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::ImageOn, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::ImageOn, g::AEECLSID_QEGL, Face::Egl, Miss::AnswerSelf),
    (Face::ImageOn, g::AEEIID_GLESIMAGEONEXT_V1, Face::ImageOnV1, Miss::Decline),
    (Face::ImageOnV1, g::AEEIID_GLES10, Face::Gl, Miss::AnswerSelf),
    (Face::ImageOnV1, g::AEEIID_EGL10, Face::Egl, Miss::AnswerSelf),
    (Face::ImageOnV1, g::AEECLSID_QEGL, Face::Egl, Miss::AnswerSelf),
    (Face::ImageOnV1, g::AEEIID_GLESIMAGEONEXT, Face::ImageOn, Miss::Decline),
];

/// What `from` answers `iid` with, or `None` for "not an edge".
pub fn answer(from: Face, iid: u32) -> Option<(Face, Miss)> {
    ANSWERS
        .iter()
        .find(|(f, i, _, _)| *f == from && *i == iid)
        .map(|(_, _, to, miss)| (*to, *miss))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_names_round_trip() {
        for face in Face::ALL {
            assert_eq!(Face::of_iface(face.iface()), Some(face));
        }
    }

    #[test]
    fn every_face_is_named_once_and_answers_as_itself() {
        let mut names = HashSet::new();
        let mut iids = HashSet::new();
        for face in Face::ALL {
            assert!(names.insert(face.iface()), "{} is two faces", face.iface());
            for iid in face.own_iids() {
                assert!(iids.insert(*iid), "{iid:#010x} is on two faces");
            }
        }
        assert_eq!(
            names.len(),
            11,
            "the family is eleven faces — a twelfth needs an entry in `Faces`, a row here and a \
             `create_instance` call site"
        );
    }

    #[test]
    fn no_face_answers_the_same_iid_twice() {
        let mut seen = HashSet::new();
        for (from, iid, to, _) in ANSWERS {
            assert!(
                seen.insert((*from, *iid)),
                "{} answers {iid:#010x} twice, so the second row is dead",
                from.iface()
            );
            assert_ne!(*from, *to, "{} answers with itself", from.iface());
        }
    }

    /// The 1.1 faces are reachable from `IGLES10` and from each other, never from the EGL object.
    /// This is the shape the shipped layers have; see the module comment.
    #[test]
    fn the_one_one_faces_hang_off_gles10() {
        assert_eq!(answer(Face::Egl, g::AEEIID_GLES11), None);
        assert_eq!(answer(Face::Egl, g::AEEIID_GLES11EXT), None);
        assert_eq!(answer(Face::Gl, g::AEEIID_GLES11), Some((Face::Gl11, Miss::Decline)));
        assert_eq!(
            answer(Face::Gl11, g::AEEIID_GLES11EXT),
            Some((Face::Gl11Ext, Miss::Decline))
        );
        assert_eq!(
            answer(Face::Gl11ExtPak, g::AEEIID_GLES11EXT),
            Some((Face::Gl11Ext, Miss::AnswerSelf))
        );
        // EGL 1.1 is a sibling of the rasterizer, not an ancestor of it.
        assert_eq!(answer(Face::Gl11, g::AEEIID_EGL11), None);
    }
}
