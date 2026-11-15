// Tests for `src/net.rs` — the worker thread behind `--net-proxy`.
//
// Every one of these stands up its own `TcpListener` and points the proxy at *that*, so what is
// being asserted is the bytes this host puts on the wire rather than anything about a real proxy or
// the internet. That is the same reason the rest of this tree builds synthetic modules: the subject
// is this code's own behaviour, and a test that needed the network would be answering a different
// question every time it ran.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

/// A listener on a port the OS picks, plus the `HOST:PORT` a proxy is told to use.
fn listening() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("an address").port();
    (listener, format!("127.0.0.1:{port}"))
}

/// Wait for the first reply, or give up. The worker owns the socket, so the test cannot know how
/// long the round trip takes — only that it is short on loopback.
///
/// The budget is in *tries*, and a try costs 5 ms of sleeping, so the wall-clock bound is
/// `tries × 5 ms` — and a wall-clock bound is a claim about this machine's load, not about the code
/// under test. That is how one of these tests came to fail once inside a full parallel suite and pass
/// alone: the reply had simply not been scheduled yet. So the refusal test gets the generous budget
/// (the one that can only be reached by a bug, since nothing here waits ten seconds for anything),
/// and every other test keeps the short one because a real round trip on loopback is under 50 ms
/// and a fast failure is worth more there than insurance.
fn first_reply_within(proxy: &Proxy, tries: u32) -> Vec<Reply> {
    for _ in 0..tries {
        let got = proxy.take_replies();
        if !got.is_empty() {
            return got;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Vec::new()
}

fn first_reply(proxy: &Proxy) -> Vec<Reply> {
    first_reply_within(proxy, 400)
}

/// Read one HTTP head — everything up to and including the blank line — off a socket.
fn read_head(sock: &mut std::net::TcpStream) -> String {
    let mut out = Vec::new();
    let mut byte = [0u8; 1];
    while out.len() < 8192 && !out.ends_with(b"\r\n\r\n") {
        match sock.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => out.push(byte[0]),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn an_address_without_a_scheme_is_read_as_http() {
    // `Proxy::new` does not connect, so this needs no listener: it is the address parsing, and the
    // point is that a developer typing `127.0.0.1:5678` gets a proxy rather than an error about a
    // missing scheme.
    let proxy = Proxy::new("127.0.0.1:1").expect("a bare host:port is an address");
    assert_eq!(proxy.at, "http://127.0.0.1:1");
    let explicit = Proxy::new("http://127.0.0.1:1").expect("a scheme is kept");
    assert_eq!(explicit.at, "http://127.0.0.1:1");
}

#[test]
fn a_request_opens_a_connect_tunnel_naming_the_origin_the_guest_asked_for() {
    let (listener, at) = listening();
    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().expect("the host connects");
        sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let connect = read_head(&mut sock);
        sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .expect("the tunnel opens");
        let inner = read_head(&mut sock);
        sock.write_all(b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n")
            .expect("an answer");
        (connect, inner)
    });

    let proxy = Proxy::new(&at).expect("a proxy");
    proxy
        .ask(1, "http://example.com/thing".to_string())
        .expect("the worker is running");

    let replies = first_reply(&proxy);
    let (connect, inner) = server.join().expect("the fake proxy finished");

    // `ureq`'s only HTTP proxy mode is a CONNECT tunnel, so this is what a proxy sees first. The port
    // is filled in from the scheme, and it has to be: naming the origin is the whole point of the
    // request, because the proxy is what resolves it.
    assert!(
        connect.starts_with("CONNECT example.com:80 HTTP/1.1\r\n"),
        "tunnel request was {connect:?}"
    );
    // Inside the tunnel the guest's own request is an ordinary origin-form one. That is what keeps an
    // `http://` request readable to a proxy that only relays bytes.
    assert!(
        inner.starts_with("GET /thing HTTP/1.1\r\n"),
        "the request inside the tunnel was {inner:?}"
    );
    assert_eq!(replies.len(), 1, "one request, one reply");
    let reply = &replies[0];
    assert_eq!(reply.id, 1, "the id the caller was given comes back");
    assert_eq!(reply.url, "http://example.com/thing");
    let answer = reply.result.as_ref().expect("a status, not an error");
    assert_eq!(answer.status, 204);
    assert!(answer.body.is_empty());
    assert!(!answer.truncated);
}

#[test]
fn a_non_2xx_status_is_an_answer_rather_than_an_error() {
    // `http_status_as_error(false)`: a pack can have a branch for a 404, and a host that reported it
    // as a transport failure would be hiding the one thing the server said.
    let (listener, at) = listening();
    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().expect("the host connects");
        sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let _ = read_head(&mut sock);
        sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .expect("the tunnel opens");
        let _ = read_head(&mut sock);
        sock.write_all(
            b"HTTP/1.1 404 Not Found\r\ncontent-type: text/plain\r\ncontent-length: 3\r\n\r\nno!",
        )
        .expect("an answer");
    });

    let proxy = Proxy::new(&at).expect("a proxy");
    proxy
        .ask(9, "http://example.com/missing".to_string())
        .expect("sent");
    let replies = first_reply(&proxy);
    let _ = server.join();

    let answer = replies[0]
        .result
        .as_ref()
        .expect("a 404 is an answer, not a failure");
    assert_eq!(answer.status, 404);
    assert_eq!(answer.body, b"no!");
    assert_eq!(answer.content_type.as_deref(), Some("text/plain"));
    assert_eq!(replies[0].id, 9);
}

#[test]
fn a_net_script_answers_routes_and_returns_a_404_for_the_rest() {
    let dir = std::env::temp_dir().join(format!(
        "quoin-net-script-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("body.json"), br#"{"ok":true}"#).expect("body");
    std::fs::write(
        dir.join("routes.tsv"),
        "GET\t/login\t200\tapplication/json\tbody.json\n\
         GET\t/api/*\t200\ttext/plain\t-\n",
    )
    .expect("routes");

    let proxy = Proxy::from_script(&dir.join("routes.tsv")).expect("script parses");
    proxy
        .ask(1, "http://offline.invalid/login?ignored=1".to_string())
        .expect("script accepts");
    let replies = proxy.take_replies();
    assert_eq!(replies.len(), 1);
    let answer = replies[0].result.as_ref().expect("an answer");
    assert_eq!(answer.status, 200);
    assert_eq!(answer.content_type.as_deref(), Some("application/json"));
    assert_eq!(answer.body, br#"{"ok":true}"#.to_vec());

    proxy
        .ask(2, "http://offline.invalid/api/list".to_string())
        .expect("prefix route");
    let replies = proxy.take_replies();
    assert_eq!(replies[0].result.as_ref().unwrap().status, 200);
    assert!(replies[0].result.as_ref().unwrap().body.is_empty());

    proxy
        .ask(3, "http://offline.invalid/missing".to_string())
        .expect("404 is still an answer");
    let replies = proxy.take_replies();
    assert_eq!(replies[0].result.as_ref().unwrap().status, 404);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_proxy_that_is_not_listening_is_a_failed_reply_rather_than_a_hang() {
    // A port no test here can take: every listener in this suite asks the OS for a random one
    // (`listening()` binds `127.0.0.1:0`, and so does `Control::bind("")`), and a second program
    // measured that this box answers `ECONNREFUSED` for port 1.
    //
    // Two ways this used to read as a failure that was not one, and they now say apart which
    // happened. **Nothing back within the budget**: the worker thread had not been scheduled, which
    // on a machine running the whole suite in parallel is a load number, not a verdict — red inside
    // `cargo test --lib --bins` and green
    // alone and green again in a second full run. **A reply that is not an error**: something *is*
    // listening on 1, which is a property of the box, not of this code. Both messages name the
    // reading, because a bare `left: 0, right: 1` invites a re-run until green — and the assertion
    // itself, that a failed connection becomes an error rather than silence, is not relaxed at all.
    let at = "127.0.0.1:1".to_string();

    let proxy = Proxy::new(&at).expect("a proxy");
    proxy
        .ask(3, "http://example.com/".to_string())
        .expect("sent");
    let replies = first_reply_within(&proxy, 2000);
    assert_eq!(
        replies.len(),
        1,
        "no reply within 10 s of polling: the worker was not scheduled (this box is loaded), \
         or the connect neither failed nor completed — the refusal path itself is what is asserted \
         below, and a missing reply is not evidence about it"
    );
    assert_eq!(replies[0].id, 3);
    assert!(
        replies[0].result.is_err(),
        "a refused connection is an error, got {:?}",
        replies[0].result.as_ref().map(|a| a.status)
    );
}

/// The `SOCKET` lines of the same file: a route list the socket layer can answer connects from, with
/// `*` for any address and a reply body read at parse time. A file with no such lines leaves the
/// socket layer's own model alone, which is what the flag did before this existed.
#[test]
fn a_net_script_can_also_declare_raw_socket_replies() {
    let dir = std::env::temp_dir().join(format!(
        "quoin-net-socket-script-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("reply.bin"), b"HTTP/1.0 200 OK\r\n\r\nhi").expect("reply");
    std::fs::write(
        dir.join("routes.tsv"),
        "SOCKET	10.1.2.3:80	reply.bin
         SOCKET	*:9000	-
         GET	/login	200	text/plain	-
",
    )
    .expect("routes");

    let proxy = Proxy::from_script(&dir.join("routes.tsv")).expect("script parses");
    let routes = proxy.sockets();
    assert_eq!(
        routes.reply_for(0x0302_010au32, 80),
        Some(b"HTTP/1.0 200 OK\r\n\r\nhi".to_vec()),
        "the named address, in the socket layer's byte order"
    );
    assert_eq!(
        routes.reply_for(0x0100_a8c0, 80),
        None,
        "a different address on the same port is not this line"
    );
    assert_eq!(
        routes.reply_for(0x0100_a8c0, 9000),
        Some(Vec::new()),
        "`*` matches any address"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
