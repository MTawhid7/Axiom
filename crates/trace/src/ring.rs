//! A single-producer, single-consumer ring of 64-bit words (OB5, IR7), over
//! storage its owner provides (CR9): shared memory in a helper, a file-backed
//! mapping in the browser, an [`OwnedRing`] in tests.
//!
//! Positions count words written and read since the ring began; a position's
//! slot is the position modulo the capacity, a power of two. The writer stores
//! a record's words, then releases its position past them; the reader
//! acquires that position, copies the words out, then releases its own. So
//! the writer never overwrites words the reader hasn't copied, and the
//! reader never reads words the writer hasn't published.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::record::{self, MAX_WORDS};

/// A ring's memory: its words and the two positions. The positions may live
/// apart from the words, as a region keeps the reader's in a page the writer
/// only reads (OB10).
#[derive(Debug, Clone, Copy)]
pub struct RingStorage<'a> {
    words: &'a [AtomicU64],
    write: &'a AtomicU64,
    read: &'a AtomicU64,
}

impl<'a> RingStorage<'a> {
    /// A ring over `words`, whose length must be a power of two that holds
    /// two of the longest records.
    pub fn new(
        words: &'a [AtomicU64],
        write: &'a AtomicU64,
        read: &'a AtomicU64,
    ) -> Result<Self, RingError> {
        if words.len().is_power_of_two() && words.len() >= MAX_WORDS.saturating_mul(2) {
            Ok(Self { words, write, read })
        } else {
            Err(RingError::Capacity(words.len()))
        }
    }

    fn capacity(&self) -> u64 {
        u64::try_from(self.words.len()).unwrap_or(0)
    }

    fn slot(&self, position: u64) -> Option<&AtomicU64> {
        let index = position & self.capacity().wrapping_sub(1);
        usize::try_from(index)
            .ok()
            .and_then(|index| self.words.get(index))
    }
}

/// A ring in ordinary memory, for tests and for a process's own threads.
#[derive(Debug)]
pub struct OwnedRing {
    words: Box<[AtomicU64]>,
    write: AtomicU64,
    read: AtomicU64,
}

impl OwnedRing {
    /// A ring of `words` words, a power of two of at least 128.
    pub fn new(words: usize) -> Result<Self, RingError> {
        let ring = Self {
            words: (0..words).map(|_| AtomicU64::new(0)).collect(),
            write: AtomicU64::new(0),
            read: AtomicU64::new(0),
        };
        ring.storage().map(|_| ())?;
        Ok(ring)
    }

    /// The ring's storage, to make its writer and reader from.
    pub fn storage(&self) -> Result<RingStorage<'_>, RingError> {
        RingStorage::new(&self.words, &self.write, &self.read)
    }
}

/// A ring's one writer: the thread that records into it.
#[derive(Debug)]
pub struct Writer<'a> {
    ring: RingStorage<'a>,
    position: u64,
    read_seen: u64,
    dropped: u64,
}

impl<'a> Writer<'a> {
    /// The writer of `ring`, continuing from its published position.
    #[must_use]
    pub fn new(ring: RingStorage<'a>) -> Self {
        let position = ring.write.load(Ordering::Relaxed);
        Self {
            ring,
            position,
            read_seen: ring.read.load(Ordering::Acquire),
            dropped: 0,
        }
    }

    /// Writes one record and publishes it, or drops it if the ring has no
    /// room, counting the drop. The first record that fits after drops is
    /// preceded by a drop record with their count (OB5). Never waits.
    pub fn write(&mut self, record: &[u64]) -> bool {
        let pending = if self.dropped > 0 {
            record::DROPPED_WORDS
        } else {
            0
        };
        let needed = u64::try_from(record.len().saturating_add(pending)).unwrap_or(u64::MAX);
        if record.len() > MAX_WORDS || !self.has_room(needed) {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }
        if self.dropped > 0 {
            let timestamp = record.get(1).copied().unwrap_or(0);
            self.put(&record::dropped(timestamp, self.dropped));
            self.dropped = 0;
        }
        self.put(record);
        self.ring.write.store(self.position, Ordering::Release);
        true
    }

    /// The records dropped since the last drop record was written.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    fn has_room(&mut self, needed: u64) -> bool {
        let capacity = self.ring.capacity();
        let fits = |read: u64| self.position.wrapping_sub(read).saturating_add(needed) <= capacity;
        if fits(self.read_seen) {
            return true;
        }
        self.read_seen = self.ring.read.load(Ordering::Acquire);
        fits(self.read_seen)
    }

    fn put(&mut self, words: &[u64]) {
        for &word in words {
            if let Some(slot) = self.ring.slot(self.position) {
                slot.store(word, Ordering::Relaxed);
            }
            self.position = self.position.wrapping_add(1);
        }
    }
}

/// A ring's one reader: the collector.
#[derive(Debug)]
pub struct Reader<'a> {
    ring: RingStorage<'a>,
    position: u64,
}

impl<'a> Reader<'a> {
    /// The reader of `ring`, continuing from its published position.
    #[must_use]
    pub fn new(ring: RingStorage<'a>) -> Self {
        let position = ring.read.load(Ordering::Relaxed);
        Self { ring, position }
    }

