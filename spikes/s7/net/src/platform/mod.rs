//! The platform layer: every foreign call this spike makes, into Security,
//! Network framework, DNS-SD, kqueue, Mach, BSD sockets and libproc. It is the
//! only place in the crate where `unsafe` is allowed (`tools/unsafe-policy.txt`).
//!
//! Hand-written declarations were checked against the macOS 27 SDK headers
//! of the Command Line Tools, named beside each. Each `unsafe` block cites one
//! of these invariants:
//!
//! - **N1, borrowed memory.** Every pointer passed to C points into memory
//!   that Rust keeps alive and unmoved for the whole call, at least as long
//!   as the length passed with it: byte slices, [`CString`]s, and
//!   out-parameters, which are live, aligned locals read only after the call
//!   returns.
//! - **N2, declarations.** Every hand-written declaration matches the SDK
//!   header named beside it, in argument types, order and return type, and so
//!   does every constant and struct layout copied from a header. Layouts are
//!   also checked by size assertions.
//! - **N3, DNS-SD contexts.** A callback's context points to a boxed value
//!   that the resolver owns, and that outlives the query's `DNSServiceRef`: the
//!   query's ref is deallocated before its box is freed, and every query's
//!   ref before the shared connection's. DNS-SD calls a callback only from
//!   `DNSServiceProcessResult` on the shared connection, which only the
//!   resolver's owner calls, so a context is never used concurrently or after
//!   it is freed. Names and addresses a callback receives are read during the
//!   callback only.
//! - **N4, CoreFoundation and Security objects.** Objects returned by Create
//!   and Copy functions are owned by one `CFRetained`, which releases them
//!   once. Each argument is of the type the function documents (an array of
//!   `SecCertificate`, a `SecPolicy`, `CFData`), so the `&CFType` upcasts that
//!   objc2-security's signatures ask for are sound.
//! - **N5, kevent lists.** The change and event lists passed to `kevent64` are
//!   live slices of `kevent64_s`, their counts the slices' lengths. A Mach
//!   receive buffer registered with `MACH_RCV_MSG` (its address in `ext[0]`,
//!   its length in `ext[1]`) belongs to the poller, which closes its kqueue,
//!   removing the registration, before the buffer is freed. The kernel writes
//!   it only while delivering events to a `kevent64` call on that kqueue, and
//!   only the poller's owner makes such calls, through `&mut self`.
//! - **N6, Network framework objects and blocks.** Each object that a
//!   `NW_RETURNS_RETAINED` function returns is owned by one Rust value, which
//!   calls `nw_release` once. A started connection is released only after its
//!   state handler has reported `cancelled`: `connection.h` says that "Once
//!   this update is delivered, the caller may clean up any associated memory
//!   or objects". Handlers reach the connection through a non-owning pointer,
//!   valid until then. Blocks are `RcBlock`s, which the framework copies, and
//!   they capture only `Send + Sync` values, since the framework calls them on
//!   its queue's thread. A received `dispatch_data_t` is read only inside its
//!   handler, through `dispatch_data_apply`, whose regions are valid only
//!   during each applier call.
//! - **N7, descriptors.** A raw descriptor passed to C belongs to a live owner
//!   (an `OwnedFd`, `TcpStream` or `UdpSocket`, or DNS-SD's connection) for the
//!   whole call. DNS-SD's socket is removed from the kqueue before its
//!   connection is deallocated, as `dns_sd.h` asks.
//! - **N8, Mach rights.** Each port right this crate allocates is owned by one
//!   value, whose `Drop` releases it once (S4's X4). A message sent carries
//!   only a copy of a send right its sender owns, and its buffer is built as
//!   bytes, 8-byte aligned, its size a multiple of 4.
//! - **N9, framework globals.** Extern statics (`mach_task_self_`, Network
//!   framework's default configuration block and default message context) are
//!   initialized by their libraries before `main`, and only read.
//!
//! [`CString`]: std::ffi::CString
#![allow(unsafe_code)]

pub(crate) mod dnssd;
pub(crate) mod kq;
pub(crate) mod mach;
pub(crate) mod nw;
pub(crate) mod sock;
pub(crate) mod trust;
pub(crate) mod usage;
