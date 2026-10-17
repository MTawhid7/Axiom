# Spikes (Phase Zero)

This directory holds validation experiments. The code is **throwaway and not production**: it exists to answer the questions in a step's plan, and its results live in `docs/research/`.
- S1 and S2 validate [ADR-0002](../docs/adr/0002-implementation-language.md) ([Step 2b plan](../docs/research/step-2b-validation-spikes.md)).
- S4 validates [ADR-0004](../docs/adr/0004-process-model-and-sandbox.md) ([Step 3 plan](../docs/research/step-3-security-and-process-model.md); design in the [macOS confinement note](../docs/research/macos-process-confinement.md) §7).

| Spike | Question | Crates |
|---|---|---|
| S1 | Can a GC heap shared by JS and the DOM keep `unsafe` inside the GC boundary, with acceptable ceremony and cost? Pattern A (gc-arena style) against Pattern B (Nova style) | `s1/*` |
| S2 | objc2 AppKit + Metal + CoreText without Xcode: ergonomics, `unsafe` surface, build cost | `s2/window` |
| S4 | Can a helper launched with `posix_spawn` enter a custom deny-by-default Seatbelt profile on macOS 27, and what does that cost? Where can it generate code? Does CoreText work inside it? | `s4/sandbox`, `s4/text` |

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

  `cargo bench -p s1-bench` runs one pass of the S1 runtime suites. `cargo run -p s2-window -- --smoke` checks the S2 window's rendering (it opens a window briefly).
- **S4.** `cargo test -p s4-sandbox` runs the sandbox probes against the linker-signed helper. `tools/s4-sign.sh` signs copies ad hoc with the hardened runtime (plus the JIT variants), and `target/s4/bin/s4-broker probe|startup|ipc|jit|text` runs each scenario against them. `tools/s4-measure.sh` runs them all on release builds and writes `target/s4-results/<timestamp>/`. `--named-profile`, and `cargo test -p s4-sandbox -- --ignored`, also check that a named profile kills the helper; each such run leaves a crash report in `~/Library/Logs/DiagnosticReports`.
