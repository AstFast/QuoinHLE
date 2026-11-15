// Moved out of `src/web.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

fn parts(url: &str) -> [usize; URL_PARTS] {
    let bytes = url.as_bytes();
    let p = parse_url(bytes);
    assert!(
        p.windows(2).all(|w| w[0] <= w[1]) && *p.last().unwrap() == bytes.len(),
        "{url} splits out of order: {p:?}"
    );
    p
}

#[test]
fn a_full_url_splits_where_the_header_says_it_does() {
    let url = "http://user:pass@host:port/path/doc.htm?srch#name";
    assert_eq!(
        parts(url),
        [0, 5, 7, 11, 17, 21, 26, 39, 44, 49],
        "scheme, authority, user, password, host, port, path, search, name, end"
    );
}

#[test]
fn an_absent_part_becomes_the_one_after_it_so_presence_is_a_pointer_comparison() {
    // "http://user@host/path#name" has no password and no port, so the two pointers that would
    // name them land on the host and the path instead — which is how `UP_HASPASS` and
    // `UP_HASPORT` are written, as a comparison rather than as a flag.
    assert_eq!(
        parts("http://user@host/path#name"),
        [0, 5, 7, 12, 12, 16, 16, 21, 21, 26]
    );
    assert_eq!(
        parts("mailto:brewery@qualcomm.com"),
        [0, 7, 7, 7, 7, 7, 7, 27, 27, 27],
        "a mailto has a scheme and a path, and no host to go looking for"
    );
    assert_eq!(parts("http://a"), [0, 5, 7, 7, 7, 8, 8, 8, 8, 8]);
    assert_eq!(parts(""), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        parts(":x"),
        [0, 0, 0, 0, 0, 0, 0, 2, 2, 2],
        "a colon at position zero is not a scheme"
    );
}

#[test]
fn a_reference_lands_in_the_directory_the_base_last_wrote_in() {
    let base = b"http://a/b/c/d;p?q";
    let cases = [
        ("g", "http://a/b/c/g"),
        ("./g", "http://a/b/c/g"),
        ("g/", "http://a/b/c/g/"),
        ("/g", "http://a/g"),
        ("/g?y", "http://a/g?y"),
        ("//h/g", "http://h/g"),
        ("?y", "http://a/b/c/d;p?y"),
        ("g?y", "http://a/b/c/g?y"),
        ("#s", "http://a/b/c/d;p?q#s"),
        ("", "http://a/b/c/d;p?q"),
        (".", "http://a/b/c/"),
        ("..", "http://a/b/"),
        ("g/./h", "http://a/b/c/g/h"),
        ("g/../h", "http://a/b/c/h"),
        ("../g", "http://a/b/g"),
        ("g;x=;/y", "http://a/b/c/g;x=;/y"),
        ("http://x/y", "http://x/y"),
        // A reference in the base's own scheme is not taken verbatim: compares the two
        // scheme lengths and then the bytes, and on a match the reference goes on through the merge
        // with its scheme taken off, which is RFC 2396 §5.2 and not "has a scheme".
        ("http:g", "http://a/b/c/g"),
        ("HTTP:g", "http://a/b/c/g"),
        ("ftp:g", "ftp:g"),
    ];
    for (rel, want) in cases {
        let (got, _) = make_url(base, rel.as_bytes());
        assert_eq!(got, want.as_bytes(), "base {base:?} + {rel:?}");
    }
}

#[test]
fn the_size_reported_for_a_coalescing_reference_is_the_size_before_it_coalesced() {
    // Two past the merged text, not one: `AEEBufBound_Wrote` counts the terminator the member puts on
    // its own (the header), and the answer is that plus one for the slash
    // the coalescing pass may still insert. A buffer of exactly `Wrote` is refused
    // (`IsFull` is `Left < 1`,), so the sizing answer has to clear it.
    let (got, needed) = make_url(b"http://a/b/c/d;p?q", b"../g");
    assert_eq!(got, b"http://a/b/g");
    assert_eq!(
        needed, 19,
        "\"http://a/b/c/d;p?q\" merged with \"../g\" is 17 bytes, plus the terminator written and the slash kept spare"
    );
    let (got, needed) = make_url(b"http://a/b/c/d;p?q", b"g");
    assert_eq!(got, b"http://a/b/c/g");
    assert_eq!(needed, 16, "with nothing to coalesce it is one less");
}

#[test]
fn a_result_with_an_authority_and_no_path_is_given_its_slash() {
    //: inside the coalescing gate, a path that does not start with a slash gets one
    // shoved in before the dot walk, with its query pointer moved to match. This is what a base
    // whose reference asks for nothing still answers as a directory rather than as a host.
    assert_eq!(make_url(b"http://a", b"").0, b"http://a/");
    assert_eq!(make_url(b"http://a/b/c", b"?q").0, b"http://a/b/c?q");
    // A reference that is a lone query against a base with nothing but an authority takes the
    // slash the shipped member only inserts when the cut stopped at the query.
    assert_eq!(make_url(b"http://a", b"?q").0, b"http://a/?q");
}

