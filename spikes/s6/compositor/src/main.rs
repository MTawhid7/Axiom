//! # Spike S6: compositing on Metal
//!
//! Questions (Step 5 tracker, §10.2; thresholds confirmed at Gate B):
//! - **S6-Q1, compositing cost:** page-shaped display lists drawn by Axiom's
//!   own pipelines (GR3) into tiled IOSurface layers of a 2,560 × 1,600
//!   window. What do a full redraw and a typical update cost per frame on the
//!   GPU and the compositor thread? Does an idle window cost nothing?
//! - **S6-Q2, damage:** for a spinner and for typing, what does redrawing only
//!   damaged tiles save over full redraws?
//! - **S6-Q3, scrolling and animation by layers:** does the compositor thread
//!   stay within 1 ms a frame, drawing only newly exposed tiles?
//! - **S6-Q4, paths:** CPU filling uploaded as images, against GPU filling in
//!   the compositor: time, memory, quality.
//! - **S6-Q5, memory and colour:** what do the device, pipelines and tiles cost
//!   for one window? Does CoreAnimation honour Display P3 tagging?
//!
//! Usage: `s6-compositor MODE [options]`
//! - `q1 [--frames N] [--updates N] [--idle S]`
//! - `q2 --animation spinner|typing --damage tiles|full [--seconds S]`
//! - `q3 --motion scroll|animate [--seconds S]`
//! - `q4 [--repeat N]` (offscreen: no window)
//! - `q5 [--show S]`
//!
//! Common options: `--tile N` (tile side in pixels, default 256) and
//! `--trace FILE` (per-frame samples as TSV). The window is 1,280 × 800
//! points, 2,560 × 1,600 pixels on this Mac's built-in display; it should stay
//! visible during a run.
#![deny(unsafe_code)]

mod compositor;
mod paths;
mod platform;
mod scene;
mod stats;
mod text;
mod tiles;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;

use crate::compositor::{Atlases, Redraw, Tiled, to_f64};
use crate::platform::app::{App, Wait};
use crate::platform::clock::{self, Usage};
use crate::platform::gpu::{Draw, Format, Glyph, Gpu, Image, Kind, Rect};
use crate::platform::layers::{self, Layer};
use crate::platform::surface::{Surface, Tag};
use crate::scene::{Dynamic, FIELD_SLOTS, IMAGE_ATLAS, Page, SIZES};
use crate::stats::Summary;
use crate::text::{ATLAS_SIDE, Atlas};

const TITLE: &str = "Axiom S6: compositing on Metal";
/// The window, in points.
const WINDOW_POINTS: (f64, f64) = (1280.0, 800.0);
/// The page's length, in pixels.
const PAGE_HEIGHT: f32 = 24_000.0;
/// One frame at 60 Hz.
const FRAME: Duration = Duration::from_nanos(16_666_667);
/// Rows of tiles kept beyond the viewport, above and below (the display port).
const PORT_MARGIN: f32 = 512.0;
/// What is typed into the field, one character per keystroke.
const TYPED: &str = "Damage limited to one field costs a tile or two per keystroke. ";

