//! The I/O thread (NW1, EL2): one kqueue, and one `kevent64` call per turn,
//! which submits the turn's changes, arms the earliest deadline as a timer
//! with leeway (EL18), and waits. What woke it goes to DNS-SD, rustls
//! fetches, QUIC fetches or, for Mach messages and user events, back to the
//! scenario.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::ClientConfig;

use crate::http;
use crate::platform::dnssd::{self, Answer, AnswerData, Resolver};
use crate::platform::kq::{Event, Poller};
use crate::quic::QuicFetch;
use crate::tls::TlsFetch;

/// Tokens for the driver's own sources; tasks use `TASK_BASE + index`.
pub(crate) const T_DNS: u64 = 1;
pub(crate) const T_MACH: u64 = 2;
pub(crate) const T_USER: u64 = 3;
const T_TIMER: u64 = 4;
const TASK_BASE: u64 = 0x1000;

/// What a task does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Job {
    /// GET https://`host`:`port``path` over rustls.
    Tls {
        host: String,
        port: u16,
        path: String,
        keep_alive: bool,
    },
    /// GET https://`host``path` over HTTP/3.
    Quic { host: String, path: String },
    /// Resolve `name`'s addresses.
    Addresses { name: String },
    /// Fetch `name`'s HTTPS records.
    HttpsRecords { name: String },
}

pub(crate) enum State {
    Resolving,
    Tls(Box<TlsFetch>),
    Quic(Box<QuicFetch>),
    Dns {
        /// Each answer, with when it arrived.
        answers: Vec<(Duration, Answer)>,
        settled: bool,
    },
    Failed(String),
}

pub(crate) struct Task {
    pub(crate) job: Job,
    pub(crate) start: Instant,
    /// Give up if not finished by then.
    pub(crate) deadline: Instant,
    pub(crate) state: State,
}

impl Task {
    pub(crate) fn finished(&self) -> bool {
        match &self.state {
            State::Resolving => false,
            State::Tls(f) => f.finished(),
            State::Quic(q) => q.complete || q.error.is_some() || q.closed(),
            State::Dns { settled, .. } => *settled,
            State::Failed(_) => true,
        }
    }

    fn timer(&mut self) -> Option<Instant> {
        let own = (!self.finished()).then_some(self.deadline);
        let quic = match &mut self.state {
            State::Quic(q) if !q.closed() => q.deadline(),
            _ => None,
        };
        [own, quic].into_iter().flatten().min()
    }
}

/// What woke a turn, for the idle analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    Dns,
    Socket,
    Timer,
    Mach,
    User,
    Other,
}

pub(crate) struct Driver {
    pub(crate) poller: Poller,
    // Declared after `poller`: dropped after its socket is unwatched.
    resolver: Resolver,
    tls: Arc<ClientConfig>,
    quic: Option<quinn_proto::ClientConfig>,
    pub(crate) tasks: Vec<Task>,
    armed: Option<Instant>,
    events: Vec<Event>,
    /// Mach messages, user events and refused changes, for the scenario.
    pub(crate) other: Vec<(Instant, Event)>,
    /// Each turn's end and what woke it.
    pub(crate) turns: Vec<(Instant, Cause)>,
}

impl Driver {
    pub(crate) fn new(
        tls: Arc<ClientConfig>,
        quic: Option<quinn_proto::ClientConfig>,
    ) -> Result<Self, String> {
        let mut poller = Poller::new().map_err(|e| format!("kqueue: {e}"))?;
        let resolver = Resolver::new().map_err(|e| format!("DNSServiceCreateConnection: {e}"))?;
        poller.watch_read(resolver.fd(), T_DNS);
        Ok(Self {
            poller,
            resolver,
            tls,
            quic,
            tasks: Vec::new(),
            armed: None,
            events: Vec::new(),
            other: Vec::new(),
            turns: Vec::new(),
        })
    }

