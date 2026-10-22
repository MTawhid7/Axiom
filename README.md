# Axiom

A browser engine designed from first principles for personal use on macOS (Apple Silicon).

Phase Zero (research, requirements and architecture) is complete, and **Milestone 1**, static documents, is under way: the owner authorized it on 2026-10-23 ([its plan](docs/milestone-1-plan.md)). The engine's crates live in `crates/`, in the workspace at the root; Phase Zero's validation spikes stay in [`spikes/`](spikes/README.md), a workspace of their own.

- Start with [`docs/status.md`](docs/status.md) (current state and handoff log) and [`docs/README.md`](docs/README.md) (index).
- How the code is organized and written: the [crate architecture](docs/architecture/crates.md) and the [engineering standards](docs/technology/engineering-standards.md), whose ES11 is the `unsafe` policy, enforced by `tools/check-unsafe-policy.sh`.

## License

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([`LICENSE-MIT`](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
