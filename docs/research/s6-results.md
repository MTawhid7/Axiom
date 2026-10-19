# S6 Results: Compositing on Metal

| | |
|---|---|
| **Status** | Complete (2026-10-20). **S6-Q1 and S6-Q3 pass at the median,** with their tails reported. S6-Q2 and S6-Q4 are reported. **S6-Q5:** each surface carries its colour tag, and whether CoreAnimation honours the tags is the owner's visual check; **its memory condition is not met as measured** (§7). The owner accepted [ADR-0012](../adr/0012-graphics-images-and-media.md) at Gate C (2026-10-20) |
| **Question** | Can the browser's compositor draw page-shaped display lists with Axiom's own Metal pipelines into tiled IOSurface layers of a 2,560 × 1,600 window, within a 60 Hz frame budget (S6-Q1)? What does damage-limited drawing save (S6-Q2)? Do scrolling and animation by layers stay within 1 ms a frame (S6-Q3)? How do CPU and GPU path filling compare (S6-Q4)? What do the device, the pipelines and the tiles cost in memory, and does CoreAnimation honour Display P3 tags (S6-Q5)? |
| **Feeds** | [ADR-0012](../adr/0012-graphics-images-and-media.md) (its conditions: S6-Q1, S6-Q3 and S6-Q5); [graphics](../architecture/graphics.md) GR2, GR5, GR6, GR8, GR12 and its open questions 1–3; [text and fonts](../architecture/text-and-fonts.md) TX12's Rust rasterizer; the [process model](../security/process-model.md)'s B1, an input to Step 6 |
| **Plan** | [Step 5 tracker](step-5-subsystems.md) §10.2, authorized at Gate B (2026-10-18), with its thresholds confirmed |
| **Code** | `spikes/s6/compositor` (`s6-compositor`) and `tools/s6-measure.sh`, on branch `step-5` (`c2c8ad5`). Formal run at commit `45bff95` (§2) |
| **Last updated** | 2026-10-20 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from S6's binary and script, in the formal run named in §2 unless stated otherwise. Counts of frames over a threshold come from the run's per-frame traces (§10).
- **Observed** facts were read on this Mac: files' times, the run's environment record, SDK headers. Each names its file or command.
- **Inference** marks an explanation drawn from them; **hypothesis** marks an untested claim, with the step that would test it.

MB means 10⁶ bytes, as B1 uses it; MiB means 2²⁰ bytes.

---

## Summary

| Question | Answer |
|---|---|
| **S6-Q1** Compositing cost? | **Passes at the median.** A typical update (a spinner each frame, and a keystroke every sixth: 2 or 12 tiles) took 0.17–0.28 ms on the GPU and 0.67–1.84 ms on the compositor thread at the median, against 4 ms and 2 ms. No frame of any run took more than 1.48 ms on the GPU. **The compositor thread's tail:** in run 1, 192 of 600 updates took more than 2 ms (95th percentile 2.93 ms), and half its tile draws allocated a new surface; runs 2 and 3 had 0 and 2 such frames. A full redraw of 96 tiles took 8.0–10.1 ms on the GPU, and 16.3–20.3 ms from start to the GPU's end, at the median: more than a 60 Hz frame. **Idle:** no package idle wakeups and no commits in 10 s; 6 interrupt wakeups in each run (§3) |
| **S6-Q2** Damage? | **Reported.** Only the damaged tiles were redrawn: 2 a frame for the spinner, 10 for each keystroke. Against full redraws of 96 tiles, the spinner cost 47–49× less GPU time, 12× less compositor time and 7–9× less energy impact by `top`; typing 9–11×, 6–7× and about 5× less (§4) |
| **S6-Q3** Scrolling and animation by layers? | **Passes at the median.** No tile was redrawn except newly exposed rows. **Animating** an embedded frame's transform and opacity drew nothing, and took at most 0.35 ms a frame. **Scrolling frames that only moved layers** took 0.32–0.40 ms at the median and 0.46–0.64 ms at the 95th percentile; 2–7 of 1,085 per run took more than 1 ms, at most 1.52 ms. The 115 frames per run that drew a newly exposed row of 10 tiles took 1.5–2.2 ms at the median, and up to 8.6 ms when the surface pool grew (§5) |
| **S6-Q4** Paths? | **Reported: GPU filling pays in time and memory, not in quality.** On icons, a chart and a map tile, filling on the GPU in the compositor (stencil then cover, 4× multisampled) reached a first draw 3.9–4.8× sooner than `vello_cpu`'s filling plus its upload, with 31–62× less memory (68–202 KB of geometry against 4.2–6.3 MB of image). Its edges differ from analytic coverage by up to 109 of 255 levels, and by more than 32 on up to 0.31% of pixels. Once filled, a cached image costs the compositor about what filling again on the GPU costs. Under GR5, the amendment is the owner's decision (§6, §8) |
| **S6-Q5** Memory and colour? | **The memory condition is not met as measured.** One window of the synthetic page, with its compositor state trimmed, came to 94.2 MB. Without the stage that uploads the synthetic renderer resources, it came to 55.8 MB, which leaves about 4 MB of B1's 60 MB for the rest of the browser (inference). The window cost 14.2 MB, Metal's device and pipelines 0.5 MB, the display lists 6.3 MB, and the visible tiles 36.5 MB, 25.2 MB of it the 96 surfaces. **Colour:** each surface carries its tag (Display P3, sRGB, none); whether CoreAnimation honours the tags is the owner's comparison of the pattern the run displayed (§7) |

