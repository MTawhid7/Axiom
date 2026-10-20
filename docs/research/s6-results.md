# S6 Results: Compositing on Metal

| | |
|---|---|
| **Status** | Complete (2026-10-20), from two formal runs. **S6-Q1 and S6-Q3 pass at the median** in both, with their tails reported; S6-Q1's idle condition held in five of six windows. S6-Q2 and S6-Q4 are reported. **S6-Q5:** CoreAnimation honours the colour tags, as measured in the owner's screenshot; **its memory condition is not met as measured** (§7). The owner accepted [ADR-0012](../adr/0012-graphics-images-and-media.md) at Gate C (2026-10-20), then approved B1's revision, deferred GR5's amendment to the graphics milestone and adopted `vello_cpu` (§8) |
| **Question** | Can the browser's compositor draw page-shaped display lists with Axiom's own Metal pipelines into tiled IOSurface layers of a 2,560 × 1,600 window, within a 60 Hz frame budget (S6-Q1)? What does damage-limited drawing save (S6-Q2)? Do scrolling and animation by layers stay within 1 ms a frame (S6-Q3)? How do CPU and GPU path filling compare (S6-Q4)? What do the device, the pipelines and the tiles cost in memory, and does CoreAnimation honour Display P3 tags (S6-Q5)? |
| **Feeds** | [ADR-0012](../adr/0012-graphics-images-and-media.md) (its conditions: S6-Q1, S6-Q3 and S6-Q5); [graphics](../architecture/graphics.md) GR2, GR5, GR6, GR8, GR12 and its open questions; [text and fonts](../architecture/text-and-fonts.md) TX12's Rust rasterizer; the [process model](../security/process-model.md)'s B1; Step 6's budgets |
| **Plan** | [Step 5 tracker](step-5-subsystems.md) §10.2, authorized at Gate B (2026-10-18), with its thresholds confirmed |
| **Code** | `spikes/s6/compositor` (`s6-compositor`) and `tools/s6-measure.sh` (`c2c8ad5`); `tools/s6-tails.py` and `tools/png-region-stats.py` for the analysis. Formal runs at commits `45bff95` and `b4948ce` (§2) |
| **Last updated** | 2026-10-20: the second formal run; the colour check from the owner's screenshot; the display's scaled mode; the owner's decisions on B1, GR5 and the rasterizer |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from S6's binary and script, in the two formal runs named in §2, or from the owner's screenshot, measured with `tools/png-region-stats.py`. Counts of frames over a threshold come from the runs' per-frame traces, counted with `tools/s6-tails.py` (§10).
- **Observed** facts were read on this Mac: files' times, the runs' environment records, SDK headers. Each names its file or command.
- **Sourced** facts link to their source, listed at the end.
- **Inference** marks an explanation drawn from them; **hypothesis** marks an untested claim, with the step that would test it.

MB means 10⁶ bytes, as B1 uses it; MiB means 2²⁰ bytes. "The first run" and "the second run" are the two formal runs; each has runs 1 to 3.

---

## Summary

