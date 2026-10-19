//! Network framework's TLS over TCP (`Network/connection.h`, `parameters.h`,
//! `endpoint.h`, `tls_options.h`, `error.h`, `nw_object.h`;
//! `Security/SecProtocolOptions.h`): the stack S7-Q2 compares with rustls.
//! It verifies certificates itself, with the system's trust store. It runs
//! threads of its own and calls handlers on a dispatch queue, so the
//! calling thread only waits for the result (against NW1).

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CString;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use block2::{Block, RcBlock};
use dispatch2::{DispatchData, DispatchQueue};

/// Any Network framework or `sec_protocol` object: `nw_connection_t` and
/// the others are pointers to opaque objects in C.
type Object = *mut c_void;
/// `nw_parameters_configure_protocol_block_t` (`parameters.h`).
type ConfigureBlock = Block<dyn Fn(Object)>;
/// `nw_connection_state_changed_handler_t` (`connection.h`).
type StateBlock = Block<dyn Fn(c_int, Object)>;
/// `nw_connection_send_completion_t` (`connection.h`).
type SentBlock = Block<dyn Fn(Object)>;
/// `nw_connection_receive_completion_t` (`connection.h`). C's `bool` is a
/// byte, passed and returned as one; block2 encodes it as `u8` (N2).
type ReceivedBlock = Block<dyn Fn(Object, Object, u8, Object)>;
/// `dispatch_data_applier_t` (`dispatch/data.h`), returning a C `bool`.
type ApplierBlock = Block<dyn Fn(Object, usize, *const c_void, usize) -> u8>;

// `nw_connection_state_t` (`connection.h`) and `nw_error_domain_t`
// (`error.h`) (N2).
const STATE_WAITING: c_int = 1;
const STATE_READY: c_int = 3;
const STATE_FAILED: c_int = 4;
const STATE_CANCELLED: c_int = 5;

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {
    static _nw_parameters_configure_protocol_default_configuration: *const ConfigureBlock;
    static _nw_content_context_default_message: Object;
    fn nw_parameters_create_secure_tcp(
        configure_tls: *const ConfigureBlock,
        configure_tcp: *const ConfigureBlock,
    ) -> Object;
    fn nw_endpoint_create_host(hostname: *const c_char, port: *const c_char) -> Object;
    fn nw_connection_create(endpoint: Object, parameters: Object) -> Object;
    fn nw_connection_set_queue(connection: Object, queue: &DispatchQueue);
    fn nw_connection_set_state_changed_handler(connection: Object, handler: *const StateBlock);
    fn nw_connection_start(connection: Object);
    fn nw_connection_send(
        connection: Object,
        content: &DispatchData,
        context: Object,
        is_complete: bool,
        completion: *const SentBlock,
    );
    fn nw_connection_receive(
        connection: Object,
        minimum_incomplete_length: u32,
        maximum_length: u32,
        completion: *const ReceivedBlock,
    );
    fn nw_connection_cancel(connection: Object);
    fn nw_tls_copy_sec_protocol_options(options: Object) -> Object;
    fn nw_connection_copy_protocol_metadata(connection: Object, definition: Object) -> Object;
    fn nw_protocol_copy_tls_definition() -> Object;
    fn nw_tls_copy_sec_protocol_metadata(metadata: Object) -> Object;
    fn nw_error_get_error_domain(error: Object) -> c_int;
    fn nw_error_get_error_code(error: Object) -> c_int;
    fn nw_release(object: Object);
}

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn sec_protocol_options_set_tls_resumption_enabled(options: Object, enabled: bool);
    fn sec_protocol_options_set_tls_tickets_enabled(options: Object, enabled: bool);
    fn sec_release(object: Object);
    fn sec_protocol_metadata_get_negotiated_tls_protocol_version(metadata: Object) -> u16;
    fn sec_protocol_metadata_get_negotiated_tls_ciphersuite(metadata: Object) -> u16;
}

// libdispatch, in libSystem: `dispatch/data.h` (N2).
unsafe extern "C" {
    fn dispatch_data_apply(data: Object, applier: *const ApplierBlock) -> bool;
}

