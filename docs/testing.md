# The Testing Strategy

| | |
|---|---|
| **Status** | **Binding** since [ADR-0018](adr/0018-corpus-standards-and-testing.md) was accepted (2026-10-20, Step 6's Gate B). Written at Step 6, CP5. Spike S9 (§9) authorized at Gate B. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](review/README.md) §5): TS2, TS3 and TS12 for RV1-01 (CP1) |
| **Question** | How is Axiom tested? Through which harnesses, oracles and fuzzers; how do tests stay deterministic; how are the security checks V1–V9 built; how are performance regressions caught; which routine keeps a locally built browser patched; where do tests run? |
| **Answers** | Step 6 question Q3 ([tracker](research/step-6-standards-budgets-roadmap.md) §3); decision-map item X2; the tracker's handed-over questions 4–12 (§2.2), with CP6 for the limits and benchmarks' thresholds |
| **Feeds** | Budgets (CP6), which tests enforce; the roadmap (CP8), which places each harness and check; Milestone 1 (CP9); the L3 gate ([threat model](security/threat-model.md) §8) |
| **Builds on** | The [standards baseline](standards.md)'s suites and expectations (SB7–SB12); the [embedder API](architecture/embedder-api.md)'s headless embedder (EM9) and its open question 3; [event loop](architecture/event-loop-and-threading.md) EL24; [invalidation](architecture/invalidation.md) IV14; [style](architecture/style.md) ST22 and ST36; [text](architecture/text-and-fonts.md) TX21; [layout](architecture/layout-and-paint.md) LY32; [HTML](architecture/html-and-dom.md) HD33; [JavaScript](architecture/javascript-engine.md) JE32; [images](architecture/images-and-media.md) IM10; the [dependency policy](technology/dependency-policy.md)'s DP15, DP16, DP20 and DP22; spikes S1, S3–S7 |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Rules are numbered TS1–TS18. Precedent and sources are dated (§2); the design is inference unless marked otherwise.

## Summary

- **Eight layers of tests** (TS1): unit tests; the conformance suites; differential oracles; fuzzing; checks of the corpus pages; performance benchmarks with budgets; the security checks V1–V9; and the checks that depend on macOS, repeated after its updates.
- **WPT runs on the headless embedder through wptrunner's `axiom` product** (TS2, TS3). **Without script (M1, M2),** reference tests run through a script-free executor, one process per test, as wptrunner's process-per-test Servo executor runs Servo. **From M3,** a WebDriver server that exists only in test builds runs every kind of test, as Ladybird's product does; `testdriver.js` maps onto it, and WebDriver BiDi comes later.
- **Axiom's own runners** cover html5lib-tests' tokenizer tests, test262 (with its `$262` host object), and Unicode's conformance files (TS4–TS6).
- **Oracles** (TS7):
  - the test-only references the designs name: html5ever, xml5ever, Taffy, `regress`, `temporal_rs` and `num-bigint`;
  - the self-oracles: incremental equals rebuilt (IV14), cache-free style (ST22), chunked parsing equals whole (HD33), round trips.
- **Determinism** (TS8): one virtual clock (EL24); injected fonts, settings and time; local test servers; no live network in automated tests, except the corpus pages that the corpus's rules let automation load.
- **Fuzzing** (TS9–TS11): every parser and IPC decoder gets a target. cargo-fuzz with libFuzzer and AddressSanitizer is expected to work on this Mac's target, which **spike S9, authorized at Gate B,** checks before any milestone depends on it. Crashes become regression tests. ThreadSanitizer covers the threads, and Miri covers every crate with `unsafe`.
- **The checks V1–V9** are planned as tests, each with its construction and its trigger (TS14).
- **The routine** (TS15, TS16):
  - weekly, an advisory check;
  - on any advisory against a crate that parses untrusted input, a rebuild before the browser is next used;
  - after every macOS update, the checks that depend on the system run again.
- **Where tests run** (TS17): this Mac is authoritative. GitHub's hosted runners could add a second line for correctness only, never for performance or sandbox results. Whether to use them is the owner's choice at Milestone 1.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **The standards baseline:** the suites, expectations kept test by test, targets per milestone (SB7–SB12) | TS2–TS6 run them |
| **EM9:** a headless embedder with offscreen presentation, synthetic input, virtual time and logged decision answers, which "runs WPT, reference tests, crash tests and fuzzing harnesses (Step 6)"; its open question 3: "WPT's harness integration (`testharness.js`, `testdriver`)" | TS2, TS3 |
| **EL24:** one deterministic scheduler with a virtual clock drives every event loop in tests, and can record and replay | TS8 |
| **IV14, ST22, HD33:** incremental equals from scratch; the cache-free style path; chunked parsing equals whole | TS7 |
| **TX21, LY32, ST36:** pixel tests use web fonts; the environment is injected | TS3, TS8 |
| **JE32, HD33, IM10, and every subsystem's oracle list:** the differential references and fuzzing that Step 6 plans | TS7, TS9 |
| **ADR-0002 rule 5:** no `unsafe` in parsers, network policy, IPC decoding, or image and font wrappers; **evaluation §15.3:** `unsafe` only in listed modules | TS11: Miri where `unsafe` is; fuzzing everywhere input is parsed |
| **Threat model §8:** the checks V1–V9 that gate L3; **§9:** fixes reach the browser only through rebuilds | TS14, TS15 |
| **DP15, DP16, DP20, DP22:** `cargo-vet` audits, `cargo-deny`'s four checks, rebuilds after advisories with "Step 6 sets the routine and its cadence", and pre-commit checks once the owner authorizes the tools | TS15 |
| **The corpus's rules CO3, CO6, CO12:** automation only where the terms permit; by hand elsewhere; copies only where permitted | TS12 |

## 2. Sources and precedent

- **A script-free precedent.** wptrunner keeps a process-per-test executor for Servo, `ServoLegacyRefTestExecutor` in `executorservolegacy.py`: it launches the browser for each screenshot with `--exit`, `--output=PATH` and `--window-size`, resolves WPT's host names through a hosts file it names in `HOST_FILE`, and reads the PNG the browser writes. A non-zero exit or a missing file is reported as a crash. Its WebDriver reference-test executor, by contrast, runs script in the page (`executorwebdriver.py`, lines 1410 and 1451) (observed at WPT `0f26972422`, 2026-10-21) [T7].
- **WPT and new engines.** wptrunner's products include Ladybird and Servo besides the shipping engines. Ladybird's product launches Ladybird's own WebDriver server, and runs testharness, reference and crash tests through wptrunner's WebDriver executors [T1]. `testdriver.js` "provides a means to automate tests that cannot be written purely using web platform APIs". Its commands mirror WebDriver's (`click` "Matches the behaviour of the [Element Click] WebDriver command"), and `test_driver.bidi` "provides access to the WebDriver BiDi protocol" [T2].
- **Expectations.** wptrunner's metadata files record each test's expected result, with known intermittent results and conditions, and `wpt update-expectations` rewrites them from a run [T3] (the [standards baseline](standards.md) SB8).
- **html5lib-tests** moved its tree-construction tests into WPT (`html/syntax/parsing`) on 2026-06-26, and keeps its tokenizer tests [T4]. HD33's oracle stays the same; only its home changed.
- **test262** [T5]:
  - each test runs in a fresh realm, twice (strict and not) unless its flags say otherwise;
  - "Any uncaught exception must be interpreted as test failure";
  - negative tests name a phase (`parse`, `resolution`, `runtime`) and an error type;
  - a host supplies `$262` with `createRealm`, `evalScript`, `detachArrayBuffer`, `gc`, `global`, `agent` (concurrent agents) and `print`;
  - asynchronous tests report through `print`.
- **Fuzzing tools:**
  - cargo-fuzz drives libFuzzer, which "needs LLVM sanitizer support". It works on "x86-64 Linux, x86-64 macOS and Apple-Silicon (aarch64) macOS", and "requires the nightly compiler" [F1]. Its latest release is 0.13.2, of 2026-06-09.
  - libFuzzer's authors "have stopped active work on it", though it "is still fully supported in that important bugs will get fixed"; they moved to Centipede [F2].
  - Rust's sanitizers on `aarch64-apple-darwin`: AddressSanitizer and ThreadSanitizer, but not MemorySanitizer, LeakSanitizer or HWAddressSanitizer. All need nightly, and the instrumented standard library is "strongly recommended" [F3].
- **Hosted runners.** GitHub's standard runners "are free and unlimited on public repositories". The arm64 macOS runners are M1 machines with 3 CPUs and 7 GB, and the newest image is macOS 26, while this Mac runs 27.2 [G1].
- **This Mac** (observed 2026-10-20, `~/.cargo/bin` and `rustup`):
  - **installed:** the nightly toolchain with `rust-src` and Miri; `cargo-deny` 0.20.2, not configured for Axiom; `samply`;
  - **not installed:** `cargo-fuzz`, `cargo-vet`.

## 3. The test layers

| # | Rule |
|---|---|
| TS1 | **Eight layers, each with a home and a trigger:**<br>1. **Unit and integration tests** in every crate, with `cargo test`: on every commit, in the five checks.<br>2. **The conformance suites** (SB7): WPT directories and the other suites, at checkpoints that touch an area, and in full for the milestone's selection at each gate.<br>3. **Differential oracles** (TS7): in unit tests, and as fuzz targets.<br>4. **Fuzzing** (TS9): campaigns run locally, with crashes kept as tests.<br>5. **The corpus checks** (TS12): at each milestone's gate.<br>6. **Performance benchmarks** with the budgets as thresholds (TS13; CP6): at each gate, and before merging a change that touches a hot path.<br>7. **The security checks V1–V9** (TS14): built at the milestones the roadmap names, all passing at the L3 gate.<br>8. **The checks that depend on macOS** (TS16): after every macOS update.<br>A layer's results never stand in for another's: a fuzz campaign is not a WPT pass, and a WPT pass is not a corpus check |

## 4. The harnesses

| # | Rule |
|---|---|
| TS2 | **WPT runs through wptrunner's `axiom` product, with two executors, staged** (RV1-01):<br>• **Without script, at M1 and M2: a script-free reference-test executor,** modelled on `ServoLegacyRefTestExecutor` [T7]. wptrunner launches `axiom-headless` once per screenshot; it loads the test, waits natively for the load event and for pending images and fonts, writes a PNG from EM9's offscreen texture (TS3), and exits. WPT's host names resolve through a hosts file the test build alone accepts. No script runs in the page, so tests that need it stay outside the milestone's selection (SB11).<br>• **From M3, with script: a WebDriver server in the headless embedder,** which then runs testharness, reference and crash tests alike. At M3 the reference-test selection runs under both executors once, and differences are explained, before WebDriver replaces the script-free one.<br>**The WebDriver server:**<br>• The server implements the subset of WebDriver that wptrunner's WebDriver executors and `testdriver.js` use: sessions, navigation, execute script (including asynchronous), window rectangles, screenshots, actions, element click and send keys, cookies, and permissions through WebDriver's extension commands.<br>• wptrunner gets an `axiom` product modelled on Ladybird's [T1], with its WebDriver testharness, reference and crash test executors.<br>• WebDriver BiDi follows when `test_driver.bidi`'s tests become part of a milestone's selection.<br>• **The server is test-only.** It is compiled into the headless test embedder and nothing else, listens only on the loopback interface, and is never present in the shell's build. That keeps a remote-control surface out of the browser the owner uses.<br>Settles the embedder API's open question 3 |
| TS3 | **Reference tests and screenshots** come from EM9's offscreen texture, the same pipelines without CoreAnimation (GR11). They are compared with WPT's references under the tests' fuzzy tolerances, at a fixed device scale and an sRGB output profile. Pixel tests use web fonts (TX21). **WPT's test fonts, Ahem among them, take the same Rust path from M1:** the headless embedder registers them so that they resolve by name, before web fonts arrive at M4 (RV1-01). Animations and carets run on virtual time (TS8) |
| TS4 | **html5lib-tests' tokenizer tests** run in a Rust harness over the JSON files at the pinned commit (SB2). The tree-construction tests run as WPT's `html/syntax/parsing` (§2) |
| TS5 | **test262** runs in the JavaScript engine's own Rust harness, as INTERPRETING.md specifies [T5]:<br>• a fresh realm per test;<br>• strict and non-strict runs as the flags say;<br>• negative tests checked by phase and type;<br>• asynchronous tests through `print`;<br>• `$262` implemented in full, `agent` included, since `Atomics` tests need concurrent agents.<br>Each test's frontmatter names the features it needs, from test262's `features.txt` [T6]. The harness uses that list to skip tests for features the engine doesn't yet have, and lists every skip, so none is silent. The harness runs tests in parallel worker processes, sized to this Mac's memory |
| TS6 | **Unicode's conformance files** (the [standards baseline](standards.md) §5.1) run against Axiom's own text code: line breaking under CSS's rules, bidi in layout, segmentation at the editing boundaries. ICU4X's own tests cover its data and algorithms, at the pinned version |

## 5. Oracles and determinism

| # | Rule |
|---|---|
| TS7 | **Oracles:**<br>• **Test-only differential references,** each in the dependency policy's `test` group, never linked into the browser: html5ever and xml5ever for the parsers (HD33); Taffy for flex and grid layout on generated trees; `regress` for `RegExp`, `temporal_rs` for Temporal and `num-bigint` for BigInt (JE32). Known divergences, such as a reference that lags a specification change, are listed with their reason.<br>• **Self-oracles:** an incremental update equals a rebuild (IV14); cached style equals the cache-free path (ST22); a chunked parse equals a whole one (HD33); serializing then parsing returns the same tree; the bytecode compiler matches a separate reference evaluator (JE32).<br>• **Reference engines** are context, never oracles. A difference from Chrome or Safari is investigated against the specification, which decides |
| TS8 | **Determinism:**<br>• **Time:** every test runs on EL24's virtual clock, including timers, animations, carets and the collector's pacing.<br>• **Randomness:** `Math.random` and every other source are seeded in test builds.<br>• **The environment is injected:** fonts, system colours and settings (ST36). Pixel tests use web fonts (TX21).<br>• **Network:** wptrunner's own server for WPT, and Rust test servers on the loopback interface for everything else. No automated test reaches the internet, except corpus checks of the pages the corpus lets automation load (CO3).<br>• **A failure is recorded with the scheduler's order,** so it can be replayed exactly (EL24) |

## 6. Fuzzing, sanitizers and Miri

| # | Rule |
|---|---|
| TS9 | **Every parser and every IPC decoder has a fuzz target:**<br>• **Markup and style:** the HTML tokenizer and tree builder; the XML parser; CSS syntax, selectors, values and media queries; layout, with generated documents within the limits CP6 sets.<br>• **Script:** the JavaScript parser, the bytecode compiler, `RegExp`, `JSON.parse`, Temporal's parsing, Web IDL's conversions.<br>• **Network and storage:** URLs and IDNA, HTTP/1.1 framing, HPACK and QPACK, cookies, MIME sniffing, the decompressors, the storage engine's records.<br>• **Images and fonts:** each decoder, and the web-font path.<br>• **IPC:** every message decoder of every privileged endpoint, generated from the IDL, with structure-aware inputs (V2).<br>The tool is cargo-fuzz with libFuzzer on nightly, under AddressSanitizer [F1], once spike S9 confirms it here (§9). Where a differential reference exists (TS7), its target compares the two |
| TS10 | **Inputs and campaigns:**<br>• **Seeds** come from the suites (WPT, html5lib-tests, test262), S5's web fonts, and the corpus only where copies are permitted (CO12).<br>• **Campaigns** run on this Mac on AC power, when it is otherwise idle, and record coverage for every target.<br>• **Minimized corpora** are kept with the targets, made of generated inputs and permitted seeds only.<br>• **Every crash** is minimized, fixed, and kept as a regression test in its crate's unit tests |
| TS11 | **Sanitizers and Miri:**<br>• **AddressSanitizer** for fuzz targets, and for a periodic run of the unit tests.<br>• **ThreadSanitizer** for the concurrent parts: the event loop's channels, IPC, the compositor's threads, the collector's helpers.<br>• **Miri** runs every crate that contains `unsafe`, as S3 did: `gc`, `jit`, `platform` and any listed module.<br>Both sanitizers exist for this target; MemorySanitizer and LeakSanitizer don't [F3], so uninitialized reads stay with Miri and leaks with the memory budgets (CP6) |

## 7. The corpus, performance and security

| # | Rule |
|---|---|
| TS12 | **The corpus checks** for a milestone's target pages (charter §8): each page renders and reads, links navigate, and there is no crash or hang.<br>• **Automated** only for the pages that the corpus lets automation load (CO3), through the headless embedder (its navigation API at M1 and M2, TS2's WebDriver server from M3), identified as CO4 requires.<br>• **By hand** for the others, with a checklist the owner follows, and with the browser's processes sampled as S8 does (CO6).<br>• **Measurable stand-ins:** no process exits; the frame deadlines hold; the main content has laid out with nonzero size; a set number of links navigate. The owner's reading is recorded in the milestone's report |
| TS13 | **Performance benchmarks:**<br>• kept from the spikes: S1's dispatch case, S3's root-stack and VM workloads, S6's compositor traces;<br>• added for the corpus: each target page's budgets (CP6).<br>They run under S1–S8's formal conditions: release builds, AC power, Low Power Mode off, the load average recorded, cool-downs, three runs, and medians with tails. A benchmark over its budget fails its gate. Results stay under `target/`, and summaries go in the milestone's report |
| TS14 | **The checks V1–V9** ([threat model](security/threat-model.md) §8), each built as a test:<br>• **V1, sandbox escape:** a suite run inside each process type, grown from S4's, S5's and S7's probes. It attempts the operations the type must not perform: reading user files, opening sockets, spawning, looking up Mach services outside its list. Every attempt must be denied. Repeated after each macOS update (TS16).<br>• **V2, IPC fuzzing:** TS9's decoder targets, run as one campaign with coverage per endpoint, ending with no open findings.<br>• **V3, lock enforcement:** a test-only renderer that lies about its origin, for cookies, storage, navigation commits and requests. The browser must terminate it each time. The lying build exists only in tests.<br>• **V4, cross-site leaks:** WPT's CORS, COOP and COEP, ORB and cookie directories at their targets. An instrumented test confirms that blocked cross-site responses carrying HTML, JSON or XML never reach the renderer's memory.<br>• **V5, the `unsafe` audit:** a written review of `gc`, `jit` and `platform` after their last change, with Miri clean.<br>• **V6, the dependency audit:** `cargo-deny` clean, and every crate audited under `cargo-vet` (DP15, DP16).<br>• **V7, the JIT policy:** sensitive sites run without the JIT, and the global switch works.<br>• **V8, the threat model:** a written review against the code as built, with every deviation recorded.<br>• **V9, out-of-process iframes:** every cross-site frame renders in a renderer locked to its site, and a renderer that asks for another site's frame or data is terminated.<br>The roadmap (CP8) builds each one at its milestone; all nine pass, with recorded evidence, at the L3 gate |

