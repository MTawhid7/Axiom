# macOS Facilities for the Runtime: Run Loops, Frame Timing, Threads, Surfaces, Input and Accessibility

| | |
|---|---|
| **Status** | Complete (2026-10-17, Step 4 CP2). ADRs 0005–0007 and 0009, which rest on it, were accepted at Gate B (2026-10-17) |
| **Question** | Which macOS facilities will Axiom's event loops, frame scheduling, threads, compositor, input routing and accessibility rest on, and what do those facilities constrain? |
| **Feeds** | Step 4 ([tracker](step-4-architecture-proposal.md)): CP3 (event loop and threads), CP4 (IPC), CP6 (pipeline and compositing), CP8 (input and accessibility), CP9 (embedder API and shell) |
| **Builds on** | [macOS confinement note](macos-process-confinement.md) §2.5–§2.7 (launch, rendezvous, IOSurfaces, memory pressure); [S2 results](s2-results.md) (idle cost of an AppKit window); [S4 results](s4-results.md) (IPC cost) |
| **Last updated** | 2026-10-17 |

## Summary

- **Waiting.**
  - The browser's main thread runs AppKit's run loop. AppKit also runs that loop in other modes, while it tracks menus, drags and live resizes or shows a modal panel. A source registered only in the default mode doesn't fire during those periods (§3.1).
  - Helpers have no AppKit. One `kevent64` call can wait on a Mach port set, timers and descriptors at once, and can receive the Mach message during the wait (§3.2).
  - Either way, an idle thread sleeps in the kernel with no wakeups. S2 measured exactly that for an idle AppKit window.
- **Frame timing.**
  - `CVDisplayLink` is deprecated since macOS 15. Its replacement, `CADisplayLink`, is vended only by `NSView`, `NSWindow` and `NSScreen` (macOS 14 and later), fires on every vsync until paused, and stops for hidden views (§4).
  - Renderers have no AppKit, so **the browser must own the frame clock and send frame signals to renderers** (inference).
  - Precedent agrees (§10). WebKit's UI process owns the display link and sends `DisplayDidRefresh` to web processes. Firefox's UI process forwards vsync to its GPU process. Chromium creates a display link outside AppKit only through a private function.
- **Refresh rate.** This Mac's display runs at 60 Hz (observed), but `NSScreen` reports variable refresh ranges, so nothing may assume 60 Hz (§4.1).
- **Quality of service** (§5).
  - A QoS class sets CPU, I/O, timer-latency and network priority together. Only the two user-facing classes seek peak throughput; the others seek energy efficiency even without contention.
  - On Apple silicon, background work is "more likely" to run on the lower-performance cores. Apple doesn't say "only".
  - Mach messages can carry the sender's QoS and importance to the receiver; socket pairs can't (inference). This is a new input to OQ-17.
- **Process priority.** A task role (`task_policy_set`) or `PRIO_DARWIN_BG` on a whole process lowers a background renderer's priority. Chromium uses task roles, plus a suppression policy taken from a private header (§5.3, §10).
- **Surfaces** (§6.1).
  - IOSurfaces cross processes only as Mach ports. Global surfaces are deprecated as "insecure".
  - Since macOS 14.4, `IOSurfaceSetOwnershipIdentity` can charge a surface's memory to another task, named by an identity token that is itself a Mach port. **Hypothesis:** this keeps per-renderer memory accounting honest when the browser allocates surfaces for a renderer.
- **Layers** (§6.2, §6.3).
  - Core Animation commits implicit transactions when the thread's run loop next iterates.
  - A public cross-process layer API exists (`CARemoteLayerServer` and `CARemoteLayerClient`), but Chrome uses the private `CAContext` and `CALayerHost` instead (observed). Axiom's renderers send display lists, so layers across processes matter only for a future GPU process (inference).
- **IME and accessibility are synchronous queries on the browser's main thread** (§7, §8). `NSTextInputClient` asks for text and geometry synchronously, and assistive technologies query the `NSAccessibility` API. The browser must answer both from state it already holds, never by blocking on a renderer (inference from IR5).
- **Power signals.** Low Power Mode, thermal state and window occlusion are all observable: inputs to frame-rate and throttling policy (§9).
- **This Mac** (observed): 4 performance and 4 efficiency cores, 128-byte cache lines, 16 KiB pages (§2).

## 1. Scope and method

**Scope.** This note covers the facilities that Step 4's runtime design depends on:
- run loops and waiting;
- frame timing;
- threads and their quality of service;
- surfaces and presentation across processes;
- input and text input;
- accessibility;
- power and visibility signals.

Sandboxing, launching and the IPC rendezvous are covered by the [macOS confinement note](macos-process-confinement.md) and are not repeated.

**Method.**
- **Environment:** MacBook Air (M1), macOS 27.2, Command Line Tools SDK 27.0.
- **Local evidence, read with system tools only:**
  - SDK headers, cited by file;
  - `sysctl` and `system_profiler`;
  - `nm -u` (imported symbols) and `grep -a` (embedded selector and class names) on two installed browsers: Chrome Canary 158.0.8108.0, and Tor Browser Alpha 16.0a11, which is built on Firefox.

  Nothing was compiled or installed. Safari's WebKit lives in the dyld shared cache, so its binaries weren't inspected; WebKit is cited from its source instead.