/// One request's result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fetched {
    /// From start to the ready state: name resolution, TCP, TLS and trust.
    pub(crate) ready: Option<Duration>,
    pub(crate) first_byte: Option<Duration>,
    pub(crate) done: Option<Duration>,
    /// Bytes received: the response's head and body.
    pub(crate) bytes: u64,
    /// Why it failed: a state, or an error's domain and code.
    pub(crate) error: Option<String>,
    /// The negotiated TLS version and cipher suite (`tls_protocol_version_t`,
    /// `tls_ciphersuite_t`: IANA's code points).
    pub(crate) negotiated: Option<(u16, u16)>,
}

/// Consumes received bytes on the queue's thread; returns true once the
/// response is complete.
pub(crate) type Sink = Box<dyn FnMut(&[u8]) -> bool + Send>;

struct Inner {
    start: Instant,
    result: Fetched,
    sink: Sink,
    /// The sink reported the response complete.
    complete: bool,
    finished: bool,
    cancelled: bool,
}

struct Shared {
    inner: Mutex<Inner>,
    changed: Condvar,
}

/// A non-owning pointer to a connection, for handlers. The connection
/// outlives every handler call (N6).
#[derive(Clone, Copy)]
struct ConnectionRef(Object);

// SAFETY: N6. Handlers use the pointer only on the connection's queue,
// while `Connection` keeps the object alive, and Network framework's objects
// are reference-counted atomically.
unsafe impl Send for ConnectionRef {}
// SAFETY: as above.
unsafe impl Sync for ConnectionRef {}

/// The owning reference to a connection, released once (N6).
struct Connection(Object);

impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY: N6. The only owning reference; dropped after the state
        // handler reported `cancelled`, or before the connection started.
        unsafe { nw_release(self.0) };
    }
}

/// TLS-over-TCP parameters, released once (N6). Connections made with the
/// same parameters can resume each other's TLS sessions.
pub(crate) struct Parameters(Object);

impl Parameters {
    /// TLS over TCP, with session resumption and session tickets (which
    /// TLS 1.3's resumption uses) enabled or not.
    pub(crate) fn new(resumption: bool) -> Option<Self> {
        let configure_tls = RcBlock::new(move |options: Object| {
            // SAFETY: N6. `options` is the TLS options object the framework
            // passes; the copied `sec_protocol_options` is released once.
            let sec = unsafe { nw_tls_copy_sec_protocol_options(options) };
            if !sec.is_null() {
                // SAFETY: N6. `sec` is live until the release below.
                unsafe { sec_protocol_options_set_tls_resumption_enabled(sec, resumption) };
                // SAFETY: as above.
                unsafe { sec_protocol_options_set_tls_tickets_enabled(sec, resumption) };
                // SAFETY: N6. Released once, by its only owner.
                unsafe { sec_release(sec) };
            }
        });
        // SAFETY: N9 (the default configuration block); N6 (the result is
        // owned by the returned value).
        let parameters = unsafe {
            nw_parameters_create_secure_tcp(
                RcBlock::as_ptr(&configure_tls),
                _nw_parameters_configure_protocol_default_configuration,
            )
        };
        (!parameters.is_null()).then_some(Self(parameters))
    }
}

impl Drop for Parameters {
    fn drop(&mut self) {
        // SAFETY: N6. Released once, by its only owner; connections hold
        // their own references.
        unsafe { nw_release(self.0) };
    }
}

