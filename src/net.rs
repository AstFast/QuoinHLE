//! The guest's own requests, really sent: what `--net-proxy` turns on.
//!
//! Without that flag this host answers every network question the way a handset with no data
//! connection does — `IWeb::GetResponse` lays out a response whose `nCode` is the negative
//! `WEB_ERROR_CONNECT` and reports it through the caller's own `AEECallback` — and a run stays
//! byte-for-byte reproducible. With
//! `--net-proxy HOST:PORT` the request is handed to that local proxy for real, and what comes back
//! is what the log and (through `IWebResp`) the guest are told. It is a developer flag: it exists so
//! that the requests a pack makes can be *watched*, and answered by a proxy of the developer's own,
//! rather than so that a pack can reach the internet.
//!
//! Two properties of this crate shape everything below.
//!
//! 1. **The guest runs on the thread that called it.** `runtime.rs` relies on exactly that where it
//!    measures its own stack, and it is why nothing here may block that thread. `ureq` is a blocking
//!    client, so it runs on one worker thread this struct owns, and the emulator only ever *polls*
//!    it — once a turn, from `Runtime::pump`. A reply that has not arrived yet is simply not there
//!    yet, which is the same shape the guest already waits on.
//! 2. **What a proxy receives from `ureq` is a CONNECT tunnel, not a forwarded request.** That is
//!    the only HTTP proxy mode `ureq` has: its own `ProxyProtocol::Http` is documented as "CONNECT
//!    proxy over HTTP". So the first line off this host is `CONNECT host:port HTTP/1.1` and the
//!    guest's own request follows *inside* that tunnel, which is why an `http://` origin stays
//!    readable to a proxy that does nothing but relay bytes — and why an `https://` one does not:
//!    what is inside that tunnel is TLS between `ureq` and the origin. This host is the one doing
//!    that handshake, so the request still works; the *proxy* is what sees only ciphertext.
//!
//! What is deliberately *not* here: retries, redirects that change the guest's URL, cookies, or a
//! connection pool shared across requests. A pack that asks twice sends two requests, and the log
//! says so, because reconciling a retry with the guest's view of one transaction would be this
//! host inventing state the pack never asked for.

use std::cell::RefCell;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

/// How long one request may take before it is reported as a failure.
///
/// A proxy on this machine answers in milliseconds, so this is not a tuning knob: it is the bound
/// that keeps one unreachable proxy from parking the worker thread for the rest of the run.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// How much of a response body this host keeps, and hands to the guest through `ISource`. A body
/// larger than this is reported as truncated rather than silently cut, so the log says which of the
/// two happened.
///
/// It bounds what is *kept*, not what is read: `ureq` has the whole response in hand before this is
/// applied, so a response far larger than this still costs the memory to receive it. Bounding that
/// would mean streaming the body into the guest's own buffer as it arrives, which is a different
/// shape of host than a local proxy needs.
pub const BODY_CAP: usize = 8 * 1024 * 1024;

/// One guest request, with the id the emulator will match the answer back to.
///
/// The id is the emulator's, not the network's: two calls can carry the same URL and must still be
/// told apart, because each has its own `AEECallback` waiting.
pub struct Ask {
    pub id: u64,
    pub url: String,
}

/// What the answer is, once there is one.
pub struct Answer {
    /// The status line's code, kept as the number the guest's `WebRespInfo.nCode` wants.
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    /// Whether the body was cut at [`BODY_CAP`]. A response that was truncated is still a response,
    /// but the log must not read as though the whole of it arrived.
    pub truncated: bool,
}

/// One request's outcome, or why there is none.
pub struct Reply {
    pub id: u64,
    pub url: String,
    pub result: Result<Answer, String>,
}

