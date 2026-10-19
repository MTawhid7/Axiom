//! DNS-SD (`dns_sd.h`): address queries and HTTPS-record queries, all on one
//! shared connection to mDNSResponder, whose socket the I/O thread watches
//! in its kqueue (NW7). Answers arrive in callbacks run by
//! `DNSServiceProcessResult`, called only once the socket is readable, so
//! no thread blocks waiting for DNS.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use std::cell::RefCell;
use std::ffi::CString;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::BorrowedFd;

/// `DNSServiceRef` (`dns_sd.h`): a pointer to an opaque struct.
type ServiceRef = *mut c_void;

// `dns_sd.h` (N2).
const FLAGS_MORE_COMING: u32 = 0x1;
const FLAGS_ADD: u32 = 0x2;
const FLAGS_RETURN_INTERMEDIATES: u32 = 0x1000;
const FLAGS_SHARE_CONNECTION: u32 = 0x4000;
const CLASS_IN: u16 = 1;
/// `kDNSServiceType_HTTPS`.
pub(crate) const TYPE_HTTPS: u16 = 65;
const ERR_NO_ERROR: i32 = 0;
// `sys/socket.h` (N2).
const AF_INET: u8 = 2;
const AF_INET6: u8 = 30;

type AddrInfoReply = unsafe extern "C" fn(
    sd: ServiceRef,
    flags: u32,
    interface: u32,
    error: i32,
    hostname: *const c_char,
    address: *const u8,
    ttl: u32,
    context: *mut c_void,
);

type QueryRecordReply = unsafe extern "C" fn(
    sd: ServiceRef,
    flags: u32,
    interface: u32,
    error: i32,
    fullname: *const c_char,
    rrtype: u16,
    rrclass: u16,
    rdlen: u16,
    rdata: *const u8,
    ttl: u32,
    context: *mut c_void,
);

