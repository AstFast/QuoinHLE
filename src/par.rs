//! The container the other half of the corpus arrives in: `"PAR "`.
//!
//! A pack's art is not always a `.bar`: some packs ship `.par` files and no `.bar` at all, and a run
//! that only knows [`crate::resource`]'s `0x11` signature mounts none of them and answers the app's
//! first `ISHELL_LoadResString` with nothing. The two formats are unrelated: this one opens with four
//! ASCII bytes instead of a binary signature, and its payloads are deflated rather than stored.
//!
//! Layout, read off the `.par` files in the corpus and self-checking on every one of them:
//!
//! ```text
//! 0x00  "PAR "          u32 file count    u32 offsets[count]
//!       a sub-file is any of: another "PAR "; an indexed payload
//!       (u16 id count, u32 ids[id count], payload); or a payload on its own
//! ```
//!
//! `offsets[j]` is measured from the start of *this* header, and `offsets[0]` is always
//! `8 + 4 * count` - the table is exactly as long as the count says. Those are the two invariants
//! that let a wrong guess about the layout fail at parse time instead of arriving as a garbled
//! picture, and they hold on all fourteen files.
//!
//! An index belongs to the sub-file that follows it, and names the sub-files of the header *it*
//! precedes: `ui.par` is `"PAR "` holding one indexed payload whose eighty-five ids label the
//! eighty-five pictures inside it. Those pictures carry no index of their own - the first two bytes
//! of one are the gzip magic, which the "id count" field of a leaf would otherwise read as a count
//! of `0x8b1f`. A payload that does name ids is `table.par`'s sort, where one payload serves ten of
//! them.
//!
//! A leaf is a gzip stream, and what comes out of it names its own type: a `BM` picture, UTF-16
//! text, or a blob.

use crate::resource::{Error, RESTYPE_BINARY, RESTYPE_IMAGE, RESTYPE_STRING};

/// The four bytes a `.par` opens with, NUL included.
pub const SIGNATURE: &[u8; 4] = b"PAR\0";

/// Bytes of a header: signature, count, and nothing else.
const HEADER_BYTES: usize = 8;

/// A gzip stream starts with these three, which is what a leaf looks like.
const GZIP: &[u8; 3] = b"\x1f\x8b\x08";

/// How many headers [`walk`] will nest before it stops looking.
///
/// The corpus nests twice: `ui.par` is a header holding an indexed payload holding a header. Sixteen
/// is therefore nowhere near the shape of a real file, and a file that keeps answering "header"
/// past that is either constructed or misread.
const MAX_NEST: u8 = 16;

type R<T> = Result<T, Error>;

/// One resource out of a `.par`, already decompressed.
///
/// Owned rather than borrowed, unlike [`crate::resource::Resource`]: a `.par` stores its payloads
/// deflated, so there is no slice of the file to point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParResource {
    /// The kind this reader decided the payload is, named with the same `RESTYPE_*` constants the
    /// `.bar` reader uses.
    pub kind: u16,
    /// The id the enclosing table gave it.
    pub id: u32,
    /// The decompressed payload.
    pub payload: Vec<u8>,
}

/// One leaf as the file stores it, whether or not a table above it named an id.
///
/// [`read`] answers only the leaves with an id because those are the ones a mount can serve. A
/// caller that wants to describe the rest — the report does — asks [`leaves`]; the payloads are the
/// same bytes, and `id: None` says "recognised, but no name to look it up by".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// The first id the enclosing table gave it, or `None` when no table above it named one.
    /// Kept beside [`Leaf::ids`] so a caller that only wants "named or not" does not have to
    /// inspect the list; every id is in `ids`.
    pub id: Option<u32>,
    /// Every id this payload serves, in the order the table listed them. A top-level indexed
    /// payload may serve several; a payload named from above serves exactly its parent's id.
    pub ids: Vec<u32>,
    /// The decompressed payload.
    pub payload: Vec<u8>,
}

/// What one `.par` yields, in the terms a report needs to explain a silent mount.
///
/// [`read`] answers only the resources that came out; this answers why the rest did not. A leaf with
/// no id above it cannot be looked up by `IShell::LoadResData`, so it is not a resource the host can
/// mount — but "recognised the container and mounted nothing" must not look the same as "not a
/// container at all" ([`crate::probe`] prints the difference).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Census {
    /// Leaves filed under an id and returned by [`read`]. A payload whose index names several ids
    /// counts once for each id, because each one is a resource a lookup can reach.
    pub emitted: usize,
    /// Leaves that were recognised but had no id above them, so nothing can name them.
    pub unnamed: usize,
    /// Sub-files that were neither a header, a gzip leaf, nor an indexed payload — including a
    /// gzip member that would not inflate.
    pub unreadable: usize,
}