- **Web sources:** Apple's documentation, and the Chromium, WebKit, Firefox and Ladybird source trees (main branches, checked 2026-10-17). Each is listed under Sources.
- **Labels** follow the [research conventions](README.md). "Observed" marks a fact read on this Mac, with the file or command given.

## 2. This Mac

| Property | Value | Source |
|---|---|---|
| CPU | Apple M1. Two performance levels: 4 "Performance" cores sharing a 12 MiB L2, and 4 "Efficiency" cores sharing a 4 MiB L2 | Observed: `sysctl hw.perflevel0.*`, `hw.perflevel1.*` |
| Cache line | 128 bytes | Observed: `sysctl hw.cachelinesize` |
| Page size | 16 KiB | Observed: `sysctl hw.pagesize` |
| Memory | 8 GiB | Observed: `sysctl hw.memsize` |
| GPU | 7 cores; "Metal 4" | Observed: `system_profiler SPDisplaysDataType` |
| Display | Built-in Retina LCD, 2560 × 1600 panel, used at "1440 x 900 @ 60.00Hz" with a 2880 × 1800 backing store | Observed: `system_profiler -json SPDisplaysDataType` |

**Inference.**
- **16 KiB pages** are the smallest unit of shared memory and of memory protection. Every separately shared region costs at least 16 KiB, which matters for IPC shared memory (IR7) and for the renderer's baseline footprint (B2).
- **128-byte cache lines** set the padding needed to keep data written by different threads from sharing a line.
- **The display is scaled:** content drawn for 1440 × 900 points at 2× is resampled to the panel. Device-pixel geometry in the pipeline must come from the backing scale and the display, never from assumptions.

## 3. Run loops and waiting

### 3.1 The browser's main thread: AppKit's run loop

- **AppKit runs the main thread's `CFRunLoop`.** It delivers events and fires sources, timers and observers (background).
- **Run loop modes.** AppKit names `NSEventTrackingRunLoopMode` and `NSModalPanelRunLoopMode` (observed, `AppKit/NSApplication.h`). It runs the loop in event-tracking mode while it tracks menus, mouse drags and live resizing, and in the modal mode while a modal panel is up (background). **A source added only to the default mode doesn't fire in those modes.** Adding a source to `kCFRunLoopCommonModes` (observed, `CFRunLoop.h`) makes it fire in every common mode.
- **Mach ports as sources.** A version-1 run-loop source wraps a Mach port: `CFRunLoopSourceContext1` has a `getPort` callback (observed, `CFRunLoop.h`). `CFMachPortCreateRunLoopSource` does the same. Chrome imports it, along with `CFRunLoopSourceCreate` and `CFRunLoopObserverCreate` (observed, `nm -u`).
- **Observers** can run code at given points of each pass, among them `kCFRunLoopBeforeSources`, `kCFRunLoopBeforeWaiting` and `kCFRunLoopAfterWaiting` (observed, `CFRunLoop.h`).
- **Core Animation commits at the end of the pass.** "Implicit transactions are created automatically by CoreAnimation when the layer tree is modified by a thread without an active transaction. They are committed automatically when the thread's run-loop next iterates. In some circumstances (i.e. no run-loop, or the run-loop is blocked) it may be necessary to use explicit transactions" (observed, `QuartzCore/CATransaction.h`).

**Inference.**
- Engine work that runs on the browser's main thread must be registered in the common modes. Otherwise IPC and compositing stall whenever the user holds a menu open or resizes a window.
- AppKit can nest run loops (menus, modal panels), so long-running engine work belongs on threads AppKit doesn't own: the IPC thread and the compositor (CP3).

### 3.2 Helpers: waiting without AppKit

Renderers, the network process and other helpers link no AppKit (ADR-0004). Each has three ways to wait:

| Mechanism | What it waits on | Evidence |
|---|---|---|
| **`kevent64` on a kqueue** | `EVFILT_MACHPORT` on a Mach **port set**; `EVFILT_TIMER`; descriptors. With `MACH_RCV_MSG` in the filter flags, the kernel **receives the message during the wait**, into a buffer the filter names, and returns `mach_msg`'s result code | Observed, `sys/event.h`: "Only portsets are supported at this time"; "directs the kevent64() call to attempt to receive the message during kevent delivery, rather than just indicate that a message exists" |
| **`mach_msg` receive on a port set, with a timeout** | Mach messages only; the timeout serves as the next timer | Background. S4's helper used `mach_msg` |
| **Grand Central Dispatch** | Dispatch sources: `DISPATCH_SOURCE_TYPE_MACH_RECV`, timers, `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` | Observed, `dispatch/source.h`. Its threads come from a system pool (background) |

Chrome and Firefox both import `kevent64`, `mach_msg` and `dispatch_source_create` (observed, `nm -u`).

**Inference.** A kqueue gives each helper thread one wait primitive for IPC, timers and file descriptors. It keeps the thread's identity and QoS under the engine's control, which GCD's pool does not. Either design sleeps with zero wakeups while idle (AC3).