#[derive(Clone, Debug, PartialEq)]
struct Options {
    tile: usize,
    frames: usize,
    updates: usize,
    idle: f64,
    seconds: f64,
    repeat: usize,
    show: f64,
    animation: String,
    damage: String,
    motion: String,
    trace: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            tile: 256,
            frames: 60,
            updates: 300,
            idle: 10.0,
            seconds: 10.0,
            repeat: 5,
            show: 0.0,
            animation: "spinner".to_owned(),
            damage: "tiles".to_owned(),
            motion: "scroll".to_owned(),
            trace: None,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().cloned() else {
        eprintln!("usage: s6-compositor q1|q2|q3|q4|q5 [options]");
        return ExitCode::from(2);
    };
    let options = match parse(&args[1..]) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("s6-compositor: {message}");
            return ExitCode::from(2);
        }
    };
    let result = match mode.as_str() {
        "q1" => q1(&options),
        "q2" => q2(&options),
        "q3" => q3(&options),
        "q4" => q4(&options),
        "q5" => q5(&options),
        other => Err(format!("unknown mode {other}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s6-compositor: {message}");
            ExitCode::FAILURE
        }
    }
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let number = || value.parse::<f64>().map_err(|e| format!("{flag}: {e}"));
        let count = || value.parse::<usize>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--tile" => options.tile = count()?,
            "--frames" => options.frames = count()?,
            "--updates" => options.updates = count()?,
            "--idle" => options.idle = number()?,
            "--seconds" => options.seconds = number()?,
            "--repeat" => options.repeat = count()?,
            "--show" => options.show = number()?,
            "--animation" => options.animation.clone_from(value),
            "--damage" => options.damage.clone_from(value),
            "--motion" => options.motion.clone_from(value),
            "--trace" => options.trace = Some(value.clone()),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    if !(64..=2048).contains(&options.tile) {
        return Err("--tile must be 64–2048".to_owned());
    }
    Ok(options)
}

/// The window's content, in pixels, and its scale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Viewport {
    width: f32,
    height: f32,
    scale: f64,
}

/// Open the window, run `scenario` on a compositor thread with the window's
/// content layer, and pump events until it finishes. Returns the app, for
/// measurements after the scenario.
fn with_window<F>(scenario: F) -> Result<App, String>
where
    F: FnOnce(Layer, Viewport) -> Result<(), String> + Send + 'static,
{
    let mtm = MainThreadMarker::new().ok_or("not on the main thread")?;
    let app = App::new(mtm, TITLE, WINDOW_POINTS.0, WINDOW_POINTS.1)?;
    // Let the window server show the window and report its occlusion.
    let shown = Instant::now() + Duration::from_millis(300);
    while let Some(left) = shown.checked_duration_since(Instant::now()) {
        app.pump(Wait::For(left.as_secs_f64()));
    }
    let scale = app.scale();
    let (w, h) = app.content_points();
    let viewport = Viewport {
        width: to_f32(w * scale),
        height: to_f32(h * scale),
        scale,
    };
    let (ordered_in, unoccluded) = app.visibility();
    println!(
        "# window {}x{} px, scale {scale}, ordered in {ordered_in}, unoccluded {unoccluded}; {}",
        viewport.width,
        viewport.height,
        app.frames()
    );
    let content = app.content_layer();
    let compositor = std::thread::Builder::new()
        .name("compositor".to_owned())
        .spawn(move || scenario(content.into_layer(), viewport))
        .map_err(|e| e.to_string())?;
    while !compositor.is_finished() {
        app.pump(Wait::For(0.25));
    }
    compositor
        .join()
        .map_err(|_| "the compositor thread panicked".to_owned())??;
    let (ordered_in, unoccluded) = app.visibility();
    println!("# at the end: ordered in {ordered_in}, unoccluded {unoccluded}");
    Ok(app)
}

/// The page and its frames on the GPU, in the window's layer tree.
struct Stage {
    gpu: Gpu,
    atlas: Atlas,
    atlases: Atlases,
    page: Tiled,
    frame: Tiled,
    inner: Tiled,
    dynamic: Dynamic,
    viewport: Viewport,
    /// The page's scroll offset, in pixels.
    scroll: f32,
    typed: usize,
}

impl Stage {
    fn new(content: &Layer, viewport: Viewport, tile: usize) -> Result<Self, String> {
        Self::reporting(content, viewport, tile, &|_| {})
    }

