// Moved out of `src/control.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::abi::generated as g;
use crate::runtime::Job;

/// One request over a real socket, and the answer that came back.
///
/// The server never runs on its own, so this is also the only way to test it: the poll that
/// would happen once per frame happens here until the answer arrives.
fn ask(ctl: &mut Control, rt: &mut Runtime, request: &str) -> (u16, Vec<u8>) {
    let mut client = TcpStream::connect(("127.0.0.1", ctl.port())).expect("connect");
    client
        .write_all(request.as_bytes())
        .expect("send the request");
    client
        .set_read_timeout(Some(Duration::from_millis(200)))
        .expect("a read timeout");
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    for _ in 0..200 {
        let _ = ctl.poll(rt);
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            // Nothing yet: the server may not have accepted the connection on the first poll.
            Err(_) if raw.is_empty() => continue,
            Err(_) => break,
        }
    }
    let split = find(&raw, b"\r\n\r\n").expect("a header block");
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, raw[split + 4..].to_vec())
}

fn text_of(body: &[u8]) -> String {
    String::from_utf8_lossy(body).into_owned()
}

#[test]
fn a_request_line_splits_into_a_path_and_a_query() {
    let req = parse_request(b"GET /log?since=12&x=a%2Bb+z HTTP/1.1\r\nHost: x\r\n").unwrap();
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/log");
    assert_eq!(req.get("since").as_deref(), Some("12"));
    // `%2B` is a literal plus and `+` is a space, which is the one place a query string is not
    // what it looks like.
    assert_eq!(req.get("x").as_deref(), Some("a+b z"));
    assert_eq!(req.get("nope"), None);
}

#[test]
fn a_request_is_get_or_post_and_nothing_else() {
    let err = parse_request(b"DELETE /quit HTTP/1.1\r\n\r\n").unwrap_err();
    assert!(err.contains("not GET or POST"), "{err}");
    let err = parse_request(b"GET\r\n\r\n").unwrap_err();
    assert!(err.contains("no path"), "{err}");
}

#[test]
fn a_code_is_a_published_name_or_a_number() {
    assert_eq!(resolve("AVK_DOWN"), abi::const_value("AVK_DOWN"));
    assert_eq!(resolve("0xe031"), Some(0xe031));
    assert_eq!(resolve("57425"), Some(57425));
    assert_eq!(resolve("not-a-key"), None);
}

#[test]
fn the_journal_hands_out_only_what_is_new() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    rt.display.quiet.push("first".to_string());
    rt.display.quiet.push("second".to_string());
    rt.debug.push("a module line".to_string());
    let _ = ctl.poll(&mut rt);

    let (lines, next, missing) = ctl.since(0);
    assert_eq!(missing, 0);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0].1, "note");
    assert_eq!(lines[0].3, "first");
    assert_eq!(lines[1].3, "second");
    assert_eq!(lines[2].1, "debug");
    assert_eq!(lines[2].3, "a module line");

    // Asking again from the cursor gives nothing: a line is handed out once.
    let (again, _, _) = ctl.since(next);
    assert!(again.is_empty(), "{again:?}");

    // And one new line is one new line, however many were seen before.
    rt.display.quiet.push("third".to_string());
    let _ = ctl.poll(&mut rt);
    let (more, _, _) = ctl.since(next);
    assert_eq!(more.len(), 1, "{more:?}");
    assert_eq!(more[0].3, "third");
}

#[test]
fn a_line_lost_to_a_source_own_cap_is_reported_as_lost() {
    // The display keeps 256 notes and drains half when it needs the room. A journal that
    // tracked an index into that list would hand out the wrong line forever after; a journal
    // that tracks a sequence says what it lost — but only for lines it never handed out, so the
    // trim has to happen before the first read for this to be the case under test.
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    for i in 0..10 {
        rt.display.quiet.push(format!("note {i}"));
    }
    rt.display.quiet.drain(..5);
    rt.display.notes_dropped = 5;

    let _ = ctl.poll(&mut rt);
    let (lines, next, _) = ctl.since(0);
    assert_eq!(lines[0].1, "lost", "{lines:?}");
    assert!(lines[0].3.contains('5'), "{}", lines[0].3);
    assert_eq!(
        lines[1].3, "note 5",
        "what is still held comes after the loss"
    );
    assert_eq!(lines.len(), 6, "{lines:?}");

    // A line that arrives after a read is simply the next line: nothing was lost from the
    // caller's point of view, because it had already been given everything before it.
    rt.display.quiet.push("eleventh".to_string());
    let _ = ctl.poll(&mut rt);
    let (more, _, _) = ctl.since(next);
    assert_eq!(more.len(), 1, "{more:?}");
    assert_eq!(more[0].3, "eleventh");
}

