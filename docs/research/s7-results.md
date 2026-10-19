# S7 Results: The Network Process and TLS

| | |
|---|---|
| **Status** | Complete (2026-10-19). S7-Q1, S7-Q3 and S7-Q4 pass; S7-Q2 is reported, and settles NW6 as designed: rustls, verified by Security |
| **Question** | Under a deny-by-default profile compiled by a broker, does a helper's TLS client verify servers through `SecTrustEvaluateWithError`, and which rules does it need (S7-Q1)? What do rustls with `aws-lc-rs` and Network framework's TLS cost in handshake latency, CPU per megabyte and footprint (S7-Q2)? Do DNS-SD's address and HTTPS-record queries, driven from kqueue, work under the profile (S7-Q3)? Does one I/O thread drive rustls, `quinn-proto`, DNS-SD, a Mach port and timers from a single `kevent64` call, with no wakeups while idle (S7-Q4)? |
| **Feeds** | [ADR-0013](../adr/0013-networking-storage-and-workers.md) (its acceptance conditions: S7-Q1, S7-Q3 and S7-Q4); [networking](../architecture/networking.md) NW1, NW5, NW6, NW7, NW17 and its open question 1; the [dependency register](../technology/dependency-register.md); a refinement of the [process model](../security/process-model.md) §4.1 for the owner |
| **Plan** | [Step 5 tracker](step-5-subsystems.md) §10.3, authorized at Gate B (2026-10-18) |
| **Code** | `spikes/s7/net` (`s7-net`) and `tools/s7-measure.sh`, on branch `step-5`. Formal run at commit `4840d7d` (§2) |
| **Last updated** | 2026-10-20: the owner's Gate C decisions on §9's items 3 and 5 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from S7's binary and script, in the formal run named in §2 unless stated otherwise.
- **Observed** facts were read on this Mac: SDK headers, the unified log, the denial log, crate sources. Each names its file or command.
- **Sourced** facts link to their source, listed at the end.
- **Inference** marks an explanation drawn from them; **hypothesis** marks an untested claim, with the step that would test it.

---

## Summary

| Question | Answer |
|---|---|
| **S7-Q1** Does verification work under the profile, and which rules does it need? | **Passes.** Under the broker-compiled profile, rustls, verified through `SecTrustEvaluateWithError`, gave the expected verdict for every host it reached, in all three runs. The ten good hosts verify, and expired, self-signed, wrong-host, untrusted-root and SHA-1-intermediate certificates are rejected with Security's codes. The verdicts are the same unsandboxed, under a profile that allows everything, and with Network framework. Three of the 54 sandboxed attempts reached no verdict: badssl.com's servers reset two connections and let one time out, before verification. **The profile has eight rules** (§3.1): the four kinds NW17 names, two metadata rules found by failure, the rendezvous lookup and the code-generation denial. Removing each rule that opens something S7 uses breaks what it serves. **Denials:** ten kinds were logged, all explained. The kernel logs some denials and not others, so reporting runs listed every operation the helper attempts, and each one the profile denies is explained (§3.3) |
| **S7-Q2** rustls against Network framework? | **Report. NW6 is settled as designed: rustls, verified by Security.** Per full handshake (medians of 57), rustls retired 3.3 M instructions with Cloudflare and 7.3 M with Apple, against Network framework's 10.9 M and 18.0 M (3.3× and 2.4×), in 2.6 ms of CPU against 8.0 ms, and 7.2 ms against 17.7 ms. From start to the handshake's end, rustls took 24 ms against 28 ms (Cloudflare) and 134 ms against 135 ms (Apple). Resumption saved rustls a round trip (16 ms and 83 ms); Network framework, in the configuration tried, never resumed. Over 50 MB downloads, rustls needed 2.1 ms of CPU per MB, 4.4 instructions and 2.5 cycles per byte, against 4.2 ms, 6.9 and 4.6. Footprints after work: 2.8–4.1 MiB with rustls, 4.3–7.8 MiB with Network framework; 3–4 threads against 4–5 (§4) |
| **S7-Q3** DNS-SD in the sandbox? | **Passes.** Nine queries at once on one shared connection: addresses in 1.4–5.7 ms, HTTPS records, with ALPN, address hints and Cloudflare's ECH configuration, in 0.2–20 ms; the whole batch in 17–23 ms, with 4–6 `kevent64` returns, 0.35–0.43 ms of CPU, and **one thread** throughout, so no thread blocked. It needs one rule beyond the socket's: the metadata of `/var` (§5) |
| **S7-Q4** One wait? | **Passes.** In each run, one I/O thread completed a rustls fetch, a 2 MB rustls download, an HTTP/3 GET over `quinn-proto` (126 KB), an HTTPS-record query, 50 Mach messages, which the wait itself received (mean latency 0.05–0.61 ms), and 12–13 timer ticks, in 116–117 turns of one `kevent64` call each. **Idle:** in each 30 s window the thread woke only for a real deadline, QUIC's idle timeout, at 27.7–27.9 s, and for the window's end; the idle-wakeup counter stayed at 0. Windows of 10 s and 60 s (informal) show no periodic wakeup (§6) |

