# Step 3 — Threat Model, Trust Boundaries, and the macOS Process and Sandbox Model: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | In progress |
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
| Code, including spikes, needs a specific authorization ([status.md](../status.md), charter §7) | No code is written in Step 3. Spike S4 is designed in CP3 and listed for the owner's decision at the review gate |

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
| Q4 | Which processes exist, and what may each one do? | Process model (`docs/security/process-model.md`) §3–§4 |
| Q5 | How is each process confined on macOS, with the Command Line Tools only and no paid developer account? | macOS confinement note (`docs/research/macos-process-confinement.md`); ADR-0004 |
| Q6 | Which rules govern IPC between processes of different trust? | Process model §6 |
| Q7 | How much memory may each process use on 8 GB, and how many processes may run? | Process model §7 |
| Q8 | JIT policy (OQ-4): under what conditions may a JIT exist, and where is it off? | ADR-0004 |

## 4. Deliverables

1. `docs/security/threat-model.md`
2. `docs/research/macos-process-confinement.md`, a research note that also designs spike S4
3. `docs/security/process-model.md`
4. `docs/adr/0004-process-model-and-sandbox.md` (Proposed), including the JIT policy for OQ-4
5. Updates to the decision map, open questions, indexes and status

## 5. Checkpoints

**Every checkpoint ends with:**
- every relative link in `docs/` resolving to an existing file. Forward links to this step's own planned documents (§4) are allowed until they are written, and must all resolve by CP5;
- the four standard checks still green (no code changes, so they must stay green): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `tools/check-unsafe-policy.sh`;
- a commit on `step-3` and an update to this tracker.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-3` | ✅ 2026-10-17 (§7) |
| CP2 | Threat model: assets, adversaries, attack surfaces, trust boundaries, goals and non-goals, security levels | ✅ 2026-10-17 (§7; [threat model](../security/threat-model.md)) |
| CP3 | Research note: process confinement on macOS, browser precedents, observations on this machine; spike S4 design | Not started |
| CP4 | Process model: process types and privileges, site isolation policy, IPC trust rules, memory budgets, staging | Not started |
| CP5 | Synthesis: ADR-0004 (Proposed) with the JIT policy; decision map, open questions, indexes, status; branch ready for review | Not started |

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
