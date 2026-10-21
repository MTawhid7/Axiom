# IPC: Transport and Interface Definitions

| | |
|---|---|
| **Status** | **Binding** since ADR-0005's acceptance (2026-10-17, Gate B). Written at Step 4, CP4. Answers OQ-17. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): IP11 for RV1-16 (CP1); IP2 for RV1-07 (CP2) |
| **Question** | How do Axiom's processes and threads exchange messages? Which transport carries them, how are interfaces defined and checked, how do handles and bulk data move, and how are the IPC trust rules IR1–IR10 enforced by construction rather than by care? |
| **Answers** | Step 4 question Q5 ([tracker](../research/step-4-architecture-proposal.md) §3); OQ-17 |
| **Feeds** | ADR-0005; CP5 (frame tree and navigation), CP6 (display lists), CP8 (input and accessibility), CP9 (embedder API), CP10 (agents and messaging) |
| **Builds on** | [Process model](../security/process-model.md) §6 (R1–R6, IR1–IR10); [ADR-0004](../adr/0004-process-model-and-sandbox.md) D3; [S4 results](../research/s4-results.md); [macOS runtime note](../research/macos-runtime-facilities.md); [event loop and threading](event-loop-and-threading.md) (EL4) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered IP1–IP22. Precedent and platform facts are sourced (§2–§3); the design is inference unless marked otherwise.

## Summary

- **Mach messages are the transport between processes** (IP1), which answers OQ-17.
  - Every handle Axiom must pass, IOSurfaces included, is a Mach port.
  - The kernel stamps each message with the sender's identity.
  - Small round trips are 24% faster than over a socket pair (S4).
  - Messages can carry the sender's priority.
  - Chromium, Servo and Ladybird use Mach on macOS.
- **The same channels work inside one process** (IP2), for tests, deterministic scheduling and single-process mode. Messages are always encoded and decoded, so the decoders run everywhere.
- **Endpoints are capabilities** (IP4). Only the browser creates channels between processes, and it hands an end only to a process whose type and site lock the interface allows. After launch, every endpoint arrives as a port right inside a message.
- **Interfaces are written in an interface definition language** (IP7–IP9). A safe-Rust generator emits types, encoders, decoders that check every length, count, enum and handle, and a fuzzing entry point per decoder. The wire format has exactly one encoding per value.
- **Trust is enforced by types** (IP10–IP12). A value from a less privileged sender arrives as `Untrusted<T>`, which can't be used until a validator compares it with the receiver's own records, as in WebKit. A violation either terminates the sender or, for an expected race, is ignored.
- **Synchronous requests flow only toward more privilege** (IP13). They exist only where the web platform demands them, are bounded by timeouts, and never re-enter. The browser never blocks on a child.
- **Bulk data** (IP14) moves out of line, as a copy-on-write copy, or through shared memory with a single writer.
- **Every queue is bounded** (IP16). The browser sends without blocking and treats a full queue as a hang.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **R1–R6** ([process model](../security/process-model.md) §6.1): asynchronous two-way channels; handle passing; peer identity from launch; bulk data in shared memory; bounds and backpressure; the same API in-process | IP1–IP6, IP14, IP16, IP2 |
| **IR1–IR10** (§6.2): untrusted input, decisions from the receiver's records, kill on violation, capabilities, no blocking on children, bounded synchronous calls, one writer, one build, only the browser launches, recovery on death | IP4, IP6, IP8–IP13, IP16, IP18 |
| **ADR-0002, rules 5 and 7:** no `unsafe` in IPC decoding; macro-generated `unsafe` counts as part of the boundary | IP8, IP9 |
| **PM4, PM6:** capabilities, not ambient authority; every future process boundary a message interface from the start | IP3, IP4, IP2 |
| **EL4:** endpoints bound to threads, with no intermediate I/O thread | IP3, IP21 |
| **AC1, AC6, AC9** | §5, §7 |

## 2. Evidence about the transport

