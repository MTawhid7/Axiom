//! The browser: it checks in the rendezvous, launches the GPU process and the
//! renderer under their profiles, introduces the renderer to the GPU process,
//! and then either offers frames at 60 Hz, applying each layer commit to its
//! layer tree (S10-Q1), or waits for the resources to be registered and reads
//! every process's footprint (S10-Q2).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::os::fd::AsFd as _;
use std::os::unix::net::UnixStream;
use std::path::{Path as FsPath, PathBuf};
use std::time::Duration;

use s4_sandbox::platform::process::{self, Command};

use crate::platform::layers::{self, Layer};
use crate::platform::mach::{self, Attach, MessageBuffer, Outgoing, ReceiveRight, SendRight};
use crate::platform::surface::Received;
use crate::platform::time;
use crate::profiles;
use crate::protocol::{Config, Path, ROLE_GPU, ROLE_RENDERER, Reader, Writer, id};
use crate::stats::{Summary, summarize};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(10));
/// 60 Hz.
const FRAME_NS: u64 = 16_666_667;

/// What the browser runs, and under which profiles.
#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) config: Config,
    pub(crate) frames: u64,
    pub(crate) gpu_profile: String,
    pub(crate) renderer_profile: String,
    pub(crate) out: Option<PathBuf>,
}

struct Helpers {
    service: ReceiveRight,
    gpu: SendRight,
    renderer: SendRight,
    gpu_pid: i32,
    renderer_pid: i32,
    children: Vec<process::Child>,
    gpu_ready_ns: u64,
    /// The device, the shader library and the pipelines, in nanoseconds.
    gpu_startup_ns: [u64; 3],
    alignment: u32,
    renderer_iosurface: bool,
}

