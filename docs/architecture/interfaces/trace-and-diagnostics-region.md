# Interface note: `trace`'s API and the diagnostics region

| | |
|---|---|
| **Status** | **Proposed** (2026-10-23), for the owner's approval at M1-CP3's gate, before the call sites, the regions and the collector are built (CR8, ES27). `trace`'s recorder in process exists since M1-CP2, its interface marked provisional until this note |
| **Interface** | How names and metrics are declared; how call sites record; where each thread's buffers come from (CR9); the diagnostics region's format and protocol between each helper and the browser; the browser's file-backed recorder; and where the one single-writer ring lives |
| **Crates** | `trace` (layer 0); `diagnostics` (the browser's collector); `ipc` (the region's memory, [its note](ipc-channels.md) §3.6); `event-loop` (threads register their slots, [its note](event-loop-tasks.md) §3.2) |
| **Settles** | `trace`'s charter's provisional interface (its item 4); [observability](../observability.md)'s open questions 1 and 3 in design, their values at M1-CP3's measurement |
| **Last updated** | 2026-10-23 |

## 1. Purpose and scope

[Observability](../observability.md) designed what is recorded and why (OB1–OB27); M1-CP2 built the record format, the ring and the recorder in one process. This note fixes the contract the rest of the engine records through, and the protocol by which helpers' records reach the browser:
- **names and metrics** declared once, in a table generated into every executable (§3.1; OB3, OB8);
- **call sites:** typed, cheap when disabled, unable to record content (§3.2; OB7, OB9, OB22);
- **where buffers come from:** a provider each assembly installs, and a slot per thread (§3.3; CR9, OB5);
- **the one ring** that `trace` and `ipc` share (§3.3; ES12), a decision for the owner;
- **the region:** its two memory entries, its layout, and the rules for each side's mapping (§3.4; OB10, OB11);
- **the protocol:** creation, notices, drains and death (§3.5; OB10–OB14);
- **the browser's file-backed recorder** (§3.6; OB16).

**Out of scope:** what the collector does with records once read, merged and kept: the flight recorder, reports, retention, exports and the health log are `diagnostics`' internals, designed in its charter (OB15–OB21); the categories and names each subsystem records, which come with each subsystem.

## 2. The binding rules it implements

| Rule | What it asks of this interface |
|---|---|
| OB1–OB4 | Four signals in one encoding; one timeline, identity stamped by the reader; records small, typed, named by IDs compiled into every executable, text only in bounded fields; flows keyed by what IPC already carries |
| OB5 | A buffer per thread, one writer; a fixed little-endian layout; no lock, allocation or system call; drops counted; `trace` declares where buffers come from (CR9) |
| OB6–OB9 | A static category list and a process-wide mask; levels with a compiled maximum; two tiers; metrics declared statically, per-thread copies summed on read; recording never changes behaviour; the panic hook's one record |
| OB10–OB14 | One region per helper, created by the browser, sent in the hello's answer; the helper maps it read-write and the browser read-only; one ring per thread; the reader's positions in a separate page the helper only reads; the reader trusts nothing; writers never wait; a watermark notice without importance; drains without polling; a dead helper's region read once, then unmapped within 1 s |
| OB16, OB22, OB24, OB25 | The browser's recorder file-backed; no content in any record; the regions a fuzzed, panic-free attack surface; minimal cost, sized from measurement |
| IR1, IR3, IR7, IR10, IP14 | Untrusted input; violations terminate and are logged; one writer per shared memory, the reader copying before validating; nothing from a dead child reaches the browser's state; rings with untrusted indices |
| ES12 | Cross-thread primitives implemented once |

## 3. The design

### 3.1 Names and metrics, declared once

**The table** is a data file, `crates/trace/names.txt`, one declaration per line:

```text
# category  name              kind       tier      fields, or unit
task        task              span       always    source
nav         nav.commit        event      always    frame
ipc         ipc.flow          flow       always
ipc         ipc.violation     log        always
layout      layout.block      span       detailed  boxes
health      health.drops      counter    always    records
health      health.footprint  gauge      always    bytes
task        task.duration     histogram  always    nanoseconds
```

