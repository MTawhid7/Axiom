# ADR-0023: Observability and Diagnostics

| | |
|---|---|
| **Status** | **Accepted** (2026-10-23), by the owner at Milestone 1's first checkpoint: "Accepted as proposed (Decisions D1 through D10)", with a direction on overhead that OB25 now carries, and PF19 amended for the watchdog's idle wakeups (decision history) |
| **Date** | 2026-10-22 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Observability and diagnostics](../architecture/observability.md) (OB1–OB27), which this ADR proposes; [ADR-0022](0022-code-architecture-and-engineering-standards.md) (its D8; the `trace` and `diagnostics` crates); [ADR-0004](0004-process-model-and-sandbox.md) and the [process model](../security/process-model.md)'s IR rules; [ADR-0005](0005-ipc-transport-and-interfaces.md); [ADR-0006](0006-event-loop-and-threading.md); [ADR-0019](0019-performance-methodology-and-budgets.md) (PF2, PF5, PF13, PF14, PF16, PF19, PF27–PF30); [ADR-0020](0020-roadmap-and-security-levels.md) (M2's crash reports, RV1-15); [ADR-0021](0021-milestone-1.md) (MA1, MA6) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-22 | **The owner's direction,** on the first draft of the plan for Milestone 1's engineering foundation: "I think we should implement a robust observability mechanism that will help us monitor the system effectively in the long run." |
| 2026-10-22 | **The owner's choice in plan mode,** from options put with a recommendation: the native diagnostics window at **"M2, with crash reports (Recommended)"**, with the whole mechanism built at M1: recording in every process, collection in the browser, reports, a bounded daily health log and exports |
| 2026-10-22 | **Proposed** at M1-CP1 |
| 2026-10-23 | **Accepted** by the owner: "ADR-0023 (Observability and Diagnostics): Accepted as proposed (Decisions D1 through D10). The trace crate, untrusted shared-memory ring buffers, flight recorder, and local health logs are approved." |
| 2026-10-23 | **The owner's direction on overhead,** with the acceptance: "Also, we should ensure that the observability architecture introduces minimal performance overhead and consumes as little CPU and memory as possible." [OB25](../architecture/observability.md#10-budgets-determinism-and-tests) now carries it: D9's values are ceilings, not targets; the always-on tier holds only records that a named consumer needs; regions and flight recorders are sized from measurement to the smallest that holds one drain interval; and every cost is measured on every commit from M1-CP2 (tier 1), and reported at each checkpoint that changes it |
| 2026-10-23 | **The owner's direction on the watchdog,** settling the design's open question 5 and the plan's §2 item 19: "Watchdog vs Idle: Confirm that watchdog pings run only during active window sessions to guarantee zero idle wakeups under PF16." **[PF19](../performance.md) is amended** with these words: the pings run only during an active session, and stop when every watched thread is idle, so that OB13's drains and OB19's samples, which ride the same cycle, stop with them. M1-CP3's interface note on task ages designs the mechanism, and PF16's measurement at M1-CP3 checks it |

## Question

How does Axiom observe itself? How are traces, metrics, logs and reports recorded in every process and collected across process boundaries, and how are they kept, exported and protected, so that the owner can monitor the system over the long run within the budgets and the security model?

## Requirements and constraints

- **Rules that already assume observability,** with no design behind them:
  - MA6's, PF13's and PF14's trace spans;
  - IR3's logged violations;
  - PF19's task ages;
  - PF5's counters;
  - M2's local crash reports (RV1-15);
  - MA1's network log.
