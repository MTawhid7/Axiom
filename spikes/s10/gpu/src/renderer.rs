//! The renderer stand-in. It enters its profile first, checks that it can't
//! create an IOSurface (RV1-09), shares its pixels with the GPU process as
//! Mach memory entries, and answers each frame offer with a display-list
//! update for the damaged tiles, sent to the GPU process out of line.

use std::time::Duration;

use crate::display_list::{self, DisplayList, Grid, ItemKind};
use crate::platform::mach::{self, Attach, MessageBuffer, Outgoing, ReceiveRight, SendRight};
use crate::platform::memory::Region;
use crate::platform::{surface, time};
use crate::profiles;
use crate::protocol::{Config, Path, ROLE_RENDERER, Reader, Writer, id};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(10));
/// The atlas page every image item samples, in the hop.
const ATLAS: u32 = 1;
const ATLAS_SIDE: usize = 256;
/// Image rows are aligned for linear textures; 256 bytes covers Apple GPUs'
/// alignment, which the GPU process checks.
pub(crate) const ROW_ALIGNMENT: usize = 256;

pub(crate) fn run(rendezvous: &str, profile: &str) -> Result<(), String> {
    profiles::enter(profile)?;
    let browser = mach::look_up(rendezvous).map_err(|e| format!("look_up: {e}"))?;
    let port = ReceiveRight::new().map_err(|e| e.to_string())?;
    // RV1-09's check: under its profile, a renderer can't create an IOSurface.
    let iosurface = surface::create(64, 64).is_some();
    let mut hello = Writer::default();
    hello.u8(ROLE_RENDERER).u8(u8::from(iosurface));
    browser
        .send(Outgoing::new(id::HELLO, &hello.0), Some(&port), TIMEOUT)
        .map_err(|e| format!("hello: {e}"))?;
    let mut buffer = MessageBuffer::new();
    let introduce = port
        .receive(&mut buffer, Some(Duration::from_secs(30)))
        .map_err(|e| format!("introduce: {e}"))?;
    let config = Config::decode(&introduce.payload).ok_or("a bad introduction")?;
    let gpu = introduce
        .ports
        .into_iter()
        .next()
        .ok_or("no GPU process in the introduction")?;
    match config.mode {
        0 => hop(&browser, &gpu, &port, &mut buffer, config),
        _ => resources(&browser, &gpu, &port, &mut buffer, config),
    }
}

/// A pattern of pixels, so the pages are written as a decoder would write them.
fn fill(bytes: &mut [u8], seed: u8) {
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i.to_le_bytes()[0].wrapping_mul(31).wrapping_add(seed);
    }
}

fn send_entry(gpu: &SendRight, resource: u32, side: usize, seed: u8) -> Result<Region, String> {
    let bytes_per_row = (side * 4).next_multiple_of(ROW_ALIGNMENT);
    let mut region = Region::allocate(bytes_per_row * side).map_err(|e| e.to_string())?;
    fill(region.bytes_mut().ok_or("a read-only region")?, seed);
    let entry = region.entry().map_err(|e| format!("memory entry: {e}"))?;
    let side_u32 = u32::try_from(side).map_err(|_| "a side out of range")?;
    let mut w = Writer::default();
    w.u32(resource)
        .u32(side_u32)
        .u32(side_u32)
        .u32(u32::try_from(bytes_per_row).map_err(|_| "rows out of range")?)
        .u8(0);
    let mut message = Outgoing::new(id::RESOURCE, &w.0);
    message.ports.push(Attach::Move(entry));
    gpu.send(message, None, TIMEOUT)
        .map_err(|e| format!("resource: {e}"))?;
    Ok(region)
}

fn send_stats(browser: &SendRight, frames: u64, cpu: u64) -> Result<(), String> {
    let mut w = Writer::default();
    w.u8(ROLE_RENDERER).u64(frames).u64(cpu);
    browser
        .send(Outgoing::new(id::STATS, &w.0), None, TIMEOUT)
        .map_err(|e| format!("stats: {e}"))
}