    /// Copies every published word into `out`, frees their slots for the
    /// writer, and returns how many there were. A writer's position that no
    /// correct writer could publish, behind the reader or more than a ring
    /// ahead, is refused, and nothing is read (OB11).
    pub fn drain(&mut self, out: &mut Vec<u64>) -> Result<usize, RingError> {
        let written = self.ring.write.load(Ordering::Acquire);
        let available = written.wrapping_sub(self.position);
        if available > self.ring.capacity() {
            return Err(RingError::Overrun {
                written,
                read: self.position,
            });
        }
        let count = usize::try_from(available).unwrap_or(0);
        out.reserve(count);
        for _ in 0..count {
            out.push(
                self.ring
                    .slot(self.position)
                    .map_or(0, |slot| slot.load(Ordering::Relaxed)),
            );
            self.position = self.position.wrapping_add(1);
        }
        self.ring.read.store(self.position, Ordering::Release);
        Ok(count)
    }
}

/// Why a ring can't be made or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingError {
    /// The words aren't a power of two holding two of the longest records.
    Capacity(usize),
    /// The writer's position is behind the reader's or more than a ring ahead.
    Overrun {
        /// The writer's published position.
        written: u64,
        /// The reader's position.
        read: u64,
    },
}

impl fmt::Display for RingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity(words) => {
                let least = MAX_WORDS.saturating_mul(2);
                write!(
                    f,
                    "a ring of {words} words: it must be a power of two of at least {least}"
                )
            }
            Self::Overrun { written, read } => write!(
                f,
                "the writer's position {written} is impossible from the reader's {read}"
            ),
        }
    }
}

impl std::error::Error for RingError {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::{OwnedRing, Reader, RingError, Writer};
    use crate::category::Category;
    use crate::record::{self, Kind, Name, Record, records};

    fn counter(timestamp: u64) -> [u64; 3] {
        record::value(
            Kind::Counter,
            Category::Task,
            Name::new(1),
            timestamp,
            timestamp,
        )
    }

    fn drain(reader: &mut Reader<'_>) -> Vec<Record> {
        let mut words = Vec::new();
        reader.drain(&mut words).unwrap();
        records(&words).map(Result::unwrap).collect()
    }

    #[test]
    fn capacity_is_a_power_of_two_holding_two_records() {
        assert_eq!(OwnedRing::new(100).unwrap_err(), RingError::Capacity(100));
        assert_eq!(OwnedRing::new(64).unwrap_err(), RingError::Capacity(64));
        assert!(OwnedRing::new(128).is_ok());
    }

    #[test]
    fn records_come_out_in_order_across_the_wrap() {
        let ring = OwnedRing::new(128).unwrap();
        let (mut writer, mut reader) = (
            Writer::new(ring.storage().unwrap()),
            Reader::new(ring.storage().unwrap()),
        );
        let mut next = 0;
        // Three passes over the ring's 128 words, 3 words a record.
        for _ in 0..3 {
            for _ in 0..40 {
                assert!(writer.write(&counter(next)));
                next += 1;
            }
            let got = drain(&mut reader);
            assert_eq!(got.len(), 40);
            assert!(
                got.iter().zip(next - 40..).all(
                    |(r, t)| matches!(r, Record::Counter(h, v) if h.timestamp == t && *v == t)
                )
            );
        }
    }

    #[test]
    fn a_full_ring_drops_and_counts_and_reports_the_count_with_the_next_record() {
        let ring = OwnedRing::new(128).unwrap();
        let (mut writer, mut reader) = (
            Writer::new(ring.storage().unwrap()),
            Reader::new(ring.storage().unwrap()),
        );
        let written = (0..60).filter(|t| writer.write(&counter(*t))).count();
        assert_eq!(written, 42, "128 words hold 42 three-word records");
        assert_eq!(writer.dropped(), 18);
        assert_eq!(drain(&mut reader).len(), 42);
        assert!(writer.write(&counter(100)));
        assert_eq!(writer.dropped(), 0);
        assert_eq!(
            drain(&mut reader),
            [
                Record::Dropped {
                    timestamp: 100,
                    count: 18
                },
                record::decode(&counter(100)).unwrap().0
            ]
        );
    }

    #[test]
    fn the_reader_refuses_an_impossible_writer_position() {
        let ring = OwnedRing::new(128).unwrap();
        let mut reader = Reader::new(ring.storage().unwrap());
        let mut words = Vec::new();
        ring.write.store(129, Ordering::Release); // More than a ring ahead.
        assert_eq!(
            reader.drain(&mut words),
            Err(RingError::Overrun {
                written: 129,
                read: 0
            })
        );
        ring.write.store(u64::MAX, Ordering::Release); // Behind the reader.
        assert!(matches!(
            reader.drain(&mut words),
            Err(RingError::Overrun { .. })
        ));
        assert!(words.is_empty());
        assert_eq!(ring.read.load(Ordering::Acquire), 0, "nothing was read");
    }

    #[test]
    fn a_new_writer_or_reader_continues_from_the_published_positions() {
        let ring = OwnedRing::new(128).unwrap();
        assert!(Writer::new(ring.storage().unwrap()).write(&counter(1)));
        let mut reader = Reader::new(ring.storage().unwrap());
        assert_eq!(drain(&mut reader).len(), 1);
        assert!(Writer::new(ring.storage().unwrap()).write(&counter(2)));
        assert_eq!(
            drain(&mut Reader::new(ring.storage().unwrap())),
            [record::decode(&counter(2)).unwrap().0]
        );
    }

    #[test]
    fn an_oversized_record_is_dropped_not_written() {
        let ring = OwnedRing::new(256).unwrap();
        let mut writer = Writer::new(ring.storage().unwrap());
        assert!(!writer.write(&[0; record::MAX_WORDS + 1]));
        assert_eq!(writer.dropped(), 1);
    }
}
