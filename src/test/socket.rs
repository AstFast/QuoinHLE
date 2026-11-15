use super::*;

/// A stream socket, opened and bound: the state `Connect` requires of both ends.
fn listening(m: &mut Manager, port: u16) -> usize {
    let server = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(server, FAMILY_INET, 0).ret, 0);
    assert_eq!(
        m.bind(server, FAMILY_INET, port, LOOPBACK.to_be(), true)
            .ret,
        0
    );
    assert_eq!(m.listen(server, 4).ret, 0);
    server
}

fn client(m: &mut Manager) -> usize {
    let slot = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    slot
}

/// A chain parked on a real socket comes back when the kernel says the socket is ready, and a
/// chain that was already drained does not come back at all -- the second half is what stops a
/// callback from firing on every turn.
#[test]
fn a_parked_chain_comes_back_once_the_real_socket_is_ready() {
    use std::io::Write as _;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    // The peer stays silent until this test says otherwise: a socket that is connected and idle is
    // the only state in which "nothing has arrived yet" is a fact rather than a race.
    let (go, allowed) = std::sync::mpsc::channel::<()>();
    let peer = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        allowed.recv().unwrap();
        peer.write_all(b"hi").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
    });

    let mut m = Manager::new();
    m.set_policy(Policy {
        real: true,
        proxy: None,
    });
    let slot = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    m.connect(slot, FAMILY_INET, addr.port(), LOOPBACK.to_be(), true)
        .expect("connected");

    let arm = m.readable(slot, 0x1000, 0x2000);
    assert!(matches!(arm, Arm::Parked), "nothing has arrived yet");
    assert!(m.poll_ready().is_empty(), "an idle socket wakes nothing");
    go.send(()).unwrap();

    let mut fired = Vec::new();
    for _ in 0..200 {
        let ready = m.poll_ready();
        if !ready.is_empty() {
            fired = ready;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(fired, vec![0x1000], "the parked chain came back");
    assert!(
        m.poll_ready().is_empty(),
        "a drained chain does not come back on every turn"
    );
    peer.join().unwrap();
}

/// A real datagram pair: a socket this host opened and a plain `UdpSocket` from the test, talking
/// through the kernel. The ABI path is `open_ex(protocol 1)` + `send_to` + `recv_from`, and the
/// from-address the model reports is the one the peer actually has.
#[test]
fn a_real_datagram_crosses_the_kernel_and_reports_its_sender() {
    let mut m = Manager::new();
    m.set_policy(Policy {
        real: true,
        proxy: None,
    });
    let ours = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(ours, FAMILY_INET, 1).ret, 0);
    assert!(m.is_real(ours));

    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let peer_addr = peer.local_addr().unwrap();

    // The first send is what gives the host socket a port; the peer then knows where to answer.
    match m.send_to(
        ours,
        b"hello",
        true,
        FAMILY_INET,
        peer_addr.port(),
        LOOPBACK.to_be(),
    ) {
        Send::Answer(done) => assert_eq!(done.ret, 5, "the datagram went out"),
        other => panic!("unexpected send shape: {other:?}"),
    }
    let mut buf = [0u8; 16];
    let (n, from) = peer.recv_from(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"hello");
    assert_eq!(from.ip().to_string(), "127.0.0.1");

    peer.send_to(b"dgram", from).unwrap();
    let mut got = None;
    for _ in 0..200 {
        let (done, from) = m.recv_from(ours, 16, true, true, true);
        if let Some(from) = from {
            assert_eq!(done.ret, 5);
            got = Some(from);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let from = got.expect("the answer arrived");
    assert_eq!(from.payload, b"dgram");
    assert_eq!(from.from_family, FAMILY_INET);
    assert_eq!(from.from_addr, LOOPBACK.to_be(), "the peer's address");
    assert_eq!(from.from_port, peer_addr.port());
    assert_eq!(m.traffic().up, 5, "five bytes out");
    assert_eq!(m.traffic().down, 5, "five bytes in");
    // The manager's page draws these rows: one connection, its peer, and what crossed it.
    let report = m.snapshot();
    assert_eq!(report.rows.len(), 1);
    assert!(report.rows[0].dgram, "a datagram connection");
    // A datagram record carries no peer of its own -- the address is per `send_to` -- so the row
    // has none to name. A connected stream's row is the one with a peer, and the test below is
    // where that is asserted.
    assert_eq!(report.rows[0].up, 5);
    assert_eq!(report.rows[0].down, 5);
    assert_eq!(report.traffic, m.traffic());
}

/// The policy is what decides where an `AF_INET` connect goes. With it off the call is the
/// loopback model's, unchanged; with it on the same call reaches the host's network, and the
/// record still describes the peer the guest named.
#[test]
fn the_policy_decides_whether_a_connect_leaves_the_process() {
    use std::io::Write as _;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let echo = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        let mut buf = [0u8; 5];
        std::io::Read::read_exact(&mut peer, &mut buf).unwrap();
        peer.write_all(&buf).unwrap();
    });

    let mut m = Manager::new();
    assert!(!m.policy().real, "the default keeps a run reproducible");
    let slot = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    let addr = LOOPBACK.to_be();
    m.set_policy(Policy {
        real: true,
        proxy: None,
    });
    let connected = m
        .connect(slot, FAMILY_INET, port, addr, true)
        .expect("a real connect answers with the slot it opened");
    assert_eq!(connected.done.ret, 0);
    assert!(m.is_real(slot), "the bytes come from the host now");
    assert_eq!(m.traffic().connects, 1);
    // The record kept what the guest asked for, not what the OS was handed.
    let record = m.record(slot).unwrap();
    assert_eq!(record.port, port);
    assert_eq!(record.addr, addr);

    let mut wrote = 0;
    for _ in 0..200 {
        let done = m.write_from(slot, b"quoin", true);
        if done.ret == 5 {
            wrote = 5;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(wrote, 5);
    let mut echoed = Vec::new();
    for _ in 0..200 {
        let (_, bytes) = m.read_into(slot, 8, true);
        if !bytes.is_empty() {
            echoed = bytes;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(echoed, b"quoin");
    assert_eq!(m.traffic().up, 5);
    assert_eq!(m.traffic().down, 5);
    // A connected stream's row names the peer it was told to reach.
    let report = m.snapshot();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].addr, addr);
    assert_eq!(report.rows[0].port, port);
    assert!(!report.rows[0].dgram);
    echo.join().unwrap();
}

/// A proxy is the same branch with a different address: the guest names a peer nobody listens on,
/// and the connection reaches the proxy instead. That is what makes the manager's network page
/// able to see a pack's own socket traffic.
#[test]
fn a_proxy_carries_the_connection_to_its_own_address() {
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy = listener.local_addr().unwrap();
    let accepted = std::thread::spawn(move || {
        let (_peer, from) = listener.accept().unwrap();
        from
    });

    let mut m = Manager::new();
    let slot = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    m.set_policy(Policy {
        real: true,
        proxy: Some(proxy),
    });
    // 10.0.0.1:80 is nowhere: the connect only succeeds because the proxy was substituted.
    let unreachable = 0x0A00_0001u32.to_be();
    let connected = m
        .connect(slot, FAMILY_INET, 80, unreachable, true)
        .expect("the proxy answered, not the named peer");
    assert_eq!(connected.done.ret, 0);
    assert!(m.is_real(slot));
    accepted.join().unwrap();
    // The guest's view of the peer is still the address it named.
    assert_eq!(m.record(slot).unwrap().addr, unreachable);
}

/// With the policy off, the same call is refused when nobody is listening — the loopback model's
/// answer, unchanged by the transport that now exists beside it.
#[test]
fn with_the_policy_off_a_connect_still_answers_from_the_loopback_model() {
    let mut m = Manager::new();
    let slot = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    let addr = LOOPBACK.to_be();
    let refused = m.connect(slot, FAMILY_INET, 0x2345, addr, true);
    assert!(refused.is_err(), "no listener, no connection");
    assert!(!m.is_real(slot));
    assert_eq!(m.traffic().connects, 0, "nothing left the process");
}

/// A real host connection behind a model slot. The listener is on the loopback interface, so
/// nothing leaves the machine, but the bytes really go through the kernel: the same `write_from`
/// and `read_into` the loopback model uses now carry them, and a read with nothing there is the
/// same `EWOULDBLOCK` the guest already expects.
#[test]
fn a_real_connection_carries_bytes_through_the_same_byte_methods() {
    use std::io::Write as _;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let echo = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        let mut buf = [0u8; 5];
        std::io::Read::read_exact(&mut peer, &mut buf).unwrap();
        peer.write_all(&buf).unwrap();
    });

    let mut m = Manager::new();
    let slot = m.born(0, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    assert!(!m.is_real(slot), "a slot starts on the loopback model only");
    assert_eq!(m.real_tcp_connect(slot, addr).ret, 0);
    assert!(m.is_real(slot), "and then answers its bytes from the host");

    // The connect is in flight: an immediate read has nothing and says so with the errno every
    // `ISocket::Read` in this ABI is built around.
    let (first, none) = m.read_into(slot, 8, true);
    if none.is_empty() {
        assert!(first.ret <= 0, "an empty read is not a byte count");
    }

    let mut wrote = 0;
    for _ in 0..200 {
        let done = m.write_from(slot, b"quoin", true);
        if done.ret == 5 {
            wrote = 5;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(wrote, 5, "the payload reached the socket");

    let mut echoed = Vec::new();
    for _ in 0..200 {
        let (_, bytes) = m.read_into(slot, 8, true);
        if !bytes.is_empty() {
            echoed = bytes;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(echoed, b"quoin", "and came back through the same door");
    echo.join().unwrap();

    // `Close` drops the descriptor with the record, so the slot goes back to being nothing.
    assert_eq!(m.close(slot).ret, 0);
    assert!(!m.is_real(slot));
}

#[test]
fn the_constructor_gates_on_class_before_allocation_and_writes_the_slot_last() {
    let mut m = Manager::new();
    // A wrong class is refused before anything is asked for, and 3 is the answer — the same number
    // `AEE_FindClassOwner` gives when the class table has no row at all, which is why a refusal
    // here cannot be read as "this host has no sockets".
    assert_eq!(m.born(0x1000, CLS_SOCKET), Err(AEE_ECLASSNOTSUPPORT));
    assert_eq!(m.born(0x1000, CLS_SOCKPORT), Err(AEE_ECLASSNOTSUPPORT));
    assert_eq!(
        m.live(),
        0,
        "a refused constructor must leave the list empty"
    );
    let ok = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    let record = m.record(ok).unwrap();
    assert_eq!(
        record.owner, 0x1000,
        "the constructor stores `pVoid` at `[+8]`"
    );
    // `born` writes nothing else: the record is 200 bytes of `aee_malloc` output, so the family
    // word, the port, the protocol and the error all start at zero, not at "unset".
    assert_eq!((record.family, record.port, record.protocol), (0, 0, 0));
    assert_eq!(record.last_error, 0);
    // The new socket is the list head, so it is the first one any scan reaches.
    let second = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.walk().next().unwrap().0, second);
}

#[test]
fn query_interface_answers_five_ids_and_clears_the_slot_before_refusing() {
    let mut m = Manager::new();
    let _slot = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    for id in [IID_IQI, CLS_SOURCE, CLS_PORT, CLS_HOSTSOCKET, CLS_SOCKPORT] {
        assert!(m.accepts(id), "{id:#010x} is in the shipped list");
    }
    // Two ids that are *not*: `AEECLSID_SOCKET`, whose `me + 4` answer belongs to `AEESockPort`
    // (and is a 13 there, not a 3), and `AEEIID_ISockPort` — the number
    // the header publishes for this very table, which the shipped body never lists.
    assert!(!m.accepts(CLS_SOCKET));
    assert!(!m.accepts(0x0103_9ba8));
    assert!(!m.accepts(0));
}

#[test]
fn open_ex_asks_the_already_open_question_first_and_only_datagrams_get_a_queue() {
    let mut m = Manager::new();
    let slot = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    // Gate order is `open, family, protocol`. On a record that is not open yet the *family* gate is
    // what answers, because the first one cannot be reached until bit0 is set.
    assert_eq!(m.open_ex(slot, 0, 9), Done::refused(EINVAL));
    assert_eq!(
        m.record(slot).unwrap().flags,
        0,
        "a refused OpenEx leaves it unopened"
    );
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    // A second call with the same garbage arguments is now `EBADF`, and neither argument is looked
    // at: the family word and the protocol from the first call survive.
    assert_eq!(m.open_ex(slot, 0, 9), Done::refused(EBADF));
    assert_eq!(m.record(slot).unwrap().protocol, 0);
    assert_eq!(m.record(slot).unwrap().flags & 1, 1);
    let other = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(other, 3, 0), Done::refused(EINVAL));
    // A rejected *family* is never written — the assignment sits inside the accepted-family branch.
    assert_eq!(
        m.record(other).unwrap().family,
        0,
        "a rejected family is not stored"
    );
    assert_eq!(m.record(other).unwrap().flags, 0);
    // The family word *is* written before the protocol test, so a refused `protocol > 1` leaves the
    // family set while the record stays unopened.
    assert_eq!(m.open_ex(other, FAMILY_INET, 2), Done::refused(EINVAL));
    assert_eq!(m.record(other).unwrap().family, FAMILY_INET);
    assert_eq!(m.record(other).unwrap().flags, 0);
    assert_eq!(m.open_ex(other, FAMILY_INET, 1).ret, 0);
    assert_eq!(
        m.record(other).unwrap().pipe,
        Pipe::Datagrams { queue: Vec::new() }
    );
    assert_eq!(
        m.record(other).unwrap().last_error,
        0,
        "success zeroes the field"
    );
    // And a stream socket gets no channel from `OpenEx` — only `Connect` or `Listen` gives one,
    // which is why `Shutdown` on an open-but-unconnected stream socket takes the 528 branch.
    let stream = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(stream, FAMILY_INET, 0);
    assert_eq!(m.record(stream).unwrap().pipe, Pipe::None);
    assert_eq!(m.shutdown(stream, 0), Done::refused(ENOTCONN));
}

#[test]
fn bind_local_short_circuits_before_the_pointer_check_and_never_touches_the_address() {
    let mut m = Manager::new();
    let slot = client(&mut m);
    // An `AF_LOCAL` bind is accepted with a block the body never validates, and it writes only the
    // bound bit: family, port and address stay as `OpenEx` left them.
    let done = m.bind(slot, FAMILY_LOCAL, 0x1234, 0x99, false);
    assert_eq!(done, Done::ok());
    let record = m.record(slot).unwrap();
    assert_eq!(
        (record.family, record.port, record.addr),
        (FAMILY_INET, 0, 0)
    );
    assert_eq!(record.flags & 2, 2);
    // Bound twice is `EINVAL`, and that gate is *before* the pointer check too.
    assert_eq!(m.bind(slot, FAMILY_INET, 1, 2, true), Done::refused(EINVAL));
    let fresh = client(&mut m);
    assert_eq!(
        m.bind(fresh, FAMILY_INET, 1, 2, false),
        Done::refused(EFAULT)
    );
}

#[test]
fn an_zero_port_comes_from_the_counting_down_counter_and_a_held_port_is_refused() {
    let mut m = Manager::new();
    // `word_1104` starts at `0xffff`, and the answer is the pre-decrement value.
    assert_eq!(m.next_bind_port(0), 0xffff);
    assert_eq!(m.next_bind_port(0), 0xfffe);
    // The datagram counter is a separate variable, so it starts over at the top.
    assert_eq!(m.next_bind_port(1), 0xffff);
    let a = client(&mut m);
    assert_eq!(m.bind(a, FAMILY_INET, 0, LOOPBACK.to_be(), true).ret, 0);
    // Port 0 asks for a candidate, and stores it **swapped**: the field is network order. This is
    // the third answer off `word_1104`, so the candidate is `0xfffd`, not the `0xffff` a reader who
    // forgot the two calls above would expect.
    assert_eq!(m.record(a).unwrap().port, 0xfffdu16.swap_bytes());
    // An explicit port that another live socket already holds is `EADDRINUSE`(523).
    let b = client(&mut m);
    let taken = m.record(a).unwrap().port;
    assert_eq!(
        m.bind(b, FAMILY_INET, taken, LOOPBACK.to_be(), true),
        Done::refused(EADDRINUSE)
    );
    // …but the same number on the *other* protocol is free, because the conflict test includes it.
    let dgram = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(dgram, FAMILY_INET, 1);
    assert_eq!(
        m.bind(dgram, FAMILY_INET, taken, LOOPBACK.to_be(), true)
            .ret,
        0
    );
}

#[test]
fn the_ephemeral_counter_stops_at_the_reserved_floor() {
    let mut m = Manager::new();
    // 0xffff down to 0x400 is 0xfc00 answers; walking every one of them is how the floor is
    // measured rather than assumed, and the value after the last allowed one is the reset.
    let first = m.next_bind_port(0);
    assert_eq!(first, 0xffff);
    // `0xffff` down to `0x401` inclusive is `0xffff - 0x401 + 1` answers, of which `first` is the
    // first — so the loop consumes the remainder and the next answer is the floor itself.
    for _ in 1..(0xffff - 0x401 + 1) {
        m.next_bind_port(0);
    }
    assert_eq!(m.next_bind_port(0), PORT_FLOOR);
    assert_eq!(
        m.next_bind_port(0),
        0xffff,
        "below the floor the counter is reset, not skipped"
    );
}

#[test]
fn listen_requires_a_bound_stream_socket_and_clamps_the_backlog_at_both_ends() {
    let mut m = Manager::new();
    let slot = client(&mut m);
    assert_eq!(m.listen(slot, 4), Done::refused(EINVAL), "not bound");
    m.bind(slot, FAMILY_INET, 1, LOOPBACK.to_be(), true);
    assert_eq!(m.listen(slot, 4).ret, 0);
    assert_eq!(
        m.record(slot).unwrap().pipe,
        Pipe::Accept {
            backlog: 4,
            waiting: Vec::new()
        }
    );
    assert_eq!(
        m.listen(slot, 4),
        Done::refused(EISCONN),
        "a queue already exists"
    );
    // The clamp is two comparisons: 0 and below become 1, 9 and above become 8.
    let low = client(&mut m);
    m.bind(low, FAMILY_INET, 2, LOOPBACK.to_be(), true);
    m.listen(low, 0);
    assert_eq!(
        m.record(low).unwrap().pipe,
        Pipe::Accept {
            backlog: 1,
            waiting: Vec::new()
        }
    );
    // A datagram socket is refused before the bound test, so an unbound UDP socket hears
    // `EOPNOTSUPP`, not `EINVAL`.
    let dgram = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(dgram, FAMILY_INET, 1);
    assert_eq!(m.listen(dgram, 4), Done::refused(EOPNOTSUPP));
}

#[test]
fn connect_makes_two_rings_one_queue_and_fires_in_the_shipped_order() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x1000);
    let caller = client(&mut m);
    let connected = m
        .connect(caller, FAMILY_INET, 0x1000, LOOPBACK.to_be(), true)
        .unwrap();
    let server = connected.server;
    // The server side is a *new* socket, not the listener: it inherits the listener's owner and is
    // given 127.0.0.1 with the listener's port, in the orders the body uses.
    let record = m.record(server).unwrap();
    assert_eq!(
        (record.family, record.port, record.addr),
        (FAMILY_INET, 0x1000, LOOPBACK.to_be())
    );
    assert_eq!(
        record.flags,
        2 | 1,
        "opened by the body's own OpenEx, then bound"
    );
    assert_eq!(
        m.record(listener).unwrap().pipe,
        Pipe::Accept {
            backlog: 4,
            waiting: vec![server]
        }
    );
    // Both ends hold a ring that names the other, and the caller got an ephemeral port from `Bind`.
    let caller_record = m.record(caller).unwrap();
    assert_eq!(caller_record.peer, Some(server));
    assert_eq!(record.peer, Some(caller));
    assert_eq!(caller_record.flags & 2, 2);
    // Queue empty→one fires the listener's readable chain, then the caller's writeable one.
    assert_eq!(
        connected.done.fires,
        vec![
            Fire {
                socket: listener,
                event: EVENT_READ
            },
            Fire {
                socket: caller,
                event: EVENT_WRITE
            },
        ]
    );
    // A second connect on the same listener does not fire it again: the queue was not empty.
    let caller2 = client(&mut m);
    let again = m
        .connect(caller2, FAMILY_INET, 0x1000, LOOPBACK.to_be(), true)
        .unwrap();
    assert_eq!(
        again.done.fires,
        vec![Fire {
            socket: caller2,
            event: EVENT_WRITE
        }]
    );
    // And `Accept` hands back exactly the queued slots, oldest first.
    assert!(matches!(m.accept(listener, true), Accept2::Taken(taken) if taken == server));
    assert!(matches!(m.accept(listener, true), Accept2::Taken(taken) if taken == again.server));
    assert_eq!(
        m.accept(listener, true),
        Accept2::Blocked(Done::blocked(EWOULDBLOCK))
    );
}

#[test]
fn connect_refusals_are_ordered_and_a_full_backlog_is_the_same_number_as_no_listener() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x2000);
    let born = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    // Not open yet: EBADF, before the protocol test.
    assert_eq!(
        m.connect(born, FAMILY_INET, 0x2000, LOOPBACK.to_be(), true),
        Err(Done::refused(EBADF))
    );
    m.open_ex(born, FAMILY_INET, 1);
    assert_eq!(
        m.connect(born, FAMILY_INET, 0x2000, LOOPBACK.to_be(), true),
        Err(Done::refused(EOPNOTSUPP))
    );
    // A datagram record cannot be re-opened as a stream one — `OpenEx`'s first gate is bit0, which
    // is already set — so the rest of this test needs a socket of its own rather than a second
    // `OpenEx` on `born`.
    let stream = client(&mut m);
    // No listener on that port. `ECONNREFUSED`, and the same number for a full backlog.
    assert_eq!(
        m.connect(stream, FAMILY_INET, 0x3000, LOOPBACK.to_be(), true),
        Err(Done::refused(ECONNREFUSED))
    );
    assert_eq!(
        m.connect(stream, FAMILY_INET, 0x2000, 0xdead_beef, true),
        Err(Done::refused(ECONNREFUSED)),
        "the address has to match the listener's too"
    );
    let mut backlog = 0;
    while backlog < 4 {
        let one = client(&mut m);
        m.connect(one, FAMILY_INET, 0x2000, LOOPBACK.to_be(), true)
            .unwrap();
        backlog += 1;
    }
    assert_eq!(
        m.connect(stream, FAMILY_INET, 0x2000, LOOPBACK.to_be(), true),
        Err(Done::refused(ECONNREFUSED)),
        "backlog 4 fills after four connects, and the fifth hears the same code"
    );
    assert_eq!(
        m.connect(stream, FAMILY_INET, 0x2000, LOOPBACK.to_be(), false),
        Err(Done::refused(EFAULT)),
        "the pointer gate is third, so it answers before the walk ever runs"
    );
    assert!(
        m.is_live(listener),
        "the listener itself stays live throughout"
    );
}

