# Interface note: `ipc`'s channels and `Untrusted<T>`

| | |
|---|---|
| **Status** | **Proposed** (2026-10-23), for the owner's approval at M1-CP3's gate, before any of `ipc`'s code (CR8, ES27) |
| **Interface** | The runtime every IPC user and the generated code call: channels over Mach and in memory, sending without blocking, received messages and their handles' owners, peers and their pinning, `Untrusted<T>`, validators and violations, shared memory and its streams |
| **Crates** | `ipc` (layer 2), over `platform-mach`'s safe API; used by `protocol`'s generated code ([its note](protocol-idl.md)), `event-loop` ([its note](event-loop-tasks.md)), `diagnostics` and the assemblies |
| **Last updated** | 2026-10-23 |

## 1. Purpose and scope

`ipc` is where IR1–IR10 become types. This note fixes what the rest of the engine sees:
- **channels** of two transports with one API, Mach between processes and in-memory queues that model Mach (IP1, IP2);
- **sending** that never blocks, with bounded backlogs (IP16, PF21);
- **receiving**, with every handle owned and every sender checked (IP9, IP19);
- **peers**: the rendezvous, the hello's pinning, one hello per child (IP4);
- **trust**: `Untrusted<T>`, validators over the receiver's records, typed violations (IP10, IP11);
- **shared memory**: memory entries, mappings that respect read-only memory, and single-writer streams (IP14).

