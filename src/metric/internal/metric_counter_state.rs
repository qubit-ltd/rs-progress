// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::MetricError;
use crate::metric::internal::MetricCounts;
use crate::metric::internal::MetricGateGuard;
use crate::metric::internal::metric_atomic_word::MetricAtomicWord;

/// Atomic gate and counters implementing the metric consistency protocol.
///
/// An even gate version denotes a stable state; an odd version denotes an
/// update in progress. An updater claims the gate by changing an observed even
/// version to the next odd version with compare-and-set. It edits a local copy
/// of all five counters and publishes that copy only when the update returns
/// `Ok`. The guard then publishes the next even gate version, including during
/// unwinding.
///
/// Readers load the gate before and after reading all counters and accept the
/// copy only when both observations are the same even version. The gate and
/// counter atomics work together for this cross-field consistency protocol;
/// this description does not make any individual ordering interchangeable.
/// Versions advance with wrapping arithmetic, so this protocol does not claim
/// to prevent ABA after a complete version cycle.
#[must_use]
pub(in crate::metric) struct MetricCounterState<G, A> {
    /// Even versions are stable; odd versions indicate an update in progress.
    gate: G,
    /// Work that has started but is not terminal.
    active: A,
    /// Terminal work without an explicit outcome classification.
    completed_unclassified: A,
    /// Terminal work classified as successful.
    succeeded: A,
    /// Terminal work classified as failed.
    failed: A,
    /// Terminal work classified as cancelled.
    cancelled: A,
}

impl<G: MetricAtomicWord, A: MetricAtomicWord> MetricCounterState<G, A> {
    /// Creates a counter state with all values initialized to zero.
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            gate: G::new(0),
            active: A::new(0),
            completed_unclassified: A::new(0),
            succeeded: A::new(0),
            failed: A::new(0),
            cancelled: A::new(0),
        }
    }

    /// Reads a consistent copy of all counters, retrying on contention.
    ///
    /// The gate and counter atomics jointly protect the cross-field snapshot.
    /// The counters are loaded with acquire ordering, and the snapshot is
    /// accepted only if the gate has the same even version before and after
    /// those loads. The supplied callback determines how a retry waits.
    #[must_use]
    pub(super) fn snapshot_counts<W: FnMut(usize)>(&self, mut wait: W) -> MetricCounts {
        let mut attempts = 0;
        loop {
            let start = self.gate.load();
            if start & 1 != 0 {
                wait(attempts);
                attempts += 1;
                continue;
            }

            let counts = self.read_counts();
            if start == self.gate.load() {
                return counts;
            }

            wait(attempts);
            attempts += 1;
        }
    }

    /// Applies a validated counter update while holding the gate.
    ///
    /// After claiming the gate with an AcqRel/Acquire compare-and-set, this
    /// method edits a local copy. It publishes the five counters with release
    /// stores only when `update` returns `Ok`; an `Err` leaves stored counters
    /// unchanged. The guard releases the gate by storing the next even version
    /// with release ordering, including if `update` unwinds. Gate and counter
    /// atomics jointly implement the protocol; their orderings are not
    /// interchangeable. The supplied callback determines retry waiting.
    pub(super) fn with_update<R, F, W>(&self, mut update: F, mut wait: W) -> Result<R, MetricError>
    where
        F: FnMut(&mut MetricCounts) -> Result<R, MetricError>,
        W: FnMut(usize),
    {
        let mut attempts = 0;
        loop {
            let version = self.gate.load();
            if version & 1 != 0 {
                wait(attempts);
                attempts += 1;
                continue;
            }

            match self.gate.compare_set(version, version.wrapping_add(1)) {
                Ok(()) => {
                    let _guard = MetricGateGuard::new(&self.gate, version.wrapping_add(2));
                    let mut counts = self.read_counts();
                    let result = update(&mut counts);
                    if result.is_ok() {
                        self.write_counts(&counts);
                    }
                    return result;
                }
                Err(_) => {
                    wait(attempts);
                    attempts += 1;
                }
            }
        }
    }

    /// Reads all counter fields with acquire order.
    fn read_counts(&self) -> MetricCounts {
        MetricCounts {
            active: self.active.load(),
            completed_unclassified: self.completed_unclassified.load(),
            succeeded: self.succeeded.load(),
            failed: self.failed.load(),
            cancelled: self.cancelled.load(),
        }
    }

    /// Publishes the five validated counter values with release ordering.
    fn write_counts(&self, counts: &MetricCounts) {
        self.active.store(counts.active);
        self.completed_unclassified.store(counts.completed_unclassified);
        self.succeeded.store(counts.succeeded);
        self.failed.store(counts.failed);
        self.cancelled.store(counts.cancelled);
    }
}

#[cfg(test)]
mod tests;