**Main findings**, beyond the pass criteria:
1. **Trust evaluation blocks its caller.** `SecTrustEvaluateWithError` waits on `trustd` over XPC: 8.4 ms median, 12.5 ms at the 90th percentile and 20.6 ms at most in the formal runs (186 evaluations). Informal runs saw it take 1.0–1.5 s three times, while rejecting chains. On the I/O thread, that is a second place to block, which EL1 forbids. **Proposed for NW6, for the owner at Gate C:** verification runs in the network process's worker pool (EL8), never on the I/O thread (§9).
2. **Two needs drew no denial in the log** (§3.1). Without the metadata of `/var`, DNS-SD fails with `kDNSServiceErr_ServiceNotRunning`. Without the metadata of the executable's directory, `SecPolicyCreateSSL` returns NULL, although its header promises a policy, and objc2-security panics on that NULL. The second rule is the one S5 found for ImageIO, so S5's proposed refinement of the process model applies to the network process too.
3. **rustls needs its `prefer-post-quantum` default feature.** The workspace had disabled rustls's default features to drop its logging, and that dropped this one too: every full handshake with Cloudflare then took a HelloRetryRequest, a round trip more. With it, X25519MLKEM768 is negotiated with Cloudflare, Apple, Wikipedia and example.com, and X25519 with GitHub, with no retry wherever the handshake's kind was recorded (Cloudflare and Apple formally, GitHub informally).
4. **Each process's first TLS connection costs 29–43 ms of CPU, once.** AWS-LC seeds its random generator from its CPU-jitter entropy source, whose start-up health tests take 367–376 M instructions on first use; later draws take 25 K. Network framework's first connection costs 19–26 M. `aws-lc-sys` builds that source on macOS unless `AWS_LC_SYS_NO_JITTER_ENTROPY` is set when it is built (§4.3).
5. **Network framework needed no new rule.** Under the same profile, its attempts to read networkd's preferences, open a kernel control socket, query interfaces and routes, and reach the `dnssd.service`, `usymptomsd` and `runningboard` services are denied, and it still connects and verifies (§3.3).
6. **Security's client side is a large share of a handshake.** Each evaluation costs about 1.3 M instructions in the helper (3.8–4.1 M the first time): 40% of a full rustls handshake with Cloudflare, before `trustd`'s own work.
7. **The helper is small:** 1.75 MiB sandboxed and idle, 2.8–4.1 MiB after rustls work, so B4's budget (≤ 25 MB) is far away. The build stays within T1–T4 (§7).

## 1. What was built

One crate, `spikes/s7/net` (`s7-net`), one executable with two roles:
- **The broker,** standing in for Axiom's browser process. It compiles `profiles/network.sb` with S4's tooling (`s4_sandbox::platform::sandbox::compile`), creates the helper's data directory, and launches this same executable as the helper with S4's `posix_spawn` launcher, passing the compiled bytecode on descriptor 3 (S4's precompiled method). The helper's environment is empty, as S4's launcher leaves it.
- **The helper,** standing in for the network process. It applies the bytecode at the top of `main`, before any network or Security call, then runs one mode.

| Mode | Question | What it does |
|---|---|---|
| `verify` | S7-Q1 | HTTPS GETs to 18 hosts (§3.2), with rustls verified through Security (`--stack rustls`), or with Network framework, which verifies by itself (`--stack nw`) |
| `probe` | S7-Q1 | What the profile allows and denies: the data directory, a home-directory listing, a write elsewhere, `/etc/hosts`, a TCP listener, a UDP bind, launching a process, and three Mach lookups |
| `handshakes` | S7-Q2 | `--count` full handshakes, then a warm-up and `--count` resumed ones, each a fresh connection with a small GET, per stack |
| `download` | S7-Q2 | One large download, after a warm-up fetch on the same host, with the process's CPU time, instructions and cycles across it |
| `ready` | S7-Q2 | Nothing: the sandboxed helper's own footprint |
| `dns` | S7-Q3 | Five address queries and four HTTPS-record queries at once, on one shared DNS-SD connection |
| `one-wait` | S7-Q4 | A rustls fetch and a 2 MB rustls download, an HTTP/3 GET over `quinn-proto`, an HTTPS-record query, 50 Mach messages from another thread, a 100 ms periodic timer and a user event, all from one I/O thread; then an idle window |

**Modules.**
- `src/platform/`, the only module with `unsafe` (allowlisted in `tools/unsafe-policy.txt`), under invariants N1–N9 in its `mod.rs`:
  - `trust.rs`: `SecCertificateCreateWithData`, `SecPolicyCreateSSL`, `SecTrustCreateWithCertificates`, `SecTrustSetVerifyDate`, `SecTrustSetOCSPResponse` and `SecTrustEvaluateWithError`, as rustls-platform-verifier's macOS verifier calls them [P1];
  - `dnssd.rs`: one `DNSServiceCreateConnection`, with `DNSServiceGetAddrInfo` and `DNSServiceQueryRecord` sharing it (`kDNSServiceFlagsShareConnection`), its socket from `DNSServiceRefSockFD`;
  - `kq.rs`: `kevent64`, with `EVFILT_READ`, `EVFILT_WRITE`, `EVFILT_TIMER` (`NOTE_LEEWAY`), `EVFILT_USER`, and `EVFILT_MACHPORT` with `MACH_RCV_MSG`, which receives the message into a registered buffer during the wait itself;
  - `mach.rs`: a port set holding a receive right, and send rights; `sock.rs`: non-blocking TCP and connected UDP sockets, which `std` cannot open; `usage.rs`: `getrusage`, `proc_pid_rusage` and `proc_pidinfo`;
  - `nw.rs`: Network framework's C API (`nw_connection_*`), with blocks and a dispatch queue, for S7-Q2's comparison only.
- `driver.rs`: the I/O thread. Each turn arms the earliest deadline as one `EVFILT_TIMER` with leeway (EL18), submits its changes and waits, all in one `kevent64` call, then dispatches.
- `tls.rs`, `quic.rs`, `http.rs`: the protocol state machines it drives: rustls over a non-blocking socket; `quinn-proto`, with the least HTTP/3 a GET needs (an empty SETTINGS frame, and QPACK with the static table only) [S1, S2, S3]; and an HTTP/1.1 response parser that rejects ambiguous framing (NW3).
- `verifier.rs`: rustls's `ServerCertVerifier`, which hands the chain to `platform::trust` and times each evaluation; the handshake's signatures are checked by the crypto provider.

**Deviations from §10.3,** none of which changes the authorization's scope:
1. **HTTP/3:** S7-Q4 names one QUIC connection. So that the connection carries work, it makes an HTTP/3 GET, with framing and QPACK of the spike's own, which NW5 also assigns to Axiom.
2. **The Mach messages come from a second thread in the helper,** not from the broker. S4 validated Mach IPC between a broker and a sandboxed helper; S7-Q4 asks whether the I/O thread's one wait receives it.
3. **Network framework needed no rule beyond `network.sb`** (§3.3), so S7-Q2 ran both stacks under the same profile, as planned, with no second profile.

**Dependencies,** each justified in the workspace manifest (DP23): rustls 0.23.46 with the `aws-lc-rs` provider, `quinn-proto` 0.11.19, `objc2-security` 0.3.2, `block2` 0.6.2, `dispatch2` 0.3.1 and `bytes` 1.12.1 (§8).

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B5112a), on AC power with Low Power Mode off (`pmset`); rustc 1.94.1; Command Line Tools only. The helper was a copy of the release binary signed ad hoc with the hardened runtime (`flags=0x10002(adhoc,runtime)`, SDK 27.0), as Axiom's helpers would be (ADR-0004, D2).
- **Formal run:** `tools/s7-measure.sh` at commit `4840d7d`, from a clean tree: `target/s7-results/20261019-071313/`. Three runs, each after a 30 s cool-down; then the removal tests and the reporting runs, once. The Mac was in normal use, not idle: the load averages were 3.3, 2.6 and 2.3 at the start. **Supplementary:** the `startup` mode (§4.3), added after the formal run, ran three times at commit `13daa44`. **Informal** observations, made while the spike was written, are labelled as such where they are used.
- **Network:** the Mac's Wi-Fi, to public hosts. Latencies depend on the path to each server, so they are reported as medians over many handshakes, and are indicative (§10.3's method). CPU, instructions and cycles are the helper's own (`getrusage`, `proc_pid_rusage`), so they don't depend on the network.
- **Counting.** CPU time is user plus system time of every thread in the helper. Instructions and cycles (`ri_instructions`, `ri_cycles`) and wakeups (`ri_pkg_idle_wkups`, `ri_interrupt_wkups`) come from `proc_pid_rusage`; threads from `proc_pidinfo`. Per-handshake figures cover one connection: name resolution, TCP, TLS, verification, a small GET and the response. Each process's first connection, which pays one-time initialization (§4.1), is reported separately, and the resumed series's warm-up is excluded.
- **Footprints** are `ri_phys_footprint`, as S4 and S5 read it, at each mode's end.
- **Denials.** The kernel logs most denials, which the script collects from the unified log (`/usr/bin/log`, sender `Sandbox`). It doesn't log every kind: two failures S7 met drew no log line (§3.1). So, after the formal runs, the script also runs the modes under a profile that allows everything **and logs it** (`(allow default (with report))`). That lists every operation the helper attempts, and each one the real profile doesn't allow is explained in §3.3.
- **Hosts and data:** badssl.com's test hosts [B1], example.com, www.wikipedia.org, www.apple.com, www.cloudflare.com, github.com, speed.cloudflare.com and cloudflare-quic.com, by read-only GETs; each run downloads about 2 × 50 MB, plus small pages.