**Main findings**, beyond the pass criteria:
1. **Damage tracking is required, not an optimization.** Redrawing the whole 2,560 × 1,600 window took longer than a 60 Hz frame at the median in every run (16.3–20.3 ms from start to the GPU's end). The spinner's full redraws scored 54–56 in `top`'s energy impact, against 6–8 when only its tiles were redrawn (§3, §4).
2. **The slowest frames allocate surfaces.** A tile draws into its spare surface, and its previous surface becomes the spare once CoreAnimation releases it; if CoreAnimation still holds it, a new surface is allocated. That happened in 63–71% of the full redraws' tile draws, in half of run 1's typical-update tile draws, and in the 4 frames per scrolling run that grew the pool, which were each run's four slowest frames (2.5–8.6 ms). **Inference:** the compositor needs more than one spare per tile at full rate, a pool of free surfaces allocated ahead of need, and newly exposed rows drawn before they are needed or spread across frames (§3, §5).
3. **Run 1 began on a busy Mac.** The load averages were 103.80, 59.25 and 24.79 at the start, and the release build compiled nothing, so other work had loaded the Mac in the minutes before. Run 1's typical updates were slower throughout: 1.84 ms at the median, against 0.67 and 0.72 ms (§2, §9).
4. **Tiles dominate the compositor's memory.** The 96 tiles of 256 px for one window with two embedded frames are 25.2 MB, 1.5 times the window's own pixels. B1 was set from S2's idle window before any tile was measured (§7).
5. **`vello_cpu` fills paths 2.1–6.8× faster than `zeno`,** with the same quality. S5 found `zeno` 1.7× faster on glyphs, which atlases cache. That answers S5's question of which one Rust rasterizer the renderer keeps (TX12), subject to grading (§6, §8).

## 1. What was built

One crate, `spikes/s6/compositor` (`s6-compositor`): one executable, with a mode per question. It stands in for the browser's compositor (GR1–GR4, GR8, GR11). Its display lists are generated in-process, so no renderer is needed.

| Mode | Question | What it does |
|---|---|---|
| `q1` | S6-Q1 | Draws the page's visible tiles; then 120 full redraws and 600 typical updates, paced at 60 Hz; then waits 10 s with nothing changing |
| `q2` | S6-Q2 | A spinner, or typing into a field, for 20 s. Each frame redraws either the damaged tiles or every visible tile, and the script samples `top`'s energy impact once a second |
| `q3` | S6-Q3 | Scrolls the page at 1,500 pixels a second, down and back, drawing only tiles newly inside the display port; or turns, scales and fades the embedded frame by its layer |
| `q4` | S6-Q4 | Offscreen: fills three path contents on the CPU and uploads them, and fills them on the GPU; compares times, memory and pixels |
| `q5` | S6-Q5 | Reports the footprint at each stage of bringing up the window and page, after updates and after trimming. Checks three surfaces' colour tags, and with `--show` displays them |

**The scene** (`scene.rs`) is a 24,000-pixel page of text as glyph quads, rectangles, rounded clips, borders, shadows, gradients and images, with two nested frames, each in a layer of its own. It has 44,372 display items for the page, 421 for the outer frame and 419 for the inner one. Its glyph atlas holds printable ASCII in Helvetica at three sizes, rasterized by S5's Rust path (`skrifa` and `zeno`) into a 1,024-pixel coverage atlas (TX13). Its images come from one 2,048-pixel atlas.

**Modules.**
- `src/platform/`, the only module with `unsafe` (allowlisted in `tools/unsafe-policy.txt`), under invariants K1–K9 in its `mod.rs`:
  - the AppKit window, as S2's was;
  - Metal: the device, pipelines compiled from Metal shading language at run time, command buffers and their GPU timestamps;
  - IOSurfaces with colour tags;
  - CoreAnimation layers, committed in explicit transactions from the compositor's thread (K7);
  - `proc_pid_rusage`, and the thread's CPU clock.
- `compositor.rs`: tiled layers. Each tile has a front surface and a spare. A redraw draws into the spare and swaps the two, allocating a new surface when CoreAnimation still holds the spare. A pool keeps the surfaces of tiles that left the display port.
- `shaders.metal`: GR3's pipelines: rectangles and rounded rectangles with analytic anti-aliasing, borders, shadows, gradients, images, glyph quads and rounded clips. One render pass per tile.
- `paths.rs`: S6-Q4's contents, and a GPU filler of the spike's own. Paths are flattened on the CPU into triangle fans, at a tolerance of 0.2 pixels, then filled by stencil then cover, with memoryless 4× multisampled colour and stencil attachments, which live only in the GPU's tile memory.

**Deviations from §10.2,** none of which changes the authorization's scope:
1. **The colour check is visual.** CoreAnimation's offscreen renderer (`CARenderer`) produced no pixels in this setting. So the surfaces' tags are checked automatically, and whether CoreAnimation honours them is the owner's comparison of the pattern displayed.
2. **Frames are paced by a timer,** sleeping to the next 16.7 ms slot, not by a display link.
3. **Footprints are read by the spike at each stage** (`ri_phys_footprint`, as S4, S5 and S7 read it), rather than by `/usr/bin/time -l` at exit.
4. **S6-Q4's CPU candidates** are S5's two rasterizers: `vello_cpu`, and `zeno` for comparison. `tiny-skia` was not tried. The GPU filler is stencil then cover, not tessellation or sparse strips.

**Dependencies,** each justified in the workspace manifest (DP23): the objc2 framework crates, with `objc2-io-surface` added; `vello_cpu` 0.3.0, `zeno` 0.3.3 and `skrifa`, already S5's; and S5's own `s5-rust-text`.

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B5112a), with its built-in Retina display at 2,560 × 1,600; on AC power with Low Power Mode off (`pmset`); rustc 1.94.1; Command Line Tools only.
- **Formal run:** `tools/s6-measure.sh` with `--show`, run by the owner at commit `45bff95` from a clean tree: `target/s6-results/20261019-201443/`, 2026-10-19 from 20:14 to 20:26. Three runs of every mode, each after a 60 s cool-down; then the Display P3 pattern was displayed (`q5-show.txt`).
- **The window** was 1,280 × 800 points at scale 2, so 2,560 × 1,600 pixels, at the top left of the built-in display. In all 25 measured runs it was ordered in and unoccluded at the start and at the end (each run's comment lines, collected in `summary.txt`).
- **A busy start.** The load averages were 103.80, 59.25 and 24.79 when the script recorded its environment (`environment.txt`). The release build the script runs first compiled nothing: the binary was unchanged since 2026-10-18 21:44:50 (observed, `ls -lT target/release/s6-compositor`). So the load came from other work on the Mac in the minutes before the run, and the method's "otherwise idle" holds at best after the first cool-down (inference). §9 discusses its effect.
- **Per frame** (`Series::frame`):
  - **GPU time** is the frame's command buffer's `GPUEndTime − GPUStartTime`;
  - **compositor time** is the compositor thread's CPU time for the frame's work, from `CLOCK_THREAD_CPUTIME_ID`, which excludes time spent blocked;
  - **wall time** runs from the frame's start until its command buffer completes, since the compositor waits for each frame;
  - **new surfaces** counts the surfaces allocated because no spare was free.

  Each scenario reports the median, the 95th percentile and the maximum per run, and its trace holds every frame.
- **Idle** (`q1`): `ri_pkg_idle_wkups` and `ri_interrupt_wkups` from `proc_pid_rusage`, across 10 s while the main thread waits in AppKit and the compositor thread has finished. Package idle wakeups are what S2 measured as idle wakeups through `top`. No CoreAnimation transaction is begun while idle, so "commits 0" holds by construction rather than by a counter.
- **Energy:** `top -stats power` sampled the process once a second for 20 s. Each figure is the mean of the samples after the first, which has no interval. It is `top`'s relative energy-impact score, not a power in watts.
- **Footprints** are `ri_phys_footprint`, read by the spike at each stage, with `ri_lifetime_max_phys_footprint` beside them.

## 3. S6-Q1: compositing cost

The first draw of the page's 96 tiles (70 for the page, 20 for the outer frame, 6 for the inner one) took 7.13–7.31 ms on the GPU. Then, in milliseconds, median / 95th percentile / maximum:

| Run | Scenario | Tiles a frame | GPU | Compositor thread | Wall | New surfaces |
|---|---|---|---|---|---|---|
| 1 | Full redraw, 120 frames | 96 | 7.97 / 10.42 / 19.73 | 6.94 / 9.15 / 12.30 | 16.26 / 21.41 / 28.99 | 7,302 of 11,520 tile draws |
| 2 | | 96 | 8.19 / 10.60 / 11.19 | 7.39 / 10.62 / 12.43 | 16.72 / 22.84 / 24.42 | 8,221 |
| 3 | | 96 | 10.12 / 11.20 / 11.56 | 8.32 / 10.08 / 10.99 | 20.26 / 22.49 / 23.62 | 8,084 |
| 1 | Typical update, 600 frames | 2 or 12 | 0.28 / 1.22 / 1.46 | **1.84 / 2.93 / 3.57** | 2.84 / 4.96 / 5.95 | 1,108 of 2,200 tile draws |
| 2 | | 2 or 12 | 0.18 / 1.32 / 1.48 | 0.67 / 1.84 / 1.97 | 1.58 / 3.92 / 4.34 | 13 |
| 3 | | 2 or 12 | 0.17 / 1.30 / 1.48 | 0.72 / 1.86 / 2.27 | 1.58 / 3.89 / 4.09 | 21 |

- **Typical updates over 2 ms on the compositor thread:** 192, 0 and 2 of 600. In run 1, updates of 2 tiles took 1.80 ms at the median (100 of 500 over 2 ms), and updates of 12 tiles 2.81 ms (92 of 100). In runs 2 and 3, updates of 2 tiles took 0.65 and 0.69 ms, and updates of 12 tiles 1.83 and 1.84 ms. Run 1 was slower in both halves of its series (medians 1.87 and 1.81 ms), so it wasn't a warm-up.
- **Idle,** 10 s in each run: 0 package idle wakeups; 6 interrupt wakeups in each run; no commits. The footprint at the end was 71.7–73.7 MB.

**Verdict: passes at the median,** the statistic "a typical update takes" describes, and the one CP11's trials reported. The GPU stayed within 4 ms in every frame of every run. The compositor thread stayed within 2 ms at the median in every run, and in all but 2 of 1,200 frames in runs 2 and 3; in run 1, a third of the updates exceeded it (§9). The idle condition holds in the measure S2 used, package idle wakeups. The 6 interrupt wakeups in 10 s are unexplained (hypothesis: AppKit's run loop on the main thread; a longer idle window at the shell milestone would show whether they are periodic).

