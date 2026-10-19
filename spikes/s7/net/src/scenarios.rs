//! The helper's modes, one per question. Each runs sandboxed or not, as the
//! broker launched it, and prints tab-separated lines (comments start with
//! `#`).

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::client::Resumption;
use rustls::crypto::CryptoProvider;
use rustls::{ClientConfig, HandshakeKind};

use crate::driver::{Driver, Job, State, T_MACH, T_USER};
use crate::http;
use crate::platform::dnssd::{AnswerData, TYPE_HTTPS};
use crate::platform::kq::Event;
use crate::platform::mach::{Port, PortSet};
use crate::platform::nw;
use crate::platform::usage::{self, Usage};
use crate::verifier::{Evaluation, SystemVerifier};

/// What every mode gets.
pub(crate) struct Context {
    pub(crate) args: Vec<String>,
    pub(crate) data_dir: Option<PathBuf>,
    provider: Arc<CryptoProvider>,
}

impl Context {
    pub(crate) fn new(args: Vec<String>, data_dir: Option<PathBuf>) -> Self {
        Self {
            args,
            data_dir,
            provider: Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        }
    }

    fn verifier(&self) -> Arc<SystemVerifier> {
        Arc::new(SystemVerifier::new(Arc::clone(&self.provider)))
    }

    /// A TLS client configuration verifying through `verifier`.
    fn tls(
        &self,
        verifier: &Arc<SystemVerifier>,
        resumption: bool,
        alpn: &[u8],
        tls13: bool,
    ) -> Result<ClientConfig, String> {
        let builder = ClientConfig::builder_with_provider(Arc::clone(&self.provider));
        let builder = if tls13 {
            builder.with_protocol_versions(&[&rustls::version::TLS13])
        } else {
            builder.with_safe_default_protocol_versions()
        }
        .map_err(|e| e.to_string())?;
        let mut config = builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::clone(verifier) as Arc<_>)
            .with_no_client_auth();
        if !resumption {
            config.resumption = Resumption::disabled();
        }
        config.alpn_protocols = vec![alpn.to_vec()];
        Ok(config)
    }

    /// A QUIC client configuration (TLS 1.3, ALPN h3, 30 s idle timeout).
    fn quic(&self, verifier: &Arc<SystemVerifier>) -> Result<quinn_proto::ClientConfig, String> {
        let tls = self.tls(verifier, true, b"h3", true)?;
        let crypto = quinn_proto::crypto::rustls::QuicClientConfig::try_from(tls)
            .map_err(|e| e.to_string())?;
        let mut config = quinn_proto::ClientConfig::new(Arc::new(crypto));
        let mut transport = quinn_proto::TransportConfig::default();
        transport.max_idle_timeout(Some(
            quinn_proto::IdleTimeout::try_from(Duration::from_secs(30))
                .map_err(|e| e.to_string())?,
        ));
        config.transport_config(Arc::new(transport));
        Ok(config)
    }

    fn option(&self, name: &str) -> Option<&str> {
        self.args
            .windows(2)
            .find(|w| w[0] == name)
            .map(|w| w[1].as_str())
    }

    fn number(&self, name: &str, default: u64) -> Result<u64, String> {
        self.option(name).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("{name} needs a number"))
        })
    }

    /// Arguments that aren't options or option values.
    fn positional(&self) -> Vec<&str> {
        let mut out = Vec::new();
        let mut skip = false;
        for arg in &self.args {
            if skip {
                skip = false;
            } else if arg.starts_with("--") {
                skip = true;
            } else {
                out.push(arg.as_str());
            }
        }
        out
    }
}

/// Which TLS stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stack {
    Rustls,
    Network,
}

fn stack(context: &Context) -> Result<Stack, String> {
    match context.option("--stack").unwrap_or("rustls") {
        "rustls" => Ok(Stack::Rustls),
        "nw" => Ok(Stack::Network),
        other => Err(format!("unknown stack {other} (rustls or nw)")),
    }
}

/// What a S7-Q1 host should do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    Verify,
    Reject,
    /// Depends on the verifier's policy (an incomplete chain, revocation).
    Either,
}

/// S7-Q1's default hosts: badssl.com's test hosts, and widely used sites.
const HOSTS: &[(&str, Expect)] = &[
    ("badssl.com", Expect::Verify),
    ("sha256.badssl.com", Expect::Verify),
    ("ecc256.badssl.com", Expect::Verify),
    ("rsa4096.badssl.com", Expect::Verify),
    ("tls-v1-2.badssl.com:1012", Expect::Verify),
    ("example.com", Expect::Verify),
    ("www.wikipedia.org", Expect::Verify),
    ("www.apple.com", Expect::Verify),
    ("www.cloudflare.com", Expect::Verify),
    ("github.com", Expect::Verify),
    ("expired.badssl.com", Expect::Reject),
    ("self-signed.badssl.com", Expect::Reject),
    ("wrong.host.badssl.com", Expect::Reject),
    ("untrusted-root.badssl.com", Expect::Reject),
    ("sha1-intermediate.badssl.com", Expect::Reject),
    ("no-subject.badssl.com", Expect::Either),
    ("incomplete-chain.badssl.com", Expect::Either),
    ("revoked.badssl.com", Expect::Either),
];

