//! Everything the Web interfaces of this host are made of, in one place.
//!
//! | module | what it is |
//! |---|---|
//! | [`html`] | the `IHtmlViewer` document model: parse, lay out, paint |
//! | [`options`] | the `HVOPT_*` ids and their `HVERR_*` flags — the one hand-written part of the ABI surface |
//! | [`viewer`] | `Runtime`'s `IWeb`, `IWebResp`, `IWebUtil` and `IHtmlViewer` slots |
//! | [`widget`] | `Widgets`' side of an `IHtmlViewer` control: the option table, the document, the decode cache |
//!
//! The plain functions below are the string work an `IWebUtil` answers with: splitting a URL into
//! its parts, resolving a reference against a base, percent encoding, base64, and
//! `x-www-urlencoded` form fields.
//!
//! Everything here is pure — byte slices in, byte slices and offsets out — because the interface
//! holds no state and each of its seven methods is a function of the text it was handed. Offsets
//! come back rather than addresses so the arithmetic is checkable without a guest;
//! [`crate::runtime`] is what turns them into addresses the module can dereference.
//!
//! The transport is not here: [`crate::net`] is also what `--net-proxy` stands for outside the
//! browser, so a request this module makes is one the socket layer could have made too.
//!
//! Tests stay in `src/test/` with every other module's, which is the one convention this tree
//! keeps for all of them.

pub mod html;
pub mod options;
pub mod viewer;
pub mod widget;

/// Members of `UrlParts`, in declaration order: `cpcSchm`, `cpcAuth`, `cpcUser`, `cpcPass`,
/// `cpcHost`, `cpcPort`, `cpcPath`, `cpcSrch`, `cpcName`, `cpcEnd`.
pub const URL_PARTS: usize = 10;

/// `UrlParts` as byte offsets into `url`, which is the text without its terminator — so `cpcEnd`
/// comes back as `url.len`, the address the caller's NUL sits at.
///
/// The published contract is that a part which is absent takes the value of the part after it, and
/// that is what lets the presence tests work as pointer comparisons: `UP_HASHOST` is "the port
/// pointer differs from the host pointer", `UP_HASAUTH` is "the path pointer differs from the
/// authority pointer". So the collapsing has to happen here rather than be
/// reported as a flag, and a caller that only compares two of the ten pointers needs no error
/// channel to read.
///
/// Every bound below is the one `ParseUrl` actually uses, including two
/// that read wider than a modern parser would: the scheme test stops only at a slash and lets
/// `isscheme` reject the rest, and the path is the first slash anywhere after the user part, so a
/// slash inside a query makes the authority reach past the question mark.
pub fn parse_url(url: &[u8]) -> [usize; URL_PARTS] {
    let n = url.len();
    let find =
        |byte: u8, from: usize, to: usize| (from..to).find(|&i| url[i] == byte).unwrap_or(to);
    //: a colon before the first slash,: not at position zero, and with
    // `isscheme`: every byte in front of it is a letter, a digit, `+`, `-` or `.`.
    // Nothing here mentions a `?` or a `#` — the character test is what rejects `a b://x`.
    let slash = (0..n).find(|&i| url[i] == b'/').unwrap_or(n);
    let colon = (0..slash).find(|&i| url[i] == b':');
    let legal = |c: usize| {
        c > 0
            && url[..c]
                .iter()
                .all(|&b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'-' | b'.'))
    };
    let auth = match colon {
        Some(c) if legal(c) => c + 1,
        _ => 0,
    };
    if !url[auth..].starts_with(b"//") {
        //: no authority, so user, password, host, port and path all collapse onto the byte
        // after the colon, and what follows is opaque data the caller reads as one path. That is how
        // `mailto:someone@where` answers as a scheme plus a path rather than as a host.
        let name = find(b'#', auth, n);
        return [
            0,
            auth,
            auth,
            auth,
            auth,
            auth,
            auth,
            find(b'?', auth, name),
            name,
            n,
        ];
    }
    let body = auth + 2;
    //: the first slash at or after the user part, searched to the end of the text. Only
    // when the walk lands on the terminator does it fall back to the question mark, and then to the
    // hash — so in `http://a?b/c` the path is the slash inside the query, not the question mark.
    let path = find(b'/', body, n);
    let host = (body..path)
        .rfind(|&i| url[i] == b'@')
        .map_or(body, |a| a + 1);
    // and: each of the two searches stops at the part it sits inside, so an absent
    // password lands on the host and an absent port on the path.
    let pass = find(b':', body, host);
    let port = find(b':', host, path);
    //: both the fragment and the query are searched from the path, and the query never
    // looks past the fragment, which is what makes a `#` before a `?` win.
    let name = find(b'#', path, n);
    let srch = find(b'?', path, name);
    [0, auth, body, pass, host, port, path, srch, name, n]
}