## 4. S6-Q2: damage

Medians for runs 1, 2 and 3. Times in milliseconds; energy impact is `top`'s mean over 16–18 samples.

| Animation | Redraw | Tiles a frame | GPU | Compositor thread | Energy impact |
|---|---|---|---|---|---|
| Spinner, 1,200 frames | Every visible tile | 96 | 8.09, 7.02, 7.89 | 7.24, 7.38, 7.49 | 54.0, 56.1, 55.8 |
| | Damaged tiles | 2 | 0.17, 0.15, 0.16 | 0.60, 0.59, 0.64 | 6.2, 7.7, 6.8 |
| Typing, 200 keystroke frames | Every visible tile | 96 | 10.18, 9.29, 9.89 | 10.72, 6.85, 10.75 | 14.6, 10.4, 14.7 |
| | Damaged tiles | 10 | 1.17, 0.83, 1.14 | 1.62, 1.12, 1.62 | 2.8, 2.3, 2.7 |

**Savings,** run by run: the spinner, 47–49× in GPU time, 12× in compositor time and 7.3–8.7× in energy impact; typing, 8.7–11.2×, 6.1–6.7× and 4.5–5.4×. Typing's energy is averaged over the whole run, in which only every sixth frame draws. Its damage is the field's whole rectangle, which touches 10 tiles; damage limited to the changed glyphs and the caret would touch fewer (inference).

