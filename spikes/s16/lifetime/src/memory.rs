//! S16-Q1 (a): a renderer's images, shared as Mach memory entries and
//! sampled in place by a compositor, as S10-Q2's no-copy path does; and what
//! becomes of their memory when the renderer dies, killed or exiting
//! cleanly, before and after the compositor lets go. The compositor is the
//! browser itself, as at M1 (MA1), or a GPU process under S10's `gpu.sb`, as
//! from M2 (GR14).

use std::path::PathBuf;
use std::time::Duration;

use crate::helper;
use crate::launch::{self, Helpers};
use crate::platform::mach::{Attach, MessageBuffer, Outgoing, ReceiveRight, Received, SendRight};
use crate::platform::memory::{Region, page_size};
use crate::platform::metal::{Gpu, Resource};
use crate::protocol::{ROLE_GPU, ROLE_RENDERER, Reader, Writer, id};
use crate::sample::{Mapped, Recorder, pause};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(30));
/// Image rows are aligned for linear textures; 256 bytes covers Apple GPUs'
/// alignment (S10).
const ROW_ALIGNMENT: usize = 256;

/// Where the compositor runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    Browser,
    Gpu,
}

/// How the process that shared the memory ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    Kill,
    Exit,
}

impl End {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Kill => "kill",
            Self::Exit => "exit",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Options {
    pub(crate) placement: Placement,
    pub(crate) end: End,
    pub(crate) cpu_touch: bool,
    pub(crate) images: u32,
    pub(crate) side: u32,
    pub(crate) out: Option<PathBuf>,
}

/// The compositor's side, in whichever process holds it.
#[derive(Debug)]
pub(crate) struct Compositor {
    gpu: Gpu,
    resources: Vec<Resource>,
    side: usize,
}

impl Compositor {
    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            gpu: Gpu::new()?,
            resources: Vec::new(),
            side: 0,
        })
    }

    /// Register one `RESOURCE`: map its entry read-only, without a copy, and
    /// read it as a linear texture. The entry's right isn't kept: the
    /// mapping holds the memory.
    pub(crate) fn register(&mut self, message: Received) -> Result<(), String> {
        let mut r = Reader::new(&message.payload);
        let side = r.u32().ok_or("a short resource")? as usize;
        let bytes_per_row = r.u32().ok_or("a short resource")? as usize;
        let entry = message
            .ports
            .into_iter()
            .next()
            .ok_or("a resource without its entry")?;
        let region = Region::map(&entry, bytes_per_row * side).map_err(|e| e.to_string())?;
        drop(entry);
        self.resources
            .push(self.gpu.no_copy(region, side, bytes_per_row)?);
        self.side = side;
        Ok(())
    }

    /// Draw every image once, so the GPU reads every page.
    pub(crate) fn draw(&self) -> Result<(), String> {
        self.gpu.draw_all(&self.resources, self.side)
    }

    /// Read one byte of every page from the CPU too, so the pages enter this
    /// task's page tables, which sampling by the GPU alone doesn't do.
    pub(crate) fn touch(&self) -> u64 {
        let page = page_size();
        self.resources
            .iter()
            .flat_map(|r| r.region().bytes().iter().step_by(page))
            .fold(0_u64, |sum, &b| sum.wrapping_add(u64::from(b)))
    }

    pub(crate) fn release(&mut self) {
        self.resources.clear();
    }

    /// Its mappings' residency, summed over the objects they map (S16-Q1).
    pub(crate) fn mapped(&self) -> Mapped {
        let (mut resident, mut dirtied, mut swapped) = (0, 0, 0);
        let (mut low, mut high) = (u64::MAX, 0);
        for resource in &self.resources {
            if let Ok(r) = resource.region().residency() {
                resident += r.resident;
                dirtied += r.dirtied;
                swapped += r.swapped;
                low = low.min(u64::from(r.ref_count));
                high = high.max(u64::from(r.ref_count));
            }
        }
        let regions = u64::try_from(self.resources.len()).unwrap_or(0);
        [regions, resident, dirtied, swapped, low.min(high), high]
    }
}

