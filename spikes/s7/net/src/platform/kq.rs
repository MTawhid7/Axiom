//! kqueue (`sys/event.h`): the I/O thread's one wait (EL2, NW1). One
//! `kevent64` call submits the pending changes and waits on sockets,
//! DNS-SD's socket, a Mach port set (receiving the message in the same call),
//! a timer with leeway (EL18) and a user event that other threads trigger.

use core::ffi::c_int;
use core::ptr::NonNull;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::sync::Arc;
use std::time::Duration;

use super::mach::PortSet;

/// `struct kevent64_s` (`sys/event.h`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Kevent64 {
    ident: u64,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: i64,
    udata: u64,
    ext: [u64; 2],
}

const _: () = assert!(size_of::<Kevent64>() == 48);

/// `struct timespec` (`sys/_types/_timespec.h`): a `time_t` and a `long`.
#[repr(C)]
struct Timespec {
    sec: i64,
    nsec: i64,
}

// `sys/event.h` (N2).
const EVFILT_READ: i16 = -1;
const EVFILT_WRITE: i16 = -2;
const EVFILT_TIMER: i16 = -7;
const EVFILT_MACHPORT: i16 = -8;
const EVFILT_USER: i16 = -10;
const EV_ADD: u16 = 0x0001;
const EV_DELETE: u16 = 0x0002;
const EV_ONESHOT: u16 = 0x0010;
const EV_CLEAR: u16 = 0x0020;
const EV_ERROR: u16 = 0x4000;
const EV_EOF: u16 = 0x8000;
const NOTE_TRIGGER: u32 = 0x0100_0000;
const NOTE_NSECONDS: u32 = 0x0000_0004;
const NOTE_LEEWAY: u32 = 0x0000_0010;
// `mach/message.h` (N2): receive options for EVFILT_MACHPORT.
const MACH_RCV_MSG: u32 = 0x0000_0002;
const MACH_RCV_LARGE: u32 = 0x0000_0004;
/// `ENOENT`, `EINTR` (`sys/errno.h`).
const ENOENT: i64 = 2;
const EINTR: i32 = 4;

// libSystem: `sys/event.h` (N2).
unsafe extern "C" {
    fn kqueue() -> c_int;
    fn kevent64(
        kq: c_int,
        changelist: *const Kevent64,
        nchanges: c_int,
        eventlist: *mut Kevent64,
        nevents: c_int,
        flags: u32,
        timeout: *const Timespec,
    ) -> c_int;
}

/// The Mach receive buffer: room for S7's messages and their trailers.
const INBOX_WORDS: usize = 64;

/// What woke a wait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Readable {
        token: u64,
        eof: bool,
    },
    Writable {
        token: u64,
        eof: bool,
    },
    Timer {
        token: u64,
    },
    User {
        token: u64,
    },
    /// A Mach message, received by the wait itself: its id and its inline
    /// bytes after the header.
    Message {
        token: u64,
        id: i32,
        body: Vec<u8>,
    },
    /// A failed receive, with `mach_msg`'s return code.
    MessageError {
        token: u64,
        code: u32,
    },
    /// A change the kernel refused, with its errno.
    Refused {
        token: u64,
        filter: i16,
        errno: i64,
    },
}

/// A kqueue and the changes waiting for the next wait.
pub(crate) struct Poller {
    /// Shared with [`Waker`]s, which only trigger user events.
    kq: Arc<OwnedFd>,
    changes: Vec<Kevent64>,
    /// The Mach receive buffer, allocated here and freed in `Drop` once no
    /// registration refers to it (N5), and the port set registered with it.
    inbox: NonNull<[u64; INBOX_WORDS]>,
    inbox_set: Option<u64>,
    events: Vec<Kevent64>,
    /// `kevent64` calls that returned, with or without events.
    pub(crate) waits: u64,
}