**Verdict:** the criterion is met. Only the tiles that damage touches were redrawn, as counted, and the saving is reported.

## 5. S6-Q3: scrolling and animation by layers

Compositor thread, in milliseconds, median / 95th percentile / maximum:

| Run | Motion | Frames | Tiles a frame | Compositor thread | Over 1 ms |
|---|---|---|---|---|---|
| 1 | Animation | 1,200 | 0 | 0.22 / 0.29 / 0.35 | 0 |
| 2 | | 1,200 | 0 | 0.19 / 0.24 / 0.29 | 0 |
| 3 | | 1,200 | 0 | 0.23 / 0.27 / 0.35 | 0 |
| 1 | Scrolling: layers moved only | 1,085 | 0 | 0.40 / 0.63 / 1.52 | 7 |
| 2 | | 1,085 | 0 | 0.32 / 0.46 / 1.03 | 2 |
| 3 | | 1,085 | 0 | 0.40 / 0.64 / 1.49 | 6 |
| 1 | Scrolling: a newly exposed row drawn | 115 | 10 | 2.17 / 2.28 / 6.31 | — |
| 2 | | 115 | 10 | 1.47 / 2.08 / 6.15 | — |
| 3 | | 115 | 10 | 2.18 / 2.32 / 8.58 | — |

