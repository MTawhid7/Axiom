//! Each thread's recorder (OB5): the writer of its ring, in a thread-local
//! slot, and the functions that encode a record on the stack and write it.
//! A thread with no ring records nothing, at the cost of the slot's check.
//!
//! The detailed tier's call sites check [`crate::enabled`] first (OB7); the
//! always-on tier's call sites don't. How call sites read, as macros with
//! declared names and metrics, is M1-CP3's interface note.

use std::cell::RefCell;

use crate::category::{Category, Level, MAX_LEVEL};
use crate::clock;
use crate::record::{self, FIELD_WORDS, Kind, MAX_WORDS, Name};
use crate::ring::{RingStorage, Writer};

thread_local! {
    static WRITER: RefCell<Option<Writer<'static>>> = const { RefCell::new(None) };
}

/// Makes `ring` this thread's ring, replacing any it had.
pub fn install(ring: RingStorage<'static>) {
    let _ = WRITER.try_with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut() {
            *slot = Some(Writer::new(ring));
        }
    });
}

/// Stops this thread's recording, returning the records its writer had
/// dropped and not yet reported.
pub fn uninstall() -> Option<u64> {
    WRITER
        .try_with(|slot| {
            slot.try_borrow_mut()
                .ok()
                .and_then(|mut slot| slot.take())
                .map(|writer| writer.dropped())
        })
        .ok()
        .flatten()
}

/// Writes `record` into this thread's ring, if it has one. A record written
/// while another is being written on the same thread, from a panic hook say,
/// is dropped rather than waited for.
fn write(record: &[u64]) {
    let _ = WRITER.try_with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut()
            && let Some(writer) = slot.as_mut()
        {
            writer.write(record);
        }
    });
}

fn write_fields(kind: Kind, category: Category, name: Name, fields: &[u64]) {
    let mut out = [0; FIELD_WORDS];
    let len = record::fields(&mut out, kind, category, name, clock::now(), fields);
    if let Some(words) = out.get(..len) {
        write(words);
    }
}

fn write_value(kind: Kind, category: Category, name: Name, value: u64) {
    write(&record::value(kind, category, name, clock::now(), value));
}

/// Work named `name` begins on this thread, with up to four fields (OB1).
#[inline]
pub fn span_begin(category: Category, name: Name, fields: &[u64]) {
    write_fields(Kind::SpanBegin, category, name, fields);
}

/// The innermost span this thread began ends.
#[inline]
pub fn span_end(category: Category, name: Name) {
    write_fields(Kind::SpanEnd, category, name, &[]);
}

/// A moment, with up to four fields.
#[inline]
pub fn event(category: Category, name: Name, fields: &[u64]) {
    write_fields(Kind::Event, category, name, fields);
}

/// A counter grows by `delta` (OB8).
#[inline]
pub fn counter(category: Category, name: Name, delta: u64) {
    write_value(Kind::Counter, category, name, delta);
}

/// A gauge holds `value` (OB8).
#[inline]
pub fn gauge(category: Category, name: Name, value: u64) {
    write_value(Kind::Gauge, category, name, value);
}

/// A histogram's sample (OB8).
#[inline]
pub fn histogram(category: Category, name: Name, sample: u64) {
    write_value(Kind::Histogram, category, name, sample);
}

/// A message leaves on its channel: a flow begins, keyed by what IPC already
/// carries (OB4).
#[inline]
pub fn flow_begin(category: Category, name: Name, key: u64) {
    write_value(Kind::FlowBegin, category, name, key);
}

/// The message arrives: its flow ends.
#[inline]
pub fn flow_end(category: Category, name: Name, key: u64) {
    write_value(Kind::FlowEnd, category, name, key);
}

/// A log at `level`, its text cut at 255 bytes (OB3). Levels past
/// [`MAX_LEVEL`] cost nothing.
#[inline]
pub fn log(level: Level, category: Category, name: Name, text: &str) {
    if level > MAX_LEVEL {
        return;
    }
    let mut out = [0; MAX_WORDS];
    let len = record::log(&mut out, level, category, name, clock::now(), text);
    if let Some(words) = out.get(..len) {
        write(words);
    }
}

/// A panic's record, from the hook (OB9).
pub(crate) fn panic(message: &str, file: &str, line: u32, column: u32) {
    let mut out = [0; MAX_WORDS];
    let len = record::panic(&mut out, clock::now(), message, file, line, column);
    if let Some(words) = out.get(..len) {
        write(words);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        counter, event, flow_begin, flow_end, gauge, histogram, install, log, span_begin, span_end,
        uninstall,
    };
    use crate::category::{Category, Level};
    use crate::record::{Name, Record, records};
    use crate::ring::{OwnedRing, Reader};

    const NAME: Name = Name::new(3);

    fn leaked_ring() -> &'static OwnedRing {
        Box::leak(Box::new(OwnedRing::new(1024).unwrap()))
    }

    #[test]
    fn each_function_writes_its_record_into_this_threads_ring() {
        let ring = leaked_ring();
        install(ring.storage().unwrap());
        span_begin(Category::Layout, NAME, &[1, 2]);
        event(Category::Layout, NAME, &[3]);
        counter(Category::Layout, NAME, 4);
        gauge(Category::Layout, NAME, 5);
        histogram(Category::Layout, NAME, 6);
        flow_begin(Category::Ipc, NAME, 7);
        flow_end(Category::Ipc, NAME, 7);
        log(Level::Warn, Category::Layout, NAME, "slow");
        span_end(Category::Layout, NAME);
        assert_eq!(uninstall(), Some(0));
        counter(Category::Layout, NAME, 99); // No ring now: recorded nowhere.

        let mut words = Vec::new();
        Reader::new(ring.storage().unwrap())
            .drain(&mut words)
            .unwrap();
        let kinds: Vec<_> = records(&words)
            .map(|r| match r.unwrap() {
                Record::SpanBegin(_, f) => format!("begin {:?}", f.as_slice()),
                Record::Event(_, f) => format!("event {:?}", f.as_slice()),
                Record::Counter(_, v) => format!("counter {v}"),
                Record::Gauge(_, v) => format!("gauge {v}"),
                Record::Histogram(_, v) => format!("histogram {v}"),
                Record::FlowBegin(_, v) => format!("flow begin {v}"),
                Record::FlowEnd(_, v) => format!("flow end {v}"),
                Record::Log(_, level, text) => format!("{} {}", level.name(), text.as_str()),
                Record::SpanEnd(_) => "end".to_owned(),
                other @ (Record::Dropped { .. } | Record::Panic { .. }) => format!("{other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "begin [1, 2]",
                "event [3]",
                "counter 4",
                "gauge 5",
                "histogram 6",
                "flow begin 7",
                "flow end 7",
                "warn slow",
                "end"
            ]
        );
    }

    #[test]
    fn a_thread_without_a_ring_records_nothing() {
        assert_eq!(uninstall(), None);
        span_begin(Category::Layout, NAME, &[]);
        assert_eq!(uninstall(), None);
    }
}
