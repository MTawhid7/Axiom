//! The GPU/compositor process (RV1-02). It enters its profile first, opens
//! Metal, registers each renderer resource by the path the run names
//! (S10-Q2), and for each display-list update: decodes and validates it
//! (PL9, S10-Q3), renders the damaged tiles into IOSurface objects it owns, waits for
//! the GPU, and sends the browser a layer commit, with a send right for each
//! surface the browser hasn't seen (S10-Q1).

use std::collections::HashMap;
use std::time::Duration;

use crate::display_list::{self, Context, DisplayList, Grid, ItemKind};
use crate::platform::mach::{self, Attach, MessageBuffer, Outgoing, ReceiveRight, SendRight};
use crate::platform::memory::Region;
use crate::platform::metal::{Gpu, ImageDraw, Quad, Resource};
use crate::platform::surface::{Received, Tile};
use crate::platform::time;
use crate::profiles;
use crate::protocol::{Config, Path, ROLE_GPU, Reader, Writer, id};
use crate::renderer::ROW_ALIGNMENT;

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(10));

/// The browser's statistics from this process, at the end.
#[derive(Debug, Default)]
struct Totals {
    frames: u64,
    violations: u64,
    cpu_start: Option<u64>,
    /// The page's display list: its bytes, and the time to decode and
    /// validate it.
    page_bytes: u64,
    page_ns: u64,
}

struct State {
    gpu: Gpu,
    config: Option<Config>,
    resources: HashMap<u32, Resource>,
    registered: Vec<u32>,
    registration_ns: Vec<u64>,
    /// Two surfaces per tile, used in turn, so the one the browser shows is
    /// never drawn into; with their IDs.
    tiles: HashMap<u32, [(u32, Tile); 2]>,
    turn: HashMap<u32, usize>,
    next_surface: u32,
    /// The whole page's display list, kept (`id::PAGE`).
    page: Option<DisplayList>,
    totals: Totals,
}

pub(crate) fn run(rendezvous: &str, profile: &str) -> Result<(), String> {
    // Before the profile, resolve the user's cache directory, which libSystem
    // then keeps for the process: Metal finds its shader caches through it,
    // and resolving it inside the profile needs opendirectoryd and dirhelper,
    // which the profile denies. Without this, every start compiles the
    // shaders again (S10).
    let _ = crate::platform::paths::user_cache_dir();
    profiles::enter(profile)?;
    let browser = mach::look_up(rendezvous).map_err(|e| format!("look_up: {e}"))?;
    let port = ReceiveRight::new().map_err(|e| e.to_string())?;
    let started = time::now();
    let gpu = Gpu::new()?;
    let ready = time::ns(time::now() - started);
    let alignment = gpu.linear_alignment();
    if !ROW_ALIGNMENT.is_multiple_of(alignment.max(1)) {
        return Err(format!("linear textures need {alignment}-byte rows"));
    }
    let mut hello = Writer::default();
    hello
        .u8(ROLE_GPU)
        .u64(ready)
        .u32(u32::try_from(alignment).unwrap_or(0));
    for ns in gpu.startup_ns {
        hello.u64(ns);
    }
    browser
        .send(Outgoing::new(id::HELLO, &hello.0), Some(&port), TIMEOUT)
        .map_err(|e| format!("hello: {e}"))?;
    let mut state = State {
        gpu,
        config: None,
        resources: HashMap::new(),
        registered: Vec::new(),
        registration_ns: Vec::new(),
        tiles: HashMap::new(),
        turn: HashMap::new(),
        next_surface: 1,
        page: None,
        totals: Totals::default(),
    };
    let mut buffer = MessageBuffer::new();
    loop {
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(60)))
            .map_err(|e| format!("receive: {e}"))?;
        match message.id {
            id::CONFIG => state.config = Config::decode(&message.payload),
            id::RESOURCE => register(&mut state, &browser, message)?,
            id::PAGE => keep_page(&mut state, &message)?,
            id::UPDATE => render(&mut state, &browser, &message)?,
            id::QUIT => {
                let cpu = time::cpu_ns() - state.totals.cpu_start.unwrap_or(0);
                let mut w = Writer::default();
                w.u8(ROLE_GPU)
                    .u64(state.totals.frames)
                    .u64(cpu)
                    .u64(state.totals.violations)
                    .u64(state.totals.page_bytes)
                    .u64(state.totals.page_ns);
                return browser
                    .send(Outgoing::new(id::STATS, &w.0), None, TIMEOUT)
                    .map_err(|e| format!("stats: {e}"));
            }
            _ => {}
        }
    }
}

