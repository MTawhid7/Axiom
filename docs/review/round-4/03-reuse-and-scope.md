# Review Round 4: Reuse and Scope

| | |
|---|---|
| **Status** | **Round 4, disposed** (2026-10-22): the owner accepted all seventeen findings as proposed, with the choices the [disposition log](../README.md#59-round-4s-dispositions) records. It stays as written, as the round's record; where the owner's words differ, they govern |
| **Question** | The owner's first two themes: does the baseline rebuild components that serve few uses, and does it reuse mature software wherever that meets Axiom's security, compatibility and architectural requirements? |
| **Covers** | Every component of Milestone 1 ([MA1](../../milestone-1.md)) and, more briefly, of M2–M9 ([roadmap](../../roadmap.md)); the [dependency policy](../../technology/dependency-policy.md) and [register](../../technology/dependency-register.md); the build-or-reuse sections of the designs |
| **Summary** | [Findings](06-findings.md): RV4-01, RV4-02, RV4-08, RV4-09, RV4-15 · [Actions](07-actions.md) |
| **Last updated** | 2026-10-22 |

**The owner's words** (2026-10-22): "we should carefully evaluate whether rebuilding components from scratch is justified when they serve only a small subset of use cases"; "prioritizing the core functionality, the most common use cases, and the features that deliver the greatest value"; "take full advantage of mature, battle-tested software wherever appropriate, provided it meets our security, compatibility, and architectural requirements", while evaluating "each dependency carefully for known vulnerabilities, legacy constraints, maintenance risks, and long-term suitability rather than assuming that maturity alone guarantees quality."

**A default this round took without asking** (the plan, approved 2026-10-22): **G1's core list and ADR-0001 R2 stand.** The engine's own mechanisms (parsing, style, layout, paint, invalidation, navigation, security policy, the JavaScript engine) stay Axiom's; where this document weighs reuse inside them, it is flagged as an amendment only the owner can make.

---

## 1. The answer

1. **Axiom already reuses most of its infrastructure** (observed, the [register](../../technology/dependency-register.md)): the system's frameworks (AppKit, Metal, CoreAnimation, CoreText for system fonts, Security's trust evaluation, DNS-SD, the sandbox); HarfRust and Fontations for shaping and fonts; ICU4X for Unicode; `vello_cpu` for web fonts' glyphs; `moxcms` for colour; the `png`, `gif` and `image-webp` decoders; `miniz_oxide`, `brotli-decompressor` and `ruzstd`; rustls with `aws-lc-rs`; `quinn-proto`; objc2.
2. **It writes its own where G1 says it must,** and also in places G1 doesn't require, where a policy rule or a poor fit pushed it (observed, the designs' build-or-reuse sections):

   | Written by Axiom, outside G1 | Why | This round |
   |---|---|---|
   | The Encoding Standard's codecs (TX3) | `encoding_rs` is graded `ub-risk-3` (DP8) | **Reuse,** under RV4-02's amended DP8, or stage (RV4-08) |
   | JPEG (IM2), if `jpeg-decoder` fails its grade | `zune-jpeg` is graded `ub-risk-3` | **`zune-jpeg` without its SIMD features,** where its crate root forbids `unsafe` (§4), measured at M1's plan |
   | HTTP/1.1, HTTP/2 with HPACK, HTTP/3 with QPACK (NW3–NW5) | `httparse` graded `ub-risk-4`; `h2` and `h3` tied to async runtimes, which NW1's state machines can't drive | **Keep:** sans-I/O is the architecture (NW1), and no graded alternative fits. Stage HTTP/2 to M2 (RV4-08) |
   | URLs (NW2) | One parser in every process, the browser included, where DP9 asks `ub-risk-1` for less-trusted data, and `idna`'s ICU4X data crates are `ub-risk-2` | **Keep;** `url` stays the differential reference |
   | Temporal's and BigInt's arithmetic (JE22, JE13) | Already reclassified as replaceable (RV1-08): `temporal_rs` and a BigInt crate are graded before M3 and M4 | **No change** |

3. **Scope by value** moves four things out of M1 and one into it (§5; RV4-08): **in,** the XML parser, without which M1's gate can't run (RV4-01); **out,** HTTP/2 and HPACK, the legacy encodings (unless `encoding_rs` is admitted), text entry into pages, and the preload scanner, which pays first at M3. M1's CSS target asks for CSS 2.1's long tail first (RV4-09).
4. **The policy's one structural flaw** is that it admits C under conditions and never admits `ub-risk-3` Rust (RV4-02, High). Fixing it is what the owner's second theme needs; nothing else in the policy stands in its way.

