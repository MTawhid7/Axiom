# Storage and Workers

| | |
|---|---|
| **Status** | **Proposed** (Step 5, CP7). Decided in [ADR-0013](../adr/0013-networking-storage-and-workers.md), whose acceptance the owner pre-approved at Gate B (2026-10-18), effective once S7-Q1, S7-Q3 and S7-Q4 pass |
| **Question** | Where and how are web storage, IndexedDB, the Cache API and quotas stored? How do service workers intercept fetches, and where do shared and service workers run when no same-site renderer exists? |
| **Answers** | Step 5 question Q7 ([tracker](../research/step-5-subsystems.md) §3); the tracker's handed-over question 11 (§2.2); [networking](networking.md) open question 2 (the storage format of the cache and the cookie jar) |
| **Feeds** | ADR-0013 (CP9); CP16 (the storage and worker APIs in JavaScript); CP17 (bindings) |
| **Builds on** | The [process model](../security/process-model.md) §3, note 2, SI1–SI3 and B4–B5; [frame tree and navigation](frame-tree-and-navigation.md) FT6, FT9–FT11; [event loop and threading](event-loop-and-threading.md) EL8 and §4.6; [script and heap](script-and-heap.md) SH1–SH2; [IPC](ipc.md) IP4, IP13, IP15; [networking](networking.md) NW10–NW12; the [dependency policy](../technology/dependency-policy.md) |
| **Last updated** | 2026-10-18 |

Everything here is **planned**; nothing is implemented. Rules are numbered SW1–SW13. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **All web storage lives in the network process,** on its storage thread, behind endpoints bound at commit to the document's storage key: its origin and its top-level site (SW1). A renderer can't name another key.
- **One storage engine:** `redb`, a pure-Rust, transactional, crash-safe key-value store, if its audit grades it `ub-risk-2` or better (SW2). Large values live in plain files beside it. Each storage key gets its own files, so clearing a site deletes them. **The fallback is SQLite,** which Firefox and WebKit use and Chromium is moving IndexedDB to, admitted only under the dependency policy's exception. The cache and the cookie jar use the same engine.
- **`localStorage` is cached in the renderer,** loaded on first access by one synchronous request, as Chromium does; writes return asynchronously (SW3).
- **IndexedDB values arrive already serialized,** so the network process stores and indexes them without ever parsing script values (SW4).
- **Quotas and eviction** follow the Storage Standard's best-effort and persistent buckets (SW6).
- **Service workers:**
  - registrations live with storage (SW8);
  - navigations are intercepted inside Fetch;
  - a controlled document reaches its worker through a channel the browser creates, as it creates every channel between agents.
- **Shared and service workers run in a renderer locked to their site** (SW9). If none exists, the browser launches one, which the site's documents will then share. Workers never share a process across sites.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **Process model §3, note 2:** web storage lives with the cookie jar, in the network process, so that renderer-supplied structured data stays out of the browser | SW1 |
| **SI3 (a):** a cross-site frame's storage is keyed by the top-level site and the frame's site | SW1 |
| **FT11:** storage endpoints are bound at commit to the document's facts. **IR2:** decisions come from the receiver's records | SW1 |
| **FT6:** every operation between agents goes through the browser; transferred ports become channels the browser creates (IP4) | SW8 |
| **FT9:** session history lives in the browser | SW3 |
| **FT10, SI1:** renderers locked to one site; reuse under the soft limit (B5) | SW9 |
| **EL8:** the network process has a storage thread and a worker pool. **EL §4.6:** shared and service workers run in a renderer locked to their site, which the browser chooses | SW1, SW9 |
| **IP13:** synchronous requests only toward more privilege, and only where the platform requires them | SW3 |
| **SH2:** between agents, data moves by structured serialization | SW4 |
| **NW10, NW11:** the HTTP cache and the cookie jar are partitioned; their format is this checkpoint's | SW7 |
| **DP9, DP10:** a pure-Rust crate at `ub-risk-2` or better may run in the network process; C code that handles web-controlled data needs DP10's exception | SW2 |

## 2. Specifications