impl Poller {
    pub(crate) fn new() -> io::Result<Self> {
        // SAFETY: no arguments; the result is a new descriptor or -1 (N2).
        let fd = unsafe { kqueue() };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: N7. `fd` is a new descriptor that nothing else owns.
        let kq = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Self {
            kq: Arc::new(kq),
            changes: Vec::new(),
            inbox: NonNull::from(Box::leak(Box::new([0; INBOX_WORDS]))),
            inbox_set: None,
            events: vec![Kevent64::default(); 64],
            waits: 0,
        })
    }

    /// Watch `fd` for reading, level-triggered, until it is closed.
    pub(crate) fn watch_read(&mut self, fd: BorrowedFd<'_>, token: u64) {
        self.push(fd_ident(fd), EVFILT_READ, EV_ADD, 0, 0, token, [0; 2]);
    }

    /// Report once when `fd` can be written.
    pub(crate) fn want_write(&mut self, fd: BorrowedFd<'_>, token: u64) {
        self.push(
            fd_ident(fd),
            EVFILT_WRITE,
            EV_ADD | EV_ONESHOT,
            0,
            0,
            token,
            [0; 2],
        );
    }

    /// Stop watching `fd` for reading, now rather than with the next wait,
    /// as `dns_sd.h` asks before a DNS-SD connection is deallocated (N7).
    pub(crate) fn unwatch_read_now(&mut self, fd: BorrowedFd<'_>) -> io::Result<()> {
        let change = Kevent64 {
            ident: fd_ident(fd),
            filter: EVFILT_READ,
            flags: EV_DELETE,
            ..Kevent64::default()
        };
        submit(&self.kq, &change)
    }

    /// Arm the one-shot timer `token` to fire after `after`, with `leeway`
    /// (EL18). Arming it again replaces it.
    pub(crate) fn arm_timer(&mut self, token: u64, after: Duration, leeway: Duration) {
        let ns = |d: Duration| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX);
        let ext = [0, u64::try_from(ns(leeway)).unwrap_or(0)];
        self.push(
            token,
            EVFILT_TIMER,
            EV_ADD | EV_ONESHOT,
            NOTE_NSECONDS | NOTE_LEEWAY,
            ns(after).max(1),
            token,
            ext,
        );
    }

    /// Disarm the timer `token`, if it hasn't fired.
    pub(crate) fn disarm_timer(&mut self, token: u64) {
        self.push(token, EVFILT_TIMER, EV_DELETE, 0, 0, token, [0; 2]);
    }

    /// Watch a user event, which [`Waker::wake`] triggers from any thread.
    pub(crate) fn watch_user(&mut self, token: u64) {
        self.push(token, EVFILT_USER, EV_ADD | EV_CLEAR, 0, 0, token, [0; 2]);
    }

    /// Watch `set`, receiving each message into the poller's buffer within
    /// the wait that reports it (EL2).
    pub(crate) fn watch_port_set(&mut self, set: &PortSet, token: u64) {
        let len = u64::try_from(size_of::<[u64; INBOX_WORDS]>()).unwrap_or(0);
        let ext = [self.inbox.as_ptr().expose_provenance() as u64, len];
        self.inbox_set = Some(u64::from(set.name()));
        self.push(
            u64::from(set.name()),
            EVFILT_MACHPORT,
            EV_ADD,
            MACH_RCV_MSG | MACH_RCV_LARGE,
            0,
            token,
            ext,
        );
    }

    /// A handle that other threads use to trigger user events.
    pub(crate) fn waker(&self) -> Waker {
        Waker {
            kq: Arc::clone(&self.kq),
        }
    }

    /// Submit the pending changes and wait, in one `kevent64` call, for at
    /// most `timeout` (forever if `None`). Appends what woke it to `out`.
    pub(crate) fn wait(
        &mut self,
        timeout: Option<Duration>,
        out: &mut Vec<Event>,
    ) -> io::Result<()> {
        let spec = timeout.map(|t| Timespec {
            sec: i64::try_from(t.as_secs()).unwrap_or(i64::MAX),
            nsec: i64::from(t.subsec_nanos()),
        });
        let spec_ptr = spec.as_ref().map_or(core::ptr::null(), core::ptr::from_ref);
        let mut changes = core::mem::take(&mut self.changes);
        let count = loop {
            let nchanges = c_int::try_from(changes.len()).map_err(io::Error::other)?;
            let nevents = c_int::try_from(self.events.len()).map_err(io::Error::other)?;
            // SAFETY: N5. Both lists are live slices of `kevent64_s` of the
            // lengths passed; the Mach buffer registered earlier belongs to
            // `self`, borrowed mutably for this call. `spec_ptr` is null or
            // points to `spec`, alive until the call returns.
            let n = unsafe {
                kevent64(
                    self.kq.as_raw_fd(),
                    changes.as_ptr(),
                    nchanges,
                    self.events.as_mut_ptr(),
                    nevents,
                    0,
                    spec_ptr,
                )
            };
            if n >= 0 {
                break n;
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(EINTR) {
                return Err(error);
            }
            // Interrupted: the changes were applied (kevent(2)).
            changes.clear();
        };
        changes.clear();
        self.changes = changes;
        self.waits += 1;
        for i in 0..usize::try_from(count).unwrap_or(0) {
            let e = self.events[i];
            out.extend(self.decode(&e));
        }
        Ok(())
    }

    fn decode(&self, e: &Kevent64) -> Option<Event> {
        let token = e.udata;
        if e.flags & EV_ERROR != 0 {
            // Deleting a one-shot timer that has fired is expected.
            if e.filter == EVFILT_TIMER && e.data == ENOENT {
                return None;
            }
            return Some(Event::Refused {
                token,
                filter: e.filter,
                errno: e.data,
            });
        }
        let eof = e.flags & EV_EOF != 0;
        Some(match e.filter {
            EVFILT_READ => Event::Readable { token, eof },
            EVFILT_WRITE => Event::Writable { token, eof },
            EVFILT_TIMER => Event::Timer { token },
            EVFILT_USER => Event::User { token },
            EVFILT_MACHPORT if e.fflags != 0 => Event::MessageError {
                token,
                code: e.fflags,
            },
            EVFILT_MACHPORT => {
                // SAFETY: N5. The kernel wrote the message during the wait
                // that just returned, and writes the buffer only during a
                // later wait, which needs `&mut self`.
                let words = unsafe { self.inbox.as_ptr().read() };
                let size = usize::try_from(e.ext[1]).unwrap_or(0);
                let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_ne_bytes()).collect();
                let bytes = &bytes[..size.min(bytes.len())];
                // `mach_msg_header_t` is 24 bytes; `msgh_size` (offset 4)
                // excludes the trailer, and `msgh_id` is at offset 20.
                let header = 24;
                let id = i32::from_ne_bytes(bytes.get(20..24)?.try_into().ok()?);
                let sent = u32::from_ne_bytes(bytes.get(4..8)?.try_into().ok()?);
                let end = usize::try_from(sent).ok()?.min(bytes.len());
                Event::Message {
                    token,
                    id,
                    body: bytes.get(header..end)?.to_vec(),
                }
            }
            _ => return None,
        })
    }

    #[expect(clippy::too_many_arguments, reason = "the fields of a kevent64_s")]
    fn push(
        &mut self,
        ident: u64,
        filter: i16,
        flags: u16,
        notes: u32,
        data: i64,
        udata: u64,
        ext: [u64; 2],
    ) {
        self.changes.push(Kevent64 {
            ident,
            filter,
            flags,
            fflags: notes,
            data,
            udata,
            ext,
        });
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        // Wakers may keep the kqueue open: delete the registration that
        // points at the buffer before freeing it (N5).
        if let Some(ident) = self.inbox_set {
            let change = Kevent64 {
                ident,
                filter: EVFILT_MACHPORT,
                flags: EV_DELETE,
                ..Kevent64::default()
            };
            let _ = submit(&self.kq, &change);
        }
        // SAFETY: N5. The buffer came from `Box::leak` in `new`, and no
        // registration refers to it any more.
        drop(unsafe { Box::from_raw(self.inbox.as_ptr()) });
    }
}