/// Whether `bytes` starts like a `.par`. Cheap, so a caller can ask before parsing.
pub fn is_par(bytes: &[u8]) -> bool {
    bytes.starts_with(SIGNATURE)
}

fn u16_at(b: &[u8], at: usize) -> R<usize> {
    let Some(w) = b.get(at..at + 2) else {
        return Err(Error::Malformed(format!("no u16 at {at:#x}")));
    };
    Ok(usize::from(u16::from_le_bytes([w[0], w[1]])))
}

fn u32_at(b: &[u8], at: usize) -> R<usize> {
    let Some(w) = b.get(at..at + 4) else {
        return Err(Error::Malformed(format!("no u32 at {at:#x}")));
    };
    Ok(u32::from_le_bytes([w[0], w[1], w[2], w[3]]) as usize)
}

/// Every resource in a `.par`, decompressed and in the order the file lists them.
///
/// A malformed sub-file is skipped, not fatal: the rest of the file still answers. A pack whose
/// seventh picture is framed unexpectedly should still show its other eighty-four. The root is the
/// exception - a file whose *first* header does not add up is not this format at all, and saying so
/// is what lets a caller tell the two containers apart by asking.
pub fn read(bytes: &[u8]) -> R<Vec<ParResource>> {
    if !is_par(bytes) {
        return Err(Error::Malformed(format!(
            "signature is {:02x}{:02x}{:02x}{:02x}, not {SIGNATURE:?}",
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        )));
    }
    spans(bytes, 0, bytes.len())?;
    let mut out = Vec::new();
    let mut count = Census::default();
    let mut seen = 0;
    // The root has no table above it, so nothing names its sub-files.
    walk(
        bytes,
        0,
        bytes.len(),
        &[],
        &mut out,
        &mut count,
        false,
        &mut seen,
        0,
    );
    Ok({
        let mut resources = Vec::new();
        for leaf in out {
            let kind = classify(&leaf.payload);
            let ids = if leaf.ids.is_empty() {
                leaf.id.into_iter().collect::<Vec<_>>()
            } else {
                leaf.ids
            };
            for id in ids {
                resources.push(ParResource {
                    kind,
                    id,
                    payload: leaf.payload.clone(),
                });
            }
        }
        resources
    })
}

/// Every leaf the file stores, including the ones no table named.
///
/// This is the same walk as [`read`], so the two cannot disagree; the difference is only that the
/// unnamed leaves are kept with `id: None` instead of being dropped. A caller must not invent an id
/// for them — `IShell::LoadResData` would never have asked for that name.
///
/// Unlike [`read`] and [`census`], this inflates the unnamed leaves too, because the caller asked
/// for their bytes. That is why it is a separate entry point: a report that only needs to count
/// them should not pay for a decompression it will not use.
pub fn leaves(bytes: &[u8]) -> R<Vec<Leaf>> {
    Ok(leaves_and_census(bytes)?.0)
}

/// [`leaves`] and the census from the same inflating walk.
///
/// [`census`] deliberately does not inflate anonymous leaves, so it cannot tell a corrupt gzip
/// from a good one. This entry point does, and its census is the one a report should use when it
/// already has to pay for the bytes — otherwise an anonymous stream that fails to inflate is
/// reported as merely unnamed.
pub fn leaves_and_census(bytes: &[u8]) -> R<(Vec<Leaf>, Census)> {
    if !is_par(bytes) {
        return Err(Error::Malformed(format!(
            "signature is {:02x}{:02x}{:02x}{:02x}, not {SIGNATURE:?}",
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        )));
    }
    spans(bytes, 0, bytes.len())?;
    let mut out = Vec::new();
    let mut count = Census::default();
    let mut seen = 0;
    walk(
        bytes,
        0,
        bytes.len(),
        &[],
        &mut out,
        &mut count,
        true,
        &mut seen,
        0,
    );
    Ok((out, count))
}

/// The same walk as [`read`], counted rather than returned: how many leaves it can file and why the
/// rest cannot be. Named payloads are inflated; anonymous leaves are only counted, because this
/// path must not pay for a decompression it does not use. Use [`leaves_and_census`] to classify
/// their bytes.
pub fn census(bytes: &[u8]) -> R<Census> {
    if !is_par(bytes) {
        return Err(Error::Malformed(format!(
            "signature is {:02x}{:02x}{:02x}{:02x}, not {SIGNATURE:?}",
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        )));
    }
    spans(bytes, 0, bytes.len())?;
    let mut out = Vec::new();
    let mut count = Census::default();
    let mut seen = 0;
    walk(
        bytes,
        0,
        bytes.len(),
        &[],
        &mut out,
        &mut count,
        false,
        &mut seen,
        0,
    );
    Ok(count)
}

