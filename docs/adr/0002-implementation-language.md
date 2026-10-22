# ADR-0002: Implementation language

| | |
|---|---|
| **Status** | **Accepted** (final, 2026-10-17). Provisionally accepted on 2026-10-16; the Step 2b spikes validated it. **Amended by the architecture review's Round 1** (2026-10-21): failure semantics, panics and overflow (RV1-16) |
| **Date** | 2026-10-16; accepted as final 2026-10-17 |
| **Deciders** | Project owner |
| **Related** | [Language evaluation](../technology/language-evaluation.md) (full analysis and sources), [ADR-0001](0001-scope-and-target.md), [S1 results](../research/s1-results.md), [S2 results](../research/s2-results.md), [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (Accepted) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-16 | Proposed: Rust, with `unsafe` confined by policy |
| 2026-10-16 | Owner **provisionally accepts** Rust (stable toolchain, 2024 edition) under the unsafe-code policy in evaluation §15.3. Step 2b (spikes S1 and S2) is authorized to validate it before final acceptance and before any production engine code |
| 2026-10-16 | Policy refinement (evaluation §15.3, rule 7): `#![forbid(unsafe_code)]` does not catch macro-generated `unsafe`, so macros that emit `unsafe` belong to the unsafe boundary |
| 2026-10-17 | **S1 complete.** The central assumption holds: Pattern A confines `unsafe` to one GC module, and Pattern B needs none anywhere. Revisit condition 1 does not trigger ([S1 results](../research/s1-results.md)) |
| 2026-10-17 | **S2 complete.** The objc2 platform stack (AppKit, Metal, CoreText) works without Xcode, with `unsafe` contained. Build thresholds T1–T4 are proposed for revisit condition 3 ([S2 results](../research/s2-results.md)) |
| 2026-10-17 | **Recommendation: final acceptance.** The GC pattern is proposed separately as [ADR-0003](0003-gc-handle-and-rooting-pattern.md) |
| 2026-10-17 | **Owner accepts ADR-0002 as final:** "The S1 and S2 evidence conclusively validates the Rust hypothesis." The owner also accepts thresholds T1–T4 as the meaning of revisit condition 3 (OQ-12), and ADR-0003 |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-16: "Enforce panic = "abort" per binary; enforce checked arithmetic / overflow checks in parsers and IPC validators." The decision gains its failure semantics, below; the [process model](../security/process-model.md)'s IR10 and the [IPC design](../architecture/ipc.md)'s IP11 apply them |

## Question

What language should Axiom be written in? This covers the browser shell, the engine, our own JavaScript engine (with its GC and any future JIT), and the macOS platform layer.

## Requirements and constraints

From [ADR-0001](0001-scope-and-target.md) and the target environment:
- our own JS engine, with one GC heap shared by JS and the DOM;
- a full app web that is reachable, which implies parallelism and multiple agents;
- a security posture fit for sensitive logins;
- macOS on Apple Silicon only;
- an 8 GB M1 with only the Command Line Tools installed;
- one human developer with AI assistance (an assumption).

## Alternatives considered

- **Rust**
- **C++** (C++23/26 with modern hardening)
- **Swift** (6.x)
- **Zig** (0.16+)
- **Mixed-language architectures**, such as a Rust engine with a Swift shell, or a C++ VM and DOM with Rust elsewhere

Excluded, with reasons given in the evaluation (§6.5): languages with their own garbage collector (Go, Java, C#), plain C, experimental languages, and Ada/SPARK.

## Decision

**Rust for all Axiom code**, using the stable toolchain and the 2024 edition. A strict policy confines `unsafe` to designated crates:
- `gc`;
- `jit`, later;
- `platform`;
- benchmark-justified hot paths.

Parsers, network policy and IPC message decoding contain no `unsafe` at all. The full policy is in the evaluation, §15.3.

**Failure semantics, as amended in Round 1** (RV1-16, 2026-10-21). Rust's defaults unwind on a panic and let integer overflow wrap in release builds; Axiom sets both:
- **Every executable is built with `panic = "abort"`.** A panic on any thread ends its process, and nothing recovers with `catch_unwind`. A helper's panic is a crash the browser handles (IR10); a panic in the browser ends it, since its state can't be trusted afterwards.
- **Parsers, decoders and IPC validators use checked arithmetic,** enforced by Clippy's `arithmetic_side_effects` lint, or are built with overflow checks on in release builds (a per-package profile setting), with the cost measured at Milestone 1.
- **The browser's message handling and validators are panic-free by construction:** Clippy's `indexing_slicing` and `unwrap_used` are denied in them, and every fuzz target reports a panic as a finding (IP11, V2).

The workspace's profiles and lints take these settings at Milestone 1's skeleton ([Milestone 1](../milestone-1.md) MA7, MA11). The spikes keep theirs, so their measurements stay comparable.

## Rationale (summary — see the evaluation for evidence)

1. **Security.**
   - About 70% of Chromium's high-severity security bugs are memory-safety bugs.
   - Android cut the memory-safety share of its vulnerabilities from 76% to 24% (2019–2024), mainly by writing new code in memory-safe languages.
   - All of Axiom's code is new.
2. **Parallelism and multiple agents.** Rust checks for data races at compile time. Mozilla built Firefox's parallel style system in Rust after two failed C++ attempts.
3. **Our own JS engine.** This is Rust's weakest criterion, and the choice accepts it deliberately:
   - viable designs exist (gc-arena, Nova, Brimstone, Boa);
   - the burden falls only on the JS, DOM and bindings layers;
   - generated code absorbs the boilerplate;
   - spike S1 validated the approach (2026-10-17).
4. **Tooling and ecosystem.**
   - The tools work without Xcode: `samply`, `cargo-fuzz`, Miri.
   - Several candidate libraries are already proven in Chromium and Firefox.
5. **Platform.** `objc2` and C FFI cover what Axiom needs. Swift's advantage is ergonomic, not a capability we would otherwise lack.
6. **Corroboration.** Ladybird adopted Rust in 2026 despite the same concern about GC and inheritance.

**Weighted matrix (/500):** Rust **432**, C++ 376, Swift 331, Zig 286. C++ overtakes Rust only if one of two things happens:
- GC convenience is weighted twice as heavily *and* memory safety half as heavily; or
- Rust's GC story fails badly *and* `unsafe` leaks out of the GC crate.

## Step 2b evidence (2026-10-17)

| Assumption | Evidence | Status |
|---|---|---|
| At least one GC design keeps `unsafe` inside the `gc` crate with acceptable ergonomics | **S1.** Both designs keep it contained: Pattern A has 24 audited blocks in one module, and Pattern B has none anywhere. Both pass the reference oracle and Miri. Ergonomics differ in kind: architectural for A, lexical for B | **Holds.** The owner judged Pattern B's ceremony acceptable (S1-Q2, 2026-10-17) |
| The owner's fluency does not rule out Rust | OQ-6, resolved 2026-10-16 | **Holds** |
| Builds on the 8 GB M1 are workable with mitigations | **S1 and S2.** The whole workspace builds clean in 8.1 s and incrementally in under 1 s. The largest `rustc` uses 0.35 GiB with trimmed bindings (1.4 GiB with objc2's defaults); the aggregate compiler peak is 0.94 GiB | **Holds.** Thresholds T1–T4 accepted 2026-10-17 ([S2 results](../research/s2-results.md) §6) |
| The macOS layer can be written from Rust without Xcode, with `unsafe` contained (Rationale 5) | **S2.** 19 `unsafe` blocks, all in the platform module, under six invariants. A pixel-exact smoke test passes. The Command Line Tools lack an offline Metal compiler, so shaders compile at run time | **Holds** |

## Assumptions

- At least one GC design keeps `unsafe` inside the `gc` crate with acceptable ergonomics. **Validated by S1**, as above.
- The owner's fluency does not rule out Rust ([OQ-6](../open-questions.md)).
- Build times on the 8 GB M1 are workable with mitigations. **Measured by S1 and S2**, as above.

## Risks and unresolved issues

See evaluation §15.5. The main ones:
- awkward GC references in the JS, DOM and bindings layers;
- modelling the inheritance-heavy DOM;
- build cost on 8 GB;
- interpreter dispatch without stable tail calls.

## Conditions for revisiting

See evaluation §15.6.
- The key condition, **S1 shows `unsafe` cannot stay inside the GC crate**, was tested in Step 2b and did not trigger.
- **Condition 3 (build cost)**, accepted 2026-10-17, means: on the 8 GB M1 with the default job count, a measure stays above its revisit level *after* the mitigations in [S2 results](../research/s2-results.md) §6 have been tried.

  | # | Measure | Target | Revisit above |
  |---|---|---|---|
  | T1 | Incremental dev rebuild and link after editing one leaf crate | 5 s | 30 s |
  | T2 | Clean dev build of the whole workspace | 5 min | 20 min |
  | T3 | Peak aggregate compiler and linker memory during a build | 3 GiB | 5 GiB |
  | T4 | Peak memory of one `rustc` | 1.5 GiB | 3 GiB |

  Measure with `tools/measure-build.sh`. At acceptance, the values were 0.82 s, 8.1 s, 0.94 GiB and 0.35 GiB.

## Consequences

- Future crates are designed around `#![forbid(unsafe_code)]` with a few audited exceptions.
- Every executable aborts on a panic, and the code that handles untrusted input uses checked arithmetic (RV1-16).
- The DOM is modelled with enums, traits and code generated from WebIDL, not class inheritance.
- The GC handle pattern is its own ADR: [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (Accepted 2026-10-17) adopts Pattern B (Nova style), with obligations C1–C5.
- macOS integration goes through objc2 and C FFI. A thin Swift shell remains an option for later, not a plan.