    /// [`Stage::new`], calling `stage` after each step with its name.
    fn reporting(
        content: &Layer,
        viewport: Viewport,
        tile: usize,
        stage: &dyn Fn(&str),
    ) -> Result<Self, String> {
        let gpu = Gpu::new()?;
        stage("device and pipelines");
        let atlas = Atlas::new(&SIZES)?;
        let atlases = Atlases {
            glyphs: gpu.texture(Format::R8, ATLAS_SIDE, ATLAS_SIDE, &atlas.pixels)?,
            images: gpu.texture(
                Format::Rgba8,
                IMAGE_ATLAS,
                IMAGE_ATLAS,
                &scene::image_atlas(),
            )?,
        };
        stage("glyph and image atlases (renderer resources)");
        let Page {
            list,
            dynamic,
            frame,
            frame_slot,
            inner,
            inner_slot,
        } = scene::page(&atlas, PAGE_HEIGHT);
        let scale = viewport.scale;
        let page = Tiled::new(&gpu, list, tile, scale, Tag::DisplayP3)?;
        let frame = Tiled::new(&gpu, frame, tile, scale, Tag::DisplayP3)?;
        let inner = Tiled::new(&gpu, inner, tile, scale, Tag::DisplayP3)?;
        stage("display lists: items and bins on the GPU");
        let clip = Layer::new(scale);
        layers::transaction(|| {
            clip.set_frame(
                0.0,
                0.0,
                f64::from(viewport.width) / scale,
                f64::from(viewport.height) / scale,
            );
            clip.set_clips(true);
            content.add(&clip);
            clip.add(&page.container);
            page.container.add(&frame.container);
            frame.container.set_origin(
                f64::from(frame_slot.x) / scale,
                f64::from(frame_slot.y) / scale,
            );
            frame.container.add(&inner.container);
            inner.container.set_origin(
                f64::from(inner_slot.x) / scale,
                f64::from(inner_slot.y) / scale,
            );
        });
        // CoreAnimation retains the clip layer as `content`'s sublayer.
        drop(clip);
        Ok(Self {
            gpu,
            atlas,
            atlases,
            page,
            frame,
            inner,
            dynamic,
            viewport,
            scroll: 0.0,
            typed: 0,
        })
    }

    /// The page tiles in the display port at the current scroll offset.
    fn port(&self) -> Vec<usize> {
        self.page.grid.rows_between(
            self.scroll - PORT_MARGIN,
            self.scroll + self.viewport.height + PORT_MARGIN,
        )
    }

    /// The page tiles the viewport shows.
    fn visible(&self) -> Vec<usize> {
        self.page
            .grid
            .rows_between(self.scroll, self.scroll + self.viewport.height)
    }

    /// Redraw every tile on screen, in the page and both frames.
    fn redraw_all(&mut self) -> Result<Redraw, String> {
        let visible = self.visible();
        let a = self.page.redraw(&self.gpu, &self.atlases, &visible)?;
        let frame_tiles: Vec<usize> = (0..self.frame.grid.count()).collect();
        let b = self.frame.redraw(&self.gpu, &self.atlases, &frame_tiles)?;
        let inner_tiles: Vec<usize> = (0..self.inner.grid.count()).collect();
        let c = self.inner.redraw(&self.gpu, &self.atlases, &inner_tiles)?;
        Ok(sum(&[a, b, c]))
    }

    /// Type the next character into the field: its glyphs and caret change.
    /// Returns the damage.
    fn type_one(&mut self) -> Result<[f32; 4], String> {
        let field = self.dynamic.field;
        let chars: Vec<char> = TYPED.chars().collect();
        self.typed += 1;
        let mut text: String = (0..self.typed).map(|i| chars[i % chars.len()]).collect();
        if self.atlas.width(&text, scene::BODY) > field[2] - 60.0
            || text.chars().count() > FIELD_SLOTS
        {
            self.typed = 1;
            text = chars[0].to_string();
        }
        let mut glyphs = Vec::with_capacity(FIELD_SLOTS);
        let (x, baseline) = (field[0] + 24.0, field[1] + 48.0);
        self.atlas.line(
            &text,
            scene::BODY,
            x,
            baseline,
            scene::rgba(28, 30, 36, 1.0),
            &mut glyphs,
        );
        glyphs.truncate(FIELD_SLOTS);
        let hidden = Glyph {
            rect: [x, field[1] + 16.0, 1.0, 1.0],
            ..Glyph::default()
        };
        glyphs.resize(FIELD_SLOTS, hidden);
        self.page.set_glyphs(self.dynamic.field_glyphs, &glyphs)?;
        let caret_x = x + self.atlas.width(&text, scene::BODY) + 2.0;
        let mut caret: Rect = self.page.list.rects[self.dynamic.caret];
        caret.bounds[0] = caret_x;
        self.page.set_rect(self.dynamic.caret, caret)?;
        Ok(field)
    }