#[test]
fn a_relative_reference_never_walks_out_of_the_authority() {
    let (got, _) = make_url(b"http://a", b"../../x");
    assert_eq!(got, b"http://a/x");
    let (got, _) = make_url(b"http://a/b", b"g");
    assert_eq!(got, b"http://a/g", "the base's own file is not a directory");
    let (got, _) = make_url(b"mailto:me@here", b"x/y");
    assert_eq!(
        got, b"mailto:x/y",
        "an opaque base has no directory to join onto, but the branch that notices that \
         (`:394-400`) still copies the base up to its own path, scheme and colon included, and \
         `:460` then refuses to coalesce because the result has neither an authority nor a \
         rooted path"
    );
    let (got, _) = make_url(b"", b"g");
    assert_eq!(got, b"g");
}

#[test]
fn encoding_passes_what_the_shipped_set_passes_and_pluses_a_space() {
    // `ISUNRESERVED` is `* -. _` only (chosen against IE and Mozilla rather than
    // against RFC), `ISQUOTE` adds "and not a space", and a space becomes a `+` at.
    // So a tilde is escaped, a star is not, and a space costs one byte rather than three.
    let (got, used) = url_encode(b"a b/c?d=e&f~-_.*", usize::MAX);
    assert_eq!(got, b"a+b%2Fc%3Fd%3De%26f%7E-_.*");
    assert_eq!(used, 16);
    let (back, used) = url_decode(&got, usize::MAX);
    assert_eq!(back, b"a b/c?d=e&f~-_.*");
    assert_eq!(used, got.len());
    assert_eq!(
        url_decode(b"a+b%2b", usize::MAX).0,
        b"a b+",
        "a plus is a space, an escaped plus is still a plus"
    );
    //: a percent with fewer than two bytes after it is not decoded and not consumed, so
    // the answer is the text before it and the byte count stops short of the input.
    let (out, used) = url_decode(b"100%", usize::MAX);
    assert_eq!(out, b"100");
    assert_eq!(
        used, 3,
        "the percent is left for the caller, which is what moves it"
    );
    // A percent with two bytes after that are not hex is a character like any other ( fails,
    // and the value written is the `%` itself).
    let (out, used) = url_decode(b"a%zz", usize::MAX);
    assert_eq!(out, b"a%zz");
    assert_eq!(used, 4);
}

#[test]
fn a_short_output_buffer_stops_at_a_whole_escape() {
    //: the walk counts the *output* bytes it may still write, and a `%XX` that has fewer
    // than three of them left ends the run before it, so the input count stops short of that byte
    // rather than in the middle of its escape.
    for limit in 0..=9 {
        let (got, used) = url_encode(b"a b/c", limit);
        assert!(got.len() <= limit);
        assert!(!got.ends_with(b"%"), "{got:?} cut an escape in half");
        let (again, all) = url_encode(&b"a b/c"[..used], usize::MAX);
        assert_eq!(again, got, "the prefix at {limit} is all of {used} bytes");
        assert_eq!(all, used);

        let (got, used) = url_decode(b"a%20b", limit);
        assert!(got.len() <= limit);
        let (again, _) = url_decode(&b"a%20b"[..used], usize::MAX);
        assert_eq!(
            again, got,
            "the decode prefix at {limit} is all of {used} bytes"
        );
    }
}

#[test]
fn base64_round_trips_every_group_length() {
    for src in [
        "".as_bytes(),
        b"b",
        b"ba",
        b"bab",
        b"babc",
        &[0u8, 255, 128, 64, 1][..],
    ] {
        let enc = b64_encode(src);
        assert_eq!(enc.len() + 1, (((src.len() + 2) / 3) * 4) + 1, "BASE64LEN");
        let (back, ok) = b64_decode(&enc);
        assert!(ok, "{src:?} came out of the encoder, so it must decode");
        assert_eq!(back, src, "{src:?} via {}", String::from_utf8_lossy(&enc));
    }
    assert_eq!(
        b64_encode(b"any carnal pleasure."),
        b"YW55IGNhcm5hbCBwbGVhc3VyZS4="
    );
    assert_eq!(b64_decode(b"YQ=="), (b"a".to_vec(), true), "two pads");
    assert_eq!(b64_decode(b"YWI="), (b"ab".to_vec(), true), "one pad");
    // refuses on the length before it looks at a single character, which is why the
    // unpadded pair and the wrapped line are both refused here: the shipped member takes a
    // null-terminated string and has no notion of folding at all.
    assert_eq!(b64_decode(b"YQ"), (Vec::new(), false), "an unpadded pair");
    assert_eq!(
        b64_decode(b"YW55\nIGNh"),
        (Vec::new(), false),
        "a wrapped line"
    );
    assert_eq!(
        b64_decode(b"Y*"),
        (Vec::new(), false),
        "out of the alphabet, and odd"
    );
    assert_eq!(
        b64_decode(b"Y"),
        (Vec::new(), false),
        "a lone sextet is not data"
    );
    // and: a whitespace or a `-` or a byte at or above 128 is not in the table either.
    assert_eq!(
        b64_decode(b"QQ QQ"),
        (Vec::new(), false),
        "a space is not in the alphabet"
    );
    assert_eq!(
        b64_decode(b"AAAA===="),
        (vec![0, 0, 0], false),
        "padding mid-string, after the first group was decoded"
    );
    // A refusal is not a rollback: the first group was already written (, all
    // precede the checks that follow them).
    assert_eq!(
        b64_decode(b"AAAA*AAA"),
        (vec![0, 0, 0], false),
        "what it decoded stays in the buffer"
    );
    assert_eq!(
        b64_decode(b""),
        (Vec::new(), true),
        "an empty string decodes to nothing"
    );
}

