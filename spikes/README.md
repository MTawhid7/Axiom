# Spikes (Phase Zero, Step 2b)

This directory holds validation experiments for [ADR-0002](../docs/adr/0002-implementation-language.md). The code is **throwaway and not production**: it exists to answer the questions in the [Step 2b plan](../docs/research/step-2b-validation-spikes.md). It will be archived or deleted after CP9; its results live in `docs/research/`.

| Spike | Question | Crates |
|---|---|---|
| S1 | Can a GC heap shared by JS and the DOM keep `unsafe` inside the GC boundary, with acceptable ceremony and cost? Pattern A (gc-arena style) against Pattern B (Nova style) | `s1/*` |
| S2 | objc2 AppKit + Metal + CoreText without Xcode: ergonomics, `unsafe` surface, build cost | `s2/*` (from CP8) |

## Rules

- **The unsafe-code policy applies to spikes too** (evaluation §15.3; `tools/check-unsafe-policy.sh`).
  - Every crate root declares `#![forbid(unsafe_code)]`, except crates listed in `tools/unsafe-policy.txt`.
  - Listed crates declare `#![deny(unsafe_code)]`, and allow `unsafe` only in their listed modules.
- **Verification before every commit:**

  ```sh
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  tools/check-unsafe-policy.sh
  ```

- **Build measurements:** `tools/measure-build.sh --help`.
