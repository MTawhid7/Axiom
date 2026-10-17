# S4 Results: Sandboxed Helper Processes on macOS 27

| | |
|---|---|
| **Status** | Complete (2026-10-17) |
| **Question** | Can Axiom's helpers enter custom deny-by-default Seatbelt profiles on macOS 27.2, signed ad hoc and built with the Command Line Tools (S4-Q1)? Does the profile deny what it should while the helper keeps working (S4-Q2)? What does a sandboxed helper cost, and what does IPC cost (S4-Q3)? Where can a helper generate code (S4-Q4)? Does CoreText work inside a renderer profile (S4-Q5)? |
| **Feeds** | [ADR-0004](../adr/0004-process-model-and-sandbox.md) (acceptance and refinements); [process model](../security/process-model.md) §7 and §10; OQ-17 (IPC transport); Steps 4 and 5 |
| **Plan** | [macOS confinement note](macos-process-confinement.md) §7; [Step 3 tracker](step-3-security-and-process-model.md), CP6–CP8 |
| **Code** | `spikes/s4/sandbox` (`s4-sandbox`) and `spikes/s4/text` (`s4-text`) on branch `step-3`. Formal run at commit `1ca3e49` |
| **Last updated** | 2026-10-17 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from this repository's binaries and scripts (§9).
- **Inference** marks an explanation drawn from measurements.
- **Hypothesis** marks an untested claim.

---

## Summary

| Question | Answer |
|---|---|
| **S4-Q1** Does a custom profile apply? | **Yes, by three methods.** A Rust helper built against SDK 27.0 (`otool -l`: minos 11.0, sdk 27.0) and signed ad hoc with the hardened runtime (`flags=0x10002(adhoc,runtime)`) enters a deny-by-default profile through `sandbox_init_with_parameters`, through `sandbox_compile_string` + `sandbox_apply`, and through `sandbox_apply` of bytecode compiled by the broker. `sandbox_check` then reports it sandboxed, and nothing kills it. **The SDK 27 rule is confirmed:** a helper that applies a *named* profile (`kSBXProfilePureComputation`) through `sandbox_init` is killed with SIGKILL, termination namespace `SANDBOX` |
| **S4-Q2** Does it deny what it should? | **Yes: 18 of 18 probes behave as specified,** by every method and with both signings. Files, TCP, UDP, name resolution, process launch and signals fail with `EPERM`. Looking up the window server, font server, pasteboard, LaunchServices and its database fails with `BOOTSTRAP_NOT_PRIVILEGED`. Allocation, threads, the inherited socket and a Mach hello to the broker keep working. The hello's kernel audit trailer names the spawned child. The control run allows all 18 |
| **S4-Q3** What does it cost? | **Nothing measurable, if the broker compiles the profile.** A helper that applies broker-compiled bytecode (498 bytes) is ready **2.11 ms** after `posix_spawn`, with a **1.14 MiB** footprint: the same as an unsandboxed helper (2.17 ms, 1.14 MiB). Compiling the profile in the helper adds about 5.7 ms and 2.8 MiB. The sandbox doesn't change IPC cost. **Mach round trips are 24% faster** than a socket pair (3.75 against 4.92 µs for 64 bytes). 1 MiB moves in 172 µs out of line over Mach, and in 150 µs over a socket pair with enlarged buffers (600 µs with the default 8 KiB buffers) |
| **S4-Q4** Where can it generate code? | **Only where both layers allow it.** Under the hardened runtime, `MAP_JIT` fails (`EINVAL`) without `com.apple.security.cs.allow-jit`, whatever the profile. With the entitlement, **`(deny default)` does not stop it; only an explicit `(deny dynamic-code-generation)` does**, for every executable. The one-region limit of Apple's porting guide applies **only with `com.apple.security.cs.single-jit`**. Every entitlement worked with ad-hoc signing |
| **S4-Q5** Does CoreText work inside? | **Yes, pixel-identically,** with the window server and font server denied, given read access to **`/System/Library/Fonts` and `/usr/share/icu`**. Without font files, CoreText silently draws different glyphs. Without ICU's data, it silently places some glyphs differently. A helper that has drawn text has a **4.05 MiB** footprint |
| S4-Q6 Fallbacks | **Not run.** It was conditional on S4-Q1 or S4-Q2 failing, and the owner's scope for S4 listed Q1–Q5. App Sandbox would also have left containers under `~/Library/Containers` |

