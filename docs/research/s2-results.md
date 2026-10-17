# S2 Results: AppKit, Metal and CoreText from Rust, without Xcode

| | |
|---|---|
| **Status** | **Reviewed** by the owner (2026-10-17). The smoke test and the owner's visual check are satisfactory; thresholds T1–T4 (§6) are accepted into [ADR-0002](../adr/0002-implementation-language.md) |
| **Question** | Can AppKit, a `CAMetalLayer` and CoreText draw text through objc2 with the Command Line Tools alone, with `unsafe` confined to platform modules (S2-Q5)? What do the bindings cost to build, and the app to run (S2-Q6)? |
| **Feeds** | [ADR-0002](../adr/0002-implementation-language.md): final acceptance, and the thresholds for revisit condition 3 (OQ-12). Also OQ-11 (pure Rust shell or a Swift layer), OQ-7 (when to install Xcode), and the platform layer of the architecture |
| **Plan and raw log** | [Step 2b tracker](step-2b-validation-spikes.md): questions §3, CP8 in §6, results in §8 |
| **Code** | `spikes/s2/window` on branch `step-2b`. Measurements ran at commit `d9e02bf` |
| **Last updated** | 2026-10-17 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** numbers come from this repository's scripts (§7).
- **Inference** marks an explanation drawn from measurements.
- **Hypothesis** marks an untested claim; each one names its validation step.

---

## Summary

| Question | Answer |
|---|---|
| **S2-Q5** Does it work, with `unsafe` contained? | **Yes.** An AppKit window hosts a `CAMetalLayer`. A Metal pipeline compiled from MSL at run time draws a CoreText line that CoreGraphics rasterized into a Rust-owned mask. Everything was built with the Command Line Tools alone. The smoke test reads the presented frame back and matches the predicted ink **pixel for pixel**: 19,052 of 19,052 in every run, and none outside the text box. All 19 `unsafe` blocks are in `src/platform/`, under six documented invariants; `main.rs` is `#![deny(unsafe_code)]` |
| **S2-Q6** What does it cost to build? | **Small, provided the bindings' features are trimmed.** With only the headers it uses, `s2-window` builds clean in **6.0 s** (dev) or **6.2 s** (release), its largest `rustc` uses 348 MiB, and incremental builds take 0.36 s. objc2's defaults compile every framework header: **29 s** clean, with **1.4 GiB** for the `objc2_app_kit` crate alone, and an identical binary. The whole S1 + S2 workspace builds clean in **8.1 s** |
| **S2-Q6** What does it cost to run? | **Little.** The binary is 533 KB (432 KB stripped). The first frame is presented **174 ms** after `main` starts (median of 5, warm shader cache). An idle window costs **0.0% CPU, no wakeups and an energy impact of 0.0**, with a **20–21 MB** memory footprint. The peak footprint while drawing is 35.6 MiB |

**Main findings:**
- **There is no offline Metal compiler without Xcode** (`xcrun -f metal` fails). Shaders must be compiled from source at run time, and the OS caches the result. The first compile after a build took about 0.5 s; afterwards, device and shaders were ready about 69 ms after `main` starts.
- **objc2 makes most of AppKit and Metal safe Rust.** By a manual count, about four platform calls in five are ordinary safe calls. What stays `unsafe` is concentrated in four places: raw pointers, GPU resource bounds, `NSWindow`'s ownership quirk, and the CoreFoundation/CoreText C APIs.
- **Default features are the main build-cost risk, and the fix is cheap.** Trimming features cut clean build time 4.9×, the largest `rustc`'s memory 4.1×, and the aggregate compiler peak 3.5×.

**Proposal to the owner (§6): thresholds for ADR-0002 revisit condition 3**, built around the inner edit-build-test loop and the 8 GB of memory. Today's build times are 6–40× below the targets, and today's memory peaks 3–4× below them.

---

## 1. What was built

