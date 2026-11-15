//! TLS as the guest sees it, over `rustls`.
//!
//! # Why this is here
//!
//! The delivered body
//! `Negotiate` drives the handshake and `Close` tears it down. The delivered member is an OpenSSL 0.9.x
//! port spread over thirty-five delivered objects, and this host has no TLS at all — so those slots
//! nothing today. This module is the transport half of closing that gap: a session that turns bytes in
//! and out of a `rustls` connection without owning a socket, so the ABI side can pump it over whatever
//! `ISocket` the guest handed to `SetSocket`.
//!
//! # What is *not* here
//!
//! The handshake is driven by whoever calls [`Session::feed`] and [`Session::take`]; nothing in here
//! blocks, opens a socket or spawns a thread. That split is deliberate: a BREW handset's `Negotiate` is
//! asynchronous (`SSLInfo`, `IX509Chain**` and an `AEECallback` all come back through it), and the
//! emulator's own loop is the only thing that may decide when the next packet moves.
//!
//! # No SNI, because the delivered member sends none
//!
//! The delivered body
//! `AEE_SSL_set_bio` over the guest's `ISocket`, `ISSL_set_version` for the protocol version, and a
//! trust decision of its own (`ISSL_check_security_trust_level`). A client that sends no server name
//! also has no name to check the certificate against — OpenSSL 0.9.x verifies the chain and stops
//! there unless someone calls `SSL_set1_host`. So the faithful shape here is a **chain-only** verifier,
//! and the name [`Session::client`] takes is the one rustls needs to build the connection, not a value
//! the ABI ever supplies.
//!
//! Time is not modelled either. `rustls` measures a session against `std::time::Instant`, and a run that
//! steps the guest deterministically must not depend on the wall clock — so the connection is built and
//! pumped as fast as the caller feeds it and no timer is created here.

use std::io::{Read, Write};
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
#[cfg(test)]
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
#[cfg(test)]
use rustls::{ServerConfig, ServerConnection};

/// One TLS session, as a byte pump.
pub struct Session {
    conn: ClientConnection,
    /// Bytes the connection wants sent.
    out: Vec<u8>,
}