## 3. S7-Q1: verification under the profile

### 3.1 The profile

`profiles/network.sb`, grown from NW17's four kinds of access by S4's method: deny by default, run, add only what a failure shows is needed, then check each rule by removing it. The removal tests are in the formal run (`q1-removal.txt`); every one passed.

| Rule | What it opens | Why | Without it (removal test) |
|---|---|---|---|
| `(deny default)` | Nothing | Deny by default (S4) | — |
| `(allow mach-lookup (global-name (param "RENDEZVOUS")))` | The broker's rendezvous service | Every helper's channel to the browser (S4). S7 doesn't exercise it | Not tested |
| `(deny dynamic-code-generation)` | Nothing: it forbids generating code | `(deny default)` doesn't cover it (S4-Q4); JIT rule J3 | — |
| `(allow network-outbound (remote tcp) (remote udp))` | Outbound TCP and UDP, to any remote address | NW17 | Without TCP, `connect` fails with EPERM; without UDP, QUIC's socket does |
| `(allow network-outbound (literal "/private/var/run/mDNSResponder"))` | DNS-SD's socket | NW17: the system's DNS service | `DNSServiceCreateConnection` returns `kDNSServiceErr_ServiceNotRunning` (-65563) |
| `(allow file-read-metadata (literal "/var"))` | The metadata of one symlink | `libsystem_dnssd` connects to `/var/run/mDNSResponder` (its log line names the path), and resolving `/var` needs its metadata. **Found by failure: the kernel logged no denial.** Metadata of `/private`, `/private/var`, `/private/var/run` or the socket instead doesn't help (informal, one test each) | As without the socket's rule |
| `(allow mach-lookup (global-name "com.apple.trustd.agent"))` | The per-user trust agent | NW17: certificate trust evaluation. The draft also allowed the system daemon, `com.apple.trustd`, which was never looked up, so it was removed | Every chain is rejected, good ones included, with OSStatus -26276, a code `SecBase.h` doesn't define and `security error` calls unknown. Verification fails closed |
| `(allow file-read-metadata (literal (param "EXECUTABLE_DIR")))` | The metadata of one directory | Without it, `CFBundleGetMainBundle` fails ("Unable to create bundle at URL (\<private\>): unable to create file system representation of URL", in the unified log), and `SecPolicyCreateSSL` returns NULL. **Found by failure: the kernel logged no denial.** It is S5's rule R1, which ImageIO needs for the same reason | Every chain is rejected ("SecPolicyCreateSSL returned NULL"). Verification fails closed |
| `(allow file-read* file-write* (subpath (param "DATA_DIR")))` | The process's own data directory | NW17 | The data directory's write fails with EPERM (`probe`) |

**The list is shorter than precedent's.** Chromium's network profile also allows the keychains, Security's and the DNS configuration's files, `AF_ROUTE` and kernel control sockets, binding local addresses, notifications, and the lookups of `configd`, `networkd`, `nehelper`, `ocspd` and the security server, among others [C1]. WebKit's allows `dnssd.service`, `nehelper`, `nesessionmanager`, `usymptomsd`, the netsrc control socket and `opendirectoryd` [W1]. S7 needed none of them for what it exercised: the system's DNS, TLS verified by the system, and QUIC. Inference: they serve features S7 didn't exercise, such as the system's proxy settings (NW9), path monitoring, Kerberos and client certificates. Each is added with its own evidence at its milestone.

### 3.2 Verification

Three runs of 18 hosts per configuration. Network framework reports SecureTransport's codes (`SecBase.h`): -9814 `errSSLCertExpired`, -9808 `errSSLBadCert`, -9813 `errSSLNoRootCert`.

