# Event Loop and Threading Model

| | |
|---|---|
| **Status** | **Binding** since ADR-0006's acceptance (2026-10-17, Gate B). Written at Step 4, CP3. EL11 amended at Gate B (OQ-19). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): the browser's compositor thread and the GPU/compositor process's threads, §4.2 (RV1-02; CP2); EL11's second exception, parallel marking in pauses (RV1-04; CP6) |
| **Question** | How does each Axiom process run its work? Which threads exist, how does each one wait, how is the HTML event loop implemented and scheduled, what drives frame production, and how do agents and workers map onto threads and heaps? |
| **Answers** | Step 4 questions Q1–Q3 and the first half of Q4 ([tracker](../research/step-4-architecture-proposal.md) §3); OQ-5 in part |
| **Feeds** | ADR-0006; CP4 (IPC), CP5 (frame tree), CP6 (pipeline), CP7 (invalidation), CP8 (input), CP9 (embedder API), CP10 (script and heap) |
| **Builds on** | [Process model](../security/process-model.md); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); [ADR-0004](../adr/0004-process-model-and-sandbox.md); [macOS runtime note](../research/macos-runtime-facilities.md); [S2 results](../research/s2-results.md); [S4 results](../research/s4-results.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered EL1–EL24. Precedent is sourced (§3, Sources); the design is inference unless marked otherwise.

## Summary

- **Every thread blocks in exactly one place** (EL1–EL3). Helper threads wait in one `kevent64` call on their Mach ports, their next timer and an in-process wakeup. Threads that must host Core Foundation sources wait in their run loop, with engine sources in the common modes. Nothing polls, so idle costs nothing.
- **Messages arrive on the thread that handles them** (EL4). Each IPC endpoint is bound to one thread, so there is no intermediate I/O thread.
- **The browser runs four kinds of thread** (EL6): AppKit's main thread for the shell only; an engine **core** thread for the frame tree, navigation, process management and policy; a **compositor** thread that owns the display links, schedules frames, composites and presents; and a small worker pool.
- **Renderers run one thread per event loop** (EL7, EL15). Each window agent and each worker gets its own thread and its own heap. Same-site tabs that share a process don't block each other, and each thread's QoS follows its agent's visibility. A control thread handles interrupts and other out-of-band requests, and a helper pool does work that never touches a heap.
- **GC heaps never cross threads** (EL10, EL11). Rust's type system enforces it; helper threads see only owned messages, immutable snapshots, or data lent for a bounded scope. The only GC-managed data that may be lent, read-only and for one frozen rendering stage, is a DOM arena (AC5 as the owner amended it at Gate B).
- **The HTML event loop's task selection is ours to define** (EL16). Discrete input first, then a due rendering update, then other tasks in arrival order, then idle work. Continuous input is coalesced into the next rendering update.
- **The browser drives frames** (EL19–EL22). The compositor owns one display link per window and pauses it when no one has asked for a frame. Each vsync it offers a frame to the event loops that subscribed. It never waits for a renderer: a late frame is composited at the next vsync, and compositor-driven scrolling continues meanwhile.
- **Background work is throttled in steps** (EL18, EL23): timer alignment for hidden documents, task roles and darwin-background for hidden renderers, using public interfaces only.
- **Tests run the same loops deterministically** (EL24): a virtual clock and a single-threaded scheduler drive every event loop, and single-process mode runs the production threads in one process.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **PM6, R6:** every future process boundary is a message interface that also works in-process | Threads communicate by messages too, through the same channel types (EL4, EL24) |
| **PM8, AC3:** idle costs nothing. S2 measured an idle AppKit window at 0.0% CPU with no wakeups | EL1–EL3, EL19 |
| **ADR-0003:** one heap per agent; heaps are single-threaded, and the root stack is deliberately not `Send` | EL10, EL15 |
| **SI7:** an agent cluster must not be split across processes; cross-origin isolated groups get their own processes | EL15 (and its clarification) |
| **IR5, IR6:** the browser never blocks on a less privileged process; a child's synchronous calls are bounded and can't re-enter | EL6, EL17, EL21 |
| **AC5**, as amended at Gate B: nothing GC-managed crosses a process, and across threads only DOM arenas lent read-only for a frozen rendering stage | EL10, EL11 |
| **AC6:** a deterministic, headless core | EL24 |
| **B1–B7:** memory targets | §4.10 |
| **macOS runtime note §11, items 1–9 and 15–16** | Throughout; cited as "macOS note item N" |

## 2. What the specifications require

**HTML** [H1–H3]:
- **An event loop per agent.** "Each agent has an associated event loop, which is unique to that agent." A similar-origin window agent has a window event loop; dedicated, shared and service workers have worker event loops; worklets have worklet event loops.
- **Threads are the implementation's choice.** "Event loops do not necessarily correspond to implementation threads. For example, multiple window event loops could be cooperatively scheduled in a single thread." For workers that can block, the forward-progress rules "effectively amount to requiring dedicated per-agent threads".
- **Choosing among task queues is ours.** Task queues are sets of tasks, and the loop takes the first runnable task from one "chosen in an implementation-defined manner". The only ordering rule is never to process "events from any one task source out of order". The specification's own example gives keyboard and mouse events "preference over other tasks three-quarters of the time".
- **One task, then the microtasks.** The loop runs one task, then performs a microtask checkpoint.
- **Rendering is a task too.** When a navigable has a rendering opportunity, a task on the rendering task source runs "update the rendering". Its 23 steps run from resize and scroll events through animation-frame callbacks, style and layout with resize observers, and intersection observers, to painting.
  - "This specification does not mandate any particular model for selecting rendering opportunities." Its examples are 30 per second under load, and 4 per second for a hidden page.
  - An update may be skipped when it "would have no visible effect", or to "coalesce timer callbacks together, with no intermediate rendering updates".
- **Idle periods** begin when a window event loop has no runnable task. Their deadline is at most 50 ms away, "to ensure responsiveness to new user input within the threshold of human perception", and earlier if a timer is due or a frame is pending [H1, H4]. A hidden page's idle periods may be throttled [H4].
- **Timers:** after five nested timers, at least 4 ms. A user agent may also "wait a further implementation-defined length of time", "to optimize the power usage of the device" [H3].
- **Agent clusters** [H2]:
  - They are keyed by site, or by origin when the group is cross-origin isolated or asks for origin-keyed clusters.
  - Dedicated workers and worklets join their owner's cluster. Shared and service workers get clusters of their own.
  - Two windows with no opener or ancestor relationship are in different clusters, even when they are same-origin.
  - "Only shared and dedicated worker agents allow the use of JavaScript Atomics APIs to potentially block."

**ECMAScript** [E1]:
- "An agent comprises a set of ECMAScript execution contexts, an execution context stack, a running execution context, an Agent Record, and an executing thread." A thread may serve several agents "provided none of the agents sharing the thread have an Agent Record whose [[CanBlock]] field is true."
- The agents of one cluster may share memory. An embedding "must not leave some agents in the cluster active while other agents in the cluster are deactivated indefinitely."
- Jobs run only when the agent's execution context stack is empty, one at a time, each to completion. Promise jobs run "in the same order as the HostEnqueuePromiseJob invocations that scheduled them"; HTML puts them in the microtask queue.

**What the specifications leave to Axiom:**
- the threads (EL15);
- the choice among task queues (EL16);
- the model of rendering opportunities (EL19–EL22);
- how much longer timers may wait (EL18).

## 3. Precedent

From documentation and source read on 2026-10-17 (Sources). How each engine waits and gets its display clock on macOS is in the [macOS runtime note](../research/macos-runtime-facilities.md) §10. "Not checked" means this document didn't look.

| Topic | Chromium | Firefox | WebKit | Ladybird | Servo |
|---|---|---|---|---|---|
| **Threads in a content process** | "There is only one main thread per render process": scripts, "the rendering event loop, the document lifecycle, hit testing". One compositor thread per renderer handles input, scrolling and animation. Web Workers run script on their own threads [C1] | Layout frames "must be accessed only from the main thread of their processes" [F3] | Not checked | The event loop's thread runs script. Style and layout run on stage threads that "only read what the host lends them and never touch the heap themselves" [L1] | One script thread per event loop: "Each event loop is the constellation's view of a script thread", and the pipelines (one per frame) of that loop share it. Event loops are kept per browsing context group and registered domain [S2]. "Layout initially starts in the same thread as Script, but may use worker threads"; in multiprocess mode, "each script thread runs in its own web content process" [S1] |
| **Choosing the next task** | "Input task runner has the highest priority." Tasks belong to frames, "to allow the scheduler to freeze or prioritise individual frames", and a started task runs to completion [C2] | Not checked | Not checked | One task queue for all sources. Normal tasks go before idle tasks. While a rendering update is unfinished, the next rendering task holds its place and the tasks behind it wait [L1] | Not checked |
| **Continuous input** | Since Chrome 60, continuous events (wheel, `touchmove`, `pointermove`, `mousemove`) wait until just before `requestAnimationFrame`; discrete events go "right away". The experiment measured "35% less hit tests" [C5] | "UI events get routed to APZ first, to minimize latency" [F3] | Not checked | Input is drained inside the rendering update, and superseded moves are dropped [L1] | Not checked |
| **What drives frames** | The compositor tells Viz it needs a frame; Viz "signals the start of the render back"; the compositor forwards it to Blink, "at a cadence matching the device display" [C1] | The UI process's vsync thread forwards vsync to the GPU process (macOS runtime note §10) | The UI process sends `DisplayDidRefresh` to web processes (macOS runtime note §10) | A rendering opportunity's source is one of `Compositor`, `LocalTimer`, `Watchdog` and `Manual`. Compositor timestamps are clamped so that they stay monotonic [L1] | Not documented: the book's compositor page is a "TODO" [S1]. The embedder sends `TickAnimation` to the constellation [S2] |
| **Background throttling** | Three levels for timers: unthrottled when visible; checked once a second when hidden; once a minute after five minutes hidden, for chained timers in silent pages without WebRTC [C4]. Eligible background pages are frozen on desktop [C2] | Inactive tabs get at least 1 s between timers on desktop, unless they hold an `AudioContext`. Tracking scripts in background tabs get 10 s [F1] | Not checked | Not checked | Not checked |

**Observations** (inference):
- **The engines checked put input first.** Chromium gives it the highest priority, and Firefox routes it to the compositor first. Chromium and Ladybird align continuous input with frames.
- **Chromium, WebKit and Ladybird drive rendering from the compositor or the display's clock,** not from a free-running timer in the content process. Ladybird also lists a local timer and a watchdog as sources; what triggers them wasn't checked.
- **Chromium multiplexes all frames of a renderer onto one main thread** and adds a compositor thread to keep scrolling smooth. In Axiom the browser scrolls, and the compositor composites: the GPU/compositor process from M2 (Round 1, RV1-02; alternative E in §5).
- **Servo's script threads are the closest precedent for EL15.** It runs one script thread per event loop, keyed by browsing context group and registered domain, which is the similar-origin window agent's boundary.
- **Ladybird's stage threads are the closest precedent for EL11.** They never touch the GC heap, and a runtime check forbids it.
- **Throttling levels have been tuned in practice** for years (Chrome since 2021). Axiom starts from them (EL18) instead of inventing its own.

## 4. Design

### 4.1 Thread loops

| # | Rule |
|---|---|
| EL1 | **One wait per thread.** Every engine thread is a *thread loop*: it blocks in exactly one place and dispatches whatever woke it. No thread polls, spins, or wakes on a timer without a real deadline (AC3). Spinning is also what Apple asks programs on Apple silicon to avoid [A1] |
| EL2 | **Helper threads wait in kqueue.** A helper thread's loop is one `kevent64` call on its own kqueue, which holds: `EVFILT_MACHPORT` on the thread's port set, with the message received in the same call; the earliest timer deadline, as the call's timeout or an `EVFILT_TIMER` with leeway; an `EVFILT_USER` event for wakeups from other threads in the process; and descriptors where needed (macOS note item 2) |
| EL3 | **Threads that host Core Foundation sources wait in their run loop.** These are the browser's main thread, under AppKit, and the compositor thread, which hosts the display links. **On the main thread,** engine sources are registered in the common modes, which in a Cocoa application include "the default, modal, and event tracking modes" [A3]. So they keep firing during menu tracking, live resize and modal panels (macOS note item 1). Firefox and WebKit register their main-thread work the same way [F2, W2]. **The compositor's run loop** is a plain Core Foundation loop with no AppKit, which only ever runs its default mode |
| EL4 | **Endpoints are bound to threads.** Each IPC endpoint's receive right belongs to the port set of the one thread that handles its messages. A message wakes that thread directly, with no I/O thread in between. The thread that owns an endpoint is a property of the interface, fixed in CP4 |
| EL5 | **Threads have roles, not pools of anonymous work.** Each thread has a named role, a fixed QoS class (or one that follows a stated rule), and a fixed set of endpoints. The exception is each process's bounded helper pool (EL9) |

### 4.2 The threads of each process

| # | Rule |
|---|---|
| EL6 | **Browser process:** the main thread, the core thread, the compositor thread and a worker pool, as in the table below |
| EL7 | **Renderer process:** one thread per event loop, a control thread and a helper pool, as in the table below. The process's initial thread is the control thread, because it lives as long as the process while event loops come and go |
| EL8 | **Network process:** an I/O thread, a storage thread and a worker pool, as in the table below |
| EL9 | **Every process has at most one helper pool,** bounded by the number of cores. Its threads run leaf jobs (decoding, style and layout stages if CP6 adopts them, disk I/O) at the QoS of the thread that asked for the job, and never touch a GC heap, except to read DOM arenas lent for a frozen rendering stage (EL11) |

**Browser process**

| Thread | Count | Role | QoS | Waits in |
|---|---|---|---|---|
| **Main** | 1 | AppKit only: windows, views and the shell's UI; taking in `NSEvent`s; answering IME and accessibility queries from state mirrored onto this thread (macOS note item 13); creating each window's display link | User-interactive | AppKit's run loop |
| **Core** | 1 | The engine's authority: frame tree, navigation and session history (CP5), process launch and site locks, permissions and user activation, the engine side of the embedder API (CP9), endpoints for renderers' and the network process's control messages | User-interactive | kqueue |
| **Compositor** | 1 | Display links and frame scheduling (§4.5); applying the layer commits of the compositor in explicit Core Animation transactions (macOS note item 11); compositor-driven scrolling and animation on layers (CP7); coarse hit testing (CP8). **As amended in Round 1** (RV1-02): receiving, validating and compositing display lists is this thread's work only at M1, behind the compositor's interface; from M2 it is the GPU/compositor process's | User-interactive | A non-AppKit `CFRunLoop` |
| **Workers** | Bounded pool | Profile I/O, compiling sandbox profiles, decoding favicons with safe Rust, writing downloads | Utility; user-initiated when the user waits for the result | Condition variable or kqueue |

**Why a core thread separate from the main thread** (inference; alternative D in §5):
- AppKit owns the main thread and can nest its run loop, so engine work there competes with the shell's UI and its tracking loops.
- A core without AppKit runs unchanged in a headless embedder, in tests and in single-process mode (AC6). The embedder API (CP9) becomes a message boundary between the shell and the core, like every other boundary in Axiom (PM6).
- AppKit's synchronous IME and accessibility queries must be answered from mirrored state anyway, because the browser may not block on a renderer (IR5). The same mirrors serve a main thread that doesn't hold the frame tree.
- The cost is one more thread hop for some operations, a few microseconds each (S4 measured 3.75 µs for a Mach round trip between processes).

**Renderer process**

| Thread | Count | Role | QoS | Waits in |
|---|---|---|---|---|
| **Event loop** | One per window agent and one per worker hosted (EL15) | Runs one agent's HTML event loop: script, DOM, style, layout and paint for that agent's documents, with that agent's heap | Follows the agent's visibility (EL23) | kqueue |
| **Control** | 1 | Out-of-band requests from the browser that must work while an event loop is busy: interrupting a script (EL17), trimming memory under pressure, shutting down; spawning and ending event-loop threads | User-interactive; almost always asleep | kqueue |
| **Helpers** | Bounded pool | Leaf jobs for event loops: image decoding, and style and layout stages if CP6 adopts them | That of the requesting event loop | Condition variable |

**Network process**

| Thread | Count | Role | QoS | Waits in |
|---|---|---|---|---|
| **I/O** | 1 | Sockets, HTTP, TLS, and the loader endpoints that renderers hold | User-initiated | kqueue (descriptors and Mach ports) |
| **Storage** | 1 | Cookie jar, web storage, cache index. Renderers' bounded synchronous reads, such as `document.cookie` (IR6), wait on this thread, never on network I/O | User-initiated | kqueue |
| **Workers** | Bounded pool | Blocking work: disk-cache reads and writes, and name resolution if Step 5 chooses a blocking resolver | Utility | Condition variable |

Storage runs on its own thread so that a slow network can't delay a synchronous cookie read. If storage later moves into its own process ([process model](../security/process-model.md) §3, note 2), only its deployment changes (AC1).

**GPU/compositor and utility processes.** **As amended in Round 1** (RV1-02): from M2 the GPU/compositor process takes over the compositor thread's validation and rasterizing (process model §3, note 1), with a receiving thread for renderers' updates and resources, and a raster thread that validates them, records Metal work and sends the browser layer commits. The browser's compositor thread keeps frame scheduling, applies layer commits, and scrolls, animates and routes input. Frame scheduling and the display-list interface don't change (AC1). A utility process runs one thread loop per job, at the job's QoS. Real-time audio, later, uses an `os_workgroup` thread (macOS runtime note §5.4).

### 4.3 Data across threads

| # | Rule |
|---|---|
| EL10 | **GC heaps never cross threads.** Heap types are neither `Send` nor `Sync`, and handles are bound to scope tokens (ADR-0003), so the compiler rejects any attempt (AC5). EL11 states the one read-only exception |
| EL11 | **What other threads may see:** owned messages, moved between threads; immutable snapshots behind `Arc`, whose contents are `Sync`; and data that a thread loop lends for a bounded scope during which it can't mutate that data, which Rust's scoped borrows enforce. A GC heap is never lent as a whole, and helper threads never allocate, mutate or collect in one; in Ladybird, stage threads "never touch the heap" [L1]. **The one exception, approved by the owner at Gate B (2026-10-17, OQ-19):** an event-loop thread may lend its DOM arenas read-only to helper threads for one rendering stage. It holds the heap's borrow for the whole stage, so the compiler forbids mutation and collection until every helper's borrow has ended (PL2, PL10). Parallel style and layout are built only when profiling justifies them. **A second exception, from Round 1** (RV1-04; the owner: "parallel marking in pauses"): during a collection pause, the event-loop thread may lend its heap's arenas read-only to helper threads that mark them, with mark state in atomic bitmaps outside the arenas. The thread holds the heap's borrow for the pause, so nothing mutates or allocates until every helper's borrow has ended; the helpers never allocate, write a field, or move an object (SH10) |
| EL12 | **Shared mutable memory exists in three places only:** `SharedArrayBuffer` storage, outside every heap (ADR-0003); the infrastructure's own queues; and atomic flags such as the interrupt flag (EL17) |
| EL13 | **Memory written by different threads is padded to 128 bytes,** the M1's cache line (macOS note item 16) |

### 4.4 The HTML event loop

| # | Rule |
|---|---|
| EL14 | **The HTML event loop is a scheduler owned by one thread loop.** It holds the agent's task queues, microtask queue, timer heap and rendering state. The thread loop wakes it with messages and deadlines |
| EL15 | **One event loop per agent, and one thread per event loop.** Each similar-origin window agent and each worker agent has its own event loop, on its own thread, with its own heap. No two window or worker agents share a thread. Worklet agents run on threads their host chooses (§4.6), which ECMAScript allows because worklets can't block [E1, H2] |
| EL16 | **Task selection** (the specification leaves it to the implementation [H1]): (1) discrete input from the user-interaction task source; (2) the rendering update, when a frame has been offered and the previous one acknowledged (§4.5); (3) all other tasks, oldest first across task sources; (4) idle tasks, such as `requestIdleCallback` and incremental collection work (CP10), only during idle periods. Each class has a starvation bound, so continuous input can't starve rendering and rendering can't starve ordinary tasks. Step 6 sets the bounds |
| EL17 | **No nested event loops.** "Pausing", as in `alert()`, blocks only that event loop's thread while the browser shows a dialog for that tab. Algorithms that "spin the event loop" are written as resumable states, not as loops re-entered from inside a task. **Interrupts:** a running script can be stopped through an atomic flag that the control thread sets at the browser's request, and that the interpreter checks at safe points (CP10) |
| EL18 | **Timers** live in each event loop's timer heap. The thread loop arms only the earliest deadline among its event loops, with leeway (macOS note item 9). Visible documents get small leeway; hidden documents are aligned to coarse boundaries. The initial levels follow Chrome's tested scheme (§3): about once a second when hidden, and once a minute after five minutes hidden for chained timers in silent pages without WebRTC. Pages playing audio are exempt. Step 6 sets the final values |

**Continuous input.** Pointer moves, wheel and touch moves are coalesced and dispatched at the start of the next rendering update. Discrete input (keys, clicks, pointer down and up) is dispatched at once, together with any pending continuous events, so the relative order is kept. Chrome has done this since 2017 [C5], and Ladybird drains input inside its rendering update [L1]. The coalesced events stay available through `getCoalescedEvents()`.

**Why one thread per event loop** (inference; alternative A in §5):
- **It matches the specifications.** ECMAScript lets agents share a thread only if none of them can block (§2). Window agents can't block, but workers can, so workers need their own threads anyway; giving window agents their own threads too makes the model uniform.
- **Same-site tabs don't block each other.** Renderers are reused for one site (SI6), so two unrelated tabs of one site can share a process. With a thread each, a long task in one tab doesn't freeze the other.
- **QoS follows visibility per agent,** not per process (EL23).
- **Every heap stays on one thread for life** (AC5).
- **The cost is small.** A thread's stack is reserved virtually and backed by memory only as it is used (background). Event-loop threads get large reservations, because script and layout recurse deeply.

### 4.5 Frame scheduling

| # | Rule |
|---|---|
| EL19 | **The compositor owns one display link per window,** obtained from the window's content view through AppKit and added to the compositor thread's run loop (macOS note item 3). Chromium adds its display link to a non-main thread's run loop the same way [C3]. The link is **paused** whenever no visible frame in that window has asked for a frame (macOS note item 4) |
| EL20 | **Event loops subscribe for frames,** per frame (navigable) that they render, with a reason (an animation, `requestAnimationFrame`, a pending update) and a preferred rate. They unsubscribe when they have nothing more to draw. Hidden and occluded frames are never offered frames |
| EL21 | **Each vsync, the compositor offers a frame** to the subscribed event loops whose frames are visible: a sequence number, the vsync timestamp and the target presentation time. The event loop runs "update the rendering" and replies with that frame's display-list update, or with "no change". **At most one offer is outstanding per event loop.** The compositor never waits for a renderer: it composites the newest update it has from each frame, so a late renderer shows its previous content, while compositor-driven scrolling and animation continue (CP7) |
| EL22 | **Frame times are monotonic and come from the display.** The time an event loop receives becomes `requestAnimationFrame`'s timestamp; delayed or reordered deliveries are clamped so that time never runs backwards, as Ladybird does [L1]. The display link's rate is the highest rate any subscriber asked for, within the display's range. Low Power Mode, thermal pressure and occlusion lower it (macOS note items 5 and 15) |

**Within one renderer,** each event loop subscribes for its own frames. A cross-site frame rendered in its embedder's process (SI3, at L1 and non-sensitive L2) still has its own subscription and its own display list, so moving it out of process changes nothing here (AC1, SI4).

### 4.6 Agents, agent clusters and workers (OQ-5, first half)

**What "one agent cluster per process" means.** The [decision map](../decision-map.md) §4 and ADR-0004 summarize SI7 as "one agent cluster per process". Precisely: **an agent cluster never spans processes**, because its agents may share memory. A renderer may host several agent clusters of its own site, since same-site processes are reused (SI6). The exception is a cross-origin isolated browsing context group, whose clusters get processes of their own (SI7).

| Agent | Thread | Process |
|---|---|---|
| Similar-origin window agent | Its own event-loop thread | A renderer locked to its site (from L2) |
| Dedicated worker | Its own thread | Its owner's process (SI7) |
| Shared worker | Its own thread | A renderer locked to its site, which the browser chooses |
| Service worker | Its own thread | A renderer locked to its site, which the browser chooses |
| Worklets (audio, paint) | Chosen by their host: the real-time audio thread for audio worklets; the rendering stages for paint worklets | Their owner's process. Designed with those features, later |

- **Messages between agents** (`postMessage`, `MessagePort`) travel over in-process channels between threads, or over IPC between processes. Their payloads are structured-cloned into the receiving heap, and transferred objects move ownership of their backing stores (CP10).
- **`SharedArrayBuffer` memory** lives outside every heap and is mapped into one process, where the agents of one cluster share it (ADR-0003; CP10).
- **Blocking:** `Atomics.wait` is allowed only on dedicated and shared worker threads [H2]. A window agent's thread blocks only to pause (EL17) or for a bounded synchronous call (IR6).

### 4.7 Hangs

- The browser measures how long each renderer takes to acknowledge input. It never waits (IR5); it only notices.
- Past a first deadline, it offers to stop the page. Stopping sets the interrupt flag through the renderer's control thread (EL17).
- Past a second deadline, it ends the process, and the tab shows the hang page ([process model](../security/process-model.md) §8).
- Step 6 sets the deadlines.

### 4.8 Quality of service and priority

| # | Rule |
|---|---|
| EL23 | **QoS follows role and visibility,** using public interfaces only (macOS note items 6–8): |

- **Browser threads:** main, core and compositor at user-interactive; the worker pool at utility, or user-initiated while the user waits.
- **Renderer event-loop threads:**
  - user-interactive while the agent has a visible frame;
  - utility while all of its frames are hidden;
  - background after a long time hidden.
- **Whole renderer processes:** when all of a renderer's frames are hidden, the browser sets its task role to `TASK_BACKGROUND_APPLICATION`. After a long time hidden, it puts the process in darwin-background with `setpriority(PRIO_DARWIN_PROCESS, pid, PRIO_DARWIN_BG)`. Pages playing audio are exempt.
- **Input carries urgency.** A message that carries input is sent with the sender's QoS where the transport allows it (macOS note item 7). CP4 checks how Mach handles this for one-way messages.
- **Throttling covers a whole agent cluster.** ECMAScript forbids leaving some of a cluster's agents running while others are suspended indefinitely [E1]. So a document's dedicated workers are throttled or frozen with it.
- **Placement on the cores follows QoS.** XNU's scheduler places each thread by the cluster that the performance controller recommends for its QoS [X1], and Apple says background work is "more likely" to run on the efficiency cores [A1].
- **Apple's target** for idle time, utility QoS or below at least 90% of it [A2], is a design check for Step 6.

### 4.9 Determinism and tests

| # | Rule |
|---|---|
| EL24 | **The thread loop is an interface:** a wait source and a clock. In production it is kqueue or a run loop, with the host clock. In tests, one deterministic scheduler with a **virtual clock** drives every event loop, including those of other "processes", on one thread in a defined order, and can record and replay that order. Single-process mode (a test mode since Round 1, RV1-07) runs the production threads in one process, connected by in-process channels (R6).<br>**Which tests use it** (**as amended in Round 2,** RV2-03): time-dependent and scheduler tests, in single-process mode; conformance runs in real processes on the host clock, where this scheduler can't reach the helpers (TS2) |

### 4.10 Memory

**Inference**, to be measured:
- Threads cost little memory: stacks are virtual reservations, backed only as used.
- A renderer with one window agent has three threads besides its helpers: the control thread, the event-loop thread and its kqueue. The renderer target, B2 at 10 MB, is dominated by the heap and engine state, not by threads.
- A per-process cap on threads, including dedicated workers, is a resource limit that Step 6 sets.

### 4.11 Staging

| Level | What changes in this design |
|---|---|
| L0 | **As amended in Round 1** (RV1-07): renderer and network threads run in their own sandboxed processes from M1 and talk over Mach channels; the compositor's threads run in the browser. Single-process mode, with every thread in one process, exists strictly for automated tests, fuzzing and Miri |
| L1 | Audit tokens, crash containment and the GPU/compositor process (M2, RV1-02). Thread roles and messages are unchanged |
| L2 | Site locks. No change here |
| L3 | Out-of-process iframes. Each frame's event loop runs in the process of its own site. No change here: a frame already has its own subscription and display list (§4.5) |
| GPU/compositor process (M2) | Validation and rasterizing move from the browser's compositor thread to the GPU process; presenting (CoreAnimation commits), scrolling, animation and input stay in the browser. Frame scheduling is unchanged |

## 5. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **One main thread per renderer, shared by every window agent in it** (Chromium) | Fewer threads, but a long task in one tab stalls same-site tabs in the same process, and QoS can only be set per process. Rejected for EL15 |
| B | **A central I/O thread per process** that receives all IPC and forwards it (Chromium's IO thread) | An extra hop on every message. Its real benefit, handling a request while an event loop is busy, comes from the renderer's control thread instead. Rejected for EL4 |
| C | **Grand Central Dispatch queues as event loops** | GCD's threads are anonymous and shared, while heap confinement depends on one heap per thread. Rejected for event loops. Leaf jobs go to Axiom's own bounded pools, whose QoS Axiom sets (EL9) |
| D | **Browser logic on AppKit's main thread** (Chromium, Firefox and WebKit do this) | Simpler, but the engine then competes with the shell's UI and AppKit's nested loops, and needs AppKit to run tests. Rejected for the core thread (EL6) |
| E | **A compositor thread in each renderer** (Chromium) | Needed when the renderer composites. In Axiom the compositor composites, in the GPU/compositor process from M2, and the browser scrolls (process model §3), so renderers don't need one. CP7 revisits this if it changes |
| F | **A fixed-rate frame timer in each renderer** | Costs wakeups while idle (AC3) and can't follow variable refresh rates. Rejected for EL19–EL22 |

## 6. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | Thread roles and messages are identical from L0 to L3 (§4.11); frames subscribe and paint individually (§4.5) |
| AC2 | The core thread holds the frame tree and decides; renderers only request (EL6) |
| AC3 | EL1–EL3 and EL19: no thread wakes without a reason, and display links are paused when unneeded |
| AC5 | EL10–EL12, EL15 |
| AC6 | EL24 |
| AC7 | §4.10 |
| AC9 | Thread loops need FFI (kqueue, run loops, Mach) only in `platform` |

## 7. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Starvation bounds for EL16, timer leeway values, and hang deadlines | Step 6 (performance budgets) |
| 2 | Does QoS travel with one-way Mach messages? (macOS runtime note §12, question 2) | CP4 |
| 3 | Do style and layout run in parallel on the helper pool, and on what data? | CP6 |
| 4 | Which thread first receives each kind of input, the compositor or the core? | CP8 |
| 5 | Process choice for shared and service workers | CP5 |

## Sources

All checked on 2026-10-17. Engine source was read at the head of each main branch, which isn't pinned to a commit.

**Specifications:**
- [H1] WHATWG HTML, §8.1.7 "Event loops" (Last Updated 16 October 2026) — https://html.spec.whatwg.org/multipage/webappapis.html#event-loops
- [H2] WHATWG HTML, §8.1.2 "Agents and agent clusters" — https://html.spec.whatwg.org/multipage/webappapis.html#integration-with-the-javascript-agent-formalism
- [H3] WHATWG HTML, §8.7 "Timers" — https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#timers
- [H4] W3C, "Cooperative Scheduling of Background Tasks" (`requestIdleCallback`) — https://w3c.github.io/requestidlecallback/
- [E1] ECMA-262 draft (16 October 2026), §9.5 "Jobs", §9.6 "Agents", §9.7 "Agent Clusters", §9.8 "Forward Progress" — https://tc39.es/ecma262/multipage/executable-code-and-execution-contexts.html

**Apple and XNU:**
- [A1] "Tuning your code's performance for Apple silicon" (undated) — https://developer.apple.com/documentation/apple-silicon/tuning-your-code-s-performance-for-apple-silicon
- [A2] Energy Efficiency Guide for Mac Apps, "Prioritize Work at the Task Level" (archived; updated 2016-09-13) — https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/PrioritizeWorkAtTheTaskLevel.html
- [A3] Threading Programming Guide, "Run Loops" (archived; updated 2014-07-15) — https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/RunLoopManagement/RunLoopManagement.html
- [X1] XNU, `doc/scheduler/sched_clutch_edge.md`: the performance controller recommends a preferred cluster "per QoS within the thread group" — https://github.com/apple-oss-distributions/xnu/blob/main/doc/scheduler/sched_clutch_edge.md

**Chromium:**
- [C1] "RenderingNG architecture" (last updated 2021-07-26) — https://developer.chrome.com/docs/chromium/renderingng-architecture
- [C2] "Overview of task scheduling in Blink" — https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/platform/scheduler/TaskSchedulingInBlink.md
- [C3] `ui/display/mac/`: display links, including one added to a non-main thread's run loop ([macOS runtime note](../research/macos-runtime-facilities.md), source C3)
- [C4] "Heavy throttling of chained JS timers beginning in Chrome 88" (2021-01-18) — https://developer.chrome.com/blog/timer-throttling-in-chrome-88
- [C5] "Aligned input events" (last updated 2017-06-22) — https://developer.chrome.com/blog/aligning-input-events

**Firefox:**
- [F1] MDN, `setTimeout()`, "Reasons for longer delays than specified" (modified 2026-09-03) — https://developer.mozilla.org/en-US/docs/Web/API/Window/setTimeout
- [F2] `widget/cocoa/nsAppShell.mm` ([macOS runtime note](../research/macos-runtime-facilities.md), source F2)
- [F3] Firefox source docs: "Graphics Overview" and "Layout Overview" (May 2024) — https://firefox-source-docs.mozilla.org/gfx/RenderingOverview.html ; https://firefox-source-docs.mozilla.org/layout/LayoutOverview.html

**WebKit:**
- [W1] `Source/WebKit/UIProcess/DisplayLink.cpp`; `Source/WebKit/WebProcess/WebPage/DrawingArea.messages.in` ([macOS runtime note](../research/macos-runtime-facilities.md), sources W1 and W2)
- [W2] `Source/WTF/wtf/cf/RunLoopCF.cpp` ([macOS runtime note](../research/macos-runtime-facilities.md), source W4)

**Ladybird:**
- [L1] `Libraries/LibWeb/HTML/EventLoop/` (`EventLoop.cpp`, `EventLoop.h`, `TaskQueue.cpp`, `Task.h`, `FrameCompletion.cpp`) — https://github.com/LadybirdBrowser/ladybird/tree/master/Libraries/LibWeb/HTML/EventLoop

**Servo:**
- [S1] Servo book, "Architecture" (`src/design-documentation/architecture.md`; last changed 2026-07-31, by a typo fix, and partly out of date) — https://github.com/servo/book/blob/main/src/design-documentation/architecture.md
- [S2] Servo, `components/constellation/constellation.rs` (main at `fc18b55`, 2026-10-17) — https://github.com/servo/servo/blob/main/components/constellation/constellation.rs
