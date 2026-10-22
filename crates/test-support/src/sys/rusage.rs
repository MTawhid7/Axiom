//! The process's resource usage, from `proc_pid_rusage` at flavor
//! `RUSAGE_INFO_V6` (`sys/resource.h`, `libproc.h`), as S16 read it.

use std::io;

/// `RUSAGE_INFO_V6`.
const RUSAGE_INFO_V6: i32 = 6;
/// `ri_instructions`'s index among the struct's counters, after the UUID.
const INSTRUCTIONS: usize = 29;

/// `struct rusage_info_v6`: a 16-byte UUID, then 50 counters and six reserved
/// words; 464 bytes, as S16 observed by compiling against the SDK.
#[repr(C)]
struct RusageInfoV6 {
    uuid: [u8; 16],
    counters: [u64; 56],
}

const _: () = assert!(size_of::<RusageInfoV6>() == 464);

unsafe extern "C" {
    fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut RusageInfoV6) -> i32;
}

/// The instructions this process has retired, on every core, since it began.
pub(crate) fn instructions_retired() -> io::Result<u64> {
    let pid = i32::try_from(std::process::id())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut info = RusageInfoV6 {
        uuid: [0; 16],
        counters: [0; 56],
    };
    // SAFETY: I2, `info` is a live, zeroed `rusage_info_v6`, the struct the V6
    // flavor fills, and it is read only after the call succeeds.
    let result = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V6, &raw mut info) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info.counters.get(INSTRUCTIONS).copied().unwrap_or(0))
}
