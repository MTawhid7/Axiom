# Review Round 1: Risk Matrix

| | |
|---|---|
| **Status** | **Round 1, disposed** (2026-10-21): the owner accepted every finding ([the disposition log](../README.md#5-the-disposition-log)). This page summarizes; the [findings](02-findings.md) carry the evidence |
| **Question** | Which flaws, gaps and outdated assumptions in the Phase Zero baseline matter most, and which must be settled before Milestone 1 starts? |
| **See also** | [Findings](02-findings.md) · [Challenged assumptions](03-challenged-assumptions.md) · [Actions for Round 2](04-round-2-actions.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-21 |

## 1. Summary

**22 findings:** 2 Critical, 6 High, 10 Medium, 4 Low. Severities are defined in the [review process](../README.md#3-findings-and-severities).

**The baseline is unusually careful, and most of it holds.** What fails falls into four groups:
- **Two defects in accepted documents.** Milestone 1's conformance gate can't pass as written ([RV1-01](02-findings.md#rv1-01-milestone-1s-css-target-and-harness-need-script)). The documents also disagree about when cross-site frames become separate agents ([RV1-03](02-findings.md#rv1-03-cross-site-agents-before-l2-the-documents-disagree)).
- **One security placement against current practice.** The unsandboxed browser rasterizes and composites renderer output through the sensitive-use gate, and the GPU process's final topology is staged rather than designed ([RV1-02](02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3)).
- **Decisions fixed early but measured late.** The collector's end-state, the width of heap slots, the content tier's memory cap, WebAssembly and the JIT ([RV1-04](02-findings.md#rv1-04-the-collectors-end-state-against-the-pause-budgets), [RV1-05](02-findings.md#rv1-05-the-content-tiers-150-mb-cap), [RV1-06](02-findings.md#rv1-06-the-width-of-heap-slots), [RV1-13](02-findings.md#rv1-13-webassembly-and-the-jit-decided-late)).
- **Feasibility for one developer.** Scope, diagnostics, the build on 8 GB, and Milestone 1's single process ([RV1-07](02-findings.md#rv1-07-milestone-1-in-one-process), [RV1-08](02-findings.md#rv1-08-scope-against-one-developer), [RV1-15](02-findings.md#rv1-15-diagnostics-for-one-developer), [RV1-18](02-findings.md#rv1-18-build-cost-of-generated-code)).

**Twelve findings block Milestone 1's plan,** in that the owner's disposition, and any amendment it calls for, should come first. Most are document changes. Three rest on spikes the owner would authorize separately ([actions](04-round-2-actions.md#4-proposed-spikes)):
- S10, the GPU process's hop (RV1-02, RV1-09);
- S12, a ThreadSanitizer route (RV1-11);
- S15, a stable signing identity (RV1-17).

Before M3, S11 measures the collector and the slot width, and S13 the bindings' build cost.

## 2. The matrix

Ranked by severity, then by the milestone each blocks.

| ID | Severity | Area | Focus | Finding | Blocks |
|---|---|---|---|---|---|
| [RV1-01](02-findings.md#rv1-01-milestone-1s-css-target-and-harness-need-script) | **Critical** | Standards | 5 | `css/CSS2` ≥ 97% can't be met without script: 196 of the 5,848 tests use script, so the ceiling is 96.65%. The WebDriver harness MA4 names runs script in the page too | M1 |
| [RV1-02](02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3) | **Critical** | Security, architecture | 1, 4 | The unsandboxed browser validates, rasterizes and composites renderer output through L3. Safari (since 2023), Chrome and Ladybird (2026) do this in a separate process. GR14's later move is undesigned | M1 |
| [RV1-03](02-findings.md#rv1-03-cross-site-agents-before-l2-the-documents-disagree) | **High** | Architecture | 1 | FT10 and FT §5's L0 row make cross-site frames separate agents from the start; the overview §6, FT §5's L2 row, roadmap M5 and bindings §8 say from L2. M3 needs cross-site frames | M1 |
| [RV1-07](02-findings.md#rv1-07-milestone-1-in-one-process) | **High** | Architecture | 1 | One process at M1 leaves shared memory, crash recovery, sandbox-silent failures and per-process budgets untested until M2 | M1 |
| [RV1-08](02-findings.md#rv1-08-scope-against-one-developer) | **High** | Program | — | DP1 (g) and DP8 rebuild Temporal, BigInt and every WHATWG encoding that graded pure-Rust work already provides; `encoding_rs` was barred on a 2023-era grade | M1, M3 |
| [RV1-04](02-findings.md#rv1-04-the-collectors-end-state-against-the-pause-budgets) | **High** | Performance | 4 | Marking only in idle time, full-heap compaction and no concurrent marking can't hold PF14/PF15 on heaps of millions of objects. Fix the end-state before M3 | M3 |
| [RV1-06](02-findings.md#rv1-06-the-width-of-heap-slots) | **High** | Performance | 3, 4 | 8-byte NaN-boxed heap slots, against V8's 4-byte compressed ones (up to 43% of the heap), are fixed at M3 and measured only at M8 | M3 |
| [RV1-05](02-findings.md#rv1-05-the-content-tiers-150-mb-cap) | **High** | Performance | 3 | The 150 MB cap rests on an uncontrolled workload (Reddit: 495–1,631 MB), and on page-owned retention no engine can shrink. Out-of-process frames and doubly held images are charged to the page | M4 |
| [RV1-09](02-findings.md#rv1-09-iosurfaces-in-renderers) | Medium | Security | — | Renderers map IOSurfaces, so they open an IOKit user client, which WebKit's WebContent process may no longer do. CPU raster needs only memory entries | M1 |
| [RV1-10](02-findings.md#rv1-10-the-hybrid-text-models-gaps) | Medium | Text | 2 | Tenable, but: CoreText's emoji ink against font-data advances; `-webkit-font-smoothing` (used by three corpus pages); optical sizing; no metric for smoothing emulation | M1 |
| [RV1-11](02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer) | Medium | Toolchain | 5 | TSan matters least for safe Rust. Lead with Miri, loom and schedule exploration on EL24; reach TSan through a newer nightly, `-Z external-clangrt` or Linux | M1 |
| [RV1-15](02-findings.md#rv1-15-diagnostics-for-one-developer) | Medium | Program | — | No dumps, console, network log or crash path until "later"; one developer needs them from M1 | M1 |
| [RV1-16](02-findings.md#rv1-16-failure-semantics-panics-and-overflow) | Medium | Robustness | — | No panic strategy (unwind is the default) and no overflow policy (release builds wrap) | M1 |
| [RV1-17](02-findings.md#rv1-17-the-signing-identity) | Medium | Platform | — | Ad-hoc signatures change with every build, so macOS forgets privacy and Keychain decisions (TN3127) | M1 |
| [RV1-18](02-findings.md#rv1-18-build-cost-of-generated-code) | Medium | Toolchain | — | T1–T4 are untested for generated code. Servo's 520,000 generated lines dominated its build. 8 GB | M1, M3 |
| [RV1-13](02-findings.md#rv1-13-webassembly-and-the-jit-decided-late) | Medium | Architecture | 4 | WebAssembly's heap integration and the JIT shape M3's object model, yet are decided at M8; M4–M6's script-heavy pages run on the interpreter | M3 |
| [RV1-12](02-findings.md#rv1-12-corpus-targets-before-their-capabilities) | Medium | Roadmap | — | Twitch's chat needs WebSockets (M8) at M7; Instagram's video (M7) at M4; the content pages' features are unmeasured | M4 |
| [RV1-14](02-findings.md#rv1-14-site-compatibility) | Medium | Standards | — | No user-agent, Client Hints or site-intervention policy; WebKit and Firefox both keep per-site interventions | M4 |
| [RV1-19](02-findings.md#rv1-19-rendering-timing-side-channels) | Low | Security | 4 | Pixel stealing across frames (GPU.zip) is outside the threat model, while the compositor applies parent effects to child frames | M5 |
| [RV1-20](02-findings.md#rv1-20-a-stale-fact-rustls-and-ech) | Low | Networking | — | rustls has had experimental client-side ECH since 2024-06; two documents say it doesn't | M5 |
| [RV1-21](02-findings.md#rv1-21-passkeys-without-the-restricted-entitlement) | Low | Security | — | Passkeys: USB HID and the hybrid transport need no restricted entitlement; Chromium implements hybrid itself | M6 |
| [RV1-22](02-findings.md#rv1-22-daily-use-gaps-and-an-orphaned-decoder) | Low | Product | — | PDFs, downloads, find in page, zoom and session restore have no milestone; `jpeg-decoder` is in maintenance mode | M6 |

**By area and severity:**

| Area | Critical | High | Medium | Low |
|---|---|---|---|---|
| Architecture | 1 (RV1-02, with security) | 2 | 1 | — |
| Security | (RV1-02) | — | 1 | 2 |
| Performance | — | 3 | — | — |
| Standards and roadmap | 1 | — | 2 | — |
| Text and platform | — | — | 2 | — |
| Toolchain and robustness | — | — | 3 | — |
| Program and product | — | 1 | 1 | 1 |
| Networking | — | — | — | 1 |

## 3. The five focus areas, answered

1. **Staging: will L0's abstractions need rewriting at M2 and M5?**
   - **Mostly no.** IP2 (messages always encoded), FT6 (every operation between agents goes through the browser), FT11 (endpoints bound at commit) and FT12 (a display list per frame) are the right defences, and they answer the lesson of Firefox's Electrolysis and Fission.
   - **Three exceptions:**
     - the documents disagree on cross-site agents ([RV1-03](02-findings.md#rv1-03-cross-site-agents-before-l2-the-documents-disagree));
     - the compositor moves processes after L3, on a topology no document designs ([RV1-02](02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3));
     - single-process mode skips shared memory, crashes and the sandbox's silent failures ([RV1-07](02-findings.md#rv1-07-milestone-1-in-one-process)).
2. **Text: is CoreText beside HarfRust and `skrifa` tenable?**
   - **Yes.** Shaping is unified, and at 2× the rasterizers agree to about 1 level in 255.
   - **The real mismatches:** CoreText's emoji ink against layout's font-data advances, which Chrome avoids by taking CoreText's advances; `-webkit-font-smoothing`; optical sizing; an unmeasured smoothing emulation. The atlases multiply with out-of-process frames, but aren't doubled by format ([RV1-10](02-findings.md#rv1-10-the-hybrid-text-models-gaps)).
3. **The 150 MB content cap.**
   - **For documents,** "half of Chrome" is plausible, because Chrome's own caches dominate its cost there.
   - **For signed-in feeds,** 150 MB is about a tenth of Chrome's renderer memory, at an uncontrolled scroll depth, with page-owned retention and out-of-process frames charged to it. As a gate it's ill-posed; as a slope with a plateau, measured on a normalized workload, it becomes testable ([RV1-05](02-findings.md#rv1-05-the-content-tiers-150-mb-cap), [RV1-06](02-findings.md#rv1-06-the-width-of-heap-slots), [RV1-09](02-findings.md#rv1-09-iosurfaces-in-renderers)).
4. **The JavaScript engine and the DOM binding layer.**
   - **What holds:**
     - safety: Pattern B, a `gc` crate with no `unsafe`, `SliceStack`;
     - the cycle story: one traced heap, so no cycle collector;
     - HTML's microtask checkpoints (SH4, JE16);
     - ECMAScript 2026 with the 2027 draft's finished proposals.
   - **What doesn't:** the pause story ([RV1-04](02-findings.md#rv1-04-the-collectors-end-state-against-the-pause-budgets)), the slot width ([RV1-06](02-findings.md#rv1-06-the-width-of-heap-slots)), and when WebAssembly and a JIT are decided ([RV1-13](02-findings.md#rv1-13-webassembly-and-the-jit-decided-late)). Rendering-timing attacks across frames are a gap in the security model ([RV1-19](02-findings.md#rv1-19-rendering-timing-side-channels)).
5. **ThreadSanitizer.** Make it the last line, not the first.
   - **First:** Miri's race detector and weak-memory emulation, loom on the primitives, and schedule exploration on the deterministic scheduler.
   - **Then TSan,** for `platform` and C code, by a newer nightly, `-Z external-clangrt` with Apple's working runtime, or Linux ([RV1-11](02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer)).

**Milestone 1's feasibility.**
- **The CSS target** is unreachable as specified ([RV1-01](02-findings.md#rv1-01-milestone-1s-css-target-and-harness-need-script)); an interim target over the 5,652 script-free tests is reachable in principle.
- **For calibration,** Servo reaches 93.7% and Ladybird 91.2% on the same scope ([Milestone 1](../../milestone-1.md) §7).
- **On the breadth,** see RV1-07.

## 4. What should be settled before Milestone 1 starts

Each item needs the owner's disposition, then the change. The detail is in the [actions](04-round-2-actions.md#1-before-milestone-1-starts).

| Finding | The decision |
|---|---|
| RV1-01 | An interim, script-free CSS target, and a script-free harness |
| RV1-02 | The GPU/compositor process topology, and its milestone (M2 recommended) |
| RV1-03 | One rule: cross-site frames are separate agents behind remote proxies from M1 |
| RV1-07 | Real processes at M1, or the minimum fidelity rules |
| RV1-08 | Grading `encoding_rs`, and the reuse line for the engine's library |
| RV1-09 | No IOKit in renderers |
| RV1-10 | One source per system font for metrics and ink; `-webkit-font-smoothing`; optical sizing |
| RV1-11 | TS11's ordered strategy, and MA7's wording |
| RV1-15 | The headless embedder's dumps |
| RV1-16 | The panic and overflow policy |
| RV1-17 | A stable signing identity |
| RV1-18 | The crate topology for compile time |
