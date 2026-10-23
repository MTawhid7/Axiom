# Interface notes

| | |
|---|---|
| **Status** | Living index. Started at M1-CP3 (2026-10-23) |
| **Purpose** | The architectural interfaces of the [crate architecture](../crates.md)'s CR8: contracts between processes, or between subsystems that several crates or processes depend on. Each is designed here before its code, and the owner approves it (the owner's choice, 2026-10-22: "Interfaces only") |
| **Template** | The [engineering standards](../../technology/engineering-standards.md)' ES27: purpose and scope; the binding rules it implements; the types and signatures, as Rust; the invariants, threads and ownership; errors and violations; efficiency; tests and oracles; the alternatives considered |

A note is **Proposed** until the owner approves it, then **Approved**, with the owner's words in its header. A later change to an approved note goes back to the owner (ES27). A crate's other public items are designed in its charter (ES15), not here.

Each note follows the conventions of the design documents: claims labelled (sourced, observed, inference, hypothesis, background), dates absolute, binding rules cited by ID. Rust in a note is the interface's shape, close to what the crate will declare; names may change in review, signatures only with the owner.

## Milestone 1

| Note | Interface | Crates | Checkpoint | State |
|---|---|---|---|---|
| [The IDL and the generated API](protocol-idl.md) | How every IPC interface is written; what `codegen` generates into `protocol`; the wire format's details; the build identifier | `codegen`, `protocol` | M1-CP3 | Proposed (2026-10-23) |
| [`ipc`'s channels and `Untrusted<T>`](ipc-channels.md) | Channels over Mach and in memory; sending without blocking; received messages and their handles' owners; peers and their pinning; `Untrusted<T>`, validators and violations; shared regions and their rings | `ipc` | M1-CP3 | Proposed (2026-10-23) |
| [`event-loop`'s tasks and task ages](event-loop-tasks.md) | Thread loops, the wait source and the clock; tasks and the HTML event loop's scheduler; task ages; the watchdog's protocol, with the active session; the interrupt flag | `event-loop` | M1-CP3 | Proposed (2026-10-23) |
| [`trace`'s API and the diagnostics region](trace-and-diagnostics-region.md) | How names and metrics are declared; how call sites record; where buffers come from; the region's format and protocol; the browser's file-backed recorder | `trace`, `diagnostics` | M1-CP3 | Proposed (2026-10-23) |

Milestone 1's other notes come with their checkpoints (the [plan](../../milestone-1-plan.md) §3): M1-CP4's (navigation, commit and the loader's stream; the display-list format; the XML sink; what `style` and `layout` read; computed styles; the lifecycle's tokens; `text`'s results; fragments; the embedder API's first version), M1-CP5's HTML tree sink, and M1-CP11's loader protocol over HTTP.
