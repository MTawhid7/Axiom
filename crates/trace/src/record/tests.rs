use super::*;
use crate::category::{Category, Level};

const NAME: Name = Name::new(7);

fn header(timestamp: u64) -> Header {
    Header {
        category: Category::Style,
        name: NAME,
        timestamp,
    }
}

fn round_trip(words: &[u64]) -> Record {
    let (record, len) = decode(words).unwrap();
    assert_eq!(len, words.len());
    record
}

#[test]
fn spans_and_events_carry_up_to_four_fields() {
    let mut out = [0; FIELD_WORDS];
    let len = fields(
        &mut out,
        Kind::SpanBegin,
        Category::Style,
        NAME,
        10,
        &[1, 2, 3, 4, 5],
    );
    assert_eq!(len, 6, "the fifth field is dropped");
    let Record::SpanBegin(h, f) = round_trip(&out[..len]) else {
        panic!()
    };
    assert_eq!((h, f.as_slice()), (header(10), &[1, 2, 3, 4][..]));

    let len = fields(&mut out, Kind::Event, Category::Style, NAME, 11, &[]);
    assert_eq!(
        round_trip(&out[..len]),
        Record::Event(header(11), Fields::from_words(&[]))
    );
    let len = fields(&mut out, Kind::SpanEnd, Category::Style, NAME, 12, &[]);
    assert_eq!(round_trip(&out[..len]), Record::SpanEnd(header(12)));
}

#[test]
fn values_round_trip() {
    for (kind, expected) in [
        (Kind::Counter, Record::Counter(header(5), 99)),
        (Kind::Gauge, Record::Gauge(header(5), 99)),
        (Kind::Histogram, Record::Histogram(header(5), 99)),
        (Kind::FlowBegin, Record::FlowBegin(header(5), 99)),
        (Kind::FlowEnd, Record::FlowEnd(header(5), 99)),
    ] {
        assert_eq!(
            round_trip(&value(kind, Category::Style, NAME, 5, 99)),
            expected
        );
    }
    assert_eq!(
        round_trip(&dropped(8, 3)),
        Record::Dropped {
            timestamp: 8,
            count: 3
        }
    );
}

#[test]
fn logs_carry_text_cut_on_a_character_boundary() {
    let mut out = [0; MAX_WORDS];
    let len = log(
        &mut out,
        Level::Warn,
        Category::Style,
        NAME,
        3,
        "fallback font",
    );
    let Record::Log(h, level, text) = round_trip(&out[..len]) else {
        panic!()
    };
    assert_eq!(
        (h, level, text.as_str()),
        (header(3), Level::Warn, "fallback font")
    );

    // 254 ASCII bytes, then a two-byte character that would end at byte 256.
    let long = format!("{}é", "a".repeat(254));
    let len = log(&mut out, Level::Info, Category::Style, NAME, 3, &long);
    let Record::Log(_, _, text) = round_trip(&out[..len]) else {
        panic!()
    };
    assert_eq!(text.as_str(), "a".repeat(254));
}

#[test]
fn panics_carry_message_file_and_location() {
    let mut out = [0; MAX_WORDS];
    let len = panic(
        &mut out,
        4,
        "index out of range",
        "crates/dom/src/tree.rs",
        120,
        9,
    );
    let Record::Panic {
        timestamp,
        message,
        file,
        line,
        column,
    } = round_trip(&out[..len])
    else {
        panic!()
    };
    assert_eq!(
        (timestamp, message.as_str(), file.as_str(), line, column),
        (4, "index out of range", "crates/dom/src/tree.rs", 120, 9)
    );
    let longest = panic(
        &mut out,
        4,
        &"m".repeat(1000),
        &"f".repeat(1000),
        u32::MAX,
        u32::MAX,
    );
    assert!(longest <= MAX_WORDS);
    assert!(matches!(
        round_trip(&out[..longest]),
        Record::Panic {
            line: u32::MAX,
            column: u32::MAX,
            ..
        }
    ));
}

