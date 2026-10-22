# Review Round 4: Actions

| | |
|---|---|
| **Status** | **Round 4, disposed** (2026-10-22): the owner accepted all seventeen findings as proposed, with the choices the [disposition log](../README.md#59-round-4s-dispositions) records. It stays as written, as the round's record; where the owner's words differ, they govern. The changes are made in the checkpoints of §2 ([the log](../README.md#511-round-4s-checkpoints) §5.11) |
| **Question** | Which changes does each finding need, and before which milestone; how are they made; which questions remain open, and when is each settled; what does Milestone 1's implementation plan look like; and what is the path to the merge? |
| **See also** | [Risk matrix](01-risk-matrix.md) · [Verification of Round 3](02-verification.md) · [Findings](06-findings.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-22 |

The owner's direction for after the round (2026-10-22): "incorporate the findings, update the documentation, resolve any remaining architectural questions, and establish a clear implementation plan." Each row below names its finding, the change, the documents and rules it touches, and when it is done. **No finding needs a new spike;** RV4-13 asks to re-run three existing ones.

## 1. Changes by milestone

### 1.1 Before Milestone 1 starts

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build) | The XML parser for documents at M1; MA4's figures recomputed with `--needs`; SB11's independent count covers every missing capability | MA1, MA2, MA4, MA7, MA11; HD §8; the roadmap's M1 and M4; RQ40; SB11; ADRs 0015, 0021 | MA4's subset and target are recomputed by two independent counts that agree, and MA1 builds what they need |
| [RV4-02](06-findings.md#rv4-02-the-dependency-policy-ranks-mature-rust-below-c) | DP8: `ub-risk-3` under DP10's conditions, with Axiom's audit of each block and a fuzz target; the burden of proof for DP2's list | DP8, DP10, DP4; charter principle 4 if the owner amends it; TX3; IM2; text §4; MA12; ADRs 0010, 0011, 0012 | DP8 and DP10 rank C below every graded Rust crate, and MA12 can decide `smallvec`, encodings and JPEG under them |
| [RV4-03](06-findings.md#rv4-03-a-hung-event-loop-goes-unnoticed) | Hang detection by task age as well as input; PF21 and IP13 corrected; no lock revocation | PF19, PF21; EL §4.7; IP13; SW10's note; MA5, MA7; ADRs 0006, 0019 | PF19's deadlines fire for a task that never yields, with no input pending, and MA7 tests a stalled storage thread |
| [RV4-04](06-findings.md#rv4-04-a-dead-renderers-memory-charged-to-no-one) | Release at death; no CPU reads of renderer memory; mapped pages in the accounting; a test | PL8; IR10; PF4; GR13's note; MA7; ADRs 0007, 0012 | MA7's test: 1 s after a renderer's death, nothing of it is mapped |
| [RV4-05](06-findings.md#rv4-05-rights-in-messages-that-are-dropped-rejected-or-returned) | Every handle owned or destroyed; returned messages kept whole or destroyed; a lying-peer test | IP9, IP11, IP16; TS14's V2; ADR-0005 | The test's port space ends where it began |
| [RV4-06](06-findings.md#rv4-06-efficiency-is-checked-only-at-gates) | Three tiers and a ratchet; PF25–PF30; an efficiency section in plans and designs; RQ5's trace | PF2, PF5, PF25–PF30; TS1, TS13; RQ5; MA6, MA11; MS7, MS10; G4's note; ADR-0019 | MA6 lists the new budgets as hypotheses with methods, and MA11 builds the harness at its second checkpoint |
| [RV4-08](06-findings.md#rv4-08-m1s-scope-against-value) | HTTP/2 to M2; encodings staged or `encoding_rs`; text entry into pages to M4; the preload scanner to M3 | MA1, MA2, MA7, MA11; NW §8; TX3; HD §8; input §5; the roadmap's M1–M4; ADRs 0011, 0013, 0015, 0021 | MA1 and the roadmap agree on each item's milestone |
| [RV4-09](06-findings.md#rv4-09-m1s-css-target-asks-for-the-long-tail-first) | The owner's choice of M1's CSS target | MA4; the roadmap's M1 and M3; ADR-0021 | MA4 states the target, with its floors if chosen |
| [RV4-11](06-findings.md#rv4-11-channels-in-one-process-and-stress-in-many) | In-memory channels that model Mach's limits and failures; a stress configuration for the gate | IP2; TS8; MA7, MA8; ADR-0005 | — |
| [RV4-13](06-findings.md#rv4-13-spikes-measured-on-an-earlier-macos-build) | S5, S6 and S7 re-run on the current build, if authorized; builds recorded | Their results' notes; the spikes' README; the status | Each results document names the build it was last confirmed on |
| [RV4-14](06-findings.md#rv4-14-stale-statements-and-couplings-the-decision-map-lacks) | The stale statements corrected; four couplings added; answered questions closed (§3.4) | The requirements; invalidation §8; networking §8; HTML §10; the decision map; the designs' open-question tables | A search for each old wording finds it only beside a note |

### 1.2 Before Milestone 2

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV4-07](06-findings.md#rv4-07-what-the-browser-takes-from-the-gpu-processs-surfaces) | One right per surface; validation of each new surface; values stripped and colour set from the commit; CAContext recorded as rejected | GR14, GR12; graphics §9; IP11; ADR-0012 | M2's plan tests a malformed surface and an attached value |
| [RV4-10](06-findings.md#rv4-10-the-tile-protocols-writer-side) | Commit once scheduled; video frames pooled by their own release | GR12; the media design; ADR-0012 | — |
| [RV4-12](06-findings.md#rv4-12-the-rendezvous-checks-a-pid-not-a-process) | The PID's version pinned; one hello per child; the code requirement considered | IP4; ADR-0004 D2's note; ADR-0005 | — |
| [RV4-16](06-findings.md#rv4-16-child-frames-geometry-after-the-gpu-process) | Child frames' rectangles forwarded with the layer commit and checked | IV13; FT12's note; ADR-0007 | — |
| [RV4-17](06-findings.md#rv4-17-m2s-cache-and-cookie-jar-wait-on-a-storage-engine-chosen-at-m3) | SW2's decision and its crash tests at M2's plan | The roadmap's M2, M3 and decisions; storage §8, SW13; NW10 | — |

### 1.3 Not tied to a milestone

| Finding | Change |
|---|---|
| [RV4-15](06-findings.md#rv4-15-an-advisory-in-the-spikes-lock) | `rustls-webpki` updated in the spikes' lock with the first checkpoint's commit; TS15's check by hand at each review gate until DP22's tools are authorized |

## 2. Proposed checkpoints

As in Rounds 1 and 2: the owner's words go in the disposition log (the review's §5); the changes are made in checkpoints on `architecture-review`, named `docs(review-r4-cpN): …`, each with the five checks green and pushed; an ADR's change is an amendment in its history; each checkpoint updates the [handover](../../handover-phase-0.md) and the [decision map](../../decision-map.md) where they summarize a rule it changes (the review's §4, step 3).

| Checkpoint | Findings | Documents |
|---|---|---|
| CP1 | RV4-01, RV4-09 | Milestone 1 (MA1, MA2, MA4, MA7, MA11); HTML §8; the roadmap; the standards (SB11); RQ40; ADRs 0015, 0018, 0021 |
| CP2 | RV4-02, RV4-15 | The dependency policy and register; text and fonts (TX3, §4); images (IM2); MA12; ADRs 0010–0012; the lock (a spike commit) |
| CP3 | RV4-03 | Performance (PF19, PF21); the event loop (§4.7, EL17); IPC (IP13); storage (SW10); MA5, MA7; ADRs 0006, 0019 |
| CP4 | RV4-04, RV4-05, RV4-07, RV4-10, RV4-11, RV4-12 | The pipeline (PL8); the process model (IR10); performance (PF4); graphics (GR12–GR14, §9); IPC (IP2, IP4, IP9, IP11, IP16); testing (TS8, TS14); MA7, MA8; ADRs 0004, 0005, 0007, 0012 |
| CP5 | RV4-06 | Performance (PF2, PF5, PF25–PF30); testing (TS1, TS13); requirements (RQ5); MA6, MA11; the roadmap (MS7, MS10); the charter's G4 note; ADR-0019 |
| CP6 | RV4-08, RV4-16, RV4-17 | Milestone 1 (MA1, MA2, MA7, MA11); networking §8, NW10; text (TX3's staging); HTML §8; input §5; invalidation (IV13); storage §8, SW13; the roadmap's M1–M4; ADRs 0007, 0011, 0013, 0015, 0021 |
| CP7 | RV4-13, RV4-14, and the verification | The requirements; the designs' open-question tables (§3.4); invalidation §8; networking §8; HTML §10; the decision map; the handover; then **the final sweep** (the review's §4, step 3, with this round's old wordings), **the verification of every change** against its "done when", and **MA4's figures recomputed by an independent count** |

**Order:** CP1 and CP2 first, since the rest of M1's scope depends on them; CP3–CP6 in any order; CP7 last. **S5–S7's re-runs** (RV4-13), if authorized, run before CP7 so that their results can be noted there.

## 3. Every open question, classified

From the [open questions](../../open-questions.md) and every design's own table, as they stand at `e71803b`. Each has a recommendation; those marked **owner** need the owner's choice.

### 3.1 Before Milestone 1 starts

The owner's dispositions of this round (§6): the XML parser (RV4-01), the dependency policy (RV4-02), the CSS target (RV4-09), the scope (RV4-08), the efficiency strategy (RV4-06), S5–S7's re-runs (RV4-13), the end of the cycle (§5).

### 3.2 At Milestone 1's plan

| Question | Recorded in | Recommendation |
|---|---|---|
| **Xcode** (OQ-7) **owner** | Open questions; the roadmap; MA12 | **Don't install it for M1:** no design needs it (GR14); shaders compile at run time; `samply` and command buffers' timestamps measure. Revisit only if M2's GPU process needs Metal's debugger |
| **QoS overrides on one-way messages** (OQ-20; IPC §8, question 1; the event loop's §7, question 2) | MA12 | **Measure at M1,** with its first Mach channels, as RV1-07 made possible |
| **`smallvec`'s grade** (text §10, question 2) | MA12 | **Grade 1.16.3** (43 `unsafe` blocks, 11 commented, §4 of [reuse and scope](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use)); admit it under RV4-02's DP8 if it is `ub-risk-3` |
| **`vello_cpu`'s grade** | MA12 | **Grade it:** its root forbids `unsafe`, so its grade rests on its dependencies' |
| **JPEG's decoder** (images §10, question 2; IM2) | MA12 | **`zune-jpeg` without its SIMD features,** which has no `unsafe` there, against `jpeg-decoder` (dormant), on the corpus's images |
| **Surfaces allocated ahead** (GR12) | MA12 | **A pool sized from S6's slow frames,** a hypothesis measured at M1's paint checkpoint |
| **AWS-LC's jitter entropy** (S7) | MA12; the roadmap | **Measure its 29–43 ms first-use cost** against PF25's helper start-up; keep AWS-LC's default unless it misses the budget |
| **The objc2 classes** (EM11; OQ-11) | MA12 | **Validate at the shell's checkpoint,** as planned; the Swift fallback stays behind the same API |
| **Hosted runners** (TS17; Milestone 1 §8, question 3; testing §10, question 2) **owner** | MA12 | **This Mac only at M1:** the five checks already run here before every commit, and GitHub's runners would repeat them on macOS 26 with 3 CPUs; revisit when a suite outgrows the Mac's time |
| **The nightly to pin** (S12's route) | MA12 | **`nightly-2026-10-21`,** the one S12 validated, re-checked at M1's start |
| **The signing identity** (RV1-17) **owner** | MA10, MA12 | **Created by the owner at M1's start,** as S15 validated |
| **The user-agent string** (SB13) | MA12 | **Safari-compatible with an Axiom token,** from the Safari shipping at M1's start |
| **The `css/CSS2` tests in scope; the local copies' date; the pins** (Milestone 1 §8, questions 1 and 2; standards §8, question 1) | MA3, MA4, MA5 | **Recompute with `--needs` at a revision wpt.fyi aligned** (`0f26972422` is one); copies made at M1's start |
| **Unicode's lag** (standards §8, question 2) **owner** | MA3 | **Accept ICU4X's Unicode 17.0** at M1: one version throughout (SB1) |
| **Style's cache sizes** (style §10, question 3); **DOM text ropes** (HTML §10, question 5); **IM10's limits** (images §10, question 4); **limits for hostile documents** (layout §10, question 5) | PF23; the designs | **Set at M1,** within B3, on the corpus copies and with M1's fuzzing |
| **On-demand font assets in renderers** (text §10, question 3) | Text §6 | **M1's text checkpoint,** by S4's method |
| **The IDL's syntax and generator** (IPC §8, question 2) | IPC | **M1's first checkpoint,** with the IPC ownership rule (RV4-05) |
| **The dependency tools; vendoring** (dependency policy §13, questions 2 and 3; DP22) **owner** | MA10 | **Install `cargo-deny` and `cargo-vet` with the first engine crate;** no vendoring at M1: the lock and the registry cache suffice |
| **Seed corpora's home** (testing §10, question 3) | TS10 | **In the repository** when small and generated |
| **A changed term stopping a census** (corpus §10, question 4) | CO3 | **Yes:** stop for the owner's review, as a changed robots file does |

### 3.3 Later, with their triggers

| Question | Trigger | Recommendation |
|---|---|---|
| **The storage engine,** `redb` or SQLite (storage §10, question 1; SW2) | M2's plan (RV4-17) | SW13's crash tests at M2 |
| **WebAssembly** (OQ-3; JE §10, question 5) | Before M3, with spike S14 (RV1-13) | Authorize S14 at M2's end |
| **The collector's end-state** (RV1-04, spike S11) | Before M3 | Authorize S11 at M2's end |
| **The bindings' generated size** (bindings §10, question 1); **M3's interfaces** (question 2); **the accumulator's encoding** (JE §10, question 1); **the skeleton replica's fields** (frame tree §8, question 1) | M3's plan | Measured as RV1-18 measured style's generated code |
| **Explicit tail calls** (JE §10, question 2) | `become` stable and no longer incomplete | Still incomplete on 2026-10-20's nightly |
| **Accessibility trees' memory** (accessibility §11, question 3) | M4 | Within B1 and B3, by AX7's tiers |
| **Compositor animations beyond `transform` and `opacity`** (invalidation §8, question 2) | M4, with CSS animations | — |
| **Path items in display lists** (OQ-24; graphics §10, question 1) | M4, with SVG | As the owner deferred it |
| **Taffy as a test oracle** (layout §10, question 3); **hyphenation** (text §10, question 4); **statistical encoding detection** (HTML §10, question 4); **3D depth sorting** (layout §10, question 2) | M4, or the milestone with each feature | — |
| **The sensitive-site list and the JIT switches' presentation** (embedder §8, question 2); **DNS over HTTPS with ECH** (networking §10, questions 3 and 4) | M5's plan | — |
| **WebAuthn and passkeys** (OQ-16) | Before M6 | The hybrid transport's feasibility check under the signing identity |
| **Demuxers and codecs; one media process per site or shared** (images §10, questions 3 and 5); **Safari's energy figures** (performance §9, question 2) | M7 | Decided by maintenance (`mp4parse` is dormant) and the threat model |
| **The bytecode cache** (JE §10, question 4); **push messaging and background sync** (storage §10, question 3); **VS Code and WebAssembly** (roadmap §11, question 2) | M8's plan | — |
| **WebGPU's implementation and process** (graphics §10, question 5); **Maps without WebAssembly** (roadmap §11, question 3) | M9's plan | Reuse `wgpu` first, in its own process (GR10) |
| **Printing to PDF** (layout §10, question 1); **dragging across frames** (input §8, question 4) | The milestone that needs each | — |
| **Parallel style and layout** (style §10, question 6; layout §10, question 4; the event loop's §7, question 3) | Profiling shows the main thread over PF13's 8 ms | OQ-19's rule |
| **Pages measured by hand; their features; Safari's automation** (roadmap §11, question 1; corpus §10, questions 1 and 2) | Each milestone's plan, with the owner | — |
| **The not-planned list** (standards §8, question 3) | Any gate | — |

### 3.4 Answered, but still listed as open (closed in CP7)

| Question | Answered by |
|---|---|
| The event loop's §7: question 1 (starvation, leeway, hang deadlines); 4 (which thread receives input); 5 (workers' processes) | PF19, PF17; IN1; EL §4.6 |
| The frame tree's §8: questions 2 (the back-forward cache), 3 (workers), 4 (`beforeunload`) | PF22; EL §4.6; PF22 |
| Input's §8: questions 1 (deadlines), 2 (key equivalents), 3 (the role mapping) | PF20; accessibility §11, question 2; ADR-0014 |
| Invalidation's §8: questions 1 and 3 | PF19, PF20; I2 |
| HTML's §10, question 3 (the parser's budget) | PF23 |
| The JavaScript engine's §10, question 3 (ropes and slices) | PF23 |
| The pipeline's §8: questions 2–4 | I1 (atlases in the renderer); I2; P6 |
| Script and heap's §7: questions 2–4 | JE6; PF15 and RV1-04; EL §4.6 |
| Style's §10, question 4 (the user-agent stylesheet) | PF23 |
| Text's §10, question 5 (IDNA in the browser) | NW2, which says so |

## 4. Milestone 1's implementation plan

**The document:** `docs/milestone-1-plan.md`, written once the checkpoints are done and the owner has ended the cycle, and adopted as M1's CP1 (MS10). It refines MA11's outline into checkpoints, each with its scope, its tests, its efficiency checks (§4.2 of [efficiency](04-efficiency.md)), the dependencies it adopts with their grades, and its exit criterion.

**Its shape,** with this round's changes marked (inference, for the owner's review):

| # | Checkpoint | This round's additions |
|---|---|---|
| 1 | The skeleton, the checks, profiles and lints; a window, a Metal clear, a layer; the helpers under their profiles, with a Mach channel and a memory entry | IPC's ownership of rights (RV4-05); channels that model Mach (RV4-11); the watchdog's task ages (RV4-03) |
| 2 | The harness end to end: the offscreen PNG, HTTP/1.1 to wptrunner on the loopback, the script-free executor; URLs and `file:`; the HTML tokenizer and tree builder; the node arena and the DOM | **The XML parser** (RV4-01); **the efficiency harness:** rusage reads, the baseline and the ratchet (RV4-06) |
| 3 | CSS: syntax, selectors, cascade, computed values; T1–T4 with generated code | Size assertions (RV4-06's first tier) |
| 4 | Text: the font database, shaping, line breaking, bidi, the atlas; the Rust path | `smallvec` admitted by grade (RV4-02); UTF-8, UTF-16 and windows-1252, or `encoding_rs` (RV4-08) |
| 5 | Block and inline layout, floats, positioning, tables; the dumps | — |
| 6 | Paint, display lists, tiles and scrolling | Release at a renderer's death (RV4-04) |
| 7 | Images behind IM9 | JPEG by MA12's decision (RV4-02) |
| 8 | HTTP/1.1 over TLS, rustls, DNS-SD, the in-memory cache, mixed content, HSTS; the network log | **HTTP/2 moved to M2** (RV4-08) |
| 9 | The shell: the address field, history, the objc2 classes | No text entry into pages (RV4-08) |
| 10 | Reference tests at scale: the `css/CSS2` selection with its expectations | The target RV4-09 sets |
| 11 | Fuzzing, determinism, budgets | The XML fuzz target; the lying-peer test; the stress configuration; PF25–PF30 measured |
| 12 | The corpus checks, and the gate | — |

## 5. The path to the merge

1. **The owner disposes** of RV4-01 to RV4-17, and accepts or overrules the verdicts on RV3-01 to RV3-18 (§6).
2. **CP1–CP7** are made, each pushed, the five checks green; S5–S7 re-run first if authorized.
3. **CP7 verifies** every change and recomputes MA4 independently.
4. **The owner ends the cycle** (the review's §6), or commissions another round.
5. **The owner merges** `architecture-review` into `main` (`git merge --no-ff`), and `main` is pushed when the owner says so.
6. **`docs/milestone-1-plan.md`** is written (§4).
7. **The owner authorizes Milestone 1** (MA10), which starts its branch with the plan as CP1.

## 6. Decisions for the owner

1. **Dispose of the findings.** **Recommendation: accept all seventeen as proposed,** with the choices below. Each is a document change with evidence behind it, and together they make M1's gate runnable and its scope proportionate.
2. **RV4-01, the XML parser at M1:** Axiom's own, or a reused tokenizer under an amendment of G1. **Recommendation: Axiom's own,** for documents only: it is small (HD §3), it is G1's parsing, and the candidates are dormant or had denial-of-service advisories in 2026.
3. **RV4-02, the dependency policy:** admit `ub-risk-3` crates under DP10's conditions, with Axiom's audit and fuzzing, and move the burden of proof for supporting infrastructure. **Recommendation: both:** it removes the policy's inversion and is what the owner's second theme asks.
4. **RV4-08, M1's scope:** HTTP/2 to M2, text entry to M4, the preload scanner to M3; encodings through `encoding_rs` at M1 if item 3 is accepted, or staged. **Recommendation: all four, with `encoding_rs`:** Firefox's codecs, maintained and without advisories, behind TX3's interface in sandboxed helpers.
5. **RV4-09, M1's CSS target:** ≥ 90% with directory floors, ≥ 97% kept, or Round 3's ≥ 88%. **Recommendation: ≥ 90% with floors,** the median at M3, as SB11 allows for an area a milestone starts.
6. **RV4-06, the efficiency strategy:** three tiers, a ratchet (1% instructions, 2% peak footprint, no unexpected writes), PF25–PF30 set at M1's plan. **Recommendation: accept;** S16-Q3 showed the counters are steady enough.
7. **RV4-13, re-running S5, S6 and S7's scripts on the current macOS build.** **Recommendation: authorize,** once, before CP7: no new code, about an hour of runs.
8. **RV4-15, `rustls-webpki` in the spikes' lock.** **Recommendation: update it** in CP2's commit.
9. **Round 3's verdicts** (RV3-01 to RV3-18): accept or overrule. **Recommendation: accept;** each is tested in the [verification](02-verification.md).
10. **The checkpoints CP1–CP7** (§2), run without stopping between them, each pushed, with no merge. **Recommendation: approve.**
11. **The stopping rule:** end the cycle after CP7's verification, or commission Round 5. **Recommendation: end it after CP7** ([risk matrix](01-risk-matrix.md#4-the-stopping-rule) §4): both blocking findings are bounded document changes whose verification is mechanical.
12. **After the cycle:** the merge into `main`, then `docs/milestone-1-plan.md`, then MA10. **Recommendation: in that order,** each at its own gate.