fn host_port(spec: &str) -> (String, u16) {
    match spec.rsplit_once(':') {
        Some((host, port)) if port.parse::<u16>().is_ok() => {
            (host.to_owned(), port.parse().unwrap_or(443))
        }
        _ => (spec.to_owned(), 443),
    }
}

/// `https://host[:port]/path` into its parts.
fn parse_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| format!("{url} is not an https URL"))?;
    let (authority, path) = rest
        .find('/')
        .map_or((rest, "/"), |i| (&rest[..i], &rest[i..]));
    let (host, port) = host_port(authority);
    Ok((host, port, path.to_owned()))
}

/// A TLS version and cipher suite by IANA code point.
fn suite_name(negotiated: Option<(u16, u16)>) -> String {
    let Some((version, suite)) = negotiated else {
        return "-".to_owned();
    };
    let version = match version {
        0x0304 => "TLSv1_3",
        0x0303 => "TLSv1_2",
        _ => "other",
    };
    let suite = match suite {
        0x1301 => "TLS13_AES_128_GCM_SHA256".to_owned(),
        0x1302 => "TLS13_AES_256_GCM_SHA384".to_owned(),
        0x1303 => "TLS13_CHACHA20_POLY1305_SHA256".to_owned(),
        other => format!("{other:#06x}"),
    };
    format!("{version}/{suite}")
}

fn ms(d: Option<Duration>) -> String {
    d.map_or_else(
        || "-".to_owned(),
        |d| format!("{:.3}", d.as_secs_f64() * 1e3),
    )
}

fn usage_line(mode: &str, label: &str, u: &Usage) {
    println!(
        "{mode}-usage\t{label}\tcpu-ms={:.3}\tinstructions={}\tcycles={}\tswitches={}\tidle-wakeups={}\tinterrupt-wakeups={}\tfootprint={}\tpeak-footprint={}\tthreads={}",
        u.cpu.as_secs_f64() * 1e3,
        u.instructions,
        u.cycles,
        u.switches,
        u.idle_wakeups,
        u.interrupt_wakeups,
        u.footprint,
        u.peak_footprint,
        usage::threads().map_or_else(|| "-".to_owned(), |n| n.to_string())
    );
}

fn evaluations(evaluations: &[Evaluation]) -> (String, String) {
    let took: Vec<String> = evaluations.iter().map(|e| ms(Some(e.took))).collect();
    let verdicts: Vec<String> = evaluations
        .iter()
        .map(|e| {
            e.rejection.as_ref().map_or_else(
                || {
                    format!(
                        "trusted ({} certificates{})",
                        e.certificates,
                        if e.stapled_ocsp { ", stapled OCSP" } else { "" }
                    )
                },
                |r| format!("{} {:?}", r.code, r.description),
            )
        })
        .collect();
    (took.join(","), verdicts.join("; "))
}

/// `ready`: nothing but the sandboxed process, for its footprint.
#[expect(clippy::unnecessary_wraps, reason = "every mode has one signature")]
pub(crate) fn ready(_context: &Context) -> Result<(), String> {
    usage_line("ready", "start", &Usage::now());
    Ok(())
}

/// `probe`: what the profile allows and denies.
#[expect(clippy::unnecessary_wraps, reason = "every mode has one signature")]
pub(crate) fn probe(context: &Context) -> Result<(), String> {
    println!("# probe\tcheck\tresult");
    let report = |check: &str, result: Result<String, String>| match result {
        Ok(detail) => println!("probe\t{check}\tallowed: {detail}"),
        Err(detail) => println!("probe\t{check}\tdenied: {detail}"),
    };
    let err = |e: std::io::Error| e.to_string();
    if let Some(dir) = &context.data_dir {
        let file = dir.join("probe.txt");
        report(
            "data-directory write and read",
            std::fs::write(&file, b"s7")
                .and_then(|()| std::fs::read(&file))
                .map(|b| format!("{} bytes", b.len()))
                .map_err(err),
        );
    }
    // The helper's environment is empty (S4's launcher): the broker's
    // caller names the home directory.
    let home = context.option("--home").unwrap_or("/Users").to_owned();
    report(
        "home directory listing",
        std::fs::read_dir(&home)
            .map(|d| format!("{} entries", d.count()))
            .map_err(err),
    );
    report(
        "write outside the data directory",
        std::fs::write(PathBuf::from(&home).join(".s7-probe"), b"x")
            .map(|()| "written".to_owned())
            .map_err(err),
    );
    report(
        "/etc/hosts",
        std::fs::read("/etc/hosts")
            .map(|b| format!("{} bytes", b.len()))
            .map_err(err),
    );
    report(
        "TCP listen on 127.0.0.1",
        std::net::TcpListener::bind("127.0.0.1:0")
            .map(|l| format!("{:?}", l.local_addr()))
            .map_err(err),
    );
    report(
        "UDP bind on 0.0.0.0",
        std::net::UdpSocket::bind("0.0.0.0:0")
            .map(|s| format!("{:?}", s.local_addr()))
            .map_err(err),
    );
    report(
        "launch /usr/bin/true",
        std::process::Command::new("/usr/bin/true")
            .status()
            .map(|s| s.to_string())
            .map_err(err),
    );
    for service in [
        "com.apple.windowserver.active",
        "com.apple.pasteboard.1",
        "com.apple.trustd.agent",
    ] {
        report(
            &format!("Mach lookup {service}"),
            s4_sandbox::platform::mach::look_up(service)
                .map(|_| "found".to_owned())
                .map_err(|e| e.to_string()),
        );
    }
    Ok(())
}