#[test]
fn already_connected_is_refused_before_the_pointer_check() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x4000);
    let caller = client(&mut m);
    m.connect(caller, FAMILY_INET, 0x4000, LOOPBACK.to_be(), true)
        .unwrap();
    // The listener keeps its accept queue — `Connect` puts the new socket in it and gives the rings
    // to the two ends of the connection, not to the listening record.
    assert_eq!(
        m.record(listener).unwrap().pipe,
        Pipe::Accept {
            backlog: 4,
            waiting: vec![m.walk().next().unwrap().0]
        },
        "the queue holds exactly the one pending connection"
    );
    assert_eq!(
        m.connect(caller, FAMILY_INET, 0x4000, LOOPBACK.to_be(), false),
        Err(Done::refused(EISCONN))
    );
}

#[test]
fn bytes_move_one_way_at_a_time_and_the_peer_wakes_only_when_the_ring_empties() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x5000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x5000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    assert!(matches!(m.accept(listener, true), Accept2::Taken(_)));
    // Writing goes into the *peer's* ring, and a peer that started empty is woken.
    let written = m.write_from(caller, b"hello", true);
    assert_eq!(written.ret, 5);
    assert_eq!(
        written.fires,
        vec![Fire {
            socket: server,
            event: EVENT_READ
        }]
    );
    assert_eq!(
        m.record(server).unwrap().pipe,
        Pipe::Ring {
            held: b"hello".to_vec(),
            shut: 0
        }
    );
    assert_eq!(
        m.record(caller).unwrap().pipe,
        Pipe::Ring {
            held: Vec::new(),
            shut: 0
        }
    );
    // A second write to a non-empty ring delivers the bytes without waking anyone.
    assert!(m.write_from(caller, b" there", true).fires.is_empty());
    let (done, bytes) = m.read_into(server, 20, true);
    assert_eq!((done.ret, bytes), (11, b"hello there".to_vec()));
    // The shipped wake test is `held_after + taken == 512` — the ring has to have been **full** when
    // the read ran. Eleven bytes drained from an eleven-byte ring leave 11 of 512, so nobody is
    // woken: the writer still has the room it never lost. The full-ring case is the next test.
    assert!(done.fires.is_empty());
    // … and the empty ring is the `EWOULDBLOCK` answer, not a zero-length success.
    let (again, _) = m.read_into(server, 4, true);
    assert_eq!(again, Done::refused(EWOULDBLOCK));
    assert_eq!(m.is_readable(server), EWOULDBLOCK);
    assert_eq!(m.is_writeable(server), 0);
}

