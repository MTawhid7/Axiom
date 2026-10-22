//! The panic hook records the message and location into the panicking
//! thread's ring (OB9, OB27). In its own binary, since the hook is
//! process-wide.
#![forbid(unsafe_code)]

use trace::{OwnedRing, Reader, Record};

#[test]
fn a_panic_leaves_its_message_and_location_in_the_ring() {
    let ring: &'static OwnedRing =
        Box::leak(Box::new(OwnedRing::new(1024).expect("a power of two")));
    trace::install(ring.storage().expect("a valid ring"));
    trace::install_panic_hook();
    let index = std::hint::black_box(7);
    let line = line!() + 1;
    let result = std::panic::catch_unwind(|| panic!("index {index} out of range"));
    assert!(result.is_err());

    let mut words = Vec::new();
    Reader::new(ring.storage().expect("a valid ring"))
        .drain(&mut words)
        .expect("a correct writer");
    let records: Vec<_> = trace::records(&words)
        .collect::<Result<_, _>>()
        .expect("records decode");
    match records.as_slice() {
        [
            Record::Panic {
                message,
                file,
                line: at,
                ..
            },
        ] => {
            assert_eq!(message.as_str(), "index 7 out of range");
            assert!(file.as_str().ends_with("tests/panic.rs"), "{file:?}");
            assert_eq!(*at, line);
        }
        other => panic!("expected one panic record, got {other:?}"),
    }
}
