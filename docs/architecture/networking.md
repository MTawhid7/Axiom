# Networking: URLs, HTTP, TLS, DNS, the Cache, Cookies and Fetch's Policy

| | |
|---|---|
| **Status** | **Binding** since [ADR-0013](../adr/0013-networking-storage-and-workers.md)'s acceptance (2026-10-19), which the owner pre-approved at Gate B (2026-10-18), effective once S7-Q1, S7-Q3 and S7-Q4 passed. Written at Step 5, CP6. **Spike S7** ([results](../research/s7-results.md)) settled NW6 as written, confirmed NW1, NW5 and NW7, and wrote NW17's profile. **Verification in the worker pool** (NW6), which S7 proposed, was confirmed by the owner at Step 5's Gate C (2026-10-20). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): open question 4, Encrypted Client Hello (RV1-20; CP5). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): NW4, NW10 and §8 for RV4-08 and RV4-17 (CP6); §10 for RV4-14 (CP7). **Amended at Milestone 1's second checkpoint** (2026-10-23): NW2's IDNA, by the owner's conditional approval once `smallvec` was graded `ub-risk-3` |
| **Question** | How does the network process fetch: URLs and sites, HTTP/1.1, HTTP/2 and HTTP/3, TLS and certificate verification against the system's trust store, DNS, connections, the HTTP cache and cookies? How does it enforce Fetch's policy from the browser's records? |
| **Answers** | Step 5 question Q6 ([tracker](../research/step-5-subsystems.md) §3); decision-map item I3; the tracker's handed-over questions 7–10 (§2.2); [text and fonts](text-and-fonts.md) open question 5 (IDNA in the browser) |
| **Feeds** | ADR-0013 (CP9); spike S7 (tracker §10); CP7 (storage, service workers); CP13 (the parser's input streams); CP16 (`URL`, `fetch`) |
| **Builds on** | The [threat model](../security/threat-model.md) (SG2, SG4, SG7, SG8, SG10; attack surfaces); the [process model](../security/process-model.md) (§3, §4, SI1–SI7, IR1–IR10, B4); [frame tree and navigation](frame-tree-and-navigation.md) FT7, FT10, FT11; [event loop and threading](event-loop-and-threading.md) EL1, EL2, EL8, EL18, EL24; [IPC](ipc.md) IP14, IP16, IP17; the [dependency policy](../technology/dependency-policy.md) |
| **Last updated** | 2026-10-23: NW2's IDNA moved out of the browser (M1-CP2). Earlier: 2026-10-22, Round 4's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered NW1–NW18. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **Every protocol is a state machine without I/O of its own** (NW1), driven by the network process's I/O thread from one `kevent64` wait. That keeps Step 4's one wait per thread and its deterministic test scheduler. rustls and `quinn-proto` are built this way. Async runtimes such as tokio, which Servo uses, are rejected: they bring threads, reactors and clocks of their own.
- **One URL parser, Axiom's own, in every process** (NW2), so no two processes can parse a URL differently. It handles IDNA through ICU4X in renderers, the network process, and the browser's address bar. In the browser, URLs from other processes must already be canonical ASCII. That keeps ICU4X away from less-trusted data in the browser, which settles CP3's IDNA question.
- **Protocol code:**
  - HTTP/1.1, HTTP/2 framing and HPACK, HTTP/3 framing and QPACK are Axiom's own (NW3–NW5). Google grades `httparse` at `ub-risk-4`, and the reusable HTTP/2 and HTTP/3 crates are bound to async runtimes.
  - QUIC is `quinn-proto`.
- **TLS is rustls,** and the system verifies certificates through Security's trust evaluation (NW6), which honours macOS's trust store and its constraints (SG7).
  - rustls has no `unsafe`, but its crypto provider is C and assembly. That is admitted under the dependency policy's exception, inside the sandboxed network process, with ADR-0013.
  - **Spike S7** validated the network process's profile, and compared rustls with Apple's Network framework: rustls retired 2.4–3.3× fewer instructions per handshake, and 1.6× fewer per byte ([results](../research/s7-results.md)).
- **DNS uses the system's resolver, asynchronously, through DNS-SD** (NW7), including HTTPS records, so no blocking resolver is needed.
- **The cache, cookies and connections are partitioned** by Fetch's network partition key: the top-level site and a second key (NW8, NW10, NW11). Third-party cookies are blocked by default.
- **Fetch's policy is enforced in the network process,** from the facts bound to each document's loader endpoint, never from what a renderer claims (NW12):
  - CORS, CORP and COEP;
  - ORB, before any byte reaches a renderer;
  - mixed content, Referrer-Policy, HSTS with its preload list;
  - Local Network Access, the permission model that replaced Private Network Access in Chrome.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0004, consequences:** Step 5 decides how the network process enforces initiator locks and ORB, and the TLS stack; Apple's TLS, if chosen, stays out of the browser | NW6, NW12 |
| **SG4:** renderers have no network access; the network process derives the initiator from the renderer's site lock and enforces CORS, SameSite, mixed content and ORB-style blocking. **SG2:** `HttpOnly` cookies never reach a renderer. **SG7:** TLS is validated against the system trust store, with no silent downgrade, HSTS honoured, and mixed content blocked. **SG8:** SameSite is Lax by default | NW6, NW11, NW12 |
| **SI3 (a), (b):** third-party state partitioned by top-level site and frame site, third-party cookies blocked by default; ORB in the network process | NW10–NW12 |
| **FT11:** loader and storage endpoints are bound at commit to the document's facts; bodies stream from the network process to the renderer | NW12, NW14 |
| **EL1, EL2, EL8, EL24:** one wait per thread; helper threads wait in `kqueue`; the network process has an I/O thread, a storage thread and a worker pool; the thread loop is an interface with virtual time for tests | NW1, NW7, NW16 |
| **IP14, IP16, IP17:** bulk data out of line or through single-writer rings; every queue bounded; rates bounded by protocol | NW14 |
| **Process model §3, note 2; B4:** web storage lives with the cookie jar in the network process; the idle network process at ≤ 25 MB | NW11, NW16 |
| **Threat model §5:** Private Network Access-style checks are Step 5's | NW12 |
| **Dependency policy:** DP1 (f) makes Fetch's algorithm and security policies core; DP2 allows reusing HTTP, QUIC, TLS and DNS infrastructure and URL parsing; DP3 forbids Axiom's own cryptography; DP9 and DP10 govern the rest | §4 |

## 2. Specifications

- **WHATWG:** URL; Fetch, including network partition keys (§2.7) and HTTP cache partitions (§2.8), last updated 2026-10-06 [W1]; MIME Sniffing.
- **IETF:**
  - HTTP semantics, HTTP/1.1, HTTP/2 and HTTP/3, with HTTP caching;
  - QUIC; TLS 1.3; HSTS; WebSocket, over HTTP/1.1 and over HTTP/2;
  - Happy Eyeballs; Extensible Priorities;
  - cookies: RFC 6265 and its revision. `draft-ietf-httpbis-rfc6265bis-22` is dated 2025-12-01 and is in the RFC Editor's queue (observed on the IETF datatracker) [I1].
- **Proposals:**
  - ORB, being upstreamed into Fetch through pull request #1442, "primarily blocked on resolving the" MVP issues [O1];
  - Local Network Access (WICG) [C2].
- **W3C:** Mixed Content; Referrer Policy; CSP, whose fetch hooks the renderer runs (CP13).
- **The Public Suffix List.**

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Networking |
|---|---|
| **Servo** | A Rust stack: `rustls` with `aws-lc-rs`, `rustls-platform-verifier` and `webpki-roots`; `hyper` for HTTP/1 and HTTP/2, on a multi-threaded tokio runtime; `async-compression` for Brotli, gzip, zlib and zstd; the `cookie` and `url` crates (`components/net/Cargo.toml`) [S1] |
| **Chromium** | BoringSSL (`third_party/boringssl`) and its own Chrome Root Store, version 42 in `net/data/ssl/chrome_root_store/root_store.md` [C1]. **Local Network Access** (2025-06-09) "restricts the ability of websites to send requests to servers on a user's local network", through a permission prompt "launching in Chrome 142". It "replaces" Private Network Access, which relied on CORS preflights to local devices [C2] |
| **Firefox** | NSS. `security.enterprise_roots.enabled` is `true`, so it also trusts the operating system's roots (`StaticPrefList.yaml` [2026-10-16]) [F1] |
| **WebKit** | Apple's networking stack, through its network process (background) |
| **Ladybird** | curl and OpenSSL, from its dependency manifest ([dependency policy](../technology/dependency-policy.md) §2) |

**The building blocks:**
- **rustls:** `#![forbid(unsafe_code, unused_must_use)]` (observed in its `lib.rs`). It implements "TLS1.2 and TLS1.3 for both clients and servers". A crypto provider must be chosen explicitly, and the project recommends its `aws-lc-rs` provider; its `ring` provider "does not support post-quantum algorithms" [R1].
- **`rustls-platform-verifier`** verifies against the "macOS platform roots and keychain certificate", using `Security.framework`, so that "the OSes CA constraints will be taken into account" [R2].
- **`quinn-proto`:** a "Deterministic state machine of the protocol which performs **no** I/O internally and is suitable for use with custom event loops". Its test suite "uses simulated IO for reproducibility" [Q1].

**This Mac's SDK 27** (observed):
- Network framework: `nw_protocol_copy_quic_definition` (macOS 12) and `nw_protocol_copy_tls_definition`.
- Security: `SecTrustEvaluateWithError` (macOS 10.14).
- `dns_sd.h`: `DNSServiceGetAddrInfo`, `DNSServiceQueryRecord` and `DNSServiceRefSockFD`, with record types `kDNSServiceType_SVCB` (64) and `kDNSServiceType_HTTPS` (65).

**Observations** (inference):
- **Servo shows that a Rust stack works,** but it runs on tokio. Step 4's thread rules (EL1, EL5, EL24) need protocol code that the network process's own loop can drive, and `quinn-proto` and rustls already have that shape.
- **Every engine verifies certificates against a root store,** its own or the system's. SG7 chose the system's, so verification belongs to Security's trust evaluation.

## 4. Build or reuse

Grades are Google's published audits for the version named (google/supply-chain [2026-09-17]; Chromium's for `flate2`) [A1].

