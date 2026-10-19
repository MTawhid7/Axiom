//! One HTTP/3 GET over `quinn-proto` (NW5), as a state machine the I/O
//! thread drives: datagrams in, timer deadlines, datagrams out. HTTP/3's
//! framing and QPACK here are the least a GET needs (RFC 9114, RFC 9204):
//! an empty SETTINGS frame, a HEADERS frame of static-table references and
//! literals, no dynamic table, and the response's DATA frames counted.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use quinn_proto::{
    ClientConfig, Connection, ConnectionHandle, DatagramEvent, Dir, Endpoint, EndpointConfig,
    Event, StreamEvent, StreamId, VarInt,
};

use crate::platform::sock;

/// `H3_NO_ERROR` (RFC 9114 §8.1).
const H3_NO_ERROR: u32 = 0x100;
// HTTP/3 frame and stream types (RFC 9114 §7.2, §6.2).
const FRAME_DATA: u64 = 0x00;
const FRAME_HEADERS: u64 = 0x01;
const STREAM_CONTROL: u8 = 0x00;
const FRAME_SETTINGS: u8 = 0x04;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Times {
    pub(crate) resolved: Option<Duration>,
    pub(crate) connected: Option<Duration>,
    pub(crate) first_byte: Option<Duration>,
    pub(crate) done: Option<Duration>,
}

pub(crate) struct QuicFetch {
    start: Instant,
    socket: UdpSocket,
    remote: SocketAddr,
    endpoint: Endpoint,
    handle: ConnectionHandle,
    conn: Connection,
    authority: String,
    path: String,
    request: Option<StreamId>,
    /// The request stream's bytes not yet parsed into frames.
    pending: Vec<u8>,
    pub(crate) status: Option<u16>,
    pub(crate) body_bytes: u64,
    pub(crate) complete: bool,
    pub(crate) error: Option<String>,
    pub(crate) times: Times,
    pub(crate) datagrams_in: u64,
    pub(crate) datagrams_out: u64,
    send_buf: Vec<u8>,
    recv_buf: Box<[u8]>,
}

impl QuicFetch {
    /// Start a connection to `addr` for `host`, to GET `path` once it is up.
    pub(crate) fn start(
        config: ClientConfig,
        host: &str,
        addr: SocketAddr,
        path: &str,
        start: Instant,
    ) -> Result<Self, String> {
        let socket = sock::connect_udp(addr).map_err(|e| format!("udp: {e}"))?;
        let mut endpoint = Endpoint::new(Arc::new(EndpointConfig::default()), None, false, None);
        let (handle, conn) = endpoint
            .connect(Instant::now(), config, addr, host)
            .map_err(|e| e.to_string())?;
        let mut fetch = Self {
            start,
            socket,
            remote: addr,
            endpoint,
            handle,
            conn,
            authority: host.to_owned(),
            path: path.to_owned(),
            request: None,
            pending: Vec::new(),
            status: None,
            body_bytes: 0,
            complete: false,
            error: None,
            times: Times {
                resolved: Some(start.elapsed()),
                ..Times::default()
            },
            datagrams_in: 0,
            datagrams_out: 0,
            send_buf: Vec::with_capacity(1500),
            recv_buf: vec![0; 65_536].into_boxed_slice(),
        };
        fetch.drive(Instant::now());
        Ok(fetch)
    }

    pub(crate) fn socket(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }

    /// The connection's next timer, if any.
    pub(crate) fn deadline(&mut self) -> Option<Instant> {
        self.conn.poll_timeout()
    }

    pub(crate) fn closed(&self) -> bool {
        self.conn.is_drained()
    }

    pub(crate) fn rtt(&self) -> Duration {
        self.conn.rtt()
    }

