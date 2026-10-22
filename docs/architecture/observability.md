# Observability and Diagnostics

| | |
|---|---|
| **Status** | **Accepted** (2026-10-23), with [ADR-0023](../adr/0023-observability-and-diagnostics.md): "Accepted as proposed (Decisions D1 through D10)." Written at Milestone 1's first checkpoint. The owner, on the first draft of the plan for this work: "I think we should implement a robust observability mechanism that will help us monitor the system effectively in the long run." The owner chose the live diagnostics window "M2, with crash reports (Recommended)", with the whole mechanism built at M1. **At acceptance,** the owner directed "minimal performance overhead", which OB25 carries, and settled open question 5 by amending PF19. Implementation begins at M1-CP2 (§11) |
| **Question** | How does Axiom see what it does and what it costs, across its processes, in development and in daily use? How does it explain a crash, a hang, an IPC violation or a budget breach after the fact? How does it keep a long-term record of its health, within its budgets and its security model? |
| **Builds on** | The [performance budgets](../performance.md) (PF2, PF5, PF13, PF14, PF16, PF19, PF27–PF30); the [process model](../security/process-model.md)'s IR1, IR3, IR7 and IR10 and its budgets B1–B4; [IPC](ipc.md) IP4, IP11, IP14, IP16, IP18 and IP20; the [event loop](event-loop-and-threading.md)'s EL1, EL23 and EL24; the [testing strategy](../testing.md) (TS8, TS9, TS14); the [roadmap](../roadmap.md)'s M2 (crash reports, RV1-15); [Milestone 1](../milestone-1.md)'s MA1 (the network log) and MA6 (trace spans); the [threat model](../security/threat-model.md) §5; the [crate architecture](crates.md) (`trace`, `diagnostics`); [S16's results](../research/s16-results.md) |
| **Feeds** | ADR-0023; [Milestone 1's plan](../milestone-1-plan.md) (M1-CP2, CP3, CP4, CP9, CP11, CP12, CP14); M2's plan (the window, the system's crash reports); the [engineering standards](../technology/engineering-standards.md)' ES14 |
| **Last updated** | 2026-10-23: accepted; OB25 carries the owner's direction on overhead; open question 5 settled by PF19's amendment |

Rules are numbered OB1–OB27. Everything here is **planned**. Precedent is sourced (§2), and the design is inference from it and from the binding rules, unless marked otherwise. Values marked **hypothesis** are measured at the checkpoint named.

## Summary

- **Several rules already assume an observability system that no document designs:**
  - MA6, PF13 and PF14 measure with "trace spans in the engine";
  - IR3 and IP11 say the browser "kills it and logs why";
  - PF19 watches every thread's task age;
  - PF5's harness reads counters;
  - M2 keeps "a crashed helper's report… in a local directory the owner controls";
  - MA1 dumps a network log.

  This document designs that system once, so no feature grows its own logging.
- **Four signals in one record model** (OB1–OB4): spans and events, metrics, logs, and reports.
  - Every record is small, typed and named by an ID compiled into the build.
  - Records carry timestamps from one system-wide clock, so the processes merge onto one timeline.
  - A flow follows each IPC message from its sender to its receiver.
- **Recording in every process** (`trace`, OB5–OB9):
  - per-thread buffers with one writer each;
  - no lock, allocation or system call on the recording path;
  - an always-on tier that costs about nothing, and a detailed tier switched on by category.
- **Helpers write into a shared-memory region the browser creates** (OB10–OB14), as Perfetto's producers do. The browser treats every record as untrusted input, a full region drops records and counts them rather than blocking, and the browser reads a dead helper's last records for its report.
- **The browser's collector** (`diagnostics`, OB15–OB20):
  - merges the streams and keeps an in-memory flight recorder of the last seconds of every process;
  - writes a report on each crash, hang, violation or budget breach;
  - keeps a daily health log for 90 days, within PF27's disk budget and with no writes while idle (PF28).
- **Exports** (OB21): traces in Chrome's JSON trace format, which Perfetto's UI opens; metrics for PF5's harness; text dumps for tests. The native diagnostics window arrives at M2, as the owner chose.
- **Privacy and security** (OB22–OB24): nothing leaves the Mac; no record holds page content; URLs are reduced to their origin unless the owner turns on detailed mode; sensitive sites are never recorded in detail. The regions are a new attack surface into the browser, fuzzed and tested with a lying producer.
- **Budgets** (OB25, hypotheses): a disabled span ≤ 2 ns, an enabled one ≤ 50 ns; the always-on tier ≤ 1% of a process's CPU; regions ≤ 256 KiB per helper; zero idle wakeups and writes.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **MA6, PF13, PF14:** the rendering update and long tasks measured by "trace spans in the engine" | OB1, OB7 |
| **PF19:** each watched thread records when its current task began, "one atomic store per task", and the control thread's reply carries the oldest age | OB7: the task record and the age share that store |
| **IR3, IP11:** a violation terminates the sender "and is logged"; **IR10:** when a child dies, the browser "discards everything that child reported and recovers from its own state" | OB11, OB14, OB17 |
| **IR7, IP14:** shared memory has one writer; the reader copies before validating and treats a ring's indices as untrusted. **IR1:** every message from a less privileged process is untrusted input | OB10, OB11 |
| **EL1, IP16:** no thread waits on a send; full queues are a backlog, never a block. **IP20:** bulk traffic goes without importance | OB12 |
| **PF16:** no idle wakeups; **PF28:** "no writes beyond session state" while idle; **PF29:** no network request of Axiom's own, "no telemetry"; **PF30:** rebuildable caches released under memory pressure | OB13, OB16, OB18, OB23 |
| **PF27:** "Crash reports and logs: the newest 20 reports and 10 MB of logs" | OB18 |
| **PF2:** `proc_pid_rusage` at `RUSAGE_INFO_V6`; **PF5:** counts on every commit, a ratchet at every checkpoint | OB19, OB21, OB27 |
| **The roadmap's M2** (RV1-15): crash reports kept locally with the build identifier; the console at M3. **MA1:** the network log among the headless embedder's text dumps | OB17, OB21 |
| **EL24, TS8:** virtual time in single-process tests; **TS9:** every decoder fuzzed; **TS14's V2:** lying peers | OB26, OB27 |
| **B1 (B1a from M2), B2, B4:** the browser's, a renderer's and the network process's memory | OB25 |
| **The threat model §5:** every input from a less trusted process is an attack surface | OB24 |

## 2. Precedent and observations

All read on 2026-10-22.

- **Perfetto,** the tracing system of Chrome and Android:
  - **The security model** [P1]: "Producers are never trusted. We assume they will try their best to DoS / crash / exploit the tracing service." "Memory is shared only point-to-point between each producer and the tracing service." "We should never ever share memory across producers." The service rejects packets that forge its own fields, and "appends the POSIX uid of the producer to each `TracePacket`".
  - **Buffers** [P2]: each producer's shared memory buffer is "a temporary staging buffer". Overflow loses data unless a producer opts into stalling. A central buffer either overwrites the oldest data (`RING_BUFFER`) or drops the newest (`DISCARD`), and both losses are counted.
  - **Formats** [P3]: "Chrome JSON files can be opened directly in the Perfetto UI", and so can two variants of the Firefox Profiler's format, which samply writes. "Both the UI and Trace Processor open archives directly and merge the traces inside onto a single timeline."
  - **The UI's page** [P4] says it lets you "view and analyze traces in the browser", but doesn't say whether a file opened there stays on the machine (OB23).
- **Chromium's persistent histograms** [C1]: an allocator over "a space that can be persisted or shared between processes", which can be created "by memory-mapping a `file`" "in order to persist the data for a later use", including "self-contained metrics coming from a previous session".
- **Servo** [S1]: tracing works by "instrumenting specific functions (or portions of code) with explicit annotations"; the build adds `--features tracing tracing-perfetto`. At run time, `SERVO_TRACING` takes `tracing-subscriber`'s filter directives and is off by default. A run writes `servo.pftrace`, which is opened in Perfetto's UI.
- **S16** (measured on this Mac; [S16 results](../research/s16-results.md)):
  - memory shared by a process that died "stays resident, charged to no process", until the reader unmaps it;
  - a read by the CPU charges it to the reader;
  - instructions retired vary 0.03–0.15% from run to run, against 1.1–37% for wall time.
- **macOS's own facilities** (inference, from the process model and the charter):
  - **`os_log`** would need each helper to reach the logging service, but a helper's profile lets it look up only "the single rendezvous name its profile allows" (IP4);
  - **`os_signpost`** intervals are read with Instruments, which needs Xcode (the charter §7).

## 3. Goals and non-goals

**Goals:**
1. **Visibility:** what each process and thread is doing and what it costs, at every layer and across processes, in development and in daily use.
2. **Diagnosis after the fact:** every crash, hang, IPC violation and budget breach explained, with the seconds before it.
3. **A long-term record:** health and efficiency over the project's life, day by day.
4. **Within the budgets and the security model:** idle costs nothing, untrusted processes can't harm the browser through it, and nothing leaves the Mac.

**Non-goals:** telemetry of any kind (PF29); a general metrics service; recording page content; replacing samply, which stays the sampling CPU profiler that traces complement.

## 4. The model

| # | Rule |
|---|---|
| OB1 | **Four signals, one record model:**<br>• **Spans and events:** a span marks the start and end of work on a thread, and nests. An event marks a moment. Both carry a category, a name and up to a few typed fields.<br>• **Metrics:** counters, gauges and histograms (OB8).<br>• **Logs:** a severity, a name and typed fields, for what must be seen without tracing (OB7).<br>• **Reports:** a record assembled by the browser around a crash, hang, violation or budget breach, with the flight recorder's last seconds of each process involved (OB17).<br>All four share one encoding, so one collector, one flight recorder and one export serve them all |
| OB2 | **One timeline.** Timestamps come from the system-wide monotonic clock (`mach_continuous_time`), so records from every process merge without conversion. In single-process tests they come from EL24's virtual clock instead (OB26). **Identity is stamped by the reader:** the collector knows which region belongs to which process, as Perfetto's service appends each producer's uid (§2), so a helper can't claim another's records. Each record names its thread's role (EL's list), not an operating-system thread ID |
| OB3 | **Records are small, typed and named by ID.** Categories and names are IDs into a table compiled into every executable. Since every process comes from one build (IP18), the browser decodes a helper's IDs with its own copy, and no name crosses a process as a string. Fields are integers, durations, IDs and enumerations. **Text appears only in bounded fields:** a panic's message, an error's description from Axiom's own code, and an origin (OB22). Each has a length limit and is validated as UTF-8 before it is shown |
| OB4 | **Flows follow messages across processes.** A send records a flow's start, keyed by the channel and the message's sequence number, and the receipt records its end. So a navigation can be followed from the shell through the browser's core, the network process and the renderer to the compositor, on one timeline. The key comes from what the IPC layer already carries; nothing is added to the wire for tracing |

## 5. Recording: `trace`

| # | Rule |
|---|---|
| OB5 | **The recorder** (`trace`, the foundation layer; [crates](crates.md) §4.1):<br>• every thread that records registers once and receives its own buffer, with one writer: its thread;<br>• a record is a fixed little-endian layout, as IP9's wire format is;<br>• writing one takes **no lock, no allocation and no system call:** it reads the clock, copies the bytes and publishes an index with a release store;<br>• a full buffer drops the record and counts the drop, and the count travels with the next record that fits.<br>`trace` declares where buffers come from (CR9): in a helper, slices of the shared region (OB10); in the browser, the file-backed recorder (OB16); in unit tests, ordinary memory |
| OB6 | **Categories and levels.** Categories are a static list, one per subsystem and concern: `task`, `ipc`, `nav`, `load`, `parse`, `style`, `layout`, `paint`, `composite`, `net`, `gc`, `text`, `image`, `health`. A process-wide mask turns each on or off with one relaxed atomic load at the call site. Logs carry the levels error, warn, info and debug, with a maximum compiled per build profile. **The collector, the flight recorder and the exports treat categories generically,** so adding one is a declaration and its call sites |
| OB7 | **Two tiers:**<br>• **Always on:** each task's start and end, written by the same store that PF19 reads for the task's age; each lifecycle phase's start and end with its counts (elements styled, fragments built, items painted); IPC counts; every log at info and above; metrics.<br>• **Detailed:** fine spans with fields (selectors matched, runs shaped, tiles drawn), switched on per category and per process by the browser over the control channel, or at launch in tests and in the headless embedder.<br>MA6's measurements of the rendering update and long tasks come from the always-on tier |
| OB8 | **Metrics are declared statically,** with a name, a unit and a kind:<br>• **counters,** which only grow;<br>• **gauges,** which hold a level;<br>• **histograms,** with fixed log-linear buckets.<br>Each thread updates its own copy without synchronization, and a read sums them. Metrics feed the always-on tier's records, PF5's harness (OB21) and the health log (OB20) |
| OB9 | **Recording never changes behaviour.** It never blocks, never allocates and never runs code the recorded operation didn't ask for. A disabled category costs its mask check. **The panic hook** writes one bounded record, the message and the location, into the thread's buffer, and the process then aborts as RV1-16 requires. The message is formatted into a fixed buffer, so the hook doesn't allocate |

## 6. Transport: helpers to the browser

| # | Rule |
|---|---|
| OB10 | **One region per helper, created by the browser.** The browser creates a memory entry for each helper it launches and sends it in its answer to the helper's hello (IP4), with the helper's first endpoints. What the helper does before that, applying its profile and saying hello, isn't recorded in the region; a failure there is a launch failure, which the browser sees directly. The helper maps the region read-write and the browser read-only. The region is a header and one ring per registered thread. **Each ring has one writer, its thread** (IR7), and an index pair: the writer's position, and the reader's position, which the browser publishes in a separate page the helper only reads. Its size is a hypothesis, 256 KiB per helper (OB25), set at M1-CP3 |
| OB11 | **The reader trusts nothing in a region** (IR1, IR7, IP14):<br>• it copies each record out before interpreting it;<br>• it checks the ring's indices against the region's bounds, and every length, ID, enumeration and field against its type;<br>• it stamps the producer's identity itself (OB2).<br>A record that no correct writer could produce is a violation: IR3 terminates the helper and logs why, as for a malformed message. Dropped records aren't violations; their counts are reported. No diagnostic value is ever used for a security or recovery decision (IR2) |
| OB12 | **Writers never wait.** A full ring drops and counts (OB5). When a ring passes a watermark (a hypothesis: half full), its writer sends the browser one notice: a one-way message without importance (IP20), with a zero timeout, at most once until the next drain. If the browser's queue is full, the notice is skipped (EL1, IP16) |
| OB13 | **Draining without polling.** The collector drains a region:<br>• when a notice arrives;<br>• when a report needs it (OB17);<br>• on the watchdog's existing ping cycle while the helper is busy (PF19);<br>• when the helper exits.<br>It has no timer of its own, and drains nothing while the system is idle, when nothing is recorded (PF16) |
| OB14 | **After a helper dies,** the browser drains its region once, then unmaps it within IR10's release window: "nothing mapped 1 s later" (MA7, RV4-04). Those last records are the post-mortem. **They only feed the report:** nothing in them reaches the browser's state, which IR10 rebuilds from the browser's own records |

## 7. Collection, reports and retention: `diagnostics`

| # | Rule |
|---|---|
| OB15 | **The collector** (`diagnostics`, in the browser) runs on the browser's worker pool at background QoS (EL23), so it adds no thread role. It merges the streams by timestamp. The browser's own threads write into their buffers like any process, and the collector reads them without validation, since they are the browser's own |
| OB16 | **The flight recorder:** each process's most recent records, kept in memory, up to the last 30 s or 1 MiB a helper and 2 MiB for the browser (hypotheses, M1-CP3).<br>• **Under memory pressure** it shrinks to the last few seconds (PF30).<br>• **The browser's own recorder is a file-backed mapping** in its data directory, as Chromium's persistent histograms can be (§2). So it survives the browser's crash, and the next launch turns it into a report. The kernel writes its pages back only as they are dirtied, and nothing is recorded while idle, so it adds no idle writes (PF28); its size counts against PF27 |
| OB17 | **Reports,** each with the flight recorder's records from every process involved:<br>• **A crash:** the helper's panic record (OB9) and its last records (OB14). From M2, the system's crash report is moved in beside it, with the build identifier, as the roadmap's M2 says (RV1-15).<br>• **A hang:** at PF19's deadline, the stuck thread's open spans, with the task's age and the recent events. So a hang names what was running, not only that something was.<br>• **A violation:** IR3's "logged" made concrete: the interface, the message's ID, the validator's reason, and the sender's process type and site lock. No message content is kept beyond OB3's bounded fields.<br>• **A budget breach** (OB19): the process's footprint against its budget, with its subsystems' counters |
| OB18 | **Retention, within PF27:**<br>• **files:** in the browser's data directory, under `diagnostics/`;<br>• **reports:** the newest 20;<br>• **logs,** the daily health log included: 10 MB, deleted oldest first;<br>• **when it writes:** when a report is made, when a day's record closes (at the first active moment after midnight), at quit, and when the owner asks.<br>Never while idle (PF28) |
| OB19 | **The health monitor.** During an active session, the browser samples each child's footprint, CPU time and energy with `proc_pid_rusage` (PF2), on the watchdog's cycle, which stops when everything is idle (PF19, as amended at acceptance). A helper never reports its own health, so a compromised helper can't hide its cost. The samples are compared with each process's budget (B1–B7) and each corpus page's ceiling ([performance](../performance.md) §4). A breach is a log entry and a report |
| OB20 | **The daily health log:** for each process type and day:<br>• peak footprint, CPU time and energy;<br>• crashes, hangs and violations;<br>• long tasks (PF14) and missed frames (PF13);<br>• budget breaches;<br>• drops.<br>Kept 90 days, within OB18's 10 MB. `tools/health-report.py` (planned) reads it for trends, so a slow growth in memory or wakeups shows up over weeks, which no single gate measures |

## 8. Access

| # | Rule |
|---|---|
| OB21 | **Exports and views:**<br>• **traces** in Chrome's JSON trace format, which Perfetto's UI opens directly (§2), with every process on one timeline;<br>• **metrics** as TOML, the format of `perf/baseline.toml`, which PF5's harness reads;<br>• **logs** as JSON lines;<br>• **text dumps** for tests, in stable formats: MA1's network log is the `net` category rendered as text;<br>• **the headless embedder** takes flags to enable categories and to save a trace with its run;<br>• **the shell** gets a command that saves the flight recorder as a trace (M1-CP12);<br>• **`tools/trace-summary.py`** (planned) summarizes a trace locally, as `tools/samply-summary.py` does a profile;<br>• **the native diagnostics window at M2:** each process's memory, CPU and energy against its budget, recent reports, categories to enable, and saving a trace (the owner's choice, 2026-10-22: "M2, with crash reports (Recommended)") |

## 9. Privacy and security

| # | Rule |
|---|---|
| OB22 | **Content is never recorded.** The record format has no field for page content, form values, cookies or credentials. **A URL is reduced to its origin** unless the owner turns on detailed mode for a session, from the shell or a headless flag. **Sites the owner marks sensitive** (L3) are never recorded in detail, whatever the mode: their renderers' detailed categories stay off, and their origins aren't recorded |
| OB23 | **Local only.** Nothing is sent anywhere, ever (PF29). Exports are files the owner controls. Whether a web-based viewer keeps an opened file on the machine is the viewer's property: Perfetto's page read on 2026-10-22 doesn't say (§2). So the owner chooses where to open a trace, and `tools/trace-summary.py` reads one locally |
| OB24 | **The regions are a new attack surface:** input from less trusted processes into the browser. Their decoder is generated or written in safe Rust (rule 5) and is `panic-free` (ES10). It gets a fuzz target (TS9) and a lying producer's test on V2's model: garbage, oversized records, a flood, forged indices and forged identities, each of which must end in a dropped record or a terminated helper, never a stalled or crashed browser. On ADR-0023's acceptance, the threat model's §5 gains the row |

## 10. Budgets, determinism and tests

| # | Rule |
|---|---|
| OB25 | **Minimal by design** (the owner, at ADR-0023's acceptance, 2026-10-23: "we should ensure that the observability architecture introduces minimal performance overhead and consumes as little CPU and memory as possible"):<br>• **the values below are ceilings, not targets:** each is measured, reported against its ceiling, and reduced where a cheaper design meets the same goal;<br>• **the always-on tier holds only what a named consumer needs:** PF19's task ages, MA6's and PF13's measurements, OB17's reports, OB20's health log and PF5's counters. Any other record is detailed, off unless asked for;<br>• **memory is sized from measurement:** each region and flight recorder is the smallest that holds one drain interval of the always-on tier on the corpus copies, set at M1-CP3;<br>• **costs are checked continuously:** recording's instructions and allocations are tier-1 tests on every commit from M1-CP2 (OB27), and each checkpoint that adds records reports what they cost.<br>**Budgets** (hypotheses, measured where named):<br>• **recording** (M1-CP2, as instructions retired on fixed work, which S16 found steady): a disabled span ≤ 2 ns; an enabled one ≤ 50 ns; no allocation;<br>• **the always-on tier** (M1-CP14, under PF1): ≤ 1% of a process's CPU time on the corpus loads;<br>• **memory** (M1-CP3, with S16's method): ≤ 256 KiB per helper's region, counted in B2 and B4; the collector and the browser's recorder ≤ 4 MiB, counted in the browser's 85.4 MB at M1 and in B1a from M2;<br>• **idle:** no wakeup (PF16) and no write (PF28) with observability on |
| OB26 | **Determinism.** In single-process tests, records take EL24's virtual time, so a replayed schedule yields an identical trace, which tests may compare as a golden file (ES21). Recording doesn't change a schedule: it takes no lock and never yields |
| OB27 | **Tests:**<br>• first-tier unit tests that recording allocates nothing and costs no more than OB25's instructions (PF5);<br>• the region decoder's fuzz target and the lying producer (OB24);<br>• **a crash test:** a helper that panics leaves a report with its message and its last spans;<br>• **a hang test:** PF19's stalled thread is named in the report;<br>• **a violation test:** IR3's log names the reason;<br>• PF16's and PF28's checks with observability on;<br>• an export round trip: a trace exported and read back by `tools/trace-summary.py` gives the same spans.<br>Tests that kill or crash helpers move the system's crash reports into their results under `target/`, as the spikes did |

## 11. Staging

| When | What |
|---|---|
| **M1-CP2** | `trace`'s recorder in process, with its first-tier tests |
| **M1-CP3** | The regions and their protocol (an interface note for the owner, CR8); the collector; the flight recorder, the browser's file-backed one included; crash, hang and violation reports; the lying producer and the fuzz target |
| **M1-CP4** | The always-on tier through the walking skeleton: tasks, lifecycle phases, IPC flows, navigations and loads; metrics into PF5's harness; the network log as a category |
| **M1-CP9, CP11** | The compositor's frames (PF13); the network process's categories |
| **M1-CP12** | The shell's command to save a trace; the health monitor and its breach reports; the daily health log |
| **M1-CP14, CP15** | OB25's budgets measured under PF1, and reported at the gate |
| **M2** | The native diagnostics window; the system's crash reports moved in (RV1-15); the GPU process as a producer |
| **M3** | Console messages (RV1-15); pages' own User Timing marks in the detailed tier |
| **Later** | Energy per tab, from each process's energy mapped to the tabs it serves; trend reports across milestones |

## 12. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **The `tracing` crate** as the front end of recording, as Servo uses it with `tracing-perfetto` (§2) | Mature and familiar, with an ecosystem of subscribers. But its spans go through dynamic dispatch and field visitors, and the cross-process half (regions, validation, the flight recorder, export) would be Axiom's anyway. As a dependency of every process, the browser included, it would need a `ub-risk-2` grade or better across its tree (DP8, DP9). **Not chosen now; the revisit option:** `trace`'s interface leaves room to put it behind `trace` if measurement favours it |
| B | **Perfetto's SDK** | C++ in every process, the browser included, and a service of its own. Rejected (DP10, DP4) |
| C | **`os_log` and `os_signpost`** | Helpers would need a lookup their profiles don't allow (IP4), and signposts are read with Instruments, which needs Xcode (§2). The browser could still forward its own logs there later. Rejected as the mechanism |
| D | **A log file per process** | Helpers can't open files under their profiles, and widening them for logs would weaken the sandbox for convenience (charter principle 7). Rejected |
| E | **Batches sent as IPC messages** | Simpler to account for, but each batch is a send, and a crash loses whatever wasn't yet sent: the post-mortem would miss the seconds that matter. Rejected |
| F | **Helpers report their own health** | A compromised helper could understate its cost. Rejected: the browser measures each child (OB19) |
| G | **Detailed tracing always on** | It would spend the budgets (OB25) on data rarely read. Rejected: two tiers (OB7) |
| H | **Tracing only on demand, with no flight recorder** | Crashes and hangs aren't scheduled. Without the recent past, reports would say that something failed, not what it was doing. Rejected |

## 13. Open questions

1. **Region sizes and the watermark** (OB10, OB12): set at M1-CP3 from the corpus copies' record rates.
2. **The always-on tier's cost** (OB25): measured at M1-CP14; categories move to the detailed tier if it exceeds 1%.
3. **The browser's file-backed recorder** (OB16): its format and size, at M1-CP3's interface note.
4. **The diagnostics window** (OB21): designed in M2's plan.
5. **The watchdog's cycle while the browser is idle:** PF19 pinged each helper once a second, and PF16 allows no idle wakeups. **Settled by the owner** (2026-10-23): "Confirm that watchdog pings run only during active window sessions to guarantee zero idle wakeups under PF16." PF19 is amended: the pings run only during an active session, so OB13's drains and OB19's samples, which ride the same cycle, stop when everything is idle. M1-CP3's note on task ages designs the restart, and PF16's measurement there checks it.

## Sources

All read on 2026-10-22.

- [P1] Perfetto, "Security model" — https://perfetto.dev/docs/design-docs/security-model
- [P2] Perfetto, "Buffers and dataflow" — https://perfetto.dev/docs/concepts/buffers
- [P3] Perfetto, "Visualizing external trace formats with Perfetto" — https://perfetto.dev/docs/getting-started/other-formats
- [P4] Perfetto, "Perfetto UI" — https://perfetto.dev/docs/visualization/perfetto-ui
- [C1] Chromium, `base/metrics/persistent_histogram_allocator.h` (GitHub mirror) — https://github.com/chromium/chromium/blob/main/base/metrics/persistent_histogram_allocator.h
- [S1] The Servo Book, "Profiling" — https://book.servo.org/contributing/profiling.html