#[test]
fn form_fields_split_and_terminate_in_place() {
    let mut buf = form("x=1&y=2;;z=&;=4&w");
    let (fields, next) = parse_form_fields(&mut buf, 0, 10, true, false);
    let got: Vec<(&str, &str)> = fields
        .iter()
        .map(|(n, v)| (text(&buf, *n), text(&buf, *v)))
        .collect();
    assert_eq!(
        got,
        [("x", "1"), ("y", "2"), ("z", ""), ("", "4"), ("w", "")]
    );
    assert_eq!(next, buf.len() - 1, "the whole string was consumed");

    let mut once = form("a=b&c=d");
    let (fields, next) = parse_form_fields(&mut once, 0, 1, true, false);
    assert_eq!(fields.len(), 1);
    assert_eq!(next, 4, "the second field is where the caller resumes");
    assert_eq!(
        (text(&once, fields[0].0), text(&once, fields[0].1)),
        ("a", "b")
    );

    let mut none = form("&&&&&");
    assert_eq!(parse_form_fields(&mut none, 0, 9, true, false).0.len(), 0);
    let mut empty = form("");
    assert_eq!(parse_form_fields(&mut empty, 0, 9, true, false).0.len(), 0);
}

#[test]
fn a_decoded_field_is_shorter_than_the_text_it_came_from() {
    let mut buf = form("name=a%20b&x=%3D");
    let (fields, next) = parse_form_fields(&mut buf, 0, 4, true, true);
    let got: Vec<(&str, &str)> = fields
        .iter()
        .map(|(n, v)| (text(&buf, *n), text(&buf, *v)))
        .collect();
    assert_eq!(got, [("name", "a b"), ("x", "=")]);
    assert_eq!(next, buf.len() - 1);

    let mut raw = form("a%20b=c");
    let (fields, _) = parse_form_fields(&mut raw, 0, 1, true, false);
    assert_eq!(text(&raw, fields[0].0), "a%20b");
    assert_eq!(
        text(&raw, fields[0].1),
        "c",
        "the value moves up beside the name"
    );

    let quiet = form("a=b;c");
    let mut untouched = quiet.clone();
    let (fields, next) = parse_form_fields(&mut untouched, 0, 3, false, false);
    assert_eq!(
        untouched, quiet,
        "without the mutate flag nothing is written"
    );
    assert_eq!(fields.len(), 2);
    assert_eq!(
        next, 5,
        "the second field was the last one, so the resume point is its end"
    );
}

/// A form buffer the way the interface receives one: the text, and its own terminator after it.
fn form(s: &str) -> Vec<u8> {
    let mut buf = s.as_bytes().to_vec();
    buf.push(0);
    buf
}

fn text(buf: &[u8], at: usize) -> &str {
    let end = buf[at..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| at + p)
        .unwrap_or(buf.len());
    std::str::from_utf8(&buf[at..end]).unwrap()
}

#[test]
fn a_content_type_is_split_into_its_media_type_and_charset() {
    assert_eq!(
        split_content_type("text/html; charset=utf-8"),
        ("text/html".to_string(), Some("utf-8".to_string()))
    );
    assert_eq!(
        split_content_type("text/html;charset=\"UTF-8\""),
        ("text/html".to_string(), Some("UTF-8".to_string()))
    );
    assert_eq!(
        split_content_type("IMAGE/PNG"),
        ("image/png".to_string(), None)
    );
    assert_eq!(
        split_content_type("application/xhtml+xml; boundary=x; charset=gb2312"),
        (
            "application/xhtml+xml".to_string(),
            Some("gb2312".to_string())
        )
    );
    // A malformed header starting with `;` has no media type; the caller keeps the original
    // text rather than publishing an empty MIME token.
    assert_eq!(
        split_content_type("; charset=utf-8"),
        (String::new(), Some("utf-8".to_string()))
    );
    assert_eq!(split_content_type(""), (String::new(), None));
    assert_eq!(
        split_content_type("text/plain; charset="),
        ("text/plain".to_string(), None)
    );
}