/// The `count` and offsets of the header at `at`, checked against the header's own extent.
///
/// Offsets are measured from the start of *this* header rather than from the file, which is what
/// makes a nested one readable without knowing where in the file it sits.
fn spans(bytes: &[u8], at: usize, limit: usize) -> R<Vec<usize>> {
    if at + HEADER_BYTES > bytes.len() || limit > bytes.len() {
        return Err(Error::Malformed(format!("no header at {at:#x}")));
    }
    let count = u32_at(bytes, at + 4)?;
    // A count that would not fit is a misread, not a big file: every real header's table ends
    // where its own first offset says it does, so this bound is the format's own.
    if count == 0 || at + HEADER_BYTES + 4 * count > limit {
        return Err(Error::Malformed(format!(
            "{count} sub-files do not fit before {limit:#x}"
        )));
    }
    let mut offs = Vec::with_capacity(count);
    for j in 0..count {
        offs.push(u32_at(bytes, at + HEADER_BYTES + 4 * j)?);
    }
    if offs[0] != HEADER_BYTES + 4 * count {
        return Err(Error::Malformed(format!(
            "the table ends at {:#x} but the first sub-file starts at {:#x}",
            HEADER_BYTES + 4 * count,
            offs[0]
        )));
    }
    Ok(offs)
}

/// What an indexed sub-file declares: the ids it serves and where its payload starts.
struct Index {
    ids: Vec<u32>,
    payload: usize,
}

/// Read a sub-file's `u16 count` / `u32 ids[count]` header, if it has a sane one.
fn index(bytes: &[u8], s: usize, e: usize) -> Option<Index> {
    let count = u16_at(bytes, s).ok()?;
    if count == 0 {
        return None;
    }
    let payload = s + 2 + 4 * count;
    if payload > e {
        return None;
    }
    let mut ids = Vec::with_capacity(count);
    for k in 0..count {
        ids.push(u32_at(bytes, s + 2 + 4 * k).ok()? as u32);
    }
    Some(Index { ids, payload })
}

/// Walk the header at `at`, collecting every leaf it reaches.
///
/// `labels[j]` is the id the table above gave this sub-file, when there was one, and that is the id
/// an app asks with: `ui.par` labels its eighty-five pictures with the eighty-five ids one level
/// up, and the pictures themselves carry no index at all. So a leaf belongs under what *its parent*
/// called it. Only when nothing named it does its own table get to answer, which is `table.par`'s
/// shape: fifty-three payloads, each naming the ten ids it serves.
fn walk(
    bytes: &[u8],
    at: usize,
    limit: usize,
    labels: &[u32],
    out: &mut Vec<Leaf>,
    count: &mut Census,
    keep_anonymous: bool,
    seen: &mut usize,
    depth: u8,
) {
    // A bound on the work, not on depth: the corpus nests twice, and anything that keeps finding
    // headers is a misread of the offsets rather than a real pack. Count sub-files rather than
    // `out.len()`, because an unnamed leaf may or may not be kept and must not change how much
    // work the named ones get.
    let Ok(offs) = spans(bytes, at, limit) else {
        count.unreadable += 1;
        return;
    };
    for (j, start) in offs.iter().enumerate() {
        *seen += 1;
        if *seen > 1 << 14 {
            count.unreadable += 1;
            return;
        }
        let s = at + start;
        let e = match offs.get(j + 1) {
            Some(next) => at + next,
            None => limit,
        };
        if s >= e || e > bytes.len() {
            count.unreadable += 1;
            continue;
        }
        // A sub-file that is itself a header. The corpus always reaches one through an index, but a
        // bare one is still worth following, and it carries no labels of its own.
        if bytes[s..].starts_with(SIGNATURE) {
            // Depth is bounded separately from the work above, because that bound cannot see this
            // branch: a file that is nothing but nested headers emits nothing, so `out` never grows
            // and the recursion runs once per twelve bytes of it until the host's stack is gone —
            // which is an abort with no report, attributed to whatever pack happened to ship it.
            if depth >= MAX_NEST {
                count.unreadable += 1;
                continue;
            }
            walk(
                bytes,
                s,
                e,
                &[],
                out,
                count,
                keep_anonymous,
                seen,
                depth + 1,
            );
            continue;
        }
        // A payload stored as it stands. This is most of `ui.par`: the sub-file *is* the gzip
        // stream, and only the table above knows which id it answers for.
        if bytes[s..].starts_with(GZIP) {
            if let Some(&id) = labels.get(j) {
                if emit(&bytes[s..e], &[id], out) {
                    count.emitted += 1;
                } else {
                    count.unreadable += 1;
                }
            } else if keep_anonymous {
                if emit(&bytes[s..e], &[], out) {
                    count.unnamed += 1;
                } else {
                    count.unreadable += 1;
                }
            } else {
                count.unnamed += 1;
            }
            continue;
        }
        let Some(idx) = index(bytes, s, e) else {
            count.unreadable += 1;
            continue;
        };
        let body = &bytes[idx.payload..e];
        if body.starts_with(SIGNATURE) {
            // The index names the *nested* header's sub-files, so it is what carries the ids down.
            if depth >= MAX_NEST {
                count.unreadable += 1;
                continue;
            }
            walk(
                bytes,
                idx.payload,
                e,
                &idx.ids,
                out,
                count,
                keep_anonymous,
                seen,
                depth + 1,
            );
            continue;
        }
        if let Some(&id) = labels.get(j) {
            if emit(body, &[id], out) {
                count.emitted += 1;
            } else {
                count.unreadable += 1;
            }
        } else if emit(body, &idx.ids, out) {
            // A top-level payload with its own table serves every id it names, not just the first:
            // `table.par` is the corpus's shape for this and a lookup for the other ids must land.
            count.emitted += idx.ids.len();
        } else {
            count.unreadable += 1;
        }
    }
}