#[test]
fn records_iterate_in_order_and_stop_at_the_first_error() {
    let mut words = Vec::new();
    words.extend(value(Kind::Counter, Category::Style, NAME, 1, 1));
    words.extend(dropped(2, 5));
    words.push(0); // Not a record.
    words.extend(value(Kind::Counter, Category::Style, NAME, 3, 1));
    let decoded: Vec<_> = records(&words).collect();
    assert_eq!(decoded.len(), 3);
    assert!(decoded[0].is_ok() && decoded[1].is_ok());
    assert_eq!(decoded[2], Err(DecodeError::UnknownKind(0)));
}

fn with_header(word: u64, rest: &[u64]) -> Vec<u64> {
    let mut words = vec![word, 1];
    words.extend_from_slice(rest);
    words
}

#[test]
fn the_decoder_refuses_what_no_correct_writer_writes() {
    let good = header_word(Kind::Counter, Category::Style, NAME, 3, 0);
    let cases: Vec<(Vec<u64>, DecodeError)> = vec![
        (vec![], DecodeError::Truncated),
        (with_header(good, &[]), DecodeError::Truncated),
        (
            with_header(good & !0xff | 0x0c, &[0]),
            DecodeError::UnknownKind(0x0c),
        ),
        (
            with_header(good & !0xff00 | 0x0e << 8, &[0]),
            DecodeError::UnknownCategory(0x0e),
        ),
        (with_header(good | 1 << 48, &[0]), DecodeError::ReservedBits),
        (with_header(good | 2 << 40, &[0]), DecodeError::BadLevel(2)),
        (
            with_header(
                header_word(Kind::Counter, Category::Style, NAME, 4, 0),
                &[0, 0],
            ),
            DecodeError::BadLength(Kind::Counter, 4),
        ),
        (
            with_header(header_word(Kind::Counter, Category::Style, NAME, 1, 0), &[]),
            DecodeError::BadLength(Kind::Counter, 1),
        ),
        (
            with_header(
                header_word(Kind::SpanEnd, Category::Style, NAME, 3, 0),
                &[0],
            ),
            DecodeError::BadLength(Kind::SpanEnd, 3),
        ),
        (
            with_header(
                header_word(Kind::Event, Category::Style, NAME, 7, 0),
                &[0; 5],
            ),
            DecodeError::BadLength(Kind::Event, 7),
        ),
        (
            with_header(
                header_word(Kind::Dropped, Category::Style, Name::RECORDER, 3, 0),
                &[1],
            ),
            DecodeError::NotTheRecorders(Kind::Dropped),
        ),
        (
            with_header(header_word(Kind::Log, Category::Style, NAME, 3, 4), &[0]),
            DecodeError::BadLevel(4),
        ),
    ];
    for (words, expected) in cases {
        assert_eq!(
            decode(&words).map(|(_, len)| len),
            Err(expected),
            "{words:x?}"
        );
    }
}

#[test]
fn the_decoder_refuses_bad_text() {
    let log = |payload: &[u64]| {
        with_header(
            header_word(Kind::Log, Category::Style, NAME, 2 + payload.len(), 1),
            payload,
        )
    };
    let refused = Err(DecodeError::BadText(Kind::Log));
    // Longer than the bound; runs past the record; nonzero padding; not UTF-8.
    assert_eq!(decode(&log(&[256, 0])).map(|(_, len)| len), refused);
    assert_eq!(decode(&log(&[9, 0])).map(|(_, len)| len), refused);
    assert_eq!(
        decode(&log(&[1, u64::from(b'a') | 1 << 8])).map(|(_, len)| len),
        refused
    );
    assert_eq!(decode(&log(&[1, 0xff])).map(|(_, len)| len), refused);
    // A valid text that leaves words over is the wrong length, not bad text.
    assert_eq!(
        decode(&log(&[1, u64::from(b'a'), 0])).map(|(_, len)| len),
        Err(DecodeError::BadLength(Kind::Log, 5))
    );
}
