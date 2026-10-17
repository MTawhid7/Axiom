//! What a helper tries once it is inside its profile (or, in the control run,
//! without one), and how it reports the results.
//!
//! Each probe is either **forbidden** (the profile must deny it: S4-Q2's
//! files, network, process launching and Mach services) or **required** (it
//! must keep working inside the profile: allocation, threads, the inherited
//! channels). In the control run, every probe must be allowed. That shows each
//! probe can detect the access it tests.
//!
//! A helper prints one line per probe on its standard output, which is an
//! inherited descriptor, so the report itself shows that logging works:
//!
//! ```text
//! STATE   sandboxed      true
//! PROBE   read-home-file DENIED  Operation not permitted (os error 1)
//! ```
//!
//! (Fields are separated by tabs.)

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::platform::{mach, process};

/// The file the broker creates in the probe directory, inside the home
/// directory, for the helper to try to read.
pub const HOME_FILE: &str = "home-probe.txt";
/// The file the helper tries to create in the probe directory.
pub const WRITE_FILE: &str = "write-probe.txt";
/// A system file every process can normally read.
pub const SYSTEM_FILE: &str = "/etc/hosts";
/// A system font file.
pub const FONT_FILE: &str = "/System/Library/Fonts/Helvetica.ttc";
/// Mach services a renderer must not reach. WebKit's content profile denies
/// the window server, font server, LaunchServices and its database, and
/// App Sandbox allows the pasteboard (macOS note §2.1–§2.2).
pub const MACH_SERVICES: &[&str] = &[
    "com.apple.windowserver.active",
    "com.apple.fonts",
    "com.apple.pasteboard.1",
    "com.apple.coreservices.launchservicesd",
    "com.apple.lsd.mapdb",
];
/// The message ID of a helper's hello to the broker.
pub const HELLO_ID: i32 = 0x5334_0001;
/// The descriptor number of the socket a helper inherits.
pub const SOCKET_FD: RawFd = 3;

/// What a probe observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The operation succeeded.
    Allowed,
    /// The operation was refused for lack of permission.
    Denied,
    /// The operation failed for some other reason. Never expected.
    Failed,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "ALLOWED",
            Self::Denied => "DENIED",
            Self::Failed => "FAILED",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Outcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ALLOWED" => Ok(Self::Allowed),
            "DENIED" => Ok(Self::Denied),
            "FAILED" => Ok(Self::Failed),
            other => Err(format!("unknown outcome {other:?}")),
        }
    }
}

/// Whether a profile must deny a probe or let it work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Forbidden,
    Required,
}

/// Every probe, in the order a helper runs them.
pub fn catalogue() -> Vec<(String, Kind)> {
    let mut probes: Vec<(String, Kind)> = [
        "read-home-file",
        "read-system-file",
        "read-font-file",
        "write-file",
        "tcp-connect",
        "udp-send",
        "dns-resolve",
        "spawn-process",
        "signal-parent",
    ]
    .iter()
    .map(|name| ((*name).to_owned(), Kind::Forbidden))
    .collect();
    probes.extend(
        MACH_SERVICES
            .iter()
            .map(|service| (format!("mach-lookup:{service}"), Kind::Forbidden)),
    );
    probes.extend(
        ["alloc-64mib", "threads", "ipc-socket", "ipc-mach"]
            .iter()
            .map(|name| ((*name).to_owned(), Kind::Required)),
    );
    probes
}

/// The outcome a probe of `kind` must have, inside a profile or not.
pub fn expected(kind: Kind, sandboxed: bool) -> Outcome {
    match (kind, sandboxed) {
        (Kind::Forbidden, true) => Outcome::Denied,
        _ => Outcome::Allowed,
    }
}

/// One probe's result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeResult {
    pub name: String,
    pub outcome: Outcome,
    pub detail: String,
}

/// A line of a helper's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Probe(ProbeResult),
    State(String, String),
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Probe(r) => write!(f, "PROBE\t{}\t{}\t{}", r.name, r.outcome, clean(&r.detail)),
            Self::State(key, value) => write!(f, "STATE\t{key}\t{}", clean(value)),
        }
    }
}

