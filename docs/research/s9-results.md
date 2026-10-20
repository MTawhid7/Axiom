# S9 Results: the Test Tools on This Mac

| | |
|---|---|
| **Status** | **Complete** (2026-10-20). **S9-Q1 passes:** cargo-fuzz finds the planted bug, runs 10 minutes and reports coverage, under AddressSanitizer. **S9-Q2 fails** with the installed nightly: ThreadSanitizer's runtime crashes while starting, on macOS 27.2; Apple's own ThreadSanitizer works. **S9-Q3 passes:** wptrunner runs `dom/nodes` against Chrome Canary within memory, and reproduces wpt.fyi's pass rate |
| **Question** | Do Axiom's planned test tools work on this Mac: cargo-fuzz with libFuzzer, AddressSanitizer and coverage (S9-Q1); ThreadSanitizer (S9-Q2); wptrunner within 8 GB (S9-Q3)? |
| **Feeds** | The [testing strategy](../testing.md)'s TS9–TS11 and TS18; [Milestone 1](../milestone-1.md)'s criteria and authorization (MA7, MA10) |
| **Plan** | [Testing](../testing.md) §9, authorized at Step 6's Gate B (2026-10-20): "Spike S9 (Testing and Fuzzing Tooling): Authorized. Proceed with validating cargo-fuzz, AddressSanitizer/ThreadSanitizer, and wptrunner on this Mac." |
| **Code** | `spikes/s9/`: `decoder` and `race` (workspace crates) and `fuzz` (cargo-fuzz's targets, its own workspace); `tools/s9-measure.sh` |
| **Last updated** | 2026-10-20 |

**Evidence labels** follow the [research conventions](README.md): **measured** facts come from the runs in §2; **observed** facts were read on this Mac, naming the file or command; **sourced** facts link to their source; **inference** marks an explanation.

---

## Summary

- **S9-Q1, fuzzing: passes.**
  - cargo-fuzz 0.13.2 built both targets on nightly with AddressSanitizer: the binaries link Rust's ASan runtime, which answers with its options.
  - **The planted bug fell in about a second,** in both runs: 77,302 executions the first time. The fuzzer had to build lists three deep to reach it.
  - **The checked decoder ran 10 minutes, twice,** with no finding, its round trip holding: 80.0 and 59.2 million inputs, at 133,064 and 98,567 a second.
  - **Coverage:** 93.25% of regions and 93.90% of lines. The misses are the planted entry point, which this target never calls, and the depth limit: in 80 million inputs the fuzzer never nested lists nine deep.
  - **Two findings:** cargo-fuzz didn't rebuild the standard library, though its help says it does by default; and a fuzzer under AddressSanitizer held about 0.5 GB.
- **S9-Q2, ThreadSanitizer: fails with the installed nightly** (1.96.0, 2026-04-08):
  - every test binary built with it crashes before `main`: the runtime's `dispatch_once` interceptor dereferences null while macOS's dyld calls it during the runtime's start;
  - **Apple clang 21's ThreadSanitizer works here:** it reported a planted race in C;
  - so the fault is the nightly's runtime, not macOS (inference). Milestone 1's plan pins a nightly whose runtime works.
- **S9-Q3, wptrunner: passes.**
  - All 435 tests of `dom/nodes` ran against Chrome Canary 158.0.8113.0, headless, through ChromeDriver 158.0.8113.0, in 140 s, including the first setup of the virtual environment.
  - **99.6% passed,** against 99.5% in wpt.fyi's Chrome run at the same revision.
  - **Memory:** the run's processes peaked at 1,152 MB resident, summed, and free memory stayed at 33% or more.

## 1. What was built

- **`s9-decoder`,** safe Rust, `#![forbid(unsafe_code)]`. A decoder of length-prefixed records, the shape of an IPC message: the magic `AXM1`, then records with a tag, a LEB128 length and a payload; tags for text, numbers and nested lists, at most eight deep. `decode` checks every length. `decode_planted` is the same decoder with a planted logic bug: inside a list nested at least two deep, it trusts a numbers record's length, so a record that claims more bytes than remain panics. **The planted bug is a panic in safe code,** the realistic bug class for Axiom's parsers, which hold no `unsafe` (rule 5). Unit tests cover round trips, every error, and the planted panic. `s9-seeds` writes four seed messages, nested one deep at most.
- **`s9-fuzz`,** cargo-fuzz's crate, in its own workspace: the targets `decode`, with a round trip as its oracle, and `planted`. Its one dependency is `libfuzzer-sys`, which cargo-fuzz's harness needs.
- **`s9-race`:** a counter that threads share through atomics, and a test-only `planted` module that shares one with no synchronization, the race ThreadSanitizer must report. The racy test is ignored by ordinary runs; it is listed in `tools/unsafe-policy.txt`, `unsafe` only in `src/planted/`.
- **`tools/s9-measure.sh`:**
  - `setup`: cargo-fuzz under `target/`, and the nightly's `llvm-tools-preview` component;
  - `fuzz` and `coverage`: S9-Q1;
  - `tsan`: S9-Q2, which moves any crash report into the results;
  - `wpt-setup` and `wpt`: a sparse checkout of WPT at the standards' revision, ChromeDriver for the installed Canary, and the run, sampling memory every 2 s.

## 2. Method and environment

| | |
|---|---|
| **This Mac** | MacBook Air (M1, 8 GB), macOS 27.2 |
| **Toolchains** | Rust nightly 1.96.0 (`900485642`, 2026-04-08), with `rust-src`, and `llvm-tools-preview` added for S9; cargo-fuzz 0.13.2, built from crates.io into `target/s9/tools`; `libfuzzer-sys` 0.4.13. Apple clang 21.0.0 (Command Line Tools), for S9-Q2's control |
| **WPT** | `0f26972422b15f587e2afbd38dbb6ea2397fded0` (committed 2026-10-18), the revision the standards baseline calibrated on, checked out sparsely (177 MB). wptrunner's Python packages in a virtual environment inside it, made by Python 3.14.3 |
| **Browser** | Chrome Canary 158.0.8113.0 with ChromeDriver 158.0.8113.0, Chrome for Testing's build for exactly that version |
| **Runs** | Under `target/s9-results/`, which git ignores: the first fuzzing run `20261020-202455` (on battery, Low Power Mode on); the repeat `20261020-204759` (AC power, Low Power Mode off); coverage and ThreadSanitizer `20261020-204048`; WPT `20261020-204228` |
| **Conditions** | The first fuzzing run, at 20:24, found the Mac on battery with Low Power Mode on (observed in its `environment.txt`), which slows timings by about 1.6× on this Mac. Every later run was on AC power with Low Power Mode off, so the fuzzing step was repeated. Other work ran during both fuzzing runs: a git clone of WPT during the first, a git checkout of WPT's `css/CSS2` during the repeat. Load averages 2.1–4.4 at the starts |

## 3. S9-Q1: fuzzing with AddressSanitizer and coverage

| Measure | First run (battery, Low Power Mode) | Repeat (AC power) |
|---|---|---|
| Build of both targets | 18 s | None needed: the same binaries |
| `planted`: time to the bug | 2 s; 77,302 executions | 1 s; 102,190 executions |
| The bug | `range end index 42 out of range for slice of length 31`, at the planted line, three `records` frames deep | `range end index 113 out of range for slice of length 4`, at the planted line |
| Reproduced from the saved input | Yes | Yes |
| `decode`: 600 s | 79,971,856 executions, 133,064 a second; no finding | 59,239,215 executions, 98,567 a second; no finding |
| Coverage counters reached | 262 edges, 1,452 features; corpus of 280 inputs, 81 KB | 262 edges, 1,433 features; corpus of 264 inputs, 52 KB |
| Peak memory | libFuzzer's `peak_rss_mb` 534; maximum resident 561 MB | libFuzzer's `peak_rss_mb` 524; maximum resident 550 MB |

- **AddressSanitizer is in the binaries** (observed): `otool -L` lists `@rpath/librustc-nightly_rt.asan.dylib`, and `ASAN_OPTIONS=help=1` prints "Available flags for AddressSanitizer". **The standard library wasn't rebuilt:** the build compiled no `core` or `std` crate, though `cargo fuzz run --help` says `--build-std` "defaults to true". So the spike's code and the harness are instrumented, and std isn't (observed in `fuzz-build.txt`).
- **The planted bug is a panic,** which libFuzzer reports as a deadly signal, with the panic's message and the stack.
- **The round-trip oracle held** for every input `decode` accepted.
- **Throughput varied by a third between the runs,** and the repeat on AC power was the slower. Each run grows its own corpus, and other work shared the Mac (§2), so Low Power Mode's effect isn't separable here (inference). S9-Q1's criteria don't depend on it.
- **Coverage** (`cargo fuzz coverage`, then `llvm-cov report`): 163 regions with 11 missed, 93.25%; 11 functions with one missed, 90.91%; 82 lines with 5 missed, 93.90%. The missed lines are `decode_planted` and its planted branch, which this target never calls, and the depth limit's rejection.
- **The depth limit is the instructive miss.** In 80 million inputs, the fuzzer never built lists nine deep, since every level needs a length prefix that matches its contents. Byte-level mutation finds shallow structure fast and deep structure slowly (inference).

## 4. S9-Q2: ThreadSanitizer

- **Built,** with `RUSTFLAGS=-Zsanitizer=thread` and `-Zbuild-std` for `aarch64-apple-darwin`: the standard library was rebuilt, and the test binary links `@rpath/librustc-nightly_rt.tsan.dylib` (observed).
- **Every run crashed before `main`,** the clean test and the planted race alike: `SIGSEGV`, `KERN_INVALID_ADDRESS at 0x0`, before ThreadSanitizer printed anything, even with `TSAN_OPTIONS=verbosity=2`. The crash report's faulting thread reads, innermost first:
  - `__tsan::SlotLock(__tsan::ThreadState*)`;
  - `__tsan::Release`;
  - `wrap_dispatch_once`;
  - `dyldFrameworkVtable()` in `libdyld.dylib`;
  - `dyld_shared_cache_iterate_text`;
  - `__sanitizer::get_dyld_hdr()`;
  - `__sanitizer::MemoryMappingLayout`;
  - `__tsan::CheckAndProtect`;
  - `__tsan::InitializePlatform()`;
  - `__tsan::Initialize`.

  So while the runtime maps memory at start, macOS's dyld calls `dispatch_once`, which ThreadSanitizer intercepts before its own thread state exists (observed in the report; the reading is inference).
- **The control:** a C program with a planted race, built by Apple clang 21.0.0 with `-fsanitize=thread`, reported "WARNING: ThreadSanitizer: data race" at the planted line (observed). Apple's runtime works on macOS 27.2, and the nightly's runtime of April 2026 doesn't (inference: it predates macOS 27's dyld).
- **A web search** on 2026-10-20 found no upstream report of this signature.
- **Clean-up:** each crash left a report in `~/Library/Logs/DiagnosticReports`. Four were removed by hand; the script now moves them into the results. The C control's report was moved there too.