### 3.3 Timers and coalescing

macOS lets every timer declare how late it may fire, so that wakeups can be merged:
- `CFRunLoopTimerSetTolerance` (macOS 10.9);
- dispatch timers take a leeway, and `DISPATCH_TIMER_STRICT` asks for the minimum;
- kqueue timers take `NOTE_LEEWAY` ("leeway for power aware timers"), `NOTE_CRITICAL` ("minimal timer coalescing") or `NOTE_BACKGROUND` ("maximum timer coalescing").

All observed in `CFRunLoop.h`, `dispatch/source.h` and `sys/event.h`.

**Apple's guidance:** "A general guideline is to set the tolerance to at least 10 percent of the interval for a repeating timer." A timer with tolerance "may fire anytime between its scheduled fire date and the scheduled fire date, plus the tolerance. The timer won't fire before the scheduled fire date" [A3].

## 4. Frame timing

### 4.1 Display links on macOS 27

- **`CVDisplayLink` is deprecated:** `API_DEPRECATED_BEGIN("use NSView.displayLink(target:selector:), NSWindow.displayLink(target:selector:), or NSScreen.displayLink(target:selector:) ", macos(10.4, 15.0))` (observed, `CoreVideo/CVDisplayLink.h`).
- **Its replacement is `CADisplayLink`, vended by AppKit objects only.** `-displayLinkWithTarget:selector:` exists on `NSView`, `NSWindow` and `NSScreen` from macOS 14. CADisplayLink's own constructor is `API_UNAVAILABLE(macos)` (observed, `AppKit/NSView.h`, `NSWindow.h`, `NSScreen.h`; `QuartzCore/CADisplayLink.h`).
- **It runs until paused.** "Unless paused, it will fire every vsync until removed." It has a `paused` property, and reports `timestamp`, `duration` and `targetTimestamp` in host time. `preferredFrameRateRange` sets a desired rate (observed, `CADisplayLink.h`).
- **It follows visibility.** A view's display link fires "in-sync with the display the view is on. If the view is hidden, or not on any display, the callback will not be invoked." A screen's display link doesn't follow windows that move to another screen (observed, `NSView.h`, `NSScreen.h`).
- **Refresh rates vary.** `NSScreen` reports `maximumFramesPerSecond` and the minimum and maximum refresh intervals, which are equal "for displays that do not support variable refresh rates" (macOS 12; observed, `NSScreen.h`). This Mac's display runs at 60 Hz (§2).

Both installed browsers still use `CVDisplayLink`. Chrome imports `CVDisplayLinkCreateWithCGDisplay` and Firefox `CVDisplayLinkCreateWithActiveCGDisplays`, with `Start`, `Stop` and `SetOutputCallback` (observed, `nm -u`). Chrome's binary also contains the string `CADisplayLink`. Chromium's source explains it: a `CADisplayLink` made through the private `SLSGetDisplayLink`, "without requiring NSApplication or AppKit objects" [C3] (§10).

### 4.2 Who can own a display link

**Inference.** On macOS 27 a display link comes from an AppKit view, window or screen, or from the deprecated `CVDisplayLink` keyed to a CoreGraphics display. Axiom's renderers have neither AppKit nor a window-server connection (ADR-0004; [macOS confinement note](macos-process-confinement.md) §2.6). So:
- **the browser owns the display link** for each window, and sends frame signals to the renderers whose frames are visible in it;
- **the link is paused** whenever no visible frame has asked for a frame, so idle costs nothing (AC3);
- **hidden windows and views get no frames**, which AppKit's view-bound links already ensure.

## 5. Threads and quality of service

### 5.1 QoS classes

The kernel schedules threads by QoS class (observed, `sys/qos.h`):

| Class | Value | The header's intended use |
|---|---|---|
| `QOS_CLASS_USER_INTERACTIVE` | 0x21 | "critical interaction with the user such as handling events on the main event loop, view drawing, animation" |
| `QOS_CLASS_USER_INITIATED` | 0x19 | Work the user started and is waiting for |
| `QOS_CLASS_DEFAULT` | 0x15 | Between the two |
| `QOS_CLASS_UTILITY` | 0x11 | Work whose results the user isn't waiting for; run "in an energy and thermally-efficient manner" |
| `QOS_CLASS_BACKGROUND` | 0x09 | Work the user may not be aware of; run "in the most energy and thermally-efficient manner" |

- **A class covers more than CPU.** It also sets "timer latency, CPU throughput, I/O throughput, network socket traffic management behavior and more" (observed, `sys/qos.h`).
- **Efficiency without contention.** "QOS classes representing user-initiated work attempt to achieve peak throughput while QOS classes for other work attempt to achieve peak energy and thermal efficiency, even in the absence of contention" (observed, `sys/qos.h`).
- **Interfaces:** `pthread_attr_set_qos_class_np`, `pthread_set_qos_class_self_np`, and `pthread_override_qos_class_start_np` for temporary overrides (observed, `pthread/qos.h`). Chrome and Firefox both import `pthread_set_qos_class_self_np` (observed, `nm -u`).