- Frames that drew a new row took 1.01–1.80 ms on the GPU at the median, and at most 2.51 ms.
- In each run, 4 of those frames also grew the surface pool, allocating 10 surfaces each. They took 2.5–8.6 ms: the four slowest frames of each run.
- The animation drew no tile and allocated nothing.

**Verdict: passes at the median.** The ≤ 1 ms threshold is read for the frames that only move layers, which the spike reports separately for this purpose. Animation stayed within it in every frame; scrolling at the median and at the 95th percentile, with 2–7 of 1,085 frames per run over it. Frames that draw a newly exposed row do a typical update's work in the same frame, and took 1.5–2.2 ms at the median.

## 6. S6-Q4: paths

Each figure is the range of the three runs' medians, each of 5 repeats. Times in milliseconds. Differences are per colour channel, against `vello_cpu`'s pixels, on a scale of 0–255; "over 32" is the share of pixels whose worst channel differs by more than 32 levels.

| Content | Paths; segments | Method | Fill or prepare (CPU) | Upload | GPU | Memory, bytes | Mean difference | Largest | Over 32 |
|---|---|---|---|---|---|---|---|---|---|
| Icons, 1,024 × 1,024 | 128; 1,173 | `vello_cpu`, uploaded as an image | 0.84–1.35 | 0.54–0.80 | 0.32–0.40 | 4,194,304 | Reference | | |
| | | `zeno` | 2.05–2.83 | | | 4,194,304 | 0.104 | 46 | 0.092% |
| | | GPU: stencil then cover, 4× | 0.16–0.19 | | 0.25–0.28 | 75,400 | 0.216 | 72 | 0.193% |
| Chart, 2,048 × 768 | 13; 12,019 | `vello_cpu`, uploaded as an image | 2.00–2.18 | 0.77–0.84 | 0.48–0.58 | 6,291,456 | Reference | | |
| | | `zeno` | 14.49–15.24 | | | 6,291,456 | 0.013 | 5 | 0 |
| | | GPU: stencil then cover, 4× | 0.14–0.16 | | 0.59–0.63 | 202,304 | 0.248 | 109 | 0.312% |
| Map tile, 1,024 × 1,024 | 511; 3,172 | `vello_cpu`, uploaded as an image | 1.54–1.57 | 0.47–0.54 | 0.39–0.41 | 4,194,304 | Reference | | |
| | | `zeno` | 7.52–7.77 | | | 4,194,304 | 0.018 | 26 | 0 |
| | | GPU: stencil then cover, 4× | 0.075–0.088 | | 0.51–0.56 | 68,144 | 0.195 | 42 | 0.001% |

