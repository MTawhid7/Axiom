# S10 Results: the GPU/Compositor Process's Hop, Zero-Copy Resources and Validation

| | |
|---|---|
| **Status** | **Complete** (2026-10-21), from one formal run of three runs; every removal test passes. **S10-Q1:** the span PF13 budgets, from the GPU process's layer commit to the browser's CoreAnimation commit, took **48.8 µs at the median** and 118.7 µs at the 99th percentile over 1,740 frames, one of them over 1 ms: PF13's hypothesis of ≤ 1 ms holds about twentyfold. **S10-Q2:** the GPU process samples a renderer's pixels from Mach memory entries **without a copy,** under the renderer profile that denies IOKit, and holds nothing for them; a blit doubles every image. **S10-Q3:** validation costs **9.3–9.6 ns an item**: about 10 µs for an update, 0.4 ms for S6's whole page, and no panic on 144,876 corrupted lists. **Also:** the GPU process's profile, each rule checked by removal; B1b's hypothesis holds as measured, narrowly; the tiles are charged to the GPU process. **The owner's decisions** (2026-10-21): the warm-up approved, as D2's one exception; the ten rules adopted into the process model §4.1, with their removal tests among M2's checks; B1b's 15 MB kept, with M2 measuring the display lists' memory per item (§8) |
| **Question** | What do display-list updates cost, in latency and CPU per frame at 60 Hz, when a sandboxed process validates and rasterizes them and returns IOSurfaces to the browser's layer tree (S10-Q1)? Can that process sample a renderer's pixels from Mach memory entries without copies (a buffer made without a copy, read as a linear texture, or one blit), and how does that compare with IOSurfaces, in time and memory (S10-Q2)? What does PL9's validation cost on display lists the size of real pages (S10-Q3)? |
| **Feeds** | The [process model](../security/process-model.md) §3 (note 1), §4.1's GPU/compositor column and §7's B1b; [performance](../performance.md) PF6 and PF13; [graphics](../architecture/graphics.md) GR14 and §9's alternative G; the [rendering pipeline](../architecture/rendering-pipeline.md) PL8 and PL9; [ADR-0004](../adr/0004-process-model-and-sandbox.md) D2; [ADR-0012](../adr/0012-graphics-images-and-media.md); the architecture review's [RV1-02](../review/round-1/02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3), [RV1-05](../review/round-1/02-findings.md#rv1-05-the-content-tiers-150-mb-cap) and [RV1-09](../review/round-1/02-findings.md#rv1-09-iosurfaces-in-renderers) |
| **Plan** | The review's [actions](../review/round-1/04-round-2-actions.md#4-proposed-spikes) §4, authorized by the owner on 2026-10-21, "Authorized Immediately (Pre-Milestone 1)": "GPU process IPC hop latency, display list validation, and zero-copy texture sharing." |
| **Code** | `spikes/s10/gpu` (`s10-gpu`): 3,631 lines of Rust; 46 `unsafe` blocks, all in `src/platform/` under invariants G1–G9; three profiles in `profiles/`. `tools/s10-measure.sh`. Formal run on 2026-10-21, 19:37–19:45, built from the working tree at `5b27486` plus S10's code as committed with this note; only `gpu.sb`'s comments were corrected afterwards, and its rules are unchanged (§2) |
| **Last updated** | 2026-10-21 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from `tools/s10-measure.sh`'s formal run (§2), or from the runs by hand named where they are used.
- **Observed** facts were read on this Mac: logs, crash reports, files' sizes, SDK headers. Each names its file or command.
- **Sourced** facts link to their source; **inference** and **hypothesis** are marked.

MB means 10⁶ bytes, as the budgets use it; MiB means 2²⁰ bytes. µs figures are medians unless a percentile is named.

---

## Summary

| Question | Answer |
|---|---|
| **S10-Q1** The hop? | **Within PF13's hypothesis about twentyfold.** Over 1,740 frames at 60 Hz under the real profiles, the browser received each layer commit 32.7 µs after the GPU process sent it, and had applied it to its layer tree 48.8 µs after the send (p95 80.6, p99 118.7 µs; one frame over 1 ms, at 1.69 ms). From offer to applied, a frame took 1.60 ms at the median, 1.26 ms of it the GPU process drawing and waiting for the GPU, whose own time was 0.28 ms. **Per frame, the GPU process used 643–816 µs of CPU,** run by run, about what S6's compositor thread used for a typical update inside the browser, and **the browser 67–85 µs.** Unsandboxed runs gave the same medians (§3) |
| **S10-Q2** Zero-copy? | **Yes, under RV1-09's renderer profile.** The GPU process registered a renderer's 4 MiB image from its memory entry in 18–62 µs at the median, run by run, as a buffer made without a copy and read as a linear texture, and its footprint didn't grow: 8.9–9.0 MiB with 64 MiB of images registered, which stayed charged to the renderer. **A blit** took 0.78–1.56 ms an image and added 64.6 MiB to the GPU process: every image held twice. **The renderer's own IOSurfaces,** which RV1-09 removes, cost about what no copy costs, 37–39 µs, but need IOKit in the renderer. **Sampling linear textures costs GPU time:** the 16 images, minified into one tile, took 13–65% longer than from private copies (§4) |
| **S10-Q3** Validation? | **9.3–9.6 ns an item, about 5.1 GB/s,** in one process: 9.4–9.6 µs for 1,000 items, 413–418 µs for S6's page of 44,372 items (2.1 MB). In the GPU process, an update of 176 items (8,756 bytes, out of line) took 10.3 µs to copy and validate, and the whole page 0.64–0.76 ms. Of 144,876 single-byte changes to a 1,000-item list, 31,894 were refused and 112,982 decoded to valid lists: no panic (§5) |

**Main findings**, beyond the questions:
1. **The GPU process's profile needs ten rules, and the failures of five are silent** (§6.1): two are crashes inside Apple's frameworks, not errors (the executable directory's metadata, `hw.memsize`), and three only slow every start (Metal's shader caches). Removal tests, not error paths, keep the profile honest.
2. **Metal's shader cache needs a warm-up before the profile** (§6.2), which ADR-0004 D2's "first act" doesn't allow. Without it every start compiles the shaders: 451–458 ms to ready, against about 20 ms. **The owner approved it** as ADR-0004 D2's one exception (§8).
3. **B1b's hypothesis holds as measured, narrowly** (§7): with S6's page kept, the GPU process came to 13.4 MB without its tiles, against ≤ 15 MB. The margin, 1.6 MB, is less than the gap between S10's compact display list (2.1 MB for the page) and S6's richer one (6.3 MB), so the list's representation in memory decides B1b at M2 (inference).
4. **The tiles are charged to the GPU process,** which creates them: the browser, holding all 32 as layer contents, stayed at 3.1–3.7 MiB (§7). So from M2 B1a carries no tiles, as the split assumed.
5. **Compiling profiles in the browser matters here too** (D2): when each helper compiled its own, the sandboxed GPU process measured 2.2 MiB more than unsandboxed; with the browser compiling, the two matched, as S4 found (§7).
6. **The GPU process starts faster sandboxed:** creating the device took 12–18 ms under the profile, against 63–69 ms unsandboxed, where Metal also reaches the window server (§3.4).

## 1. What was built

One crate, `spikes/s10/gpu`, with one executable, `s10`, in three roles. The browser launches itself twice, as the GPU process and as the renderer, with S4's launcher: `posix_spawn`, an empty environment, and every descriptor closed but standard error and the socket that carries the helper's profile. No window is opened: the layer tree is built but never attached to one.

```text
            offer (60 Hz)                     update, out of line
 browser ─────────────────────▶ renderer ───────────────────────────▶ GPU process
    ▲                              │      resources: memory entries        │
    │                              └──────────────────────────────────────▶│
    │          layer commit: tile IDs, and a send right for each new        │
    └────────────────────────── IOSurface (IOSurfaceCreateMachPort) ────────┘
```

| Process | Profile | What it does |
|---|---|---|
| **Browser** | None | Checks in a rendezvous service, launches the helpers and introduces the renderer to the GPU process (a send right). Offers a frame every 16.7 ms, receives each layer commit, looks up the surfaces it hasn't seen (`IOSurfaceLookupFromMachPort`) and sets them as the contents of its 70 tile layers in one explicit CoreAnimation transaction. Reads every process's footprint at the end |
| **GPU process** | `gpu.sb` (§6) | Opens Metal and compiles two pipelines from source at run time (S6's way). Registers the renderer's resources by the run's path (S10-Q2). For each update: copies it out of the message, validates the copy (PL9, §5), draws the damaged tiles into IOSurfaces it owns, 256 × 256 pixels, two per tile used in turn, waits for the GPU, and sends the commit. Keeps the page's whole display list, sent once |
| **Renderer** | `renderer.sb`: S4's renderer profile, with RV1-09's explicit `(deny iokit-open)` | Checks that it can't create an IOSurface. Writes its pixels into memory it allocates (`mach_vm_allocate`), and shares them as Mach memory entries (`mach_make_memory_entry_64`). Sends the page's display list once, then answers each offer with an update for a 2 × 2 block of tiles that moves round a 3 × 3 path, with every page item that touches it, recoloured |

**The display list** (`display_list.rs`) stands in for IP8's generated format: a header, fixed-size property-tree nodes (transforms, clips, effects), items (rectangles, and images that sample a registered resource), the resources the update uses, and the tiles it damages. **Its decoder is PL9's validator** in safe Rust: it copies each value out of the received bytes once and checks the copy (IR7): counts within limits and the length exact; each node's parent earlier in the list, so the tree is acyclic; every number finite and within 10⁷ pixels; every item in an existing node; every image's resource registered by this renderer and listed by the update; every damaged tile real, in order, none twice. A violation is counted, as IP11's `Terminate` would end the sender.

**The page** is S6's size, 44,372 items, laid out as lines of words; one in fifty is an image from a 256-pixel atlas the renderer shares at the start. Each update carries the 176 items that touch its four tiles (§3).

**The three resource paths** (S10-Q2), each registering 16 images of 1,024 × 1,024 pixels (4 MiB each):

| Path | Renderer | GPU process |
|---|---|---|
| `no-copy` | Allocates the pixels, rows aligned to 256 bytes, and sends a memory entry | Maps the entry read-only (`mach_vm_map`), makes a shared buffer over the mapping without a copy (`newBufferWithBytesNoCopy`), and reads it as a linear texture (`newTextureWithDescriptor:offset:bytesPerRow:`) |
| `blit` | As `no-copy` | Maps the entry, copies it once into a private texture, then unmaps it |
| `iosurface` | Creates an IOSurface per image, under `renderer-iosurface.sb`, which adds IOSurface's user client to `renderer.sb`: **the path RV1-09 removes**, measured for comparison | A texture over the renderer's surface (`newTextureWithDescriptor:iosurface:plane:`) |

**Modules.** `src/platform/` holds every foreign call, under invariants G1–G9 in its `mod.rs`: Mach ports, messages with out-of-line memory and port descriptors, and memory entries; IOSurface; Metal; CoreAnimation; the clock. The rest is safe Rust: the browser, the GPU process, the renderer, the display list, the protocol and the statistics. S4's library supplies the launcher and the sandbox calls.

**Profiles are entered as [ADR-0004](../adr/0004-process-model-and-sandbox.md) D2 has them:** the browser compiles each helper's profile with that helper's parameters and sends the bytecode on an inherited socket; the helper applies it before anything else, with one exception, the GPU process's warm-up (§6.2).

**Dependencies:** none new. S4's library, and the objc2 framework crates S6 used, with `objc2-metal`'s `block2` feature for buffers made without a copy.

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B118), on AC power with Low Power Mode off (`environment.txt`); rustc 1.94.1; the Command Line Tools only.
- **The formal run:** `tools/s10-measure.sh` with its defaults: three runs, each after a 30 s cool-down, with 600 frames in each hop run. The script built the release binary and signed a copy ad hoc with the hardened runtime (`flags=0x10002(adhoc,runtime)`), which the helpers run too. It started once Spotlight's `spotlightknowledged`, which had been using 90% of a core, settled.
- **The Mac was not idle.** During the runs, macOS's media analysis (`mediaanalysisd`, 150–190% of a core) ran, with `mobileassetd` and then Spotlight's indexer (`mds`) at about 60% each (observed with `ps` at about 19:39 and 19:43). The load averages at the runs' starts were 2.91, 5.31 and 5.56 (`loads.txt`). §9 discusses the effect.
- **Each run, after its cool-down:** validation (S10-Q3); the hop under `gpu.sb` and `renderer.sb`, then unsandboxed (S10-Q1); the three resource paths (S10-Q2); a cold start of the GPU process (a new nonce) and a warm one (the same nonce again). **Then, once:** the removal tests (§6) and two reporting runs, warm and cold, under a profile that allows and logs everything.
- **The hop's timestamps** come from `mach_absolute_time`, which every process shares: the offer; the renderer's receipt and send; the GPU process's receipt, the end of validation, the GPU's completion and the commit's send; the browser's receipt, and the end of its CoreAnimation transaction. **GPU time** is the command buffer's `GPUEndTime − GPUStartTime`. The first 20 frames create the tiles' surfaces, so the statistics use frames 20–599 of each run, pooled across runs (1,740 frames), as nearest-rank percentiles.
- **CPU per frame** is each process's user and system time (`getrusage`, all its threads) from its first frame to the end, divided by the frames. The GPU process waits for each command buffer, as S6's compositor did.
- **Footprints** are each process's `ri_phys_footprint` (`proc_pid_rusage`), read by the browser at the end of each run, before the helpers exit.
- **S10-Q2** registers 16 images of 1,024 × 1,024 pixels (64 MiB in all) by each path, then draws all 16 into one 256-pixel tile once, and ten more times (GPU time, the median of the ten).
- **S10-Q3** runs in one process. Each size is decoded for 100 ms first, so the core reaches its working frequency, then 200 times. Then each byte of a 1,000-item list (48,292 bytes) is changed three ways (XOR `0x01`, `0x80` and `0xFF`), and each changed list decoded.

## 3. S10-Q1: the hop

### 3.1 Latency

Every run's steady frames, 1,740 per configuration (`summary.txt`, and the frames' CSVs for PF13's span):

| Stage | Under the profiles: median µs | p95 | p99 | Max | Unsandboxed: median µs | p99 |
|---|---|---|---|---|---|---|
| Offer → renderer | 41.3 | 76.2 | 132.5 | 2,943.8 | 41.4 | 237.9 |
| Renderer: build and encode the update | 133.0 | 245.5 | 277.7 | 1,905.0 | 135.2 | 422.1 |
| Renderer → GPU process (8,756 bytes, out of line) | 63.2 | 112.5 | 141.8 | 680.4 | 60.3 | 184.5 |
| GPU process: copy and validate | 10.3 | 16.8 | 22.5 | 360.9 | 10.1 | 24.8 |
| GPU process: draw 4 tiles, and wait for the GPU | 1,260.5 | 1,765.5 | 5,835.5 | 15,005.6 | 1,209.9 | 4,659.1 |
| GPU process: build the commit | 4.1 | 11.1 | 14.1 | 61.3 | 4.2 | 13.7 |
| **GPU process → browser: the hop RV1-02 adds** | **32.7** | 53.9 | 95.5 | 1,666.5 | 32.5 | 127.6 |
| Browser: surfaces, layer contents, commit | 18.0 | 26.6 | 36.0 | 90.8 | 18.3 | 45.9 |
| **PF13's span: commit sent → browser's commit done** | **48.8** | 80.6 | 118.7 | 1,688.0 | 49.1 | 150.3 |
| End to end: offer → applied | 1,599.8 | 2,281.2 | 6,244.5 | 15,546.2 | 1,542.0 | 5,161.4 |
| GPU time (the command buffer) | 275.0 | 538.0 | 4,745.5 | 6,179.6 | 259.4 | 3,241.9 |

PF13's span, run by run under the profiles: 46.0, 55.5 and 49.0 µs at the median; 118.3, 130.2 and 131.3 µs at the 99th percentile.

- **PF13's hypothesis holds,** about twentyfold at the median: one frame of 1,740 took more than 1 ms, under the profiles and unsandboxed alike.
- **What the hop costs is waking the receiver** (inference). S4 measured a 64-byte Mach round trip at 3.75 µs in a busy loop; here each receiver had waited most of a frame, and every one-way message, the offer, the update and the commit alike, took 30–65 µs at the median.
- **The browser's part is small:** 18 µs at the median to look up new surfaces and commit the layers' contents.
- **The rest of the frame is the compositor's work, moved out of the browser.** Drawing and waiting took 1.26 ms at the median, of which the GPU's own time was 0.28 ms. S6's compositor did the same kind of work on its thread in the browser (S6 results §3).
- **The sandbox costs nothing measurable per frame:** the medians under the profiles and unsandboxed agree within 5%, as S4 found for IPC.
- **The tails come mostly from the GPU.** Of 1,740 frames under the profiles, 28 took more than 4 ms from offer to applied, and 22 of those spent most of it in GPU work; none exceeded a 16.7 ms frame. The slowest took 15.6 ms, 15.0 ms of it drawing, with 6.2 ms of GPU time. Unsandboxed, one frame took 28.2 ms, because the renderer received its offer 26.8 ms late. The GPU time's 99th percentile was 5.69, 0.54 and 2.90 ms in runs 1 to 3, while media analysis ran (§2, §9).

### 3.2 CPU per frame

| Run | Browser µs | GPU process µs | Renderer µs |
|---|---|---|---|
| 1, under the profiles | 70.4 | 679.9 | 170.4 |
| 2 | 84.9 | 808.0 | 188.6 |
| 3 | 74.0 | 750.8 | 186.2 |
| 1–3, unsandboxed | 75.9, 82.0, 67.1 | 763.1, 815.7, 643.2 | 190.8, 189.6, 159.8 |

- **The GPU process's 0.64–0.82 ms a frame** is the compositor's work, moved: S6's compositor thread used 0.58–0.85 ms of CPU for a typical update in its later runs, inside the browser (S6 results, summary). The spikes draw different content, S10's with one draw call per image item, so the comparison is of magnitude only (inference).
- **The browser keeps 67–85 µs a frame:** offering, receiving the commit, and the CoreAnimation transaction. From M2 its threads no longer validate or draw.
- **The renderer's 160–191 µs** is mostly the spike's update generator, which scans the 44,372-item page every frame; a renderer wouldn't (inference).

### 3.3 The page across processes

The renderer sent S6's whole page once: 2,130,148 bytes out of line, which the GPU process copied, validated and kept in 683–690 µs under the profiles (641–757 µs unsandboxed). In one process, from a buffer already in cache, the same list took 413–418 µs (§5); the difference is the first touch of freshly mapped pages (inference).

### 3.4 Start-up

| GPU process starts | Ready | Device | Shader library | Pipelines |
|---|---|---|---|---|
| Warm, under `gpu.sb` (the three start-up runs) | 19.7–20.0 ms | 18.0–18.4 ms | 0.7–0.8 ms | 0.8 ms |
| In the hop runs, under `gpu.sb` | 14.5–17.1 ms | 12.3–15.1 ms | 0.8–1.1 ms | 0.8–1.1 ms |
| In the hop runs, unsandboxed | 64.3–70.8 ms | 62.6–69.3 ms | 0.8–1.0 ms | 0.6–0.7 ms |
| **Cold** (a new nonce), under `gpu.sb` | 192–302 ms | 12.3–14.1 ms | 178–289 ms | 0.8 ms |

- **A cold compile is rare** (the first start after a macOS update, or after Metal evicts the entry). On a quieter Mac, by hand, it took 101–107 ms under the profile and 96 ms unsandboxed; it needs the compiler service (§6.1).
- **Creating the device is 4–5 times faster under the profile.** Unsandboxed, Metal also reaches the window server, which the profile denies (`com.apple.windowserver.active`, 10 lookups denied a start); inference: it skips work for displays it can't reach. The GPU process never needs the window server: the browser presents.

### 3.5 Graphics' alternative G

Graphics §9's alternative G, two streams as WebKit splits them, was to be revisited "if S10 shows the second hop costs a frame". It costs about 50 µs, so G stays rejected, and GR14's one ordered stream through the GPU process stands.

## 4. S10-Q2: zero-copy resources

| Path | Registration per 4 MiB image, median (p95), runs 1–3 | GPU time to draw the 16 images into a tile (median of 10) | GPU process | Renderer |
|---|---|---|---|---|
| **`no-copy`** | 20.9 (82.7), 18.2 (61.8), 61.6 (204.4) µs | 0.709, 0.824, 0.874 ms | **8.9–9.0 MiB** | 65.7 MiB |
| `blit` | 1,558 (12,953), 779 (4,114), 943 (6,313) µs | 0.534, 0.500, 0.774 ms | **73.5 MiB** | 65.7 MiB |
| `iosurface` (the old design) | 37.2 (122.6), 38.8 (134.4), 38.0 (138.5) µs | 0.721, 0.767, 0.882 ms | 8.9 MiB | 66.3 MiB |

- **Zero-copy works under RV1-09's profile.** The renderer can't open IOSurface's user client (its profile's denials show the check, 9 in the formal runs), and memory entries are enough: the GPU process maps each read-only and samples it in place.
- **Memory:** sampling without a copy adds nothing to the GPU process. The 64 MiB stay charged to the renderer, which allocated them. A blit adds 64.6 MiB to the GPU process: the image held twice, as RV1-05 warned. The renderer's IOSurfaces are charged to it too.
- **So PL8's charging hypothesis holds for renderer resources** ("charged to it where macOS allows"): memory a renderer allocates stays on its ledger when another process maps it, with no API to move the charge (observed). The runtime note's `IOSurfaceSetOwnershipIdentity` isn't needed for them.
- **Registration:** mapping the entry, making the buffer and the texture took 18–62 µs at the median; IOSurface's lookup and texture 37–39 µs; a blit 0.78–1.56 ms, waiting for the copy, with a 95th percentile of 4.1–13.0 ms.
- **Sampling costs GPU time.** Drawing all 16 images into one tile took 13–65% more GPU time from linear textures than from private copies, run by run (0.71–0.87 ms against 0.50–0.77 ms). Each 1,024-pixel image is drawn into 256 pixels, minified fourfold without mipmaps, where a private texture's layout keeps neighbouring pixels close (inference). The IOSurfaces, linear too, cost the same as no copy.
- **Inference:** zero-copy stays the right default, as the owner made it a rule (RV1-05). The sampling cost matters only for images drawn much smaller than they were decoded, and is the graphics milestone's to weigh against the memory a copy costs.

