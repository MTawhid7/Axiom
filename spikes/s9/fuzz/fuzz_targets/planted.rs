//! S9-Q1: the decoder with its planted bug, which the fuzzer must find.
#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = s9_decoder::decode_planted(data);
});
