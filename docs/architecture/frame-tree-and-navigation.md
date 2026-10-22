# Frame Tree, Navigation and Loading

| | |
|---|---|
| **Status** | **Binding** since ADR-0007's acceptance (2026-10-17, Gate B). Written at Step 4, CP5. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): FT10 and §5, cross-site frames as separate agents from M1 (RV1-03; CP3). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): FT12's note for RV4-16 (CP6); §8 for RV4-14 (CP7) |
| **Question** | What is the browser-owned frame tree? How do renderers represent frames held by other processes? How do cross-frame operations, navigation and resource loading work, with every decision taken from the browser's own records? |
| **Answers** | Step 4 questions Q6 and Q7 ([tracker](../research/step-4-architecture-proposal.md) §3); the frame-tree part of SI4 |
| **Feeds** | ADR-0007; CP6 (per-frame display lists), CP7 (frame scheduling), CP8 (input routing and focus), CP9 (embedder API), CP10 (agents and messaging) |
| **Builds on** | [Process model](../security/process-model.md) §5 (SI1–SI7) and §6 (IR1–IR10); [threat model](../security/threat-model.md) §8 (V9); [event loop and threading](event-loop-and-threading.md); [IPC](ipc.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered FT1–FT14. Precedent and specification text are sourced (§2–§3); the design is inference unless marked otherwise.

## Summary

- **The browser's core thread holds the only authoritative frame tree** (FT1–FT3): tabs, frames, documents, browsing context groups, agent clusters and processes. Every fact in it the browser derived itself, from URLs, responses, its own process launches and the input it routed. A renderer's claim becomes a fact only after validation against those records.
- **Each renderer keeps a skeleton replica** of the frame trees it takes part in (FT4, FT5). The replica holds the tree's shape, the frames' names and their open or closed state: just enough to answer script's synchronous questions such as `window.length` or `frames[1]`. Frames held by other agents appear as remote window proxies that offer only the operations the specification allows across origins.
- **Every operation between agents goes through the browser** (FT6): `postMessage`, navigating another frame, `window.open`, focus, closing, renaming, creating and removing frames. This holds even when both agents share a process, so moving a frame into its own process, as L3 requires, changes no interface.
- **Navigation is browser-driven** (FT7–FT10). The browser validates the request, collects `beforeunload` answers without waiting indefinitely, has the network process fetch, and chooses the process and event loop once the response arrives. It then commits the document with its records and capabilities. The renderer's acknowledgement makes the commit authoritative, and session history lives in the browser.
- **Loading uses capabilities bound at commit** (FT11). Each document receives loader and storage endpoints bound to its origin, its partition and its process's lock. Response bodies stream from the network process straight to the renderer.
- **Out-of-process iframes are tested as required** (FT14, check V9): renderers that name frames they weren't given, or claim another site's data, are terminated.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **SI4:** the browser owns the frame tree from the first line of code; each frame produces its own display list; the browser routes input | FT1, FT6; CP6 and CP8 build on FT3's IDs |
| **SI1–SI7:** site locks, process switches decided by the browser alone, compensating rules for SI3, sensitive sites, process reuse that never merges sites, agent clusters | FT10 |
| **V9:** out-of-process iframe tests; a renderer that requests another site's frame or data is terminated | FT14 |
| **IR2, IR4, IR10; IP10–IP12:** decisions from the browser's records; capabilities; recovery on death; `Untrusted<T>` | FT2, FT3, FT11, FT13 |
| **PM3:** the browser is the policy authority | FT1 |
| **EL6, EL15, EL20:** the core thread holds the tree; one event loop per agent; frames subscribe for frames individually | FT1, FT4, FT12 |

## 2. What the HTML standard defines

From the WHATWG HTML standard, last updated 16 October 2026 [H1, H2]:
- **Navigables and traversables.** A navigable is "a user-facing representation of a sequence of documents". Traversable navigables "control the session history of themselves and of their descendant navigables". A top-level traversable is a tab; a child navigable belongs to a container, such as an `iframe`.
- **Browsing contexts** are "a developer-facing representation of a series of documents", one per `WindowProxy`. A **browsing context group** holds a set of top-level browsing contexts and an **agent cluster map**. It also has a cross-origin isolation mode: "none", "logical" or "concrete".
- **Session history lives in the traversable.** A navigable's current entry "can only be modified within the session history traversal queue" of its traversable.
  - A **session history entry** holds a step, a URL, a document state, history and navigation API state, a scroll restoration mode and scroll positions.
  - A **document state** holds the document, a history policy container, the request referrer, the initiator origin, the origin, nested histories, the navigable's target name, and more.
- **Choosing a navigable by name** returns only a navigable the current one is "familiar with": same origin, same top-level traversable, an opener relationship, or an ancestor's origin. The sandboxing check is currently optional; issue #10849 tracks making it required.
- **Navigation** checks "if unloading is canceled", fetches, may switch browsing context groups because of `Cross-Origin-Opener-Policy`, and finalizes through the traversable's session history steps. Same-document updates are "the only synchronous updates to session history".

## 3. Precedent

From source and documentation read on 2026-10-17 (Sources).

| | Chromium | Firefox | WebKit | Servo |
|---|---|---|---|---|
| **The authoritative tree** | "WebContents hosts a tree of FrameTreeNode objects, mirroring the frame tree of the current page" [C2] | A `CanonicalBrowsingContext` in the parent process, where "more information about the context is available" [F1] | The UI process is the source of truth for frame state [W1] | The constellation holds every browsing context, each with its session history, and every pipeline [S1] |
| **Frames in other processes** | "a slimmed down blink::RemoteFrame as a placeholder". "The remote classes have very little state: generally only what is needed to service synchronous operations" [C2] | A `BrowsingContext` "will exist in every 'relevant' content process for the navigation" [F1] | "Each process needs to have the entire representation of the FrameTree"; a `RemoteFrame` marks the position of a frame held elsewhere [W1] | Not checked |
| **Replicated state** | Replicated by the browser's frame managers [C2] | "Synced fields"; their documentation is a work in progress [F1] | "A set of properties that we try to keep as up to date as possible in all processes". Only non-sensitive information should be shared, because sharing creates a speculative-execution side channel [W1] | Not checked |
| **Cross-frame operations** | `postMessage` goes to "the browser and then to the correct document"; focus is managed in the browser [C2] | Not checked | `postMessage` is delivered through the `RemoteFrame` [W1] | Creating iframes, loading them, `postMessage`, `MessagePort`s, `BroadcastChannel` and storage events all go through the constellation [S1] |
| **Navigation and commit** | The browser chooses the process after the response and sends a "commit"; the renderer's acknowledgement "marks the authoritative commit time" [C1] | "The final process to load the document in is selected by the logic in ProcessIsolation.cpp" in the parent [F2] | During a provisional load, the `RemoteFrame` is swapped for a `LocalFrame` when the response arrives [W1] | A loaded iframe URL makes "a new pipeline being spawned and a child being added to the parent browsing context" [S1] |
| **Lessons** | Many places "assume that main frame will always be local". Frames can greatly outnumber tabs, so per-frame memory matters [C2] | — | Planned as "a simple 3 step project", it was still at step 2 in January 2025. Most remaining bugs come from code that "used to follow a pointer to another frame" [W1] | — |

**Observations** (inference):
- **All four engines converge on one model:** an authoritative tree in the privileged process, a replica in each content process, and placeholders for remote frames that carry little state.
- **The retrofit is where the cost was.** Chromium's and WebKit's lessons are about code that assumed a frame was local. Axiom avoids the class of bug by never giving one agent a pointer to another agent's frame (FT5, FT6).
- **What crosses to other processes is a security decision.** WebKit's warning about side channels sets FT4's minimum.

## 4. Design

### 4.1 The browser's records

| # | Rule |
|---|---|
| FT1 | **The core thread holds the only authoritative frame tree** (EL6), and decides structure, navigation, process assignment and policy (PM3, AC2). Its records are listed below |
| FT2 | **Records hold facts the browser derived itself:** origins from URLs and sandboxing flags; sites from origins and the Public Suffix List; policy containers from response headers; user activation from the input it routed (CP8); process locks from its own launches. A renderer's statement enters a record only after validation against the existing records (IP10) |
| FT3 | **The browser issues every ID** for tabs, frames, documents and browsing context groups. The tokens a renderer sees are scoped to that process (IP12). A renderer holds tokens only for frames in the browsing context groups where it hosts a document, so it can't even name anything else |

| Record | Holds |
|---|---|
| **Tab** (top-level traversable) | Its session history (entries and current step); its browsing context group; visibility; its window in the shell |
| **Frame** (navigable) | Its parent and position among siblings; the container token in the parent document; its target name; its sandboxing flags and permissions policy as declared by the container; its active document; any pending navigation |
| **Document** (one per committed document) | Origin, site and agent cluster key; policy container (CSP, referrer policy, embedder policy, sandboxing); the process and event loop that host it; user activation; granted permissions; its loader and storage endpoints (FT11) |
| **Browsing context group** | Its tabs; its agent cluster map, from cluster key to process and event loop; its cross-origin isolation mode |
| **Process** | Type, site lock, executable (with or without the JIT), hosted event loops, granted capabilities, memory budget state |

### 4.2 What renderers hold

| # | Rule |
|---|---|
| FT4 | **A skeleton replica, and nothing more.** Each event loop that hosts a document in a browsing context group receives that group's frame-tree skeleton: the tree's shape, each frame's target name, whether each window is closed, and which frames are local to this event loop. That is enough to answer script's synchronous questions: `window.length`, `frames[i]`, `parent`, `top`, `opener`, `closed`, and named access to child frames, as far as the specification allows it across origins. **Nothing else is replicated:** no URLs and no other documents' origins, following WebKit's caution about side channels. The browser sends changes as an ordered stream on the event loop's frame-host channel (IP3) |
| FT5 | **Frames held elsewhere are remote window proxies.** A frame whose document lives in another agent, in another process or in another event loop of the same process, appears as a remote `WindowProxy` in each agent that can reach it. It offers only the properties the specification allows across origins: `postMessage`, `location` (set `href`, `replace`), `close`, `closed`, `focus`, `blur`, `frames`, `length`, `top`, `parent`, `opener`, `self`, `window`, and indexed and named child access. Each operation becomes a message to the browser (FT6). A remote frame is always cross-site from L2, so it is always cross-origin, and every access to it is a cross-origin access |

**Synchronous answers, asynchronous authority.** Script gets synchronous answers from the replica: `window.open` returns a `WindowProxy` at once, and named access resolves at once. The action itself goes to the browser, which re-checks it against its own records. A replica that was stale by a moment yields `Violation::Ignore`, an expected race, never `Terminate` (IP11).

### 4.3 Operations between agents

| # | Rule |
|---|---|
| FT6 | **Every operation between agents goes through the browser,** even when both agents share a process, as cross-site frames do under SI3. So moving a frame into its own process changes no interface (AC1, SI4). The operations: |

| Operation | What the browser does |
|---|---|
| `postMessage` | Checks that the target is reachable from the source in its records, stamps the source origin from its records (IR2), and routes the message. The target's agent checks the `targetOrigin` restriction against its own document, so the sender never learns the target's origin. Transferred `MessagePort`s become channels the browser creates (IP4; CP10) |
| Navigating another frame (`location`, links, forms with a target) | Checks "allowed to navigate": familiarity, sandboxing flags, and user activation for top-level navigation by sandboxed frames, all from its records. Then runs the navigation (FT7) |
| `window.open` | Resolves the target name with the "familiar with" rule, or creates a tab in the same browsing context group, unless `noopener` or `Cross-Origin-Opener-Policy` says otherwise. Popup blocking uses the user activation it recorded |
| `focus`, `blur`, `close` | Decides by its own policy and records: user activation, and whether script opened the window (CP8) |
| Renaming (`window.name`) | Updates the frame record, and sends the change to every replica in the group |
| Creating a child frame | Creates the frame record from the container's declared name, sandboxing flags, permissions policy and referrer policy. The initial `about:blank` document is created synchronously in the parent's agent, as the specification requires, and the browser records it |
| Removing a child frame | Detaches the subtree in its records, unloads its documents in every process, and updates every replica |

### 4.4 Navigation

| # | Rule |
|---|---|
| FT7 | **Navigation is browser-driven.** Its stages: |

1. **Request.** A renderer asks to navigate (a link, `location`, a form, `window.open`), or the shell does (the address bar, a bookmark, back or forward, reload). The browser validates the initiator document, the target frame and the permission to navigate from its records (FT6). The initiator's origin and user activation come from its records, never from the request.
2. **Unload check.** The browser asks every document that the navigation would unload to run `beforeunload`, including documents in other processes. It never waits indefinitely (IR5). If a page asks, the browser shows the prompt itself.
3. **Fetch.** The network process fetches on the browser's behalf: navigation requests come from the browser, never from a renderer. Each redirect is reported to the browser, which applies policy (mixed content, `Cross-Origin-Opener-Policy`, frame-ancestor rules) and can cancel.
4. **Response.** The browser computes the final URL, the origin (opaque if sandboxed), the site, the policy container, the browsing context group (switching groups if `Cross-Origin-Opener-Policy` requires), and the agent cluster key: the site, or the origin if cross-origin isolated or origin-keyed.
5. **Process and event loop.** The browser chooses them by FT10.
6. **Commit.** The browser sends the chosen event loop the new document's records: its ID, origin and policy container. With them come the frame-tree skeleton (FT4), loader and storage endpoints (FT11), and the endpoint of the response's body stream.
7. **Acknowledgement.** The renderer creates the document and acknowledges. The acknowledgement makes the commit authoritative, as in Chromium [C1]. The browser then updates session history, and the previous document unloads. For a process switch, it unloads in its own process while the new one loads.

| # | Rule |
|---|---|
| FT8 | **Same-document navigations stay synchronous.** Fragments, `pushState`, `replaceState` and navigation-API intercepts run in the renderer synchronously, as the specification requires, and are then reported to the browser. The browser checks that the document is the same and that the new URL is same-origin with the document's origin in its records. Then it updates session history |
| FT9 | **Session history lives in the browser,** per tab, where the specification places it. Back, forward and `history.go` are browser operations, whether the shell or a document in the tab asks. Serialized history state (from `pushState`) is stored with the browser's entry, within a size bound, and handed back to the document when its entry is restored. A back-forward cache can be added later: it keeps documents alive, frozen, in their renderers, under the memory budgets, and commits "from cache" through the same commit message |
| FT10 | **Process and event-loop selection** happens at the response (SI1–SI7): |

- **The agent cluster map decides.** If the group's map already holds the cluster key, the document joins that agent: the same process and the same event loop. Otherwise the browser picks a renderer and creates an event loop for the new agent there.
- **Picking a renderer:**
  - **L1:** one renderer per tab, not locked.
  - **From L2:** a renderer locked to the site. It reuses a same-site renderer under the soft limit (B5, SI6), takes the spare renderer and locks it, or launches one. It never merges sites.
  - **Cross-origin isolated groups** get processes of their own (SI7).
  - **Sensitive sites** get a dedicated, no-JIT renderer and are never placed in another site's process (SI5, J4).
  - **Cross-site child frames** stay in their embedder's renderer, in a separate agent and event loop, only at L1 and non-sensitive L2 (SI3). From L3 they always get a renderer locked to their own site.
  - **As amended in Round 1** (RV1-03; the owner: "Adopt the single binding rule: cross-site frames are separate agents with independent event loops and heaps behind remote proxies from Milestone 1."): **from M1, every cross-site frame is its own agent,** with its own event loop and heap, reached only through remote window proxies (WB20), in whichever process it renders: its embedder's renderer at L0–L2 (at L2 only for sites that aren't sensitive, SI3), a renderer locked to its own site from L3. L2 adds site locks, not agents. Documents of different sites never share an agent, as HTML's agent clusters require.
