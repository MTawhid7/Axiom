# Dependency Register

| | |
|---|---|
| **Status** | Living document. Started at Step 5, CP2 |
| **Purpose** | Every dependency proposed for the engine, with the record the [dependency policy](dependency-policy.md) requires (DP6). Nothing listed here is added to the workspace until its approval (DP5) and, for code, an authorized milestone |
| **Last updated** | 2026-10-22 (Round 4: `encoding_rs` admitted under DP8 as amended; `smallvec`'s path; `zune-jpeg` a candidate again; `rustls-webpki` 0.103.16). Earlier: 2026-10-21 (Round 1: `encoding_rs` 0.8.42 graded; Temporal's and BigInt's candidates); 2026-10-20 (Gate C: CP3's, CP4's and CP5's entries decided; S6-Q4's evidence) |

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

The Step 5 facilities (CoreText, ImageIO, Security, DNS-SD, SystemConfiguration and the media frameworks) are listed with their checkpoints in §3, decided by ADRs 0011–0013.

## 2. Crates

### objc2 and its framework crates

| Field | Value |
|---|---|
| **Crates** | `objc2` 0.6.5; the framework crates `objc2-foundation`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-metal`, `objc2-core-text`, `objc2-core-graphics`, `objc2-core-foundation`, `objc2-io-surface`, `objc2-image-io` (S5) and `objc2-security` (S7), 0.3.2; transitively `block2` 0.6.2, `dispatch2` 0.3.1, `objc2-encode` 4.1.0 and `bitflags` 2.13.2 (the versions in `Cargo.lock`) |
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

CP3–CP8 add entries here as their documents propose dependencies, and CP9's ADRs decide them. **At Gate B** (2026-10-18) the owner accepted ADR-0010, which makes this register's format binding, and ADR-0014, whose entry (CP8) is now decided and listed in §1. **ADR-0013 was accepted on 2026-10-19,** once S7's conditions passed, so CP6's and CP7's entries are decided as ADR-0013 records them (below). **ADRs 0011 and 0012 were accepted at Gate C** (2026-10-20), so CP3's, CP4's and CP5's entries are decided as those ADRs record them, with S5's and S6's evidence. A crate's adoption still waits for its milestone, its audit and its grade (DP8, DP15). Published grades are Google's or Chromium's `cargo-vet` audits ([text and fonts](../architecture/text-and-fonts.md) §4, source A1). Each applies to the version audited, so every version adopted is audited again (DP15). Build costs are measured when a crate is first added (DP19).

### From CP3: text, fonts and Unicode

**Decided** by ADR-0011's acceptance (2026-10-20), with spike S5's evidence ([S5 results](../research/s5-results.md)) and S6-Q4's for the rasterizer.

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **CoreText and CoreGraphics** | Required platform facility | — | — (C) | Renderer only | The system font database and fallback cascades; reading tables of system fonts; rasterizing system fonts, as S5-Q2 recommended and the owner approved at Gate C (TX12). Never given a web font | TX6, TX7, TX12 |
| **ICU4X 2.3:** `icu_segmenter`, `icu_properties`, `icu_normalizer`, `icu_casemap`, with `icu_provider`, `icu_collections`, `icu_locale_core`, `zerovec`, `zerotrie`, `yoke`, `potential_utf`, `utf8_iter`, `displaydoc` and `smallvec` | Foundational | `trusted` in the renderer. Browser use for less-trusted data waits for CP6 (IDNA) | Segmenter `ub-risk-1`; properties `ub-risk-0`; normalizer and provider `ub-risk-1`; `zerovec`, `zerotrie`, `yoke`, `icu_collections` `ub-risk-2` (2.0 betas; `zerovec` 0.11.1) | Renderer; others as CP6 and CP16 decide | Unicode data and algorithms for the whole engine, compiled data; behind Axiom's `unicode` interface. License Unicode-3.0. Precedent: Firefox's segmenters; vendored by Chromium and Servo | TX1, TX2, TX16 |
| **`unicode-bidi`** 0.3.18 | Replaceable implementation detail | `trusted` | `ub-risk-1` | Renderer | UAX #9, over ICU4X's character data. "MIT OR Apache-2.0". Servo's | TX17 |
| **HarfRust** 0.14, with `read-fonts`, `font-types`, `bytemuck`, `bitflags` and `smallvec`, and, through `read-fonts` 0.45, `yoke`, `once_cell`, `foldhash`, `stable_deref_trait` and `zerofrom` (S5, observed in `Cargo.lock`) | Replaceable implementation detail | `trusted` in the renderer | Not graded itself; it forbids `unsafe` (observed, 0.14.0). Graded only at older versions: `read-fonts` `ub-risk-0` (0.27.3; S5 used 0.45.1); `font-types` `ub-risk-1` (0.8.3; 0.12.7); `bytemuck` `ub-risk-2` (1.22.0; 1.25.2); `yoke` `ub-risk-2` (0.8.0; 0.8.3); `foldhash` `ub-risk-2` (0.1.5; 0.2.0); `stable_deref_trait` `ub-risk-2` (1.2.0); `zerofrom` `ub-risk-0` (0.1.6). `once_cell` has only `safe-to-run` audits | Renderer | Shaping, behind Axiom's shaping interface. MIT. Vendored by Chromium, at status `test`. S5-Q1: the same glyphs as CoreText in 432 runs ([S5 results](../research/s5-results.md) §3) | TX9, TX11 |
| **`read-fonts`** and **`skrifa`** (Fontations) | Replaceable implementation detail | `trusted` in the renderer | `read-fonts` `ub-risk-0` (0.27.3); `skrifa` `ub-risk-0` (0.29.0); `bytemuck` `ub-risk-2`. Both forbid `unsafe` (observed, 0.45.1 and 0.48.0, the versions S5 used); the newer versions are ungraded | Renderer | Parsing every font; outlines for the Rust rasterizer. "MIT OR Apache-2.0". Chrome's web-font path on Linux, Android and ChromeOS | TX8, TX12 |
| **`brotli-decompressor`** 6.1.0 | Replaceable implementation detail | `trusted` in the renderer, if its audit agrees | Not graded. It forbids `unsafe` unless its `unsafe` or `ffi-api` feature is on, and its default features enable neither (observed in its `lib.rs` and `Cargo.toml`) | Renderer | Brotli, for WOFF2. "BSD-3-Clause/MIT". Its allocator crates, `alloc-no-stdlib` 3.0.0 and `alloc-stdlib` 0.3.0, also forbid `unsafe` without their `unsafe` feature; Google's grades are for older major versions (2.0.4 `ub-risk-4`, 0.2.2 `ub-risk-2`). S5 decoded WPT's WOFF2 fonts and 4,000 mutants with it, without a crash | TX7 |
| **A Rust rasterizer: `vello_cpu` 0.3.0, adopted** (2026-10-20); `zeno` 0.3.3 the alternative, both compared in S5 | Replaceable implementation detail | `trusted` in the renderer | Neither graded. `zeno` has no dependencies and one `unsafe` operation, an unchecked float-to-integer conversion, unsound for coordinates out of range, so callers must range-check (S5 does). `vello_cpu` forbids `unsafe`, but its tree holds `fearless_simd`, `hashbrown`, `bytemuck`, `peniko` and `kurbo` (observed) | Renderer | Filling `skrifa`'s outlines for web fonts. S5-Q2: equal quality, `zeno` 1.7× faster on glyphs. **S6-Q4:** `vello_cpu` filled paths 2.1–6.8× faster than `zeno`, at the same quality. **Adopted by the owner:** "Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone." Its tree (`fearless_simd`, `hashbrown`, `bytemuck`, `peniko`, `kurbo`) is graded then, under DP8 and DP15 ([S5 results](../research/s5-results.md) §4, §8; [S6 results](../research/s6-results.md) §6, §8) | TX12 |

**Open risk: `smallvec`,** which HarfRust and ICU4X's segmenter both require. Google graded 1.11.0 at `ub-risk-3`, and later versions carry only `safe-to-deploy` audits. Axiom grades the version it adopts, at M1's plan (MA12). **As amended in Round 4** (RV4-02): a `ub-risk-3` grade admits it in the renderer under DP8's conditions, with Axiom's audit of each block and a fuzz target: 1.16.3 compiles 43 `unsafe` blocks, 11 with a safety comment, with the `const_generics` and `const_new` features the workspace enables (observed, `tools/unsafe-census.py`, 2026-10-22) ([text and fonts](../architecture/text-and-fonts.md) §4).

**Added in Round 4** (RV4-02, RV4-08; the owner: "Adopt encoding_rs at Milestone 1 under the amended DP8."):

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **`encoding_rs`** 0.8.42, with `simdutf8` 0.1.5 | Replaceable implementation detail, behind TX3's interface | `sandboxed` (DP8, DP9) | **`ub-risk-3`** by Axiom's audit (2026-10-21; [audit](../research/encoding-rs-audit.md)) and by Google's at 0.8.33; `safe-to-deploy` at 0.8.31–0.8.32 for Mozilla, by its author | The renderer and the network process; never the browser, which decodes only UTF-8 with the standard library | The Encoding Standard's decoders and encoders; Firefox's. MIT OR Apache-2.0. No RustSec advisory in its history (2026-10-21's database). **Adopted at M1 under DP8's conditions,** which M1's plan completes: Axiom's audit of each `unsafe` block across the tree, `simdutf8` included (143 compiled blocks, 22 with a safety comment, in 2026-10-21's census), and a libFuzzer target under AddressSanitizer through TX3's interface. **Revisited** if the audit or fuzzing finds a defect upstream doesn't fix, or a better-graded release appears. Behind TX3's interface | Round 4 (TX3) |

### From CP4: graphics

**Decided** by ADR-0012's acceptance (2026-10-20), with spike S6's evidence ([S6 results](../research/s6-results.md)).

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **Metal, QuartzCore (CoreAnimation) and IOSurface** | Required platform facility (already decided, §1) | — | — | Browser; a GPU process later | Drawing tiles, presenting through layer trees, sharing surfaces. Through `platform`, over `objc2-metal` | GR1, GR2 |
| **A path rasterizer for the renderer:** a candidate such as `vello_cpu` 0.3.0 or `tiny-skia` 0.12.0, or the same crate TX12 adopts | Replaceable implementation detail | `trusted` in the renderer | Not looked up | Renderer | Filling SVG shapes, clip paths and complex borders into image resources. **S6-Q4:** `vello_cpu` was 2.1–6.8× faster than `zeno` on paths, and **is adopted** for paths and web fonts alike (TX12), so the renderer has one rasterizer. Filling on the GPU in the compositor would need GR5's amendment, which the owner deferred to the graphics milestone ([OQ-24](../open-questions.md)) | GR5 |

**The compositor needs no new crate:** its pipelines are Axiom's own (GR3). wgpu was rejected for the browser's compositor by design (GR1), not by grade. It remains a candidate for WebGPU in the GPU process (GR10).

### From CP5: images and media

**Decided** by ADR-0012's acceptance (2026-10-20), with S5-Q5's evidence, and IM3 as the owner amended it.

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **ImageIO** | Required platform facility | — | — (C) | The media utility process | AVIF, under DP10's exception, accepted by ADR-0012. **S5-Q5:** in the renderer it needs a Mach lookup of VideoToolbox's decoder service, which the process model denies renderers, so it runs in the media utility process, as the owner approved at Gate C ([S5 results](../research/s5-results.md) §7) | IM3 |
| **VideoToolbox, AudioToolbox, CoreAudio** | Required platform facility | — | — (C) | Media utility process only | Hardware and Apple codecs; audio output | IM11 |
| **`png`** 0.18.1, with `fdeflate`, `miniz_oxide`, `crc32fast`, `simd-adler32`, `adler2`, and **`flate2`** 1.1.10, which `png` requires unconditionally for its encoder (S5, observed in its `Cargo.toml` and `src/encoder.rs`) | Replaceable implementation detail | `trusted` in the renderer | `png` `ub-risk-0` (0.18.0-rc); `fdeflate` `ub-risk-0` (0.3.7, the version used); `miniz_oxide` `ub-risk-0` (0.8.5); `crc32fast` and `simd-adler32` `ub-risk-2`; `flate2` `ub-risk-2` (1.1.0) | Renderer | PNG and APNG, behind the decoder interface. Chromium's PNG decoder. S5-Q5: faster than ImageIO in the sandbox, with less memory | IM2 |
| **`jpeg-decoder`** 0.3.2, `platform_independent` | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` in this mode | Renderer | JPEG. In maintenance mode. **S5-Q5 keeps it:** 1.1–1.8× ImageIO's time in the sandbox, the same memory, and the same pixels every time ([S5 results](../research/s5-results.md) §7). **Round 1** (RV1-22): decided at M1's plan, graded under DP8 or replaced by a decoder of Axiom's own | IM2 |
| **`gif`** 0.14.2 | Replaceable implementation detail | `trusted` in the renderer | `ub-risk-1` (0.12.1) | Renderer | GIF | IM2 |
| **`image-webp`** 0.2.4 | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` | Renderer | WebP | IM2 |
| **`jxl`** 0.7.4 | Replaceable implementation detail | `trusted` in the renderer | `ub-risk-1` (0.1.1) | Renderer | JPEG XL, staged. Chromium's and Firefox's decoder | IM2 |
| **`moxcms`** 0.9.1, with no SIMD feature | Replaceable implementation detail | `trusted` in the renderer | Not graded; forbids `unsafe` without SIMD features | Renderer | ICC and CICP color management | IM8 |

`zune-jpeg` is listed under §4 below. Media's demuxers and software codecs are chosen at the media milestone (IM11).

### From CP6: networking

**Decided** by ADR-0013's acceptance (2026-10-19), with spike S7's evidence ([S7 results](../research/s7-results.md)). Network framework, S7-Q2's comparison, is rejected for TLS.

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **Security: trust evaluation** | Required platform facility | — | — (C) | Network | Certificate verification against the system's trust store (SG7). Through `platform`, over `objc2-security`. **S7:** works in the network profile with one Mach lookup (`com.apple.trustd.agent`) and the executable directory's metadata; `SecPolicyCreateSSL` returns NULL without the latter, despite its header, so `platform` treats it as fallible. Each evaluation blocks its caller (8.4 ms median) and costs about 1.3 M instructions in-process ([S7 results](../research/s7-results.md) §3, §4.3) | NW6 |
| **DNS-SD** (`dns_sd.h`, libSystem) | Required platform facility | — | — (C) | Network | Asynchronous name resolution, and HTTPS records. **S7-Q3:** works from kqueue in the network profile, with the metadata of `/var` beside the socket's rule ([S7 results](../research/s7-results.md) §5) | NW7 |
| **SystemConfiguration** | Required platform facility | — | — (C) | Network | The system's proxy settings | NW9 |
| **rustls** 0.23.46, with `rustls-webpki` 0.103.16 (0.103.15 until Round 4, which updated the spikes' lock for RUSTSEC-2026-0382, RV4-15), `rustls-pki-types` 1.15.1, `subtle`, `zeroize` and `once_cell` (S7, observed) | Foundational: TLS shapes the network process | `sandboxed`, with its crypto provider | Not graded; `forbid(unsafe_code)`; `rustls-webpki` has no `unsafe` either | Network | TLS 1.2 and 1.3, as a state machine (NW1). "Apache-2.0 OR ISC OR MIT". **Its `prefer-post-quantum` default feature is required** (S7). **S7-Q2:** 2.4–3.3× fewer instructions per handshake than Network framework, 1.6× fewer per byte ([S7 results](../research/s7-results.md) §4, §8) | NW6 |
| **`aws-lc-rs`** 1.18.2, with `aws-lc-sys` 0.46.0 | Replaceable implementation detail | `sandboxed`: C and assembly, under DP10's exception | — (C and assembly) | Network | rustls's crypto provider. Admitted by ADR-0013. **S7:** built through `cc`, within T1–T4 (17.7 s for the crate's clean dev tree). Its CPU-jitter entropy source costs 29–43 ms of CPU on a process's first random bytes, once; `AWS_LC_SYS_NO_JITTER_ENTROPY` leaves it out, a decision for the network milestone ([S7 results](../research/s7-results.md) §4.3, §7) | NW6 |
| **`quinn-proto`** 0.11.19, with `tracing` 0.1.44 (unconditional), `rand` 0.10.3, `chacha20`, `getrandom`, `rand_pcg`, `slab`, `lru-slab`, `tinyvec`, `rustc-hash`, `thiserror` and `bytes` (S7, observed) | Replaceable implementation detail | `trusted` in the network process, if its audit agrees | Not graded. Four `unsafe` uses, all `VarInt::from_u64_unchecked` on bounded values. `tracing` is graded `ub-risk-4` at 0.1.39 and `ub-risk-2` for the 0.1.40→0.1.41 delta; `tinyvec` `ub-risk-0` | Network | QUIC, as a deterministic state machine. "MIT OR Apache-2.0". **S7-Q4:** driven from the I/O thread's wait, with an HTTP/3 GET ([S7 results](../research/s7-results.md) §6, §8) | NW5 |
| **Network framework** (`Network/connection.h`) | Required platform facility, if used | — | — (C) | Network | **Rejected for TLS by S7-Q2:** S7's comparison only. It needs no rule beyond the network profile, but costs 2.4–3.3× rustls's instructions per handshake and 1.6× per byte, runs its own threads, and didn't resume sessions in the configuration tried ([S7 results](../research/s7-results.md) §4) | NW6 |
| **`miniz_oxide`** | Replaceable implementation detail | `trusted` in the network process and renderer | `ub-risk-0` (0.8.5) | Network, renderer | gzip and deflate (and PNG's inflate) | NW13 |
| **`ruzstd`** 0.9.1 | Replaceable implementation detail | `trusted` in the network process | `ub-risk-2` (0.6.0) | Network | zstd. MIT | NW13 |

`brotli-decompressor` is CP3's entry, shared with NW13.

### From CP7: storage

**Decided** by ADR-0013's acceptance (2026-10-19), on SW2's terms: `redb` if its audit and SW13's crash tests pass at the storage milestone (M2's plan, since Round 4, RV4-17), otherwise SQLite.

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **`redb`** 4.4.0 | Replaceable implementation detail | `trusted` in the network process, if its audit agrees | Not graded. No required dependencies; 54 mentions of `unsafe`, mostly in a vendored hash, synchronization and file locking (observed) | Network | Every structured store: web storage, IndexedDB, the Cache API's index, the cookie jar, the HTTP cache's index. Behind Axiom's storage interface. "MIT OR Apache-2.0". **Conditional:** its audit must grade it `ub-risk-2` or better, and it must pass SW13's crash tests | SW2 |
| **SQLite** (`rusqlite`, `libsqlite3-sys` with its bundled C) | Replaceable implementation detail | `sandboxed`: C, under DP10's exception | — (C) | Network | **Fallback only,** if `redb` fails SW2's conditions; then admitted by ADR-0013. Firefox's, WebKit's and Ladybird's engine, and Chromium's for IndexedDB going forward | SW2 |

### From CP8: accessibility

| Dependency | Class | Group | Published grade | Processes | Purpose, and the interface in front of it | Proposed by |
|---|---|---|---|---|---|---|
| **AppKit's `NSAccessibility` protocols and HIServices' text markers** (`AXTextMarkerCreate`) | Required platform facility | — | — | Browser (main thread) | The accessibility bridge, through the shell's `platform` module | AX4, AX5, AX8 |

AccessKit was considered and not adopted, by design rather than grade: its macOS adapter lacks rich text and hypertext (AX8).

### From CP13–CP17: part 2

**No runtime dependency.** The parsers, the DOM, style, layout, paint, the JavaScript engine and the WebIDL generator are Axiom's own (DP1 (a), (b), (g), (h)); `Intl` uses ICU4X, already proposed at CP3 (TX2, JE21). The part 2 documents name **test references** and **data**, each recorded here, with its group (`test` or `build`) and license, when a test or the build first uses it:
- test references: `html5ever` and `xml5ever` (HD33), `regress`, `temporal_rs` and `num-bigint` (JE32), and Taffy as a candidate (LY32). **Since Round 1** (RV1-08), `temporal_rs` is also the candidate for Temporal's arithmetic, graded under DP8 before M3, and `num-bigint`, or a crate with arithmetic over limb slices, the candidate for BigInt's, graded before M4 (JE13, JE22);
- data: webref's curated IDL (WB1), html5lib-tests (HD33), css-parsing-tests (ST36) and test262 (JE32).

## 4. Considered and not admitted

| Dependency | Reason | Recorded at |
|---|---|---|
| `encoding_rs` | **Admitted in Round 4** (RV4-02, RV4-08): the entry in §3 records it. Before: graded `ub-risk-3` at 0.8.33 by Google, so DP8 barred it. Axiom writes its own encodings in safe Rust and uses `encoding_rs` as a test reference (TX3). **Graded again by Axiom's own audit** (Round 1, RV1-08; 2026-10-21; [audit](../research/encoding-rs-audit.md)): **0.8.42 is `ub-risk-3`** as it builds on this Mac, with `simd-accel` off: safety comments on 22 of 143 compiled `unsafe` blocks; uninitialized pages made a `&mut [u8]` behind an empty `asm!` barrier; and `simdutf8` 0.1.5's NEON validator, ungraded, unconditional on aarch64. DP8 still bars it. Reconsidered on a release that fixes all three, by a new audit (DP15); the owner approves this one with the dependency | CP3; Round 1 |
| `zune-jpeg` | Graded `ub-risk-3` at 0.4.19 by Google, so DP8 barred it, and JPEG used a safe decoder instead (IM2). **A candidate again in Round 4** (RV4-02): 0.5.15, released 2026-10-19, forbids `unsafe` at its root unless its `x86` or `neon` features are on (observed, `src/lib.rs`), so without them it has none; M1's plan chooses between it and `jpeg-decoder` (IM2, MA12) | CP5; Round 4 |
| `httparse` | A Google delta audit (1.8.0 → 1.9.4) graded it `ub-risk-4`, so DP8 bars it. HTTP/1.1 parsing is Axiom's own (NW3) | CP6 |
| `flate2` | Graded `ub-risk-3` at 1.0.28 by Google (`ub-risk-2` at 1.1.0 by Chromium). Unneeded: `miniz_oxide` is used directly (NW13). **But S5 found that `png` 0.18.1 requires it** for its encoder, so it enters the renderer with `png` (§3, CP5); its version's grade then matters as part of `png`'s tree | CP6 |

## 5. Spikes only (exempt, DP23)

The rest of `Cargo.lock`, justified in the workspace manifest:
- `syn`, `quote`, `proc-macro2` and `unicode-ident`, for S1's `#[derive(Trace)]` in Pattern A. Pattern B, the pattern ADR-0003 chose, needs no procedural macro.
- `libc` 0.2.191. It appears in the lock without being built on this platform: `cargo tree -i libc` finds no path to it (observed 2026-10-18).
