use crate::socket;

/// `[+168]`/`[+172]`/`[+176]`/`[+178]` of the record, in the offsets the bodies read them at.
#[derive(Debug, Default, Clone, Copy)]
pub struct Port {
    /// `[+168]`: the family `OpenEx` was handed, recorded before it is tested.
    pub family: u16,
    /// `[+176]`: `AEE_SOCKPORT_STREAM`(1) or `AEE_SOCKPORT_DGRAM`(2); `Accept`/`Connect`/`GetPeerName`
    /// refuse a non-zero value with `EOPNOTSUPP`.
    pub kind: u8,
    /// `[+178]`: the net errno every refusal writes here.
    pub error: u16,
    /// The `host_sockets` slot behind `[+172]` when the family is `AF_LOCAL`.
    pub host: Option<usize>,
    /// The guest address of the 112-byte `AF_INET` provider record behind `[+172]`.
    pub net: Option<u32>,
    /// `[+108]`: the net object `SelectNetwork` links. Only `AEENet_SelectNetwork` writes it, and this
    /// host has no net module, so it is 0 here and `TCPReady` answers 554 for every INET path.
    pub net_object: u32,
    /// `[+160]`/`[+164]`: the eight bytes `IOCtl` 4103 reads and 4104 writes, and nothing else in the
    /// member touches. `SOCKPORT_IOCTL_GET_SDB_ACK_INFO`(`4103`)/`SET`(`4104`) are the pair.
    pub ioctl_words: [u32; 2],
}

impl Port {
    pub fn is_local(&self) -> bool {
        self.family == socket::FAMILY_LOCAL && self.host.is_some()
    }

    pub fn is_inet(&self) -> bool {
        self.family == socket::FAMILY_INET && (self.net.is_some() || self.host.is_some())
    }

    /// `IS_OPEN`: `IS_LOCAL || IS_INET`. `Close` clears the delegate, so this is also the
    /// answer that turns false after one close and makes the second one `EBADF`.
    pub fn is_open(&self) -> bool {
        self.is_local() || self.is_inet()
    }

    /// `ISockPort_TCPReady` (39 bytes): no net object is `AEE_NET_ENETNONET`(554), a non-stream
    /// socket is `EINVAL`(538), and an INET socket whose provider is neither OEM- nor loopback-realized
    /// is `ENOTCONN`(528). With `[+108] == 0` the first test decides every call, which is why the
    /// `Read`/`Write`/`ReadV`/`WriteV` arms below answer 554 and stop.
    pub fn tcp_ready(&self) -> u32 {
        if self.net_object == 0 {
            return socket::ENETNONET;
        }
        if self.kind != 0 {
            return socket::EINVAL;
        }
        socket::ENOTCONN
    }

    /// [`Port::tcp_ready`] with the provider substitution: a port this host gave a provider has a
    /// socket where the body's `[+108]` would hold a net object, so the first test asks "is there a
    /// provider" instead, and the "unrealized" test is answered by the same fact.
    pub fn tcp_ready_with(&self, provider: bool) -> u32 {
        if self.net_object == 0 && !provider {
            return socket::ENETNONET;
        }
        if self.kind != 0 {
            return socket::EINVAL;
        }
        if !provider && self.inet_unrealized() {
            return socket::ENOTCONN;
        }
        0
    }

    /// `IS_INET_UNREALIZED`: `IS_INET_NONE` or an INET socket with no realized provider. The
    /// delivered object's `IS_INET_OEM*` bits come from the net module, so they are always clear here.
    /// Kept beside [`Port::tcp_ready`] because that body's third test is this predicate.
    pub fn inet_unrealized(&self) -> bool {
        self.is_inet() && self.net_object == 0
    }
}
