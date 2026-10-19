//! # Spike S7 (Step 5): the network process and TLS
//!
//! Validates the network process's sandbox (NW17) and settles NW6, as the
//! Step 5 tracker's §10.3 designs it. A broker, standing in for Axiom's
//! browser process, compiles the network profile and launches this same
//! executable as a helper, which applies the bytecode at the top of `main`
//! (S4's precompiled method), then runs one mode.
//!
//! Questions:
//! - **S7-Q1:** under a deny-by-default profile, does a helper's TLS client
//!   verify servers through `SecTrustEvaluateWithError`, and which rules
//!   does it need? (`verify`, `probe`)
//! - **S7-Q2:** rustls with `aws-lc-rs`, verified through Security, against
//!   Network framework's TLS: handshake latency (full and resumed), CPU per
//!   megabyte, footprint. (`handshakes`, `download`, `ready`)
//! - **S7-Q3:** do `DNSServiceGetAddrInfo` and `DNSServiceQueryRecord`
//!   (HTTPS records), driven through `DNSServiceRefSockFD` in kqueue, work
//!   under the profile? (`dns`)
//! - **S7-Q4:** does one I/O thread drive rustls, `quinn-proto`, DNS-SD, a
//!   Mach port and timers from a single `kevent64` call, with no wakeups
//!   while idle? (`one-wait`)
//!
//! Usage:
//!
//! ```text
//! s7-net [--profile network|none|FILE] MODE [ARGS...]
//!   ready                          the helper's footprint, doing nothing
//!   startup [--host HOST] [--path P]  one-time costs, stage by stage
//!   probe [--home DIR]             what the profile allows and denies
//!   verify [--stack rustls|nw] [HOST[:PORT]...]
//!   handshakes --stack rustls|nw --host HOST[:PORT] [--path P] [--count N] [--pause-ms M]
//!   download --stack rustls|nw --url https://HOST/PATH
//!   dns [NAME...]
//!   one-wait [--messages N] [--idle SECONDS] [--tick-ms M]
//! ```
//!
//! The profile defaults to `network` (`profiles/network.sb`), under which
//! both TLS stacks run; `none` runs the helper unsandboxed, for comparison.
//! The broker creates the helper's data directory next to the executable
//! and removes it afterwards.
//!
//! Layout:
//! - [`platform`]: every foreign call; the only module with `unsafe`.
//! - [`driver`]: the I/O thread's loop. [`tls`], [`quic`] and [`http`]: the
//!   protocol state machines it drives. [`verifier`]: rustls's verifier,
//!   through Security.
//! - [`scenarios`]: the modes.
#![deny(unsafe_code)]

mod driver;
mod http;
mod platform;
mod quic;
mod scenarios;
mod tls;
mod verifier;

use std::io::Write;
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use s4_sandbox::bench;
use s4_sandbox::platform::process::{self, Command, Exit};
use s4_sandbox::platform::sandbox;
use s4_sandbox::probe::SOCKET_FD;
use s4_sandbox::profile::RENDEZVOUS_PARAM;

