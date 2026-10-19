//! Clocks and counters: the calling thread's CPU time, and the process's
//! wakeups and footprint from `proc_pid_rusage`.

use core::ffi::{c_int, c_void};

/// `CLOCK_THREAD_CPUTIME_ID`, from `<_time.h>`.
const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
/// `RUSAGE_INFO_V4`, from `<sys/resource.h>`.
const RUSAGE_INFO_V4: c_int = 4;

unsafe extern "C" {
    fn clock_gettime_nsec_np(clock_id: u32) -> u64;
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut c_void) -> c_int;
}

/// CPU time the calling thread has used so far, in nanoseconds. Time spent
/// blocked (waiting for the GPU, sleeping) isn't counted.
pub(crate) fn thread_cpu_ns() -> u64 {
    // SAFETY: K9. The declaration matches <_time.h>, and the clock is one it
    // defines.
    unsafe { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }
}

/// `struct rusage_info_v4`: a UUID, then 35 64-bit counters.
#[repr(C)]
struct RusageInfoV4 {
    uuid: [u8; 16],
    counters: [u64; 35],
}

/// Indices into the counters: `ri_pkg_idle_wkups`, `ri_interrupt_wkups`,
/// `ri_phys_footprint` and `ri_lifetime_max_phys_footprint`.
const IDLE_WAKEUPS: usize = 2;
const INTERRUPT_WAKEUPS: usize = 3;
const PHYS_FOOTPRINT: usize = 7;
const LIFETIME_MAX_PHYS_FOOTPRINT: usize = 28;

/// This process's counters at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Usage {
    /// Wakeups from package idle.
    pub(crate) idle_wakeups: u64,
    /// Wakeups by interrupts (timers among them).
    pub(crate) interrupt_wakeups: u64,
    /// The physical footprint, in bytes.
    pub(crate) footprint: u64,
    /// Its highest value so far.
    pub(crate) lifetime_max_footprint: u64,
}

/// Read this process's counters.
pub(crate) fn usage() -> Result<Usage, String> {
    let mut info = RusageInfoV4 {
        uuid: [0; 16],
        counters: [0; 35],
    };
    let pid = c_int::try_from(std::process::id()).map_err(|e| e.to_string())?;
    // SAFETY: K9, K2. The declaration matches <libproc.h>; `info` is a live
    // `rusage_info_v4` (<sys/resource.h>), the size RUSAGE_INFO_V4 asks for.
    let status = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V4, (&raw mut info).cast()) };
    if status != 0 {
        return Err(format!(
            "proc_pid_rusage failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(Usage {
        idle_wakeups: info.counters[IDLE_WAKEUPS],
        interrupt_wakeups: info.counters[INTERRUPT_WAKEUPS],
        footprint: info.counters[PHYS_FOOTPRINT],
        lifetime_max_footprint: info.counters[LIFETIME_MAX_PHYS_FOOTPRINT],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_time_advances_with_work_and_usage_reads() {
        let start = thread_cpu_ns();
        let mut x = 0_u64;
        for i in 0..2_000_000_u64 {
            x = x.wrapping_mul(31).wrapping_add(i);
        }
        std::hint::black_box(x);
        assert!(thread_cpu_ns() > start);
        let usage = usage().expect("this process's counters");
        assert!(usage.footprint > 0 && usage.lifetime_max_footprint >= usage.footprint);
    }
}