    /// Turn the spinner. Returns the damage.
    fn spin(&mut self, radians: f32) -> Result<[f32; 4], String> {
        let mut spinner = self.page.list.rects[self.dynamic.spinner];
        spinner.extra[1] = radians;
        self.page.set_rect(self.dynamic.spinner, spinner)?;
        Ok(self.dynamic.spinner_box)
    }
}

fn sum(parts: &[Redraw]) -> Redraw {
    let mut total = Redraw::default();
    for p in parts {
        total.tiles += p.tiles;
        total.allocated += p.allocated;
        total.gpu.seconds += p.gpu.seconds;
        total.gpu.passes += p.gpu.passes;
    }
    total
}

/// One frame's measurements.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sample {
    tiles: usize,
    allocated: usize,
    gpu_ms: f64,
    cpu_ms: f64,
    wall_ms: f64,
}

/// Per-frame samples for one scenario, written to the trace and summarized.
#[derive(Debug, Default)]
struct Series {
    name: String,
    samples: Vec<Sample>,
}

impl Series {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            samples: Vec::new(),
        }
    }

    /// Measure `work` as one frame.
    fn frame(&mut self, work: impl FnOnce() -> Result<Redraw, String>) -> Result<(), String> {
        let (cpu0, wall0) = (clock::thread_cpu_ns(), Instant::now());
        let redraw = work()?;
        let cpu = clock::thread_cpu_ns().saturating_sub(cpu0);
        self.samples.push(Sample {
            tiles: redraw.tiles,
            allocated: redraw.allocated,
            gpu_ms: redraw.gpu.seconds * 1000.0,
            cpu_ms: to_f64_u64(cpu) / 1e6,
            wall_ms: wall0.elapsed().as_secs_f64() * 1000.0,
        });
        Ok(())
    }

    fn summary(&self) -> String {
        let pick =
            |f: fn(&Sample) -> f64| Summary::of(&self.samples.iter().map(f).collect::<Vec<_>>());
        let tiles = pick(|s| to_f64(s.tiles));
        format!(
            "{}\tframes {}\ttiles/frame {}\tgpu-ms {}\tcpu-ms {}\twall-ms {}\tallocated {}",
            self.name,
            self.samples.len(),
            tiles.show(1),
            pick(|s| s.gpu_ms).show(3),
            pick(|s| s.cpu_ms).show(3),
            pick(|s| s.wall_ms).show(3),
            self.samples.iter().map(|s| s.allocated).sum::<usize>()
        )
    }

    fn trace(&self, out: &mut String) {
        for (i, s) in self.samples.iter().enumerate() {
            let _ = writeln!(
                out,
                "{}\t{i}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}",
                self.name, s.tiles, s.allocated, s.gpu_ms, s.cpu_ms, s.wall_ms
            );
        }
    }
}

fn write_trace(options: &Options, series: &[&Series]) -> Result<(), String> {
    let Some(path) = &options.trace else {
        return Ok(());
    };
    let mut out = String::from("# series\tframe\ttiles\tallocated\tgpu-ms\tcpu-ms\twall-ms\n");
    for s in series {
        s.trace(&mut out);
    }
    std::fs::write(path, out).map_err(|e| format!("{path}: {e}"))
}

/// Sleep until `start + n` frames.
fn pace(start: Instant, n: usize) {
    let due = start + FRAME * u32::try_from(n).unwrap_or(u32::MAX);
    if let Some(wait) = due.checked_duration_since(Instant::now()) {
        std::thread::sleep(wait);
    }
}

