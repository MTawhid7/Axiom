//! S4-Q3 and S4-Q4 measurements: what a sandboxed helper costs to start and
//! to keep, what IPC costs over Mach messages and over a socket pair, and
//! whether a helper can generate code.
//!
//! The helper's side of each measurement is a mode of `s4-helper`; the
//! broker's side is here too, so the protocol lives in one file.

use std::io::{self, BufReader, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::platform::mach::{self, MessageBuffer, Outgoing, ReceiveRight, Reply, SendRight};
use crate::platform::process::{self, Child, Command, Exit, Footprint};
use crate::probe::{self, Line, SOCKET_FD};

/// Sent by a helper on the inherited socket once it is ready.
pub const READY: u8 = b'R';
/// Sent by the broker to end a `ready` helper.
pub const QUIT: u8 = b'Q';
/// A `serve` helper's hello, carrying a send right to its port.
pub const SERVE_HELLO_ID: i32 = 0x5334_0002;
/// Mach request IDs: echo the inline payload; checksum the out-of-line
/// region; stop.
pub const ECHO_ID: i32 = 0x5334_0010;
pub const BULK_ID: i32 = 0x5334_0011;
pub const QUIT_ID: i32 = 0x5334_0012;
/// Socket request opcodes, each followed by a little-endian `u32` length and
/// that many bytes.
const OP_ECHO: u8 = b'E';
const OP_BULK: u8 = b'B';
const OP_QUIT: u8 = b'Q';

static RUN: AtomicU32 = AtomicU32::new(0);

/// The checksum both sides compute over bulk transfers: Fletcher-style over
/// 64-bit words, so it reads every byte and depends on their order, yet costs
/// a fraction of a nanosecond per byte. (A per-byte multiply-add chain cost
/// about 1 ns per byte on the M1, which would dominate a 1 MiB measurement.)
pub fn checksum(bytes: &[u8]) -> u64 {
    let (mut low, mut high) = (0_u64, 0_u64);
    let words = bytes.chunks_exact(8);
    let tail = words.remainder();
    for word in words {
        low = low.wrapping_add(u64::from_le_bytes(word.try_into().expect("eight bytes")));
        high = high.wrapping_add(low);
    }
    for &byte in tail {
        low = low.wrapping_add(u64::from(byte));
        high = high.wrapping_add(low);
    }
    high ^ low.rotate_left(32)
}

// ----- helper side -------------------------------------------------------

/// `ready` mode: say ready on the inherited socket, then wait until the
/// broker says quit or closes the socket.
pub fn ready_and_wait() -> io::Result<()> {
    ready_and_wait_on(process::adopt_inherited_socket(SOCKET_FD)?)
}

/// [`ready_and_wait`] on a socket the helper has already adopted.
pub fn ready_and_wait_on(mut socket: UnixStream) -> io::Result<()> {
    socket.write_all(&[READY])?;
    let mut byte = [0_u8; 1];
    loop {
        match socket.read(&mut byte) {
            Ok(0) => return Ok(()),
            Ok(_) if byte[0] == QUIT => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// `serve` mode: answer echo and bulk requests on the inherited socket and on
/// a Mach port, until each transport receives its quit request.
pub fn serve(rendezvous: &str) -> io::Result<()> {
    let socket = process::adopt_inherited_socket(SOCKET_FD)?;
    let broker = mach::look_up(rendezvous).map_err(io::Error::other)?;
    let port = ReceiveRight::new().map_err(io::Error::other)?;
    broker
        .send(
            Outgoing::new(SERVE_HELLO_ID, &[]),
            Some(&port),
            Some(Duration::from_secs(5)),
        )
        .map_err(io::Error::other)?;
    (&socket).write_all(&[READY])?;
    thread::scope(|scope| {
        let socket_server = scope.spawn(|| serve_socket(&socket));
        let mach_result = serve_mach(&port);
        let socket_result = socket_server
            .join()
            .unwrap_or_else(|_| Err(io::Error::other("socket server panicked")));
        mach_result.and(socket_result)
    })
}

fn serve_mach(port: &ReceiveRight) -> io::Result<()> {
    let mut buffer = MessageBuffer::new();
    let mut message = port.receive(&mut buffer, None).map_err(io::Error::other)?;
    loop {
        let response = match message.id {
            ECHO_ID => message.payload.clone(),
            BULK_ID => {
                let region = message.out_of_line.take();
                let sum = checksum(region.as_ref().map_or(&[], |r| r.bytes()));
                drop(region);
                sum.to_ne_bytes().to_vec()
            }
            QUIT_ID => return Ok(()),
            other => return Err(io::Error::other(format!("unexpected message {other:#x}"))),
        };
        let Some(Reply::SendOnce(reply)) = message.reply.take() else {
            return Err(io::Error::other("a request without a reply right"));
        };
        message = port
            .reply_and_receive(
                reply,
                Outgoing::new(message.id, &response),
                &mut buffer,
                None,
            )
            .map_err(io::Error::other)?;
    }
}

fn serve_socket(socket: &UnixStream) -> io::Result<()> {
    let mut reader = BufReader::with_capacity(64 * 1024, socket);
    let mut writer = socket;
    let mut payload = Vec::new();
    let mut reply = Vec::new();
    loop {
        let mut header = [0_u8; 5];
        reader.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header[1..].try_into().expect("four bytes")) as usize;
        payload.resize(len, 0);
        reader.read_exact(&mut payload)?;
        reply.clear();
        match header[0] {
            OP_ECHO => {
                reply.extend_from_slice(&header[1..]);
                reply.extend_from_slice(&payload);
            }
            OP_BULK => reply.extend_from_slice(&checksum(&payload).to_ne_bytes()),
            OP_QUIT => return Ok(()),
            other => return Err(io::Error::other(format!("unexpected opcode {other}"))),
        }
        writer.write_all(&reply)?;
    }
}

/// Read compiled profile bytecode sent by [`measure_startup`]: a
/// little-endian `u32` length, then the bytes.
pub fn read_bytecode(socket: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut len = [0_u8; 4];
    socket.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > 16 << 20 {
        return Err(io::Error::other("bytecode too large"));
    }
    let mut bytecode = vec![0_u8; len];
    socket.read_exact(&mut bytecode)?;
    Ok(bytecode)
}

// ----- broker side: start-up and footprint ---------------------------------

/// One start-up measurement.
#[derive(Clone, Debug)]
pub struct StartupSample {
    /// From just before `posix_spawn` to the helper's ready byte.
    pub spawn_to_ready: Duration,
    /// The helper's footprint once ready, while it waits.
    pub footprint: Footprint,
    /// The helper's own report (profile timings).
    pub states: Vec<(String, String)>,
    pub exit: Exit,
}

/// Launch `helper args…`, which must be in a mode that ends with the ready
/// protocol, and measure it. With `bytecode`, the broker sends a compiled
/// profile on the socket right after launching (the precompiled method).
pub fn measure_startup(
    helper: &Path,
    args: &[String],
    bytecode: Option<&[u8]>,
) -> io::Result<StartupSample> {
    let (broker_end, helper_end) = UnixStream::pair()?;
    let (stdout_reader, stdout_writer) = io::pipe()?;
    let mut command = Command::new(helper)?;
    for arg in args {
        command.arg(arg)?;
    }
    command
        .stdout(stdout_writer.as_fd())
        .inherit_stderr()
        .map_fd(helper_end.as_fd(), SOCKET_FD);
    let start = Instant::now();
    let child = command.spawn()?;
    drop(command);
    drop(stdout_writer);
    drop(helper_end);
    if let Some(bytecode) = bytecode {
        let len = u32::try_from(bytecode.len()).map_err(io::Error::other)?;
        (&broker_end).write_all(&len.to_le_bytes())?;
        (&broker_end).write_all(bytecode)?;
    }
    let mut byte = [0_u8; 1];
    (&broker_end).read_exact(&mut byte)?;
    let spawn_to_ready = start.elapsed();
    let footprint = process::footprint(child.id());
    // End the helper whatever happened, then report.
    let _ = (&broker_end).write_all(&[QUIT]);
    let output = io::read_to_string(stdout_reader)?;
    let exit = child.wait()?;
    if byte[0] != READY {
        return Err(io::Error::other(format!(
            "unexpected ready byte {}",
            byte[0]
        )));
    }
    Ok(StartupSample {
        spawn_to_ready,
        footprint: footprint?,
        states: states(&output),
        exit,
    })
}

fn states(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(probe::parse_line)
        .filter_map(|line| match line {
            Line::State(key, value) => Some((key, value)),
            Line::Probe(_) => None,
        })
        .collect()
}

// ----- broker side: IPC ----------------------------------------------------

/// The shape of an IPC measurement.
#[derive(Clone, Copy, Debug)]
pub struct IpcPlan {
    pub echo_bytes: usize,
    pub echo_rounds: usize,
    pub bulk_bytes: usize,
    pub bulk_rounds: usize,
    /// Each series is measured in this many batches, after one warm-up batch.
    pub batches: usize,
    /// Send and receive buffer size for the socket pair, if not the system's
    /// default (8 KiB for local stream sockets on macOS 27.2).
    pub socket_buffer: Option<usize>,
}

impl Default for IpcPlan {
    fn default() -> Self {
        Self {
            echo_bytes: 64,
            echo_rounds: 10_000,
            bulk_bytes: 1 << 20,
            bulk_rounds: 40,
            batches: 5,
            socket_buffer: None,
        }
    }
}

/// Mean time per operation in each batch.
#[derive(Clone, Debug, Default)]
pub struct Series(pub Vec<Duration>);

impl Series {
    pub fn median(&self) -> Duration {
        let mut sorted = self.0.clone();
        sorted.sort();
        sorted.get(sorted.len() / 2).copied().unwrap_or_default()
    }

    pub fn min(&self) -> Duration {
        self.0.iter().copied().min().unwrap_or_default()
    }

    pub fn max(&self) -> Duration {
        self.0.iter().copied().max().unwrap_or_default()
    }
}

/// What an IPC measurement found.
#[derive(Clone, Debug)]
pub struct IpcReport {
    pub socket_echo: Series,
    pub mach_echo: Series,
    pub socket_bulk: Series,
    pub mach_bulk: Series,
    /// The helper's hello came from the spawned child, by its audit trailer.
    pub hello_from_child: bool,
    pub exit: Exit,
}

/// Launch a `serve` helper (optionally sandboxed) and measure both transports.
pub fn measure_ipc(helper: &Path, profile_args: &[String], plan: IpcPlan) -> io::Result<IpcReport> {
    let id = format!(
        "{}.bench{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::SeqCst)
    );
    let rendezvous = format!("org.axiom.s4.{id}");
    let service = mach::check_in(&rendezvous).map_err(io::Error::other)?;
    let (broker_end, helper_end) = UnixStream::pair()?;
    if let Some(bytes) = plan.socket_buffer {
        process::set_socket_buffers(&broker_end, bytes)?;
        process::set_socket_buffers(&helper_end, bytes)?;
    }
    let mut command = Command::new(helper)?;
    command.arg("serve")?;
    for arg in profile_args {
        command.arg(arg)?;
    }
    command.arg("--rendezvous")?.arg(&rendezvous)?;
    command
        .inherit_stderr()
        .map_fd(helper_end.as_fd(), SOCKET_FD);
    let child = command.spawn()?;
    drop(command);
    drop(helper_end);
    let result = drive_ipc(&service, &broker_end, &child, plan);
    // On failure, don't leave the helper waiting.
    if result.is_err() {
        let _ = child.signal(9);
    }
    let exit = child.wait()?;
    let (socket_echo, mach_echo, socket_bulk, mach_bulk, hello_from_child) = result?;
    Ok(IpcReport {
        socket_echo,
        mach_echo,
        socket_bulk,
        mach_bulk,
        hello_from_child,
        exit,
    })
}

type Measured = (Series, Series, Series, Series, bool);

fn drive_ipc(
    service: &ReceiveRight,
    socket: &UnixStream,
    child: &Child,
    plan: IpcPlan,
) -> io::Result<Measured> {
    let timeout = Some(Duration::from_secs(10));
    let mut buffer = MessageBuffer::new();
    let hello = service
        .receive(&mut buffer, timeout)
        .map_err(io::Error::other)?;
    let hello_from_child = hello.id == SERVE_HELLO_ID && hello.sender_pid == child.id();
    let Some(Reply::Send(to_helper)) = hello.reply else {
        return Err(io::Error::other("the helper's hello carried no send right"));
    };
    let mut ready = [0_u8; 1];
    socket.set_read_timeout(timeout)?;
    (&*socket).read_exact(&mut ready)?;

    let echo: Vec<u8> = (0..plan.echo_bytes).map(|i| byte(i % 251)).collect();
    let bulk: Vec<u8> = (0..plan.bulk_bytes).map(|i| byte(i * 7 % 253)).collect();
    let bulk_sum = checksum(&bulk);
    let mut channel = SocketChannel::new(socket);
    let reply_port = ReceiveRight::new().map_err(io::Error::other)?;
    let mut mach_channel = MachChannel {
        to_helper: &to_helper,
        reply_port: &reply_port,
        buffer: &mut buffer,
    };

    let socket_echo = series(plan.batches, plan.echo_rounds, || channel.echo(&echo))?;
    let mach_echo = series(plan.batches, plan.echo_rounds, || mach_channel.echo(&echo))?;
    let socket_bulk = series(plan.batches, plan.bulk_rounds, || {
        channel.bulk(&bulk, bulk_sum)
    })?;
    let mach_bulk = series(plan.batches, plan.bulk_rounds, || {
        mach_channel.bulk(&bulk, bulk_sum)
    })?;

    channel.send(OP_QUIT, &[])?;
    to_helper
        .send(Outgoing::new(QUIT_ID, &[]), None, timeout)
        .map_err(io::Error::other)?;
    Ok((
        socket_echo,
        mach_echo,
        socket_bulk,
        mach_bulk,
        hello_from_child,
    ))
}

/// A test-pattern byte; `value` is below 256.
fn byte(value: usize) -> u8 {
    u8::try_from(value).expect("a pattern value below 256")
}

/// Run one warm-up batch and then `batches` timed batches of `rounds`
/// operations each.
fn series(
    batches: usize,
    rounds: usize,
    mut operation: impl FnMut() -> io::Result<()>,
) -> io::Result<Series> {
    for _ in 0..rounds {
        operation()?;
    }
    let mut means = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..rounds {
            operation()?;
        }
        means.push(start.elapsed() / u32::try_from(rounds).expect("rounds fit"));
    }
    Ok(Series(means))
}

struct SocketChannel<'a> {
    reader: BufReader<&'a UnixStream>,
    writer: &'a UnixStream,
    request: Vec<u8>,
    reply: Vec<u8>,
}

impl<'a> SocketChannel<'a> {
    fn new(socket: &'a UnixStream) -> Self {
        Self {
            reader: BufReader::with_capacity(64 * 1024, socket),
            writer: socket,
            request: Vec::new(),
            reply: Vec::new(),
        }
    }

    fn send(&mut self, op: u8, payload: &[u8]) -> io::Result<()> {
        self.request.clear();
        self.request.push(op);
        let len = u32::try_from(payload.len()).map_err(io::Error::other)?;
        self.request.extend_from_slice(&len.to_le_bytes());
        self.request.extend_from_slice(payload);
        self.writer.write_all(&self.request)
    }

    fn echo(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(OP_ECHO, payload)?;
        let mut len = [0_u8; 4];
        self.reader.read_exact(&mut len)?;
        self.reply.resize(u32::from_le_bytes(len) as usize, 0);
        self.reader.read_exact(&mut self.reply)?;
        if self.reply == payload {
            Ok(())
        } else {
            Err(io::Error::other("the socket echo differs"))
        }
    }

    fn bulk(&mut self, payload: &[u8], sum: u64) -> io::Result<()> {
        self.send(OP_BULK, payload)?;
        let mut answer = [0_u8; 8];
        self.reader.read_exact(&mut answer)?;
        if u64::from_ne_bytes(answer) == sum {
            Ok(())
        } else {
            Err(io::Error::other("the socket checksum differs"))
        }
    }
}

struct MachChannel<'a> {
    to_helper: &'a SendRight,
    reply_port: &'a ReceiveRight,
    buffer: &'a mut MessageBuffer,
}