    /// Read every waiting datagram, then make progress.
    pub(crate) fn on_readable(&mut self, now: Instant) {
        loop {
            match self.socket.recv(&mut self.recv_buf) {
                Ok(n) => {
                    self.datagrams_in += 1;
                    let data = BytesMut::from(&self.recv_buf[..n]);
                    let mut response = Vec::new();
                    match self
                        .endpoint
                        .handle(now, self.remote, None, None, data, &mut response)
                    {
                        Some(DatagramEvent::ConnectionEvent(_, event)) => {
                            self.conn.handle_event(event);
                        }
                        Some(DatagramEvent::Response(transmit)) => {
                            self.send(&response[..transmit.size]);
                        }
                        Some(DatagramEvent::NewConnection(_)) | None => {}
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.error.get_or_insert(format!("recv: {e}"));
                    break;
                }
            }
        }
        self.drive(now);
    }

    /// The timer fired: let the connection handle whatever expired.
    pub(crate) fn on_timeout(&mut self, now: Instant) {
        if self.conn.poll_timeout().is_some_and(|t| t <= now) {
            self.conn.handle_timeout(now);
        }
        self.drive(now);
    }

    /// Close the connection (`H3_NO_ERROR`), sending the close.
    pub(crate) fn close(&mut self, now: Instant) {
        self.conn
            .close(now, VarInt::from_u32(H3_NO_ERROR), Bytes::new());
        self.drive(now);
    }

    /// Process events and send what the connection wants to send.
    fn drive(&mut self, now: Instant) {
        while let Some(event) = self.conn.poll_endpoint_events() {
            if let Some(event) = self.endpoint.handle_event(self.handle, event) {
                self.conn.handle_event(event);
            }
        }
        while let Some(event) = self.conn.poll() {
            match event {
                Event::Connected => {
                    self.times.connected = Some(self.start.elapsed());
                    self.send_request();
                }
                Event::Stream(StreamEvent::Readable { id }) => self.read(id),
                Event::Stream(StreamEvent::Opened { dir: Dir::Uni }) => {
                    while let Some(id) = self.conn.streams().accept(Dir::Uni) {
                        self.read(id);
                    }
                }
                Event::ConnectionLost { reason } => {
                    if !self.complete {
                        self.error
                            .get_or_insert(format!("connection lost: {reason}"));
                    }
                    self.times.done.get_or_insert(self.start.elapsed());
                }
                _ => {}
            }
        }
        loop {
            self.send_buf.clear();
            let Some(transmit) = self.conn.poll_transmit(now, 1, &mut self.send_buf) else {
                break;
            };
            let datagram = self.send_buf[..transmit.size].to_vec();
            self.send(&datagram);
        }
    }

    fn send(&mut self, datagram: &[u8]) {
        match self.socket.send(datagram) {
            Ok(_) => self.datagrams_out += 1,
            // A full socket buffer drops the datagram; QUIC recovers.
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
                self.error.get_or_insert(format!("send: {e}"));
            }
        }
    }

    /// Open the control stream (an empty SETTINGS frame) and the request.
    fn send_request(&mut self) {
        let Some(control) = self.conn.streams().open(Dir::Uni) else {
            self.error
                .get_or_insert("no unidirectional stream".to_owned());
            return;
        };
        let _ = self
            .conn
            .send_stream(control)
            .write(&[STREAM_CONTROL, FRAME_SETTINGS, 0]);
        let Some(id) = self.conn.streams().open(Dir::Bi) else {
            self.error
                .get_or_insert("no bidirectional stream".to_owned());
            return;
        };
        let frame = headers_frame(&self.authority, &self.path);
        let mut stream = self.conn.send_stream(id);
        if stream.write(&frame).ok() != Some(frame.len()) || stream.finish().is_err() {
            self.error
                .get_or_insert("the request did not fit the stream".to_owned());
        }
        self.request = Some(id);
    }