/// The worker thread and the two channels around it.
///
/// `rt.net` is `None` unless a run was given `--net-proxy`, so every other path in the emulator is
/// unchanged: there is no thread, no socket, and no wall clock in a run that did not ask for one.
pub struct Proxy {
    tx: Sender<Ask>,
    rx: Receiver<Reply>,
    /// Held so the thread is named and joinable at drop; dropping the `Sender` is what ends it.
    worker: Option<JoinHandle<()>>,
    /// The address as it will be printed in the log, scheme included.
    pub at: String,
    /// `--net-script`: answer here instead of sending to a worker. `None` is the real proxy.
    script: Option<ScriptServer>,
    /// Replies a script produced, drained by the same poll as the worker's channel.
    script_replies: RefCell<Vec<Reply>>,
    /// How many requests this run has handed to the backend, sent or answered. The manager's network
    /// page reports it next to the socket rows, because a script run has no socket to count.
    asked: RefCell<u64>,
}

/// Split a `host:port` as a `SOCKET` line spells it: `*` or an IPv4 literal, then the port.
fn split_host_port(text: &str) -> Option<(Option<std::net::Ipv4Addr>, u16)> {
    let (host, port) = text.rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    let host = match host {
        "*" | "" => None,
        literal => Some(literal.parse::<std::net::Ipv4Addr>().ok()?),
    };
    Some((host, port))
}

