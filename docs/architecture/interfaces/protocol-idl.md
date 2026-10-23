# Interface note: the IDL and the generated API

| | |
|---|---|
| **Status** | **Proposed** (2026-10-23), for the owner's approval at M1-CP3's gate, before any of `protocol`'s code (CR8, ES27) |
| **Interface** | The language every IPC interface is written in; the code `codegen` generates from it into `protocol`; the wire format's details beyond IP9; the build identifier IP18 compares |
| **Crates** | `codegen` (the parser and the generator), `protocol` (the generated code), and the runtime it calls in `ipc` ([its note](ipc-channels.md)) |
| **Settles** | [IPC](../ipc.md)'s open question 2, "the IDL's syntax and the generator's design"; IP7's "its syntax is chosen at implementation" |
| **Last updated** | 2026-10-23 |

## 1. Purpose and scope

Every message between Axiom's processes and threads is declared once, in the **Axiom IDL**, and `codegen` turns the declarations into Rust: types, canonical encoders, total decoders, dispatch traits, fuzzing entry points and generators of valid messages (IP8). This note fixes:
- the language: its files, declarations, types and the static rules the generator enforces (§3.1–§3.4);
- the wire format, where IP9 leaves room: message IDs, handle slots, out-of-line bodies, requests (§3.5);
- the generated Rust API that the assemblies and subsystems call (§3.6);
- the build identifier (§3.7).

**Out of scope:** the interfaces themselves. Each is declared by the checkpoint that needs it: launch and control at M1-CP3, with this note's example in §3.4 as its draft; frame host, loader and compositing at M1-CP4 (IPC §4.12). The runtime types the generated code names (`Untrusted<T>`, `Violation`, `Received`, the handle owners, channels) are [`ipc`'s note](ipc-channels.md).

## 2. The binding rules it implements

| Rule | What it asks of this interface |
|---|---|
| IP3 | An endpoint carries interfaces whose messages stay ordered; order holds per channel only |
| IP5 | Port rights travel only as individual port descriptors, with a cap per message far below XNU's 16,383 |
| IP7 | Each interface declares its messages with stable IDs, parameters, replies for requests, maximum sizes, the process types that may hold each end, the thread role owning each end, and synchronous requests' direction. Parameter types: bounded integers, enums, bounded strings, byte arrays and arrays, structures, typed handles. Renderer-facing interfaces declare no `iosurface` (RV2-14) and no `file` (IP15, with SW11's one exception) |
| IP8 | The generator is safe Rust run by the tooling, its output committed and checked fresh (ADR-0022 D5, ES19); decoders check every length, count, enum and handle slot; dispatch rejects unknown IDs; one fuzzing entry point per decoder; no `unsafe` in generated code |
| IP9 | One encoding per value: little-endian, fixed-width, length-prefixed, no padding, nothing optional at the end; non-canonical input and trailing bytes refused; handles out of the bytes, matched by index and type; decoders take the handle list by value (RV4-05) |
| IP10, IP11, IP12 | Values from a less privileged sender arrive as `Untrusted<T>`; decode failures, unknown IDs and wrong handles are `Terminate`; every value that decodes means something (typed IDs, enums, no parallel arrays) |
| IP13 | Synchronous requests only toward more privilege, only where the IDL marks them, no timeout of their own |
| IP18 | One build: the hello carries a build identifier; message IDs are stable within a build; no negotiation |
| IP21 | The IDL names the thread that owns each end |
| IP22 | A fuzzing entry point and a generator of valid messages per decoder; lying-peer tests per interface |
| CR10, CR13 | One source of truth, the IDL; dispatch an exhaustive `match`; the build identifier carries the build's flavor |
| ES6, ES7, ES15, ES19 | Exhaustive matches; parse, don't validate; documented public items; generators in `codegen`, output committed under `src/generated/` |
| DP: none added | The parser and generator are Axiom's own; `codegen` gains no dependency |

## 3. The design

### 3.1 Files and lexical rules

- **Files:** `crates/protocol/idl/*.idl`, one file per interface family (`control.idl`, `frame_host.idl`, …). All files share one namespace, since one build compiles them together (IP18).
- **Text:** UTF-8. `//` starts a comment. `///` starts a documentation comment, which the generator copies onto the item it precedes. **Every declaration, message, field, enum variant and end is documented,** or the generator refuses the file, so the generated code passes `missing_docs` (ES15) and reviewers read the intent beside the bytes.
- **Names:** types and interfaces in `UpperCamelCase`; messages, fields, ends and enum variants in `snake_case`. A name may not shadow a built-in type (§3.2).
- **Numbers:** decimal, with `_` allowed, and the suffixes `KiB` and `MiB` in limits and sizes.

