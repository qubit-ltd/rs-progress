// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::atomic::AtomicU64;

use qubit_fast_cas::CasCell;

use crate::MetricError;
use crate::metric::Metric;
use crate::metric::internal::MetricCounterState;
use crate::metric::internal::MetricCounts;
use crate::metric::internal::wait_for_contention;

/// Immutable metadata and atomic dynamic state for one handle.
pub struct MetricInner {
    /// Fixed metric definition supplied to the progress builder.
    pub(in crate::metric) metric: Metric,
    /// Gate and counters implementing the consistent metric protocol.
    state: MetricCounterState<CasCell, AtomicU64>,
}

impl MetricInner {
    /// Builds one live metric inner state with zeroed counters.
    ///
    /// # Parameters
    ///
    /// - `metric`: The fixed definition used to validate future updates.
    ///
    /// # Returns
    ///
    /// A live metric inner state with zeroed counters.
    #[must_use]
    pub fn new(metric: Metric) -> Self {
        Self {
            metric,
            state: MetricCounterState::new(),
        }
    }

    /// Reads a consistent copy of all counters.
    ///
    /// If an update overlaps the read, this method retries until it observes
    /// the same even gate version before and after reading. Under sustained
    /// contention, the method may wait for an update to finish.
    ///
    /// # Returns
    ///
    /// A consistent copy of the current counters.
    #[must_use]
    #[inline]
    pub fn snapshot_counts(&self) -> MetricCounts {
        self.state.snapshot_counts(wait_for_contention)
    }

    /// Runs one validated update while exclusively holding the gate.
    ///
    /// The closure receives a local copy of the counters. Its changes are
    /// committed only when it returns `Ok`, so a validation error leaves the
    /// stored counters unchanged. The gate is released on return or unwind.
    ///
    /// # Type Parameters
    ///
    /// - `R`: Value returned after a successful update.
    /// - `F`: Closure that validates and modifies the local counter copy.
    ///
    /// # Parameters
    ///
    /// - `update`: Validates and modifies a local copy of the counters.
    ///
    /// # Returns
    ///
    /// The value returned by `update` when the update succeeds.
    ///
    /// # Errors
    ///
    /// Returns the error produced by `update` without committing its local
    /// counter changes.
    #[inline]
    pub fn with_update<R, F>(&self, update: F) -> Result<R, MetricError>
    where
        F: FnMut(&mut MetricCounts) -> Result<R, MetricError>,
    {
        self.state.with_update(update, wait_for_contention)
    }
}