/// Keep a field on one line and free of separators.
fn clean(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

/// Parse one report line. Lines that aren't part of the report are ignored.
pub fn parse_line(line: &str) -> Option<Line> {
    let mut fields = line.splitn(4, '\t');
    match fields.next()? {
        "PROBE" => {
            let name = fields.next()?.to_owned();
            let outcome = fields.next()?.parse().ok()?;
            let detail = fields.next().unwrap_or("").to_owned();
            Some(Line::Probe(ProbeResult {
                name,
                outcome,
                detail,
            }))
        }
        "STATE" => {
            let key = fields.next()?.to_owned();
            let value = fields.next().unwrap_or("").to_owned();
            Some(Line::State(key, value))
        }
        _ => None,
    }
}

/// What a helper needs in order to run the probes.
#[derive(Clone, Debug)]
pub struct Config {
    /// Holds [`HOME_FILE`]; [`WRITE_FILE`] must not exist in it yet.
    pub dir: PathBuf,
    pub tcp_port: u16,
    pub udp_port: u16,
    pub parent_pid: i32,
    pub rendezvous: String,
}

/// Run every probe in [`catalogue`] order, reporting each line to `out` as
/// soon as it is known.
pub fn run_all(config: &Config, out: &mut dyn Write) -> io::Result<()> {
    for (name, _) in catalogue() {
        let (outcome, detail) = run_one(&name, config);
        writeln!(
            out,
            "{}",
            Line::Probe(ProbeResult {
                name,
                outcome,
                detail
            })
        )?;
        out.flush()?;
    }
    Ok(())
}

fn run_one(name: &str, config: &Config) -> (Outcome, String) {
    if let Some(service) = name.strip_prefix("mach-lookup:") {
        return match mach::look_up(service) {
            Ok(_right) => (Outcome::Allowed, "found".to_owned()),
            Err(error) => (Outcome::Denied, error.to_string()),
        };
    }
    match name {
        "read-home-file" => from_io(read_file(&config.dir.join(HOME_FILE))),
        "read-system-file" => from_io(read_file(SYSTEM_FILE.as_ref())),
        "read-font-file" => from_io(read_file(FONT_FILE.as_ref())),
        "write-file" => from_io(write_file(config)),
        "tcp-connect" => from_io(tcp_connect(config.tcp_port)),
        "udp-send" => from_io(udp_send(config.udp_port)),
        "dns-resolve" => dns_resolve(),
        "spawn-process" => from_io(spawn_process()),
        "signal-parent" => {
            from_io(process::send_signal(config.parent_pid, 0).map(|()| "signal 0".to_owned()))
        }
        "alloc-64mib" => (Outcome::Allowed, allocate()),
        "threads" => from_io(threads()),
        "ipc-socket" => from_io(ipc_socket()),
        "ipc-mach" => ipc_mach(config),
        other => (Outcome::Failed, format!("unknown probe {other}")),
    }
}

/// Classify an I/O result: a permission error is a denial.
fn from_io(result: io::Result<String>) -> (Outcome, String) {
    match result {
        Ok(detail) => (Outcome::Allowed, detail),
        Err(error) if is_denial(&error) => (Outcome::Denied, error.to_string()),
        Err(error) => (Outcome::Failed, error.to_string()),
    }
}

/// `EPERM` (1) or `EACCES` (13).
fn is_denial(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied || matches!(error.raw_os_error(), Some(1 | 13))
}

fn read_file(path: &std::path::Path) -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    let read = File::open(path)?.read(&mut bytes)?;
    Ok(format!("read {read} bytes"))
}

fn write_file(config: &Config) -> io::Result<String> {
    let path = config.dir.join(WRITE_FILE);
    File::create(&path)?.write_all(b"s4")?;
    std::fs::remove_file(&path)?;
    Ok("created and removed".to_owned())
}

fn tcp_connect(port: u16) -> io::Result<String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    Ok(format!("connected to {address}"))
}

fn udp_send(port: u16) -> io::Result<String> {
    let socket = UdpSocket::bind(("127.0.0.1", 0))?;
    socket.send_to(b"s4", ("127.0.0.1", port))?;
    Ok(format!("sent to 127.0.0.1:{port}"))
}

