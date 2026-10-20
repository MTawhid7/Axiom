# Spikes (Phase Zero)

This directory holds validation experiments. The code is **throwaway and not production**: it exists to answer the questions in a step's plan, and its results live in `docs/research/`.
- S1 and S2 validate [ADR-0002](../docs/adr/0002-implementation-language.md) ([Step 2b plan](../docs/research/step-2b-validation-spikes.md)).
- S4 validates [ADR-0004](../docs/adr/0004-process-model-and-sandbox.md) ([Step 3 plan](../docs/research/step-3-security-and-process-model.md); design in the [macOS confinement note](../docs/research/macos-process-confinement.md) §7).
- S3 validates ADR-0003's obligation C1 for [ADR-0008](../docs/adr/0008-script-agents-and-heap.md), and measures dispatch, layout and barriers ([Step 4 plan](../docs/research/step-4-architecture-proposal.md) §10).
- S5 tests the text stack for [ADR-0011](../docs/adr/0011-text-fonts-and-unicode.md), and ImageIO in the renderer's sandbox for [ADR-0012](../docs/adr/0012-graphics-images-and-media.md) ([Step 5 plan](../docs/research/step-5-subsystems.md) §10.1).
- S6 measures compositing on Metal for [ADR-0012](../docs/adr/0012-graphics-images-and-media.md) ([Step 5 plan](../docs/research/step-5-subsystems.md) §10.2).
- S7 validates the network process's sandbox and TLS for [ADR-0013](../docs/adr/0013-networking-storage-and-workers.md) ([Step 5 plan](../docs/research/step-5-subsystems.md) §10.3).

| Spike | Question | Crates |
|---|---|---|
| S1 | Can a GC heap shared by JS and the DOM keep `unsafe` inside the GC boundary, with acceptable ceremony and cost? Pattern A (gc-arena style) against Pattern B (Nova style) | `s1/*` |
| S2 | objc2 AppKit + Metal + CoreText without Xcode: ergonomics, `unsafe` surface, build cost | `s2/window` |
| S4 | Can a helper launched with `posix_spawn` enter a custom deny-by-default Seatbelt profile on macOS 27, and what does that cost? Where can it generate code? Does CoreText work inside it? | `s4/sandbox`, `s4/text` |
| S3 | Can a root stack without borrow tracking cost at most 1 ns per root and per read (C1)? How much rooting does an interpreter with callback-heavy builtins do? How fast is each bytecode dispatch technique on stable Rust? Do layouts without index arithmetic speed up traversal (C2)? What do generated setters and a card-marking barrier cost (C3)? | `s3/gc`, `s3/vm`, `s3/bench` |
| S6 | What do Axiom's own Metal pipelines cost, drawing page-shaped display lists into tiled IOSurface layers of a 2,560 × 1,600 window? What does damage-limited drawing save? Do scrolling and animation by layers stay within 1 ms a frame? How do CPU and GPU path filling compare? What do the device and tiles cost in memory, and are Display P3 tags honoured? | `s6/compositor` |
| S5 | Does HarfRust shape like CoreText on this Mac's system fonts? How do Rust rasterizers' glyphs compare with CoreText's? Do valid and malformed web fonts go through a Rust-only path inside the renderer's profile? What does each text stack cost in footprint, speed and size? Does ImageIO decode AVIF in the renderer's sandbox, and how do the safe image decoders compare with it? | `s5/rust-text` (no `unsafe`, no Apple framework), `s5/text`, `s5/image` |
| S7 | Does a TLS client verify servers through Security's trust evaluation inside a deny-by-default network profile, and which rules does it need? How do rustls and Network framework compare in handshakes, CPU per megabyte and footprint? Does DNS-SD resolve addresses and HTTPS records from kqueue in the sandbox? Does one `kevent64` wait drive rustls, QUIC, DNS-SD, a Mach port and timers, with no wakeups while idle? | `s7/net` |

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
  | `tools/s5-fetch.sh` | S5: downloads the test web fonts listed in `spikes/s5/corpus/webfonts.txt` from web-platform-tests at its pinned commit, checking each file's git blob hash |
  | `tools/s5-images.sh` | S5: makes the test images from pictures that ship with macOS and a generated pattern |
  | `tools/s6-measure.sh` | S6: every question on a release build, three runs after cool-downs, with `top` sampling energy during the damage runs; needs the Mac idle and the window visible |
  | `tools/s6-tails.py` | S6: from a run's traces, the frames over S6-Q1's and S6-Q3's thresholds, and the surfaces allocated per tile drawn |
  | `tools/png-region-stats.py` | Pixel values of regions of a PNG, with its cICP chunk and ICC colorants: S6-Q5's Display P3 check on a screenshot of the test pattern |
  | `tools/s5-measure.sh` | S5: every question on signed release builds: shaping and rasterization against CoreText, web fonts and mutants in the renderer's profile, footprint, speed and size, and image decoding in and out of the sandbox |
  | `tools/s7-measure.sh` | S7: every question on a signed release build, three runs after cool-downs, with read-only requests to public test hosts; then removal tests of each profile rule, and reporting runs that list every operation the helper attempts |

  `cargo bench -p s1-bench` runs one pass of the S1 runtime suites. `cargo run -p s2-window -- --smoke` checks the S2 window's rendering (it opens a window briefly).