/// The update for `frame`: a 2 × 2 block of tiles that moves round a 3 × 3
/// path near the middle of the viewport, with every page item that touches
/// it, recoloured, as an animation would change them.
fn update(page: &DisplayList, grid: Grid, frame: u64) -> DisplayList {
    let step = u32::try_from(frame % 9).unwrap_or(0);
    let (column, row) = (
        grid.columns / 2 - 2 + step % 3,
        grid.rows / 2 - 2 + step / 3,
    );
    let first = row * grid.columns + column;
    let damage = vec![
        first,
        first + 1,
        first + grid.columns,
        first + grid.columns + 1,
    ];
    let origin = grid.origin(first);
    let side = 2.0 * grid.origin(1)[0];
    let touches = |rect: &[f32; 4]| {
        rect[0] < origin[0] + side
            && rect[0] + rect[2] > origin[0]
            && rect[1] < origin[1] + side
            && rect[1] + rect[3] > origin[1]
    };
    let tint = u32::try_from(frame % 256).unwrap_or(0) << 16;
    let items = page
        .items
        .iter()
        .filter(|item| touches(&item.rect))
        .map(|item| {
            let mut item = *item;
            if item.kind == ItemKind::Rect {
                item.color ^= tint;
            }
            item
        })
        .collect();
    DisplayList {
        frame,
        nodes: page.nodes.clone(),
        items,
        resources: page.resources.clone(),
        damage,
    }
}

fn hop(
    browser: &SendRight,
    gpu: &SendRight,
    port: &ReceiveRight,
    buffer: &mut MessageBuffer,
    config: Config,
) -> Result<(), String> {
    let grid = Grid {
        columns: config.columns,
        rows: config.rows,
        side: config.side,
    };
    let _atlas = send_entry(gpu, ATLAS, ATLAS_SIDE, 7)?;
    let width = 2.0 * grid.origin(grid.columns / 2)[0];
    let page = display_list::page(config.items as usize, width, Some(ATLAS));
    let mut bytes = Vec::new();
    page.encode(&mut bytes);
    let mut whole = Outgoing::new(id::PAGE, &[]);
    whole.out_of_line = Some(&bytes);
    gpu.send(whole, None, TIMEOUT)
        .map_err(|e| format!("page: {e}"))?;
    let (mut frames, mut cpu_start) = (0_u64, None);
    loop {
        let message = port
            .receive(buffer, Some(Duration::from_secs(30)))
            .map_err(|e| format!("receive: {e}"))?;
        match message.id {
            id::OFFER => {
                let received = time::now();
                cpu_start.get_or_insert_with(time::cpu_ns);
                let mut r = Reader::new(&message.payload);
                let (frame, offered) = (r.u64().unwrap_or(0), r.u64().unwrap_or(0));
                bytes.clear();
                update(&page, grid, frame).encode(&mut bytes);
                let mut w = Writer::default();
                w.u64(frame).u64(offered).u64(received).u64(time::now());
                let mut out = Outgoing::new(id::UPDATE, &w.0);
                out.out_of_line = Some(&bytes);
                gpu.send(out, None, TIMEOUT)
                    .map_err(|e| format!("update: {e}"))?;
                frames += 1;
            }
            id::QUIT => {
                let cpu = time::cpu_ns() - cpu_start.unwrap_or(0);
                return send_stats(browser, frames, cpu);
            }
            _ => {}
        }
    }
}

fn resources(
    browser: &SendRight,
    gpu: &SendRight,
    port: &ReceiveRight,
    buffer: &mut MessageBuffer,
    config: Config,
) -> Result<(), String> {
    let side = config.image_side as usize;
    let mut regions = Vec::new();
    let mut surfaces = Vec::new();
    for resource in 1..=config.images {
        let seed = u8::try_from(resource % 251).unwrap_or(0);
        if config.path == Path::Surface {
            let owned =
                surface::Owned::new(side, side).ok_or("IOSurfaceCreate failed in the renderer")?;
            owned.fill(seed)?;
            let mut w = Writer::default();
            w.u32(resource)
                .u32(config.image_side)
                .u32(config.image_side)
                .u32(0)
                .u8(1);
            let mut message = Outgoing::new(id::RESOURCE, &w.0);
            message.ports.push(Attach::Move(owned.port()));
            gpu.send(message, None, TIMEOUT)
                .map_err(|e| format!("resource: {e}"))?;
            surfaces.push(owned);
        } else {
            regions.push(send_entry(gpu, resource, side, seed)?);
        }
    }
    loop {
        let message = port
            .receive(buffer, Some(Duration::from_secs(60)))
            .map_err(|e| format!("receive: {e}"))?;
        if message.id == id::QUIT {
            return send_stats(browser, 0, time::cpu_ns());
        }
    }
}