    /// Start `job`, giving up after `timeout`. Returns its index.
    pub(crate) fn add(&mut self, job: Job, timeout: Duration) -> usize {
        let index = self.tasks.len();
        let start = Instant::now();
        let query = index as u64;
        let started = match &job {
            Job::Tls { host, .. } | Job::Quic { host, .. } => self.resolver.addresses(query, host),
            Job::Addresses { name } => self.resolver.addresses(query, name),
            Job::HttpsRecords { name } => self.resolver.records(query, name, dnssd::TYPE_HTTPS),
        };
        let state = match (&job, started) {
            (_, Err(code)) => State::Failed(format!("DNS-SD refused the query: {code}")),
            (Job::Addresses { .. } | Job::HttpsRecords { .. }, Ok(())) => State::Dns {
                answers: Vec::new(),
                settled: false,
            },
            (_, Ok(())) => State::Resolving,
        };
        self.tasks.push(Task {
            job,
            start,
            deadline: start + timeout,
            state,
        });
        index
    }

    pub(crate) fn all_finished(&self) -> bool {
        self.tasks.iter().all(Task::finished)
    }

    /// Take task `index` out, closing its sockets when dropped; its index
    /// stays taken.
    pub(crate) fn retire(&mut self, index: usize) -> Option<Task> {
        let task = self.tasks.get_mut(index)?;
        let state = std::mem::replace(&mut task.state, State::Failed("retired".to_owned()));
        Some(Task {
            job: task.job.clone(),
            start: task.start,
            deadline: task.deadline,
            state,
        })
    }

    /// Turn until `done` holds.
    pub(crate) fn run_until(&mut self, mut done: impl FnMut(&Self) -> bool) -> Result<(), String> {
        while !done(self) {
            self.turn(None)?;
        }
        Ok(())
    }

    /// One turn: expire what is due, arm the earliest deadline (tasks' and
    /// `wake_by`), wait once, and dispatch what woke the wait.
    pub(crate) fn turn(&mut self, wake_by: Option<Instant>) -> Result<(), String> {
        let now = Instant::now();
        self.expire(now);
        let earliest = self
            .tasks
            .iter_mut()
            .filter_map(Task::timer)
            .chain(wake_by)
            .min();
        match earliest {
            Some(at) if self.armed != Some(at) => {
                let after = at.saturating_duration_since(now);
                // Leeway for coalescing: a sixteenth of the delay, within
                // 0.5–10 ms (EL18).
                let leeway =
                    (after / 16).clamp(Duration::from_micros(500), Duration::from_millis(10));
                self.poller.arm_timer(T_TIMER, after, leeway);
                self.armed = Some(at);
            }
            None if self.armed.is_some() => {
                self.poller.disarm_timer(T_TIMER);
                self.armed = None;
            }
            _ => {}
        }
        let mut events = std::mem::take(&mut self.events);
        events.clear();
        self.poller
            .wait(None, &mut events)
            .map_err(|e| format!("kevent64: {e}"))?;
        let now = Instant::now();
        let mut cause = Cause::Other;
        for event in events.drain(..) {
            cause = self.dispatch(event, now);
        }
        self.events = events;
        self.turns.push((now, cause));
        Ok(())
    }

    fn dispatch(&mut self, event: Event, now: Instant) -> Cause {
        match event {
            Event::Readable { token: T_DNS, .. } => {
                match self.resolver.process() {
                    Ok(answers) => {
                        // `kDNSServiceFlagsMoreComing` is collective on a
                        // shared connection (`dns_sd.h`): queries settle only
                        // once a batch ends without it.
                        let batch_ended = answers.last().is_some_and(|a| !a.more);
                        for answer in answers {
                            self.on_answer(answer, now);
                        }
                        if batch_ended {
                            self.settle();
                        }
                    }
                    Err(code) => self.other.push((
                        now,
                        Event::Refused {
                            token: T_DNS,
                            filter: 0,
                            errno: i64::from(code),
                        },
                    )),
                }
                Cause::Dns
            }
            Event::Timer { token: T_TIMER } => {
                self.armed = None;
                self.expire(now);
                Cause::Timer
            }
            Event::Readable { token, .. } | Event::Writable { token, .. } if token >= TASK_BASE => {
                let index = usize::try_from(token - TASK_BASE).unwrap_or(usize::MAX);
                self.on_socket(index, now);
                Cause::Socket
            }
            event => {
                let cause = match event {
                    Event::Message { .. } | Event::MessageError { .. } => Cause::Mach,
                    Event::User { .. } => Cause::User,
                    _ => Cause::Other,
                };
                self.other.push((now, event));
                cause
            }
        }
    }