- **Special documents** derive their origin and process from the browser's records of the initiator or creator: `about:blank`, `about:srcdoc`, `data:`, `blob:`. For `blob:`, those records are the browser's `blob:` URL store ([storage](storage-and-workers.md) SW15; Round 2, RV2-11). Error pages are internal documents, committed in a renderer with an internal lock, never in the failing site's process.

### 4.5 Loading and storage

| # | Rule |
|---|---|
| FT11 | **Loader and storage endpoints are bound at commit.** At commit, the browser asks the network process for two endpoints. They are bound to the document's facts: origin, partition key (the top-level site, from L2), embedder policy, sandboxing flags and the process's lock. The renderer receives them with the commit (IR4). A request or storage call on those endpoints can't claim other facts: a mismatch is `Terminate`. The network process applies ORB, CORS and partitioning from the bound facts (SI3a, SI3b). **Response bodies stream** from the network process straight to the renderer through a single-writer ring (IP14), so the browser is never in the data path. Workers receive endpoints bound to their own global's facts |

### 4.6 Children, death and tests

| # | Rule |
|---|---|
| FT12 | **Each frame is a unit of rendering.** Each frame has its own document ID, subscribes for frames separately (EL20), and produces its own display list, which its parent's display list references (CP6). So frames are composited, hit-tested (CP8) and moved between processes individually. **Round 4** (RV4-16): from M2 a child frame's rectangle reaches the browser with the GPU process's layer commit (IV13) |
| FT13 | **When a renderer dies,** the browser marks its documents gone and puts placeholders in their frames: a crash page in a tab, a placeholder in a subframe. It updates every replica in the affected groups, and keeps nothing that renderer reported (IR10) |
| FT14 | **Out-of-process iframes are tested** (check V9). "Lying renderer" tests check that a renderer is terminated when it: names a frame token it wasn't given; tries to commit, request or read another site's document; claims another origin in a loader request or storage call; or forges user activation |