/// `verify`: S7-Q1. Handshakes to good and bad hosts, each verified by
/// Security (rustls) or by Network framework itself (`--stack nw`).
pub(crate) fn verify(context: &Context) -> Result<(), String> {
    let stack = stack(context)?;
    let named = context.positional();
    let hosts: Vec<(String, Expect)> = if named.is_empty() {
        HOSTS.iter().map(|&(h, e)| (h.to_owned(), e)).collect()
    } else {
        named
            .iter()
            .map(|h| ((*h).to_owned(), Expect::Either))
            .collect()
    };
    println!("# verify\tstack\thost\texpected\toutcome\tdetail\ttrust-eval-ms\ttotal-ms\tverdicts");
    let mut unexpected = 0;
    match stack {
        Stack::Rustls => {
            let verifier = context.verifier();
            let config = Arc::new(context.tls(&verifier, false, b"http/1.1", false)?);
            let mut driver = Driver::new(config, None)?;
            for (spec, expect) in &hosts {
                let (host, port) = host_port(spec);
                let index = driver.add(
                    Job::Tls {
                        host,
                        port,
                        path: "/".to_owned(),
                        keep_alive: false,
                    },
                    Duration::from_secs(20),
                );
                driver.run_until(|d| d.tasks[index].finished())?;
                let task = driver.retire(index).ok_or("no task")?;
                let (outcome, detail, total) = match &task.state {
                    State::Tls(fetch) => match &fetch.phase {
                        crate::tls::Phase::Done => (
                            "verified",
                            format!(
                                "HTTP {} {}",
                                fetch.response.status.unwrap_or(0),
                                fetch.negotiated()
                            ),
                            fetch.times.done,
                        ),
                        crate::tls::Phase::Failed(e) if fetch.times.connected.is_some() => {
                            ("rejected", e.clone(), fetch.times.done)
                        }
                        crate::tls::Phase::Failed(e) => ("failed", e.clone(), fetch.times.done),
                        phase => ("failed", format!("{phase:?}"), None),
                    },
                    State::Failed(e) => ("failed", e.clone(), None),
                    _ => ("failed", "unfinished".to_owned(), None),
                };
                let (took, verdicts) = evaluations(&verifier.take());
                unexpected += usize::from(surprising(*expect, outcome));
                println!(
                    "verify\trustls\t{spec}\t{expect:?}\t{outcome}\t{detail}\t{took}\t{}\t{verdicts}",
                    ms(total)
                );
            }
        }
        Stack::Network => {
            let queue = dispatch2::DispatchQueue::new("org.axiom.s7.nw", None);
            for (spec, expect) in &hosts {
                let (host, port) = host_port(spec);
                let response = Arc::new(Mutex::new(http::Response::default()));
                let sink = sink(&response);
                let parameters = nw::Parameters::new(false).ok_or("no parameters")?;
                let fetched = nw::fetch(
                    &queue,
                    &parameters,
                    &host,
                    port,
                    &http::request(&host, "/", false),
                    sink,
                    Duration::from_secs(20),
                );
                let response = response.lock().unwrap_or_else(PoisonError::into_inner);
                let (outcome, detail) = match (&fetched.error, fetched.ready) {
                    (None, _) => (
                        "verified",
                        format!(
                            "HTTP {} {}",
                            response.status.unwrap_or(0),
                            suite_name(fetched.negotiated)
                        ),
                    ),
                    (Some(e), None) if e.contains("tls") => ("rejected", e.clone()),
                    (Some(e), _) => ("failed", e.clone()),
                };
                unexpected += usize::from(surprising(*expect, outcome));
                println!(
                    "verify\tnw\t{spec}\t{expect:?}\t{outcome}\t{detail}\t-\t{}\t-",
                    ms(fetched.done)
                );
            }
        }
    }
    println!("# verify: {unexpected} unexpected outcomes");
    usage_line("verify", "end", &Usage::now());
    Ok(())
}

fn surprising(expect: Expect, outcome: &str) -> bool {
    match expect {
        Expect::Verify => outcome != "verified",
        Expect::Reject => outcome != "rejected",
        Expect::Either => false,
    }
}