**Measured on this Mac by S4** ([S4 results](../research/s4-results.md) §4):
- A 64-byte round trip takes 3.75 µs over Mach and 4.92 µs over a socket pair.
- 1 MiB takes 172–177 µs over Mach out of line. Over a socket it takes 599 µs with the default 8 KiB buffers, and 150 µs with 1 MiB buffers.
- The sandbox adds nothing.
- The kernel's audit trailer carries the sender's PID, which matched the spawned child.

**Read from the SDK 27 headers** (observed):

| Facility | What it offers | Header |
|---|---|---|
| Port rights in messages | Endpoints, memory entries and IOSurfaces travel as rights the kernel checks. Files travel as fileports (`fileport_makeport`, `fileport_makefd`) | `mach/message.h`, `sys/fileport.h` |
| Port construction | `mach_port_construct` with options: a guard (`MPO_CONTEXT_AS_GUARD`, `MPO_STRICT`), a queue limit (`MPO_QLIMIT`), an immovable receive right (`MPO_IMMOVABLE_RECEIVE`), an importance receiver (`MPO_IMPORTANCE_RECEIVER`) | `mach/port.h` |
| Queue limits | Default 5 messages (`MACH_PORT_QLIMIT_BASIC`); at most 1,024 for user ports (`MACH_PORT_QLIMIT_LARGE`) | `mach/port.h` |
| Message size | "There is no fixed upper bound", but messages above 256 KiB (`MACH_MSG_SIZE_RELIABLE`) may fail unpredictably | `mach/message.h` |
| Out-of-line memory | Virtual (copy-on-write) or physical copies | `mach/message.h` |
| Shared memory | `mach_make_memory_entry_64`, a memory entry sent as a port | `mach/vm_map.h`, `mach/memory_object_types.h` |
| Port arrays | Out-of-line arrays of ports may be sent only to ports created with an entitlement ("com.apple.developer.allow-connection-port-with-port-array") | `mach/port.h` |
| Lifetime | No-senders, dead-name and port-destroyed notifications | `mach/notify.h` |
| Priority | `MACH_SEND_OVERRIDE`, `MACH_SEND_PROPAGATE_QOS`, importance (`MACH_MSGH_BITS_RAISEIMP`) ([macOS runtime note](../research/macos-runtime-facilities.md) §5.4) | `mach/message.h` |

**Unix-domain sockets**, by contrast, pass descriptors (`SCM_RIGHTS`) and can report a peer's audit token once per connection (`LOCAL_PEERTOKEN`), but carry no Mach ports (observed, `sys/socket.h`, `sys/un.h`). IOSurfaces cross processes only as Mach ports ([macOS runtime note](../research/macos-runtime-facilities.md) §6.1). So a socket design would need Mach anyway.

## 3. Precedent

From source and documentation read on 2026-10-17 (Sources).

| | Chromium (Mojo) | Firefox (IPDL) | WebKit | Servo (`ipc-channel`) | Ladybird (LibIPC) |
|---|---|---|---|---|---|
| **Transport on macOS** | Mach messages only; files as fileports. Messages of a page or more go out of line [C2] | Not checked | Not verified. Its IPC code has a `darwin/` directory, whose contents weren't read [W2] | "Mach ports on macOS" [V1] | A Mach-port transport for macOS and iOS beside its socket transport, with a Mach bootstrap listener [L1] |
| **Interfaces** | `.mojom` files; generated bindings | A protocol language of actors, each "explicitly bound to its endpoint" [F1] | `.messages.in` files; generated receivers [W2] | Rust types serialized with serde; client and server must use "identical message types" [V1] | Not checked |
| **Untrusted input** | "treat the data as if it were generated by a malicious adversary"; on failure, "ignore the IPC and terminate the renderer"; "never use `CHECK()`" on message content [C1] | A handler returns `IPC_FAIL`; a failed read "will usually result in crashing the process"; `[Tainted]` parameters are validated by hand [F1] | `IPC::Untrusted<T>`: a value "cannot be used until a designated validation procedure has confirmed the sender was entitled to name it". A failure is either an expected race, to ignore, or grounds to terminate [W1] | Not stated | Not checked |
| **Peer identity** | Pins the audit token of the first message, then rejects "message from unauthorized peer" [C2] | Not checked | Not checked | Not stated | Not checked |
| **Backpressure** | A queue limit of 1,024; non-blocking sends that retry when the queue is full [C2] | Not checked | Not checked | `send()` never blocks; "channels are always unbounded" [V1] | At most 16,000 descriptors or ports per message, "below the 16383 port rights that XNU accepts in one Mach message" [L2] |
| **Synchronous calls** | "avoid them whenever they are not absolutely necessary"; `[Sync]` "does **not** guard against re-entrancy, especially when the client is untrusted" [C3] | Only "from the child actor to the parent", to "avoid potential deadlocks"; "an easy source of browser stalls" [F1] | Not checked | — | Not checked |
| **Ordering** | Associated interfaces run "multiple interfaces over a single message pipe while preserving message ordering" [C3] | Messages arrive in send order within one actor tree only [F1] | Not checked | FIFO per channel [V1] | Not checked |

