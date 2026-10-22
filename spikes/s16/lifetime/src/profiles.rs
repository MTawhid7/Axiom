//! The helpers' Seatbelt profiles: S10's own files, unchanged, entered as
//! ADR-0004's D2 has it: the browser compiles each helper's profile with that
//! helper's parameters and passes the bytecode on an inherited socket, and
//! the helper applies it as its first act (the GPU process resolving its
//! cache directory just before, D2's one exception).

use std::borrow::Cow;
use std::io::Write as _;
use std::os::fd::RawFd;
use std::os::unix::net::UnixStream;

use s4_sandbox::bench;
use s4_sandbox::platform::{process, sandbox};

const GPU: &str = include_str!("../../../s10/gpu/profiles/gpu.sb");
const RENDERER: &str = include_str!("../../../s10/gpu/profiles/renderer.sb");

/// The descriptor on which a helper receives its profile's bytecode.
pub(crate) const SOCKET_FD: RawFd = 3;

/// The source of the profile named `name`; `none` has none (the helper runs
/// unsandboxed).
fn source(name: &str) -> Result<Option<Cow<'static, str>>, String> {
    Ok(Some(match name {
        "none" => return Ok(None),
        "gpu" => Cow::Borrowed(GPU),
        "renderer" => Cow::Borrowed(RENDERER),
        other => return Err(format!("no profile named {other}")),
    }))
}

/// In the browser: the bytecode of the profile named `name` for one helper,
/// compiled with the rendezvous and the paths its rules name as parameters;
/// `None` for `none`.
pub(crate) fn compile(name: &str, rendezvous: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(source) = source(name)? else {
        return Ok(None);
    };
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = executable
        .parent()
        .and_then(std::path::Path::to_str)
        .ok_or("no executable directory")?
        .to_owned();
    let user_cache = crate::platform::paths::user_cache_dir()
        .and_then(|dir| std::fs::canonicalize(dir).ok())
        .unwrap_or_default();
    let metal_cache = user_cache.join("com.apple.metal");
    let (user_cache, metal_cache) = (user_cache.to_string_lossy(), metal_cache.to_string_lossy());
    let params = [
        ("RENDEZVOUS", rendezvous),
        ("EXECUTABLE_DIR", directory.as_str()),
        ("USER_CACHE_DIR", &user_cache),
        ("METAL_CACHE", &metal_cache),
    ];
    let used: Vec<(&str, &str)> = params
        .into_iter()
        .filter(|(key, _)| source.contains(&format!("\"{key}\"")))
        .collect();
    sandbox::compile(&source, &used)
        .and_then(|compiled| compiled.bytecode())
        .map(Some)
        .map_err(|e| format!("compiling the {name} profile: {e}"))
}

/// In the browser: send a helper its profile's bytecode, length first.
pub(crate) fn send(mut socket: &UnixStream, bytecode: &[u8]) -> Result<(), String> {
    let length = u32::try_from(bytecode.len()).map_err(|_| "a profile too large")?;
    socket
        .write_all(&length.to_le_bytes())
        .and_then(|()| socket.write_all(bytecode))
        .map_err(|e| format!("sending the profile: {e}"))
}

/// In a helper, first: apply the bytecode the browser sent, unless the
/// profile is `none`. The GPU process resolves its cache directory first, as
/// S10 found it must (D2's exception).
pub(crate) fn enter(name: &str) -> Result<(), String> {
    if name == "gpu" {
        let _ = crate::platform::paths::user_cache_dir();
    }
    if name == "none" {
        return Ok(());
    }
    let mut socket = process::adopt_inherited_socket(SOCKET_FD)
        .map_err(|e| format!("the profile's socket: {e}"))?;
    let bytecode =
        bench::read_bytecode(&mut socket).map_err(|e| format!("reading the profile: {e}"))?;
    sandbox::apply_bytecode(&bytecode).map_err(|e| format!("entering the {name} profile: {e}"))
}