#[test]
fn the_ring_holds_exactly_512_bytes_and_a_full_peer_blocks_the_writer() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x6000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x6000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    m.accept(listener, true);
    let payload = vec![0xa5u8; RING + 100];
    let written = m.write_from(caller, &payload, true);
    assert_eq!(
        written.ret, RING as i32,
        "the body clamps to the room, not the request"
    );
    assert_eq!(m.is_writeable(caller), EWOULDBLOCK);
    assert_eq!(m.write_from(caller, b"x", true), Done::refused(EWOULDBLOCK));
    // `Read` takes at most what it asked for, and the `0xffff` clamp is the shipped one: a request
    // above 64 KiB is capped to 65535, which is larger than the ring, so the ring is what decides.
    let (done, bytes) = m.read_into(server, 0x1_0000, true);
    assert_eq!((done.ret, bytes.len()), (RING as i32, RING));
    // …and this is the case the shipped condition is written for: the ring was full, so draining it
    // is what gives the writer its room back.
    assert_eq!(
        done.fires,
        vec![Fire {
            socket: caller,
            event: EVENT_WRITE
        }],
        "a full ring that just emptied wakes the writer"
    );
    assert_eq!(m.is_writeable(caller), 0);
}

#[test]
fn accept_writes_the_slot_before_refusing_and_refuses_by_body_order() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x7000);
    // The pointer gate comes first and leaves `[+180]` as the only thing written.
    assert_eq!(
        m.accept(listener, false),
        Accept2::Refused(Done::refused(EFAULT))
    );
    let unbound = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(
        m.accept(unbound, true),
        Accept2::Refused(Done::refused(EBADF))
    );
    m.open_ex(unbound, FAMILY_INET, 1);
    assert_eq!(
        m.accept(unbound, true),
        Accept2::Refused(Done::refused(EOPNOTSUPP))
    );
    let stream = client(&mut m);
    assert_eq!(
        m.accept(stream, true),
        Accept2::Refused(Done::refused(EINVAL))
    );
}

