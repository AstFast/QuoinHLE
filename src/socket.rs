use crate::socket::Pipe::{Accept, Datagrams, Ring};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read as _, Write as _};
use std::net::SocketAddr;

/// `NET_ERROR_BASE` — 3.1.2 the header. The names are that header's; the numbers are what
/// the platform's own routines store in `[+180]`, so the numbers are what the tests pin.
pub const NET_ERROR_BASE: u32 = 0x200;
/// `AEE_NET_EBADF` "Invalid socket descriptor" — the `[+184]` bit0 gate, in every slot but `OpenEx`.
pub const EBADF: u32 = NET_ERROR_BASE + 0x02;
/// `AEE_NET_EFAULT` "Invalid buffer or argument" — every `AEE_CheckPtr` refusal.
pub const EFAULT: u32 = NET_ERROR_BASE + 0x03;
/// `AEE_NET_EWOULDBLOCK` — the empty-queue answer; `Accept` pairs it with a return of -2.
pub const EWOULDBLOCK: u32 = NET_ERROR_BASE + 0x04;
/// `AEE_NET_ESOCKNOSUPPORT` "Socket parameter not supported" — the name
/// the header gives for a socket type this address family cannot provide, which is the answer
/// `INetMgr::OpenSocket` leaves for the manager's own `GetLastError` to report.
pub const ESOCKNOSUPPORT: u32 = NET_ERROR_BASE + 0x07;
/// `AEE_NET_GENERAL_FAILURE` — the header names this one for "An ISocket could
/// not be allocated", the other reason the manager's door can only answer NULL.
pub const GENERAL_FAILURE: u32 = NET_ERROR_BASE + 0x2D;
/// `AEE_NET_EOPNOTSUPP` — the `[+176] != 0` gate: a datagram socket cannot listen, accept or connect.
pub const EOPNOTSUPP: u32 = NET_ERROR_BASE + 0x0A;
/// `AEE_NET_EAFNOSUPPORT` — `Bind`, `Connect` and `SendTo` on an address block whose family word is not
/// `AF_INET`. (the delivered body's own gate; the delivered body folds this case into `EINVAL`.)
pub const EAFNOSUPPORT: u32 = NET_ERROR_BASE + 0x05;
/// `AEE_NET_EMFILE` "No more sockets available for opening" — `Accept` could not build the new port.
pub const EMFILE: u32 = NET_ERROR_BASE + 0x09;
/// `AEE_NET_ETIMEDOUT` — `Connect` on an INET socket that is `IS_INET_NONE`.
pub const ETIMEDOUT: u32 = NET_ERROR_BASE + 0x12;
/// `AEE_NET_ENOMEM` — `OpenEx`'s `aee_malloc(112)` refusal for the INET provider record.
pub const ENOMEM: u32 = NET_ERROR_BASE + 0x1F;
/// `AEE_NET_ENETNONET` "network subsystem unavailable" — `ISockPort_TCPReady`'s first answer, and
/// `TryStart`'s when no PPP link can be opened.
pub const ENETNONET: u32 = NET_ERROR_BASE + 0x2A;
/// `AEE_NET_EADDRINUSE` — `Bind` found the {protocol, port} pair live on another socket.
pub const EADDRINUSE: u32 = NET_ERROR_BASE + 0x0B;
/// `AEE_NET_ENOTCONN` — no channel, in `IsReadable`, `IsWriteable` and `Shutdown`'s last guard.
pub const ENOTCONN: u32 = NET_ERROR_BASE + 0x10;
/// `AEE_NET_EISCONN` — `[+196]` already taken, for both `Listen` and `Connect`.
pub const EISCONN: u32 = NET_ERROR_BASE + 0x0E;
/// `AEE_NET_ECONNREFUSED` — `Connect` found no listener, or found one whose backlog is full.
pub const ECONNREFUSED: u32 = NET_ERROR_BASE + 0x11;
/// `AEE_NET_ECONNRESET` — `IsWriteable` with no peer at all, and `GetPeerName` on a peer half gone.
pub const ECONNRESET: u32 = NET_ERROR_BASE + 0x13;
/// `AEE_NET_EINVAL` — "wrong state for this call": unbound, not listening, bad family, bad `how`.
pub const EINVAL: u32 = NET_ERROR_BASE + 0x1A;
/// `AEE_NET_EMSGSIZE` — `RecvFrom`'s note when it truncated a datagram, left beside a *success* count.
pub const EMSGSIZE: u32 = NET_ERROR_BASE + 0x1B;
/// `AEE_NET_ESHUTDOWN` — the direction asked about is already shut. Returned in `r0`, never stored.
pub const ESHUTDOWN: u32 = NET_ERROR_BASE + 0x20;
/// `AEE_NET_ENOPROTOOPT` — what `GetOpt` and `SetOpt` answer for every argument, unconditionally.
pub const ENOPROTOOPT: u32 = NET_ERROR_BASE + 0x22;

/// `AEE_EUNSUPPORTED`(20) — `Open` and `SelectNetwork` are four-byte bodies that return it having read
/// nothing; `IOCtl` is twelve bytes that write it into `[+180]` first and then return -1.
pub const AEE_EUNSUPPORTED: u32 = 20;
/// `AEE_ECLASSNOTSUPPORT`(3) — `AEEHostSocket_New`'s class test, and `QueryInterface`'s default arm.
pub const AEE_ECLASSNOTSUPPORT: u32 = 3;
/// `AEE_ENOMEMORY`(2) — `aee_malloc(200)` returning null.
pub const AEE_ENOMEMORY: u32 = 2;
/// `AEE_EBADSTATE`(13) — `ISockPort_QueryInterface`'s answer for the `ISocket` face of a port object that
/// is not `IS_INET`: that predicate is **two** conditions (`*(WORD)[+168] == 2`
/// **and** `*(DWORD)[+172] != 0`), and the `return 13` is at.
pub const AEE_EBADSTATE: u32 = 13;
/// `AEE_EPRIVLEVEL`(21) — `AEE_FindClassOwner`'s answer for a masked class, which is what makes
/// `0x01012d97` a different question from `0x01012d96`.
pub const AEE_EPRIVLEVEL: u32 = 21;

/// The id `gICList` registers this constructor under, and the one `Connect` passes when it builds the
/// server side of a loopback pair (`AEEHostSocket_New` refuses `a2 != 16854422`).
pub const CLS_HOSTSOCKET: u32 = 0x01012d96;
/// The socket-port class id — not this object's own, yet `IHostSocket_QueryInterface` answers it.
pub const CLS_SOCKPORT: u32 = 0x01012d97;
/// `AEECLSID_CORE+45`, the app-visible face. It belongs to `AEESockPort`, whose `QueryInterface` hands
/// out `me + 4`; **this** object's pool does not name it, so asking a host socket for the `ISocket`
/// face is a `3`, not a `13`.
pub const CLS_SOCKET: u32 = 0x0100102d;
/// The three other ids in the reference `QueryInterface` list: `AEEIID_IQI`, `AEECLSID_SOURCE` (which is
/// why an `ISource`-shaped caller works on this object) and `AEECLSID_PORT`.
pub const IID_IQI: u32 = 0x01000001;
pub const CLS_SOURCE: u32 = 0x01001012;
pub const CLS_PORT: u32 = 0x01001018;

/// The two families `OpenEx` accepts; anything else answers `EINVAL`. The 3.1.2 platform declares neither —
/// its only socket-bearing header (72 018 bytes, 317 "sock" hits) has neither these nor
/// the interfaces — so the two numbers rest on the bodies alone.
pub const FAMILY_LOCAL: u16 = 1;
pub const FAMILY_INET: u16 = 2;
/// `aee_swapl(2130706501)`: the address `Connect` gives the server side, in network order.
pub const LOOPBACK: u32 = 0x7f00_0001;
/// The ring's capacity: `IsWriteable` compares the peer's held count against 512, and `WriteV` clamps
/// each entry to `512 - held`.
pub const RING: usize = 512;
/// `Listen`'s backlog clamp — `> 8 → 8`, `< 1 → 1` — which is also the slot count of its 44-byte queue.
pub const ACCEPT_MAX: u16 = 8;
/// `SendTo` stores at most this many datagrams per socket (`if (count < 4)`) and drops the rest.
pub const DATAGRAMS: usize = 4;
/// `HostSocketMgr_NextBindPort` resets its counter to `0xffff` once it has counted below `0x400`, so no
/// loopback socket is ever handed a reserved port.
pub const PORT_FLOOR: u16 = 0x400;
/// `EventOccurred`'s two bits, named by 3.1.2 the header (`AEE_NET_WRITE_EVENT 0x01`,
/// `AEE_NET_READ_EVENT 0x02`). The documented behaviour fires the **writeable** chain for bit0 and the
/// **readable** chain for bit1; `Close` sends `3`, both.
pub const EVENT_WRITE: u32 = 0x01;
pub const EVENT_READ: u32 = 0x02;
/// The value a chain entry carries in its notify slot when the entry is a **socket source** rather
/// than a caller's callback block. The published description registers `SockSource_ReadableCB`, a function of
/// the runtime itself — so what the chain has to remember is not an address but the fact that waking
/// this entry means draining that source's own list. The number itself carries no meaning; the arm
/// that parks it is the only reader, and it only asks whether the slot is non-zero.
pub const RUNTIME_NOTIFY: u32 = 1;