- **The trust rules:** IR1 (every message from a less privileged process is untrusted input), IR7 (one writer per shared region; the reader copies before validating), IR10 (a dead child's reports are discarded for recovery), IP16 and EL1 (nobody blocks on a send).
- **The budgets:**
  - PF16: no idle wakeups;
  - PF27: 20 reports and 10 MB of logs;
  - PF28: no idle writes;
  - PF29: no request of Axiom's own, "no telemetry";
  - PF30: release rebuildable caches under pressure;
  - B1–B4: memory.
- **Determinism** (EL24, TS8) and **fuzzing** of every decoder (TS9).
- **The environment:** no Xcode, so no Instruments; helpers' profiles allow one Mach lookup, the rendezvous (IP4).

## Alternatives considered

From the design's §12:
- **A.** The `tracing` crate as the recording front end, as Servo uses it with `tracing-perfetto`.
- **B.** Perfetto's C++ SDK.
- **C.** `os_log` and `os_signpost`.
- **D.** A log file per process.
- **E.** Batches sent as IPC messages.
- **F.** Helpers reporting their own health.
- **G.** Detailed tracing always on.
- **H.** Tracing only on demand, with no flight recorder.

## Historical context and modern developments

- **Perfetto's model** (sourced, design §2): untrusted producers ("Producers are never trusted"), memory "shared only point-to-point between each producer and the tracing service", a service that stamps each producer's identity, and losses counted rather than hidden. Axiom's renderers are untrusted by its own threat model (AD2), so the same shape fits.
- **Chromium's persistent histograms** (sourced) keep metrics in shared or file-backed memory "in order to persist the data for a later use", which is how a crashed browser's last records survive.
- **Servo** traces with annotations and Perfetto's format, filtered at run time (sourced).
- **macOS's own facilities** don't fit the constraints: helpers' profiles allow no logging service, and signposts need Instruments (inference, design §2).

## Analysis

| Alternative | For | Against |
|---|---|---|
| **Axiom's own recorder and format** (chosen) | Sized to the budgets: no allocation, no lock, no system call; records of IDs and numbers that the browser validates cheaply; one design for in-process and cross-process recording | A few hundred lines to write and maintain |
| A, `tracing` | Mature, familiar, with an ecosystem | Dynamic dispatch per span; a dependency of every process, the browser's included, needing `ub-risk-2` or better (DP8, DP9); the cross-process half would be Axiom's anyway |
| B, Perfetto's SDK | Complete | C++ in every process, the browser included |
| C, `os_log` and signposts | System-integrated | Helpers can't reach the logging service under their profiles; signposts need Instruments |
| D, log files | Simple | Helpers can't open files, and widening their profiles for logging would cut a corner (charter principle 7) |
| E, IPC batches | Simple accounting | A send per batch; a crash loses the unsent tail, which is the part a report needs |
| F, self-reported health | — | A compromised helper could understate its cost |
| G, always detailed | Everything recorded | Spends the budgets on data rarely read |
| H, no flight recorder | Cheapest | Reports would say that something failed, not what it was doing |

## Evidence and sources

The design's §2 and Sources, read on 2026-10-22: Perfetto's security-model, buffers, formats and UI pages; Chromium's `persistent_histogram_allocator.h`; the Servo Book's profiling chapter; S16's measurements on this Mac (attribution of shared memory; the steadiness of instructions retired).

## Decision

**Accepted** (2026-10-23): adopt [observability and diagnostics](../architecture/observability.md) (OB1–OB27):

- **D1. Four signals in one record model** (OB1–OB4): spans and events, metrics, logs and reports. Records are small, typed and named by IDs compiled into the build, on one system-wide clock, with flows that follow IPC messages across processes.
- **D2. Axiom's own recorder, `trace`** (OB5–OB9): per-thread single-writer buffers; no lock, allocation or system call on the recording path; static categories, levels and metrics. An always-on tier shares PF19's task-age store, and a detailed tier is switched on per category. Recording never changes behaviour, and the panic hook writes a bounded record before the process aborts. `tracing` is the revisit option behind `trace`'s interface.
- **D3. One shared region per helper, created by the browser** (OB10–OB14). Each ring has one writer. The reader copies and validates everything and stamps the producer's identity itself; a record no correct writer could produce is a violation (IR3). Full rings drop and count, a watermark notice is sent without importance, and nothing polls. A dead helper's region is read once for its report, then unmapped within IR10's window, and its contents never reach the browser's state.
- **D4. The collector, `diagnostics`, on the browser's worker pool** (OB15–OB16), with an in-memory flight recorder per process. The browser's own recorder is a file-backed mapping, so it survives the browser's crash.
- **D5. Reports** (OB17) on each crash, hang, violation and budget breach, each with the flight recorder's last seconds of every process involved. The system's crash reports join them at M2 (RV1-15).
- **D6. Retention within PF27, with no idle writes** (OB18, PF28). The health monitor samples each child with `proc_pid_rusage` while windows are active (OB19). A daily health log is kept for 90 days for long-term trends (OB20).
- **D7. Exports** (OB21): traces in Chrome's JSON trace format, which Perfetto's UI opens; metrics as TOML for PF5's harness; logs as JSON lines; text dumps for tests (MA1's network log); a shell command that saves a trace at M1; **the native diagnostics window at M2,** as the owner chose.
- **D8. Privacy and security** (OB22–OB24): nothing leaves the Mac; no page content, form values, cookies or credentials in any record; URLs reduced to origins unless the owner turns on detailed mode; sensitive sites never recorded in detail. The regions are a new attack surface, fuzzed and tested with a lying producer.
- **D9. Budgets** (OB25), hypotheses measured at M1:
  - a disabled span ≤ 2 ns, an enabled one ≤ 50 ns;
  - the always-on tier ≤ 1% of a process's CPU;
  - regions ≤ 256 KiB per helper;
  - the collector and the browser's recorder ≤ 4 MiB;
  - no idle wakeups or writes.

  **As directed by the owner at acceptance** (2026-10-23), these are ceilings, and the aim is the least overhead that still meets the goals (OB25).
