//! The host clock that every process shares, and each process's CPU time.

use std::sync::OnceLock;

#[repr(C)]
struct TimebaseInfo {
    numer: u32,
    denom: u32,
}

#[repr(C)]
struct Timeval {
    seconds: i64,
    microseconds: i32,
}

/// `struct rusage`: two `timeval`s, then fourteen `long` counters.
#[repr(C)]
struct Rusage {
    user: Timeval,
    system: Timeval,
    counters: [i64; 14],
}

const RUSAGE_SELF: i32 = 0;

unsafe extern "C" {
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut TimebaseInfo) -> i32;
    fn mach_wait_until(deadline: u64) -> i32;
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

/// Now, in ticks of `mach_absolute_time`, which every process on this Mac
/// shares.
pub(crate) fn now() -> u64 {
    // SAFETY: no arguments, no state.
    unsafe { mach_absolute_time() }
}

/// Sleep until `deadline`, in ticks.
pub(crate) fn wait_until(deadline: u64) {
    // SAFETY: no pointers; an interrupted wait just returns early.
    unsafe { mach_wait_until(deadline) };
}

fn timebase() -> (u64, u64) {
    static BASE: OnceLock<(u64, u64)> = OnceLock::new();
    *BASE.get_or_init(|| {
        let mut info = TimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: G2, `info` is a live local.
        let status = unsafe { mach_timebase_info(&raw mut info) };
        if status != 0 || info.denom == 0 {
            (1, 1)
        } else {
            (u64::from(info.numer), u64::from(info.denom))
        }
    })
}

/// Ticks to nanoseconds.
pub(crate) fn ns(ticks: u64) -> u64 {
    let (numer, denom) = timebase();
    ticks.saturating_mul(numer) / denom
}

/// Nanoseconds to ticks.
pub(crate) fn ticks(ns: u64) -> u64 {
    let (numer, denom) = timebase();
    ns.saturating_mul(denom) / numer
}

/// This process's CPU time so far, user and system, in nanoseconds.
pub(crate) fn cpu_ns() -> u64 {
    let zero = || Timeval {
        seconds: 0,
        microseconds: 0,
    };
    let mut usage = Rusage {
        user: zero(),
        system: zero(),
        counters: [0; 14],
    };
    // SAFETY: G2, `usage` is a live `struct rusage`.
    let status = unsafe { getrusage(RUSAGE_SELF, &raw mut usage) };
    if status != 0 {
        return 0;
    }
    let total = |t: &Timeval| {
        u64::try_from(t.seconds).unwrap_or(0) * 1_000_000_000
            + u64::try_from(t.microseconds).unwrap_or(0) * 1_000
    };
    total(&usage.user) + total(&usage.system)
}
