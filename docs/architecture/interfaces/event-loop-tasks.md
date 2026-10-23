# Interface note: `event-loop`'s tasks and task ages

| | |
|---|---|
| **Status** | **Proposed** (2026-10-23), for the owner's approval at M1-CP3's gate, before any of `event-loop`'s code (CR8, ES27) |
| **Interface** | Thread loops, with the wait source and the clock that EL24 makes an interface; tasks and the HTML event loop's scheduler; each watched thread's task age; the watchdog's protocol, with the active session that PF19's amendment asks for; the interrupt flag |
| **Crates** | `event-loop` (layer 2), over `platform-event` and `platform-mach`; used by the assemblies (`browser`, `renderer`, `network`); the deterministic scheduler in `test-scheduler` implements its wait source (CR9) |
| **Settles** | PF19's amendment at M1-CP1: "M1-CP3's interface note on task ages designs the restart"; [observability](../observability.md)'s open question 5 in detail |
| **Last updated** | 2026-10-23 |

## 1. Purpose and scope

Every engine thread is a thread loop that blocks in one place (EL1). This note fixes the contract between those loops and the code they run:
- **the clock and the wait source** (§3.1, §3.2), production's and the tests' behind one interface (EL24);
- **tasks** and the HTML event loop's scheduler, generic over each assembly's own task type (§3.3; EL14–EL18);
- **task ages**: when each watched thread's current task began, one store per task (§3.4; PF19, OB7);
- **the watchdog's protocol** between the browser and each helper: pings with task ages during an active session, rest when everything is idle, and the restart (§3.5; PF19, PF16);
- **the interrupt flag** (§3.6; EL17).

**Out of scope:** the helper pools (EL9), designed with their first user, image decoding at M1-CP10, as an amendment here; the microtask checkpoint, which arrives with script at M3; QoS's details beyond each role's class (EL23), which `platform-event`'s charter carries; what each assembly does at a deadline, which PF19 states and `browser` implements.

## 2. The binding rules it implements

| Rule | What it asks of this interface |
|---|---|
| EL1–EL5 | One wait per thread; helper threads in one `kevent64` on their ports, earliest timer and in-process wakeup; Core Foundation threads in their run loop; endpoints bound to threads; threads by role |
| EL13 | Memory written by different threads padded to 128 bytes |
| EL14–EL18 | The HTML event loop is a scheduler owned by one thread loop; one event loop per agent; task selection by class with starvation bounds; no nested loops, an interrupt flag; timers with leeway, the earliest one armed |
| EL19–EL21 | Frames are offered by the compositor; at most one offer outstanding per event loop |
| EL24 | The thread loop is an interface, a wait source and a clock; tests drive every loop on one thread with a virtual clock |
| PF19 | Pings once a second; deadlines by unacknowledged input and by **task age,** one atomic store per task; for a renderer 5 s (the offer) and 30 s (ended), for a helper's thread 5 s (ended and recovered). **As amended at M1-CP1:** pings only during an active session; the cycle stops when every watched thread is idle; it restarts when the browser sends work, and when a helper's event loop leaves idle on its own, which sends one notice without importance |
| PF16 | No wakeups while idle |
| IP16, IP20, IP21 | Zero-timeout sends; the notice without importance; the control end owned by the control thread |
| OB7, OB13, OB19 | The task's trace record and its age share one timestamp; drains and health samples ride the watchdog's cycle |
| ES12, ES13 | Threads come only from `event-loop`; the clock is injected, never read directly; the interrupt flag is a TS11 primitive implemented here |

## 3. The design

### 3.1 The clock

```rust
/// A moment on the process's clock: `mach_continuous_time`'s ticks in
/// production, so every process shares one timeline (OB2); the virtual
/// clock's in tests (EL24).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);

/// The clock a process installs once, as `trace` does (ES13: nothing else
/// reads a clock).
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Instant;
    /// Nanoseconds between two instants, with the clock's timebase.
    fn nanos(&self, from: Instant, to: Instant) -> u64;
}
```

`platform-event` supplies the host clock; `test-scheduler` supplies the virtual one. The same clock feeds `trace::set_clock` (OB2, OB26), so a task's record and its age agree.

### 3.2 Thread loops

