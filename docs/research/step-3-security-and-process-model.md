# Step 3 — Threat Model, Trust Boundaries, and the macOS Process and Sandbox Model: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **Complete** (2026-10-17): CP1–CP9. The threat model and ADR-0004 are accepted; spike S4 passed |
| **Authorized** | 2026-10-17, by the owner ("Continue implementation"), as Step 3 design work. Documents only (§2) |
| **Branch** | `step-3`, merged into `main` (`--no-ff`) on 2026-10-17, as the owner instructed |
| **Feeds** | ADR-0004 (process model and sandbox); OQ-4 (JIT policy); the architecture proposal in Step 4 |
| **Last updated** | 2026-10-17 |

> **Resuming in a new session?**
> 1. Read §5 to find the first checkpoint that isn't ✅.
> 2. Check the repository against §7, the results log.
> 3. Continue from that checkpoint's deliverables.

---

## 1. Purpose

Decide what Axiom protects, from whom, and where the trust boundaries lie. Then decide how the browser is split into processes and how each process is confined on macOS, so that a compromised renderer exposes neither credentials, nor other sites' data, nor the file system (ADR-0001 R3).

## 2. Owner input and scope (2026-10-17)

| Input | Effect |
|---|---|
| "Continue implementation", with [status.md](../status.md) naming Step 3 as next | This plan. Step 3 is design work under the Phase Zero rule |
| Code, including spikes, needs a specific authorization ([status.md](../status.md), charter §7) | No code was written in CP1–CP5. Spike S4 was designed in CP3 and put to the owner at the review gate |
| **The owner's review, 2026-10-17:** threat model accepted; L0–L3 and V1–V8 accepted, with out-of-process iframes mandatory for L3; JIT policy accepted, with global and per-site switches; SI1–SI7 accepted as staged, SI3 only at L1 and non-sensitive L2; memory targets accepted as initial baselines; the guiding principle that no architectural corners are cut for convenience | Recorded in ADR-0004's decision history, the threat model (check V9), the process model, open questions (OQ-4, OQ-18 resolved), the decision map and charter principle 7 |
| **Spike S4 authorized:** S4a (S4-Q1, Q2), S4b (S4-Q3, Q4, Q5), S4c (results) | CP6–CP8 (§5) |
| ADR-0004 moves to Accepted once S4-Q1 and S4-Q2 pass | CP8 |
| Push `step-3` now. Merge it into `main` with `--no-ff` once the S4 results are written and ADR-0004 is Accepted | Pushed 2026-10-17; CP9 |

**Evidence gathering without code.** Some facts about this Mac were read with existing system tools, which change nothing on the machine:
- SDK headers, `.tbd` export stubs and man pages;
- the sandbox profiles that macOS ships;
- `codesign -d` and `nm -u` on installed browsers;
- `/usr/bin/time -l` on a system binary.

Nothing was compiled or installed. Each such fact names the command or file it comes from.

## 3. Questions

| ID | Question | Answered in |
|---|---|---|
| Q1 | What does Axiom protect, and from whom? | [Threat model](../security/threat-model.md) §3–§4 |
| Q2 | Where are the trust boundaries? | [Threat model](../security/threat-model.md) §6 |
| Q3 | When may Axiom be used for sensitive logins? | [Threat model](../security/threat-model.md) §8 (security levels and the gate) |
| Q4 | Which processes exist, and what may each one do? | [Process model](../security/process-model.md) §3–§4 |
| Q5 | How is each process confined on macOS, with the Command Line Tools only and no paid developer account? | [macOS confinement note](macos-process-confinement.md); ADR-0004 |
| Q6 | Which rules govern IPC between processes of different trust? | [Process model](../security/process-model.md) §6 |
| Q7 | How much memory may each process use on 8 GB, and how many processes may run? | [Process model](../security/process-model.md) §7 |
| Q8 | JIT policy (OQ-4): under what conditions may a JIT exist, and where is it off? | [ADR-0004](../adr/0004-process-model-and-sandbox.md), D7 |

## 4. Deliverables

1. [`docs/security/threat-model.md`](../security/threat-model.md)
2. [`docs/research/macos-process-confinement.md`](macos-process-confinement.md), a research note that also designs spike S4
3. [`docs/security/process-model.md`](../security/process-model.md)
4. [`docs/adr/0004-process-model-and-sandbox.md`](../adr/0004-process-model-and-sandbox.md) (Proposed), including the JIT policy for OQ-4
5. Updates to the decision map, open questions, indexes and status
6. Spike S4: code in `spikes/s4/`, scripts in `tools/`, results in `docs/research/s4-results.md`