**Main findings**, beyond the pass criteria:
1. **The browser should compile profiles, not the helpers.** Compilation is where the time and memory go (5.7 ms, 2.8 MiB per process). `sandbox_apply` of precompiled bytecode costs 17 µs. This is how Chromium hands profiles to its helpers ([macOS note](macos-process-confinement.md), source 3).
2. **Default deny doesn't cover JIT.** A renderer profile that relies on `(deny default)` would let a process holding `allow-jit` map executable memory. JIT rule J3's explicit `(deny dynamic-code-generation)` is therefore necessary, not just defence in depth. The denial is logged as `deny(1) dynamic-code-generation`.
3. **The single-JIT-region limit is opt-in**, through `single-jit`, which WebKit's content process also carries (observed).
4. **CoreText fails silently** when the sandbox withholds what it needs. A renderer has to verify its fonts rather than trust a successful draw.

---

## 1. What was built

| Part | What it does |
|---|---|
| `s4-sandbox` library | `src/platform/` holds every foreign call, under invariants X1–X11 (48 `unsafe` blocks, allowlisted): libsandbox (init with parameters, compile, apply, bytecode, check); `posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT` and mapped descriptors; Mach rendezvous (`bootstrap_check_in` and `bootstrap_look_up`); RPC with send-once reply rights, combined reply-and-receive and out-of-line regions; the audit trailer; `proc_pid_rusage`; `MAP_JIT` with per-thread W^X. Messages are built and parsed as bytes in safe code |
| `s4-helper` | Applies its profile first, then runs one mode: `probe` (S4-Q2), `ready` (start-up), `serve` (IPC), `jit` (S4-Q4) or `named-profile` |
| `s4-broker` | Launches helpers and checks their reports: `probe`, `startup`, `ipc`, `jit`, `text` |
| `s4-text` | S2's CoreText rasterizer, inside a profile (S4-Q5). 7 `unsafe` blocks, allowlisted |
| Profiles | `renderer.sb`: deny by default, one `mach-lookup` for the broker's rendezvous name (a parameter), explicit `(deny dynamic-code-generation)`. `renderer-jit.sb`: the same, but allowing dynamic code. `bare.sb`: deny by default and the rendezvous only (evidence for finding 2). `text.sb`: the renderer profile plus read access to system fonts and ICU data |
| Scripts | `tools/s4-sign.sh` signs ad hoc: the broker, the helper and `s4-text` with the hardened runtime; `s4-helper-jit` (`allow-jit`); `s4-helper-jit1` (`allow-jit` + `single-jit`); `s4-helper-plain` (no hardened runtime). `tools/s4-measure.sh` runs everything and saves the results |
| Tests | 15 unit tests and 4 integration scenarios under `cargo test` (control, two in-helper methods, precompiled), plus 1 opt-in test (the named-profile kill) |

## 2. Method and environment

- **Machine.** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B5112a), on AC power; rustc 1.94.1; Command Line Tools only.
- **Builds.** Release builds, signed by `tools/s4-sign.sh`. Every binary records SDK 27.0.
- **Formal run.** `tools/s4-measure.sh --named-profile` at commit `1ca3e49` with a clean tree. Output in `target/s4-results/20261017-134336/`.
  - **Start-up:** 30 launches per configuration.
  - **IPC:** 5 timed batches per series after one warm-up batch: 10,000 echoes of 64 bytes, or 40 transfers of 1 MiB per batch.
- **Footprint** is the process's physical footprint (`ri_phys_footprint` from `proc_pid_rusage`), the figure S2 and Activity Monitor report, read while the helper waits.
- **Bulk transfers.** The receiver checksums every byte with a Fletcher-style sum over 64-bit words. A first per-byte checksum cost about 1 ns per byte and dominated the 1 MiB times, so it was replaced before the formal run.