/// Fetch over TLS from `host`:`port`, with `parameters`: send `request`
/// once ready, and pass what arrives to `sink` until it reports the
/// response complete or the connection ends; then cancel. Waits at most
/// `timeout`.
pub(crate) fn fetch(
    queue: &DispatchQueue,
    parameters: &Parameters,
    host: &str,
    port: u16,
    request: &[u8],
    sink: Sink,
    timeout: Duration,
) -> Fetched {
    let (Ok(c_host), Ok(c_port)) = (CString::new(host), CString::new(port.to_string())) else {
        return failed("a name contains a NUL byte");
    };
    // SAFETY: N1 (both strings live through the call); N6.
    let endpoint = unsafe { nw_endpoint_create_host(c_host.as_ptr(), c_port.as_ptr()) };
    if endpoint.is_null() {
        return failed("nw_endpoint_create_host returned null");
    }
    // SAFETY: N6. The connection retains the endpoint and the parameters;
    // the endpoint's own reference is released right after.
    let connection = Connection(unsafe { nw_connection_create(endpoint, parameters.0) });
    // SAFETY: N6. Released once, by its only owner.
    unsafe { nw_release(endpoint) };
    if connection.0.is_null() {
        return failed("nw_connection_create returned null");
    }

    let shared = Arc::new(Shared {
        inner: Mutex::new(Inner {
            start: Instant::now(),
            result: Fetched::default(),
            sink,
            complete: false,
            finished: false,
            cancelled: false,
        }),
        changed: Condvar::new(),
    });
    let handler = {
        let shared = Arc::clone(&shared);
        let conn = ConnectionRef(connection.0);
        let request = request.to_vec();
        RcBlock::new(move |state: c_int, error: Object| {
            on_state(conn, &shared, &request, state, error);
        })
    };
    // SAFETY: N6. The queue outlives the connection (the caller's borrow);
    // the handler block is copied by the framework; then it starts.
    unsafe { nw_connection_set_queue(connection.0, queue) };
    // SAFETY: as above.
    unsafe { nw_connection_set_state_changed_handler(connection.0, RcBlock::as_ptr(&handler)) };
    // SAFETY: as above.
    unsafe { nw_connection_start(connection.0) };

    let deadline = Instant::now() + timeout;
    let mut inner = lock(&shared);
    let mut cancelled_here = false;
    while !inner.cancelled {
        let now = Instant::now();
        if now >= deadline && !cancelled_here {
            inner
                .result
                .error
                .get_or_insert_with(|| "timed out".to_owned());
            // SAFETY: N6. The connection is alive until `cancelled`.
            unsafe { nw_connection_cancel(connection.0) };
            cancelled_here = true;
        }
        let wait = deadline
            .saturating_duration_since(now)
            .max(Duration::from_millis(100));
        inner = shared
            .changed
            .wait_timeout(inner, wait)
            .map_or_else(|e| e.into_inner().0, |(guard, _)| guard);
    }
    let result = inner.result.clone();
    drop(inner);
    // `connection` is released here, after `cancelled` (N6).
    drop(connection);
    result
}

fn on_state(
    conn: ConnectionRef,
    shared: &Arc<Shared>,
    request: &[u8],
    state: c_int,
    error: Object,
) {
    match state {
        STATE_READY => {
            let negotiated = negotiated(conn);
            {
                let mut inner = lock(shared);
                inner.result.ready = Some(inner.start.elapsed());
                inner.result.negotiated = negotiated;
            }
            let content = DispatchData::from_bytes(request);
            let sent = RcBlock::new(|_error: Object| {});
            // SAFETY: N6 (the connection is alive; the default context is a
            // framework global, N9); the data is retained by the framework.
            unsafe {
                nw_connection_send(
                    conn.0,
                    &content,
                    _nw_content_context_default_message,
                    true,
                    RcBlock::as_ptr(&sent),
                );
            }
            receive(conn, Arc::clone(shared));
        }
        STATE_WAITING | STATE_FAILED => {
            finish(conn, shared, Some(describe(state, error)));
        }
        STATE_CANCELLED => {
            let mut inner = lock(shared);
            inner.cancelled = true;
            shared.changed.notify_all();
        }
        _ => {}
    }
}

/// The connection's negotiated TLS version and cipher suite.
fn negotiated(conn: ConnectionRef) -> Option<(u16, u16)> {
    // SAFETY: N6. Each object returned retained is released once, below.
    let definition = unsafe { nw_protocol_copy_tls_definition() };
    if definition.is_null() {
        return None;
    }
    // SAFETY: N6. The connection is alive until `cancelled`.
    let metadata = unsafe { nw_connection_copy_protocol_metadata(conn.0, definition) };
    // SAFETY: N6. Released once, by its only owner.
    unsafe { nw_release(definition) };
    if metadata.is_null() {
        return None;
    }
    // SAFETY: N6. `metadata` is live until its release below.
    let sec = unsafe { nw_tls_copy_sec_protocol_metadata(metadata) };
    // SAFETY: N6. Released once, by its only owner.
    unsafe { nw_release(metadata) };
    if sec.is_null() {
        return None;
    }
    // SAFETY: N6. `sec` is live until its release below.
    let version = unsafe { sec_protocol_metadata_get_negotiated_tls_protocol_version(sec) };
    // SAFETY: as above.
    let suite = unsafe { sec_protocol_metadata_get_negotiated_tls_ciphersuite(sec) };
    // SAFETY: N6. Released once, by its only owner.
    unsafe { sec_release(sec) };
    Some((version, suite))
}