## 8. The routine and the platform

| # | Rule |
|---|---|
| TS15 | **Advisories and rebuilds** (DP20, threat model §9):<br>• **Weekly,** and before every gate: `cargo-deny`'s advisory check against a fresh copy of the RustSec database, and `cargo-vet`'s status.<br>• **Each new advisory against a crate in the lock is assessed.** If the crate handles untrusted input, the fix is taken and the browser rebuilt before it is next used for anything the flaw could affect.<br>• **Every dependency change** runs `cargo-deny`'s four checks and `cargo-vet` (DP16, DP15).<br>• **The toolchain moves deliberately,** as ADR-0002's pin requires, with the five checks.<br>The tools become pre-commit checks when the owner authorizes installing and configuring them (DP22). `cargo-deny` is already on this Mac, but `cargo-vet` isn't (§2) |
| TS16 | **macOS updates.** The build of macOS is recorded with every test run. When it changes, the checks that depend on the system run again before the browser is next used for anything sensitive:<br>• the sandbox probes (V1), and each profile's rules;<br>• CoreText, ImageIO, Security's trust evaluation, CoreAnimation's colour handling and Metal;<br>• the spikes' successors: S4's sandbox, S5's text, S6's compositing and S7's network checks.<br>A failure means the browser isn't used for sensitive work until it is understood |
| TS17 | **Where tests run:**<br>• **This Mac is authoritative** for every result that depends on the platform or the machine: performance, energy, sandboxing, the GPU, and macOS 27's frameworks.<br>• **GitHub's hosted runners** could add a second line for correctness only: the five checks, unit tests and WPT subsets, on macOS 26 with 3 CPUs and 7 GB [G1]. They would never be used for performance figures or sandbox results. A workflow would use no third-party actions beyond GitHub's own, pinned by commit, and no secrets.<br>Whether to use them is the owner's decision when Milestone 1 starts (CP9) |
| TS18 | **WPT within 8 GB.** WPT runs by directory: a milestone's selection, never the whole suite. wptrunner's parallelism is sized to memory, starting at two browsers. Each directory's run time and peak memory are recorded, so the selection's cost is known before a gate depends on it |

