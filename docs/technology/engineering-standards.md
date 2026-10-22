# Engineering Standards

| | |
|---|---|
| **Status** | **Accepted** (2026-10-23), with [ADR-0022](../adr/0022-code-architecture-and-engineering-standards.md), and **binding on engine code.** Written at Milestone 1's first checkpoint, at the owner's request (2026-10-22): "I want us to establish the architectural principles, project structure, coding standards, testing strategy, dependency management approach, and key technical decisions before implementation begins." On ES1, the owner: "Approved. Enforce 600-line warning / 1,000-line failure for files, and 100-line failure for functions. We will review empirical distribution at M1-CP15." They are carried into CLAUDE.md's code rules at M1-CP2, which builds their tools |
| **Question** | How is Axiom's code written, documented, tested in place, reviewed and checked, so that it stays modular and readable as it grows to hundreds of thousands of lines? |
| **Builds on** | [ADR-0002](../adr/0002-implementation-language.md) and the [language evaluation](language-evaluation.md) §15.3 (`unsafe`); the [dependency policy](dependency-policy.md); the [testing strategy](../testing.md) (TS1, TS7–TS11); the [performance budgets](../performance.md) (PF5's tiers); RV1-16's failure semantics ([Milestone 1](../milestone-1.md) MA7); the [crate architecture](../architecture/crates.md); [observability](../architecture/observability.md); the [charter](../charter.md)'s principles 5–7; CLAUDE.md's code rules |
| **Feeds** | ADR-0022; CLAUDE.md's code rules, at M1-CP2; every crate's charter; the review of every commit and checkpoint |
| **Last updated** | 2026-10-23: ES10's lists and ES20's scenarios corrected at M1-CP2, with the crate map. Earlier: accepted |

Rules are numbered ES1–ES29. A rule enforced by a tool names it; the others are checked in review (ES28). The rules are inference from the binding documents and the sources in §2, unless marked otherwise.

## Summary

- **Size is bounded by tools** (ES1–ES3): a file warns at 600 lines and fails at 1,000; a function fails at 100 (Clippy's `too_many_lines`, already in force); a crate splits when its own rebuild passes T1, or when it holds two concerns.
- **Interfaces are small and typed** (ES4–ES7): items are private by default; IDs and units are newtypes; Axiom's enums are matched exhaustively, so a new variant breaks every place that must handle it.
- **Errors are values, and panics are bugs** (ES8–ES10): typed errors written by hand; `unwrap` denied outside tests; RV1-16's panic-free and checked-arithmetic crates marked in the crate map and checked.
- **`unsafe` stays in the platform layer's `src/sys/`** (ES11), finalizing the language evaluation's draft §15.3.
- **Architecture rules are lints** (ES12, ES13): reading the clock, the environment or files, spawning threads or processes, and std's randomly seeded hash maps are disallowed outside the crates whose design owns them. That keeps tests deterministic (TS8) and placement visible.
- **Everything is instrumented through `trace`** (ES14). Nothing prints, and no record holds page content.
- **Each crate's documentation is its charter** (ES15): purpose, rules, invariants, threads and tests. Code cites the specification's section and the binding rule it implements (ES16). TODOs name a milestone (ES17), and suppressing a lint takes `#[expect]` with a reason (ES18).
- **Generated code is committed and checked fresh** (ES19).
- **Tests sit beside the code,** with golden files updated only on purpose, and no sleeps (ES20–ES23).
- **Checks run in two tiers from one entry point** (ES25): `tools/check.sh` per commit, and `--checkpoint` per checkpoint.
- **Design notes for architectural interfaces wait for the owner** (ES27, the owner's choice). A review checklist and a definition of done close each commit and checkpoint (ES28, ES29).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0002** and **evaluation §15.3:** `unsafe` only in designated crates, under `// SAFETY:` comments naming their invariants; none in parsers, network policy, IPC decoding, or image and font wrappers; macro-generated `unsafe` counts (rule 7). §15.3 is marked "draft; finalized with the architecture" and still lists `gc` as an `unsafe` crate, which ADR-0008 settled has none (SH7, AC9) | ES11 |
| **RV1-16** (MA7): `panic = "abort"` everywhere; the browser's validators and message handling panic-free, with Clippy's `indexing_slicing`, `unwrap_used` and `arithmetic_side_effects` denied; parsers, decoders and IPC validators with checked arithmetic | ES9, ES10 |
| **TS8:** deterministic tests: virtual time, seeded randomness, injected environment, loopback networking only. **EL24:** the clock is an interface. **EL1:** every thread a thread loop | ES13, ES22 |
| **TS11:** state crosses threads through a short list of primitives, each in one crate | ES12 |
| **PF5:** deterministic counts as unit tests on every commit: sizes, allocations, items, messages, wakeups | ES23 |
| **DP6, DP18:** each dependency justified beside its manifest entry, with minimal features | ES24 |
| **The charter's principles 5 and 6:** keep fact and inference, implemented and planned, compliant and approximate apart; identify technical debt openly | ES15–ES17 |
| **CLAUDE.md:** five checks before every commit; commits named by checkpoint; code written like the surrounding code | ES25, ES26 |
| **MS10:** each checkpoint leaves the workspace clean and working, with a tracker updated | ES29 |
| **The owner's choice** (2026-10-22): design notes for cross-crate interfaces gate their code ("Interfaces only") | ES27 |

## 2. Sources and observations

- **Clippy's lints and settings,** read from the pinned toolchain (observed, 2026-10-22, `cargo clippy --explain <lint>` with clippy 0.1.94):
  - `too_many_lines` (in `pedantic`): "Checks for functions with a large amount of lines", with `too-many-lines-threshold` "(default: `100`)";
  - `wildcard_enum_match_arm`: "New enum variants added by library updates can be missed";
  - `self_named_module_files`: "Checks that module layout uses only `mod.rs` files";
  - `allow_attributes`: suggests `#[expect]`, which "emit[s] a warning, if the expectation is unfulfilled";
  - `allow_attributes_without_reason`: "Justifying each `allow` helps readers understand the reasoning";
  - `unwrap_used`, `expect_used` and `indexing_slicing`, each with an `allow-…-in-tests` setting, by default `false`;
  - `print_stdout` and `dbg_macro`, each with an `allow-…-in-tests` setting;
  - `todo`;
  - `disallowed_methods` and `disallowed_types`, configured with `path`, `reason` and `replacement`.
- **`#[expect]`** (sourced; the Rust blog, 2024-09-05, read 2026-10-22): "1.81 stabilizes a new lint level, `expect`, which allows explicitly noting that a particular lint _should_ occur, and warning if it doesn't", alongside `reason = "…"` on lint attributes [R1].
- **Workspace dependencies** (sourced; the Cargo Book, read 2026-10-22): members inherit with `workspace = true`, and "`features` declared in this table are additive with the `features` from `[dependencies]`". It notes that before Rust 1.99 a package-level `default-features = false` "may be ignored or rejected unless the workspace dependency also disables default features" [K1]. The pinned toolchain is 1.94.1, so the workspace entry sets it.
- **Committed generated code** (sourced; rust-analyzer's architecture document, read 2026-10-22): "Generated code is generally committed to the git repository", and "Generated code is updated automatically on `cargo test`" [A1].
- **Design documents** (secondary; Malte Ubl, "Design Docs at Google", 2020-07-06, read 2026-10-22): a design doc records the high-level strategy and the key decisions, "with emphasis on the trade-offs that were considered during those decisions", in sections from context and scope to alternatives considered and cross-cutting concerns. When a design has no real trade-offs, the author suggests writing the program instead [U1].
- **The spikes, written without size limits** (observed, 2026-10-22, `wc -l` over `spikes/**/*.rs`): 219 files and 53,451 lines; 16 files over 600 lines and 2 over 1,000. They already suppress lints mostly with `#[expect(…, reason = "…")]`: 86 `#[expect(` against 11 outer `#[allow(`. Their `platform` modules open with numbered invariants (for example `spikes/s16/lifetime/src/platform/mod.rs`).

## 3. Code organization

| # | Rule |
|---|---|
| ES1 | **Files: 600 lines warn, 1,000 lines fail,** counting every line of a `.rs` file. `tools/check-file-size.py` (planned, M1-CP2) enforces it. Exempt are files under a crate's `src/generated/` (ES19) and those listed in `tools/file-size-allowlist.txt`, one per line with its justification, as `tools/unsafe-policy.txt` is written. Typical exemptions: a specification's data table, or a decoder that the specification's structure makes one unit. **Unit tests count,** so a file that nears 600 lines moves its tests into `<module>/tests.rs` before anything else. The limits are hypotheses: the owner reviews their distribution at M1-CP15, Milestone 1's gate ("We will review empirical distribution at M1-CP15") |
| ES2 | **Functions: at most 100 lines,** Clippy's `too_many_lines` at its default, already in force through `pedantic` and `-D warnings`. An exception takes `#[expect(clippy::too_many_lines, reason = "…")]` (ES18). The expected ones are a dispatch loop that must be one function to be fast (JE6's `match` loop) and an assembly listing in a test. **Within the limit, a function does one thing:** a state machine gets a function per state, not one `match` over every state's body |
| ES3 | **Crates: one concern and one dependency set each.** A crate splits when its own incremental rebuild passes T1's 5 s, or when it holds two concerns whose dependencies differ. **`lib.rs` is a table of contents:** the crate's charter (ES15), module declarations and re-exports, with no logic |
| ES4 | **Modules: one concept each, named for it.** A module with submodules is a directory with a `mod.rs`, as the spikes write them, enforced by `clippy::self_named_module_files`. Modules nest no deeper than their concepts do |
| ES5 | **Visibility: private by default.** `pub(crate)` for what a crate shares internally; `pub` only for its interface, re-exported from `lib.rs` (`unreachable_pub` warns already). **A type with invariants has no public fields:** its constructors establish the invariants, and its methods keep them |

## 4. Types and interfaces

| # | Rule |
|---|---|
| ES6 | **Types carry meaning.** IDs are newtypes per kind (IP12), and units are types: the layout unit (LY1), CSS and device pixels, durations on the event loop's clock. Closed sets are enums. **Axiom's own enums are matched exhaustively,** with no wildcard arm (`clippy::wildcard_enum_match_arm`), and no enum inside the workspace is `#[non_exhaustive]`. So adding a variant, such as an IPC message (CR10), makes the compiler list every place that must handle it |
| ES7 | **Parse, don't validate.** Data from outside a crate's trust becomes a typed value at one place, which checks it there; the rest of the code holds the typed value. `Untrusted<T>` (IP10) is the IPC form of this rule. Every crate follows it at its interface: a parser returns a typed tree, a decoder a typed image. **No third-party type crosses a crate's public interface** (CR11) |

## 5. Errors, panics and arithmetic

| # | Rule |
|---|---|
| ES8 | **Errors are typed values, written by hand.** Each fallible interface returns `Result` with an error enum named for the operation (`DecodeError`, `SpawnError`), implementing `Display` and `std::error::Error`. Its variants carry what a caller needs to decide, not prose. **No `anyhow`, `thiserror` or boxed `dyn Error` in library interfaces:** they would add dependencies to every process, the browser's included, and hide which failures an interface has. **Recoveries a specification defines aren't Rust errors.** HTML's parse errors, CSS's invalid declarations and the like follow the specification. They are reported to a diagnostics sink where a test or the trace wants them (ES14) |
| ES9 | **A panic is a bug.** Every executable aborts on a panic (RV1-16), so code panics only when an invariant of Axiom's own is broken, never because of input. **`unwrap` is denied outside tests** (`clippy::unwrap_used`, with `allow-unwrap-in-tests`). `expect` is allowed where an invariant guarantees the value, with a message that names the invariant, for example `expect("IV1: style ran before layout")`. `todo!` and `unimplemented!` are denied (ES17) |
| ES10 | **Panic classes, from RV1-16,** recorded in the crate map (CR2). Each crate's root declares its class's lints, which the dependency checker verifies (CR16):<br>• **`panic-free`:** the browser's message handling and every validator of less-trusted data, today `display-list`, `compositor`, `presenter`, `frame-tree`, `diagnostics` and `browser`, and, as the crate map recorded at M1-CP2 under this same rule, `trace`, `url`, `ipc` and `protocol`. They deny `indexing_slicing`, `unwrap_used`, `expect_used`, `arithmetic_side_effects` and `panic`. Every fallible step returns a `Violation` or an error.<br>• **`checked-arithmetic`:** parsers and decoders of untrusted bytes, today `html`, `xml`, `css`, `image` and `http`, and `encoding` (M1-CP2). They deny `arithmetic_side_effects`, and so use checked, saturating or wrapping arithmetic by intent. Where checking costs more than the budget allows, the alternative RV1-16 names, overflow checks on in release builds, is measured first.<br>• **Ordinary:** the rest, under ES9.<br>Every fuzz target reports a panic as a finding (TS9) |

## 6. `unsafe`

| # | Rule |
|---|---|
| ES11 | **The `unsafe` policy, finalized** (evaluation §15.3, the draft this completes; ADR-0002's rules 5 and 7):<br>• **Where:** only the platform layer's crates, in their `src/sys/` (CR6); later `jit`, under J1–J7; and the counting allocator in `test-support`, which PF5's allocation counts need and which never ships. Each is listed in `tools/unsafe-policy.txt` with its justification. Every other crate root declares `#![forbid(unsafe_code)]`; `gc` has none (SH7).<br>• **How:** one unsafe operation per block (`multiple_unsafe_ops_per_block`), each with a `// SAFETY:` comment naming a numbered invariant from its crate's `src/sys/mod.rs` (`undocumented_unsafe_blocks`); unsafe functions document their contract (`missing_safety_doc`).<br>• **Shape:** foreign calls stay thin, and the logic around them is safe code that Miri and unit tests run (CR6). Miri runs every crate with `unsafe`, over what it can execute (TS11). The foreign calls themselves are covered by tests on this Mac, by AddressSanitizer runs, and by V5's audit.<br>• **Macros:** a macro that expands `unsafe` into another crate is part of the boundary (rule 7, DP11).<br>`tools/check-unsafe-policy.sh` already enforces where; Clippy enforces how |

## 7. Concurrency and determinism

| # | Rule |
|---|---|
| ES12 | **State crosses threads only through TS11's primitives, each implemented once:**<br>• channels and shared-memory rings in `ipc`;<br>• the read-only lending of arenas in `gc`;<br>• the interrupt flag in `event-loop`;<br>• `SharedArrayBuffer`'s atomics in `js`, from M3.<br>Locks (`std::sync::Mutex`, `RwLock`, `Condvar`) are disallowed types elsewhere (ES13). Each new primitive is a decision for the owner, with its loom tests (TS11's second check). **Threads come only from `event-loop`:** a thread role, or a helper pool, from EL's list. **No async runtime:** protocols are state machines driven by their thread's one wait (NW1, EL1) |
| ES13 | **Determinism and placement, by lint.** `clippy.toml` disallows these, each with its reason and replacement. An allowed use takes `#[expect(clippy::disallowed_methods, reason = "…")]` in the crates named:<br>• **the clock:** `Instant::now` and `SystemTime::now`. Time comes from the event loop's clock (EL24), so single-process tests run on virtual time (TS8). Allowed in `platform-event`, which implements the host clock, and nowhere else; `trace` takes its clock from its assembly.<br>• **the environment:** `std::env::var`, `var_os`, `vars` and `vars_os`. Settings and fonts are injected (TS8, ST36), and helpers start with an empty environment (the process model §8). Allowed in executables' `main`, `headless` and test crates.<br>• **threads and sleeping:** `std::thread::spawn`, `Builder::spawn` and `sleep`, under ES12. Allowed in `event-loop` and `test-scheduler`.<br>• **processes:** `std::process::Command`. Only the browser launches processes, through `posix_spawn` (IR9).<br>• **files:** `std::fs::File`, `OpenOptions` and the module's functions. Allowed in `diagnostics` (OB18), `browser` (the profile, from M3), `headless`, `codegen` and test crates. Renderers and the network process can't open files under their profiles anyway, but the lint fails at compile time rather than as a silent denial at run time, which S5 and S7 met.<br>• **sockets:** `std::net`'s types. Allowed in `net` and `test-support`'s loopback server.<br>• **hash maps:** `std::collections::HashMap` and `HashSet` with std's random seed, because their iteration order changes from run to run. `strings` provides the engine's maps instead: a fast fixed hasher for keys the engine assigns, such as atoms and IDs, and a keyed hasher for strings from web content, keyed per process from `getentropy` in production and fixed in test builds (TS8: "every other source" of randomness seeded). The second also resists collision attacks from pages |

## 8. Instrumentation

| # | Rule |
|---|---|
| ES14 | **Every crate reports through `trace`** ([observability](../architecture/observability.md)):<br>• each task, lifecycle phase, IPC message, navigation and load gets its span or count, in its category;<br>• logs are kept for what must be seen without tracing: violations, recoveries, degraded modes;<br>• metrics are declared statically, with units.<br>**Nothing prints:** `print!`, `println!`, `eprint!`, `eprintln!` and `dbg!` are denied (`print_stdout`, `print_stderr`, `dbg_macro`), except in executables' `main` before `trace` starts, and in tools. **No record holds page content,** form values, cookies or credentials. A URL is reduced to its origin unless detailed mode is on (OB22) |

## 9. Documentation and comments

| # | Rule |
|---|---|
| ES15 | **Each crate's `//!` documentation is its charter,** written before its code in the checkpoint that creates it, and kept current. It states:<br>1. its purpose, in a paragraph;<br>2. the binding rules it implements, by ID;<br>3. its layer, processes, group and panic class, as the crate map has them;<br>4. its interface: the main types and the order of calls;<br>5. its invariants, numbered (and for a platform crate, the `SAFETY` invariants of `src/sys/mod.rs`);<br>6. its threads and ownership;<br>7. its tests and oracles;<br>8. its efficiency notes: size assertions, budgets.<br>Every module starts with a `//!` paragraph. Public items are documented (`missing_docs` for library crates); `cargo doc` runs warning-free at each checkpoint |
| ES16 | **Code cites what it implements.** An algorithm from a specification cites the section with its URL and anchor, and numbers its steps as the specification does: `// HTML §13.2.5.1, "Data state": https://html.spec.whatwg.org/multipage/parsing.html#data-state`, then `// Step 3.` The crate's charter records the pinned snapshot (SB1). Code that implements a binding rule names it, for example `// IP9: the decoder owns every handle.` Approximate behaviour is marked `// Approximate (LY10): …` (charter principle 5) |
| ES17 | **Comments say why, and debt is named.** A TODO takes the form `TODO(<milestone or checkpoint>, <rule or tracker item>): <what>`, for example `TODO(M1-CP8, LY10): automatic table layout`. Each checkpoint lists them in its report (ES25), which keeps debt visible (charter principle 6). `FIXME`, `XXX` and `HACK` aren't used. Comments don't restate the code |
| ES18 | **A lint is suppressed with `#[expect(lint, reason = "…")]`** on the smallest item that needs it, never with a bare `#[allow]` (`clippy::allow_attributes`, `allow_attributes_without_reason`). So a suppression that is no longer needed warns. Crate-level `#![deny]` and `#![forbid]` for policy, such as ES10's classes and `unsafe_code`, are the exception, since they tighten rather than loosen |

## 10. Generated code

| # | Rule |
|---|---|
| ES19 | **Generators live in `codegen`; their output is committed and checked fresh.** Each generator's input is a plain-text file beside the crate it feeds: the IDL beside `protocol`, the property table beside `css`, the atom list beside `strings`. Its output goes to that crate's `src/generated/`, under a header naming the generator and the input. **On every commit,** `cargo run -p codegen -- --check` regenerates in memory and fails if a committed file differs (ES25). Generated files are never edited by hand, are formatted, and are deterministic: the same input gives the same bytes. They follow the `unsafe` policy (IP8, rule 7) and are exempt from ES1. **Why committed:**<br>• a change to a generator shows its effect in the diff, which matters most for the decoders that guard the browser (IR1);<br>• builds don't run generators, so T1 stays low;<br>• Axiom's crates need no build scripts, which DP12 prefers.<br>This reads IP8's "run at build time" and PL11's "generated at build time" as "by the build's tooling", an amendment of wording that ADR-0022 puts to the owner. The bindings' size may call for a different answer at M3, decided with spike S13 |

## 11. Tests in code

| # | Rule |
|---|---|
| ES20 | **Tests sit beside what they test.**<br>• **Unit tests** go in the module's `tests` submodule: inline while the file is small, or in `<module>/tests.rs` (ES1).<br>• **A crate's integration tests** go in its `tests/`. **Scenarios across processes** go in `headless`'s tests, which compose the assemblies in one process (CR7). (Corrected at M1-CP2: this rule first put them in the assembly crates through `test-support`, which, as a leaf every crate's tests use, can't depend on the assemblies without a cycle, CR3.)<br>• **Names say the behaviour and the condition,** for example `late_meta_charset_switches_decoder_when_prefix_matches`.<br>• **Every bug fixed** gets a test that failed before the fix, and every fuzzing crash becomes a unit test (TS10).<br>• **A parser or decoder** gets its fuzz target in `fuzz/` in the commit that adds it (TS9); a differential oracle where the design names one (TS7) |
| ES21 | **Golden files change only on purpose.** Text dumps (MA1's DOM, styles, fragments, display lists and network log) and other stable outputs are compared with files under the crate's `tests/golden/`. They are rewritten only when `AXIOM_BLESS=1` is set for `cargo test`, and every rewritten file is reviewed in the commit's diff |
| ES22 | **Tests are deterministic and self-contained** (TS8):<br>• no sleeping: time-dependent tests run on EL24's virtual clock;<br>• no network beyond the loopback, and no live site except the corpus checks that CO3 allows;<br>• randomness from a seed the test sets and reports;<br>• no dependence on the order tests run in, or on files outside the repository and `target/`.<br>A test that fails intermittently is a defect, fixed before the checkpoint ends |
| ES23 | **PF5's first tier is unit tests:**<br>• `const` assertions on the sizes of the types that dominate memory, such as the node's 36 bytes (HD10), computed styles, fragments and display items;<br>• allocations counted with `test-support`'s allocator for operations whose allocations are fixed;<br>• messages, handles and display items per operation;<br>• wakeups over a fixed interval on the virtual clock.<br>A change that alters one updates the assertion in the same commit, with the reason in its message |

## 12. Dependencies

| # | Rule |
|---|---|
| ES24 | **Every dependency is declared once, in `[workspace.dependencies]`,** with its exact version, `default-features = false` and only the features Axiom uses (DP18). A short justification sits beside it, as the manifest already does for the spikes' crates (DP6). Members inherit with `workspace = true`. Internal crates are listed there by path too. Adding an Axiom dependency also edits the crate map (CR2). Adding a third-party one first adds its register entry and grade (DP6, DP8), with the owner's approval at the milestone's gate (MS10) |

## 13. Checks

| # | Rule |
|---|---|
| ES25 | **Two tiers, one entry point: `tools/check.sh`** (planned, M1-CP2; it prints its usage with `--help`).<br>• **The commit tier**, before every commit:<br>  – `cargo fmt --all --check`;<br>  – `cargo clippy --workspace --all-targets -- -D warnings`;<br>  – `cargo test --workspace`;<br>  – `tools/check-unsafe-policy.sh`;<br>  – `tools/check-doc-links.py`;<br>  – `tools/check-dependencies.py` (CR16);<br>  – `tools/check-file-size.py` (ES1);<br>  – `cargo run -p codegen -- --check` (ES19).<br>  The tier runs the spikes' workspace too, while they are kept. **Its time is budgeted:** about 3 minutes for an incremental change on this Mac, a hypothesis measured at M1-CP2. When it is exceeded, slow suites move to the checkpoint tier, so the budget is never met by skipping checks.<br>• **The checkpoint tier,** `tools/check.sh --checkpoint`, before a checkpoint's last commit:<br>  – the conformance suites of the areas the checkpoint touched (TS1);<br>  – PF5's ratchet against `perf/baseline.toml`;<br>  – Miri on the crates with `unsafe`, and loom on TS11's primitives;<br>  – `cargo doc` warning-free;<br>  – `cargo deny check` and `cargo vet` (DP15, DP16);<br>  – a short fuzzing run of each target;<br>  – T1–T4 (PF18);<br>  – the TODO list (ES17).<br>• **The gate tier** is PF1's formal measurement and the milestone's criteria, as before (MA9) |

## 14. Process

| # | Rule |
|---|---|
| ES26 | **Commits and checkpoints.** Each commit is one logical change and passes the commit tier. Its message is named `<scope>(m1-cpK): <summary>`, where the scope is a crate, `docs`, `tools` or `workspace`, and its body says why. A checkpoint is one or more such commits, ending with the commit that updates its tracker after the checkpoint tier has passed. Branches, merges and pushes follow CLAUDE.md |
| ES27 | **Design notes come before code.** **An architectural interface** (CR8) gets an interface note in `docs/architecture/interfaces/` (planned), approved by the owner before its code is written (the owner's choice, 2026-10-22: "Interfaces only"). The note has these sections:<br>1. purpose and scope;<br>2. the binding rules it implements;<br>3. the types and signatures, as Rust;<br>4. the invariants, threads and ownership;<br>5. errors and violations;<br>6. efficiency: sizes, copies, allocations, the budget it draws on;<br>7. tests and oracles;<br>8. the alternatives considered.<br>A change to an approved interface goes back to the owner. **A crate's internals** are designed in its charter (ES15), committed before its code in the same checkpoint, and reviewed by the owner asynchronously. A design with no real alternative needs no note beyond the charter (§2: when there are no trade-offs, the program is the design) |
| ES28 | **The review checklist,** applied to every commit before it is made, and by the owner at each gate:<br>1. **Design:** it implements a binding rule or an approved note; any new edge between crates is in the crate map; any change to an architectural interface has its note approved.<br>2. **Placement:** the dependency check passes; no test-only code reaches a shipping executable.<br>3. **Correctness:** new behaviour has tests; each fix has its regression test; oracles where the design names one; a fuzz target for each new parser or decoder.<br>4. **Safety:** `unsafe` only in `src/sys/`, each block under a named invariant; the panic class respected.<br>5. **Efficiency:** first-tier assertions current; no allocation on a hot path without a reason.<br>6. **Observability:** new tasks, phases and messages instrumented; no content in records.<br>7. **Documentation:** the charter and module documentation current; specifications and rules cited; TODOs in their form.<br>8. **Size:** files and functions within their limits, or excepted with a reason |
| ES29 | **A checkpoint is done when:**<br>• its plan's exit criterion is met;<br>• every commit passed the commit tier, and the checkpoint tier passes;<br>• its interface notes are approved;<br>• its new dependencies are registered and approved;<br>• its TODOs are listed;<br>• its measured values are in the tracker (MS10).<br>The status log records the session |

## 15. The configuration this implies

For M1-CP2, which writes it (inference). The workspace's current lints stay. Added:

| Where | Setting | Rule |
|---|---|---|
| `[workspace.lints.rust]` | `missing_docs = "warn"` (library crates; an executable sets `#![expect(missing_docs, reason = …)]` if it needs to) | ES15 |
| `[workspace.lints.clippy]` | `wildcard_enum_match_arm`, `self_named_module_files`, `allow_attributes`, `allow_attributes_without_reason`, `unwrap_used`, `print_stdout`, `print_stderr`, `dbg_macro`, `todo`, `unimplemented`: each `"warn"`, an error under `-D warnings` | ES4, ES6, ES9, ES14, ES17, ES18 |
| Crate roots, by panic class | `panic-free`: `#![deny(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::arithmetic_side_effects, clippy::panic)]`; `checked-arithmetic`: `#![deny(clippy::arithmetic_side_effects)]` | ES10 |
| `clippy.toml` | `too-many-lines-threshold = 100`; `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-indexing-slicing-in-tests` and `allow-print-in-tests`, all `true`; `disallowed-methods` and `disallowed-types` as ES13 lists them, each with `reason` and `replacement` | ES2, ES9, ES13 |
| Release profiles | `panic = "abort"` for every executable (RV1-16), as MA10 already authorizes | ES9 |

## 16. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **No size limits,** review only | Files grow by small accretions that no single review rejects; two spike files passed 1,000 lines (§2). Rejected |
| B | **Tighter limits,** such as 400 lines a file and 50 a function | They fragment cohesive code: a decoder's states, a specification's table. Kept open: the owner may tighten them at ADR-0022's gate, and ES1's are revisited at M1's gate with data |
| C | **`thiserror` and `anyhow`** | Both add dependencies to every process, `thiserror` a procedural macro with its build cost, and `anyhow` hides which failures an interface has. Hand-written errors are a few lines each. Rejected |
| D | **`#[non_exhaustive]` on internal enums** | Within one workspace it forces wildcard arms, which hide the places a new variant must be handled. Rejected |
| E | **Unit tests always in separate files** | Small modules read better with their tests beside them. Rejected: tests move out when a file grows (ES1) |
| F | **Coverage as a gate** | Gating on a number invites tests written for the number. Rejected: coverage may be reported at checkpoints with the tools S9 set up, and never gates |
| G | **A pre-commit hook that runs `tools/check.sh`** | A local convenience, which needs `core.hooksPath` set in this clone's configuration. Offered to the owner at M1-CP2, not required |

## 17. Open questions

1. **The commit tier's time budget:** about 3 minutes for an incremental change, a hypothesis measured at M1-CP2 (ES25).
2. **A pre-commit hook:** the owner's choice at M1-CP2 (§16, G).
3. **The limits of ES1:** approved by the owner (2026-10-23), and their distribution reviewed at M1-CP15, Milestone 1's gate.

## Sources

All read on 2026-10-22.

- [R1] The Rust Blog, "Announcing Rust 1.81.0" (2024-09-05) — https://blog.rust-lang.org/2024/09/05/Rust-1.81.0/
- [K1] The Cargo Book, "Workspaces", `[workspace.dependencies]` — https://doc.rust-lang.org/cargo/reference/workspaces.html
- [A1] rust-analyzer, "Architecture" — https://github.com/rust-lang/rust-analyzer/blob/master/docs/book/src/contributing/architecture.md
- [U1] Malte Ubl, "Design Docs at Google" (2020-07-06), secondary — https://www.industrialempathy.com/posts/design-docs-at-google/
- Clippy's lint documentation, read with `cargo clippy --explain` from clippy 0.1.94, the pinned toolchain's (observed)
