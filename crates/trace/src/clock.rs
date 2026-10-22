//! The clock that timestamps records (OB2), installed once by the process:
//! `mach_continuous_time` through `platform-event` in production, so that
//! every process's records share one timeline, and EL24's virtual clock in
//! single-process tests (OB26). The crate never reads a clock itself (ES13).

use std::fmt;
use std::sync::OnceLock;

static CLOCK: OnceLock<fn() -> u64> = OnceLock::new();

/// Installs the process's clock. The first call wins.
pub fn set_clock(clock: fn() -> u64) -> Result<(), ClockAlreadySet> {
    CLOCK.set(clock).map_err(|_| ClockAlreadySet)
}

/// The installed clock's reading, or 0 before one is installed.
#[inline]
#[must_use]
pub fn now() -> u64 {
    CLOCK.get().map_or(0, |clock| clock())
}

/// The process had already installed its clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockAlreadySet;

impl fmt::Display for ClockAlreadySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the process's trace clock is already installed")
    }
}

impl std::error::Error for ClockAlreadySet {}