fn launch(options: &Options) -> Result<Helpers, String> {
    let rendezvous = format!("org.axiom.s10.{}", std::process::id());
    let service = mach::check_in(&rendezvous).map_err(|e| format!("check_in: {e}"))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let spawn = |role: &str, profile: &str| -> Result<process::Child, String> {
        // The helper's profile, compiled here (D2); it arrives on the
        // helper's inherited socket.
        let bytecode = profiles::compile(profile, &rendezvous)?;
        let (socket, helper_end) = UnixStream::pair().map_err(|e| e.to_string())?;
        let mut command = Command::new(&exe).map_err(|e| e.to_string())?;
        for arg in [role, "--rendezvous", &rendezvous, "--profile", profile] {
            command.arg(arg).map_err(|e| e.to_string())?;
        }
        // Helpers start with an empty environment (S4); the nonce that forces
        // a cold shader compile is passed on explicitly.
        if let Ok(nonce) = std::env::var("S10_SHADER_NONCE") {
            command
                .env("S10_SHADER_NONCE", &nonce)
                .map_err(|e| e.to_string())?;
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
    };
    let children = vec![
        spawn("gpu", &options.gpu_profile)?,
        spawn("renderer", &options.renderer_profile)?,
    ];
    let (gpu_pid, renderer_pid) = (children[0].id(), children[1].id());
    let mut buffer = MessageBuffer::new();
    let (mut gpu, mut renderer) = (None, None);
    let (mut gpu_ready_ns, mut alignment, mut renderer_iosurface) = (0, 0, false);
    let mut gpu_startup_ns = [0; 3];
    while gpu.is_none() || renderer.is_none() {
        let hello = service
            .receive(&mut buffer, Some(Duration::from_secs(30)))
            .map_err(|e| format!("hello: {e}"))?;
        let mut r = Reader::new(&hello.payload);
        match r.u8() {
            Some(ROLE_GPU) => {
                gpu_ready_ns = r.u64().unwrap_or(0);
                alignment = r.u32().unwrap_or(0);
                for ns in &mut gpu_startup_ns {
                    *ns = r.u64().unwrap_or(0);
                }
                gpu = hello.reply;
            }
            Some(ROLE_RENDERER) => {
                renderer_iosurface = r.u8() == Some(1);
                renderer = hello.reply;
            }
            _ => return Err("an unexpected hello".to_owned()),
        }
    }
    let (gpu, renderer) = (gpu.ok_or("no GPU process")?, renderer.ok_or("no renderer")?);
    let config = options.config.encode();
    gpu.send(Outgoing::new(id::CONFIG, &config), None, TIMEOUT)
        .map_err(|e| format!("config: {e}"))?;
    let mut introduce = Outgoing::new(id::INTRODUCE, &config);
    introduce.ports.push(Attach::Copy(&gpu));
    renderer
        .send(introduce, None, TIMEOUT)
        .map_err(|e| format!("introduce: {e}"))?;
    Ok(Helpers {
        service,
        gpu,
        renderer,
        gpu_pid,
        renderer_pid,
        children,
        gpu_ready_ns,
        gpu_startup_ns,
        alignment,
        renderer_iosurface,
    })
}

/// Ask both helpers for their statistics, then wait for them to exit.
fn finish(helpers: &Helpers, buffer: &mut MessageBuffer) -> Result<HashMap<u8, Vec<u64>>, String> {
    for helper in [&helpers.gpu, &helpers.renderer] {
        helper
            .send(Outgoing::new(id::QUIT, &[]), None, TIMEOUT)
            .map_err(|e| format!("quit: {e}"))?;
    }
    let mut stats = HashMap::new();
    while stats.len() < 2 {
        let message = helpers
            .service
            .receive(buffer, Some(Duration::from_secs(30)))
            .map_err(|e| format!("stats: {e}"))?;
        if message.id == id::STATS {
            let mut r = Reader::new(&message.payload);
            let role = r.u8().unwrap_or(0);
            let values = std::iter::from_fn(|| r.u64()).collect();
            stats.insert(role, values);
        }
    }
    for child in &helpers.children {
        child.wait().map_err(|e| e.to_string())?;
    }
    Ok(stats)
}

fn mib(bytes: u64) -> f64 {
    to_f64(bytes) / 1_048_576.0
}

#[allow(clippy::cast_precision_loss)] // measurements far below 2^52
fn to_f64(value: u64) -> f64 {
    value as f64
}

fn us(ticks: u64) -> f64 {
    to_f64(time::ns(ticks)) / 1_000.0
}

fn footprints(helpers: &Helpers) -> String {
    let browser = process::footprint(i32::try_from(std::process::id()).unwrap_or(0));
    let gpu = process::footprint(helpers.gpu_pid);
    let renderer = process::footprint(helpers.renderer_pid);
    let show = |f: std::io::Result<process::Footprint>| {
        f.map_or_else(
            |e| format!("unavailable ({e})"),
            |f| format!("{:.1} MiB", mib(f.current)),
        )
    };
    format!(
        "footprints: browser {}, GPU process {}, renderer {}\nprocesses: browser {}, GPU process {}, renderer {}",
        show(browser),
        show(gpu),
        show(renderer),
        std::process::id(),
        helpers.gpu_pid,
        helpers.renderer_pid
    )
}

fn write_out(out: Option<&FsPath>, name: &str, text: &str) -> Result<(), String> {
    if let Some(dir) = out {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// One frame's times, in ticks: 0 offered, 1 renderer received, 2 renderer
/// sent, 3 GPU process received, 4 validated, 5 rendered (the GPU finished),
/// 6 commit sent, 7 browser received, 8 applied. And what it drew.
struct Row {
    frame: u64,
    times: [u64; 9],
    gpu_seconds: f64,
    items: u32,
    tiles: u32,
}

/// S10-Q1: offer frames at 60 Hz and apply each commit to the layer tree.
pub(crate) fn hop(options: &Options) -> Result<(), String> {
    let config = options.config;
    let helpers = launch(options)?;
    let mut buffer = MessageBuffer::new();
    let root = Layer::new();
    let mut layers = Vec::new();
    for tile in 0..config.columns * config.rows {
        let layer = Layer::new();
        let points = f64::from(config.side) / 2.0;
        layer.set_frame(
            f64::from(tile % config.columns) * points,
            f64::from(tile / config.columns) * points,
            points,
            points,
        );
        root.add(&layer);
        layers.push(layer);
    }
    let mut surfaces: HashMap<u32, Received> = HashMap::new();
    let mut rows = Vec::new();
    let cpu_start = time::cpu_ns();
    let period = time::ticks(FRAME_NS);
    let mut deadline = time::now() + period;
    for frame in 0..options.frames {
        time::wait_until(deadline);
        let offered = time::now();
        let mut w = Writer::default();
        w.u64(frame).u64(offered);
        helpers
            .renderer
            .send(Outgoing::new(id::OFFER, &w.0), None, TIMEOUT)
            .map_err(|e| format!("offer: {e}"))?;
        let commit = loop {
            let message = helpers
                .service
                .receive(&mut buffer, Some(Duration::from_secs(5)))
                .map_err(|e| format!("commit for frame {frame}: {e}"))?;
            if message.id == id::COMMIT {
                break message;
            }
        };
        let browser_received = time::now();
        let mut r = Reader::new(&commit.payload);
        let _frame = r.u64();
        let mut times = [0_u64; 9];
        for slot in times.iter_mut().take(6) {
            *slot = r.u64().unwrap_or(0);
        }
        let gpu_seconds = r.f64().unwrap_or(0.0);
        let items = r.u32().unwrap_or(0);
        let shown_count = r.u32().unwrap_or(0);
        let shown: Vec<(u32, u32)> = (0..shown_count)
            .map(|_| (r.u32().unwrap_or(0), r.u32().unwrap_or(0)))
            .collect();
        let new_count = r.u32().unwrap_or(0);
        let new_ids: Vec<u32> = (0..new_count).map(|_| r.u32().unwrap_or(0)).collect();
        let committed = r.u64().unwrap_or(0);
        for (surface, port) in new_ids.iter().zip(&commit.ports) {
            let received =
                Received::look_up(port).ok_or("a surface right that names no IOSurface")?;
            surfaces.insert(*surface, received);
        }
        layers::transaction(|| {
            for (tile, surface) in &shown {
                if let (Some(layer), Some(surface)) =
                    (layers.get(*tile as usize), surfaces.get(surface))
                {
                    layer.set_contents(surface);
                }
            }
        });
        let applied = time::now();
        times[6] = committed;
        times[7] = browser_received;
        times[8] = applied;
        rows.push(Row {
            frame,
            times,
            gpu_seconds,
            items,
            tiles: shown_count,
        });
        deadline += period;
        let now = time::now();
        while deadline <= now {
            deadline += period;
        }
    }
    let browser_cpu = time::cpu_ns() - cpu_start;
    let side = u64::from(config.side);
    let footprint = format!(
        "{}\ntile surfaces: {} of {side} × {side} px, {:.1} MiB, created by the GPU process; the browser holds them as layer contents",
        footprints(&helpers),
        surfaces.len(),
        mib(surfaces.len() as u64 * side * side * 4)
    );
    let stats = finish(&helpers, &mut buffer)?;
    report_hop(options, &helpers, &rows, browser_cpu, &stats, &footprint)
}

fn report_hop(
    options: &Options,
    helpers: &Helpers,
    rows: &[Row],
    browser_cpu: u64,
    stats: &HashMap<u8, Vec<u64>>,
    footprint: &str,
) -> Result<(), String> {
    // The first frames create the tiles' surfaces; the steady state follows.
    let steady: Vec<&Row> = rows.iter().filter(|row| row.frame >= 20).collect();
    let mut text = String::new();
    let _ = writeln!(
        text,
        "S10-Q1: {} frames at 60 Hz ({} measured after 20 to warm up); {} items on the page; tiles of {} px; profiles: GPU {}, renderer {}",
        rows.len(),
        steady.len(),
        options.config.items,
        options.config.side,
        options.gpu_profile,
        options.renderer_profile
    );
    let _ = writeln!(
        text,
        "GPU process ready (device and pipelines) in {:.1} ms (device {:.1}, shader library {:.1}, pipelines {:.1}); linear-texture alignment {} bytes; renderer could create an IOSurface: {}",
        to_f64(helpers.gpu_ready_ns) / 1e6,
        to_f64(helpers.gpu_startup_ns[0]) / 1e6,
        to_f64(helpers.gpu_startup_ns[1]) / 1e6,
        to_f64(helpers.gpu_startup_ns[2]) / 1e6,
        helpers.alignment,
        helpers.renderer_iosurface
    );
    let items: Vec<f64> = steady.iter().map(|row| f64::from(row.items)).collect();
    let tiles: Vec<f64> = steady.iter().map(|row| f64::from(row.tiles)).collect();
    let _ = writeln!(
        text,
        "per update: {:.0} items (median), {:.0} tiles",
        summarize(&items).median,
        summarize(&tiles).median
    );
    stage_table(&mut text, &steady);
    let gpu_stats = stats.get(&ROLE_GPU).cloned().unwrap_or_default();
    let renderer_stats = stats.get(&ROLE_RENDERER).cloned().unwrap_or_default();
    let _ = writeln!(
        text,
        "\nCPU per frame, µs (steady state, every frame offered): browser {:.1}, GPU process {:.1}, renderer {:.1}; GPU process violations: {}",
        to_f64(browser_cpu) / 1_000.0 / to_f64(rows.len() as u64).max(1.0),
        to_f64(gpu_stats.get(1).copied().unwrap_or(0))
            / 1_000.0
            / to_f64(gpu_stats.first().copied().unwrap_or(1)).max(1.0),
        to_f64(renderer_stats.get(1).copied().unwrap_or(0))
            / 1_000.0
            / to_f64(renderer_stats.first().copied().unwrap_or(1)).max(1.0),
        gpu_stats.get(2).copied().unwrap_or(0)
    );
    let _ = writeln!(
        text,
        "the page's display list: {} items, {} bytes out of line, decoded and validated by the GPU process in {:.1} µs, and kept",
        options.config.items,
        gpu_stats.get(3).copied().unwrap_or(0),
        to_f64(gpu_stats.get(4).copied().unwrap_or(0)) / 1_000.0
    );
    let _ = writeln!(text, "{footprint}");
    print!("{text}");
    let mut csv = String::from(
        "frame,offered,renderer_received,renderer_sent,gpu_received,validated,rendered,commit_sent,browser_received,applied,gpu_seconds,items,tiles\n",
    );
    for row in rows {
        let _ = write!(csv, "{}", row.frame);
        for t in row.times {
            let _ = write!(csv, ",{}", time::ns(t));
        }
        let _ = writeln!(csv, ",{},{},{}", row.gpu_seconds, row.items, row.tiles);
    }
    write_out(options.out.as_deref(), "hop-summary.md", &text)?;
    write_out(options.out.as_deref(), "hop-frames.csv", &csv)
}

/// Each stage's latency over the steady frames, and the GPU's time.
fn stage_table(text: &mut String, steady: &[&Row]) {
    let span = |a: usize, b: usize| -> Vec<f64> {
        steady
            .iter()
            .map(|row| us(row.times[b].saturating_sub(row.times[a])))
            .collect()
    };
    let stages: [(&str, Vec<f64>); 9] = [
        ("offer → renderer", span(0, 1)),
        ("renderer: build and encode the update", span(1, 2)),
        ("renderer → GPU process", span(2, 3)),
        ("GPU process: decode and validate", span(3, 4)),
        ("GPU process: render, and wait for the GPU", span(4, 5)),
        ("GPU process: build the commit", span(5, 6)),
        ("GPU process → browser: **the hop RV1-02 adds**", span(6, 7)),
        ("browser: surfaces, contents, commit", span(7, 8)),
        ("end to end: offer → applied", span(0, 8)),
    ];
    let gpu_us: Vec<f64> = steady
        .iter()
        .map(|row| row.gpu_seconds * 1_000_000.0)
        .collect();
    let _ = writeln!(
        text,
        "\n| Stage | Median µs | p95 µs | p99 µs | Max µs |\n|---|---|---|---|---|"
    );
    let mut line = |name: &str, s: Summary| {
        let _ = writeln!(
            text,
            "| {name} | {:.1} | {:.1} | {:.1} | {:.1} |",
            s.median, s.p95, s.p99, s.max
        );
    };
    for (name, values) in &stages {
        line(name, summarize(values));
    }
    line("GPU time (command buffer)", summarize(&gpu_us));
}

/// S10-Q2: wait for the GPU process to register and draw every resource, then
/// read the footprints.
pub(crate) fn resources(options: &Options) -> Result<(), String> {
    let helpers = launch(options)?;
    let mut buffer = MessageBuffer::new();
    let ready = loop {
        let message = helpers
            .service
            .receive(&mut buffer, Some(Duration::from_secs(60)))
            .map_err(|e| format!("ready: {e}"))?;
        if message.id == id::RESOURCES_READY {
            break message;
        }
    };
    let mut r = Reader::new(&ready.payload);
    let first = r.f64().unwrap_or(0.0);
    let steady = r.f64().unwrap_or(0.0);
    let count = r.u32().unwrap_or(0);
    let registrations: Vec<f64> = (0..count)
        .map(|_| to_f64(r.u64().unwrap_or(0)) / 1_000.0)
        .collect();
    let footprint = footprints(&helpers);
    let _stats = finish(&helpers, &mut buffer)?;
    let config = options.config;
    let image_mib = mib(u64::from(config.image_side) * u64::from(config.image_side) * 4);
    let registration = summarize(&registrations);
    let text = format!(
        "S10-Q2: path {}: {} images of {}×{} ({:.1} MiB each, {:.1} MiB in all); renderer profile {}, GPU profile {}\n\
         registration per image, µs: median {:.1}, p95 {:.1}, max {:.1}\n\
         drawing every image into one tile: first {:.3} ms, then {:.3} ms at the median of 10 (GPU time)\n\
         {footprint}\n",
        config.path.name(),
        config.images,
        config.image_side,
        config.image_side,
        image_mib,
        image_mib * f64::from(config.images),
        options.renderer_profile,
        options.gpu_profile,
        registration.median,
        registration.p95,
        registration.max,
        first * 1_000.0,
        steady * 1_000.0,
    );
    print!("{text}");
    write_out(
        options.out.as_deref(),
        &format!("resources-{}.md", config.path.name()),
        &text,
    )
}

/// The renderer profile a path needs: the old path's renderer may open
/// IOSurface's user client.
pub(crate) fn renderer_profile_for(path: Path) -> &'static str {
    if path == Path::Surface {
        "renderer-iosurface"
    } else {
        "renderer"
    }
}