### 5.2 The M1's two core types

Apple's guidance [A1, A2]:
- **Placement follows QoS.** "On Apple silicon, a task's QoS class influences whether the system runs that task. For example, the system is more likely to run background tasks on lower performance cores to maximize battery life." The page says "more likely", not "only", and names no class that is kept off the performance cores.
- **QoS instead of priorities:** "If you manually configure your thread's priority using pthread_setschedparam, setpriority, or thread_set_policy, transition to APIs that set QoS classes instead."
- **Block, don't spin:** "eliminate spin locks and other spin-wait code that causes your thread to hold on to a core", in favour of "an os_unfair_lock, a condition variable, or a standard mutex that lets your thread block".
- **Idle at low QoS:** "Optimally, run your app at a QoS level of utility or lower at least 90% of the time when user activity is not occurring" (archived guide, updated 2016-09-13).
- **GCD knows the core types:** it distributes work "to the appropriate core type to get the needed performance or efficiency".

### 5.3 Process-level controls

| Control | Effect | Evidence |
|---|---|---|
| **Task role**, `task_policy_set(TASK_CATEGORY_POLICY)` | Tells the kernel a task's role: `TASK_FOREGROUND_APPLICATION`, `TASK_BACKGROUND_APPLICATION`, `TASK_DARWINBG_APPLICATION`, `TASK_DEFAULT_APPLICATION` and others. "The role is not inherited from the parent" | Observed, `mach/task_policy.h`. Chrome imports `task_policy_set` (observed, `nm -u`) |
| **Darwin background**, `setpriority(PRIO_DARWIN_PROCESS, pid, PRIO_DARWIN_BG)` | "lowers CPU, disk IO, and networking priorites until thread terminates or 'background' state is revoked". `PRIO_DARWIN_PROCESS` takes a process ID; `PRIO_DARWIN_THREAD` applies to the calling thread | Observed, `sys/resource.h` |
| **Activities**, `NSProcessInfo beginActivityWithOptions:` | Declares user-initiated or latency-critical work ("reserved for activities like audio or video recording"), and can hold off idle sleep | Observed, `Foundation/NSProcessInfo.h` |
| **Power assertions**, `IOPMAssertionCreateWithName` | Keeps the display or system awake, for example during video playback | Chrome and Firefox import it (observed, `nm -u`) |

### 5.4 Priority across IPC

Mach messages can carry priority (observed, `mach/message.h`):
- `MACH_SEND_PROPAGATE_QOS`: "IPC should propagate the caller's QoS";
- `MACH_SEND_OVERRIDE`: "priority override for send";
- importance: a message can raise the receiver's importance (`MACH_MSGH_BITS_RAISEIMP`, "importance raised due to msg"), unless sent with `MACH_SEND_NOIMPORTANCE`;
- `MACH_RCV_VOUCHER`: the receiver accepts the voucher that carries this context.

**Inference.** A user-interactive browser thread can pass its urgency along with a message, so a renderer thread that wakes for input doesn't run at a stale background priority. Unix-domain sockets carry no such context. This is a further input to the transport decision (OQ-17, CP4). How each option behaves for asynchronous one-way messages needs checking at CP4.

**Real-time work.** `os_workgroup` (macOS 11) groups threads with real-time deadlines, such as audio (observed, `os/workgroup.h`). It is relevant to Web Audio later, not to Step 4.

## 6. Surfaces and presentation

### 6.1 IOSurfaces

This extends the [macOS confinement note](macos-process-confinement.md) §2.6 (all observed, `IOSurface/IOSurfaceRef.h` and `IOSurfaceAPI.h`):
- **Only as Mach ports.** `kIOSurfaceIsGlobal` is deprecated as "Global surfaces are insecure" (macOS 10.6–10.11). `IOSurfaceCreateMachPort` is "useful if you need to atomically or securely pass an IOSurface to another task without making the surface global". An XPC form also exists (`IOSurfaceCreateXPCObject`).
- **Lifetime.** "Any live mach ports created from an IOSurfaceRef implicity increase the IOSurface's global use count by one until the port is deleted." `IOSurfaceIsInUse` "is the only call that should be used" to decide whether a surface is still in use, for example before reusing a buffer.
- **Purgeable.** `IOSurfaceSetPurgeable` marks contents volatile, so the kernel may discard them under memory pressure, "pretty much exactly like the Mach vm_purgeable object stuff". This suits caches of rendered content.
- **Ownership.** `IOSurfaceSetOwnershipIdentity(buffer, task_id_token, ledger_tag, options)` (macOS 14.4) moves a surface's memory to another task's ledger, with tags such as `kIOSurfaceMemoryLedgerTagGraphics` and a `NoFootprint` flag. The target is named by a `task_id_token_t`, which is a Mach port, created with `task_create_identity_token` (observed, `mach/mach_types.h`, `mach/task.h`).

**Hypothesis.** When the browser allocates surfaces on a renderer's behalf, it can charge them to that renderer, so that the renderer's footprint, and B2, B3 and B7, stay honest. Whether this needs an entitlement isn't known; to be tested if the design relies on it. Neither installed browser imports it (observed).

