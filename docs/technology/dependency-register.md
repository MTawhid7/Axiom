# Dependency Register

| | |
|---|---|
| **Status** | Living document. Started at Step 5, CP2 |
| **Purpose** | Every dependency proposed for the engine, with the record the [dependency policy](dependency-policy.md) requires (DP6). Nothing listed here is added to the workspace until its approval (DP5) and, for code, an authorized milestone |
| **Last updated** | 2026-10-18 |

**How to read it.** Each entry gives the fields of DP6. The values used are:
- **class:** required platform facility, replaceable implementation detail, or foundational dependency (DP5);
- **group:** `trusted`, `sandboxed`, `build` or `test` (DP9);
- **grade:** `ub-risk-0` to `ub-risk-2` (DP8), or "not graded" until an audit is recorded.

Crate facts are dated and come from crates.io's API and GitHub, read on the date shown. Every **status** is one of: *decided* (by an accepted ADR), *proposed* (by a Step 5 document, awaiting its ADR), or *spike only* (exempt, DP23).

## 1. Required platform facilities

Placement and purpose are in the policy's table ([DP21](dependency-policy.md#8-apples-frameworks)). Those decided by accepted ADRs:

| Facility | Processes | Status |
|---|---|---|
| libSystem: Mach messages, kqueue, `posix_spawn`, `os_sync_wait_on_address` | All | Decided: ADRs [0005](../adr/0005-ipc-transport-and-interfaces.md), [0006](../adr/0006-event-loop-and-threading.md), [0008](../adr/0008-script-agents-and-heap.md) |
| libsandbox's private interfaces | Browser (compile), helpers (apply) | Decided: [ADR-0004](../adr/0004-process-model-and-sandbox.md) |
| AppKit, Foundation, QuartzCore, Metal, IOSurface | Browser | Decided: ADRs [0007](../adr/0007-frame-tree-pipeline-and-input.md), [0009](../adr/0009-embedder-api-and-shell.md) |
| AppKit's `NSAccessibility` protocols and HIServices' text markers | Browser (main thread) | Decided: [ADR-0014](../adr/0014-accessibility.md), accepted at Step 5's Gate B (2026-10-18) |

The Step 5 checkpoints add proposed facilities (CoreText, ImageIO, Security, Network, media frameworks) as they decide them.

## 2. Crates

### objc2 and its framework crates