## 3. S4-Q1 and S4-Q2: entering the profile, and what it denies

| Probe | Control (no profile) | Renderer profile (each method) |
|---|---|---|
| Read a file in the home directory; `/etc/hosts`; a system font | Allowed | Denied, `EPERM` |
| Create a file | Allowed | Denied, `EPERM` |
| TCP connect and UDP send to the broker on `127.0.0.1` | Allowed; the broker sees both | Denied, `EPERM`; the broker sees neither |
| Resolve `localhost` | Allowed | Fails ("nodename nor servname provided") |
| Launch `/usr/bin/true` | Allowed | Denied, `EPERM` |
| Signal the broker (signal 0) | Allowed | Denied, `EPERM` |
| Look up `com.apple.windowserver.active`, `com.apple.fonts`, `com.apple.pasteboard.1`, `com.apple.coreservices.launchservicesd`, `com.apple.lsd.mapdb` | Found | `BOOTSTRAP_NOT_PRIVILEGED` (1100) |
| Allocate 64 MiB; run 4 threads; hello over the inherited socket; Mach hello to the broker | Allowed | Allowed |

**Applying the profile** (formal probe run; single observations):

| Method | Time in the helper |
|---|---|
| `sandbox_init_with_parameters` | 9.2 ms |
| `sandbox_compile_string` + `sandbox_apply` | 6.7 ms + 0.07 ms |
| Precompiled | 0.06 ms to apply 499 bytes |

**Every denial is logged by the kernel,** for example `Sandbox: s4-helper(24549) deny(1) mach-lookup com.apple.fonts`. The formal run's 356 lines are saved with its results.

**The named-profile rule.** The helper ended with `Signal(9)`. Its crash report gives `"termination": {"namespace": "SANDBOX", "code": 1}` and the stack `sandbox_init` → `sandbox_init_with_parameters` → `sandbox_apply_container` → `__mac_syscall`.

## 4. S4-Q3: cost

**Start-up and footprint** (30 launches each; median, with the range):

| Configuration | Spawn to ready | Footprint when ready | Profile work in the helper |
|---|---:|---:|---|
| No profile | **2.17 ms** (2.06–2.63) | **1.14 MiB** | — |
| `sandbox_init_with_parameters` | 8.71 ms (7.93–10.10) | 3.94 MiB | 6.1 ms |
| `sandbox_compile_string` + `sandbox_apply` | 8.14 ms (7.67–9.75) | 3.94 MiB | 5.7 ms compile + 33 µs apply |
| **Bytecode compiled by the broker** | **2.11 ms** (1.99–2.90) | **1.14 MiB** | **17 µs** apply |

**Inference.** Compiling SBPL inside each helper costs about 6 ms and 2.8 MiB. Its footprint stays after the profile is applied, which suggests libsandbox's compiler state remains resident. With 16 renderers (B5), that would be about 45 MiB of avoidable memory. A helper that only applies bytecode costs what an unsandboxed process costs. A minimal C process (`/bin/sleep`) measured 1.03 MB.

