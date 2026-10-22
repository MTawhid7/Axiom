//! S16-Q2: port rights that arrive in messages a receiver drops, rights a
//! failed send hands back, rights in messages still queued when their port is
//! destroyed, and IOSurface lookups of names that aren't surfaces. Counted by
//! the port space's entries, a name's user references and no-senders
//! notifications. The port table itself isn't exhausted here; its limits come
//! from XNU's source.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use crate::launch;
use crate::platform::mach::{
    self, Attach, Failed, MachError, MessageBuffer, NOTIFY_NO_SENDERS, Outgoing, ReceiveRight,
    SEND_TIMED_OUT, SendRight, send_refs, space,
};
use crate::platform::surface::{Owned, Received};
use crate::protocol::{ROLE_SENDER, Reader, Writer, id};
use crate::sample::write_file;

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(30));

/// Rows of `case`, `variant`, `measure`, `value`.
#[derive(Debug, Default)]
struct Table(String);

impl Table {
    fn row(&mut self, case: &str, variant: &str, measure: &str, value: impl std::fmt::Display) {
        let _ = writeln!(self.0, "{case}\t{variant}\t{measure}\t{value}");
    }
}

fn entries() -> Result<u32, String> {
    space().map(|(in_use, _)| in_use).map_err(|e| e.to_string())
}

/// Count the messages on `port` until none arrives for `wait`, destroying
/// each; the no-senders notifications among them.
fn drain(port: &ReceiveRight, wait: Duration) -> (u32, u32) {
    let mut buffer = MessageBuffer::new();
    let (mut all, mut no_senders) = (0, 0);
    while let Ok(mut message) = port.receive_raw(&mut buffer, Some(wait)) {
        all += 1;
        if message.id() == NOTIFY_NO_SENDERS {
            no_senders += 1;
        }
        message.destroy();
    }
    (all, no_senders)
}

/// The browser's side of S16-Q2.
pub(crate) fn run(count: u32, out: Option<PathBuf>) -> Result<(), String> {
    let mut t = Table::default();
    t.row("case", "variant", "measure", "value");
    dropped(&mut t, count)?;
    returned(&mut t)?;
    queued(&mut t)?;
    lookups(&mut t)?;
    if let Some(dir) = out {
        write_file(&dir.join("rights.tsv"), &t.0)?;
    }
    print!("{}", t.0);
    Ok(())
}

/// Rights from a sandboxed sender, in messages this process receives and
/// then either leaks or destroys: each to a port of its own (`distinct`), or
/// copies of one send right (`same`).
fn dropped(t: &mut Table, count: u32) -> Result<(), String> {
    let helpers = launch::launch(&[("sender", "renderer")])?;
    let sender = helpers.get(ROLE_SENDER)?;
    let sink = ReceiveRight::new().map_err(|e| e.to_string())?;
    sink.set_queue_limit(1_024).map_err(|e| e.to_string())?;
    let to_sink = sink.make_send().map_err(|e| e.to_string())?;
    let mut buffer = MessageBuffer::new();
    for (kind, variant) in [(0_u8, "leak"), (0, "destroy"), (1, "leak"), (1, "destroy")] {
        let case = if kind == 0 {
            "dropped-distinct"
        } else {
            "dropped-same"
        };
        let before = entries()?;
        let mut request = Writer::default();
        request.u8(kind).u32(count);
        let mut message = Outgoing::new(id::INTRODUCE, &request.0);
        message.ports.push(Attach::Copy(&to_sink));
        sender
            .port
            .send(message, None, TIMEOUT)
            .map_err(|e| format!("request: {e}"))?;
        let mut names = Vec::new();
        for _ in 0..count {
            let mut raw = sink
                .receive_raw(&mut buffer, TIMEOUT)
                .map_err(|e| format!("a right: {e}"))?;
            names.extend(raw.port_names());
            if variant == "leak" {
                raw.leak();
            } else {
                raw.destroy();
            }
        }
        let after = entries()?;
        names.sort_unstable();
        names.dedup();
        t.row(case, variant, "messages", count);
        t.row(case, variant, "distinct names received", names.len());
        t.row(case, variant, "entries before", before);
        t.row(case, variant, "entries after", after);
        if let Some(&name) = names.first().filter(|_| kind == 1) {
            t.row(
                case,
                variant,
                "user references on the name",
                send_refs(name).map_or_else(|e| e.to_string(), |r| r.to_string()),
            );
        }
        let reply = crate::helper::request(&helpers, sender, id::COUNT, id::COUNT, &mut buffer)?;
        let reported = Reader::new(&reply.payload).u32().unwrap_or(u32::MAX);
        t.row(
            case,
            variant,
            "no-senders notifications at the sender",
            reported,
        );
    }
    sender
        .port
        .send(Outgoing::new(id::QUIT, &[]), None, TIMEOUT)
        .map_err(|e| format!("quit: {e}"))?;
    sender.child.wait().map_err(|e| e.to_string())?;
    Ok(())
}

