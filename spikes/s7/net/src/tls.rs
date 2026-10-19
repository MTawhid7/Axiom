//! One HTTPS fetch over rustls, as a state machine the I/O thread drives
//! from readiness events (NW1): a non-blocking TCP connect, the handshake
//! (verified by Security, `verifier`), the request, and the response.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, HandshakeKind};

use crate::http::Response;
use crate::platform::sock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Connecting,
    Handshaking,
    Exchanging,
    Done,
    Failed(String),
}

/// When each step finished, from the fetch's start (before name
/// resolution, which the caller does).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Times {
    pub(crate) resolved: Option<Duration>,
    pub(crate) connected: Option<Duration>,
    pub(crate) handshaken: Option<Duration>,
    pub(crate) first_byte: Option<Duration>,
    pub(crate) done: Option<Duration>,
}

pub(crate) struct TlsFetch {
    start: Instant,
    sock: TcpStream,
    conn: ClientConnection,
    request: Vec<u8>,
    pub(crate) phase: Phase,
    pub(crate) times: Times,
    pub(crate) response: Response,
    /// Plaintext buffer.
    buf: Box<[u8]>,
    eof: bool,
}

/// What the fetch waits for next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Interest {
    pub(crate) write: bool,
}

impl TlsFetch {
    /// Connect to `addr` for `host` and queue `request`. `start` is when
    /// the fetch began, and `resolved` when its address arrived.
    pub(crate) fn start(
        config: Arc<ClientConfig>,
        host: &str,
        addr: SocketAddr,
        request: Vec<u8>,
        start: Instant,
    ) -> Result<Self, String> {
        let name = ServerName::try_from(host.to_owned()).map_err(|e| e.to_string())?;
        let conn = ClientConnection::new(config, name).map_err(|e| e.to_string())?;
        let sock = sock::connect_tcp(addr).map_err(|e| format!("connect: {e}"))?;
        Ok(Self {
            start,
            sock,
            conn,
            request,
            phase: Phase::Connecting,
            times: Times {
                resolved: Some(start.elapsed()),
                ..Times::default()
            },
            response: Response::default(),
            buf: vec![0; 64 * 1024].into_boxed_slice(),
            eof: false,
        })
    }

    pub(crate) fn socket(&self) -> BorrowedFd<'_> {
        self.sock.as_fd()
    }

    pub(crate) fn finished(&self) -> bool {
        matches!(self.phase, Phase::Done | Phase::Failed(_))
    }

    pub(crate) fn handshake_kind(&self) -> Option<HandshakeKind> {
        self.conn.handshake_kind()
    }

    /// The negotiated version, cipher suite and key exchange group.
    pub(crate) fn negotiated(&self) -> String {
        format!(
            "{:?}/{:?}/{:?}",
            self.conn.protocol_version(),
            self.conn.negotiated_cipher_suite().map(|s| s.suite()),
            self.conn
                .negotiated_key_exchange_group()
                .map(rustls::crypto::SupportedKxGroup::name)
        )
    }

    /// Make whatever progress the socket allows.
    pub(crate) fn advance(&mut self) -> Interest {
        match self.step() {
            Ok(interest) => interest,
            Err(error) => {
                self.fail(error);
                Interest { write: false }
            }
        }
    }

    /// Give up, with `error`.
    pub(crate) fn fail(&mut self, error: String) {
        if !self.finished() {
            self.times.done = Some(self.start.elapsed());
            self.phase = Phase::Failed(error);
        }
    }

    fn step(&mut self) -> Result<Interest, String> {
        if self.phase == Phase::Connecting {
            // Writability ended the attempt; see how.
            if let Some(error) = self.sock.take_error().map_err(|e| e.to_string())? {
                return Err(format!("connect: {error}"));
            }
            if self.sock.peer_addr().is_err() {
                return Ok(Interest { write: true });
            }
            self.times.connected = Some(self.start.elapsed());
            self.phase = Phase::Handshaking;
        }
        loop {
            let mut progressed = false;
            let mut blocked_on_write = false;
            while self.conn.wants_write() {
                match self.conn.write_tls(&mut self.sock) {
                    Ok(_) => progressed = true,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        blocked_on_write = true;
                        break;
                    }
                    Err(e) => return Err(format!("write: {e}")),
                }
            }
            while !self.eof {
                match self.conn.read_tls(&mut self.sock) {
                    Ok(0) => self.eof = true,
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(format!("read: {e}")),
                }
                progressed = true;
                self.conn
                    .process_new_packets()
                    .map_err(|e| format!("tls: {e}"))?;
                self.read_plaintext()?;
            }
            if self.phase == Phase::Handshaking && !self.conn.is_handshaking() {
                self.times.handshaken = Some(self.start.elapsed());
                self.phase = Phase::Exchanging;
                self.conn
                    .writer()
                    .write_all(&self.request)
                    .map_err(|e| e.to_string())?;
                continue;
            }
            if self.response.complete() {
                self.times.done = Some(self.start.elapsed());
                self.phase = Phase::Done;
                return Ok(Interest { write: false });
            }
            if self.eof {
                self.read_plaintext()?;
                self.response.end()?;
                self.times.done = Some(self.start.elapsed());
                self.phase = Phase::Done;
                return Ok(Interest { write: false });
            }
            if !progressed || blocked_on_write {
                return Ok(Interest {
                    write: blocked_on_write,
                });
            }
        }
    }

    fn read_plaintext(&mut self) -> Result<(), String> {
        loop {
            match self.conn.reader().read(&mut self.buf) {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    if self.times.first_byte.is_none() {
                        self.times.first_byte = Some(self.start.elapsed());
                    }
                    self.response.feed(&self.buf[..n])?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                // The peer closed TCP without close_notify: the response's
                // framing decides whether anything was lost.
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
                Err(e) => return Err(format!("plaintext: {e}")),
            }
        }
    }
}