| Host | Expected | rustls, in the profile (Security's code) | rustls, unsandboxed | Network framework, in the profile |
|---|---|---|---|---|
| badssl.com, sha256., ecc256., rsa4096. and tls-v1-2.badssl.com:1012; example.com, www.wikipedia.org, www.apple.com, www.cloudflare.com, github.com | Verify | Verified whenever it connected; two connections were reset (run 3) | Verified; one timeout | Verified |
| expired.badssl.com | Reject | -67818 `errSecCertificateExpired`; one timeout | -67818 | -9814 |
| self-signed.badssl.com | Reject | -67843 `errSecNotTrusted` | -67843 | -9808 |
| wrong.host.badssl.com | Reject | -67602 `errSecHostNameMismatch` | -67602 | -9808; one timeout |
| untrusted-root.badssl.com | Reject | -67843 | -67843 | -9808 |
| sha1-intermediate.badssl.com | Reject | -25318 `errSecCreateChainFailed` | -25318 | -9813 |
| no-subject.badssl.com | Either | -67818: its certificate has expired | -67818 | -9814 |
| incomplete-chain.badssl.com | Either | Verified: `trustd` found the missing intermediate | Verified | Verified |
| revoked.badssl.com | Either | -67820 `errSecCertificateRevoked` | -67820 | -9808 |

- **Of 162 attempts, 157 reached a verdict, each as expected.** The other 5 failed before any verification, all on badssl.com's servers: 2 connection resets and 3 timeouts. The formal run's own report called the resets and two of the timeouts "rejected", because it labelled every failure after TCP's connect so. The raw lines show what each was, and `verify` now labels only the verifier's errors as rejections (commit `13daa44`).
- rustls maps the common codes to its own certificate errors, so that it sends the matching alert. Others arrive with Security's description.
- **The incomplete chain and the revocation were resolved by `trustd`,** outside the sandbox. Nothing in the profile lets the helper fetch an intermediate or a revocation status, and nothing needs to: the reporting runs show no such attempt by the helper (observed).

**Evaluation time,** measured in the verifier around the call, so including the XPC round trip to `trustd`:
- sandboxed, 186 evaluations: 8.4 ms median, 12.5 ms at the 90th percentile, 20.6 ms at most;
- unsandboxed, 53: 9.1, 15.7 and 22.9 ms.

**Informal runs saw three far longer ones,** all rejecting chains on 2026-10-19: 997 ms for expired.badssl.com and 1,436 ms for wrong.host.badssl.com (unsandboxed), and 1,480 ms for revoked.badssl.com (a reporting run). **Hypothesis:** they happen when `trustd` first fetches revocation or certificate data; measured at the network milestone, where verification leaves the I/O thread in any case (§9).

### 3.3 Denials, explained

The kernel's log is not a complete census of denials (observed):
- the two failures in §3.1 drew no log line;
- Security's bundle reads were logged twice in three runs, though attempted in every run;
- right after the reporting runs, a pass to attribute denials to modes drew no log line at all, presumably because of rate limiting.

So the method adds **reporting runs:** each mode in its own process, under `(allow default (with report))`, which allows every operation and logs it (`q1-attempted.txt`). The table covers both: denials logged under the real profile in the formal runs (`sandbox-denials.txt`, counts over 3 runs), and operations attempted in the reporting runs that the profile doesn't allow.

| Operation | By | Logged when denied | Effect of the denial |
|---|---|---|---|
| Listing the executable's directory; the executable's metadata and contents | CoreFoundation's main bundle | Yes (60; 23) | None: the directory's metadata, which the profile allows, is enough for `SecPolicyCreateSSL` |
| Security.framework's bundle: its metadata, `Info.plist`, `en.lproj`, and the string tables `Trust.loctable`, `Certificate.loctable`, `SecErrorMessages.loctable` | Security, when a chain is rejected: attempted in `verify`, not in `one-wait`, where every chain verifies | Partly (2) | None on verdicts or codes. The rejections' descriptions still arrived, in English |
| ICU's data (`/usr/share/icu/icudt78l.dat`), the time-zone database, `/etc` and `/private/etc/localtime`, the home directory's metadata (`~`, `/Users`) | CoreFoundation's locale and time zone | Only `/etc` (23) and `/Users` (11) | None observed. Inference: CoreFoundation falls back to its defaults, and verification uses the absolute time rustls supplies |
| `cfprefsd`'s agent and daemon, and their shared memory | CoreFoundation's preferences | No | None observed |
| `system-info vfs.disk-space` | Unidentified framework code, in every mode that verifies | No | None observed |
| networkd's preferences (`/Library/Preferences/com.apple.networkd.plist`); a kernel control socket (`AF_SYSTEM`, `SYSPROTO_CONTROL`) and its connection; `net.link.addr`; `net.routetable` | Network framework only | The preferences (21) and the socket (16) | None observed: it connected and verified as it does unsandboxed |
| The `com.apple.dnssd.service`, `com.apple.usymptomsd` and `com.apple.runningboard` services | Network framework only | The first two (9; 6) | None observed: it resolved names through DNS-SD's socket, which the profile allows |
| Binding a local address (a TCP listener, a UDP bind) | `probe`, on purpose | Yes (3) | Denied, as intended |

**No denial affects verification** (measured): the verdicts under the profile equal those of the reporting runs, where nothing was denied, and those of the unsandboxed runs (§3.2).

### 3.4 What the profile denies (`probe`)

In every run, the data directory was written and read. Denied: listing the home directory; writing outside the data directory; reading `/etc/hosts`; a TCP listener; a UDP bind; launching `/usr/bin/true` (each EPERM); looking up `com.apple.windowserver.active` and `com.apple.pasteboard.1` (`BOOTSTRAP_NOT_PRIVILEGED`). `com.apple.trustd.agent` is found.

## 4. S7-Q2: rustls against Network framework

### 4.1 Handshakes

Medians over the formal runs, 57 full and 60 resumed handshakes per row; each process's first connection (§4.3) and the resumed series' warm-up are excluded.
- **Start to the handshake's end:** name resolution, TCP and TLS, with verification; for Network framework, until its ready state.
- **TLS alone** (rustls): from TCP's connect to the handshake's end.
- **CPU and instructions:** the whole connection: name resolution, TCP, TLS, verification, a small GET and its response.

| Server | Kind | Stack | Start to the handshake's end, ms (25th–75th percentile) | TLS alone, ms | CPU, ms | Instructions, M |
|---|---|---|---|---|---|---|
| www.cloudflare.com, on Cloudflare's network: a 9–12 ms round trip (QUIC's estimate to cloudflare-quic.com, §6) | Full | rustls | 24.2 (21.8–26.2) | 15.2 | 2.6 | 3.30 |
| | | Network framework | 28.2 (26.4–30.4) | — | 8.0 | 10.92 |
| | Resumed | rustls | 16.3 (15.6–17.8) | 7.5 | 1.9 | 1.49 |
| | | Network framework | 26.4 (24.2–27.5) | — | 8.7 | 11.10 |
| www.apple.com, a 40 ms round trip (inference, from resumed TLS) | Full | rustls | 133.8 (130.9–136.2) | 90.6 | 7.2 | 7.35 |
| | | Network framework | 134.7 (131.0–137.2) | — | 17.7 | 17.98 |
| | Resumed | rustls | 83.0 (80.9–85.2) | 40.3 | 5.5 | 5.39 |
| | | Network framework | 134.8 (131.5–138.1) | — | 16.9 | 18.16 |