**What it shows:**
- **To a first draw** (the medians of the three runs): filling with `vello_cpu`, uploading and drawing the image took 2.18 ms for the icons, 3.55 for the chart and 2.44 for the map tile; preparing and filling on the GPU took 0.45, 0.75 and 0.63 ms. That is **3.9–4.8× sooner on the GPU.** Uploading the image, which the compositor would do, costs 0.47–0.84 ms, against 0.075–0.19 ms to prepare the geometry.
- **Memory:** 68–202 KB of geometry against 4.2–6.3 MB of image, **31–62× less.** The multisampled attachments are memoryless, so they add nothing.
- **Once filled,** drawing the cached image costs the GPU 0.32–0.58 ms, about what filling again costs (0.25–0.63 ms).
- **Quality:** 4× multisampling gives coarser edges than analytic coverage. On the chart, the content with the most edges, the largest difference is 109 levels, and 0.31% of pixels differ by more than 32; on the icons, 72 levels and 0.19%. `zeno` stays closer to `vello_cpu` (0–0.092% of pixels over 32).
- **Between the CPU rasterizers,** `vello_cpu` filled 2.1× (icons), 4.9× (map tile) and 6.8× (chart) faster than `zeno`.

**Verdict:** reported. GPU filling in the compositor pays in time and memory, not in quality, and GR5 makes that the owner's decision (§8).

## 7. S6-Q5: memory and colour

**Footprint at each stage** (the mean of the three runs, which differ by at most 0.2 MB except after the updates):

| Stage | Footprint, MB | Change |
|---|---|---|
| The process at start | 1.9 | |
| Window shown | 16.1 | +14.2 |
| Metal device and pipelines | 16.6 | +0.5 |
| Glyph and image atlases (renderer resources) | 55.0 | +38.4 |
| Display lists: items and bins on the GPU | 61.3 | +6.3 |
| Visible tiles drawn: 96 surfaces of 256 KiB, 25.2 MB | 97.9 | +36.5 |
| After 60 typical updates: the surfaces with spares, 28.3 MB | 98.5–102.9 | +0.7 to +5.0 |
| Spares freed, 1 s later: the surfaces, 25.2 MB | 94.2 | −7.1 |
| Idle, 2 s later, with the compositor's state dropped and CoreAnimation still showing the tiles | 67.3 | −26.9 |
| Peak (lifetime maximum) | 101.6–104.7 | |

