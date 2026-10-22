# Review Round 4: Efficiency First

| | |
|---|---|
| **Status** | **Round 4, disposed** (2026-10-22): the owner accepted all seventeen findings as proposed, with the choices the [disposition log](../README.md#59-round-4s-dispositions) records. It stays as written, as the round's record; where the owner's words differ, they govern |
| **Question** | The owner's third theme: should efficiency be a first-class architectural requirement from the first phase, and what would an efficiency-first strategy need, across battery, CPU, memory, disk and background activity, without giving up quality? |
| **Covers** | Charter G4; [ADR-0019](../../adr/0019-performance-methodology-and-budgets.md) and the [performance budgets](../../performance.md) (PF1–PF24, B1–B7, T1–T4); [testing](../../testing.md) TS1 and TS13; [Milestone 1](../../milestone-1.md)'s MA6 and MA9; the designs' efficiency rules; spike S16-Q3 ([results](../../research/s16-results.md#5-s16-q3-counters) §5) |
| **Summary** | [Findings](06-findings.md): RV4-06, with RV4-04 · [Actions](07-actions.md) |
| **Last updated** | 2026-10-22 |

**The owner's words** (2026-10-22): "should we make efficiency a first-class architectural requirement from the very first phase, rather than treating it as a later optimization exercise?" "We should aim for the smallest practical resource footprint across battery consumption, CPU utilization, RAM, disk storage, and background activity, while preserving a polished, reliable, and professional, industry-grade user experience." Five headings follow: efficiency by design; advanced optimization; data-driven engineering; quality without compromise; continuous performance validation.

---

## 1. The answer

**Yes, and it already is, in goals and design.** What is missing is the validation between gates, and budgets for some of the resources the owner named.

- **Observed: a goal since 2026-10-16.** Charter G4: "**Efficiency is a primary architectural goal, not an afterthought:** low memory use, CPU efficiency and low power consumption, with measurable budgets on the target machine (owner direction, 2026-10-16)". This Mac is "the **performance floor**" (charter §4).
- **Observed: in the architecture's decisions,** not added later. Each of these was decided on efficiency grounds, most with a spike's measurement:

  | Decision | Rule | Evidence |
  |---|---|---|
  | One wait per thread, so idle costs nothing | EL (ADR-0006), AC3, NW1 | S7-Q4: one I/O thread, no idle wakeup but real deadlines |
  | No idle wakeups on a static page | PF16 | S6's compositor and S7's network process had none in most windows; Chrome's browser used CPU in every window (S8) |
  | Scrolling and animation move layers, not pixels | GR8 | S6-Q3: 0.19–0.40 ms a frame |
  | Tiles budgeted by the pixel, released or purgeable when hidden | B1, GR12 | S6-Q5 |
  | Renderer images sampled in place, never copied | PL8 (RV1-09) | S10-Q2; S16-Q1 |
  | Helpers apply bytecode the browser compiled | ADR-0004 D2 | S4: 2.11 ms and 1.14 MiB to ready, against 8 ms and 3.9 MiB |
  | Memory budgets per process, and a ceiling per corpus page | B1–B7, PF4 | S8's Chrome and Safari measurements |
  | Hidden pages throttled; energy no worse than Safari's | PF17 | S8 §7.4 |
  | Bounded parser slices, long tasks, collector pauses | PF14, PF15, PF23 | The engines' constants, sourced |
  | Build time and memory | T1–T4 (PF18) | ADR-0002 |

- **Observed: what is missing** (RV4-06):
  - **between gates, nothing measures:** PF5 runs every budget "at each milestone gate that touches it", and TS1 runs benchmarks "at each gate, and before merging a change that touches a hot path";
  - **budgets for start-up, CPU per page load, disk footprint, disk writes, background activity and memory pressure** don't exist;
  - **plans and designs don't state their efficiency cost** as a section, so a change's cost is found by measurement after it lands.

## 2. The owner's five headings, against the baseline

| Heading | What the baseline has | What it lacks | Proposal |
|---|---|---|---|
| **Efficiency by design:** measurable budgets early, for memory, CPU, storage, start-up and battery | B1–B7 for memory; PF13–PF15 for frames, long tasks and pauses; PF16 and PF17 for idle and energy; T1–T4; PF19–PF24's limits | Start-up; CPU per page load; the bundle, profile and cache on disk (NW10's cache has no size); bytes written; background activity; memory pressure | **PF25–PF30** (§5), set at M1's plan |
| **Advanced optimization:** profiling, algorithms, caching, memory management, lazy loading | `samply` and `/usr/bin/time -l` (PF2); caches by design (§6); arenas and the collector's end-state (RV1-04) | A list of the techniques in place, so a plan can see what is missing; memory-pressure release; purgeable decoded images | **The catalogue** (§6) |
| **Data-driven engineering:** representative and demanding workloads | The 13-site corpus; S8's Chrome and Safari figures; the reference engines on the same pages (PF3); PF1's conditions | Fixed workloads small enough to run at every checkpoint | **Fixed workloads** on EL24's virtual clock (§4.2) |
| **Quality without compromise** | The charter's G2 and G3; the gates' conformance, corpus and security criteria (MS7); PF5's revision through the owner | A rule that a budget never cuts scope silently | **§7** |
| **Continuous performance validation:** monitoring and regression checks so features don't erode efficiency | Budgets at gates (PF5, TS13) | Every commit and every checkpoint; a baseline that ratchets; a trend | **Three tiers** (§4) |

## 3. The measurement basis: S16-Q3

Spike S16 measured whether macOS's per-process counters can serve as regression checks ([results](../../research/s16-results.md#5-s16-q3-counters) §5; measured, 2026-10-22, PF1's conditions; ten runs of each workload in a fresh child, idle and with four spinning processes):

| Counter (`rusage_info_v6`) | Steadiness on fixed work | Use |
|---|---|---|
| **Instructions retired** (`ri_instructions`) | **0.03–0.15%** from run to run, idle or loaded; means moved 0.10–0.44% under load | **Every checkpoint** |
| Cycles (`ri_cycles`) | Move with frequency and load | Diagnosis |
| **Peak footprint** (`ri_lifetime_max_phys_footprint`, `ri_interval_max_phys_footprint`) | Exact idle; within 0.8% loaded | **Every checkpoint** |
| **Disk bytes written** (`ri_diskio_byteswritten`) | **Exact** (33,554,432 in every run); logical writes within 0.2% | **Every checkpoint** |
| Energy (`ri_energy_nj`) | 0.4–2.4% idle on CPU-bound work, but its mean moved 8.5% under load | **Gates,** under PF1 |
| Wall time | 1.1–37% idle; 24–137% longer under load | **Gates,** under PF1 |
| `ri_user_time` | In ticks of the Mach clock (× 125 ÷ 3 for nanoseconds) | Converted, or `wait4`'s instead |

- **Every counter is filled in for sandboxed children,** and readable by the parent after the child exits and before it is reaped (`waitid` with `WNOWAIT`), without privileges.
- **Fixed work is the condition:** S10's `validate` warms up for a fixed time, not a fixed count, and its instructions moved 2.7% under load. A workload whose work depends on elapsed time measures the clock, not the code.

## 4. The strategy: three tiers and a ratchet

### 4.1 Every commit: counts that don't vary

Unit tests that assert deterministic quantities, in the five checks, so a regression fails `cargo test`:
- **sizes:** `const` assertions on the types that dominate memory (a node's columns, a computed style, a display item, a fragment);
- **allocations:** a counting allocator in tests, for operations whose allocations are fixed (parsing a fixture, styling a tree, building a display list);
- **work counts:** display items, messages and handles per operation, atoms interned, cache hits on fixtures;
- **wakeups:** timers and tasks over a fixed interval on EL24's virtual clock, for idle and hidden pages (PF16, PF17).

### 4.2 Every checkpoint: counters against a ratchet

- **Fixed workloads** in the headless embedder: the corpus copies and a slice of MA4's tests, loaded in single-process mode on EL24's virtual clock, so the work is the same every run; and, in real processes, start-up and an idle window.
- **Measured per process,** by the parent at exit (S16's method): instructions retired, peak footprint, bytes written, idle wakeups.
- **A baseline committed in the repository,** one value per workload and counter, with the commit and the build of macOS. **It only ratchets down:** an improvement past the tolerance lowers it; **a regression past the tolerance fails the checkpoint** unless the commit states its reason, and the milestone's report lists every accepted regression. Tolerances are hypotheses for M1's plan: 1% for instructions, 2% for peak footprint, none for bytes written by workloads that shouldn't write.
- **The trend** is kept beside the baseline, a row per checkpoint, so drift inside a budget is visible long before the gate.

### 4.3 Every gate: PF1's formal measurements

As now (PF1, PF5, MA9): release builds, AC power, Low Power Mode off, the load below 3.0, cool-downs, three runs, medians with ranges; wall time, frames, footprints and energy; every budget of the milestone. Energy impact by `top`, and `powermetrics` where it needs root, run by the owner's hand.

### 4.4 What builds it

A measurement harness of `proc_pid_rusage` reads in the headless embedder's runner, a baseline file and a ratchet check: M1's first checkpoints, with the harness (MA11's second). It is test tooling, so it needs M1's authorization like the rest (MA10).

## 5. Budgets the owner named and the baseline lacks

Each is a **hypothesis** to set at M1's plan from measurement, as B1–B7 were: Axiom's spikes for the floor, and Safari's and Chrome's figures on this Mac (PF3) for the bar.

| # | Budget | What is measured | Starting evidence |
|---|---|---|---|
| PF25 | **Start-up** | Cold and warm launch to the first window drawn; a helper's spawn to ready; the GPU process's, from M2 | S2: first frame 174.2 ms from `main`, warm; S4: a helper ready in 2.11 ms with the browser's bytecode; S10: the GPU process ready in about 20 ms warm, 451–458 ms without its cache |
| PF26 | **CPU per page load** | Instructions retired and CPU time, across every process, to load each corpus copy and settle | S16-Q3's counters; S8's Chrome and Safari CPU figures, where taken |
| PF27 | **Disk footprint** | The bundle; the profile; the HTTP cache, given a size (NW10); logs and crash reports, given a bound | None yet: the bundle exists at M1 |
| PF28 | **Disk writes** | Bytes written per hour idle, with one window, and over a corpus session | S16-Q3: bytes written are exact; flash wear and energy both scale with them (background) |
| PF29 | **Background activity** | With every window hidden: wakeups, CPU and network requests per hour; **no request Axiom didn't need** (no telemetry exists to send) | PF16 and PF17 cover idle and hidden pages, not the browser as a whole |
| PF30 | **Memory pressure** | What each process releases at the system's warning and critical levels, in what order, and how fast; tested by injecting the event | B6 names discarding; PF22 empties the back-forward cache first; GR12 marks hidden tiles purgeable |

## 6. Techniques: in place, and missing

| Technique | In place | Missing or unstated |
|---|---|---|
| **Memory layout** | The node arena of `u32` columns (HD11); 8- and 16-bit strings (PL11); atoms; immutable fragments (LY6) | Size assertions in tests (§4.1) |
| **Caching** | Shared computed styles, the rule tree and the sharing cache (ST14–ST17); glyph atlases (TX); decoded images at display size (IM); the HTTP cache (NW10) | Bounds for each cache in bytes, and what each releases under pressure (PF30) |
| **Incremental work** | Invalidation sets (ST19); stage reuse (IV); damage-limited tiles (GR); display lists diffed per frame | — |
| **Lazy work** | Accessibility built on first query (ADR-0014: "Cost when unused: None: built on first query"); the user-agent stylesheet at build time if parsing it costs over 1 ms (PF23); images decoded when needed | Start-up deferral of subsystems not needed for the first frame, measured by PF25 |
| **Scheduling** | QoS by role and visibility (EL23); idle tasks only in idle periods (EL16); hidden-page throttling (PF17) | Background activity as a whole (PF29) |
| **Memory release** | Hidden windows' tiles released or purgeable (GR12); the back-forward cache first under pressure (PF22) | A dead renderer's resources at once (RV4-04); purgeable memory for decoded images and caches (`vm_purgable`), a candidate for PF30 |
| **Processes** | Bytecode profiles (D2); 16 renderers' soft limit (B5) | — |
| **Binary size** | Release profiles with `panic = "abort"` (RV1-16) | A size budget (PF27), and a check of what each dependency adds (DP19 records it where measured) |

## 7. Quality without compromise

- **A budget never cuts scope silently.** A feature in a milestone's scope that misses a budget goes to the owner, who accepts a revision or the cost (PF5's rule, applied to the new budgets too).
- **Efficiency never weakens a security boundary** (charter principle 7): processes aren't merged to save memory beyond what the process model allows, and site isolation's cost is budgeted (B5), not avoided.
- **The user's experience is measured with the rest:** frames (PF13), input latency (PF19's deadlines and PF20), and the corpus checks' readability (MA5) sit beside the resource budgets at every gate.

## 8. In plans and designs

Every milestone plan, and every change to a design, gets an **efficiency section** with:
- what the change costs in steady and peak memory, CPU (instructions per operation), energy (wakeups, GPU work), disk (size, writes) and start-up;
- which tier catches a regression (§4), and with which workload;
- the budget it draws on, and its margin.

MS7's acceptance criteria then include the new budgets, and MS10's plan template the section.