## 5. S10-Q3: validation

In one process (`q3-run*/validate.md`):

| Items | Bytes | Median µs, runs 1–3 | ns an item | GB/s |
|---|---|---|---|---|
| 1,000 | 48,572 | 9.6, 9.4, 9.4 | 9.4–9.6 | 5.07–5.16 |
| 10,000 | 480,572 | 92.8, 92.7, 94.4 | 9.3–9.4 | 5.09–5.18 |
| 44,372 (S6's page) | 2,130,428 | 412.7, 417.8, 417.8 | 9.3–9.4 | 5.10–5.16 |

- **Linear in the list's size:** about 9.4 ns an item of 48 bytes, copying every value once and checking the copy (IR7).
- **At display rate it is negligible:** an update of 176 items took 10.3 µs at the median in the GPU process (p99 22.5 µs), about 0.06% of a frame; a whole page 0.4 ms in one process, 0.68 ms across processes (§3.3).
- **Corrupted lists are refused or decoded, never a panic:** every single-byte change of a 1,000-item list, 144,876 lists, gave 31,894 refusals and 112,982 valid lists (a changed colour or coordinate is still a valid list), in each run. The crate's tests cover truncation, unregistered resources and cycles in the property tree.
- **The decoder is hand-written** to PL9's rules; Axiom's will be generated from the IDL (IP8). The spike shows the checks' cost, not the generator's (inference).

## 6. The GPU process's profile

`spikes/s10/gpu/profiles/gpu.sb` was grown by S4's and S7's method: deny by default; run once under a profile that allows everything and reports it; allow only what a failure shows is needed, with its reason in the profile; then take each rule out in turn and check the failure it causes. Every rule below is checked by removal in the formal run (`q1-removal.txt`, all PASS).

### 6.1 The rules

| Rule | Why | Without it (removal test) |
|---|---|---|
| `mach-lookup` of the browser's rendezvous (`RENDEZVOUS`) | Every helper's, as S4's | "look_up: Mach error 0x44c" (`BOOTSTRAP_NOT_PRIVILEGED`) |
| `file-read-metadata` of the executable's directory (`EXECUTABLE_DIR`) | IOSurface's connection reads the main bundle's URL while Metal creates the device | **A crash, not an error:** `SIGSEGV` in `__CFCheckCFInfoPACSignature`, under `CFBundleCopyBundleURL` ← `_ioSurfaceConnectInternal` ← `IOSurfaceClientCopyGPUPolicies` ← `MTLCreateSystemDefaultDevice` (crash report). S5 and S7 found the same rule for ImageIO, CoreText and Security |
| `iokit-open-user-client`: `AGXDeviceUserClient` | The GPU itself (the M1's G13; the class is the driver's) | "no Metal device" |
| `iokit-open-user-client`: `IOSurfaceRootUserClient` | The tiles' surfaces | "IOSurfaceCreate failed" at the first tile |
| `file-read*` of `/System/Library/Extensions/AGXMetal*.bundle` | The GPU's Metal driver, which Metal loads (`AGXMetal13_3` here) | "no Metal device" |
| `file-read-data` of `AGXCompilerCore.framework/agxa_compiler_hash` | The shader compiler's identity, which Metal checks against its pipeline cache | **No error, a slower start:** the pipelines are compiled at every start, 49.5 ms instead of 0.8 ms |
| `mach-lookup` of the XPC service `com.apple.MTLCompilerService` | Compiling shaders from source | A cold compile fails: "Unable to reach MTLCompilerService … Connection init failed at lookup with error 159 - Sandbox restriction". A warm start doesn't look it up |
| `file-read*` and `file-write*` under `METAL_CACHE`, the user's cache directory's `com.apple.metal` | Metal's shader caches | **No error, a slower start:** every start compiles, the library in 99.6 ms and the pipelines in 19.1 ms |
| `file-read-metadata` of `/var` and of the user's cache directory (`USER_CACHE_DIR`), as two literals | Metal reaches its caches by their `/var/folders/…` path, through the `/var` symbolic link | **No error, a slower start,** for each literal: every start compiles, 429.4 ms for the library without `/var` and 415.1 ms without the directory |
| `sysctl-read` of `hw.memsize` | The GPU's driver reads it while creating the device | **A crash:** `abort()` in `-[AGXG13GFamilyDevice initWithAcceleratorPort:simultaneousInstances:]` (crash report) |
| `(deny dynamic-code-generation)` | J3, as in every profile | Not a grant, so not removed; S4-Q4 tested it |

**Five rules fail silently:** two as crashes inside Apple's frameworks rather than errors (the executable directory's metadata, `hw.memsize`), three as slower starts (the compiler's identity, the caches, and the caches' path). So removal tests, not error paths, are what keep this profile honest, as S5 and S7 found for theirs (inference). Two sysctls the first profile allowed, `kern.bootargs` and `kern.osproductversion`, were dropped: denying them changed nothing measured, warm or cold (by hand, 2026-10-21).