| Field | Value |
|---|---|
| **Crates** | `objc2` 0.6.5; the framework crates `objc2-foundation`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-metal`, `objc2-core-text`, `objc2-core-graphics`, `objc2-core-foundation` and `objc2-io-surface`, 0.3.2; transitively `block2` 0.6.2, `dispatch2` 0.3.1, `objc2-encode` 4.1.0 and `bitflags` 2.13.2 (the versions in `Cargo.lock`) |
| **Source and license** | crates.io; `objc2`, `block2` and `objc2-encode` MIT; the framework crates and `dispatch2` "Zlib OR Apache-2.0 OR MIT"; `bitflags` "MIT OR Apache-2.0" (crates.io, 2026-10-18) |
| **Purpose** | The Objective-C runtime, and generated bindings to Apple's frameworks |
| **Used by** | `platform`, and the shell's `platform` module ([overview](../architecture/overview.md) §2) |
| **Processes** | Browser (AppKit, Metal); renderers, only for frameworks a Step 5 checkpoint places there (CoreText, if CP3 chooses it) |
| **Untrusted input** | None of its own: it marshals calls. The frameworks behind it are classed separately (§1) |
| **Class** | **Foundational dependency:** its types appear throughout `platform` and the shell |
| **Group, grade** | `trusted` is required, because the browser uses it. Not graded yet: audits begin when `cargo-vet` is installed (DP22) |
| **`unsafe` and non-Rust code** | Bindings to Objective-C are FFI, so the crates contain `unsafe`. Their generated methods make most calls safe: S2's window needed 19 `unsafe` sites, all in its `platform` module ([S2 results](../research/s2-results.md)). Their class-definition macros expand `unsafe`, so they are used only in allowlisted modules (DP11) |
| **Alternatives** | Hand-written FFI, which S4 used for the C interfaces of libsandbox and Mach; a Swift shell, the fallback EM11 keeps behind the same message API |
| **Maintenance** | One repository, `madsmtm/objc2`, last pushed 2026-10-17. Its contributors are led by one principal maintainer (3,237 commits; the next is the original `objc` crate's author, with 1,210) (GitHub, 2026-10-18). **Risk:** a small maintainer base. Precedent: wgpu moved its Metal backend to `objc2-metal` in January 2026 ([language evaluation](language-evaluation.md) §6.1, §9) |
| **Build cost** | Measured by S2 (S2-Q6): `s2-window` built clean in 6.0 s with only the headers it used, and in 29 s with the defaults, which also needed 1.4 GiB for `objc2_app_kit` alone. Both are within T1–T4's targets |
| **Features** | `default-features = false`, enabling per framework only the headers used, as the workspace manifest already does (DP18) |
| **Interface** | The `platform` modules, which expose Axiom's own types (ADR-0002) |
| **Revisit if** | The crates become unmaintained, or the shell milestone's first validation of objc2-defined classes for text input and accessibility fails (EM11) |
| **Status** | **Decided:** [ADR-0009](../adr/0009-embedder-api-and-shell.md) (EM11, the pure-Rust shell with objc2), on the evidence of [S2](../research/s2-results.md) and the language evaluation (§9, §13) |

## 3. Proposed by Step 5

CP3–CP8 add entries here as their documents propose dependencies, and CP9's ADRs decide them. **At Gate B** (2026-10-18) the owner accepted ADR-0010, which makes this register's format binding, and ADR-0014, whose entry (CP8) is now decided and listed in §1. ADRs 0011–0013 are pre-approved pending spikes S5–S7, so their entries stay proposed until those ADRs are accepted. Published grades are Google's or Chromium's `cargo-vet` audits ([text and fonts](../architecture/text-and-fonts.md) §4, source A1). Each applies to the version audited, so every version adopted is audited again (DP15). Build costs are measured when a crate is first added (DP19).

### From CP3: text, fonts and Unicode

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **CoreText and CoreGraphics** | Required platform facility | — | — (C) | Renderer only | The system font database and fallback cascades; reading tables of system fonts; rasterizing system fonts if S5 chooses it. Never given a web font | TX6, TX7, TX12 |
| **ICU4X 2.3:** `icu_segmenter`, `icu_properties`, `icu_normalizer`, `icu_casemap`, with `icu_provider`, `icu_collections`, `icu_locale_core`, `zerovec`, `zerotrie`, `yoke`, `potential_utf`, `utf8_iter`, `displaydoc` and `smallvec` | Foundational | `trusted` in the renderer. Browser use for less-trusted data waits for CP6 (IDNA) | Segmenter `ub-risk-1`; properties `ub-risk-0`; normalizer and provider `ub-risk-1`; `zerovec`, `zerotrie`, `yoke`, `icu_collections` `ub-risk-2` (2.0 betas; `zerovec` 0.11.1) | Renderer; others as CP6 and CP16 decide | Unicode data and algorithms for the whole engine, compiled data; behind Axiom's `unicode` interface. License Unicode-3.0. Precedent: Firefox's segmenters; vendored by Chromium and Servo | TX1, TX2, TX16 |
| **`unicode-bidi`** 0.3.18 | Replaceable implementation detail | `trusted` | `ub-risk-1` | Renderer | UAX #9, over ICU4X's character data. "MIT OR Apache-2.0". Servo's | TX17 |
| **HarfRust** 0.14, with `read-fonts`, `font-types`, `bytemuck`, `bitflags` and `smallvec` | Replaceable implementation detail | `trusted` in the renderer | Not graded itself. `read-fonts` `ub-risk-0` (0.27.3); `font-types` `ub-risk-1`; `bytemuck` `ub-risk-2` (1.22.0) | Renderer | Shaping, behind Axiom's shaping interface. MIT. Vendored by Chromium, at status `test` | TX9, TX11 |
| **`read-fonts`** and **`skrifa`** (Fontations) | Replaceable implementation detail | `trusted` in the renderer | `read-fonts` `ub-risk-0` (0.27.3); `skrifa` `ub-risk-0` (0.29.0); `bytemuck` `ub-risk-2` | Renderer | Parsing every font; outlines for the Rust rasterizer. "MIT OR Apache-2.0". Chrome's web-font path on Linux, Android and ChromeOS | TX8, TX12 |
| **`brotli-decompressor`** 6.1.0 | Replaceable implementation detail | `trusted` in the renderer, if its audit agrees | Not graded. It forbids `unsafe` unless its `unsafe` or `ffi-api` feature is on, and its default features enable neither (observed in its `lib.rs` and `Cargo.toml`) | Renderer | Brotli, for WOFF2. "BSD-3-Clause/MIT". Its allocator crates need an audit too | TX7 |
| **A Rust rasterizer:** a candidate such as `zeno`, `vello_cpu` or `tiny-skia` | Replaceable implementation detail | `trusted` in the renderer | Not looked up | Renderer | Filling `skrifa`'s outlines; chosen in spike S5 | TX12 |

**Open risk: `smallvec`,** which HarfRust and ICU4X's segmenter both require. Google graded 1.11.0 at `ub-risk-3`, and later versions carry only `safe-to-deploy` audits. Axiom grades the version it adopts; while it is graded `ub-risk-3`, DP8 bars it ([text and fonts](../architecture/text-and-fonts.md) §4).

### From CP4: graphics

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **Metal, QuartzCore (CoreAnimation) and IOSurface** | Required platform facility (already decided, §1) | — | — | Browser; a GPU process later | Drawing tiles, presenting through layer trees, sharing surfaces. Through `platform`, over `objc2-metal` | GR1, GR2 |
| **A path rasterizer for the renderer:** a candidate such as `vello_cpu` 0.3.0 or `tiny-skia` 0.12.0, or the same crate TX12 adopts | Replaceable implementation detail | `trusted` in the renderer | Not looked up | Renderer | Filling SVG shapes, clip paths and complex borders into image resources; chosen in S6 | GR5 |

**The compositor needs no new crate:** its pipelines are Axiom's own (GR3). wgpu was rejected for the browser's compositor by design (GR1), not by grade. It remains a candidate for WebGPU in the GPU process (GR10).

### From CP5: images and media

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **ImageIO** | Required platform facility | — | — (C) | Renderer only | AVIF, under DP10's exception, accepted by ADR-0012; pending S5-Q5 | IM3 |
| **VideoToolbox, AudioToolbox, CoreAudio** | Required platform facility | — | — (C) | Media utility process only | Hardware and Apple codecs; audio output | IM11 |
| **`png`** 0.18.1, with `fdeflate`, `miniz_oxide`, `crc32fast`, `simd-adler32`, `adler2` | Replaceable implementation detail | `trusted` in the renderer | `png` `ub-risk-0` (0.18.0-rc); `crc32fast` and `simd-adler32` `ub-risk-2` | Renderer | PNG and APNG, behind the decoder interface. Chromium's PNG decoder | IM2 |
| **`jpeg-decoder`** 0.3.2, `platform_independent` | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` in this mode | Renderer | JPEG, unless S5-Q5 favours a decoder of our own. In maintenance mode | IM2 |
| **`gif`** 0.14.2 | Replaceable implementation detail | `trusted` in the renderer | `ub-risk-1` (0.12.1) | Renderer | GIF | IM2 |
| **`image-webp`** 0.2.4 | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` | Renderer | WebP | IM2 |
| **`jxl`** 0.7.4 | Replaceable implementation detail | `trusted` in the renderer | `ub-risk-1` (0.1.1) | Renderer | JPEG XL, staged. Chromium's and Firefox's decoder | IM2 |
| **`moxcms`** 0.9.1, with no SIMD feature | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` without SIMD features | Renderer | ICC and CICP color management | IM8 |