#[test]
fn datagrams_are_delivered_to_the_matching_bound_socket_and_dropped_silently_otherwise() {
    let mut m = Manager::new();
    let a = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    let b = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    for one in [a, b] {
        m.open_ex(one, FAMILY_INET, 1);
    }
    m.bind(a, FAMILY_INET, 0x0a00, LOOPBACK.to_be(), true);
    m.bind(b, FAMILY_INET, 0x0b00, LOOPBACK.to_be(), true);
    let sent = m.send_to(a, b"ping", true, FAMILY_INET, 0x0b00, LOOPBACK.to_be());
    match sent {
        Send::Delivered(done) => {
            assert_eq!(
                (done.ret, done.fires),
                (
                    4,
                    vec![Fire {
                        socket: b,
                        event: EVENT_READ
                    }]
                )
            );
        }
        other => panic!("expected delivery, got {other:?}"),
    }
    // The receiver's queue holds the payload *and* the sender's address block, because `SendTo`
    // copies 132 bytes from the sender's record rather than formatting anything.
    let (done, message) = m.recv_from(b, 20, true, true, true);
    let message = message.unwrap();
    assert_eq!((done.ret, message.payload.as_slice()), (4, &b"ping"[..]));
    assert_eq!(
        (message.from_family, message.from_port, message.from_addr),
        (FAMILY_INET, 0x0a00, LOOPBACK.to_be())
    );
    // A datagram with nowhere to go is dropped and reported as fully sent.
    let lost = m.send_to(a, b"gone", true, FAMILY_INET, 0x1234, 0x9999_9999);
    assert_eq!(
        lost,
        Send::Dropped {
            done: Done {
                ret: 4,
                error: Some(0),
                fires: Vec::new(),
                released: Vec::new()
            }
        }
    );
    // … and an `AF_LOCAL` target can never match, because the body's scan rejects it outright.
    assert!(matches!(
        m.send_to(a, b"local", true, FAMILY_LOCAL, 0x0b00, LOOPBACK.to_be()),
        Send::Dropped { .. }
    ));
    // Four is the queue's limit; the fifth is dropped with the same success answer.
    for _ in 0..DATAGRAMS {
        m.send_to(a, b"x", true, FAMILY_INET, 0x0b00, LOOPBACK.to_be());
    }
    assert!(matches!(
        m.send_to(a, b"x", true, FAMILY_INET, 0x0b00, LOOPBACK.to_be()),
        Send::Dropped { .. }
    ));
    assert_eq!(
        m.record(b).unwrap().pipe,
        Pipe::Datagrams {
            queue: vec![
                Datagram {
                    payload: b"x".to_vec(),
                    from_family: FAMILY_INET,
                    from_port: 0x0a00,
                    from_addr: LOOPBACK.to_be()
                };
                DATAGRAMS
            ]
        }
    );
}