- **S3.** `cargo test -p s3-gc -p s3-vm -p s3-bench` runs the heap and root-stack tests, the interpreter's oracle (every technique on every stack, at normal pacing and when collecting at every allocation) and a quick pass of every benchmark suite. Add `--features count` for the check that bytecode never roots. C1's stack, `SliceStack`, has no `unsafe`; Miri covers the archived `RawStack`'s: `cargo +nightly miri test -p s3-gc`, and `cargo +nightly miri test -p s3-vm --test miri`. `target/release/s3-probe roots` prints the root stacks' unit costs, and `/usr/bin/time -l target/release/s3-probe footprint slice 1000` shows what a stack commits.
- **S4.** `cargo test -p s4-sandbox` runs the sandbox probes against the linker-signed helper. `tools/s4-sign.sh` signs copies ad hoc with the hardened runtime (plus the JIT variants), and `target/s4/bin/s4-broker probe|startup|ipc|jit|text` runs each scenario against them. `tools/s4-measure.sh` runs them all on release builds and writes `target/s4-results/<timestamp>/`. `--named-profile`, and `cargo test -p s4-sandbox -- --ignored`, also check that a named profile kills the helper; each such run leaves a crash report in `~/Library/Logs/DiagnosticReports`.
- **S5.** `cargo test -p s5-rust-text -p s5-text -p s5-image` runs the container, shaping, tracking, rasterizer and decoder tests, and a Latin line against CoreText. `tools/s5-fetch.sh` and `tools/s5-images.sh` prepare the inputs in `target/s5/`. Then `target/release/s5-text shape|raster|speed|footprint`, `target/release/s5-webfont check FILE...` and `target/release/s5-image decode|compare` run each question by hand, and `tools/s5-measure.sh` runs them all and writes `target/s5-results/<timestamp>/`. Its `--crash-checks` runs what is expected to crash, and moves the crash reports into the results.
- **S6.** `cargo test -p s6-compositor` runs the tiling, text, scene, path and counter tests. `target/release/s6-compositor q1|q2|q3|q5` opens a 1,280 × 800-point window at the top left of the screen (it can't appear over another app in full screen); `q4` runs offscreen. `tools/s6-measure.sh` runs them all and writes `target/s6-results/<timestamp>/`; its formal runs need AC power, the Mac otherwise idle, and the window visible, which each run records. `python3 -I tools/s6-tails.py target/s6-results/<timestamp>` counts the frames over each threshold, and `tools/png-region-stats.py` reads a screenshot of the colour pattern (`q5 --show`).
- **S7.** `cargo test -p s7-net` runs the HTTP/1.1 framing, QPACK and varint, kqueue (sockets, timers, user events and Mach messages from one wait), socket, counter and record-parsing tests, offline. `target/release/s7-net MODE` is the broker: it compiles `spikes/s7/net/profiles/network.sb` and launches itself as the sandboxed helper (`--profile none` runs it unsandboxed); the modes are `verify`, `probe`, `handshakes`, `download`, `dns`, `one-wait` and `ready`, and every mode but `ready` and `probe` contacts public hosts. `tools/s7-measure.sh` runs them all and writes `target/s7-results/<timestamp>/`.
