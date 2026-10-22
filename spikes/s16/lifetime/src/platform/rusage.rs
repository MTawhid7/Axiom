//! A process's resource usage, from `proc_pid_rusage` at flavor
//! `RUSAGE_INFO_V6` (`sys/resource.h`, `libproc.h`), and the waits around it:
//! `waitid` with `WNOWAIT`, which leaves an exited child unreaped so that its
//! usage can still be read, and `wait4`, which reaps it and returns the
//! classic `struct rusage` beside it.

use std::io;

/// `RUSAGE_INFO_V6`.
const RUSAGE_INFO_V6: i32 = 6;
/// `P_PID`, `WEXITED` and `WNOWAIT` (`sys/wait.h`).
const P_PID: u32 = 1;
const WEXITED: i32 = 0x04;
const WNOWAIT: i32 = 0x20;

/// `struct rusage_info_v6`: a 16-byte UUID, then 50 counters and six
/// reserved words; 464 bytes (observed by compiling against the SDK,
/// 2026-10-22).
#[repr(C)]
struct RusageInfoV6 {
    uuid: [u8; 16],
    counters: [u64; 56],
}

const _: () = assert!(size_of::<RusageInfoV6>() == 464);

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

unsafe extern "C" {
    fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut RusageInfoV6) -> i32;
    fn waitid(idtype: u32, id: u32, info: *mut [u64; 13], options: i32) -> i32;
    fn wait4(pid: i32, status: *mut i32, options: i32, usage: *mut Rusage) -> i32;
}

/// Indices into [`Usage`]'s counters, in the header's order.
pub(crate) mod index {
    pub(crate) const USER_TIME: usize = 0;
    pub(crate) const SYSTEM_TIME: usize = 1;
    pub(crate) const PKG_IDLE_WKUPS: usize = 2;
    pub(crate) const INTERRUPT_WKUPS: usize = 3;
    pub(crate) const PAGEINS: usize = 4;
    pub(crate) const PHYS_FOOTPRINT: usize = 7;
    pub(crate) const DISKIO_BYTESREAD: usize = 16;
    pub(crate) const DISKIO_BYTESWRITTEN: usize = 17;
    pub(crate) const LOGICAL_WRITES: usize = 27;
    pub(crate) const LIFETIME_MAX_PHYS_FOOTPRINT: usize = 28;
    pub(crate) const INSTRUCTIONS: usize = 29;
    pub(crate) const CYCLES: usize = 30;
    pub(crate) const BILLED_ENERGY: usize = 31;
    pub(crate) const SERVICED_ENERGY: usize = 32;
    pub(crate) const INTERVAL_MAX_PHYS_FOOTPRINT: usize = 33;
    pub(crate) const RUNNABLE_TIME: usize = 34;
    pub(crate) const USER_PTIME: usize = 36;
    pub(crate) const SYSTEM_PTIME: usize = 37;
    pub(crate) const PINSTRUCTIONS: usize = 38;
    pub(crate) const PCYCLES: usize = 39;
    pub(crate) const ENERGY_NJ: usize = 40;
    pub(crate) const PENERGY_NJ: usize = 41;
}

/// The counters of one `rusage_info_v6`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Usage(pub(crate) [u64; 56]);

impl Usage {
    pub(crate) fn get(&self, index: usize) -> u64 {
        self.0.get(index).copied().unwrap_or(0)
    }
}

/// `pid`'s usage: a process this user owns, running or exited and not yet
/// reaped.
pub(crate) fn usage(pid: i32) -> io::Result<Usage> {
    let mut info = RusageInfoV6 {
        uuid: [0; 16],
        counters: [0; 56],
    };
    // SAFETY: L2 and L10, `info` is a live `rusage_info_v6`, the struct the
    // V6 flavor fills.
    let result = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V6, &raw mut info) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Usage(info.counters))
}

/// Wait until the child `pid` has exited, leaving it unreaped.
pub(crate) fn wait_exited(pid: i32) -> io::Result<()> {
    let id = u32::try_from(pid).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    loop {
        let mut info = [0_u64; 13];
        // SAFETY: L2 and L10, `info` is a live 104-byte, 8-byte aligned
        // `siginfo_t`.
        let result = unsafe { waitid(P_PID, id, &raw mut info, WEXITED | WNOWAIT) };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// Reap the child `pid`: its wait status, and its user and system time from
/// `struct rusage`, in nanoseconds.
pub(crate) fn reap(pid: i32) -> io::Result<(i32, u64, u64)> {
    let zero = || Timeval {
        seconds: 0,
        microseconds: 0,
    };
    let mut usage = Rusage {
        user: zero(),
        system: zero(),
        counters: [0; 14],
    };
    let mut status: i32 = 0;
    loop {
        // SAFETY: L2, `status` and `usage` are live locals of the C types.
        let result = unsafe { wait4(pid, &raw mut status, 0, &raw mut usage) };
        if result == pid {
            break;
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    let ns = |t: &Timeval| {
        u64::try_from(t.seconds).unwrap_or(0) * 1_000_000_000
            + u64::try_from(t.microseconds).unwrap_or(0) * 1_000
    };
    Ok((status, ns(&usage.user), ns(&usage.system)))
}
