use super::*;

fn self_signed() -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert = generated.cert.der().clone();
    let key = PrivateKeyDer::Pkcs8(generated.signing_key.serialize_der().into());
    (cert, key)
}

/// A whole handshake and a payload each way, with no socket and no network: the two connections are
/// handed each other's bytes. This is the unit the `ISSL` slots will drive, held to the only
/// property that matters for them — the session completes and moves data, and it says so through
/// `handshaking()` rather than by blocking.
#[test]
fn a_session_completes_a_handshake_and_carries_bytes_both_ways() {
    let (cert, key) = self_signed();
    let mut roots = RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    let mut client = Session::client("localhost", roots).unwrap();
    let mut server = Server::new(cert, key);
    assert!(client.handshaking() && server.handshaking());

    // The pump: each round hands the other side everything the first produced, until both stop.
    for _ in 0..8 {
        client.feed(&[]).unwrap();
        let to_server = client.take();
        server.feed(&to_server).unwrap();
        let to_client = server.take();
        client.feed(&to_client).unwrap();
        if !client.handshaking() && !server.handshaking() {
            break;
        }
    }
    assert!(!client.handshaking(), "the client finished");
    assert!(!server.handshaking(), "the server finished");

    client.write(b"quoin").unwrap();
    let to_server = client.take();
    server.feed(&to_server).unwrap();
    assert_eq!(server.read().unwrap(), b"quoin");

    server.write(b"tls").unwrap();
    let to_client = server.take();
    client.feed(&to_client).unwrap();
    assert_eq!(client.read().unwrap(), b"tls");
}

#[test]
fn pending_counts_the_plaintext_without_taking_it() {
    let (cert, key) = self_signed();
    let mut roots = RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    let mut client = Session::client("localhost", roots).unwrap();
    let mut server = Server::new(cert, key);
    for _ in 0..8 {
        client.feed(&[]).unwrap();
        server.feed(&client.take()).unwrap();
        client.feed(&server.take()).unwrap();
        if !client.handshaking() && !server.handshaking() {
            break;
        }
    }
    assert_eq!(client.pending(), 0, "the peer has sent nothing yet");
    server.write(b"quoin-tls").unwrap();
    client.feed(&server.take()).unwrap();
    assert_eq!(client.pending(), 9, "one record of nine bytes is decrypted");
    assert_eq!(client.pending(), 9, "and asking again took nothing away");
    assert_eq!(client.read().unwrap(), b"quoin-tls");
    assert_eq!(
        client.pending(),
        0,
        "read, so there is nothing left to report"
    );
}

/// The delivered member never checks the name, so neither may this: a certificate issued for
/// `localhost` completes a connection made under `example.com`, and bytes still move. A verifier
/// that quietly checked the name would refuse this handshake, which is what makes the assertion
/// mean something.
#[test]
fn the_chain_only_verifier_ignores_the_name_and_still_checks_the_chain() {
    let (cert, key) = self_signed();
    let mut roots = RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    let mut client = Session::client_chain_only("example.com", roots).unwrap();
    let mut server = Server::new(cert, key);
    for _ in 0..8 {
        client.feed(&[]).unwrap();
        let to_server = client.take();
        server.feed(&to_server).unwrap();
        let to_client = server.take();
        client.feed(&to_client).unwrap();
        if !client.handshaking() && !server.handshaking() {
            break;
        }
    }
    assert!(
        !client.handshaking(),
        "a name the certificate does not carry is not a reason to fail here"
    );
    client.write(b"name-blind").unwrap();
    let to_server = client.take();
    server.feed(&to_server).unwrap();
    assert_eq!(server.read().unwrap(), b"name-blind");

    // The chain is still what is checked: a session rooted at another certificate does not
    // complete.
    let (cert_b, _key_b) = self_signed();
    let mut wrong = RootCertStore::empty();
    wrong.add(cert_b).unwrap();
    let mut blind = Session::client_chain_only("example.com", wrong).unwrap();
    let (cert_s, key_s) = self_signed();
    let mut stranger = Server::new(cert_s, key_s);
    let mut refused = false;
    for _ in 0..4 {
        blind.feed(&[]).unwrap();
        let to_server = blind.take();
        stranger.feed(&to_server).unwrap();
        let to_client = stranger.take();
        if blind.feed(&to_client).is_err() {
            refused = true;
            break;
        }
        if !blind.handshaking() && !stranger.handshaking() {
            break;
        }
    }
    assert!(
        refused || blind.handshaking(),
        "an untrusted chain is still refused"
    );
}

/// The platform bundle is a real one and can back a session. An empty store would turn every
/// handshake into `UnknownIssuer` without anything failing to compile, which is the shape that
/// would only show up on a real connection.
#[test]
fn the_platform_bundle_has_roots_and_a_session_can_be_built_on_it() {
    let roots = platform_roots();
    assert!(
        roots.roots.len() > 50,
        "the public bundle is not a stub: {} roots",
        roots.roots.len()
    );
    assert!(Session::client("example.com", roots).is_ok());
}

/// The handshake really is verified: a session rooted at a different certificate refuses the server
/// instead of completing. Without this, the test above would pass on a connection that trusts
/// everything.
#[test]
fn a_session_rooted_elsewhere_refuses_the_server() {
    let (cert, key) = self_signed();
    let (other, _) = self_signed();
    let mut roots = RootCertStore::empty();
    roots.add(other).unwrap();
    let mut client = Session::client("localhost", roots).unwrap();
    let mut server = Server::new(cert, key);
    let mut refused = false;
    for _ in 0..4 {
        client.feed(&[]).unwrap();
        let to_server = client.take();
        server.feed(&to_server).unwrap();
        let to_client = server.take();
        if client.feed(&to_client).is_err() {
            refused = true;
            break;
        }
        if !client.handshaking() && !server.handshaking() {
            break;
        }
    }
    assert!(
        refused || client.handshaking(),
        "an unknown issuer is not accepted"
    );
}
