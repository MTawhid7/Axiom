# Step 3 — Threat Model, Trust Boundaries, and the macOS Process and Sandbox Model: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **In progress:** CP1–CP5 complete and reviewed by the owner (2026-10-17); spike S4 (CP6–CP8) authorized and running |
| **Authorized** | 2026-10-17, by the owner ("Continue implementation"), as Step 3 design work. Documents only (§2) |
| **Branch** | `step-3`, to be merged into `main` after the owner's review |
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
| CP6 | **S4a:** sandbox entry and denial probes (S4-Q1, S4-Q2), automated under `cargo test`; signing script | Not started |
| CP7 | **S4b:** measurements: helper start-up and footprint, IPC latency and throughput over Mach messages and socket pairs (S4-Q3); the JIT matrix (S4-Q4); CoreText inside a renderer profile (S4-Q5) | Not started |
| CP8 | **S4c:** `docs/research/s4-results.md`. ADR-0004 moves to Accepted if S4-Q1 and S4-Q2 pass. Docs synthesis | Not started |
| CP9 | Merge `step-3` into `main` (`--no-ff`) | Not started |

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