/// S6-Q1: full redraws, then typical updates (typing and the spinner), then,
/// on the main thread with the compositor idle, wakeups and commits.
fn q1(options: &Options) -> Result<(), String> {
    let opts = options.clone();
    let app = with_window(move |content, viewport| {
        let mut stage = Stage::new(&content, viewport, opts.tile)?;
        // Warm up: allocate every tile and compile anything lazy.
        let first = stage.redraw_all()?;
        println!(
            "# first draw: {} tiles ({} page, {} frame, {} inner), {:.2} ms GPU",
            first.tiles,
            stage.page.tile_count(),
            stage.frame.tile_count(),
            stage.inner.tile_count(),
            first.gpu.seconds * 1000.0
        );
        let mut full = Series::new("full-redraw");
        let start = Instant::now();
        for n in 0..opts.frames {
            full.frame(|| stage.redraw_all())?;
            pace(start, n + 1);
        }
        let mut update = Series::new("typical-update");
        let start = Instant::now();
        for n in 0..opts.updates {
            update.frame(|| {
                let mut damage = vec![stage.spin(to_f32_usize(n) * 0.08)?];
                if n % 6 == 0 {
                    damage.push(stage.type_one()?);
                }
                let mut tiles: Vec<usize> = damage
                    .iter()
                    .flat_map(|d| stage.page.grid.touched(*d))
                    .collect();
                tiles.sort_unstable();
                tiles.dedup();
                stage.page.redraw(&stage.gpu, &stage.atlases, &tiles)
            })?;
            pace(start, n + 1);
        }
        println!("q1\t{}", full.summary());
        println!("q1\t{}", update.summary());
        println!(
            "# items: page {}, frame {}, inner {}; tile {} px",
            stage.page.list.items(),
            stage.frame.list.items(),
            stage.inner.list.items(),
            opts.tile
        );
        write_trace(&opts, &[&full, &update])?;
        // The layers stay on screen: CoreAnimation retains them and their
        // contents after this thread's references are dropped.
        drop(stage);
        Ok(())
    })?;
    // Idle: nothing changes, nothing is committed; the main thread waits in
    // AppKit, and the compositor thread has finished.
    let before = clock::usage()?;
    let deadline = Instant::now() + Duration::from_secs_f64(options.idle);
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        app.pump(Wait::For(left.as_secs_f64()));
    }
    let after = clock::usage()?;
    println!(
        "q1\tidle\tseconds {}\tidle-wakeups {}\tinterrupt-wakeups {}\tcommits 0\tfootprint {}",
        options.idle,
        after.idle_wakeups - before.idle_wakeups,
        after.interrupt_wakeups - before.interrupt_wakeups,
        after.footprint
    );
    Ok(())
}

/// S6-Q2: an animation at 60 Hz, redrawing either the damaged tiles or every
/// visible tile. Energy impact is sampled outside (`top`).
fn q2(options: &Options) -> Result<(), String> {
    let opts = options.clone();
    println!("# pid {}", std::process::id());
    with_window(move |content, viewport| {
        let mut stage = Stage::new(&content, viewport, opts.tile)?;
        stage.redraw_all()?;
        let full = opts.damage == "full";
        let name = format!("{}-{}", opts.animation, opts.damage);
        let mut series = Series::new(&name);
        let frames = to_frames(opts.seconds);
        let start = Instant::now();
        for n in 0..frames {
            let damage = match opts.animation.as_str() {
                "spinner" => Some(stage.spin(to_f32_usize(n) * 0.08)?),
                "typing" => (n % 6 == 0).then(|| stage.type_one()).transpose()?,
                other => return Err(format!("unknown animation {other}")),
            };
            if let Some(d) = damage {
                series.frame(|| {
                    if full {
                        stage.redraw_all()
                    } else {
                        let tiles = stage.page.grid.touched(d);
                        stage.page.redraw(&stage.gpu, &stage.atlases, &tiles)
                    }
                })?;
            }
            pace(start, n + 1);
        }
        println!("q2\t{}", series.summary());
        write_trace(&opts, &[&series])?;
        drop(stage);
        Ok(())
    })?;
    Ok(())
}

