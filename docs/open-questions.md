# Open Questions and Risks

| | |
|---|---|
| **Status** | Living document |
| **Last updated** | 2026-10-16 |

Each question records why it matters, what evidence or input would resolve it, and when it must be decided.

| ID | Question | Why it matters | Evidence / input needed | Decide by | Status |
|---|---|---|---|---|---|
| OQ-1 | What is the time budget (hours per week) and horizon? | Sets how realistic the roadmap is, given the own-JS-engine and app-web scope | Owner input | Step 6 (roadmap) | **Resolved 2026-10-16:** no calendar deadline and no fixed hourly budget. Work is paced by bite-sized, self-contained checkpoints |
| OQ-2 | What is the target-site corpus? (Concrete URLs per tier, for example documents, content sites, interactive sites, apps) | Success criteria and milestone acceptance depend on it | Owner's list of sites they actually use | Step 6; ideally earlier | Open |
| OQ-3 | WebAssembly: write our own engine or embed one (for example Wasmtime with Cranelift)? | App-web reachability (Figma-class sites); size of the attack surface; schedule | JS engine design (Step 4–5); evaluation of embedding options | Before any Wasm milestone | Open |
| OQ-4 | JIT policy: if and when to add a baseline JIT, and when to disable JIT (sensitive sites)? | The JIT is the highest-risk attack surface; it determines performance on app-web sites | Isolation and sandbox design (Step 3); interpreter performance data | Step 3 (policy); much later (implementation) | Policy drafted in ADR-0001 |
| OQ-5 | Per-agent heaps: how are workers, `SharedArrayBuffer` and cross-agent messaging represented in the GC design? | Affects GC architecture and the process model | S1 results; Step 4 design | Step 4 | Open |
| OQ-6 | Which languages is the owner fluent in? | Learning cost is a factor in ADR-0002 | Owner input | Before ADR-0002 is accepted | **Resolved 2026-10-16:** the owner is fluent in Rust. Code must not be simplified for learning reasons |
| OQ-7 | When should full Xcode be installed (Instruments, Metal tooling)? | Serious profiling; Metal debugging | Owner preference | Before the first performance-critical milestone | Open |
| OQ-8 | Version control: initialize a git repository for `docs/` now? | ADR history and diffs; continuity across sessions | Owner authorization | Now (recommended) | **Resolved 2026-10-16:** repository initialized; baseline `a223fe2` on `main`. Step 2b work is on branch `step-2b` |
| OQ-9 | Is the owner the only human developer? | Roadmap, review process, workload | Owner input | Step 6 | Assumed yes |
| OQ-10 | Which GC handle/rooting pattern: gc-arena-style, Nova-style scoped handles, or an unsafe core with a precise shadow stack? | Decides whether `unsafe` stays contained (the key ADR-0002 hypothesis) | Spike S1 | Before ADR-0002 is accepted | **In progress:** S1 authorized 2026-10-16, comparing Pattern A (gc-arena style) with Pattern B (Nova style) |
| OQ-11 | Platform shell: pure Rust (objc2) or a thin Swift layer? | Ergonomics against the cost of two toolchains | S2; Step 4 | Step 4 | Leaning pure Rust (evaluation §13) |
| OQ-12 | What are the build-time and memory thresholds on the 8 GB M1? | Revisit condition 3 for ADR-0002 | `tools/measure-build.sh` (from CP1); S1 and S2 measurements | Thresholds set in CP7 and CP8 | **In progress** |