/// A zero-timeout send to a full queue, as IP16's browser sends: what the
/// kernel hands back, and what leaking or destroying it leaves.
fn returned(t: &mut Table) -> Result<(), String> {
    let case = "failed-send";
    let queue = ReceiveRight::new().map_err(|e| e.to_string())?;
    queue.set_queue_limit(1).map_err(|e| e.to_string())?;
    let to_queue = queue.make_send().map_err(|e| e.to_string())?;
    let carried = ReceiveRight::new().map_err(|e| e.to_string())?;
    // Fill the queue.
    let mut first = Outgoing::new(id::RIGHT, &[]);
    first.ports.push(Attach::Move(
        carried.make_send().map_err(|e| e.to_string())?,
    ));
    to_queue
        .try_send(first, None, Some(Duration::ZERO))
        .map_err(|f| format!("the first send: {}", f.status))?;
    for variant in ["leak", "destroy"] {
        let refs = |name| send_refs(name).map_or_else(|e| e.to_string(), |r| r.to_string());
        let right = carried.make_send().map_err(|e| e.to_string())?;
        t.row(
            case,
            variant,
            "carried right's references before",
            refs(carried.name()),
        );
        t.row(
            case,
            variant,
            "destination's references before",
            refs(to_queue.name()),
        );
        t.row(case, variant, "entries before", entries()?);
        let mut message = Outgoing::new(id::RIGHT, &[]);
        message.ports.push(Attach::Move(right));
        match to_queue.try_send(message, None, Some(Duration::ZERO)) {
            Ok(()) => t.row(case, variant, "status", "sent, unexpectedly"),
            Err(Failed {
                status,
                mut message,
            }) => {
                let label = if status == MachError(SEND_TIMED_OUT) {
                    "MACH_SEND_TIMED_OUT"
                } else {
                    "another error"
                };
                t.row(case, variant, "status", format!("{status} ({label})"));
                let names = message.port_names();
                t.row(
                    case,
                    variant,
                    "names in the returned message",
                    format!("{names:?} (carried {})", carried.name()),
                );
                t.row(
                    case,
                    variant,
                    "carried right's references after the failure",
                    refs(carried.name()),
                );
                t.row(
                    case,
                    variant,
                    "destination's references after the failure",
                    refs(to_queue.name()),
                );
                if variant == "leak" {
                    message.leak();
                } else {
                    message.destroy();
                }
                t.row(
                    case,
                    variant,
                    "carried right's references at the end",
                    refs(carried.name()),
                );
                t.row(
                    case,
                    variant,
                    "destination's references at the end",
                    refs(to_queue.name()),
                );
                t.row(case, variant, "entries at the end", entries()?);
            }
        }
    }
    Ok(())
}

/// Messages carrying the only send right to each of five ports, still queued
/// when the port they wait in is destroyed.
fn queued(t: &mut Table) -> Result<(), String> {
    let case = "queued-at-destruction";
    let notify = ReceiveRight::new().map_err(|e| e.to_string())?;
    notify.set_queue_limit(64).map_err(|e| e.to_string())?;
    let queue = ReceiveRight::new().map_err(|e| e.to_string())?;
    queue.set_queue_limit(8).map_err(|e| e.to_string())?;
    let to_queue = queue.make_send().map_err(|e| e.to_string())?;
    let mut ports = Vec::new();
    for _ in 0..5 {
        let port = ReceiveRight::new().map_err(|e| e.to_string())?;
        let right = port.make_send().map_err(|e| e.to_string())?;
        port.request_no_senders(&notify)
            .map_err(|e| e.to_string())?;
        let mut message = Outgoing::new(id::RIGHT, &[]);
        message.ports.push(Attach::Move(right));
        to_queue
            .try_send(message, None, Some(Duration::ZERO))
            .map_err(|f| format!("queueing: {}", f.status))?;
        ports.push(port);
    }
    let (early, early_no_senders) = drain(&notify, Duration::from_millis(100));
    t.row(
        case,
        "-",
        "notifications while queued",
        format!("{early} ({early_no_senders} no-senders)"),
    );
    drop(to_queue);
    drop(queue);
    let (late, late_no_senders) = drain(&notify, Duration::from_millis(500));
    t.row(
        case,
        "-",
        "notifications after the queue's port is destroyed",
        format!("{late} ({late_no_senders} no-senders, of 5 ports)"),
    );
    drop(ports);
    Ok(())
}