impl Session {
    /// A client session trusting `roots` for `server`.
    ///
    /// The name is what rustls needs to build a connection; the delivered member sends no SNI and
    /// checks no name, so a caller that wants that shape passes [`platform_roots`] and accepts that the
    /// name is not presented on the wire.
    pub fn client(server: &str, roots: RootCertStore) -> Result<Self, rustls::Error> {
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|_| rustls::Error::General("protocol versions".into()))?
                .with_root_certificates(roots)
                .with_no_client_auth();
        let name = ServerName::try_from(server.to_string())
            .map_err(|_| rustls::Error::General("server name".into()))?;
        let mut session = Self {
            conn: ClientConnection::new(Arc::new(config), name)?,
            out: Vec::new(),
        };
        session.drain();
        Ok(session)
    }

    /// A session whose certificate is checked **without** the name, which is the delivered member's
    /// shape: it sends no SNI and never sets a host to verify against.
    pub fn client_chain_only(server: &str, roots: RootCertStore) -> Result<Self, rustls::Error> {
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|_| rustls::Error::General("protocol versions".into()))?
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(ChainOnly::new(roots)))
                .with_no_client_auth();
        let name = ServerName::try_from(server.to_string())
            .map_err(|_| rustls::Error::General("server name".into()))?;
        let mut session = Self {
            conn: ClientConnection::new(Arc::new(config), name)?,
            out: Vec::new(),
        };
        session.drain();
        Ok(session)
    }

    /// Hand the peer's bytes to the connection; whatever it produces lands in [`Session::take`].
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), rustls::Error> {
        if !bytes.is_empty() {
            self.conn
                .read_tls(&mut &bytes[..])
                .map_err(|_| rustls::Error::General("read_tls".into()))?;
            self.conn.process_new_packets()?;
        }
        self.drain();
        Ok(())
    }

    /// Take what [`Session::feed`] left, so the caller can send it and ask for the next bytes.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    pub fn handshaking(&self) -> bool {
        self.conn.is_handshaking()
    }

    pub fn pending(&mut self) -> usize {
        self.conn
            .reader()
            .into_first_chunk()
            .map(|chunk| chunk.len())
            .unwrap_or(0)
    }

    /// Queue plaintext for the peer; the ciphertext comes out through [`Session::take`].
    pub fn write(&mut self, plain: &[u8]) -> Result<(), rustls::Error> {
        self.conn
            .writer()
            .write_all(plain)
            .map_err(|_| rustls::Error::General("write".into()))?;
        self.drain();
        Ok(())
    }

    /// Read whatever plaintext the peer's bytes produced.
    ///
    /// A connection with nothing decrypted yet answers `WouldBlock`, which is not a failure — it is the
    /// shape every `ISocket::Read` in this ABI has — so it comes back as an empty read.
    pub fn read(&mut self) -> Result<Vec<u8>, rustls::Error> {
        let mut out = Vec::new();
        match self.conn.reader().read_to_end(&mut out) {
            Ok(_) => Ok(out),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(out),
            // An unclean close — a TCP FIN with no `close_notify` behind it — is reported only *after*
            // the reader has handed over everything it had buffered. `out` already holds those bytes
            // at that point, so trading them for the error would delete plaintext the caller is entitled
            // to read; the error is the answer only when nothing came at all.
            Err(_) if !out.is_empty() => Ok(out),
            Err(_) => Err(rustls::Error::General("read".into())),
        }
    }

    /// Read at most `want` bytes and **leave the rest** for the next call.
    ///
    /// This is the one a stream slot wants: `ISocket::Read(po, buf, len)` is answered with the number
    /// taken, and a caller that asked for less than has arrived reads the remainder on its next call.
    /// Draining the reader here would silently discard whatever did not fit the caller's buffer.
    ///
    /// It fills in fixed-size steps instead of reserving `want` up front: `want` reaches here from a
    /// guest register, and a bad length must not become a host allocation — the same reason
    /// `Memory::read_bytes` copies page by page rather than reserving. What a caller asks for now costs
    /// the host exactly what the peer has sent.
    pub fn read_up_to(&mut self, want: usize) -> Result<Streamed, rustls::Error> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 4096];
        // `Ok(0)` is rustls's "the peer sent `close_notify`"; an error with nothing in hand is the
        // *unclean* end (a TCP FIN with nothing behind it), which the delivered body answers as a
        // failure rather than as an end. Only the first is remembered as `Closed`.
        let mut closed = false;
        while out.len() < want {
            let take = (want - out.len()).min(chunk.len());
            match self.conn.reader().read(&mut chunk[..take]) {
                Ok(0) => {
                    closed = true;
                    break;
                }
                Ok(n) => out.extend_from_slice(&chunk[..n]),
                // `WouldBlock` is "nothing readable yet", not a failure — the same reading `read` gives.
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    // Same rule as above, one step further: a mid-walk failure must not un-hand the
                    // bytes already taken. An error that arrives with nothing in hand is the real one.
                    if out.is_empty() {
                        return Err(rustls::Error::General("read".into()));
                    }
                    break;
                }
            }
        }
        if !out.is_empty() {
            return Ok(Streamed::Have(out));
        }
        Ok(if closed {
            Streamed::Closed
        } else {
            Streamed::WouldBlock
        })
    }

    fn drain(&mut self) {
        while self.conn.wants_write() {
            let mut buf = Vec::new();
            if self.conn.write_tls(&mut buf).is_err() {
                break;
            }
            if buf.is_empty() {
                break;
            }
            self.out.extend_from_slice(&buf);
        }
    }
}

/// Certificate verification as the delivered member does it: the **chain**, not the name.
///
/// the delivered body sends no SNI and never calls `SSL_set1_host`, so there is no name to check against
/// — that is the OpenSSL 0.9.x default, and this verifier reproduces it exactly: `verify_for_usage`
/// builds and validates the path, and `verify_is_valid_for_subject_name` is deliberately never called.
/// The `server_name` rustls hands in is ignored, which is why a certificate issued for one name
/// completes a connection made under another.
#[derive(Debug)]
pub struct ChainOnly {
    roots: Vec<rustls::pki_types::TrustAnchor<'static>>,
    provider: Arc<CryptoProvider>,
}