| Part | How |
|---|---|
| Application | `NSApplication` with a regular activation policy (Dock icon, can come to the front) and no app bundle. `finishLaunching` is called directly, and events are pumped by hand with `nextEventMatchingMask:untilDate:inMode:dequeue:`. When nothing needs drawing, the loop blocks inside AppKit |
| Window | `NSWindow` (titled, closable, miniaturizable, resizable), with `releasedWhenClosed = false` (P1). Its content view is *layer-hosting*: it is given a `CAMetalLayer`, then layer backing is turned on |
| Metal | Default device and command queue. The render pipeline's MSL source is compiled at run time with `newLibraryWithSource:options:error:`. The quad's corners come from `vertex_id`, so there is no vertex buffer; blending is premultiplied alpha. The pipeline draws into the layer's `BGRA8Unorm` drawables |
| Text | CoreText lays out one `CTLine` (Helvetica, 26 pt at the window's scale). CoreGraphics rasterizes it into an 8-bit grayscale bitmap context over a Rust `Vec<u8>`. The mask is uploaded as an `R8Unorm` texture and drawn 1:1 in pixels |
| `--smoke` | Draws 3 frames, blits the last presented drawable into a shared buffer, and checks three things: the background is exact; no ink lies outside the text box; the ink inside matches the count predicted from the mask by the blending formula |
| Tests | 6 tests that need no window, under `cargo test`: option parsing; centring; the oracle accepting a correct frame and rejecting stray, missing or misplaced ink; CoreText rasterization; device creation, MSL compilation, pipeline creation and texture upload |

**Dependencies:**
- `objc2` 0.6.5, plus `objc2-foundation`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-metal`, `objc2-core-text`, `objc2-core-graphics` and `objc2-core-foundation`, all 0.3.2.
- Default features are off; `s2-window` enables 6 Foundation, 9 AppKit, 4 QuartzCore, 16 Metal, 4 CoreText, 4 CoreGraphics and 5 CoreFoundation features.
- A `full-bindings` cargo feature restores the defaults, for measurement only.

---

## 2. Method and environment

- **Machine and toolchain.** MacBook Air M1 (8 GB), macOS 27.2, on AC power; Command Line Tools only; rustc 1.94.1. The drawable was 1920×400 pixels: a 960×200-point window at scale 2.
- **Build.** `tools/measure-build.sh --runs 3`: clean, no-op and incremental builds, with per-`rustc` and per-link peak memory and the aggregate compiler peak (tracker §7). Medians are reported, with ranges where they matter.
- **Run time.** `tools/s2-measure.sh --runs 5 --idle 20`:
  - **5 smoke runs** under `/usr/bin/time -l`, each with the app's own timings from `main`.
  - **5 idle runs** of 20 s each, with the window open and no input. Three seconds in, `top` samples %CPU, idle wakeups, its energy-impact score and memory footprint over two seconds. `ps` reads the accumulated CPU time over the next 13 s.
- **What isn't measurable without root:** energy in joules (`powermetrics`). `top`'s energy impact is macOS's relative score, the one Activity Monitor shows.

---

## 3. S2-Q5: does it work, and where is the `unsafe`?

**It works** (measured):
- every smoke run passed its pixel oracle with an exact match;
- the 6 tests pass under `cargo test`;
- the policy check passes: `s2-window` is allowlisted for `src/platform/` only.

**Pending:** your visual check. Run `cargo run -p s2-window`; the window stays open until it is closed.

**The `unsafe` inventory.** There are 19 blocks, each with a `// SAFETY:` comment citing an invariant in `src/platform/mod.rs`.

| Kind | Blocks | Invariant | Where |
|---|---:|---|---|
| Raw pointers into Rust memory: texture upload, vertex bytes, the bitmap context's buffer, three out-parameters in one call, a null font matrix | 5 | P2 | Metal, CoreGraphics, CoreText |
| Reading a shared GPU buffer after its command buffer completed | 1 | P3 | Metal |
| GPU bounds: indexed subscripts (2), binding the texture, the draw, the blit, the texture descriptor | 6 | P4, P5 | Metal |
| Window ownership: `initWithContentRect…` and `setReleasedWhenClosed` | 2 | P1 | AppKit |
| Extern statics: `NSDefaultRunLoopMode`, and CoreText's attribute keys | 2 | P6 | Foundation, CoreText |
| C functions the bindings mark `unsafe` without a pointer argument (`CFAttributedStringCreate`, `CTLineCreateWithAttributedString`, `CTLineDraw`) | 3 | Contract documented at each call | CoreFoundation, CoreText |

**Where objc2 helped (measured by reading the code, a manual count).** About 78 platform call sites are safe Rust calls, against the 19 `unsafe` ones. They include:
- the whole application and window set-up, apart from P1;
- every layer, device, queue, library, pipeline, descriptor, encoder, command-buffer and presentation call;
- CoreFoundation strings, dictionaries and booleans;
- CoreGraphics colour spaces and context state.

The bindings place `unsafe` where a Rust type can't express the contract, which is a reasonable line.
- **Metal is inherently `unsafe`.** A shader can read beyond a buffer. P4 states the shader's resource contract once, and every draw honours it.
- **The CoreText and CoreGraphics C APIs are the most `unsafe`-dense:** 7 of their 15 calls.

**Findings:**
1. **There is no offline Metal compiler with the Command Line Tools** (measured: `xcrun -f metal` fails). The only Xcode-free path is run-time compilation from source, which macOS caches per user.
   - The first compile after a build took about 0.5 s (one observation).
   - With a warm cache, device and shaders were ready at a median of 69 ms (range 68–82, n = 5).
   - Clearing the cache to force cold starts was ruled out: it is shared by every program the user runs.
2. **CoreText draws black unless told otherwise.** It uses the context's fill colour only when `kCTForegroundColorFromContextAttributeName` is set. The first smoke run caught this as an empty mask, against a prediction of no ink.
3. **`NSWindow` releases itself on close unless told not to.** objc2-app-kit makes window creation `unsafe` for this reason, and requires `releasedWhenClosed(false)` outside a window controller [1].
4. **No Objective-C class had to be defined.** A layer-hosting `NSView` with a hand-pumped loop needs no subclass and no `define_class!`.
   - **The cost:** during a live resize, AppKit runs its own tracking loop, so the last frame is stretched until the mouse is released.
   - **The fix in an engine:** a view subclass, or a display link, would redraw during the resize.
5. **objc2's framework crates enable every header by default**: 307 features for AppKit alone, which also pull in CloudKit, CoreData, CoreImage, CoreVideo and IOSurface bindings. Their build cost is in §4.

---

## 4. S2-Q6: build cost

**Measured**, 3 runs each, medians, with ranges for clean builds:

| Build of `s2-window` | Clean | Incremental (touch `main.rs`) | `rustc` runs | Largest `rustc` (crate) | Largest link | Aggregate compiler peak |
|---|---:|---:|---:|---|---:|---:|
| Minimal bindings, dev | **5.97 s** (5.78–6.12) | 0.36 s | 13 | 348 MiB (`objc2_foundation`) | 161 MiB | 386–415 MiB |
| Minimal bindings, release | **6.17 s** (6.10–6.58) | 0.50 s | 13 | 340 MiB (`objc2_foundation`) | 143 MiB | 365–380 MiB |
| Full bindings (objc2 defaults), dev | **29.1 s** (29.0–29.5) | 0.45 s | 21 | **1,426 MiB** (`objc2_app_kit`) | 258 MiB | 1,387–1,410 MiB |
| Full bindings, release | **29.9 s** (29.6–30.8) | 0.50 s | 21 | 1,365 MiB (`objc2_app_kit`) | 174 MiB | 1,334–1,406 MiB |

**Where the full bindings' time goes** (dev, run 2):

| Crate | Time | Peak `rustc` memory |
|---|---:|---:|
| `objc2_app_kit` | 14.2 s | 1,412 MiB |
| `objc2_foundation` | 8.0 s | 1,125 MiB |
| `objc2_metal` | 4.3 s | 626 MiB |
| Everything else | under 1 s each | under 300 MiB each |

**Binary size** (release): 533,104 bytes as built and 432,112 stripped with minimal bindings; 533,088 and 432,104 with full bindings. Unused bindings cost compile time, not binary size.

**The whole workspace** (S1 and S2, dev, all targets, 3 runs): **8.12 s** clean, with 44 `rustc` runs and 23 links. The largest `rustc` used 342 MiB (`objc2_foundation`), and the aggregate peak was 867–941 MiB. An incremental rebuild after touching `s1-a-dom` took 0.82 s.

**Inference.** Feature-trimming the bindings is the one build-cost mitigation S2 shows to be necessary:
- it cuts clean build time 4.9×, the largest `rustc`'s memory 4.1×, and the aggregate compiler peak 3.5×;
- it costs one feature list per crate, which the compiler checks: a missing feature is a compile error, not a run-time failure.

The rest of the platform stack adds about 6 s to a clean build.

---

## 5. S2-Q6: run-time cost

**Startup** (measured; 5 smoke runs; times from `main`; warm shader cache):

| Milestone | Median | Range |
|---|---:|---:|
| Metal device and shaders ready | 69.0 ms | 67.7–82.3 |
| Window shown | 135.9 ms | 133.1–146.0 |
| First frame presented (the GPU has completed it) | **174.2 ms** | 172.2–186.2 |
| Whole smoke run, 3 frames, readback and check (wall) | 0.2 s | 0.2–0.5 |
| Peak memory footprint | 35.6 MiB | 35.5–35.6 |
| Maximum RSS (includes shared AppKit and Metal pages) | 86.6 MiB | 86.6–86.7 |
| Instructions retired, whole run | 693 M | 690–720 M |

**Idle** (measured; 5 runs; window open, no input):

| Measure | All 5 runs |
|---|---|
| %CPU (`top`) | 0.0 |
| Idle wakeups (`top`) | 0 |
| Energy impact (`top`) | 0.0 |
| Memory footprint (`top`) | 20–21 MB |
| CPU time accumulated over 13 s of idle (`ps`) | 0.00–0.01 s |
| RSS (`ps`; includes shared framework pages) | 80.3–80.4 MiB |

**Inference.** The idle numbers are what the event-driven design predicts: the loop sleeps inside AppKit until an event arrives, and draws only when something changed. For an engine whose charter makes power a primary goal (G4), this is the baseline to protect: no timers, animation or display link may run while nothing is changing.

The **footprint**, not the RSS, is the per-process cost on this machine, since RSS includes framework pages shared with every other app:
- about 20 MB for an idle window with a Metal device;
- 35.6 MB at the drawing peak.

---

## 6. Proposal: thresholds for ADR-0002 revisit condition 3 (OQ-12)

Evaluation §15.6 condition 3 reopens the language decision if "measured clean or incremental builds on the M1 make iteration impractical, even after mitigations", and leaves the thresholds to S2.

The proposal rests on two things:
- the inner loop (edit, incremental build, test);
- the machine's memory, which the compiler shares with the editor and a browser.

The condition would trigger only if a measure stays above its "revisit" level **after** the mitigations below have been tried.

| # | Measure (8 GB M1, default job count) | Target | Revisit if, after mitigations | Today (S1 + S2) |
|---|---|---|---|---|
| T1 | Incremental dev rebuild and link after editing one leaf crate | ≤ 5 s | > 30 s | 0.13–0.82 s |
| T2 | Clean dev build of the whole workspace | ≤ 5 min | > 20 min | 8.1 s |
| T3 | Peak aggregate compiler and linker memory during a build | ≤ 3 GiB | > 5 GiB (swap pressure on 8 GB) | 0.94 GiB. For reference: 1.4 GiB with full objc2 bindings, 2.0 GiB for the 281-crate `samply` build |
| T4 | Peak memory of one `rustc` | ≤ 1.5 GiB | > 3 GiB | 0.35 GiB. For reference: 1.4 GiB for `objc2_app_kit` with every feature, 0.91 GiB for `samply`'s binary crate |

**Mitigations to exhaust first:**
- split crates along the architecture's seams;
- trim binding features (as S2 does);
- tune `opt-level`, `debuginfo` and `codegen-units` for development builds;
- run fewer jobs when memory-bound;
- try the Cranelift backend for development builds, and `sccache`;
- avoid generic-heavy public APIs at crate boundaries.

The thresholds are a proposal. If accepted at CP9, they resolve OQ-12.

---

## 7. Implications for Axiom

- **ADR-0002.** S2 raises no objection to Rust. The platform stack works without Xcode, through bindings that make most calls safe, with `unsafe` contained and documented. Its build cost is small once features are trimmed.
- **OQ-11, a pure-Rust shell or a Swift layer.** The evidence supports pure Rust through objc2 for the window, rendering and text path. **Not tested:** accessibility (AccessKit), text input and IME (`NSTextInputClient`), menus, multiple windows, and an app bundle. Each is a candidate for a later, focused spike.
- **Shader strategy (OQ-7, when to install Xcode).**
  - Without Xcode, shaders are compiled at run time. That costs about 0.5 s once per shader set per machine, then about 70 ms on later starts, device creation included.
  - Precompiled `metallib` files would need the Metal toolchain from Xcode.
  - **Hypothesis:** run-time compilation is acceptable for development and personal use. Validation: measure shader start-up cost again once the renderer has a realistic shader count.
- **The GC boundary (S1 condition C5).**
  - **Measured:** in this spike, every buffer handed to a framework was Rust-owned memory. It was either copied during the call (texture upload, vertex bytes) or lent for a scope that ends before Rust reads it (the bitmap context). Nothing managed by a GC crossed the boundary.
  - **Inference:** glyph masks, image pixels and command data can live outside the GC heap, so a pinning or non-moving space is needed only for script-visible memory handed to the GPU (array buffers for WebGL or WebGPU, later).

---

## 8. Threats to validity

1. **One cold-start observation.** The 0.5 s first compile was seen once, because the OS shader cache is shared and was not cleared (§3, finding 1).
2. **One machine and one scale factor.** Results were measured at scale 2 on one display. An external display at scale 1 was not tried.
3. **A small app.** One window and one pipeline, with no menus, IME, accessibility, app bundle or app delegate. objc2-app-kit recommends doing UI set-up from the delegate's `applicationDidFinishLaunching` [1]; the spike calls `finishLaunching` itself.
4. **Idle sampled briefly.** Each idle run was sampled over two seconds by `top` and 13 seconds by `ps`. Longer runs, and an app in the background, were not tried.
5. **Live resize stretches the last frame** (§3, finding 4). That is a limit of the spike's event loop, not of the stack.

---

## 9. Reproducing these results

```sh
cargo run -p s2-window                     # the window; close it to quit
cargo run -p s2-window -- --smoke          # draw, read back, check, exit 0
cargo test -p s2-window                    # 6 tests, no window needed
tools/s2-measure.sh --runs 5 --idle 20     # startup and idle; target/s2-results/
tools/measure-build.sh -p s2-window [--release] --runs 3 [-- --features full-bindings]
tools/measure-build.sh --runs 3 -- --all-targets   # the whole workspace
```

The runs behind this note:

| Kind | Directory |
|---|---|
| Startup and idle | `target/s2-results/20261017-074201` |
| `s2-window` builds | `target/measure/20261017-0744*` to `20261017-0748*` |
| Whole workspace | `target/measure/20261017-075010` |

---

## Sources

1. objc2-app-kit 0.3.2, crate documentation (`src/lib.rs`): AppKit set-up belongs after `applicationDidFinishLaunching`; windows created outside a window controller need `releasedWhenClosed(false)`, "which is also why the creation methods for `NSWindow` are `unsafe`". https://docs.rs/objc2-app-kit/0.3.2 (checked 2026-10-17)
2. objc2 0.6.5 and its framework crates 0.3.2: the generated bindings for Metal, QuartzCore, CoreText, CoreGraphics and CoreFoundation, read from the crate sources for every signature used. https://github.com/madsmtm/objc2 (checked 2026-10-17)
3. Apple, `kCTForegroundColorFromContextAttributeName`. https://developer.apple.com/documentation/coretext/kctforegroundcolorfromcontextattributename (behaviour also observed directly, §3)
4. This repository: the [Step 2b tracker](step-2b-validation-spikes.md) §8, `spikes/s2/window`, `tools/s2-measure.sh` and `tools/measure-build.sh`.
