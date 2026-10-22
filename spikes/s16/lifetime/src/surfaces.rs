//! S16-Q1 (b): the GPU process's tiles, IOSurface objects the browser holds
//! as layer contents, when the GPU process dies, as GR13's recovery keeps
//! "each window's last layer contents meanwhile"; and, for S16-Q2, which
//! values a surface's creator can attach that its receiver then reads.

use std::path::PathBuf;
use std::time::Duration;

use crate::helper;
use crate::launch;
use crate::memory::End;
use crate::platform::layers::{self, Layer};
use crate::platform::mach::{self, Attach, MessageBuffer, Outgoing, ReceiveRight, SendRight};
use crate::platform::metal::Gpu;
use crate::platform::surface::{Received, Tile};
use crate::protocol::{ROLE_GPU, Reader, Writer, id};
use crate::sample::{Recorder, pause};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(30));
/// The key under which the GPU process attaches a value to its last surface.
const KEY: &str = "org.axiom.s16.attached";
/// The sizes of what it attaches: a value under [`KEY`], and an ICC profile
/// that is only bytes.
const ATTACHED: usize = 64 * 1024;
const ICC: usize = 4 * 1024;

#[derive(Debug)]
pub(crate) struct Options {
    pub(crate) end: End,
    pub(crate) surfaces: u32,
    pub(crate) side: u32,
    pub(crate) out: Option<PathBuf>,
}

/// The browser's side of S16-Q1 (b).
pub(crate) fn run(o: &Options) -> Result<(), String> {
    if o.surfaces == 0 || o.surfaces as usize >= mach::MAX_PORTS {
        return Err(format!("--surfaces must be 1 to {}", mach::MAX_PORTS - 1));
    }
    let helpers = launch::launch(&[("gpu", "gpu")])?;
    let gpu = helpers.get(ROLE_GPU)?;
    let name = format!("surfaces-{}", o.end.name());
    let mut rec = Recorder::new(&name, o.out.clone());
    rec.note(format!(
        "{name}: {} tiles of {} x {} pixels, and one more carrying attached values",
        o.surfaces, o.side, o.side
    ));
    let mut buffer = MessageBuffer::new();
    let sample = |rec: &mut Recorder, stage: &str, buffer: &mut MessageBuffer, gpu_alive: bool| {
        rec.this(stage, "browser", None);
        if gpu_alive {
            let reported = helper::state(&helpers, gpu, buffer);
            rec.process(stage, "gpu", gpu.child.id(), reported.map(|s| s.0), None);
        }
        rec.host(stage);
    };
    sample(&mut rec, "start", &mut buffer, true);

    let (entries_start, _) = mach::space().map_err(|e| e.to_string())?;
    rec.note(format!(
        "port space before the tiles' rights arrive: {entries_start} entries in use"
    ));
    let mut config = Writer::default();
    config.u8(1).u32(o.surfaces).u32(o.side);
    gpu.port
        .send(Outgoing::new(id::INTRODUCE, &config.0), None, TIMEOUT)
        .map_err(|e| format!("configure the GPU process: {e}"))?;
    let commit = loop {
        let m = helpers
            .service
            .receive(&mut buffer, TIMEOUT)
            .map_err(|e| format!("waiting for the tiles: {e}"))?;
        if m.id == id::SURFACES {
            break m;
        }
    };
    let (mut surfaces, note) = look_up_all(commit.ports)?;
    rec.note(note);
    let extra = surfaces.pop().ok_or("no surfaces")?;
    rec.note(attached_values(&extra));
    // Show the tiles, as the browser's layer tree does.
    let tiles = show(&surfaces, f64::from(o.side));
    let in_use = |s: &[Received]| s.iter().filter(|s| s.is_in_use()).count();
    rec.note(format!(
        "shown: {} of {} in use; use counts {:?}; {} bytes allocated each",
        in_use(&surfaces),
        surfaces.len(),
        surfaces.iter().map(Received::use_count).collect::<Vec<_>>(),
        surfaces.first().map_or(0, Received::alloc_size)
    ));
    sample(&mut rec, "shown", &mut buffer, true);
    rec.footprint("shown", &[helper::this_pid(), gpu.child.id()]);

    // The GPU process dies; the browser keeps showing what it has.
    rec.note(format!("GPU process ended: {}", helper::end(gpu, o.end)?));
    for (stage, ms) in [("dead+0.1s", 100), ("dead+1s", 900), ("dead+5s", 4_000)] {
        pause(ms);
        sample(&mut rec, stage, &mut buffer, false);
    }
    rec.note(format!(
        "after the GPU process's death: {} of {} in use; IDs {:?}; the extra surface's ID {}",
        in_use(&surfaces),
        surfaces.len(),
        surfaces
            .iter()
            .map(Received::id)
            .take(3)
            .collect::<Vec<_>>(),
        extra.id()
    ));
    rec.footprint("dead", &[helper::this_pid()]);

    // The browser lets go.
    layers::transaction(|| {
        for layer in &tiles {
            layer.set_contents(None);
        }
    });
    drop(surfaces);
    drop(extra);
    for (stage, ms) in [("released+0.1s", 100), ("released+1s", 900)] {
        pause(ms);
        sample(&mut rec, stage, &mut buffer, false);
    }
    rec.footprint("released", &[helper::this_pid()]);
    print!("{}", rec.finish()?);
    Ok(())
}