/// A Network framework sink feeding an HTTP/1.1 response parser.
fn sink(response: &Arc<Mutex<http::Response>>) -> nw::Sink {
    let response = Arc::clone(response);
    Box::new(move |bytes: &[u8]| {
        let mut r = response.lock().unwrap_or_else(PoisonError::into_inner);
        r.feed(bytes).is_err() || r.complete()
    })
}

/// `handshakes`: S7-Q2's latency. `--count` full handshakes, then a warm-up
/// and `--count` resumed ones, to `--host` (fetching `--path`).
pub(crate) fn handshakes(context: &Context) -> Result<(), String> {
    let stack = stack(context)?;
    let (host, port) = host_port(context.option("--host").ok_or("--host is required")?);
    let path = context.option("--path").unwrap_or("/").to_owned();
    let count = context.number("--count", 10)?;
    let pause = Duration::from_millis(context.number("--pause-ms", 100)?);
    println!(
        "# handshakes\tstack\thost\tkind\ti\tresolved-ms\tconnected-ms\thandshake-ms\ttls-ms\ttrust-eval-ms\tnegotiated\toutcome\tcpu-us\tinstructions"
    );
    let before = Usage::now();
    match stack {
        Stack::Rustls => {
            for resumption in [false, true] {
                let verifier = context.verifier();
                let config = Arc::new(context.tls(&verifier, resumption, b"http/1.1", false)?);
                let mut driver = Driver::new(config, None)?;
                let label = if resumption { "resumed" } else { "full" };
                // The resumed series starts with a full handshake that
                // stores the session; it is reported as `warm-up`.
                for i in 0..count + u64::from(resumption) {
                    let start = Usage::now();
                    let index = driver.add(
                        Job::Tls {
                            host: host.clone(),
                            port,
                            path: path.clone(),
                            keep_alive: false,
                        },
                        Duration::from_secs(20),
                    );
                    driver.run_until(|d| d.tasks[index].finished())?;
                    let task = driver.retire(index).ok_or("no task")?;
                    let used = Usage::now().since(&start);
                    let (took, _) = evaluations(&verifier.take());
                    let kind = if resumption && i == 0 {
                        "warm-up"
                    } else {
                        label
                    };
                    handshake_line(&host, kind, i, &task.state, took, &used);
                    std::thread::sleep(pause);
                }
            }
        }
        Stack::Network => {
            let queue = dispatch2::DispatchQueue::new("org.axiom.s7.nw", None);
            for resumption in [false, true] {
                let label = if resumption { "resumed" } else { "full" };
                // The resumed series shares one parameters object, whose
                // TLS session cache resumes; the full series makes new ones,
                // with resumption off.
                let shared = nw::Parameters::new(true).ok_or("no parameters")?;
                for i in 0..count + u64::from(resumption) {
                    let start = Usage::now();
                    let fresh = (!resumption).then(|| nw::Parameters::new(false)).flatten();
                    let response = Arc::new(Mutex::new(http::Response::default()));
                    let fetched = nw::fetch(
                        &queue,
                        fresh.as_ref().unwrap_or(&shared),
                        &host,
                        port,
                        &http::request(&host, &path, false),
                        sink(&response),
                        Duration::from_secs(20),
                    );
                    let kind = if resumption && i == 0 {
                        "warm-up"
                    } else {
                        label
                    };
                    let used = Usage::now().since(&start);
                    let outcome = fetched.error.clone().unwrap_or_else(|| "done".to_owned());
                    println!(
                        "handshakes\tnw\t{host}\t{kind}\t{i}\t-\t-\t{}\t-\t-\t-\t{outcome}\t{}\t{}",
                        ms(fetched.ready),
                        used.cpu.as_micros(),
                        used.instructions
                    );
                    std::thread::sleep(pause);
                }
            }
        }
    }
    usage_line("handshakes", "work", &Usage::now().since(&before));
    usage_line("handshakes", "end", &Usage::now());
    Ok(())
}

/// One rustls handshake's line.
fn handshake_line(host: &str, kind: &str, i: u64, state: &State, took: String, used: &Usage) {
    match state {
        State::Tls(fetch) => {
            let t = fetch.times;
            let tls = t
                .handshaken
                .zip(t.connected)
                .map(|(h, c)| h.saturating_sub(c));
            let outcome = match &fetch.phase {
                crate::tls::Phase::Done => format!(
                    "{:?}",
                    fetch.handshake_kind().unwrap_or(HandshakeKind::Full)
                ),
                phase => format!("{phase:?}"),
            };
            println!(
                "handshakes\trustls\t{host}\t{kind}\t{i}\t{}\t{}\t{}\t{}\t{}\t{}\t{outcome}\t{}\t{}",
                ms(t.resolved),
                ms(t.connected),
                ms(t.handshaken),
                ms(tls),
                if took.is_empty() {
                    "-".to_owned()
                } else {
                    took
                },
                fetch.negotiated(),
                used.cpu.as_micros(),
                used.instructions
            );
        }
        State::Failed(e) => {
            println!("handshakes\trustls\t{host}\t{kind}\t{i}\t-\t-\t-\t-\t-\t-\tfailed: {e}");
        }
        _ => {}
    }
}

