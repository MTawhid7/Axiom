# Review Round 1: Challenged Assumptions

| | |
|---|---|
| **Status** | **Round 1, disposed** (2026-10-21): the owner accepted every finding ([the disposition log](../README.md#5-the-disposition-log)) |
| **Question** | Which assumptions in ADRs 0001–0021, and in the handover's list, are outdated, over-optimistic or contradictory against WebKit, Blink and V8, Gecko, Ladybird and Servo as of 2024–2026, and which hold and should not be reopened? |
| **See also** | [Risk matrix](01-risk-matrix.md) · [Findings](02-findings.md) · [Actions for Round 2](04-round-2-actions.md) |
| **Last updated** | 2026-10-21 |

## 1. How to read it

**Verdicts:**
- **Sound:** holds against current practice and the evidence; don't reopen.
- **Sound, with a caveat:** holds, but a stated part needs a decision or a fix.
- **Over-optimistic:** the direction is right, but the plan assumes an outcome the evidence doesn't support.
- **Outdated:** current practice, or a fact, has moved past the decision.
- **Contradictory:** documents of the baseline disagree.

**Evidence** is in the [findings](02-findings.md), with sources; this page cites them by ID. Claims here without a finding are labelled.

## 2. The ADRs

| ADR | Decides | Verdict | What is challenged, and against which practice | Findings |
|---|---|---|---|---|
| [0001](../../adr/0001-scope-and-target.md) | Scope: the full app web reachable; Axiom's own JavaScript engine; sensitive logins, gated | **Sound, with a caveat** | The goals stand. The schedule mitigation ("strict milestone staging", oracles) doesn't reach the largest lever, which is what Axiom rebuilds | RV1-08 |
| [0002](../../adr/0002-implementation-language.md) | Rust, with `unsafe` confined; T1–T4 | **Sound, with a caveat** | The language is settled. What safe Rust does on failure (panics, overflow) is not. T1–T4 are untested where Servo's build suffered most, on generated code | RV1-16, RV1-18 |
| [0003](../../adr/0003-gc-handle-and-rooting-pattern.md) | Pattern B: `u32` handles, scope tokens, a root stack, a moving collector | **Sound, with a caveat** | Measured twice (S1, S3); Nova 1.0 is a shipped precedent. The caveat: a moving collector that rewrites handles has an O(heap) step, which ADR-0008 must design around | RV1-04 |
| [0004](../../adr/0004-process-model-and-sandbox.md) | Processes, Seatbelt confinement, site isolation, levels, the JIT policy | **Outdated** on one point; **sound** otherwise | **Outdated:** the browser composites through L3. Safari moved DOM rendering to its GPU process in 2023, Chrome's Viz runs outside the browser, Ladybird moved its compositor in 2026.<br>**Sound:** O1 (browser-compiled profiles, as every engine on macOS confines content); J1–J7 (WebKit ships content-process variants without the JIT entitlement, as ADR-0004 observed).<br>**Caveats:** renderers' IOKit access, where WebKit is stricter than Chrome; ad-hoc signing's effect on privacy permissions | RV1-02, RV1-09, RV1-17 |
| [0005](../../adr/0005-ipc-transport-and-interfaces.md) | Mach messages, capabilities, the IDL, `Untrusted<T>` | **Sound, with a caveat** | Generated decoders that check every length and enum, typed violations, and audit tokens follow Chromium's IPC guidance. The caveat: what single-process mode exercises of IP14 and IR7 | RV1-07 |
| [0006](../../adr/0006-event-loop-and-threading.md) | One wait per thread; a core thread; a thread and heap per agent | **Sound, with a caveat** | EL1–EL3 and EL24 are strong. EL10 ("GC heaps never cross threads") is right for mutators, but silently rules out concurrent marking, which Oilpan combines with thread-local heaps | RV1-04, RV1-11 |
| [0007](../../adr/0007-frame-tree-pipeline-and-input.md) | The browser-owned frame tree, the pipeline, invalidation, input | **Sound; contradictory** in staging | Skeleton replicas and remote proxies follow Chromium's design and WebKit's caution about side channels. Per-frame display lists embedded by the compositor follow Viz's surfaces. **Contradictory:** when cross-site frames become separate agents | RV1-03 |
| [0008](../../adr/0008-script-agents-and-heap.md) | Agents and heaps; C1–C5; the collection plan | **Sound** on C1–C3 and C5; **over-optimistic** on C4 | `SliceStack` with no `unsafe` is a real result (S3). C4's plan (idle-time incremental marking, full compaction, no concurrency) can't meet PF14/PF15 on application heaps | RV1-04, RV1-06 |
| [0009](../../adr/0009-embedder-api-and-shell.md) | The shell as an embedder; a pure-Rust shell with objc2 | **Sound** | De-risked by precedent: winit implements `NSTextInputClient` through objc2's `define_class!` (`winit-appkit/src/view.rs`, read 2026-10-21). `NSAccessibility` remains to validate (EM11) | — |
| [0010](../../adr/0010-dependency-and-supply-chain-policy.md) | The dependency and supply-chain policy | **Sound** in method; **over-strict** in application | cargo-vet with imported audits, Google's soundness grades and Chromium's groups are current practice. Applied to old grades without Axiom's own audits (which DP15 allows), it forces rewrites that add logic risk | RV1-08 |
| [0011](../../adr/0011-text-fonts-and-unicode.md) | ICU4X; Axiom's encodings; CoreText for system fonts; HarfRust; a Rust path for web fonts | **Sound, with a caveat** | Shaping with HarfBuzz's algorithm on every font matches Chromium and Firefox. Keeping web fonts from C parsers goes beyond every shipping engine on macOS. The caveats: metrics against ink for system fonts; `-webkit-font-smoothing`; optical sizing; calibration | RV1-10, RV1-08 (encodings) |
| [0012](../../adr/0012-graphics-images-and-media.md) | Metal directly; CoreAnimation layers with damaged tiles; safe decoders; AVIF via ImageIO in the media process | **Sound** on presentation; **outdated** on placement | Retained CoreAnimation layers drawn by damage follow Firefox's measured power savings, and S6 confirms them. The compositor's process placement is RV1-02. IOSurfaces in renderers, RV1-09. JPEG's decoder is unmaintained, RV1-22 | RV1-02, RV1-09, RV1-22 |
| [0013](../../adr/0013-networking-storage-and-workers.md) | Protocols as state machines on one thread; rustls with the system's verifier; partitioning; storage in the network process | **Sound, with a stale fact** | rustls with Security's verification off the I/O thread is well measured (S7). Storage in the network process is WebKit's model, and SG10 states its exposure. **Stale:** rustls has client-side ECH | RV1-20 |
| [0014](../../adr/0014-accessibility.md) | Renderer-pushed trees; the browser's cache; text markers; Axiom's own bridge | **Sound** (not reviewed in depth) | Round 2 scope | — |
| [0015](../../adr/0015-html-parsing-and-the-dom.md) | Axiom's parsers; the node arena; one mutation path | **Sound** | Current with the standards: declarative shadow roots, `moveBefore`, scoped registries, sanitizing while parsing for `setHTML`, a base-appearance `<select>` | — |
| [0016](../../adr/0016-style-layout-and-paint.md) | Axiom's CSS engine; container queries in nested lifecycles; `:visited` by partitioned history; tables approximate | **Sound** (algorithms not reviewed in depth) | LayoutNG's immutable fragments and a 1/64-pixel unit match Blink. Partitioned `:visited` matches what Chrome shipped in Chrome 136 (2025-04; Chrome for Developers, "Making :visited more private", read 2026-10-21). Two properties are missing from text: RV1-10 | RV1-10 |
| [0017](../../adr/0017-javascript-engine-and-bindings.md) | The engine entirely Axiom's own; heap-free compiled scripts; a register machine; Axiom's WebIDL compiler | **Sound** in architecture; **over-optimistic** in scope and timing | Heap-free compiled scripts (SpiderMonkey's Stencil), a register machine with an accumulator (Ignition), and caches as data (Ladybird's interpreter) are current. Over-optimistic: rebuilding Temporal and BigInt; 8-byte heap slots; the WebAssembly and JIT decisions at M8 | RV1-06, RV1-08, RV1-13 |
| [0018](../../adr/0018-corpus-standards-and-testing.md) | The corpus, the standards baseline, testing | **Sound** in method; **contradictory** for M1 | Pinned baselines, expectation files and the shipping median as a target are current practice. The harness for M1 needs script (RV1-01). The hand-measured pages' features are unmeasured (RV1-12). TSan carries too much weight (RV1-11). No compatibility policy (RV1-14) | RV1-01, RV1-11, RV1-12, RV1-14 |
| [0019](../../adr/0019-performance-methodology-and-budgets.md) | The method (PF1–PF5); B1–B7; the ceilings | **Sound** in method; **over-optimistic** for feeds | PF1's conditions are exemplary. The content cap and the pause budgets assume outcomes the design doesn't yet support | RV1-04, RV1-05 |
| [0020](../../adr/0020-roadmap-and-security-levels.md) | M1–M9 and L0–L3 | **Sound** in principle; **outdated** in one ordering | Security before attack surface is right. The GPU process after L3 is outdated. Corpus targets precede capabilities. The WebAssembly and JIT decisions are late | RV1-02, RV1-12, RV1-13 |
| [0021](../../adr/0021-milestone-1.md) | Milestone 1's scope and criteria | **Contradictory** (MA4 against MA7); **over-optimistic** in breadth | The CSS target and its harness need script. The milestone carries every risk of one process for its whole length | RV1-01, RV1-07, RV1-11 |

## 3. The handover's assumptions

From the [handover](../../handover-phase-0.md#8-assumptions) §8.

| Assumption | Verdict | Why | Finding |
|---|---|---|---|
| One human developer, with an assistant; no external audit | Accepted as a constraint | Its consequences are under-planned: scope, diagnostics, the build | RV1-08, RV1-15, RV1-18 |
| No deadline; work paced by checkpoints | Sound | — | — |
| This Mac is the performance floor | Sound | — | — |
| Custom Seatbelt profiles keep working | Accepted risk | Every engine on macOS shares it; V1 after each macOS update (TS16) | — |
| Mach messages between ad-hoc-signed helpers | Sound (S4) | Keep it valid under a stable signing identity | RV1-17 |
| The interfaces hold from L0 to L3 without a rewrite | **Over-optimistic** in three places | The compositor's move after L3; the agents' staging; single-process fidelity | RV1-02, RV1-03, RV1-07 |
| Skeleton replicas suffice for synchronous script | Sound | Chromium's proxies replicate as much and no more (inference) | — |
| Validating display lists in the browser is cheap at display rate | **Untested** | Measure in spike S10, in whichever process validates | RV1-02 |
| Safe-Rust decoders keep up with display-list traffic | **Untested** | As above | RV1-02 |
| A thread per agent stays cheap at B5's counts | Plausible | Recount with out-of-process frames | RV1-05 |
| Most rooting traffic comes from builtins | **Confirmed** (S3) | — | — |
| `u32` handles per arena suffice | Sound | Four billion per arena | — |
| An interpreter with inline caches meets the first milestones' budgets | **Over-optimistic** for M4–M6 | Script-heavy feeds arrive before the JIT decision | RV1-13 |
| HarfRust keeps tracking HarfBuzz; ICU4X's data fits | Sound | Chromium vendors both (text-and-fonts §3) | — |
| objc2-defined classes implement `NSTextInputClient` and `NSAccessibility` | Half confirmed | winit implements `NSTextInputClient` through objc2; `NSAccessibility` untested | — |
| Chrome's figures of 2026-10-20 represent the corpus; the ceilings are frozen | **Over-optimistic** for feeds | An uncontrolled workload, with a wide spread | RV1-05 |
| wpt.fyi keeps publishing aligned runs | Sound | — | — |
| The sites' terms stay as read | Sound, watched | — | — |
| Partitioning third-party state is compatible with the corpus | Sound | Safari blocks third-party cookies by default (background) | — |
| The owner can name the sensitive sites | Sound | — | — |
| A nightly with a working ThreadSanitizer exists by M1 | **Unknown** | Remove the dependence instead | RV1-11 |
| Published audits stay importable | Sound | — | — |

## 4. What holds against current practice

These decisions survive the comparison, with their evidence. Round 1 recommends not reopening them.

1. **One traced heap for JavaScript and the DOM,** with Pattern B and `SliceStack`. There are no wrappers and no cycle collector, and the `gc` crate has no `unsafe` (ADR-0003, ADR-0008; S1, S3). Blink keeps the DOM in Oilpan's heap and script objects in V8's, traced together across wrappers (background); Axiom starts with one heap.
2. **Node identity as index plus generation** (HD11). Stale references outside the heap are detected rather than misread.
3. **IPC as generated, total decoders,** with `Untrusted<T>` and typed violations (IP8–IP12). It is Chromium's guidance ("never use `CHECK()`" on message content) made structural.
4. **The browser-owned frame tree, skeleton replicas and browser-routed operations** (FT1–FT6), which make out-of-process frames a change of process selection.
5. **Browser-compiled Seatbelt profiles,** with `(deny dynamic-code-generation)` explicit (S4), and the JIT confined to one executable (J3, J5).
6. **Damage-limited tiles in retained CoreAnimation layers** (GR2, GR7), which S6 measured.
7. **rustls, with Security verifying certificates in the worker pool** (NW6). S7 measured it at 2.4–3.3 times fewer instructions per handshake than Network framework.
8. **Partitioned connections, caches, cookies and `:visited`** (NW8–NW11, ST10). Chrome shipped partitioned `:visited` in 2025.
9. **Web fonts away from C parsers** (TX7): stricter than any shipping engine on macOS.
10. **A current DOM:** declarative shadow roots, `moveBefore`, scoped custom-element registries, sanitizing during parsing (HD8, HD15, HD24, HD25).
11. **The method:** PF1's measurement conditions, dated sources, labelled evidence, owner gates. Round 1 found both of its target defects by recounting with the project's own tools and data. That is a property of the method, not a failure of it.

## 5. The handover's thirty flagged areas

Where Round 1 answers each item of the [handover](../../handover-phase-0.md#11-areas-flagged-for-critical-review) §11.

| # | Area | Round 1's answer |
|---|---|---|
| 1 | What Axiom builds itself | RV1-08: reclassify Temporal, BigInt and the encodings; keep the engine's core |
| 2 | Milestone 1's breadth | RV1-07 (processes at M1), RV1-01 (an interim CSS target); a headless-first split is left to the owner as part of RV1-07's options |
| 3 | The order of milestones | RV1-02 (the GPU process before L3), RV1-12 (targets after their capabilities) |
| 4 | WebAssembly at M8's gate | RV1-13: decide its architecture before M3 |
| 5 | The sandbox on undocumented functions | Accepted risk, shared by every engine on macOS; V1 after each update. No finding |
| 6 | The unsandboxed browser composites | RV1-02 (Critical) |
| 7 | Live loads at L0 | RV1-07: real processes at M1 make M1's live loads sandboxed |
| 8 | SI3 until M6 | Acceptable for non-sensitive L2 with SI3's rules, including the Storage Access API rule; RV1-03 fixes the agents' staging; RV1-19 adds pixel stealing |
| 9 | The JIT policy's endgame | RV1-13: decide at M5's gate, on M3–M4's measurements |
| 10 | Passkeys | RV1-21 |
| 11 | Staying patched | RV1-17 (rebuilds shouldn't cost permissions); the routine (TS15) stands |
| 12 | The ceilings' basis | RV1-05 |
| 13 | B6 with out-of-process frames | RV1-05 (re-derive B6 with them counted) |
| 14 | The network process holds storage | WebKit's model; SG10 states the exposure. No finding; B4's idle-only bound is noted under RV1-05's re-derivation |
| 15 | Stop-the-world pauses | RV1-04 |
| 16 | Frame budgets from a synthetic page | Round 2: with spike S10's measurements on real display lists |
| 17 | Young crates on the critical path | `vello_cpu` and `smallvec` are already gated by grading at M1; `jpeg-decoder`, RV1-22; HarfRust, which Chromium vendors (sound) |
| 18 | WebGPU's implementation | Round 2, with RV1-02's GPU process design |
| 19 | Rust's own features | RV1-11 (sanitizers); `become` is rightly optional (JE6) |
| 20 | Network privacy | RV1-20 (ECH is available now) |
| 21 | Accessibility through Axiom's own bridge | Sound; Round 2 in depth |
| 22 | Unicode's version | Minor: ICU4X's release cadence decides; no finding |
| 23 | One Mac, varying conditions | The method's labels already carry it; no finding |
| 24 | A tool error found late | RV1-01: a second target defect found by recounting; spot-check tool-derived targets |
| 25 | Rules resting on inference | RV1-02 (spike S10), RV1-07 |
| 26 | No external review | This review is the first; the bar for L3 stays the threat model's own |
| 27 | WebRTC | RV1-22: not planned until a corpus page needs it |
| 28 | PDF viewer, developer tools, printing | RV1-22 (PDF in Preview), RV1-15 (diagnostics) |
| 29 | Daily features | RV1-22 |
| 30 | Recovery and diagnosis | RV1-15, RV1-16 |