- **The atlas stage** uploaded 17.8 MB of texture data, a 1 MiB coverage atlas and a 16 MiB image atlas in shared storage. The other 20.6 MB are unattributed (hypothesis: Metal's allocations for shared textures, and the font file and image buffer read to build them; attributed at the graphics milestone with `footprint` or `vmmap`, which ship with macOS, observed in `/usr/bin`).
- **The tile stage** holds 25.2 MB of surfaces; its other 11.4 MB are likewise unattributed.
- **The window's visibility changes the figures.** The informal quick run of 2026-10-18 (on battery, in Low Power Mode, with the window occluded by a full-screen app) ended at 46.6 MB idle and 73.6 MB after trimming, about 21 MB below the formal run at the same stages (`target/s6-results/20261018-214604/summary.txt`). Hypothesis: the window server holds memory charged to a visible window's process; attributed at the graphics milestone.

**Against B1** (the [process model](../security/process-model.md) §7: the browser process at ≤ 60 MB, idle, with one window; set from "S2's 20 MB, plus browser UI, profile and compositor"):
- The spike, idle with one window and its compositor state trimmed, uses 94.2 MB.
- Without the atlas stage, 55.8 MB remain: the window and process 16.1 MB, Metal's device and pipelines 0.5 MB, the display lists 6.3 MB, and the tiles, less what trimming returned. The atlases stand in for renderer resources, whose textures GR12 counts in the browser but whose size depends on the page.
- That leaves about 4 MB of B1 for the browser's UI, its profile data and every other structure, and none for the page's resources. **The condition is not met as measured** (inference).
- **Why B1 missed** (inference): S2's 20 MB was a 960 × 200-point window with a `CAMetalLayer`'s drawables. A window of this size with Metal and no content costs 16.6 MB, but tiles scale with the window's pixels: 25.2 MB of surfaces here, 1.5 times the window's own pixels, because tiles overhang the edges and the embedded frames have layers of their own.

**Colour:** three 256-pixel surfaces were filled with pure red. The surface tagged Display P3 and the one tagged sRGB carry their tags, the untagged one carries none, and each reads back [255, 0, 0, 255] at its centre, since a tag doesn't change the stored values. The run then displayed them at the window's top left (`q5-show.txt`; the window was visible at its end). On this Mac's P3 panel, the P3-tagged red should look deeper than the sRGB-tagged one if CoreAnimation honours the tags. That comparison is the owner's.

**Verdict:** the device, pipelines and tiles are measured, as the question asks. The memory condition is not met as measured (§8, item 6); the colour condition rests on the owner's comparison.

## 8. Implications for Axiom

1. **ADR-0012's conditions, and the owner's decision.** The owner's Gate B decision made acceptance depend on "S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox". S6-Q1 and S6-Q3 pass at the median, with the tails in §3 and §5. S6-Q5's memory condition is not met as measured, and its colour condition rests on the owner's comparison. S5-Q5's condition was not met ([S5 results](s5-results.md) §7). **At Gate C (2026-10-20) the owner accepted ADR-0012:** "The formal S6 benchmark run is complete (target/s6-results/20261019-201443/summary.txt) and easily meets all frame time thresholds. IM3's fallback (AVIF in the media utility process) is approved."
2. **GR2 holds, and damage tracking is required:** full redraws of a full-screen window miss a 60 Hz frame, and damage-limited tiles cost 47× less GPU time for a spinner (§3, §4).
3. **GR8 holds:** moving layers costs 0.19–0.40 ms a frame at the median (§5).
4. **Surfaces must be ready before they are needed** (inference from §3 and §5). With one spare per tile, CoreAnimation often still holds a tile's previous surface at full rate, and allocating a new one made the slowest frames. At the graphics milestone: enough spares for full rate, a pool kept allocated ahead of need, and newly exposed rows drawn ahead or spread across frames, measured against this report's traces.
5. **GR5: the amendment is the owner's decision.** GR5 says: "If S6 shows that GPU path filling in the compositor pays, in time, memory or energy, an amendment to PL7 goes to the owner." It pays in time (3.9–4.8× sooner to a first draw) and in memory (31–62× less), not in quality (§6). **Recommended: keep GR5 as written for now,** and propose the amendment at the graphics milestone with a GPU filler whose edges match analytic coverage (sparse strips, as Vello's hybrid renderer fills them), measured on S6-Q4's contents:
   - once filled, a cached path image costs the compositor about what filling again would, so the gain is at first draws and changes of scale;
   - the memory saving matters on pages with many path images, which Step 6's corpus can size;
   - path items bring renderer-supplied geometry into the browser process, a trade to make once, with the method that meets quality.
