//! Axiom's code generators, and the framework that runs and checks them.
//!
//! # Charter (ES15)
//!
//! 1. **Purpose.** Every generated interface has one source of truth, a
//!    plain-text input beside the crate it feeds (CR10): the IDL beside
//!    `protocol` (M1-CP3), the property table beside `css` and `style`, the
//!    static atoms beside `strings` (M1-CP4), and the lists of named character
//!    references, public suffixes and HSTS preloads. This crate turns each
//!    input into Rust under its crate's `src/generated/`, and the output is
//!    committed, so a change to a generator shows its effect in the diff, the
//!    build runs no generator, and no Axiom crate needs a build script (ES19,
//!    ADR-0022 D5).
//! 2. **Rules.** ES19, CR10; IP8 and PL11 as ADR-0022 worded them ("by the
//!    build's tooling, with its output committed"); DP12.
//! 3. **Place** (`tools/crate-map.toml`): outside the layers, in the `build`
//!    group; no shipping crate depends on it (CR12); panic class ordinary; no
//!    `unsafe`. Like the repository's other tools it reads and writes files
//!    and runs rustfmt (ES13's exceptions name it).
//! 4. **Interface.** `cargo run -p codegen` regenerates every output that
//!    differs; `-- --check` regenerates in memory and fails on any committed
//!    output that differs or is missing (`tools/check.sh`'s commit tier);
//!    `-- --list` names each generator with its inputs and outputs. A
//!    generator implements [`Generator`] and joins [`registry::generators`].
//! 5. **Invariants.**
//!    1. Generation is deterministic: the same inputs give the same bytes.
//!    2. Every output begins with [`header`], naming its generator and
//!       inputs, and is formatted by the pinned toolchain's rustfmt with the
//!       repository's `rustfmt.toml`.
//!    3. Outputs are written only under a crate's `src/generated/`, which
//!       ES1's size limit exempts, and hold no `unsafe` (IP8, rule 7).
//! 6. **Threads.** One; generation is sequential.
//! 7. **Tests.** `tests/framework.rs` runs the framework on a fixture
//!    generator in a directory under `target/`: writing, checking fresh,
//!    finding a stale or missing output, and refusing an output outside
//!    `src/generated/`.
//! 8. **Efficiency.** `--check` is in every commit's checks, so generators
//!    keep to the standard library and to their inputs' size.

#![forbid(unsafe_code)]

mod framework;
pub mod registry;

pub use framework::{Error, Generator, Output, Stale, check, header, list, write};