```rust
/// Code that runs on a thread loop: one method per kind of wakeup.
pub trait Handler {
    fn message(&mut self, endpoint: EndpointKey, message: ipc::Received) -> Flow;
    fn closed(&mut self, endpoint: EndpointKey) -> Flow;
    fn send_possible(&mut self, sender: SenderKey) -> Flow;
    fn timer(&mut self, timer: TimerKey) -> Flow;
    fn woken(&mut self) -> Flow;
}
pub enum Flow { Continue, Exit }

/// A thread loop's registry: what its one wait waits on (EL2, EL4).
pub trait Registry {
    /// Binds an endpoint to this thread: from now on its messages arrive here only.
    fn add_endpoint<M: ipc::Message>(&mut self, endpoint: ipc::Endpoint<M>) -> EndpointKey;
    /// Asks for the send-possible event of a sender with a backlog (IP16).
    fn watch_sender<M: ipc::Message>(&mut self, sender: &ipc::Sender<M>) -> SenderKey;
    /// Arms a deadline, with leeway (EL18). Only the earliest is armed in the kernel.
    fn set_timer(&mut self, at: Instant, leeway_nanos: u64) -> TimerKey;
    fn cancel_timer(&mut self, timer: TimerKey);
    /// A handle other threads use to wake this one (EVFILT_USER, EL2).
    fn waker(&self) -> Waker;
}

/// The one way a thread is started (ES12): a role from EL's lists, a QoS, a
/// registry to fill, and the handler to run.
pub fn spawn<H: Handler + Send + 'static>(
    role: trace::Role,
    setup: impl FnOnce(&mut dyn Registry) -> H + Send + 'static,
) -> Result<ThreadHandle, SpawnError>;
```