/// `download`: S7-Q2's CPU per megabyte: one download of `--url`, after a
/// warm-up fetch of `--warmup` (default `/`) on the same host, so that
/// one-time initialization (Security's, the crypto provider's) is not
/// counted.
pub(crate) fn download(context: &Context) -> Result<(), String> {
    let stack = stack(context)?;
    let (host, port, path) = parse_url(context.option("--url").ok_or("--url is required")?)?;
    let warmup = context.option("--warmup").unwrap_or("/").to_owned();
    println!(
        "# download\tstack\thost\tbytes\twall-ms\tcpu-ms\tcpu-ms-per-MB\tinstructions-per-byte\tcycles-per-byte\tMB/s\toutcome"
    );
    let mut fetcher = Fetcher::new(context, stack)?;
    let (_, _, warm) = fetcher.fetch(&host, port, &warmup);
    println!("# download: warm-up {warmup}: {warm}");
    let before = Usage::now();
    let (bytes, wall, outcome) = fetcher.fetch(&host, port, &path);
    let used = Usage::now().since(&before);
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte counts stay far below 2^53"
    )]
    let (mb, per_byte) = (bytes as f64 / 1e6, |n: u64| {
        if bytes == 0 {
            0.0
        } else {
            n as f64 / bytes as f64
        }
    });
    let cpu_ms = used.cpu.as_secs_f64() * 1e3;
    println!(
        "download\t{}\t{host}\t{bytes}\t{}\t{cpu_ms:.3}\t{:.3}\t{:.2}\t{:.2}\t{:.2}\t{outcome}",
        if stack == Stack::Rustls {
            "rustls"
        } else {
            "nw"
        },
        ms(wall),
        if mb > 0.0 { cpu_ms / mb } else { 0.0 },
        per_byte(used.instructions),
        per_byte(used.cycles),
        wall.map_or(0.0, |w| mb / w.as_secs_f64().max(1e-9)),
    );
    usage_line("download", "work", &used);
    usage_line("download", "end", &Usage::now());
    Ok(())
}

/// Sequential fetches with one stack, its state kept between them.
enum Fetcher {
    Rustls(Box<Driver>),
    Network(dispatch2::DispatchRetained<dispatch2::DispatchQueue>),
}

impl Fetcher {
    fn new(context: &Context, stack: Stack) -> Result<Self, String> {
        Ok(match stack {
            Stack::Rustls => {
                let verifier = context.verifier();
                let config = Arc::new(context.tls(&verifier, false, b"http/1.1", false)?);
                Self::Rustls(Box::new(Driver::new(config, None)?))
            }
            Stack::Network => Self::Network(dispatch2::DispatchQueue::new("org.axiom.s7.nw", None)),
        })
    }

    /// GET `path`: the body's size, the time from the handshake's end to
    /// the response's, and how it went.
    fn fetch(&mut self, host: &str, port: u16, path: &str) -> (u64, Option<Duration>, String) {
        match self {
            Self::Rustls(driver) => {
                let index = driver.add(
                    Job::Tls {
                        host: host.to_owned(),
                        port,
                        path: path.to_owned(),
                        keep_alive: false,
                    },
                    Duration::from_secs(300),
                );
                if let Err(e) = driver.run_until(|d| d.tasks[index].finished()) {
                    return (0, None, e);
                }
                let Some(task) = driver.retire(index) else {
                    return (0, None, "no task".to_owned());
                };
                match &task.state {
                    State::Tls(fetch) => (
                        fetch.response.body_bytes,
                        fetch
                            .times
                            .done
                            .zip(fetch.times.handshaken)
                            .map(|(d, h)| d.saturating_sub(h)),
                        format!(
                            "{:?} HTTP {} {}",
                            fetch.phase,
                            fetch.response.status.unwrap_or(0),
                            fetch.negotiated()
                        ),
                    ),
                    State::Failed(e) => (0, None, e.clone()),
                    _ => (0, None, "unfinished".to_owned()),
                }
            }
            Self::Network(queue) => {
                let Some(parameters) = nw::Parameters::new(false) else {
                    return (0, None, "no parameters".to_owned());
                };
                let response = Arc::new(Mutex::new(http::Response::default()));
                let fetched = nw::fetch(
                    queue,
                    &parameters,
                    host,
                    port,
                    &http::request(host, path, false),
                    sink(&response),
                    Duration::from_secs(300),
                );
                let r = response.lock().unwrap_or_else(PoisonError::into_inner);
                (
                    r.body_bytes,
                    fetched
                        .done
                        .zip(fetched.ready)
                        .map(|(d, h)| d.saturating_sub(h)),
                    fetched.error.clone().unwrap_or_else(|| {
                        format!(
                            "HTTP {} {}",
                            r.status.unwrap_or(0),
                            suite_name(fetched.negotiated)
                        )
                    }),
                )
            }
        }
    }
}