/// One slot's answer, with everything the documented behaviour did beside it.
///
/// `error` distinguishes the two things a reader would otherwise merge: `Some(n)` is "the body wrote
/// `n` into `[+180]`" — including `Some(0)`, which several success paths do — and `None` is "the field
/// was left alone", which is `Open`'s shape, `Shutdown`'s already-shut answer, and `Connect`'s
/// allocation-failure path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub ret: i32,
    pub error: Option<u32>,
    /// Callback chains to resume, in the order the body walks them.
    pub fires: Vec<Fire>,
    /// Sockets this call released one reference on (`Close` of a listener, `Release` at zero).
    pub released: Vec<usize>,
}

/// A callback chain to run through `IShell::Resume` — `gvtIShell + 144`, the same slot
/// `Runtime::fire_sock` already reaches for when a generator's `Readable` hands a callback back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub socket: usize,
    /// `EVENT_READ` runs the readable chain, `EVENT_WRITE` the writeable one, `3` both.
    pub event: u32,
}

impl Done {
    const fn ok() -> Self {
        Self {
            ret: 0,
            error: Some(0),
            fires: Vec::new(),
            released: Vec::new(),
        }
    }
    fn refused(error: u32) -> Self {
        Self {
            ret: -1,
            error: Some(error),
            fires: Vec::new(),
            released: Vec::new(),
        }
    }
    /// `AEE_NET_WOULDBLOCK` (-2), this member's only other return value.
    fn blocked(error: u32) -> Self {
        Self {
            ret: -2,
            error: Some(error),
            fires: Vec::new(),
            released: Vec::new(),
        }
    }
    /// A return value with the error word untouched — `Open`'s 20, `Shutdown`'s 544.
    const fn bare(ret: i32) -> Self {
        Self {
            ret,
            error: None,
            fires: Vec::new(),
            released: Vec::new(),
        }
    }
    fn fired(mut self, socket: usize, event: u32) -> Self {
        self.fires.push(Fire { socket, event });
        self
    }
}

/// One inbound datagram: the payload, plus the sender's `{family, port, addr}` — which `SendTo` copies
/// straight out of the sender's record (`_rt_memcpy_w(record + 8, sender + 44, 132)`) rather than
/// formatting, so `RecvFrom` can report an address it never asked anybody to build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datagram {
    pub payload: Vec<u8>,
    pub from_family: u16,
    /// Network order, exactly as stored at the sender's `[+46]`.
    pub from_port: u16,
    /// Network order, as stored at the sender's `[+48]`.
    pub from_addr: u32,
}

/// The three shapes `[+196]` takes, plus "nothing allocated".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Pipe {
    #[default]
    None,
    /// A listening socket's pending-connection queue, oldest-first.
    Accept { backlog: u16, waiting: Vec<usize> },
    /// One stream socket's half of a 524-byte ring, shared with its peer.
    Ring { held: Vec<u8>, shut: u32 },
    /// A datagram socket's inbound queue.
    Datagrams { queue: Vec<Datagram> },
}

/// What a slot reads and writes about one object — the reference record's fields, by their offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// `[+8]`. The object handed to the constructor, held by one reference.
    pub owner: u32,
    /// `[+44]`.
    pub family: u16,
    /// `[+46]`, network order as it arrived from the caller.
    pub port: u16,
    /// `[+48]`, network order.
    pub addr: u32,
    /// `[+176]`. 0 = stream, 1 = datagram.
    pub protocol: u32,
    /// `[+180]`. `GetLastError` is a load of this word.
    pub last_error: u32,
    /// `[+184]`. bit0 open, bit1 bound, bit2 listening.
    pub flags: u32,
    /// `[+188]` — readable chain, head-inserted.
    pub readable: Vec<usize>,
    /// `[+192]` — writeable chain, head-inserted.
    pub writeable: Vec<usize>,
    pub pipe: Pipe,
    /// `[+516]` of this socket's ring: the peer, as a slot. Only a ring carries one.
    pub peer: Option<usize>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            owner: 0,
            family: 0,
            port: 0,
            addr: 0,
            protocol: 0,
            last_error: 0,
            flags: 0,
            readable: Vec::new(),
            writeable: Vec::new(),
            pipe: Pipe::None,
            peer: None,
        }
    }
}

/// What a successful `Connect` hands the runtime: the slot of the **server-side socket it created**,
/// and the chains to run — which are not in the caller's return value, because the documented behaviour fires
/// them before it returns 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connected {
    pub server: usize,
    pub done: Done,
}

/// Where a guest's `AF_INET` connect goes.
///
/// The default is the loopback model, which is what keeps a run reproducible: two guest sockets talk
/// to each other and nothing leaves the process. Turning `real` on sends the connection to the host's
/// own network instead, and `proxy` redirects it to a local address first — the same idea as
/// `--net-proxy`, moved down to the socket layer so it covers a pack that writes its own protocol
/// rather than only the ones that go through `IWeb`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub real: bool,
    pub proxy: Option<SocketAddr>,
}

/// One live host connection, as the manager's network page draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub port: u16,
    /// The address word the model stores, network order read as a little-endian word.
    pub addr: u32,
    pub dgram: bool,
    pub up: u64,
    pub down: u64,
}

/// What the host has seen: the totals, and one row per live connection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub traffic: Traffic,
    pub rows: Vec<Row>,
    /// The guest clock when the counters were last zeroed, if they ever were. `None` means "since the
    /// run started", which is what a reader needs in order to read a rate rather than a total.
    pub cleared_at: Option<u64>,
}

/// What the host has seen on its real connections. The manager's network page reads this; a capture
/// would hang off the same counters, which is why they are counted here rather than at the callers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Traffic {
    pub connects: u64,
    pub up: u64,
    pub down: u64,
}

/// The reference file-scope state of the member: the `bss$2` list, its two per-protocol ephemeral port
/// counters, and the order the scans walk in. Owned by one `Runtime`, never global — a second emulator
/// run must not see the first one's sockets, and cannot inherit its port counter.
#[derive(Debug, Default)]
pub struct Manager {
    slots: Vec<Option<Record>>,
    /// `bss$2`'s order, head first. `AEEHostSocket_New` does `v7[3] = bss$2; bss$2 = v7`, so the
    /// **newest** live socket is scanned first — an observable rule, because two listeners on one port
    /// decide which one a `Connect` lands on.
    order: Vec<usize>,
    /// `word_1104` (protocol 0) and `word_1106` (any other), both initialized to `0xffff` — measured as
    /// the `.data` image bytes `ff ff` at each of the two addresses.
    next_stream: u16,
    next_dgram: u16,
    /// The real connections, keyed by the slot they stand behind. A slot here answers its bytes from
    /// the host's own socket instead of from a ring; everything about its *record* — family, port,
    /// flags, the error word — stays the model's, so the ABI sees one shape either way.
    real: HashMap<usize, Real>,
    /// Connections a `--net-script` file answers; see [`ScriptRoutes`].
    script: HashMap<usize, ScriptConn>,
    routes: ScriptRoutes,
    /// Bytes each live slot has carried, in and out. Kept here rather than on `Record` so the
    /// loopback model and a host connection are counted by the same fields -- the network page does not
    /// care which transport a row came from, only that the number is the same number.
    bytes: HashMap<usize, (u64, u64)>,
    /// The clock at the last [`Manager::clear_traffic`], for the same reason the report carries it.
    cleared_at: Option<u64>,
    policy: Policy,
    traffic: Traffic,
}

/// One host connection, non-blocking, behind a model slot.
///
/// It holds its own `error` word because the byte methods run while the map is borrowed; `last_error`
/// prefers it, so a real socket's refusals read the same way a loopback socket's do.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct Real {
    socket: socket2::Socket,
    error: u32,
    /// A datagram socket answers `send_to`/`recv_from` rather than a byte stream.
    dgram: bool,
}

