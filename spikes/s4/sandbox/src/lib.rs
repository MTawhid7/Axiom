//! # Spike S4 (Step 3): sandboxed helper processes on macOS
//!
//! Validates option O1 of the macOS confinement note
//! (`docs/research/macos-process-confinement.md` §6–§7). A broker, standing in
//! for Axiom's browser process, launches helper executables with
//! `posix_spawn`. Each helper applies its own deny-by-default Seatbelt profile
//! at the top of `main`, then reports what it can and can't do.
//!
//! Questions (the note's §7):
//! - **S4-Q1:** can an ad-hoc-signed, hardened-runtime helper built against
//!   SDK 27.0 enter a custom profile on macOS 27.2 without being killed, via
//!   `sandbox_init_with_parameters` and via `sandbox_compile_string` plus
//!   `sandbox_apply`?
//! - **S4-Q2:** inside the profile, are files, the network, process launching
//!   and the named Mach services denied, while allocation, threads, inherited
//!   IPC channels and logging keep working?
//!
//! Layout:
//! - [`platform`]: every foreign call (libsandbox, Mach, `posix_spawn`,
//!   libbsm). The only module where `unsafe` is allowed.
//! - [`profile`]: the SBPL profiles and their parameters.
//! - [`probe`]: what a helper tries, and how outcomes are encoded and checked.
//! - [`scenario`]: the broker's side: launch a helper with its channels, run a
//!   scenario, collect a report.
#![deny(unsafe_code)]

pub mod platform;
pub mod probe;
pub mod profile;
pub mod scenario;