#[test]
fn status_answers_over_http() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let want = format!("\"device\":\"{}x{}\"", rt.device.width, rt.device.height);
    let (status, body) = ask(&mut ctl, &mut rt, "GET /status HTTP/1.1\r\nHost: x\r\n\r\n");
    assert_eq!(status, 200);
    let body = text_of(&body);
    assert!(body.contains(&want), "{body}");
    assert!(body.contains("\"presentations\":0"), "{body}");
    assert!(body.contains("\"fault\":null"), "{body}");
    assert!(body.ends_with("}\n"), "{body}");
}

#[test]
fn the_screen_comes_back_as_a_png_and_as_a_bmp() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let (status, png) = ask(&mut ctl, &mut rt, "GET /screen.png HTTP/1.1\r\n\r\n");
    assert_eq!(status, 200);
    assert_eq!(&png[..4], b"\x89PNG", "a PNG signature");
    let (status, bmp) = ask(&mut ctl, &mut rt, "GET /screen.bmp HTTP/1.1\r\n\r\n");
    assert_eq!(status, 200);
    assert_eq!(&bmp[..2], b"BM", "a BMP signature");
}

#[test]
fn a_key_verb_reaches_the_queue_the_guest_is_served_from() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    assert!(rt.queue.is_empty());
    let (status, body) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/key?code=AVK_DOWN HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 200);
    assert!(text_of(&body).contains("AVK_DOWN"), "{}", text_of(&body));
    // Posted, not delivered: the guest sees it on its next turn, exactly as a real key arrives.
    assert!(!rt.queue.is_empty(), "the key was queued");
}

#[test]
fn a_bad_code_names_what_was_wrong_with_it() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let (status, body) = ask(&mut ctl, &mut rt, "POST /input/key?code=AVK_NOPE\r\n\r\n");
    assert_eq!(status, 400);
    assert!(text_of(&body).contains("AVK_NOPE"), "{}", text_of(&body));
    let (status, _) = ask(&mut ctl, &mut rt, "POST /input/key HTTP/1.1\r\n\r\n");
    assert_eq!(status, 400, "a missing code is a bad request");
}

/// What `/advance` returns is a *request for the guest loop*, not a completed action.
///
/// Kept apart from [`an_advance_answers_after_the_guest_has_moved`] on purpose: that one follows the
/// answer through to the socket, this one says what the owning loop is told to do — advance, and not
/// quit. It used to sit and `read()` the answer too, with a 200 ms timeout in a 200-iteration loop and
/// no `pump` anywhere: the answer is withheld until the clock passes the target, so the loop ran its
/// full 200 × 200 ms (~39 s measured — most of the suite's wall clock when it ran serially) and the
/// assertions never saw a byte. Waiting on something this function does not drive is not
/// a test of it, so the socket read went away and the poll is bounded by iterations, not by a clock.
#[test]
fn advance_comes_back_as_a_request_for_the_guest_loop() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let mut client = TcpStream::connect(("127.0.0.1", ctl.port())).unwrap();
    client
        .write_all(b"POST /advance?ms=1500 HTTP/1.1\r\n\r\n")
        .unwrap();
    // The request may not have reached the listener on the first accept, same as the test above.
    let mut asked = ctl.poll(&mut rt);
    for _ in 0..200 {
        if asked.advance_ms != 0 {
            break;
        }
        asked = ctl.poll(&mut rt);
    }
    assert_eq!(asked.advance_ms, 1500);
    assert!(!asked.quit, "asking for time is not asking to end the run");
    drop(client);
}

#[test]
fn quit_asks_the_loop_to_end_the_run() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let mut client = TcpStream::connect(("127.0.0.1", ctl.port())).unwrap();
    client.write_all(b"POST /quit\r\n\r\n").unwrap();
    client
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let mut asked = Asked::default();
    let mut buf = [0u8; 1024];
    for _ in 0..200 {
        asked = ctl.poll(&mut rt);
        if client.read(&mut buf).is_ok() {
            break;
        }
    }
    assert!(asked.quit);
    assert_eq!(asked.advance_ms, 0);
}