/// Web has no kernel sockets. The map stays empty; the methods keep the same shape so the generic
/// socket code compiles and reports the model's loopback answers.
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct Real {
    error: u32,
    dgram: bool,
}

/// What one `SOCKET` line of a `--net-script` file says: which peer, and what it sends back.
///
/// The HTTP side of the same file answers `IWeb`; this side answers a pack that writes its own
/// protocol over `ISocket`, which is the half a raw socket has no route for.
#[derive(Debug, Clone)]
pub struct ScriptRoute {
    /// `None` matches any address (`*` in the file).
    pub host: Option<std::net::Ipv4Addr>,
    pub port: u16,
    pub reply: Vec<u8>,
}

/// The `SOCKET` lines of a script, in file order — the first match answers.
#[derive(Debug, Clone, Default)]
pub struct ScriptRoutes {
    routes: Vec<ScriptRoute>,
}

impl ScriptRoutes {
    pub fn add(&mut self, host: Option<std::net::Ipv4Addr>, port: u16, reply: Vec<u8>) {
        self.routes.push(ScriptRoute { host, port, reply });
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// The reply for a connect to `addr:port`, or `None` when no line names it.
    pub fn reply_for(&self, addr: u32, port: u16) -> Option<Vec<u8>> {
        let wanted = std::net::Ipv4Addr::from(addr.to_le_bytes());
        self.routes
            .iter()
            .find(|route| route.port == port && route.host.is_none_or(|host| host == wanted))
            .map(|route| route.reply.clone())
    }
}

/// A connection answered by a script instead of a socket: the reply bytes are readable once, then the
/// peer is gone (a read answers 0), and whatever the guest writes is accepted and dropped — there is
/// no server behind the line to read it.
#[derive(Debug, Default)]
struct ScriptConn {
    reply: Vec<u8>,
    sent: usize,
}

/// `socket2` 0.6 asks for `MaybeUninit` buffers. A `u8` has no invalid bit patterns, so viewing an
/// initialised byte slice that way is sound; every byte a call reports written is one the kernel wrote.
#[cfg(not(target_arch = "wasm32"))]
fn unfilled(buf: &mut [u8]) -> &mut [std::mem::MaybeUninit<u8>] {
    unsafe {
        std::slice::from_raw_parts_mut(
            buf.as_mut_ptr() as *mut std::mem::MaybeUninit<u8>,
            buf.len(),
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Real {
    fn is_readable(&self) -> u32 {
        // A one-byte peek answers both questions a stream has: a byte is there, or the peer closed
        // (a zero), and both are "readable" to the caller.
        let mut probe = [0u8; 1];
        match self.socket.peek(unfilled(&mut probe)) {
            Ok(_) => 0,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => EWOULDBLOCK,
            Err(_) => ECONNRESET,
        }
    }

    fn is_writeable(&mut self) -> u32 {
        // The only writability fact this host can ask for is the pending socket error: a non-blocking
        // connect that has not landed reports one, and a refused one keeps reporting it until it is
        // taken. Taking it here is also what lets the answer survive into `GetLastError`.
        match self.socket.take_error() {
            Ok(None) => 0,
            Ok(Some(e)) if e.raw_os_error() == Some(0) => 0,
            Ok(Some(_)) => {
                self.error = ECONNREFUSED;
                ECONNREFUSED
            }
            Err(_) => EWOULDBLOCK,
        }
    }
}
#[cfg(target_arch = "wasm32")]
impl Real {
    fn is_readable(&self) -> u32 {
        ENETNONET
    }

    fn is_writeable(&mut self) -> u32 {
        ENETNONET
    }
}

impl Manager {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            order: Vec::new(),
            next_stream: 0xffff,
            next_dgram: 0xffff,
            real: HashMap::new(),
            script: HashMap::new(),
            routes: ScriptRoutes::default(),
            bytes: HashMap::new(),
            cleared_at: None,
            policy: Policy::default(),
            traffic: Traffic::default(),
        }
    }

    /// The live records, `bss$2` order (newest first).
    pub fn walk(&self) -> impl Iterator<Item = (usize, &Record)> {
        self.order
            .iter()
            .filter_map(move |slot| self.slots[*slot].as_ref().map(|record| (*slot, record)))
    }

    pub fn record(&self, slot: usize) -> Option<Record> {
        self.slots.get(slot).and_then(|one| one.clone())
    }

    pub fn live(&self) -> usize {
        self.order.len()
    }

    pub fn is_live(&self, slot: usize) -> bool {
        self.order.contains(&slot)
    }

    /// `AEEHostSocket_New(pVoid, cls, ppv)` (`@0x0`, 76 bytes). The order of its answers is the body's:
    /// the class test comes **before** the allocation, and unlike `AEESockPort_New` there is no
    /// `AEE_CheckPtr` on the out-parameter — the write of `*ppv` happens last, only on success.
    pub fn born(&mut self, owner: u32, cls: u32) -> Result<usize, u32> {
        if cls != CLS_HOSTSOCKET {
            return Err(AEE_ECLASSNOTSUPPORT);
        }
        let slot = self
            .slots
            .iter()
            .position(|one| one.is_none())
            .unwrap_or(self.slots.len());
        if slot == self.slots.len() {
            self.slots.push(None);
        }
        // The one reference the constructor establishes lives where the documented behaviour puts it — `[+4]`
        // of the record, which is the word the host's object layer counts — so the model stores none.
        self.slots[slot] = Some(Record {
            owner,
            ..Record::default()
        });
        self.order.insert(0, slot);
        Ok(slot)
    }

    pub fn next_bind_port(&mut self, protocol: u32) -> u16 {
        loop {
            let counter = if protocol == 0 {
                &mut self.next_stream
            } else {
                &mut self.next_dgram
            };
            if *counter < PORT_FLOOR {
                *counter = 0xffff;
            }
            let candidate = *counter;
            *counter = candidate.wrapping_sub(1);
            let taken = self.walk().any(|(_, one)| {
                one.family == FAMILY_INET && one.protocol == protocol && one.port == candidate
            });
            if !taken {
                return candidate;
            }
        }
    }

    fn take(&mut self, slot: usize) -> Record {
        self.slots[slot]
            .take()
            .expect("the runtime only hands out live slots")
    }

    fn put(&mut self, slot: usize, record: Record) {
        self.slots[slot] = Some(record);
    }

    /// Store a refusal's errno without any other change — the shape `Readable`/`Writeable` need when
    /// the caller's callback block is not reachable: `[+180]` gets the errno and `r0` returns it.
    pub fn touch_error(&mut self, slot: usize, error: u32) {
        self.touch(slot, error);
    }

    pub fn forget(&mut self, slot: usize) {
        self.real.remove(&slot);
        self.script.remove(&slot);
        self.put(slot, Record::default());
        self.slots[slot] = None;
        self.order.retain(|one| *one != slot);
    }

    pub fn accepts(&self, id: u32) -> bool {
        matches!(
            id,
            IID_IQI | CLS_SOURCE | CLS_PORT | CLS_HOSTSOCKET | CLS_SOCKPORT
        )
    }

    pub fn open_ex(&mut self, slot: usize, family: u16, protocol: u32) -> Done {
        let mut record = self.take(slot);
        let done = if record.flags & 1 != 0 {
            record.last_error = EBADF;
            Done::refused(EBADF)
        } else if family != FAMILY_LOCAL && family != FAMILY_INET {
            record.last_error = EINVAL;
            Done::refused(EINVAL)
        } else {
            record.family = family;
            if protocol > 1 {
                record.last_error = EINVAL;
                Done::refused(EINVAL)
            } else {
                record.protocol = protocol;
                if protocol == 1 {
                    record.pipe = Datagrams { queue: Vec::new() };
                }
                record.last_error = 0;
                record.flags |= 1;
                #[cfg(not(target_arch = "wasm32"))]
                if self.policy.real && family == FAMILY_INET && protocol == 1 {
                    // A real datagram socket, unbound until the first `send_to`; the record keeps the
                    // family and protocol the guest asked for.
                    match socket2::Socket::new(
                        socket2::Domain::IPV4,
                        socket2::Type::DGRAM,
                        Some(socket2::Protocol::UDP),
                    ) {
                        Ok(socket) if socket.set_nonblocking(true).is_ok() => {
                            self.real.insert(
                                slot,
                                Real {
                                    socket,
                                    error: 0,
                                    dgram: true,
                                },
                            );
                        }
                        _ => {
                            record.last_error = ENOMEM;
                            self.put(slot, record);
                            return Done::refused(ENOMEM);
                        }
                    }
                }
                Done::ok()
            }
        };
        self.put(slot, record);
        done
    }

    pub fn bind(&mut self, slot: usize, family: u16, port: u16, addr: u32, addr_ok: bool) -> Done {
        let snapshot = self.record(slot).expect("live");
        let (open, bound) = (snapshot.flags & 1 != 0, snapshot.flags & 2 != 0);
        if !open {
            self.touch(slot, EBADF);
            return Done::refused(EBADF);
        }
        if bound {
            self.touch(slot, EINVAL);
            return Done::refused(EINVAL);
        }
        if family == FAMILY_LOCAL {
            self.set_flags(slot, snapshot.flags | 2);
            return Done::ok();
        }
        if !addr_ok {
            self.touch(slot, EFAULT);
            return Done::refused(EFAULT);
        }
        if port == 0 {
            let candidate = self.next_bind_port(snapshot.protocol);
            let mut record = self.take(slot);
            record.family = FAMILY_INET;
            record.addr = addr;
            record.port = candidate.swap_bytes();
            record.flags |= 2;
            self.put(slot, record);
            return Done::ok();
        }
        let clash = self.walk().any(|(other, one)| {
            other != slot
                && one.family == FAMILY_INET
                && one.protocol == snapshot.protocol
                && one.port == port
        });
        if clash {
            self.touch(slot, EADDRINUSE);
            return Done::refused(EADDRINUSE);
        }
        let mut record = self.take(slot);
        record.family = FAMILY_INET;
        record.addr = addr;
        record.port = port;
        record.flags |= 2;
        self.put(slot, record);
        Done::ok()
    }

    /// A refusal that still lands in `[+180]` — the platform's own routines write the errno before returning in
    /// every case, including the pointer gates.
    fn touch(&mut self, slot: usize, error: u32) {
        let mut record = self.take(slot);
        record.last_error = error;
        self.put(slot, record);
    }

    fn set_flags(&mut self, slot: usize, flags: u32) {
        let mut record = self.take(slot);
        record.flags = flags;
        self.put(slot, record);
    }

    pub fn listen(&mut self, slot: usize, backlog: i32) -> Done {
        let mut record = self.take(slot);
        let done = if record.flags & 1 == 0 {
            record.last_error = EBADF;
            Done::refused(EBADF)
        } else if record.protocol != 0 {
            record.last_error = EOPNOTSUPP;
            Done::refused(EOPNOTSUPP)
        } else if record.flags & 2 == 0 {
            record.last_error = EINVAL;
            Done::refused(EINVAL)
        } else if record.pipe != Pipe::None {
            record.last_error = EISCONN;
            Done::refused(EISCONN)
        } else {
            record.pipe = Accept {
                backlog: if backlog > ACCEPT_MAX as i32 {
                    ACCEPT_MAX
                } else if backlog < 1 {
                    1
                } else {
                    backlog as u16
                },
                waiting: Vec::new(),
            };
            record.flags |= 4;
            Done::ok()
        };
        self.put(slot, record);
        done
    }

    pub fn connect(
        &mut self,
        slot: usize,
        family: u16,
        port: u16,
        addr: u32,
        addr_ok: bool,
    ) -> Result<Connected, Done> {
        let mine = self.record(slot).expect("live");
        if mine.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Err(Done::refused(EBADF));
        }
        if mine.protocol != 0 {
            self.touch(slot, EOPNOTSUPP);
            return Err(Done::refused(EOPNOTSUPP));
        }
        if mine.pipe != Pipe::None {
            self.touch(slot, EISCONN);
            return Err(Done::refused(EISCONN));
        }
        if family != FAMILY_LOCAL && !addr_ok {
            self.touch(slot, EFAULT);
            return Err(Done::refused(EFAULT));
        }
        if family == FAMILY_INET {
            if let Some(reply) = self.routes.reply_for(addr, port) {
                return self.connect_script(slot, port, addr, reply);
            }
        }
        if self.policy.real && family == FAMILY_INET {
            return self.connect_real(slot, port, addr);
        }
        let listener = self.walk().find_map(|(other, one)| {
            let address_ok = if family == FAMILY_LOCAL {
                true
            } else {
                one.family == mine.family && one.addr == addr
            };
            (one.flags & 4 != 0 && one.protocol == 0 && address_ok && one.port == port)
                .then_some(other)
        });
        let Some(listener) = listener else {
            self.touch(slot, ECONNREFUSED);
            return Err(Done::refused(ECONNREFUSED));
        };
        let room = match &self.record(listener).expect("live").pipe {
            Accept { backlog, waiting } => (waiting.len() as u16) < *backlog,
            _ => false,
        };
        if !room {
            self.touch(slot, ECONNREFUSED);
            return Err(Done::refused(ECONNREFUSED));
        }
        let owner = self.record(listener).expect("live").owner;
        // `AEEHostSocket_New(pListener[+8], 0x01012d96, &pNew)`: the server side is owned by the
        // *listener's* shell, which is why a pair created through one shell shares its resume path.
        let Ok(server) = self.born(owner, CLS_HOSTSOCKET) else {
            return Err(Done::bare(-1));
        };
        let opened = self.open_ex(server, family, mine.protocol);
        if opened.ret != 0 {
            self.forget(server);
            return Err(opened);
        }
        if family != FAMILY_LOCAL {
            let listener_port = self.record(listener).expect("live").port;
            let mut record = self.take(server);
            record.family = FAMILY_INET;
            record.addr = LOOPBACK.to_be();
            record.port = listener_port;
            self.put(server, record);
        }
        self.set_flags(server, self.record(server).expect("live").flags | 2);
        // `if (!(me[+184] & 2) && family != AF_LOCAL)` — the caller binds itself to an ephemeral port
        // through its own `Bind`, so the port it reports to `GetSockName` is that one, in network order.
        if self.record(slot).expect("live").flags & 2 == 0 && family != FAMILY_LOCAL {
            let candidate = self.next_bind_port(mine.protocol);
            let answer = self.bind(
                slot,
                FAMILY_INET,
                candidate.swap_bytes(),
                LOOPBACK.to_be(),
                true,
            );
            if answer.ret != 0 {
                self.forget(server);
                return Err(answer);
            }
        }
        for one in [slot, server] {
            let mut record = self.take(one);
            record.pipe = Ring {
                held: Vec::new(),
                shut: 0,
            };
            record.peer = if one == slot {
                Some(server)
            } else {
                Some(slot)
            };
            self.put(one, record);
        }
        let first = {
            let mut record = self.take(listener);
            let was_empty = match &mut record.pipe {
                Accept { waiting, .. } => {
                    let empty = waiting.is_empty();
                    waiting.push(server);
                    empty
                }
                _ => unreachable!("the listener's pipe is an accept queue"),
            };
            self.put(listener, record);
            was_empty
        };
        self.touch(slot, 0);
        let mut done = Done::ok();
        if first {
            done = done.fired(listener, EVENT_READ);
        }
        done = done.fired(slot, EVENT_WRITE);
        Ok(Connected { server, done })
    }

    pub fn accept(&mut self, slot: usize, ppv_ok: bool) -> Accept2 {
        if !ppv_ok {
            self.touch(slot, EFAULT);
            return Accept2::Refused(Done::refused(EFAULT));
        }
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Accept2::Refused(Done::refused(EBADF));
        }
        if record.protocol != 0 {
            self.touch(slot, EOPNOTSUPP);
            return Accept2::Refused(Done::refused(EOPNOTSUPP));
        }
        let Accept {
            backlog,
            mut waiting,
        } = record.pipe
        else {
            self.touch(slot, EINVAL);
            return Accept2::Refused(Done::refused(EINVAL));
        };
        if waiting.is_empty() {
            self.touch(slot, EWOULDBLOCK);
            return Accept2::Blocked(Done::blocked(EWOULDBLOCK));
        }
        let server = waiting.remove(0);
        self.put(
            slot,
            Record {
                pipe: Accept { backlog, waiting },
                ..record
            },
        );
        // The success branch is `return 0` with the error word **untouched**: a `GetLastError` taken after
        // an accepted connection still reports whatever the empty-queue answer left behind.
        Accept2::Taken(server)
    }