    /// Read what stream `id` has: the request's response, or a server
    /// stream (control, QPACK), which is drained and ignored.
    fn read(&mut self, id: StreamId) {
        let mut recv = self.conn.recv_stream(id);
        let Ok(mut chunks) = recv.read(true) else {
            return;
        };
        let mut finished = false;
        let mut received = Vec::new();
        loop {
            match chunks.next(usize::MAX) {
                Ok(Some(chunk)) => received.extend_from_slice(&chunk.bytes),
                Ok(None) => {
                    finished = true;
                    break;
                }
                Err(_) => break,
            }
        }
        let _ = chunks.finalize();
        if Some(id) != self.request {
            return;
        }
        if !received.is_empty() && self.times.first_byte.is_none() {
            self.times.first_byte = Some(self.start.elapsed());
        }
        self.pending.extend_from_slice(&received);
        self.parse_frames();
        if finished && !self.complete {
            self.complete = true;
            self.times.done = Some(self.start.elapsed());
        }
    }

    fn parse_frames(&mut self) {
        loop {
            let mut at = 0;
            let (Some(kind), Some(len)) = (
                varint(&self.pending, &mut at),
                varint(&self.pending, &mut at),
            ) else {
                return;
            };
            let Ok(len) = usize::try_from(len) else {
                return;
            };
            if self.pending.len() < at + len {
                return;
            }
            let payload = &self.pending[at..at + len];
            match kind {
                FRAME_HEADERS if self.status.is_none() => self.status = status(payload),
                FRAME_DATA => self.body_bytes += u64::try_from(len).unwrap_or(0),
                _ => {}
            }
            self.pending.drain(..at + len);
        }
    }
}

/// A HEADERS frame for GET https://`authority``path`: QPACK with no dynamic
/// table (Required Insert Count and Delta Base both 0), static-table
/// indices from RFC 9204 Appendix A.
fn headers_frame(authority: &str, path: &str) -> Vec<u8> {
    let mut block = vec![0x00, 0x00];
    // Indexed field lines, static (0b11 + 6-bit index): `:method GET` (17),
    // `:scheme https` (23), `accept */*` (29).
    block.push(0xc0 | 0x11);
    block.push(0xc0 | 0x17);
    // Literal field lines with a static name reference (0b0101 + 4-bit
    // index): `:authority` (0), `:path` (1), `user-agent` (95).
    literal(&mut block, 0, authority);
    literal(&mut block, 1, path);
    block.push(0xc0 | 0x1d);
    literal(&mut block, 95, "axiom-s7-spike");
    let mut frame = Vec::with_capacity(block.len() + 4);
    put_varint(&mut frame, FRAME_HEADERS);
    put_varint(&mut frame, u64::try_from(block.len()).unwrap_or(0));
    frame.extend_from_slice(&block);
    frame
}

fn literal(out: &mut Vec<u8>, index: u64, value: &str) {
    prefixed(out, 0x50, 4, index);
    // Not Huffman-coded: H = 0, then a 7-bit-prefix length.
    prefixed(out, 0x00, 7, u64::try_from(value.len()).unwrap_or(0));
    out.extend_from_slice(value.as_bytes());
}

/// An integer with an `n`-bit prefix (RFC 7541 §5.1, used by QPACK), the
/// first byte's other bits set from `high`.
fn prefixed(out: &mut Vec<u8>, high: u8, n: u32, value: u64) {
    let max = (1_u64 << n) - 1;
    if value < max {
        out.push(high | u8::try_from(value).unwrap_or(0));
        return;
    }
    out.push(high | u8::try_from(max).unwrap_or(0));
    let mut rest = value - max;
    while rest >= 0x80 {
        out.push(0x80 | u8::try_from(rest & 0x7f).unwrap_or(0));
        rest >>= 7;
    }
    out.push(u8::try_from(rest).unwrap_or(0));
}

/// Read a prefixed integer whose first byte is `data[*at]`.
fn read_prefixed(data: &[u8], at: &mut usize, n: u32) -> Option<u64> {
    let max = (1_u64 << n) - 1;
    let first = u64::from(*data.get(*at)?) & max;
    *at += 1;
    if first < max {
        return Some(first);
    }
    let mut value = max;
    let mut shift = 0;
    loop {
        let byte = *data.get(*at)?;
        *at += 1;
        value += u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift > 56 {
            return None;
        }
    }
}

