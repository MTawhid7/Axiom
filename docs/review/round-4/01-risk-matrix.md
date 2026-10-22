# Review Round 4: Risk Matrix

| | |
|---|---|
| **Status** | **Round 4, delivered** (2026-10-22), for the owner's dispositions. This page summarizes; the [findings](06-findings.md) carry the evidence |
| **Question** | Do Round 3's claims hold when tested? Does the baseline build what serves few uses, write what mature software already does, and validate efficiency as a first-class requirement? What is wrong in the areas no round reviewed? Is the stopping rule met? |
| **See also** | [Verification of Round 3](02-verification.md) · [Reuse and scope](03-reuse-and-scope.md) · [Efficiency](04-efficiency.md) · [Unreviewed areas](05-unreviewed-areas.md) · [Findings](06-findings.md) · [Actions](07-actions.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-22 |

## 1. Summary

**17 findings: 1 Critical, 1 High, 7 Medium, 8 Low.** Severities are defined in the [review process](../README.md#3-findings-and-severities).

**Round 3's verdict was tested, not taken** ([verification](02-verification.md)). Of its 18 claims, numbered RV3-01 to RV3-18:
- **4 confirmed:** the tool's unread references (with no effect on the count), the hang detection that waits for input, the watchdog that watches the wrong thread, and encodings as scope creep;
- **7 partly confirmed:** the IOSurface lookups, the port-table leak, the reuse check, the 97% target, concurrency, in-memory queues and M1's staging, each right in part and wrong in its numbers, precedents or remedy;
- **5 refuted:** "ghost memory" (no re-attribution happens, measured by spike S16, though a different flaw does), antialiasing, pseudo-classes, the ancestor bit and the rendezvous's precedent;
- **1 already covered:** fonts before the screenshot;
- **and its verdict, "Blocked", agreed with for other reasons.**

**The two findings that block the most** are ones Round 3 didn't make:
- **[RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build), Critical:** 5,423 of the 5,652 tests in M1's CSS target are XML documents or reach one, and the XML parser is staged at M4. As written, M1's gate can't pass.
- **[RV4-02](06-findings.md#rv4-02-the-dependency-policy-ranks-mature-rust-below-c), High:** the dependency policy admits C under conditions but never `ub-risk-3` Rust, so `smallvec`'s grade could bar M1's text stack, and Axiom writes codecs that mature crates provide.

**The owner's three themes:**
- **Scope by value** ([reuse and scope](03-reuse-and-scope.md) §5): add the XML parser to M1; move HTTP/2, the legacy encodings, text entry into pages and the preload scanner to the milestones where they first pay (RV4-08); make CSS 2.1 an area M1 starts, at ≥ 90%, and M3 completes (RV4-09).
- **Reuse** (§1–§4 there): Axiom already reuses most infrastructure; with DP8 fixed, `smallvec`, `encoding_rs` and `zune-jpeg` (without its SIMD code, where it has no `unsafe`) can join, and the core stays Axiom's.
- **Efficiency first** ([efficiency](04-efficiency.md)): it has been a goal since 2026-10-16 and shapes the architecture; what it lacks is validation between gates and budgets for start-up, CPU per load, disk, writes, background activity and memory pressure (RV4-06). S16 showed instructions retired vary 0.03–0.15% run to run, enough to check every checkpoint.

**The areas no round reviewed are sound in their architecture** ([unreviewed areas](05-unreviewed-areas.md)): the requirements' trace is complete; S1–S4's methods support their ADRs; the JavaScript engine's details hold. Their findings are the XML gap, stale text and missing couplings.

**Eleven findings block Milestone 1's plan,** one Critical, one High, six Medium and three Low. None needs a new spike; RV4-13 asks to re-run three existing ones.

**The stopping rule isn't met** (§4).

## 2. The matrix

Ranked by severity, then by the milestone each blocks.

| ID | Severity | Area | Finding | Blocks |
|---|---|---|---|---|
| [RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build) | **Critical** | Conformance, HTML | 5,423 of MA4's 5,652 tests need an XML parser, which is staged at M4; at most 229 could pass | M1 |
| [RV4-02](06-findings.md#rv4-02-the-dependency-policy-ranks-mature-rust-below-c) | **High** | Dependencies | DP8 bars `ub-risk-3` Rust everywhere while DP10 admits C in sandboxed helpers; `smallvec`'s grade gates HarfRust and ICU4X; `encoding_rs` and `zune-jpeg` are barred | M1 |
| [RV4-03](06-findings.md#rv4-03-a-hung-event-loop-goes-unnoticed) | Medium | Event loop, IPC, storage | Hangs are detected only through unacknowledged input; a looping holder of a Web Lock, or a stalled storage thread, is never noticed; PF21 and IP13 name the wrong thread | M1 |
| [RV4-04](06-findings.md#rv4-04-a-dead-renderers-memory-charged-to-no-one) | Medium | Graphics, performance | A dead renderer's images stay resident, charged to no process, until the compositor unmaps them; a CPU read charges them twice (S16-Q1) | M1 |
| [RV4-05](06-findings.md#rv4-05-rights-in-messages-that-are-dropped-rejected-or-returned) | Medium | IPC | Rights in dropped, rejected or returned messages leak (1,000 messages, 1,000 entries, S16-Q2); at 305,834 entries the kernel ends the process | M1 |
| [RV4-06](06-findings.md#rv4-06-efficiency-is-checked-only-at-gates) | Medium | Performance, testing | Budgets are tested only at gates; no budget for start-up, CPU per load, disk, writes, background activity or memory pressure | M1 |
| [RV4-08](06-findings.md#rv4-08-m1s-scope-against-value) | Medium | Program | M1 builds HTTP/2, every legacy encoding and text entry into pages, which none of its criteria use; the preload scanner waits until M4 | M1 |
| [RV4-09](06-findings.md#rv4-09-m1s-css-target-asks-for-the-long-tail-first) | Medium | Conformance | ≥ 97% asks M1 for CSS 2.1's long tail, beyond Servo's 93.7% and Ladybird's 91.4% | M1 |
| [RV4-07](06-findings.md#rv4-07-what-the-browser-takes-from-the-gpu-processs-surfaces) | Medium | Graphics, security | The browser uses the GPU process's surfaces without validating them, and reads the values attached to them | M2 |
| [RV4-11](06-findings.md#rv4-11-channels-in-one-process-and-stress-in-many) | Low | IPC, testing | In-memory channels needn't behave as Mach's; no test varies timing between real processes | M1 |
| [RV4-13](06-findings.md#rv4-13-spikes-measured-on-an-earlier-macos-build) | Low | Method | S5–S7 measured macOS 26B5112a; TS16's re-checks haven't run on 26B118 | M1 |
| [RV4-14](06-findings.md#rv4-14-stale-statements-and-couplings-the-decision-map-lacks) | Low | Program | Stale requirements and open questions; four couplings missing from the decision map | M1 |
| [RV4-10](06-findings.md#rv4-10-the-tile-protocols-writer-side) | Low | Graphics | GR12 lacks the writer's rule: commit a tile once its commands are scheduled | M2 |
| [RV4-12](06-findings.md#rv4-12-the-rendezvous-checks-a-pid-not-a-process) | Low | IPC, security | IP4 pins a PID, not its version, and accepts any number of hellos | M2 |
| [RV4-16](06-findings.md#rv4-16-child-frames-geometry-after-the-gpu-process) | Low | Invalidation | IV13 sends child frames' rectangles in display lists, which from M2 go to the GPU process | M2 |
| [RV4-17](06-findings.md#rv4-17-m2s-cache-and-cookie-jar-wait-on-a-storage-engine-chosen-at-m3) | Low | Networking, storage | M2 builds the cache and cookie jar on a storage engine chosen at M3 | M2 |
| [RV4-15](06-findings.md#rv4-15-an-advisory-in-the-spikes-lock) | Low | Dependencies | `rustls-webpki` 0.103.15 in the spikes' lock is affected by RUSTSEC-2026-0382 | — |

**By area and severity:**

| Area | Critical | High | Medium | Low |
|---|---|---|---|---|
| Conformance and scope | 1 (RV4-01) | — | 2 (RV4-08, RV4-09) | — |
| Dependencies | — | 1 (RV4-02) | — | 1 (RV4-15) |
| Robustness: hangs, memory, rights | — | — | 3 (RV4-03, RV4-04, RV4-05) | — |
| Efficiency | — | — | 1 (RV4-06) | — |
| Graphics and the GPU process | — | — | 1 (RV4-07) | 2 (RV4-10, RV4-16) |
| IPC and testing | — | — | — | 2 (RV4-11, RV4-12) |
| Method and documents | — | — | — | 2 (RV4-13, RV4-14) |
| Networking and storage | — | — | — | 1 (RV4-17) |

## 3. The commission, answered

1. **"Perform one final round of review, as we should not take Gemini's verdict for granted without testing."** Every claim was tested against primary sources (WebKit, Chromium and XNU at named revisions, the SDK), the repository and spike S16's measurements. The verdicts are in §1; the detail, claim by claim, in the [verification](02-verification.md). Round 3 was right that M1 shouldn't start as written, and wrong about most of its reasons.
2. **"Prioritizing the core functionality, the most common use cases, and the features that deliver the greatest value":** [reuse and scope](03-reuse-and-scope.md) §5, RV4-08 and RV4-09. M1 gains the one capability its gate needs and loses four that its criteria don't use.
3. **"Take full advantage of mature, battle-tested software ... evaluate each dependency carefully":** [reuse and scope](03-reuse-and-scope.md) §1–§4, with dated facts for the crates on Axiom's path and RV4-02's policy change. The core (G1) stays Axiom's, as the plan presumed.
4. **"Should we make efficiency a first-class architectural requirement from the very first phase?"** Yes, and it already is, in goals and in design; the strategy that would make it continuous is [efficiency](04-efficiency.md) §4–§8 and RV4-06.
5. **"Everything unreviewed":** [unreviewed areas](05-unreviewed-areas.md).
6. **Spike S16,** authorized for three questions, answered them ([S16 results](../../research/s16-results.md)): Q1 refutes Round 3's "ghost memory" and finds RV4-04; Q2 confirms the leak and measures it (RV4-05); Q3 makes per-checkpoint efficiency checks practical (RV4-06).
7. **"Incorporate the findings, update the documentation, resolve any remaining architectural questions, and establish a clear implementation plan":** the [actions](07-actions.md): checkpoints for the accepted changes, every open question classified with a recommendation, and the shape of `docs/milestone-1-plan.md`.

## 4. The stopping rule

**The rule** ([the review process](../README.md#6-the-stopping-rule) §6): "The cycle ends when a round raises no Critical or High finding, after the previous round's accepted changes have been made and verified."

**Evaluation:**
- **Round 2's accepted changes are made and verified:** CP1–CP6 and their sweep (the [disposition log](../README.md#57-round-2s-checkpoints) §5.7); this round found nothing they left wrong, beyond the stale text in RV4-14.
- **Round 4 raises one Critical and one High finding.** **So the rule isn't met.**

**Recommendation:** the owner disposes of the findings; the accepted changes are made in checkpoints ([actions](07-actions.md) §2); **the last checkpoint verifies every change** and recomputes MA4's figures by an independent count; **then the owner ends the cycle,** as §6 allows, rather than commissioning a Round 5. **Why that is enough** (inference): both blocking findings are bounded changes to documents (a parser moved between milestones, and a policy rule), their verification is mechanical, and no finding questions the architecture's structure.

**What a Critical this late says about the process.** The XML gap was found by checking what a test's file type means to the server that delivers it, a data-level check no round had made; Round 1 counted script, Round 3 asked about references. **One process change follows** (part of RV4-01): every target a tool computes is checked against every capability the milestone lacks, not script alone (`--needs`), before it binds (SB11's independent count, extended).

**Independence.** Round 4 was written by Claude (Opus 5.5), the model family that wrote the baseline and Rounds 1 and 2. Round 3 was independent, by Gemini; this round tested it rather than adopting it, and the owner may weigh both.

## 5. What should be settled before Milestone 1 starts

Each needs the owner's disposition, then a change in documents; the detail is in the [actions](07-actions.md#1-changes-by-milestone).

| Finding | The decision |
|---|---|
| RV4-01 | The XML parser at M1, Axiom's own (or a tokenizer reused, as an amendment of G1) |
| RV4-02 | DP8 amended so that `ub-risk-3` crates enter under DP10's conditions; the burden of proof for supporting infrastructure |
| RV4-03 | Hang detection by task progress; PF21 and IP13 corrected |
| RV4-04 | Release at a renderer's death; no CPU reads of renderer memory; accounting by mapped pages |
| RV4-05 | Ownership of every right in a received or returned message |
| RV4-06 | The three tiers, the ratchet, PF25–PF30 and the efficiency section |
| RV4-08 | HTTP/2, the legacy encodings, text entry and the preload scanner moved; or `encoding_rs` at M1 |
| RV4-09 | M1's CSS target: ≥ 90% with per-directory floors, or 97% kept |
| RV4-11, RV4-13, RV4-14 | Channels that model Mach and a stress run; S5–S7 re-run; the stale text corrected |