/// The network process's profile: what NW17 allows, plus the rules S7
/// found necessary, each explained in the file.
const NETWORK: &str = include_str!("../profiles/network.sb");
/// The profile parameters naming the data directory and the executable's.
const DATA_DIR_PARAM: &str = "DATA_DIR";
const EXECUTABLE_DIR_PARAM: &str = "EXECUTABLE_DIR";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.first().map(String::as_str) == Some("helper") {
        helper(&args[1..])
    } else {
        broker(&args)
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("s7-net: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The broker: compile the profile, launch the helper with the bytecode on
/// descriptor 3, wait, and clean up.
fn broker(args: &[String]) -> Result<ExitCode, String> {
    let (profile, rest) = match args {
        [flag, name, rest @ ..] if flag == "--profile" => (name.as_str(), rest),
        rest => ("network", rest),
    };
    if rest.is_empty() || rest[0].starts_with('-') {
        return Err(
            "usage: s7-net [--profile NAME] MODE [ARGS...] (see the crate docs)".to_owned(),
        );
    }
    let source = match profile {
        "none" => None,
        "network" => Some(NETWORK.to_owned()),
        file => Some(std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?),
    };
    // Canonical, as profile paths must be.
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or("the executable has no directory")?
        .join(format!("s7-data-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let result = launch(&exe, &dir, profile, source.as_deref(), rest);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn launch(
    exe: &Path,
    dir: &Path,
    profile: &str,
    source: Option<&str>,
    args: &[String],
) -> Result<ExitCode, String> {
    let dir_text = dir.to_str().ok_or("the data directory is not UTF-8")?;
    let exe_dir = exe
        .parent()
        .and_then(Path::to_str)
        .ok_or("the executable's directory is not UTF-8")?;
    let bytecode = match source {
        Some(source) => sandbox::compile(
            source,
            &[
                (DATA_DIR_PARAM, dir_text),
                (EXECUTABLE_DIR_PARAM, exe_dir),
                (RENDEZVOUS_PARAM, "org.axiom.s7.rendezvous.unused"),
            ],
        )
        .and_then(|compiled| compiled.bytecode())?,
        None => Vec::new(),
    };
    println!(
        "# s7-net: profile {profile} ({} bytes of bytecode)",
        bytecode.len()
    );
    let (broker_end, helper_end) = UnixStream::pair().map_err(|e| e.to_string())?;
    let stdout = std::io::stdout();
    let mut command = Command::new(exe).map_err(|e| e.to_string())?;
    command.arg("helper").map_err(|e| e.to_string())?;
    command.arg("--data").map_err(|e| e.to_string())?;
    command.arg(dir_text).map_err(|e| e.to_string())?;
    for arg in args {
        command.arg(arg).map_err(|e| e.to_string())?;
    }
    command
        .stdout(stdout.as_fd())
        .inherit_stderr()
        .map_fd(helper_end.as_fd(), SOCKET_FD);
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let child = command.spawn().map_err(|e| format!("spawn: {e}"))?;
    drop(command);
    drop(helper_end);
    let len = u32::try_from(bytecode.len()).map_err(|e| e.to_string())?;
    (&broker_end)
        .write_all(&len.to_le_bytes())
        .and_then(|()| (&broker_end).write_all(&bytecode))
        .map_err(|e| e.to_string())?;
    let exit = child.wait().map_err(|e| e.to_string())?;
    println!("# s7-net: helper {exit:?}");
    Ok(match exit {
        Exit::Code(0) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    })
}

/// The helper: apply the profile (if any arrived), then run the mode.
fn helper(args: &[String]) -> Result<ExitCode, String> {
    let (data_dir, args) = match args {
        [flag, dir, rest @ ..] if flag == "--data" => (Some(PathBuf::from(dir)), rest),
        rest => (None, rest),
    };
    let mut socket =
        process::adopt_inherited_socket(SOCKET_FD).map_err(|e| format!("descriptor 3: {e}"))?;
    let bytecode = bench::read_bytecode(&mut socket).map_err(|e| format!("bytecode: {e}"))?;
    if !bytecode.is_empty() {
        sandbox::apply_bytecode(&bytecode)?;
    }
    println!("# helper: sandboxed {}", sandbox::is_sandboxed());
    let (mode, rest) = args.split_first().ok_or("no mode")?;
    let context = scenarios::Context::new(rest.to_vec(), data_dir);
    match mode.as_str() {
        "ready" => scenarios::ready(&context),
        "startup" => scenarios::startup(&context),
        "probe" => scenarios::probe(&context),
        "verify" => scenarios::verify(&context),
        "handshakes" => scenarios::handshakes(&context),
        "download" => scenarios::download(&context),
        "dns" => scenarios::dns(&context),
        "one-wait" => scenarios::one_wait(&context),
        other => Err(format!("unknown mode {other}")),
    }?;
    Ok(ExitCode::SUCCESS)
}