/// The browser's side of S16-Q1 (a).
pub(crate) fn run(o: &Options) -> Result<(), String> {
    let mut specs = vec![("renderer", "renderer")];
    if o.placement == Placement::Gpu {
        specs.push(("gpu", "gpu"));
    }
    let helpers = launch::launch(&specs)?;
    let renderer = helpers.get(ROLE_RENDERER)?;
    let gpu = if o.placement == Placement::Gpu {
        Some(helpers.get(ROLE_GPU)?)
    } else {
        None
    };
    let name = format!(
        "memory-{}-{}{}",
        if gpu.is_some() { "gpu" } else { "browser" },
        o.end.name(),
        if o.cpu_touch { "-touch" } else { "" }
    );
    let mut rec = Recorder::new(&name, o.out.clone());
    rec.note(format!(
        "{name}: {} images of {} x {} pixels, rows aligned to {ROW_ALIGNMENT} bytes",
        o.images, o.side, o.side
    ));
    let mut buffer = MessageBuffer::new();

    // The compositor: this process, or the GPU process, told what to expect.
    let local = ReceiveRight::new().map_err(|e| e.to_string())?;
    let own = local.make_send().map_err(|e| e.to_string())?;
    let mut compositor = None;
    let target: &SendRight = if let Some(gpu) = gpu {
        let mut config = Writer::default();
        config.u8(0).u32(o.images).u8(u8::from(o.cpu_touch));
        gpu.port
            .send(Outgoing::new(id::INTRODUCE, &config.0), None, TIMEOUT)
            .map_err(|e| format!("configure the GPU process: {e}"))?;
        &gpu.port
    } else {
        compositor = Some(Compositor::new()?);
        &own
    };
    let compositor_pid = gpu.map_or_else(helper::this_pid, |g| g.child.id());
    let role = if gpu.is_some() { "gpu" } else { "browser" };

    sample_all(
        &mut rec,
        "start",
        &helpers,
        &mut buffer,
        Some(renderer.child.id()),
        compositor.as_ref(),
    );

    // The renderer shares its images with the compositor.
    let mut introduce = Writer::default();
    introduce.u32(o.images).u32(o.side);
    let mut message = Outgoing::new(id::INTRODUCE, &introduce.0);
    message.ports.push(Attach::Copy(target));
    renderer
        .port
        .send(message, None, TIMEOUT)
        .map_err(|e| format!("introduce the renderer: {e}"))?;
    if let Some(compositor) = compositor.as_mut() {
        for _ in 0..o.images {
            let resource = local
                .receive(&mut buffer, TIMEOUT)
                .map_err(|e| format!("a resource: {e}"))?;
            compositor.register(resource)?;
        }
        compositor.draw()?;
        if o.cpu_touch {
            rec.note(format!("touched: checksum {}", compositor.touch()));
        }
    }
    let wanted: &[i32] = if gpu.is_some() {
        &[id::SENT, id::READY]
    } else {
        &[id::SENT]
    };
    helper::wait_for(&helpers, wanted, &mut buffer)?;
    sample_all(
        &mut rec,
        "registered",
        &helpers,
        &mut buffer,
        Some(renderer.child.id()),
        compositor.as_ref(),
    );
    rec.footprint("registered", &[compositor_pid, renderer.child.id()]);

    // The renderer dies.
    rec.note(format!("renderer ended: {}", helper::end(renderer, o.end)?));
    after_death(
        &mut rec,
        &helpers,
        compositor.as_mut(),
        compositor_pid,
        &mut buffer,
    )?;
    rec.note(format!("compositor: {role}, pid {compositor_pid}"));
    if let Some(gpu) = gpu {
        helper::end(gpu, End::Exit)?;
    }
    print!("{}", rec.finish()?);
    Ok(())
}

/// After the renderer's death: sample; read every mapped page from the CPU,
/// to see whether the compositor then pays for the orphaned pages; let go;
/// sample again.
fn after_death(
    rec: &mut Recorder,
    helpers: &Helpers,
    mut compositor: Option<&mut Compositor>,
    pid: i32,
    buffer: &mut MessageBuffer,
) -> Result<(), String> {
    for (stage, ms) in [("dead+0.1s", 100), ("dead+1s", 900), ("dead+5s", 4_000)] {
        pause(ms);
        sample_all(rec, stage, helpers, buffer, None, compositor.as_deref());
    }
    rec.footprint("dead", &[pid]);
    let checksum = if let Some(c) = compositor.as_deref() {
        c.touch()
    } else {
        let gpu = helpers.get(ROLE_GPU)?;
        let reply = helper::request(helpers, gpu, id::TOUCH, id::TOUCHED, buffer)?;
        Reader::new(&reply.payload).u64().unwrap_or(0)
    };
    rec.note(format!(
        "read from the CPU after the death: checksum {checksum}"
    ));
    pause(100);
    let stage = "touched-after-death";
    sample_all(rec, stage, helpers, buffer, None, compositor.as_deref());
    rec.footprint("touched", &[pid]);
    if let Some(c) = compositor.as_deref_mut() {
        c.release();
    } else {
        helper::request(
            helpers,
            helpers.get(ROLE_GPU)?,
            id::RELEASE,
            id::RELEASED,
            buffer,
        )?;
    }
    for (stage, ms) in [("released+0.1s", 100), ("released+1s", 900)] {
        pause(ms);
        sample_all(rec, stage, helpers, buffer, None, compositor.as_deref());
    }
    rec.footprint("released", &[pid]);
    Ok(())
}