**User activation.** The browser records activation from the input it routes (CP8), and propagates it as the specification requires. It tells renderers the result, which they need for APIs gated on activation, and itself enforces every gate that matters (popups, top-level navigation from sandboxed frames).

## 5. Staging

| Level | What changes in this design |
|---|---|
| L0 | Renderers are sandboxed processes from M1 (Round 1, RV1-07). The frame tree, replicas, remote window proxies and browser-routed operations all exist, and **every cross-site frame is a separate agent, with its own event loop and heap, from M1** (Round 1, RV1-03). Agents talk through the browser |
| L1 | Renderers are hardened (M2), one per tab. Nothing in the frame tree or its messages changes |
| L2 | Process selection adds site locks. Cross-site frames **remain** separate agents and event loops, in their embedder's process (SI3, non-sensitive sites only), as they have been since M1 (RV1-03). The interfaces are unchanged |
| L3 | Process selection puts every cross-site frame in a renderer locked to its own site. **Nothing else changes:** the frame was already a separate agent, with its own subscription, display list and browser-routed operations (AC1, SI4). V9's tests pass |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Renderers hold the authoritative tree** and tell the browser about it | It contradicts PM3 and IR2: a compromised renderer could misreport structure, names or origins. Rejected |
| B | **Full replication of frame state to every process** | Simplifies some features, but leaks URLs and origins across sites, the side channel WebKit warns about. Rejected for FT4's skeleton |
| C | **Direct calls between agents in the same process** (SI3) | Faster for same-process cross-site frames, but it creates exactly the code that Chromium and WebKit later had to rewrite for out-of-process frames. Rejected for FT6 |
| D | **Renderer-driven navigation**, with the browser informed afterwards | Simpler, but process selection and policy would trust the renderer's view of the response. Chromium moved navigation into the browser for this reason (inference from [C1]). Rejected for FT7 |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | One frame tree, one replica protocol and browser-routed operations at every level; L3 changes only process selection (§5) |
| AC2 | FT1–FT3, FT6, FT7 |
| AC7 | Records and skeleton replicas are small. Frames can outnumber tabs, so per-frame cost is measured in Step 6 (Chromium's lesson) |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The exact fields of the skeleton replica, and how each synchronous `Window` property is answered | The implementation milestone for frames |
| 2 | The back-forward cache, and how it fits the memory budgets | **Answered** at Step 6 by PF22: 2 documents per tab, within B6, emptied first under pressure; measured at the navigation milestone (Round 4, RV4-14) |
| 3 | Where shared and service workers run when no same-site renderer exists | **Answered:** in a renderer locked to their site, which the browser chooses ([event loop](event-loop-and-threading.md) §4.6; SW9) (Round 4, RV4-14) |
| 4 | How `beforeunload` across many processes is bounded, and what the prompt shows | **Answered** at Step 6 by PF22: each frame's renderer has 1 s to answer (Round 4, RV4-14) |

## Sources

All checked on 2026-10-17.

- [H1] WHATWG HTML, §7.3 "Infrastructure for sequences of documents" (Last Updated 16 October 2026) — https://html.spec.whatwg.org/multipage/document-sequences.html
- [H2] WHATWG HTML, §7.4 "Navigation and session history" — https://html.spec.whatwg.org/multipage/browsing-the-web.html
- [C1] Chromium, "Life of a Navigation" (`docs/navigation.md`) — https://chromium.googlesource.com/chromium/src/+/HEAD/docs/navigation.md
- [C2] Chromium, "Out-of-Process iframes (OOPIFs)" (undated) — https://www.chromium.org/developers/design-documents/oop-iframes/
- [F1] Firefox source docs, "BrowsingContext and WindowContext" (synced fields marked as work in progress) — https://firefox-source-docs.mozilla.org/dom/navigation/BrowsingContext.html
- [F2] Firefox source docs, "Process Model" — https://firefox-source-docs.mozilla.org/dom/ipc/process_model.html
- [W1] WebKit, "Site Isolation" (status as of January 2025) — https://docs.webkit.org/Deep%20Dive/SiteIsolation.html
- [S1] Servo, `components/constellation/constellation.rs` (main at `fc18b55`, 2026-10-17) — https://github.com/servo/servo/blob/main/components/constellation/constellation.rs