    fn on_answer(&mut self, answer: Answer, now: Instant) {
        let index = usize::try_from(answer.query).unwrap_or(usize::MAX);
        let Some(task) = self.tasks.get_mut(index) else {
            return;
        };
        let elapsed = now - task.start;
        let address = match &mut task.state {
            State::Dns { answers, .. } => {
                answers.push((elapsed, answer));
                return;
            }
            State::Resolving => match (&answer.data, answer.error, answer.add) {
                (AnswerData::Address(ip), 0, true) => *ip,
                _ => return,
            },
            _ => return,
        };
        self.resolver.cancel(index as u64);
        let token = TASK_BASE + index as u64;
        let task = &mut self.tasks[index];
        task.state = match &task.job {
            Job::Tls {
                host,
                port,
                path,
                keep_alive,
            } => {
                let request = http::request(host, path, *keep_alive);
                let addr = SocketAddr::new(address, *port);
                match TlsFetch::start(Arc::clone(&self.tls), host, addr, request, task.start) {
                    Ok(fetch) => {
                        self.poller.watch_read(fetch.socket(), token);
                        self.poller.want_write(fetch.socket(), token);
                        State::Tls(Box::new(fetch))
                    }
                    Err(e) => State::Failed(e),
                }
            }
            Job::Quic { host, path } => {
                let addr = SocketAddr::new(address, 443);
                match self
                    .quic
                    .clone()
                    .map(|config| QuicFetch::start(config, host, addr, path, task.start))
                {
                    Some(Ok(fetch)) => {
                        self.poller.watch_read(fetch.socket(), token);
                        State::Quic(Box::new(fetch))
                    }
                    Some(Err(e)) => State::Failed(e),
                    None => State::Failed("no QUIC configuration".to_owned()),
                }
            }
            _ => State::Failed("unexpected job".to_owned()),
        };
    }

    /// Settle the DNS queries that have enough answers: for addresses, a
    /// positive one or negatives for both families; for records, any.
    fn settle(&mut self) {
        for (index, task) in self.tasks.iter_mut().enumerate() {
            let State::Dns { answers, settled } = &mut task.state else {
                continue;
            };
            let positive = answers.iter().any(|(_, a)| a.error == 0 && a.add);
            let negative = answers.iter().filter(|(_, a)| a.error != 0).count();
            let enough = match task.job {
                Job::Addresses { .. } => positive || negative >= 2,
                _ => !answers.is_empty(),
            };
            if enough && !*settled {
                *settled = true;
                self.resolver.cancel(index as u64);
            }
        }
    }

    fn on_socket(&mut self, index: usize, now: Instant) {
        let token = TASK_BASE + index as u64;
        let Some(task) = self.tasks.get_mut(index) else {
            return;
        };
        match &mut task.state {
            State::Tls(fetch) => {
                if fetch.finished() {
                    // A kept-alive connection the server closed, or sent
                    // something on: nothing more to do in this spike.
                    return;
                }
                if fetch.advance().write {
                    self.poller.want_write(fetch.socket(), token);
                }
            }
            State::Quic(fetch) => fetch.on_readable(now),
            _ => {}
        }
    }

    /// Fail overdue tasks, settle overdue DNS queries, and run QUIC timers.
    fn expire(&mut self, now: Instant) {
        for (index, task) in self.tasks.iter_mut().enumerate() {
            if let State::Quic(fetch) = &mut task.state {
                fetch.on_timeout(now);
            }
            if task.finished() || now < task.deadline {
                continue;
            }
            match &mut task.state {
                State::Tls(fetch) => fetch.fail("timed out".to_owned()),
                State::Quic(fetch) => {
                    fetch.error.get_or_insert("timed out".to_owned());
                }
                State::Dns { settled, .. } => {
                    *settled = true;
                    self.resolver.cancel(index as u64);
                }
                state @ State::Resolving => {
                    *state = State::Failed("timed out resolving".to_owned());
                    self.resolver.cancel(index as u64);
                }
                State::Failed(_) => {}
            }
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // `dns_sd.h`: remove the socket from the wait before deallocating
        // the connection (N7).
        let _ = self.poller.unwatch_read_now(self.resolver.fd());
    }
}