fn register(state: &mut State, browser: &SendRight, message: mach::Received) -> Result<(), String> {
    let config = state.config.ok_or("a resource before the configuration")?;
    let mut r = Reader::new(&message.payload);
    let (resource, width, height, bytes_per_row, kind) = (
        r.u32().ok_or("short")?,
        r.u32().ok_or("short")? as usize,
        r.u32().ok_or("short")? as usize,
        r.u32().ok_or("short")? as usize,
        r.u8().ok_or("short")?,
    );
    let port = message
        .ports
        .into_iter()
        .next()
        .ok_or("a resource without its right")?;
    let started = time::now();
    let registered = if kind == 1 {
        let surface = Received::look_up(&port).ok_or("no IOSurface for the right")?;
        if surface.width() != width || surface.height() != height {
            return Err("the surface's size isn't the one claimed".to_owned());
        }
        state
            .gpu
            .surface_texture(surface.into_inner(), width, height)?
    } else {
        let region =
            Region::map(&port, bytes_per_row * height).map_err(|e| format!("mach_vm_map: {e}"))?;
        match config.path {
            Path::Blit => state.gpu.copied(region, width, height, bytes_per_row)?,
            Path::NoCopy | Path::Surface => {
                state.gpu.no_copy(region, width, height, bytes_per_row)?
            }
        }
    };
    state.registration_ns.push(time::ns(time::now() - started));
    state.resources.insert(resource, registered);
    state.registered.push(resource);
    state.registered.sort_unstable();
    if config.mode == 1 && state.registered.len() == config.images as usize {
        sample_all(state, browser, config)?;
    }
    Ok(())
}

/// Decode, validate and keep the whole page's display list.
fn keep_page(state: &mut State, message: &mach::Received) -> Result<(), String> {
    let started = time::now();
    let config = state.config.ok_or("a page before the configuration")?;
    let bytes = message
        .out_of_line
        .as_ref()
        .map_or(&[][..], mach::OutOfLine::bytes);
    let context = Context {
        registered: &state.registered,
        tiles: config.columns * config.rows,
    };
    let page = display_list::decode(bytes, &context).map_err(|v| format!("the page: {v}"))?;
    state.totals.page_ns = time::ns(time::now() - started);
    state.totals.page_bytes = bytes.len() as u64;
    state.page = Some(page);
    Ok(())
}

/// S10-Q2: draw every image into one tile, once and then ten times, and
/// report the GPU times with each registration's.
fn sample_all(state: &mut State, browser: &SendRight, config: Config) -> Result<(), String> {
    let side = config.side as usize;
    let target = Tile::new(state.gpu.device(), side)?;
    let quad = Quad {
        rect: [0.0, 0.0, side_f32(side), side_f32(side)],
        color: [1.0; 4],
        src: [0.0, 0.0, 1.0, 1.0],
    };
    let draws: Vec<ImageDraw<'_>> = state
        .registered
        .iter()
        .filter_map(|id| state.resources.get(id))
        .map(|resource| ImageDraw { resource, quad })
        .collect();
    let empty = state.gpu.quads(&[])?;
    let mut first = state.gpu.record()?;
    first.tile(target.texture(), [0.0, 0.0], &empty, &draws)?;
    let first_seconds = first.finish();
    let mut steady = Vec::with_capacity(10);
    for _ in 0..10 {
        let mut recording = state.gpu.record()?;
        recording.tile(target.texture(), [0.0, 0.0], &empty, &draws)?;
        steady.push(recording.finish());
    }
    steady.sort_by(f64::total_cmp);
    let mut w = Writer::default();
    w.f64(first_seconds).f64(steady[steady.len() / 2]);
    w.u32(u32::try_from(state.registration_ns.len()).unwrap_or(0));
    for ns in &state.registration_ns {
        w.u64(*ns);
    }
    browser
        .send(Outgoing::new(id::RESOURCES_READY, &w.0), None, TIMEOUT)
        .map_err(|e| format!("ready: {e}"))
}

#[allow(clippy::cast_precision_loss)] // tile sides are small
fn side_f32(side: usize) -> f32 {
    side as f32
}

fn color(rgba: u32) -> [f32; 4] {
    let channel = |shift: u32| f32::from((rgba >> shift).to_le_bytes()[0]) / 255.0;
    [channel(0), channel(8), channel(16), channel(24)]
}