### 6.2 Core Animation in the presenting process

- Implicit transactions commit when the thread's run loop next iterates; without a run loop, or with a blocked one, explicit transactions are needed (§3.1).
- **Inference.** A compositor that presents from its own thread must commit Core Animation transactions explicitly. A compositor on the main thread is subject to AppKit's nested loops (§3.1).

### 6.3 Layers across processes

- **Public API:** `CARemoteLayerServer` and `CARemoteLayerClient` (macOS 10.7; observed, `QuartzCore/CARemoteLayer*.h`). The server is a per-process singleton with a Mach port that "should be sent to other processes that want to export layer trees into the server". A client created from that port exports its root layer, and the server shows it with `+[CALayer layerWithRemoteClientId:]`.
- **Private API:** Chrome imports the classes `CAContext` and `CALayerHost` (observed, `nm -u`). Neither appears in the SDK's headers (observed, `grep`). Chromium declares them itself: the process that renders creates a `CAContext` from its window-server connection, and the process that displays creates a `CALayerHost` with that context's ID [C4].

**Hypothesis.** A remote-layer client must reach Core Animation's render server, a Mach service that Axiom's renderer profile denies. Chromium's private route needs a window-server connection in the producer [C4]. Under ADR-0004, renderers send display lists to the browser, which composites them, so cross-process layers matter only for a future GPU process (CP6).

### 6.4 GPU synchronization across processes

`MTLSharedEvent` can be shared between processes through `newSharedEventHandle`. The handle is an `NSSecureCoding` object, and a device rebuilds the event with `newSharedEventWithHandle:` (macOS 10.14; observed, `Metal/MTLEvent.h`, `MTLDevice.h`). Chrome imports `MTLSharedEventListener` (observed). This matters once a GPU process exists (CP6).

## 7. Input and text input

- **Events arrive on the browser's main thread** as `NSEvent`s (background). Besides keys and pointer events, they include magnify, rotate, smart-magnify, swipe and pressure events (observed, `AppKit/NSEvent.h`).
- **Scroll gestures have phases.** Scroll events carry `phase` (began, changed, ended, may-begin…), `momentumPhase` for inertial scrolling after the fingers lift, and `hasPreciseScrollingDeltas` for trackpads (macOS 10.7; observed, `NSEvent.h`). The old begin- and end-gesture events are deprecated in favour of the phase.
- **Text input is a protocol with synchronous queries.** The input context drives a view through `NSTextInputClient`. Its required methods include the edits (`insertText:replacementRange:`, `setMarkedText:selectedRange:replacementRange:`, `unmarkText`) and synchronous queries: `selectedRange`, `markedRange`, `hasMarkedText`, `attributedSubstringForProposedRange:actualRange:`, `firstRectForCharacterRange:actualRange:` and `characterIndexForPoint:` (observed, `AppKit/NSTextInputClient.h`).

**Inference.**
- A whole scroll gesture, momentum included, can be routed to the frame chosen when it began, because the phases mark its start and end (CP8).
- The IME's queries need answers within the call. The text, the selection and the caret and composition geometry of the focused editable element must already be in the browser, kept current by the focused frame's renderer. Blocking on a renderer would break IR5 (CP8).
- Chromium shows the cost of the alternative. Its UI thread blocks for up to 1.5 s on a request to the renderer, built from two one-way messages "because Mojo sync calls do not have timeouts, and therefore they are unsuitable for use with an untrusted process" [C6].

## 8. Accessibility

- **The role-based protocols replaced the informal protocol.** `accessibilityAttributeValue:` and its relatives are `API_DEPRECATED("Use the NSAccessibility protocol methods instead (see NSAccessibilityProtocols.h)", macos(10.1,10.10))`. The protocols define roles such as `NSAccessibilityElement`, `NSAccessibilityGroup`, `NSAccessibilityButton` and `NSAccessibilityStaticText` (observed, `AppKit/NSAccessibility.h`, `NSAccessibilityProtocols.h`).
- **Notifications:** Chrome and Firefox both post changes with `NSAccessibilityPostNotification` (observed, `nm -u`). Both binaries also contain the deprecated `accessibilityAttributeValue:` selector, and Chrome's contains the name of the private class `NSAccessibilityRemoteUIElement` (observed, `grep -a`).
- **Assistive technologies query the app on its main thread** (background).

**Two precedents:**
- **Chromium:** the browser wraps a tree that the renderer sends over IPC [C7].
- **WebKit:** the web process answers accessibility queries itself, through private SPI and tokens registered with the UI process [W5].

**Inference.** As with IME, the browser answers accessibility queries from trees it holds: renderers serialize their accessibility trees, and the browser validates and merges them ([process model](../security/process-model.md) §4.2). It never blocks on a renderer to answer. WebKit's route doesn't fit: it relies on private SPI, and Axiom's renderers have neither AppKit nor a window-server connection.

## 9. Power, memory and visibility signals

