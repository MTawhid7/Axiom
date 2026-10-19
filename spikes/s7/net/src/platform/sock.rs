//! Sockets that don't block while connecting (`sys/socket.h`,
//! `netinet/in.h`): `std` has no non-blocking connect, which the I/O thread
//! needs so that a connection in progress never blocks its one wait (NW1).
//! The descriptor becomes a `TcpStream` or `UdpSocket` before `connect`.

use core::ffi::c_int;
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

// `sys/socket.h`, `netinet/in.h`, `sys/errno.h` (N2).
const AF_INET: u8 = 2;
const AF_INET6: u8 = 30;
const SOCK_STREAM: c_int = 1;
const SOCK_DGRAM: c_int = 2;
const EINPROGRESS: i32 = 36;

/// `struct sockaddr_in6` (`netinet6/in6.h`), also large enough for, and
/// prefixed like, `struct sockaddr_in` (`netinet/in.h`): length, family,
/// port.
#[repr(C)]
struct SockAddr6 {
    len: u8,
    family: u8,
    port: u16,
    flowinfo: u32,
    addr: [u8; 16],
    scope_id: u32,
}

const _: () = assert!(size_of::<SockAddr6>() == 28);

// libSystem: `sys/socket.h` (N2).
unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn connect(fd: c_int, address: *const SockAddr6, len: u32) -> c_int;
}

/// Start a TCP connection to `addr`. Writability on the returned stream
/// means the attempt has finished; then [`TcpStream::take_error`] says how.
pub(crate) fn connect_tcp(addr: SocketAddr) -> io::Result<TcpStream> {
    let fd = open(addr, SOCK_STREAM)?;
    let stream = TcpStream::from(fd);
    stream.set_nonblocking(true)?;
    stream.set_nodelay(true)?;
    start(stream.as_raw_fd(), addr)?;
    Ok(stream)
}

/// A UDP socket connected to `addr` (so bound implicitly, to an ephemeral
/// port, and receiving only from `addr`), non-blocking.
pub(crate) fn connect_udp(addr: SocketAddr) -> io::Result<UdpSocket> {
    let fd = open(addr, SOCK_DGRAM)?;
    let socket = UdpSocket::from(fd);
    socket.set_nonblocking(true)?;
    start(socket.as_raw_fd(), addr)?;
    Ok(socket)
}

fn open(addr: SocketAddr, kind: c_int) -> io::Result<OwnedFd> {
    let family = if addr.is_ipv4() { AF_INET } else { AF_INET6 };
    // SAFETY: plain integer arguments; the result is a new descriptor or -1
    // (N2).
    let fd = unsafe { socket(c_int::from(family), kind, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: N7. `fd` is a new descriptor that nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn start(fd: c_int, addr: SocketAddr) -> io::Result<()> {
    let mut raw = SockAddr6 {
        len: 0,
        family: 0,
        port: addr.port().to_be(),
        flowinfo: 0,
        addr: [0; 16],
        scope_id: 0,
    };
    let len = match addr {
        SocketAddr::V4(v4) => {
            // `struct sockaddr_in`: the address follows the port, then
            // eight zero bytes; it overlays `flowinfo` and `addr`.
            raw.family = AF_INET;
            raw.flowinfo = u32::from_ne_bytes(v4.ip().octets());
            16
        }
        SocketAddr::V6(v6) => {
            raw.family = AF_INET6;
            // `std` keeps `sin6_flowinfo` as the kernel does.
            raw.flowinfo = v6.flowinfo();
            raw.addr = v6.ip().octets();
            raw.scope_id = v6.scope_id();
            28
        }
    };
    raw.len = len;
    // SAFETY: N1 (`raw` is live, and `len` is the size of the structure
    // the family selects); N7 (`fd` belongs to the caller's live socket).
    let result = unsafe { connect(fd, &raw const raw, u32::from(len)) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(EINPROGRESS) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    #[test]
    fn connects_without_blocking_over_ipv4_and_ipv6() {
        for bind in ["127.0.0.1:0", "[::1]:0"] {
            let listener = TcpListener::bind(bind).unwrap();
            let addr = listener.local_addr().unwrap();
            let mut stream = connect_tcp(addr).unwrap();
            let (mut peer, _) = listener.accept().unwrap();
            peer.write_all(b"hi").unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut buf = [0_u8; 2];
            loop {
                match stream.read(&mut buf) {
                    Ok(2) => break,
                    Ok(_) => panic!("short read"),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline);
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            }
            assert_eq!(&buf, b"hi");
            assert!(stream.take_error().unwrap().is_none());
        }
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        let socket = connect_udp(peer.local_addr().unwrap()).unwrap();
        socket.send(b"ping").unwrap();
        let mut buf = [0_u8; 4];
        let (n, from) = peer.recv_from(&mut buf).unwrap();
        assert_eq!(
            (&buf[..n], from),
            (&b"ping"[..], socket.local_addr().unwrap())
        );
    }
}
