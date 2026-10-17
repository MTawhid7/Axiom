# Spikes (Phase Zero)

This directory holds validation experiments. The code is **throwaway and not production**: it exists to answer the questions in a step's plan, and its results live in `docs/research/`.
- S1 and S2 validate [ADR-0002](../docs/adr/0002-implementation-language.md) ([Step 2b plan](../docs/research/step-2b-validation-spikes.md)).
- S4 validates [ADR-0004](../docs/adr/0004-process-model-and-sandbox.md) ([Step 3 plan](../docs/research/step-3-security-and-process-model.md); design in the [macOS confinement note](../docs/research/macos-process-confinement.md) §7).
- S3 validates ADR-0003's obligation C1 for [ADR-0008](../docs/adr/0008-script-agents-and-heap.md), and measures dispatch, layout and barriers ([Step 4 plan](../docs/research/step-4-architecture-proposal.md) §10).

| Spike | Question | Crates |
|---|---|---|
| S1 | Can a GC heap shared by JS and the DOM keep `unsafe` inside the GC boundary, with acceptable ceremony and cost? Pattern A (gc-arena style) against Pattern B (Nova style) | `s1/*` |
| S2 | objc2 AppKit + Metal + CoreText without Xcode: ergonomics, `unsafe` surface, build cost | `s2/window` |
| S4 | Can a helper launched with `posix_spawn` enter a custom deny-by-default Seatbelt profile on macOS 27, and what does that cost? Where can it generate code? Does CoreText work inside it? | `s4/sandbox`, `s4/text` |
| S3 | Can a root stack without borrow tracking cost at most 1 ns per root and per read (C1)? How much rooting does an interpreter with callback-heavy builtins do? How fast is each bytecode dispatch technique on stable Rust? Do layouts without index arithmetic speed up traversal (C2)? What do generated setters and a card-marking barrier cost (C3)? | `s3/gc`, `s3/vm`, `s3/bench` |

## Rules

- **The unsafe-code policy applies to spikes too** (evaluation §15.3; `tools/check-unsafe-policy.sh`).
  - Every crate root declares `#![forbid(unsafe_code)]`, except crates listed in `tools/unsafe-policy.txt`.
  - Listed crates declare `#![deny(unsafe_code)]`, and allow `unsafe` only in their listed modules.
- **Verification before every commit:**

  ```sh
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  tools/check-unsafe-policy.sh
  tools/check-doc-links.py   # every relative Markdown link and anchor resolves
  ```

- **Build measurements:** `tools/measure-build.sh --help`.
- **Runtime measurements** (CP7 and CP8). Each script prints its usage with `--help`, and writes its results under `target/s1-results/` or `target/s2-results/`.

  | Script | Measures |
  |---|---|
  | `tools/s1-bench.sh` | Runtime suites: build, traverse, mutate, dispatch, collect, whole scenarios. Each run is a separate process; results are aggregated across runs |
  | `tools/s1-memory.sh` | Peak footprint, maximum RSS and instructions retired per scenario, one fresh process per measurement; bytes per live node |
  | `tools/s1-profile.sh` | `samply` CPU profiles per case, summarized to functions and source lines by `tools/samply-summary.py` |
  | `tools/s1-ceremony.sh` | GC-API tokens and code size of the two DOM crates (S1-Q2) |
  | `tools/s2-measure.sh` | S2: binary size, time to first frame, and idle CPU, wakeups, energy impact and memory of `s2-window` |
  | `tools/s3-bench.sh` | S3: the root stacks' unit costs, S1's dispatch case, the VM workloads on every technique and stack, traversal layouts, mutation and barrier costs; counts from the `count` build; the nightly tail-call technique |
  | `tools/s3-profile.sh` | S3: `samply` profiles of every workload, technique and stack, with the share of time in the root stack, builtins and collector |

  `cargo bench -p s1-bench` runs one pass of the S1 runtime suites. `cargo run -p s2-window -- --smoke` checks the S2 window's rendering (it opens a window briefly).
- **S3.** `cargo test -p s3-gc -p s3-vm -p s3-bench` runs the heap and root-stack tests, the interpreter's oracle (every technique on every stack, at normal pacing and when collecting at every allocation) and a quick pass of every benchmark suite. Add `--features count` for the check that bytecode never roots. Miri covers C1's `unsafe`: `cargo +nightly miri test -p s3-gc`, and `cargo +nightly miri test -p s3-vm --test miri`. `target/release/s3-probe roots` prints the root stacks' unit costs.
- **S4.** `cargo test -p s4-sandbox` runs the sandbox probes against the linker-signed helper. `tools/s4-sign.sh` signs copies ad hoc with the hardened runtime (plus the JIT variants), and `target/s4/bin/s4-broker probe|startup|ipc|jit|text` runs each scenario against them. `tools/s4-measure.sh` runs them all on release builds and writes `target/s4-results/<timestamp>/`. `--named-profile`, and `cargo test -p s4-sandbox -- --ignored`, also check that a named profile kills the helper; each such run leaves a crash report in `~/Library/Logs/DiagnosticReports`.