**Observations** (inference):
- **Chromium, Servo and Ladybird use Mach on macOS,** and Servo and Ladybird keep socket transports for other systems. WebKit's transport wasn't verified here.
- **Chromium and WebKit converge on "decode, then validate against the receiver's state, then terminate on lies".** WebKit's 2026 `Untrusted<T>` moves that rule into the type system.
- **Ordering is guaranteed per channel only.** Engines that need ordering between interfaces put them on one channel (Mojo's associated interfaces, IPDL's actor trees).
- **Synchronous IPC is everyone's regret.** Firefox restricts it to child-to-parent; Mojo warns against it.

## 4. Design

### 4.1 Transport

| # | Rule |
|---|---|
| IP1 | **Mach messages carry everything between processes** (resolves OQ-17). The reasons, from §2–§3: every handle Axiom passes is a port right, IOSurfaces included (R2); the audit trailer identifies the sender of each message (R3); small round trips are 24% faster than over sockets; priority can travel with a message (IP20); a helper waits on its ports and its timers in one `kevent64` call (EL2); and ports take guards and queue limits (IP5, IP16). The only other channel is the one descriptor through which a helper receives its sandbox profile at launch, before its first Mach message (S4, ADR-0004 D2) |
| IP2 | **The same channels work in one process.** Between threads of one process, and in single-process mode, a channel is an in-memory queue with the same API (R6). **As amended in Round 1** (RV1-07): single-process mode exists strictly for automated tests, fuzzing and Miri; from M1 the browser, renderers and the network process talk over Mach channels and memory entries (IP14), so IR7's copy-then-validate runs from the first milestone. Tests use it with the deterministic scheduler (EL24). Messages are encoded and decoded in every configuration, so the decoders that guard the browser run in every test |

### 4.2 Channels and endpoints

| # | Rule |
|---|---|
| IP3 | **A channel joins two endpoints.** Each endpoint is a receive right (or an in-process queue), and the peer holds a send right to it. An endpoint belongs to one thread loop (EL4). It may carry several interfaces whose messages must stay mutually ordered. **Order is guaranteed per channel and nowhere else.** Where two flows must be ordered, they share a channel or carry explicit sequence numbers (CP5, CP6) |
| IP4 | **Endpoints are capabilities** (IR4, PM4). Only the browser creates channels between processes, and it hands an end only to a process whose type and site lock the interface allows (IP7). No process can reach another by name. **Launch:** the helper applies the profile it received through its one inherited descriptor, then looks up the single rendezvous name its profile allows (ADR-0004 D2). It says hello with a fresh reply port. The browser checks that the audit token's PID is the child it spawned (R3), and pins that token to the channel. From then on, endpoints arrive only as port rights inside messages |
| IP5 | **Port hygiene.** Receive rights are made with `mach_port_construct`. They are guarded, so a stray deallocation or move faults instead of silently breaking the channel. They are immovable unless the interface transfers them, and they always carry a queue limit. Endpoints that receive input are importance receivers. **Port rights travel only as individual port descriptors,** never as out-of-line port arrays, which need an entitlement. The IDL caps the number per message, well below XNU's 16,383 (§2–§3) |
| IP6 | **Lifetimes are observed, not polled.** A no-senders notification tells a receiver that every sender is gone; a dead-name notification tells a sender that the receiver died. The browser also watches each child process for exit with `EVFILT_PROC` (IR10). A closed channel is delivered as an event after its last message |

