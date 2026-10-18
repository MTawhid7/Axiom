//! Launching helpers with `posix_spawn`, waiting for them, signalling them,
//! and adopting the socket a helper inherits.
//!
//! Helpers are launched with `POSIX_SPAWN_CLOEXEC_DEFAULT`, so a child
//! inherits only the descriptors its file actions name: `/dev/null` as
//! standard input, a chosen standard output, optionally standard error, and
//! any extra descriptors mapped to fixed numbers (process model §8).

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use std::ffi::{CString, OsStr};
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

type SpawnAttr = *mut c_void;
type FileActions = *mut c_void;

/// `POSIX_SPAWN_CLOEXEC_DEFAULT` (`sys/spawn.h`).
const POSIX_SPAWN_CLOEXEC_DEFAULT: i16 = 0x4000;
/// `O_RDONLY` (`fcntl.h`).
const O_RDONLY: c_int = 0;
/// `SOL_SOCKET` and `SO_TYPE` (`sys/socket.h`).
const SOL_SOCKET: c_int = 0xffff;
const SO_TYPE: c_int = 0x1008;
/// `SO_SNDBUF` and `SO_RCVBUF` (`sys/socket.h`).
const SO_SNDBUF: c_int = 0x1001;
const SO_RCVBUF: c_int = 0x1002;

unsafe extern "C" {
    fn posix_spawn(
        pid: *mut c_int,
        path: *const c_char,
        file_actions: *const FileActions,
        attrp: *const SpawnAttr,
        argv: *const *mut c_char,
        envp: *const *mut c_char,
    ) -> c_int;
    fn posix_spawnattr_init(attr: *mut SpawnAttr) -> c_int;
    fn posix_spawnattr_setflags(attr: *mut SpawnAttr, flags: i16) -> c_int;
    fn posix_spawnattr_destroy(attr: *mut SpawnAttr) -> c_int;
    fn posix_spawn_file_actions_init(actions: *mut FileActions) -> c_int;
    fn posix_spawn_file_actions_adddup2(
        actions: *mut FileActions,
        fd: c_int,
        newfd: c_int,
    ) -> c_int;
    fn posix_spawn_file_actions_addinherit_np(actions: *mut FileActions, fd: c_int) -> c_int;
    fn posix_spawn_file_actions_addopen(
        actions: *mut FileActions,
        fd: c_int,
        path: *const c_char,
        oflag: c_int,
        mode: u16,
    ) -> c_int;
    fn posix_spawn_file_actions_destroy(actions: *mut FileActions) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn getsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *mut c_void,
        length: *mut u32,
    ) -> c_int;
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut c_void) -> c_int;
    fn setsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *const c_void,
        length: u32,
    ) -> c_int;
}