// libSystem (libsystem_dnssd): `dns_sd.h` (N2).
unsafe extern "C" {
    fn DNSServiceCreateConnection(sd: *mut ServiceRef) -> i32;
    fn DNSServiceGetAddrInfo(
        sd: *mut ServiceRef,
        flags: u32,
        interface: u32,
        protocol: u32,
        hostname: *const c_char,
        callback: AddrInfoReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceQueryRecord(
        sd: *mut ServiceRef,
        flags: u32,
        interface: u32,
        fullname: *const c_char,
        rrtype: u16,
        rrclass: u16,
        callback: QueryRecordReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceRefSockFD(sd: ServiceRef) -> c_int;
    fn DNSServiceProcessResult(sd: ServiceRef) -> i32;
    fn DNSServiceRefDeallocate(sd: ServiceRef);
}

/// One callback's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    /// The caller's id for the query.
    pub(crate) query: u64,
    /// `kDNSServiceErr_NoError`, or the error (`kDNSServiceErr_NoSuchRecord`
    /// for a negative answer).
    pub(crate) error: i32,
    /// Added (`kDNSServiceFlagsAdd`) rather than removed.
    pub(crate) add: bool,
    /// More answers are already queued (`kDNSServiceFlagsMoreComing`).
    pub(crate) more: bool,
    pub(crate) ttl: u32,
    pub(crate) data: AnswerData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnswerData {
    Address(IpAddr),
    /// A record's type and RDATA.
    Record(u16, Vec<u8>),
    None,
}

/// What a callback needs: its query's id, and where answers go (N3).
struct Context {
    query: u64,
    sink: *const RefCell<Vec<Answer>>,
}

/// A shared connection to mDNSResponder and its queries (N3).
pub(crate) struct Resolver {
    main: ServiceRef,
    /// Each query's id, ref and context box, in start order.
    queries: Vec<(u64, ServiceRef, Box<Context>)>,
    answers: Box<RefCell<Vec<Answer>>>,
}

impl Resolver {
    pub(crate) fn new() -> Result<Self, i32> {
        let mut main: ServiceRef = ptr::null_mut();
        // SAFETY: N1 (`main` is a live out-parameter); N2.
        let error = unsafe { DNSServiceCreateConnection(&raw mut main) };
        if error != ERR_NO_ERROR || main.is_null() {
            return Err(error);
        }
        Ok(Self {
            main,
            queries: Vec::new(),
            answers: Box::new(RefCell::new(Vec::new())),
        })
    }

    /// The connection's socket, for the kqueue (N7: valid while `self` is).
    pub(crate) fn fd(&self) -> BorrowedFd<'_> {
        // SAFETY: N3. `main` is a live shared connection; the descriptor
        // stays open until it is deallocated, which needs `&mut self`.
        let fd = unsafe { DNSServiceRefSockFD(self.main) };
        // SAFETY: N7. The descriptor belongs to the connection, which
        // outlives the borrow.
        unsafe { BorrowedFd::borrow_raw(fd) }
    }

    /// Resolve `host`'s addresses, both families as the system decides
    /// (`protocol` 0), negative answers included.
    pub(crate) fn addresses(&mut self, query: u64, host: &str) -> Result<(), i32> {
        let name = CString::new(host).map_err(|_| -1)?;
        let mut context = self.context(query);
        let mut sd = self.main;
        // SAFETY: N1 (`name` and `sd` live through the call); N3 (`context`
        // is boxed and kept with the query's ref, below).
        let error = unsafe {
            DNSServiceGetAddrInfo(
                &raw mut sd,
                FLAGS_SHARE_CONNECTION | FLAGS_RETURN_INTERMEDIATES,
                0,
                0,
                name.as_ptr(),
                on_address,
                ptr::from_mut(context.as_mut()).cast(),
            )
        };
        self.started(query, error, sd, context)
    }

    /// Query `name`'s records of type `rrtype` (such as [`TYPE_HTTPS`]),
    /// negative answers included.
    pub(crate) fn records(&mut self, query: u64, name: &str, rrtype: u16) -> Result<(), i32> {
        let name = CString::new(name).map_err(|_| -1)?;
        let mut context = self.context(query);
        let mut sd = self.main;
        // SAFETY: as in `addresses`: N1, N3.
        let error = unsafe {
            DNSServiceQueryRecord(
                &raw mut sd,
                FLAGS_SHARE_CONNECTION | FLAGS_RETURN_INTERMEDIATES,
                0,
                name.as_ptr(),
                rrtype,
                CLASS_IN,
                on_record,
                ptr::from_mut(context.as_mut()).cast(),
            )
        };
        self.started(query, error, sd, context)
    }

    /// Stop query `query`, if it is running.
    pub(crate) fn cancel(&mut self, query: u64) {
        if let Some(i) = self.queries.iter().position(|q| q.0 == query) {
            let (_, sd, context) = self.queries.remove(i);
            // SAFETY: N3. The query's ref is deallocated once, before its
            // context box (dropped after this statement).
            unsafe { DNSServiceRefDeallocate(sd) };
            drop(context);
        }
    }

    /// Read one reply from the socket, which must be readable, and return
    /// the answers its callbacks produced.
    pub(crate) fn process(&mut self) -> Result<Vec<Answer>, i32> {
        // SAFETY: N3. Callbacks run during this call, on this thread, and
        // push into `answers`, which no borrow holds across it.
        let error = unsafe { DNSServiceProcessResult(self.main) };
        let answers = self.answers.take();
        if error == ERR_NO_ERROR {
            Ok(answers)
        } else {
            Err(error)
        }
    }

    #[expect(
        clippy::unnecessary_box_returns,
        reason = "DNS-SD keeps a pointer to it: it needs a stable address (N3)"
    )]
    fn context(&self, query: u64) -> Box<Context> {
        Box::new(Context {
            query,
            sink: ptr::from_ref(self.answers.as_ref()),
        })
    }

    fn started(
        &mut self,
        query: u64,
        error: i32,
        sd: ServiceRef,
        context: Box<Context>,
    ) -> Result<(), i32> {
        if error == ERR_NO_ERROR {
            self.queries.push((query, sd, context));
            Ok(())
        } else {
            Err(error)
        }
    }
}