### 4.3 Interface definitions and generated code

| # | Rule |
|---|---|
| IP7 | **Interfaces are written in an interface definition language** (the Axiom IDL; its syntax is chosen at implementation). Each interface declares: its messages, each with a stable ID within the build, its parameters and, for requests, its reply; parameter types; the maximum size of each message; the process types allowed to hold each end; the thread role that owns each end (EL4); and whether synchronous requests are allowed, and in which direction (IP13). **Parameter types:** bounded integers; enums; strings and byte arrays with length limits; arrays with count limits; structures; typed handles: `endpoint<Interface>`, `shared_memory(read_only or read_write, max_bytes)`, `iosurface`, `file`. **As amended in Round 2** (RV2-14): interfaces that face renderers can't declare `iosurface` handles, as IP15 bars `file`, and the generator rejects one that does: renderers never hold IOSurfaces (RV1-09) |
| IP8 | **A generator emits the code.** The generator is ordinary safe Rust, run at build time. For each interface it emits: Rust types; encoders; **decoders in safe Rust that check every length, count, enum value and handle slot** (IR1; ADR-0002 rule 5); dispatch traits that reject unknown message IDs; and a fuzzing entry point per decoder (threat-model check V2). Generated code contains no `unsafe`, and the unsafe-policy checker covers it like any other crate (rule 7) |
| IP9 | **One encoding per value.** The wire format is little-endian fixed-width integers and length-prefixed byte strings, with no pointers, no implicit padding and nothing optional at the end. A decoder rejects non-canonical encodings and trailing bytes. **Handles are not in the bytes.** The `platform` layer receives the Mach message and hands the decoder plain bytes plus a typed list of handles ([process model](../security/process-model.md) §6). The decoder matches handles to their slots by index and type, so decoding needs no `unsafe` |

**Why an IDL and a generator rather than serde-style derives** (inference):
- Bounds and capability annotations live in the interface, where they can be reviewed as data.
- Derive macros are macro-generated code, which rule 7 puts inside the `unsafe` boundary. Self-describing serialization formats accept more than an interface needs.
- A generator can also emit a Swift side, if OQ-11 chooses a Swift shell (CP9).
- Servo's `ipc-channel` shows the cost of the other route: unbounded queues and sends that never block (§3).

### 4.4 Trust enforced by types

| # | Rule |
|---|---|
| IP10 | **Untrusted values can't be used before they are validated.** Every value decoded from a less privileged sender arrives as `Untrusted<T>`. Its only accessor is `validate(validator)`, which returns `T` or a `Violation`, plus one explicitly named escape for values validated elsewhere, which must state where. **Validators compare claims with the receiver's own records** (IR2): the sender's process type, its site lock, its granted capabilities, and the frame, document and resource IDs the browser issued to it. This is WebKit's `IPC::Untrusted<T>`, which Rust can enforce without exceptions: the type has no `Deref` and no public field |
| IP11 | **Violations are typed.** `Violation::Terminate` means the sender could not have produced the value legitimately: the browser kills it and logs why (IR3). `Violation::Ignore` means an expected race, such as a message about a frame that was destroyed a moment earlier: the message is dropped. Failure to decode, an oversized message, an unknown message ID, a handle of the wrong type, and a message invalid in the receiver's current state are always `Terminate`. **Validation never panics in the browser** (Chromium: "never use `CHECK()`" on message content). **As amended in Round 1** (RV1-16): validators are panic-free by construction, with Clippy's `indexing_slicing`, `unwrap_used` and `arithmetic_side_effects` denied in them, and use checked arithmetic; every fuzz target reports a panic as a finding (V2). A panic anywhere aborts its process ([ADR-0002](../adr/0002-implementation-language.md)'s failure semantics; IR10) |
| IP12 | **Every value that decodes means something.** Origins are origins, never strings or URLs; IDs are typed per kind; choices are enums; there are no parallel arrays and no methods that aren't implemented yet (Chromium's guidance [C1]). IDs that the browser issues are scoped to the process it issued them to, so a renderer can name only what it was given |

### 4.5 Synchronous requests