- **The WHATWG Storage Standard** (last updated 2026-10-06) [W1]:
  - "A storage key is a tuple consisting of an origin", a definition "expected to change; see Client-Side Storage Partitioning";
  - a user agent's storage shed holds local storage, and each traversable navigable's holds session storage;
  - "A local storage bucket has a mode, which is "best-effort" or "persistent"".
- **HTML:** Web Storage (`localStorage`, `sessionStorage`).
- **Indexed Database API.**
- **Service Workers:** registrations, the Cache API, and Fetch's "handle fetch" step.
- **Other APIs:** the File System Standard (the origin private file system); Web Locks; `BroadcastChannel` (HTML).
- **Client-Side Storage Partitioning** (Privacy CG).

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Storage |
|---|---|
| **Chromium** | **IndexedDB is being rewritten on SQLite** (a PSA dated 2025-12-09) [C1]:<br>• it is "rewritten on top of SQLite", "to replace the previous implementation that uses a hybrid of LevelDB and flat files";<br>• it is "expected to improve reliability and, to a lesser extent, performance";<br>• so far it is "applied only to in-memory contexts such as Incognito mode";<br>• "All data on disk is still segregated by storage bucket (origin)."<br>**`localStorage`:** `CachedStorageArea` "Maintains a complete cache of the BlinkStorageKey's Map of key/value pairs for fast access. The cache is primed on first access and changes are written to the backend through the StorageArea mojo interface" (`cached_storage_area.h`) [C2]. **A controlled page reaches its service worker** through a connector in the renderer (`controller_service_worker_connector`, `service_worker_subresource_loader`) [C3] |
| **Firefox** | IndexedDB and `localStorage` store through mozStorage, its SQLite layer (`mozIStorageConnection` in `dom/indexedDB/ActorsParent.cpp` and `dom/localstorage/ActorsParent.cpp`) [F1] |
| **WebKit** | IndexedDB on SQLite (`SQLiteIDBBackingStore`) [W2] |
| **Ladybird** | SQLite, from its dependency manifest ([dependency policy](../technology/dependency-policy.md) §2) |

**The pure-Rust candidate, `redb`** (README; 4.4.0 on crates.io, released 2026-10-13) [R1]:
- "A simple, portable, high-performance, ACID, embedded key-value store", "written in pure Rust", "loosely inspired by LMDB", storing data in "a collection of copy-on-write B+trees";
- "Fully ACID-compliant transactions", "Crash-safe by default", "MVCC support for concurrent readers & writer, without blocking";
- "The file format is stable";
- in its own benchmark, the fastest at individual writes, slower than LMDB at bulk loads and random reads, and large before compaction.

Observed in its source: no required dependencies, and 54 mentions of `unsafe`, most in a vendored hash (`xxh3.rs`), in synchronization (`sync.rs`) and in file locking. One principal maintainer: 1,634 commits against at most 47 for anyone else.

**Observations** (inference):
- **Every engine stores IndexedDB in SQLite,** or is moving to it. Chromium's stated reason is reliability.
- **Every engine segregates data by storage bucket,** and keeps a renderer-side cache for `localStorage`'s synchronous API.

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **A transactional store** | **`redb`** 4.4.0: pure Rust, no required dependencies, not graded. **SQLite,** through `rusqlite` and a bundled C build: the industry's choice (§3), but C code handling web-controlled data. **`fjall`** 3.1.12, an LSM tree, not graded. **Our own:** a crash-safe engine is subtle, specialized work (DP4) | **`redb`, if its audit grades it `ub-risk-2` or better;** otherwise SQLite under DP10's exception (SW2) |
| **Large values** | Files in the network process's data directory | **Files,** referenced from the store |

## 5. Design

### 5.1 Where storage lives