| Question | Answer |
|---|---|
| **S6-Q1** Compositing cost? | **Passes at the median, in both formal runs.** A typical update (a spinner each frame, and a keystroke every sixth: 2 or 12 tiles) took 0.12–0.28 ms on the GPU and 0.58–1.93 ms on the compositor thread at the median, against 4 ms and 2 ms. No frame took more than 1.61 ms on the GPU. **The compositor thread's tail** was in each session's first runs: 192 and 225 of 600 updates took more than 2 ms in the two runs 1, and 127 in the second run's run 2, while their tile draws allocated new surfaces 14–54% of the time; the other three runs had 0–2 such updates. A full redraw of 96 tiles took 7.0–10.1 ms on the GPU, and 16.1–21.3 ms from start to the GPU's end, at the median: more than a 60 Hz frame. **Idle:** no package idle wakeup in five of the six 10 s windows, one in the sixth; 5–7 interrupt wakeups; no commits (§3) |
| **S6-Q2** Damage? | **Reported.** Only the damaged tiles were redrawn: 2 a frame for the spinner, 10 for each keystroke. Against full redraws of 96 tiles, the spinner cost 46–54× less GPU time, 5.4–12.5× less compositor time and 5.5–8.7× less energy impact by `top`; typing 8.7–11.5×, 6.1–7.7× and 4.5–6.4× less (§4) |
| **S6-Q3** Scrolling and animation by layers? | **Passes at the median, in both formal runs.** No tile was redrawn except newly exposed rows. **Animating** an embedded frame's transform and opacity drew nothing, and took at most 0.39 ms a frame. **Scrolling frames that only moved layers** took 0.32–0.44 ms at the median and 0.46–0.64 ms at the 95th percentile; 1–7 of 1,085 per run took more than 1 ms, at most 1.52 ms. The 115 frames per run that drew a newly exposed row of 10 tiles took 1.3–2.2 ms at the median, and up to 8.6 ms when the surface pool grew (§5) |
| **S6-Q4** Paths? | **Reported: GPU filling pays in time and memory, not in quality.** On icons, a chart and a map tile, filling on the GPU in the compositor (stencil then cover, 4× multisampled) reached a first draw 3.8–5.6× sooner than `vello_cpu`'s filling plus its upload, run by run, with 31–62× less memory (68–202 KB of geometry against 4.2–6.3 MB of image). Its edges differ from analytic coverage by up to 109 of 255 levels, and by more than 32 on up to 0.31% of pixels. Once filled, a cached image costs the compositor about what filling again on the GPU costs. The owner deferred GR5's amendment to the graphics milestone (§6, §8) |
| **S6-Q5** Memory and colour? | **The memory condition is not met as measured.** One window of the synthetic page, with its compositor state trimmed, came to 94.2 MB. Without the stage that uploads the synthetic renderer resources, it came to 55.8 MB, which leaves about 4 MB of B1's 60 MB for the rest of the browser (inference). The window cost 14.2 MB, Metal's device and pipelines 0.5 MB, the display lists 6.3 MB, and the visible tiles 36.5 MB, 25.2 MB of it the 96 surfaces. **The colour condition is met:** in the owner's screenshot, the P3-tagged red is (255, 0, 0) in Display P3, and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted to Display P3. The untagged surface is not converted (§7) |