| # | Rule |
|---|---|
| IP13 | **Only toward more privilege, and only where the platform requires it.** The browser never sends a synchronous request to a child (IR5). A child may send one only to a more privileged process, only on an interface whose IDL marks it, and only where the web platform demands synchronous semantics (IR6): `document.cookie`, `localStorage`, synchronous `XMLHttpRequest`. These go to the network process's storage or I/O thread (EL8), never to the browser's core. **While waiting, the calling thread handles nothing else**, so nothing can re-enter, unlike Mojo's sync calls [C3]. Every synchronous request has a timeout. If it expires, the operation fails as the platform specifies for an unavailable resource. **As amended in Round 2** (RV2-14): a synchronous request has **no timeout of its own;** its answer is bounded by PF19's watchdog over the caller's thread ([performance](../performance.md) PF21). **One exception to "toward more privilege":** in a document a service worker controls, a synchronous `XMLHttpRequest` goes to that worker through the channel the browser made between the two agents (SW8), a peer of the same origin, usually in the same renderer; PF19's watchdog over the worker's renderer bounds the wait, and the IDL marks that one interface's request. The reply comes back on a send-once reply right |

### 4.6 Bulk data and shared memory

| # | Rule |
|---|---|
| IP14 | **Data moves by size and lifetime.** Small payloads travel inline, kept under each interface's limit and well below the 256 KiB beyond which sends may fail. One-shot large payloads go **out of line as virtual copies**: copy-on-write, so the sender can't change what the receiver reads after sending (background, Mach VM semantics). S4 measured 1 MiB in 172–177 µs. **Long-lived or repeatedly updated data** goes in shared memory from a memory entry, mapped read-only in the receiver where the OS allows (R4). It has one writer (IR7): the reader copies what it interprets before validating it, and the region holds offsets, never pointers. Streams, such as display-list updates (CP6), use single-producer, single-consumer rings whose indices the reader treats as untrusted |
| IP15 | **Descriptors cross only where an interface allows them,** as fileports. Interfaces that face renderers can't declare `file` handles, so a renderer never receives a descriptor to a user's file ([process model](../security/process-model.md) §4.1) |

### 4.7 Bounds, backpressure and fairness

| # | Rule |
|---|---|
| IP16 | **Every queue is bounded.** Every receive port has a queue limit (R5). **The browser sends to children without blocking** (a zero send timeout, as in Chromium [C2]). If a child's queue is full, the browser keeps a bounded backlog for that channel. Past it, it treats the child as hung (EL hang policy, [event loop](event-loop-and-threading.md) §4.7), and never blocks its own threads (IR5). **A child that floods the browser stalls only itself:** its sends wait or time out against the browser's queue limit. The browser drains its ports fairly, round-robin across children with bounded work per wakeup, so one renderer can't starve another.<br>**As amended in Round 2** (RV2-13):<br>• **Every process that serves more than one less privileged peer** follows the browser's rule (IR5): the network process from M1, the GPU/compositor process from M2, the media utility process from M7. It sends to each peer with a zero timeout, keeps a bounded backlog per peer (PF21), and past it reports the peer to the browser as hung and closes the peer's endpoints; it drains its ports fairly, round-robin across peers, with bounded work per wakeup. No peer can stall the others (SG11).<br>• **No thread waits on a send** (EL1): a child's sends use a zero timeout too. A message that doesn't fit goes into a bounded local backlog, drained when Mach's send-possible notification (`MACH_NOTIFY_SEND_POSSIBLE`) says the queue has room; past the backlog, the child stops producing until it drains. So "its sends wait" above means that the child's work waits, never its thread.<br>• **Tested:** a renderer that stops receiving while another loads and draws, at M1 for the network process ([Milestone 1](../milestone-1.md) MA7) and at M2 for the GPU process |
| IP17 | **Rates that cost the browser work are bounded by protocol,** not left to chance: for example, at most one display-list update per offered frame (EL21). A breach is a violation (IP11) |

### 4.8 Identity and versions

