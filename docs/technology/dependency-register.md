# Dependency Register

| | |
|---|---|
| **Status** | Living document. Started at Step 5, CP2 |
| **Purpose** | Every dependency proposed for the engine, with the record the [dependency policy](dependency-policy.md) requires (DP6). Nothing listed here is added to the workspace until its approval (DP5) and, for code, an authorized milestone |
| **Last updated** | 2026-10-18 |

**How to read it.** Each entry gives the fields of DP6. The values used are:
- **class:** required platform facility, replaceable implementation detail, or foundational dependency (DP5);
- **group:** `trusted`, `sandboxed`, `build` or `test` (DP9);
- **grade:** `ub-risk-0` to `ub-risk-2` (DP8), or "not graded" until an audit is recorded.

Crate facts are dated and come from crates.io's API and GitHub, read on the date shown. Every **status** is one of: *decided* (by an accepted ADR), *proposed* (by a Step 5 document, awaiting its ADR), or *spike only* (exempt, DP23).

## 1. Required platform facilities

Placement and purpose are in the policy's table ([DP21](dependency-policy.md#8-apples-frameworks)). Those decided by accepted ADRs:

| Facility | Processes | Status |
|---|---|---|
| libSystem: Mach messages, kqueue, `posix_spawn`, `os_sync_wait_on_address` | All | Decided: ADRs [0005](../adr/0005-ipc-transport-and-interfaces.md), [0006](../adr/0006-event-loop-and-threading.md), [0008](../adr/0008-script-agents-and-heap.md) |
| libsandbox's private interfaces | Browser (compile), helpers (apply) | Decided: [ADR-0004](../adr/0004-process-model-and-sandbox.md) |
| AppKit, Foundation, QuartzCore, Metal, IOSurface | Browser | Decided: ADRs [0007](../adr/0007-frame-tree-pipeline-and-input.md), [0009](../adr/0009-embedder-api-and-shell.md) |

The Step 5 checkpoints add proposed facilities (CoreText, ImageIO, Security, Network, media frameworks) as they decide them.

## 2. Crates

### objc2 and its framework crates

| Field | Value |
|---|---|
| **Crates** | `objc2` 0.6.5; the framework crates `objc2-foundation`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-metal`, `objc2-core-text`, `objc2-core-graphics`, `objc2-core-foundation` and `objc2-io-surface`, 0.3.2; transitively `block2` 0.6.2, `dispatch2` 0.3.1, `objc2-encode` 4.1.0 and `bitflags` 2.13.2 (the versions in `Cargo.lock`) |
| **Source and license** | crates.io; `objc2`, `block2` and `objc2-encode` MIT; the framework crates and `dispatch2` "Zlib OR Apache-2.0 OR MIT"; `bitflags` "MIT OR Apache-2.0" (crates.io, 2026-10-18) |
| **Purpose** | The Objective-C runtime, and generated bindings to Apple's frameworks |
| **Used by** | `platform`, and the shell's `platform` module ([overview](../architecture/overview.md) §2) |
| **Processes** | Browser (AppKit, Metal); renderers, only for frameworks a Step 5 checkpoint places there (CoreText, if CP3 chooses it) |
| **Untrusted input** | None of its own: it marshals calls. The frameworks behind it are classed separately (§1) |
| **Class** | **Foundational dependency:** its types appear throughout `platform` and the shell |
| **Group, grade** | `trusted` is required, because the browser uses it. Not graded yet: audits begin when `cargo-vet` is installed (DP22) |
| **`unsafe` and non-Rust code** | Bindings to Objective-C are FFI, so the crates contain `unsafe`. Their generated methods make most calls safe: S2's window needed 19 `unsafe` sites, all in its `platform` module ([S2 results](../research/s2-results.md)). Their class-definition macros expand `unsafe`, so they are used only in allowlisted modules (DP11) |
| **Alternatives** | Hand-written FFI, which S4 used for the C interfaces of libsandbox and Mach; a Swift shell, the fallback EM11 keeps behind the same message API |
| **Maintenance** | One repository, `madsmtm/objc2`, last pushed 2026-10-17. Its contributors are led by one principal maintainer (3,237 commits; the next is the original `objc` crate's author, with 1,210) (GitHub, 2026-10-18). **Risk:** a small maintainer base. Precedent: wgpu moved its Metal backend to `objc2-metal` in January 2026 ([language evaluation](language-evaluation.md) §6.1, §9) |
| **Build cost** | Measured by S2 (S2-Q6): `s2-window` built clean in 6.0 s with only the headers it used, and in 29 s with the defaults, which also needed 1.4 GiB for `objc2_app_kit` alone. Both are within T1–T4's targets |
| **Features** | `default-features = false`, enabling per framework only the headers used, as the workspace manifest already does (DP18) |
| **Interface** | The `platform` modules, which expose Axiom's own types (ADR-0002) |
| **Revisit if** | The crates become unmaintained, or the shell milestone's first validation of objc2-defined classes for text input and accessibility fails (EM11) |
| **Status** | **Decided:** [ADR-0009](../adr/0009-embedder-api-and-shell.md) (EM11, the pure-Rust shell with objc2), on the evidence of [S2](../research/s2-results.md) and the language evaluation (§9, §13) |

## 3. Proposed by Step 5

None yet. CP3–CP8 add entries here as their documents propose dependencies, and CP9's ADRs decide them.

## 4. Spikes only (exempt, DP23)

The rest of `Cargo.lock`, justified in the workspace manifest:
- `syn`, `quote`, `proc-macro2` and `unicode-ident`, for S1's `#[derive(Trace)]` in Pattern A. Pattern B, the pattern ADR-0003 chose, needs no procedural macro.
- `libc` 0.2.191. It appears in the lock without being built on this platform: `cargo tree -i libc` finds no path to it (observed 2026-10-18).
