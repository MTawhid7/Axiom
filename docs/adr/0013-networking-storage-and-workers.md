# ADR-0013: Networking, storage and workers

| | |
|---|---|
| **Status** | **Accepted** (2026-10-19). The owner pre-approved its acceptance at Step 5's Gate B, effective once S7-Q1, S7-Q3 and S7-Q4 passed. They passed on 2026-10-19 ([S7 results](../research/s7-results.md)), and S7-Q2 confirmed NW6's stack. **Verification in the worker pool, off the I/O thread** (NW6), was confirmed by the owner at Step 5's Gate C (2026-10-20). **The networking design's open question 4 corrected by the architecture review's Round 1** (2026-10-21): rustls has client-side ECH; planned with DNS over HTTPS at M5's plan (RV1-20) |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Networking](../architecture/networking.md) (NW1–NW18); [storage and workers](../architecture/storage-and-workers.md) (SW1–SW13); [threat model](../security/threat-model.md) SG2, SG4, SG7, SG8, SG10; [process model](../security/process-model.md) SI3, B4; [frame tree](../architecture/frame-tree-and-navigation.md) FT6, FT10, FT11; [ADR-0004](0004-process-model-and-sandbox.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [Step 5 tracker](../research/step-5-subsystems.md) §10.3 (spike S7); [S7 results](../research/s7-results.md); decision-map item I3 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9 |
| 2026-10-18 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing." Spike S7 is "Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts" |
| 2026-10-19 | **Spike S7: S7-Q1, S7-Q3 and S7-Q4 pass; S7-Q2 reported** ([S7 results](../research/s7-results.md), formal run at `4840d7d`).<br>• S7-Q1: under a broker-compiled deny-by-default profile, rustls verified through Security gave the expected verdict for every host it reached, the same as unsandboxed and as Network framework. The profile has eight rules, two of them metadata rules found by failure (`/var`; the executable's directory); each opening rule S7 uses was checked by removal, and every denial is explained.<br>• S7-Q2: rustls retired 2.4–3.3× fewer instructions per handshake than Network framework, and 1.6× fewer per byte; it settles NW6 as designed.<br>• S7-Q3: DNS-SD resolved addresses and HTTPS records from kqueue, with one thread.<br>• S7-Q4: one I/O thread drove every source from one `kevent64` per turn, and woke only for real deadlines when idle.<br>S7 also refines NW6: rustls's `prefer-post-quantum` feature is required; and it **proposes**, for the owner at Gate C, that verification run in the worker pool, because `SecTrustEvaluateWithError` blocks its caller (8.4 ms median; 1.0–1.5 s in three informal cases), which EL1 forbids on the I/O thread |
| 2026-10-19 | **Accepted,** the owner's condition met: S7-Q1, S7-Q3 and S7-Q4 pass |
| 2026-10-20 | **NW6's verification confirmed** by the owner at Step 5's Gate C: "NW6 TLS Verification: Confirmed. Certificate evaluation must run on worker pool threads to keep the network I/O thread non-blocking." [Networking](../architecture/networking.md) NW6 records it. In the same decisions, the network profile's two metadata rules go into the process model's change record: "Process Model: Approved. Record the S5 and S7 sandbox metadata rules in the process model change record." |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-20: "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." A stale fact is corrected in [networking](../architecture/networking.md)'s open question 4 and in the handover: rustls has had experimental client-side Encrypted Client Hello since 2024-06-13; ECH is planned with DNS over HTTPS at M5's plan |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-04, with the owner's placement: "Approved. Arbitrate Web Locks in the browser process (matching Chromium's LockManager pattern) so that helper process crashes/restarts cannot violate mutual exclusion." SW10: Web Locks are arbitrated in the browser's core, by storage key. SW3: the browser's copy of `sessionStorage` is kept current and restores the network process's after a relaunch. SW13: the crash tests check what the clients see. In the [storage design](../architecture/storage-and-workers.md) |

## Question

How does the network process fetch, verify, cache and store, and how does it enforce Fetch's policy? ADR-0004 left three questions here: initiator locks and ORB, and the TLS stack. Where do service and shared workers run?

## Requirements and constraints

- **SG4:** policy from the renderer's lock, never from its claims.
- **SG2:** `HttpOnly` cookies never reach a renderer.
- **SG7:** TLS validated against the system trust store.
- **SI3 (a), (b):** partitioned third-party state; ORB.
- **FT11:** endpoints bound at commit.
- **EL1, EL2, EL24:** one wait per thread; deterministic tests.
- **ADR-0004:** Apple's TLS, if used, stays out of the browser.
- **ADR-0010:** `httparse` (`ub-risk-4`) and `flate2` are barred or unneeded; C crypto needs DP10's exception.

## Alternatives considered

- **Network stack:**
  - hyper on tokio, as Servo does;
  - Network framework for TLS and QUIC;
  - CFNetwork or `URLSession`;
  - a bundled root store;
  - the `url` crate;
  - the proposal: state machines on one I/O thread, with rustls and the system's certificate verification.
- **Storage:**
  - SQLite first;
  - storage in the browser, or in renderers;
  - the proposal: `redb`, with SQLite as the fallback.

## Historical context and modern developments

- **Servo** runs rustls, the platform verifier and hyper on tokio.
- **Chromium** ships BoringSSL and its own root store. It replaced Private Network Access with Local Network Access in Chrome 142.
- **Firefox** trusts macOS's roots alongside NSS's.
- **Fetch** defines network partition keys and HTTP cache partitions; ORB is being upstreamed into it.
- **Storage:** Firefox and WebKit store IndexedDB in SQLite, and Chromium is moving there.

Sourced in [networking](../architecture/networking.md) §3 and [storage and workers](../architecture/storage-and-workers.md) §3.

## Analysis

| | Proposal | Main alternative |
|---|---|---|
| Threads and tests | State machines on one wait; virtual time | tokio: its own threads, reactor and clock |
| TLS | rustls, with no `unsafe`; C crypto under DP10's exception; the system verifies | Network framework: patched by macOS updates, but run on dispatch queues |
| URLs | One parser in every process; canonical ASCII only in the browser | Two parsers: a risk of parser differentials |
| Storage | `redb`, pure Rust, if audited; crash tests decide | SQLite: the best-tested, but C on web data |

## Evidence and sources

[Networking](../architecture/networking.md) and [storage and workers](../architecture/storage-and-workers.md), §3–§4, and their Sources. Observations: SDK 27's Network, Security and DNS-SD headers. **Measured:** [S7 results](../research/s7-results.md): the network process's profile, verification inside it, both TLS stacks' costs, DNS-SD and the one wait.

## Decision

**Accepted: NW1–NW18 and SW1–SW13** (2026-10-19), S7-Q1, S7-Q3 and S7-Q4 having passed. S7-Q2 confirmed NW6's stack. The storage engine is settled at the first storage milestone, by SW13's crash tests and `redb`'s audit, under SW2's rule as written. In summary:
1. **Protocols are state machines** driven by the I/O thread from one `kevent64` wait; no async runtime (NW1).
2. **One URL parser, Axiom's own, everywhere.** In the browser, only canonical ASCII URLs from other processes, so ICU4X stays off less-trusted data there (NW2).
3. **Our own HTTP/1.1, HTTP/2 and HTTP/3 framing,** with HPACK and QPACK; `quinn-proto` for QUIC (NW3–NW5).
4. **rustls,** with the system verifying certificates. Its `aws-lc-rs` crypto is accepted under DP10's exception, in the network process only (NW6).
5. **DNS-SD** for name resolution and HTTPS records (NW7).
6. **Partitioning:** connections, TLS sessions, the cache and cookies, by network partition key. Third-party cookies blocked (NW8, NW10, NW11).
7. **Fetch's policy from bound endpoints:** CORS, CORP, COEP, ORB, mixed content, Referrer-Policy, HSTS, Local Network Access. That answers ADR-0004's questions (NW12).
8. **Web storage in the network process,** by storage key. One engine: `redb` if audited, otherwise SQLite under DP10's exception, accepted by this ADR (SW1, SW2, SW7).
9. **`localStorage` cached in the renderer;** IndexedDB values never parsed outside it (SW3, SW4).
10. **Service workers intercepted in Fetch.** Shared and service workers run in a renderer locked to their site, launched when none exists (SW8, SW9).

## Rationale

- One wait per thread, and deterministic tests, are binding (EL1, EL24). Sans-I/O protocol code is the shape that keeps them.
- Keying policy to the browser's records is the core of SG4. Partitioning everything denies cross-site tracking by construction.
- Every component that parses network bytes is safe Rust, except the audited crypto provider.

## Assumptions

- Security's trust evaluation works inside a deny-by-default profile with a reviewed set of rules. **Confirmed by S7-Q1:** eight rules, each explained ([S7 results](../research/s7-results.md) §3.1).
- `quinn-proto` and rustls stay sans-I/O.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Our own HTTP/2 and HTTP/3 take longer than reusing | High | Medium | Staging: HTTP/1.1 and HTTP/2 first, HTTP/3 later; the protocols are fully specified |
| Trust evaluation needs broad sandbox rules | **Resolved** by S7-Q1 | — | Two metadata rules beyond NW17's four kinds: `/var`, and the executable's directory ([S7 results](../research/s7-results.md) §3.1) |
| Trust evaluation blocks the I/O thread | Certain: 8.4 ms median, up to 1.5 s seen (S7) | Medium: every connection, message and timer waits meanwhile | Verification in the worker pool (EL8), confirmed in NW6 by the owner at Gate C |
| Each network process's first TLS use costs 29–43 ms of CPU, for AWS-LC's entropy source | Certain (S7) | Low: once per process | Decided at the network milestone, possibly building without the jitter source ([S7 results](../research/s7-results.md) §4.3) |
| `redb` fails its audit or crash tests | Medium | Low | SQLite, already accepted here as the fallback (SW2) |
| ORB's specification shifts before it settles in Fetch | Medium | Low | Follow the upstreaming pull request |

## Conditions for revisiting

- rustls or `quinn-proto` stops supporting external event loops.
- The system's trust evaluation becomes unavailable to sandboxed helpers.

## Consequences

- `platform` gains Security, DNS-SD and SystemConfiguration calls for the network process. `SecPolicyCreateSSL` is treated as fallible: it returns NULL when the main bundle can't be created, although its header promises a policy, and verification then fails closed (S7).
- The network process's sandbox profile starts from S7's `spikes/s7/net/profiles/network.sb` (NW17).
- The [register](../technology/dependency-register.md) records rustls, `aws-lc-rs`, `quinn-proto`, `miniz_oxide`, `ruzstd`, `redb`, and SQLite as the fallback.
- CP13 consumes the URL parser and the body streams. CP16 builds `fetch`, the storage APIs and workers on these endpoints.