## 5. S9-Q3: wptrunner within memory

| Measure | Value |
|---|---|
| Directory | `dom/nodes`: 435 tests, 15,369 subtests |
| Command | `./wpt run --channel canary --binary <Canary> --webdriver-binary <ChromeDriver> --headless --processes 1 --no-fail-on-unexpected --no-install-fonts chrome dom/nodes` |
| Wall time | 140 s, including the virtual environment's creation; the tests ran in about 122 s |
| Results | 15,804 results: 15,747 as expected, 57 not. Harness statuses: 408 OK, 26 PASS, 1 FAIL; no error, timeout or crash |
| **Pass rate, as wpt.fyi counts** | **99.6%**, against wpt.fyi's Chrome 99.5%, Firefox 98.6%, Safari 97.8% and Ladybird 98.7% at the same revision (`tools/wpt-pass-rates.py`, with the counting corrected on 2026-10-20: §6) |
| Memory, every process of the run summed | Peak 1,152 MB resident, across 27 processes; mean 714 MB over 70 samples. Resident sums count shared pages once per process, so they overstate |
| Free memory | 33% at its lowest, for the whole Mac |

- **No system change was needed** (observed): wptrunner's Chrome product maps the test hosts with `--host-resolver-rules`, so `/etc/hosts` stayed untouched. Fonts weren't installed.
- **The sparse checkout** needed `docs/`, for the `commands.json` that `./wpt` reads at start; the first attempt failed without it.
- **Headless:** `--headless` makes wptrunner pass `--headless=new` to Chrome (its `chrome.py`). The run's `run_info` says `headless: false`, the default it records when a product doesn't set the field.

