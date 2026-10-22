# ADR-0005: IPC transport and interface definitions

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17), by the owner at Step 4's Gate B. **IP11 amended by the architecture review's Round 1** (2026-10-21): panic-free validators with checked arithmetic (RV1-16); IP2 and the compositing family for real processes from M1 and the GPU/compositor process (RV1-07, RV1-02). **Amended by Round 4** (2026-10-22): IP13's bound (RV4-03) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [IPC](../architecture/ipc.md) (the full design, IP1–IP22); [process model](../security/process-model.md) §6 (R1–R6, IR1–IR10); [ADR-0004](0004-process-model-and-sandbox.md) D3; [S4 results](../research/s4-results.md); [macOS runtime note](../research/macos-runtime-facilities.md); [Step 4 tracker](../research/step-4-architecture-proposal.md); OQ-17 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12 |
| 2026-10-17 | **Accepted** by the owner at Gate B: "ADR-0005 (Mach IPC): Accepted." OQ-17 is resolved |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-16: "Enforce panic = "abort" per binary; enforce checked arithmetic / overflow checks in parsers and IPC validators." IP11 is amended in the [IPC design](../architecture/ipc.md): validators are panic-free by lint and use checked arithmetic |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-07: "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri." And RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." IP2 now confines single-process mode to tests, fuzzing and Miri, so Mach channels and memory entries run from M1; the compositing family gains the GPU process's channels: display lists from renderers, layer commits to the browser |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-13: IP16's never-block rules for every process that serves several peers, and children's sends with a zero timeout and a bounded local backlog, under EL1. RV2-14: IP13's synchronous requests have no timeout of their own (PF21), with one exception for a service worker's answer; IP7 bars `iosurface` from renderers' interfaces; three interface families added; open question 3 answered. In the [IPC design](../architecture/ipc.md) |
| 2026-10-21 | **Open question 4 answered** with Round 2's RV2-05, accepted in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." S10 found memory charged to the process that allocates it, so the GPU process accounts by page ([IPC](../architecture/ipc.md) §8) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-11: IP15 states its one exception, the origin private file system's synchronous access handle; the user's files reach renderers only as data ([IPC](../architecture/ipc.md)) |
| 2026-10-22 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 4 ([the review](../review/README.md) §5.9): "All 17 findings (RV4-01 through RV4-17) are Accepted as proposed." RV4-03: IP13's synchronous requests are bounded by PF19's task-age deadline over the answering thread ([IPC](../architecture/ipc.md) IP13) |

## Question

Which transport carries messages between Axiom's processes (OQ-17)? How are interfaces defined, generated and checked so that the IPC trust rules IR1–IR10 hold by construction?

## Requirements and constraints

- **R1–R6:** asynchronous channels; handle passing, IOSurfaces included; peer identity from launch; bulk data in shared memory; bounds and backpressure; the same API in-process.
- **IR1–IR10:** untrusted input decoded in safe Rust; decisions from the receiver's records; kill on violation; capabilities; the browser never blocks; bounded synchronous calls; one writer; one build; only the browser launches; recovery on death.
- **ADR-0002, rules 5 and 7:** no `unsafe` in IPC decoding; macro-generated `unsafe` counts.
- **macOS facts:** IOSurfaces cross processes only as Mach ports; ports take guards and queue limits; messages carry an audit trailer, and can carry the sender's QoS.

## Alternatives considered

- **Transport:** Mach messages; Unix-domain socket pairs; XPC.
- **Interfaces:** an IDL with a safe-Rust generator; serde-derived typed channels (`ipc-channel`); reusing Mojo or IPDL.

## Historical context and modern developments

- Chromium's Mojo runs only on Mach messages on macOS. It pins the first message's audit token and sends without blocking.
- Servo and Ladybird use Mach on macOS too, keeping socket transports for other systems.
- Chromium and WebKit both validate renderer data against browser state and terminate liars. WebKit's 2026 `IPC::Untrusted<T>` makes the rule a type.
- Firefox restricts synchronous messages to child-to-parent; Mojo warns against them.