/// `IOSurfaceLookupFromMachPort` given names that aren't surfaces, with a
/// real surface's for comparison.
fn lookups(t: &mut Table) -> Result<(), String> {
    let case = "lookup";
    let found = |name| Received::look_up(name).is_some();
    let plain = ReceiveRight::new().map_err(|e| e.to_string())?;
    let plain_right = plain.make_send().map_err(|e| e.to_string())?;
    let before = send_refs(plain_right.name()).unwrap_or(0);
    t.row(
        case,
        "a plain port's send right",
        "found a surface",
        found(plain_right.name()),
    );
    t.row(
        case,
        "a plain port's send right",
        "references before and after",
        format!("{before} {}", send_refs(plain_right.name()).unwrap_or(0)),
    );
    t.row(case, "the null name", "found a surface", found(0));
    t.row(case, "MACH_PORT_DEAD", "found a surface", found(mach::DEAD));
    let dying = ReceiveRight::new().map_err(|e| e.to_string())?;
    let dead: SendRight = dying.make_send().map_err(|e| e.to_string())?;
    drop(dying);
    t.row(case, "a dead name", "found a surface", found(dead.name()));
    let surface = Owned::new(64).ok_or("IOSurfaceCreate failed")?;
    let port = surface.port();
    let before = send_refs(port.name()).unwrap_or(0);
    t.row(
        case,
        "a surface's own right",
        "found a surface",
        found(port.name()),
    );
    t.row(
        case,
        "a surface's own right",
        "references before and after",
        format!("{before} {}", send_refs(port.name()).unwrap_or(0)),
    );
    Ok(())
}

/// The sender, a renderer under S10's profile: on request, send `count`
/// messages to the browser's sink, each carrying a send right, and report
/// how many of those rights the browser released.
pub(crate) fn sender(browser: &SendRight, port: &ReceiveRight) -> Result<(), String> {
    let notify = ReceiveRight::new().map_err(|e| e.to_string())?;
    notify.set_queue_limit(1_024).map_err(|e| e.to_string())?;
    let mut buffer = MessageBuffer::new();
    let mut kept: Vec<ReceiveRight> = Vec::new();
    loop {
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(120)))
            .map_err(|e| format!("waiting: {e}"))?;
        match message.id {
            id::INTRODUCE => {
                let mut r = Reader::new(&message.payload);
                let (kind, count) = (r.u8().unwrap_or(0), r.u32().unwrap_or(0));
                let sink = message.ports.into_iter().next().ok_or("no sink")?;
                send_batch(&sink, &notify, kind, count, &mut kept)?;
            }
            id::COUNT => {
                let (_, no_senders) = drain(&notify, Duration::from_millis(500));
                let mut reply = Writer::default();
                reply.u32(no_senders);
                browser
                    .send(Outgoing::new(id::COUNT, &reply.0), None, TIMEOUT)
                    .map_err(|e| format!("count: {e}"))?;
                kept.clear();
                drain(&notify, Duration::from_millis(100));
            }
            id::QUIT => return Ok(()),
            _ => {}
        }
    }
}

fn send_batch(
    sink: &SendRight,
    notify: &ReceiveRight,
    kind: u8,
    count: u32,
    kept: &mut Vec<ReceiveRight>,
) -> Result<(), String> {
    let watched = || -> Result<(ReceiveRight, SendRight), String> {
        let port = ReceiveRight::new().map_err(|e| e.to_string())?;
        let right = port.make_send().map_err(|e| e.to_string())?;
        port.request_no_senders(notify).map_err(|e| e.to_string())?;
        Ok((port, right))
    };
    if kind == 0 {
        for _ in 0..count {
            let (port, right) = watched()?;
            let mut message = Outgoing::new(id::RIGHT, &[]);
            message.ports.push(Attach::Move(right));
            sink.send(message, None, None)
                .map_err(|e| format!("send: {e}"))?;
            kept.push(port);
        }
    } else {
        let (port, right) = watched()?;
        for _ in 0..count {
            let mut message = Outgoing::new(id::RIGHT, &[]);
            message.ports.push(Attach::Copy(&right));
            sink.send(message, None, None)
                .map_err(|e| format!("send: {e}"))?;
        }
        drop(right);
        kept.push(port);
    }
    Ok(())
}