/// S6-Q3: scroll the page by moving its layer, drawing only tiles that enter
/// the display port; or animate a layer's transform and opacity.
fn q3(options: &Options) -> Result<(), String> {
    let opts = options.clone();
    with_window(move |content, viewport| {
        let mut stage = Stage::new(&content, viewport, opts.tile)?;
        let port = stage.port();
        stage.page.redraw(&stage.gpu, &stage.atlases, &port)?;
        stage.redraw_all()?;
        let frames = to_frames(opts.seconds);
        let scale = viewport.scale;
        let mut series = Series::new(&opts.motion);
        let start = Instant::now();
        match opts.motion.as_str() {
            "scroll" => {
                // 1,500 pixels a second, down the page and back up.
                let span = PAGE_HEIGHT - viewport.height;
                for n in 0..frames {
                    series.frame(|| {
                        let travel = (to_f32_usize(n) * 25.0) % (2.0 * span);
                        stage.scroll = if travel < span {
                            travel
                        } else {
                            2.0 * span - travel
                        };
                        let needed: HashSet<usize> = stage.port().into_iter().collect();
                        let existing = stage.page.existing();
                        let mut fresh: Vec<usize> = needed.difference(&existing).copied().collect();
                        fresh.sort_unstable();
                        let redraw = stage.page.redraw(&stage.gpu, &stage.atlases, &fresh)?;
                        stage.page.retain(&needed);
                        let y = -f64::from(stage.scroll) / scale;
                        let page = &stage.page.container;
                        layers::transaction(|| page.set_origin(0.0, y));
                        Ok(redraw)
                    })?;
                    pace(start, n + 1);
                }
            }
            "animate" => {
                // The outer frame turns, scales and fades, as a compositor
                // animation would move it (GR8, GR9).
                for n in 0..frames {
                    series.frame(|| {
                        let t = to_f64(n) / 60.0;
                        let frame = &stage.frame.container;
                        layers::transaction(|| {
                            frame.set_rotation_scale(
                                (t * 1.5).sin() * 0.2,
                                1.0 + 0.08 * (t * 2.0).sin(),
                            );
                            frame.set_opacity(to_f32(0.75 + 0.25 * (t * 3.0).cos()));
                        });
                        Ok(Redraw::default())
                    })?;
                    pace(start, n + 1);
                }
            }
            other => return Err(format!("unknown motion {other}")),
        }
        let quiet: Vec<f64> = series
            .samples
            .iter()
            .filter(|s| s.tiles == 0)
            .map(|s| s.cpu_ms)
            .collect();
        println!("q3\t{}", series.summary());
        println!(
            "q3\t{}-without-new-tiles\tframes {}\tcpu-ms {}",
            opts.motion,
            quiet.len(),
            Summary::of(&quiet).show(3)
        );
        write_trace(&opts, &[&series])?;
        drop(stage);
        Ok(())
    })?;
    Ok(())
}

/// S6-Q4: each path content filled on the CPU and uploaded, or filled on the
/// GPU, offscreen; times, memory and the GPU result against `vello_cpu`'s.
fn q4(options: &Options) -> Result<(), String> {
    let gpu = Gpu::new()?;
    println!(
        "# q4\tcontent\tsize\tpaths\tsegments\tmethod\tcpu-ms\tupload-ms\tgpu-ms\tbytes\tmean-diff\tmax-diff\tshare-over-32"
    );
    let repeat = options.repeat.max(1);
    for content in paths::contents() {
        q4_content(&gpu, &content, repeat)?;
    }
    Ok(())
}