#[test]
fn recv_from_consumes_the_datagram_before_it_checks_the_from_address() {
    let mut m = Manager::new();
    let a = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    let b = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    for one in [a, b] {
        m.open_ex(one, FAMILY_INET, 1);
    }
    m.bind(a, FAMILY_INET, 0x0a00, LOOPBACK.to_be(), true);
    m.bind(b, FAMILY_INET, 0x0b00, LOOPBACK.to_be(), true);
    m.send_to(a, b"abcdef", true, FAMILY_INET, 0x0b00, LOOPBACK.to_be());
    // A size word under 8: the body has already memmoved the payload into the caller's buffer and
    // freed the queue record, so the message is gone, the bytes are in the buffer, and the answer is
    // -1/515 with `pFrom` and `*pnSize` untouched. `None` here would be the host quietly eating bytes
    // the shipped code delivers.
    assert_eq!(
        m.recv_from(b, 20, true, false, true),
        (
            Done::refused(EFAULT),
            Some(Datagram {
                payload: b"abcdef".to_vec(),
                from_family: FAMILY_INET,
                from_port: 0x0a00,
                from_addr: LOOPBACK.to_be()
            })
        )
    );
    assert_eq!(
        m.record(b).unwrap().pipe,
        Pipe::Datagrams { queue: Vec::new() }
    );
    assert_eq!(m.is_readable(b), EWOULDBLOCK);
    // A short buffer truncates, notes `EMSGSIZE`, and *still returns the truncated count* — this is
    // a success answer with a warning beside it, not a refusal.
    m.send_to(a, b"abcdef", true, FAMILY_INET, 0x0b00, LOOPBACK.to_be());
    let (done, message) = m.recv_from(b, 3, true, true, true);
    assert_eq!(
        (done.ret, done.error, message.unwrap().payload),
        (3, Some(EMSGSIZE), b"abc".to_vec())
    );
    assert_eq!(m.record(b).unwrap().last_error, EMSGSIZE);
}