## 6. Implications for Axiom

Inferences, for the testing strategy and Milestone 1:
- **TS9 stands:** cargo-fuzz with libFuzzer works on this target. **Fuzzing `unsafe` code needs `--build-std` named explicitly** if std's internals should be instrumented; Axiom's parsers hold no `unsafe`, so their panics are what fuzzing finds there.
- **Throughput and memory:** a small decoder ran at 99,000–133,000 inputs a second, and a fuzzer under AddressSanitizer held about 0.5 GB. **On 8 GB, campaigns run one or two at a time,** not one per core.
- **Nested formats need help:** dictionaries, seeds from the suites (TS10), and structure-aware generation, such as `arbitrary`, for parsers of deep structure: the DOM's trees, IPC messages, CSS's nesting.
- **TS11's ThreadSanitizer waits for a working runtime.** Milestone 1 pins a nightly at its start, checked with `tools/s9-measure.sh tsan` (MA7, MA10). Until then, Miri's own data-race detection covers `unsafe` code, and the threaded crates rely on review.
- **TS18 stands:** WPT runs by directory on this Mac, within memory, and wptrunner's results agree with wpt.fyi's. An `axiom` product follows Ladybird's (TS2).
- **WPT's testharness tests are script,** so an engine without script runs only reference and crash tests. Milestone 1 runs html5lib's tree-construction tests from WPT's `.dat` files in its own Rust runner (MA4).
- **`tools/wpt-pass-rates.py` had left reference tests out.** wpt.fyi's summaries give a reference or crash test no subtest counts, only a status, and the tool read only the counts. Found while computing Milestone 1's target on `css/CSS2`'s reference tests, it was corrected, and the [standards baseline](../standards.md) §6's calibration recomputed: 42 of its 48 rows changed, `css/CSS2/`'s median from 98.9% to 98.1%. The tool also gained `--manifest` and `--types`, to count one type of test.