/// Set a socket's send and receive buffer sizes.
pub fn set_socket_buffers(socket: &UnixStream, bytes: usize) -> io::Result<()> {
    let value = c_int::try_from(bytes).map_err(|_| invalid("buffer size too large"))?;
    let length = u32::try_from(size_of::<c_int>()).expect("an int's size fits");
    for name in [SO_SNDBUF, SO_RCVBUF] {
        // SAFETY: X2. `value` is a live, aligned int of the length passed,
        // which setsockopt only reads during the call; the descriptor belongs
        // to `socket`, borrowed for the call.
        let result = unsafe {
            setsockopt(
                socket.as_raw_fd(),
                SOL_SOCKET,
                name,
                (&raw const value).cast(),
                length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// `RUSAGE_INFO_V4` (`sys/resource.h`).
const RUSAGE_INFO_V4: c_int = 4;

/// `struct rusage_info_v4`: a 16-byte UUID followed by 35 counters.
#[repr(C)]
struct RusageInfoV4 {
    uuid: [u8; 16],
    counters: [u64; 35],
}

const _: () = assert!(size_of::<RusageInfoV4>() == 296);
/// Indices of `ri_phys_footprint` and `ri_lifetime_max_phys_footprint`.
const PHYS_FOOTPRINT: usize = 7;
const LIFETIME_MAX_PHYS_FOOTPRINT: usize = 28;

/// A process's memory footprint, in bytes: what macOS charges it for, the
/// figure Activity Monitor and S2 report (not resident size, which includes
/// shared framework pages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub current: u64,
    pub lifetime_max: u64,
}

/// Read `pid`'s footprint (a process this user owns).
pub fn footprint(pid: i32) -> io::Result<Footprint> {
    let mut info = RusageInfoV4 {
        uuid: [0; 16],
        counters: [0; 35],
    };
    // SAFETY: X2. `info` is a live `rusage_info_v4`, the struct the V4 flavor
    // fills.
    let result = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V4, (&raw mut info).cast()) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Footprint {
        current: info.counters[PHYS_FOOTPRINT],
        lifetime_max: info.counters[LIFETIME_MAX_PHYS_FOOTPRINT],
    })
}

/// A helper to launch.
#[derive(Debug)]
pub struct Command<'a> {
    program: CString,
    args: Vec<CString>,
    env: Vec<CString>,
    stdout: Option<BorrowedFd<'a>>,
    inherit_stderr: bool,
    map: Vec<(BorrowedFd<'a>, RawFd)>,
}

impl<'a> Command<'a> {
    pub fn new(program: &Path) -> io::Result<Self> {
        Ok(Self {
            program: c_string(program.as_os_str())?,
            args: Vec::new(),
            env: Vec::new(),
            stdout: None,
            inherit_stderr: false,
            map: Vec::new(),
        })
    }

    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> io::Result<&mut Self> {
        self.args.push(c_string(arg.as_ref())?);
        Ok(self)
    }

    /// Add `key=value` to the child's environment, which is otherwise empty.
    pub fn env(&mut self, key: &str, value: &str) -> io::Result<&mut Self> {
        self.env
            .push(c_string(OsStr::new(&format!("{key}={value}")))?);
        Ok(self)
    }

    /// The child's standard output. Without one, the child has none.
    pub fn stdout(&mut self, fd: BorrowedFd<'a>) -> &mut Self {
        self.stdout = Some(fd);
        self
    }

    /// Let the child share this process's standard error.
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.inherit_stderr = true;
        self
    }

    /// Give the child `fd` as descriptor number `target` (3 or above).
    pub fn map_fd(&mut self, fd: BorrowedFd<'a>, target: RawFd) -> &mut Self {
        self.map.push((fd, target));
        self
    }

    /// Launch the child.
    pub fn spawn(&self) -> io::Result<Child> {
        self.check_mappings()?;
        let actions = Actions::new()?;
        actions.open_null(0)?;
        if let Some(fd) = self.stdout {
            actions.dup2(fd.as_raw_fd(), 1)?;
        }
        if self.inherit_stderr {
            actions.dup2(2, 2)?;
        }
        for &(fd, target) in &self.map {
            actions.dup2(fd.as_raw_fd(), target)?;
        }
        let attr = Attr::new()?;
        attr.set_flags(POSIX_SPAWN_CLOEXEC_DEFAULT)?;

        let mut argv: Vec<*mut c_char> = Vec::with_capacity(self.args.len() + 2);
        argv.push(self.program.as_ptr().cast_mut());
        argv.extend(self.args.iter().map(|a| a.as_ptr().cast_mut()));
        argv.push(ptr::null_mut());
        let mut envp: Vec<*mut c_char> = self.env.iter().map(|e| e.as_ptr().cast_mut()).collect();
        envp.push(ptr::null_mut());

        let mut pid: c_int = 0;
        // SAFETY: X6 (`argv` and `envp` are null-terminated and point into
        // CStrings held by `self`; `actions` and `attr` are initialized and
        // live); X1 (the program path); X2 (`pid`).
        let status = unsafe {
            posix_spawn(
                &raw mut pid,
                self.program.as_ptr(),
                &raw const actions.0,
                &raw const attr.0,
                argv.as_ptr(),
                envp.as_ptr(),
            )
        };
        errno_result(status)?;
        Ok(Child { pid })
    }

    /// A source descriptor must not be another mapping's target, or an
    /// earlier `dup2` would replace it before it is copied.
    fn check_mappings(&self) -> io::Result<()> {
        let targets: Vec<RawFd> = self.map.iter().map(|&(_, target)| target).collect();
        for &(fd, target) in &self.map {
            if target < 3 {
                return Err(invalid("extra descriptors must map to 3 or above"));
            }
            let source = fd.as_raw_fd();
            if source != target && (targets.contains(&source) || source <= 2) {
                return Err(invalid("a mapped descriptor collides with a target"));
            }
        }
        Ok(())
    }
}

