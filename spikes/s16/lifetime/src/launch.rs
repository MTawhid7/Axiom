//! The browser's side of launching helpers, as S10 does it (ADR-0004 D2):
//! check in a rendezvous, compile each helper's profile and pass it on an
//! inherited socket, spawn with an empty environment, and wait for each
//! helper's hello.

use std::os::fd::AsFd as _;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use s4_sandbox::platform::process::{self, Command};

use crate::platform::mach::{self, MessageBuffer, ReceiveRight, SendRight};
use crate::profiles;
use crate::protocol::Reader;

/// One launched helper.
#[derive(Debug)]
pub(crate) struct Helper {
    pub(crate) role: u8,
    pub(crate) port: SendRight,
    pub(crate) child: process::Child,
}

/// The rendezvous, and the helpers that said hello on it, in launch order.
#[derive(Debug)]
pub(crate) struct Helpers {
    pub(crate) service: ReceiveRight,
    pub(crate) list: Vec<Helper>,
}

impl Helpers {
    pub(crate) fn get(&self, role: u8) -> Result<&Helper, String> {
        self.list
            .iter()
            .find(|h| h.role == role)
            .ok_or_else(|| format!("no helper with role {role}"))
    }
}

/// Spawn `s16 helper ROLE` under `profile`, with `extra` arguments.
pub(crate) fn spawn(
    rendezvous: &str,
    role: &str,
    profile: &str,
    extra: &[&str],
) -> Result<process::Child, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytecode = profiles::compile(profile, rendezvous)?;
    let (socket, helper_end) = UnixStream::pair().map_err(|e| e.to_string())?;
    let mut command = Command::new(&exe).map_err(|e| e.to_string())?;
    for arg in [
        "helper",
        role,
        "--rendezvous",
        rendezvous,
        "--profile",
        profile,
    ] {
        command.arg(arg).map_err(|e| e.to_string())?;
    }
    for arg in extra {
        command.arg(arg).map_err(|e| e.to_string())?;
    }
    command.inherit_stderr();
    if bytecode.is_some() {
        command.map_fd(helper_end.as_fd(), profiles::SOCKET_FD);
    }
    let child = command.spawn().map_err(|e| format!("spawn {role}: {e}"))?;
    drop(command);
    drop(helper_end);
    if let Some(bytecode) = &bytecode {
        profiles::send(&socket, bytecode)?;
    }
    Ok(child)
}

/// Launch each `(role, profile)` and wait for their hellos.
pub(crate) fn launch(specs: &[(&str, &str)]) -> Result<Helpers, String> {
    let rendezvous = format!("org.axiom.s16.{}", std::process::id());
    let service = mach::check_in(&rendezvous).map_err(|e| format!("check_in: {e}"))?;
    let mut children = Vec::new();
    for (role, profile) in specs {
        children.push(spawn(&rendezvous, role, profile, &[])?);
    }
    let mut buffer = MessageBuffer::new();
    let mut hellos = Vec::new();
    while hellos.len() < specs.len() {
        let hello = service
            .receive(&mut buffer, Some(Duration::from_secs(30)))
            .map_err(|e| format!("hello: {e}"))?;
        let role = Reader::new(&hello.payload).u8().ok_or("an empty hello")?;
        let port = hello.reply.ok_or("a hello without a port")?;
        hellos.push((role, port));
    }
    // Children and hellos pair by role: each launch's roles are distinct, and
    // the role a child was spawned with is the one its hello names.
    let mut list = Vec::new();
    for ((role_name, _), child) in specs.iter().zip(children) {
        let role = crate::helper::role_code(role_name)?;
        let index = hellos
            .iter()
            .position(|(r, _)| *r == role)
            .ok_or_else(|| format!("no hello from the {role_name}"))?;
        let (_, port) = hellos.swap_remove(index);
        list.push(Helper { role, port, child });
    }
    Ok(Helpers { service, list })
}