## 7. Threats to validity

- **The first fuzzing run was on battery with Low Power Mode on,** so its throughput is low; the repeat is on AC power (§2).
- **One small decoder** stands for Axiom's parsers; larger targets run slower and cover less per minute.
- **ThreadSanitizer was tried with one nightly.** A newer one may work; S9 didn't install another toolchain, which its authorization didn't list.
- **WPT ran one directory,** of testharness tests, at one process. Reference tests, and directories that need HTTPS or HTTP/2, weren't run.
- **Memory sums of resident sizes** overstate a multi-process run's footprint.

## 8. Reproducing these results

```sh
cargo test -p s9-decoder -p s9-race       # the unit tests; the racy test is ignored
tools/s9-measure.sh setup                 # cargo-fuzz into target/s9/tools; llvm-tools-preview
tools/s9-measure.sh fuzz coverage         # S9-Q1 (10 minutes per fuzzing run; --seconds N)
tools/s9-measure.sh tsan                  # S9-Q2
tools/s9-measure.sh wpt-setup wpt         # S9-Q3 (--wpt-dir DIR; WPT_PYTHON=...)
python3 -I tools/wpt-pass-rates.py canary=target/s9-results/<run>/summary_v2.json \
    chrome=target/s8/wpt-runs/chrome.json.gz --dirs /dom/nodes/
```

The WPT run's `summary_v2.json` is converted from its `wptreport.json` with wpt.fyi's counting: passing subtests over subtests, or the test itself when it has none.

## Sources

All read on 2026-10-20.
- [R1] The Rust Unstable Book, "sanitizer", as the [testing strategy](../testing.md) cites it ([F3]) — https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html
- [R2] cargo-fuzz 0.13.2, `cargo fuzz run --help` and `cargo fuzz coverage --help` (observed, in the installed binary)
- [W1] web-platform-tests at `0f26972422`, `tools/wptrunner/wptrunner/browsers/chrome.py` (the `--headless=new` and `--host-resolver-rules` arguments), `tools/wpt/paths`, `html/syntax/parsing/README` — https://github.com/web-platform-tests/wpt/tree/0f26972422b15f587e2afbd38dbb6ea2397fded0
- [W2] Chrome for Testing, ChromeDriver 158.0.8113.0 for `mac-arm64` — https://storage.googleapis.com/chrome-for-testing-public/158.0.8113.0/mac-arm64/chromedriver-mac-arm64.zip