impl Proxy {
    /// Bind the worker to `addr`, which is `HOST:PORT` with or without a scheme.
    ///
    /// A missing scheme is filled in as `http://`: a proxy the developer runs locally is not going to
    /// be TLS, and requiring the scheme on the command line would only be one more thing to get
    /// wrong. The address is *not* resolved or connected to here — a proxy that is not up yet is a
    /// request that fails, not a run that refuses to start, because a developer restarts their proxy
    /// far more often than they restart the emulator.
    pub fn new(addr: &str) -> Result<Proxy, String> {
        let url = if addr.contains("://") {
            addr.to_string()
        } else {
            format!("http://{addr}")
        };
        let proxy = ureq::Proxy::new(&url).map_err(|e| format!("--net-proxy {addr}: {e}"))?;
        // `http_status_as_error(false)` is not a detail: a 404 is an *answer* a pack may well have a
        // branch for, and ureq would otherwise turn it into an `Err` indistinguishable from a
        // connection that never happened.
        let config = ureq::config::Config::builder()
            .proxy(Some(proxy))
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .build();
        let agent = ureq::Agent::new_with_config(config);

        let (tx, asks) = channel::<Ask>();
        let (replies, rx) = channel::<Reply>();
        let worker = std::thread::Builder::new()
            .name("quoin-net".to_string())
            .spawn(move || {
                // Serial on purpose: one request at a time keeps the log in the order the guest made
                // the calls, and a local proxy leaves nothing to gain from overlapping them.
                while let Ok(ask) = asks.recv() {
                    let reply = Reply {
                        id: ask.id,
                        url: ask.url.clone(),
                        result: fetch(&agent, &ask.url),
                    };
                    if replies.send(reply).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("cannot start the network thread: {e}"))?;

        Ok(Proxy {
            tx,
            rx,
            worker: Some(worker),
            at: url,
            script: None,
            script_replies: RefCell::new(Vec::new()),
            asked: RefCell::new(0),
        })
    }

    /// Build the local, scripted backend: the guest's `IWeb::GetResponse` is answered from a route
    /// file, with no socket and no worker thread.
    ///
    /// This is the offline-server counterpart to [`Proxy::new`]. It exists for packs whose origin is
    /// gone: the route file is the contract, and an unmatched request is an ordinary 404 the pack can
    /// branch on rather than a connection failure.
    /// The `SOCKET` lines of a `--net-script` file, for [`Manager::set_script_routes`].
    ///
    /// [`Manager::set_script_routes`]: crate::socket::Manager::set_script_routes
    pub fn sockets(&self) -> &crate::socket::ScriptRoutes {
        static EMPTY: std::sync::OnceLock<crate::socket::ScriptRoutes> = std::sync::OnceLock::new();
        match &self.script {
            Some(server) => server.sockets(),
            None => EMPTY.get_or_init(crate::socket::ScriptRoutes::default),
        }
    }

    pub fn from_script(path: &Path) -> Result<Proxy, String> {
        let script = ScriptServer::from_file(path)?;
        let (tx, _asks) = channel::<Ask>();
        let (_replies, rx) = channel::<Reply>();
        Ok(Proxy {
            tx,
            rx,
            worker: None,
            at: format!("script:{}", path.display()),
            script: Some(script),
            script_replies: RefCell::new(Vec::new()),
            asked: RefCell::new(0),
        })
    }

    /// Hand one request to the worker. A script backend answers immediately; a send that fails means
    /// the worker is gone, which the caller reports as a failed request rather than as a dead run.
    pub fn ask(&self, id: u64, url: String) -> Result<(), String> {
        *self.asked.borrow_mut() += 1;
        if let Some(script) = &self.script {
            let reply = Reply {
                id,
                url: url.clone(),
                result: script.answer(&url),
            };
            self.script_replies.borrow_mut().push(reply);
            return Ok(());
        }
        self.tx
            .send(Ask { id, url })
            .map_err(|_| "the network thread is no longer running".to_string())
    }

    /// How many requests this run has handed to the backend.
    pub fn asked(&self) -> u64 {
        *self.asked.borrow()
    }

    /// Every reply that has arrived since the last call, without waiting for one.
    pub fn take_replies(&self) -> Vec<Reply> {
        if self.script.is_some() {
            return self.script_replies.borrow_mut().drain(..).collect();
        }
        let mut out = Vec::new();
        while let Ok(reply) = self.rx.try_recv() {
            out.push(reply);
        }
        out
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        // Ends the worker's `recv()`. The handle is deliberately *not* joined: the thread may be
        // inside a 10-second request, and a run that is over should not wait on the network to end.
        // The thread owns nothing the emulator needs back — every byte it produces arrives by
        // channel — so letting it finish and exit on its own is enough.
        self.worker.take();
    }
}

/// One route in a `--net-script` file.
struct Route {
    method: String,
    path: String,
    status: u16,
    content_type: Option<String>,
    body: Vec<u8>,
    truncated: bool,
}

/// A local, declarative response table for packs whose origin no longer exists.
///
/// The file is one route per line, tab-separated:
///
/// ```text
/// # method	path	status	content-type	body-file
/// GET	/login	200	application/json	login.json
/// *	/health	200	text/plain	-
/// ```
///
/// A `*` method matches any request (the `IWeb` path this host exposes is GET today). A path ending
/// in `*` is a prefix; otherwise it is exact. Query strings and fragments are ignored when matching.
/// The body file is relative to the script, and `-` means an empty body. No match is a normal 404 so
/// the pack can exercise its own failure branch.
struct ScriptServer {
    routes: Vec<Route>,
    /// The `SOCKET` lines: connects that never leave the process. See [`crate::socket::ScriptRoutes`].
    sockets: crate::socket::ScriptRoutes,
}

impl ScriptServer {
    fn from_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("--net-script {}: {e}", path.display()))?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let mut routes = Vec::new();
        let mut sockets = crate::socket::ScriptRoutes::default();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
            if fields.len() < 2 {
                return Err(format!(
                    "--net-script {}:{}: want method<TAB>path[<TAB>status<TAB>content-type<TAB>body-file]",
                    path.display(),
                    number + 1
                ));
            }
            let method = fields[0].to_ascii_uppercase();
            if method == "SOCKET" {
                // `SOCKET<TAB>host:port<TAB>reply-file` — the reply file is read out and kept in
                // memory, because the socket layer answers synchronously.
                let Some(target) = fields.get(1).filter(|t| !t.is_empty()) else {
                    return Err(format!(
                        "--net-script {}:{}: SOCKET needs host:port",
                        path.display(),
                        number + 1
                    ));
                };
                let (host, port) = split_host_port(target).ok_or_else(|| {
                    format!(
                        "--net-script {}:{}: {target:?} is not host:port",
                        path.display(),
                        number + 1
                    )
                })?;
                let reply = match fields
                    .get(2)
                    .copied()
                    .filter(|s| !s.is_empty() && *s != "-")
                {
                    Some(name) => {
                        let at = if Path::new(name).is_absolute() {
                            Path::new(name).to_path_buf()
                        } else {
                            base.join(name)
                        };
                        std::fs::read(&at).map_err(|e| {
                            format!("--net-script {}:{}: {e}", path.display(), number + 1)
                        })?
                    }
                    None => Vec::new(),
                };
                sockets.add(host, port, reply);
                continue;
            }
            if method.is_empty() {
                return Err(format!(
                    "--net-script {}:{}: the method is empty",
                    path.display(),
                    number + 1
                ));
            }
            let status = match fields.get(2).filter(|s| !s.is_empty()) {
                Some(value) => value.parse::<u16>().map_err(|_| {
                    format!(
                        "--net-script {}:{}: status {value:?} is not a number",
                        path.display(),
                        number + 1
                    )
                })?,
                None => 200,
            };
            let content_type = fields
                .get(3)
                .filter(|s| !s.is_empty() && **s != "-")
                .map(|s| s.to_string());
            let body_name = fields.get(4).copied().unwrap_or("-");
            let (body, truncated) = if body_name.is_empty() || body_name == "-" {
                (Vec::new(), false)
            } else {
                let body_path = if Path::new(body_name).is_absolute() {
                    Path::new(body_name).to_path_buf()
                } else {
                    base.join(body_name)
                };
                let bytes = std::fs::read(&body_path).map_err(|e| {
                    format!(
                        "--net-script {}:{}: body {}: {e}",
                        path.display(),
                        number + 1,
                        body_path.display()
                    )
                })?;
                let truncated = bytes.len() > BODY_CAP;
                let mut body = bytes;
                if truncated {
                    body.truncate(BODY_CAP);
                }
                (body, truncated)
            };
            routes.push(Route {
                method,
                path: fields[1].to_string(),
                status,
                content_type,
                body,
                truncated,
            });
        }
        Ok(Self { routes, sockets })
    }

    /// The `SOCKET` lines, for the socket layer. See [`Proxy::sockets`].
    fn sockets(&self) -> &crate::socket::ScriptRoutes {
        &self.sockets
    }

    fn answer(&self, url: &str) -> Result<Answer, String> {
        let path = request_path(url);
        let mut best: Option<&Route> = None;
        for route in &self.routes {
            if route.method != "*" && !route.method.eq_ignore_ascii_case("GET") {
                continue;
            }
            let hit = match route.path.strip_suffix('*') {
                Some(prefix) => path.starts_with(prefix),
                None => path == route.path,
            };
            if hit && best.is_none_or(|b| route.path.len() > b.path.len()) {
                best = Some(route);
            }
        }
        Ok(match best {
            Some(route) => Answer {
                status: route.status,
                content_type: route.content_type.clone(),
                body: route.body.clone(),
                truncated: route.truncated,
            },
            None => Answer {
                status: 404,
                content_type: Some("text/plain".to_string()),
                body: format!("no virtual route for {path}\n").into_bytes(),
                truncated: false,
            },
        })
    }
}

/// The path part of a guest URL, with scheme/host/query/fragment removed.
fn request_path(url: &str) -> String {
    let no_fragment = url.split('#').next().unwrap_or(url);
    let no_query = no_fragment.split('?').next().unwrap_or(no_fragment);
    let path = if let Some(start) = no_query.find("://") {
        let rest = &no_query[start + 3..];
        match rest.find('/') {
            Some(at) => &rest[at..],
            None => "/",
        }
    } else {
        match no_query.find('/') {
            Some(at) => &no_query[at..],
            None => "/",
        }
    };
    if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    }
}

/// One request, blocking, on the worker thread.
fn fetch(agent: &ureq::Agent, url: &str) -> Result<Answer, String> {
    let mut response = agent.get(url).call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    let truncated = body.len() > BODY_CAP;
    let mut body = body;
    if truncated {
        body.truncate(BODY_CAP);
    }
    Ok(Answer {
        status,
        content_type,
        body,
        truncated,
    })
}

#[cfg(test)]
#[path = "test/net.rs"]
mod tests;