    pub fn is_readable(&self, slot: usize) -> u32 {
        if self.script.contains_key(&slot) {
            // Data or an end of stream: both are "there is something to read", and the read itself
            // says which (a zero-length answer is the stream's end).
            return 0;
        }
        if let Some(real) = self.real.get(&slot) {
            return real.is_readable();
        }
        let Some(record) = self.get(slot) else {
            return ESHUTDOWN;
        };
        if record.protocol == 1 {
            if record.flags & 2 == 0 {
                return EINVAL;
            }
            return match &record.pipe {
                Datagrams { queue } if !queue.is_empty() => 0,
                _ => EWOULDBLOCK,
            };
        }
        if record.flags & 4 != 0 {
            return match &record.pipe {
                Accept { waiting, .. } if !waiting.is_empty() => 0,
                _ => EWOULDBLOCK,
            };
        }
        let Ring { held, shut } = &record.pipe else {
            return ENOTCONN;
        };
        if shut & 1 != 0 {
            return ESHUTDOWN;
        }
        if !held.is_empty() {
            return 0;
        }
        let Some(peer) = record.peer else {
            return 0;
        };
        match self.get(peer).map(|one| &one.pipe) {
            Some(Ring { shut, .. }) if shut & 2 != 0 => 0,
            _ => EWOULDBLOCK,
        }
    }