### 3.2 Types

| IDL type | Rust type | Encoding (§3.5) | Decoder's checks |
|---|---|---|---|
| `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64` | The same | Little-endian, fixed width | — |
| `bool` | `bool` | One byte, 0 or 1 | Any other byte refused |
| `range Name: u32 in 1..=16384;` (declared, on any integer type) | A newtype `Name` with `new(value) -> Option<Name>` and `get()` | As its integer | Outside the range refused |
| `enum Name: u8 { … }` (explicit values) | A Rust `enum` with those discriminants | Its declared width | Unknown values refused |
| `struct Name { … }` | A Rust `struct` with public fields | Fields in order, no padding | Each field's |
| `id Name: u64;` | A newtype (IP12), one type per kind | As its integer | — (the validator decides whether the receiver issued it) |
| `string(N)` | `ipc::BoundedString<N>` | `u32` byte length, then the bytes | Length ≤ N; valid UTF-8 |
| `bytes(N)` | `ipc::BoundedBytes<N>` | `u32` length, then the bytes | Length ≤ N |
| `array<T, N>` | `ipc::BoundedVec<T, N>` | `u32` count, then the elements | Count ≤ N; each element's |
| `[T; N]` | `[T; N]` | Exactly N elements, no count | Each element's |
| `option<T>` | `Option<T>` | One byte, 0 or 1, then the value if 1 | Tag 0 or 1 only |
| `endpoint<Interface.end>` | `ipc::EndpointGrant<Interface's marker>` | Two handle slots: the end's receive right, and a send right to the other end | Two ports, of the right dispositions |
| `shared_memory(read_only, N)`, `shared_memory(read_write, N)` | `ipc::MemoryEntry<ipc::ReadOnly>` or `<ipc::ReadWrite>` | One handle slot, the memory entry; its size as a `u64` in the body | Size ≤ N and a whole number of pages |
| `iosurface` | `ipc::SurfacePort` | One handle slot | A port |
| `file` | `ipc::FilePort` | One handle slot, a fileport | A port |

**Vocabulary types** join with the interfaces that need them, at M1-CP4: `origin`, `url(N)` and `site` from `url`, rectangles and sizes from `geometry`. Each is encoded as its own crate defines canonically and decoded through that crate's parser, so an origin decodes to an `Origin` or the message fails (IP12). No vocabulary type is a string on the wire without its parser behind it.

**No floating point** in M1's interfaces: geometry crosses as layout units, which are integers (LY1). A float type, with NaN and negative zero refused, is added if an interface needs one, as an amendment to this note.

### 3.3 Declarations

```text
/// A frame's ID, issued by the browser to one renderer (IP12).
id FrameId: u64;

/// A width in device pixels, within Metal's largest texture.
range Width: u32 in 1..=16384;

/// Why a document is drawn or not.
enum Visibility: u8 {
    /// Shown in a window.
    visible = 0,
    /// Not shown.
    hidden = 1,
}

/// The build a process was compiled from (IP18, CR13).
struct BuildId {
    /// The hash of every IDL file and of the generator (§3.7).
    protocol: [u8; 16],
    /// The source revision the bundle was built from; empty in development.
    revision: string(64),
    /// Release, test or stress.
    flavor: Flavor,
}
```

An **interface** declares its ID, its two ends, its limit, its synchronous direction if any, and its messages by direction:

```text
/// <what the interface is for, and the rules it implements>
interface 1 Control {
    /// <the browser's end>
    end browser: browser on core;
    /// <the helper's end>
    end helper: renderer, network on control;
    /// The largest encoded message.
    limit 4 KiB;

    helper -> browser {
        /// <doc>
        1 hello(build: BuildId);
    }
    browser -> helper {
        /// <doc>
        2 ping(sequence: u64);
    }
}
```