/// The S7-Q3 defaults: names to resolve, and names with HTTPS records to
/// fetch (and one, example.com, that may have none).
const ADDRESS_NAMES: &[&str] = &[
    "www.apple.com",
    "www.wikipedia.org",
    "example.com",
    "github.com",
    "cloudflare.com",
];
const HTTPS_NAMES: &[&str] = &[
    "cloudflare.com",
    "crypto.cloudflare.com",
    "www.google.com",
    "example.com",
];

/// `dns`: S7-Q3. Addresses and HTTPS records, all queries at once, from the
/// one wait.
pub(crate) fn dns(context: &Context) -> Result<(), String> {
    let verifier = context.verifier();
    let config = Arc::new(context.tls(&verifier, false, b"http/1.1", false)?);
    let mut driver = Driver::new(config, None)?;
    let named = context.positional();
    let before = Usage::now();
    let threads_before = usage::threads();
    let start = Instant::now();
    if named.is_empty() {
        for name in ADDRESS_NAMES {
            driver.add(
                Job::Addresses {
                    name: (*name).to_owned(),
                },
                Duration::from_secs(10),
            );
        }
        for name in HTTPS_NAMES {
            driver.add(
                Job::HttpsRecords {
                    name: (*name).to_owned(),
                },
                Duration::from_secs(10),
            );
        }
    } else {
        for name in &named {
            driver.add(
                Job::Addresses {
                    name: (*name).to_owned(),
                },
                Duration::from_secs(10),
            );
            driver.add(
                Job::HttpsRecords {
                    name: (*name).to_owned(),
                },
                Duration::from_secs(10),
            );
        }
    }
    let threads_during = usage::threads();
    driver.run_until(Driver::all_finished)?;
    let elapsed = start.elapsed();
    let used = Usage::now().since(&before);
    println!("# dns\tquery\tname\tfirst-answer-ms\tanswers");
    for task in &driver.tasks {
        let (kind, name) = match &task.job {
            Job::Addresses { name } => ("addresses", name),
            Job::HttpsRecords { name } => ("https", name),
            _ => continue,
        };
        let State::Dns { answers, .. } = &task.state else {
            continue;
        };
        let first = answers.first().map(|(t, _)| *t);
        let shown: Vec<String> = answers
            .iter()
            .map(|(_, a)| match (&a.data, a.error) {
                (AnswerData::Address(ip), 0) => ip.to_string(),
                (AnswerData::Record(TYPE_HTTPS, rdata), 0) => svcb(rdata),
                (AnswerData::Address(IpAddr::V4(_)), e) => format!("no IPv4 ({e})"),
                (AnswerData::Address(IpAddr::V6(_)), e) => format!("no IPv6 ({e})"),
                (_, e) => format!("none ({e})"),
            })
            .collect();
        println!("dns\t{kind}\t{name}\t{}\t{}", ms(first), shown.join(" | "));
    }
    println!(
        "dns-loop\twall-ms={:.3}\tkevent64-returns={}\tthreads-before={}\tthreads-during={}\tthreads-after={}\tcpu-ms={:.3}",
        elapsed.as_secs_f64() * 1e3,
        driver.poller.waits,
        threads_before.map_or(0, |n| n),
        threads_during.map_or(0, |n| n),
        usage::threads().map_or(0, |n| n),
        used.cpu.as_secs_f64() * 1e3
    );
    usage_line("dns", "end", &Usage::now());
    Ok(())
}