| Signal | Interface | Evidence |
|---|---|---|
| Low Power Mode | `NSProcessInfo.isLowPowerModeEnabled`, and `NSProcessInfoPowerStateDidChangeNotification` (macOS 12) | Observed, `NSProcessInfo.h` |
| Thermal state | `NSProcessInfo.thermalState`, and its change notification | Observed, `NSProcessInfo.h` |
| Window occlusion | `NSWindow.occlusionState`; `NSWindowOcclusionStateVisible` is clear when the whole window is covered (macOS 10.9) | Observed, `NSWindow.h` |
| Memory pressure | `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE`: normal, warning, critical | [macOS confinement note](macos-process-confinement.md) §2.7 |
| App Nap | The system lowers an app's priority and throttles its timers and I/O when it isn't in the foreground, hasn't recently updated visible content, isn't audible and holds no power assertion. An XPC message or Apple event wakes it | [A4] |

**Inference.** These feed the frame scheduler and the throttling policy (CP3, CP7):
- an occluded window gets no frames;
- Low Power Mode and rising thermal state lower animation frame rates and stretch background timers;
- memory pressure discards background renderers, as the [process model](../security/process-model.md) §7 already specifies.

## 10. Browser precedent

All from the engines' main branches, read on 2026-10-17 (Sources). The branches aren't pinned to commits, so details may move. "Not checked" means this note didn't look.

| Facility | Chromium | Firefox | WebKit | Ladybird |
|---|---|---|---|---|
| **Main-thread run loop** | Message pumps built on `MessagePumpCFRunLoopBase`: a signalled `CFRunLoopSource` for work, a `CFRunLoopTimer` for delayed work, and observers. One runs before the loop sleeps, "performing idle-priority work". Work "not runnable within a nested run loop" is deferred [C1] | One `CFRunLoopSource` in `kCFRunLoopCommonModes`, signalled from any thread, plus an observer for before-waiting, after-waiting and exit, also in the common modes [F2] | WTF's `RunLoop` is one version-0 `CFRunLoopSource` in `kCFRunLoopCommonModes`. Its timers go in the common modes too [W4] | A `poll()` loop per thread, with a wake pipe [L1]. Whether macOS builds use it wasn't confirmed |
| **Other threads** | `MessagePumpKqueue` drives "an IO MessageLoop that is capable of watching both POSIX file descriptors and Mach ports". An empty Mach message wakes it [C2] | Content processes run "the native Gecko event loop", not `[NSApp run]` [F2] | Not checked | The same `poll()` loop |
| **Display link** | Three paths [C3]: `CVDisplayLink` as the fallback; a `CADisplayLink` created in the GPU process through the private `SLSGetDisplayLink`; or "Request BeginFrames from the browser via IPC". Without "a working WindowServer event pump", a `CADisplayLink` "may stop firing after display reconfiguration or power resume" | `CVDisplayLinkCreateWithActiveCGDisplays`, called back "on the vsync thread" [F1]. The UI process forwards vsync to the GPU process over `PVsyncBridge` [F3] | A `CVDisplayLink` per display, in the UI process, which asserts window-server access. Clients register with a frame rate, and web processes receive `DisplayDidRefresh`. The link stops after 20 callbacks with no observers, and its callback runs off the main thread [W1, W2] | Not checked |
| **Content across processes** | The private `CAContext` (in the rendering process, made from its window-server connection) and `CALayerHost` (in the displaying process) [C4] | Composites into Core Animation layers that host IOSurfaces (`NativeLayerCA`), and marks surfaces in use "to prevent other processes from recycling it" [F4] | The web process commits layer-tree transactions with backing-store handles; the UI process owns the `CALayer`s [W3] | IOSurfaces as Mach send rights, and POSIX shared memory [L2]. WebContent "paints the web content into shared bitmaps" [L3] |
| **Process priority** | `task_policy_set` with `TASK_FOREGROUND_APPLICATION` or `TASK_BACKGROUND_APPLICATION`, plus a suppression policy, "taken from task_policy_private.h", "to match the effect of App Nap" [C5] | Not checked | Not checked | Has `TaskRoleMacOS.cpp` and `ThreadQoSMacOS.cpp` (not read) |
| **IME** | The UI thread blocks for up to 1,500 ms on two one-way messages to the renderer. On timeout it returns `NSZeroRect` or `UINT32_MAX` [C6] | Not checked | Not checked | Not checked |
| **Accessibility** | The browser wraps the tree the renderer sends over IPC (`BrowserAccessibilityCocoa`, built from an `AXTreeUpdate`) [C7] | Not checked | The web process answers queries itself, through private SPI and registered tokens. "The following attributes can be handled off the main thread" [W5] | Not checked |

**Observations** (inference):
- **Firefox and WebKit register their main-thread work in the common modes,** and Chromium registers its display-link source in them [C3]. So engine work keeps flowing during menu tracking and live resize.
- **The display link always lives in a process with window-server access:** WebKit's UI process, Firefox's UI process, or Chromium's GPU process. None of the three runs one in a sandboxed content process. Chromium resorts to a private function to create one without AppKit.
- **Content processes are told when to draw.** WebKit sends `DisplayDidRefresh`; Chromium has a path that requests frames "from the browser via IPC". Content processes can register a preferred frame rate (WebKit).
- **Idle links are stopped,** after 20 empty callbacks in WebKit. Firefox enables vsync only through explicit calls.
- **Synchronous AppKit queries are the hard edge.** Chromium answers IME queries by blocking with a timeout, and WebKit answers accessibility queries from the web process through private SPI. Neither is open to Axiom: blocking violates IR5, and the SPI route needs AppKit in the renderer.