| # | Rule |
|---|---|
| IP18 | **One build only** (IR8). The hello carries a build identifier compiled into every executable, and a mismatch refuses the connection. Message IDs come from the IDL and are stable within one build. There is no negotiation and no compatibility code |
| IP19 | **Every message's sender is checked** (R3). Each received message's audit token must match the token pinned when its channel was established (IP4). A mismatch is `Terminate`, as in Chromium, which rejects messages from an unauthorized peer [C2] |

### 4.9 Priority

| # | Rule |
|---|---|
| IP20 | **Urgency travels with the message.** Messages that carry input or a frame offer are sent with the sender's QoS as an override (`MACH_SEND_OVERRIDE` with an encoded priority, as libdispatch does for asynchronous sends [D1]). Bulk transfers and background work are sent without importance (`MACH_SEND_NOIMPORTANCE`). **Hypothesis:** a receiving thread that waits in `kevent64` is boosted by such a message. The sources checked don't settle it: libdispatch sets the override, and XNU's documentation doesn't describe the receiving side. It is measured in the first implementation milestone. If it fails, the event-loop thread's QoS (EL23) still applies |

### 4.10 Threads

| # | Rule |
|---|---|
| IP21 | **The IDL names the thread that owns each end** (EL4). Messages between threads of one process use the same channels and the same generated code as messages between processes (PM6) |

### 4.11 Testing

| # | Rule |
|---|---|
| IP22 | **Every decoder is fuzzed, and lying is tested.** The generator emits a fuzzing entry point per decoder (V2) and a generator of valid messages for property tests. Each interface that faces a less privileged process gets "lying peer" tests: a test renderer that claims another origin, frame or resource is terminated (V3, V9) |

### 4.12 Initial interface families

CP5–CP10 refine these. Each row names the ends and their threads.

| Family | Between | Purpose | Defined in |
|---|---|---|---|
| Launch and control | Browser core ⇄ helper control thread | Hello, build identifier, shutdown, memory pressure, interrupting a script (EL17) | CP5 |
| Frame host | Browser core ⇄ renderer event loop, per frame | Navigation commits, frame-tree changes, cross-frame messages through the browser | CP5 |
| Loader and storage | Renderer event loop ⇄ network I/O and storage threads | Fetch-shaped requests bound to the site lock; cookies and web storage, including the synchronous reads | CP5 |
| Compositing | Renderer event loop ⇄ browser compositor; **from M2** (Round 1, RV1-02), renderer event loop → GPU/compositor process, and GPU/compositor process ⇄ browser compositor | Frame subscriptions and offers (EL20, EL21); display-list updates and resources; hit-test data; from M2, layer commits with IOSurface ports, and scroll offsets back | CP6, CP7 |
| Input | Browser compositor or core → renderer event loop | Routed input events and their acknowledgements | CP8 |
| Text input and accessibility | Renderer event loop → browser main thread | The mirrors that answer AppKit's synchronous queries | CP8 |
| Embedder | Browser main thread ⇄ browser core | The engine/embedder API as messages | CP9 |
| Network control | Browser core ⇄ network process | Starting service workers (SW8, SW9); permission prompts, such as Local Network Access's; clearing data (EM4); endpoints issued again after a relaunch (IR10) | Round 2 (RV2-14, RV2-04) |
| Locks | Renderer event loop ⇄ browser core | Web Locks' requests, grants and releases, by storage key (SW10) | Round 2 (RV2-04) |
| Video frames | Media utility process → GPU/compositor process | Decoded video frames as IOSurfaces, for layers of their own (GR10, IM11), from M7 | Round 2 (RV2-14) |
| Agents | Event loop ⇄ event loop | `postMessage`, `MessagePort`, workers | CP10 |

## 5. Staging