`codegen` reads it and writes `crates/trace/src/generated/names.rs` (ES19): one typed constant per name, and the table that maps each ID to its category, kind, tier, text and fields, which the collector and the exports read. Every executable of one build carries the same table, so the browser decodes a helper's IDs with its own copy and no name crosses a process as a string (OB3, IP18).

```rust
// Generated.
pub const TASK: SpanName<Always> = /* category Task, name 1 */;
pub const NAV_COMMIT: EventName<Always> = /* category Nav, name 2 */;
pub const LAYOUT_BLOCK: SpanName<Detailed> = /* category Layout, name 5 */;
pub const IPC_FLOW: FlowName = /* … */;
pub const IPC_VIOLATION: LogName = /* … */;
pub const HEALTH_DROPS: Counter = /* metric 0 */;
pub const HEALTH_FOOTPRINT: Gauge = /* metric 1 */;
pub const TASK_DURATION: Histogram = /* histogram 0 */;
pub static TABLE: [NameInfo; 8] = [ /* … */ ];
```

- **Kinds are types:** a counter can't be given a span's name, nor a span a counter's. **Tiers are type parameters:** a detailed name's call site checks the mask; an always-on name's doesn't, and the check compiles away (OB7).
- **Categories stay an enum** in `trace` (OB6); a name's category is part of its declaration.
- **The table is one source of truth** (CR10): adding a name is a line, regenerated; removing one leaves no stale ID, because the build fails at its call sites.

### 3.2 Call sites

```rust
// An always-on span: the guard records its end when dropped.
let _task = names::TASK.enter([task.source() as u64]);

// A detailed span: fields computed only when the category is on (OB7, OB9).
let _block = names::LAYOUT_BLOCK.enter_with(|| [count_boxes(&tree)]);

// An event, a flow, a log.
names::NAV_COMMIT.event([frame.id()]);
names::IPC_FLOW.begin(flow_key(channel, sequence));    // OB4
names::IPC_VIOLATION.log(Level::Warn, "decode failed", [interface, message]);

// Metrics: the thread's own copy, no record (OB8).
names::HEALTH_DROPS.add(n);
names::HEALTH_FOOTPRINT.set(bytes);
names::TASK_DURATION.record(nanos);
```

- **Spans** are guards (`#[must_use]`), so a span always ends on its thread, in order, even when an error returns early. Under `panic = "abort"` a panicking thread records the panic instead (OB9).
- **Logs take a `&'static str`** description and up to four integer fields, not a formatted string. So a log can't carry page content by accident (OB22): text from Axiom's own code is static, and values are fields. A panic's message is the one dynamic text, bounded to 255 bytes (OB3, OB9). **Origins,** which OB3 also allows, join at M1-CP4 with a typed field fed only by `url`'s reduced form (OB22), as an amendment here.
- **Metrics** update the calling thread's metrics block (§3.4): a counter adds to its slot, a gauge stores, a histogram adds one to a bucket (OB8's log-linear buckets, built at M1-CP2). One writer per block, so an update is a relaxed load and store, with no atomic read-modify-write. The collector sums threads' blocks when it drains (OB8: "a read sums them").
- **A thread without a slot records nothing,** at the cost of its thread-local check (OB5), as M1-CP2 built it.

### 3.3 Where buffers come from, and the one ring

```rust
/// A thread's slot: its ring and its metrics block, in memory its provider owns.
pub struct Slot<'a> { /* ring storage, metrics block, slot header */ }

/// What an assembly installs once, before its threads start (CR9, OB5).
pub trait Buffers: Sync + 'static {
    /// A slot for a thread of `role`, or `None` when the process has no room left.
    fn slot(&self, role: Role) -> Option<Slot<'static>>;
}

pub fn set_buffers(buffers: &'static dyn Buffers) -> Result<(), BuffersAlreadySet>;

/// Called by `event-loop`'s `spawn` for each thread it starts: takes a slot and
/// installs it in the thread's recorder.
pub fn register(role: Role) -> Option<SlotId>;

/// The thread roles (EL6–EL8), recorded once per slot, not per record (OB2).
pub enum Role { Main, Core, Compositor, Worker, EventLoop, Control, Helper, Io, Storage }
```

