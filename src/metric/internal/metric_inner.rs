// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use qubit_fast_cas::CasCell;

use crate::MetricError;
use crate::metric::Metric;
use crate::metric::internal::MetricCounts;
use crate::metric::internal::MetricGateGuard;
use crate::metric::internal::wait_for_contention;

/// Immutable metadata and atomic dynamic state for one handle.
pub struct MetricInner {
    /// Fixed metric definition supplied to the progress builder.
    pub(in crate::metric) metric: Metric,
    /// Dynamic updates are serialized by this gate.
    gate: CasCell,
    /// Work that has started but is not terminal.
    active: AtomicU64,
    /// Terminal work without explicit success, failure, or cancellation.
    completed_unclassified: AtomicU64,
    /// Terminal work classified as successful.
    succeeded: AtomicU64,
    /// Terminal work classified as failed.
    failed: AtomicU64,
    /// Terminal work classified as cancelled.
    cancelled: AtomicU64,
}

impl MetricInner {
    /// Builds one live metric inner state with zeroed counters.
    pub fn new(metric: Metric) -> Self {
        Self {
            metric,
            gate: CasCell::new(0),
            active: AtomicU64::new(0),
            completed_unclassified: AtomicU64::new(0),
            succeeded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        }
    }

    /// Runs one validated update while exclusively holding the gate.
    ///
    /// The closure receives a local copy of the counters. Its changes are
    /// committed only when it returns `Ok`, so a validation error leaves the
    /// stored counters unchanged. The gate is released on return or unwind.
    ///
    /// # Errors
    ///
    /// Returns the error produced by `update` without committing its local
    /// counter changes.
    pub fn with_update<R, F>(&self, mut update: F) -> Result<R, MetricError>
    where
        F: FnMut(&mut MetricCounts) -> Result<R, MetricError>,
    {
        let mut attempts = 0;
        loop {
            let version = self.gate.load();
            if version & 1 != 0 {
                wait_for_contention(attempts);
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
                    wait_for_contention(attempts);
                    attempts += 1;
                }
            }
        }
    }

    /// Reads all counter fields with acquire order and copies them by value.
    fn read_counts(&self) -> MetricCounts {
        MetricCounts {
            active: self.active.load(Ordering::Acquire),
            completed_unclassified: self.completed_unclassified.load(Ordering::Acquire),
            succeeded: self.succeeded.load(Ordering::Acquire),
            failed: self.failed.load(Ordering::Acquire),
            cancelled: self.cancelled.load(Ordering::Acquire),
        }
    }

    /// Writes all counter fields after successful validation.
    fn write_counts(&self, counts: &MetricCounts) {
        self.active.store(counts.active, Ordering::Release);
        self.completed_unclassified
            .store(counts.completed_unclassified, Ordering::Release);
        self.succeeded.store(counts.succeeded, Ordering::Release);
        self.failed.store(counts.failed, Ordering::Release);
        self.cancelled.store(counts.cancelled, Ordering::Release);
    }

    /// Reads a consistent copy of all counters.
    ///
    /// If an update overlaps the read, this method retries until it observes
    /// the same even gate version before and after reading. Under sustained
    /// contention, the method may wait for an update to finish.
    pub fn snapshot_counts(&self) -> MetricCounts {
        let mut attempts = 0;
        loop {
            let start = self.gate.load();
            if start & 1 != 0 {
                wait_for_contention(attempts);
                attempts += 1;
                continue;
            }

            let counts = self.read_counts();
            if start == self.gate.load() {
                return counts;
            }

            wait_for_contention(attempts);
            attempts += 1;
        }
    }
}
