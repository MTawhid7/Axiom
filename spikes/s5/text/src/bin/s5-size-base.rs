//! # Spike S5, S5-Q4: executable-size baseline
//!
//! Reads a file named on the command line and prints its length: just the
//! standard library's I/O, against which the other `s5-size-*` probes measure
//! what their component adds.
#![forbid(unsafe_code)]

fn main() {
    let path = std::env::args().nth(1).unwrap_or_default();
    let len = std::fs::read(&path).map_or(0, |data| data.len());
    println!("{len}");
}