## 9. Spike S9

Designed here because trying the tools on this Mac is the only check (tracker §10). **Authorized at Gate B** (2026-10-20): "Spike S9 (Testing and Fuzzing Tooling): Authorized. Proceed with validating cargo-fuzz, AddressSanitizer/ThreadSanitizer, and wptrunner on this Mac."

| ID | Question | Passes if |
|---|---|---|
| S9-Q1 | Does cargo-fuzz build and run a fuzz target on this Mac, with libFuzzer, on nightly, under AddressSanitizer, and report coverage? | A target over a small parser written for the spike runs for 10 minutes, finds a planted bug, and a coverage report is produced |
| S9-Q2 | Does ThreadSanitizer run a threaded Rust test here? | A planted data race is reported, and a clean test passes |
| S9-Q3 | Does wptrunner run here, against a WebDriver browser, within memory? | One WPT directory runs against Chrome Canary through chromedriver, or Safari through `safaridriver` if the owner enables remote automation, with its run time and peak memory recorded |

**What it would need, for the owner to authorize:**
- `cargo-fuzz` 0.13.2, installed with `cargo install --root target/s9/tools`, so it stays under `target/`;
- the nightly toolchain's `llvm-tools-preview` component, for coverage. rustup installs it beside the toolchain, outside the repository, and `rustup component remove` takes it away;
- a checkout of WPT under `target/`, whose Python tools install their packages into a virtual environment there;
- chromedriver for the installed Canary, downloaded into `target/`.