## 5. Checkpoints

**Every checkpoint ends with:**
- every relative link in `docs/` resolving to an existing file. Forward links to this step's own planned documents (§4) are allowed until they are written, and must all resolve by CP5;
- the four standard checks still green (no code changes, so they must stay green): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `tools/check-unsafe-policy.sh`;
- a commit on `step-3` and an update to this tracker.

From CP6 on, the four standard checks cover real code again. S4-Q6, the fallbacks, runs only if S4-Q1 or S4-Q2 fails: the owner's scope for S4 lists Q1–Q5, and App Sandbox would leave containers under `~/Library/Containers`.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-3` | ✅ 2026-10-17 (§7) |
| CP2 | Threat model: assets, adversaries, attack surfaces, trust boundaries, goals and non-goals, security levels | ✅ 2026-10-17 (§7; [threat model](../security/threat-model.md)) |
| CP3 | Research note: process confinement on macOS, browser precedents, observations on this machine; spike S4 design | ✅ 2026-10-17 (§7; [note](macos-process-confinement.md)) |
| CP4 | Process model: process types and privileges, site isolation policy, IPC trust rules, memory budgets, staging | ✅ 2026-10-17 (§7; [process model](../security/process-model.md)) |
| CP5 | Synthesis: ADR-0004 (Proposed) with the JIT policy; decision map, open questions, indexes, status; branch ready for review | ✅ 2026-10-17 (§7; [ADR-0004](../adr/0004-process-model-and-sandbox.md)); reviewed by the owner |
| CP6 | **S4a:** sandbox entry and denial probes (S4-Q1, S4-Q2), automated under `cargo test`; signing script | ✅ 2026-10-17 (§7): **S4-Q1 and S4-Q2 pass** |
| CP7 | **S4b:** measurements: helper start-up and footprint, IPC latency and throughput over Mach messages and socket pairs (S4-Q3); the JIT matrix (S4-Q4); CoreText inside a renderer profile (S4-Q5) | ✅ 2026-10-17 (§7) |
| CP8 | **S4c:** `docs/research/s4-results.md`. ADR-0004 moves to Accepted if S4-Q1 and S4-Q2 pass. Docs synthesis | ✅ 2026-10-17 (§7; [S4 results](s4-results.md)); ADR-0004 **Accepted** |
| CP9 | Merge `step-3` into `main` (`--no-ff`) | ✅ 2026-10-17 |

## 6. Method

- **Sources** follow the [research conventions](README.md): primary sources first, each dated; claims labelled as sourced fact, inference, hypothesis or background.
- **Primary sources on this machine** (macOS 27.2, Command Line Tools SDK 27.0): headers, export stubs and man pages; WebKit's shipped sandbox profiles; entitlements and imports of the installed Safari, Chrome Canary and Tor Browser (Firefox) builds.
- **Web sources:** Chromium, WebKit, Firefox and Ladybird documentation and source; Apple developer documentation; the WHATWG and W3C specifications; the Site Isolation paper (USENIX Security 2019).

## 7. Results log

### CP1 — completed 2026-10-17

- Branch `step-3` created from `main` at `33df346`.
- This plan written.
- The four standard checks pass unchanged (recorded at commit time).

### CP2 — completed 2026-10-17

[Threat model](../security/threat-model.md) written:
- assumptions TA1–TA5, including that Apple's complex-format parsers are not trusted with untrusted input in a privileged process;
- assets AS1–AS9 and adversaries AD1–AD4, with the compromised renderer (AD2) treated as an expected case;
- attack surfaces mapped to the process that must handle each;
- trust boundaries TB1–TB7 and trust zones Z0–Z4;
- goals SG1–SG12 and non-goals NG1–NG8;
- security levels L0–L3. The sensitive-use gate (L3) requires checks V1–V8.

New question raised: OQ-16, WebAuthn and passkeys without a paid developer account.

### CP3 — completed 2026-10-17