/// What the GPU process attached to `extra`, as the browser reads it, before
/// and after the browser removes every value.
fn attached_values(extra: &Received) -> String {
    let (count, attached, icc) = extra.values(KEY);
    let before = format!(
        "values the GPU process attached, as the browser reads them: {count} values; {KEY}: {attached:?} bytes (attached {ATTACHED}); ICC profile: {icc:?} bytes (attached {ICC})"
    );
    extra.strip();
    let (count, attached, icc) = extra.values(KEY);
    format!(
        "{before}\nafter the browser removes every value: {count} values; {KEY}: {attached:?}; ICC profile: {icc:?}"
    )
}

/// One layer per surface, in a row under a root layer, each showing its
/// surface, set in one transaction.
fn show(surfaces: &[Received], side: f64) -> Vec<Layer> {
    let root = Layer::new();
    let tiles: Vec<Layer> = surfaces.iter().map(|_| Layer::new()).collect();
    layers::transaction(|| {
        for (i, (layer, surface)) in tiles.iter().zip(surfaces).enumerate() {
            layer.set_frame(f64::from(u32::try_from(i).unwrap_or(0)) * side, 0.0, side);
            layer.set_contents(Some(surface));
            root.add(layer);
        }
    });
    tiles
}

/// Look each surface up, then delete its right at once (GR12); with a note on
/// the port space before and after.
fn look_up_all(ports: Vec<SendRight>) -> Result<(Vec<Received>, String), String> {
    let entries = || {
        mach::space()
            .map(|(in_use, _)| in_use)
            .map_err(|e| e.to_string())
    };
    let before = entries()?;
    let mut surfaces = Vec::new();
    let mut after_each = Vec::new();
    for port in ports {
        surfaces.push(Received::look_up(port.name()).ok_or("a right that names no surface")?);
        drop(port);
        after_each.push(entries()?);
    }
    let note = format!(
        "port space: {before} entries in use with the tiles' rights; after each lookup and deletion: {after_each:?}"
    );
    Ok((surfaces, note))
}

/// The GPU process: make, clear and send the tiles, attach values to one
/// more, then serve the browser until told to exit or killed.
pub(crate) fn gpu(browser: &SendRight, port: &ReceiveRight, config: &[u8]) -> Result<(), String> {
    let mut r = Reader::new(config);
    let count = r.u32().ok_or("a short configuration")? as usize;
    let side = r.u32().ok_or("a short configuration")? as usize;
    let gpu = Gpu::new()?;
    let mut tiles = (0..=count)
        .map(|_| Tile::new(gpu.device(), side))
        .collect::<Result<Vec<_>, _>>()?;
    gpu.clear_all(&tiles.iter().map(Tile::texture).collect::<Vec<_>>())?;
    if let Some(last) = tiles.last() {
        last.attach(KEY, &vec![0x5a; ATTACHED]);
        last.attach_icc(&vec![0xa5; ICC]);
    }
    let mut message = Outgoing::new(id::SURFACES, &[]);
    for tile in &tiles {
        message.ports.push(Attach::Move(tile.port()));
    }
    browser
        .send(message, None, TIMEOUT)
        .map_err(|e| format!("surfaces: {e}"))?;
    let mut buffer = MessageBuffer::new();
    loop {
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(120)))
            .map_err(|e| format!("waiting: {e}"))?;
        match message.id {
            id::SAMPLE => helper::report_state(browser, [0; 6])?,
            id::RELEASE => {
                tiles.clear();
                browser
                    .send(Outgoing::new(id::RELEASED, &[]), None, TIMEOUT)
                    .map_err(|e| format!("released: {e}"))?;
            }
            id::QUIT => return Ok(()),
            _ => {}
        }
    }
}