## 2. Method and evidence

- **The inventory** (§3) takes each component MA1 names, and those of later milestones that a decision now would affect. For each: who writes it; how much of M1 uses it (the corpus, MA4's tests); the mature alternatives with their dated facts; their fit with Axiom's architecture (sans-I/O state machines, 8- and 16-bit strings, the node arena, the process it runs in, its budgets); and a verdict: **keep our own, reuse now, reuse when needed, stage,** or **defer**.
- **Dated facts** (§4) come from `tools/crate-facts.py`, a tool this round adds, on 2026-10-22: crates.io's API [I1], GitHub through `gh api`, RustSec's advisory database at `4b587f0099` (2026-10-21) [R1], and the six `cargo-vet` audit sets Firefox imports, from cargo-vet's registry [V1]. Downloaded data is never run (`python3 -I`).
- **Sources read for `unsafe`:** `smallvec` 1.16.3, `jpeg-decoder` 0.3.2, `url` 2.5.8, `idna` 1.1.0 and `brotli-decompressor` 6.1.0 from the local registry cache, which Cargo checked against the lock; `zune-jpeg` 0.5.15, `quick-xml` 0.42.0, `xmlparser` 0.13.6 and `xml` 1.4.0 downloaded from crates.io into their own directories under `target/review-r4/crates/`, each `.crate` checked against the index's SHA-256. Counts are `tools/unsafe-census.py`'s, in the features Axiom would use. **One limitation:** the census evaluates `#[cfg]` on items and modules, not a module file's own inner `#![cfg]`, so it overcounts crates that gate their SIMD modules that way; for `zune-jpeg` the crate root's attribute is quoted instead.
- **Usage data:** the corpus ([S8](../../research/s8-results.md)); MA4's tests ([verification](02-verification.md)); W3Techs for encodings, a **secondary** source [W2].
- **Not done:** no candidate was built or run, and no grade was assigned; grading is M1's plan's task (MA12). No benchmark compares a crate with Axiom's own design.

## 3. What M1 builds, and who writes it

| Component (MA1) | Today | What M1 needs of it | Mature alternatives | Fit | Verdict |
|---|---|---|---|---|---|
| HTML tokenizer and tree builder | Axiom's (G1, HD1–HD8) | All of it: the corpus and every HTML test | `html5ever` 0.40.1 (Servo) | Its strings are UTF-8 (Servo's README, HD §3), against HD's 8- and 16-bit text | **Keep** (G1); `html5ever` is TS7's differential reference |
| **XML parser** | Axiom's (HD9), **staged at M4** | **5,423 of MA4's 5,652 tests** (RV4-01) | `xml` 1.4.0, `quick-xml` 0.42.0 and `xmlparser` 0.13.6, each forbidding `unsafe`; `xml5ever` 0.40.0 | A tokenizer is most of a small parser; Axiom's tree builder and arena stay | **Bring to M1; keep our own** (G1); reuse of a tokenizer as the owner's alternative (RV4-01) |
| CSS syntax, selectors, cascade | Axiom's (G1) | All of it | `cssparser` 0.38.0, `selectors` 0.41.0, Stylo 0.22.0 | Stylo's parallel traversal keeps element data behind `AtomicRefCell` and uses `unsafe` to send nodes between threads, where Axiom lends arenas (style §3) | **Keep** (G1) |
| Layout, paint, compositor | Axiom's (G1) | CSS 2.1's visual formatting model | Taffy 0.15.0: block, flexbox and grid | Servo uses it for grid only (layout §3) | **Keep;** Taffy as a candidate reference at M4 (LY32) |
| Shaping, fonts | HarfRust 0.14.0, `read-fonts` and `skrifa` (reused) | All text | — | Each forbids `unsafe` (observed, their crate roots) | **Reused;** blocked by `smallvec`'s grade unless RV4-02 is accepted |
| Unicode | ICU4X 2.3 (foundational) | Line breaking, bidi, properties | — | — | **Reused;** the same `smallvec` dependency |
| **Encodings** | Axiom's, every encoding (TX3) | **UTF-8, UTF-16 and windows-1252** decide all but 38 tests (0.7%) | `encoding_rs` 0.8.42, Firefox's | Behind TX3's interface, in sandboxed helpers | **Reuse under RV4-02,** or stage the rest to M4 (RV4-08) |
| Images | `png`, `gif`, `image-webp` (reused); JPEG open (IM2) | Wikipedia's and MDN's images | `zune-jpeg` 0.5.15; `jpeg-decoder` 0.3.2 | Both incremental enough for IM9 (S5-Q5 used `jpeg-decoder`) | **`zune-jpeg`, without SIMD,** against `jpeg-decoder` at M1's plan |
| Colour | `moxcms` 0.9.1 (reused) | Display P3 output, tagged images | — | Young: first released 2025-02-26 | **Reused;** its maintenance watched (DP6) |
| Compression | `miniz_oxide`, `brotli-decompressor`, `ruzstd` (reused) | Content encodings | — | — | **Reused** |
| HTTP/1.1, TLS | Axiom's framing; rustls with `aws-lc-rs` | Every load | `httparse` 1.10.1 (`ub-risk-4`); Network framework | NW1's sans-I/O | **Keep** |
| **HTTP/2 and HPACK** | Axiom's (NW4), **at M1** | **No M1 criterion** (RV4-08) | `h2` 0.4.20 (async); `hpack` 0.3.0, unmaintained and vulnerable | — | **Keep our own; stage to M2** |
| URLs | Axiom's (NW2) | Every load and link | `url` 2.5.8, with `idna` 1.1.0 | One parser in every process; DP9 in the browser | **Keep** |
| DNS | DNS-SD (platform) | Live loads | — | One wait (S7-Q4) | **Kept** |
| The shell's text input and accessibility | objc2 classes (EM11) | The address field | A Swift shell (EM11's fallback) | — | **Keep:** it retires OQ-11's risk at M1 |
| **Text entry into pages** | IN7's mirror and HD26's text controls, implied at L0 | **No M1 criterion** types into a page | — | — | **Stage to M4,** with form controls (RV4-08) |
| **The preload scanner** | HD, staged at M4 | Nothing before script | — | — | **Move to M3,** with script (RV4-08) |

