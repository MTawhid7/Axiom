# ADR-0022: Code Architecture and Engineering Standards

| | |
|---|---|
| **Status** | **Accepted** (2026-10-23), by the owner at Milestone 1's first checkpoint: "Accepted as proposed (Decisions D1 through D15)." The acceptance sweep is made (Consequences): the [overview](../architecture/overview.md) §2, [Milestone 1](../milestone-1.md)'s MA1 and MA10, IP8, PL11 and the [language evaluation](../technology/language-evaluation.md) §15.3 now follow this ADR. CLAUDE.md follows at M1-CP2, when the tools exist |
| **Date** | 2026-10-22 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | The [crate architecture](../architecture/crates.md) (CR1–CR16) and the [engineering standards](../technology/engineering-standards.md) (ES1–ES29), which this ADR proposes; [ADR-0023](0023-observability-and-diagnostics.md), observability; [ADR-0002](0002-implementation-language.md); [ADR-0004](0004-process-model-and-sandbox.md); [ADR-0005](0005-ipc-transport-and-interfaces.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [ADR-0021](0021-milestone-1.md); [Milestone 1's plan](../milestone-1-plan.md); the [charter](../charter.md)'s principles |

## Decision history

| Date | Event |
|---|---|
| 2026-10-22 | **Commissioned** by the owner, at Milestone 1's first checkpoint: "Given the scale and potential long-term evolution of this project, which could grow to hundreds of thousands of lines of code, we need to approach it with the discipline and rigor of a mature software engineering organization." "Our primary goal should be to build a codebase that is **modular, maintainable, and extensible**." "We must also avoid common architectural pitfalls, particularly monolithic files that accumulate thousands of lines of code". "I want us to establish the architectural principles, project structure, coding standards, testing strategy, dependency management approach, and key technical decisions before implementation begins. We should document the rationale behind significant decisions so that future contributors can understand not just *what* we built, but *why* we built it that way." And on the plan: "Feel free to refine or rewrite it as needed. Once the plan is finalized, we can proceed with implementation in stages." |
| 2026-10-22 | **The owner's choices in plan mode,** before drafting, each from options put with a recommendation:<br>• on Milestone 1's order: "Walking skeleton first (Recommended)", recorded in [ADR-0021](0021-milestone-1.md)'s history and the plan;<br>• on design reviews: "Interfaces only (Recommended)": notes for cross-crate interfaces wait for the owner's approval before code, and notes on internals are reviewed asynchronously (D14);<br>• on observability, after the first draft: "I think we should implement a robust observability mechanism that will help us monitor the system effectively in the long run" ([ADR-0023](0023-observability-and-diagnostics.md); D8) |
| 2026-10-22 | **Proposed** at M1-CP1, with the crate architecture and the engineering standards |
| 2026-10-23 | **Accepted** by the owner: "ADR-0022 (Code Architecture and Engineering Standards): Accepted as proposed (Decisions D1 through D15). The 7-layer architecture, facility-split platform layer, separate spikes workspace, and check-dependencies script are approved." On D9's limits: "File & Function Size Limits (ES1): Approved. Enforce 600-line warning / 1,000-line failure for files, and 100-line failure for functions. We will review empirical distribution at M1-CP15." |
| 2026-10-23 | **The owner's conditional approval** on the crate architecture's open question 1: "smallvec & IDNA: If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." It takes effect only if M1-CP2's grade is `ub-risk-3` (CLAUDE.md: a conditional approval counts once its condition is met), and the evidence is recorded with it |
| 2026-10-23 | **The condition is met** at M1-CP2: the [audit of `smallvec` 1.16.3](../research/smallvec-audit.md) grades it `ub-risk-3`, with undefined behaviour in `insert_many` that Miri reports under Stacked Borrows and Tree Borrows. NW2 is amended as the owner approved ([ADR-0013](0013-networking-storage-and-workers.md)'s history); the crate architecture gains `url-idna`, `sandboxed`, which `axiom`'s closure excludes (CR4) |

## Question

How is Axiom's code organized into crates, and how is it written, checked and reviewed, so that it stays modular, maintainable and extensible as it grows, with the rules enforced by tools wherever a tool can?

## Requirements and constraints

- **The charter's principles:** the smallest well-engineered system (2); security and performance as architecture (3); facts, plans and approximations kept apart (5); technical debt named (6); **no architectural corners cut** (7).
- **The binding rules that place code:** AC4, AC9, EM3, RV1-09, DP2, DP9, DP21, NW2, TX3, PL11, IV1, IV2, MS4, IP2, IP8, IP18 and MA7, each listed in the crate architecture's §1.
- **ADR-0002:** Rust with `unsafe` confined (rules 5 and 7); the build thresholds T1–T4; RV1-16's failure semantics.
- **ADR-0010:** the dependency policy, with DP22's planned checker.
- **The testing strategy** (TS8's determinism, TS9's fuzzing, TS11's concurrency checks) and **PF5's tiers.**
- **The owner's choices** above.
- **The environment:** one developer with an assistant; Cargo without Xcode; this Mac's 8 GB.

## Alternatives considered

Each decision below records its own alternatives, at length in the documents it cites. The main ones:
- **For the crate graph:** the overview's one `platform` crate; layering by convention; a build system with visibility rules (Bazel, GN); a few large crates; a crate per module.
- **For generated code:** generators run by build scripts at every build, as Servo runs its bindings generator.
- **For test-only code:** Cargo features.
- **For errors:** `anyhow` and `thiserror`.
- **For limits:** none; or tighter ones.

## Historical context and modern developments

- **Chromium checks its layering mechanically:** `DEPS` files' `include_rules`, checked by its bots, with a review required for new entries (sourced, crate architecture §2). It shows that layering in a large engine survives only when a tool enforces it (inference).
- **Servo generates its script bindings at build time.** Its script crate, with 520,000 generated lines, dominated its builds until it was split ([RV1-18](../review/round-1/02-findings.md#rv1-18-build-cost-of-generated-code)).
- **rust-analyzer commits its generated code** and regenerates it in its tests (sourced, standards §2).
- **Cargo unifies features across the packages of one build** (sourced, crate architecture §2), which is why test capabilities can't safely be features in a workspace that builds tests and products together.
- **Design documents** are how large organizations record trade-offs before building (secondary, standards §2). This project's ADRs and designs already do that for the system. This ADR extends it to the code.

## Analysis

- **What the overview's proposal would have cost** (crate architecture §2, §11):
  - URLs in `net` and encodings in `text` would have linked the TLS stack into renderers and the browser, and CoreText into the HTML parser and the network process;
  - one `platform` crate would have put every Apple framework in every executable;
  - a crate named `core` would have hidden Rust's `core` from its dependents (observed).

  Each is cheap to fix before code exists and expensive after: many crates' imports would change.
- **Enforcement over convention.** The rules that matter most are checked by tools: the crate graph, placement, `unsafe`, size, generated code and determinism. Review then spends its attention on design. One person with an assistant can't rely on several reviewers to catch drift.
- **Locality of change** is checked by scenarios (crate architecture §8): each common change touches only the crates it names.
- **Costs:**
  - about forty crates at M1, whose build overhead T1–T4 measures at M1-CP4 with every crate present;
  - a few hundred lines of checking scripts, in standard-library Python like the existing tools;
  - stricter lints, whose exceptions each take a stated reason.

## Evidence and sources

The [crate architecture](../architecture/crates.md)'s and the [engineering standards](../technology/engineering-standards.md)' §2 and Sources, all read or observed on 2026-10-22: Cargo's resolver and workspace documentation; Chromium's `checkdeps`; Miri's README; crates.io's record of `icu_normalizer` 2.3.0; Clippy's documentation from the pinned toolchain; the Rust 1.81 announcement; rust-analyzer's architecture document; "Design Docs at Google" (secondary); the scratch experiment with a crate named `core`; the spikes' measured sizes and conventions.

## Decision

**Accepted** (2026-10-23): adopt the [crate architecture](../architecture/crates.md) (CR1–CR16) and the [engineering standards](../technology/engineering-standards.md) (ES1–ES29), with these decisions:

- **D1. A layered crate graph, checked mechanically** (CR1–CR3, CR16). Seven layers; an explicit allowlist of internal dependencies for every crate in `tools/crate-map.toml`; no cycles; `tools/check-dependencies.py` in the commit checks, extending DP22's planned tool.
- **D2. The platform layer split by facility and placed by process** (CR6): `platform-mach`, `-event`, `-process`, `-metal`, `-layers`, `-text`, `-net` and `-appkit`, with `unsafe` only in each one's `src/sys/`. AC9's "the shell's `platform` module" becomes `platform-appkit`.
- **D3. Shared vocabulary at the foundation** (CR5): `strings`, `geometry`, `url`, `encoding` and `trace`, so that NW2's one URL parser and TX3's encodings can be linked where they are needed, and only there.
- **D4. Process assemblies as libraries** (CR7): `browser`, `renderer` and `network`, with thin executables. `document` owns a document's lifecycle (IV1, IV2), and single-process mode composes the same assemblies, in test code only.
- **D5. Generated code committed and checked fresh** (CR10, ES19), by one `codegen` crate, with no build scripts in Axiom's crates. **This amends the wording** of IP8 ("run at build time") and PL11 ("generated at build time") to "by the build's tooling, with its output committed". The bindings at M3 are decided with spike S13.
- **D6. Test-only capabilities as crates or executables, never features** (CR12), with **the build's flavor in IP18's identifier** (CR13).
- **D7. Errors and panics** (ES8–ES10): typed errors written by hand; panics only for broken invariants; `unwrap` denied outside tests; RV1-16's panic classes recorded in the crate map and checked.
- **D8. Observability as a subsystem of its own,** decided by [ADR-0023](0023-observability-and-diagnostics.md): `trace` at the foundation, `diagnostics` in the browser (ES14).
- **D9. Size limits enforced by tools** (ES1–ES3): files warn at 600 lines and fail at 1,000; functions at 100; crates by T1 and by concern. The owner reviews their distribution at M1-CP15, Milestone 1's gate.
- **D10. Architecture rules as lints** (ES4, ES6, ES13, ES14, ES17, ES18): disallowed methods and types for the clock, environment, files, processes, threads, sockets, locks and randomly seeded hash maps; exhaustive matching; `#[expect]` with reasons.
- **D11. Checks in two tiers with one entry point** (ES25): `tools/check.sh` for every commit, `--checkpoint` for every checkpoint, with the commit tier's time budgeted.
- **D12. Separate workspaces for the engine, the spikes and fuzzing** (CR15). The spikes stay in `spikes/`, as MA10 says, with the current lock moved there; the engine's lock holds only the engine's dependencies; the register covers all three locks.
- **D13. Naming without collisions** (CR14): unprefixed names, none equal to a built-in crate or a package in the lock; `core` becomes `browser`.
- **D14. Design notes** (CR8, ES27): notes for architectural interfaces wait for the owner's approval before code, as the owner chose. A crate's internal design is its charter, committed first and reviewed asynchronously.
- **D15. The `unsafe` policy finalized** (ES11). The language evaluation's draft §15.3 becomes: the platform layer's `src/sys/`, later `jit`, and `test-support`'s counting allocator, with `gc` free of `unsafe` as ADR-0008 decided.

## Rationale

- **The owner's goals, met by mechanism rather than intention.** Modularity is the allowlisted graph (D1). Separation of concerns is the layers and the facility split (D1, D2). Explicit interfaces are D14's notes and CR11's wrapping of third-party crates. Small files are D9. "Why, not just what" is these documents and each crate's charter (ES15).
- **Security rules become build facts.** AC4, RV1-09, TX3, DP21 and MA7 are checked on every commit by CR4's placement rules, instead of being argued at gates.
- **Extensibility.** New features are mostly data and local code (crate architecture §8). A replaced dependency stays inside the crate that wraps it (DP2). The move of M2's compositor into its own process changes assemblies, not subsystems (MS4).
- **Determinism and testability.** The lints of D10 keep the clock, the environment and randomness injectable (TS8, EL24). D4 lets every process's composition run in one test process (IP2).
- **Proportion.** Each rule replaces a review judgement with a check, or records a convention the spikes already follow: `#[expect]` with reasons, numbered invariants, `mod.rs` directories.

## Assumptions

- About forty crates fit T1–T4 on this Mac; measured at M1-CP4.
- The commit tier stays near its budget as the code grows, by moving slow suites to the checkpoint tier (ES25).
- Clippy's restriction lints named in ES's §15 stay available on toolchain updates; a removal is handled when the toolchain moves (TS15).
- `cargo metadata` describes the resolved graph faithfully for the checker; its unified features over-approximate a single executable's, which errs toward refusing (CR4).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Too many crates slow builds | Medium | T1–T4 missed | Measured at M1-CP4 with every crate present; crates merged where a split buys nothing (ES3) |
| The lints prove noisy | Medium | Exceptions pile up | Each exception takes a reason and warns once it's no longer needed (ES18); lints that cost more than they catch go back to the owner |
| Interface notes slow M1 | Medium | Longer checkpoints | Gated only for architectural interfaces (D14), batched at M1-CP3 and M1-CP4 |
| `smallvec` graded `ub-risk-3` | Medium | The browser can't link ICU4X's normalizer for the address bar's IDNA (NW2) | Crate architecture's open question 1: full IDNA in a `sandboxed` crate and the address bar's lookup in the network process, an amendment for the owner, decided with the grade at M1-CP2 |
| The commit tier grows slow | Medium | Fewer, larger commits | ES25's budget and its rule: suites move tiers, never skipped |

## Conditions for revisiting

- T1–T4 are missed because of the number of crates.
- A Rust or Cargo feature appears that changes the trade-offs, such as features scoped per package within a build, or per-crate dependency visibility in Cargo itself.
- The commit tier's budget can't be held without dropping a check.
- A rule here is found to block a binding requirement.

## Consequences

**On acceptance, these documents are amended** (the sweep), each with the owner's words. **Done on 2026-10-23,** at M1-CP1's close, except CLAUDE.md, which M1-CP2 updates with the tools:
- **the overview §2:** its crate table points to the crate architecture, with the renamed and split crates;
- **Milestone 1:** MA1's crate names; MA10's list gains the repository tools, the workspace split and `codegen`;
- **[IPC](../architecture/ipc.md) IP8 and the [pipeline](../architecture/rendering-pipeline.md)'s PL11:** the wording of D5;
- **the [language evaluation](../technology/language-evaluation.md):** §15.3 is marked finalized by ES11, and §15.4's "not pinned now" is corrected (the toolchain was pinned on 2026-10-17);
- **the [dependency policy](../technology/dependency-policy.md):** DP22's tool gains the crate map's checks; the register's spikes section points to the spikes' own lock;
- **the [requirements](../requirements.md):** rows for the layering and placement checks, the size limits, the panic classes and the commit tier;
- **the [handover](../handover-phase-0.md):** §4.12 notes the refined crates;
- **CLAUDE.md**, at M1-CP2, when the tools exist: the code rules, the repository's layout and the checks.

**A reading rule for the earlier records.** ADRs 0002–0021 and the Phase Zero records (the steps' trackers, the review's rounds, the handover's lists, the spikes' results) stay as written, as decision records do. In them, `platform` reads as the platform layer (CR6), "the shell's `platform` module" as `platform-appkit`, `core` as `browser`, and `idl` as `protocol` with `codegen`. The binding designs that an implementer reads were updated in the sweep: the overview §2, MA1, MA7 and MA10, IP8, PL11, TS11, DP5, DP21 and DP22, GR1, AX8, the register's objc2 entry, the process model's unsafe boundary, the threat model's AD2 and V5, and the event loop's, embedder API's and text design's checks of AC9.

**Milestone 1's plan** adds an engineering-foundation checkpoint (M1-CP2) that builds the crate map, the checkers, the configuration and the first crates, before any engine code.

**The sweep list** for after acceptance, in `docs/`:
- the crate names `` `core` `` and `` `platform` ``;
- "at build time";
- "the shell's `platform` module".
