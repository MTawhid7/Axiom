# S12 Results: a ThreadSanitizer Route on macOS 27

| | |
|---|---|
| **Status** | **Complete** (2026-10-21). **Both routes work.** S12-Q1: the nightly of 2026-10-21 runs ThreadSanitizer with its own runtime on macOS 27.2. S12-Q2: the nightly S9 found broken, of 2026-04-08, runs it with `-Z external-clangrt` and the Command Line Tools' runtime. In each, the clean test passes and the planted race is reported. **Recommended for Milestone 1:** pin a nightly of 2026-10-21 or later with its own runtime, and keep `-Z external-clangrt` as the fallback |
| **Question** | Is there a route to a working ThreadSanitizer for Rust on this Mac: an updated nightly's own runtime (S12-Q1), or rustc's instrumentation with Apple's runtime through `-Z external-clangrt` (S12-Q2)? |
| **Feeds** | The [testing strategy](../testing.md)'s TS11, step 4; [Milestone 1](../milestone-1.md)'s MA7, MA10 and MA12; the architecture review's [RV1-11](../review/round-1/02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer) |
| **Plan** | The review's [actions](../review/round-1/04-round-2-actions.md#4-proposed-spikes) §4, authorized by the owner on 2026-10-21: "Spike S12: ThreadSanitizer workaround routes on macOS 27 (-Z external-clangrt)." RV1-11's disposition names both routes: "TSan via updated nightly or -Z external-clangrt" |
| **Code** | No new crate: S9's `spikes/s9/race`; `tools/s12-measure.sh` |
| **Last updated** | 2026-10-21 |

**Evidence labels** follow the [research conventions](README.md): **measured** facts come from the run in §1; **observed** facts were read on this Mac, naming the file or command; **inference** is marked.

## 1. The run

`tools/s12-measure.sh bundled external baseline`, 2026-10-21 14:49–14:50, results in `target/s12-results/20261021-144927/` (measured). The environment (observed, `environment.txt`): macOS 27.2 (26B118); Apple clang 21.0.0 (clang-2100.3.34.2); `nightly-2026-10-21`, rustc 1.101.0-nightly (19833af72, 2026-10-20); `nightly`, rustc 1.96.0-nightly (900485642, 2026-04-08), the one S9 used.

Each route builds S9's `s9-race` crate with `-Zsanitizer=thread` and the standard library rebuilt (`-Zbuild-std`), then runs two tests: the atomic counter, which must pass, and the planted race, which must be reported. `TSAN_OPTIONS=abort_on_error=0` makes a reported race exit with code 66 rather than abort.

| Route | Toolchain | Runtime the test binary links (`otool -L`) | Clean test | Planted race |
|---|---|---|---|---|
| **S12-Q1: bundled** | 2026-10-21 nightly | `@rpath/librustc-nightly_rt.tsan.dylib` | Passes, exit 0 | **2 reports**, "data race mod.rs:18" in `Racy::add_one`; exit 66 |
| **S12-Q2: external** | 2026-04-08 nightly, with `-Zexternal-clangrt -Clink-arg=-L$RT -Clink-arg=-lclang_rt.tsan_osx_dynamic -Clink-arg=-Wl,-rpath,$RT`, where `$RT` is `/Library/Developer/CommandLineTools/usr/lib/clang/21/lib/darwin` | `@rpath/libclang_rt.tsan_osx_dynamic.dylib` | Passes, exit 0 | **2 reports,** the same line; exit 66 |
| Baseline: S9's failure | 2026-04-08 nightly, its own runtime | `@rpath/librustc-nightly_rt.tsan.dylib` | **Crashes before `main`** (`SIGSEGV`), as in S9 | Not run |

**The planted race** is a counter in an `UnsafeCell`, shared between threads by an `unsafe impl Sync` and incremented by each of them without synchronization, in `spikes/s9/race/src/planted/mod.rs`; line 18 is the increment. Each run reports it twice (observed). An earlier run by hand of the external route, without `abort_on_error=0`, printed "Running under ThreadSanitizer v3" and the same report, then aborted, leaving a crash report, which was moved out of `~/Library/Logs/DiagnosticReports` (observed).

## 2. What it means

- **S12-Q1 passes:** a nightly from October 2026 has a runtime that starts on macOS 27.2. **Inference:** S9's crash, in `__tsan::InitializePlatform` while dyld called `dispatch_once`, was fixed upstream between the two nightlies; no upstream change was looked for.
- **S12-Q2 passes:** rustc's instrumentation from April works with Apple clang 21's runtime. **The risk it carries** is an interface between two LLVM versions that nobody tests together: the runtime changes with the Command Line Tools, the instrumentation with the nightly (inference).
- **So TS11's fourth step has a route.** The recommendation for Milestone 1's plan (MA10, MA12): pin a nightly of 2026-10-21 or later, with its own runtime, so instrumentation and runtime come from one build; keep `-Z external-clangrt` with the Command Line Tools' runtime as the fallback if a later nightly or macOS update breaks the bundled one; run with `TSAN_OPTIONS=abort_on_error=0`, so a report fails the test without writing a crash report.
- **The first three steps of TS11 don't change:** ThreadSanitizer sees data races in the code it instruments, not logic races or Mach messages ([RV1-11](../review/round-1/02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer)).

## 3. Side effects

- **Installed:** the toolchain `nightly-2026-10-21` (minimal profile, with `rust-src`), beside the existing `nightly`, which Miri and S3's measurements use and which is unchanged. `rustup toolchain uninstall nightly-2026-10-21` removes it.
- **Not intended:** rustup updated itself during the install, from 1.29.0 to 1.29.1 (observed in its output and `rustup --version`). It manages toolchains, not builds, so no measurement depends on it (inference).
- **Crash reports:** the baseline's report and the one from the run by hand were moved into the results under `target/` (observed); `~/Library/Logs/DiagnosticReports` holds none from S12.
- **Build output** stays under `target/s12/`, which git ignores.