## 11. Implications for Axiom

All inference, for the checkpoints named. Later documents cite them as "macOS runtime note §11, item N".

1. **The browser's main thread does UI only.** It runs AppKit's run loop. The engine sources it needs there, such as the IPC wake-up source and the display link, go in the common modes. IPC receiving and compositing run on their own threads, which AppKit's nested loops can't stall (CP3).
2. **Helper threads wait in kqueue.** A helper thread waits in one `kevent64` call on a port set, timers and descriptors, and receives Mach messages in the same call. Idle means no wakeups (CP3, CP4).
3. **The browser owns the frame clock.** Each window's display link comes from AppKit (`NSView` or `NSWindow`), with no private API, and frame signals go to renderers over IPC. Chromium needs a private function to create a display link outside AppKit, which confirms that a process without AppKit has no public route (CP3, CP7).
4. **Frames are requested, not broadcast.** A renderer subscribes for frames only while it needs them (an animation, a pending update), with a preferred rate. The browser pauses the display link when no subscriber remains, and hidden or occluded views get no frames (AC3; CP7).
5. **No fixed 60 Hz.** The frame scheduler reads each display's refresh range. This Mac happens to run at 60 Hz.
6. **QoS follows the role of each thread.** User-interactive for the browser's main and compositor threads, and for renderer work that answers input or drives a visible animation. Utility or background for background renderers and speculative work. Apple asks for utility or lower at least 90% of the idle time. CP3's thread table records each choice.
7. **Priority can travel with messages.** Mach messages can carry QoS and importance, so a renderer thread woken by input from the browser can run at the right priority. CP4 checks how this works for one-way messages (OQ-17).
8. **Background renderers are demoted per process.** The public mechanisms come first: a task role such as `TASK_BACKGROUND_APPLICATION`, or `PRIO_DARWIN_BG`, plus timer coalescing. Chromium's extra suppression policy relies on a private header (CP3).
9. **Every timer has tolerance:** at least 10% of the interval for repeating timers, as Apple recommends. Timers in background frames are aligned and coalesced (CP3, CP7).
10. **Surfaces are capabilities.** An IOSurface moves only as a Mach send right, like every other handle (IR4). Reuse waits for `IOSurfaceIsInUse` to clear, and caches can use purgeable surfaces. Ownership identity might charge surfaces to the renderer they serve (hypothesis; tested only if the design relies on it) (CP4, CP6).
11. **A threaded compositor commits explicitly.** If the browser presents through Core Animation from a compositor thread, it uses explicit transactions; implicit ones would wait for the main run loop (CP6).
12. **Renderers never send layers.** They send display lists (ADR-0004). Remote layers, public or private, matter only for a future GPU process, which would need render-server access to produce them (CP6).
13. **IME and accessibility state lives in the browser.**
    - The focused renderer keeps the browser's copy of the text-input state and caret geometry current.
    - Renderers send accessibility tree updates.
    - The browser answers AppKit's synchronous queries from those copies, without blocking (IR5).
    - Chromium's 1.5 s blocking wait shows the cost of the alternative (CP8).
14. **Scroll gestures are latched.** Event phases let the browser route a whole gesture, momentum included, to the frame chosen when it began (CP8).
15. **Platform signals drive throttling:** occlusion, Low Power Mode, thermal state and memory pressure (CP3, CP7).
16. **Memory granularity matters.** 16 KiB pages and 128-byte cache lines shape shared-memory sizes and the layout of data shared between threads (CP4, CP10).

## 12. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Does `IOSurfaceSetOwnershipIdentity` work for an ad-hoc-signed browser charging surfaces to its sandboxed child, without an entitlement? | Only if the design relies on it. A test would need the owner's authorization |
| 2 | Does QoS propagation or importance donation apply to asynchronous one-way Mach messages, or only to synchronous request-and-reply? | **Not settled by sources** (CP4): libdispatch sends one-way messages with an override, and XNU's documentation doesn't describe the receiving side. Measured in the first implementation milestone ([IPC](../architecture/ipc.md), IP20) |
| 3 | When a renderer has been demoted (task role or `PRIO_DARWIN_BG`), does a message from the browser that carries QoS lift the receiving thread? | CP3 and CP4 |
| 4 | Do App Nap's policies reach helper processes that the browser launched with `posix_spawn`? Apple's guide describes apps, not helpers | CP3. If unknown, Axiom sets the helpers' policies explicitly |
| 5 | How does an AppKit display link behave across display reconfiguration and sleep? Chromium notes that a `CADisplayLink` "may stop firing after display reconfiguration or power resume" without a window-server event pump | CP7. The browser has a full AppKit event loop, so this may not apply |

## Sources