impl ChainOnly {
    pub fn new(roots: RootCertStore) -> Self {
        Self {
            roots: roots.roots,
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }
}

impl ServerCertVerifier for ChainOnly {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = webpki::EndEntityCert::try_from(end_entity)
            .map_err(|_| rustls::Error::General("end entity certificate".into()))?;
        cert.verify_for_usage(
            self.provider.signature_verification_algorithms.all,
            &self.roots,
            intermediates,
            now,
            webpki::KeyUsage::server_auth(),
            None,
            None,
        )
        .map_err(|e| rustls::Error::General(format!("certificate chain: {e:?}")))?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// A root bundle that can be cloned, because a session takes the store and a host keeps the source.
#[derive(Clone, Default)]
pub struct Roots(Vec<rustls::pki_types::TrustAnchor<'static>>);

impl Roots {
    pub fn from_store(store: RootCertStore) -> Self {
        Self(store.roots)
    }

    pub fn store(&self) -> RootCertStore {
        RootCertStore {
            roots: self.0.clone(),
        }
    }
}

pub fn platform_roots() -> RootCertStore {
    let mut store = RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    store
}

#[derive(Debug)]
pub enum Streamed {
    /// Plaintext, at most what the caller had room for; the rest stays buffered for its next call.
    Have(Vec<u8>),
    /// Nothing decrypted yet, and the stream is still open.
    WouldBlock,
    /// The peer ended the stream cleanly, so there will never be more bytes.
    Closed,
}

#[derive(Default)]
pub struct Abi {
    /// `[+16]`: the `ISocket` `SetSocket` was given.
    pub socket: u32,
    /// The session built for that socket, when one could be built.
    pub session: Option<Session>,
    /// `[+176]`: 256 born, 257 socket set, 512 negotiating.
    pub state: u32,
    /// `[+108]`: the `SSLInfo*` `Negotiate` was given.
    pub info: u32,
    /// `[+112]`: the `IX509Chain**` it was given.
    pub certs: u32,
    /// `[+96]`: the `AEECallback*` to resume when the negotiation ends.
    pub cb: u32,
    /// `[+116]`: the result word. 2569 while negotiating, 2560 when the handshake completed.
    pub status: u32,
    /// Ciphertext the socket has not taken yet. A stream write may carry less than it was handed, and
    /// dropping the rest would corrupt the handshake rather than delay it.
    pub pending: Vec<u8>,
    pub last_error: i32,
    pub armed_read: Option<(u32, u32)>,
    pub armed_write: Option<(u32, u32)>,
}

/// The other end, for tests and for a host that has to answer a guest acting as a server. Not reachable
/// from any ABI slot: `ISSL` is a client, and this exists so the client above can be measured without a
/// network.
#[cfg(test)]
pub struct Server {
    conn: ServerConnection,
    out: Vec<u8>,
}

#[cfg(test)]
impl Server {
    pub fn new(cert: CertificateDer<'static>, key: PrivateKeyDer<'static>) -> Self {
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![cert], key)
                .unwrap();
        let mut session = Self {
            conn: ServerConnection::new(Arc::new(config)).unwrap(),
            out: Vec::new(),
        };
        session.drain();
        session
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), rustls::Error> {
        if !bytes.is_empty() {
            self.conn
                .read_tls(&mut &bytes[..])
                .map_err(|_| rustls::Error::General("read_tls".into()))?;
            self.conn.process_new_packets()?;
        }
        self.drain();
        Ok(())
    }

    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    pub fn handshaking(&self) -> bool {
        self.conn.is_handshaking()
    }

    /// Queue a `close_notify` alert so the other end sees a **clean** end of stream; the bytes leave
    /// through [`Server::take`]. Worth naming because the two kinds of end are different answers at
    /// `ISocket::Read` (a plain `0` after a `close_notify`, an error after a bare TCP FIN), and a test
    /// that wants to tell them apart has to be able to produce the first.
    pub fn close(&mut self) {
        self.conn.send_close_notify();
        self.drain();
    }

    pub fn write(&mut self, plain: &[u8]) -> Result<(), rustls::Error> {
        self.conn
            .writer()
            .write_all(plain)
            .map_err(|_| rustls::Error::General("write".into()))?;
        self.drain();
        Ok(())
    }

    pub fn read(&mut self) -> Result<Vec<u8>, rustls::Error> {
        let mut out = Vec::new();
        match self.conn.reader().read_to_end(&mut out) {
            Ok(_) => Ok(out),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(out),
            // An unclean close — a TCP FIN with no `close_notify` behind it — is reported only *after*
            // the reader has handed over everything it had buffered. `out` already holds those bytes
            // at that point, so trading them for the error would delete plaintext the caller is entitled
            // to read; the error is the answer only when nothing came at all.
            Err(_) if !out.is_empty() => Ok(out),
            Err(_) => Err(rustls::Error::General("read".into())),
        }
    }

    fn drain(&mut self) {
        while self.conn.wants_write() {
            let mut buf = Vec::new();
            if self.conn.write_tls(&mut buf).is_err() {
                break;
            }
            if buf.is_empty() {
                break;
            }
            self.out.extend_from_slice(&buf);
        }
    }
}

/// A self-signed certificate for the loopback server the end-to-end test runs. Test-only: nothing in
/// the ABI ever needs a certificate of its own, and a shipping build must not be able to make one.
#[cfg(test)]
pub fn test_pair() -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert = generated.cert.der().clone();
    let key = PrivateKeyDer::Pkcs8(generated.signing_key.serialize_der().into());
    (cert, key)
}

#[cfg(test)]
#[path = "test/tls.rs"]
mod tests;