/// Inflate one leaf and file it under every id it serves.
fn emit(body: &[u8], ids: &[u32], out: &mut Vec<Leaf>) -> bool {
    let Ok(payload) = inflate(body) else {
        return false;
    };
    out.push(Leaf {
        id: ids.first().copied(),
        ids: ids.to_vec(),
        payload,
    });
    true
}

/// Undo the deflate a leaf stores its payload in.
///
/// The ceiling on what one leaf may inflate to, in bytes.
///
/// The same number and the same reasoning as the resource inflater: large enough for the biggest
/// payload in the corpus, small enough that reaching it says something about the stream rather than
/// about the machine. Without it a leaf of a few kilobytes can ask for gigabytes — deflate tops out
/// around a thousand to one — and `read_to_end` will happily grow a `Vec` until the process dies.
const INFLATE_CAP: u64 = 64 << 20;

/// Every leaf in the corpus is a gzip member. A stream that does not decompress is skipped by the
/// caller rather than reported: the file as a whole is still readable.
fn inflate(body: &[u8]) -> R<Vec<u8>> {
    if !body.starts_with(GZIP) {
        // Not deflated: the corpus has none of these, but a payload that is already plain is
        // strictly more useful than a hole, so it is passed through.
        return Ok(body.to_vec());
    }
    let mut out = Vec::new();
    let mut decoder = std::io::Read::take(flate2::read::GzDecoder::new(body), INFLATE_CAP);
    std::io::Read::read_to_end(&mut decoder, &mut out)
        .map_err(|e| Error::Malformed(format!("gzip payload: {e}")))?;
    // Exactly the cap means the stream was cut off, not that it ended: a payload that really is
    // that size is not one of these.
    if out.len() as u64 == INFLATE_CAP {
        return Err(Error::Malformed(format!(
            "gzip payload inflates to {INFLATE_CAP} bytes or more"
        )));
    }
    Ok(out)
}

/// What a decompressed payload is, in the `RESTYPE_*` terms the rest of the emulator speaks.
///
/// The `.par` format records no type: the leaves are bare pictures and text, so the payload has to
/// name itself. A BMP says so in its first two bytes; text is the only other shape one of these
/// containers holds, and it is UTF-16, so it is dense in NULs.
fn classify(payload: &[u8]) -> u16 {
    if payload.starts_with(b"BM") {
        return RESTYPE_IMAGE;
    }
    if is_utf16_text(payload) {
        return RESTYPE_STRING;
    }
    RESTYPE_BINARY
}

/// Whether `payload` reads as NUL-terminated UTF-16, the shape `ISHELL_LoadResString` answers with.
fn is_utf16_text(payload: &[u8]) -> bool {
    if payload.len() < 4 || payload.len() % 2 != 0 {
        return false;
    }
    let chars = payload.len() / 2;
    let mut sane = 0;
    for c in payload.chunks_exact(2) {
        let unit = u16::from_le_bytes([c[0], c[1]]);
        if unit == 0 || matches!(unit, 0x09 | 0x0a | 0x0d) || (0x20..0x3000).contains(&unit) {
            sane += 1;
        }
    }
    sane * 10 >= chars * 9
}

#[cfg(test)]
#[path = "test/par.rs"]
mod tests;