- **`interface <id> <Name>`:** the ID is explicit, unique in the build, and stable while the source doesn't change. Explicit IDs keep the wire, traces and violation reports readable (OB4, OB17), and a renumbering shows up in review.
- **`end <name>: <process types> on <thread role>`:** the process types that may hold the end, from the process model (browser, renderer, network; `gpu` from M2, `utility` from M7), and the role of the thread that owns it, from the event loop's lists (EL6–EL8: browser `main`, `core`, `compositor`, `worker`; renderer `event_loop`, `control`, `helper`; network `io`, `storage`, `worker`, `control`).
- **`within process;`** marks an interface that never crosses a process boundary, such as the embedder's between the browser's main and core threads (IPC §4.12). Its messages aren't wrapped as untrusted, and it may use only the in-memory transport.
- **`limit <size>;`** bounds every message's encoded body. The generator computes each message's largest encoding from its bounds and refuses a message that could exceed the limit. A limit above 256 KiB is refused: such data belongs in `shared_memory` (IP14, PF21).
- **Messages:** `<id> <name>(<fields>)`, with IDs unique within the interface across both directions.
- **Requests:** `<id> <name>(<fields>) -> <id> <reply>(<fields>)`. The reply travels the other way, on the same channel, so it stays ordered with the rest (IP3).
- **Synchronous requests:** the interface declares `sync <end> -> <end>;`, and each synchronous request is marked `sync`. The generator accepts them only in that direction, only toward more privilege (below), and never toward a renderer's end from the browser (IP13).
- **Channels:** each interface is a channel of its own by default. `channel <Name> { <Interface>, <Interface> }` declares one that carries several, whose messages stay mutually ordered because they share one queue (IP3). Its interfaces must have the same two ends; the generator emits, per end, one message enum over all of them, dispatched by interface.
- **Exceptions:** `exception "<rule>";` on an interface lets it declare a handle type otherwise barred to it. The generator keeps the list of the exceptions the design names, today one: SW11, the origin private file system's `file` handle (IP15). An exception the list doesn't name is refused, so widening a renderer's handles takes an edit to `codegen` and an amendment, not a line of IDL.

**Privilege, from the process model** (inference: no document gives a total order, but IR1, IR5, IR6 and IP13 imply this one): the browser first; then the network and GPU processes, which serve renderers and receive their synchronous requests; then the utility and renderer processes, whose content the others parse. **A direction's messages arrive as `Untrusted<T>`** when its sender's process type isn't more privileged than its receiver's, peers of one type in different processes included. Messages within one process don't, since a thread doesn't defend against its own process.

### 3.4 The static rules the generator enforces

The generator refuses a file, naming its line and column, when:
1. a name, interface ID or message ID repeats, or an enum value does, or a channel joins interfaces with different ends;
2. a declaration, message, field, variant or end lacks documentation;
3. a string, byte array or array lacks its bound, or a type is recursive;
4. a message's largest encoding exceeds its interface's limit, or a limit exceeds 256 KiB;
5. a message carries more than 64 handle slots (IP5's cap, far below XNU's 16,383);
6. an end facing a renderer can receive an `iosurface`, or a `file` without a listed exception (IP7, IP15, RV2-14);
7. a synchronous request runs from more to less privilege, toward the browser's core (IR6 sends them to the network process), or without the interface's `sync` line;
8. an end names a process type or thread role that doesn't exist, or a role that process doesn't have;
9. a `within process` interface names two process types.

**The draft of launch and control,** which M1-CP3 declares, to show the language at work. Its messages follow IP4, IP18, PF19 and OB10; [the event loop's note](event-loop-tasks.md) and [the region's note](trace-and-diagnostics-region.md) specify their semantics. `BuildId` is §3.3's; `RegionLayout` and `PressureLevel` are declared beside it in the same file:

```text
/// Launch and control: a helper's first exchange with the browser, then the
/// watchdog, the diagnostics region's notices, memory pressure and shutdown
/// (IPC §4.12; IP4, IP18, PF19, OB10, OB12).
interface 1 Control {
    /// The browser's end, on its core thread.
    end browser: browser on core;
    /// The helper's end, on its control thread.
    end helper: renderer, network on control;
    /// No control message is large.
    limit 4 KiB;

    helper -> browser {
        /// The helper's first message, on the rendezvous (IP4), with its build (IP18).
        1 hello(build: BuildId);
        /// The answer to a ping (PF19): the oldest running task's age in
        /// nanoseconds, 0 when no watched thread runs one, and how many do.
        3 pong(sequence: u64, oldest_task: u64, busy: u8);
        /// A watched thread left idle while the watchdog rested (PF19).
        6 wake();
        /// A thread's ring passed its watermark (OB12).
        7 trace_watermark(slot: u8);
    }

    browser -> helper {
        /// The browser's answer to a hello: the diagnostics region and its layout (OB10).
        /// The helper's working endpoints follow in messages of their own, from M1-CP4.
        2 welcome(region: shared_memory(read_write, 1 MiB), reader: shared_memory(read_only, 16 KiB), layout: RegionLayout);
        /// PF19's ping, while a session is active.
        4 ping(sequence: u64);
        /// The watchdog rests: send `wake` when a watched thread next leaves idle.
        5 rest();
        /// Switch detailed recording on or off, per category (OB7).
        8 detail(mask: u64);
        /// Trim memory (PF30).
        9 memory_pressure(level: PressureLevel);
        /// End cleanly.
        10 shutdown();
    }
}
```

### 3.5 The wire format

- **The message ID:** Mach's 32-bit `msgh_id` (observed, `mach/message.h`: `typedef integer_t mach_msg_id_t`) carries the interface ID in its high 16 bits and the message ID in its low 16. So `platform-mach` can route a message to its interface's decoder before reading its body.
- **The body:** the fields in declaration order, encoded per §3.2, with no padding and nothing after the last field. A request's body starts with a `u64` request number, which its reply repeats.
- **Handles:** one port descriptor per handle slot, in the order a depth-first walk of the fields meets them. A handle field takes no bytes in the body: an `option` or `array` around handles says by its tag or count how many slots follow. The decoder takes each slot from the handle list by index and checks its kind (IP9). Any handle left over at the end is a violation.
- **Inline or out of line:** a body under 16 KiB, one page on this Mac (observed, `sysctl hw.pagesize`: 16384), travels inline. A larger body travels as one out-of-line virtual copy, so the sender can't change it after sending (IP14), as Chromium sends "messages of a page or more" out of line (IPC §3). The receiver's platform layer owns the copy and deallocates it; the decoder sees bytes either way. The threshold is a constant, measured at M1-CP3 against S4's figures (3.75 µs for a 64-byte round trip, 172–177 µs for 1 MiB out of line).
- **Canonical decoding:** a decoder refuses an unknown message ID, a length or count past its bound, a tag or `bool` other than 0 or 1, an enum value or range outside its declaration, invalid UTF-8, a short or long body, and a missing, extra or wrong-kind handle. Each refusal is `Violation::Terminate` with a reason naming the interface, the message and the check (IP11).

### 3.6 The generated API

For each interface, `codegen` writes `crates/protocol/src/generated/<interface>.rs`, a module named after the interface. For `Control`:

```rust
pub mod control {
    /// The interface's ID (§3.3).
    pub const INTERFACE: ipc::InterfaceId = ipc::InterfaceId::new(1);

    /// The helper's first message (IP4, IP18).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Hello { pub build: crate::BuildId }
    // … one struct per message …

    /// Every message to the browser's end, as received.
    #[derive(Debug)]
    pub enum ToBrowser { Hello(Hello), Pong(Pong), Wake(Wake), TraceWatermark(TraceWatermark) }
    /// Every message to the helper's end.
    #[derive(Debug)]
    pub enum ToHelper { Welcome(Welcome), Ping(Ping), Rest(Rest), Detail(Detail), MemoryPressure(MemoryPressure), Shutdown(Shutdown) }

    /// The browser's end. Each message from a less privileged sender arrives
    /// as `ipc::Untrusted<T>`, which only a validator unwraps (IP10).
    pub trait BrowserEnd {
        fn hello(&mut self, from: &ipc::Peer, message: ipc::Untrusted<Hello>) -> Result<(), ipc::Violation>;
        fn pong(&mut self, from: &ipc::Peer, message: ipc::Untrusted<Pong>) -> Result<(), ipc::Violation>;
        fn wake(&mut self, from: &ipc::Peer) -> Result<(), ipc::Violation>;
        fn trace_watermark(&mut self, from: &ipc::Peer, message: ipc::Untrusted<TraceWatermark>) -> Result<(), ipc::Violation>;
    }

    /// Decodes a message for the browser's end and calls the handler's method
    /// for it, through an exhaustive `match` (ES6, CR10). It consumes the
    /// message, handles included, whatever happens (IP9).
    pub fn dispatch_to_browser(handler: &mut impl BrowserEnd, message: ipc::Received) -> Result<(), ipc::Violation>;

    /// The helper's end: from the more privileged browser, messages arrive as
    /// plain values. A message the helper can't decode is still a
    /// `Violation::Terminate`, which a helper's assembly answers by ending
    /// itself, since the browser it trusts sent something no build makes.
    pub trait HelperEnd { /* fn ping(&mut self, message: Ping) -> Result<(), ipc::Violation>; … */ }
    pub fn dispatch_to_helper(handler: &mut impl HelperEnd, message: ipc::Received) -> Result<(), ipc::Violation>;

    impl ipc::Message for ToHelper { /* INTERFACES, ids, encode into ipc::Encoder, the largest body */ }
    impl ipc::Message for ToBrowser { /* … */ }

    /// IP22: the decoder's fuzzing entry point, over bytes and handle kinds
    /// that the in-memory transport's fake handles stand for.
    pub fn fuzz_to_browser(id: u16, body: &[u8], handles: &[ipc::HandleKind]);
    /// IP22: a valid message for property tests, from the caller's entropy.
    pub fn generate_to_browser(entropy: &mut impl ipc::Entropy) -> ToBrowser;
}
```