### 6.2 The warm-up before the profile

**Metal finds its caches through `confstr(_CS_DARWIN_USER_CACHE_DIR)`.** Resolving that directory needs the user's UUID from opendirectoryd (`com.apple.system.opendirectoryd.membership`), and dirhelper (`com.apple.bsd.dirhelper`) when it seems missing: two services the profile denies (observed: 28 and 42 denials in one start, in the unified log). libSystem keeps the answer for the process once it has it (inference, from the behaviour below).

**So the GPU process resolves the directory before it applies its profile:** one libSystem call that reads nothing from another process (`gpu_process.rs`). Without it, every start compiled the shaders again: 451–458 ms to ready, against 19–23 ms with it (by hand, three starts each, 2026-10-21).

**This departs from [ADR-0004](../adr/0004-process-model-and-sandbox.md) D2,** under which a helper applies its profile "as its first act". Chromium's GPU process is the one process type its V2 sandbox doesn't cover, because it warms up unsandboxed ([confinement note](macos-process-confinement.md) §3, source 1); S10's warm-up is a single call. **The owner approved it** as D2's one exception (2026-10-21; §8). The alternatives were worse:
- allowing the two services would let the process that handles every renderer's display lists reach opendirectoryd, and a root-owned service that creates directories;
- going without the shader cache costs about 0.44 s at every start of the GPU process, each restart after a crash included.