- **The providers:** in a helper, `ipc`'s region provider over the mapped region (§3.4); in the browser, `diagnostics`' file-backed provider (§3.6); in tests, a provider of `OwnedRing`s, as M1-CP2's tests use. Each process installs its clock (`set_clock`, OB2), then its provider, then starts its threads.
- **The one ring** (ES12): M1-CP2 built a single-producer, single-consumer ring of words in `trace`. `ipc`'s shared-memory streams, the display list's from M1-CP4, need the same structure with larger frames, and ES12 asks for each primitive "implemented once". `trace` sits at the foundation and can't depend on `ipc`. **Recommended:** move the ring into a new foundation crate, `ring`, with no dependency, no `unsafe` and the `panic-free` class, used by both `trace` and `ipc`, and parameterized by its largest frame (64 words for `trace`'s records). That is a new entry in the crate map (CR2) and a change to ES12's wording, "the single-writer ring in `ring`, used by `ipc` and `trace`". **The alternative:** leave the ring in `trace`, and let `ipc`'s streams use `trace::ring`: no new crate, but the display list would ride a module named for recording.
- **Read-only sides:** where a side maps memory read-only, its loads are relaxed, followed by an acquire fence where order is needed, because Rust defines no other atomic access to read-only memory ([`ipc`'s note](ipc-channels.md) §3.6). M1-CP2's ring loads positions with `Acquire`; at M1-CP3 those loads become a relaxed load and a fence on the sides that map read-only: the writer's load of the reader's position, and the reader's of the writer's. Ordering is unchanged.

### 3.4 The region

**Two memory entries per helper,** created by the browser at launch and sent in `Control::welcome` ([the IDL's note](protocol-idl.md) §3.4):

| Entry | The helper maps it | The browser maps it | Holds |
|---|---|---|---|
| **The data region** | Read-write | Read-only | A header page, then one slot per thread: its write position, its metrics block, its ring |
| **The reader page** | Read-only (`MemoryEntry::read_only`) | Read-write | The browser's read position for each slot |

**The data region's layout,** a whole number of 16 KiB pages:

| Part | Size | Contents | Who trusts it |
|---|---|---|---|
| Header | One page | A magic (`AXTRACE1`), the layout's version, the slot count, each slot's size and its ring's length, the metric counts | Only the helper, as a check that its constants match the layout it was sent; the browser keeps its own copy and never reads layout from the region |
| Slot header, per slot | 128 bytes (EL13) | The role and instance the thread registered under; the write position | Read by the browser as claims: a role the process type doesn't have, or an instance past the cap, is a violation |
| Metrics block, per slot | 8 bytes per counter and gauge, 2,016 bytes per histogram (252 buckets) | The thread's own metric copies (§3.2) | Read by the browser as claims: a counter or bucket that decreases is a violation |
| Ring, per slot | A power of two of words, at least 128 | Records, in M1-CP2's format | Each record copied out and decoded totally before use |

**`RegionLayout`,** sent with the entries: the slot count, the ring's length in words, and the counts of counters, gauges and histograms, which in one build equal the helper's generated table (IP18); a helper whose constants differ ends itself.

**Sizes** (hypotheses, set at M1-CP3 by OB25's rule, "the smallest that holds one drain interval of the always-on tier"): a renderer with 8 slots of 2,048-word rings (16 KiB each) and M1's metrics comes to about 180 KiB, within OB25's 256 KiB per helper, counted in B2 and B4 as S16 found shared memory charged.

**What each side may do** (IR7):
- **The helper** writes its slots' headers, metrics and rings, and only reads the reader page: one writer per word.
- **The browser** writes only the reader page. It keeps every read position in its own memory and copies it to the page after each drain, so a helper that could write the page anyway (if the read-only entry's hypothesis fails, [`ipc`'s note](ipc-channels.md) §3.6) would confuse only itself.

### 3.5 The region's protocol

1. **Launch** (OB10): the browser creates both entries and maps them; the helper applies its profile and says hello, recording nothing yet; the browser answers with `welcome(region, reader, layout)`, and its first endpoints.
2. **Installation:** the helper maps both entries, installs `ipc`'s region provider, and starts its threads; each takes the next slot under its role (§3.3). Slots aren't reused: a helper's threads are few and long-lived (EL7, EL8), and the slot count bounds them.
3. **Recording** (OB5): as M1-CP2 built it, into the slot's ring and metrics. A full ring drops and counts.
4. **The watermark** (OB12): when a ring passes half full, its thread asks the control thread, which owns the helper's end of `Control` (IP21), to send `trace_watermark(slot)`: one-way, without importance, zero timeout, skipped if the browser's queue is full. At most once until the reader's position next advances.
5. **Draining** (OB13): the collector drains a helper's slots on a watermark notice, when a report needs them, at each pong while the watchdog is active ([the event loop's note](event-loop-tasks.md) §3.5), and when the helper exits. It has no timer, so it drains nothing while idle (PF16).
6. **Reading** (OB11): per slot, the collector loads the write position (relaxed, then a fence) and checks that it hasn't gone backwards, and that `write − read` is within the ring's length. It copies the words in between into its own buffer, and decodes each record with `trace::decode`, which is total. It stamps the producer's identity itself (OB2) and copies the read position to the reader page. A position, record, role or metric that no correct writer produces is a violation: IR3 ends the helper and logs why (OB17). Dropped records are counted, never violations. No diagnostic value is used for a security or recovery decision (IR2).
7. **Detail** (OB7): the browser's `detail(mask)` sets the helper's process-wide mask, per category.
8. **Death** (OB14): the browser drains the region once, unmaps both entries within 1 s (IR10's release window, RV4-04), and passes those last records only to the report.

### 3.6 The browser's file-backed recorder

The browser records into a file, `<data directory>/diagnostics/recorder`, mapped shared and read-write, with the data region's layout: a header page, then one slot per browser thread. The header adds a session state, *running* or *clean*, set to clean at an orderly quit.
- **At launch,** a recorder left *running* means the last session ended abruptly: `diagnostics` turns it into a report (OB17), then resets it.
- **Its size** is at most 2 MiB (OB16's hypothesis), within OB25's 4 MiB for the collector and the recorder, and counted against PF27's disk budget.
- **No idle writes** (PF28): the kernel writes back only dirtied pages, and nothing is recorded while idle.
- **The mapping** of a file is a virtual-memory facility, so it joins `platform-mach`'s mappings, beside memory entries. Two edits to the crate map follow (CR2): the facility in `platform-mach`'s entry, and `platform-mach` among `diagnostics`' allowed dependencies, which today list `trace`, `ipc`, `protocol`, `platform-process`, `event-loop` and `url`.

## 4. Invariants, threads and ownership

1. **One writer per word:** a slot's header, metrics and ring are written only by its thread; the reader page only by the browser (IR7, OB5).
2. **The reader trusts nothing it reads from a region** and keeps its own positions privately (OB11).
3. **Recording never waits, allocates or makes a system call;** a notice is one zero-timeout send from the control thread (OB9, OB12).
4. **Every executable of one build carries the same table,** so IDs mean the same in every process (OB3).
5. **No call site can record a page's content:** logs take static text and integer fields (OB22).
6. **Read-only mappings are only loaded, relaxed** (§3.3).
7. **Threads:** a slot belongs to the thread that registered it, for that thread's life; the collector reads on the browser's worker pool (OB15).

## 5. Errors and violations

| Situation | Result |
|---|---|
| A write position past the ring's length beyond the reader, or one that went backwards | Violation: the helper is ended and the reason logged (IR3) |
| A record `trace::decode` refuses | Violation |
| A slot header with a role the process type lacks, or an instance past the cap | Violation |
| A counter or histogram bucket that decreases | Violation |
| Dropped records | Counted, reported in the health log (OB20); not a violation |
| A helper's constants that differ from the layout it was sent | The helper ends itself (one build, IP18) |
| No slot left for a thread | It records nothing; `register` returns `None`, which `spawn` logs |
| A recorder file that can't be created or mapped | The browser records into memory only, and logs it; no report survives a crash that session |

## 6. Efficiency

- **Records:** as M1-CP2 measured, a span record 130.5 instructions and 28 cycles, a disabled detailed check one instruction (the plan's §9), under OB25's 50 ns and 2 ns.
- **Metrics:** a thread-local lookup and a relaxed load and store, a few instructions (a hypothesis, measured at M1-CP3 beside the records).
- **Memory:** about 180 KiB per renderer region (§3.4), at most 2 MiB for the browser's recorder, both set at M1-CP3 from measured record rates (OB25).
- **Idle:** no timer and no notice while idle; drains ride the watchdog's active cycle (PF16, OB13).

## 7. Tests and oracles

- **The lying producer** (OB24): a helper of the test flavor that writes garbage words, oversized records, a flood, forged and backwards positions, forged roles and instances, and decreasing counters. Each must end in a dropped record or an ended helper, never a stalled or crashed browser.
- **A fuzz target** for the region reader, over arbitrary bytes standing for a region (TS9).
- **loom** on the ring with the relaxed-load-and-fence orderings, writer against reader (TS11).
- **Reports** (OB27): a helper's panic leaves a report with its message and last spans; a stalled thread is named in the hang report; a violation's log names its reason; a killed browser leaves a recorder that the next launch reports.
- **The table:** `codegen --check` keeps `names.rs` fresh (ES19); a test decodes every generated name through the table.
- **Costs:** M1-CP2's cost test extended to metrics; no allocation while recording, as M1-CP2 tests (ES23).
- **Determinism** (OB26): on the virtual clock, a replayed schedule's trace is identical, compared as a golden file.

## 8. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Typed name constants generated from one table** | IDs agree in every process of a build; kinds and tiers checked by the compiler; one line to add a name. **Recommended** |
| B | **Names as strings at call sites, interned at run time** | Strings would cross processes, or IDs would be assigned per process and need a mapping (OB3). Rejected |
| C | **A declaration macro per crate** | Each crate's IDs would need a registry to stay unique across crates, at run time or by convention. Rejected |
| D | **Metrics as records** | About 100 instructions per update instead of a few, and a ring's space spent on numbers that only their sums need. OB8's per-thread copies. Rejected |
| E | **One ring per process,** shared by its threads | Writers would need a lock or atomic read-modify-writes, against OB5. Rejected |
| F | **The reader's positions inside the data region** | The helper could rewrite them; OB10 puts them in a page the helper only reads, and the browser keeps its own. Rejected |
| G | **Logs with formatted strings** | Convenient, but the easiest way for content to reach a record (OB22). Static text and fields. Rejected |
| H | **Two ring implementations,** one in `trace` and one in `ipc` | Simplest now, against ES12's "implemented once": two primitives for loom to check and two to keep correct. Rejected, as §3.3 proposes |

## Sources

- `crates/trace` as built at M1-CP2 (`4fb2a92`): its charter, record format, ring, recorder and cost test.
- The binding documents cited by rule: [observability](../observability.md) (OB1–OB27, open questions 1 and 3), the [process model](../../security/process-model.md) (IR1–IR10), [IPC](../ipc.md) (IP14, IP18, IP20, IP21), the [event loop](../event-loop-and-threading.md) (EL6–EL8, EL13), the [performance budgets](../../performance.md) (PF16, PF27, PF28), the [engineering standards](../../technology/engineering-standards.md) (ES12, ES19, ES23).
- [S16 results](../../research/s16-results.md) §4: shared memory charged to the reader that touches it, and kept resident after its writer dies until unmapped.
