//! The broker's side of a probe run: prepare the probe directory and the
//! listening sockets, check in a rendezvous service, launch the helper with
//! its channels, answer it, and collect its report.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, UdpSocket};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use crate::platform::mach::{self, MessageBuffer};
use crate::platform::process::{Command, Exit};
use crate::platform::sandbox;
use crate::probe::{self, HELLO_ID, Line, Outcome, ProbeResult, SOCKET_FD};
use crate::profile::{Method, Profile, RENDEZVOUS_PARAM};

/// Makes rendezvous names and probe directories unique within this process.
static RUN: AtomicU32 = AtomicU32::new(0);

/// A probe run: which helper, and which profile it applies (none for the
/// control run).
#[derive(Clone, Copy, Debug)]
pub struct ProbeRun<'a> {
    pub helper: &'a Path,
    pub profile: Option<(Profile, Method)>,
    /// A directory inside the home directory, for the probe files.
    pub work_dir: &'a Path,
}

/// The helper's hello over Mach, as the broker received it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachHello {
    /// The process ID in the kernel's audit trailer.
    pub sender_pid: i32,
    /// The process ID the helper wrote in its message.
    pub claimed_pid: i32,
}

/// What the broker itself observed during a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrokerView {
    /// The helper's Mach hello, if one arrived.
    pub mach_hello: Option<MachHello>,
    /// The helper said hello on the inherited socket.
    pub socket_greeted: bool,
    /// A TCP connection reached the broker's listener.
    pub tcp_accepted: bool,
    /// A UDP datagram reached the broker's socket.
    pub udp_received: bool,
}

/// What a probe run produced.
#[derive(Clone, Debug)]
pub struct ProbeReport {
    pub sandboxed_run: bool,
    pub child_pid: i32,
    pub exit: Exit,
    pub results: Vec<ProbeResult>,
    pub states: Vec<(String, String)>,
    pub broker: BrokerView,
}