- **rustls:** all 120 full handshakes were `Full`, and all 120 resumed ones `Resumed`. Both take one round trip in TLS 1.3, with verification on top of the full ones (§3.2). Apple's full handshake takes about two round trips (90.6 ms of TLS on a 40 ms path) and its resumption one (40.3 ms). Inference: its first flight, two certificates and a stapled OCSP response, exceeds what TCP's initial window carries.
- **Network framework never resumed.** Its resumed series shared one parameters object, with session resumption and tickets enabled (`sec_protocol_options_set_tls_resumption_enabled`, `sec_protocol_options_set_tls_tickets_enabled`), and matched its full series in latency and instructions. So it did unsandboxed (informal). Whether another configuration resumes is not established (§10).
- **Before `prefer-post-quantum`** (informal, 2026-10-19), every full rustls handshake with Cloudflare was `FullWithHelloRetryRequest`, 25–37 ms from start. With the feature, GitHub, which doesn't offer X25519MLKEM768, negotiates X25519 with no retry (informal), so rustls evidently also sends a classical key share (inference). GitHub never resumed a session either (informal).
- **Each process's first connection** cost rustls 29–43 ms of CPU and 377–387 M instructions, and Network framework 7–19 ms and 19–26 M (formal). §4.3 explains rustls's.

### 4.2 CPU per megabyte

Three runs of a 50 MB download from speed.cloudflare.com per stack, each after a warm-up fetch on the same host; medians, with ranges:

| Stack | CPU per MB, ms | Instructions per byte | Cycles per byte | MB/s | Context switches | Threads | Footprint after, MiB |
|---|---|---|---|---|---|---|---|
| rustls | 2.11 (1.64–2.82) | 4.43 (4.35–4.46) | 2.53 (2.07–2.82) | 66.7 (61.5–67.4) | 593–681 | 3 | 2.84–2.91 |
| Network framework | 4.24 (3.97–4.28) | 6.93 (6.74–6.98) | 4.61 (4.60–4.72) | 58.6 (55.8–60.4) | 1,692–1,756 | 4–5 | 5.05–7.81 |

- Both negotiated TLS 1.3 with AES-256-GCM. The download's own handshake is included: about 2% of rustls's instructions.
- Instructions are steadier than time: CPU time varies with frequency, and with which cores ran the threads, on a busy Mac.
- Network framework makes three times as many context switches, between its threads and the queue that receives the data.

### 4.3 One-time costs (`startup`)

Added after the formal run, and run three times in fresh helpers, sandboxed, on a signed copy at commit `13daa44` (`target/s7-results/20261019-073956-startup/`; the Mac was busy, with a load average of 5.7):

| Stage | CPU | Instructions |
|---|---|---|
| The provider and a client configuration | 0.2–0.4 ms | 0.5–1.2 M |
| **The first 32 random bytes** | **33.6–35.8 ms** | **367–376 M** |
| The next 32 | 5–7 µs | 25 K |
| Each key-exchange group's first use, after that | 0.01–0.6 ms | 0.1–2.4 M |
| kqueue and the DNS-SD connection | 0.1 ms | 0.2 M |
| The first handshake (Cloudflare) | 2.2–5.5 ms | 7.6–8.3 M, of which 3.8–4.1 M for the first trust evaluation |
| The second | 1.0–2.2 ms | 3.4–3.5 M, of which 1.3–1.4 M for its trust evaluation |

**The first random bytes pay for AWS-LC's seed.** Its tree DRBG seeds from the CPU-jitter entropy source, which it allocates with `JENT_FORCE_FIPS`, and that runs the source's start-up health tests (observed in the vendored source: `crypto/fipsmodule/rand/entropy/tree_drbg_jitter_entropy.c` and `third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c`). `aws-lc-sys` 0.46.0 builds that source on Apple targets whenever the SDK has CoreServices' header, unless `AWS_LC_SYS_NO_JITTER_ENTROPY=1` is set when it is built. Without the source, AWS-LC seeds from the operating system (observed: `builder/main.rs` and `entropy/entropy_sources.c`). **Hypothesis,** tested at the network milestone: building without the jitter source removes the 30–40 ms. Whether Axiom should is a question of entropy policy, not only of cost.

### 4.4 Footprint and threads

- **Sandboxed and idle** (`ready`): 1.75 MiB in every run, with one thread.
- **After work with rustls:** 3.9–4.1 MiB after `verify`, 3.6 after 41 handshakes, 2.8–2.9 after the download, 3.3–3.4 after `one-wait`. 3–4 threads: the helper's own, plus the threads Security's XPC connection to `trustd` adds (inference).
- **After work with Network framework:** 5.4–5.6 MiB after `verify`, 4.3 after 41 handshakes, 5.1–7.8 after the download. 4–5 threads.
- B4 budgets the idle network process at ≤ 25 MB. Either stack leaves ample room.

### 4.5 How each fits NW1, and the verdict

