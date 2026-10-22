//! Axiom's recorder: spans, events, metrics and logs in per-thread buffers.
//!
//! # Charter (ES15)
//!
//! 1. **Purpose.** Every process records what it does and what it costs
//!    through this crate: spans and events on one timeline, metric updates,
//!    logs, and the panic record that a crash report needs. Records are small,
//!    typed and named by IDs, and each thread writes its own ring, with no
//!    lock, no allocation and no system call. The browser's `diagnostics`
//!    crate collects them (`docs/architecture/observability.md`).
//! 2. **Rules.** OB1–OB3 (the model), OB5 (the recorder), OB6 (categories,
//!    levels, the mask), OB7 (two tiers), OB8 (metrics' buckets), OB9
//!    (recording never changes behaviour; the panic hook), OB25 and OB27's
//!    first tier (cost); ES13 (the clock is injected); ES14.
//! 3. **Place** (`tools/crate-map.toml`): layer 0, the foundation; every
//!    process; group `trusted`; panic class `panic-free`, since its decoder
//!    reads records from less trusted processes inside the browser (OB24),
//!    and a panic in a recorder every process links would abort that process;
//!    no `unsafe`, and no dependency.
//! 4. **Interface.** *Provisional until M1-CP3's interface note on `trace`'s
//!    API and the diagnostics region (CR8),* which settles how names and
//!    metrics are declared, the call sites' macros, where buffers come from
//!    (CR9) and the region's format. Until then no other crate calls it.
//!    - [`record`]: the format. Each record is a run of little-endian 64-bit
//!      words: a header (kind, category, name, length, level), a timestamp,
//!      and a payload of at most four fields, one value, or bounded UTF-8
//!      text. [`decode`] is total: it checks every word, as a reader of
//!      untrusted memory must (OB11).
//!    - [`ring`]: a single-producer, single-consumer ring of words over
//!      storage the caller provides ([`RingStorage`]: shared memory in a
//!      helper, a file-backed mapping in the browser, [`OwnedRing`] in
//!      tests). A [`Writer`] drops a record that doesn't fit and counts it,
//!      and the count travels in a [`Kind::Dropped`] record before the next
//!      record that fits (OB5). A [`Reader`] copies records out and refuses an
//!      index no correct writer could publish.
//!    - [`category`]: [`Category`] and [`Level`], and the mask that switches
//!      the detailed tier on per category ([`enabled`]: one relaxed load).
//!    - [`clock`]: timestamps come from the clock the process installs
//!      ([`set_clock`]): `mach_continuous_time` through `platform-event` in
//!      production (OB2), EL24's virtual clock in single-process tests (OB26).
//!    - [`recorder`]: each thread [`install`]s its ring once; then
//!      [`span_begin`], [`span_end`], [`event`], [`counter`], [`gauge`],
//!      [`histogram`], [`flow_begin`], [`flow_end`] and [`log`] write records
//!      into it. A thread without a ring records nothing.
//!    - [`metric`]: the log-linear buckets of histograms (OB8).
//!    - [`install_panic_hook`]: records each panic's message and location,
//!      without allocating, before the previous hook runs (OB9).
//! 5. **Invariants.**
//!    1. Each ring has one writer, its thread, and one reader (OB5, IR7).
//!    2. A record is published whole: its words are stored, then the write
//!       index is released past them.
//!    3. Recording takes no lock, allocates nothing, makes no system call
//!       and never blocks; a full ring loses the record, never the thread's
//!       time (OB9, EL1).
//!    4. Encoding is canonical: one encoding per record, zero padding, zero
//!       reserved bits, so the decoder can refuse everything else (IP9).
//!    5. No function in this crate panics (the class's lints).
//! 6. **Threads.** A ring's [`Writer`] lives in its thread's thread-local
//!    slot; its [`Reader`] is the collector's, on another thread or in
//!    another process. The mask and the clock are process-wide.
//! 7. **Tests.** Each module's unit tests (round trips, the decoder's
//!    refusals, the ring's wrap, drops and overruns); `tests/allocation.rs`
//!    (no allocation while recording) and `tests/panic.rs` (the hook) in
//!    their own binaries; `tests/cost.rs`, run optimized, for OB25's
//!    instructions and cycles per record. The ring's concurrency is checked
//!    with loom at M1-CP3 (TS11), when the regions arrive.
//! 8. **Efficiency.** A disabled detailed record costs its mask check; an
//!    enabled one, the clock, a few stores and a release. OB25's ceilings are
//!    2 ns and 50 ns, and the owner asked for "minimal performance overhead"
//!    (2026-10-23), so `tests/cost.rs` asserts the measured cost with
//!    headroom, not the ceiling.

#![forbid(unsafe_code)]
// Panic-free (ES10; the crate map): its decoder reads records from less
// trusted processes inside the browser (OB24), and every process records
// through it, so a panic here would abort any of them.
#![deny(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

pub mod category;
pub mod clock;
pub mod metric;
mod panic;
pub mod record;
pub mod recorder;
pub mod ring;

pub use category::{Category, Level, MAX_LEVEL, disable, enable, enabled};
pub use clock::{ClockAlreadySet, now, set_clock};
pub use panic::install_panic_hook;
pub use record::{DecodeError, Fields, Header, Kind, Name, Record, Records, Text, decode, records};
pub use recorder::{
    counter, event, flow_begin, flow_end, gauge, histogram, install, log, span_begin, span_end,
    uninstall,
};
pub use ring::{OwnedRing, Reader, RingError, RingStorage, Writer};