[macOS confinement note](macos-process-confinement.md) written. Main findings:
- Chrome, Firefox and WebKit all confine content processes with Seatbelt profiles applied through undocumented libsandbox functions (observed with `nm -u` and in WebKit's shipped profiles).
- In the macOS 27 SDK, the named sandbox profiles kill the process; custom profiles still work for binaries built with SDK 27 (Chrome Canary 158).
- App Sandbox's base profile is too broad for a renderer, and can't make a child stricter than its parent.
- Multiple XPC service instances need WebKit's private `_MultipleInstances` key.
- BrowserEngineKit is not available to an ad-hoc-signed browser.

Recommendation for ADR-0004: helpers launched with `posix_spawn` that apply their own deny-by-default profile first (option O1); the browser process unsandboxed but hardened. Spike S4 (Q1–Q6) is designed in §7 of the note and needs the owner's authorization.

Correction during review: WebKit's XPC services don't carry the hardened-runtime flag. They are platform binaries with library validation only (`codesign -dv`).

### CP4 — completed 2026-10-17

[Process model](../security/process-model.md) written:
- principles PM1–PM8, including the Rule of 2 adapted to Axiom;
- five process types (browser, renderer, network, GPU, utility), with an OS-capability matrix and a Rule of 2 placement table;
- site isolation rules SI1–SI7. Cross-site iframes stay in the embedder's process until out-of-process iframes exist, under three compensating rules (partitioned third-party state, ORB, no sensitive site in another site's process);
- IPC requirements R1–R6 and trust rules IR1–IR10;
- memory targets B1–B7, based on S2, the 1 MB process floor, and the Site Isolation paper's measurements;
- lifecycle, and staging by security level L0–L3.

### CP5 — completed 2026-10-17

- [ADR-0004](../adr/0004-process-model-and-sandbox.md) written and **Proposed**: decisions D1–D8, including the JIT policy J1–J7 for OQ-4. It recommends acceptance once S4-Q1 and S4-Q2 pass.
- [Decision map](../decision-map.md): F3 marked Proposed; Step 3 in review; a new §4 lists the consequences for Step 4.
- [Open questions](../open-questions.md): OQ-4's policy proposed; Step 3 inputs added to OQ-5 and OQ-11; OQ-16 (WebAuthn), OQ-17 (IPC transport) and OQ-18 (out-of-process iframes) added.
- Indexes ([docs](../README.md), [research](README.md)) and [status](../status.md) updated. Status lists the owner's decisions.
- Every relative link in `docs/` resolves, and the four standard checks pass.

**Review gate.** The owner decides on the threat model, the security levels, spike S4, ADR-0004, the JIT policy, SI3, the memory targets, and the merge ([status](../status.md), "Next steps").

### The owner's review — 2026-10-17

Decisions 1–8 are recorded in §2 and in the documents named there. `step-3` was pushed to `origin` for off-site backup. Spike S4 starts at CP6.

### CP6 (S4a) — completed 2026-10-17

**Code.**
- `spikes/s4/sandbox` (package `s4-sandbox`, std only):
  - `src/platform/` holds every foreign call (libsandbox, Mach, `posix_spawn`, libbsm) under invariants X1–X8: 34 `unsafe` blocks, allowlisted in `tools/unsafe-policy.txt`;
  - the probe suite (`probe.rs`), the broker side (`scenario.rs`), the `s4-helper` and `s4-broker` binaries, and the profile `profiles/renderer.sb`. The profile is `(deny default)` plus one `mach-lookup` for the broker's rendezvous name, passed as a parameter.
- `tools/s4-sign.sh` signs copies ad hoc with the hardened runtime: `flags=0x10002(adhoc,runtime)`.

**S4-Q1: passes.**
- The helper is built against SDK 27.0 (`otool -l`: minos 11.0, sdk 27.0). Signed ad hoc with the hardened runtime, it enters the custom deny-by-default profile by **both** methods: `sandbox_init_with_parameters`, and `sandbox_compile_string` + `sandbox_apply`.
- `sandbox_check` then reports it as sandboxed, and nothing kills it.
- Applying the profile took 9–12 ms with `init` and 7–8 ms with `compile`. These are single observations from dev builds; S4b measures properly.

**S4-Q2: passes.** 18 of 18 probes behave as expected, with both methods and both signings (linker-signed under `cargo test`; ad hoc with the hardened runtime from `target/s4/bin`):
- Reading a file in the home directory, `/etc/hosts` and a system font, creating a file, TCP connect, UDP send, process launch and signalling the broker all fail with `EPERM`.
- `getaddrinfo("localhost")` fails.
- Looking up the window server, font server, pasteboard, LaunchServices and its database fails with `BOOTSTRAP_NOT_PRIVILEGED` (1100).
- The broker saw no TCP connection or UDP datagram.
- Allocating 64 MiB, four threads, a hello over the inherited socket, and a Mach hello to the broker all work. The PID in the hello's kernel audit trailer equals the spawned child's PID (process model R3).
- **The control run** (no profile) allows all 18, so every probe can detect the access it tests.

**The SDK 27 rule for named profiles is confirmed.** A helper that applies `kSBXProfilePureComputation` through `sandbox_init` is killed with SIGKILL. Its crash report gives the termination namespace `SANDBOX`, code 1, and the stack `sandbox_init` → `sandbox_init_with_parameters` → `sandbox_apply_container` → `__mac_syscall`. Each such kill leaves a crash report in `~/Library/Logs/DiagnosticReports`, so this check is opt-in: `--named-profile`, or `cargo test -- --ignored`.