/// The `:status` of a QPACK field section, when its first line is a static
/// reference to a status entry, or a literal with an unencoded value.
fn status(block: &[u8]) -> Option<u16> {
    let mut at = 0;
    read_prefixed(block, &mut at, 8)?;
    read_prefixed(block, &mut at, 7)?;
    let first = *block.get(at)?;
    let from_index = |i: u64| -> Option<u16> {
        Some(match i {
            24 => 103,
            25 => 200,
            26 => 304,
            27 => 404,
            28 => 503,
            63 => 100,
            64 => 204,
            65 => 206,
            66 => 302,
            67 => 400,
            68 => 403,
            69 => 421,
            70 => 425,
            71 => 500,
            _ => return None,
        })
    };
    if first & 0xc0 == 0xc0 {
        return from_index(read_prefixed(block, &mut at, 6)?);
    }
    if first & 0xf0 == 0x50 {
        let index = read_prefixed(block, &mut at, 4)?;
        from_index(index)?;
        let huffman = *block.get(at)? & 0x80 != 0;
        let len = usize::try_from(read_prefixed(block, &mut at, 7)?).ok()?;
        if huffman {
            return None;
        }
        return std::str::from_utf8(block.get(at..at + len)?)
            .ok()?
            .parse()
            .ok();
    }
    None
}

/// A QUIC variable-length integer (RFC 9000 §16).
fn varint(data: &[u8], at: &mut usize) -> Option<u64> {
    let first = *data.get(*at)?;
    let len = 1_usize << (first >> 6);
    let bytes = data.get(*at..*at + len)?;
    *at += len;
    Some(
        bytes[1..]
            .iter()
            .fold(u64::from(first & 0x3f), |v, &b| (v << 8) | u64::from(b)),
    )
}

fn put_varint(out: &mut Vec<u8>, value: u64) {
    match value {
        0..=0x3f => out.push(u8::try_from(value).unwrap_or(0)),
        0x40..=0x3fff => {
            out.extend_from_slice(&(0x4000 | u16::try_from(value).unwrap_or(0)).to_be_bytes());
        }
        0x4000..=0x3fff_ffff => {
            out.extend_from_slice(&(0x8000_0000 | u32::try_from(value).unwrap_or(0)).to_be_bytes());
        }
        _ => out.extend_from_slice(&(0xc000_0000_0000_0000 | value).to_be_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_round_trip() {
        for v in [
            0,
            37,
            63,
            64,
            15_293,
            16_383,
            16_384,
            494_878_333,
            151_288_809_941_952_652,
        ] {
            let mut out = Vec::new();
            put_varint(&mut out, v);
            let mut at = 0;
            assert_eq!(varint(&out, &mut at), Some(v));
            assert_eq!(at, out.len());
        }
        // RFC 7541 C.1.2: 1337 with a 5-bit prefix.
        let mut out = Vec::new();
        prefixed(&mut out, 0, 5, 1337);
        assert_eq!(out, [31, 154, 10]);
        let mut at = 0;
        assert_eq!(read_prefixed(&out, &mut at, 5), Some(1337));
    }

    #[test]
    fn statuses_decode_from_static_references_and_literals() {
        assert_eq!(status(&[0, 0, 0xc0 | 0x19]), Some(200));
        assert_eq!(status(&[0, 0, 0xc0 | 0x3f, 1]), Some(204));
        assert_eq!(
            status(&[0, 0, 0x50 | 0x0f, 13, 3, b'2', b'0', b'1']),
            Some(201)
        );
        assert_eq!(status(&[0, 0, 0x50 | 0x0f, 13, 0x83, 1, 2, 3]), None);
        let frame = headers_frame("example.com", "/");
        let mut at = 0;
        assert_eq!(varint(&frame, &mut at), Some(FRAME_HEADERS));
        assert_eq!(
            varint(&frame, &mut at),
            Some(u64::try_from(frame.len() - at).unwrap())
        );
        assert_eq!(&frame[at..at + 4], &[0, 0, 0xd1, 0xd7]);
    }
}