/// `IWEBUTIL_MakeUrl`: the resolved URL, and the number of bytes a caller has to be told before it
/// can fit the answer.
///
/// The two sizes differ on purpose, and the answer is **two** past the merged text rather than one.
/// `MakeUrl` writes through an `AEEBufBound` whose `Wrote` counts the terminator it puts on its own, and the counting answer is
/// `Wrote + 1` because the coalescing pass may need to insert a slash it has not written
/// yet. A caller that asks once with a null buffer and allocates what it was told has
/// to be given that room, or the second call refuses a buffer the first one sized.
pub fn make_url(base: &[u8], rel: &[u8]) -> (Vec<u8>, usize) {
    let b = parse_url(base);
    let r = parse_url(rel);
    let rel_scheme = r[1] - r[0];
    //. `&&` binds tighter than `||`, so the case-insensitive comparison of the base's
    // leading bytes against the reference's scheme runs whatever the length test said — and it is 0
    // when the reference names no scheme, because then its length is 0. A reference in the base's
    // own scheme therefore keeps going through the merge with its scheme taken off, which is
    // RFC 2396 §5.2 and not what a plain "has a scheme" test would do.
    let verbatim = (r[1] != r[0] && rel_scheme != b[1] - b[0])
        || (rel_scheme > 0
            && rel_scheme <= base.len()
            && !equal_folded(&base[..rel_scheme], &rel[..rel_scheme]));
    //: from here the reference is contributed from its authority on, which is its beginning
    // when it names no scheme and the byte after the colon when it names the base's own.
    let (base_end, slash) = if verbatim {
        //: the base contributes nothing, and the reference contributes all of it.
        (0, false)
    } else if r[6] != r[1] {
        //: a comes with its own authority keeps only the base's scheme.
        (b[1], false)
    } else if r[7] != r[6]
        && ((b[6] == b[1] && byte_at(base, b[6]) != b'/') || byte_at(rel, r[6]) == b'/')
    {
        //: a rooted path, or a base with no authority at all, keeps scheme and authority.
        (b[6], false)
    } else if r[7] != r[6] {
        (
            (b[6]..b[7]).rfind(|&i| base[i] == b'/').unwrap_or(b[6]),
            true,
        )
    } else {
        //: a query, a fragment, or nothing at all in the reference.
        let end = if r[8] != r[7] {
            b[7]
        } else if r[9] != r[8] {
            b[8]
        } else {
            b[9]
        };
        // The one `bNeedSlash` of this group: an authority with no path that the cut stopped at.
        (
            end,
            b[6] != b[1] && byte_at(base, b[6]) != b'/' && end == b[7],
        )
    };
    let mut merged = base[..base_end].to_vec();
    if slash {
        merged.push(b'/');
    }
    merged.extend_from_slice(&rel[if verbatim { 0 } else { r[1] }..]);
    let needed = merged.len() + 2;
    (remove_dots(&merged), needed)
}

