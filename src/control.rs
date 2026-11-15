//! Driving a running guest from outside the process.
//!
//! A run is otherwise something you start and then read the report of. This is the other half: a
//! listener a script, a harness or an agent can talk to *while* the guest is running — post a key,
//! ask for the screen, ask what the pack has said since last time.
//!
//! Of the shapes this could have had, this is the one that does not care how the caller was
//! started: a local HTTP server, so anything that can open a socket can drive a run that is already
//! going. stdin commands would tie the caller to the console's parentage, and a named pipe is one
//! platform's answer. Nothing here is a web framework: the grammar a control channel needs is a
//! request line with a path and a query, and a response is a status, two headers and a body.
//!
//! Two rules keep it honest:
//!
//! 1. **Time stays the run's to decide.** Under `--window` the guest advances on the wall clock, as
//!    it always did. Headless it does not advance at all unless the caller asks with
//!    `/advance?ms=…`, so a session driven from outside is still reproducible to the byte — which
//!    is the property headless mode exists for.
//! 2. **Nothing here runs on its own thread.** [`Control::poll`] is called by whichever loop is
//!    already running the guest (a window's frame, or the headless service loop), so a request is
//!    served between two turns of the guest and no other thread ever touches the runtime.
//!
//! `GET /` prints the verbs, and they are here too, because a caller with only this file should not
//! have to open a socket to find them:
//!
//! ```text
//! GET  /status                        what the run is doing now, as JSON
//! GET  /screen.png                    the device framebuffer (?format=bmp, or /screen.bmp)
//! GET  /log?since=N                   journal lines newer than sequence N, as NDJSON
//! POST /input/key?code=AVK_DOWN       press and release one key
//! POST /input/keydown?code=AVK_DOWN   press it and leave it down, for a caller that wants to hold
//! POST /input/keyup?code=AVK_DOWN     release that key
//! POST /input/event?evt=EVT_KEY&wp=…&dwp=…
//! POST /input/pointer?kind=down&x=10&y=20    kind is down, move, up or tap
//! POST /input/start?cls=auto          create and start the applet the module declares
//! POST /advance?ms=1000               let that much virtual time pass
//! POST /quit                          end the run, the way closing the window does
//! ```
//!
//! A code or an event is a published SDK name (`AVK_DOWN`) or a number (`0xe031`), the same two
//! forms the command line takes — because a report says names, and this interface is meant to be
//! driven by whoever is reading the report.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::abi::{self, generated as g};
use crate::runtime::Runtime;

/// Connections that may be part-way through a request at once. A caller that opens more gets a 503
/// rather than a slot: this list is walked once per turn of the guest, so a leak here is paid for
/// by the emulator.
const PENDING: usize = 32;

/// Largest request accepted, headers and body together. A control channel carries a key code and a
/// path; anything larger is a client that has lost its way.
const REQUEST_MAX: usize = 16 * 1024;

/// How long a half-read request may sit before its connection is dropped. Without it, a client that
/// connects and then says nothing holds a slot for the length of the run.
const CONN_IDLE: Duration = Duration::from_secs(5);

/// Journal lines kept. The journal exists to be read once and soon, so it holds the recent past and
/// says how much it dropped rather than growing with a run that plays for an hour.
const JOURNAL: usize = 8192;

/// What one poll asked the guest loop to do.
///
/// The loop that runs the guest owns its clock, so "let time pass" and "stop" come back from here
/// rather than being done inside [`Control::poll`]: a poll that pumped the guest itself would make
/// how far a run gets depend on how often a client happened to connect.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Asked {
    /// Virtual milliseconds to let pass; zero is "don't". Several requests in one turn add up.
    pub advance_ms: u32,
    /// Whether the run was asked to end.
    pub quit: bool,
}

