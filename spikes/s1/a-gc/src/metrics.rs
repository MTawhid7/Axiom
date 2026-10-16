//! Heap accounting, exposed for tests and benchmarks.

use core::time::Duration;

/// A snapshot of an arena's heap accounting.
///
/// Byte counts are the sizes of the boxes (header plus value), not what the
/// system allocator actually reserves. Peak resident memory is measured outside
/// the process (`/usr/bin/time -l`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Boxes currently allocated: reachable objects, garbage not yet collected,
    /// and dead boxes kept only for weak references.
    pub objects: usize,
    pub bytes: usize,
    pub peak_objects: usize,
    pub peak_bytes: usize,
    pub allocated_since_collection: usize,
    pub total_allocated_objects: u64,
    pub total_allocated_bytes: u64,
    /// Write-barrier invocations (`Gc::write`). This measures ceremony; the
    /// barrier itself does no work in this non-incremental collector.
    pub barriers: u64,
    pub collections: u64,
    pub last_pause: Duration,
    pub max_pause: Duration,
    pub total_pause: Duration,
    pub objects_after_collection: usize,
    pub bytes_after_collection: usize,
}
