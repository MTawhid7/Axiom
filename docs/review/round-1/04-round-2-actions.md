# Review Round 1: Actions Before Round 2

| | |
|---|---|
| **Status** | **Round 1, awaiting the owner's dispositions** (2026-10-21). Nothing here is authorized: each change follows the owner's disposition of its finding, and each spike needs its own authorization (CLAUDE.md, "Authorization and review gates") |
| **Question** | Which documentation changes must be made before implementation can safely start, which can wait for their milestone, which spikes would settle the open evidence, and what should Round 2 examine? |
| **See also** | [Risk matrix](01-risk-matrix.md) · [Findings](02-findings.md) · [Challenged assumptions](03-challenged-assumptions.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-21 |

Each row names its finding, the change proposed, the documents and rules it touches, and when the change is done. If the owner rejects or defers a finding, its rows fall away, and the disposition log records why.

## 1. Before Milestone 1 starts

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV1-01](02-findings.md#rv1-01-milestone-1s-css-target-and-harness-need-script) | An interim CSS target over the script-free tests, with the full target at M3. A script-free reference-test executor for M1 (modelled on wptrunner's `executorservo.py`), with WebDriver from M3. Test fonts at M1 on the Rust path, or the dependence recorded. The harness run end to end at M1's second checkpoint | [Milestone 1](../../milestone-1.md) MA4, MA7, MA11; [testing](../../testing.md) TS2, TS3; [ADR-0021](../../adr/0021-milestone-1.md)'s history | MA4's target is recomputed at the pinned revision over the script-free tests that `tools/wpt-script-deps.py --list` leaves (`tools/wpt-pass-rates.py` first gains a filter for a list of tests), and TS2 names both executors |
| [RV1-02](02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3) | The end-state GPU/compositor process designed: its topology, where scrolling and animation run, how hit-test data flows; its milestone (M2 recommended, M6 at the latest); B1 split | [ADR-0004](../../adr/0004-process-model-and-sandbox.md) D1; the [process model](../../security/process-model.md) §3, §4, §7 and change record; [graphics](../../architecture/graphics.md) GR10, GR12, GR14; the [rendering pipeline](../../architecture/rendering-pipeline.md) PL8, PL9, PL15, PL16; [input](../../architecture/input-and-accessibility.md) IN1; the [roadmap](../../roadmap.md) MS5 and §7; the [overview](../../architecture/overview.md) §1 and §6 | ADR-0004 and ADR-0012 carry the amendment, with S10's measurements |
| [RV1-03](02-findings.md#rv1-03-cross-site-agents-before-l2-the-documents-disagree) | One rule: separate agents and remote proxies from M1. WB20 in the first script stage. Three staging texts corrected | FT10; the [frame tree](../../architecture/frame-tree-and-navigation.md#5-staging) §5; the overview §6; the roadmap's M5; [bindings](../../architecture/bindings.md#8-staging-and-the-complete-model) §8; SH1 | A search of the documents for "move to separate agents" and "become their own agents" finds nothing |
| [RV1-07](02-findings.md#rv1-07-milestone-1-in-one-process) | The owner's choice: real processes at M1 (recommended), or the minimum fidelity rules: real memory entries in-process, IR7 in-process, injected agent deaths, tests under the profiles | MS4, the roadmap §4.1 and §10; IP2; the process model §9; MA1, MA6, MA10 | The roadmap and Milestone 1 say which, and M2's content is restated |
| [RV1-08](02-findings.md#rv1-08-scope-against-one-developer) | `encoding_rs`'s current release graded by Axiom (DP15), its SIMD feature off; the encodings written only if it fails. The reuse line for Temporal and BigInt decided before M3 (§2) | [Text and fonts](../../architecture/text-and-fonts.md) TX3 and §4; the [register](../../technology/dependency-register.md) | The register records the grade, and TX3 states the outcome |
| [RV1-09](02-findings.md#rv1-09-iosurfaces-in-renderers) | Renderers write pixels only into memory entries; the compositor owns IOSurfaces; renderer profiles deny `iokit-open` explicitly; V1 tests it | The process model §4.1; [images and media](../../architecture/images-and-media.md) IM1; GR10; PL8; the [threat model](../../security/threat-model.md) V1 (through TS14) | The process model's change record has the rule |
| [RV1-10](02-findings.md#rv1-10-the-hybrid-text-models-gaps) | For each system font, one source for layout's metrics and the rasterizer's ink (CoreText's for `sbix` faces), or normalized bitmaps. `-webkit-font-smoothing` honoured on both paths and listed under SB5. Optical sizing, and a rule for `opsz`. An acceptance metric for smoothing emulation, in TS16 | TX8–TX13; the [standards](../../standards.md) SB5; TS16 | TX12 and TX13 state the rules; SB5 lists the property |
| [RV1-11](02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer) | TS11 as an ordered strategy: few primitives; loom and Miri; schedule exploration on EL24; TSan by the first route that works | TS11; MA7, MA10; [ADR-0021](../../adr/0021-milestone-1.md)'s risk row | MA7 requires steps 1–3 and records step 4's route |
| [RV1-15](02-findings.md#rv1-15-diagnostics-for-one-developer) | The headless embedder's text dumps at M1; the crash path at M2; the console at M3 | [Embedder API](../../architecture/embedder-api.md) EM9; MA1, MA11; the roadmap §4.1 | MA1 lists the dumps |
| [RV1-16](02-findings.md#rv1-16-failure-semantics-panics-and-overflow) | `panic = "abort"` per executable; panic-free validators enforced by lint; overflow checks or checked arithmetic in parsers and validators; L0's behaviour | [ADR-0002](../../adr/0002-implementation-language.md)'s history, or the overview; IP11; the workspace profiles, at M1's first checkpoint | The rule is written, and the profiles follow at M1's skeleton |
| [RV1-17](02-findings.md#rv1-17-the-signing-identity) | A stable local signing identity with an explicit designated requirement, checked against S4's probes (spike S15) | ADR-0004 D2; the process model §10; MA10 | The signing script's design names the identity; S15 passes |
| [RV1-18](02-findings.md#rv1-18-build-cost-of-generated-code) | The crate topology for compile time: bindings split by interface group, table-driven glue, split debug info, the linker. Style's generated code measured at M1's first checkpoint | The overview §2; WB22; PF18; MA6 | The overview's crate table shows the split; S13 is planned before M3 |

## 2. Before Milestone 3 fixes the object model

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV1-04](02-findings.md#rv1-04-the-collectors-end-state-against-the-pause-budgets) | The collector's end-state: a nursery; allocation-driven and idle-time incremental marking; parallel marking in pauses; non-moving tenured arenas with selective evacuation. Concurrent marking kept possible, or excluded with evidence. The barrier's semantics and black allocation chosen | [ADR-0008](../../adr/0008-script-agents-and-heap.md) decision 8; [script and heap](../../architecture/script-and-heap.md) SH9, SH10; EL10, EL11; JE29; PF15 | ADR-0008 carries the amendment, with S11's measurements |
| [RV1-06](02-findings.md#rv1-06-the-width-of-heap-slots) | Heap slots of 32 or 64 bits, decided from a memory model and S11 | [The JavaScript engine](../../architecture/javascript-engine.md) JE9, JE10; SH13; the [performance](../../performance.md) §4.1 | JE9 and JE10 state the slot encoding |
| [RV1-08](02-findings.md#rv1-08-scope-against-one-developer) | Temporal's arithmetic and BigInt reclassified as replaceable details behind Axiom's interfaces, subject to DP8; what G1 loses recorded | DP1 (g); the JavaScript engine §4, JE13, JE22; [ADR-0017](../../adr/0017-javascript-engine-and-bindings.md)'s history | The build-or-reuse table states the owner's choice |
| [RV1-13](02-findings.md#rv1-13-webassembly-and-the-jit-decided-late) | OQ-3 decided at the architecture level (own or embedded; WasmGC as arena kinds; frames; tiering under J6). J1's decision moved to M5's gate | [Open questions](../../open-questions.md) OQ-3; JE30, JE31; the roadmap §7 (MS9) | OQ-3's architecture is settled, with S14's measurements |

## 3. Before Milestones 4 to 6

| Finding | Change | Documents and rules | Before |
|---|---|---|---|
| [RV1-05](02-findings.md#rv1-05-the-content-tiers-150-mb-cap) | A normalized workload; a by-hand memory breakdown of Chrome's cost; a gate on slope and plateau, with 150 MB as a stretch figure; zero-copy resources; B6 re-derived with out-of-process frames | The performance §4.1, PF4, PF11; the process model §7; OQ-25; [ADR-0019](../../adr/0019-performance-methodology-and-budgets.md)'s history | M4 |
| [RV1-12](02-findings.md#rv1-12-corpus-targets-before-their-capabilities) | The eight pages' use counters saved by hand; targets re-placed by MS6, with exclusions named; WebSockets (and dedicated workers, if measured) at M4 | The roadmap §4–§5; the [corpus](../../corpus.md) §2 and §10 | M4 |
| [RV1-14](02-findings.md#rv1-14-site-compatibility) | A compatibility section: the user-agent policy, and per-site interventions as data outside the core, under SB5 | The [standards](../../standards.md); the corpus's live checks | M4; the user-agent string before M1's live loads |
| [RV1-19](02-findings.md#rv1-19-rendering-timing-side-channels) | A threat-model entry for pixel stealing; a rule in PL9 on effects over cross-origin child frames; a V4 test | The threat model §7.2 and §9; PL9; TS14 | M5 |
| [RV1-20](02-findings.md#rv1-20-a-stale-fact-rustls-and-ech) | Correct the stale fact; plan ECH with DNS over HTTPS | [Networking](../../architecture/networking.md#10-open-questions) §10 and §8; the [handover](../../handover-phase-0.md) §4.9 | M5's plan |
| [RV1-21](02-findings.md#rv1-21-passkeys-without-the-restricted-entitlement) | Hybrid added to OQ-16's candidates; a feasibility check | OQ-16; the threat model §9 | M6 |
| [RV1-22](02-findings.md#rv1-22-daily-use-gaps-and-an-orphaned-decoder) | PDFs opened in Preview, on the not-planned list; a milestone for each daily feature; WebRTC not planned; JPEG's decoder decided | The standards §5.3; the roadmap §4; IM2; the register | M1 for JPEG; M5 for the rest |

## 4. Proposed spikes

Each needs its own authorization, and covers only the questions it names. Numbering continues from S9. Each would run under PF1's conditions, with its results in `docs/research/`.

| Spike | Questions | Method | Feeds |
|---|---|---|---|
| **S10: the GPU process's hop and zero-copy resources** | S10-Q1: the latency and CPU cost, per frame at 60 Hz, of display-list updates sent to a sandboxed process that validates and rasterizes them, with IOSurfaces returned to the browser's layer tree. S10-Q2: zero-copy sampling of renderer pixels from Mach memory entries (no-copy Metal buffers, linear textures or one blit) against IOSurfaces, in time and memory. S10-Q3: PL9's validation cost on display lists the size of real pages | S6's pipelines and traces, split across two processes under S4's profiles | RV1-02, RV1-09; the handover's §8 assumptions on validation and decoding |
| **S11: the collector's latency and the slot width** | S11-Q1: pauses on a synthetic heap of a million objects, for stop-the-world mark–compact, allocation-driven incremental marking, parallel marking in pauses, and selective evacuation, in safe Rust. S11-Q2: memory and speed of 32-bit heap slots against 64-bit NaN-boxed ones, on heaps shaped like the corpus's | Extends S3's `s3-gc` and `s3-vm` | RV1-04, RV1-06 |
| **S12: a ThreadSanitizer route** | S12-Q1: does the newest nightly's runtime start on macOS 27? S12-Q2: does `-Z external-clangrt` with the Command Line Tools' runtime report S9's planted race? | S9's `s9-race` and `tools/s9-measure.sh tsan` | RV1-11 |
| **S13: the build cost of generated bindings** | S13-Q1: T1–T4 for bindings generated from webref's IDL with stub implementations, as one crate and split by interface group | A throwaway generator under `spikes/s13/`, with `tools/measure-build.sh` | RV1-18 |
| **S14: WebAssembly, interpreted and compiled** | S14-Q1: the gap between a Rust Wasm interpreter (`wasmi`) and Cranelift (Wasmtime) on a compute kernel compiled to Wasm, here. S14-Q2: what each supports of WasmGC | A kernel built with the Command Line Tools' clang, run in both | RV1-13 |
| **S15: a stable signing identity** | S15-Q1: under a self-signed identity with an explicit designated requirement, do S4's eighteen probes, `allow-jit` with `single-jit`, and the custom profiles behave as under ad-hoc signing? S15-Q2: do privacy decisions persist across rebuilds? | S4's helpers and probes. Creating the identity changes the owner's keychain, so it is the owner's act, not a script's | RV1-17 |

**A census addendum, by hand** (no automation in any page, as CO6 requires):
- **Use counters:** `chrome://histograms/Blink.UseCounter` saved after each of the eight hand-measured pages.
- **Memory breakdowns:** a memory-infra trace for Google Search, Instagram and Reddit, under RV1-05's normalized workload.

It feeds RV1-05 and RV1-12.

## 5. How accepted findings become changes

1. **The owner's disposition goes in the [disposition log](../README.md#5-the-disposition-log),** in the owner's words, with the date.
2. **Accepted changes are made in checkpoints on `architecture-review`,** grouped by document, named `docs(review-r1-cpN): …`, each with the five checks green.
3. **An accepted ADR's change is an amendment:** its decision history records the finding's ID and the owner's words. The process model's change record does the same.
4. **Changes that need evidence** (RV1-02, RV1-04, RV1-06, RV1-13, RV1-17) wait for their spike, whose authorization is a separate decision.
5. **Round 2 checks each change** against its finding.

## 6. Round 2's scope

1. **The amendments:**
   - each accepted finding's change is made as proposed, or as the owner amended it;
   - no amendment contradicts another document; the checks' searches and the link checker run clean.
2. **The areas Round 1 read but didn't review in depth:**
   - accessibility (ADR-0014, AX1–AX12);
   - storage and workers (SW1–SW13);
   - the style system's cascade and invalidation (ST1–ST36) and the layout algorithms (LY1–LY32);
   - the embedder API (EM1–EM12);
   - IPC's details (IP13–IP22);
   - the requirements' trace (RQ1–RQ66);
   - the methods of spikes S1–S4;
   - the decision map's couplings.
3. **The new evidence:** S10–S15's results, if authorized, and the census addendum.
4. **Dated facts:** sources older than six months at Round 2's date read again, the crates on the critical path first: HarfRust, `read-fonts` and `skrifa`, `vello_cpu`, `quinn-proto`, `redb`, `smallvec`.
5. **The stopping rule** ([the review process](../README.md#6-the-stopping-rule)): Round 2 ends the cycle if it raises no Critical or High finding.