/// Each item of `list`, placed by its node's translation: the solid quads,
/// and the image quads with the resource each samples.
fn to_quads(list: &DisplayList) -> (Vec<Quad>, Vec<(u32, Quad)>) {
    let translations = list.translations();
    let mut solids = Vec::with_capacity(list.items.len());
    let mut images = Vec::new();
    for item in &list.items {
        let [tx, ty] = translations[item.node as usize];
        let rect = [
            item.rect[0] + tx,
            item.rect[1] + ty,
            item.rect[2],
            item.rect[3],
        ];
        match item.kind {
            ItemKind::Rect => solids.push(Quad {
                rect,
                color: color(item.color),
                src: [0.0; 4],
            }),
            ItemKind::Image => images.push((
                item.resource,
                Quad {
                    rect,
                    color: [1.0; 4],
                    src: item.src,
                },
            )),
        }
    }
    (solids, images)
}

fn render(state: &mut State, browser: &SendRight, message: &mach::Received) -> Result<(), String> {
    let received = time::now();
    state.totals.cpu_start.get_or_insert_with(time::cpu_ns);
    let config = state.config.ok_or("an update before the configuration")?;
    let grid = Grid {
        columns: config.columns,
        rows: config.rows,
        side: config.side,
    };
    let mut r = Reader::new(&message.payload);
    let header = [r.u64(), r.u64(), r.u64(), r.u64()];
    let [frame, offered, renderer_received, renderer_sent] = header.map(|v| v.unwrap_or(0));
    let bytes = message
        .out_of_line
        .as_ref()
        .map_or(&[][..], mach::OutOfLine::bytes);
    let context = Context {
        registered: &state.registered,
        tiles: grid.tiles(),
    };
    let list = match display_list::decode(bytes, &context) {
        Ok(list) => list,
        Err(violation) => {
            // In Axiom the sender would be terminated (IP11); the spike counts.
            state.totals.violations += 1;
            eprintln!("s10 gpu: frame {frame}: {violation}");
            return Ok(());
        }
    };
    let validated = time::now();

    let (solids, images) = to_quads(&list);
    let solids = state.gpu.quads(&solids)?;
    let mut new_surfaces = Vec::new();
    for &tile in &list.damage {
        if !state.tiles.contains_key(&tile) {
            let a = Tile::new(state.gpu.device(), grid.side as usize)?;
            let b = Tile::new(state.gpu.device(), grid.side as usize)?;
            let ids = (state.next_surface, state.next_surface + 1);
            state.next_surface += 2;
            new_surfaces.push((ids.0, a.port()));
            new_surfaces.push((ids.1, b.port()));
            state.tiles.insert(tile, [(ids.0, a), (ids.1, b)]);
        }
    }
    let mut recording = state.gpu.record()?;
    let mut shown = Vec::with_capacity(list.damage.len());
    for &tile in &list.damage {
        let turn = state.turn.entry(tile).or_insert(0);
        *turn ^= 1;
        let (surface, target) = &state.tiles[&tile][*turn];
        let draws: Vec<ImageDraw<'_>> = images
            .iter()
            .filter_map(|(resource, quad)| {
                state.resources.get(resource).map(|resource| ImageDraw {
                    resource,
                    quad: *quad,
                })
            })
            .collect();
        recording.tile(target.texture(), grid.origin(tile), &solids, &draws)?;
        shown.push((tile, *surface));
    }
    let gpu_seconds = recording.finish();
    let rendered = time::now();

    let mut w = Writer::default();
    for t in [
        frame,
        offered,
        renderer_received,
        renderer_sent,
        received,
        validated,
        rendered,
    ] {
        w.u64(t);
    }
    w.f64(gpu_seconds)
        .u32(u32::try_from(list.items.len()).unwrap_or(0));
    w.u32(u32::try_from(shown.len()).unwrap_or(0));
    for (tile, surface) in &shown {
        w.u32(*tile).u32(*surface);
    }
    w.u32(u32::try_from(new_surfaces.len()).unwrap_or(0));
    for (surface, _) in &new_surfaces {
        w.u32(*surface);
    }
    let sent = time::now();
    w.u64(sent);
    let mut out = Outgoing::new(id::COMMIT, &w.0);
    for (_, port) in new_surfaces {
        out.ports.push(Attach::Move(port));
    }
    browser
        .send(out, None, TIMEOUT)
        .map_err(|e| format!("commit: {e}"))?;
    state.totals.frames += 1;
    Ok(())
}