    pub fn is_writeable(&mut self, slot: usize) -> u32 {
        if self.script.contains_key(&slot) {
            // `Write` accepts and discards, so there is never a reason to wait.
            return 0;
        }
        if let Some(real) = self.real.get_mut(&slot) {
            return real.is_writeable();
        }
        let Some(record) = self.get(slot) else {
            return ECONNRESET;
        };
        if record.protocol == 1 {
            return if record.flags & 2 != 0 { 0 } else { EINVAL };
        }
        if record.flags & 4 != 0 {
            return ENOTCONN;
        }
        let Ring { shut, .. } = &record.pipe else {
            return ENOTCONN;
        };
        if shut & 2 != 0 {
            return ESHUTDOWN;
        }
        let Some(peer) = record.peer else {
            return ECONNRESET;
        };
        match self.get(peer).map(|one| &one.pipe) {
            Some(Ring { held, .. }) if held.len() >= RING => EWOULDBLOCK,
            Some(Ring { .. }) => 0,
            _ => ECONNRESET,
        }
    }

    fn get(&self, slot: usize) -> Option<&Record> {
        self.slots.get(slot).and_then(|one| one.as_ref())
    }

    pub fn read_into(&mut self, slot: usize, want: usize, buffer_ok: bool) -> (Done, Vec<u8>) {
        if self.script.contains_key(&slot) {
            if !buffer_ok {
                self.touch(slot, EFAULT);
                return (Done::refused(EFAULT), Vec::new());
            }
            let Some(conn) = self.script.get_mut(&slot) else {
                return (Done::refused(EBADF), Vec::new());
            };
            let room = want.min(conn.reply.len() - conn.sent);
            let bytes = conn.reply[conn.sent..conn.sent + room].to_vec();
            conn.sent += room;
            self.touch(slot, 0);
            self.count_down(slot, bytes.len() as u64);
            let mut done = Done::ok();
            done.ret = bytes.len() as i32;
            return (done, bytes);
        }
        if self.real.contains_key(&slot) {
            return self.real_read(slot, want, buffer_ok);
        }
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), Vec::new());
        }
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return (Done::refused(EBADF), Vec::new());
        }
        if record.protocol != 0 {
            self.touch(slot, EOPNOTSUPP);
            return (Done::refused(EOPNOTSUPP), Vec::new());
        }
        let blocked = self.is_readable(slot);
        if blocked != 0 {
            self.touch(slot, blocked);
            return (Done::refused(blocked), Vec::new());
        }
        let mut record = self.take(slot);
        let Ring { held, .. } = &mut record.pipe else {
            unreachable!("is_readable answered 0 only for a non-empty ring");
        };
        let taken = want.min(held.len()).min(RING);
        let bytes = held.drain(..taken).collect::<Vec<_>>();
        let after = held.len();
        let peer = record.peer;
        record.last_error = 0;
        self.put(slot, record);
        let mut done = Done {
            ret: bytes.len() as i32,
            error: Some(0),
            fires: Vec::new(),
            released: Vec::new(),
        };
        // `if (peer != 0 && held_now + taken == 512) EventOccurred(peer, WRITE)`: the reference condition
        // is about the ring having been **full** when this read ran, not about it being empty now — a
        // partial drain of a full ring still wakes the writer, and a full drain of a short one does not.
        if let Some(peer) = peer {
            if after + bytes.len() == RING {
                done = done.fired(peer, EVENT_WRITE);
            }
        }
        // Counted here rather than in the runtime so a loopback row and a host row are the same kind
        // of number: what crossed *this* socket.
        self.count_down(slot, bytes.len() as u64);
        (done, bytes)
    }

    pub fn write_from(&mut self, slot: usize, data: &[u8], buffer_ok: bool) -> Done {
        if self.script.contains_key(&slot) {
            if !buffer_ok {
                self.touch(slot, EFAULT);
                return Done::refused(EFAULT);
            }
            // A script has no peer to read these; accepting them is the whole contract, so the
            // caller's own protocol keeps running instead of blocking on a write nobody drains.
            self.touch(slot, 0);
            self.count_up(slot, data.len() as u64);
            let mut done = Done::ok();
            done.ret = data.len() as i32;
            return done;
        }
        if self.real.contains_key(&slot) {
            return self.real_write(slot, data, buffer_ok);
        }
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return Done::refused(EFAULT);
        }
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Done::refused(EBADF);
        }
        if record.protocol != 0 {
            self.touch(slot, EOPNOTSUPP);
            return Done::refused(EOPNOTSUPP);
        }
        let blocked = self.is_writeable(slot);
        if blocked != 0 {
            self.touch(slot, blocked);
            return Done::refused(blocked);
        }
        let peer = record.peer.expect("is_writeable required a ring");
        let mut target = self.take(peer);
        let Ring { held, .. } = &mut target.pipe else {
            self.put(peer, target);
            self.touch(slot, ECONNRESET);
            return Done::refused(ECONNRESET);
        };
        let room = RING - held.len();
        let given = data.len().min(room);
        let was_empty = held.is_empty();
        held.extend_from_slice(&data[..given]);
        self.put(peer, target);
        self.touch(slot, 0);
        let mut done = Done {
            ret: given as i32,
            error: Some(0),
            fires: Vec::new(),
            released: Vec::new(),
        };
        // The body's test is `if (held_after_this_call == bytes_written)`, true exactly when the ring
        // started empty and this call wrote everything it was given.
        if let Some(Ring { held, .. }) = self.get(peer).map(|one| &one.pipe) {
            if held.len() == given && was_empty {
                done = done.fired(peer, EVENT_READ);
            }
        }
        self.count_up(slot, given as u64);
        done
    }

    pub fn send_to(
        &mut self,
        slot: usize,
        payload: &[u8],
        buffer_ok: bool,
        family: u16,
        port: u16,
        addr: u32,
    ) -> Send {
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return Send::Answer(Done::refused(EFAULT));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(real) = self.real.get_mut(&slot) {
            if !real.dgram {
                real.error = EOPNOTSUPP;
                return Send::Answer(Done::refused(EOPNOTSUPP));
            }
            let target = SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::from(addr.to_le_bytes())),
                port,
            );
            let done = match real.socket.send_to(payload, &target.into()) {
                Ok(n) => {
                    real.error = 0;
                    self.count_up(slot, n as u64);
                    let mut done = Done::ok();
                    done.ret = n as i32;
                    done
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    real.error = EWOULDBLOCK;
                    Done::blocked(EWOULDBLOCK)
                }
                Err(_) => {
                    real.error = ECONNRESET;
                    Done::refused(ECONNRESET)
                }
            };
            let _ = family;
            return Send::Answer(done);
        }
        let mine = self.record(slot).expect("live");
        if mine.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Send::Answer(Done::refused(EBADF));
        }
        if mine.protocol != 1 {
            self.touch(slot, EOPNOTSUPP);
            return Send::Answer(Done::refused(EOPNOTSUPP));
        }
        let blocked = self.is_writeable(slot);
        if blocked != 0 {
            self.touch(slot, blocked);
            return Send::Answer(Done::refused(blocked));
        }
        let target = self.walk().find_map(|(other, one)| {
            (one.protocol == mine.protocol
                && one.family == mine.family
                && family != FAMILY_LOCAL
                && one.addr == addr
                && one.port == port)
                .then_some(other)
        });
        let record = Datagram {
            payload: payload.to_vec(),
            from_family: mine.family,
            from_port: mine.port,
            from_addr: mine.addr,
        };
        let Some(target) = target else {
            self.touch(slot, 0);
            return Send::Dropped {
                done: Done {
                    ret: payload.len() as i32,
                    error: Some(0),
                    fires: Vec::new(),
                    released: Vec::new(),
                },
            };
        };
        let mut receiver = self.take(target);
        let stored = match &mut receiver.pipe {
            Datagrams { queue } if queue.len() < DATAGRAMS => {
                let empty = queue.is_empty();
                queue.push(record);
                Some(empty)
            }
            _ => None,
        };
        self.put(target, receiver);
        self.touch(slot, 0);
        let done = Done {
            ret: payload.len() as i32,
            error: Some(0),
            fires: Vec::new(),
            released: Vec::new(),
        };
        match stored {
            // `if (count == 1) EventOccurred(receiver, AEE_NET_READ_EVENT)` — the receiver is woken on
            // the first datagram of a burst, never again until its queue drains.
            // A dropped datagram carried nothing to its destination, so only these two count.
            Some(true) => {
                self.count_up(slot, payload.len() as u64);
                Send::Delivered(done.fired(target, EVENT_READ))
            }
            Some(false) => {
                self.count_up(slot, payload.len() as u64);
                Send::Delivered(done)
            }
            None => Send::Dropped { done },
        }
    }

    pub fn recv_from(
        &mut self,
        slot: usize,
        want: usize,
        buffer_ok: bool,
        size_at_least_8: bool,
        addr_ok: bool,
    ) -> (Done, Option<Datagram>) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(real) = self.real.get_mut(&slot) {
            if !real.dgram {
                real.error = EOPNOTSUPP;
                return (Done::refused(EOPNOTSUPP), None);
            }
            // The loopback body pops the datagram and *then* refuses a bad size or from-block. A real
            // socket cannot put a datagram back, so the same gates are asked before the read: the
            // caller gets the same errno, and loses nothing it could have kept anyway.
            if !addr_ok || !size_at_least_8 {
                real.error = EFAULT;
                return (Done::refused(EFAULT), None);
            }
            if want == 0 {
                return (Done::ok(), None);
            }
            let mut buf = vec![0u8; want.min(65536)];
            match real.socket.recv_from(unfilled(&mut buf)) {
                Ok((n, from)) => {
                    buf.truncate(n);
                    real.error = 0;
                    let (from_family, from_port, from_addr) = match from.as_socket() {
                        // The model stores the address word the way a little-endian read of the four
                        // network-order bytes sees it -- the shape every other `from_addr` in this file
                        // has, and the one `GetPeerName` writes back out.
                        Some(SocketAddr::V4(v4)) => {
                            (FAMILY_INET, v4.port(), u32::from_le_bytes(v4.ip().octets()))
                        }
                        // Only IPv4 sockets are opened here (`open_ex` builds an `IPV4` domain),
                        // so a v6 sender cannot occur -- and if one ever did, inventing 0.0.0.0:0
                        // would be worse than saying the family is not supported.
                        Some(SocketAddr::V6(_)) | None => {
                            real.error = EAFNOSUPPORT;
                            return (Done::refused(EAFNOSUPPORT), None);
                        }
                    };
                    // Counted after the borrow ends: `count_down` touches the same map that holds
                    // `real`, and the socket's own fields are done with by now.
                    self.count_down(slot, n as u64);
                    let mut done = Done::ok();
                    done.ret = n as i32;
                    return (
                        done,
                        Some(Datagram {
                            payload: buf,
                            from_family,
                            from_port,
                            from_addr,
                        }),
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    real.error = EWOULDBLOCK;
                    return (Done::blocked(EWOULDBLOCK), None);
                }
                Err(_) => {
                    real.error = ECONNRESET;
                    return (Done::refused(ECONNRESET), None);
                }
            }
        }
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), None);
        }
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return (Done::refused(EBADF), None);
        }
        if record.protocol != 1 {
            self.touch(slot, EOPNOTSUPP);
            return (Done::refused(EOPNOTSUPP), None);
        }
        if record.flags & 2 == 0 {
            self.touch(slot, EINVAL);
            return (Done::refused(EINVAL), None);
        }
        let blocked = self.is_readable(slot);
        if blocked != 0 {
            self.touch(slot, blocked);
            return (Done::refused(blocked), None);
        }
        let mut record = self.take(slot);
        let Datagrams { queue } = &mut record.pipe else {
            self.put(slot, record);
            self.touch(slot, EWOULDBLOCK);
            return (Done::refused(EWOULDBLOCK), None);
        };
        let mut message = queue.remove(0);
        record.last_error = if message.payload.len() > want {
            message.payload.truncate(want);
            EMSGSIZE
        } else {
            0
        };
        let error = record.last_error;
        let from_local = message.from_family == FAMILY_LOCAL;
        self.put(slot, record);
        if !from_local && (!size_at_least_8 || !addr_ok) {
            // The body has already `aee_memmove`d the payload into the caller's buffer and freed the
            // queue record **before** it tests the from-address block, so a bad `pFrom` costs the message:
            // the bytes are delivered, `*pnSize`/`pFrom` are not written, and the answer is -1 with 515.
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), Some(message));
        }
        (
            Done {
                ret: message.payload.len() as i32,
                error: Some(error),
                fires: Vec::new(),
                released: Vec::new(),
            },
            Some(message),
        )
    }

    pub fn shutdown(&mut self, slot: usize, how: i32) -> Done {
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Done::refused(EBADF);
        }
        if record.protocol != 0 {
            self.touch(slot, EOPNOTSUPP);
            return Done::refused(EOPNOTSUPP);
        }
        if !matches!(how, 0 | 1 | 2) {
            self.touch(slot, EINVAL);
            return Done::refused(EINVAL);
        }
        let Pipe::Ring { shut, .. } = &record.pipe else {
            self.touch(slot, ENOTCONN);
            return Done::refused(ENOTCONN);
        };
        if shut & (how as u32 + 1) != 0 {
            return Done::bare(ESHUTDOWN as i32);
        }
        let peer = match record.peer {
            Some(peer) => peer,
            None => {
                self.touch(slot, ENOTCONN);
                return Done::refused(ENOTCONN);
            }
        };
        let mut record = self.take(slot);
        if let Ring { shut, .. } = &mut record.pipe {
            *shut |= how as u32 + 1;
        }
        let read_shut = match &record.pipe {
            Ring { shut, .. } => shut & 1 != 0,
            _ => false,
        };
        self.put(slot, record);
        let done = Done::ok();
        // `if (channel[+520] & 1) EventOccurred(peer, AEE_NET_READ_EVENT)` — half-closing the read side
        // is how the other end learns its bytes will never be drained.
        if read_shut {
            return done.fired(peer, EVENT_READ);
        }
        done
    }

    pub fn close(&mut self, slot: usize) -> Done {
        // The host connection goes first: the record path below is what the ABI sees, and it must not
        // report a socket as closed while the descriptor is still open.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(real) = self.real.remove(&slot) {
            let _ = real.socket.shutdown(std::net::Shutdown::Both);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.real.remove(&slot);
        }
        let mut record = self.take(slot);
        if record.flags & 1 == 0 {
            record.last_error = EBADF;
            self.put(slot, record);
            return Done::refused(EBADF);
        }
        let queued = match &record.pipe {
            Accept { waiting, .. } => waiting.clone(),
            _ => Vec::new(),
        };
        let listening = record.flags & 4 != 0;
        let peer = record.pipe_peer(record.protocol);
        record.pipe = Pipe::None;
        record.peer = None;
        record.flags &= !3;
        record.readable.clear();
        record.writeable.clear();
        self.put(slot, record);
        let mut done = Done::ok();
        if listening {
            // `for (n = 0; n < count; n++) IHostSocket_Release(queue[n])` — one reference off each
            // queued socket. The runtime finishes the job; this reports which ones.
            done.released = queued;
        } else if let Some(peer) = peer {
            let mut other = self.take(peer);
            other.peer = None;
            self.put(peer, other);
            done = done.fired(peer, EVENT_READ | EVENT_WRITE);
        }
        done
    }

    /// Take a chain out of the record, in the order the reference `EventOccurred` walks it.
    ///
    /// ⚠ **This is where the host deliberately stops following the body.** The reference walk is
    /// `for (i = head; i != 0; i = head)` — it re-reads the head **after** each resume, so a callback whose
    /// own notify re-arms it is fired again by the same event, and one that re-arms unconditionally never
    /// leaves the loop. Draining first means one event fires each parked callback once. Kept as a
    /// deviation rather than a transcription because the spin the body allows would hang the host process,
    /// not just the guest. The cancel-before-resume half **is** copied: the chain is emptied before anything
    /// runs, so a callback parked twice appears once.
    pub fn drain_chain(&mut self, slot: usize, read: bool) -> Vec<usize> {
        let mut record = self.take(slot);
        let chain = if read {
            std::mem::take(&mut record.readable)
        } else {
            std::mem::take(&mut record.writeable)
        };
        self.put(slot, record);
        chain
    }

    pub fn cancel_entry(&mut self, slot: usize, entry: usize) {
        if let Some(Some(record)) = self.slots.get_mut(slot) {
            record.readable.retain(|one| *one != entry);
            record.writeable.retain(|one| *one != entry);
        }
    }

    /// Whether a block is armed on either list — the same question the reference hook answers by reading
    /// `block[+8]`, which is only an address once a registration has written one.
    ///
    /// A block lives on at most one list, because [`Manager::arm`] cancels before it re-arms; the reference
    /// hook unlinks the chain named inside the block, and this host keeps the membership on its own side,
    /// so the question has to be asked of both lists.
    pub fn armed(&self, slot: usize, entry: usize) -> bool {
        self.slots
            .get(slot)
            .and_then(|one| one.as_ref())
            .map(|record| record.readable.contains(&entry) || record.writeable.contains(&entry))
            .unwrap_or(false)
    }

    pub fn last_error(&self, slot: usize) -> u32 {
        if let Some(real) = self.real.get(&slot) {
            return real.error;
        }
        self.get(slot).map(|one| one.last_error).unwrap_or(0)
    }

    /// Open a real TCP connection behind `slot` and record the peer on the model's own fields.
    ///
    /// Non-blocking throughout: `connect` on a non-blocking socket answers `EINPROGRESS` (here
    /// `WouldBlock`), which is the state `ISocket::Connect` is built to report as "in progress" and
    /// later confirm through `Writeable`. The record's family, address and port are written from the
    /// address actually asked for, so `GetPeerName` and `GetSockName` have something true to say.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn real_tcp_connect(&mut self, slot: usize, addr: SocketAddr) -> Done {
        let snapshot = self.record(slot).expect("live");
        if snapshot.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return Done::refused(EBADF);
        }
        let domain = if addr.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        };
        let socket =
            match socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))
            {
                Ok(socket) => socket,
                Err(_) => {
                    self.touch(slot, ENOMEM);
                    return Done::refused(ENOMEM);
                }
            };
        if socket.set_nonblocking(true).is_err() {
            self.touch(slot, EBADF);
            return Done::refused(EBADF);
        }
        match socket.connect(&addr.into()) {
            Ok(()) => {}
            // A non-blocking connect in flight is the success shape here, not a failure.
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                self.touch(slot, ECONNREFUSED);
                return Done::refused(ECONNREFUSED);
            }
        }
        let mut record = self.take(slot);
        record.family = FAMILY_INET;
        record.addr = match addr.ip() {
            std::net::IpAddr::V4(v4) => u32::from_be_bytes(v4.octets()),
            std::net::IpAddr::V6(_) => 0,
        };
        record.port = addr.port();
        record.flags |= 1 | 2;
        record.last_error = 0;
        self.put(slot, record);
        self.real.insert(
            slot,
            Real {
                socket,
                error: 0,
                dgram: false,
            },
        );
        Done::ok()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn real_tcp_connect(&mut self, slot: usize, _addr: SocketAddr) -> Done {
        self.touch(slot, ENETNONET);
        Done::refused(ENETNONET)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn real_read(&mut self, slot: usize, want: usize, buffer_ok: bool) -> (Done, Vec<u8>) {
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), Vec::new());
        }
        // A caller that asks for nothing gets nothing: reading one byte into a one-byte buffer would
        // answer a byte-count to a question whose answer is zero, and would consume it.
        if want == 0 {
            return (Done::ok(), Vec::new());
        }
        let Some(real) = self.real.get_mut(&slot) else {
            return (Done::refused(EBADF), Vec::new());
        };
        let mut buf = vec![0u8; want.min(65536)];
        match real.socket.read(&mut buf) {
            // A stream read of zero is the peer's close, not an empty packet.
            Ok(0) => {
                real.error = ECONNRESET;
                (Done::refused(ECONNRESET), Vec::new())
            }
            Ok(n) => {
                buf.truncate(n);
                real.error = 0;
                self.count_down(slot, n as u64);
                (Done::ok(), buf)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                real.error = EWOULDBLOCK;
                (Done::blocked(EWOULDBLOCK), Vec::new())
            }
            Err(_) => {
                real.error = ECONNRESET;
                (Done::refused(ECONNRESET), Vec::new())
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn real_write(&mut self, slot: usize, data: &[u8], buffer_ok: bool) -> Done {
        if !buffer_ok {
            self.touch(slot, EFAULT);
            return Done::refused(EFAULT);
        }
        let Some(real) = self.real.get_mut(&slot) else {
            return Done::refused(EBADF);
        };
        match real.socket.write(data) {
            Ok(n) => {
                real.error = 0;
                self.count_up(slot, n as u64);
                let mut done = Done::ok();
                done.ret = n as i32;
                done
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                real.error = EWOULDBLOCK;
                Done::blocked(EWOULDBLOCK)
            }
            Err(_) => {
                real.error = ECONNRESET;
                Done::refused(ECONNRESET)
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn real_read(&mut self, slot: usize, _want: usize, _buffer_ok: bool) -> (Done, Vec<u8>) {
        self.touch(slot, ENETNONET);
        (Done::refused(ENETNONET), Vec::new())
    }

    #[cfg(target_arch = "wasm32")]
    fn real_write(&mut self, slot: usize, _data: &[u8], _buffer_ok: bool) -> Done {
        self.touch(slot, ENETNONET);
        Done::refused(ENETNONET)
    }

    /// Whether a slot answers its bytes from a host connection rather than from a ring.
    pub fn is_real(&self, slot: usize) -> bool {
        self.real.contains_key(&slot)
    }

    /// The parked chains whose real socket is ready now, drained and handed back.
    ///
    /// The loopback model fires its chains where the bytes move, because it *is* the thing moving
    /// them. A real socket has nothing of the sort: readiness is the kernel's answer, so it is asked
    /// once per turn and the chains that are now satisfied come out here. A chain parked on a socket
    /// that is not ready stays where it is, which is the whole point of parking it.
    pub fn poll_ready(&mut self) -> Vec<usize> {
        let slots: Vec<usize> = self.real.keys().copied().collect();
        let mut out = Vec::new();
        for slot in slots {
            if !self.real.contains_key(&slot) {
                continue;
            }
            // Read first, then write: the same order `EventOccurred` walks the two bits in.
            if self.is_readable(slot) == 0 {
                out.extend(self.drain_chain(slot, true));
            }
            // A refused connect is "ready" in the only sense the caller cares about: it has an answer.
            let write = self.is_writeable(slot);
            if write == 0 || write == ECONNREFUSED {
                out.extend(self.drain_chain(slot, false));
            }
        }
        out
    }

    pub fn policy(&self) -> Policy {
        self.policy
    }

    pub fn set_policy(&mut self, policy: Policy) {
        self.policy = policy;
    }

    /// Hand the socket layer the `SOCKET` lines of a `--net-script` file.
    ///
    /// It is a separate knob from [`Manager::set_policy`] because the two answer different things: the
    /// policy says where an *unscripted* connect goes, and this table says which connects never leave
    /// the process at all. A file with no `SOCKET` lines leaves every connect on the policy's path.
    pub fn set_script_routes(&mut self, routes: ScriptRoutes) {
        self.routes = routes;
    }

    pub fn traffic(&self) -> Traffic {
        self.traffic
    }

    fn count_up(&mut self, slot: usize, n: u64) {
        self.traffic.up += n;
        self.bytes.entry(slot).or_insert((0, 0)).0 += n;
    }

    fn count_down(&mut self, slot: usize, n: u64) {
        self.traffic.down += n;
        self.bytes.entry(slot).or_insert((0, 0)).1 += n;
    }

    /// Zero the totals and every row's counters. The connections stay: this is a monitor's reset, not
    /// a close.
    pub fn clear_traffic(&mut self, at_ms: u64) {
        self.traffic = Traffic::default();
        self.bytes.clear();
        self.cleared_at = Some(at_ms);
    }

    /// The totals plus one row per live connection.
    ///
    /// A value rather than a borrow because the window that draws it is on another thread and may not
    /// touch the machine: the worker publishes this once a turn, the way it publishes the flame.
    pub fn snapshot(&self) -> Report {
        let mut rows: Vec<Row> = self
            .order
            .iter()
            .filter_map(|slot| {
                let record = self.record(*slot)?;
                let (up, down) = self.bytes.get(slot).copied().unwrap_or((0, 0));
                Some(Row {
                    port: record.port,
                    addr: record.addr,
                    dgram: record.protocol == 1,
                    up,
                    down,
                })
            })
            .collect();
        rows.sort_by_key(|row| (row.addr, row.port, row.dgram));
        Report {
            traffic: self.traffic,
            rows,
            cleared_at: self.cleared_at,
        }
    }

    /// Where a guest's `AF_INET` connect really goes, and what it costs to say so.
    ///
    /// The record keeps the address the guest asked for even when a proxy carried it, so
    /// `GetPeerName` still describes the peer the pack named; `traffic.connects` counts the
    /// connection, which is the fact the manager's page reports.
    /// A connect a `--net-script` line answers: nothing leaves the process, the record names the peer
    /// it asked for, and the reply is handed to the reader through [`ScriptConn`].
    fn connect_script(
        &mut self,
        slot: usize,
        port: u16,
        addr: u32,
        reply: Vec<u8>,
    ) -> Result<Connected, Done> {
        let mut record = self.take(slot);
        record.family = FAMILY_INET;
        record.addr = addr;
        record.port = port;
        record.flags |= 1 | 2;
        record.last_error = 0;
        self.put(slot, record);
        self.script.insert(slot, ScriptConn { reply, sent: 0 });
        self.traffic.connects += 1;
        Ok(Connected {
            // A script has no server end on the loopback model.
            server: usize::MAX,
            done: Done::ok(),
        })
    }

    fn connect_real(&mut self, slot: usize, port: u16, addr: u32) -> Result<Connected, Done> {
        let target = self.policy.proxy.unwrap_or_else(|| {
            SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::from(addr.to_le_bytes())),
                port,
            )
        });
        let done = self.real_tcp_connect(slot, target);
        if done.ret < 0 {
            return Err(done);
        }
        let mut record = self.take(slot);
        record.family = FAMILY_INET;
        record.addr = addr;
        record.port = port;
        self.put(slot, record);
        self.traffic.connects += 1;
        Ok(Connected {
            // A real connection has no loopback server end to name.
            server: usize::MAX,
            done,
        })
    }

    pub fn get_opt(&mut self, slot: usize) -> Done {
        self.touch(slot, ENOPROTOOPT);
        Done::refused(ENOPROTOOPT)
    }

    pub fn set_opt(&mut self, slot: usize) -> Done {
        self.touch(slot, ENOPROTOOPT);
        Done::refused(ENOPROTOOPT)
    }

    pub fn ioctl(&mut self, slot: usize) -> Done {
        self.touch(slot, AEE_EUNSUPPORTED);
        Done::refused(AEE_EUNSUPPORTED)
    }

    pub const fn unnamed() -> Done {
        Done::bare(AEE_EUNSUPPORTED as i32)
    }

    pub fn readable(&mut self, slot: usize, cb: usize, cb_pfn: u32) -> Arm {
        self.arm(slot, cb, cb_pfn, true)
    }

    /// Park a **socket source** on a socket's readable chain.
    ///
    /// `SockSource_Readable` registers the runtime's own `SockSource_ReadableCB` with the socket — one
    /// registration for the whole source, however many callbacks are waiting behind it — so the entry
    /// that goes on the chain names the source object rather than a caller's callback block, and the
    /// notify slot carries [`RUNTIME_NOTIFY`] to say which kind it is. Everything else is the same
    /// gate order the caller-facing door uses, including `Arm::Resume`: when the bytes are already
    /// there, it is the *source* that has to be drained, not one block that gets resumed.
    pub fn readable_source(&mut self, slot: usize, source: usize) -> Arm {
        self.arm(slot, source, RUNTIME_NOTIFY, true)
    }

    /// Park a runtime-owned object on a socket's **writeable** chain — the same registration
    /// [`Manager::readable_source`] makes on the other one, for the direction `ISSL_Socket_Writeable`
    /// asks of the socket underneath.
    pub fn writeable_source(&mut self, slot: usize, source: usize) -> Arm {
        self.arm(slot, source, RUNTIME_NOTIFY, false)
    }

    pub fn writeable(&mut self, slot: usize, cb: usize, cb_pfn: u32) -> Arm {
        self.arm(slot, cb, cb_pfn, false)
    }

    fn arm(&mut self, slot: usize, cb: usize, cb_pfn: u32, read: bool) -> Arm {
        let snapshot = self.take(slot);
        // Gate order straight from the body: `CheckPtr` (the runtime), then the open bit, and only then
        // `pcb[+16]`. An unopened record answers 514 whatever the caller's block holds — the shape this
        // file keeps wrong unless it is read off the pseudocode.
        if snapshot.flags & 1 == 0 {
            let mut record = snapshot;
            record.last_error = EBADF;
            self.put(slot, record);
            return Arm::Refused(EBADF);
        }
        if cb_pfn == 0 {
            self.put(slot, snapshot);
            return Arm::Ignored;
        }
        self.put(slot, snapshot);
        let ready = if read {
            self.is_readable(slot) == 0
        } else {
            self.is_writeable(slot) == 0
        };
        if ready {
            return Arm::Resume;
        }
        let mut record = self.take(slot);
        record.readable.retain(|one| *one != cb);
        record.writeable.retain(|one| *one != cb);
        let chain = if read {
            &mut record.readable
        } else {
            &mut record.writeable
        };
        chain.insert(0, cb);
        self.put(slot, record);
        Arm::Parked
    }

    pub fn get_sock_name(&mut self, slot: usize, size: u32, addr_ok: bool) -> (Done, Option<Name>) {
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return (Done::refused(EBADF), None);
        }
        if record.flags & 2 == 0 {
            self.touch(slot, EINVAL);
            return (Done::refused(EINVAL), None);
        }
        if record.family == FAMILY_LOCAL {
            return (Done::ok(), None);
        }
        if size < 8 || !addr_ok {
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), None);
        }
        (
            Done::ok(),
            Some(Name {
                family: record.family,
                port: record.port,
                addr: record.addr,
            }),
        )
    }

    pub fn get_peer_name(&mut self, slot: usize, size: u32, addr_ok: bool) -> (Done, Option<Name>) {
        let record = self.record(slot).expect("live");
        if record.flags & 1 == 0 {
            self.touch(slot, EBADF);
            return (Done::refused(EBADF), None);
        }
        if record.protocol != 0 {
            self.touch(slot, EINVAL);
            return (Done::refused(EINVAL), None);
        }
        if !matches!(record.pipe, Ring { .. }) {
            // The body's first of two channel gates is on `[+196]` itself, not on the peer: a socket that
            // never connected has no ring, and that is `ENOTCONN`.
            self.touch(slot, ENOTCONN);
            return (Done::refused(ENOTCONN), None);
        }
        let Some(peer) = record.peer else {
            // The ring is still there but its peer pointer is gone — `Close` on the other side clears it
            // without tearing the ring down. That is the second gate, and it is a different number.
            self.touch(slot, ECONNRESET);
            return (Done::refused(ECONNRESET), None);
        };
        let Some(other) = self.get(peer) else {
            self.touch(slot, ECONNRESET);
            return (Done::refused(ECONNRESET), None);
        };
        let name = Name {
            family: other.family,
            port: other.port,
            addr: other.addr,
        };
        if name.family == FAMILY_LOCAL {
            return (Done::ok(), None);
        }
        if size < 8 || !addr_ok {
            self.touch(slot, EFAULT);
            return (Done::refused(EFAULT), None);
        }
        (Done::ok(), Some(name))
    }
}