### 6.3 What stays denied

Under the real profiles the formal runs logged these denials, and every run worked (`summary.txt`; inference: none is needed):
- **The GPU process:** the window server (`com.apple.windowserver.active`, which Metal looks up while creating the device: the GPU process never talks to it); DiskArbitration, the volumes' metadata and `system-info vfs.disk-space` (inference: a check of free space before writing the cache, which is written anyway, since each cold start's next start is warm); `tccd`, `analyticsd`, `coreservicesd` and the syslog socket; the listing and contents of the executable's directory; `.AppleSetupDone`, the user's and global preferences, `SystemVersion.plist`, ICU's data; and the sysctls `kern.bootargs`, `kern.osproductversion` and the performance levels' CPU counts.
- **The renderer:** the IOSurface user client RV1-09 denies (its check that it can't create a surface); DiskArbitration and the volumes, as above; `coreservicesd`.

The reporting runs list what the GPU process attempts with everything allowed: 81 kinds of operation on a warm start and 89 on a cold one (`q1-attempted-warm.txt`, `q1-attempted-cold.txt`). A cold start adds the compiler service, and sandbox extensions it would issue to that service for Metal's front-end and archive caches. Under the profile those caches' metadata is denied and the cold compile still works: by hand it took 101–107 ms, against 96 ms unsandboxed.

### 6.4 A correction to the informal trials

The first trials by hand reported a "cold compile" of 948 ms under an early profile. **It wasn't one.** S4's launcher gives helpers an empty environment, so the variable that forces a cold compile (`S10_SHADER_NONCE`) never reached the GPU process. Those starts were slow because that profile denied the cache's paths (§6.1). The browser now passes the variable on explicitly. A true cold compile takes 178–289 ms for the library in the formal run, and 101–107 ms by hand on a quieter Mac (§3.4), and its failure without the compiler service is the removal test above.

## 7. Memory: B1b, and who pays for the tiles

Footprints at the end of each hop run, in MiB:

| Run | Under the profiles: browser, GPU process, renderer | Unsandboxed |
|---|---|---|
| 1 | 3.7, 20.8, 6.3 | 3.2, 20.9, 6.8 |
| 2 | 3.6, 20.8, 6.3 | 3.1, 20.9, 6.9 |
| 3 | 3.5, 20.8, 6.4 | 3.1, 20.9, 6.8 |

- **The GPU process: 20.8 MiB,** of which the tiles are 8.0 MiB: 32 surfaces of 256 × 256 × 4 bytes, two for each of the 16 tiles the updates touch. That leaves **12.8 MiB (13.4 MB)** for the process, Metal's device and pipelines, the kept page (2.1 MB of items) and each frame's buffers. In the resources runs, with one tile and no page, the GPU process measured 8.9–9.0 MiB: a process with Metal's device and pipelines costs about 8.7 MiB before any content (inference).
- **B1b's hypothesis holds as measured:** "≤ 15 MB for its device, pipelines and display lists" (process model §7), against 13.4 MB with S6's page kept. **Its margin is thin.** S6 held the same page's display lists in 6.3 MB, where S10's items take 2.1 MB; with S6's representation the GPU process would come to about 17.6 MB (inference). So the representation in memory decides B1b, and M2's measurement settles it.
- **Tiles are charged to the process that creates them.** The browser held all 32 surfaces as layer contents and stayed at 3.1–3.7 MiB; the GPU process carried their 8.0 MiB. A renderer's memory entries and IOSurfaces were likewise charged to the renderer (§4). So from M2 B1a carries no tiles, as the split assumed, and B1b carries them at 6.2 bytes per window pixel. Attached to a window, the window server maps the surfaces too; whose ledger shows them then is unmeasured (§9).
- **Compile profiles in the browser** (D2). In the quick pass, where each helper compiled its own profile, the sandboxed GPU process measured 22.9 MiB against 20.7 MiB unsandboxed. With the browser compiling, it was 20.8 against 20.9 MiB. The browser pays for libsandbox's compiler once instead (S4-Q3: 2.8 MiB).
- **The browser here has no window and no AppKit,** so its 3.1–4.5 MiB says nothing about B1a.

## 8. Implications for Axiom

1. **PF13's hop is measured:** 48.8 µs at the median, against ≤ 1 ms. The budget stands as written, and its evidence becomes S10's.
2. **GR14's one ordered stream stands,** and graphics §9's alternative G stays rejected: the second hop costs about 50 µs, not a frame (§3.5).
3. **B1b holds as measured, narrowly** (§7). Its 15 MB is met with S10's compact items (48 bytes each). **The owner kept it** (2026-10-21): M2 measures the display lists' memory per item, and scales the budget per window if complex pages exceed the margin.
4. **Zero-copy works as designed** (PL8, RV1-05, RV1-09): memory entries, buffers made without a copy and linear textures, for a renderer that can't open IOKit. PL8's charging hypothesis holds for renderer resources. Minified images sample more slowly from linear textures, which is a cost for the graphics milestone (§4).
5. **Validation costs 9.4 ns an item,** about 10 µs a frame at display rate: IR7's copy-then-check is affordable (§5).
6. **The GPU/compositor process's rules, for the process model's §4.1,** which said "rules from S10" in that column. **Confirmed by the owner** (2026-10-21), with the removal tests among Milestone 2's checks (TS14's V1); the process model's change record has the owner's words. From §6.1:
   - **Own data directory:** Metal's shader caches only, `com.apple.metal` under the user's cache directory, read and write.
   - **GPU (Metal, IOKit):** the GPU's user client and IOSurface's; read access to the GPU's driver bundle and the shader compiler's identity; the sysctl `hw.memsize`.
   - **Metadata of paths outside its own files:** the executable's directory, `/var` and the user's cache directory.
   - **Mach service lookups:** the browser's rendezvous, and Metal's compiler service (an XPC service); nothing else, logging included.