- **D10. Staging** (design §11): M1 builds the mechanism across its checkpoints CP2, CP3, CP4, CP9, CP11, CP12 and CP14; M2 the window and the system's crash reports; M3 the console and pages' User Timing marks.

## Rationale

- **"Monitor the system effectively in the long run"** needs three things, and D1–D7 give each:
  - visibility across processes: one timeline with flows;
  - explanations after the fact: the flight recorder and reports;
  - a record over months: the daily health log.
- **Robustness, in the security model's terms:** helpers' records are untrusted input (IR1). Recording can't block (EL1) or allocate. A crash can't take its last seconds with it: the region outlives a helper, and the file-backed recorder outlives the browser.
- **Efficiency:** the always-on tier costs about one store per task, which PF19 already pays. Nothing polls, and nothing is written while idle.
- **One mechanism instead of many:** MA6's spans, IR3's log, PF19's ages, PF5's counters, MA1's network log and M2's crash reports all become uses of one subsystem.

## Assumptions

- Records of IDs and numbers are enough for diagnosis, with bounded text only where OB3 allows.
- 256 KiB per helper holds the always-on tier's records between drains on the corpus pages; measured at M1-CP3.
- `mach_continuous_time` is consistent across processes on one Mac (background: it is a system clock).
- `proc_pid_rusage` reads a sandboxed child's counters from the unsandboxed browser, as S16 read them at a child's exit; checked at M1-CP12.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The always-on tier costs more than 1% | Low | Budget pressure | Measured at M1-CP14; categories move to the detailed tier |
| Region memory charged unexpectedly (S16) | Medium | B2 or B4 pressure | Measured at M1-CP3 with S16's method; sizes set from it |
| A bug in Axiom's own writer looks like a violation and ends helpers (OB11) | Low | Crashes in development | The writer's tests and the decoder's fuzz target; a violation's report names the record |
| PF19's pings while idle conflict with PF16 | Medium | Idle wakeups | **Settled by the owner** (2026-10-23): PF19 amended so the pings run only during an active session; PF16 measured at M1-CP3 |
| A trace opened in a web viewer leaves the Mac | Low | Privacy | No content in records (OB22); the owner chooses the viewer; a local summary tool (OB23) |

## Conditions for revisiting

- `tracing`, or another crate, meets the budgets and DP8's grades with less code: D2's revisit option.
- The always-on tier can't meet its budget.
- A milestone needs a signal this model can't express, such as sampled stacks in-engine, which samply covers today.
- macOS offers a logging path that sandboxed helpers can use without widening their profiles.

## Consequences

- **Milestone 1's plan** builds observability across its checkpoints (design §11) and reports its budgets at the gate.
- **On acceptance, these documents gain entries,** each with the owner's words (**done on 2026-10-23**):
  - the [threat model](../security/threat-model.md)'s §5, a row for the diagnostics regions;
  - the [roadmap](../roadmap.md)'s M2, the diagnostics window;
  - the [requirements](../requirements.md), rows for reports, retention and the privacy rules (RQ71–RQ73).
- **No accepted rule changes for the design itself.** It uses IP14's shared memory, IP20's messages without importance and EL23's background QoS on the browser's existing worker pool. It needs no new thread role and no profile rule, and it stays within PF27's disk budget. **One rule changes by the owner's direction at acceptance:** PF19, whose pings now run only during an active session (decision history).
- **ADR-0022's D8** places the crates: `trace` at the foundation, `diagnostics` in the browser.