#[test]
fn an_advance_answers_after_the_guest_has_moved() {
    // The answer is the thing the first version of this interface could not give: `/advance`
    // used to answer the moment it was *asked*, so finding out whether the run had done anything
    // took a second request — and while the run was moving, that second request was not read at
    // all. The connection is held instead, and what comes back says what the time bought.
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let mut client = TcpStream::connect(("127.0.0.1", ctl.port())).unwrap();
    client
        .write_all(b"POST /advance?ms=250 HTTP/1.1\r\n\r\n")
        .unwrap();
    client
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut buf = [0u8; 2048];

    // Poll until the request has actually landed. `write_all` on a loopback socket is not
    // synchronous with the listener's `accept`, so the first `poll` can run while no byte is
    // readable yet — which answers `advance_ms == 0` and fails the assertion below. Under a full
    // `cargo test --lib` (376 tests on one machine) it did exactly that, twice, and read as a
    // regression in `/advance` rather than as what it is. Bounded by polls, not by a wall clock:
    // the guest is not running here, `poll` is a non-blocking accept, and a wait that can expire
    // would only move the flake into a hang.
    let mut asked = ctl.poll(&mut rt);
    for _ in 0..200 {
        if asked.advance_ms != 0 {
            break;
        }
        asked = ctl.poll(&mut rt);
    }
    assert_eq!(asked.advance_ms, 250);
    assert!(
        client.read(&mut buf).is_err(),
        "the answer is not due before the guest has moved"
    );

    // The loop that owns the clock takes the time, and the next poll brings the answer. The answer is
    // **accumulated**, not read once: the header and the body leave the server in separate segments
    // (`Connection: close`, so the end of the answer is a zero-length read), and a single `read` can hand
    // back the header alone — which under a loaded machine made this test report a regression in
    // `/advance` that was never there.
    rt.pump(250).unwrap();
    let mut raw = Vec::new();
    for _ in 0..40 {
        let _ = ctl.poll(&mut rt);
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            // Nothing yet: the answer is written during the poll, so an empty read is still "not due".
            Err(_) if raw.is_empty() => continue,
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    assert!(text.starts_with("HTTP/1.1 200"), "{text}");
    assert!(text.contains("advance 250 ms"), "{text}");
    assert!(text.contains("clock 0 -> 250"), "{text}");
    assert!(text.contains("instructions"), "{text}");
}

#[test]
fn an_advance_bigger_than_one_turn_is_refused_with_the_limit_in_the_answer() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let (status, body) = ask(&mut ctl, &mut rt, "POST /advance?ms=600000\r\n\r\n");
    assert_eq!(status, 400);
    assert!(text_of(&body).contains("10000"), "{}", text_of(&body));
}

#[test]
fn the_call_sequence_keeps_what_decides_and_drops_what_draws() {
    // `call_log` is for "what did the pack ask at that moment", and a list that is mostly
    // `IDisplay::BitBlt` cannot answer it: a pack draws thousands of times a frame.
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let draw = abi::method_of("IDisplay", "BitBlt").unwrap();
    let heartbeat = abi::method_of("IShell", "SetTimer").unwrap();
    let shell = abi::method_of("IShell", "LoadResData").unwrap();
    let _ = rt.dispatch(draw, 0);
    let _ = rt.dispatch(heartbeat, 0);
    let _ = rt.dispatch(shell, 0);
    let _ = ctl.poll(&mut rt);

    let (lines, _, _) = ctl.since(0);
    let calls: Vec<_> = lines.iter().filter(|line| line.1 == "call").collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].3.contains("IShell::LoadResData"), "{:?}", calls[0]);
    assert!(
        !calls[0].3.contains("BitBlt"),
        "the drawing interface is left out"
    );
    assert!(
        !calls[0].3.contains("SetTimer"),
        "and so is the pack's own heartbeat"
    );
}