## 4. Dated facts on the crates Axiom would use

Read on 2026-10-22. **Newest** is the newest release that isn't yanked, with its date; **Grades** are the newest `ub-risk` grade any of the six audit sets records, with the version or delta it covers; **`unsafe`** is in the configuration Axiom would use (§2). Advisories are RustSec's, with the year's in brackets.

| Crate | Newest | First | Advisories (2026) | Grades | `unsafe` | Role |
|---|---|---|---|---|---|---|
| `smallvec` | 1.16.3, 2026-10-16 (2.0.0-beta.3) | 2015 | 5, the last in 2021 (0) | **`ub-risk-3` at 1.11.0** (Google); later versions `safe-to-deploy` only (Google, Chromium, Mozilla, the Bytecode Alliance) | **43 blocks,** 11 with a safety comment; 10 functions, 6 impls, 1 trait (`const_generics`, `const_new`) | HarfRust, ICU4X's segmenter, `kurbo` |
| `encoding_rs` | 0.8.42, 2026-09-24 | 2016 | 0 | **`ub-risk-3`** at 0.8.33 (Google: "Needs extensive safety comments"); Axiom's audit at 0.8.42; `safe-to-deploy` at 0.8.31 by its author for Mozilla | Axiom's [audit](../../research/encoding-rs-audit.md) | TX3's candidate |
| `zune-jpeg` | 0.5.15, 2026-10-19 (0.5.16-rc3) | 2022 | 0 | **`ub-risk-3` at 0.4.19** (Google) | **None without its `x86` and `neon` features:** its root says `#![cfg_attr(not(any(feature = "x86", feature = "neon")), forbid(unsafe_code))]` | IM2's candidate |
| `jpeg-decoder` | 0.3.2, **2025-06-21**; repository last pushed the same day | 2016 | 0 | None | **None with `platform_independent`,** where its root forbids `unsafe`, S5's configuration. *Corrected in CP2 (2026-10-22): this cell first read "2 blocks", a census run without the feature; the 2 blocks are its SIMD paths* | S5's JPEG decoder |
| `xml` (formerly `xml-rs`) | 1.4.0, 2026-08-06 | 2014 | 0 (an "unmaintained" notice of 2022, withdrawn) | None | Forbidden at the root | XML candidate |
| `quick-xml` | 0.42.0, 2026-08-22 | 2016 | **2 (2):** quadratic attribute checks and unbounded namespace declarations, both denial of service, fixed in 0.41.0 | `safe-to-run` only | Forbidden at the root | XML candidate |
| `xmlparser` | 0.13.6, **2023-09-30**; repository pushed 2025-12-29 | 2017 | 0 | None | Forbidden at the root | XML candidate |
| `xml5ever` | 0.40.0, 2026-09-11 | 2015 | 0 | None | — | HD9's differential reference |
| `html5ever` | 0.40.1, 2026-09-14 | 2015 | 0 | None | — | TS7's differential reference |
| HarfRust | 0.14.0, 2026-10-04 | **2025-06-10** | 0 | None | Forbidden at the root | Shaping |
| `read-fonts`, `skrifa` | 0.46.0, 0.49.0, 2026-10-21 | 2022, 2023 | 0 | `ub-risk-0` at 0.27.3 and 0.29.0 | Forbidden at the root (S5's 0.45.1 and 0.48.0) | Fonts |
| `bytemuck` | 1.25.2, 2026-07-19 | 2019 | 0 | `ub-risk-2` to 1.22.0; one delta `ub-risk-3` (1.16.3 → 1.17.0) and one `ub-risk-4` (1.13.1 → 1.14.3) | — | Under fonts, ICU4X, `png` |
| ICU4X (`icu_segmenter`, `icu_properties`, `icu_normalizer`, `icu_provider`) | 2.3.x, 2026-08 | 2020–2021 | 0 | `ub-risk-1`, `ub-risk-0`, `ub-risk-1`, `ub-risk-1` at 2.0.0 betas | — | Unicode |
| `zerovec` | 0.11.8, 2026-08-19 | 2021 | 1, 2024, fixed | `ub-risk-2` at 0.11.1 | — | Under ICU4X |
| `vello_cpu` | 0.3.0, 2026-10-02 | **2025-05-09** | 0 | None | Forbidden at the root | Web fonts' glyphs |
| `moxcms` | 0.9.1, 2026-09-15 | **2025-02-26** | 0 | None | — | Colour |
| `png`, `fdeflate`, `miniz_oxide` | 0.18.1, 0.3.7, 0.9.1 | 2015, 2022, 2017 | 0 | `ub-risk-0` | — | PNG, deflate |
| `gif` | 0.14.2, 2026-04-09 | 2015 | 0 | `ub-risk-1` at 0.12.1 | — | GIF |
| `image-webp` | 0.2.4, 2025-08-27 | 2024 | 0 | None | Forbidden at the root | WebP |
| `brotli-decompressor` | 6.1.1, 2026-10-21 | 2017 | 0 | None | 0 blocks with `std` (6.1.0) | WOFF2, content encoding |
| `ruzstd` | 0.9.1, 2026-10-11 | 2019 | 2 (1): a decompression bomb, fixed in 0.9.1 | `ub-risk-2` at 0.6.0 | — | Content encoding |
| rustls | 0.23.46, 2026-10-14 | 2016 | **7 (5),** all fixed in 0.23.46, the lock's | None; no `unsafe` (NW §4) | — | TLS |
| `rustls-webpki` | 0.103.16, 2026-10-20 | 2023 | **6 (5);** the lock's 0.103.15 is affected by RUSTSEC-2026-0382 (RV4-15) | None | — | Under rustls |
| `aws-lc-sys` | 0.46.0, 2026-10-12 | 2022 | **5 (5),** fixed from 0.38 and 0.39; the lock has 0.46.0 | None (C and assembly, DP3) | — | Cryptography |
| `quinn-proto` | 0.11.19, 2026-09-30 | 2018 | **10 (8),** denial of service and panics, fixed by 0.11.18; the lock has 0.11.19 | None | — | QUIC, later |
| `h2` | 0.4.20, 2026-10-06 | 2017 | 4 (1) | `safe-to-run` only | — | Not used (NW4) |
| `hpack` | 0.3.0, **2015-10-22** | 2015 | **Unmaintained,** and a vulnerability (2023) | None | — | Not used |
| `httparse` | 1.10.1, 2025-03-03 | 2015 | 0 | **`ub-risk-4`** on the delta 1.8.0 → 1.9.4 | — | Not used (NW3) |
| `url`, `idna` | 2.5.8, 1.1.0 | 2014, 2016 | 0; `idna` 1 (2024, fixed) | None | 1 block; 16 blocks | NW2's differential reference |
| `temporal_rs` | 0.2.6, 2026-08-05 | 2024 | 0 | `ub-risk-1` at **0.0.6** (Google, Chromium) | — | JE22: graded again before M3 |
| `redb`; `rusqlite` | 4.4.0; 0.40.2 | 2018; 2014 | 0; 2 (old) | None; `ub-risk-3` at 0.32.0 (it binds SQLite's C) | — | SW13's choice, by crash tests |
| `taffy` | 0.15.0, 2026-10-12 | 2022 | 0 | None | — | M4's reference candidate |
| `accesskit_macos` | 0.27.1, 2026-09-25 | 2022 | 0 | None | — | AX8's comparison, at M4 |
| `mp4parse` | 0.17.0, **2023-05-29**; repository pushed 2026-10-16 | 2015 | 0 | None | — | M7's demuxer candidate |
| `wgpu`, `wgpu-hal` | 30.0.1, 2026-08-22 | 2019, 2021 | 0 | None | A large tree (GR §4) | WebGPU at M9 (GR10) |
| `objc2` and its framework crates | 0.6.5 (2026-10-05); 0.3.2 | 2021 | 0 | None | Foundational (DP5) | `platform` |

**What the facts say** (inference):
- **No crate on M1's path has an open advisory** in the version Axiom would take, except `rustls-webpki` in the spikes' lock (RV4-15).
- **The security-critical networking crates are actively patched:** rustls had five advisories in 2026 and `quinn-proto` eight, each fixed within weeks. That favours reuse under TS15's routine, and is a reason to add HTTP/3 only when it measurably helps (§5.4).
- **Four candidates are dormant:** `jpeg-decoder` (no release since 2025-06-21), `xmlparser` (2023-09-30), `mp4parse` (2023-05-29) and `hpack` (2015, unmaintained). A dormant crate is a maintenance risk the owner named; each is either avoided or has an active alternative.
- **Three adopted crates are young:** HarfRust, `vello_cpu` and `moxcms`, all first released in 2025. Their maintenance is watched under DP6, and their interfaces are Axiom's (DP2).
- **Grades are old where they exist:** `temporal_rs` at 0.0.6, `smallvec` at 1.11.0, `zune-jpeg` at 0.4.19, `bytemuck` to 1.22.0; Axiom grades the version it adopts (DP8), as MA12 already plans for `smallvec`.

## 5. Scope by value

### 5.1 Milestone 1

M1's purpose (MS5; the roadmap): static documents, Wikipedia's and MDN's, readable from local copies and live; CSS 2.1's visual formatting model, validated by `css/CSS2`; every interface of the later levels, in real sandboxed processes. Weighed against that ([RV4-08](06-findings.md#rv4-08-m1s-scope-against-value)):

| Item | Value at M1 | Cost at M1 | Recommendation |
|---|---|---|---|
| **The XML parser** | 5,423 of the 5,652 tests can't run without it | A small parser (HD §3) and a fuzz target | **Add** (RV4-01) |
| **HTTP/2 and HPACK** | No criterion; fewer connections on live pages | Framing, HPACK, flow control, priorities, coalescing (NW4); two fuzz targets | **To M2,** with the disk cache and cookie jar |
| **Legacy encodings** | At most 38 tests (0.7%); about 1% of websites use windows-1252's labels, the rest below 0.1% each (secondary) | Every decoder and encoder, their index tables and tests | **UTF-8, UTF-16 and windows-1252 at M1; the rest at M4,** or every encoding at M1 through `encoding_rs` if RV4-02 is accepted |
| **Text entry into pages** | No criterion; 3 tests use form controls, whose rendering is M4's | IN7's mirror for pages, editing in text controls | **To M4,** with form controls |
| **The preload scanner** | None without script | A second tokenizer pass | **To M3,** from M4 |

### 5.2 What M1 keeps, and why

- **Tables** (LY10, approximate): Wikipedia's infoboxes and MDN's tables (the roadmap's reason).
- **DNS-SD and one wait per thread** (S7-Q3, S7-Q4): the architecture's idle cost (AC3).
- **HSTS, mixed content and Referrer-Policy:** G1's security policies, cheap at M1.
- **The objc2 validation** (EM11): it retires the shell's language risk (OQ-11) before the shell grows.
- **The Rust path for the test fonts** (TX21): what makes the references independent of macOS.
- **Real sandboxed processes** (RV1-07): the owner's decision, and the reason Round 3's M1-A is rejected ([RV3-15](02-verification.md#rv3-15-staging-m1)).

### 5.3 M1's CSS target

Pass rates over MA4's 5,652 tests, by directory, from wpt.fyi's runs at `0f26972422` [W1] (`tools/wpt-pass-rates.py --tests`), in percent. The median is of Chrome, Firefox and Safari; the last column is RV4-09's recommended floor, 10 points below the median, for directories of 100 tests or more.

| Directory | Tests | Chrome | Firefox | Safari | Servo | Ladybird | Median | Floor |
|---|---|---|---|---|---|---|---|---|
| `normal-flow` | 736 | 99.3 | 99.3 | 97.1 | 99.3 | 89.0 | 99.3 | 89.3 |
| `margin-padding-clear` | 682 | 99.3 | 99.1 | 99.1 | 98.8 | 96.6 | 99.1 | 89.1 |
| `positioning` | 513 | 99.8 | 99.8 | 97.1 | 99.8 | 96.7 | 99.8 | 89.8 |
| `borders` | 504 | 100 | 100 | 97.4 | 100 | 96.2 | 100 | 90.0 |
| `selectors` | 468 | 97.9 | 90.6 | 97.4 | 95.3 | 95.5 | 97.4 | 87.4 |
| `text` | 388 | 98.2 | 97.9 | 97.4 | 82.7 | 93.6 | 97.9 | 87.9 |
| `backgrounds` | 333 | 99.7 | 99.7 | 90.1 | 96.7 | 97.0 | 99.7 | 89.7 |
| `syntax` | 275 | 98.9 | 98.9 | 98.9 | 93.8 | 97.5 | 98.9 | 88.9 |
| `floats-clear` | 211 | 100 | 97.6 | 97.6 | 96.7 | 84.4 | 97.6 | 87.6 |
| `generated-content` | 205 | 95.1 | 97.1 | 91.7 | 69.8 | 80.0 | 95.1 | 85.1 |
| `linebox` | 191 | 96.9 | 99.0 | 97.4 | 97.4 | 92.7 | 97.4 | 87.4 |
| `css1` | 164 | 97.6 | 98.2 | 95.7 | 93.3 | 80.5 | 97.6 | 87.6 |
| `fonts` | 159 | 98.7 | 98.7 | 98.1 | 89.9 | 95.0 | 98.7 | 88.7 |
| `lists` | 155 | 100 | 100 | 99.4 | 62.6 | 88.4 | 100 | 90.0 |
| `bidi-text` | 105 | 97.1 | 89.5 | 92.4 | 78.1 | 58.1 | 92.4 | 82.4 |
| `floats` | 103 | 88.3 | 90.3 | 81.6 | 96.1 | 76.7 | 88.3 | 78.3 |
| `box-display` | 86 | 89.5 | 97.7 | 96.5 | 91.9 | 93.0 | 96.5 | — |
| `visufx` | 49 | 100 | 100 | 98.0 | 100 | 100 | 100 | — |
| `ui` | 52 | 100 | 100 | 100 | 100 | 84.6 | 100 | — |
| `visudet` | 37 | 64.9 | 89.2 | 89.2 | 64.9 | 54.1 | 89.2 | — |
| `cascade` | 31 | 96.8 | 96.8 | 96.8 | 83.9 | 87.1 | 96.8 | — |
| `visuren` | 31 | 100 | 100 | 96.8 | 100 | 83.9 | 100 | — |
| `zindex` | 29 | 100 | 100 | 100 | 100 | 89.7 | 100 | — |
| `abspos` | 25 | 96.0 | 96.0 | 100 | 100 | 84.0 | 96.0 | — |
| `values` | 25 | 88.0 | 96.0 | 100 | 80.0 | 88.0 | 96.0 | — |
| `sec5` | 23 | 100 | 100 | 100 | 95.7 | 95.7 | 100 | — |
| `colors` | 19 | 100 | 100 | 57.9 | 100 | 100 | 100 | — |
| `media` | 17 | 100 | 100 | 100 | 100 | 100 | 100 | — |
| Seven directories of 1 to 9 tests, and 9 tests directly in `css/CSS2/` | 36 | — | — | — | — | — | — | — |
| **All** | **5,652** | **98.2** | **97.8** | **96.4** | **93.7** | **91.4** | **97.8** | **90.0 overall** |

The seven small directories are `box`, `cascade-import`, `css21-errata`, `csswg-issues`, `other-formats`, `stacking-context` and `zorder` (every row in `scratch/review-r4/css2-pass-rates-by-dir.md`, not committed); `css21-errata`'s 9 tests pass at 33.3% in Chrome and Safari.

### 5.4 Later milestones

Each is a note for that milestone's plan, not a change now (inference, from §4 and the designs):

| Milestone | What to weigh | Lean |
|---|---|---|
| M2 | HTTP/2 and HPACK, from M1; the disk cache's format with storage (NW10, CP7); `IOSurfaceSetOwnershipIdentity` (PF4) | Axiom's own framing, as NW4 |
| M3 | `redb` against SQLite (SW13, by crash tests); `temporal_rs`, graded again (JE22); the preload scanner, from M4 | Reuse what passes its grade; SQLite's C would need DP10's path |
| M4 | Taffy as a flexbox and grid reference; AccessKit's macOS adapter checked again for rich text (AX8); the legacy encodings, if staged; text entry and form controls | References and checks, not dependencies |
| M7 | Demuxers (IM11): `mp4parse` is dormant since 2023, so an active alternative or Axiom's own | Decided by maintenance |
| M8, M9 | WebAssembly (OQ-3, S14); WebGPU through `wgpu`, as Firefox does ([graphics](../../architecture/graphics.md) [F4]), in its own process (GR10); WebGL through ANGLE, the engines' common choice, under DP10's path for C++ | **Reuse first:** these are large specifications where mature implementations exist, and they run sandboxed |
| Any | HTTP/3: added when the corpus measures a benefit, not by default (S8: HTTP/3 carries most of Maps' traffic) | Measured |

## 6. The policy

- **DP8 against DP10** is RV4-02, the round's High finding: C can enter under three conditions, `ub-risk-3` Rust never.
- **The burden of proof.** Principle 4 and DP4 ask reuse to justify itself; the owner's direction asks the same of writing our own, for supporting infrastructure. RV4-02's third item proposes the change for DP2's list, leaving G1 alone.
- **What already serves the owner's criteria:** DP6's register records maintenance, advisory history, precedent and alternatives; DP15's audits and DP16's `cargo-deny` checks cover vulnerabilities; DP19 measures each dependency's cost against T1–T4; DP20 and TS15 bring advisories to rebuilds. The gap is the routine's start: the tools wait for DP22's authorization, so until then the advisories are checked by hand at gates (RV4-15).

## 7. Limits

- **The core wasn't re-litigated:** reuse of `html5ever`, Stylo, oxc or Boa's parser, or `regress`, would amend G1 and ADR-0001 R2, and isn't recommended: they are what the project exists to build (G1), and none fits HD's or the engine's string and memory model without rework (the designs' §3 and §4).
- **No grade was assigned.** The grades in §4 are others' or Axiom's earlier audit; M1's plan grades what it adopts.
- **Usage shares** for encodings are W3Techs' (secondary); the corpus's own encodings are checked from its dated copies at M1's plan (hypothesis: all UTF-8).

## Sources

All read on 2026-10-22.

- [I1] crates.io's API, by `tools/crate-facts.py` — https://crates.io/api/v1/crates/ ; the sparse index for checksums — https://index.crates.io/
- [R1] RustSec's advisory database at `4b587f0099` (2026-10-21) — https://github.com/rustsec/advisory-db/tree/4b587f00996ebe5c05a73eaef105ef241e876414
- [V1] `cargo-vet` audits at `main`, from cargo-vet's registry — https://raw.githubusercontent.com/mozilla/cargo-vet/main/registry.toml : Google's, Mozilla's, Chromium's, the Bytecode Alliance's, Embark's and ISRG's, as listed in the [findings](06-findings.md#sources)' [V1]
- [W1] wpt.fyi's run summaries at `0f26972422` — https://wpt.fyi/api/runs?sha=0f26972422
- [W2] **Secondary.** W3Techs, "Usage statistics of character encodings for websites", dated 2026-10-22 — https://w3techs.com/technologies/overview/character_encoding
- **Local:** the crates' sources under `~/.cargo/registry/src/` and `target/review-r4/crates/`; `scratch/review-r4/crate-facts.md` and `.json` (not committed).