Except the rustup component, everything stays under `target/`, which git ignores and which can be deleted. S9's results would settle TS9–TS11 and TS18 before Milestone 1 depends on them. Without S9, those rules rest on the sources above, and the first milestone that fuzzes tries the tools first.

**Results** (2026-10-20, [S9 results](research/s9-results.md)):
- **S9-Q1 passes:** cargo-fuzz found the planted bug in about a second, ran the checked decoder for 10 minutes with no finding, and reported coverage, under AddressSanitizer.
- **S9-Q2 fails with the installed nightly:** its ThreadSanitizer runtime crashes while starting on macOS 27.2, where Apple clang's works.
- **S9-Q3 passes:** wptrunner ran `dom/nodes` against Chrome Canary within memory, at wpt.fyi's pass rate.

**What follows, within these rules:**
- Milestone 1 pins a nightly whose ThreadSanitizer works (TS11; [Milestone 1](milestone-1.md) MA7, MA10).
- Campaigns run one or two at a time within 8 GB, since a fuzzer under AddressSanitizer held about 0.5 GB.
- Deep formats get structure-aware generation beside their seeds (TS10).
- **WPT keeps html5lib's tree-construction tests as `.dat` files** beside their script wrappers (observed), so an engine without script runs them in its own runner (TS4; Milestone 1's MA4).

