//! Pattern-independent workload model for spike S1.
//!
//! A [`Script`] is a deterministic sequence of DOM operations ([`Op`]). Pattern A
//! (gc-arena style) and Pattern B (Nova style) both execute it natively, and each
//! must reproduce the reference interpreter's [`Outcome`] (added in CP2) exactly.
//!
//! [`Op::Checkpoint`] always performs a full collection before it records
//! anything. That makes the live-object counts deterministic, and turns them into
//! a precise detector for both leaks and premature frees. They are counted by
//! the [`counters`] tokens that the pattern implementations embed in their
//! objects.
#![forbid(unsafe_code)]

pub mod counters;
pub mod digest;
pub mod outcome;
pub mod rng;
pub mod script;

pub use digest::{Fnv1a, TreeDigest};
pub use outcome::{CheckpointRecord, LogEntry, Outcome, Phase};
pub use rng::Rng;
pub use script::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, NodeId, Op, Script, ScriptBuilder,
    Tag, WeakId,
};