/// S6-Q4 for one content: `vello_cpu` and upload, `zeno`, and the GPU.
fn q4_content(gpu: &Gpu, content: &paths::Content, repeat: usize) -> Result<(), String> {
    let (w, h) = (content.width, content.height);
    let label = format!(
        "{}\t{w}x{h}\t{}\t{}",
        content.name,
        content.items.len(),
        content.segments()
    );
    // The CPU: vello_cpu fills; the result is uploaded and drawn as an
    // image, as a renderer's image resource would be.
    let mut fill_ms = Vec::new();
    let mut reference = Vec::new();
    for _ in 0..repeat {
        let start = Instant::now();
        reference = paths::fill_vello(content)?;
        fill_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let mut upload_ms = Vec::new();
    let mut draw_ms = Vec::new();
    let target = gpu.target(w, h)?;
    for _ in 0..repeat {
        let start = Instant::now();
        let texture = gpu.texture(Format::Rgba8, w, h, &reference)?;
        upload_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        let instance = Image {
            rect: [0.0, 0.0, to_f32_usize(w), to_f32_usize(h)],
            uv: [0.0, 0.0, 1.0, 1.0],
            radii: [0.0; 4],
            extra: [1.0, 0.0, 0.0, 0.0],
        };
        let instances = gpu.upload(&[instance])?;
        let ids = gpu.indices(&[0], 1)?;
        let mut recording = gpu.record()?;
        recording.tile(
            target.raw(),
            [0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            &[Draw {
                kind: Kind::Image,
                instances: &instances,
                instance_count: 1,
                ids: &ids,
                first: 0,
                count: 1,
                texture: Some(&texture),
            }],
        )?;
        draw_ms.push(recording.finish().seconds * 1000.0);
    }
    println!(
        "q4\t{label}\tcpu-vello\t{:.3}\t{:.3}\t{:.3}\t{}\t0\t0\t0",
        Summary::of(&fill_ms).median,
        Summary::of(&upload_ms).median,
        Summary::of(&draw_ms).median,
        w * h * 4
    );
    // zeno, for comparison.
    let mut zeno_ms = Vec::new();
    let mut zeno = Vec::new();
    for _ in 0..repeat {
        let start = Instant::now();
        zeno = paths::fill_zeno(content);
        zeno_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let (mean, max, share) = paths::compare(&reference, &zeno);
    println!(
        "q4\t{label}\tcpu-zeno\t{:.3}\t-\t-\t{}\t{mean:.3}\t{max}\t{share:.5}",
        Summary::of(&zeno_ms).median,
        w * h * 4
    );
    // The GPU: flatten into fans on the CPU, then stencil and cover.
    let mut prep_ms = Vec::new();
    let mut gpu_ms = Vec::new();
    let mut bytes = 0;
    for _ in 0..repeat {
        let start = Instant::now();
        let fans = paths::fans(content);
        let vertices = gpu.upload(&fans.vertices)?;
        let covers = gpu.upload(&fans.covers)?;
        prep_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        bytes = vertices.bytes() + covers.bytes();
        let mut recording = gpu.record()?;
        recording.paths(
            target.raw(),
            [0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            &vertices,
            &covers,
            &fans.draws,
        )?;
        gpu_ms.push(recording.finish().seconds * 1000.0);
    }
    let bgra = platform::gpu::read(target.raw());
    let rgba: Vec<u8> = bgra
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    let (mean, max, share) = paths::compare(&reference, &rgba);
    println!(
        "q4\t{label}\tgpu-stencil-cover-4x\t{:.3}\t-\t{:.3}\t{bytes}\t{mean:.3}\t{max}\t{share:.5}",
        Summary::of(&prep_ms).median,
        Summary::of(&gpu_ms).median
    );
    Ok(())
}

/// S6-Q5: the footprint at each stage of bringing up one window with the page,
/// and CoreAnimation's handling of tagged surfaces.
fn q5(options: &Options) -> Result<(), String> {
    let start_usage = clock::usage()?;
    println!("q5\tstage\tstart\tfootprint {}", start_usage.footprint);
    let opts = options.clone();
    let app = with_window(move |content, viewport| {
        report("window shown", clock::usage()?);
        let mut stage = Stage::reporting(&content, viewport, opts.tile, &|name| {
            if let Ok(usage) = clock::usage() {
                report(name, usage);
            }
        })?;
        stage.redraw_all()?;
        let tile_bytes =
            stage.page.tile_bytes() + stage.frame.tile_bytes() + stage.inner.tile_bytes();
        let tiles = stage.page.tile_count() + stage.frame.tile_count() + stage.inner.tile_count();
        report("visible tiles drawn", clock::usage()?);
        std::thread::sleep(Duration::from_secs(1));
        report("visible tiles drawn, 1 s later", clock::usage()?);
        println!(
            "q5\ttiles\tdrawn\tcount {tiles}\tbytes {tile_bytes}\ttile {} px",
            opts.tile
        );
        // Typical updates leave a spare surface behind each updated tile.
        for n in 0..60 {
            let mut damage = vec![stage.spin(to_f32_usize(n) * 0.08)?];
            if n % 6 == 0 {
                damage.push(stage.type_one()?);
            }
            let tiles: Vec<usize> = damage
                .iter()
                .flat_map(|d| stage.page.grid.touched(*d))
                .collect();
            stage.page.redraw(&stage.gpu, &stage.atlases, &tiles)?;
        }
        let bytes = |s: &Stage| s.page.tile_bytes() + s.frame.tile_bytes() + s.inner.tile_bytes();
        report("after 60 typical updates", clock::usage()?);
        println!("q5\ttiles\twith spares\tbytes {}", bytes(&stage));
        stage.page.trim();
        std::thread::sleep(Duration::from_secs(1));
        report("spares freed, 1 s later", clock::usage()?);
        println!("q5\ttiles\ttrimmed\tbytes {}", bytes(&stage));
        color_check(&stage.gpu, &content, viewport, opts.show > 0.0)?;
        drop(stage);
        Ok(())
    })?;
    let deadline = Instant::now() + Duration::from_secs_f64(2.0_f64.max(options.show));
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        app.pump(Wait::For(left.as_secs_f64()));
    }
    report("idle, 2 s later", clock::usage()?);
    Ok(())
}

fn report(stage: &str, usage: Usage) {
    println!(
        "q5\tstage\t{stage}\tfootprint {}\tlifetime-max {}",
        usage.footprint, usage.lifetime_max_footprint
    );
}

/// Pure red (1, 0, 0) in three surfaces: tagged Display P3, tagged sRGB, and
/// untagged. Whether each carries its tag is checked here; whether
/// CoreAnimation honours it is a visual check: with `show`, the three appear in
/// the window's top-left corner, and on this Mac's P3 panel the P3-tagged red
/// should look deeper than the sRGB-tagged one.
fn color_check(gpu: &Gpu, content: &Layer, viewport: Viewport, show: bool) -> Result<(), String> {
    let side = 256;
    let tags = [Tag::DisplayP3, Tag::Srgb, Tag::None];
    let mut surfaces = Vec::new();
    for tag in tags {
        let surface = Surface::new(gpu.device(), side, side, tag)?;
        let mut recording = gpu.record()?;
        recording.tile(surface.texture(), [0.0, 0.0], [1.0, 0.0, 0.0, 1.0], &[])?;
        recording.finish();
        let pixels = platform::gpu::read(surface.texture());
        println!(
            "q5\tcolour\tsurface {tag:?}\ttagged {}\tcentre {:?}",
            surface.tagged(),
            platform::gpu::pixel_rgba(&pixels, side, side / 2, side / 2)
        );
        surfaces.push(surface);
    }
    if show {
        let scale = viewport.scale;
        let mut shown = Vec::new();
        layers::transaction(|| {
            for (i, surface) in surfaces.iter().enumerate() {
                let layer = Layer::new(scale);
                layer.set_frame(40.0 + to_f64(i) * 140.0, 40.0, 128.0, 128.0);
                layer.set_contents(surface);
                content.add(&layer);
                shown.push(layer);
            }
        });
        // Retained by `content` and by the layers' contents.
        drop(shown);
    }
    Ok(())
}

fn to_frames(seconds: f64) -> usize {
    let frames = (seconds * 60.0).round().max(1.0);
    to_usize(frames)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "frame counts are small and positive"
)]
fn to_usize(v: f64) -> usize {
    v as usize
}

#[expect(clippy::cast_possible_truncation, reason = "window sizes are small")]
fn to_f32(v: f64) -> f32 {
    v as f32
}

fn to_f32_usize(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "nanosecond counts far below 2^52"
)]
fn to_f64_u64(n: u64) -> f64 {
    n as f64
}