`zune-jpeg` is listed under §4 below. Media's demuxers and software codecs are chosen at the media milestone (IM11).

### From CP6: networking

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **Security: trust evaluation** | Required platform facility | — | — (C) | Network | Certificate verification against the system's trust store (SG7). Through `platform` | NW6 |
| **DNS-SD** (`dns_sd.h`, libSystem) | Required platform facility | — | — (C) | Network | Asynchronous name resolution, and HTTPS records | NW7 |
| **SystemConfiguration** | Required platform facility | — | — (C) | Network | The system's proxy settings | NW9 |
| **rustls** 0.23.46 | Foundational: TLS shapes the network process | `sandboxed`, with its crypto provider | Not graded; `forbid(unsafe_code)` | Network | TLS 1.2 and 1.3, as a state machine (NW1). "Apache-2.0 OR ISC OR MIT" | NW6 |
| **`aws-lc-rs`** 1.18.2, with `aws-lc-sys` | Replaceable implementation detail | `sandboxed`: C and assembly, under DP10's exception | — (C and assembly) | Network | rustls's crypto provider. Admitted by ADR-0013 | NW6 |
| **`quinn-proto`** 0.11.19 | Replaceable implementation detail | `trusted` in the network process, if its audit agrees | Not graded | Network | QUIC, as a deterministic state machine. "MIT OR Apache-2.0" | NW5 |
| **`miniz_oxide`** | Replaceable implementation detail | `trusted` in the network process and renderer | `ub-risk-0` (0.8.5) | Network, renderer | gzip and deflate (and PNG's inflate) | NW13 |
| **`ruzstd`** 0.9.1 | Replaceable implementation detail | `trusted` in the network process | `ub-risk-2` (0.6.0) | Network | zstd. MIT | NW13 |

`brotli-decompressor` is CP3's entry, shared with NW13.

### From CP7: storage

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **`redb`** 4.4.0 | Replaceable implementation detail | `trusted` in the network process, if its audit agrees | Not graded. No required dependencies; 54 mentions of `unsafe`, mostly in a vendored hash, synchronization and file locking (observed) | Network | Every structured store: web storage, IndexedDB, the Cache API's index, the cookie jar, the HTTP cache's index. Behind Axiom's storage interface. "MIT OR Apache-2.0". **Conditional:** its audit must grade it `ub-risk-2` or better, and it must pass SW13's crash tests | SW2 |
| **SQLite** (`rusqlite`, `libsqlite3-sys` with its bundled C) | Replaceable implementation detail | `sandboxed`: C, under DP10's exception | — (C) | Network | **Fallback only,** if `redb` fails SW2's conditions; then admitted by ADR-0013. Firefox's, WebKit's and Ladybird's engine, and Chromium's for IndexedDB going forward | SW2 |

### From CP8: accessibility

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **AppKit's `NSAccessibility` protocols and HIServices' text markers** (`AXTextMarkerCreate`) | Required platform facility | — | — | Browser (main thread) | The accessibility bridge, through the shell's `platform` module | AX4, AX5, AX8 |

AccessKit was considered and not adopted, by design rather than grade: its macOS adapter lacks rich text and hypertext (AX8).

## 4. Considered and not admitted

| Dependency | Reason | Recorded at |
|---|---|---|
| `encoding_rs` | Graded `ub-risk-3` at 0.8.33 by Google, so DP8 bars it. Axiom writes its own encodings in safe Rust and uses `encoding_rs` as a test reference (TX3). It would be reconsidered only if a newer version earns a published grade of `ub-risk-2` or better | CP3 |
| `zune-jpeg` | Graded `ub-risk-3` at 0.4.19 by Google, so DP8 bars it. JPEG uses a safe decoder instead (IM2) | CP5 |
| `httparse` | A Google delta audit (1.8.0 → 1.9.4) graded it `ub-risk-4`, so DP8 bars it. HTTP/1.1 parsing is Axiom's own (NW3) | CP6 |
| `flate2` | Graded `ub-risk-3` at 1.0.28 by Google (`ub-risk-2` at 1.1.0 by Chromium). Unneeded: `miniz_oxide` is used directly (NW13) | CP6 |

## 5. Spikes only (exempt, DP23)

The rest of `Cargo.lock`, justified in the workspace manifest:
- `syn`, `quote`, `proc-macro2` and `unicode-ident`, for S1's `#[derive(Trace)]` in Pattern A. Pattern B, the pattern ADR-0003 chose, needs no procedural macro.
- `libc` 0.2.191. It appears in the lock without being built on this platform: `cargo tree -i libc` finds no path to it (observed 2026-10-18).