**Local evidence** (macOS 27.2, Command Line Tools SDK 27.0; read 2026-10-17):
- SDK headers under `MacOSX.sdk`: `CoreVideo/CVDisplayLink.h`; `QuartzCore/CADisplayLink.h`, `CATransaction.h`, `CARemoteLayerServer.h`, `CARemoteLayerClient.h`; `AppKit/NSView.h`, `NSWindow.h`, `NSScreen.h`, `NSApplication.h`, `NSEvent.h`, `NSTextInputClient.h`, `NSAccessibility.h`, `NSAccessibilityProtocols.h`; `Foundation/NSProcessInfo.h`; `CoreFoundation/CFRunLoop.h`; `IOSurface/IOSurfaceRef.h`, `IOSurfaceAPI.h`; `Metal/MTLEvent.h`, `MTLDevice.h`; `usr/include/sys/qos.h`, `pthread/qos.h`, `sys/resource.h`, `sys/event.h`, `dispatch/source.h`, `mach/task_policy.h`, `mach/message.h`, `mach/task.h`, `mach/mach_types.h`, `os/workgroup*.h`.
- Commands: `sysctl hw.*`; `system_profiler [-json] SPDisplaysDataType`; `nm -u` and `grep -a` on Chrome Canary 158.0.8108.0 (`Google Chrome Framework`) and Tor Browser Alpha 16.0a11 (`XUL`).

**Apple** (checked 2026-10-17):
- [A1] "Tuning your code's performance for Apple silicon" (undated), read in its JSON form — https://developer.apple.com/documentation/apple-silicon/tuning-your-code-s-performance-for-apple-silicon
- [A2] Energy Efficiency Guide for Mac Apps, "Prioritize Work at the Task Level" (archived; updated 2016-09-13) — https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/PrioritizeWorkAtTheTaskLevel.html
- [A3] Energy Efficiency Guide for Mac Apps, "Timers" (archived; updated 2016-09-13) — https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/Timers.html
- [A4] Energy Efficiency Guide for Mac Apps, "Extend App Nap" (archived; updated 2016-09-13): priority reduction, timer throttling and I/O throttling for apps that aren't in the foreground, haven't updated visible content, aren't audible and hold no power assertions — https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/AppNap.html
- [A5] `CADisplayLink` and `NSView.displayLink(target:selector:)` reference pages (macOS 14.0) — https://developer.apple.com/documentation/quartzcore/cadisplaylink ; https://developer.apple.com/documentation/appkit/nsview/displaylink(target:selector:)

**Chromium** (`main`, via raw.githubusercontent.com/chromium/chromium; checked 2026-10-17):
- [C1] `base/message_loop/message_pump_apple.h`
- [C2] `base/message_loop/message_pump_kqueue.h`
- [C3] `ui/display/mac/`: `display_link_mac.h`, `display_link_mac.mm`, `ca_display_link_mac.mm`, `cv_display_link_mac.mm`, `external_display_link_mac.mm`, `vsync_provider_mac.cc`
- [C4] `ui/base/cocoa/remote_layer_api.h`
- [C5] `base/process/process_mac.cc`
- [C6] `content/browser/renderer_host/text_input_client_mac.h`
- [C7] `ui/accessibility/platform/browser_accessibility_cocoa.h`

**Firefox** (`main`, via raw.githubusercontent.com/mozilla-firefox/firefox; checked 2026-10-17):
- [F1] `gfx/thebes/gfxPlatformMac.cpp`
- [F2] `widget/cocoa/nsAppShell.mm`
- [F3] `gfx/ipc/PVsyncBridge.ipdl`, `VsyncBridgeChild.cpp`, `VsyncBridgeParent.cpp`
- [F4] `gfx/layers/NativeLayerCA.h`

**WebKit** (`main`, via raw.githubusercontent.com/WebKit/WebKit; checked 2026-10-17):
- [W1] `Source/WebKit/UIProcess/mac/DisplayLinkMac.cpp`; `Source/WebKit/UIProcess/DisplayLink.h`, `DisplayLink.cpp`
- [W2] `Source/WebKit/WebProcess/WebPage/DrawingArea.messages.in`; `Source/WebKit/UIProcess/RemoteLayerTree/RemoteLayerTreeDrawingAreaProxy.h`, `RemoteLayerTreeDrawingAreaProxy.messages.in`
- [W3] `Source/WebKit/UIProcess/RemoteLayerTree/RemoteLayerTreeHost.h`, `RemoteLayerTreeNode.h`, `RemoteLayerTreeNode.mm`
- [W4] `Source/WTF/wtf/cf/RunLoopCF.cpp`
- [W5] `Source/WebKit/WebProcess/WebPage/mac/WKAccessibilityWebPageObjectMac.mm`

**Ladybird** (`master`, via raw.githubusercontent.com/LadybirdBrowser/ladybird; checked 2026-10-17):
- [L1] `Libraries/LibCore/EventLoopImplementationUnix.cpp`
- [L2] `Libraries/LibCore/MachPort.cpp`, `IOSurface.cpp`, `AnonymousBuffer.cpp`
- [L3] `Documentation/ProcessArchitecture.md`, which notes that it is "partly aspirational"