- **rustls** is a state machine that the I/O thread drives: S7-Q4 ran it from one wait. Its one blocking call is the verifier's, which §9 moves to the worker pool. Its timing can be virtual (EL24), and its session cache is Axiom's, keyed by network partition (NW8).
- **Network framework** runs threads of its own and calls back on a dispatch queue. Joining it to the I/O thread would take a hop per callback, and its timing can't be virtual. It costs 2.4–3.3× rustls's instructions per handshake and 1.6× per byte, and it didn't resume here. It verifies through the same `trustd`, so it brings no advantage in trust. Its advantage is being patched with macOS.
- **So NW6 stands as designed:** rustls with `aws-lc-rs`, verified by Security.

## 5. S7-Q3: DNS-SD in the sandbox

`dns`, in each formal run: five address queries and four HTTPS-record queries, all started at once on one shared connection (`kDNSServiceFlagsShareConnection`), whose socket the I/O thread waits on. The spike decodes the records' RDATA itself [S4].

| Run | Wall time for all nine | `kevent64` returns | CPU | Threads before, during, after |
|---|---|---|---|---|
| 1 | 23.1 ms | 6 | 0.43 ms | 1, 1, 1 |
| 2 | 19.7 ms | 4 | 0.37 ms | 1, 1, 1 |
| 3 | 17.2 ms | 6 | 0.35 ms | 1, 1, 1 |