impl Drop for Resolver {
    fn drop(&mut self) {
        while let Some((_, sd, context)) = self.queries.pop() {
            // SAFETY: N3. Each query's ref goes before its context, and all
            // of them before the shared connection.
            unsafe { DNSServiceRefDeallocate(sd) };
            drop(context);
        }
        // SAFETY: N3. The shared connection is deallocated once, last.
        unsafe { DNSServiceRefDeallocate(self.main) };
    }
}

/// Push `answer` to the sink `context` names.
///
/// # Safety
///
/// `context` must be a `Context` pointer that DNS-SD was given (N3).
unsafe fn deliver(context: *mut c_void, make: impl FnOnce(u64) -> Answer) {
    // SAFETY: N3. The box lives as long as the query, and callbacks run
    // only inside `process`, on the resolver's thread.
    let context = unsafe { &*context.cast::<Context>() };
    // SAFETY: N3. The sink is the resolver's boxed answer list.
    let sink = unsafe { &*context.sink };
    sink.borrow_mut().push(make(context.query));
}

unsafe extern "C" fn on_address(
    _sd: ServiceRef,
    flags: u32,
    _interface: u32,
    error: i32,
    _hostname: *const c_char,
    address: *const u8,
    ttl: u32,
    context: *mut c_void,
) {
    // SAFETY: N3. DNS-SD passes a full `struct sockaddr` of its family, or
    // null, valid during the callback.
    let data = unsafe { socket_address(address) }.map_or(AnswerData::None, AnswerData::Address);
    // SAFETY: N3. `context` is the pointer `addresses` passed.
    unsafe {
        deliver(context, |query| Answer {
            query,
            error,
            add: flags & FLAGS_ADD != 0,
            more: flags & FLAGS_MORE_COMING != 0,
            ttl,
            // A negative answer's address is its family's, unspecified.
            data,
        });
    }
}

unsafe extern "C" fn on_record(
    _sd: ServiceRef,
    flags: u32,
    _interface: u32,
    error: i32,
    _fullname: *const c_char,
    rrtype: u16,
    _rrclass: u16,
    rdlen: u16,
    rdata: *const u8,
    ttl: u32,
    context: *mut c_void,
) {
    let data = if error == ERR_NO_ERROR && !rdata.is_null() {
        // SAFETY: N3. DNS-SD passes `rdlen` bytes of RDATA, valid during
        // the callback; they are copied here.
        let bytes = unsafe { core::slice::from_raw_parts(rdata, usize::from(rdlen)) };
        AnswerData::Record(rrtype, bytes.to_vec())
    } else {
        AnswerData::None
    };
    // SAFETY: N3. `context` is the pointer `records` passed.
    unsafe {
        deliver(context, |query| Answer {
            query,
            error,
            add: flags & FLAGS_ADD != 0,
            more: flags & FLAGS_MORE_COMING != 0,
            ttl,
            data,
        });
    }
}

/// Decode a `struct sockaddr_in` or `struct sockaddr_in6`.
///
/// # Safety
///
/// `address` must be null or point to a whole socket address of the
/// family it names (N3).
unsafe fn socket_address(address: *const u8) -> Option<IpAddr> {
    if address.is_null() {
        return None;
    }
    // SAFETY: N3. Every socket address starts with its length and family.
    let family = unsafe { core::slice::from_raw_parts(address, 2) }[1];
    let len = match family {
        AF_INET => 16,
        AF_INET6 => 28,
        _ => return None,
    };
    // SAFETY: N3. A socket address of that family is `len` bytes long.
    let bytes = unsafe { core::slice::from_raw_parts(address, len) };
    if family == AF_INET {
        return Some(IpAddr::V4(Ipv4Addr::new(
            bytes[4], bytes[5], bytes[6], bytes[7],
        )));
    }
    let octets: [u8; 16] = bytes[8..24].try_into().ok()?;
    Some(IpAddr::V6(Ipv6Addr::from(octets)))
}