/// Triggers a poller's user events from other threads.
#[derive(Clone)]
pub(crate) struct Waker {
    kq: Arc<OwnedFd>,
}

impl Waker {
    pub(crate) fn wake(&self, token: u64) -> io::Result<()> {
        let change = Kevent64 {
            ident: token,
            filter: EVFILT_USER,
            fflags: NOTE_TRIGGER,
            udata: token,
            ..Kevent64::default()
        };
        submit(&self.kq, &change)
    }
}

/// Apply one change now, receiving no events.
fn submit(kq: &OwnedFd, change: &Kevent64) -> io::Result<()> {
    // SAFETY: N5. A one-element change list and an empty event list, so no
    // event (and no Mach message) is delivered by this call.
    let n = unsafe {
        kevent64(
            kq.as_raw_fd(),
            core::ptr::from_ref(change),
            1,
            core::ptr::null_mut(),
            0,
            0,
            core::ptr::null(),
        )
    };
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn fd_ident(fd: BorrowedFd<'_>) -> u64 {
    u64::try_from(fd.as_raw_fd()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::fd::AsFd;
    use std::os::unix::net::UnixStream;
    use std::time::Instant;

    #[test]
    fn one_wait_reports_sockets_timers_users_and_messages() {
        let mut poller = Poller::new().unwrap();
        let (mut a, b) = UnixStream::pair().unwrap();
        poller.watch_read(b.as_fd(), 7);
        poller.watch_user(9);
        let set = PortSet::new().unwrap();
        let port = crate::platform::mach::Port::new().unwrap();
        set.insert(&port).unwrap();
        poller.watch_port_set(&set, 11);
        let mut out = Vec::new();

        a.write_all(b"x").unwrap();
        poller.wait(Some(Duration::from_secs(1)), &mut out).unwrap();
        assert_eq!(
            out,
            vec![Event::Readable {
                token: 7,
                eof: false
            }]
        );

        out.clear();
        poller.waker().wake(9).unwrap();
        // The socket is still readable (level-triggered); drain it first.
        let mut byte = [0_u8; 1];
        (&b).read_exact(&mut byte).unwrap();
        poller.wait(Some(Duration::from_secs(1)), &mut out).unwrap();
        assert_eq!(out, vec![Event::User { token: 9 }]);

        out.clear();
        port.make_send()
            .unwrap()
            .send(5, &[1, 2, 3, 4, 5, 6, 7, 8])
            .unwrap();
        poller.wait(Some(Duration::from_secs(1)), &mut out).unwrap();
        assert_eq!(
            out,
            vec![Event::Message {
                token: 11,
                id: 5,
                body: vec![1, 2, 3, 4, 5, 6, 7, 8]
            }]
        );

        out.clear();
        let start = Instant::now();
        poller.arm_timer(13, Duration::from_millis(20), Duration::from_millis(1));
        poller.wait(Some(Duration::from_secs(1)), &mut out).unwrap();
        assert_eq!(out, vec![Event::Timer { token: 13 }]);
        assert!(start.elapsed() >= Duration::from_millis(19));

        out.clear();
        poller
            .wait(Some(Duration::from_millis(30)), &mut out)
            .unwrap();
        assert!(out.is_empty(), "a one-shot timer fires once: {out:?}");
    }
}