- **Sending** is `ipc::Sender<M>::send(M)`, generic over the generated `ipc::Message` types ([`ipc`'s note](ipc-channels.md) §3.3). No generated function sends on its own.
- **Requests:** the requester's `send` returns an `ipc::RequestNumber`. The responder's handler method receives an `ipc::Responder<Reply>`, which `reply(self, reply)` consumes. The requester's handler receives the reply with its number. A responder dropped without replying is a bug, reported through `trace` as an error log; the requester's subsystem owns what a missing reply means.
- **Synchronous requests:** generated as `call_<name>(&mut sender, request) -> Result<Reply, ipc::CallError>` on the one direction the interface allows. The thread blocks in one receive on a send-once reply right, with no timeout of its own and nothing else handled (IP13). PF19's task age bounds it.
- **Generated code is ordinary safe Rust,** with no `unsafe` and no macros, so the unsafe-policy checker and Clippy see it as they see any crate (IP8, rule 7). `codegen --check` keeps it fresh on every commit (ES19).

### 3.7 The build identifier

`protocol::BUILD_PROTOCOL: [u8; 16]` is a 128-bit FNV-1a hash, computed by `codegen`, of every IDL file in its canonical form and of the generator's own version. Each executable builds its `BuildId` from it, from the revision the bundling step passes at compile time (`AXIOM_REVISION`, empty in development builds), and from its flavor: release, test or stress (CR13). The browser refuses a hello whose `BuildId` differs in any field (IP18).

The hash detects a helper and a browser built from different interfaces, by accident; it isn't a security boundary, since the browser launches only executables from its own bundle (IR9), and a compromised helper can lie in any field. That is why a non-cryptographic hash suffices (inference). The revision catches the rest of a mismatched bundle in release builds.

## 4. Invariants, threads and ownership

1. **One encoding per value:** for every message `m`, `decode(encode(m)) == m`, and for every accepted body `b`, `encode(decode(b)) == b`.
2. **Decoders are total:** every byte string and handle list either decodes or yields `Violation::Terminate`, without panicking, allocating past the message's bounds, or reading past its body.
3. **Every received handle has one owner** from the moment `platform-mach` receives it: the message, then the decoded field, then whoever takes it. A message that fails anywhere is destroyed whole (IP9; [`ipc`'s note](ipc-channels.md) §3.2).
4. **Dispatch is exhaustive:** adding a message to the IDL fails the build at every end that doesn't handle it (CR10).
5. **Generation is deterministic:** the same IDL gives the same bytes, so `codegen --check` compares them (ES19).
6. **Threads:** a generated decoder and handler run on the thread that owns the end (IP21, EL4). Generated types are `Send` when their fields are, so a message can move to another thread only as an owned value (EL11).

## 5. Errors and violations

- **A decode failure from a less privileged sender** is `Violation::Terminate` (IP11): the receiver ends the sender and logs the reason (IR3; OB17's violation report). The reason carries the interface, the message ID and the failed check, never the message's content (OB22).
- **A decode failure from a more privileged sender** is a fault in one build. The receiving helper ends itself with a panic record (OB9), so its report names the check.
- **A validator's verdict** on a decoded value is `ipc`'s (its note §3.4); the generated code never validates semantics, only shape.
- **The generator's errors** name the file, line, column and rule (§3.4), and fail `codegen --check` and so the commit.

## 6. Efficiency

- **Encoding** writes into a per-thread buffer that `ipc` reuses, so a steady stream of messages allocates nothing; a message with no variable-length field encodes in a straight line of stores.
- **Decoding** allocates only for variable-length fields (`string`, `bytes`, `array`), within their bounds. If a hot interface needs it, a borrowed decoding over the received body is an amendment, measured first.
- **Sizes, as first-tier tests** (ES23, PF5): each message's largest encoding is a generated constant, and tests assert the limits; the per-message overhead is Mach's header and trailer plus the request number.
- **The build's cost:** generated code is committed, so no build script runs (DP12), and `protocol` recompiles only when the IDL changes. Its compile time joins T1–T4 at M1-CP4 (PF18).

## 7. Tests and oracles

- **The generator:** unit tests that parse each declaration and print it back; one refusal test per static rule in §3.4, from seeded bad files, as the checkers' self-tests do.
- **The generated code:**
  - property tests from the generated generators: encode, decode, compare (invariant 1), on every message of every interface;
  - the canonical-form oracle in every fuzz target: for each accepted input, re-encode and compare bytes (invariant 1);
  - one fuzz target per decoder (TS9, IP22), in `fuzz/`, under AddressSanitizer, with M1-CP3's first target on `Control`;
  - Miri over the decoders' unit tests (TS11), since they run in every process;
  - lying-peer tests per interface facing a less privileged process (IP22): each check in §3.5 provoked, each ending in `Terminate` with the right reason.
- **Freshness:** `codegen --check` in the commit tier (ES19).
- **Sizes:** the generated largest encodings checked against the limits (PF5's first tier).

## 8. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **The Axiom IDL above,** parsed by `codegen` | A small grammar that states exactly what IP7 asks, nothing more: bounds, ends, roles, privilege, handles. The parser is a few hundred lines of safe Rust over the repository's own files, so it adds no dependency. **Recommended** |
| B | **Mojom's syntax, with Axiom's generator** | Familiar to anyone who knows Chromium. But Mojom's semantics (pending remotes and receivers, associated interfaces, nullable handles, `MinVersion` versioning) are ones IP3 and IP18 exclude, and borrowing the syntax would suggest them. Rejected |
| C | **Data files (TOML or JSON)** | No grammar to write, but `codegen` would need a parser dependency (DP12) or a hand-written subset, and nested structures and two-way message lists read worse than a declaration. Rejected |
| D | **Rust syntax with attributes, read by `syn` in `codegen`** | Reviewers read Rust, and editors help. But `syn` is a large build-time dependency (DP12), Rust's syntax invites types the wire can't carry (references, floats, unbounded `Vec`), and bounds become attribute noise. Rejected |
| E | **An existing schema format** (Protocol Buffers, Cap'n Proto, FlatBuffers) | Protocol Buffers has several encodings per value and keeps unknown fields, against IP9. Cap'n Proto and FlatBuffers read in place through offsets into the untrusted buffer, which their Rust runtimes do with `unsafe`, against rule 5. None models ends, roles or handles. Rejected |
| F | **Implicit IDs,** numbered by the generator | Shorter IDL, but a reordering silently renumbers messages in traces and reports. Explicit IDs cost a number per line. Rejected |
| G | **Ranges written on fields** (`width: u32 in 1..=16384`) | Concise, but the generated field would be a bare `u32` that a sender could fill out of range, so the encoder would need to check and fail. Named ranges make the newtype, so an out-of-range value can't be built (ES7). Rejected |

## Sources

- **Observed** (2026-10-23): `mach/message.h` in the Command Line Tools' SDK (`mach_msg_id_t`, `MACH_MSG_SIZE_RELIABLE`); `sysctl hw.pagesize` on this Mac (16384).
- The binding documents cited by rule: [IPC](../ipc.md) (IP1–IP22, its §3 precedent and §4.12 families), the [process model](../../security/process-model.md) (IR1–IR10), the [event loop](../event-loop-and-threading.md) (EL4, EL6–EL8), the [crate architecture](../crates.md) (CR8, CR10, CR13), the [engineering standards](../../technology/engineering-standards.md) (ES6, ES7, ES15, ES19, ES27).
