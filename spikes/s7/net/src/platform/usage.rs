//! The process's own counters: CPU time, context switches, wakeups,
//! footprint, instructions and threads (`sys/resource.h`, `libproc.h`,
//! `sys/proc_info.h`).

use core::ffi::{c_int, c_void};
use std::time::Duration;

/// `struct timeval` (`sys/_types/_timeval.h`): a `long` and an `int`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Timeval {
    sec: i64,
    usec: i32,
}

/// `struct rusage` (`sys/resource.h`): two timevals, then 14 `long`s, of
/// which `ru_nvcsw` and `ru_nivcsw` are the last two.
#[repr(C)]
struct Rusage {
    utime: Timeval,
    stime: Timeval,
    longs: [i64; 14],
}

const _: () = assert!(size_of::<Rusage>() == 144);
const RUSAGE_SELF: c_int = 0;

/// `struct rusage_info_v4` (`sys/resource.h`): a 16-byte UUID, then 35
/// counters.
#[repr(C)]
struct RusageInfoV4 {
    uuid: [u8; 16],
    counters: [u64; 35],
}

const _: () = assert!(size_of::<RusageInfoV4>() == 296);
const RUSAGE_INFO_V4: c_int = 4;
/// Indices into `counters`.
const PKG_IDLE_WKUPS: usize = 2;
const INTERRUPT_WKUPS: usize = 3;
const PHYS_FOOTPRINT: usize = 7;
const LIFETIME_MAX_PHYS_FOOTPRINT: usize = 28;
const INSTRUCTIONS: usize = 29;
const CYCLES: usize = 30;

/// `struct proc_taskinfo` (`sys/proc_info.h`): six 64-bit fields, then
/// twelve 32-bit ones, of which `pti_threadnum` is the tenth.
#[repr(C)]
struct TaskInfo {
    wide: [u64; 6],
    narrow: [i32; 12],
}

const _: () = assert!(size_of::<TaskInfo>() == 96);
const PROC_PIDTASKINFO: c_int = 4;
const THREADNUM: usize = 9;

// libSystem: `sys/resource.h`, `libproc.h` (N2).
unsafe extern "C" {
    fn getrusage(who: c_int, usage: *mut Rusage) -> c_int;
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut c_void) -> c_int;
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

/// A snapshot of this process's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Usage {
    /// User plus system CPU time, all threads.
    pub(crate) cpu: Duration,
    /// Voluntary and involuntary context switches.
    pub(crate) switches: u64,
    /// Wakeups of an idle processor package to run this process.
    pub(crate) idle_wakeups: u64,
    /// Interrupt wakeups attributed to this process.
    pub(crate) interrupt_wakeups: u64,
    /// The physical footprint now, and its lifetime maximum, in bytes.
    pub(crate) footprint: u64,
    pub(crate) peak_footprint: u64,
    /// Instructions retired and cycles spent, all threads.
    pub(crate) instructions: u64,
    pub(crate) cycles: u64,
}

impl Usage {
    /// The counters now. A counter that can't be read is zero.
    pub(crate) fn now() -> Self {
        let mut usage = Self::default();
        let mut r = Rusage {
            utime: Timeval { sec: 0, usec: 0 },
            stime: Timeval { sec: 0, usec: 0 },
            longs: [0; 14],
        };
        // SAFETY: N1 (`r` is a live `struct rusage`); N2.
        if unsafe { getrusage(RUSAGE_SELF, &raw mut r) } == 0 {
            usage.cpu = duration(r.utime) + duration(r.stime);
            usage.switches = u64::try_from(r.longs[12] + r.longs[13]).unwrap_or(0);
        }
        let mut info = RusageInfoV4 {
            uuid: [0; 16],
            counters: [0; 35],
        };
        // SAFETY: N1 (`info` is a live `rusage_info_v4`, the struct the V4
        // flavor fills); N2.
        if unsafe { proc_pid_rusage(pid(), RUSAGE_INFO_V4, (&raw mut info).cast()) } == 0 {
            let c = &info.counters;
            usage.idle_wakeups = c[PKG_IDLE_WKUPS];
            usage.interrupt_wakeups = c[INTERRUPT_WKUPS];
            usage.footprint = c[PHYS_FOOTPRINT];
            usage.peak_footprint = c[LIFETIME_MAX_PHYS_FOOTPRINT];
            usage.instructions = c[INSTRUCTIONS];
            usage.cycles = c[CYCLES];
        }
        usage
    }

    /// The counters that accumulate, from `earlier` to `self`; the
    /// footprints are `self`'s.
    pub(crate) fn since(&self, earlier: &Self) -> Self {
        Self {
            cpu: self.cpu.saturating_sub(earlier.cpu),
            switches: self.switches.saturating_sub(earlier.switches),
            idle_wakeups: self.idle_wakeups.saturating_sub(earlier.idle_wakeups),
            interrupt_wakeups: self
                .interrupt_wakeups
                .saturating_sub(earlier.interrupt_wakeups),
            footprint: self.footprint,
            peak_footprint: self.peak_footprint,
            instructions: self.instructions.saturating_sub(earlier.instructions),
            cycles: self.cycles.saturating_sub(earlier.cycles),
        }
    }
}

/// The number of threads in this process, if libproc answers.
pub(crate) fn threads() -> Option<u32> {
    let mut info = TaskInfo {
        wide: [0; 6],
        narrow: [0; 12],
    };
    let size = c_int::try_from(size_of::<TaskInfo>()).ok()?;
    // SAFETY: N1 (`info` is a live `proc_taskinfo` of `size` bytes); N2.
    let written = unsafe { proc_pidinfo(pid(), PROC_PIDTASKINFO, 0, (&raw mut info).cast(), size) };
    (written == size)
        .then(|| u32::try_from(info.narrow[THREADNUM]).ok())
        .flatten()
}

fn pid() -> c_int {
    c_int::try_from(std::process::id()).unwrap_or(0)
}

fn duration(t: Timeval) -> Duration {
    Duration::from_secs(u64::try_from(t.sec).unwrap_or(0))
        + Duration::from_micros(u64::try_from(t.usec).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_read_and_accumulate() {
        let before = Usage::now();
        let mut x = 0_u64;
        for i in 0..2_000_000_u64 {
            x = x.wrapping_mul(31).wrapping_add(i);
        }
        std::hint::black_box(x);
        let after = Usage::now();
        assert!(after.footprint > 0 && after.peak_footprint >= after.footprint);
        assert!(after.since(&before).instructions > 0);
        assert!(threads().is_some_and(|n| n >= 1));
    }
}