impl MachChannel<'_> {
    fn echo(&mut self, payload: &[u8]) -> io::Result<()> {
        let answer = self
            .to_helper
            .call(
                Outgoing::new(ECHO_ID, payload),
                self.reply_port,
                self.buffer,
                None,
            )
            .map_err(io::Error::other)?;
        // Mach pads payloads to four bytes.
        if answer.payload.get(..payload.len()) == Some(payload) {
            Ok(())
        } else {
            Err(io::Error::other("the Mach echo differs"))
        }
    }

    fn bulk(&mut self, region: &[u8], sum: u64) -> io::Result<()> {
        let message = Outgoing {
            id: BULK_ID,
            payload: &[],
            out_of_line: Some(region),
        };
        let answer = self
            .to_helper
            .call(message, self.reply_port, self.buffer, None)
            .map_err(io::Error::other)?;
        if answer.payload.get(..8) == Some(&sum.to_ne_bytes()[..]) {
            Ok(())
        } else {
            Err(io::Error::other("the Mach checksum differs"))
        }
    }
}

// ----- broker side: JIT ----------------------------------------------------

/// Launch `helper args…` and return how it ended and its `STATE` lines.
pub fn run_report(helper: &Path, args: &[String]) -> io::Result<(Exit, Vec<(String, String)>)> {
    let (reader, writer) = io::pipe()?;
    let mut command = Command::new(helper)?;
    for arg in args {
        command.arg(arg)?;
    }
    command.stdout(writer.as_fd()).inherit_stderr();
    let child = command.spawn()?;
    drop(command);
    drop(writer);
    let output = io::read_to_string(reader)?;
    Ok((child.wait()?, states(&output)))
}

/// Launch `helper jit [profile args]` and return its report.
pub fn run_jit(
    helper: &Path,
    profile_args: &[String],
) -> io::Result<(Exit, Vec<(String, String)>)> {
    let mut args = vec!["jit".to_owned()];
    args.extend_from_slice(profile_args);
    args.extend(["--rendezvous".to_owned(), "org.axiom.s4.unused".to_owned()]);
    run_report(helper, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_depends_on_order_and_content() {
        assert_eq!(checksum(&[]), 0);
        assert_ne!(checksum(b"ab"), checksum(b"ba"));
        assert_eq!(checksum(b"abc"), checksum(b"abc"));
    }

    #[test]
    fn series_statistics() {
        let series = Series(vec![
            Duration::from_nanos(30),
            Duration::from_nanos(10),
            Duration::from_nanos(20),
        ]);
        assert_eq!(series.median(), Duration::from_nanos(20));
        assert_eq!(series.min(), Duration::from_nanos(10));
        assert_eq!(series.max(), Duration::from_nanos(30));
    }
}