#[test]
fn an_unknown_verb_lists_the_ones_that_exist() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let (status, body) = ask(&mut ctl, &mut rt, "GET /nope HTTP/1.1\r\n\r\n");
    assert_eq!(status, 404);
    assert!(text_of(&body).contains("/status"), "{}", text_of(&body));
    let (status, root) = ask(&mut ctl, &mut rt, "GET / HTTP/1.1\r\n\r\n");
    assert_eq!(status, 200);
    assert!(text_of(&root).contains("/input/key"), "{}", text_of(&root));
}

#[test]
fn a_pointer_tap_is_two_contacts_and_a_hold_is_two_requests() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let (status, body) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/pointer?kind=tap&x=10&y=20 HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 200);
    assert_eq!(
        text_of(&body).matches("EVT_POINTER_").count(),
        2,
        "{}",
        text_of(&body)
    );
    // A negative coordinate is what a pack dragging a scene off the edge asks for.
    let (status, body) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/pointer?kind=down&x=-5&y=0x10 HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 200);
    assert!(text_of(&body).contains("-5 16"), "{}", text_of(&body));
    let (status, _) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/pointer?kind=wiggle&x=1&y=1 HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 400, "a kind nobody has is a bad request");
}

#[test]
fn json_escapes_what_would_break_a_document() {
    assert_eq!(json("plain"), "\"plain\"");
    assert_eq!(json("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(json("line\nbreak"), "\"line\\nbreak\"");
    assert_eq!(json("\u{1}"), "\"\\u0001\"");
    assert_eq!(opt_json(None), "null");
    assert_eq!(opt_json(Some("x")), "\"x\"");
}

#[test]
fn a_client_that_says_nothing_does_not_hold_the_guest_up() {
    // A connection that never sends a request sits in `pending` and is dropped after CONN_IDLE;
    // what must never happen is the emulator waiting on it.
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    let idle = TcpStream::connect(("127.0.0.1", ctl.port())).unwrap();
    let asked = ctl.poll(&mut rt);
    assert_eq!(asked, Asked::default());
    drop(idle);
    // And the loop still answers the next caller with the previous one gone.
    let (status, _) = ask(&mut ctl, &mut rt, "GET /status HTTP/1.1\r\n\r\n");
    assert_eq!(status, 200);
}

/// The hold verbs refuse the two shapes no handset can produce, and say why.
///
/// A second `keydown` of a held key would queue a second press pair, which a pack reading a mask of the
/// keys *down* resolves as a key that outlives its one release; a `keyup` of a key nothing is holding
/// invents an up-edge outright. Both used to be answered 200, which is the shape this repository has
/// been burned by most: accepted, then silent, then wrong three turns later.
#[test]
fn the_hold_verbs_refuse_a_press_without_an_up_and_an_up_without_a_press() {
    let mut rt = Runtime::new();
    let mut ctl = Control::bind("").unwrap();
    // First hold: accepted, and the device is now holding it.
    let (status, _) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/keydown?code=AVK_DOWN HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 200, "the first hold is a real key down");
    assert_eq!(rt.held.len(), 1, "and the device is holding one key");
    // The duplicate is the thing being refused, and it must not have queued a second press.
    let (status, body) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/keydown?code=AVK_DOWN HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 409, "a key already down is not pressed twice");
    assert!(
        text_of(&body).contains("already holding"),
        "and the answer names the state it refused: {}",
        text_of(&body)
    );
    assert_eq!(rt.held.len(), 1, "refusing changed nothing");
    let (status, _) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/keyup?code=AVK_DOWN HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 200, "the real release of a held key is accepted");
    assert!(rt.held.is_empty(), "and the device is holding nothing now");
    // The release of a key that is already up. This is the second half of the same claim: an up-edge
    // needs a down-edge under it, whether or not the first release's event has reached the guest yet.
    let (status, _) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/keyup?code=AVK_DOWN HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 409, "one hold is one release");
    // And the up-edge of a key nobody is holding.
    let (status, body) = ask(
        &mut ctl,
        &mut rt,
        "POST /input/keyup?code=AVK_UP HTTP/1.1\r\n\r\n",
    );
    assert_eq!(status, 409, "there is nothing to release");
    assert!(
        text_of(&body).contains("no such key is down"),
        "{}",
        text_of(&body)
    );
    assert!(
        !rt
            .queue
            .iter()
            .any(|j| matches!(j, Job::Event { evt, wp, .. } if *evt == g::EVT_KEY_RELEASE && *wp == g::AVK_UP)),
        "and the refused release never reached the queue"
    );
}