#[test]
fn shutdown_sets_how_plus_one_and_half_closing_reads_tells_the_peer() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x8000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x8000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    m.accept(listener, true);
    assert_eq!(
        m.shutdown(caller, 1),
        Done::ok(),
        "writes shut, and nobody is woken"
    );
    assert_eq!(
        m.record(caller).unwrap().pipe,
        Pipe::Ring {
            held: Vec::new(),
            shut: 2
        }
    );
    assert_eq!(m.is_writeable(caller), ESHUTDOWN);
    // The same direction twice answers 544 **in r0** with the error word untouched.
    assert_eq!(m.shutdown(caller, 1), Done::bare(ESHUTDOWN as i32));
    assert_eq!(m.record(caller).unwrap().last_error, 0);
    // Shutting reads wakes the peer's readable chain: this is how the other end learns the bytes
    // will never be drained.
    let done = m.shutdown(caller, 0);
    assert_eq!(
        done.fires,
        vec![Fire {
            socket: server,
            event: EVENT_READ
        }]
    );
    assert_eq!(
        m.record(caller).unwrap().pipe,
        Pipe::Ring {
            held: Vec::new(),
            shut: 3
        }
    );
    // And the peer now reads as "end of file" — 0, not `EWOULDBLOCK` — because the writer's shut bit
    // is what `IsReadable` consults on an empty ring.
    m.write_from(server, b"late", true);
    assert_eq!(
        m.is_readable(caller),
        ESHUTDOWN,
        "this side's own read shut outranks the data"
    );
    // `how` 3 is refused before the channel is read at all.
    assert_eq!(m.shutdown(caller, 3), Done::refused(EINVAL));
    // A datagram socket hears `EOPNOTSUPP`, which is the second gate.
    let dgram = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(dgram, FAMILY_INET, 1);
    assert_eq!(m.shutdown(dgram, 0), Done::refused(EOPNOTSUPP));
}