/// One journal line, as [`Control::since`] hands it out.
pub type Line = (u64, &'static str, u64, String);

pub struct Control {
    listener: TcpListener,
    /// The port actually bound — what a caller needs when the request was "any free one".
    port: u16,
    /// Connections whose request has not arrived in full yet.
    pending: Vec<Conn>,
    journal: Journal,
}

/// The largest `/advance` a caller may ask for in one request.
///
/// A poll serves requests between two turns of the guest, so a request for a minute of virtual time
/// holds the whole interface for as long as that takes to run: the next request — including the one
/// polling for this answer — is not even read until it is done. A caller that wants more asks again;
/// that is what a loop is for. Ten seconds is roughly a second of wall clock on a light pack.
const ADVANCE_MAX: u32 = 10_000;

/// A connection whose answer waits for the guest.
///
/// `/advance` cannot be answered when it is asked, because nothing has moved yet — and an answer
/// written then says only that time was *requested*, which is what made the first version of this
/// interface need a second request to find out whether the run had done anything. So the connection
/// is held until the run reaches the moment that was asked for, and the answer carries what the time
/// bought.
struct Advance {
    /// The `clock_ms` the run has to reach before this answer is due.
    until: u32,
    from_clock: u32,
    from_steps: u64,
}

struct Conn {
    stream: TcpStream,
    buf: Vec<u8>,
    opened: Instant,
    /// Set while this connection is a `/advance` waiting for its moment.
    advance: Option<Advance>,
}

impl Control {
    /// Listen on `addr`: a port (`8765`), a host and port (`127.0.0.1:8765`), or empty for any free
    /// port on the loopback interface.
    ///
    /// Loopback by default and by intention — this interface has no authentication, and
    /// `--serve 0.0.0.0:8765` is a deliberate thing to type and not a default this should pick.
    pub fn bind(addr: &str) -> Result<Control, String> {
        let addr = addr.trim();
        let full = if addr.is_empty() {
            "127.0.0.1:0".to_string()
        } else if addr.chars().all(|c| c.is_ascii_digit()) {
            format!("127.0.0.1:{addr}")
        } else {
            addr.to_string()
        };
        let listener =
            TcpListener::bind(&full).map_err(|e| format!("cannot listen on {full}: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("cannot make the control socket non-blocking: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("cannot read back the control address: {e}"))?
            .port();
        Ok(Control {
            listener,
            port,
            pending: Vec::new(),
            journal: Journal::default(),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The address actually bound, which is what to print: a caller needs the port when the request
    /// was "any free one", and the host when it was not the loopback interface.
    pub fn local(&self) -> Option<std::net::SocketAddr> {
        self.listener.local_addr().ok()
    }

    /// Serve whatever has arrived, and say what the guest loop should do about it.
    ///
    /// Called once per frame by the window and once per turn by the headless loop. Nothing here
    /// blocks on a client: a request that has not fully arrived stays in `pending` for the next
    /// call, which is what keeps a slow caller from stopping the emulator.
    pub fn poll(&mut self, rt: &mut Runtime) -> Asked {
        self.journal.observe(rt);
        let mut asked = Asked::default();
        // Answers that were waiting for the guest come first, before anything new is read: a caller
        // that asked for time is waiting on this one.
        self.flush_advances(rt);
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if self.pending.len() >= PENDING || stream.set_nonblocking(true).is_err() {
                        // Say so rather than dropping it silently: a caller that is being refused
                        // has to be able to tell that apart from a run that stopped answering.
                        let _ = refuse(stream, "too many requests in flight");
                        continue;
                    }
                    self.pending.push(Conn {
                        stream,
                        buf: Vec::new(),
                        opened: Instant::now(),
                        advance: None,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // A listener that can no longer accept is not something a run can act on; whatever
                // is already in flight is still served below.
                Err(_) => break,
            }
        }
        // Taken out so an answer can borrow the journal; the connections that are still waiting go
        // back at the end.
        let mut pending = std::mem::take(&mut self.pending);
        let mut keep = Vec::with_capacity(pending.len());
        for mut conn in pending.drain(..) {
            let (step, need) = serve_conn(rt, &mut self.journal, &mut conn);
            asked.advance_ms = asked.advance_ms.saturating_add(need.advance_ms);
            asked.quit |= need.quit;
            match step {
                Step::Wait if conn.opened.elapsed() < CONN_IDLE => keep.push(conn),
                Step::Wait => {}
                // A `/advance` waits for the run rather than for the caller, so it is not on the
                // caller's clock: it goes back with its moment recorded and is answered below.
                Step::Held(advance) => {
                    conn.advance = Some(advance);
                    keep.push(conn);
                }
                Step::Done | Step::Broken => {}
            }
        }
        self.pending = keep;
        asked
    }

    /// Answer the connections that asked for time and have now been given it.
    ///
    /// The answer says what the time bought — the clock it reached and the instructions the guest
    /// retired getting there — because "advance 5000" alone would leave a caller to ask a second
    /// question to find out whether anything happened.
    fn flush_advances(&mut self, rt: &Runtime) {
        let mut pending = std::mem::take(&mut self.pending);
        let mut keep = Vec::with_capacity(pending.len());
        for mut conn in pending.drain(..) {
            let Some(advance) = conn.advance.take() else {
                keep.push(conn);
                continue;
            };
            if rt.clock_ms < advance.until {
                conn.advance = Some(advance);
                keep.push(conn);
                continue;
            }
            let body = format!(
                "advance {} ms: clock {} -> {}, guest {} -> {} ({} instructions)\n",
                advance.until - advance.from_clock,
                advance.from_clock,
                rt.clock_ms,
                advance.from_steps,
                rt.steps(),
                rt.steps() - advance.from_steps,
            );
            let _ = send(&mut conn, &Answer::text(200, "OK", body));
        }
        self.pending = keep;
    }

    /// Journal lines with a sequence number of `since` or higher, the sequence to ask with next,
    /// and how many earlier lines are gone.
    pub fn since(&self, since: u64) -> (Vec<Line>, u64, u64) {
        self.journal.since(since)
    }
}

enum Step {
    /// The request was answered (or refused) and the connection is spent.
    Done,
    /// Not all of it has arrived yet.
    Wait,
    /// The peer went away.
    Broken,
    /// The answer has to wait for the guest; the connection goes back with its moment recorded.
    Held(Advance),
}

fn serve_conn(rt: &mut Runtime, journal: &mut Journal, conn: &mut Conn) -> (Step, Asked) {
    let mut chunk = [0u8; 4096];
    loop {
        match conn.stream.read(&mut chunk) {
            Ok(0) => return (Step::Broken, Asked::default()),
            Ok(n) => {
                conn.buf.extend_from_slice(&chunk[..n]);
                if conn.buf.len() > REQUEST_MAX {
                    let answer = Answer::text(413, "Payload Too Large", "request too large\n");
                    let _ = send(conn, &answer);
                    return (Step::Done, Asked::default());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => return (Step::Broken, Asked::default()),
        }
    }
    let Some(end) = find(&conn.buf, b"\r\n\r\n") else {
        return (Step::Wait, Asked::default());
    };
    let want = content_length(&conn.buf[..end]);
    if conn.buf.len() < end + 4 + want {
        return (Step::Wait, Asked::default());
    }
    let answer = match parse_request(&conn.buf[..end]) {
        Ok(req) => route(rt, journal, &req),
        Err(why) => Answer::text(400, "Bad Request", format!("{why}\n")),
    };
    let asked = answer.asked;
    if asked.advance_ms > 0 {
        // Nothing has moved yet, and the answer is what the run does with the time — so it is held
        // until the guest gets there rather than written now.
        return (
            Step::Held(Advance {
                until: rt.clock_ms.saturating_add(asked.advance_ms),
                from_clock: rt.clock_ms,
                from_steps: rt.steps(),
            }),
            asked,
        );
    }
    let _ = send(conn, &answer);
    (Step::Done, asked)
}

/// A whole response, and then the connection closed. One request per connection is the shape a
/// control channel wants: there is no state between requests to lose, and a client reusing a
/// connection would have to be trusted to frame its own body.
fn send(conn: &mut Conn, answer: &Answer) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\
         Cache-Control: no-store\r\n\r\n",
        answer.status,
        answer.reason,
        answer.content_type,
        answer.body.len()
    );
    conn.stream.write_all(head.as_bytes())?;
    conn.stream.write_all(&answer.body)?;
    conn.stream.flush()
}

fn refuse(mut stream: TcpStream, why: &str) -> std::io::Result<()> {
    let body = format!("{why}\n");
    let head = format!(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

/// One completed exchange: what to send back, and what the guest loop was asked to do.
struct Answer {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    asked: Asked,
}

impl Answer {
    fn text(status: u16, reason: &'static str, body: impl Into<String>) -> Answer {
        Answer {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            body: body.into().into_bytes(),
            asked: Asked::default(),
        }
    }

    fn json(status: u16, body: impl Into<String>) -> Answer {
        Answer {
            status,
            reason: "OK",
            content_type: "application/json",
            body: body.into().into_bytes(),
            asked: Asked::default(),
        }
    }
}

/// The verbs, printed by `GET /` and by anything the router does not know.
const VERBS: &str = "\
QuoinHLE control interface

  GET  /status                      what the run is doing now, as JSON
  GET  /screen.png                  the device framebuffer (?format=bmp, or /screen.bmp)
  GET  /log?since=N                 journal lines newer than N, as NDJSON
  POST /input/key?code=AVK_DOWN     press and release one key
  POST /input/keydown?code=AVK_DOWN press a key and leave it down
  POST /input/keyup?code=AVK_DOWN   release it
  POST /input/event?evt=EVT_KEY&wp=…&dwp=…
  POST /input/pointer?kind=down&x=10&y=20     kind is down, move, up or tap
  POST /input/start?cls=auto        create and start the module's own applet
  POST /advance?ms=1000             let that much virtual time pass. The answer arrives once the
                                    run has got there, and says what the time bought (the clock and
                                    the guest instruction count it reached). At most 10000 ms in one
                                    request, because one request is one turn of the guest loop and
                                    everything else waits behind it — ask again for more
  POST /quit                        end the run

  A code or event is a published name (AVK_DOWN, EVT_KEY) or a number (0xe031, 0x100).
  Under --window the guest follows the wall clock; without one it moves only on /advance.
  A pack that polls which keys are down cannot see a press and a release that land in
  one turn: to hold a key, use /input/keydown, then /advance, then /input/keyup.
  The hold verbs keep the device's edge: /input/keydown of a key already down, and /input/keyup of
  one nothing is holding, are refused with 409 and a sentence, because the duplicate would leave a
  pack believing a key is held after its release, or invent an up-edge no finger made.
  GET /log kinds: note, text, screen, debug, sound, music, gap, call, net.
";

/// One finished HTTP request.
#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
}

impl Request {
    fn get(&self, key: &str) -> Option<String> {
        self.query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// A field that has to be there, and has to be a number or a published SDK name.
    fn code(&self, key: &str) -> Result<u32, String> {
        let raw = self.get(key).ok_or_else(|| format!("{key} is required"))?;
        resolve(&raw)
            .ok_or_else(|| format!("{key}={raw:?} is neither a number nor a published SDK name"))
    }

    fn number<T: std::str::FromStr>(&self, key: &str, default: T) -> Result<T, String> {
        match self.get(key) {
            None => Ok(default),
            Some(raw) => raw
                .parse::<T>()
                .map_err(|_| format!("{key}={raw:?} is not a number")),
        }
    }

    /// A screen coordinate: signed, and hex if the caller writes it that way, because the handset
    /// reports them signed and a pack that drags a scene can be told to start one off the edge.
    fn coord(&self, key: &str) -> Result<i32, String> {
        let raw = self.get(key).ok_or_else(|| format!("{key} is required"))?;
        let (neg, body) = match raw.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, raw.as_str()),
        };
        let n = match body.strip_prefix("0x").or(body.strip_prefix("0X")) {
            Some(hex) => i64::from_str_radix(hex, 16),
            None => body.parse::<i64>(),
        }
        .map_err(|_| format!("{key}={raw:?} is not a coordinate"))?;
        i32::try_from(if neg { -n } else { n })
            .map_err(|_| format!("{key}={raw:?} is out of range"))
    }
}

fn route(rt: &mut Runtime, journal: &mut Journal, req: &Request) -> Answer {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/" | "") => Answer::text(200, "OK", VERBS),
        ("GET", "/status") => Answer::json(200, status_json(rt)),
        ("GET", "/screen.png") => screen(rt, false),
        ("GET", "/screen.bmp") => screen(rt, true),
        ("GET", "/log") => match req.number::<u64>("since", 0) {
            Ok(since) => {
                // Fresh as of this request rather than as of the turn's start: a caller asking for
                // the log has just done something, and what it did is what it wants to read.
                journal.observe(rt);
                Answer {
                    status: 200,
                    reason: "OK",
                    content_type: "application/x-ndjson",
                    body: journal.body(since),
                    asked: Asked::default(),
                }
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/key") => match req.code("code") {
            Ok(code) => {
                rt.post_key(code);
                Answer::text(200, "OK", format!("key {}\n", named(code, "AVK_")))
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/keydown") => match req.code("code") {
            Ok(code) => {
                // A key that is already down is not pressed a second time. The device sends its own
                // repeats from there (`AEEVCodes.h`: the first one `KB_AUTOREPEAT_START` ms after the
                // initial `EVT_KEY`, then every `KB_AUTOREPEAT_RATE`), and a second press pair would
                // tell the pack the key went down twice — which a pack keeping a mask of the keys down
                // reads as a key that stays down after its one release. Refusing is the honest answer,
                // and a silent 200 is the worst shape this repository knows.
                if rt.held.iter().any(|k| k.code == code) {
                    Answer::text(
                        409,
                        "Conflict",
                        format!(
                            "keydown {}: the device is already holding it down; repeats come from the \
                             host, so POST /advance to let them come due, or /input/keyup first\n",
                            named(code, "AVK_")
                        ),
                    )
                } else {
                    rt.post_key_press(code);
                    Answer::text(200, "OK", format!("keydown {}\n", named(code, "AVK_")))
                }
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/keyup") => match req.code("code") {
            Ok(code) => {
                // The same reasoning upside down: releasing a key nobody is holding invents an up-edge.
                // `held` is the whole test, because a release leaves it immediately even when its event
                // is the deferred kind — so a second `keyup` of one hold is refused here rather than
                // queueing a second release into a later turn.
                if !rt.held.iter().any(|k| k.code == code) {
                    Answer::text(
                        409,
                        "Conflict",
                        format!(
                            "keyup {}: no such key is down, so there is nothing to release; \
                             POST /input/keydown {} first\n",
                            named(code, "AVK_"),
                            named(code, "AVK_")
                        ),
                    )
                } else {
                    rt.post_key_release(code);
                    Answer::text(200, "OK", format!("keyup {}\n", named(code, "AVK_")))
                }
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/event") => match event_args(req) {
            Ok((evt, wp, dwp)) => {
                rt.post_event(evt, wp, dwp);
                Answer::text(
                    200,
                    "OK",
                    format!("event {} {wp:#010x} {dwp:#010x}\n", named(evt, "EVT_")),
                )
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/pointer") => match pointer_args(req) {
            Ok(contacts) => {
                for (evt, x, y) in &contacts {
                    // A pointer event carries its coordinates in a guest string, so this can fail on
                    // memory the way any guest write can — say which rather than answering OK.
                    if let Err(e) = rt.post_pointer(*evt, *x, *y) {
                        return Answer::text(400, "Bad Request", format!("{e}\n"));
                    }
                }
                let what: Vec<String> = contacts
                    .iter()
                    .map(|(evt, x, y)| format!("{} {x} {y}", named(*evt, "EVT_POINTER_")))
                    .collect();
                Answer::text(200, "OK", format!("{}\n", what.join(", ")))
            }
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/input/start") => match req.get("cls").as_deref() {
            None | Some("auto") => match rt.start_auto() {
                Ok(found) => Answer::json(
                    200,
                    format!(
                        "{{\"cls\":\"{:#010x}\",\"applet\":\"{:#010x}\",\"tried\":{},\"note\":{}}}\n",
                        found.cls,
                        found.applet,
                        found.tried,
                        opt_json(found.note.as_deref())
                    ),
                ),
                Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
            },
            Some(raw) => match resolve(raw) {
                None => Answer::text(
                    400,
                    "Bad Request",
                    format!("cls={raw:?} is neither a number nor a published SDK name\n"),
                ),
                Some(cls) => match rt.start_app(cls, None) {
                    Ok(rv) => Answer::json(
                        200,
                        format!(
                            "{{\"cls\":\"{cls:#010x}\",\"result\":\"{rv:#010x}\",\"applet\":\"{:#010x}\"}}\n",
                            rt.applet
                        ),
                    ),
                    Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
                },
            },
        },
        ("POST", "/advance") => match req.number::<u32>("ms", 0) {
            // The cap is not a kindness to the caller: one request of this is one turn of the guest
            // loop, and everything else waits behind it. Asking twice is the caller's business.
            Ok(ms) if ms > ADVANCE_MAX => Answer::text(
                400,
                "Bad Request",
                format!(
                    "{ms} ms is more than one turn carries (limit {ADVANCE_MAX}); ask again for the \
                     rest\n"
                ),
            ),
            Ok(ms) => Answer {
                status: 200,
                reason: "OK",
                content_type: "text/plain; charset=utf-8",
                body: format!("advance {ms}\n").into_bytes(),
                asked: Asked {
                    advance_ms: ms,
                    quit: false,
                },
            },
            Err(e) => Answer::text(400, "Bad Request", format!("{e}\n")),
        },
        ("POST", "/quit") => Answer {
            status: 200,
            reason: "OK",
            content_type: "text/plain; charset=utf-8",
            body: b"quitting\n".to_vec(),
            asked: Asked {
                advance_ms: 0,
                quit: true,
            },
        },
        _ if req.path.starts_with("/input/") => Answer::text(
            404,
            "Not Found",
            format!("no such verb: {}\n\n{VERBS}", req.path),
        ),
        _ => Answer::text(
            404,
            "Not Found",
            format!("no such path: {}\n\n{VERBS}", req.path),
        ),
    }
}

fn screen(rt: &Runtime, bmp: bool) -> Answer {
    if bmp {
        return match rt.screenshot_bmp() {
            Ok(bytes) => Answer {
                status: 200,
                reason: "OK",
                content_type: "image/bmp",
                body: bytes,
                asked: Asked::default(),
            },
            Err(e) => Answer::text(500, "Internal Server Error", format!("{e}\n")),
        };
    }
    let rgb = match rt.screenshot_rgb() {
        Ok(rgb) => rgb,
        // The pixels live in guest memory, so a screenshot can fail the way any guest read can.
        Err(e) => return Answer::text(500, "Internal Server Error", format!("{e}\n")),
    };
    let (w, h) = (u32::from(rt.device.width), u32::from(rt.device.height));
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    match encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&rgb))
    {
        Ok(()) => Answer {
            status: 200,
            reason: "OK",
            content_type: "image/png",
            body: out,
            asked: Asked::default(),
        },
        Err(e) => Answer::text(500, "Internal Server Error", format!("PNG: {e}\n")),
    }
}

fn event_args(req: &Request) -> Result<(u32, u32, u32), String> {
    let evt = req.code("evt")?;
    let word = |key: &str| -> Result<u32, String> {
        match req.get(key) {
            None => Ok(0),
            Some(raw) => resolve(&raw).ok_or_else(|| {
                format!("{key}={raw:?} is neither a number nor a published SDK name")
            }),
        }
    };
    Ok((evt, word("wp")?, word("dwp")?))
}

/// The contacts a `/input/pointer` request asks for, in the order they should be posted.
///
/// A tap is two contacts in one turn — the shorthand the command line's `--pointer tap` means. It
/// is also the wrong one for a pack that reads which contacts are *currently* down: such a caller
/// wants `down`, an `advance`, then `up`, which is why both forms exist.
fn pointer_args(req: &Request) -> Result<Vec<(u32, i32, i32)>, String> {
    let kind = req
        .get("kind")
        .unwrap_or_else(|| "tap".to_string())
        .to_ascii_lowercase();
    let (x, y) = (req.coord("x")?, req.coord("y")?);
    Ok(match kind.as_str() {
        "down" => vec![(g::EVT_POINTER_DOWN, x, y)],
        "move" => vec![(g::EVT_POINTER_MOVE, x, y)],
        "up" => vec![(g::EVT_POINTER_UP, x, y)],
        "tap" => vec![(g::EVT_POINTER_DOWN, x, y), (g::EVT_POINTER_UP, x, y)],
        other => return Err(format!("kind={other:?} is not down, move, up or tap")),
    })
}

fn status_json(rt: &Runtime) -> String {
    let d = &rt.display;
    let list = |items: &[String]| items.iter().map(|s| json(s)).collect::<Vec<_>>().join(",");
    let classes: Vec<String> = rt
        .missing_classes
        .iter()
        .map(|c| named(*c, "AEECLSID_"))
        .collect();
    // Which player holds which clip right now, which is the half of a music question the clip list
    // cannot answer: the report summarises the *clips* a run named, and a music bug is about the
    // object that was handed one.
    let players: Vec<String> = rt
        .audio_state()
        .iter()
        .map(|(addr, clip, playing)| {
            format!(
                "{{\"player\":\"{addr:#010x}\",\"clip\":{},\"playing\":{playing}}}",
                json(clip)
            )
        })
        .collect();
    // The JIT cache's block count is the one number that turns "the frame froze" into "the code
    // cache was flushed and the hot path was recompiled". It costs a scan of 65536 slots, so it is
    // gated on the same diagnostic switch that draws the HUD line; a `--serve` client polls /status
    // before and after a stall and watches for the drop.
    let jit_blocks = if crate::session::stats_wanted() {
        rt.jit_cached_blocks()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "null".to_string())
    } else {
        "null".to_string()
    };
    format!(
        "{{\"device\":\"{}x{}\",\"model\":{},\"presentations\":{},\"text_runs\":{},\"steps\":{},\
         \"clock_ms\":{},\"applet\":\"{:#010x}\",\"applet_cls\":\"{:#010x}\",\"last_call\":{},\
         \"notes\":{},\"notes_dropped\":{},\"debug_lines\":{},\"sounds\":{},\"audio_players\":[{}],\
         \"missing_resources\":[{}],\"missing_classes\":[{}],\"fault\":{},\"jit_blocks\":{}}}\n",
        rt.device.width,
        rt.device.height,
        json(&rt.device.model_name),
        d.presentations,
        d.runs_seen,
        rt.steps(),
        rt.clock_ms,
        rt.applet,
        rt.app_cls,
        json(rt.last_call),
        d.quiet.len(),
        d.notes_dropped,
        rt.debug.len(),
        rt.audio.len(),
        players.join(","),
        list(&rt.missing_resources),
        list(&classes),
        opt_json(rt.crash.map(|c| fault_text(&c)).as_deref()),
        jit_blocks,
    )
}

fn fault_text(c: &crate::runtime::Crash) -> String {
    format!(
        "pc {:#010x} ({}) after {} instructions",
        c.pc,
        if c.thumb { "Thumb" } else { "ARM" },
        c.steps
    )
}

/// A number, or an SDK name for one — the two forms the command line takes too, for the same
/// reason: a report says `AVK_DOWN`, so an interface meant to be driven by someone reading reports
/// has to accept it.
fn resolve(v: &str) -> Option<u32> {
    if let Some(n) = abi::const_value(v) {
        return Some(n);
    }
    let v = v.trim();
    match v.strip_prefix("0x").or(v.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => v.parse::<u32>().ok(),
    }
}

/// The published name for a value, or the number when the SDK has none. Header enums share values
/// freely, so the caller says which family it is naming rather than taking whichever entry the
/// table lists first.
fn named(value: u32, prefers: &str) -> String {
    if let Some(entry) = abi::CONST_TABLE
        .iter()
        .find(|e| e.value == value && e.name.starts_with(prefers))
    {
        return entry.name.to_string();
    }
    format!("{value:#x}")
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// `Content-Length` from a header block, or 0 when it is absent — which is every GET.
fn content_length(head: &[u8]) -> usize {
    let text = String::from_utf8_lossy(head);
    for line in text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse().unwrap_or(0);
        }
    }
    0
}

fn parse_request(head: &[u8]) -> Result<Request, String> {
    let text = String::from_utf8_lossy(head);
    let line = text.lines().next().ok_or("empty request")?;
    let mut parts = line.split_whitespace();
    let method = parts.next().ok_or("no method")?.to_ascii_uppercase();
    if method != "GET" && method != "POST" {
        return Err(format!("{method} is not GET or POST"));
    }
    let target = parts.next().ok_or("no path")?;
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), split_query(q)),
        None => (target.to_string(), Vec::new()),
    };
    Ok(Request {
        method,
        path,
        query,
    })
}

fn split_query(q: &str) -> Vec<(String, String)> {
    q.split('&')
        .filter(|f| !f.is_empty())
        .map(|field| match field.split_once('=') {
            Some((k, v)) => (decode(k), decode(v)),
            None => (decode(field), String::new()),
        })
        .collect()
}

/// `%XX` and `+`: the two things a query string does to a value. A control channel carries key
/// names and paths, so this is the whole of the decoding it needs.
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn opt_json(v: Option<&str>) -> String {
    match v {
        None => "null".to_string(),
        Some(s) => json(s),
    }
}

/// Which source a cursor belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Notes,
    Runs,
    Screens,
    Debug,
    Sound,
    /// The two lists of what the host could not serve. Two cursors rather than one, because they
    /// are two lists that grow independently: a single index over their concatenation would shift
    /// under the other one and replay or skip lines.
    Resources,
    Classes,
    /// The order of the run's `ISoundPlayer` calls: the only place a music switch is visible as a
    /// sequence rather than as a state.
    Music,
    /// The order of every host call that is not one a pack makes by the thousand, with the virtual
    /// time each happened at — the answer to "what did the pack ask just before that".
    Call,
    /// The requests a `--net-proxy` run made and what came back, in order, with the virtual time of
    /// each. Separate from `Call` rather than folded into it because the two halves of a transaction
    /// have to be readable together, and because the answer arrives outside any call the pack makes:
    /// a `call` line can say the pack asked, and only this list can say it was ever told anything.
    Net,
}

/// What a run has said, in order, so that "since N" is a question with an answer.
///
/// Every source here is a list a run appends to and may also *trim*: the display caps its notes and
/// its text runs, and a module's `dbgprintf` lines are capped too. So a cursor is not an index into
/// any of them — it is a sequence number this journal assigns, and trimming is handled by comparing
/// it against how much each source says it has dropped. Without that, a run long enough to trim
/// would replay old lines or skip new ones.
#[derive(Debug, Default)]
struct Journal {
    entries: VecDeque<Entry>,
    /// The sequence number the next entry will be given.
    next: u64,
    /// How many entries were dropped to stay under [`JOURNAL`].
    dropped: u64,
    notes: u64,
    runs: u64,
    screens: u64,
    debug: u64,
    sound: u64,
    resources: u64,
    classes: u64,
    music: u64,
    call: u64,
    net: u64,
}

#[derive(Debug)]
struct Entry {
    seq: u64,
    kind: &'static str,
    at: u64,
    text: String,
}

impl Journal {
    fn cursor(&self, slot: Slot) -> u64 {
        match slot {
            Slot::Notes => self.notes,
            Slot::Runs => self.runs,
            Slot::Screens => self.screens,
            Slot::Debug => self.debug,
            Slot::Sound => self.sound,
            Slot::Resources => self.resources,
            Slot::Classes => self.classes,
            Slot::Music => self.music,
            Slot::Call => self.call,
            Slot::Net => self.net,
        }
    }

    fn set_cursor(&mut self, slot: Slot, at: u64) {
        match slot {
            Slot::Notes => self.notes = at,
            Slot::Runs => self.runs = at,
            Slot::Screens => self.screens = at,
            Slot::Debug => self.debug = at,
            Slot::Sound => self.sound = at,
            Slot::Resources => self.resources = at,
            Slot::Classes => self.classes = at,
            Slot::Music => self.music = at,
            Slot::Call => self.call = at,
            Slot::Net => self.net = at,
        }
    }

    fn push(&mut self, kind: &'static str, at: u64, text: String) {
        if self.entries.len() >= JOURNAL {
            let drop = self.entries.len() - JOURNAL / 2;
            self.entries.drain(..drop);
            self.dropped += drop as u64;
        }
        self.entries.push_back(Entry {
            seq: self.next,
            kind,
            at,
            text,
        });
        self.next += 1;
    }

    /// Take whatever a source has appended since the last call.
    ///
    /// `base` is the sequence number of the first line the source still holds and `total` how many
    /// it has produced altogether; the difference is what trimming removed, and that difference is
    /// the only way to tell "it said nothing" from "the beginning was thrown away".
    fn take(
        &mut self,
        kind: &'static str,
        slot: Slot,
        base: u64,
        total: u64,
        items: &[(u64, String)],
    ) {
        let seen = self.cursor(slot);
        if seen < base {
            self.push(
                "lost",
                0,
                format!("{} line(s) of {kind} were not kept", base - seen),
            );
        }
        for (at, text) in items.iter().skip((seen.max(base) - base) as usize) {
            self.push(kind, *at, text.clone());
        }
        self.set_cursor(slot, total);
    }

    /// Pull whatever each source has appended since the last call.
    fn observe(&mut self, rt: &Runtime) {
        let d = &rt.display;
        // Notes: the display keeps 256 of them and drains half when it needs room, counting what it
        // dropped — which is what turns its list into a sequence.
        let notes: Vec<(u64, String)> = d.quiet.iter().map(|n| (d.now, n.clone())).collect();
        self.take(
            "note",
            Slot::Notes,
            d.notes_dropped,
            d.notes_dropped + notes.len() as u64,
            &notes,
        );
        // Text runs: the same shape, but their total is `runs_seen` rather than a dropped counter.
        let runs: Vec<(u64, String)> = d
            .runs
            .iter()
            .map(|r| {
                (
                    r.step,
                    format!(
                        "{:?} at ({},{}) {}x{} in {:#06x}, flags {:#010x}",
                        r.text, r.x, r.y, r.width, r.height, r.font, r.flags
                    ),
                )
            })
            .collect();
        let runs_base = d.runs_seen - runs.len() as u64;
        self.take("text", Slot::Runs, runs_base, d.runs_seen, &runs);
        // Screens, sounds and the two gap lists are append-only, so their base is zero.
        let screens: Vec<(u64, String)> = d.screens.iter().map(|s| (*s, String::new())).collect();
        self.take("screen", Slot::Screens, 0, screens.len() as u64, &screens);
        let debug: Vec<(u64, String)> = rt
            .debug
            .iter()
            .map(|line| (rt.steps(), line.clone()))
            .collect();
        let debug_base = u64::from(rt.debug_dropped);
        self.take(
            "debug",
            Slot::Debug,
            debug_base,
            debug_base + debug.len() as u64,
            &debug,
        );
        let sounds: Vec<(u64, String)> = rt
            .audio
            .iter()
            .map(|s| {
                (
                    0,
                    match &s.problem {
                        Some(why) => format!("{} ({} bytes, {why})", s.name, s.bytes.len()),
                        None => format!("{} ({} bytes)", s.name, s.bytes.len()),
                    },
                )
            })
            .collect();
        self.take("sound", Slot::Sound, 0, sounds.len() as u64, &sounds);
        let resources: Vec<(u64, String)> = rt
            .missing_resources
            .iter()
            .map(|r| (0, r.clone()))
            .collect();
        self.take(
            "gap",
            Slot::Resources,
            0,
            resources.len() as u64,
            &resources,
        );
        let classes: Vec<(u64, String)> = rt
            .missing_classes
            .iter()
            .map(|c| (0, named(*c, "AEECLSID_")))
            .collect();
        self.take("gap", Slot::Classes, 0, classes.len() as u64, &classes);
        // The music sequence, which is a log a run *keeps* rather than a list it derives: the order
        // of `SetInfo`/`Play`/`Stop` is the whole question when a pack's music comes back wrong, and
        // the answers above are states, which cannot show an order.
        let music: Vec<(u64, String)> = rt
            .sound_log
            .iter()
            .map(|line| (u64::from(rt.clock_ms), line.clone()))
            .collect();
        let music_base = u64::from(rt.sound_dropped);
        self.take(
            "music",
            Slot::Music,
            music_base,
            music_base + music.len() as u64,
            &music,
        );
        // The call sequence, which is the same idea one level up: every host call that is not one a
        // pack makes by the thousand. Each line carries its own clock, so it lines up against
        // anything else in the log.
        let calls: Vec<(u64, String)> = rt.call_log.iter().map(|line| (0, line.clone())).collect();
        let call_base = u64::from(rt.call_dropped);
        self.take(
            "call",
            Slot::Call,
            call_base,
            call_base + calls.len() as u64,
            &calls,
        );
        // The network side of a `--net-proxy` run, append-only for the same reason the screens are:
        // a run makes as many requests as it makes, and this list is never trimmed to make room.
        self.take("net", Slot::Net, 0, rt.net_log.len() as u64, &rt.net_log);
    }

    /// The lines a caller has not seen yet, as NDJSON, plus the sequence to ask with next time.
    fn body(&self, since: u64) -> Vec<u8> {
        let mut out = Vec::new();
        // A `since` older than what is still held means lines were missed; say how many rather than
        // letting the record look complete.
        if let Some(first) = self.entries.front() {
            if since < first.seq {
                out.extend_from_slice(
                    format!(
                        "{{\"seq\":{since},\"kind\":\"lost\",\"at\":0,\"text\":{}}}\n",
                        json(&format!("{} earlier line(s) are gone", first.seq - since))
                    )
                    .as_bytes(),
                );
            }
        }
        for e in self.entries.iter().filter(|e| e.seq >= since) {
            out.extend_from_slice(
                format!(
                    "{{\"seq\":{},\"kind\":{},\"at\":{},\"text\":{}}}\n",
                    e.seq,
                    json(e.kind),
                    e.at,
                    json(&e.text)
                )
                .as_bytes(),
            );
        }
        out
    }

    fn since(&self, since: u64) -> (Vec<Line>, u64, u64) {
        let wanted = self
            .entries
            .iter()
            .filter(|e| e.seq >= since)
            .map(|e| (e.seq, e.kind, e.at, e.text.clone()))
            .collect();
        let missing = match self.entries.front() {
            Some(first) if since < first.seq => first.seq - since,
            _ => 0,
        };
        (wanted, self.next, missing)
    }
}

#[cfg(test)]
#[path = "test/control.rs"]
mod tests;