/// An HTTPS record's RDATA (RFC 9460 §2.2): priority, target, parameters.
fn svcb(rdata: &[u8]) -> String {
    let mut at = 2;
    let Some(priority) = rdata.get(..2).map(|b| u16::from_be_bytes([b[0], b[1]])) else {
        return "malformed".to_owned();
    };
    let mut target = Vec::new();
    while let Some(&len) = rdata.get(at) {
        at += 1;
        if len == 0 {
            break;
        }
        let Some(label) = rdata.get(at..at + usize::from(len)) else {
            return "malformed".to_owned();
        };
        target.push(String::from_utf8_lossy(label).into_owned());
        at += usize::from(len);
    }
    let mut parts = vec![
        format!("priority={priority}"),
        format!("target={}.", target.join(".")),
    ];
    while let (Some(key), Some(len)) = (rdata.get(at..at + 2), rdata.get(at + 2..at + 4)) {
        let key = u16::from_be_bytes([key[0], key[1]]);
        let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
        at += 4;
        let Some(value) = rdata.get(at..at + len) else {
            return "malformed".to_owned();
        };
        at += len;
        parts.push(match key {
            1 => {
                let mut ids = Vec::new();
                let mut i = 0;
                while let Some(&n) = value.get(i) {
                    ids.push(
                        String::from_utf8_lossy(
                            value.get(i + 1..i + 1 + usize::from(n)).unwrap_or_default(),
                        )
                        .into_owned(),
                    );
                    i += 1 + usize::from(n);
                }
                format!("alpn={}", ids.join(","))
            }
            3 if len == 2 => format!("port={}", u16::from_be_bytes([value[0], value[1]])),
            4 => format!(
                "ipv4hint={}",
                value
                    .chunks_exact(4)
                    .map(|c| IpAddr::from([c[0], c[1], c[2], c[3]]).to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            5 => format!("ech={}B", value.len()),
            6 => format!(
                "ipv6hint={}",
                value
                    .chunks_exact(16)
                    .map(|c| <[u8; 16]>::try_from(c)
                        .map_or_else(|_| String::new(), |o| IpAddr::from(o).to_string()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            key => format!("key{key}={}B", value.len()),
        });
    }
    parts.join(" ")
}

/// `one-wait`: S7-Q4. One I/O thread, one `kevent64` per turn, driving a
/// rustls fetch, a QUIC fetch, DNS-SD, a Mach port fed by another thread,
/// a periodic timer and a user event; then `--idle` seconds idle.
#[expect(clippy::too_many_lines, reason = "one scenario, reported in one place")]
pub(crate) fn one_wait(context: &Context) -> Result<(), String> {
    let messages = context.number("--messages", 50)?;
    let idle = Duration::from_secs(context.number("--idle", 10)?);
    let tick = Duration::from_millis(context.number("--tick-ms", 100)?);
    let verifier = context.verifier();
    let tls = Arc::new(context.tls(&verifier, true, b"http/1.1", false)?);
    let quic = context.quic(&verifier)?;
    let mut driver = Driver::new(tls, Some(quic))?;

    let set = PortSet::new().map_err(|e| format!("port set: {e}"))?;
    let port = Port::new().map_err(|e| format!("port: {e}"))?;
    set.insert(&port).map_err(|e| format!("insert: {e}"))?;
    driver.poller.watch_port_set(&set, T_MACH);
    driver.poller.watch_user(T_USER);
    let sender = port.make_send().map_err(|e| format!("send right: {e}"))?;
    let waker = driver.poller.waker();

    let threads_before = usage::threads();
    let start = Instant::now();
    let tls_task = driver.add(
        Job::Tls {
            host: "www.cloudflare.com".to_owned(),
            port: 443,
            path: "/cdn-cgi/trace".to_owned(),
            keep_alive: true,
        },
        Duration::from_secs(20),
    );
    let download_task = driver.add(
        Job::Tls {
            host: "speed.cloudflare.com".to_owned(),
            port: 443,
            path: "/__down?bytes=2000000".to_owned(),
            keep_alive: true,
        },
        Duration::from_secs(60),
    );
    let quic_task = driver.add(
        Job::Quic {
            host: "cloudflare-quic.com".to_owned(),
            path: "/".to_owned(),
        },
        Duration::from_secs(20),
    );
    let dns_task = driver.add(
        Job::HttpsRecords {
            name: "cloudflare.com".to_owned(),
        },
        Duration::from_secs(10),
    );

    // The browser's stand-in: posts `messages` Mach messages, 20 ms apart,
    // each carrying its sequence number and send time, then triggers the
    // user event, and exits.
    let origin = Instant::now();
    let stand_in = std::thread::spawn(move || {
        for seq in 0..messages {
            let stamp = u64::try_from(origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let mut body = [0_u8; 16];
            body[..8].copy_from_slice(&seq.to_le_bytes());
            body[8..].copy_from_slice(&stamp.to_le_bytes());
            if sender.send(0x5337, &body).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        waker.wake(T_USER).is_ok()
    });

    let mut received = 0_u64;
    let mut latencies = Vec::new();
    let mut user_seen = None;
    let mut ticks = 0_u64;
    let mut lateness = Vec::new();
    let mut next_tick = start + tick;
    let active_end;
    loop {
        driver.turn(Some(next_tick))?;
        let now = Instant::now();
        if now >= next_tick {
            ticks += 1;
            lateness.push(now - next_tick);
            next_tick += tick;
        }
        for (at, event) in driver.other.drain(..) {
            match event {
                Event::Message { body, .. } if body.len() >= 16 => {
                    received += 1;
                    let posted = u64::from_le_bytes(body[8..16].try_into().unwrap_or_default());
                    let at = u64::try_from((at - origin).as_nanos()).unwrap_or(0);
                    latencies.push(Duration::from_nanos(at.saturating_sub(posted)));
                }
                Event::User { .. } => user_seen = Some(at - start),
                other => println!("# one-wait: other event {other:?}"),
            }
        }
        if driver.all_finished() && received >= messages && user_seen.is_some() {
            active_end = now;
            break;
        }
        if now - start > Duration::from_secs(90) {
            return Err("the active phase did not finish in 90 s".to_owned());
        }
    }
    let active_turns = driver.turns.len();
    let threads_active = usage::threads();
    let stand_in_ok = stand_in.join().unwrap_or(false);

    // Settle, then the idle window: nothing to do, the connections open.
    let settle_end = active_end + Duration::from_secs(1);
    while Instant::now() < settle_end {
        driver.turn(Some(settle_end))?;
    }
    let idle_start_turns = driver.turns.len();
    let idle_before = Usage::now();
    let idle_start = Instant::now();
    let idle_end = idle_start + idle;
    while Instant::now() < idle_end {
        driver.turn(Some(idle_end))?;
    }
    let idle_used = Usage::now().since(&idle_before);
    let idle_turns: Vec<_> = driver.turns[idle_start_turns..].to_vec();
    // The last turn is the window's own end.
    let woke: Vec<String> = idle_turns
        .iter()
        .take(idle_turns.len().saturating_sub(1))
        .map(|(at, cause)| {
            format!(
                "{:.3}s:{cause:?}",
                at.saturating_duration_since(idle_start).as_secs_f64()
            )
        })
        .collect();

    println!("# one-wait\tcomponent\tresult");
    for (index, label) in [
        (tls_task, "rustls"),
        (download_task, "rustls-download"),
        (quic_task, "quic"),
        (dns_task, "dns-sd"),
    ] {
        let line = match &driver.tasks[index].state {
            State::Tls(f) => format!(
                "{:?} HTTP {} body={} resolved={} handshaken={} done={} {}",
                f.phase,
                f.response.status.unwrap_or(0),
                f.response.body_bytes,
                ms(f.times.resolved),
                ms(f.times.handshaken),
                ms(f.times.done),
                f.negotiated()
            ),
            State::Quic(q) => format!(
                "HTTP/3 {} body={} complete={} error={:?} resolved={} connected={} done={} rtt={} datagrams-in={} out={}",
                q.status.unwrap_or(0),
                q.body_bytes,
                q.complete,
                q.error,
                ms(q.times.resolved),
                ms(q.times.connected),
                ms(q.times.done),
                ms(Some(q.rtt())),
                q.datagrams_in,
                q.datagrams_out
            ),
            State::Dns { answers, .. } => {
                let shown: Vec<String> = answers
                    .iter()
                    .map(|(t, a)| match &a.data {
                        AnswerData::Record(TYPE_HTTPS, rdata) if a.error == 0 => {
                            format!("{} at {}", svcb(rdata), ms(Some(*t)))
                        }
                        _ => format!("error {} at {}", a.error, ms(Some(*t))),
                    })
                    .collect();
                shown.join(" | ")
            }
            State::Failed(e) => format!("failed: {e}"),
            State::Resolving => "unresolved".to_owned(),
        };
        println!("one-wait\t{label}\t{line}");
    }
    let max_latency = latencies.iter().max().copied();
    let mean = |v: &[Duration]| {
        let n = u32::try_from(v.len()).unwrap_or(u32::MAX).max(1);
        v.iter().sum::<Duration>() / n
    };
    println!(
        "one-wait\tmach\treceived={received}/{messages} mean-latency-ms={} max-latency-ms={} sender-ok={stand_in_ok}",
        ms(Some(mean(&latencies))),
        ms(max_latency)
    );
    println!("one-wait\tuser-event\tseen-at-ms={}", ms(user_seen));
    println!(
        "one-wait\ttimer\tticks={ticks} mean-lateness-ms={} max-lateness-ms={}",
        ms(Some(mean(&lateness))),
        ms(lateness.iter().max().copied())
    );
    let (took, verdicts) = evaluations(&verifier.take());
    println!("one-wait\ttrust\teval-ms={took}\t{verdicts}");
    println!(
        "one-wait\tloop\tturns-active={active_turns} active-ms={:.3} threads-before={} threads-active={}",
        (active_end - start).as_secs_f64() * 1e3,
        threads_before.unwrap_or(0),
        threads_active.unwrap_or(0)
    );
    println!(
        "one-wait\tidle\tseconds={}\tturns={}\tidle-wakeups={}\tinterrupt-wakeups={}\tcpu-ms={:.3}\tthreads={}\twoke={}",
        idle.as_secs(),
        idle_turns.len(),
        idle_used.idle_wakeups,
        idle_used.interrupt_wakeups,
        idle_used.cpu.as_secs_f64() * 1e3,
        usage::threads().unwrap_or(0),
        if woke.is_empty() {
            "none".to_owned()
        } else {
            woke.join(",")
        }
    );
    if let State::Quic(q) = &mut driver.tasks[quic_task].state {
        q.close(Instant::now());
    }
    usage_line("one-wait", "end", &Usage::now());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_records_render() {
        // priority 1, target ".", alpn h3,h2, ipv4hint 104.16.132.229
        let rdata = [
            0, 1, 0, 0, 1, 0, 6, 2, b'h', b'3', 2, b'h', b'2', 0, 4, 0, 4, 104, 16, 132, 229,
        ];
        assert_eq!(
            svcb(&rdata),
            "priority=1 target=. alpn=h3,h2 ipv4hint=104.16.132.229"
        );
        assert_eq!(
            parse_url("https://a.b:8443/x?y=1").unwrap(),
            ("a.b".to_owned(), 8443, "/x?y=1".to_owned())
        );
        assert_eq!(
            parse_url("https://a.b").unwrap(),
            ("a.b".to_owned(), 443, "/".to_owned())
        );
    }
}