/// Sample the browser (with its mappings, if it composites), the GPU process
/// (with its own breakdown and mappings), the renderer while it lives, and
/// the host.
fn sample_all(
    rec: &mut Recorder,
    stage: &str,
    helpers: &Helpers,
    buffer: &mut MessageBuffer,
    renderer: Option<i32>,
    local: Option<&Compositor>,
) {
    rec.this(stage, "browser", local.map(Compositor::mapped));
    if let Ok(gpu) = helpers.get(ROLE_GPU) {
        let reported = helper::state(helpers, gpu, buffer);
        rec.process(
            stage,
            "gpu",
            gpu.child.id(),
            reported.map(|s| s.0),
            reported.map(|s| s.1),
        );
    }
    if let Some(pid) = renderer {
        rec.process(stage, "renderer", pid, None, None);
    }
    rec.host(stage);
}

/// A pattern of pixels, so the pages are written as a decoder would write
/// them.
fn fill(bytes: &mut [u8], seed: u8) {
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i.to_le_bytes()[0].wrapping_mul(31).wrapping_add(seed);
    }
}

/// The renderer: allocate and write each image, share it as a memory entry
/// with the compositor, tell the browser, and wait to be told to exit (or be
/// killed).
pub(crate) fn renderer(browser: &SendRight, port: &ReceiveRight) -> Result<(), String> {
    let mut buffer = MessageBuffer::new();
    let introduce = port
        .receive(&mut buffer, Some(Duration::from_secs(60)))
        .map_err(|e| format!("introduce: {e}"))?;
    let mut r = Reader::new(&introduce.payload);
    let (images, side) = (
        r.u32().ok_or("a short introduction")?,
        r.u32().ok_or("a short introduction")? as usize,
    );
    let compositor = introduce
        .ports
        .into_iter()
        .next()
        .ok_or("no compositor in the introduction")?;
    let bytes_per_row = (side * 4).next_multiple_of(ROW_ALIGNMENT);
    let mut regions = Vec::new();
    for image in 0..images {
        let mut region = Region::allocate(bytes_per_row * side).map_err(|e| e.to_string())?;
        fill(
            region.bytes_mut().ok_or("a read-only region")?,
            image.to_le_bytes()[0],
        );
        let entry = region.entry().map_err(|e| e.to_string())?;
        let mut payload = Writer::default();
        payload
            .u32(u32::try_from(side).unwrap_or(0))
            .u32(u32::try_from(bytes_per_row).unwrap_or(0));
        let mut message = Outgoing::new(id::RESOURCE, &payload.0);
        message.ports.push(Attach::Move(entry));
        compositor
            .send(message, None, TIMEOUT)
            .map_err(|e| format!("resource: {e}"))?;
        regions.push(region);
    }
    browser
        .send(Outgoing::new(id::SENT, &[]), None, TIMEOUT)
        .map_err(|e| format!("sent: {e}"))?;
    loop {
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(120)))
            .map_err(|e| format!("waiting: {e}"))?;
        if message.id == id::QUIT {
            drop(regions);
            return Ok(());
        }
    }
}

/// The GPU process as compositor: register the renderer's images as they
/// arrive, draw them, tell the browser, then serve its requests.
pub(crate) fn gpu(browser: &SendRight, port: &ReceiveRight, config: &[u8]) -> Result<(), String> {
    let mut r = Reader::new(config);
    let images = r.u32().ok_or("a short configuration")?;
    let cpu_touch = r.u8() == Some(1);
    let mut compositor = Compositor::new()?;
    let mut buffer = MessageBuffer::new();
    let mut registered = 0;
    loop {
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(120)))
            .map_err(|e| format!("waiting: {e}"))?;
        match message.id {
            id::RESOURCE => {
                compositor.register(message)?;
                registered += 1;
                if registered == images {
                    compositor.draw()?;
                    if cpu_touch {
                        std::hint::black_box(compositor.touch());
                    }
                    browser
                        .send(Outgoing::new(id::READY, &[]), None, TIMEOUT)
                        .map_err(|e| format!("ready: {e}"))?;
                }
            }
            id::RELEASE => {
                compositor.release();
                browser
                    .send(Outgoing::new(id::RELEASED, &[]), None, TIMEOUT)
                    .map_err(|e| format!("released: {e}"))?;
            }
            id::SAMPLE => helper::report_state(browser, compositor.mapped())?,
            id::TOUCH => {
                let mut reply = Writer::default();
                reply.u64(compositor.touch());
                browser
                    .send(Outgoing::new(id::TOUCHED, &reply.0), None, TIMEOUT)
                    .map_err(|e| format!("touched: {e}"))?;
            }
            id::QUIT => return Ok(()),
            _ => {}
        }
    }
}
