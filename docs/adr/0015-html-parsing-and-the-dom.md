# ADR-0015: HTML Parsing and the DOM

| | |
|---|---|
| **Status** | **Proposed** (Step 5, CP18), for the owner at Gate C. HD11 depends on the amendment to [ADR-0008](0008-script-agents-and-heap.md) proposed with it |
| **Date** | 2026-10-19 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [HTML and the DOM](../architecture/html-and-dom.md) (HD1–HD33); [ADR-0003](0003-gc-handle-and-rooting-pattern.md); [ADR-0007](0007-frame-tree-pipeline-and-input.md); [ADR-0008](0008-script-agents-and-heap.md) (the HD11 amendment); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [ADR-0011](0011-text-fonts-and-unicode.md) (TX3); [ADR-0013](0013-networking-storage-and-workers.md) (NW12, NW14); decision-map item P5; [OQ-22](../open-questions.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-19 | Proposed at Step 5, CP18, with the amendment to ADR-0008 (HD11) and XSLT ([OQ-22](../open-questions.md)) for the owner |

## Question

How is HTML parsed, from bytes to tree, and how is the DOM represented and mutated in Pattern B's heap: nodes, attributes, text, events, mutation observers, ranges and selection, Shadow DOM, custom elements, forms and editing? How are XML documents parsed?

## Requirements and constraints

- **DP1 (a):** parsing HTML and CSS, and the DOM, are Axiom's own.
- **SH1, SH8–SH11:** DOM objects are JavaScript objects in one heap; hot links as `u32` columns; generated setters; a mark–compact collector.
- **PL2, PL11, PL12:** DOM arenas without interior mutability, lendable; one string representation; node identity stable for a node's life.
- **IV3, IV4, IV10, IV14:** selector-driven invalidation, box rebuilds where needed, parsing that yields, incremental equals a rebuild.
- **TX3, NW12, NW14, FT11:** Axiom's own decoders; ORB in the network process; bodies streamed with backpressure.
- **Rule 5, SC2:** every parser of web content is safe Rust in the sandboxed renderer.

## Alternatives considered

- **html5ever** as the parser.
- **A parser thread** building tree operations, as Firefox does.
- **A compacting node arena,** with identities remapped when nodes move.
- **Nodes as individual heap objects** with pointer fields.
- **libxml2 or expat** for XML.
- **The proposal:** parsers of Axiom's own on the event loop's thread, a node arena of columns whose slots never move, and one mutation path.

## Historical context and modern developments

- **Blink parses on the main thread** in budgeted chunks (10 ms for the first yields), with a preload scanner on a background thread. **Firefox** builds the tree on a parser thread and speculates past scripts, stopping after 100 failures. **WebKit** parses on the main thread. **Servo** uses html5ever, which keeps strings in UTF-8; **Ladybird** has its own parser.
- **XML:** Blink, WebKit and Ladybird use libxml2; Firefox runs expat inside an RLBox sandbox in its content process.
- **Servo's DOM** keeps every field in a cell: what a shared, collected DOM costs in Rust without Pattern B's borrow.

Sourced in [HTML and the DOM](../architecture/html-and-dom.md) §3.

## Analysis

| | Proposal | Alternatives |
|---|---|---|
| Strings | Decoded straight into PL11's 8- and 16-bit forms | html5ever's UTF-8 would convert every text node and attribute |
| Responsiveness | Budgeted parse tasks; a sink interface keeps a parser thread possible | A parser thread needs speculation that can fail, and synchronous answers from the DOM |
| Identity | Slots never move, so side tables and the browser hold stable identities | A compacting node arena invalidates identities held outside the heap (PL2, AX2) |
| Safety | Safe Rust for HTML and XML | libxml2 and expat are C on web content |

## Evidence and sources

[HTML and the DOM](../architecture/html-and-dom.md) §2–§4 and its sources: the HTML and DOM standards (both 2026-10-16), the Encoding Standard, the Web Editing drafts, html5lib-tests, and Blink, Firefox, WebKit, Servo and Ladybird at their latest commits. [S1 results](../research/s1-results.md) (bytes per node) and [S3 results](../research/s3-results.md) §6 and §9 (link columns; rooting one array).

## Decision

**Recommended: accept HD1–HD33.** In summary:
1. **Axiom's own HTML parser** (HD1–HD8): the standard's tokenizer and tree builder; sniffing as HTML §13.2.3 specifies, decoding straight into PL11's strings; budgeted tasks on the event loop's thread, with a preload scanner on the helper pool; fragments and the `setHTML` family through one tree builder that sanitizes while parsing.
2. **Axiom's own XML parser** (HD9), non-validating, fetching no external entity. XPath staged. **XSLT not planned** until a use requires it ([OQ-22](../open-questions.md)).
3. **One node arena of `u32` columns, 36 bytes a node,** with per-kind arenas and rare data (HD10, HD12, HD13). **Node slots never move** (HD11), which needs **the amendment to ADR-0008** proposed in its decision history: SH10's compaction skips the node arena.
4. **One mutation path** with a fixed, ordered list of hooks (HD15); live collections cached against a tree version (HD16).
5. **Events** (HD17–HD21): listeners in a side table; the event path and each listener snapshot rooted as one heap array; trust and activation only from the browser.
6. **The rest of the DOM** (HD22–HD29): mutation observers, live ranges and selection, Shadow DOM, custom elements with scoped registries, forms submitted through the browser, editing through Input Events and the IME mirror, the top layer. `execCommand` is approximate.
7. **The JavaScript facet** is an interface that [the bindings](../architecture/bindings.md) close (HD30).

## Rationale

- DP1 (a) makes the parsers and the DOM core, and PL11 rules out a UTF-8 parser.
- Stable node identity follows from PL2, PL12 and AX2; the amendment states its cost, fragmentation, and its fallback.
- One mutation path makes every subsystem's reaction to the DOM visible in one place, which IV14's oracle then checks.
- Safe Rust parsers keep the Rule of 2 without a second sandbox inside the renderer.

## Assumptions

- Blink's field-measured parse budgets are a sound starting point for Axiom's (Step 6 sets Axiom's own).
- The node arena stays dense enough after churn, with slots reused last in, first out (Step 6 measures).
- A parse costs well under a frame per chunk at budgeted sizes (hypothesis, HD5; Step 6).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The node arena fragments on long-lived, churning pages | Medium | Medium | Measured in Step 6; fallback: compact one document's nodes while remapping its side tables (HD11) |
| Parsing on the event loop's thread delays input on huge documents | Low | Medium | Budgeted tasks with input first (EL16); the sink interface keeps a parser thread possible (HD5) |
| `execCommand`'s behaviour diverges from sites' expectations | Medium | Low | Follow the engines where they agree; marked approximate (HD28) |
| The owner rejects the ADR-0008 amendment | Low | Medium | HD11 would then need identities remapped at every compaction, with the browser told (alternative C) |

## Conditions for revisiting

- Step 6 shows parse tasks routinely exceeding their budget on the corpus: reconsider a parser thread behind HD5's sink.
- The node arena's density degrades on the corpus: adopt HD11's fallback.
- A site the owner uses needs XSLT: decide OQ-22.

## Consequences

- [Style](../architecture/style.md) reads the DOM's columns and atoms, and receives HD15's invalidation hook (ST8, ST19).
- [Layout and paint](../architecture/layout-and-paint.md) build boxes from the flat tree and record node identities (LY3).
- [The JavaScript engine](../architecture/javascript-engine.md) and [the bindings](../architecture/bindings.md) treat a node's handle as its platform object (JE11, WB4).
