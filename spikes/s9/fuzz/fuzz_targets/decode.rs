//! S9-Q1: the checked decoder, with a round trip as its oracle.
#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use s9_decoder::{decode, encode};

fuzz_target!(|data: &[u8]| {
    if let Ok(records) = decode(data) {
        assert_eq!(decode(&encode(&records)), Ok(records));
    }
});