**Out of scope:**
- the Mach calls themselves, which are `platform-mach`'s `src/sys/` (CR6), behind the safe types this note names;
- how a thread waits on its endpoints, which is [the event loop's note](event-loop-tasks.md);
- the wire format and generated API, which are [the IDL's note](protocol-idl.md);
- each interface's validators, which come with their interfaces.

## 2. The binding rules it implements

| Rule | What it asks of this interface |
|---|---|
| IP1, IP2 | Mach between processes. In one process, in-memory channels with the same API that **model Mach's behaviour:** queue limits, a zero-timeout send that fails at a full queue and returns its message, the send-possible notification, handles' lifetimes with no-senders (RV4-11) |
| IP3, IP4 | A channel joins two endpoints; order per channel only. Endpoints are capabilities that only the browser creates between processes. The rendezvous: one lookup, a hello with a fresh reply port, the PID and its version pinned, **one hello per spawned child** (RV4-12) |
| IP5, IP6 | Guarded, queue-limited receive rights, immovable unless transferred; port rights only as individual descriptors. Lifetimes by no-senders and dead-name notifications, and a closed channel delivered after its last message |
| IP9 | Handles out of the bytes; **a received message owns its handles until each has an owner;** a dropped, rejected or unparsable message is destroyed before anything else (RV4-05; S16: 1,000 dropped messages left 1,000 names) |
| IP10, IP11, IR2, IR3 | `Untrusted<T>` with one accessor, `validate`, and one named escape; validators compare claims with the receiver's records; `Violation::Terminate` and `Violation::Ignore`; no panic in validation (RV1-16) |
| IP14, IR7 | Inline, out-of-line copy-on-write, or shared memory with one writer, offsets never pointers, the reader copying before it validates and treating indices as untrusted |
| IP16, PF21, EL1 | Every queue bounded; sends with a zero timeout; a bounded backlog per peer (1,024 messages or 8 MiB); past it, the peer is hung. **A returned message is kept whole or destroyed** (RV4-05). Queue limits: 1,024 for input and display lists, 16 for control, 5 otherwise |
| IP19 | Every message's audit token checked against the channel's pin |
| ES10, ES12 | `ipc` is `panic-free`; channels and shared-memory streams are TS11 primitives, implemented once |

## 3. The design

### 3.1 Channels and ends

```rust
/// The process types (the process model §3).
pub enum ProcessType { Browser, Renderer, Network, Gpu, Utility }

/// A message type `protocol` generates: every message one end of a channel
/// receives, from one interface or from the several a channel carries (IP3).
pub trait Message: Sized + Send {
    /// The interfaces whose messages this type holds.
    const INTERFACES: &'static [InterfaceId];
    /// The largest encoded body (the IDL's limits, as computed per message).
    const LARGEST_BODY: usize;
    /// This value's interface and message IDs.
    fn ids(&self) -> (InterfaceId, u16);
    /// Writes the body and moves the handles into `out`.
    fn encode(self, out: &mut Encoder);
}

/// One side of a channel: where `In` messages arrive, and a sender of `Out`.
pub struct End<In: Message, Out: Message> {
    pub endpoint: Endpoint<In>,
    pub sender: Sender<Out>,
}

/// Makes a channel. Only the browser makes channels between processes (IP4);
/// a helper makes only in-memory channels between its own threads.
pub fn channel<A: Message, B: Message>(
    transport: Transport,
    limits: Limits,
) -> Result<(End<A, B>, End<B, A>), ChannelError>;

pub enum Transport { Mach, Memory }

/// Queue limits per end (PF21): 1,024, 16 or 5.
pub struct Limits { pub a: QueueLimit, pub b: QueueLimit }
```

- **An end is a capability.** To give one to another process, the browser wraps it as an `EndpointGrant`, which the IDL's `endpoint<…>` field carries as two handle slots: the end's receive right, moved, and a send right to the other end (IP3, IP5). A process can reach another only through ends it was given (IP4).
- **An endpoint belongs to one thread.** `Endpoint<In>` is `Send` until it is registered with a thread loop ([the event loop's note](event-loop-tasks.md) §3.2), which consumes it; from then on its messages arrive on that thread only (EL4). An endpoint that was never registered is dropped with its queue, and its peer sees the channel close (IP6).
- **Mach channels** are made with `mach_port_construct`: guarded, with the end's queue limit, immovable unless the interface transfers them, and importance receivers when they take input (IP5). `platform-mach` makes them; `ipc` never names a port.
- **Memory channels** are a queue per end in the process, with the same limits and the same events (§3.5).
- **Several interfaces on one channel** (IP3): the IDL's `channel` declaration ([its note](protocol-idl.md) §3.3) generates one `Message` type per end over all their messages, so they share one queue and one order. A received message whose interface isn't among its endpoint's `INTERFACES` is `Check::UnknownInterface`.

### 3.2 Received messages and their handles

```rust
/// A message as it arrives, before decoding. It owns its body and every
/// handle; dropping it destroys whatever no one took (IP9).
pub struct Received {
    pub interface: InterfaceId,
    pub id: u16,
    pub sender: Peer,
    body: Body,        // inline bytes, or an out-of-line copy this owns
    handles: Handles,  // every port right that came with it
}

/// The handles of one message, in slot order. Each kind's owner frees its
/// right when dropped.
pub struct Handles { /* inline array of up to 64 slots */ }

impl Handles {
    /// Takes slot `index` as kind `H`, or reports the wrong kind or a missing slot.
    pub fn take<H: Handle>(&mut self, index: usize) -> Result<H, DecodeCheck>;
    /// Every slot was taken: no leftover handle (IP9's "no extra handle").
    pub fn finish(self) -> Result<(), DecodeCheck>;
}

/// The owners, each of which deallocates its right when dropped.
pub struct SendRight;            // a send right to an endpoint
pub struct ReceiveRight;         // an endpoint's receive right, moved
pub struct SendOnceRight;        // a synchronous request's reply
pub struct MemoryEntry<A>;       // shared_memory(…) (§3.6)
pub struct SurfacePort;          // iosurface
pub struct FilePort;             // file
```

- **Ownership from the first instant:** `platform-mach` receives into a buffer, then moves each descriptor's right into its typed owner before returning. No path holds a raw name.
- **Destroyed first on any failure:** a message whose sender check fails, whose interface the endpoint doesn't carry, whose body doesn't decode, or which a handler ignores (`Violation::Ignore`) is dropped whole, which releases every right and the out-of-line copy before any other work (IP9, IP11). S16's 1,000 dropped messages leaving 1,000 names is the regression this prevents (§7).

### 3.3 Sending without blocking

```rust
pub struct Sender<M: Message> { /* send right, encoder buffer, backlog */ }

impl<M: Message> Sender<M> {
    /// Sends with a zero timeout (IP16). Never blocks.
    pub fn send(&mut self, message: M) -> Result<Sent, SendError>;
    /// Messages waiting in this sender's backlog.
    pub fn backlog(&self) -> BacklogLen;
}

pub enum Sent {
    /// The kernel queued it at the receiver.
    Delivered,
    /// The receiver's queue was full; the message is in the backlog, whole.
    Backlogged,
}

pub enum SendError {
    /// The receiver is gone (a dead-name notification, IP6). The message was destroyed.
    Closed,
    /// The backlog is past its bound (PF21): the receiver is hung. The message was destroyed.
    PeerHung,
}
```

- **A full queue returns the message:** XNU hands back a message that a zero-timeout send couldn't queue, with its rights and an extra reference to the destination (S16 §4.2). `platform-mach` turns it into a `Returned` value that owns all of it; the backlog keeps it, to send again, or drops it, releasing both (IP16, RV4-05).
- **Order holds:** once a backlog exists, later messages join it rather than overtaking it. The sender asks for Mach's send-possible notification (`MACH_NOTIFY_SEND_POSSIBLE`; observed in `mach/notify.h`), and the thread loop drains the backlog in order when it arrives.
- **The bound** is PF21's: 1,024 messages or 8 MiB, whichever comes first. Past it, `send` returns `PeerHung`. The browser treats the child as hung (PF19); a helper reports the peer to the browser and closes its ends (IP16, as amended in Round 2); a child that can't send to the browser stops producing until it drains.
- **Synchronous requests** (IP13) are generated `call_*` functions over a `SendOnceRight` and one blocking receive on the calling thread, with no timeout of their own (PF21). They are the one place a thread waits on IPC, and only in the IDL's marked direction.

### 3.4 Peers, trust and violations

```rust
/// Who sent a message: the channel's pinned identity (IP4, IP19).
pub struct Peer {
    pub process: ProcessType,
    pub pid: i32,
    pub pid_version: u32,
    pub id: PeerId,           // the browser's own handle for this child
}

/// A value from a less privileged sender (IP10). No `Deref`, no public field.
pub struct Untrusted<T>(T);

impl<T> Untrusted<T> {
    /// The only accessor: the validator decides, from the receiver's records.
    pub fn validate<V: Validator<T>>(self, validator: &V) -> Result<V::Valid, Violation>;
    /// The one escape, for a value validated elsewhere, which must say where.
    pub fn validated_elsewhere(self, by: Elsewhere) -> T;
}

/// Where a value was validated instead: a rule ID and a place, as a constant.
pub struct Elsewhere(&'static str);

/// A check of a claim against the receiver's own records (IR2): the sender's
/// process type, site lock and capabilities, and the IDs the browser issued.
pub trait Validator<T> {
    type Valid;
    fn validate(&self, value: T) -> Result<Self::Valid, Violation>;
}

pub enum Violation {
    /// The sender couldn't have produced this legitimately: end it, and log why (IR3).
    Terminate(Reason),
    /// An expected race, such as a message about a frame just destroyed: drop the message.
    Ignore(Reason),
}

/// What a violation names in its report (OB17): never the message's content.
pub struct Reason { pub interface: InterfaceId, pub message: u16, pub check: Check }

pub enum Check {
    Sender,                 // the audit token didn't match the pin (IP19)
    SecondHello,            // a child said hello twice (IP4, RV4-12)
    Build,                  // the build identifier differed (IP18)
    UnknownInterface,       // the endpoint doesn't carry it
    Decode(DecodeCheck),    // the IDL note's §3.5 checks
    State(&'static str),    // a validator's rule, by ID
}
```

- **The rendezvous** (IP4): the browser checks in a receive right under a name made for this launch, compiles it into the helper's profile as the one name it may look up (ADR-0004 D2), and spawns the helper (`platform-process`). The helper looks the name up and sends `Control::hello` with a fresh reply port. S4 built the same exchange with `bootstrap_check_in` and `bootstrap_look_up` ([S4 results](../../research/s4-results.md), its library's row), and its profiles allowed exactly that one look-up. The browser accepts the hello only if its audit token's PID is a child it spawned and hasn't heard from (`Check::SecondHello` otherwise), pins that token's PID and version, and answers on the reply port with `welcome`. From then on, every endpoint the helper holds arrives inside messages.
- **Every message is checked** against its channel's pin before decoding (IP19); a mismatch is `Terminate(Check::Sender)`, and the message is destroyed.
- **Validators can't panic:** they live in `panic-free` crates (ES10) and use checked arithmetic (IP11). A validator that finds no record of an ID it should know returns `Ignore` when a race explains it and `Terminate` when nothing does; which is which is each validator's documented rule.
- **What `Terminate` does is the receiver's policy:** the browser ends the sender and recovers (IR3, IR10); a helper ends itself when the browser, which it trusts, sent a message it can't decode ([the IDL's note](protocol-idl.md) §5).

### 3.5 Memory channels model Mach

The in-memory transport implements, per end, the behaviours tests depend on (IP2, RV4-11):
- a queue limit, with a zero-timeout send that fails at a full queue and returns the message whole;
- the send-possible event, when the queue drains below its limit;
- no-senders, when the last `Sender` to an end is dropped, and dead-name, when the `Endpoint` is;
- handles as fake rights with the same owners and the same leak checks, so a test can count live handles as `platform-mach` counts port names;
- peers as fake audit tokens, so lying-peer tests can claim another PID (IP22).

Single-process mode (tests, fuzzing and Miri only; RV1-07) and in-process interfaces (`within process`) use it.

### 3.6 Shared memory and streams

```rust
pub struct ReadOnly;
pub struct ReadWrite;

/// A memory entry: shared memory as a port (IP14).
pub struct MemoryEntry<A> { /* entry port, size */ }

impl MemoryEntry<ReadWrite> {
    pub fn create(bytes: usize) -> Result<Self, MemoryError>;
    /// An entry for the same memory whose holder can map it only for reading.
    pub fn read_only(&self) -> Result<MemoryEntry<ReadOnly>, MemoryError>;
    pub fn map(&self) -> Result<Mapping<ReadWrite>, MemoryError>;
}
impl MemoryEntry<ReadOnly> {
    pub fn map(&self) -> Result<Mapping<ReadOnly>, MemoryError>;
}

/// A mapping, unmapped on drop. Read-write mappings lend `&[AtomicU64]`.
/// Read-only mappings lend `ReadOnlyWords`, whose only operation is a relaxed
/// load, followed by a fence where acquire order is needed: Rust defines no
/// other atomic access to read-only memory.
pub struct Mapping<A> { /* address, length */ }
pub struct ReadOnlyWords<'a> { /* … */ }
impl ReadOnlyWords<'_> {
    pub fn load(&self, index: usize) -> Option<u64>;   // Ordering::Relaxed
}
```

- **Why the read-only type exists** (sourced, Rust 1.94.1's `core::sync::atomic`, observed in its source): "all atomic accesses on read-only memory are undefined behavior", except "'sufficiently small' atomic loads with `Ordering::Relaxed`", 8 bytes on aarch64; "If you need to do an acquire load on read-only memory, you can do a relaxed load followed by an acquire fence instead." A reader that maps a writer's memory read-only, as the browser maps a helper's region and S10's compositor mapped a renderer's images, gets a type that can't do anything else.
- **Read-only entries:** `read_only` derives an entry with `mach_make_memory_entry_64`'s `permission` limited to reading, from the parent entry (observed: the parameters `permission` and `parent_entry` in `mach/vm_map.h`). **Hypothesis,** tested at M1-CP3: a holder of that entry can't map it writable. No security property rests on it: a reader keeps its own positions privately ([the region's note](trace-and-diagnostics-region.md) §3.4), and the entry only stops a writer from confusing itself.
- **Streams over shared memory:** single-producer, single-consumer rings of words, whose indices the reader treats as untrusted, copying each frame out before decoding it (IP14, IR7). **One implementation serves `trace` and `ipc`** (ES12's "each implemented once"); where it lives is the region note's §3.3 and a decision for the owner, since `trace`, at the foundation, can't depend on `ipc`.
- **Out-of-line copies** for one-shot payloads are the IDL's encoder's choice (its note §3.5); `ipc` owns the received copy and deallocates it with the message.

### 3.7 What the generated code calls

The rest of `ipc`'s public surface exists for `protocol`'s generated code ([the IDL's note](protocol-idl.md) §3.6), and subsystems meet it only through generated types:

```rust
/// An interface's ID, as the IDL declares it (16 bits on the wire).
pub struct InterfaceId(u16);

/// Where `Message::encode` writes: the body's bytes and the handles, moved in.
pub struct Encoder { /* reusable buffer, inline handle slots */ }

/// A bounded string, byte array and array (the IDL's `string(N)`,
/// `bytes(N)`, `array<T, N>`): constructed only within their bounds.
pub struct BoundedString<const N: usize>(/* … */);
pub struct BoundedBytes<const N: usize>(/* … */);
pub struct BoundedVec<T, const N: usize>(/* … */);

/// An end given to another process: its receive right and a send right to
/// the other end (the IDL's `endpoint<…>`; §3.1).
pub struct EndpointGrant<I> { /* … */ }

/// A request's number, which its reply repeats; and the owed reply.
pub struct RequestNumber(u64);
pub struct Responder<R: Message> { /* channel, number */ }
impl<R: Message> Responder<R> { pub fn reply(self, reply: R) -> Result<Sent, SendError>; }

/// A synchronous request's failure: the peer closed, or the reply didn't decode.
pub enum CallError { Closed, Violation(Violation) }

/// For fuzzing and property tests (IP22): handle kinds the memory
/// transport's fake handles stand for, and the caller's entropy.
pub enum HandleKind { Send, Receive, SendOnce, MemoryEntry, Surface, File }
pub trait Entropy { fn next_u64(&mut self) -> u64; }
```

- **The bounded types are the IDL's bounds in Rust:** a value that exceeds its bound can't be built, so a generated encoder never fails (ES7). Each dereferences to its contents, read-only.
- **`Entropy`** keeps `ipc` free of a randomness dependency: the fuzz targets feed it from their input, and property tests from a seeded source in `test-support`.

## 4. Invariants, threads and ownership

1. **No raw port name leaves `platform-mach`.** Every right has a typed owner whose drop releases it, from receipt to the last holder.
2. **A failed or ignored message is destroyed before anything else happens** (IP9).
3. **No thread waits on a send** (EL1). The one wait on IPC is a synchronous request's reply, only where the IDL marks it (IP13).
4. **Every queue is bounded,** in the kernel by its limit and in the sender by its backlog (IP16).
5. **Order holds per channel,** including across a backlog, and nowhere else (IP3).
6. **Every message is checked against its channel's pin** before its body is read (IP19).
7. **An `Untrusted<T>` yields its value only to a validator or to `validated_elsewhere`** (IP10).
8. **Threads:** an endpoint is drained only by the thread loop it is registered with (EL4). A `Sender` may move between threads as an owned value, and each thread that sends holds its own (EL11).
9. **`ipc` is `panic-free`** (ES10), so a malformed message can't abort the browser through it.

## 5. Errors and violations

| Situation | Result |
|---|---|
| A send to a full queue | `Sent::Backlogged`; the message waits whole |
| A send past the backlog's bound | `SendError::PeerHung`; the message destroyed; the browser treats the peer as hung (PF19) |
| A send to a dead receiver | `SendError::Closed`; the message destroyed |
| A message whose audit token differs from the pin | `Violation::Terminate(Check::Sender)`; destroyed unread |
| A second hello from one child | `Violation::Terminate(Check::SecondHello)` |
| A hello with another build | `Violation::Terminate(Check::Build)`; the browser refuses the connection (IP18) |
| A message on an interface the endpoint doesn't carry | `Violation::Terminate(Check::UnknownInterface)` |
| A body or handle list that doesn't decode | `Violation::Terminate(Check::Decode(…))` |
| A claim the receiver's records contradict | The validator's `Terminate` or `Ignore` |
| The peer closed its end | A closed event after its last message (IP6) |
| A memory entry or mapping that fails | `MemoryError`, which the creator's assembly treats as a launch failure |

## 6. Efficiency

- **The transport's cost,** measured by S4: 3.75 µs for a 64-byte round trip over Mach, 172–177 µs for 1 MiB out of line; the sandbox adds nothing (IPC §2).
- **No allocation in steady state:** each `Sender` reuses its encoder's buffer; handle lists are inline arrays of up to 64 slots; a thread's receive buffer is sized once, for the largest inline message its interfaces allow.
- **Memory:** a backlog is bounded by PF21's 8 MiB; an empty one costs a few words. Queue limits bound the kernel's side.
- **First-tier tests** (ES23, PF5): the sizes of `Received`, `Handles`, `Peer` and the backlog's entries as `const` assertions; no allocation on a send or receive after warm-up, counted by `test-support`.

## 7. Tests and oracles

- **One behaviour suite, both transports:** queue limit, the returned message whole at a full queue, the send-possible event draining the backlog in order, no-senders on the last sender's drop, dead-name on the endpoint's, handles freed with their owners, a sender mismatch ending in `Terminate`. Run over Mach in real processes and over memory in one, so the model is checked against the thing it models (IP2).
- **S16's regression as a test:** 1,000 dropped, ignored and rejected messages leave the receiver's port-space count where it began (`platform-mach` reads the count).
- **loom** on the memory channel, the backlog's drain against the send-possible event, and the shared-memory streams (TS11).
- **Miri** over `ipc`'s safe logic: the memory transport, `Untrusted<T>`, the handles' bookkeeping (TS11).
- **Lying peers** (IP22): a peer claiming another PID, a second hello, another build, an unknown interface; each must end in its `Check`.
- **The streams' reader** under a fuzz target fed arbitrary shared memory: forged indices, lengths and records must end in a refused record or a violation, never a panic or an out-of-bounds read.

## 8. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **`Untrusted<T>` with one validating accessor** (WebKit's design, IPC §3) | The type system stops use before validation, at no cost at run time. **Recommended** |
| B | **Generated "unvalidated" twin types per message** | The same guarantee with twice the types, and a conversion per message to keep in step. Rejected |
| C | **Validation inside the decoders** | Decoders would need the receiver's records, mixing shape with meaning, and couldn't be fuzzed alone. IP10 keeps them apart. Rejected |
| D | **Blocking sends with timeouts** | A thread would wait on a peer, against EL1 and IR5. Rejected |
| E | **Unbounded queues,** as Servo's `ipc-channel` | A flooding peer would grow the browser's memory without limit (IP16). Rejected |
| F | **A receive thread per endpoint,** or an I/O thread per process | An extra hop per message, against EL4 and the event loop's alternative B. Rejected |
| G | **`std::sync::mpsc` or `crossbeam` for in-memory channels** | Neither models Mach's limits, returned messages or handles, so tests in one process wouldn't exercise IP16 (RV4-11); and ES12 keeps the primitives in one implementation. Rejected |

## Sources

- **Observed** (2026-10-23): `mach/notify.h` (`MACH_NOTIFY_SEND_POSSIBLE`) and `mach/vm_map.h` (`mach_make_memory_entry_64`'s `permission` and `parent_entry`) in the Command Line Tools' SDK; `library/core/src/sync/atomic.rs` in Rust 1.94.1's sources, "Atomic accesses to read-only memory".
- [S4 results](../../research/s4-results.md) §4 (Mach's costs), [S10 results](../../research/s10-results.md) (memory entries mapped read-only), [S16 results](../../research/s16-results.md) §4 (dropped messages and port names; the returned message).
- The binding documents cited by rule: [IPC](../ipc.md), the [process model](../../security/process-model.md), the [performance budgets](../../performance.md) (PF19, PF21), the [event loop](../event-loop-and-threading.md) (EL1, EL4, EL11), the [engineering standards](../../technology/engineering-standards.md) (ES10, ES12, ES23).