/// One way a report differs from what S4-Q1 and S4-Q2 require.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch(pub String);

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ProbeReport {
    pub fn state(&self, key: &str) -> Option<&str> {
        self.states
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn result(&self, name: &str) -> Option<&ProbeResult> {
        self.results.iter().find(|r| r.name == name)
    }

    /// How many probes had their expected outcome.
    pub fn probes_as_expected(&self) -> usize {
        probe::catalogue()
            .into_iter()
            .filter(|(name, kind)| {
                self.result(name).map(|r| r.outcome)
                    == Some(probe::expected(*kind, self.sandboxed_run))
            })
            .count()
    }

    /// Every way this report departs from the expectations.
    pub fn mismatches(&self) -> Vec<Mismatch> {
        let mut found = Vec::new();
        let mut note = |text: String| found.push(Mismatch(text));
        if self.exit != Exit::Code(0) {
            note(format!("helper ended with {:?}", self.exit));
        }
        if let Some(error) = self.state("enter-error") {
            note(format!("the profile did not apply: {error}"));
        }
        let sandboxed = self.state("sandboxed") == Some("true");
        if sandboxed != self.sandboxed_run {
            note(format!("sandbox_check says sandboxed = {sandboxed}"));
        }
        for (name, kind) in probe::catalogue() {
            let want = probe::expected(kind, self.sandboxed_run);
            match self.result(&name) {
                Some(r) if r.outcome == want => {}
                Some(r) => note(format!(
                    "{name}: expected {want}, observed {} ({})",
                    r.outcome, r.detail
                )),
                None => note(format!("{name}: no result")),
            }
        }
        match self.broker.mach_hello {
            Some(hello)
                if hello.sender_pid == self.child_pid && hello.claimed_pid == self.child_pid => {}
            Some(hello) => note(format!(
                "Mach hello from pid {} claiming {}, but the child is {}",
                hello.sender_pid, hello.claimed_pid, self.child_pid
            )),
            None => note("no Mach hello reached the broker".to_owned()),
        }
        if !self.broker.socket_greeted {
            note("no hello on the inherited socket".to_owned());
        }
        // The broker's own view must agree with the helper's.
        let allowed = |name: &str| self.result(name).map(|r| r.outcome) == Some(Outcome::Allowed);
        if self.broker.tcp_accepted != allowed("tcp-connect") {
            note(format!(
                "broker saw a TCP connection: {}",
                self.broker.tcp_accepted
            ));
        }
        if self.broker.udp_received != allowed("udp-send") {
            note(format!(
                "broker received a UDP datagram: {}",
                self.broker.udp_received
            ));
        }
        found
    }
}

/// Run one probe scenario.
pub fn run_probe(run: &ProbeRun<'_>) -> io::Result<ProbeReport> {
    let id = format!(
        "{}.{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::SeqCst)
    );
    let dir = run.work_dir.join(format!("probe-{id}"));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(probe::HOME_FILE), b"S4 probe file\n")?;
    let report = run_in(run, &id, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    report
}

fn run_in(run: &ProbeRun<'_>, id: &str, dir: &Path) -> io::Result<ProbeReport> {
    let rendezvous = format!("org.axiom.s4.{id}");
    let service = mach::check_in(&rendezvous).map_err(io::Error::other)?;
    let tcp = TcpListener::bind(("127.0.0.1", 0))?;
    tcp.set_nonblocking(true)?;
    let udp = UdpSocket::bind(("127.0.0.1", 0))?;
    udp.set_read_timeout(Some(Duration::from_millis(5)))?;
    let (broker_end, helper_end) = UnixStream::pair()?;
    let (stdout_reader, stdout_writer) = io::pipe()?;

    let mut args: Vec<String> = vec!["probe".into()];
    if let Some((profile, method)) = run.profile {
        args.extend([
            "--profile".into(),
            profile.name().into(),
            "--method".into(),
            method.name().into(),
        ]);
    }
    args.extend([
        "--rendezvous".into(),
        rendezvous.clone(),
        "--dir".into(),
        path_arg(dir)?,
        "--tcp".into(),
        tcp.local_addr()?.port().to_string(),
        "--udp".into(),
        udp.local_addr()?.port().to_string(),
        "--parent".into(),
        std::process::id().to_string(),
    ]);
    let mut command = Command::new(run.helper)?;
    for arg in &args {
        command.arg(arg)?;
    }
    command
        .stdout(stdout_writer.as_fd())
        .inherit_stderr()
        .map_fd(helper_end.as_fd(), SOCKET_FD);
    // The precompiled method: this process compiles the profile, with the
    // helper's parameters, before launching it.
    let bytecode = match run.profile {
        Some((profile, Method::Precompiled)) => Some(
            sandbox::compile(profile.source(), &[(RENDEZVOUS_PARAM, &rendezvous)])
                .and_then(|compiled| compiled.bytecode())
                .map_err(io::Error::other)?,
        ),
        _ => None,
    };
    let child = command.spawn()?;
    drop(command);
    drop(stdout_writer);
    drop(helper_end);
    if let Some(bytecode) = &bytecode {
        let len = u32::try_from(bytecode.len()).map_err(io::Error::other)?;
        (&broker_end).write_all(&len.to_le_bytes())?;
        (&broker_end).write_all(bytecode)?;
    }

    let done = AtomicBool::new(false);
    let (output, exit, broker) = thread::scope(|scope| -> io::Result<_> {
        let socket_task = scope.spawn(|| answer_hello(&broker_end));
        let mach_task = scope.spawn(|| receive_hello(&service, &done));
        let network_task = scope.spawn(|| watch_sockets(&tcp, &udp, &done));
        let output = io::read_to_string(stdout_reader);
        let exit = child.wait();
        done.store(true, Ordering::SeqCst);
        let (tcp_accepted, udp_received) = network_task.join().unwrap_or_default();
        let broker = BrokerView {
            mach_hello: mach_task.join().unwrap_or(None),
            socket_greeted: socket_task.join().unwrap_or(false),
            tcp_accepted,
            udp_received,
        };
        Ok((output?, exit?, broker))
    })?;

    let mut results = Vec::new();
    let mut states = Vec::new();
    for line in output.lines().filter_map(probe::parse_line) {
        match line {
            Line::Probe(result) => results.push(result),
            Line::State(key, value) => states.push((key, value)),
        }
    }
    Ok(ProbeReport {
        sandboxed_run: run.profile.is_some(),
        child_pid: child.id(),
        exit,
        results,
        states,
        broker,
    })
}

/// Answer "ok" to the helper's "hello" on the inherited socket.
fn answer_hello(mut socket: &UnixStream) -> bool {
    if socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .is_err()
    {
        return false;
    }
    let mut line = String::new();
    match BufReader::new(socket).read_line(&mut line) {
        Ok(_) if line.trim() == "hello" => socket.write_all(b"ok\n").is_ok(),
        _ => false,
    }
}

/// Wait for the helper's Mach hello until the helper has exited.
fn receive_hello(service: &mach::ReceiveRight, done: &AtomicBool) -> Option<MachHello> {
    let mut buffer = MessageBuffer::new();
    // Keep receiving briefly after the helper exits, since its message may
    // still be queued.
    let mut grace = 20;
    loop {
        if let Ok(message) = service.receive(&mut buffer, Some(Duration::from_millis(50)))
            && message.id == HELLO_ID
            && message.payload.len() >= 4
        {
            let claimed = i32::from_ne_bytes(message.payload[..4].try_into().expect("four bytes"));
            return Some(MachHello {
                sender_pid: message.sender_pid,
                claimed_pid: claimed,
            });
        }
        if done.load(Ordering::SeqCst) {
            grace -= 1;
            if grace == 0 {
                return None;
            }
        }
    }
}

/// Record whether the helper reached the TCP listener or the UDP socket.
fn watch_sockets(tcp: &TcpListener, udp: &UdpSocket, done: &AtomicBool) -> (bool, bool) {
    let (mut accepted, mut received) = (false, false);
    let mut buffer = [0_u8; 16];
    loop {
        let finished = done.load(Ordering::SeqCst);
        if tcp.accept().is_ok() {
            accepted = true;
        }
        if udp.recv_from(&mut buffer).is_ok() {
            received = true;
        }
        if finished {
            return (accepted, received);
        }
    }
}

/// Launch the helper in its `named-profile` mode: it applies the built-in
/// "pure computation" profile with `sandbox_init`. Returns how it ended and
/// what it printed.
pub fn run_named_profile(helper: &Path) -> io::Result<(Exit, String)> {
    let (reader, writer) = io::pipe()?;
    let mut command = Command::new(helper)?;
    command.arg("named-profile")?;
    command.stdout(writer.as_fd()).inherit_stderr();
    let child = command.spawn()?;
    drop(command);
    drop(writer);
    let output = io::read_to_string(reader)?;
    Ok((child.wait()?, output))
}

fn path_arg(path: &Path) -> io::Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "a non-UTF-8 path"))
}

/// The default directory for probe files: `target/s4/work` below the current
/// directory, which for this repository lies inside the home directory.
pub fn default_work_dir() -> PathBuf {
    PathBuf::from("target").join("s4").join("work")
}
