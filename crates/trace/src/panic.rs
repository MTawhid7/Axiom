//! The panic hook (OB9): it records the panic's message and location into the
//! panicking thread's ring, so that the browser's report has them (OB17), and
//! then runs the hook that was installed before it. With `panic = "abort"`
//! (RV1-16) the process then aborts.
//!
//! It allocates nothing: the runtime hands it the message already formatted,
//! and the record is encoded on the stack.

use std::panic::{self, PanicHookInfo};

use crate::recorder;

/// Installs the hook, keeping the previous one to run after it. The process's
/// assembly installs it once, after its clock and its threads' rings.
pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        record(info);
        previous(info);
    }));
}

fn record(info: &PanicHookInfo<'_>) {
    let message = info
        .payload_as_str()
        .unwrap_or("a panic without a text message");
    let (file, line, column) = info.location().map_or(("", 0, 0), |location| {
        (location.file(), location.line(), location.column())
    });
    recorder::panic(message, file, line, column);
}