- **Two production drivers, one `Handler`:** a kqueue driver, whose loop is one `kevent64` call receiving a message in the same call (EL2), for every helper thread and the browser's core; and a run-loop driver for the browser's main and compositor threads, whose sources are added to their run loop in the common modes (EL3). The handler can't tell them apart.
- **The test driver** (`test-scheduler`, EL24) implements the same registry over a virtual clock and runs every thread loop of every "process" on one thread, in an order it can record, replay and permute (TS11's third check).
- **Fairness** (IP16): per wakeup, a loop handles at most a burst of messages from each endpoint, round-robin across endpoints with pending messages, so one peer can't starve another. The burst is a constant, 16 to begin with (a hypothesis, measured at M1-CP3).
- **No polling:** a loop without a timer waits with no deadline. A timer is armed only with a real deadline (EL1).

### 3.3 Tasks and the HTML event loop

```rust
/// A task, defined by the assembly: the renderer's own enum, matched
/// exhaustively where it runs (ES6). The scheduler sees only this.
pub trait Task {
    fn source(&self) -> TaskSource;
}

/// HTML's task sources that M1 uses; script's join at M3.
pub enum TaskSource {
    UserInteraction,     // discrete input (EL16, class 1)
    DomManipulation,
    Networking,
    HistoryTraversal,
    Timer,
    Idle,                // class 4: only in idle periods
}

/// One agent's HTML event loop: its queues, timers and rendering state (EL14).
pub struct EventLoop<T: Task> { /* … */ }

impl<T: Task> EventLoop<T> {
    pub fn queue(&mut self, task: T, now: Instant);
    /// Continuous input, coalesced into the next rendering update (EL16).
    pub fn queue_continuous(&mut self, input: T, now: Instant);
    /// The compositor's offer of a frame (EL21). At most one is outstanding.
    pub fn offer_frame(&mut self, offer: FrameOffer);
    /// What to do next, by EL16's classes and PF19's starvation bounds.
    pub fn next(&mut self, now: Instant) -> Next<T>;
}

pub enum Next<T> {
    /// Run this task.
    Run(T),
    /// Run "update the rendering" for this offer, after dispatching the coalesced input.
    Render(FrameOffer),
    /// Run idle work until this deadline (≤ 50 ms away, and before the next timer or frame).
    Idle { deadline: Instant },
    /// Nothing to do: wait, until the earliest timer if there is one.
    Wait { until: Option<Instant> },
}
```

- **Selection** (EL16): discrete input; then the rendering update, when a frame is offered and the previous one acknowledged; then other tasks, oldest first across sources, never reordering one source; then idle work, only in idle periods.
- **Starvation bounds** (PF19): continuous input delays a rendering update by at most two frames (33 ms); rendering and input delay ordinary tasks by at most 50 ms, after which the oldest ordinary task runs before more of either; idle work only in idle periods, with `requestIdleCallback`'s timeouts honoured when script exists.
- **Generic over the task type:** each assembly defines its tasks as an enum, so queuing allocates nothing beyond the queue's slot, and the code that runs them matches every variant (ES6). The scheduler never runs a task; the thread's handler does.
- **No nested loops** (EL17): "spinning the event loop" is a resumable state in the task that needs it, never a call back into `next`.

### 3.4 Task ages

```rust
/// One watched thread's task age: when its current task began, or 0 while
/// it is idle. Written only by its thread; read by the control thread (PF19).
#[repr(align(128))] // EL13: its own cache line
pub struct TaskAge { started: AtomicU64 }

/// The process's watched threads, for its control thread to read.
pub struct Watched { /* a fixed array of TaskAge slots */ }

impl Watched {
    /// A thread registers once; the slot is freed when the handle drops.
    pub fn register(&'static self, role: trace::Role) -> Result<AgeHandle, WatchedFull>;
    /// The oldest running task's age, and how many watched threads run one.
    pub fn oldest(&self, clock: &dyn Clock, now: Instant) -> Oldest;
}

pub struct Oldest { pub age_nanos: u64, pub busy: u8 }

impl AgeHandle {
    /// A task begins: one relaxed store of its start, the same instant its
    /// trace record carries (OB7); after idle, also the watchdog's check (§3.5).
    pub fn begin(&self, at: Instant);
    /// The thread goes idle: one relaxed store of 0.
    pub fn idle(&self);
}
```

- **Which threads are watched** (PF19): each renderer event loop; the network process's I/O and storage threads; from M2, the GPU process's threads. The browser's own threads aren't watched by this protocol; the browser can't ping itself, and a hung browser is the owner's to see.
- **One store per task:** a thread loop calls `begin` as a task starts and `idle` before it waits with nothing runnable. Between tasks it calls `begin` again, so a busy thread makes one store per task (PF19's "one atomic store per task").
- **The always-on task record** (OB7) is written by the same call, with the same instant, so a hang report's open spans and the watchdog's age come from one source.

### 3.5 The watchdog's protocol

The messages are `Control`'s (the IDL's note §3.4): `ping(sequence)`, `pong(sequence, oldest_task, busy)`, `rest()` and `wake()`.

**The browser's side,** per helper, in `browser`'s process manager:

| State | Meaning | Leaves it when |
|---|---|---|
| **Active** | Pings once a second (PF19), each helper's deadlines measured | The session goes idle: at a round where every pong says `busy = 0`, and the browser has nothing outstanding toward any helper. It sends each helper `rest()` and disarms its timer |
| **Resting** | No timer, no ping, no wakeup (PF16) | The browser sends a helper work, or a helper sends `wake()`. It re-arms its timer and pings at once |

- **An active session** (PF19 as amended): a window is open, and some watched process has work: a task running or queued, which pongs report; a navigation or load in flight, input being delivered, or a frame offered, which the browser knows as outstanding requests it hasn't had answered; or a `wake` since the last round. A window merely open on a loaded static page is not an active session.
- **Deadlines** (PF19): a renderer whose oldest task reaches 5 s gets the offer to stop the page, and at 30 s is ended; another helper whose oldest task reaches 5 s, or whose control thread leaves pings unanswered for 5 s, is ended and recovered (IR10). An unanswered ping counts as a task age growing from the last answer, so a wedged renderer reaches its deadlines too.
- **Riding the cycle:** each pong is the moment the collector drains that helper's region (OB13) and the health monitor samples its footprint, CPU and energy (OB19). Neither has a timer of its own, so both stop when the watchdog rests.

**The helper's side,** on its control thread, with a process-wide `WatchState` (watched, resting or woken):
- **On `ping`:** set watched, and answer `pong` with `Watched::oldest`.
- **On `rest`:** set resting; then, if any watched thread is busy, move to woken and send `wake()` at once. That closes the race of a task that began just after the last pong.
- **When a watched thread leaves idle** (`AgeHandle::begin` after `idle`) while resting: the first thread to move the state from resting to woken wakes the control thread (its `Waker`), which sends `wake()`: one-way, without importance (IP20), with a zero timeout (IP16). Further threads find the state woken and do nothing, so a rest period yields one notice. This is how work the browser didn't send, data from the network process at M1 or a timer from M3, restarts the watchdog, so no task runs unwatched.
- **Ordering, so no wakeup is lost:** a thread leaving idle stores its start, issues a sequentially consistent fence, then loads the state; the control thread stores resting, issues the same fence, then reads the ages. So either the thread sees resting and sends the notice, or the control thread sees the thread busy and does. A thread already busy, moving from one task to the next, only stores: it was busy before and after, so the control thread's reading can't miss it. loom checks both paths (§7).

### 3.6 The interrupt flag

```rust
/// Stops a running script at its next safe point (EL17): one per event loop,
/// set by the control thread at the browser's request, read by the interpreter.
#[repr(align(128))]
pub struct Interrupt { flag: AtomicBool }
impl Interrupt {
    pub fn request(&self);   // the control thread; Release
    pub fn take(&self) -> bool; // the event loop at a safe point; Acquire swap
}
```

M1 runs no script, so nothing takes the flag until M3's interpreter does. It is built at M1-CP3 anyway, because the renderer's control thread and its message exist from M1 (IPC §4.12), and TS11 counts it among the primitives loom checks.

## 4. Invariants, threads and ownership

1. **One wait per thread** (EL1): each thread loop blocks only in its driver's one call. A handler never waits; a synchronous request is the IDL's one marked exception (IP13).
2. **No deadline, no timer:** a loop arms the kernel only for its earliest real deadline. A resting watchdog has none (PF16).
3. **A task age has one writer,** its thread, and readers that only load it. It is 0 exactly while the thread waits with nothing runnable.
4. **No task runs unwatched** during an active session, and a rest ends at the first task that begins (§3.5's ordering).
5. **The scheduler runs no code:** `next` chooses; the handler runs. Tasks are owned values of the assembly's type.
6. **Threads come only from `spawn`,** with a role from EL's lists (ES12), and each thread's trace slot is registered under that role ([the region's note](trace-and-diagnostics-region.md) §3.3).

## 5. Errors and violations

- **A driver that can't wait** (`kevent64` failing with anything but an interruption) leaves the thread unable to work: the process logs the error and aborts, as RV1-16's failure semantics ask; the browser sees a helper's exit and recovers (IR10).
- **`spawn` past the process's thread cap** returns `SpawnError::Cap`; the cap is a resource limit, set from the event loop's §4.10 at M1-CP3.
- **`Watched::register` with every slot taken** returns `WatchedFull`, which the assembly treats as the thread cap.
- **Watchdog verdicts** aren't errors of this crate: they are the browser's actions at PF19's deadlines.
- **A `pong` or `wake` that no correct helper sends** (a sequence never pinged, a `busy` larger than the helper's thread cap) is `Violation::Terminate` in `Control`'s validator.

## 6. Efficiency

- **Idle:** no timer and no wakeup in any process once the watchdog rests (PF16). M1-CP3 measures it: an idle Axiom on a loaded static page, renderer, browser and network process, no wakeups over 30 s, with observability on.
- **Active:** one ping and one pong per helper per second, about 4 µs each by S4's round trip; one drain and one health sample riding each.
- **Per task:** one relaxed store for the age and the always-on task record, which M1-CP2 measured at 130.5 instructions and 28 cycles per record (the plan's §9).
- **Transitions:** one `wake` per rest period, and one fence each time a thread leaves idle, about the cost of the record beside it (a hypothesis, measured at M1-CP3). Back-to-back tasks pay only their store.

## 7. Tests and oracles

- **On the virtual clock** (EL24): the watchdog's deadlines (an offer at 5 s, an end at 30 s for a renderer, 5 s for a helper's thread and for unanswered pings); resting at the first idle round and restarting on work and on `wake`; the race of §3.5 under schedule exploration (TS11's third check); EL16's order and PF19's starvation bounds on queued tasks and offered frames.
- **loom:** the task-age store and `WatchState` together, permuting the begin, rest and scan to show the notice is never lost; the interrupt flag.
- **In real processes:** PF16's idle measurement (§6); a renderer whose task never yields and a network process whose storage thread stalls, each named in its hang report with the stalled thread's open spans (OB27).
- **First tier** (ES23): `TaskAge` and `Interrupt` are 128 bytes; `begin` and `idle` allocate nothing.

## 8. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Task ages as start times, read by the control thread, reported in pongs** | One store per task, an age read directly, and the browser in charge of the cycle. **Recommended** |
| B | **Progress counters** that the control thread compares between pings | Also one store per task, but a hang shows only after two pings, and the report can't say how long the task has run. Rejected |
| C | **Heartbeats sent by helpers** | The helpers would decide the cycle, so an idle helper would wake to send them, or stop and need the same restart protocol; and a heartbeat from a control thread says nothing about a stuck event loop unless it carries ages, which is design A from the other end. Rejected |
| D | **A watchdog that never rests** | Once-a-second wakeups in every process while idle, which PF16 forbids and the owner's amendment excludes. Rejected |
| E | **Tasks as boxed closures** | Any code could be a task, at an allocation per task and with no exhaustive view of what a renderer runs. Each assembly's enum gives both. Rejected |
| F | **An async executor** | A second scheduler beside the thread loops, against ES12 and NW1. Rejected |

## Sources

- The binding documents cited by rule: the [event loop](../event-loop-and-threading.md) (EL1–EL24 and §4.7), the [performance budgets](../../performance.md) (PF16, PF19), [IPC](../ipc.md) (IP13, IP16, IP20, IP21), [observability](../observability.md) (OB7, OB13, OB19, open question 5), the [engineering standards](../../technology/engineering-standards.md) (ES6, ES12, ES13, ES23), the [testing strategy](../../testing.md) (TS11).
- [S4 results](../../research/s4-results.md) §4: 3.75 µs for a 64-byte round trip over Mach.