All sourced in [IPC](../architecture/ipc.md) §3.

## Analysis

| | Mach | Sockets | XPC |
|---|---|---|---|
| IOSurfaces and memory entries | Native port rights | Impossible: would need Mach as well | Native |
| Peer identity | Audit token on every message | Once per connection (`LOCAL_PEERTOKEN`) | Peer requirements |
| Small round trip (S4) | 3.75 µs | 4.92 µs | Not measured |
| Priority | QoS override and importance | None | Built in |
| Threads | One `kevent64` wait (EL2) | One `kevent64` wait | Dispatch queues, outside Axiom's thread loops |
| Decoding | Axiom's safe decoders | Axiom's safe decoders | Apple's C deserializer, inside the privileged browser |

| | IDL + generator | serde derives | Mojo or IPDL |
|---|---|---|---|
| Bounds and capabilities in the interface | Yes | No; `ipc-channel`'s queues are unbounded | Yes |
| `unsafe` | None generated | Macro-generated code (rule 7) | C++ |
| A Swift side if needed | The generator can emit it | No | No |

## Evidence and sources

[IPC](../architecture/ipc.md) §2–§3 and its Sources; [S4 results](../research/s4-results.md) §4; SDK 27 headers (`mach/port.h`, `mach/message.h`).

## Decision

1. **Mach messages carry everything between processes** (IP1). The one exception is the inherited descriptor that delivers a helper's sandbox profile at launch. The same channels run in-process, always encoding and decoding (IP2). This resolves OQ-17.
2. **Endpoints are capabilities** that only the browser creates and hands out. Each child's audit token is pinned at launch and checked on every message. Ports are guarded and immovable, with queue limits (IP3–IP6, IP19).
3. **Interfaces are defined in an IDL.** A safe-Rust generator emits types, canonical encoders, decoders that check everything, and a fuzzing entry point per decoder. Handles travel outside the bytes (IP7–IP9, IP22).
4. **Values from less privileged senders arrive as `Untrusted<T>`,** validated against the receiver's records. Violations are `Terminate` or `Ignore` (IP10–IP12).
5. **Synchronous requests go only toward more privilege,** only where the platform requires them, with timeouts and no re-entry (IP13).
6. **Bulk data and backpressure:** bulk data goes out of line or through single-writer shared memory. Every queue is bounded, and the browser never blocks (IP14–IP17).
7. **One build only;** urgency travels with messages where Mach allows (IP18, IP20).

## Rationale

- Mach is the only transport that carries every handle Axiom needs, identifies each message's sender, and is fastest for small messages on this Mac.
- An IDL keeps bounds and capabilities reviewable as data, with decoders free of `unsafe`.
- `Untrusted<T>` turns IR2 from a convention into a compile-time obligation.

## Assumptions

- Mach messages keep working for unsandboxed-to-sandboxed processes signed ad hoc (shown by S4), and under the stable local identity Round 1 adopted (S15, RV1-17).
- A safe-Rust generator can produce decoders fast enough for display-list traffic (to measure when built).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A priority override doesn't boost a receiver waiting in `kevent64` (IP20) | Medium | Low | Event-loop QoS (EL23) still applies. Measured at the first implementation milestone |
| Decoders become the browser's main attack surface | High (by design) | High | Generated safe decoders; fuzzing every decoder (V2); `Terminate` on violation |
| Apple restricts Mach features further for third-party code | Low | Medium | Only public interfaces are used. Port arrays, which need an entitlement, are avoided |

## Conditions for revisiting

- A public macOS change makes Mach messages unavailable between Axiom's helpers.
- Decoding measurably bottlenecks display-list traffic despite optimization.

## Consequences

- `platform` gains Mach channel FFI; `ipc` and `idl` are safe crates.
- Every interface in CP5–CP10 is written in the IDL, with its owning thread and the process types allowed to hold each end.
- V2 (fuzzing) and V3/V9 (lying-peer tests) have a generated foundation.