7. **A warm-up before the GPU process's profile** (§6.2). D2 has a helper apply its profile as its first act. The GPU process may first resolve the user's cache directory, one fixed libSystem call that reads nothing from another process. **Approved by the owner** (2026-10-21) as D2's one exception, recorded in ADR-0004 and the process model §10: the alternatives were a wider profile, reaching opendirectoryd and dirhelper, or a 0.44 s start every time the GPU process starts.
8. **Metal works in an ad hoc signed helper with the hardened runtime** (observed: the formal run's binary).

## 9. Threats to validity

1. **A busy Mac.** Media analysis and Spotlight's indexer ran throughout the formal run (§2). The medians agree across the three runs, and with the quick pass at a load of 2.0. The tails (the 99th percentiles, the maxima and the GPU-time outliers) are this session's. Media analysis uses the GPU (inference), and runs 1 and 3, with GPU-time 99th percentiles of 5.69 and 2.90 ms against run 2's 0.54 ms, show contention. A run on an idle Mac would tighten them.
2. **No window.** The layer tree is never attached to one, so the window server's composition, the same in both topologies and measured by S6, isn't in the figures, and CoreAnimation never holds a surface the GPU process wants to reuse. Axiom's GPU process would check `IOSurfaceIsInUse` before drawing into a tile's spare, a use count kept across processes ([runtime note](macos-runtime-facilities.md) §6.1); S10 doesn't exercise it.
3. **Timer pacing,** not a display link, as in S6.
4. **Synthetic content and a simple rasterizer:** rectangles and atlas images; no glyphs, clips or effects; one draw call per image item. S6's richer content took 6.3 MB of display lists where S10's takes 2.1 MB (§7).
5. **One renderer, one window, one GPU process:** no contention among renderers for the GPU process.
6. **Footprints at one moment,** at each run's end.
7. **One machine and one OS build:** macOS 27.2 (26B118), with an M1's G13 GPU. The driver's class and bundle names change with GPU generations, and what Metal needs may change with macOS updates; the removal tests check them again.
8. **Metal's cache carries state between runs.** The removal tests that depend on it warm up under the real profile first, and judge the variant's second start.

## 10. Reproducing these results

```sh
cargo test -p s10-gpu                  # display-list round trip and refusals, statistics
target/release/s10 hop                 # S10-Q1 by hand; --gpu-profile none|report|file:PATH
target/release/s10 resources --path no-copy|blit|iosurface
target/release/s10 validate            # S10-Q3, in one process
tools/s10-measure.sh                   # everything: target/s10-results/<timestamp>/
                                       # formal runs: AC power, Low Power Mode off
```

| Run | Directory |
|---|---|
| **Formal run,** 2026-10-21 19:37–19:45, S10's code as committed | `target/s10-results/20261021-193709/`: `summary.txt`, `environment.txt`, `loads.txt`; per run `q1-hop-run*/` and `q1-hop-none-run*/` (each with `hop-summary.md` and `hop-frames.csv`), `q2-run*/`, `q3-run*/` and `startup-*-run*.txt`; `q1-removal*.txt` with `profiles/rm-*.sb`; `q1-attempted-warm.txt` and `q1-attempted-cold.txt` (81 and 89 kinds of operation); `sandbox-denials.txt`; `crash-reports/` |
| Quick pass, 19:27, before the profile method and validation's warm-up changed | `target/s10-results/20261021-192721/` |
| Discovery by hand, earlier that day | `target/s10-discovery/`: the first reporting runs, and the trials' crash reports |

## 11. Side effects

- **Crash reports:** the formal run's two, from the removal tests of the executable directory and `hw.memsize`, were moved by the script into its results, as were the quick pass's two. The five that trials by hand left were moved into `target/s10-discovery/crash-reports/`. `~/Library/Logs/DiagnosticReports` holds none from S10 (observed).
- **Metal's per-user shader cache** (`/var/folders/…/C/com.apple.metal`, which command-line tools using Metal share) holds S10's compiled shaders and its cold-start variants. It measured 3,060 KiB at the start of each scripted run, 3,092 KiB after the quick pass and 3,172 KiB after the formal run (observed, `du -sk`). It shrank between the two runs, so Metal manages its size (inference); it was left in place.
- **Nothing else:** no keychain, preference, privacy setting or launchd job was touched. Builds and results stay under `target/`, which git ignores.
