//! Observable results of running a [`Script`](crate::Script).

use crate::digest::Fnv1a;
use crate::script::{ListenerId, NodeId};

/// The event phase of a listener invocation (DOM `Event.eventPhase`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Capturing = 1,
    AtTarget = 2,
    Bubbling = 3,
}

/// One listener invocation, appended before the listener's behaviour runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LogEntry {
    pub listener: ListenerId,
    pub current_target: NodeId,
    pub phase: Phase,
}

/// State observed at an [`Op::Checkpoint`](crate::Op::Checkpoint), which always
/// runs a full collection first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CheckpointRecord {
    /// [`TreeDigest`](crate::TreeDigest) of the tree rooted at the document.
    pub tree_digest: u64,
    /// Live nodes after the collection, counting the document and id-less nodes.
    pub live_nodes: usize,
    /// Live closures after the collection (one per live listener).
    pub live_closures: usize,
    /// For each weak slot: `None` if not yet created, otherwise whether its
    /// target is still alive.
    pub weak_alive: Vec<Option<bool>>,
}

/// Everything a script run produces. Two runs agree exactly when their
/// `Outcome`s are equal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Outcome {
    /// Every listener invocation, in order.
    pub log: Vec<LogEntry>,
    /// One record per `Checkpoint` op, in order.
    pub checkpoints: Vec<CheckpointRecord>,
}

impl Outcome {
    /// A single stable digest of the whole outcome, for cheap comparison in
    /// benchmarks. Tests should compare the outcomes themselves.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(self.log.len() as u64);
        for e in &self.log {
            h.write_u32(e.listener.0);
            h.write_u32(e.current_target.0);
            h.write_u8(e.phase as u8);
        }
        h.write_u64(self.checkpoints.len() as u64);
        for c in &self.checkpoints {
            h.write_u64(c.tree_digest);
            h.write_u64(c.live_nodes as u64);
            h.write_u64(c.live_closures as u64);
            h.write_u64(c.weak_alive.len() as u64);
            for w in &c.weak_alive {
                h.write_u8(match w {
                    None => 0,
                    Some(false) => 1,
                    Some(true) => 2,
                });
            }
        }
        h.finish()
    }

    /// A human-readable description of the first difference from `expected`,
    /// or `None` if the two are equal. Used for test failure messages.
    #[must_use]
    pub fn first_difference(&self, expected: &Self) -> Option<String> {
        if let Some(i) = self.log.iter().zip(&expected.log).position(|(a, b)| a != b) {
            return Some(format!(
                "log entry {i}: got {:?}, expected {:?}",
                self.log[i], expected.log[i]
            ));
        }
        if self.log.len() != expected.log.len() {
            return Some(format!(
                "log length: got {}, expected {}",
                self.log.len(),
                expected.log.len()
            ));
        }
        if let Some(i) = self
            .checkpoints
            .iter()
            .zip(&expected.checkpoints)
            .position(|(a, b)| a != b)
        {
            return Some(format!(
                "checkpoint {i}: got {:?}, expected {:?}",
                self.checkpoints[i], expected.checkpoints[i]
            ));
        }
        (self.checkpoints.len() != expected.checkpoints.len()).then(|| {
            format!(
                "checkpoint count: got {}, expected {}",
                self.checkpoints.len(),
                expected.checkpoints.len()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Outcome {
        Outcome {
            log: vec![LogEntry {
                listener: ListenerId(0),
                current_target: NodeId(3),
                phase: Phase::AtTarget,
            }],
            checkpoints: vec![CheckpointRecord {
                tree_digest: 0xFEED,
                live_nodes: 4,
                live_closures: 1,
                weak_alive: vec![Some(true), None],
            }],
        }
    }

    #[test]
    fn equal_outcomes_have_equal_digests_and_no_difference() {
        assert_eq!(sample().digest(), sample().digest());
        assert_eq!(sample().first_difference(&sample()), None);
    }

    #[test]
    fn differences_are_reported_precisely() {
        let mut other = sample();
        other.checkpoints[0].live_nodes = 5;
        assert_ne!(sample().digest(), other.digest());
        let diff = other.first_difference(&sample()).expect("must differ");
        assert!(diff.starts_with("checkpoint 0"), "{diff}");

        let mut shorter = sample();
        shorter.log.clear();
        let diff = shorter.first_difference(&sample()).expect("must differ");
        assert!(diff.starts_with("log length"), "{diff}");
    }
}