**Main findings**, beyond the pass criteria:
1. **Damage tracking is required, not an optimization.** Redrawing the whole 2,560 × 1,600 window took longer than a 60 Hz frame at the median in every run (16.1–21.3 ms from start to the GPU's end). The spinner's full redraws scored 54–56 in `top`'s energy impact, against 6–10 when only its tiles were redrawn (§3, §4).
2. **The slowest frames allocate surfaces.** A tile draws into its spare surface, and its previous surface becomes the spare once CoreAnimation releases it; if CoreAnimation still holds it, a new surface is allocated. That happened in 20–71% of the full redraws' tile draws, in half of each run 1's typical-update tile draws, and in the 4 frames per scrolling run that grew the pool, which were each run's four slowest frames (2.5–8.6 ms). **Inference:** the compositor needs more than one spare per tile at full rate, a pool of free surfaces allocated ahead of need, and newly exposed rows drawn before they are needed or spread across frames (§3, §5).
3. **The first runs of each session were the slowest.** Both formal runs began with the Mac recently busy (§2). In both, run 1's typical updates were slower throughout (1.84 and 1.93 ms at the median, against 0.58–0.85 ms in the other runs), with the most surfaces allocated (§9).
4. **Tiles dominate the compositor's memory.** The 96 tiles of 256 px for one window with two embedded frames are 25.2 MB, 1.5 times the window's own pixels. B1 was set from S2's idle window before any tile was measured (§7).
5. **`vello_cpu` fills paths 2.1–7.6× faster than `zeno`,** run by run, with the same quality. S5 found `zeno` 1.7× faster on glyphs, which atlases cache. The owner adopted `vello_cpu` as the renderer's Rust rasterizer, subject to grading (§6, §8).
6. **CoreAnimation colour-manages tagged surfaces exactly, and leaves untagged ones alone.** An untagged surface's values are shown as the display's own, so Axiom must tag every surface it draws (GR6; §7).
7. **This Mac's display runs in a scaled mode,** 1,440 × 900 points in a 2,880 × 1,800 framebuffer, which the window server scales to the 2,560 × 1,600 panel. S6's window covered 1,280 × 800 of those points: it was not full screen. A full-screen window here has 27% more pixels, and the window server's scaling is not in S6's figures (§2, §8).

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
1. **The colour check is read from a screenshot.** CoreAnimation's offscreen renderer (`CARenderer`) produced no pixels in this setting. So the surfaces' tags are checked automatically, and whether CoreAnimation honours them is read from the pattern displayed: the owner's screenshot of it, measured (§7).
2. **Frames are paced by a timer,** sleeping to the next 16.7 ms slot, not by a display link.
3. **Footprints are read by the spike at each stage** (`ri_phys_footprint`, as S4, S5 and S7 read it), rather than by `/usr/bin/time -l` at exit.
4. **S6-Q4's CPU candidates** are S5's two rasterizers: `vello_cpu`, and `zeno` for comparison. `tiny-skia` was not tried. The GPU filler is stencil then cover, not tessellation or sparse strips.

**Dependencies,** each justified in the workspace manifest (DP23): the objc2 framework crates, with `objc2-io-surface` added; `vello_cpu` 0.3.0, `zeno` 0.3.3 and `skrifa`, already S5's; and S5's own `s5-rust-text`.

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B5112a); on AC power with Low Power Mode off (`pmset`); rustc 1.94.1; Command Line Tools only.
- **The display runs scaled.** The panel is 2,560 × 1,600 (`system_profiler`), but the screen is 1,440 points wide (each run's comment lines), and the owner's screenshot is 2,880 × 1,800 pixels: macOS draws a 1,440 × 900-point desktop into a 2,880 × 1,800 framebuffer and scales it to the panel (inference from those observations). The window was 1,280 × 800 points at scale 2, so 2,560 × 1,600 pixels, at the top left: not full screen.
- **Two formal runs,** each of `tools/s6-measure.sh` with `--show`, run by the owner from a clean tree. Each makes three runs of every mode, each after a 60 s cool-down, then displays the Display P3 pattern (`q5-show.txt`):

  | Formal run | Commit | Directory | Load averages at the start | The script's build |
  |---|---|---|---|---|
  | First, 2026-10-19 20:14–20:26 | `45bff95` | `target/s6-results/20261019-201443/` | 103.80, 59.25, 24.79 | Compiled nothing: the binary dated from 2026-10-18 21:44:50 |
  | Second, 2026-10-20 07:47–07:59 | `b4948ce` | `target/s6-results/20261020-074751/` | 14.35, 25.44, 12.90 | Recompiled the spike: merging `step-5` had re-created its sources at 01:35:16 (observed, `ls -lT`) |

  In both, the window was ordered in and unoccluded at the start and the end of all 25 measured runs (each run's comment lines, collected in `summary.txt`). Neither began on an idle Mac, so the method's "otherwise idle" holds at best after the first cool-down (inference). §9 discusses the effect.
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

The first draw of the page's 96 tiles (70 for the page, 20 for the outer frame, 6 for the inner one) took 6.58–9.71 ms on the GPU. Then, in milliseconds, median / 95th percentile / maximum:

| Formal run | Run | Scenario | Tiles a frame | GPU | Compositor thread | Wall | New surfaces |
|---|---|---|---|---|---|---|---|
| First | 1 | Full redraw, 120 frames | 96 | 7.97 / 10.42 / 19.73 | 6.94 / 9.15 / 12.30 | 16.26 / 21.41 / 28.99 | 7,302 of 11,520 tile draws |
| | 2 | | 96 | 8.19 / 10.60 / 11.19 | 7.39 / 10.62 / 12.43 | 16.72 / 22.84 / 24.42 | 8,221 |
| | 3 | | 96 | 10.12 / 11.20 / 11.56 | 8.32 / 10.08 / 10.99 | 20.26 / 22.49 / 23.62 | 8,084 |
| Second | 1 | | 96 | 8.12 / 9.03 / 14.22 | 6.66 / 7.15 / 8.26 | 16.06 / 17.28 / 24.86 | 8,064 |
| | 2 | | 96 | 10.13 / 10.73 / 12.14 | 9.20 / 10.45 / 11.05 | 21.34 / 22.97 / 25.06 | 7,425 |
| | 3 | | 96 | 6.97 / 7.55 / 7.91 | 8.10 / 8.98 / 15.00 | 16.59 / 17.21 / 23.63 | 2,343 |
| First | 1 | Typical update, 600 frames | 2 or 12 | 0.28 / 1.22 / 1.46 | **1.84 / 2.93 / 3.57** | 2.84 / 4.96 / 5.95 | 1,108 of 2,200 tile draws |
| | 2 | | 2 or 12 | 0.18 / 1.32 / 1.48 | 0.67 / 1.84 / 1.97 | 1.58 / 3.92 / 4.34 | 13 |
| | 3 | | 2 or 12 | 0.17 / 1.30 / 1.48 | 0.72 / 1.86 / 2.27 | 1.58 / 3.89 / 4.09 | 21 |
| Second | 1 | | 2 or 12 | 0.28 / 1.46 / 1.61 | **1.93 / 3.14 / 3.36** | 2.94 / 5.25 / 5.80 | 1,183 |
| | 2 | | 2 or 12 | 0.24 / 1.29 / 1.53 | **0.85 / 2.44 / 3.55** | 1.78 / 4.02 / 5.71 | 318 |
| | 3 | | 2 or 12 | 0.12 / 0.89 / 1.34 | 0.58 / 1.56 / 2.52 | 1.24 / 2.98 / 4.43 | 12 |

- **Typical updates over 2 ms on the compositor thread:** 192, 0 and 2 of 600 in the first run; 225, 127 and 1 in the second. In the slow runs, updates of 2 tiles took 1.80 and 1.89 ms at the median (runs 1), and updates of 12 tiles 2.81 and 3.01 ms. In the second run's run 2, updates of 2 tiles took 0.81 ms at the median, but 112 of 500 exceeded 2 ms. In the fast runs, updates of 2 tiles took 0.52–0.69 ms, and of 12 tiles 1.20–1.84 ms. Each slow run was slow in both halves of its series, so it wasn't a warm-up within the run.
- **Idle,** 10 s in each run: 0 package idle wakeups in five runs, 1 in the second run's run 2; 6, 6 and 6 interrupt wakeups in the first run, 6, 7 and 5 in the second; no commits. The footprint at the end was 70.0–73.9 MB.

**Verdict: passes at the median,** the statistic "a typical update takes" describes, and the one CP11's trials reported. The GPU stayed within 4 ms in every frame of every run. The compositor thread stayed within 2 ms at the median in every run; its tail is in the sessions' first runs (§9). The idle condition held in five of six windows in the measure S2 used, package idle wakeups; the sixth had one. The interrupt wakeups are unexplained (hypothesis: AppKit's run loop on the main thread; a longer idle window at the shell milestone would show whether they are periodic).

## 4. S6-Q2: damage

Medians for runs 1, 2 and 3 of each formal run. Times in milliseconds; energy impact is `top`'s mean over 16–18 samples.

| Animation | Redraw | Tiles a frame | Formal run | GPU | Compositor thread | Energy impact |
|---|---|---|---|---|---|---|
| Spinner, 1,200 frames | Every visible tile | 96 | First | 8.09, 7.02, 7.89 | 7.24, 7.38, 7.49 | 54.0, 56.1, 55.8 |
| | | | Second | 7.28, 7.20, 6.28 | 7.10, 7.42, 7.17 | 53.7, 55.1, 55.5 |
| | Damaged tiles | 2 | First | 0.17, 0.15, 0.16 | 0.60, 0.59, 0.64 | 6.2, 7.7, 6.8 |
| | | | Second | 0.16, 0.14, 0.12 | 0.62, 1.16, 1.34 | 6.6, 8.9, 10.1 |
| Typing, 200 keystroke frames | Every visible tile | 96 | First | 10.18, 9.29, 9.89 | 10.72, 6.85, 10.75 | 14.6, 10.4, 14.7 |
| | | | Second | 9.83, 9.26, 6.22 | 10.81, 5.96, 6.69 | 15.0, 9.0, 10.8 |
| | Damaged tiles | 10 | First | 1.17, 0.83, 1.14 | 1.62, 1.12, 1.62 | 2.8, 2.3, 2.7 |
| | | | Second | 1.11, 0.84, 0.54 | 1.62, 0.98, 0.87 | 3.3, 1.9, 1.7 |

**Savings,** run by run: the spinner, 46–54× in GPU time, 5.4–12.5× in compositor time and 5.5–8.7× in energy impact; typing, 8.7–11.5×, 6.1–7.7× and 4.5–6.4×. The lower spinner figures come from the second run's runs 2 and 3, whose damage-limited compositor time doubled with no change in the tiles drawn (§9). Typing's energy is averaged over the whole run, in which only every sixth frame draws. Its damage is the field's whole rectangle, which touches 10 tiles; damage limited to the changed glyphs and the caret would touch fewer (inference).

**Verdict:** the criterion is met. Only the tiles that damage touches were redrawn, as counted, and the saving is reported.

## 5. S6-Q3: scrolling and animation by layers

Compositor thread, in milliseconds, median / 95th percentile / maximum:

| Motion | Frames a run | Tiles a frame | Formal run | Run 1 | Run 2 | Run 3 | Over 1 ms, runs 1–3 |
|---|---|---|---|---|---|---|---|
| Animation | 1,200 | 0 | First | 0.22 / 0.29 / 0.35 | 0.19 / 0.24 / 0.29 | 0.23 / 0.27 / 0.35 | 0, 0, 0 |
| | | | Second | 0.27 / 0.31 / 0.35 | 0.26 / 0.31 / 0.39 | 0.19 / 0.26 / 0.31 | 0, 0, 0 |
| Scrolling: layers moved only | 1,085 | 0 | First | 0.40 / 0.63 / 1.52 | 0.32 / 0.46 / 1.03 | 0.40 / 0.64 / 1.49 | 7, 2, 6 |
| | | | Second | 0.44 / 0.59 / 1.24 | 0.33 / 0.53 / 1.09 | 0.34 / 0.51 / 1.20 | 5, 1, 1 |
| Scrolling: a newly exposed row drawn | 115 | 10 | First | 2.17 / 2.28 / 6.31 | 1.47 / 2.08 / 6.15 | 2.18 / 2.32 / 8.58 | — |
| | | | Second | 2.14 / 2.36 / 7.50 | 1.28 / 2.57 / 6.55 | 1.84 / 2.02 / 5.76 | — |

- Frames that drew a new row took 0.80–1.80 ms on the GPU at the median, and at most 2.51 ms.
- In each run, 4 of those frames also grew the surface pool, allocating 10 surfaces each. They took 2.5–8.6 ms: the four slowest frames of each run.
- The animation drew no tile and allocated nothing.

**Verdict: passes at the median.** The ≤ 1 ms threshold is read for the frames that only move layers, which the spike reports separately for this purpose. Animation stayed within it in every frame; scrolling at the median and at the 95th percentile, with 1–7 of 1,085 frames per run over it. Frames that draw a newly exposed row do a typical update's work in the same frame, and took 1.3–2.2 ms at the median.

## 6. S6-Q4: paths

Each figure is the range over both formal runs of the six runs' medians, each of 5 repeats. Times in milliseconds. Differences are per colour channel, against `vello_cpu`'s pixels, on a scale of 0–255; "over 32" is the share of pixels whose worst channel differs by more than 32 levels. The pixels were identical in every run.

| Content | Paths; segments | Method | Fill or prepare (CPU) | Upload | GPU | Memory, bytes | Mean difference | Largest | Over 32 |
|---|---|---|---|---|---|---|---|---|---|
| Icons, 1,024 × 1,024 | 128; 1,173 | `vello_cpu`, uploaded as an image | 0.84–1.35 | 0.54–0.80 | 0.32–0.40 | 4,194,304 | Reference | | |
| | | `zeno` | 2.03–2.83 | | | 4,194,304 | 0.104 | 46 | 0.092% |
| | | GPU: stencil then cover, 4× | 0.16–0.20 | | 0.25–0.28 | 75,400 | 0.216 | 72 | 0.193% |
| Chart, 2,048 × 768 | 13; 12,019 | `vello_cpu`, uploaded as an image | 2.00–2.33 | 0.75–0.86 | 0.46–0.58 | 6,291,456 | Reference | | |
| | | `zeno` | 14.46–15.24 | | | 6,291,456 | 0.013 | 5 | 0 |
| | | GPU: stencil then cover, 4× | 0.14–0.16 | | 0.59–0.63 | 202,304 | 0.248 | 109 | 0.312% |
| Map tile, 1,024 × 1,024 | 511; 3,172 | `vello_cpu`, uploaded as an image | 1.53–1.62 | 0.47–0.56 | 0.32–0.42 | 4,194,304 | Reference | | |
| | | `zeno` | 7.52–7.87 | | | 4,194,304 | 0.018 | 26 | 0 |
| | | GPU: stencil then cover, 4× | 0.072–0.088 | | 0.49–0.56 | 68,144 | 0.195 | 42 | 0.001% |

**What it shows:**
- **To a first draw,** filling with `vello_cpu`, uploading and drawing the image took 3.8–5.6× longer than preparing and filling on the GPU, run by run over both formal runs (icons 4.1–5.6×, chart 4.4–5.0×, map tile 3.8–4.4×). By the first run's medians: 2.18 ms against 0.45 for the icons, 3.55 against 0.75 for the chart, 2.44 against 0.63 for the map tile, **3.9–4.8× sooner on the GPU,** the figure the other documents quote. Uploading the image, which the compositor would do, costs 0.47–0.86 ms, against 0.072–0.20 ms to prepare the geometry.
- **Memory:** 68–202 KB of geometry against 4.2–6.3 MB of image, **31–62× less.** The multisampled attachments are memoryless, so they add nothing.
- **Once filled,** drawing the cached image costs the GPU 0.32–0.58 ms, about what filling again costs (0.25–0.63 ms).
- **Quality:** 4× multisampling gives coarser edges than analytic coverage. On the chart, the content with the most edges, the largest difference is 109 levels, and 0.31% of pixels differ by more than 32; on the icons, 72 levels and 0.19%. `zeno` stays closer to `vello_cpu` (0–0.092% of pixels over 32).
- **Between the CPU rasterizers,** `vello_cpu` filled 2.1–2.5× (icons), 4.9× (map tile) and 6.2–7.6× (chart) faster than `zeno`, run by run; 2.1–6.8× by the first run's medians.

**Verdict:** reported. GPU filling in the compositor pays in time and memory, not in quality, which made GR5's amendment the owner's decision (§8).

## 7. S6-Q5: memory and colour

**Footprint at each stage** (the mean of the first run's three runs; every stage of the second run is within 0.1 MB of it, except after the updates):

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
| Peak (lifetime maximum) over both runs | 101.6–105.3 | |

- **The atlas stage** uploaded 17.8 MB of texture data, a 1 MiB coverage atlas and a 16 MiB image atlas in shared storage. The other 20.6 MB are unattributed (hypothesis: Metal's allocations for shared textures, and the font file and image buffer read to build them; attributed at the graphics milestone with `footprint` or `vmmap`, which ship with macOS, observed in `/usr/bin`).
- **The tile stage** holds 25.2 MB of surfaces; its other 11.4 MB are likewise unattributed.
- **The window's visibility changes the figures.** The informal quick run of 2026-10-18 (on battery, in Low Power Mode, with the window occluded by a full-screen app) ended at 46.6 MB idle and 73.6 MB after trimming, about 21 MB below the formal runs at the same stages (`target/s6-results/20261018-214604/summary.txt`). Hypothesis: the window server holds memory charged to a visible window's process; attributed at the graphics milestone.

**Against B1** (the [process model](../security/process-model.md) §7: the browser process at ≤ 60 MB, idle, with one window; set from "S2's 20 MB, plus browser UI, profile and compositor"):
- The spike, idle with one window and its compositor state trimmed, uses 94.2 MB.
- Without the atlas stage, 55.8 MB remain: the window and process 16.1 MB, Metal's device and pipelines 0.5 MB, the display lists 6.3 MB, and the tiles, less what trimming returned. The atlases stand in for renderer resources, whose textures GR12 counts in the browser but whose size depends on the page.
- That leaves about 4 MB of B1 for the browser's UI, its profile data and every other structure, and none for the page's resources. **The condition is not met as measured** (inference).
- **Why B1 missed** (inference): S2's 20 MB was a 960 × 200-point window with a `CAMetalLayer`'s drawables. A window of this size with Metal and no content costs 16.6 MB, but tiles scale with the window's pixels: 25.2 MB of surfaces here, 1.5 times the window's own pixels, because tiles overhang the edges and the embedded frames have layers of their own. A full-screen window in this Mac's display mode would be 2,880 × 1,800 pixels, 27% more.

**Colour.** Three 256-pixel surfaces were filled with pure red: one tagged Display P3, one tagged sRGB, one untagged. In both runs, each carries its tag as intended and reads back [255, 0, 0, 255] at its centre, since a tag doesn't change the stored values. At the end of the second run the pattern was displayed at the window's top left, and **the owner took a screenshot** (a 2,880 × 1,800 PNG; kept with the run as `q5-show-screenshot.png`). Measured with `tools/png-region-stats.py`, over a 100 × 80-pixel region inside each square:

| Square | Surface's tag | In the screenshot | If the tag is honoured |
|---|---|---|---|
| Left | Display P3 | (255, 0, 0), in all 8,000 pixels | (255, 0, 0): P3's red primary |
| Middle | sRGB | (234, 51, 35), in all 8,000 pixels | (234, 51, 35): sRGB's red, in Display P3 |
| Right | None | (255, 0, 0), in all 8,000 pixels | Not defined |

- **How the values are encoded:** the screenshot's `cICP` chunk declares colour primaries 12 and transfer function 13 in ITU-T H.273's code points, which are Display P3's primaries with sRGB's transfer function (background), and PNG makes `cICP` the highest-precedence colour chunk [P1]. Its embedded ICC profile, named "Display", has Display P3's colorants (`rXYZ` 0.51512, 0.24120, −0.00105, and so on). Display P3 uses DCI-P3's primaries with a D65 white and sRGB's transfer curve [C1].
- **The expected value of sRGB's red in Display P3,** computed independently from the two spaces' primaries: linear (0.8226, 0.0332, 0.0171), encoded with the sRGB curve as (234, 51, 35). With a plain 2.2 gamma the same linear values would encode as (233, 54, 40), so the conversion follows the standard curve.
- **So CoreAnimation honours both tags.** The P3-tagged red reaches the panel's full red, outside sRGB's gamut, and the sRGB-tagged red is converted exactly.
- **The untagged surface is not converted:** its values are taken as the display's own. On this P3 panel that matches P3; on a display with another gamut, the same values would show a different colour (inference).
- **Limits:** a screenshot holds the framebuffer's values, before the display's calibration curve (the profile's `vcgt` tag) shapes the light; only red was tested; extended range and HDR were not, as GR6 defers them.

**Verdict:** the device, pipelines and tiles are measured, as the question asks. The memory condition is not met as measured (§8, item 6). **The colour condition is met:** a known test pattern displays correctly in P3.

## 8. Implications for Axiom

1. **ADR-0012's conditions, and the owner's decision.** The owner's Gate B decision made acceptance depend on "S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox". S6-Q1 and S6-Q3 pass at the median in both formal runs, with the tails in §3 and §5, and S6-Q1's idle condition held in five of six windows. S6-Q5's colour condition is met (§7); its memory condition is not met as measured. S5-Q5's condition was not met ([S5 results](s5-results.md) §7). **At Gate C (2026-10-20) the owner accepted ADR-0012:** "The formal S6 benchmark run is complete (target/s6-results/20261019-201443/summary.txt) and easily meets all frame time thresholds. IM3's fallback (AVIF in the media utility process) is approved."
2. **GR2 holds, and damage tracking is required:** full redraws of the 2,560 × 1,600 window miss a 60 Hz frame, and damage-limited tiles cost a spinner 46–54× less GPU time (§3, §4).
3. **GR8 holds:** moving layers costs 0.19–0.44 ms a frame at the median (§5).
4. **Surfaces must be ready before they are needed** (inference from §3 and §5). With one spare per tile, CoreAnimation often still holds a tile's previous surface at full rate, and allocating a new one made the slowest frames. At the graphics milestone: enough spares for full rate, a pool kept allocated ahead of need, and newly exposed rows drawn ahead or spread across frames, measured against this report's traces.
5. **GR5: the amendment is deferred to the graphics milestone,** by the owner (2026-10-20): "Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone." GPU filling paid in time (3.9–4.8× sooner to a first draw, by the first run's medians) and in memory (31–62× less), not in quality (§6). At that milestone the amendment would come with a GPU filler whose edges match analytic coverage, such as sparse strips, measured on S6-Q4's contents.
6. **B1 is revised,** as the owner approved (2026-10-20): "In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state." The process model's change record notes it, and Step 6 sets the tile budget. The unexplained parts in §7 are attributed when the compositor is built.
7. **`vello_cpu` is adopted** as the renderer's Rust rasterizer for web fonts and paths, by the owner (2026-10-20): "Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone." It filled paths 2.1–7.6× faster than `zeno` (§6), and S5 found `zeno` 1.7× faster on glyphs, which atlases cache ([S5 results](s5-results.md) §4). `zeno` stays the alternative, with its unsound conversion guarded as S5 guards it.
8. **GR6 holds, and every surface must be tagged.** CoreAnimation colour-manages tagged surfaces exactly, and shows untagged ones unconverted, in whatever space the display has (§7). Axiom tags every surface it draws with the window's working space.
9. **The display's scaled mode is an input to Step 6** (§2). A full-screen window on this Mac is 2,880 × 1,800 pixels, 27% more than S6's window, so its tiles would cost about 32 MB of surfaces at the same tile size (inference). The window server's composition and its scaling to the panel are costs outside the compositor's own figures.
10. **The tile size:** 256-pixel tiles were used throughout and met the thresholds; no other size was measured formally. 256 px is GR2's starting point at the graphics milestone.
11. **Idle (AC3, PM8):** no package idle wakeups and no commits in five of six 10 s windows, one wakeup in the sixth. The interrupt wakeups are left for the shell milestone, which owns the main thread's run loop (§3).

## 9. Threats to validity

1. **The start of each session.** Both formal runs began with the Mac recently busy (§2). In both, run 1's typical updates were the slowest, and half their tile draws allocated a surface; the second run's run 2 was in between (127 updates over 2 ms; 14.5% of draws allocating). **Hypothesis:** the window server, still busy or not yet back at full performance, released surfaces late, so the compositor allocated, and paid for it. A run after a longer idle period would test it. The later runs agree with each other and with CP11's informal trials.
2. **Something else ran during the second run's damage measurements.** The spinner's damage-limited runs 2 and 3 took 1.16 and 1.34 ms of compositor time at the median, against 0.59–0.64 ms in the four other runs, and scored higher in energy (8.9 and 10.1), with the same tiles drawn (inference: other work on the Mac).
3. **Synthetic content.** The display lists are generated: one page shape, fixed atlases (the 16 MiB image atlas is larger than most pages need), and damage the spike chooses. Real pages' costs differ.
4. **Timer pacing,** not a display link (deviation 2), so frame times aren't aligned with the display's refresh.
5. **`top`'s energy impact** is a relative score, sampled once a second (16–18 samples a run).
6. **Footprint attribution** is by stage, and leaves a third of the atlas stage and of the tile stage unexplained (§7). The window's visibility changes the totals by about 21 MB.
7. **One machine, one OS build, one display mode.** macOS 27.2 (26B5112a) on an M1, in a scaled 1,440 × 900-point mode. CoreAnimation's and the window server's behaviour may change with updates, and a native mode would spare the window server its scaling.
8. **The colour check rests on one screenshot** of the framebuffer, before the display's calibration, and on one colour (§7).

## 10. Reproducing these results

```sh
cargo test -p s6-compositor          # tiling, text, scene, path and counter tests
tools/s6-measure.sh --show 30        # everything; target/s6-results/<timestamp>/
                                     # formal runs: AC power, the Mac otherwise idle, the window visible
python3 -I tools/s6-tails.py target/s6-results/<timestamp>     # frames over each threshold
python3 -I tools/png-region-stats.py SCREENSHOT.png p3:160,300,260,380 srgb:440,300,540,380 none:720,300,820,380
```

The regions are those used for the screenshot of the second run; the squares' positions depend on the display mode.

| Run | Directory |
|---|---|
| First formal run (commit `45bff95`, clean tree, run by the owner) | `target/s6-results/20261019-201443/`: `summary.txt`, `environment.txt`, `q1-run*`, `q2-*-run*` with their `.top.txt` samples, `q3-*-run*`, `q4-run*`, `q5-run*`, `q5-show.txt`, and each timed scenario's `.trace.tsv` |
| Second formal run (commit `b4948ce`, clean tree, run by the owner) | `target/s6-results/20261020-074751/`: the same files, and `q5-show-screenshot.png`, the owner's screenshot |
| Informal quick run (2026-10-18, on battery, the window occluded) | `target/s6-results/20261018-214604/` |

## Sources

All checked on 2026-10-20.
- [C1] W3C, *CSS Color Module Level 4*, Candidate Recommendation Draft, 9 October 2026, §10.4 "The Predefined Display P3 Color Space": red (0.680, 0.320), green (0.265, 0.690) and blue (0.150, 0.060), the D65 white point, and "the same transfer curve as sRGB" — https://www.w3.org/TR/css-color-4/#predefined-display-p3
- [P1] W3C, *Portable Network Graphics (PNG) Specification (Third Edition)*, Recommendation, 24 June 2025, §11.3.2.6 "cICP": four fields (colour primaries, transfer function, matrix coefficients, video full range flag) in ITU-T H.273's code points; "the highest-precedence color chunk" — https://www.w3.org/TR/png-3/
- Background: ITU-T H.273's code points, colour primaries 12 (SMPTE EG 432-1, P3 with a D65 white) and transfer characteristics 13 (IEC 61966-2-1, sRGB).
- Observed on this Mac: the SDK 27 headers `Metal/MTLCommandBuffer.h` (`GPUStartTime`, `GPUEndTime`), `IOSurface/IOSurfaceRef.h`, `QuartzCore/CALayer.h`, `libproc.h`, `sys/resource.h` (`rusage_info_v4`) and `_time.h`; `top(1)`; `pmset`; `system_profiler SPDisplaysDataType`; `ls -lT`; the owner's screenshot, read with `tools/png-region-stats.py`.
- This repository: `spikes/s6/`, `tools/s6-measure.sh`, `tools/s6-tails.py`, `tools/png-region-stats.py`; the [Step 5 tracker](step-5-subsystems.md) §10.2; [graphics](../architecture/graphics.md); [S2 results](s2-results.md) for the idle window; [S5 results](s5-results.md) for the rasterizers.