| # | Rule |
|---|---|
| SW1 | **All web storage lives in the network process,** on its storage thread (EL8), behind storage endpoints bound at commit to the document (FT11). **The storage key is the origin and the top-level site,** the partitioned key of Client-Side Storage Partitioning. So a cross-site frame's storage is keyed by both sites (SI3 (a)), and a top-level document's by its own origin and site. The network process derives the key from the endpoint; a renderer never names one, and a request that restates a different key is a violation (IR2, IP11) |
| SW2 | **One storage engine: `redb`, if its audit grades it `ub-risk-2` or better** (DP9). It is transactional and crash-safe by its own account (§3), and has no required dependencies. **Each storage key gets its own database file,** so clearing a site deletes files rather than editing a shared store, as Chromium segregates by bucket (§3). **Large values** live in plain files beside it: cache bodies, Cache API bodies, IndexedDB blobs. **The fallback is SQLite,** which Firefox and WebKit use and Chromium is moving to. It would be admitted under DP10's exception, by ADR-0013, only if `redb` fails its audit or its durability tests on this Mac (SW13). `redb`'s single principal maintainer is a recorded risk, which the storage interface contains: the engine sits behind Axiom's interface (DP2) |

### 5.2 The storage APIs

| # | Rule |
|---|---|
| SW3 | **`localStorage` is cached in the renderer.** The first access in a document loads its storage area with one synchronous request to the network process. That is toward more privilege, and required by the API's synchronous design (IP13). Reads are then served from the cache. Writes go asynchronously, and the network process sends `storage` events to the other documents of the same key. Chromium's renderer cache works this way (§3). Each area has a size limit, set in Step 6. **`sessionStorage`** works the same way, per tab (traversable navigable), held in the network process's memory, and saved with the tab's session history for session restore (FT9) |
| SW4 | **IndexedDB's values arrive already serialized.** The renderer serializes each value (SH2) and evaluates key paths itself. It sends the network process opaque value bytes, with keys in an order-preserving encoding. The network process runs transactions, ranges and indexes over those keys, and **never parses a script value.** A compromised renderer can corrupt only its own storage key's data |
| SW5 | **The Cache API** keeps request and response pairs per storage key, with `Vary` matching in the network process. Bodies are files (SW2) |
| SW6 | **Quotas and eviction follow the Storage Standard.** Each storage key has a bucket: best-effort, and evictable, until the browser grants persistence (`navigator.storage.persist`), by permission or by its own heuristics (EM5). The network process accounts each key's usage. Under disk pressure it evicts best-effort keys, least recently used first. The `Clear-Site-Data` header and the browser's own clearing delete a key's files. Step 6 sets the quotas against this Mac's disk |
| SW7 | **The HTTP cache and the cookie jar use the same engine** (answering networking open question 2). The cookie jar is one store. The cache's index is a store, keyed by network partition key (NW10), with each body in a file. Both are evicted by size, least recently used first |

### 5.3 Workers

| # | Rule |
|---|---|
| SW8 | **Service workers.** Registrations are stored with storage, by storage key and scope (SW2).<br>• **Navigations:** Fetch's "handle fetch" step runs in the network process, which consults the registrations. If a worker controls the URL, it asks the browser to start that worker or find it running (SW9). The `fetch` event goes to the worker's agent, and its response streams back through the same path.<br>• **A controlled document's requests** reach its worker through a channel the browser creates between the two agents. That is how the browser creates every channel between agents (FT6, IP4), and how Chromium's renderer connects to the controller (§3). Requests don't detour through the browser one by one, and the network process sees only what the worker itself fetches.<br>• **Lifetime:** a service worker's agent stops after an idle timeout; a shared worker lives while it has client documents |
| SW9 | **Shared and service workers run in a renderer locked to their site** (EL §4.6, SI1). **When none exists, the browser launches one,** or locks the spare renderer, exactly as for a navigation (FT10). Workers never share a process across sites. Because a navigation to the site needs that process anyway, FT10's reuse rule puts the site's documents in the renderer that already hosts its worker. A renderer that hosts only workers counts against the soft limit (B5). This answers the handed-over question 11 |
| SW10 | **Coordination between agents of one storage key.** Web Locks are arbitrated in the network process, by storage key, next to the data they guard. `BroadcastChannel` messages are routed through the browser, like any operation between agents (FT6) |

### 5.4 Later APIs, memory and tests

