# Phase Zero Handover

| | |
|---|---|
| **Status** | **The Phase Zero baseline,** as the owner approved it at Step 6's Gate C (2026-10-20), **as amended by the architecture review's Rounds 1 and 2** (2026-10-21; [the review](review/README.md)), on branch `architecture-review`. It is the starting point of the independent external review the owner commissions before that branch merges into `main`. Milestone 1 is on hold until the review is done (the owner's decision 8 at Gate C). Updated when a review round changes the baseline |
| **Question** | What did Phase Zero decide, on what evidence, with which trade-offs, assumptions and risks? What did the review change? What is still open, and what should an independent review challenge first? |
| **Covers** | Steps 0–6, from 2026-10-16 to 2026-10-20, and the review's Rounds 1 and 2 (2026-10-21): ADRs 0001–0021 with their amendments, spikes S1–S10, S12 and S15, and the binding documents |
| **Builds on** | Every document listed in §12. This one adds no rule |
| **Last updated** | 2026-10-21 |

**How to read it.** Each section summarizes the documents it links to. Where this summary is shorter than its source, or seems to differ from it, the source governs: its rules, its numbers, its evidence labels and its dated sources. Every figure was measured on the owner's Mac (§2) between 2026-10-16 and 2026-10-21, as the linked results record. Statements marked **assessment** are this handover's own inference, written for the review. They are not decisions.

**The review's amendments** are marked with their round and finding, for example "(Round 1, RV1-07)". The owner's words for each are in the review's [disposition log](review/README.md#5-the-disposition-log), and each finding's evidence is in its round's documents under `review/`.

---

## 1. Summary

- **What Axiom is.** A browser engine designed from first principles for one user, the owner, on one Mac: a MacBook Air with an M1 and 8 GB, on macOS 27.2, with the Command Line Tools only ([charter](charter.md)). It is to have its own JavaScript engine, reach the full modern app web, and reach a posture fit for sensitive logins in verified stages. Efficiency in memory, CPU and power is a primary goal, measured on this Mac.
- **Where it stands.** Phase Zero is complete by the charter's criteria (§8):
  - the owner approved the architecture proposal (the [overview](architecture/overview.md) v2), the [roadmap](roadmap.md) and [Milestone 1](milestone-1.md)'s specification at Step 6's Gate C;
  - all 21 ADRs are accepted;
  - the [corpus](corpus.md) of 13 sites is defined.

  **No engine code exists.** The repository holds documents, throwaway validation spikes under `spikes/` (S1–S10 and S15; S12 ran on S9's code), and tools under `tools/`.
- **The review so far** ([the review](review/README.md)):
  - **Round 1** (2026-10-21): 22 findings, 2 Critical, 6 High, 10 Medium and 4 Low, **all accepted by the owner** and made in checkpoints, with spikes S10, S12 and S15 run and their results approved.
  - **Round 2** (2026-10-21): verified Round 1's changes, and audited accessibility, storage and workers, style and layout, the embedder API and IPC. 15 findings, none Critical or High, so **the review's stopping rule is met** for the scope commissioned. **All accepted** and made in checkpoints (the review's §5.6).
- **What comes next.** The owner commissions **one final, independent external review** before the review phase ends and `architecture-review` merges into `main`. Milestone 1 starts only when the owner then authorizes its list (MA10). The owner's words: "Implementation should begin only when the plan has been rigorously validated, no significant flaws or gaps remain, and further review yields no meaningful improvements."
- **The decisions that matter most:**
  1. **Its own JavaScript engine,** sharing one garbage-collected heap with the DOM ([ADR-0001](adr/0001-scope-and-target.md) R2; ADRs [0003](adr/0003-gc-handle-and-rooting-pattern.md), [0008](adr/0008-script-agents-and-heap.md), [0017](adr/0017-javascript-engine-and-bindings.md)).
  2. **Rust throughout,** with `unsafe` confined to listed `platform` modules and, later, a `jit` crate; the collector has none ([ADR-0002](adr/0002-implementation-language.md), ADR-0008). Every executable aborts on a panic (Round 1, RV1-16).
  3. **A multi-process architecture staged from L0 to L3,** with every interface present from the first milestone, **real sandboxed renderer and network processes from M1** (Round 1, RV1-07), and **a sandboxed GPU/compositor process from M2** that validates, rasterizes and composites renderer output, so the browser never consumes display lists (Round 1, RV1-02) ([ADR-0004](adr/0004-process-model-and-sandbox.md), [ADR-0005](adr/0005-ipc-transport-and-interfaces.md), [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [ADR-0020](adr/0020-roadmap-and-security-levels.md)).
  4. **Out-of-process iframes and the checks V1–V9 before any sensitive use** (L3, at M6, certified on the final topology; the [threat model](security/threat-model.md) §8).
  5. **An interpreter first.** A JIT only on evidence, never before L2, and only in one executable (ADR-0004's J1–J7); whether to build one is decided at M5's gate (Round 1, RV1-13).
  6. **Budgets set on this Mac:** B1–B7, with B1 split between the browser (B1a) and the GPU process (B1b) (Round 1, RV1-02), and a memory ceiling for every corpus page at half of Chrome's measured cost, capped by tier; the content tier is gated by a slope and a plateau on a normalized workload (Round 1, RV1-05) ([ADR-0019](adr/0019-performance-methodology-and-budgets.md)).
  7. **Standards first,** with each completed area targeting the median of Chrome, Firefox and Safari on its WPT directories ([ADR-0018](adr/0018-corpus-standards-and-testing.md)); Milestone 1's CSS target is interim, over the tests that need no script (Round 1, RV1-01).
- **The largest risks** (assessment; §9, §11):
  - the scope against one developer;
  - an interpreter alone on the app web;
  - a sandbox built on undocumented macOS interfaces;
  - memory ceilings far below what shipping browsers spend;
  - WebAssembly's implementation arriving last, though its architecture is now decided before M3 (Round 1, RV1-13);
  - ThreadSanitizer for Rust on macOS 27 only by the routes spike S12 found (Round 1, RV1-11).

## 2. The project frame

**The charter** ([charter](charter.md)):

| Item | Content |
|---|---|
| **Goals** | G1 implement the core mechanisms ourselves (parsing, DOM, style, layout, painting, incremental updates, navigation and loading, security policies, the JavaScript engine); G2 be useful on a growing set of real sites, with an architecture that can reach the full app web; G3 a posture fit for sensitive logins, in verified stages; **G4 efficiency as a primary architectural goal**; G5 decisions from evidence, recorded in ADRs |
| **Users** | One: the owner, on their own Mac. No other users, accounts or profiles |
| **Environment** | MacBook Air (M1, `MacBookAir10,1`, fanless), 4 performance and 4 efficiency cores, **8 GB**, macOS 27.2; Rust 1.94.1, Swift 6.4, Apple clang 21; **Command Line Tools only**, so no Instruments. This machine is the performance floor |
| **Non-goals** | Cross-platform support; Intel Macs and older macOS; sync, accounts, cloud services; extensions; enterprise administration; multiple profiles; translating the browser's own interface; DRM; bug-for-bug compatibility beyond the specifications |
| **Constraints** | Memory (8 GB makes per-process budgets first-class); thermals (fanless); the toolchain (no Xcode); **one human developer with AI assistance** (an assumption, OQ-9); **no deadline** (OQ-1) |
| **Principles** | Research first; the smallest well-engineered system; security and performance as architecture; build the core, reuse infrastructure only with a stated class; keep fact, inference and hypothesis apart; name technical debt; **7. no architectural corners cut for convenience:** staging the implementation, never the architecture |

**Scope** ([ADR-0001](adr/0001-scope-and-target.md), 2026-10-16): R1, the full modern app web must stay reachable, while milestones start with static documents; R2, Axiom's own JavaScript engine; R3, a posture for sensitive logins, gated until isolation is verified. Apple Silicon and the current macOS only.

**Governance.** Only the owner moves an ADR to Accepted, and only the owner disposes of a review's findings. Work stops at gates: a step's plan, ADR acceptance, authorizing code or a spike, merging into `main`, and pushing a new branch. Between gates, authorized checkpoints run without pausing, each with five checks green (CLAUDE.md, "Verification"). [status.md](status.md) carries the handoff log.

**Phase Zero's steps, and the review:**

| Step | Topic | Done | Produced |
|---|---|---|---|
| 0–1 | The decision landscape; requirements | 2026-10-16 | The [decision map](decision-map.md); ADR-0001 |
| 2a, 2b | The implementation language; spikes S1 and S2 | 2026-10-17 | ADR-0002 (Rust), ADR-0003 (the GC pattern); the [language evaluation](technology/language-evaluation.md) |
| 3 | Threat model, process model and sandbox | 2026-10-17 | The [threat model](security/threat-model.md), the [process model](security/process-model.md), ADR-0004; spike S4 |
| 4 | Event loop, threads, pipeline, invalidation, embedder | 2026-10-18 | ADRs 0005–0009; the overview v0; spike S3 |
| 5 | The subsystems | 2026-10-20 | ADRs 0010–0017; the dependency policy and eleven subsystem designs; the overview v1; spikes S5–S7 |
| 6 | Standards, testing, budgets, the roadmap | 2026-10-21 (merged) | ADRs 0018–0021; the corpus, standards, testing, performance, roadmap, Milestone 1 and requirements documents; the overview v2; spikes S8 and S9 |
| Review | Rounds 1 and 2 of the adversarial review | 2026-10-21 (on `architecture-review`) | The [review](review/README.md)'s rounds and disposition log; amendments to the ADRs and designs; spikes S10, S12 and S15; the [audit of `encoding_rs`](research/encoding-rs-audit.md) |

## 3. How to review this repository

**A reading order:** this handover; the [review](review/README.md), with its rounds' findings and the owner's dispositions; the [charter](charter.md); the [decision map](decision-map.md); the [overview](architecture/overview.md); the [threat model](security/threat-model.md) and [process model](security/process-model.md); the ADRs and the designs under `architecture/`; the spike results under `research/`; the [roadmap](roadmap.md), [Milestone 1](milestone-1.md) and the [requirements](requirements.md).

**Where the evidence is:**
- each spike's results document (`docs/research/sN-results.md`) gives its method, environment, figures, threats to validity and how to reproduce it;
- the raw records stay under `target/` on the owner's Mac, which git ignores, so a reviewer elsewhere has the documents' figures and the tools that produced them, not the records;
- the trackers (`docs/research/step-N-*.md`) record each gate's decisions in the owner's words, and the review's [disposition log](review/README.md#5-the-disposition-log) records its rounds'.

**Conventions:**
- **Evidence labels** ([research conventions](research/README.md)): sourced (with a link and dates), measured or observed (on this Mac, naming the file or command), inference, hypothesis (with the step that tests it), background.
- **Planned and implemented** are kept apart: everything under `architecture/` is planned.
- **Rule IDs belong to their document** (Appendix A).
- **Dates are absolute.**
- **Amendments** keep the text they amend where it is a record, marked "as amended in Round N" with the finding's ID, and the ADR's decision history quotes the owner.

**The review's format for findings** ([the review](review/README.md) §3): its severity (Critical, High, Medium or Low, each defined there), the milestone it blocks, the claim challenged with its rule ID, labelled and dated evidence, the failure mode, a counter-proposal, the cost of making the change later instead of now, and the rules it would amend. Each finding then gets a disposition from the owner, accepted, rejected or deferred, with a reason, in the review's log (§5 there). Rounds 1 and 2 are written that way; an external review needn't be, but findings in that form are the cheapest to dispose.

**What a reviewer can't do from the repository:** repeat the measurements, which need this Mac and its conditions (PF1); open the owner's accounts; or automate the corpus sites beyond what their terms allow ([corpus](corpus.md) CO3, CO4).

## 4. The architectural blueprint

### 4.1 The system on one page

From the [overview](architecture/overview.md) §1, as redrawn in Round 1 (RV1-02, RV1-07, RV1-09):

```
 Browser process (trusted, hardened, unsandboxed)
 ┌────────────────────────────────────────────────────────────────────────────┐
 │ main thread (AppKit)     core thread               compositor thread       │
 │ shell UI · NSEvents ──▶  frame tree · navigation   display links · frames  │
 │ IME mirror · AX cache    session & visited history CALayer tree · scroll   │
 │                          process manager · policy  hit test · animation    │
 │            ◀── embedder API (IDL messages) ──▶     ◀── layer commits       │
 └──────────────────────┬──────────────────────────────────┬──────────────────┘
          Mach IPC: browser-created channels (capabilities), audit-token pinned
 ┌──────────────────────┴────────────────────┐   ┌─────────┴────────────────────┐
 │ Renderer (sandboxed, site-locked from L2) │   │ GPU/compositor (sandboxed,   │
 │ event-loop thread per agent:              │──▶│ from M2): validate display   │
 │   JS heap + DOM · parse · style · layout  │   │ lists · property trees ·     │
 │   · paint → display list per frame        │   │ Metal tiles → IOSurfaces     │
 │ helper pool: decode, glyph & path raster, │   └──────────────────────────────┘
 │   compile scripts and stylesheets         │   ┌──────────────────────────────┐
 │ CoreText (system fonts) · safe decoders   │◀─▶│ Network process (sandboxed)  │
 │ pixels in Mach memory entries, no IOKit   │   │ I/O thread: rustls, HTTP/1-3,│
 └───────────────────────────────────────────┘   │ quinn-proto, DNS-SD, ORB     │
                                                 │ worker pool: SecTrust, disk  │
                                                 │ storage thread: cookies,     │
                                                 │ cache index, redb stores     │
                                                 └──────────────────────────────┘
                                                  later: media utility (AVIF)
```

At M1 the GPU/compositor process's code runs as a browser thread behind its interface; M2 moves it into its own process (Round 1, RV1-02).

The overview §4 traces seven scenarios end to end through the rules: opening a URL, typing, scrolling a cross-site frame at L3, script that forces layout, a renderer crash, a late web font and image, and a click.

### 4.2 The 21 decisions

| ADR | Decides | Accepted | Amended since |
|---|---|---|---|
| [0001](adr/0001-scope-and-target.md) | Scope: the full app web reachable; our own JavaScript engine; sensitive logins, gated; defaults | 2026-10-16 | — |
| [0002](adr/0002-implementation-language.md) | Rust, stable, 2024 edition, with `unsafe` confined; build thresholds T1–T4 | 2026-10-17, final | Round 1: failure semantics, `panic = "abort"` and checked arithmetic (RV1-16) |
| [0003](adr/0003-gc-handle-and-rooting-pattern.md) | The GC pattern: Pattern B (Nova style), `u32` handles into typed arenas, scope tokens, a root stack; obligations C1–C5 | 2026-10-17 | Its obligations settled by ADR-0008 |
| [0004](adr/0004-process-model-and-sandbox.md) | Process types, Seatbelt confinement, launch, site isolation (SI1–SI7), levels L0–L3 with V1–V9, memory targets, the JIT policy J1–J7 | 2026-10-17, after S4 | S4's refinements confirmed; out-of-process iframes mandatory before L3. Round 1: the GPU/compositor process from M2, real processes from M1, no IOKit in renderers (RV1-02, RV1-07, RV1-09); a stable local signing identity (RV1-17, S15); the GPU process's one exception to D2, after S10 |
| [0005](adr/0005-ipc-transport-and-interfaces.md) | IPC: Mach messages, endpoints as capabilities, an IDL with generated safe decoders, `Untrusted<T>` | 2026-10-17 | Round 1: IP2 across real processes from M1 (RV1-07); IP11's validators panic-free (RV1-16) |
| [0006](adr/0006-event-loop-and-threading.md) | Threads and event loops: one wait per thread, a browser core thread, a thread and heap per event loop | 2026-10-17 | EL11: DOM arenas lendable read-only during frozen stages (OQ-19). Round 1: the compositor's threads split across the browser and the GPU process (RV1-02); EL11's second exception, for parallel marking (RV1-04) |
| [0007](adr/0007-frame-tree-pipeline-and-input.md) | The browser-owned frame tree, the five-stage pipeline, display lists, the compiler-checked lifecycle, input | 2026-10-17 | IV1, IV2: nested lifecycles for size containers (2026-10-20). Round 1: the GPU process in the pipeline (RV1-02); cross-site frames as separate agents from M1 (RV1-03); PL9's rule on effects over cross-origin frames (RV1-19) |
| [0008](adr/0008-script-agents-and-heap.md) | Agents and heaps; C1 as the safe `SliceStack`; struct-of-arrays columns; setters as the barrier point; stop-the-world mark–compact for now | 2026-10-18, after S3 | SH10: the node arena is never compacted (2026-10-20). Round 1: separate agents from M1 (RV1-03); **the collector's end-state, fixed before M3** (RV1-04) |
| [0009](adr/0009-embedder-api-and-shell.md) | The shell as an embedder over a message API; native UI; a headless embedder; a pure-Rust shell with objc2 | 2026-10-17 | Round 1: the headless embedder's script-free test path and text dumps (RV1-01, RV1-15) |
| [0010](adr/0010-dependency-and-supply-chain-policy.md) | The dependency and supply-chain policy, DP1–DP23 | 2026-10-18 | DP10 refined before acceptance. Round 1: DP1 (g) and DP2 make encodings, Temporal's arithmetic and BigInt replaceable details (RV1-08) |
| [0011](adr/0011-text-fonts-and-unicode.md) | Text: ICU4X, our own encodings, CoreText for system fonts, HarfRust shaping, a Rust path for web fonts | 2026-10-20, after S5 | `vello_cpu` adopted, subject to grading. Round 1: the encodings stay Axiom's own after Axiom's audit graded `encoding_rs` `ub-risk-3` (RV1-08); TX22–TX24, metrics from the rasterizer, `-webkit-font-smoothing`, optical sizing (RV1-10) |
| [0012](adr/0012-graphics-images-and-media.md) | Graphics: Metal directly, CoreAnimation layers, damaged tiles; safe image decoders; AVIF through ImageIO in the media process | 2026-10-20, after S6 | B1 revised; GR5's amendment deferred. Round 1: the GPU/compositor process from M2 (RV1-02); renderers' pixels in memory entries only (RV1-09); JPEG's decoder at M1's plan (RV1-22); S10's results |
| [0013](adr/0013-networking-storage-and-workers.md) | Networking and storage: state machines on one thread, our own HTTP framing, rustls with the system's verifier, DNS-SD, partitioning, storage in the network process | 2026-10-19, after S7 | Verification in the worker pool confirmed. Round 1: a stale fact about Encrypted Client Hello corrected (RV1-20) |
| [0014](adr/0014-accessibility.md) | Accessibility: trees pushed by renderers, the browser's cache, text markers, our own bridge; settings and reserved keys | 2026-10-18 | — |
| [0015](adr/0015-html-parsing-and-the-dom.md) | Our own HTML and XML parsers; the node arena; one mutation path; events and the rest of the DOM; XSLT deferred | 2026-10-20 | — |
| [0016](adr/0016-style-layout-and-paint.md) | Our own CSS engine; container queries in nested lifecycles; `:visited` by partitioned history; a 1/64-pixel unit; tables approximate | 2026-10-20 | — |
| [0017](adr/0017-javascript-engine-and-bindings.md) | The engine entirely our own; heap-free compiled scripts; a register machine; our own WebIDL compiler | 2026-10-20 | Round 1: separate agents from M1 (RV1-03); the slot width open until S11 (RV1-06); Temporal and BigInt as replaceable details (RV1-08); WebAssembly's architecture before M3 and the JIT at M5's gate (RV1-13); the console at M3 (RV1-15); bindings split by interface group (RV1-18) |
| [0018](adr/0018-corpus-standards-and-testing.md) | The corpus (CO1–CO14), the standards baseline (SB1–SB12) and the testing strategy (TS1–TS18) | 2026-10-20 | The calibration corrected (§5, S9). Round 1: the interim CSS target and two executors (RV1-01); TS11's ordered concurrency checks (RV1-11, S12); text's calibration after macOS updates (RV1-10); SB13's user agent and SB14's interventions (RV1-14) |
| [0019](adr/0019-performance-methodology-and-budgets.md) | Performance methodology (PF1–PF24) and the final B1–B7, with a ceiling for every corpus page | 2026-10-20 | The content cap kept as a hypothesis for review (Gate C). Round 1: B1 split (RV1-02); the content tier's slope and plateau (RV1-05); PF18 and the build's measurement (RV1-18); S10's results, and B1b kept at 15 MB |
| [0020](adr/0020-roadmap-and-security-levels.md) | The roadmap: nine milestones, ordered and gated, never dated (MS1–MS12) | 2026-10-20, as the baseline under review | Round 1: real processes at M1 and the GPU process at M2 (RV1-07, RV1-02); WebSockets at M4 (RV1-12); WebAssembly's architecture before M3, the JIT at M5's gate (RV1-13); daily features scheduled (RV1-22); and the rest of the round's placements |
| [0021](adr/0021-milestone-1.md) | Milestone 1: static documents at L0, with measurable criteria (MA1–MA12) | 2026-10-20, as M1's specification; starting it on hold | Round 1: real, sandboxed processes (RV1-07); the interim CSS target and the harness at the second checkpoint (RV1-01); the dumps (RV1-15); failure semantics (RV1-16); ordered concurrency checks (RV1-11, S12); the stable signing identity (RV1-17, S15) |

### 4.3 Security: the threat model, the levels and the process model

- **Adversaries and assets** ([threat model](security/threat-model.md) §3, §4):
  - four adversaries: a web attacker (AD1); **a compromised renderer (AD2), treated as a normal case, not an edge case**; a network attacker (AD3); and a compromised helper other than a renderer (AD4);
  - nine assets, from credentials and sessions (AS1) to the Mac's availability (AS9);
  - not adversaries: local malware running as the user, physical access, kernel and firmware attacks, a malicious toolchain;
  - Spectre within one site's process is accepted (§9); pixel stealing across frames through rendering timing is in the model since Round 1 (RV1-19), with a rule in PL9 from M5 and a test under V4.
- **Levels** (threat model §8, ADR-0004 D5):

  | Level | What must be true | Permitted use |
  |---|---|---|
  | L0 | Anything, including one process and no sandbox; **M1 nevertheless runs sandboxed renderer and network processes** (Round 1, RV1-07) | Local test pages and public documents; never credentials |
  | L1 | Web content only in sandboxed renderers; a sandboxed network process; the browser consumes no renderer display lists, **the GPU/compositor process does** (Round 1, RV1-02); IPC under IR1–IR10 | Browsing without logging in |
  | L2 | Renderers locked to one site; credentials and storage mediated by lock checks; ORB; third-party state partitioned | Logins to accounts that aren't sensitive |
  | L3 | Every cross-site frame in a renderer locked to its own site; **V1–V9 pass**, on the final topology (Round 1, RV1-02) | Email, banking, accounts with write access |

  **V1–V9:** sandbox escape tests; IPC fuzzing; lock enforcement; cross-site leak tests; an `unsafe` audit; a dependency audit; the JIT policy; a written threat-model review; out-of-process iframe tests.
- **Processes** (ADR-0004 D1, as amended in Round 1):
  - **the browser:** hardened runtime, **unsandboxed**, never parses web content, and **from M2 never consumes renderer display lists:** it keeps the windows, the CoreAnimation layer tree, scrolling and animation on layers, input routing and policy;
  - **renderers:** deny-by-default Seatbelt profiles: no files, network, window server or launching, and **no IOKit:** pixels go only into Mach memory entries (RV1-09);
  - **one network process:** outbound network and its own data only;
  - **the GPU/compositor process, from M2** (RV1-02): it validates every renderer's display lists, holds their property trees, rasterizes tiles with Metal into IOSurfaces it owns, and sends the browser layer commits. **S10** measured the commit's hop to the browser at 48.8 µs at the median, against PF13's 1 ms; zero-copy sampling of renderer memory; validation at about 9.4 ns an item. Its profile has ten rules, each checked by removal (S10 §6.1). WebGL and WebGPU never run in the browser, and M9's plan places them;
  - **utility processes** on demand, media first.
- **Recovery** (IR10, as amended in Round 2, RV2-04): a renderer's death gives its tabs a crash page; the network process is relaunched, renderers get new endpoints, and `sessionStorage` comes back from the browser's copy; the GPU process is relaunched with each window's last layers kept (GR13). PF19's watchdog covers every helper, and the shell hears of each crash or hang.
- **Confinement** (D2, after S4): each helper is its own executable, **signed by a stable local identity** with an explicit designated requirement, with ad hoc as the fallback (Round 1, RV1-17; S15 found it behaves as ad hoc for S4's probes and keeps Keychain access across rebuilds). **The browser compiles each profile and sends the bytecode;** the helper applies it as its first act: ready in 2.1 ms with a 1.1 MiB footprint. **One exception, approved after S10:** the GPU process resolves its cache directory with `confstr(_CS_DARWIN_USER_CACHE_DIR)` just before applying its profile, so Metal's shader cache works without widening the profile. Profiles are reviewed like `unsafe` code. Option O2 is the fallback if custom profiles stop working.
- **Launch:** only the browser launches processes, with `posix_spawn` under `POSIX_SPAWN_CLOEXEC_DEFAULT`, a minimal environment, and only the helper's IPC bootstrap handle and a log descriptor (the process model §8).
- **Site isolation** (SI1–SI7, [process model](security/process-model.md) §5): site locks from L2; process limits never merge sites. **SI3:** at L1 and non-sensitive L2 only, a cross-site iframe may render in its embedder's process, under three compensating rules: partitioned third-party state, ORB, and no sensitive site inside another site's process. **Cross-site frames are always separate agents behind remote proxies, from M1** (Round 1, RV1-03).
- **The JIT policy** (J1–J7):
  - interpreter first;
  - no JIT before L2; whether to build one is decided at M5's gate (Round 1, RV1-13);
  - only `renderer-jit` carries `allow-jit`, and every other profile carries an explicit `(deny dynamic-code-generation)` (S4 showed `(deny default)` doesn't cover it);
  - sensitive sites always run without a JIT, with global and per-site switches;
  - code written through allowlisted callbacks only;
  - a WebAssembly compiler counts as a JIT;
  - differential testing and a containment design before any JIT ships.
- **Memory** (the process model §7, final at Step 6's Gate B, amended in Round 1):

  | # | Budget |
  |---|---|
  | B1a | The browser, idle, one window: ≤ 60 MB for the core. From M2 it holds no Metal device and no tiles, only the layer tree (Round 1, RV1-02) |
  | B1b | The GPU/compositor process, idle, one window: ≤ 15 MB for its device, pipelines and display lists (S10 measured 13.4 MB; kept by the owner), plus each visible window's tiles at ≤ 6.2 bytes per pixel (25.4 MB at 2,560 × 1,600). At M1, while the compositor runs in the browser, the browser holds both |
  | B2 | A ready renderer: ≤ 10 MB |
  | B3 | A typical page: ≤ 50 MB as the median over the documents and content pages; every corpus page within its ceiling (§4.13), the content tier gated by a slope and a plateau (Round 1, RV1-05) |
  | B4 | The network process, idle: ≤ 25 MB |
  | B5 | A soft limit of 16 renderers |
  | B6 | ≤ 1 GB for 10 tabs of document and content pages on about 6 sites |
  | B7 | A 2 GiB cap per renderer |

### 4.4 IPC, threads and the event loop

- **IPC** ([ADR-0005](adr/0005-ipc-transport-and-interfaces.md), [IPC](architecture/ipc.md) IP1–IP22):
  - Mach messages carry everything, and in-process channels encode and decode too (IP2), so single-process test mode runs the same decoders;
  - endpoints are capabilities only the browser creates, and each child's audit token is pinned at launch and checked on every message;
  - an IDL generates safe-Rust types, canonical encoders, total decoders and a fuzzing entry point per decoder;
  - values from less privileged senders arrive as `Untrusted<T>`, validated against the receiver's records. A violation terminates the sender, or is ignored where a race is expected (IP11). The browser's validators can't panic, by lint (Round 1, RV1-16);
  - synchronous requests go only toward more privilege, to the network process for every case the platform requires, bounded by PF19's watchdog rather than a timeout of their own; a service worker's answer is the one exception (IP13; Round 2, RV2-14);
  - queues are bounded; neither the browser nor any process that serves several renderers ever blocks on one, and no thread waits on a send (Round 2, RV2-13).
- **Threads** ([ADR-0006](adr/0006-event-loop-and-threading.md), [event loop and threading](architecture/event-loop-and-threading.md) EL1–EL24):
  - one wait per thread (kqueue, or run loops for AppKit and the compositor);
  - in the browser, the main thread (AppKit only), a **core** thread (the engine's authority), a compositor thread (the layer tree, scrolling, animation and hit testing) and a worker pool; from M2 the GPU process's own threads validate, rasterize and composite (Round 1, RV1-02);
  - in a renderer, one thread per event loop, a control thread, and a helper pool that never touches a heap;
  - heaps never cross threads, except a DOM arena lent read-only while the borrow checker freezes the heap (parallel style and layout are built only when profiling justifies them), and the heap lent to parallel marking during a pause (Round 1, RV1-04);
  - the compositor owns a display link per window and never waits for a renderer;
  - QoS follows role and visibility;
  - the same loops run under a deterministic scheduler with a virtual clock in single-process test mode, for time-dependent tests; conformance runs in real processes on the host clock (Round 2, RV2-03).

### 4.5 The frame tree, the pipeline and invalidation

[ADR-0007](adr/0007-frame-tree-pipeline-and-input.md); [frame tree and navigation](architecture/frame-tree-and-navigation.md) FT1–FT13, [rendering pipeline](architecture/rendering-pipeline.md) PL1–PL16, [invalidation](architecture/invalidation.md) IV1–IV14, [input and accessibility](architecture/input-and-accessibility.md) IN1–IN12.
- **The browser's core holds the only authoritative frame tree.** Renderers hold skeleton replicas and remote window proxies. Every operation between agents goes through the browser, even in one process (FT6). That is what lets out-of-process iframes arrive by changing process selection only (AC1). **Cross-site frames are separate agents with their own event loops and heaps from M1** (Round 1, RV1-03).
- **Navigation is browser-driven,** through to an acknowledged commit. The process is chosen at the response, and loader and storage endpoints are bound at commit.
- **Five owned stages:** the DOM in the heap; computed styles, the box tree and immutable fragments outside it; then paint into **a display list per frame, of validated primitives over property trees.** Glyphs are rasterized in the renderer, so neither the browser nor the GPU process parses fonts or decodes images (AC4).
- **One string representation,** 8- or 16-bit, for JavaScript and the DOM; names are atoms.
- **A lifecycle the compiler checks,** with two entry points and no synchronous flush across agents. Reuse at every stage. **Every incremental update is checked against a rebuild in tests** (IV14).
- **Display lists go to the GPU/compositor process from M2,** which validates them (PL9), rasterizes their tiles and commits layers to the browser; **the browser scrolls and animates layers between commits,** from the scroll and animation nodes each commit carries (PL15, as amended in Round 1, RV1-02).
- **Input enters at the browser's compositor thread** and is routed by hit testing, from regions the GPU process checks and forwards (PL16). Focus, capture and activation are browser records; IME is served from a mirror, accessibility from a browser-side cache.

### 4.6 The heap and the JavaScript engine

- **The heap** ([ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md), [ADR-0008](adr/0008-script-agents-and-heap.md), [script and heap](architecture/script-and-heap.md) SH1–SH14):
  - one heap per agent, on its event loop's thread, and **DOM objects are the JavaScript objects, with no wrappers**;
  - between agents, only copies, transfers and `SharedArrayBuffer` memory outside the heaps;
  - typed arenas of `u32` handles, bound to `GcScope`/`NoGcScope` tokens, so holding an unrooted handle across a possible collection is a compile error;
  - native holds go on `SliceStack`, a root stack with no `unsafe`, so the `gc` crate is `#![forbid(unsafe_code)]`;
  - hot links are one-based struct-of-arrays `u32` columns;
  - generated setters are the single place for the write barrier;
  - **the collector's end-state is fixed before M3** (Round 1, RV1-04): a generational nursery; incremental marking driven by allocation as well as idle time; parallel marking in pauses; non-moving tenured arenas with selective evacuation. Version 0 is stop-the-world mark–compact. Concurrent marking and the barrier's semantics are decided with spike S11, before M3. Pauses ≤ 4 ms while interacting and ≤ 50 ms in idle time (PF15). The node arena is never compacted;
  - memory that reaches platform APIs lives outside the heap and never moves.
- **The engine** ([ADR-0017](adr/0017-javascript-engine-and-bindings.md), [the JavaScript engine](architecture/javascript-engine.md) JE1–JE32, [bindings](architecture/bindings.md) WB1–WB23):
  - parser, bytecode, interpreter, object model, builtins, `RegExp` and collector are all Axiom's own, on ICU4X and Rust's standard library;
  - compiled scripts are heap-free, so compilation runs on the helper pool and functions compile lazily;
  - a register machine with an accumulator, dispatched by a `match` loop, with explicit tail calls once `become` is stable;
  - NaN-boxed values that carry the scope's lifetime; ropes and slices. **Whether heap slots hold 64-bit NaN-boxed values or compact 32-bit ones is open until spike S11, before M3** (Round 1, RV1-06);
  - ECMAScript 2026 and the 2027 draft's finished proposals. `RegExp` backtracks with a linear-time fallback; `Intl` runs on ICU4X. **Temporal's arithmetic and BigInt are replaceable details behind Axiom's interfaces** (Round 1, RV1-08): `temporal_rs` is graded before M3 and BigInt's candidate before M4, and either may be reused if it passes;
  - a JIT stays under J1–J7, fed by the interpreter's feedback, and is decided at M5's gate; **WebAssembly's architecture is decided before M3,** with spike S14 (own or embedded; WasmGC's objects as arena kinds; frames; tiering), its implementation for M9 (Round 1, RV1-13);
  - **Axiom's own WebIDL compiler** reads webref's IDL and emits safe Rust, split into crates by interface group for build cost (Round 1, RV1-18). Platform objects are heap entries, and remote window proxies offer only HTML's cross-origin properties;
  - console messages reach the embedder's log and the test runners from M3 (JE27; Round 1, RV1-15).

### 4.7 Documents: HTML, the DOM, style, layout and paint

- **HTML and the DOM** ([ADR-0015](adr/0015-html-parsing-and-the-dom.md), [HTML and the DOM](architecture/html-and-dom.md) HD1–HD33):
  - Axiom's own HTML parser, which decodes straight into the DOM's strings and parses in budgeted tasks on the event loop's thread, with a preload scanner on the helper pool. Fragments and the `setHTML` family go through one tree builder that sanitizes while parsing;
  - a non-validating XML parser that fetches no external entity. XSLT is deferred until a use appears (OQ-22);
  - one node arena of `u32` columns, 36 bytes a node, whose slots never move;
  - one mutation path with a fixed list of hooks;
  - events, observers, ranges and selection, Shadow DOM, custom elements with scoped registries, forms submitted through the browser, editing through Input Events and the IME mirror, and the top layer. `execCommand` is marked approximate.
- **Style, layout and paint** ([ADR-0016](adr/0016-style-layout-and-paint.md), [style](architecture/style.md) ST1–ST36, [layout and paint](architecture/layout-and-paint.md) LY1–LY32):
  - Axiom's own CSS parser, generic over the strings' code units, with one property table generating parsers, computed-style groups, impact classes and CSSOM accessors;
  - right-to-left matching with rule buckets, an ancestor Bloom filter and `:has()` invalidation, and **no selector JIT**;
  - Cascade 5 and 6, with immutable computed styles shared through a rule tree, a sharing cache and an evictable cascade cache. A cache-free path is the oracle;
  - container queries resolved during layout, in nested lifecycles;
  - `:visited` by partitioned history: the browser keeps the history, and each renderer receives only its own partitions;
  - a layout unit of 1/64 CSS pixel in an `i32`, saturating; LayoutNG's contract for every algorithm; **tables marked approximate**;
  - paint in CSS 2.1's order, with display items reused by identity; form controls in Axiom's own theme; hit testing in the renderer.

### 4.8 Text, graphics, images and media

- **Text** ([ADR-0011](adr/0011-text-fonts-and-unicode.md), [text and fonts](architecture/text-and-fonts.md) TX1–TX24):
  - ICU4X is the one Unicode source; encodings are Axiom's own, behind an interface (DP2), because Axiom's audit graded `encoding_rs` 0.8.42 `ub-risk-3` with SIMD off (Round 1, RV1-08; [audit](research/encoding-rs-audit.md));
  - web content sees system fonts and web fonts, never fonts the user installed;
  - **CoreText's database and fallback serve system fonts, and CoreText rasterizes them** (TX12);
  - **web fonts never reach CoreText:** Axiom's containers, `read-fonts` and a Rust rasterizer, `vello_cpu`, handle them;
  - **HarfRust shapes every run;**
  - **metrics follow the rasterizer:** for `sbix` faces such as Apple Color Emoji, CoreText's advances and bounds (TX22); `-webkit-font-smoothing` honoured on both paths (TX23); optical sizing from the used size (TX24) (Round 1, RV1-10);
  - glyphs are rasterized in the renderer into atlases;
  - ICU4X segments text, `unicode-bidi` runs the bidi algorithm, and hyphenation is staged.
- **Graphics** ([ADR-0012](adr/0012-graphics-images-and-media.md), [graphics](architecture/graphics.md) GR1–GR14):
  - Metal directly, through `platform`; wgpu wasn't admitted;
  - the window server composites retained CoreAnimation layers, and Axiom redraws only damaged tiles, **in the GPU/compositor process from M2,** into IOSurfaces it owns (Round 1, RV1-02);
  - Axiom's own pipelines;
  - **paths are rasterized in the renderer** (GR5). GPU filling was faster but coarser at the edges, and its amendment is deferred to the graphics milestone (OQ-24);
  - Display P3 at 8 bits, with every surface colour-tagged;
  - canvas 2D in the renderer; **WebGL and WebGPU never in the browser:** M9's plan places them, and their WebGPU implementation is decided then.
- **Images and media** ([images and media](architecture/images-and-media.md) IM1–IM13):
  - images decode in the renderer, **into Mach memory entries only,** never IOSurfaces (Round 1, RV1-09); the GPU process samples them in place, without a copy (S10); safe Rust decoders: `png`, `gif`, `image-webp`, `jxl`, and a JPEG decoder decided at M1's plan, since `jpeg-decoder` is in maintenance mode (Round 1, RV1-22);
  - colour management with `moxcms`;
  - AVIF goes through ImageIO, under DP10's exception, **in the media utility process**, since ImageIO needed sandbox rules the renderer may not have (S5);
  - media runs in a sandboxed utility process with VideoToolbox and AudioToolbox, and video frames get layers of their own.

### 4.9 Networking, storage and workers

[ADR-0013](adr/0013-networking-storage-and-workers.md); [networking](architecture/networking.md) NW1–NW18, [storage and workers](architecture/storage-and-workers.md) SW1–SW13.
- **Protocols are state machines** driven by one I/O thread from one `kevent64` wait, with no async runtime.
- **Axiom's own URL parser and HTTP/1.1, HTTP/2 and HTTP/3 framing,** with `quinn-proto` for QUIC.
- **rustls, with `aws-lc-rs`** (C and assembly, under DP10's exception, in the network process only). Certificates are verified by the system's Security framework **in the worker pool,** never on the I/O thread.
- **DNS-SD** for names and HTTPS records.
- **Partitioning** of connections, TLS sessions, the cache and cookies; third-party cookies blocked.
- **Fetch's policy enforced from bound endpoints:** CORS, CORP, COEP, ORB, mixed content, Referrer-Policy, HSTS, Local Network Access.
- **Web storage lives in the network process,** by storage key, in one engine: `redb` if it passes its audit, otherwise SQLite. `localStorage` is cached in the renderer, and IndexedDB values are never parsed outside it.
- **Service workers** intercept within Fetch. Shared and service workers run in a renderer locked to their site.
- **Web Locks are arbitrated in the browser,** as Chromium's are, so a relaunch of the network process can't break mutual exclusion (Round 2, RV2-04, the owner's choice).
- **Later:** Encrypted Client Hello, planned with DNS over HTTPS at M5's plan: rustls has had experimental client-side ECH since 2024-06 (Round 1, RV1-20).

### 4.10 The shell, the embedder API and accessibility

- **The embedder** ([ADR-0009](adr/0009-embedder-api-and-shell.md), [embedder API](architecture/embedder-api.md) EM1–EM12):
  - the shell is an embedder on AppKit's main thread, and the engine never depends on it;
  - commands go in, and events and requests for decisions come out, each request with a default, listed per request (Round 2, RV2-03); a helper's crash or hang is an event (RV2-04);
  - the browser's own interface is native, never web content;
  - a headless embedder, with offscreen surfaces, synthetic input and virtual time, runs WPT and the reference tests: **without script at M1 and M2,** by a path that loads a URL, waits natively and writes a PNG, and **through a test-only WebDriver server from M3** (Round 1, RV1-01). It dumps the DOM, computed styles, the box and fragment trees, the display list and the network log as text from M1; a helper's crash report is kept locally from M2; console messages reach it from M3 (Round 1, RV1-15);
  - **a pure-Rust shell with objc2.** Its milestone first validates objc2-defined classes for text input and accessibility, with a Swift shell behind the same API as the fallback;
  - one bundle, signed by the stable local identity (Round 1, RV1-17), built by a script without Xcode.
- **Accessibility** ([ADR-0014](adr/0014-accessibility.md), [accessibility](architecture/accessibility.md) AX1–AX12):
  - renderers build each document's tree when an assistive technology asks, and push it, then its updates;
  - the browser answers every query from its cache, stitching frames together by token;
  - text navigation uses macOS's text markers;
  - the bridge is Axiom's own, not AccessKit;
  - system accessibility settings become media features;
  - the settings list and the reserved key equivalents are approved.

### 4.11 Language, `unsafe` and dependencies

- **Rust** ([ADR-0002](adr/0002-implementation-language.md), [language evaluation](technology/language-evaluation.md)), pinned at 1.94.1, 2024 edition, resolver 3; Clippy pedantic. `unsafe` only in crates listed in `tools/unsafe-policy.txt`, in listed modules, with a `// SAFETY:` comment naming its invariant. Parsers, network policy, IPC decoding, and image and font wrappers contain none.
- **Failure semantics** (Round 1, RV1-16): every executable is built with `panic = "abort"`, so a panic ends its process; parsers, decoders and IPC validators use checked arithmetic, and the browser's validators are panic-free by lint.
- **Build thresholds T1–T4:**

  | # | Measure | Target | Revisit above |
  |---|---|---|---|
  | T1 | Incremental rebuild and link after editing one leaf crate | 5 s | 30 s |
  | T2 | Clean dev build of the workspace | 5 min | 20 min |
  | T3 | Peak compiler and linker memory, summed | 3 GiB | 5 GiB |
  | T4 | Peak memory of one `rustc` | 1.5 GiB | 3 GiB |

  Generated code is measured early: style's property code at M1's third checkpoint, generated bindings by spike S13 before M3 (Round 1, RV1-18).
- **Dependencies** ([ADR-0010](adr/0010-dependency-and-supply-chain-policy.md), [dependency policy](technology/dependency-policy.md) DP1–DP23, [register](technology/dependency-register.md)):
  - the core is Axiom's own, and Axiom never writes its own cryptography;
  - **the reuse line** (Round 1, RV1-08): encodings, Temporal's arithmetic and BigInt are replaceable details behind Axiom's interfaces (DP1 (g), DP2), reused if a candidate passes DP8; the parser, interpreter, object model and collector stay Axiom's own;
  - each dependency has a class (required platform facility, replaceable implementation detail, or foundational dependency) and a register entry for its whole tree;
  - grades on Google's `ub-risk` scale, with `ub-risk-3` and `ub-risk-4` barred, and Axiom's own audit allowed (DP15), as [`encoding_rs`'s](research/encoding-rs-audit.md) was;
  - a reused parser of untrusted input must be pure Rust at `ub-risk-2` or better, and `ub-risk-1` in the browser;
  - C parsing untrusted input runs only in a sandboxed helper, with no Rust alternative and an ADR;
  - crates.io only, `--locked` builds, `cargo-vet` importing published audits, `cargo-deny`;
  - Axiom's license is "MIT OR Apache-2.0".
- **Foundational dependencies:** `objc2` and its framework crates, ICU4X, rustls. **Open risk:** `smallvec`, which HarfRust and ICU4X require, carries a published `ub-risk-3` grade on an old version, so the version adopted must be graded first.

### 4.12 Crates

The proposed crates ([overview](architecture/overview.md) §2), each named and split for real only when code is authorized:
- `platform`, with all FFI, and **the only crate with `unsafe`**, apart from a later `jit` and the shell's own `platform` module;
- `gc`, with none;
- `js`, `webidl` and `bindings`, the generated bindings split by interface group (Round 1, RV1-18);
- `html`, `xml` and `dom`; `css` and `style`; `layout` and `paint`;
- `text` and `image`;
- `ipc`, `idl` and `event-loop`;
- `core`, `frame-tree` and `embedder-api`;
- `compositor` and `display-list`, in the GPU/compositor process from M2, and `presenter`, the browser's half of compositing: the layer tree, scrolling, animation and hit testing (Round 1, RV1-02);
- `net` and `storage`;
- `headless` and `shell`.

**Executables:** `axiom` (the shell and the browser), `axiom-renderer` and `axiom-network` from M1 (Round 1, RV1-07); `axiom-gpu` from M2 (RV1-02); later `axiom-renderer-jit` and `axiom-utility`.

**Dependency rules:** engine crates never depend on `shell`; renderer crates reach browser crates only through the protocol crates; `net` and `storage` never depend on renderer crates.

### 4.13 The corpus, standards, testing and performance

- **The corpus** ([corpus](corpus.md), CO1–CO14): the owner's 13 sites in tiers:
  - documents: Wikipedia, MDN;
  - content: Google Search, Instagram, Reddit;
  - media: YouTube, Twitch; media and maps: Google Maps;
  - applications: Discord, Three.js's WebGPU example, Figma, VS Code for the Web, Prism.

  **Automation loads only the five pages whose sites' terms and robots files permit it** (Wikipedia, Maps, MDN, Three.js, VS Code), at most five times a census, identified, never signed in. The other eight are measured by hand, with process sampling only. Copies are kept only where terms and licences allow, and never committed. **A census addendum by hand,** authorized before M4, saves each page's use counters and memory-infra traces on normalized feeds (Round 1, RV1-05, RV1-12).
- **Standards** ([standards](standards.md), SB1–SB14):
  - specifications are pinned by body, and moved only at a milestone's start;
  - five support classes, and an API is exposed only when it works;
  - four approximate behaviours are listed with their reasons: `execCommand`, automatic table layout, `Error.prototype.stack`, and `Date.parse`'s legacy formats;
  - priorities come from the corpus, then Baseline status, then Interop;
  - **a completed area targets the median of Chrome's, Firefox's and Safari's pass rates** on each WPT directory, rounded down, calibrated on wpt.fyi at WPT `0f26972422`;
  - **site compatibility** (Round 1, RV1-14): a Safari-compatible user agent with an Axiom token (SB13); per-site interventions as data outside the engine, each with its evidence and removal condition (SB14);
  - the not-planned list was approved at Gate B: DRM, plugins, AppCache and Web SQL, one-engine device APIs, WebXR, Gamepad, Web MIDI, Web Speech, Push, extensions; Round 1 added in-engine PDF viewing (PDFs open in Preview) and WebRTC (RV1-22).
- **Testing** ([testing](testing.md), TS1–TS18):
  - eight layers of tests;
  - **the process configuration** (Round 2, RV2-03, the owner's choice): conformance in real, sandboxed processes on the host clock; time-dependent and scheduler tests in single-process mode on the virtual clock; visual reference tests under both at each milestone's gate;
  - WPT through wptrunner's `axiom` product with two executors: **a script-free reference-test executor at M1 and M2,** and a test-only WebDriver server in the headless embedder **from M3** (Round 1, RV1-01);
  - Axiom's own runners for html5lib-tests, test262 and Unicode's files;
  - test-only differential oracles (html5ever, xml5ever, Taffy, `regress`, `temporal_rs`, `num-bigint`) under virtual time;
  - every parser and IPC decoder fuzzed, with AddressSanitizer and Miri; **concurrency checked in order** (Round 1, RV1-11): Miri's race detector, loom on the primitives, schedule exploration on EL24, then ThreadSanitizer by S12's routes;
  - V1–V9 built as tests;
  - weekly advisory checks, and the OS-dependent checks after every macOS update;
  - this Mac is authoritative.
- **Performance** ([performance](performance.md), PF1–PF24):
  - **the conditions** (PF1): a release build, AC power with Low Power Mode off, a load average below 3.0, cool-downs, three runs, the median with its range;
  - **a page's cost** includes the GPU and compositor memory allocated on its behalf (PF4);
  - frames from S6's budgets, and the GPU process's hop to the browser within 1 ms (PF13; S10 measured 48.8 µs); no task over 50 ms; pauses as in §4.6;
  - no idle wakeups on a static page, and Chrome's throttling of hidden pages; energy no higher than Safari's on animating pages;
  - the build within T1–T4.

  **The ceilings** are half of Chrome Canary 158's measured cost on 2026-10-20, capped by tier (documents 100 MB, content 150, media and maps 300, applications 300), rounded down to 5 MB. **The content tier is gated differently since Round 1** (RV1-05): on a normalized workload, scrolled to the 50th item, by a slope at most half of Chrome's and a plateau after idle, with 150 MB a stretch target reported at each gate:

  | Page | Ceiling | Chrome's cost (S8) | Share of Chrome's cost |
  |---|---|---|---|
  | Wikipedia | 100 MB | 319 MB | 31% |
  | MDN | 70 MB | 145 MB | 48% |
  | Google Search | 150 MB, a stretch target; gated by slope and plateau | 582 MB, signed in | 26% |
  | Instagram | 150 MB, a stretch target; gated by slope and plateau | 578 MB, signed in | 26% |
  | Reddit | 150 MB, a stretch target; gated by slope and plateau | 1,260 MB, signed in | 12% |
  | YouTube | 300 MB | 908 MB, signed in | 33% |
  | Twitch | 200 MB | 401 MB, signed in | 50% |
  | Google Maps | 300 MB | 627 MB | 48% |
  | Discord | 210 MB | 427 MB, signed in | 49% |
  | Three.js | 235 MB | 470 MB | 50% |
  | Figma | 300 MB | 1,111 MB, signed in | 27% |
  | VS Code for the Web | 225 MB | 457 MB | 49% |
  | Prism | 245 MB | 494 MB, signed in | 50% |

### 4.14 The roadmap and Milestone 1

- **The milestones** ([roadmap](roadmap.md), MS1–MS12), ordered and gated, never dated, as amended in Round 1:

  | # | Milestone | Level | Corpus targets |
  |---|---|---|---|
  | M1 | Static documents: **the browser with sandboxed renderer and network processes** (RV1-07), the shell and the headless embedder; HTML and the DOM without script; CSS 2.1's visual model with tables; system fonts; images; paint and the compositor (a browser thread); HTTP/1.1 and HTTP/2 over rustls | L0 | Wikipedia and MDN, readable |
  | M2 | The sandboxed engine: **the GPU/compositor process** (RV1-02); audit tokens; V1's and V2's L1 parts; crashes contained; the disk cache, the cookie jar and CORS | L1 | The same pages, sandboxed |
  | M3 | Script: the engine, bindings, the event loop, `fetch`, Web Storage, canvas 2D; WebAssembly's architecture decided (RV1-13) | L1 | MDN in full, its live examples in cross-site frames |
  | M4 | The modern document: flexbox, grid, SVG, style's second stage, Shadow DOM, web fonts, the accessibility tree, HTTP/3, **WebSockets** (RV1-12) | L1 | Wikipedia in full; Google Search, Instagram (without video), Reddit (without its interactive posts in frames), logged out |
  | M5 | Site isolation and logins: site locks; ORB, COEP, CORP; IndexedDB; **downloads, find in page and zoom** (RV1-22); the JIT decided (RV1-13) | L2 | The content sites, signed in with test accounts |
  | M6 | The sensitive-use gate: out-of-process iframes; V1–V9 complete; **session restore** (RV1-22) | L3 | The content sites with the owner's accounts |
  | M7 | Media: the media utility process, MSE, video layers, AVIF | L3 | YouTube, Twitch |
  | M8 | Applications: workers, `SharedArrayBuffer`, the engine's second stage, EditContext | L3 | VS Code for the Web, Discord, Prism |
  | M9 | The GPU and WebAssembly: WebGL and WebGPU in a process M9's plan places, never the browser; WebAssembly, as OQ-3 decides | L3 | Google Maps, Three.js, Figma |

  **No script from the web before L1** (MS3): M1's renderers are sandboxed processes, but L0 still runs no web script. **Every interface from M1** (MS4), with single-process mode kept strictly for automated tests, fuzzing and Miri (Round 1, RV1-07). After M9, the designs' remaining "later" rows, printing among them, each get a milestone planned when it starts.
- **Milestone 1** ([Milestone 1](milestone-1.md), MA1–MA12):
  - **scope:** static documents at L0, with CSS 2.1's visual formatting model without paged media, in the browser with sandboxed renderer and network processes under S4's, S5's and S7's profiles (Round 1, RV1-07). No script, cookies, storage, web fonts, SVG, flexbox or grid; the GPU/compositor process is M2's;
  - **conformance:** html5lib's tokenizer tests at 100%; the 1,973 tree-construction tests in WPT's `.dat` files at ≥ 96%; **`css/CSS2`'s reference tests at ≥ 97% of the 5,652 that need no script** (196 of the 5,848 in scope contain script), the median of Chrome, Firefox and Safari on those same tests, an interim target, with the full 5,848 at M3 (Round 1, RV1-01); Unicode's line-breaking and bidi files at 100%. M1 runs reference and crash tests through the script-free executor, end to end from its second checkpoint;
  - **the corpus:** Wikipedia and MDN readable, from dated copies and live; links navigate; 10 loads with no crash or hang;
  - **budgets:** the browser idle ≤ 85.4 MB with the compositor's tiles, a renderer ≤ 10 MB, the network process ≤ 25 MB; Wikipedia ≤ 100 MB and MDN ≤ 70 MB above it; S6's frame budgets; no task over 50 ms; no idle wakeups; pauses within 4 ms; T1–T4;
  - **security:** no script engine linked; no credentials by construction, with password forms refused below L2; the helpers apply their profiles before touching input, and every renderer profile denies `iokit-open`; every executable aborts on a panic; the network process killed and stalled, and a renderer that stops receiving, as tests (Round 2, RV2-04, RV2-13); every parser and decoder fuzzed at least an hour, with no open finding; the concurrency checks in TS11's order;
  - **starting it** needs the owner's authorization of MA10's list: the branch, the skeleton (signed by the stable local identity), the dependencies, the test data, the tools (a nightly whose ThreadSanitizer works, as S12 found), and the live loads. **On hold.**

## 5. Empirical findings: spikes S1–S10, S12 and S15

| Spike | Step | Question | Result |
|---|---|---|---|
| [S1](research/s1-results.md) | 2b | Which GC handle and rooting pattern: gc-arena style (A) or Nova style (B)? | **B.** Neither dominates at run time; B has **no `unsafe` at all,** A 24 blocks. B collects 1.3–2.8× faster from 64k elements, with 13% less memory per live node; A chases pointers faster, and B's dispatch took 2.06× A's time. Decided by R2: under A, every builtin that calls back into script would have to be resumable |
| [S2](research/s2-results.md) | 2b | Do AppKit, Metal and CoreText work from Rust with `unsafe` contained, and at what cost? | **Yes.** A window drew a CoreText line through Metal, matching the predicted ink pixel for pixel; 19 `unsafe` blocks, all in `platform`. An idle window: 0.0% CPU, no wakeups, 20–21 MB. **No offline Metal compiler without Xcode,** so shaders compile at run time (about 0.5 s the first time). objc2's default features cost 29 s and 1.4 GiB to build, against 6.0 s trimmed |
| [S3](research/s3-results.md) | 4 | Can rooting cost ≤ 1 ns, and how should the interpreter dispatch? | **Reads yes, pushes no** as S1 measured them: `SliceStack` reads in 0.53 ns with no `unsafe`, and pushes in 2.12 ns by S1's method (0.58 ns back to back). Rooting took 0.8–1.0% of an interpreter workload. **A `match` loop** dispatches fastest on stable Rust (2.64 ns an instruction); struct-of-arrays columns traverse 31–36% faster; setters are free, card marking about 3% |
| [S4](research/s4-results.md) | 3 | Do custom Seatbelt profiles work for ad-hoc-signed helpers on macOS 27? | **Yes,** by three methods, and all 18 probes behaved as specified. **A named profile kills the process under SDK 27.** Browser-compiled bytecode costs nothing measurable (2.11 ms, 1.14 MiB); Mach round trips are 24% faster than a socket pair. **`(deny default)` doesn't stop `MAP_JIT`;** only an explicit rule does. CoreText works inside, with fonts and ICU data readable, and **fails silently** without them |
| [S5](research/s5-results.md) | 5 | Does HarfRust shape like CoreText? How do Rust rasterizers compare? Do web fonts go through safely? ImageIO in the renderer? | HarfRust chose **the same glyphs in all 432 runs;** 14 placed marks differently, in a font with no positioning data. CoreText rasterizes system fonts, as recommended and approved. 322 WPT fonts and **4,000 malformed variants: no crash, no hang,** no sandbox denial. **ImageIO needs rules the renderer may not have,** so AVIF moved to the media process; `jpeg-decoder` runs at 1.1–1.8× ImageIO's time, with deterministic pixels where ImageIO's varied |
| [S6](research/s6-results.md) | 5 | What does compositing with Metal and CoreAnimation cost? | **Passes at the median:** a typical update took 0.12–0.28 ms on the GPU and 0.58–1.93 ms on the compositor thread. A full redraw took longer than a 60 Hz frame, so **damage tracking is required.** The slowest frames allocate surfaces. **Tiles took 25.2 MB for one window,** so B1 was revised. GPU path filling was 3.8–5.6× sooner with 31–62× less memory, but edges were up to 109 of 255 levels off. Colour tags are honoured exactly; untagged surfaces aren't converted |
| [S7](research/s7-results.md) | 5 | Does certificate verification work in the network sandbox? rustls or Network framework? DNS-SD? One wait? | **Passes:** eight profile rules, each explained. **Network framework used 3.3× and 2.4× rustls's instructions** per full handshake. **Trust evaluation blocks its caller** (8.4 ms median, up to 1.5 s seen), so it runs in the worker pool. DNS-SD needs one thread. One I/O thread ran TLS, HTTP/3, DNS and IPC from one wait, with no idle wakeups. AWS-LC's entropy source costs 29–43 ms of CPU once per process. Post-quantum key exchange needs rustls's default feature |
| [S8](research/s8-results.md) | 6 | What do the corpus pages use and cost in shipping browsers here? | **Chrome, automated:** Maps 627 MB, Three.js 470, VS Code 457, Wikipedia 319, MDN 145 above about:blank; 72 of the 116 web-features the five pages use are widely available. **Chrome, signed in, by hand:** Reddit 1,260 MB, Figma 1,111, YouTube 908, Google Search 582, Instagram 578, Prism 494, Discord 427, Twitch 401. **Safari, logged out, by hand,** with the owner's advanced tasks: Maps 1,880 MB, YouTube 1,624, VS Code 1,076, Twitch 795, Reddit 736, Google Search 568, Three.js 433, Wikipedia 385, Instagram 278, MDN 181 |
| [S9](research/s9-results.md) | 6 | Do cargo-fuzz with AddressSanitizer, ThreadSanitizer and wptrunner work on this Mac? | **Fuzzing passes:** the planted bug fell in about a second; 10 minutes without a finding; 93.25% region coverage; a fuzzer held about 0.5 GB. **ThreadSanitizer fails** with the installed nightly (2026-04-08): its runtime crashes before `main` on macOS 27.2, while Apple clang's works. **wptrunner passes:** 435 DOM tests in 140 s at 99.6%, against wpt.fyi's 99.5% |
| [S10](research/s10-results.md) | Review | The GPU/compositor process's hop to the browser; zero-copy resources; display-list validation (RV1-02, RV1-09) | **Passes:** the layer commit reached the browser's CoreAnimation commit in **48.8 µs at the median** (118.7 µs at the 99th percentile), against PF13's 1 ms. A renderer's 4 MiB image registered from its memory entry in 18–62 µs, **without a copy**, under RV1-09's renderer profile; a blit would hold it twice. **Validation cost about 9.4 ns an item.** The GPU process's profile needs ten rules, each checked by removal, and its cache directory resolved before the profile applies. **Tiles are charged to the GPU process,** which creates them |
| [S12](research/s12-results.md) | Review | Can ThreadSanitizer run on macOS 27 (RV1-11)? | **Both routes work:** the nightly of 2026-10-21 with its own runtime, and the April nightly with `-Z external-clangrt` and the Command Line Tools' runtime. Each reports S9's planted race and passes the clean test |
| [S15](research/s15-results.md) | Review | Does a stable self-signed identity behave as ad-hoc signing, and keep the system's decisions across rebuilds (RV1-17)? | **Yes for the sandbox and the Keychain:** S4's probes, the JIT matrix, CoreText's profile, start-up and IPC behave exactly as under ad-hoc signing, and a rebuild keeps its Keychain item without a dialog. Privacy permissions (TCC) are checked by hand at M1, with the shell |

**Authorized, not yet run** (the review's §5.2): S11, the collector's pauses and the slot width, and S13, the build cost of generated bindings, before M3; S14, WebAssembly interpreted and compiled, before M3; the census addendum, by hand, before M4.

**Findings that changed the design:**
- **S1, S3:** Pattern B and `SliceStack`, so the collector needs no `unsafe`.
- **S4:** browser-compiled profiles, and J3's explicit denial of code generation.
- **S5:** AVIF out of the renderer, a check that each font is file-backed (CoreText substitutes a stub for PingFang when its file is unreadable), and Axiom applying a font's track after shaping.
- **S6:** surfaces allocated ahead of need, every surface tagged, and B1 revised for tiles.
- **S7:** verification off the I/O thread, the network profile's two metadata rules, and the post-quantum feature kept on.
- **S8:** every page's ceiling. Signed-in feeds cost as much as applications, and on the GPU pages the GPU process carries much of the cost.
- **S9:** M1 pins a nightly whose ThreadSanitizer works; S9 also found a counting error in the pass-rate tool, corrected (§11, E2).
- **S10:** the GPU process's ten profile rules and its one exception to D2; B1b kept at 15 MB; the one ordered stream of layer commits kept.
- **S12:** MA7's ThreadSanitizer route: a nightly of 2026-10-21 or later, with `-Z external-clangrt` as the fallback.
- **S15:** MA10 signs with the stable local identity, created by the owner at M1's start.

**The measurement record** (assessment): the formal runs were on AC power with Low Power Mode off, and each results document records its conditions. Not every run met PF1's load threshold of 3.0, and the documents say so where it was exceeded. A run made on battery was discarded and repeated.

## 6. Trade-offs

| Trade-off | Chosen | Given up | Recorded in |
|---|---|---|---|
| Implementation language | Rust, with `unsafe` confined | C++'s ecosystem and engine precedent | ADR-0002 |
| JavaScript engine | Axiom's own, sharing the DOM's heap | An embedded engine's maturity and speed; embedding later would cost a rewrite of the DOM's memory model | ADR-0001 R2, ADR-0017 |
| GC pattern | Pattern B: lexical ceremony (`bind`, `scope`), a moving collector | Pattern A's faster pointer chasing; A's cost would have been resumable state machines everywhere | ADR-0003 |
| Collection | Stop-the-world mark–compact as version 0, with the end-state fixed before M3: a nursery, incremental and parallel marking, selective evacuation (Round 1, RV1-04) | Concurrent marking, until S11 measures it; a simpler collector for longer | ADR-0008 SH10, PF15 |
| JIT | An interpreter first; a JIT on evidence, never before L2, in one executable, decided at M5's gate | Speed on script-heavy pages until then | ADR-0004 J1–J7, ADR-0017 |
| **Processes at M1** | **Real, sandboxed renderer and network processes from M1,** with single-process mode for tests only (Round 1, RV1-07) | A smaller first milestone in one process | Roadmap MS3, MS4, §10 |
| **Compositing** | **A sandboxed GPU/compositor process from M2** that validates, rasterizes and composites; the browser keeps the layer tree, scrolling, animation and input (Round 1, RV1-02) | One process fewer, and a hop measured at 48.8 µs (S10) | ADR-0004 D1, GR14, PL15 |
| Cross-site iframes | In the embedder's process at L1 and non-sensitive L2 (SI3), always as separate agents (RV1-03); out-of-process iframes at M6 | Isolation of cross-site frames before L3 | ADR-0004 D4, OQ-18 |
| Sandbox mechanism | Custom Seatbelt profiles through libsandbox's undocumented functions (option O1), as Chromium, Firefox and WebKit do | App Sandbox entitlements (O2), public, but its base profile reaches the window server, font server, pasteboard and LaunchServices (170 Mach services), and a child can't be stricter than its parent | ADR-0004 D2, the [macOS note](research/macos-process-confinement.md) |
| The browser process | Unsandboxed, under the hardened runtime; from M2 it consumes no display lists | A sandboxed browser process, which ADR-0004 revisits if a public route to strict helper sandboxes appears | ADR-0004 D1, GR10 |
| IPC transport | Mach messages | Unix sockets' simplicity | ADR-0005 |
| **Text** | **CoreText for system fonts** (database, fallback, rasterization), with metrics from the same source (TX22); **HarfRust for all shaping; a Rust path (`read-fonts`, `skrifa`, `vello_cpu`) for web fonts** | One text path: macOS's look for web fonts too, or Rust everywhere with system fonts differing from macOS (the single-rasterizer alternative, rejected by the owner in Round 1) | ADR-0011 TX6–TX12, TX22–TX24, S5 |
| Paths | Rasterized in the renderer, with analytic edges | GPU filling's speed and memory (GR5's amendment deferred) | ADR-0012 GR5, OQ-24 |
| Graphics interface | Metal directly | A portability layer (wgpu) and its `unsafe` tree | GR1 |
| Renderers' pixels | Mach memory entries, sampled in place by the GPU process (RV1-09, S10) | IOSurfaces in renderers, which need an IOKit user client | IM1, GR10, PL8 |
| AVIF | ImageIO (C) in the media utility process | A safe AV1 decoder, which doesn't exist yet | IM3 |
| HTTP | Axiom's own framing, staged: HTTP/1.1 and HTTP/2 first | Reusing an HTTP stack | NW3–NW5 |
| TLS | rustls with `aws-lc-rs` (C) and the system's verifier | Network framework: 2.4–3.3× the instructions per handshake, and no resumption as tried | NW6, S7 |
| Storage engine | `redb` if audited, otherwise SQLite | — | SW2 |
| Bindings | Axiom's own WebIDL compiler; platform objects as heap entries; split by interface group | Wrappers and an existing generator | WB1–WB16, WB22 |
| Style | No selector JIT; shared immutable styles; tables approximate | A selector JIT's speed; exact table layout | ST7–ST11, LY10 |
| Memory ceilings | Half of Chrome's cost, capped by tier; the content tier gated by a slope and a plateau (RV1-05) | Ceilings that shipping browsers already meet | ADR-0019, PF3 |
| Measurement | Eight sites by hand, within their terms | Repeatable automated baselines for those sites | CO3–CO6 |
| Shell | Pure Rust with objc2 | Swift's direct AppKit API (the fallback) | ADR-0009 EM11 |

## 7. Deferred decisions

Each is taken by the owner at its milestone, on its evidence ([roadmap](roadmap.md) §7, MS9). A decision that changes an accepted ADR is an amendment (MS12).

| Decision | Where it stands | Taken at |
|---|---|---|
| WebAssembly: our own engine or an embedded one (OQ-3) | Open; JE31 lists what Axiom's engine must offer; spike S14 authorized | **Its architecture before M3** (Round 1, RV1-13); its implementation for M9 |
| Whether to build a JIT (J1) | Policy accepted | **M5's gate,** on M3's and M4's measurements (Round 1, RV1-13) |
| The collector's concurrent marking and barrier semantics; the heap-slot width | The end-state fixed (RV1-04); the slot width open (RV1-06); spike S11 authorized | Before M3 |
| The GPU process for WebGL and WebGPU, and the WebGPU implementation | The GPU/compositor process exists from M2 (RV1-02); WebGL and WebGPU never in the browser | M9's plan |
| Passkeys (OQ-16) | Open; roaming keys over USB HID, and the hybrid transport (RV1-21) | Before M6 starts |
| Xcode (OQ-7) | Open; no design needs it | M1's plan |
| QoS overrides on one-way Mach messages (OQ-20) | Open | M2 (M1 already sends Mach messages between processes, so M1's plan may measure it first) |
| DNS over HTTPS, with Encrypted Client Hello | Later; rustls has client-side ECH (RV1-20) | M5's plan |
| GR5's amendment, path items in PL7 (OQ-24) | Deferred by the owner | M4, with path-heavy SVG |
| Surfaces allocated ahead of need (S6) | Planned | M1 |
| `vello_cpu`'s and `smallvec`'s grades | Adopted subject to grading; `smallvec`'s grade open | M1 |
| JPEG's decoder: Axiom's own, or a graded alternative (RV1-22) | Open | M1's plan |
| `temporal_rs`'s grade; BigInt's candidate (RV1-08) | Replaceable details, to be graded | Before M3; before M4 |
| The generated bindings' build cost (RV1-18) | Spike S13 authorized | Before M3 |
| The storage engine (SW2) | Open | M3 |
| AWS-LC's jitter entropy source (S7) | Open | M1, when TLS arrives |
| objc2's text-input and accessibility classes (EM11) | To validate first | M1 |
| Hosted runners (TS17); the nightly to pin | The owner's choice; S12 found a working nightly | M1's start |
| The census addendum (RV1-05, RV1-12) | Authorized, by the owner's hand | Before M4 |
| Demuxers; media instances per site | Open | M7 |
| Password saving | Open | M6 |
| Parallel style and layout (PL10) | Allowed by design | When profiling justifies them |
| XSLT (OQ-22) | Deferred by the owner | When a site the owner uses needs it |
| Printing, anchor positioning, view transitions, vertical text, colour fonts, JPEG XL, WebTransport and others | Later rows of the designs | After M9, each in a milestone planned when it starts |

## 8. Assumptions

| Assumption | Source | Status, or where it is tested |
|---|---|---|
| One human developer, with an assistant; no external audit | Charter §7, OQ-9, MS11 | Assumed; an external review of the architecture is the owner's next step |
| No deadline; work paced by checkpoints | OQ-1 | Owner's decision |
| This Mac is the performance floor, and budgets are set on it | Charter §4 | ADR-0019's revisit condition: the Mac is replaced |
| Custom Seatbelt profiles keep working for binaries built against current SDKs | ADR-0004 | S4 confirmed on macOS 27.2, S10 for the GPU process; V1 reruns after every macOS update (TS16) |
| Mach messages stay available between helpers signed by the local identity | ADR-0005 | S4; S15 for the identity |
| The interfaces hold from L0 to L3 without a rewrite | AC1, MS4 | By design; first tested at M1 (real processes), M2 (the GPU process), M5 (site locks) and M6 (out-of-process iframes) |
| Skeleton replicas suffice for synchronous script semantics | ADR-0007 | At the frames milestone |
| Validating display lists is cheap enough at display rate | ADR-0007 | **Measured** by S10: about 9.4 ns an item, in the GPU process |
| Safe-Rust decoders keep up with display-list traffic | ADR-0005 | S10's hop for one page; real pages when built |
| A thread per agent stays cheap at B5's process counts | ADR-0006 | To measure |
| Most rooting traffic comes from builtins, not bytecode | ADR-0003 | **Confirmed** by S3-Q2 |
| `u32` handles per arena suffice for one agent's heap | ADR-0003 | Assumed |
| An interpreter with inline caches meets the first milestones' budgets on the corpus | ADR-0017 | From M3; the JIT decided at M5's gate |
| HarfRust keeps tracking HarfBuzz; ICU4X's data fits the renderer's budget | ADR-0011 | S5-Q4 for the data; tracking watched |
| objc2-defined classes can implement `NSTextInputClient` and `NSAccessibility` | ADR-0009, ADR-0014 | At M1 (EM11) |
| Chrome's figures of 2026-10-20 represent the corpus pages; the ceilings are frozen | ADR-0019 | Rechecked against a fresh Chrome at each milestone; the content tier by the census addendum's slope |
| wpt.fyi keeps publishing aligned runs | ADR-0018 | Watched |
| The sites' terms stay as read on 2026-10-20 | ADR-0018, CO3 | Read again before each census and milestone |
| Partitioning third-party state is compatible with the corpus | ADR-0004 | Safari and Firefox do it by default (background) |
| The owner can name the sites they consider sensitive | ADR-0004 | At L3 |
| A Rust nightly with a working ThreadSanitizer exists by M1's start | ADR-0021 | **Confirmed** by S12 (a nightly of 2026-10-21) |
| Published audits stay importable for `cargo-vet` | ADR-0010 | Watched |

## 9. Known risk areas

The first five are the ones the owner named at Gate C; Round 1 answered each (the review's focus areas).

| Risk | What is known | Mitigation in the plan | Assessment |
|---|---|---|---|
| **ThreadSanitizer on macOS 27** | The installed nightly's runtime crashed before `main` (S9-Q2); a nightly of 2026-10-21 works, and so does `-Z external-clangrt` (S12) | TS11's order: Miri's race detector, loom, schedule exploration on EL24, then ThreadSanitizer (Round 1, RV1-11) | Low impact while threads share only immutable data; the risk grows with `platform`'s FFI and any lock-free code |
| **The content tier's memory** | 12–26% of Chrome's cost, signed in, at an uncontrolled scroll depth (S8 §7.4) | Gated by a slope and a plateau on a normalized workload, with 150 MB a stretch target; Chrome's slope from the census addendum (Round 1, RV1-05) | A feed keeps what its own script holds, so the slope measures the engine better than a cap |
| **Processes from M1** | M1 runs sandboxed renderer and network processes (Round 1, RV1-07); M2 adds the GPU process; M5 site locks; M6 out-of-process iframes | In-process channels always encode and decode (IP2); every operation between agents goes through the browser (FT6); a deterministic scheduler in single-process test mode | M1 is larger, but shared memory, crashes, sandbox denials and per-process budgets are exercised from the first milestone |
| **A JIT no earlier than L2** | Policy only; whether to build one is decided at M5's gate (Round 1, RV1-13) | The interpreter's feedback feeds any JIT (JE30); a JIT lives only in `renderer-jit` | App-web pages (VS Code, Figma, Discord) may be slow until then. A WebAssembly compiler counts as a JIT too (J6) |
| **HarfRust and `skrifa` beside CoreText** | Two rasterizers: CoreText for system fonts, `vello_cpu` for web fonts. S5 measured the differences: CoreText's ink 0.10–0.18 px higher at 12–16 px on a 1× display; font smoothing adds 16–52% ink; Apple Color Emoji's metrics | Metrics from the rasterizer that draws (TX22); `-webkit-font-smoothing` and optical sizing (TX23, TX24); TX13 emulates smoothing for web fonts, with a calibration metric rerun after macOS updates (Round 1, RV1-10) | System and web fonts may look slightly different side by side. `smallvec`'s grade gates HarfRust and ICU4X, and HarfRust is young |
| The scope, against one developer | An own JS engine, an own WebIDL compiler, own HTTP framing, own parsers and layout | No deadline (principle 7); checkpoints; differential oracles; staged milestones; encodings, Temporal's arithmetic and BigInt as replaceable details (Round 1, RV1-08) | ADR-0001 rates it high in likelihood and impact |
| Interpreter speed on the app web | No measurement yet on real scripts | J1's evidence path, decided at M5's gate; S3's dispatch results | High impact at M8 |
| The sandbox's undocumented interfaces | libsandbox's functions are private; SDK 27 already kills named profiles (S4) | Fallback O2; V1 after each macOS update; the GPU process's rules each checked by removal (S10) | A macOS change could stop the sandbox, and with it L1–L3 |
| No automatic security updates | Axiom is built locally | Weekly advisory checks, rebuilds (TS15) | The routine depends on the owner's discipline |
| System parsers inside renderers | CoreText parses system fonts in renderers (not web fonts); ImageIO runs only in the media process | Sandboxes; web fonts never reach CoreText | Accepted by SC2 |
| Passkeys | The platform authenticator may need an Apple entitlement that a local identity can't claim (OQ-16) | Roaming security keys over USB HID, and the hybrid transport (Round 1, RV1-21); decided before M6 | Could limit which accounts reach L3 |
| `smallvec`'s grade | `ub-risk-3` on an old version | Grade the version adopted (DP8) | Blocks HarfRust and ICU4X if it stays barred |
| Milestone 1's size and its CSS 2 target | The whole static pipeline, now with real processes; 97% of the 5,652 script-free tests, where Servo has 93.7% and Ladybird 91.2% of the 5,848 | MA11's checkpoints, with the harness end to end at the second; an interim target (SB11; Round 1, RV1-01) | Likely a long first milestone |
| Memory on 8 GB | Shipping browsers' costs per corpus page run from 145 MB to 1.9 GB above their baselines (S8) | Per-process budgets, discarding, B5 | — |
| WebAssembly arrives last | Maps, Three.js and Figma need M9; VS Code uses WebAssembly | Its architecture decided before M3 with S14, so WasmGC's integration with the heap is known before M3 fixes the object model (Round 1, RV1-13); MS12 lets a target move at a gate | The implementation still lands after L3 |
| Collector pauses on large heaps | About 4 ms at 130k live nodes and 15 ms at 333k objects (S1) | The end-state fixed before M3; S11 measures pauses on a million-object heap (Round 1, RV1-04) | Application pages will test PF15 first |
| C in the network process | `aws-lc-rs` under DP10's exception | Network process only, sandboxed | Accepted at ADR-0013 |
| Trust evaluation latency | 8.4 ms median, up to 1.5 s seen (S7) | The worker pool | — |
| Measurements by hand | Wide spread (Reddit 495–1,631 MB in Chrome); interaction differs between sessions | Dated, with their spread; hypotheses until a milestone measures; a normalized workload for the content tier (RV1-05) | The ceilings inherit that noise |

## 10. Open questions

**In the [open questions](open-questions.md):**

| ID | Question | Status |
|---|---|---|
| OQ-3 | WebAssembly: our own engine or an embedded one? | Open; its architecture before M3, with S14 (Round 1, RV1-13) |
| OQ-7 | When to install Xcode? | Open; M1's plan |
| OQ-9 | Is the owner the only human developer? | Assumed yes |
| OQ-16 | WebAuthn and passkeys without a paid developer account? | Open; the hybrid transport added (RV1-21); before M6 |
| OQ-20 | Does a QoS override on a one-way Mach message boost a receiver in `kevent64`? | Open; measured at M2, or at M1, which already has processes |
| OQ-22 | XSLT? | Deferred until a use appears |
| OQ-24 | GR5's amendment: path items in PL7? | Deferred to M4; since Round 1 path items would reach the sandboxed GPU process, not the browser |
| OQ-25 | Is the content tier's 150 MB cap right? | **Answered in Round 1** (RV1-05): a slope and a plateau on a normalized workload, 150 MB a stretch target |

**In the documents:**
- **The corpus** ([corpus](corpus.md) §10): the eight hand-measured pages' features, which stay inference until the census addendum; Safari's remote automation, off unless the owner enables it; when terms are read again.
- **The roadmap** ([roadmap](roadmap.md) §11): whether hand-measured pages' checks may sample fewer runs; VS Code at M8 or M9; Google Maps without WebAssembly.
- **Milestone 1** ([Milestone 1](milestone-1.md) §8): the `css/CSS2` tests in scope; the local copies' date; hosted runners.
- **The standards baseline** ([standards](standards.md) §8): pinning only revisions wpt.fyi aligns; Unicode 17 in ICU4X while 18 is out; the not-planned list, which the owner may move.
- **The testing strategy** ([testing](testing.md) §10): where fuzzing's seed corpora live.
- **The performance budgets** ([performance](performance.md) §9): Safari's figures; the hypotheses, measured at their milestones.
- **The census** ([S8 results](research/s8-results.md) §7.5): Chrome logged out by hand, and Safari signed in, not yet measured.
- **The designs:** each document under `architecture/` ends with its own open questions, placed by the roadmap where they bind a milestone.

## 11. Areas flagged for critical review

Each item is an **assessment**: a claim or decision this handover thinks most deserves challenge, why, and what evidence would change it. None changes the baseline by being listed here. **Where the review's rounds answered an item, a note says how;** an external review may still challenge the answer.

**A. Scope, sequencing and feasibility**
1. **What Axiom builds itself** (DP1; ADR-0001 R2; ADR-0017). The engine, Temporal, `Intl`'s glue, `RegExp`, the WebIDL compiler, the HTTP/2 and HTTP/3 framing and the parsers are all its own. Is DP1's line drawn where reuse would cost the goals nothing? *Would change it:* a reused component that meets DP8's grades and the budgets, and saves substantial work. **Round 1** (RV1-08): encodings, Temporal's arithmetic and BigInt became replaceable details; `encoding_rs` failed Axiom's audit (`ub-risk-3`); `temporal_rs` is graded before M3.
2. **Milestone 1's breadth** (MA1, MA11): the whole static pipeline, the shell, the compositor and networking in one milestone. Would a narrower first milestone, such as headless rendering of local files to reference tests, give earlier feedback on the riskiest interfaces? *Would change it:* a split that keeps MS4's rule, every interface from M1. **Round 1** made it broader, with real processes (RV1-07), and ran the harness end to end at its second checkpoint (RV1-01).
3. **The order of milestones** (MS5): L3 at M6, before media, applications, the GPU and WebAssembly. Security comes before surface, but eight of the thirteen corpus pages wait for M7–M9. *Would change it:* evidence that a capability forces an architectural change that would be cheaper found earlier. **Round 1:** the GPU/compositor process at M2 (RV1-02); WebAssembly's architecture before M3 and the JIT at M5's gate (RV1-13); WebSockets at M4 (RV1-12).
4. **WebAssembly decided at M8's gate** (OQ-3), though three corpus pages need or use it. **Round 1** (RV1-13): its architecture is decided before M3, with S14.

**B. Security architecture**

5. **The sandbox rests on undocumented libsandbox functions** (ADR-0004 D2; threat model §9). Chromium, Firefox and WebKit use the same route, and SDK 27 already kills processes that use named profiles (S4). The fallback, O2, is App Sandbox entitlements on each helper, whose base profile reaches 170 Mach services. What would L1–L3 mean under O2?
6. **The browser process is unsandboxed** (D1). **Round 1** (RV1-02) moved validation, rasterization and compositing into a sandboxed GPU/compositor process from M2, so L3 is certified on that topology; the browser still decodes IPC from every process and holds the layer tree.
7. **M1 loads live pages at L0** (MA5): networking (rustls, HTTP/2), safe-Rust parsers, and CoreText on system fonts. The threat model permits public documents at L0, and MS3 forbids script. **Round 1** (RV1-07): M1's renderer and network process are sandboxed, under S4's, S5's and S7's profiles.
8. **SI3 until M6:** cross-site iframes share their embedder's process at L1 and L2, including M5's logins with test accounts. Are SI3's three compensating rules enough against Spectre-class reads at L2? (Round 1 made them separate agents from M1, RV1-03, which doesn't change their process.)
9. **The JIT policy's endgame** (J1–J7): whether the app-web corpus can be served acceptably with no JIT at all, and whether J7's containment design is credible. **Round 1** (RV1-13) moved the decision to M5's gate.
10. **Passkeys** (OQ-16): if the platform authenticator needs a restricted entitlement, L3 may not cover passkey-only accounts. **Round 1** (RV1-21) added the hybrid transport.
11. **Staying patched without automatic updates** (TS15): the routine's cadence and coverage.

**C. Memory and performance budgets**

12. **The ceilings' basis** (ADR-0019, PF3):
    - one day's live pages in one Chrome build, with interaction by hand whose spread is wide;
    - Safari's costs exceed Chrome's on six of the ten pages it measured, under heavier interaction (S8 §7.4).

    Is "half of Chrome's cost" a sound target? Is the metric right: the steady-state footprint above a blank baseline, with GPU memory charged to the page? Are the ceilings achievable, or only aspirational? **Round 1** (RV1-05) replaced the content tier's cap with a slope and a plateau on a normalized workload, and the census addendum splits Chrome's cost by component.
13. **B6, 1 GB for 10 tabs, on 8 GB,** once L3's out-of-process iframes multiply processes within B5's soft limit of 16.
14. **The network process holds storage** (SW1), but B4 bounds it only while idle. Safari's network process held 233–241 MB with VS Code (S8 §7.4).
15. **Pauses** (PF15: ≤ 4 ms while interacting) **with a stop-the-world collector.** **Round 1** (RV1-04): the end-state is planned, not triggered, and fixed before M3, with S11's measurements.
16. **Frame budgets from a synthetic page** (S6), on a scaled display mode whose window-server scaling isn't counted.

**D. Technology choices worth checking against current practice**

17. **Young crates on the critical path:** HarfRust, `read-fonts` and `skrifa`, `vello_cpu`, `quinn-proto`, `redb`; and `smallvec`'s grade. Their maintenance, audits and maturity as of the review's date. Neither round re-checked these dated facts (the review's §8).
18. **WebGPU's implementation** is open until M9 (GR10), and wgpu isn't admitted. Writing a WebGPU implementation is a large piece of work, and it would run in a GPU process under DP10.
19. **Rust's own features:** explicit tail calls (`become`, JE6), and nightly sanitizers on macOS 27 (**S12** found working routes).
20. **Network privacy:** post-quantum key exchange is on (S7); Encrypted Client Hello and DNS over HTTPS at M5's plan, since rustls has client-side ECH (Round 1, RV1-20).
21. **Accessibility through Axiom's own bridge** rather than AccessKit (AX8), whose macOS adapter lacked rich text and hypertext when Step 5 read it (ADR-0014's condition for revisiting). **Round 2** confirmed the choice and found gaps in geometry, enablement, tests and notifications (RV2-06 to RV2-09).
22. **Unicode's version:** ICU4X implements 17.0.0 while 18.0.0 is out (standards §8).

**E. The quality of the evidence**

23. **One Mac, varying conditions:** some runs above PF1's load threshold; by-hand runs with wide spreads and varying interaction; one Safari run without a Private window.
24. **A tool error was found late:** `tools/wpt-pass-rates.py` left WPT's reference tests out, until S9 found it. It was corrected and 42 of 48 rows recomputed, so Gate B's report had overstated where Ladybird meets the median. Numbers derived by tools deserve spot checks against wpt.fyi. **Round 1** found a second: M1's CSS target couldn't be met without script (RV1-01), recounted with `tools/wpt-script-deps.py`.
25. **Design rules resting on inference, not measurement:** the speed of IPC decoding, skeleton replicas, a thread per agent (§8). **S10** measured display-list validation and the GPU process's hop.
26. **No external review yet:** L3's bar is the threat model's own, without an external audit or a bug bounty (threat model §8). Rounds 1 and 2 were written by the same model that helped write the baseline; **the owner commissions an independent external review next.**

**F. Gaps to look for**

27. **WebRTC** was "staged" in the standards baseline but placed in no milestone. **Round 1** (RV1-22) put it on the not-planned list until a corpus page needs it.
28. **A built-in PDF viewer, developer tools and printing.** **Round 1:** PDFs open in Preview, and in-engine viewing is not planned (RV1-22); text dumps, local crash reports and the console come at M1–M3, a developer-tools protocol later (RV1-15). Printing comes after M9.
29. **Features outside the corpus** that a daily browser needs: downloads, permissions prompts, password saving (M6), autofill, find in page, zoom, session restore, spellchecking. **Round 1** (RV1-22): downloads, find in page and zoom at M5; session restore at M6.
30. **Recovery and diagnosis:** crash reporting and logs on a single-user machine, without telemetry. **Round 1** (RV1-15): crash reports kept locally from M2.

## 12. Where everything is

**The repository** ([github.com/MTawhid7/Axiom](https://github.com/MTawhid7/Axiom), public):
- **`docs/`**:
  - the [status](status.md) log, [charter](charter.md), [decision map](decision-map.md) and [open questions](open-questions.md);
  - the binding documents: [corpus](corpus.md), [standards](standards.md), [testing](testing.md), [performance](performance.md), [roadmap](roadmap.md), [Milestone 1](milestone-1.md) and [requirements](requirements.md);
  - `adr/`; `security/`, the [threat model](security/threat-model.md) and [process model](security/process-model.md);
  - `architecture/`: the [overview](architecture/overview.md) and 19 designs;
  - `technology/`: the [language evaluation](technology/language-evaluation.md), [dependency policy](technology/dependency-policy.md) and [register](technology/dependency-register.md);
  - `research/`: the step trackers, the macOS notes, the spike results and the audit of `encoding_rs`;
  - `review/`: the [review](review/README.md)'s process, its rounds' documents and the owner's disposition log.

  The [index](README.md) lists every document.
- **`spikes/`:** S1–S10 and S15, throwaway code, each with its results document; S12 reused S9's code and `tools/s9-measure.sh` ([spikes/README.md](../spikes/README.md)).
- **`tools/`:** the unsafe-policy and link checkers, and the measurement, analysis and signing scripts, each printing its usage with `--help`.
- **Not in the repository:** `target/`, which holds the spikes' raw records and downloads, and `scratch/`, which holds working files, among them the corpus's full URLs with their identifying parameters. Git ignores both.

**Branches:**
- `main` holds all of Phase Zero, Steps 0–6, as approved at Gate C;
- the step branches `step-2b` to `step-6` are kept;
- `architecture-review` is branched from `main` and holds the review's Rounds 1 and 2 and their amendments. It merges into `main` only when the owner says so, after the external review.

**The checks,** before every commit:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
tools/check-unsafe-policy.sh
tools/check-doc-links.py
```

**Resuming work:** read [status.md](status.md) first, then the decision map and the current tracker (for the review, its [log](review/README.md#5-the-disposition-log)), and check them against `git status` and `git log`.

## Appendix A. Rule prefixes

| Prefix | Rules | Document |
|---|---|---|
| G, R | Goals G1–G5; scope R1–R3 | [Charter](charter.md), [ADR-0001](adr/0001-scope-and-target.md) |
| T | Build thresholds T1–T4 | [ADR-0002](adr/0002-implementation-language.md) |
| C | GC obligations C1–C5 | [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md) |
| SG, V | Security goals; the L3 checks V1–V9 | [Threat model](security/threat-model.md) |
| PM, SI, IR, B, J | Process model, site isolation, IPC trust, budgets, JIT | [Process model](security/process-model.md), [ADR-0004](adr/0004-process-model-and-sandbox.md) |
| AC | Step 4's architecture criteria | [Overview](architecture/overview.md) §5.1 |
| EL, IP, FT, PL, IV, IN, EM, SH | Event loop, IPC, frame tree, pipeline, invalidation, input, embedder, script and heap | Step 4's designs under `architecture/` |
| SC | Step 5's subsystem criteria | [Overview](architecture/overview.md) §5.2 |
| DP | Dependency policy | [Dependency policy](technology/dependency-policy.md) |
| TX, GR, IM, NW, SW, AX, HD, ST, LY, JE, WB | Text, graphics, images and media, networking, storage, accessibility, HTML and the DOM, style, layout and paint, the JavaScript engine, bindings | Step 5's designs under `architecture/` |
| RC | Step 6's readiness criteria | [Overview](architecture/overview.md) §5.3 |
| CO, SB, TS, PF | Corpus, standards, testing, performance | [Corpus](corpus.md), [standards](standards.md), [testing](testing.md), [performance](performance.md) |
| MS, MA, RQ | Roadmap, Milestone 1, requirements | [Roadmap](roadmap.md), [Milestone 1](milestone-1.md), [requirements](requirements.md) |
| OQ | Open questions | [Open questions](open-questions.md) |
| RV | The review's findings, `RV<round>-<nn>` | [The review](review/README.md) and its rounds |
| SN-QK | Spike N's question K | `research/sN-results.md` |
| L0–L3, M1–M9 | Security levels; milestones | [Threat model](security/threat-model.md) §8; [roadmap](roadmap.md) §4 |

## Sources

This handover cites only the repository's own documents, as of 2026-10-21. Each of them names its sources, dated when published and when read, and labels its own evidence.