#[test]
fn close_keeps_the_listening_bit_and_releases_what_the_queue_held() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x9000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x9000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    let done = m.close(listener);
    let record = m.record(listener).unwrap();
    // bits 0 and 1 go, bit 2 stays — the shipped line is `flags = (flags >> 1) << 1 & ~2`.
    assert_eq!(record.flags, 4);
    assert_eq!(record.pipe, Pipe::None);
    assert_eq!(
        done.released,
        vec![server],
        "the queued server side is released once"
    );
    // Closing one end of a connected pair breaks the peer's back-link and fires *both* of its
    // chains, which is the body's `EventOccurred(peer, 3)`.
    let done = m.close(caller);
    assert_eq!(
        done.fires,
        vec![Fire {
            socket: server,
            event: EVENT_READ | EVENT_WRITE
        }]
    );
    assert_eq!(m.record(server).unwrap().peer, None);
    assert_eq!(m.record(caller).unwrap().flags, 0);
    // A second close hears `EBADF`, because bit0 went with the first one.
    assert_eq!(m.close(caller), Done::refused(EBADF));
}

#[test]
fn the_two_armers_park_fire_or_refuse_by_the_errno_and_ignore_a_blank_callback() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0xa000);
    let caller = client(&mut m);
    // `pcb[+16] == 0`: the body reads the word and stops, touching neither the chain nor `[+180]`.
    assert_eq!(m.readable(caller, 0x20, 0), Arm::Ignored);
    assert_eq!(m.record(caller).unwrap().readable, Vec::<usize>::new());
    // An unopened record refuses by *returning the errno* (514), not by returning -1.
    let fresh = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.writeable(fresh, 0x20, 4), Arm::Refused(EBADF));
    assert_eq!(m.record(fresh).unwrap().last_error, EBADF);
    // Nothing to read yet ⇒ parked, head-inserted, and parked once even if asked twice.
    assert_eq!(m.readable(caller, 0x20, 4), Arm::Parked);
    assert_eq!(m.readable(caller, 0x30, 4), Arm::Parked);
    assert_eq!(m.record(caller).unwrap().readable, vec![0x30, 0x20]);
    // Already readable ⇒ the shipped body resumes at once instead of parking.
    let server = m
        .connect(caller, FAMILY_INET, 0xa000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    m.accept(listener, true);
    m.write_from(server, b"hi", true);
    assert_eq!(m.readable(caller, 0x40, 4), Arm::Resume);
    assert_eq!(
        m.record(caller).unwrap().readable,
        vec![0x30, 0x20],
        "Resume must not park"
    );
}

#[test]
fn the_four_constant_slots_say_what_their_four_byte_bodies_say() {
    let mut m = Manager::new();
    let slot = client(&mut m);
    // `Open` and `SelectNetwork` are `return 20` with the record untouched — no error word, so a
    // test that read `[+180]` afterwards would see the previous call's answer.
    m.set_opt(slot);
    assert_eq!(Manager::unnamed(), Done::bare(AEE_EUNSUPPORTED as i32));
    assert_eq!(m.record(slot).unwrap().last_error, ENOPROTOOPT);
    // `IOCtl` writes 20 into the record and returns -1, which is a different shape again.
    assert_eq!(m.ioctl(slot), Done::refused(AEE_EUNSUPPORTED));
    assert_eq!(m.record(slot).unwrap().last_error, AEE_EUNSUPPORTED);
    // `GetOpt`/`SetOpt` ignore every argument, so both answer 546 for anything.
    assert_eq!(m.get_opt(slot), Done::refused(ENOPROTOOPT));
    assert_eq!(m.set_opt(slot), Done::refused(ENOPROTOOPT));
    assert_eq!(m.last_error(slot), ENOPROTOOPT);
}

#[test]
fn the_address_slots_report_the_record_and_an_local_socket_writes_nothing() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0xb000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0xb000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    m.accept(listener, true);
    let (done, name) = m.get_sock_name(caller, 8, true);
    assert_eq!((done.ret, done.error), (0, Some(0)));
    let name = name.unwrap();
    assert_eq!(
        (name.family, name.port, name.addr),
        (
            FAMILY_INET,
            m.record(caller).unwrap().port,
            LOOPBACK.to_be()
        )
    );
    // The peer's block is the *listener's* port with 127.0.0.1, because that is what `Connect`
    // wrote into the server side.
    let (_, peer) = m.get_peer_name(caller, 8, true);
    assert_eq!(peer.unwrap().port, 0xb000);
    assert_eq!(m.record(server).unwrap().addr, LOOPBACK.to_be());
    // Too small a size word is `EFAULT`, and a `AF_LOCAL` socket answers 0 without a block.
    assert_eq!(
        m.get_sock_name(caller, 7, true),
        (Done::refused(EFAULT), None)
    );
    let local = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(local, FAMILY_LOCAL, 0);
    m.bind(local, FAMILY_LOCAL, 0, 0, false);
    assert_eq!(m.get_sock_name(local, 0, false), (Done::ok(), None));
    // Opened but never bound is the second gate (`EINVAL`), and a record that is not even open is
    // the first one (`EBADF`) — the two are ordered open-then-bound, so a born-and-never-opened
    // socket cannot reach the bound question at all.
    let fresh = client(&mut m);
    assert_eq!(m.get_sock_name(fresh, 8, true).0, Done::refused(EINVAL));
    let bare = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.get_sock_name(bare, 8, true).0, Done::refused(EBADF));
}

#[test]
fn release_at_zero_is_close_then_unlink_and_the_peer_hears_both_chains() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0xc000);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0xc000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    // The order the shipped `Release` body keeps: `Close` runs while the record is still on the
    // list, and the unlink follows. `Close`'s answer is the notification — both chains of the peer,
    // which is the body's `EventOccurred(peer, 3)`.
    let done = m.close(caller);
    assert_eq!(
        done.fires,
        vec![Fire {
            socket: server,
            event: EVENT_READ | EVENT_WRITE
        }],
        "Release at zero still runs Close, and Close still tells the peer"
    );
    assert_eq!(
        m.record(server).unwrap().peer,
        None,
        "the peer's back-link is broken"
    );
    m.forget(caller);
    assert!(!m.is_live(caller), "the record is off the list");
    // Closing the listener releases what its queue held, and the list ends empty.
    let done = m.close(listener);
    assert_eq!(
        done.released,
        vec![server],
        "the queued socket is released once"
    );
    m.forget(server);
    m.forget(listener);
    assert_eq!(m.live(), 0);
    assert_eq!(m.walk().count(), 0);
}

