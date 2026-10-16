# ADR-0002: Implementation language

| | |
|---|---|
| **Status** | **Provisionally accepted** (2026-10-16). Final acceptance follows the Step 2b spikes S1 and S2 |
| **Date** | 2026-10-16 |
| **Deciders** | Project owner. Final acceptance requires the owner's explicit confirmation after the [Step 2b spikes](../research/step-2b-validation-spikes.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-16 | Proposed: Rust, with `unsafe` confined by policy |
| 2026-10-16 | Owner **provisionally accepts** Rust (stable toolchain, 2024 edition) under the unsafe-code policy in evaluation §15.3. Step 2b (spikes S1 and S2) is authorized to validate it before final acceptance and before any production engine code |
| 2026-10-16 | Policy refinement (evaluation §15.3, rule 7): `#![forbid(unsafe_code)]` does not catch macro-generated `unsafe`, so macros that emit `unsafe` belong to the unsafe boundary |
| **Related** | [Language evaluation](../technology/language-evaluation.md) (full analysis and sources), [ADR-0001](0001-scope-and-target.md) |

## Question

What language should Arbor be written in? This covers the browser shell, the engine, our own JavaScript engine (with its GC and any future JIT), and the macOS platform layer.

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

## Recommendation

**Rust for all Arbor code**, using the stable toolchain and the 2024 edition. A strict policy confines `unsafe` to designated crates:
- `gc`;
- `jit`, later;
- `platform`;
- benchmark-justified hot paths.

Parsers, network policy and IPC message decoding contain no `unsafe` at all. The full policy is in the evaluation, §15.3.

## Rationale (summary — see the evaluation for evidence)

1. **Security.**
   - About 70% of Chromium's high-severity security bugs are memory-safety bugs.
   - Android cut the memory-safety share of its vulnerabilities from 76% to 24% (2019–2024), mainly by writing new code in memory-safe languages.
   - All of Arbor's code is new.
2. **Parallelism and multiple agents.** Rust checks for data races at compile time. Mozilla built Firefox's parallel style system in Rust after two failed C++ attempts.
3. **Our own JS engine.** This is Rust's weakest criterion, and the choice accepts it deliberately:
   - viable designs exist (gc-arena, Nova, Brimstone, Boa);
   - the burden falls only on the JS, DOM and bindings layers;
   - generated code absorbs the boilerplate;
   - spike S1 validates the approach before acceptance.
4. **Tooling and ecosystem.**
   - The tools work without Xcode: `samply`, `cargo-fuzz`, Miri.
   - Several candidate libraries are already proven in Chromium and Firefox.
5. **Platform.** `objc2` and C FFI cover what Arbor needs. Swift's advantage is ergonomic, not a capability we would otherwise lack.
6. **Corroboration.** Ladybird adopted Rust in 2026 despite the same concern about GC and inheritance.

**Weighted matrix (/500):** Rust **432**, C++ 376, Swift 331, Zig 286. C++ overtakes Rust only if one of two things happens:
- GC convenience is weighted twice as heavily *and* memory safety half as heavily; or
- Rust's GC story fails badly *and* `unsafe` leaks out of the GC crate.

## Assumptions

- At least one GC design keeps `unsafe` inside the `gc` crate with acceptable ergonomics. **This is the S1 hypothesis.**
- The owner's fluency does not rule out Rust ([OQ-6](../open-questions.md)).
- Build times on the 8 GB M1 are workable with mitigations. S2 measures this.

## Risks and unresolved issues

See evaluation §15.5. The main ones:
- awkward GC references in the JS, DOM and bindings layers;
- modelling the inheritance-heavy DOM;
- build cost on 8 GB;
- interpreter dispatch without stable tail calls.

## Conditions for revisiting

See evaluation §15.6. The key condition: **S1 shows `unsafe` cannot stay inside the GC crate**, which is the scenario in which C++ overtakes Rust in the matrix.

## Consequences if accepted

- Future crates are designed around `#![forbid(unsafe_code)]` with a few audited exceptions.
- The DOM is modelled with enums, traits and code generated from WebIDL, not class inheritance.
- Spike S1 chooses the GC handle pattern, which becomes its own ADR.
- macOS integration goes through objc2 and C FFI. A thin Swift shell remains an option for later, not a plan.