| Need | Candidates | Decision |
|---|---|---|
| **URL parsing** | **`url`** 2.5.8 (Servo's): audited `safe-to-run` only; its IDNA goes through ICU4X, whose data crates are graded `ub-risk-2`. **Our own** | **Our own,** one parser in every process (NW2). `url` is the differential reference |
| **HTTP/1.1** | **`httparse`:** a delta audit 1.8.0 → 1.9.4 graded **`ub-risk-4`**, so DP8 bars it. **Our own** | **Our own** (NW3) |
| **HTTP/2** | **`h2`** 0.4.20: tokio-bound, audited `safe-to-run` only. **Our own** framing and HPACK | **Our own** (NW4) |
| **QUIC** | **`quinn-proto`** 0.11.19: no I/O, deterministic, not graded. Network framework's QUIC (macOS 12) | **`quinn-proto`** (NW5) |
| **HTTP/3** | **`h3`** 0.0.8, async. **Our own** framing and QPACK | **Our own** (NW5) |
| **TLS** | **rustls** 0.23.46: no `unsafe`, not graded; with **`aws-lc-rs`** 1.18.2 (C and assembly) or **`ring`** 0.17.14 (C and assembly; graded `ub-risk-2` at 0.16.20, a grade that covers its Rust, not its C and assembly: DP8). **Network framework,** Apple's, patched by macOS updates. **Our own:** barred (DP3) | **rustls with the `aws-lc-rs` provider,** under DP10's exception; Network framework is S7's comparison (NW6) |
| **Certificate verification** | **Security's trust evaluation,** directly or through `rustls-platform-verifier` (`core-foundation` `ub-risk-2` at 0.10.0); `rustls-webpki` with a bundled root list, which would ignore the system's trust settings | **Security's trust evaluation,** through `platform` (NW6) |
| **DNS** | **DNS-SD,** the system resolver, asynchronous; `getaddrinfo`, which blocks; a resolver of our own (`hickory-resolver`), which ignores the system's per-domain configuration | **DNS-SD** (NW7) |
| **Decompression** | **`miniz_oxide`:** `ub-risk-0`. **`flate2`:** graded `ub-risk-3` (1.0.28) and `ub-risk-2` (1.1.0, Chromium). **`brotli-decompressor`** (TX7). **`ruzstd`:** pure Rust, `ub-risk-2` at 0.6.0. **`zstd`:** binds C | **`miniz_oxide`, `brotli-decompressor`, `ruzstd`** (NW13) |
| **The Public Suffix List** | **`publicsuffix`** 2.3.0; **our own** lookup over the list, compiled in at build time | **Our own lookup:** site computation is security policy (DP1 (f)) |

## 5. Design

### 5.1 Shape and URLs

| # | Rule |
|---|---|
| NW1 | **Protocols are state machines; the I/O thread drives them.** TLS, QUIC, HTTP/1.1, HTTP/2, HTTP/3, WebSocket and DNS-SD are each components that take bytes and timer ticks in and produce bytes, events and deadlines. None owns a socket, a thread or a clock. The network process's I/O thread waits once, in `kevent64`, on sockets, DNS-SD's socket, its Mach ports and its timers (EL2, EL8), and feeds each component. So:<br>• idle costs nothing (AC3);<br>• the whole stack runs under the test scheduler with virtual time and simulated sockets (EL24, AC6), as `quinn-proto`'s own tests do;<br>• no async runtime is in the engine. Tokio, which Servo uses, would bring its own threads, reactor and timers, against EL1, EL5 and EL24.<br>**S7-Q4 confirmed it, sandboxed** (2026-10-19): one thread, one `kevent64` per turn, drove rustls, `quinn-proto`, DNS-SD, a Mach port set (receiving in the wait itself), a timer with leeway and a user event, and woke for nothing but real deadlines when idle ([S7 results](../research/s7-results.md) §6) |
| NW2 | **One URL parser, Axiom's own, in every process.** The WHATWG URL parser runs in renderers (HTML, `URL`), in the network process (redirects, requests) and in the browser (navigation, the address bar, site computation). With one implementation, no two processes can parse a URL differently. **Its IDNA mode varies:**<br>• **full** UTS #46 processing through ICU4X (TX1), in renderers, the network process, and the browser's address bar, whose input is the user's and so trusted (TB5);<br>• **canonical ASCII only,** in the browser, for URLs that come from renderers or the network process. A URL that isn't a canonical ASCII serialization is a violation (IP11).<br>So ICU4X, whose data crates are graded `ub-risk-2`, never handles less-trusted data in the browser (DP9). That answers [text and fonts](text-and-fonts.md) open question 5.<br>**As amended at Milestone 1's second checkpoint** (2026-10-23), by the owner's conditional approval, whose condition the [audit of `smallvec`](../research/smallvec-audit.md) met: "If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." ICU4X's normalizer, which full UTS #46 processing needs, depends on `smallvec`, graded `ub-risk-3`, which DP8 admits only in sandboxed helpers. So:<br>• **full** UTS #46 processing runs only in renderers and the network process, in `url-idna`, a `sandboxed` crate that supplies `url`'s IDNA step through a trait `url` owns (CR9; the [crate architecture](crates.md)'s open question 1);<br>• **the browser parses every URL in canonical-ASCII mode,** the address bar's included: the network process turns a typed host into its ASCII form over the browser's interface to it, and the browser checks the answer as it checks any URL from another process (IP11).<br>Neither ICU4X's normalizer nor `smallvec` then runs in the browser at all. **The trust this adds is none the network process lacks:** it already decides which server answers for a host and whether its certificate is valid (TB1), so a compromised network process could misdirect a typed host whether or not it computed the ASCII form. How the address bar shows a host in Unicode is the shell's (M1-CP12), on the same path **Origins and sites** are Axiom's own, as security policy (DP1 (f)). A site is a scheme with a registrable domain, from the Public Suffix List, which is compiled in at build time and refreshed by rebuilds. **The oracle** is WPT's `url` directory, with the `url` crate as a differential reference |

### 5.2 HTTP and transport

| # | Rule |
|---|---|
| NW3 | **HTTP/1.1 is Axiom's own,** a strict parser and serializer. Ambiguous framing is rejected: conflicting `Content-Length` and `Transfer-Encoding`, obsolete line folding, invalid field bytes. It does chunked encoding and keep-alive, and no pipelining |
| NW4 | **HTTP/2 is Axiom's own:** framing, HPACK, flow control, extensible priorities, and connection coalescing within a partition (NW8). **From M2** (as amended in Round 4, RV4-08): no criterion of M1 needs it, and the harness and the corpus copies speak HTTP/1.1 |
| NW5 | **HTTP/3 runs over `quinn-proto`,** with framing and QPACK of Axiom's own. Servers are discovered through `Alt-Svc` and HTTPS DNS records (NW7). Its UDP sockets are the I/O thread's. Connections fall back to TCP when QUIC is blocked. S7 drove `quinn-proto` from the I/O thread's wait, with an HTTP/3 GET to Cloudflare over framing and QPACK of its own ([S7 results](../research/s7-results.md) §6) |
| NW6 | **TLS is rustls, and certificates are verified by the system.** rustls drives TLS 1.2 and 1.3 as a state machine (NW1), with no `unsafe` of its own (§3). **Verification** calls Security's trust evaluation (`SecTrustEvaluateWithError`) through `platform`, as `rustls-platform-verifier` does. So the trust store, the user's trust settings, Apple's CA constraints and revocation are macOS's (SG7). **The crypto provider** is `aws-lc-rs`, which the rustls project recommends, and which, unlike its `ring` provider, supports post-quantum algorithms (§3). Its C and assembly fall under DP10's exception: it runs only in the sandboxed network process, no pure-Rust provider has the audit record a browser's TLS needs (inference), and ADR-0013 accepts it. DP3 holds, since the cryptography is not Axiom's own. **TLS sessions and tickets are keyed by network partition key** (NW8), so resumption can't link a user across sites. **Network framework** (Apple's TLS and QUIC) is spike S7's comparison. It is patched by macOS updates, but delivers callbacks on dispatch queues with threads of its own, against NW1.<br>**S7 settled this stack** (2026-10-19; [results](../research/s7-results.md) §4). In the network process's profile, rustls verified through Security retired 3.3 M instructions per full handshake with Cloudflare and 7.3 M with Apple, against Network framework's 10.9 M and 18.0 M, and 4.4 against 6.9 instructions per downloaded byte; Network framework, in the configuration tried, never resumed a session. **rustls's `prefer-post-quantum` feature is required:** without it, servers that prefer X25519MLKEM768 answer every full handshake with a HelloRetryRequest. `SecPolicyCreateSSL` is treated as fallible, and verification fails closed.<br>**Confirmed by the owner at Gate C** (2026-10-20): "Certificate evaluation must run on worker pool threads to keep the network I/O thread non-blocking." **Verification runs in the worker pool (EL8), never on the I/O thread.** `SecTrustEvaluateWithError` blocks its caller while `trustd` answers: 8.4 ms at the median in S7's formal runs, and 1.0–1.5 s in three informal cases. EL1 gives the I/O thread one place to block. So a connection still in its handshake is handed, with the records read, to the pool, which processes them, and so verifies; it returns with an `EVFILT_USER` wakeup. The sockets, timers and wait stay the I/O thread's (inference; designed at the network milestone) |
| NW7 | **DNS is the system's, through DNS-SD.** `DNSServiceGetAddrInfo` resolves addresses, and `DNSServiceQueryRecord` fetches HTTPS records. Both run asynchronously, with `DNSServiceRefSockFD` registered in the I/O thread's `kqueue`. So the system's configuration applies: VPNs, per-domain resolvers, multicast DNS. **No blocking resolver is needed,** and the worker pool does no DNS work, which answers the handed-over question 10. DNS over HTTPS is a later setting, through Axiom's own HTTP stack. **S7-Q3 confirmed it in the sandbox** (2026-10-19): nine queries on one shared connection, from kqueue, with one thread. On a shared connection `kDNSServiceFlagsMoreComing` speaks for all queries (`dns_sd.h`), so the resolver settles queries only when a batch ends ([S7 results](../research/s7-results.md) §5) |
| NW8 | **Connections are partitioned.** Connection pools, TLS sessions, HTTP/2 and HTTP/3 coalescing, and DNS caches are keyed by Fetch's network partition key: the top-level site, and a second key Axiom sets to the frame's site (Fetch §2.7 leaves the second key implementation-defined) [W1]. **Why this key has no ancestor bit,** unlike the storage key since Round 2 (SW1, RV2-10): for a frame of site A inside site B inside site A it gives (A, A), as Chromium's network anonymization key does ("a->b->a => is_cross_site = false"), and Round 2 left that parity alone. The storage key differs because there B would control a frame holding A's first-party data. Happy Eyeballs races IPv6 and IPv4. Idle connections close after a timeout, with leeway (EL18) |
| NW9 | **Proxies come from the system's settings** (SystemConfiguration): HTTP, HTTPS and SOCKS proxies. Proxy auto-configuration scripts come later, evaluated by Axiom's JavaScript engine in a sandboxed utility process, never in the network process or the browser |

### 5.3 Cache and cookies

| # | Rule |
|---|---|
| NW10 | **The HTTP cache is partitioned by network partition key** (Fetch §2.8: the HTTP cache is "the unique HTTP cache associated with key") [W1]. It is stored in the network process's data directory, in a format CP7 chooses together with storage; **as amended in Round 4** (RV4-17), chosen at M2's plan with SW2's engine, by SW13's crash tests, since the cache arrives at M2. It is bounded in size, evicted least-recently-used, and follows RFC 9111's semantics through Fetch's cache modes, which are core (DP1 (e)) |
| NW11 | **Cookies follow RFC 6265's revision,** and live only in the network process (process model §3).<br>• **Defaults:** `SameSite=Lax` by default (SG8); the `Secure`, `HttpOnly`, `__Host-` and `__Secure-` rules; the `Partitioned` attribute.<br>• **Third-party cookies are blocked by default,** and all third-party state is partitioned by top-level site and frame site (SI3 (a)). Whether the `Partitioned` attribute's key needs the storage key's ancestor bit (SW1, Round 2) is checked at M2, when the jar is built.<br>• **`HttpOnly` cookies never reach a renderer** (SG2). `document.cookie` goes through the document's storage endpoint, bound at commit (FT11), and the network process filters what it returns.<br>• **Storage:** the jar's format is CP7's. Domain cookies are checked against the Public Suffix List (NW2) |

### 5.4 Fetch's policy

| # | Rule |
|---|---|
| NW12 | **The network process enforces Fetch's policy from the browser's records, never from a renderer's claims** (SG4, IR2). Every request arrives on a loader endpoint bound at commit to its document's origin, site lock, policy container and network partition key (FT11). The network process derives the initiator and the partition from the endpoint. A request field that restates them is checked against it, and a mismatch is a violation (IR2, IP11). On that basis it enforces:<br>• **CORS,** with preflights and a partitioned preflight cache;<br>• **CORP and COEP,** for cross-origin isolation;<br>• **ORB:** every no-CORS cross-origin response is checked before any byte reaches a renderer, by the proposal being upstreamed into Fetch [O1], so cross-site HTML, JSON and XML never enter a renderer (SI3 (b)). This answers how the network process enforces initiator locks and ORB (ADR-0004);<br>• **mixed content:** optionally blockable content upgraded, the rest blocked;<br>• **Referrer-Policy:** `strict-origin-when-cross-origin` by default;<br>• **redirects:** every hop re-checked against this whole list;<br>• **HSTS,** with the preload list compiled in and refreshed by rebuilds;<br>• **bad ports** (Fetch §2.9);<br>• **Local Network Access:** a request from a public site to a local or loopback address needs the user's permission, which the browser asks for (EM5). Chrome launched this model in version 142, replacing Private Network Access's preflights [C2]. The address space is judged from the address actually connected to, after DNS, which defeats DNS rebinding. This answers the threat model's check.<br>CSP's fetch directives run in the renderer, as Fetch's hooks place them (CP13). The network process enforces what protects other sites' data |
| NW13 | **Decoding content** (gzip, deflate, Brotli, zstd) happens in the network process, through `miniz_oxide`, `brotli-decompressor` and `ruzstd`. Ratio and size limits stop decompression bombs |
| NW14 | **Bodies stream to renderers** through single-writer rings or out-of-line Mach memory (IP14), with backpressure from the renderer (IP16, IP17). Sniffing for ORB happens in the network process; sniffing for rendering, in the renderer |
| NW15 | **WebSockets are Axiom's own framing,** over HTTP/1.1's upgrade, and later over HTTP/2's extended CONNECT. WebTransport comes later, over HTTP/3. Both go through NW12's checks |

### 5.5 Threads, memory, confinement and tests

| # | Rule |
|---|---|
| NW16 | **Threads and memory.** The I/O thread runs everything in NW1. The storage thread owns the cookie jar and the cache index. The worker pool does disk-cache reads and writes (EL8). Budget: the idle network process at ≤ 25 MB (B4), with the disk cache's mapped files excluded |
| NW17 | **The network process is a compartment** (SG10). Its sandbox profile allows outbound TCP and UDP, the system's DNS service, certificate trust evaluation and its own data directory, and nothing else. **S7 validated it by S4's method** (2026-10-19; [results](../research/s7-results.md) §3): eight rules, `spikes/s7/net/profiles/network.sb`. Besides the rendezvous lookup and the code-generation denial, they are the four kinds above (`com.apple.trustd.agent` being the trust service) and two metadata rules the frameworks need, which the kernel's log didn't reveal: `/var` (DNS-SD's socket path) and the executable's directory (without it `SecPolicyCreateSSL` fails). Its removal tests and reporting runs are the profile's regression check. A compromise exposes traffic and cookies, but not user files, the interface or other secrets |
| NW18 | **Tests:**<br>• WPT's `fetch`, `cors`, `mixed-content`, `cookies`, `url` and `websockets` directories, against a local test server;<br>• every protocol state machine under simulated sockets and virtual time (NW1);<br>• fuzzing for every parser: URLs, HTTP/1.1, HPACK, QPACK, cookies, decompression (Step 6) |

## 6. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **The browser's core** (FT7, FT10, FT11) | Navigation requests on the browser's behalf; endpoints bound at commit; permission requests for Local Network Access |
| **Renderers** (CP13, CP16) | Loader endpoints for subresources and `fetch()`; storage endpoints for `document.cookie`; streamed bodies |
| **Storage and workers** (CP7) | The cache's and the cookie jar's storage engine; service workers' interception, inside Fetch |
| **Text** (CP3) | IDNA through ICU4X (NW2) |

## 7. Security

- **Rule of 2:** every network parser runs in the sandboxed network process, mostly as safe Rust of Axiom's own. The browser parses only canonical ASCII URLs from other processes (NW2), and validates IPC with generated decoders (IP8).
- **Memory-unsafe code:** the TLS crypto provider (C and assembly) and Security's trust evaluation (Apple's C) run only in the network process, under DP10's exception and DP21, accepted by ADR-0013.
- **Policy from records:** initiators, partitions and locks come from endpoints bound by the browser (NW12, IR2).
- **Tracking resistance:** caches, connections, TLS sessions and cookies are partitioned, and third-party cookies are blocked (NW8, NW10, NW11).

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First network milestone (M1) | URLs; HTTP/1.1 over rustls; DNS-SD; an in-memory cache; mixed content, Referrer-Policy, HSTS. **As amended in Round 4** (RV4-08): HTTP/2 moves to M2 |
| M2 | HTTP/2 with HPACK (RV4-08); the partitioned disk cache and cookie jar, on the storage engine chosen at M2's plan (RV4-17); CORS |
| L1 | The network process is sandboxed with S7's rules (NW17) from M1, since Round 1 (RV1-07); L1, at M2, certifies it with audit tokens and the complete validators |
| L2 | Site locks bound into endpoints; ORB; Local Network Access; COEP and CORP in full |
| Later | HTTP/3 over `quinn-proto`; WebSockets over HTTP/2; WebTransport; DNS over HTTPS; proxy auto-configuration |

**The complete model** (SC9): service workers intercept within Fetch (CP7); Early Hints, compression dictionaries and Encrypted Client Hello fit the same state machines, when their specifications settle.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **hyper on tokio,** as Servo does | Mature, but it brings an async runtime with its own threads and clocks into the engine, against EL1, EL5 and EL24. Rejected (NW1) |
| B | **Network framework for TLS and QUIC** | Patched by macOS updates, and Safari's behaviour. But its callbacks run on dispatch queues, and it controls session caching itself. **S7-Q2:** 2.4–3.3× rustls's instructions per handshake and 1.6× per byte, with more threads, and no resumption in the configuration tried. Rejected (NW6) |
| C | **CFNetwork or `URLSession`** | It would take over HTTP, caching, cookies and redirects, the policies DP1 makes core. Rejected |
| D | **A bundled root store** (`webpki-roots`) | It would ignore the system's trust settings, against SG7. Rejected |
| E | **The `url` crate** | It works, but a second parser beside the browser's would invite parser differentials between processes, and its IDNA path would bring ICU4X into the browser for untrusted URLs. Rejected (NW2); kept as a reference |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The network process's sandbox rules, and rustls against Network framework | **Settled by spike S7** (2026-10-19): NW6, NW17 |
| 2 | The storage format for the cache and the cookie jar | CP7 |
| 3 | DNS over HTTPS, and the settings that control it | M5's plan, with Encrypted Client Hello (the roadmap's decisions; RV1-20) (Round 4, RV4-14) |
| 4 | Encrypted Client Hello, once the client side is available in rustls and the DNS records are deployed | A later milestone. **Corrected in Round 1** (RV1-20): rustls has had experimental client-side ECH since 2024-06-13 [P1]. ECH is planned with DNS over HTTPS at M5's plan, since its keys come from DNS records and the server's name stays private only if the lookup does too (background) |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [P1] Prossimo (ISRG), "Encrypted Client Hello (ECH) Support for Rustls", 2024-06-13, read 2026-10-21 (Round 1, RV1-20) — https://memorysafety.org/blog/rustls-ech-support
- [W1] WHATWG, Fetch Standard (Living Standard, last updated 2026-10-06), §2.7 "Network partition keys", §2.8 "HTTP cache partitions", §2.9 "Port blocking" — https://fetch.spec.whatwg.org/
- [I1] IETF datatracker, `draft-ietf-httpbis-rfc6265bis` — https://datatracker.ietf.org/doc/draft-ietf-httpbis-rfc6265bis/
- [O1] "Opaque Response Blocking (ORB)", README of `annevk/orb` — https://github.com/annevk/orb
- [C1] Chromium, `third_party/boringssl` and `net/data/ssl/chrome_root_store/root_store.md` — https://github.com/chromium/chromium/blob/main/net/data/ssl/chrome_root_store/root_store.md
- [C2] "Local Network Access", Chrome for Developers (2025-06-09) — https://developer.chrome.com/blog/local-network-access ; specification — https://wicg.github.io/local-network-access/
- [F1] Firefox, `modules/libpref/init/StaticPrefList.yaml` (`security.enterprise_roots.enabled`) — https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/StaticPrefList.yaml
- [S1] Servo, `components/net/Cargo.toml` — https://github.com/servo/servo/blob/main/components/net/Cargo.toml
- [R1] rustls, README and `rustls/src/lib.rs` — https://github.com/rustls/rustls
- [R2] `rustls-platform-verifier`, README — https://github.com/rustls/rustls-platform-verifier
- [Q1] Quinn, README — https://github.com/quinn-rs/quinn
- [A1] Google's audits — https://github.com/google/supply-chain/blob/main/audits.toml ; Chromium's — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml ; crates.io's API, for versions and dates
- Local: the SDK's `Network.framework`, `Security.framework` and `usr/include/dns_sd.h` headers