/// Name resolution goes through `getaddrinfo`, which reads `/etc/hosts` and
/// asks mDNSResponder over Mach; either way, a sandbox shows up as a failure.
fn dns_resolve() -> (Outcome, String) {
    match ("localhost", 80).to_socket_addrs() {
        Ok(mut addresses) => match addresses.next() {
            Some(address) => (Outcome::Allowed, format!("localhost is {}", address.ip())),
            None => (Outcome::Denied, "no addresses".to_owned()),
        },
        Err(error) => (Outcome::Denied, error.to_string()),
    }
}

fn spawn_process() -> io::Result<String> {
    let status = Command::new("/usr/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(format!("/usr/bin/true: {status}"))
}

/// Allocate 64 MiB and touch every page.
fn allocate() -> String {
    let buffer = vec![0x5a_u8; 64 << 20];
    let sum = buffer
        .iter()
        .step_by(16384)
        .fold(0_u64, |sum, &b| sum.wrapping_add(u64::from(b)));
    format!("touched {} pages", sum / 0x5a)
}

fn threads() -> io::Result<String> {
    let handles = (0..4_u64)
        .map(|i| {
            thread::Builder::new()
                .name(format!("probe-{i}"))
                .spawn(move || (0..100_000_u64).fold(i, |a, x| a ^ x))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let joined = handles.into_iter().filter_map(|h| h.join().ok()).count();
    Ok(format!("{joined} threads joined"))
}

/// Say hello to the broker over the inherited socket, and expect "ok".
fn ipc_socket() -> io::Result<String> {
    let mut socket = process::adopt_inherited_socket(SOCKET_FD)?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.write_all(b"hello\n")?;
    let mut answer = String::new();
    BufReader::new(&socket).read_line(&mut answer)?;
    if answer.trim() == "ok" {
        Ok("hello answered".to_owned())
    } else {
        Err(io::Error::other(format!("unexpected answer {answer:?}")))
    }
}

/// Look up the broker's rendezvous service and send it a hello that carries
/// this process's ID, which the broker compares with the audit trailer.
fn ipc_mach(config: &Config) -> (Outcome, String) {
    let pid = i32::try_from(std::process::id()).expect("pids fit in an i32");
    let result = mach::look_up(&config.rendezvous).and_then(|broker| {
        broker.send(
            HELLO_ID,
            &pid.to_ne_bytes(),
            None,
            Some(Duration::from_secs(2)),
        )
    });
    match result {
        Ok(()) => (
            Outcome::Allowed,
            format!("hello sent to {}", config.rendezvous),
        ),
        Err(error) => (Outcome::Failed, error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lines_round_trip() {
        let line = Line::Probe(ProbeResult {
            name: "tcp-connect".to_owned(),
            outcome: Outcome::Denied,
            detail: "Operation\tnot\npermitted".to_owned(),
        });
        let text = line.to_string();
        assert_eq!(text, "PROBE\ttcp-connect\tDENIED\tOperation not permitted");
        assert_eq!(
            parse_line(&text),
            Some(Line::Probe(ProbeResult {
                name: "tcp-connect".to_owned(),
                outcome: Outcome::Denied,
                detail: "Operation not permitted".to_owned(),
            }))
        );
        assert_eq!(
            parse_line("STATE\tsandboxed\ttrue"),
            Some(Line::State("sandboxed".to_owned(), "true".to_owned()))
        );
        assert_eq!(parse_line("noise"), None);
    }

    #[test]
    fn expectations_follow_the_kind_and_the_sandbox() {
        assert_eq!(expected(Kind::Forbidden, true), Outcome::Denied);
        assert_eq!(expected(Kind::Required, true), Outcome::Allowed);
        assert_eq!(expected(Kind::Forbidden, false), Outcome::Allowed);
        let catalogue = catalogue();
        assert_eq!(catalogue.len(), 9 + MACH_SERVICES.len() + 4);
        assert!(
            catalogue
                .iter()
                .any(|(n, k)| n == "ipc-mach" && *k == Kind::Required)
        );
    }

    #[test]
    fn permission_errors_are_denials() {
        assert!(is_denial(&io::Error::from_raw_os_error(1)));
        assert!(is_denial(&io::Error::from_raw_os_error(13)));
        assert!(!is_denial(&io::Error::from_raw_os_error(2)));
    }
}