/// `STRNICMP` over whole slices: equal length, equal bytes with `A` and `a` the same letter.
fn equal_folded(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// The byte at `at`, or the zero of the terminator when `at` is the end of the text — which is what
/// the reference member reads there too, since every one of its `cpcPath[0]` tests is allowed to land
/// on the NUL.
fn byte_at(s: &[u8], at: usize) -> u8 {
    s.get(at).copied().unwrap_or(0)
}

/// RFC 2396 section 5.2's `remove_dot_segments`, over the path component only.
///
/// `MakeUrl` runs this only on a result that has an authority or whose path
/// begins with a slash: for anything else the remainder is opaque data, and rewriting `..` inside a
/// `mailto:` address would be wrong on its face. Inside that gate it first **puts a slash at the
/// path** if the answer came out without one — which is how a base of `http://a` with
/// nothing in the reference becomes `http://a/` and not `http://a` — and then deletes every `/.` and
/// `/element/..` it finds, by moving the tail of the buffer down over them.
///
/// A `..` with nothing above it is left where it is, because the deletion's start point comes from
/// `aee_memrchrbegin` over the path only and the walk re-reads the same position after each move;
/// what that helper answers for a miss has no body in this drop.
fn remove_dots(url: &[u8]) -> Vec<u8> {
    let u = parse_url(url);
    if u[6] == u[1] && byte_at(url, u[6]) != b'/' {
        return url.to_vec();
    }
    // The slash the reference member makes room for, when the answer came out of the merge with an
    // authority and no path at all.
    let insert = byte_at(url, u[6]) != b'/';
    let mut seg = Vec::new();
    if insert {
        seg.push(b'/');
    }
    seg.extend_from_slice(&url[u[6]..u[7]]);
    let mut out = url[..u[6]].to_vec();
    out.extend(remove_dot_segments(&seg));
    out.extend_from_slice(&url[u[7]..]);
    out
}

fn remove_dot_segments(path: &[u8]) -> Vec<u8> {
    let mut input = path.to_vec();
    let mut out: Vec<u8> = Vec::new();
    while !input.is_empty() {
        if input.starts_with(b"/./") {
            input.drain(..3);
            out.push(b'/');
        } else if input == b"/." {
            input.clear();
            out.push(b'/');
        } else if input.starts_with(b"/../") {
            // Only the `/..` goes, not the slash after it: the slash belongs to the segment that
            // follows, and pushing one of our own onto what has been kept would turn `a/../../x`
            // into `a//x` rather than `/x`.
            input.drain(..3);
            pop_segment(&mut out);
        } else if input == b"/.." {
            input.clear();
            pop_segment(&mut out);
            out.push(b'/');
        } else {
            let skip = usize::from(input[0] == b'/');
            let next = skip
                + input[skip..]
                    .iter()
                    .position(|&b| b == b'/')
                    .unwrap_or(input.len() - skip);
            out.extend_from_slice(&input[..next]);
            input.drain(..next);
        }
    }
    out
}

/// Drops the last segment of what has been kept so far, its slash with it.
fn pop_segment(out: &mut Vec<u8>) {
    let keep = out.iter().rposition(|&b| b == b'/').unwrap_or(0);
    out.truncate(keep);
}

/// Percent encoding, limited to `limit` bytes of output. Returns the bytes emitted and how many of
/// `src` they came from, so a caller that ran out of room can tell a truncated result from a
/// complete one without walking the escapes to find a boundary.
///
/// The set that survives unescaped is the reference one — letters, digits, and `* -. _`
/// (`ISUNRESERVED`, the delivered body, chosen against IE and Mozilla rather than against RFC), and a
/// space becomes a `+` rather than an escape. A `~` is therefore escaped, and a `*` is
/// not. Everything else becomes `%` and two **uppercase** hex digits from `gszHexDigits`.
pub fn url_encode(src: &[u8], limit: usize) -> (Vec<u8>, usize) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = Vec::new();
    for (i, &b) in src.iter().enumerate() {
        let cost = if needs_escape(b) { 3 } else { 1 };
        if out.len() + cost > limit {
            return (out, i);
        }
        if needs_escape(b) {
            out.extend_from_slice(&[b'%', HEX[(b >> 4) as usize], HEX[(b & 15) as usize]]);
        } else {
            out.push(if b == b' ' { b'+' } else { b });
        }
    }
    (out, src.len())
}

/// `ISQUOTE`: not alphanumeric, not one of `* -. _`, and not a space — which is
/// the last of those because a space has its own one-byte form and so is never an escape.
fn needs_escape(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && !matches!(b, b'*' | b'-' | b'.' | b'_' | b' ')
}

