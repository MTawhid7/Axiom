//! The helpers' side: each enters its profile first (ADR-0004 D2), looks up
//! the browser's rendezvous, says hello with a port of its own, and then
//! plays its role. Also the small exchanges the browser makes with them.

use std::time::Duration;

use crate::launch::{Helper, Helpers};
use crate::memory::End;
use crate::platform::mach::{self, MessageBuffer, Outgoing, ReceiveRight, SendRight};
use crate::platform::memory::{self, TaskVm};
use crate::protocol::{ROLE_GPU, ROLE_RENDERER, ROLE_SENDER, Reader, Writer, id};
use crate::sample::Mapped;
use crate::{memory as q1a, profiles, rights, surfaces, work};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(30));

pub(crate) fn role_code(role: &str) -> Result<u8, String> {
    match role {
        "renderer" => Ok(ROLE_RENDERER),
        "gpu" => Ok(ROLE_GPU),
        "sender" => Ok(ROLE_SENDER),
        other => Err(format!("no role named {other}")),
    }
}

pub(crate) fn this_pid() -> i32 {
    i32::try_from(std::process::id()).unwrap_or(0)
}

/// A helper's life. A `worker` enters its profile and runs one workload
/// (S16-Q3), without a rendezvous.
pub(crate) fn run(
    role: &str,
    rendezvous: &str,
    profile: &str,
    work: Option<&str>,
) -> Result<(), String> {
    profiles::enter(profile)?;
    if role == "worker" {
        return work::run(work.ok_or("--work is required")?, None);
    }
    let code = role_code(role)?;
    let browser = mach::look_up(rendezvous).map_err(|e| format!("look_up: {e}"))?;
    let port = ReceiveRight::new().map_err(|e| e.to_string())?;
    let mut hello = Writer::default();
    hello.u8(code);
    browser
        .send(Outgoing::new(id::HELLO, &hello.0), Some(&port), TIMEOUT)
        .map_err(|e| format!("hello: {e}"))?;
    match code {
        ROLE_RENDERER => q1a::renderer(&browser, &port),
        ROLE_SENDER => rights::sender(&browser, &port),
        _ => {
            // Answer samples until the browser says what to do.
            let mut buffer = MessageBuffer::new();
            let introduce = loop {
                let message = port
                    .receive(&mut buffer, Some(Duration::from_secs(60)))
                    .map_err(|e| format!("introduce: {e}"))?;
                match message.id {
                    id::INTRODUCE => break message,
                    id::SAMPLE => report_state(&browser, [0; 6])?,
                    _ => {}
                }
            };
            let mut r = Reader::new(&introduce.payload);
            let config = introduce.payload.get(1..).unwrap_or_default();
            match r.u8() {
                Some(0) => q1a::gpu(&browser, &port, config),
                Some(1) => surfaces::gpu(&browser, &port, config),
                _ => Err("an unknown configuration".to_owned()),
            }
        }
    }
}

/// In a helper: send the browser this task's VM statistics, and its
/// mappings of renderer memory.
pub(crate) fn report_state(browser: &SendRight, mapped: Mapped) -> Result<(), String> {
    let vm = memory::task_vm().unwrap_or_default();
    let mut payload = Writer::default();
    for value in vm.values().into_iter().chain(mapped) {
        payload.u64(value);
    }
    browser
        .send(Outgoing::new(id::STATE, &payload.0), None, TIMEOUT)
        .map_err(|e| format!("state: {e}"))
}

/// In the browser: a helper's own VM statistics, which only it can read, and
/// its mappings of renderer memory.
pub(crate) fn state(
    helpers: &Helpers,
    helper: &Helper,
    buffer: &mut MessageBuffer,
) -> Option<(TaskVm, Mapped)> {
    helper
        .port
        .send(Outgoing::new(id::SAMPLE, &[]), None, TIMEOUT)
        .ok()?;
    loop {
        let message = helpers.service.receive(buffer, TIMEOUT).ok()?;
        if message.id == id::STATE {
            let mut r = Reader::new(&message.payload);
            let values: Vec<u64> = std::iter::from_fn(|| r.u64()).collect();
            let vm = TaskVm::from_values(&values)?;
            let mapped = values.get(TaskVm::NAMES.len()..)?.try_into().ok()?;
            return Some((vm, mapped));
        }
    }
}

/// In the browser: end a helper, killed or told to exit, and reap it; how it
/// ended.
pub(crate) fn end(helper: &Helper, how: End) -> Result<String, String> {
    match how {
        End::Kill => helper.child.signal(9).map_err(|e| e.to_string())?,
        End::Exit => helper
            .port
            .send(Outgoing::new(id::QUIT, &[]), None, TIMEOUT)
            .map_err(|e| format!("quit: {e}"))?,
    }
    let exit = helper.child.wait().map_err(|e| e.to_string())?;
    Ok(format!("{exit:?}"))
}

/// In the browser: wait until a message with each of `ids` has arrived on the
/// rendezvous, in any order.
pub(crate) fn wait_for(
    helpers: &Helpers,
    ids: &[i32],
    buffer: &mut MessageBuffer,
) -> Result<(), String> {
    let mut missing = ids.to_vec();
    while !missing.is_empty() {
        let message = helpers
            .service
            .receive(buffer, TIMEOUT)
            .map_err(|e| format!("waiting for {missing:?}: {e}"))?;
        missing.retain(|id| *id != message.id);
    }
    Ok(())
}

/// In the browser: send `request` to a helper and wait for `reply`.
pub(crate) fn request(
    helpers: &Helpers,
    helper: &Helper,
    request: i32,
    reply: i32,
    buffer: &mut MessageBuffer,
) -> Result<mach::Received, String> {
    helper
        .port
        .send(Outgoing::new(request, &[]), None, TIMEOUT)
        .map_err(|e| format!("request {request}: {e}"))?;
    loop {
        let message = helpers
            .service
            .receive(buffer, TIMEOUT)
            .map_err(|e| format!("waiting for {reply}: {e}"))?;
        if message.id == reply {
            return Ok(message);
        }
    }
}