/// Ask for the next bytes; the completion asks again until done.
fn receive(conn: ConnectionRef, shared: Arc<Shared>) {
    let completion = RcBlock::new(
        move |content: Object, _context: Object, complete: u8, error: Object| {
            let mut done = false;
            if !content.is_null() {
                done = apply(content, &shared);
            }
            if done {
                finish(conn, &shared, None);
            } else if !error.is_null() {
                finish(conn, &shared, Some(describe(STATE_FAILED, error)));
            } else if complete != 0 {
                finish(conn, &shared, None);
            } else {
                receive(conn, Arc::clone(&shared));
            }
        },
    );
    // SAFETY: N6. The connection is alive until `cancelled`, the final
    // callback; the block is copied by the framework.
    unsafe { nw_connection_receive(conn.0, 1, 1 << 20, RcBlock::as_ptr(&completion)) };
}

/// Pass `content`'s regions to the sink. Returns true once it reports the
/// response complete.
fn apply(content: Object, shared: &Arc<Shared>) -> bool {
    {
        let mut inner = lock(shared);
        if inner.finished {
            return true;
        }
        if inner.result.first_byte.is_none() {
            inner.result.first_byte = Some(inner.start.elapsed());
        }
    }
    let state = Arc::clone(shared);
    let applier = RcBlock::new(
        move |_region: Object, _offset: usize, buffer: *const c_void, size: usize| -> u8 {
            if buffer.is_null() || size == 0 {
                return 1;
            }
            // SAFETY: N6. `dispatch_data_apply` passes a region of `size`
            // readable bytes, valid during this call.
            let bytes = unsafe { core::slice::from_raw_parts(buffer.cast::<u8>(), size) };
            let mut inner = lock(&state);
            inner.result.bytes += u64::try_from(size).unwrap_or(0);
            if (inner.sink)(bytes) {
                inner.complete = true;
                return 0;
            }
            1
        },
    );
    // SAFETY: N6. `content` is the live data object the handler received;
    // the applier runs synchronously, before this returns.
    unsafe { dispatch_data_apply(content, RcBlock::as_ptr(&applier)) };
    lock(shared).complete
}

/// Record the end (with `error`, if any) and cancel the connection.
fn finish(conn: ConnectionRef, shared: &Arc<Shared>, error: Option<String>) {
    let mut inner = lock(shared);
    if inner.finished {
        return;
    }
    inner.finished = true;
    inner.result.done = Some(inner.start.elapsed());
    if let Some(error) = error {
        inner.result.error.get_or_insert(error);
    }
    drop(inner);
    // SAFETY: N6. The connection is alive until `cancelled`, which this
    // starts.
    unsafe { nw_connection_cancel(conn.0) };
}

fn describe(state: c_int, error: Object) -> String {
    let state = match state {
        STATE_WAITING => "waiting",
        STATE_FAILED => "failed",
        _ => "state",
    };
    if error.is_null() {
        return state.to_owned();
    }
    // SAFETY: N6. `error` is the live error object the handler received.
    let domain = unsafe { nw_error_get_error_domain(error) };
    // SAFETY: as above.
    let code = unsafe { nw_error_get_error_code(error) };
    let domain = match domain {
        1 => "posix",
        2 => "dns",
        3 => "tls",
        _ => "other",
    };
    format!("{state}: {domain} {code}")
}

fn failed(error: &str) -> Fetched {
    Fetched {
        error: Some(error.to_owned()),
        ..Fetched::default()
    }
}

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, Inner> {
    shared
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