**IPC** (median of 5 batches; the full ranges are in the run's `ipc.txt`):

| Configuration | 64 B round trip: socket | 64 B round trip: Mach | 1 MiB: socket | 1 MiB: Mach (out of line) |
|---|---:|---:|---:|---:|
| No profile; default socket buffers (8 KiB) | 4.93 µs | **3.81 µs** | 580 µs (1.8 GB/s) | **168 µs** (6.3 GB/s) |
| Renderer profile; default socket buffers | 4.92 µs | **3.75 µs** | 599 µs (1.8 GB/s) | **177 µs** (5.9 GB/s) |
| Renderer profile; 1 MiB socket buffers | 4.99 µs | **3.71 µs** | **150 µs** (7.0 GB/s) | 172 µs (6.1 GB/s) |

**Inference.**
- The sandbox adds nothing to message cost: it checks resource acquisition, not message passing.
- A Mach round trip needs one `mach_msg` per side (send and receive combined). A socket round trip needs a `write` and a `read` per side, which plausibly explains Mach's 24% lead.
- For bulk data, a socket pair needs its buffers raised from macOS's 8 KiB default; Mach out-of-line memory needs no tuning.
- Each 1 MiB figure includes the receiver checksumming every byte.

## 5. S4-Q4: where code can be generated

`MAP_JIT`, then write `mov w0, #42; ret` under per-thread W^X, flush, call. Then a second `MAP_JIT` region while the first exists. "Runs" means it returned 42.

| Executable | No profile | `bare.sb`: `(deny default)` only | `renderer.sb`: explicit deny | `renderer-jit.sb`: explicit allow |
|---|---|---|---|---|
| Hardened runtime, no entitlement | `EINVAL` | `EINVAL` | `EINVAL` | `EINVAL` |
| Hardened runtime + `allow-jit` | Runs; 2nd region OK | Runs; 2nd region OK | **`EINVAL`** | Runs; 2nd region OK |
| Hardened runtime + `allow-jit` + `single-jit` | Runs; **2nd region `EINVAL`** | Runs; 2nd region `EINVAL` | `EINVAL` | Runs; 2nd region `EINVAL` |
| No hardened runtime | Runs; 2nd region OK | Runs; 2nd region OK | **`EINVAL`** | Runs; 2nd region OK |

**Findings.**
- The hardened runtime's entitlement check and the sandbox's `dynamic-code-generation` operation are independent gates. Each one alone can refuse `MAP_JIT`.
- `(deny default)` doesn't include `dynamic-code-generation`. The operation is checked, and logged, only when a rule names it.
- Apple's porting guide says that with the hardened runtime and `allow-jit`, a process "can only create one memory region with the MAP_JIT flag set" ([macOS note](macos-process-confinement.md), source 9). On macOS 27.2 that held only with `single-jit` (measured). Whether the guide's statement is out of date, or describes other platforms, isn't known.
- AMFI logs each ad-hoc helper as "not valid … adhoc signed", yet macOS runs it and honours its `allow-jit` and `single-jit` entitlements (measured).

## 6. S4-Q5: CoreText inside a renderer profile

S2's rasterizer draws two lines at 40 px: Helvetica, and Times New Roman. Each mask is compared, by FNV-1a hash, with an unsandboxed run. Rendering is deterministic: three runs gave identical hashes in each configuration.

| Profile | Helvetica | Times New Roman |
|---|---|---|
| None | 709×44 px, 8,186 ink pixels | 561×49 px, 6,324 ink pixels |
| `text.sb` | **Identical** | **Identical** |
| `renderer.sb` (no font access) | 1795×48 px, 38,181 ink pixels: other glyphs, with no error | 1427×48 px: likewise |

**How the profile was grown.**
- With only `/System/Library/Fonts` readable, Helvetica was identical, but Times New Roman differed pixel by pixel at the same size and ink count.
- Allowing metadata reads, user preferences, directory services, sysctls, even the font server, changed nothing.
- **Bisection settled it.** Starting from `(allow default)` and denying one class of operation at a time showed the cause to be read access to `/usr/share/icu`. Denying every Mach look-up didn't alter the output.
- **Inference:** CoreText uses ICU's character data in layout, and falls back silently without it.
- The window server and font server stay denied in `text.sb`, as in WebKit's content profile.

**Cost.** A helper that has drawn both lines is ready in 10.84 ms with a 4.05 MiB footprint unsandboxed, or 14.27 ms and 6.33 MiB when it compiles `text.sb` itself. With broker-compiled bytecode the difference would shrink to the 17 µs apply, by §4's result (inference; not measured for `s4-text`).

## 7. Implications for Axiom

1. **ADR-0004's D2 holds, with a refinement.** The browser compiles each profile once, with the helper's parameters, and passes the bytecode over the inherited channel. The helper applies it as its first act, before it reads any untrusted input. The private `sandbox_profile_t` layout this relies on is Chromium's (invariant X11).
2. **JIT rules J3 and J5 are confirmed and sharpened.**
   - Every non-JIT profile *must* contain an explicit `(deny dynamic-code-generation)`.
   - `renderer-jit` should carry `single-jit` as well as `allow-jit`.
   - The hardened runtime on every executable is what makes the entitlement decide.
3. **IPC transport (OQ-17).** The evidence favours Mach messages:
   - lower latency;
   - port rights for IOSurfaces and memory, which sockets can't carry;
   - the kernel's audit trailer for peer identity (R3, shown working);
   - out-of-line memory for bulk data without buffer tuning.

   Step 4 decides.
4. **Memory budgets.** A sandboxed helper costs 1.14 MiB, and 4.05 MiB once CoreText has drawn. Both are well inside B2 (10 MB for an empty renderer).
5. **Spawning is fast.** A spare renderer (process model §8) saves about 2 ms for a bare helper. Its value depends on how long a real renderer takes to initialize, which Step 4 should measure.
6. **Text (Step 5).**
   - CoreText can run in the renderer, with system fonts and ICU data readable and the font and window servers denied.
   - User-installed fonts (`/Library/Fonts`, `~/Library/Fonts`) and on-demand system font assets would need further, reviewed rules.
   - A renderer must check that its fonts resolved, because failures are silent.
7. **Threat-model check V1.** The probe suite is a first version of V1's sandbox escape tests. It already runs under `cargo test`.

## 8. Threats to validity

1. **One machine, one OS build.** Results are for macOS 27.2 (26B5112a) on an M1. A later macOS could change any of them, which is why V1 must run on each update.
2. **A stand-in helper.** `s4-helper` links only libSystem and libsandbox, so a real renderer will start more slowly and use more memory. `s4-text` adds CoreText and CoreGraphics. Neither has an engine.
3. **Short measurements.** Start-up was measured over 30 launches and IPC over 5 batches, with the machine otherwise in normal use; memory was under pressure from other applications. The variance shown is small, but one unsandboxed start-up in a trial run took 371 ms.
4. **Single observations** for profile application inside the probe runs (§3). The start-up table has 30.
5. **The JIT test writes one tiny function.** It shows which gates allow `MAP_JIT`, not how a real JIT behaves under them.
6. **The Times New Roman difference** was traced to ICU data by bisection, but the mechanism inside CoreText is inferred, not observed.

## 9. Reproducing these results

```sh
cargo test -p s4-sandbox                      # probes, linker-signed (15 unit + 4 scenarios)
cargo test -p s4-sandbox -- --ignored         # the named-profile kill (leaves a crash report)
tools/s4-sign.sh --release                    # signed copies in target/s4/bin
target/s4/bin/s4-broker probe|startup|ipc|jit|text
tools/s4-measure.sh [--named-profile]         # all of it; target/s4-results/<timestamp>/
```

| Run | Directory |
|---|---|
| Formal run (commit `1ca3e49`, clean tree) | `target/s4-results/20261017-134336/`: `environment.txt`, `probe.txt`, `startup.txt`, `ipc.txt`, `jit.txt`, `text.txt`, `sandbox-denials.txt`, `named-profile-crash-report.ips` |

## Sources

1. This repository: `spikes/s4/sandbox`, `spikes/s4/text`, `tools/s4-sign.sh`, `tools/s4-measure.sh`; the [Step 3 tracker](step-3-security-and-process-model.md), CP6–CP8.
2. Chromium, `sandbox/mac/seatbelt.cc` (checked 2026-10-17): `struct sandbox_profile_t { char* builtin; const uint8_t* data; size_t size; };`. `Seatbelt::Compile` copies `data` and `size`, then frees the profile. `Seatbelt::ApplyCompiledProfile` passes a stack struct with `builtin = nullptr` to `sandbox_apply`. https://chromium.googlesource.com/chromium/src/+/HEAD/sandbox/mac/seatbelt.cc
3. The [macOS confinement note](macos-process-confinement.md) and its sources: Apple's porting guide for JIT compilers (source 9), WebKit's entitlements and profiles (§9).
