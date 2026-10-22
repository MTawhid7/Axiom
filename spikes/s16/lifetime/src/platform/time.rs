//! The host clock that every process shares.

use std::sync::OnceLock;

#[repr(C)]
struct TimebaseInfo {
    numer: u32,
    denom: u32,
}

unsafe extern "C" {
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut TimebaseInfo) -> i32;
}

/// Now, in ticks of `mach_absolute_time`.
pub(crate) fn now() -> u64 {
    // SAFETY: no arguments, no state.
    unsafe { mach_absolute_time() }
}

fn timebase() -> (u64, u64) {
    static BASE: OnceLock<(u64, u64)> = OnceLock::new();
    *BASE.get_or_init(|| {
        let mut info = TimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: L2, `info` is a live local.
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