/// The decoding half, in the same limited form. A plus is a space, and the only other
/// escape accepted is `%` followed by two hex digits.
///
/// A `%` that is not followed by two hex digits is emitted as itself and costs one byte; a `%` with
/// fewer than two bytes left after it stops the walk **without consuming itself**,
/// because the reference member does not decode partial escapes. That is why the second half of the
/// answer is a count of bytes read and not simply the length of the input.
pub fn url_decode(src: &[u8], limit: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut at = 0;
    while at < src.len() {
        let (byte, cost) = match src[at] {
            b'%' if at + 2 < src.len() && hex2(&src[at + 1..at + 3]).is_some() => {
                (hex2(&src[at + 1..at + 3]).unwrap_or(0), 3)
            }
            b'%' if at + 2 >= src.len() => return (out, at),
            b'+' => (b' ', 1),
            b => (b, 1),
        };
        if out.len() + 1 > limit {
            return (out, at);
        }
        out.push(byte);
        at += cost;
    }
    (out, src.len())
}

fn hex2(pair: &[u8]) -> Option<u8> {
    let digit = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    };
    Some(digit(*pair.first()?)? << 4 | digit(*pair.get(1)?)?)
}

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 with the standard alphabet and `=` padding, without a terminator — a caller sizing its
/// buffer with `BASE64LEN` has already counted one.
pub fn b64_encode(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len().div_ceil(3) * 4);
    for chunk in src.chunks(3) {
        let n = u32::from_be_bytes([
            0,
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ]);
        let sextet = |i: u32| ALPHABET[(n >> i & 63) as usize];
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { b'=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { b'=' });
    }
    out
}

/// `DecodeBase64`: the bytes it wrote, and whether it finished.
///
/// The second half of the answer is the interface's `-1`, and it is not a rollback: the reference
/// member writes each byte as it decodes it, so a string that turns out to be bogus part-way
/// through leaves its prefix in the caller's buffer (precedes the checks at, and
/// every `return -1` abandons the walk from there). A caller that reads the buffer only on a
/// non-negative answer never sees it; one that reads it anyway sees what a handset wrote.
///
/// Three ways a string is refused, all of them before a byte is looked at in the reference order:
/// its length is not a multiple of four (so `"QQ"` is refused and `"QQ=="` is not); a
/// character outside the alphabet, which the table `index_64` extends to every control
/// character, space, `-`, `.`, and every byte at or above 128; and padding in the wrong
/// place — `=` is legal only as the third or fourth character of the last group, and only if the
/// string ends right after it.
pub fn b64_decode(src: &[u8]) -> (Vec<u8>, bool) {
    if src.len() % 4 != 0 {
        return (Vec::new(), false);
    }
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    let six = |b: u8| ALPHABET.iter().position(|&c| c == b).map(|p| p as u32);
    // and: padding is allowed for the third and fourth characters of a group and for
    // nothing else. Its value is never used, because both branches that could have read it return
    // before the byte that needs it.
    let sextet = |b: u8| if b == b'=' { Some(0) } else { six(b) };
    let three = |n: u32| [(n >> 16 & 255) as u8, (n >> 8 & 255) as u8, (n & 255) as u8];
    let mut out = Vec::new();
    let mut group = 0;
    while group < src.len() {
        // The walk ends on the terminator, which for a string whose length is a multiple of four can
        // only land on a group boundary — that is what the length test at buys.
        if at(group) == 0 {
            return (out, true);
        }
        let (Some(a), Some(b)) = (six(at(group)), six(at(group + 1))) else {
            return (out, false);
        };
        let (Some(c), Some(d)) = (sextet(at(group + 2)), sextet(at(group + 3))) else {
            return (out, false);
        };
        let word = (a << 18) | (b << 12) | (c << 6) | d;
        out.push(three(word)[0]);
        if at(group + 2) == b'=' {
            //: one byte, then the string must be over, and the fourth character must be
            // padding too.
            if at(group + 4) != 0 || at(group + 3) != b'=' {
                return (out, false);
            }
            return (out, true);
        }
        out.push(three(word)[1]);
        if at(group + 3) == b'=' {
            if at(group + 4) != 0 {
                return (out, false);
            }
            return (out, true);
        }
        out.push(three(word)[2]);
        group += 4;
    }
    (out, true)
}