impl Record {
    /// Which pointer `[+196]` stands for when `Close` decides what to release: a datagram socket's queue
    /// is freed by a different branch (`aee_free` per record) than a stream socket's peer notification,
    /// and the body picks by `[+176]`, not by the pipe's shape.
    fn pipe_peer(&self, protocol: u32) -> Option<usize> {
        if protocol == 1 {
            None
        } else {
            self.peer
        }
    }
}

/// One socket's 8-byte block, as the caller's `pAddr` receives it: `{family, port}` in the first word,
/// address in the second, with the size word set to 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Name {
    pub family: u16,
    pub port: u16,
    pub addr: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accept2 {
    /// The server-side socket to hand the caller. The body returns 0 from this branch without writing
    /// `[+180]`, so whatever the previous refusal left there is still what `GetLastError` reports.
    Taken(usize),
    /// -2 with `EWOULDBLOCK`, the reference empty-queue answer.
    Blocked(Done),
    /// -1 with the errno in the record.
    Refused(Done),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Send {
    /// Copied into a receiver's queue; its readable chain fires when the queue was empty.
    Delivered(Done),
    /// No receiver at all, or a queue already holding four: the payload is gone and the byte count is
    /// reported as sent.
    Dropped {
        done: Done,
    },
    Answer(Done),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arm {
    /// `pcb[+16]` was zero: the documented behaviour touches nothing at all.
    Ignored,
    /// The predicate already says yes, so the body resumes the callback instead of parking it.
    Resume,
    /// Pushed at the head of the chain.
    Parked,
    /// 514 or 515, returned as `r0` — not as -1.
    Refused(u32),
}

#[cfg(test)]
#[path = "test/socket.rs"]
mod tests;
