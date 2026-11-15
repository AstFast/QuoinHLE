// Moved out of `src/json.rs` so that file is implementation only. The `#[path]` attribute on the
// `mod tests;` declaration in it is what keeps this a child module of that file, and that is what
// lets these tests reach its private items with `use super::*`.

use super::*;

/// The tree, the types, and the order the keys were written in — the three things every reader of
/// this module assumes at once.
#[test]
fn a_document_reads_as_the_tree_it_describes() {
    let value = parse(r#"{"a": 1, "b": [true, null, "x"], "c": {"d": -2.5e3}, "e": {}}"#)
        .expect("a valid document parses");
    let object = value.as_object().expect("an object");
    assert_eq!(object.get("a").and_then(Value::as_u64), Some(1));
    assert_eq!(
        object.get("c").and_then(Value::as_object).map(Map::len),
        Some(1)
    );
    let items = object.get("b").and_then(Value::as_array).expect("an array");
    assert_eq!(items[0].as_bool(), Some(true));
    assert_eq!(items[1], Value::Null);
    assert_eq!(items[2].as_str(), Some("x"));
    // A float literal is *not* a whole number: a field that means "pixels" should say so by failing.
    let d = object
        .get("c")
        .and_then(Value::as_object)
        .and_then(|o| o.get("d"))
        .expect("d");
    assert_eq!(d.as_u64(), None);
    assert_eq!(d.as_f64(), Some(-2500.0));
    // Insertion order, not sorted: `a`, `b`, `c`, `e` would be the same sentence and a different file.
    let keys: Vec<&str> = object.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["a", "b", "c", "e"]);
}

/// A number keeps the digits it was written with. `18446744073709551615` is `u64::MAX` and an `f64`
/// rounds it; a device's storage figures are `u64`s.
#[test]
fn a_number_keeps_its_own_digits() {
    let big = "18446744073709551615";
    let value = parse(big).expect("parses");
    assert_eq!(value.number(), Some(big));
    assert_eq!(value.as_u64(), Some(u64::MAX));
    let rounded = parse("1e3").expect("parses");
    assert_eq!(rounded.number(), Some("1e3"));
    assert_eq!(
        rounded.as_u64(),
        None,
        "an exponent is not an integer literal"
    );
    assert_eq!(rounded.as_f64(), Some(1000.0));
    // And a written document is read back as the same value, not as a re-formatted one.
    assert_eq!(write(&Value::from(u64::MAX)), format!("{big}\n"));
}

#[test]
fn a_file_that_is_not_json_is_refused_with_a_position() {
    // The position is the character that disagreed, not the start of the file: the point of carrying
    // a line and a column at all.
    let err = parse("{\n  \"a\": 1\n  \"b\": 2\n}").expect_err("no comma");
    // The third line, at the quote that should have been preceded by a comma.
    assert_eq!((err.line, err.column), (3, 3));
    assert!(
        err.message.contains("expected `,` or `}`"),
        "{}",
        err.message
    );

    // Every refusal carries a position that lands on the character that disagreed — including the
    // ones `bump`-first code would report one past. `None` is "at the end of the file", which is the
    // only position that has no character under it.
    fn char_at(text: &str, line: usize, column: usize) -> Option<char> {
        text.split('\n').nth(line - 1)?.chars().nth(column - 1)
    }
    for (text, why, at) in [
        ("{\"a\": 01}", "leading zero", Some('1')),
        ("{\"a\": .5}", ".5 is not a number", Some('.')),
        ("{\"a\": 1.}", "1. is not a number", Some('}')),
        ("{\"a\": 1,}", "a trailing comma", Some('}')),
        ("[1, 2", "an array that never closes", None),
        ("{\"a\" 1}", "a missing colon", Some('1')),
        ("\"unterminated", "a string that never closes", None),
        ("{\"a\": 1} 2", "a second document", Some('2')),
        ("", "an empty file", None),
    ] {
        let err = parse(text).expect_err(why);
        assert_eq!(
            char_at(text, err.line, err.column),
            at,
            "{why}: {} (points at the wrong place)",
            err
        );
    }

    // `true`/`false`/`null` are the case the first version of this test missed: it compared after
    // stepping, so `truX` was reported at the column after the `X`.
    let err = parse("truX").expect_err("not true");
    assert_eq!((err.line, err.column), (1, 4), "{}", err);
}

/// Two keys with one name is a file whose author believed something untrue about it, so it is
/// refused rather than resolved to whichever came last.
#[test]
fn a_duplicate_key_is_an_error_rather_than_a_decision() {
    let err = parse(r#"{"width": 240, "width": 320}"#).expect_err("duplicate");
    assert!(
        err.message.contains("\"width\" is given twice"),
        "{}",
        err.message
    );
}

#[test]
fn strings_carry_escapes_and_surrogate_pairs() {
    let value = parse(r#""a\"b\\c\/d\be\ff\ng\rh\ti\u00e9 \ud83d\ude00""#).expect("parses");
    assert_eq!(
        value.as_str(),
        Some("a\"b\\c/d\u{8}e\u{c}f\ng\rh\ti\u{e9} \u{1f600}")
    );
    for bad in [
        r#""\ud83d""#,
        r#""\ud83dx""#,
        r#""\udc00""#,
        r#""\q""#,
        r#""a\""#,
    ] {
        assert!(parse(bad).is_err(), "{bad} is not a string");
    }
    // A raw control character is refused: it is legal in no JSON and usually means a broken editor.
    let raw = format!("\"a{}b\"", '\u{1}');
    assert!(parse(&raw).is_err());
    // …and everything a string can hold comes back out through the writer unchanged.
    let tricky = Value::String("tab\there \"quote\" back\\slash \u{7f} \u{1f600}".to_string());
    assert_eq!(parse(&write(&tricky)).expect("round trip"), tricky);
}

#[test]
fn the_writer_is_the_shape_a_person_edits() {
    let mut object = Map::new();
    object.insert("screen", Value::from(320u16));
    object.insert("empty_object", Value::from(Map::new()));
    object.insert("empty_array", Value::from(Vec::new()));
    object.insert(
        "keys",
        Value::from(vec![Value::from("AVK_SOFT1"), Value::from(7u16)]),
    );
    assert_eq!(
        write(&Value::Object(object)),
        "{\n  \"screen\": 320,\n  \"empty_object\": {},\n  \"empty_array\": [],\n  \"keys\": [\n    \
         \"AVK_SOFT1\",\n    7\n  ]\n}\n"
    );
}

/// A `[` per byte is a file that would take the stack with it; the reflex is a branch, and this is
/// the test that keeps the branch there.
#[test]
fn nesting_deeper_than_the_limit_is_refused_instead_of_recursing() {
    let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
    let err = parse(&deep).expect_err("too deep");
    assert!(err.message.contains("nested deeper"), "{}", err.message);
    // The limit counts containers, so it must not depend on whether the innermost one is empty:
    // both spellings of `MAX_DEPTH` deep have to parse.
    let empty = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(parse(&empty).is_ok(), "{MAX_DEPTH} nested empties");
    let full = format!("{}1{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(parse(&full).is_ok(), "{MAX_DEPTH} nested around a value");
    // …and `MAX_DEPTH + 1` is refused with the value inside too, not only when it is empty.
    let full = format!(
        "{}1{}",
        "[".repeat(MAX_DEPTH + 1),
        "]".repeat(MAX_DEPTH + 1)
    );
    assert!(
        parse(&full).is_err(),
        "{} nested around a value",
        MAX_DEPTH + 1
    );
}