/// A launched helper.
#[derive(Debug)]
pub struct Child {
    pid: c_int,
}

/// How a child ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// It exited with this status.
    Code(i32),
    /// It was terminated by this signal.
    Signal(i32),
}

impl Child {
    pub fn id(&self) -> i32 {
        self.pid
    }

    /// Wait for the child to end. Call once.
    pub fn wait(&self) -> io::Result<Exit> {
        let mut status: c_int = 0;
        loop {
            // SAFETY: X2 (`status`).
            let result = unsafe { waitpid(self.pid, &raw mut status, 0) };
            if result == self.pid {
                return Ok(decode_wait_status(status));
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    /// Send `signal` to the child.
    pub fn signal(&self, signal: i32) -> io::Result<()> {
        send_signal(self.pid, signal)
    }
}

/// Darwin's `<sys/wait.h>` encoding: the low 7 bits hold the terminating
/// signal (0 if the child exited; 0x7f if it stopped), and bits 8–15 hold the
/// exit status.
fn decode_wait_status(status: c_int) -> Exit {
    let signal = status & 0x7f;
    if signal == 0 {
        Exit::Code((status >> 8) & 0xff)
    } else {
        Exit::Signal(signal)
    }
}

/// Send `signal` to `pid`. Signal 0 only checks permission (`kill(2)`).
pub fn send_signal(pid: i32, signal: i32) -> io::Result<()> {
    // SAFETY: plain integer arguments.
    let result = unsafe { kill(pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Descriptors already adopted by [`adopt_inherited_socket`], one bit each
/// for descriptors 0–63 (X7).
static ADOPTED: AtomicU64 = AtomicU64::new(0);

/// Take ownership of the socket this process inherited as descriptor `fd`.
/// Fails if `fd` isn't an open socket, or has been adopted already.
pub fn adopt_inherited_socket(fd: RawFd) -> io::Result<UnixStream> {
    let bit = u32::try_from(fd)
        .ok()
        .filter(|&n| n < 64)
        .map(|n| 1_u64 << n)
        .ok_or_else(|| invalid("descriptor out of range"))?;
    let mut socket_type: c_int = 0;
    let mut length = u32::try_from(size_of::<c_int>()).expect("an int's size fits");
    // SAFETY: X2 (`socket_type` and `length` are live locals of the sizes
    // passed). On a closed descriptor or a non-socket, the call only fails.
    let result = unsafe {
        getsockopt(
            fd,
            SOL_SOCKET,
            SO_TYPE,
            (&raw mut socket_type).cast(),
            &raw mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if ADOPTED.fetch_or(bit, Ordering::SeqCst) & bit != 0 {
        return Err(invalid("descriptor already adopted"));
    }
    // SAFETY: X7. `fd` is an open socket (checked above), nothing else in
    // this process owns it (the browser hands it over at spawn), and the
    // `ADOPTED` bit makes this the only adoption.
    Ok(unsafe { UnixStream::from_raw_fd(fd) })
}

/// Owns initialized `posix_spawn` file actions (X6).
struct Actions(FileActions);

impl Actions {
    fn new() -> io::Result<Self> {
        let mut raw: FileActions = ptr::null_mut();
        // SAFETY: X2 (`raw`).
        errno_result(unsafe { posix_spawn_file_actions_init(&raw mut raw) })?;
        Ok(Self(raw))
    }

    fn open_null(&self, fd: c_int) -> io::Result<()> {
        let path = c"/dev/null";
        // SAFETY: X6 (initialized actions); X1 (a static C string, which
        // posix_spawn copies).
        let status = unsafe {
            posix_spawn_file_actions_addopen(
                (&raw const self.0).cast_mut(),
                fd,
                path.as_ptr(),
                O_RDONLY,
                0,
            )
        };
        errno_result(status)
    }

    /// Copy `fd` to `target` in the child. Copying a descriptor onto itself
    /// would keep its close-on-exec flag, so that case uses
    /// `addinherit_np`, which `POSIX_SPAWN_CLOEXEC_DEFAULT` honours.
    fn dup2(&self, fd: c_int, target: c_int) -> io::Result<()> {
        let actions = (&raw const self.0).cast_mut();
        let status = if fd == target {
            // SAFETY: X6 (initialized actions).
            unsafe { posix_spawn_file_actions_addinherit_np(actions, fd) }
        } else {
            // SAFETY: X6 (initialized actions).
            unsafe { posix_spawn_file_actions_adddup2(actions, fd, target) }
        };
        errno_result(status)
    }
}

impl Drop for Actions {
    fn drop(&mut self) {
        // SAFETY: X6. Destroyed once, by its only owner.
        unsafe { posix_spawn_file_actions_destroy(&raw mut self.0) };
    }
}

/// Owns initialized `posix_spawn` attributes (X6).
struct Attr(SpawnAttr);

impl Attr {
    fn new() -> io::Result<Self> {
        let mut raw: SpawnAttr = ptr::null_mut();
        // SAFETY: X2 (`raw`).
        errno_result(unsafe { posix_spawnattr_init(&raw mut raw) })?;
        Ok(Self(raw))
    }

    fn set_flags(&self, flags: i16) -> io::Result<()> {
        // SAFETY: X6 (initialized attributes).
        errno_result(unsafe { posix_spawnattr_setflags((&raw const self.0).cast_mut(), flags) })
    }
}

impl Drop for Attr {
    fn drop(&mut self) {
        // SAFETY: X6. Destroyed once, by its only owner.
        unsafe { posix_spawnattr_destroy(&raw mut self.0) };
    }
}

/// `posix_spawn` and its helpers return an errno value instead of setting
/// `errno`.
fn errno_result(status: c_int) -> io::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status))
    }
}

fn c_string(s: &OsStr) -> io::Result<CString> {
    CString::new(s.as_bytes()).map_err(|_| invalid("a NUL byte in an argument"))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_statuses_decode_as_on_darwin() {
        assert_eq!(decode_wait_status(0), Exit::Code(0));
        assert_eq!(decode_wait_status(3 << 8), Exit::Code(3));
        assert_eq!(decode_wait_status(9), Exit::Signal(9));
        // A core-dump flag (0x80) doesn't change the signal.
        assert_eq!(decode_wait_status(0x80 | 6), Exit::Signal(6));
    }

    /// Run `/bin/sh -c script` and return what it prints.
    fn shell(script: &str, inherit_stderr: bool) -> String {
        let (reader, writer) = io::pipe().expect("pipe");
        let mut command = Command::new(Path::new("/bin/sh")).expect("command");
        command.arg("-c").and_then(|c| c.arg(script)).expect("args");
        command.stdout(std::os::fd::AsFd::as_fd(&writer));
        if inherit_stderr {
            command.inherit_stderr();
        }
        let child = command.spawn().expect("spawn");
        drop(command);
        drop(writer);
        let output = io::read_to_string(reader).expect("read");
        assert_eq!(child.wait().expect("wait"), Exit::Code(0));
        output.trim().to_owned()
    }

    #[test]
    fn a_child_inherits_only_the_descriptors_it_is_given() {
        // Standard error is open in this process and not close-on-exec, so
        // only POSIX_SPAWN_CLOEXEC_DEFAULT keeps it from the child.
        let probe = "if [ -e /dev/fd/2 ]; then echo open; else echo closed; fi";
        assert_eq!(shell(probe, false), "closed");
        assert_eq!(shell(probe, true), "open");
    }

    #[test]
    fn this_process_has_a_plausible_footprint() {
        let pid = i32::try_from(std::process::id()).expect("pid fits");
        let footprint = footprint(pid).expect("own footprint");
        assert!(footprint.current > 100 * 1024, "{footprint:?}");
        // The two counters aren't one snapshot: the lifetime maximum can trail
        // the current footprint (observed 2026-10-17: by 16 KiB, one page, in
        // about one run in 75). So only its scale is checked.
        assert!(footprint.lifetime_max > 100 * 1024, "{footprint:?}");
    }

    #[test]
    fn adopting_anything_but_an_open_socket_fails() {
        // A pipe is open but not a socket. The test owns it, so a wrong
        // adoption couldn't close a descriptor someone else depends on.
        let (reader, _writer) = io::pipe().expect("pipe");
        let error = adopt_inherited_socket(reader.as_raw_fd()).expect_err("a pipe");
        assert_eq!(error.raw_os_error(), Some(38), "ENOTSOCK: {error}");
        assert!(adopt_inherited_socket(-1).is_err());
        assert!(adopt_inherited_socket(64).is_err());
    }
}