| Level | What changes |
|---|---|
| L0 | All endpoints live in one process, as in-memory channels or Mach ports within the process. Interfaces and generated code are those of L1. **As amended in Round 1** (RV1-07): from M1 the renderer and network endpoints are in their own sandboxed processes, over Mach channels and memory entries; one process remains only for automated tests, fuzzing and Miri (IP2) |
| L1 | Channels cross processes over Mach. Launch, pinning and violation handling run for real. Nothing in any interface changes (AC1) |
| L2 | Site locks join the validators' records (IP10). The interfaces are unchanged |
| L3 | Out-of-process iframes add channels, one frame-host channel per frame. They add no new kind of channel |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Unix-domain socket pairs** | They pass descriptors, and give a peer's audit token per connection. But they can't carry Mach ports, so IOSurfaces would need a second transport, and S4 measured them slower for small messages. Rejected |
| B | **XPC** | Apple's high-level IPC over Mach, with self-describing dictionaries. Decoding renderer messages would then run Apple's C deserializer inside the privileged browser, against the Rule of 2 (PM2), and XPC delivers on dispatch queues rather than Axiom's thread loops (EL1). Rejected |
| C | **serde-derived typed channels** (`ipc-channel`) | Convenient, but unbounded and never blocking, without capability annotations, and built from macro-generated code. Rejected for IP7–IP9 |
| D | **Reusing Mojo or IPDL** | Large C++ systems with their own threading. Contrary to charter principle 4, which reserves reuse for justified infrastructure, when this core is small enough to own. Rejected |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | The same interfaces and generated code run from L0 to L3 (§5) |
| AC2 | Only the browser creates channels; validators decide from its records (IP4, IP10) |
| AC6 | In-memory channels with the deterministic scheduler; decoders run in every configuration (IP2) |
| AC9 | Mach FFI lives in `platform`. Decoders and the generator are safe Rust (IP8, IP9) |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Does a QoS override on a one-way message boost a receiver waiting in `kevent64`? | The first implementation milestone (IP20) |
| 2 | The IDL's syntax and the generator's design | The implementation milestone that builds IPC |
| 3 | Timeouts for synchronous requests, backlog sizes and queue limits per interface | **Answered** at Step 6 by [PF21](../performance.md): queue limits, the browser's backlog, and no timeout of their own for synchronous requests, which IP13 now says too (Round 2, RV2-14) |
| 4 | Do browser-allocated surfaces get charged to the renderer they serve (macOS runtime note §6.1)? | CP6 |

## Sources

All checked on 2026-10-17. Engine source was read at the head of each main branch, which isn't pinned to a commit.

- **Local:** SDK 27 headers `mach/message.h`, `mach/port.h`, `mach/notify.h`, `mach/vm_map.h`, `mach/memory_object_types.h`, `sys/event.h`, `sys/fileport.h`, `sys/socket.h`, `sys/un.h`.
- [C1] Chromium, "Mojo Style Guide" (`docs/security/mojo.md`) — https://chromium.googlesource.com/chromium/src/+/HEAD/docs/security/mojo.md
- [C2] Chromium, `mojo/core/channel_mac.cc` — https://github.com/chromium/chromium/blob/main/mojo/core/channel_mac.cc
- [C3] Chromium, "Mojo C++ Bindings API": associated interfaces, synchronous calls — https://chromium.googlesource.com/chromium/src/+/HEAD/mojo/public/cpp/bindings/README.md
- [F1] Firefox source docs, "IPDL: Inter-Thread and Inter-Process Message Passing" — https://firefox-source-docs.mozilla.org/ipc/ipdl.html
- [W1] WebKit, `Source/WebKit/Platform/IPC/Untrusted.h` (2026) — https://github.com/WebKit/WebKit/blob/main/Source/WebKit/Platform/IPC/Untrusted.h
- [W2] WebKit, `Source/WebKit/Platform/IPC/` (directory listing: `darwin/`, `cocoa/`, `*.messages.in` coders) — https://github.com/WebKit/WebKit/tree/main/Source/WebKit/Platform/IPC
- [V1] Servo, `ipc-channel` README — https://github.com/servo/ipc-channel
- [L1] Ladybird, `Libraries/LibIPC/` (directory listing; `TransportMachPort.h`) — https://github.com/LadybirdBrowser/ladybird/tree/master/Libraries/LibIPC
- [L2] Ladybird, `Libraries/LibIPC/Limits.h` — https://github.com/LadybirdBrowser/ladybird/blob/master/Libraries/LibIPC/Limits.h
- [D1] libdispatch, `src/mach.c`: asynchronous sends with `MACH_SEND_OVERRIDE` and an encoded priority — https://github.com/apple-oss-distributions/libdispatch/blob/main/src/mach.c