| # | Rule |
|---|---|
| SW11 | **The origin private file system** comes later: files per storage key in the network process's data directory. A synchronous access handle in a worker receives a descriptor for one file, where its interface allows a descriptor (IP15). That file is web storage, not a user file (SG1). **Push messaging and background sync** need a push service and periodic wakeups, and are not designed here |
| SW12 | **Memory and idle.** Databases are opened on demand and closed after an idle period. The storage thread waits with nothing to do when no requests arrive. Within the network process's budget (B4), its caches are bounded |
| SW13 | **Tests:**<br>• WPT's `IndexedDB`, `webstorage`, `service-workers`, `storage` and `fs` directories;<br>• for the engine: crash tests that kill the network process during writes, then check every store's consistency and each key's isolation. These tests, run at the first storage milestone, decide `redb` against SQLite (SW2) |

## 6. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **The browser's core** | Storage keys from its records (FT11); persistence grants and clearing (EM5); starting worker agents (SW9) |
| **Networking** (CP6) | Fetch's "handle fetch" step; the cache and the cookie jar (SW7) |
| **Renderers** (CP16, CP17) | Storage endpoints for `localStorage`, `sessionStorage`, IndexedDB, the Cache API and locks; channels to service workers |

## 7. Security

- **The browser holds no web storage** (process model §3, note 2).
- **Keys come from the browser's records,** so a renderer can reach only its own partition (SW1, IR2).
- **The network process never parses script values** (SW4).
- **The storage engine** is pure Rust if `redb` passes its audit. Otherwise it is SQLite, accepted under DP10's exception by ADR-0013 (SW2).

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First storage milestone | `localStorage`, `sessionStorage`, the cookie jar and the cache on the engine; the crash tests that decide it |
| Later | IndexedDB; the Cache API and service workers; shared workers; quotas and persistence; Web Locks; the origin private file system |
| L0–L3 | No interface changes: storage stays behind endpoints at every level, and from L2 the endpoints carry site locks |

**The complete model** (SC9): Storage Buckets fit the bucket model (SW6); private browsing is a profile with in-memory stores only, which `redb`'s in-memory backend or SQLite's provide.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **SQLite as the first choice,** as the other engines use | The best-tested engine there is. But it is C handling web-controlled data, and DP10 admits that only when no pure-Rust option meets the need. Kept as the fallback, decided by tests (SW2, SW13) |
| B | **Storage in the browser process** | The process model chose the network process, to keep renderer-supplied structured data out of the browser. Rejected |
| C | **Storage in each renderer** | It would put one site's persistent data where any compromise of that renderer could rewrite it for good, and complicate partitioning and quotas. Rejected |
| D | **A worker process shared across sites** | It breaks SI1. Rejected (SW9) |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | `redb`'s audit grade, and its durability under SW13's crash tests, against SQLite | The first storage milestone |
| 2 | Quotas, area limits and the eviction policy's thresholds | Step 6 |
| 3 | Push messaging and background sync | A later decision, with the owner |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [W1] WHATWG, Storage Standard (Living Standard, last updated 2026-10-06) — https://storage.spec.whatwg.org/
- [W2] WebKit, `Source/WebCore/Modules/indexeddb/server` (`SQLiteIDBBackingStore`) — https://github.com/WebKit/WebKit/tree/main/Source/WebCore/Modules/indexeddb/server
- [C1] "Web-Facing Change PSA: IndexedDB: SQLite backend (in-memory contexts)", blink-api-owners-discuss (2025-12-09) — https://groups.google.com/a/chromium.org/g/blink-api-owners-discuss/c/ggfnEsdbd0I
- [C2] Chromium, `third_party/blink/renderer/modules/storage/cached_storage_area.h` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/modules/storage/cached_storage_area.h
- [C3] Chromium, `content/renderer/service_worker/controller_service_worker_connector.h` — https://github.com/chromium/chromium/tree/main/content/renderer/service_worker
- [F1] Firefox, `dom/indexedDB/ActorsParent.cpp` and `dom/localstorage/ActorsParent.cpp` — https://github.com/mozilla-firefox/firefox/tree/main/dom/indexedDB
- [R1] `redb`, README and sources — https://github.com/cberner/redb ; crates.io's API, for `redb`, `fjall`, `rusqlite` and `libsqlite3-sys`