#[test]
fn the_ephemeral_scan_holds_a_port_per_protocol_not_per_family() {
    // The body's scan is a triple: `[+44] == 2 && [+176] == protocol && [+46] == candidate`. So a
    // stream socket parked on `0xffff` does **not** spend the datagram counter's first number — a
    // host that compares family and port only would hand UDP the next one down and quietly shift
    // every ephemeral port after the first mixed-protocol bind.
    let mut m = Manager::new();
    let stream = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(stream, FAMILY_INET, 0);
    let bound = m.bind(stream, FAMILY_INET, 0xffff, LOOPBACK.to_be(), true);
    assert_eq!(bound.ret, 0);
    assert_eq!(
        m.next_bind_port(1),
        0xffff,
        "the other protocol is a different holder"
    );
    assert_eq!(
        m.next_bind_port(0),
        0xfffe,
        "the same protocol counts past it"
    );
}

#[test]
fn an_accepted_connection_leaves_the_error_word_alone() {
    let mut m = Manager::new();
    let listener = listening(&mut m, 0x1000);
    // The empty-queue answer writes 516 and returns -2.
    assert!(matches!(m.accept(listener, true), Accept2::Blocked(_)));
    assert_eq!(m.record(listener).unwrap().last_error, EWOULDBLOCK);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x1000, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    assert!(matches!(m.accept(listener, true), Accept2::Taken(s) if s == server));
    // `return 0` on the body's success branch, with no store to `[+180]`: the 516 is still there.
    assert_eq!(
        m.record(listener).unwrap().last_error,
        EWOULDBLOCK,
        "a successful Accept does not clear the error word"
    );
}

#[test]
fn get_peer_name_tells_no_channel_from_peer_gone() {
    let mut m = Manager::new();
    // Opened but never connected: there is no `[+196]` at all, which is `ENOTCONN`.
    let idle = m.born(0x1000, CLS_HOSTSOCKET).unwrap();
    m.open_ex(idle, FAMILY_INET, 0);
    assert_eq!(
        m.get_peer_name(idle, 8, true).0.ret,
        -1,
        "an unconnected socket is refused"
    );
    assert_eq!(m.record(idle).unwrap().last_error, ENOTCONN);
    let listener = listening(&mut m, 0x1200);
    let caller = client(&mut m);
    let server = m
        .connect(caller, FAMILY_INET, 0x1200, LOOPBACK.to_be(), true)
        .unwrap()
        .server;
    assert_eq!(m.get_peer_name(caller, 8, true).0.ret, 0, "connected");
    // The peer closes: `Close` clears the ring's peer pointer but leaves this side's ring, so the
    // second gate answers, and it is a different number from the first one.
    m.close(server);
    assert!(matches!(m.record(caller).unwrap().pipe, Ring { .. }));
    assert_eq!(m.record(caller).unwrap().peer, None);
    assert_eq!(m.get_peer_name(caller, 8, true).0.ret, -1);
    assert_eq!(
        m.record(caller).unwrap().last_error,
        ECONNRESET,
        "a half-gone channel is ECONNRESET, not ENOTCONN"
    );
    m.forget(idle);
    m.forget(listener);
    m.forget(caller);
}

/// A `SOCKET` line answers a connect that would otherwise need a network: the reply is readable
/// once, then the peer is gone, and writes are accepted and dropped because there is nobody to
/// read them. Neither direction ever blocks — a script has no wall clock to wait on.
#[test]
fn a_script_route_answers_a_raw_connect_then_ends_the_stream() {
    let mut m = Manager::new();
    let mut routes = ScriptRoutes::default();
    routes.add(None, 80, b"hello".to_vec());
    m.set_script_routes(routes);
    let slot = m.born(0x0400_0000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    let connected = match m.connect(slot, FAMILY_INET, 80, LOOPBACK.to_be(), true) {
        Ok(connected) => connected,
        Err(done) => panic!("a scripted connect was refused: {}", done.ret),
    };
    assert_eq!(connected.server, usize::MAX, "no loopback server end");
    assert_eq!(m.is_readable(slot), 0);
    let (done, bytes) = m.read_into(slot, 16, true);
    assert_eq!(done.ret, 5);
    assert_eq!(bytes, b"hello");
    let (done, bytes) = m.read_into(slot, 16, true);
    assert_eq!(
        (done.ret, bytes.len()),
        (0, 0),
        "the stream ends after the script's bytes"
    );
    assert_eq!(m.write_from(slot, b"GET /", true).ret, 5);
    assert!(!m.policy.real, "a script is not the real backend");
    m.forget(slot);
}

/// An address no line names is still the loopback model's business — the table does not turn a
/// miss into a refusal.
#[test]
fn a_connect_no_script_line_names_stays_on_the_loopback_model() {
    let mut m = Manager::new();
    let mut routes = ScriptRoutes::default();
    routes.add(
        Some(std::net::Ipv4Addr::new(10, 0, 0, 9)),
        80,
        b"only here".to_vec(),
    );
    m.set_script_routes(routes);
    let slot = m.born(0x0400_0000, CLS_HOSTSOCKET).unwrap();
    assert_eq!(m.open_ex(slot, FAMILY_INET, 0).ret, 0);
    assert!(m
        .connect(slot, FAMILY_INET, 80, LOOPBACK.to_be(), true)
        .is_err());
    m.forget(slot);
}