| Query | First answer, ms (runs 1–3) | Answers |
|---|---|---|
| Addresses: www.apple.com, www.wikipedia.org, example.com, github.com, cloudflare.com | 1.4–5.7 (median 3.0) | IPv4 addresses for each, and a negative answer for IPv6 (`kDNSServiceErr_NoSuchRecord`). Some of these names have IPv6 addresses (cloudflare.com's HTTPS record lists them as hints). Inference: this Mac had no routable IPv6 on its network, and mDNSResponder answers that family negatively |
| HTTPS records: cloudflare.com, example.com | 16.4, 1.0, 10.9; 19.6, 0.2, 10.7 | `priority=1 target=. alpn=h3,h2`, with IPv4 and IPv6 hints |
| HTTPS record: crypto.cloudflare.com | 20.1, 0.8, 14.0 | `alpn=h2`, hints, and a 71-byte ECH configuration |
| HTTPS record: www.google.com | 1.3, 0.4, 0.5 | `alpn=h2,h3` |

- **No thread blocked:** the helper had one thread throughout, waiting in `kevent64`, and `DNSServiceProcessResult` ran only when DNS-SD's socket was readable.
- Fast answers come from mDNSResponder's cache.
- **`kDNSServiceFlagsMoreComing` is collective on a shared connection** (`dns_sd.h`): it reports more answers queued for any query, not for the one at hand. S7's first version settled each query on its own answers' flag, and waited out its 10 s deadline; queries now settle when a batch ends. Axiom's resolver must do the same.

## 6. S7-Q4: one wait

`one-wait`, in each formal run. One I/O thread waits in one `kevent64` call per turn, which also submits the turn's changes and arms the earliest deadline as one timer with leeway (EL18). It drives:
- a rustls fetch (`/cdn-cgi/trace` on www.cloudflare.com), kept alive;
- a 2 MB rustls download from speed.cloudflare.com, kept alive;
- an HTTP/3 GET over `quinn-proto` to cloudflare-quic.com;
- an HTTPS-record query for cloudflare.com;
- 50 Mach messages, 20 ms apart, from a second thread standing in for the browser, sent to a port in the I/O thread's port set and **received by the wait itself** (`EVFILT_MACHPORT` with `MACH_RCV_MSG`), then a user event (`EVFILT_USER`) from the same thread;
- a 100 ms periodic timer, merged into the one deadline.

| | Run 1 | Run 2 | Run 3 |
|---|---|---|---|
| rustls fetch (HTTP 200, 220 bytes): handshake's end, done | 49 ms, 58 ms | 49 ms, 60 ms | 50 ms, 59 ms |
| rustls download (2,000,000 bytes): done | 116 ms | 369 ms | 132 ms |
| HTTP/3 (HTTP 200, 125,959 bytes): connected, done; round trip; datagrams in, out | 42 ms, 72 ms; 11.2 ms; 121, 12 | 43 ms, 73 ms; 12.0 ms; 121, 12 | 42 ms, 68 ms; 8.9 ms; 120, 12 |
| HTTPS record | 0.37 ms | 0.37 ms | 0.28 ms |
| Mach messages received; latency, mean and maximum | 50 of 50; 0.05, 0.55 ms | 50 of 50; 0.05, 0.11 ms | 50 of 50; 0.61, 28.8 ms |
| Timer ticks; lateness, mean and maximum | 12; 4.2, 5.5 ms | 13; 5.6, 9.8 ms | 13; 4.3, 10.1 ms |
| Turns for all of it, over | 117, 1.21 s | 116, 1.34 s | 116, 1.38 s |
| Threads: before; while active | 1; 4 | 1; 4 | 1; 4 |
| **30 s idle:** turns, and what woke them | **2:** QUIC's idle timeout at 27.86 s, and the window's end | **2:** 27.73 s, and the end | **2:** 27.69 s, and the end |
| Idle wakeups; interrupt wakeups; CPU | 0; 3; 0.29 ms | 0; 3; 0.31 ms | 0; 3; 0.27 ms |

- **Everything progressed from the one wait.** Times are from the start, and include name resolution and verification (1.7–4.8 ms per evaluation here).
- **Idle, nothing woke the thread but deadlines.** The wakeup at about 27.8 s is QUIC's idle timeout: the connection's last activity was about 2.2 s before the window opened, and its idle timeout is 30 s. It is a real deadline, as EL1 allows. The kept-alive TLS connections need no timer. The counters add one interrupt wakeup to the turns in every window: two for 10 s (one turn) and three for 30 s and 60 s (two turns, informal for 10 s and 60 s). Inference: one wakeup at the window's boundary, not a periodic one.
- **Timer lateness** was 4.2–5.6 ms on average and 10.1 ms at most. The tick's deadline is armed with up to 6.25 ms of leeway (a sixteenth of the delay, at most 10 ms); inference: the rest is the kernel's coalescing and the turn's own work.
- **Run 3's 28.8 ms Mach latency** shows a turn delayed while the Mac was busy. Inference: the thread was descheduled, or busy with a burst of the download. Either way it is the cost one thread pays for every source, and why it must never block (§9).
- The threads while active are the I/O thread, the browser's stand-in, and two that Security's XPC connection to `trustd` adds; two remain when idle (inference, from `proc_pidinfo`'s count).

## 7. Build cost

Measured with `tools/measure-build.sh -p s7-net` on the dev profile, in an isolated target directory, on 2026-10-19 (`target/measure/20261019-070309/`):

| Threshold (S2) | Measured | Target |
|---|---|---|
| T1: incremental rebuild and link, touching `main.rs` | 0.52 s | ≤ 5 s |
| T2: clean build of `s7-net`'s tree: 60 compilations, 12 links | 17.7 s | ≤ 5 min (for the whole workspace) |
| T3: peak aggregate compiler and linker memory | 0.82 GiB (840.6 MiB) | ≤ 3 GiB |
| T4: peak memory of one `rustc` (rustls) | 0.44 GiB (446.5 MiB) | ≤ 1.5 GiB |

AWS-LC's C and assembly are compiled through `cc`; no CMake is needed. An informal release build of the spike's first scaffold, with the same dependencies, took 39.8 s, 21.8 s of it in `aws-lc-sys`'s build script (2026-10-19).

## 8. Dependencies found

Trees from `cargo tree -p s7-net` for this Mac's target; `unsafe` counts are lines using the keyword outside comments, in each crate's `src/`, as published to crates.io (observed 2026-10-19). Grades: none of rustls, `rustls-webpki`, `aws-lc-rs` or `quinn-proto` is graded in Google's or Chromium's audit files [G1, G2].
- **rustls 0.23.46** forbids `unsafe`. It brings `rustls-webpki` 0.103.15 (no `unsafe`; used here only to check the handshake's signatures), `rustls-pki-types` 1.15.1, `subtle` 2.6.1, `zeroize` 1.9.1 and `once_cell` 1.21.4. **Its default features include `prefer-post-quantum`,** which orders X25519MLKEM768 first [R1]; the workspace had disabled default features to drop logging, and lost it (§4.1).
- **`aws-lc-rs` 1.18.2 and `aws-lc-sys` 0.46.0:** AWS-LC's C and assembly, compiled by the build script through `cc` (build dependencies `cc`, `cmake`, `jobserver`, `shlex`, `dunce`, `fs_extra`, `pkg-config`, `find-msvc-tools`). `aws-lc-rs` itself has 367 lines with `unsafe`, its FFI layer.
- **`quinn-proto` 0.11.19:** four uses of `unsafe`, all `VarInt::from_u64_unchecked` on values already bounded. **It depends on `tracing` 0.1.44 unconditionally** (Google grades 0.1.39 `ub-risk-4` and the 0.1.40→0.1.41 delta `ub-risk-2`), and on `rand` 0.10.3 with `chacha20` 0.10.2 and `getrandom` 0.4.3, `rand_pcg`, `slab`, `lru-slab`, `tinyvec` (`ub-risk-0` through 1.9.0), `rustc-hash` and `thiserror`.
- **`ring` appears in `Cargo.lock`** only for other targets: nothing builds it for this Mac (`cargo tree -i ring` prints nothing; no `ring-*` build directory). The lock file records every target's dependencies.
- **`objc2-security` 0.3.2** marks `SecPolicyCreateSSL`'s result as non-null, from the header, and panics when it is NULL, which happens in a sandbox that hides the main bundle (§3.1). S7 declares that one function itself, with a nullable result.
- **`block2` and `dispatch2`** were already in the tree through objc2; `dispatch2` gains its `block2` feature, for dispatch data. `bytes` was already in the tree through `quinn-proto`.
- **S7's own `unsafe`:** 75 blocks, 4 `unsafe fn`s (DNS-SD's callbacks and their helpers) and 2 `unsafe impl`s (a Network framework connection pointer's `Send` and `Sync`), all in `src/platform/` (`tools/check-unsafe-policy.sh`). Network framework's comparison accounts for 29 blocks; the rustls path needs the other 46.

## 9. Implications for Axiom

1. **ADR-0013's condition is met.** The owner's Gate B decision: "Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing." All three pass (§3, §5, §6), so ADR-0013 is accepted, as of 2026-10-19, with this report as its evidence.
2. **NW6 is settled as designed:** rustls on `aws-lc-rs`, with its `prefer-post-quantum` default, verified by Security's trust evaluation (§4.5).
3. **Verification leaves the I/O thread** (EL1; proposed in NW6, for the owner at Gate C; **confirmed by the owner** on 2026-10-20). `SecTrustEvaluateWithError` waits on `trustd` for 8.4 ms at the median and up to 1.5 s in the informal runs. Waiting there is a second place to block, which EL1 forbids, and while it lasts every other connection, message and timer waits too. rustls's verifier is synchronous, so the inference is:
   - while a connection is still in its handshake, the I/O thread hands the rustls connection (which is `Send`) and the records it has read to the worker pool (EL8), which processes them, and so verifies;
   - the pool returns it with an `EVFILT_USER` wakeup;
   - connections past their handshakes are processed on the I/O thread.

   The sockets, timers and wait stay the I/O thread's, as NW1 has them. The detail is designed at the network milestone, and tested under the deterministic scheduler (EL24).
4. **NW17's profile is written down:** `profiles/network.sb`'s eight rules (§3.1) are the network process's profile at L1, with the removal tests and the reporting runs as its regression check. A feature that needs more (proxies, client certificates, path monitoring) adds its rule with its own evidence.
5. **The process model's network column** (§4.1) lists only the data directory among files. The profile adds the metadata of two paths: `/var`, a system symlink, and the executable's own directory, in Axiom's bundle. Neither is a user file, a service or a capability. **For the owner at Gate C:** note them in the model, with S5's same refinement for renderers. The model is binding, so it is not edited here. **Recorded** in the process model's change record, with the owner's approval at Gate C (2026-10-20).
6. **`platform` must treat `SecPolicyCreateSSL` as fallible:** its header promises a policy, but it returns NULL when the main bundle can't be created. objc2-security's binding panics on that NULL. S7 declares the function with a nullable result and fails closed, and Axiom's `platform` does the same.
7. **NW7 holds,** with one obligation from §5: on a shared DNS-SD connection, `kDNSServiceFlagsMoreComing` speaks for all queries, so the resolver settles queries when a batch ends.
8. **NW1, EL2 and EL18 hold in a sandbox:** one `kevent64` per turn received Mach messages in the wait itself, merged every deadline into one timer with leeway, and woke for nothing else when idle (§6).
9. **NW5 holds:** `quinn-proto` ran from the same wait, with an HTTP/3 GET over Axiom-style framing and QPACK, against Cloudflare.
10. **For the register** (§8): rustls's `prefer-post-quantum` is required; `quinn-proto` brings `tracing` unconditionally; and AWS-LC's jitter entropy costs 30–40 ms of CPU once per process (§4.3), a decision for the network milestone.

## 10. Threats to validity

1. **One machine, one OS build.** macOS 27.2 (26B5112a) on an M1. Security's, DNS-SD's and Network framework's sandbox needs could change with any update; the removal tests in `tools/s7-measure.sh` recheck them.
2. **Public hosts and a home network.** The servers, their certificates and the path to them are outside the spike's control. badssl.com's servers were slow from here, and some connections to them timed out (§3.2). Latencies are indicative; the CPU figures are not affected.
3. **A busy Mac.** The formal run was made with the Mac in normal use, not idle (load averages 3.3, 2.6 and 2.3 at the start; 5.7 during the supplementary run). CPU time varies with it; instruction counts much less (§4.2).
4. **trustd caches.** The first evaluations of a chain in a session can be slower than later ones; trustd also fetches missing intermediates and revocation data over the network outside the helper. The evaluation times are those this setup saw.
5. **Network framework was driven through its C API,** with one configuration. Its resumption may need a configuration S7 didn't find (§4.1). Its CPU figures include its own threads, as S7-Q2 asks.
6. **The reporting runs allow everything,** so frameworks may take paths there that they don't take under the real profile, such as asking `cfprefsd` instead of reading preference files. The list of attempted operations is what the helper does when nothing is denied; §3.3 also lists what was denied under the real profile.
7. **The helper is a stand-in** without an engine: its footprints show what each stack adds, not a real network process's total.

## 11. Reproducing these results

```sh
cargo test -p s7-net        # 8 tests, offline
tools/s7-measure.sh         # everything; target/s7-results/<timestamp>/; needs the network
```

| Run | Directory |
|---|---|
| Formal run (commit `4840d7d`, clean tree) | `target/s7-results/20261019-071313/`: `summary.txt`, `environment.txt`, the `q1-` to `q4-` files named above, `q1-removal-*.txt`, `q1-report-*.txt`, `q1-attempted.txt`, `profiles/`, `sandbox-denials.txt`, and an empty `crash-reports/`: no helper crashed |
| Supplementary `startup` runs (commit `13daa44`) | `target/s7-results/20261019-073956-startup/` |
| Build cost | `target/measure/20261019-070309/` |

## Sources

All checked 2026-10-19, except [G1] and [G2], fetched for S5 on 2026-10-18.
- [P1] rustls, `rustls-platform-verifier`, `rustls-platform-verifier/src/verification/apple.rs`, at `a8d640d88f2a370433f023eed68acf4a5c47605a` (2026-07-29) — https://github.com/rustls/rustls-platform-verifier/blob/a8d640d88f2a370433f023eed68acf4a5c47605a/rustls-platform-verifier/src/verification/apple.rs
- [W1] WebKit, `Source/WebKit/NetworkProcess/mac/com.apple.WebKit.NetworkProcess.sb.in`, at `09db792d4a1ce73e416eca5cfc168a6d5aa41028` (2026-10-17) — https://github.com/WebKit/WebKit/blob/09db792d4a1ce73e416eca5cfc168a6d5aa41028/Source/WebKit/NetworkProcess/mac/com.apple.WebKit.NetworkProcess.sb.in
- [C1] Chromium, `sandbox/policy/mac/network.sb`, at `b0741813d5fd3ed6d1643f5035b766369962cce9` (2026-09-08) — https://github.com/chromium/chromium/blob/b0741813d5fd3ed6d1643f5035b766369962cce9/sandbox/policy/mac/network.sb
- [R1] rustls 0.23.46, `Cargo.toml` (default features) and `src/crypto/aws_lc_rs/mod.rs` (`DEFAULT_KX_GROUPS`), read in the crate as published to crates.io — https://github.com/rustls/rustls
- [B1] badssl.com, the test hosts' index page, titled "badssl.com" (no date on the page) — https://badssl.com/
- [S1] IETF, RFC 9114, *HTTP/3* (2022-06) — https://www.rfc-editor.org/rfc/rfc9114
- [S2] IETF, RFC 9204, *QPACK: Field Compression for HTTP/3* (2022-06), including Appendix A, the static table — https://www.rfc-editor.org/rfc/rfc9204
- [S3] IETF, RFC 9000, *QUIC* (2021-05), §16, variable-length integers — https://www.rfc-editor.org/rfc/rfc9000
- [S4] IETF, RFC 9460, *Service Binding and Parameter Specification via the DNS (SVCB and HTTPS Resource Records)* (2023-11) — https://www.rfc-editor.org/rfc/rfc9460
- [G1] Google, `rust-crate-audits`, `audits.toml`, at `508eb72` (2026-09-17) — https://github.com/google/rust-crate-audits/blob/main/audits.toml
- [G2] Chromium, `third_party/rust/chromium_crates_io/supply-chain/audits.toml`, at `12e6ddd` (2025-03-22) — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml
- Observed on this Mac: the SDK 27 headers `dns_sd.h`, `sys/event.h`, `mach/message.h`, `Security/SecPolicy.h`, `SecTrust.h`, `SecBase.h`, `SecProtocolOptions.h`, `Network/connection.h`, `parameters.h`, `tls_options.h`; `/usr/bin/security error`; the unified log.
- This repository: `spikes/s7/`, `tools/s7-measure.sh`; the [Step 5 tracker](step-5-subsystems.md) §10.3; [S4 results](s4-results.md) for the sandbox method and tooling; [S5 results](s5-results.md) §7 for the executable directory's metadata.