## 10. Open questions

1. **Spike S9** (§9): authorized by the owner at Gate B (2026-10-20).
2. **Hosted runners** (TS17): for the owner when Milestone 1 starts.
3. **Seed corpora's home:** in the repository, if small and made only of generated or permitted inputs (TS10), or under `scratch/`. Settled when the first campaign runs.

## Sources

All read on 2026-10-20.

- [T1] web-platform-tests, `tools/wptrunner/wptrunner/browsers/` (including `ladybird.py`, `servo.py`) — https://github.com/web-platform-tests/wpt/tree/master/tools/wptrunner/wptrunner/browsers
- [T2] web-platform-tests, *testdriver.js Automation* — https://web-platform-tests.org/writing-tests/testdriver.html
- [T3] web-platform-tests, *Expectation Data* — https://web-platform-tests.org/tools/wptrunner/docs/expectation.html
- [T4] html5lib-tests, commit `224991ec10db` (2026-06-26), "Tree construction tests have moved to WPT" — https://github.com/html5lib/html5lib-tests/commit/224991ec10db
- [T5] test262, `INTERPRETING.md` — https://github.com/tc39/test262/blob/main/INTERPRETING.md
- [T6] test262, `CONTRIBUTING.md`, "Frontmatter": `features`, "See the features.txt file for a complete list of available values" — https://github.com/tc39/test262/blob/main/CONTRIBUTING.md
- [F1] rust-fuzz, *Setup* (cargo-fuzz) — https://rust-fuzz.github.io/book/cargo-fuzz/setup.html ; releases (0.13.2, 2026-06-09) — https://github.com/rust-fuzz/cargo-fuzz/releases
- [F2] LLVM, *libFuzzer*, "Status" (LLVM 24 development documentation) — https://llvm.org/docs/LibFuzzer.html
- [F3] *The Rust Unstable Book*, "sanitizer" — https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html
- [T7] web-platform-tests at `0f26972422`, `tools/wptrunner/wptrunner/executors/executorservolegacy.py` and `executorwebdriver.py`, read on 2026-10-21 in S9's checkout under `target/s9/wpt` (observed) — https://github.com/web-platform-tests/wpt/tree/0f26972422b15f587e2afbd38dbb6ea2397fded0/tools/wptrunner/wptrunner/executors
- [G1] GitHub, *GitHub-hosted runners* — https://docs.github.com/en/actions/reference/runners/github-hosted-runners
