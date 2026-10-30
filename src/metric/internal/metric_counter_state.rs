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
pub(in crate::metric) struct MetricCounterState<G, A> {
    gate: G,
    active: A,
    completed_unclassified: A,
    succeeded: A,
    failed: A,
    cancelled: A,
}

impl<G: MetricAtomicWord, A: MetricAtomicWord> MetricCounterState<G, A> {
    /// Creates a counter state with all values initialized to zero.
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

    /// Reads a consistent copy of all counters, waiting on contention.
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