6. **GR12 and B1: B1 needs revising in Step 6,** which sets the budgets with B1–B7 as inputs ([decision map](../decision-map.md) §4, item 6). **Recommended:** count a visible window's tile surfaces apart from the browser's own memory, since they scale with the window's pixels (25.2 MB for this window and page), and attribute the unexplained parts in §7 when the compositor is built. The process model is binding, so B1 is for the owner.
7. **One Rust rasterizer for the renderer** (TX12 for web fonts; GR5 for paths). `vello_cpu` filled paths 2.1–6.8× faster than `zeno` (§6), and S5 found `zeno` 1.7× faster on glyphs, which atlases cache ([S5 results](s5-results.md) §4). **Recommended: `vello_cpu`,** subject to grading its tree when it is adopted (DP8, DP15); `zeno` stays the alternative, with its unsound conversion guarded as S5 guards it.
8. **GR6:** the surfaces carry their tags. Whether CoreAnimation honours them is the owner's visual comparison (§7).
9. **The tile size:** 256-pixel tiles were used throughout and met the thresholds; no other size was measured formally. 256 px is GR2's starting point at the graphics milestone.
10. **Idle (AC3, PM8):** no package idle wakeups and no commits in 10 s. The 6 interrupt wakeups are left for the shell milestone, which owns the main thread's run loop (§3).

## 9. Threats to validity

1. **A busy start.** The Mac had been heavily loaded in the minutes before the run (§2). Run 1's typical updates were slower throughout, and half their tile draws allocated a surface, against 0.6–1.0% in runs 2 and 3. **Hypothesis:** the window server, busy with that load, released surfaces late, so run 1 allocated and paid for it. A run after a longer idle period would test it. Runs 2 and 3 agree closely with each other and with CP11's informal trials.
2. **Synthetic content.** The display lists are generated: one page shape, fixed atlases (the 16 MiB image atlas is larger than most pages need), and damage the spike chooses. Real pages' costs differ.
3. **Timer pacing,** not a display link (deviation 2), so frame times aren't aligned with the display's refresh.
4. **`top`'s energy impact** is a relative score, sampled once a second (16–18 samples a run).
5. **Footprint attribution** is by stage, and leaves a third of the atlas stage and of the tile stage unexplained (§7). The window's visibility changes the totals by about 21 MB.
6. **One machine, one OS build.** macOS 27.2 (26B5112a) on an M1. CoreAnimation's and the window server's behaviour may change with updates.
7. **The colour check depends on the owner's eyes,** under the display's settings at the time (deviation 1).

## 10. Reproducing these results

```sh
cargo test -p s6-compositor          # tiling, text, scene, path and counter tests
tools/s6-measure.sh --show 30        # everything; target/s6-results/<timestamp>/
                                     # formal runs: AC power, the Mac otherwise idle, the window visible
```

Counting frames over a threshold in a run's traces, for example run 1:

```sh
awk -F'\t' '$1=="typical-update" && $6>2' q1-run1.trace.tsv | wc -l       # updates over 2 ms: 192
awk -F'\t' '$1=="scroll" && $3==0 && $6>1' q3-scroll-run1.trace.tsv | wc -l  # layer-only scroll frames over 1 ms: 7
```

| Run | Directory |
|---|---|
| Formal run (commit `45bff95`, clean tree, run by the owner) | `target/s6-results/20261019-201443/`: `summary.txt`, `environment.txt`, `q1-run*`, `q2-*-run*` with their `.top.txt` samples, `q3-*-run*`, `q4-run*`, `q5-run*`, `q5-show.txt`, and each timed scenario's `.trace.tsv` |
| Informal quick run (2026-10-18, on battery, the window occluded) | `target/s6-results/20261018-214604/` |

## Sources

- Observed on this Mac: the SDK 27 headers `Metal/MTLCommandBuffer.h` (`GPUStartTime`, `GPUEndTime`), `IOSurface/IOSurfaceRef.h`, `QuartzCore/CALayer.h`, `libproc.h`, `sys/resource.h` (`rusage_info_v4`) and `_time.h`; `top(1)`; `pmset`; `ls -lT`.
- This repository: `spikes/s6/`, `tools/s6-measure.sh`; the [Step 5 tracker](step-5-subsystems.md) §10.2; [graphics](../architecture/graphics.md); [S2 results](s2-results.md) for the idle window; [S5 results](s5-results.md) for the rasterizers.