/// The `IWEBUTIL_ParseFormFields` walk, over the caller's own buffer, which has to end in a NUL
/// of its own because that NUL is where the last field stops.
///
/// Returns the offset of each field's name and value, then the offset of the next field after
/// them. `mutate` is the interface's null-termination: text is moved up to sit right after the
/// previous field's terminator, which is also what makes the decoding safe, since decoded text is
/// never longer than the escapes it came from. With `mutate` off nothing is written at all and the
/// two offsets point at raw, unterminated text — which is what a call that only wants a count is
/// for.
pub fn parse_form_fields(
    buf: &mut [u8],
    from: usize,
    max: usize,
    mutate: bool,
    decode: bool,
) -> (Vec<(usize, usize)>, usize) {
    let is_sep = |b: u8| b == b'&' || b == b';';
    let end = buf.len() - 1;
    let mut fields = Vec::new();
    let mut i = from;
    while fields.len() < max {
        while i < end && is_sep(buf[i]) {
            i += 1;
        }
        if i >= end {
            i = end;
            break;
        }
        let stop = (i..end).find(|&k| is_sep(buf[k])).unwrap_or(end);
        let eq = (i..stop).find(|&k| buf[k] == b'=');
        let name_len = eq.unwrap_or(stop) - i;
        let src = eq.map_or(stop, |e| e + 1);
        let (name, value) = if mutate {
            let mut at = place(buf, i, name_len, i, decode);
            buf[at] = 0;
            at += 1;
            let dst = if eq.is_some() { at } else { stop };
            buf[place(buf, src, stop - src, dst, decode)] = 0;
            (i, dst)
        } else {
            (i, src)
        };
        fields.push((name, value));
        i = if stop < end { stop + 1 } else { end };
    }
    (fields, i)
}

/// Copies `len` bytes from `src` to `dst`, decoding as it goes, and returns where it stopped.
/// `dst` never runs ahead of `src`, so the two may only overlap in the direction that is safe.
///
/// This is `ForceUrlDecode`: the decoded run, then whatever the decode
/// refused to consume moved up behind it. A `%` with fewer than two bytes after it is left
/// unconsumed by the decoder, so without this second move the tail of a field would
/// simply vanish — and the length the caller writes its terminator at would be short.
fn place(buf: &mut [u8], src: usize, len: usize, dst: usize, decode: bool) -> usize {
    if !decode {
        buf.copy_within(src..src + len, dst);
        return dst + len;
    }
    let (out, used) = url_decode(&buf[src..src + len], usize::MAX);
    buf[dst..dst + out.len()].copy_from_slice(&out);
    buf.copy_within(src + used..src + len, dst + out.len());
    dst + out.len() + (len - used)
}

/// Split a `Content-Type` value into the media type `WebRespInfo.cpszContentType` wants and the
/// `charset` parameter `WebRespInfo.cpszCharset` wants.
///
/// The two are separate fields in the ABI (`cpszContentType` is the MIME type,
/// `cpszCharset` is the character set), and a server sends one header: `text/html; charset=utf-8`.
/// A host that copied the whole header into the first field would answer a pack's
/// `strcmp(cpszContentType, "text/html")` with "not equal" and leave the second one null, which is
/// a document whose encoding nobody named. Parameter names are case-insensitive and values may be
/// quoted (RFC 2045); the media type comes back lowercased, because that is how MIME tokens are
/// compared, while the charset value keeps its spelling and only loses its quotes.
///
/// An empty media type (a header that begins with `;`) is returned as `""`; the caller keeps the
/// original text in that case rather than publishing an empty type.
pub fn split_content_type(value: &str) -> (String, Option<String>) {
    let mut fields = value.split(';');
    let media = fields.next().unwrap_or("").trim().to_ascii_lowercase();
    let mut charset = None;
    for field in fields {
        let Some((name, raw)) = field.split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("charset") {
            continue;
        }
        let value = raw.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(value)
            .trim();
        if !value.is_empty() {
            charset = Some(value.to_string());
        }
    }
    (media, charset)
}

#[cfg(test)]
#[path = "../test/web.rs"]
mod tests;