**Denials are logged.** The kernel logs every denial to the unified log, for example `Sandbox: s4-helper(24549) deny(1) file-read-data …/home-probe.txt` and `deny(1) mach-lookup com.apple.fonts`. Read them with `/usr/bin/log show --predicate 'sender == "Sandbox"'`; zsh shadows `log` with a builtin. This is evidence for threat-model check V1, and a way to grow profiles from evidence (S4b, S4-Q5).

**Tests.** `s4-sandbox` has 11 unit tests (including the Mach round trip with the audit trailer, and descriptor inheritance under `POSIX_SPAWN_CLOEXEC_DEFAULT`) and 3 integration scenarios, plus 1 opt-in test. The four standard checks pass: 110 tests, 5 ignored.

**Housekeeping.** The named-profile runs left three crash reports under `~/Library/Logs/DiagnosticReports/Retired/`. They will be removed at S4c, after the final evidence run.

### CP7 (S4b) — completed 2026-10-17

**Code** (commit `1ca3e49`):
- helper modes `ready`, `serve` and `jit`, and the precompiled method, in which the broker compiles and the helper applies bytecode (invariant X11, Chromium's `sandbox_profile_t`);
- Mach RPC with send-once rights, reply-and-receive and out-of-line memory; footprint via `proc_pid_rusage`; a `MAP_JIT` probe;
- profiles `bare.sb`, `renderer-jit.sb` and `text.sb`. `renderer.sb` now denies dynamic code explicitly;
- the `s4-text` crate; `tools/s4-measure.sh`.

The four standard checks pass: 115 tests, 5 ignored. `unsafe`: 48 blocks in `s4-sandbox` and 7 in `s4-text`, all in their `platform` modules.

**Formal run.** `tools/s4-measure.sh --named-profile` at `1ca3e49` with a clean tree, output in `target/s4-results/20261017-134336/`. Every command passed. Medians:
- **Start-up** (30 launches each):
  - no profile: 2.17 ms, 1.14 MiB;
  - `init`: 8.71 ms, 3.94 MiB;
  - `compile` + `apply`: 8.14 ms, 3.94 MiB (5.7 ms compile, 33 µs apply);
  - **precompiled: 2.11 ms, 1.14 MiB (17 µs apply, 498 bytes).**
- **IPC:** 64 B round trip 4.92 µs over a socket against **3.75 µs over Mach**; 1 MiB in 599 µs over a socket with default 8 KiB buffers, 150 µs with 1 MiB buffers, and 172–177 µs over Mach out of line. The sandbox adds nothing.
- **JIT:**
  - hardened without `allow-jit`: `EINVAL` everywhere;
  - `(deny default)` alone doesn't stop `MAP_JIT`; an explicit `(deny dynamic-code-generation)` does, for every executable;
  - a second region is refused only with `single-jit`.
- **Text:** `text.sb` renders pixel-identically to an unsandboxed run, for Helvetica and Times New Roman. The bare renderer profile silently draws other glyphs. A helper that has drawn text costs 10.84 ms and 4.05 MiB unsandboxed, or 14.27 ms and 6.33 MiB when it compiles its own profile.

**How the text profile was grown.** Fonts alone left Times New Roman different pixel by pixel. Bisecting from `(allow default)` traced this to ICU's data in `/usr/share/icu`. Denying every Mach look-up, the font server included, changed nothing.

**Housekeeping.** The formal run's named-profile crash report was copied into the results directory. All four `s4-helper` crash reports were then removed from `~/Library/Logs/DiagnosticReports/Retired/`.

### CP8 (S4c) — completed 2026-10-17

- [S4 results](s4-results.md) written.
- **ADR-0004 Accepted**, per the owner's instruction, since S4-Q1 and S4-Q2 passed. S4's refinements are recorded in its decision history and decisions:
  - D2: the browser compiles profiles;
  - D3: the evidence favours Mach (OQ-17);
  - J3: the explicit `(deny dynamic-code-generation)` is required;
  - J5: `renderer-jit` carries `single-jit`.
- Updated: process model (now binding: B2 basis, the font rules, §8 and §10), macOS note (outcome box), OQ-17 (evidence), decision map (F3 Accepted; Step 3 done), indexes, spikes README, status.

### CP9 — completed 2026-10-17

`step-3` merged into `main` with `git merge --no-ff step-3`, as the owner instructed once the S4 results were written and ADR-0004 was Accepted.
