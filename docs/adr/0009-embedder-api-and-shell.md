# ADR-0009: Engine/embedder API and the shell

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17), by the owner at Step 4's Gate B. **EM9 amended by the architecture review's Round 1** (2026-10-21): the script-free reference-test path and staged diagnostics (RV1-01, RV1-15) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Embedder API](../architecture/embedder-api.md) (EM1–EM12); [S2 results](../research/s2-results.md); [event loop and threading](../architecture/event-loop-and-threading.md) (EL6, EL24); [IPC](../architecture/ipc.md); [input and accessibility](../architecture/input-and-accessibility.md); decision-map item F4; OQ-11 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12 |
| 2026-10-17 | **Accepted** by the owner at Gate B: "ADR-0009 (Embedder API & Pure-Rust Shell): Accepted." OQ-11 is resolved, subject to the shell milestone's validation (decision 6) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1): RV1-15, "Require headless embedder text dumps at M1, crash reporting at M2, and console logging at M3." And RV1-01, "Implement a script-free screenshot harness for Milestone 1". EM9 is amended in the [embedder API](../architecture/embedder-api.md): the headless embedder writes a PNG per test without script, and its diagnostics are staged (dumps at M1, crash reports at M2, the console at M3). Decision 5 records it |
| 2026-10-21 | **Amendment recorded** in Round 2's first checkpoint ([the review](../review/README.md) §5.6, RV2-02; the owner: "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed."): decision 7 and EM12 follow Round 1's RV1-17, "Validate local self-signed codesigning persistence via Spike S15.", and [ADR-0004](0004-process-model-and-sandbox.md) D2's amendment: the bundle is signed by the stable local identity S15 validated, with ad hoc as the fallback (MA10). EM10 now says single-process mode is a test mode (RV1-07) |

## Question

Where is the boundary between the engine and the browser's shell, what crosses it, how is the engine tested headless, and in which language is the shell written (OQ-11)?

## Requirements and constraints

- **EL6:** AppKit's main thread runs the shell only.
- **PM6:** every boundary is a message interface.
- **AC6:** a headless, deterministic core.
- **IN7 and IN9:** IME and accessibility are answered from mirrors on the main thread.
- **The process model:** the browser holds profile data, mediates user files and permissions, and never parses web content.
- **S2:** objc2 drove AppKit, Metal and CoreText. Menus, IME, accessibility and app bundles were untested, and no Objective-C class was defined.

## Alternatives considered

- The engine as a library of direct calls.
- The browser's UI as privileged web content.
- A Swift shell.
- SwiftUI.

Each is assessed in [embedder API](../architecture/embedder-api.md) §6.

## Historical context and modern developments

- Chromium separates `content` from `chrome` ("content doesn't depend on chrome at all") and tests with `content_shell`.
- WKWebView splits the boundary into navigation and UI delegates.
- Servo's embedder talks to its constellation by messages.

All sourced in [embedder API](../architecture/embedder-api.md) §2.

## Analysis

See [embedder API](../architecture/embedder-api.md) §4. The language question turns on one unproven point: defining Objective-C classes with objc2's `define_class!` for `NSTextInputClient` and the `NSAccessibility` protocols. A message boundary makes the engine indifferent to the answer.

## Evidence and sources

[Embedder API](../architecture/embedder-api.md) Sources; [S2 results](../research/s2-results.md) §3, §8.

## Decision

1. **The shell is an embedder:** AppKit's main thread, talking to the engine's core through a message API in the IDL. The engine never depends on AppKit or the shell, enforced by the build (EM1–EM3).
2. **Commands in; events and requests for decisions out.** The engine applies its policy first, and every request has a default (EM4–EM5).
3. **Presentation, input, IME and accessibility** cross as shell-hosted objects that the engine keeps current. Profile data lives with the engine (EM6–EM7).
4. **The browser's own UI is native, never web content** (EM8).
5. **A headless embedder** with offscreen surfaces, synthetic input and virtual time, for WPT and reference tests. Single-process mode uses the same API (EM9–EM10). **As amended in Round 1** (RV1-01, RV1-15): it writes a reference test's PNG without script, and dumps the DOM, styles, boxes and fragments, the display list and the network log as text from M1, with crash reports at M2 and the console at M3.
6. **OQ-11: a pure-Rust shell with objc2.** The shell milestone first validates an objc2-defined view class for text input and accessibility, a menu bar and a bundle. A Swift shell can sit behind the same API if that validation fails (EM11).
7. **One ad-hoc-signed bundle** holding the shell and helper executables, built by a script without Xcode (EM12). **As amended** (Round 1, RV1-17; recorded in Round 2, RV2-02): signed by the stable local identity, with ad hoc as the fallback.

## Rationale

- A message boundary gives a headless test harness for free, keeps AppKit out of the engine, and makes the shell replaceable.
- Pure Rust keeps one toolchain and lets the shell use the engine's mirrors directly. The owner is fluent in Rust.

## Assumptions

- objc2's `define_class!` can implement `NSTextInputClient` and the `NSAccessibility` protocols, to be validated first at the shell milestone.
- WPT's harness can run on a headless embedder (Step 6).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| objc2 can't comfortably define the classes IME and accessibility need | Low to medium | Medium | A Swift shell behind the same message API (EM11); the engine is unaffected |
| Decision requests stall the engine | Low | Medium | Every request has a default and no unbounded wait (EM5) |

## Conditions for revisiting

- The shell milestone's validation fails (switch to the Swift fallback).
- A future platform requirement can't be met through AppKit from Rust.

## Consequences

- The crates `shell` (AppKit, with objc2 in its `platform` module), `embedder-api` and `headless` ([overview](../architecture/overview.md) §2).
- OQ-11 is resolved by this ADR's acceptance (2026-10-17), subject to the shell milestone's validation.
