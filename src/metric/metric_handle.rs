// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::Arc;

use crate::MetricError;
use crate::internal::OperationState;
use crate::metric::Metric;
use crate::metric::MetricDelta;
use crate::metric::MetricSnapshot;
use crate::metric::internal::MetricInner;
use crate::metric::internal::apply_delta_to_counts;

/// Cloneable capability for one live metric owned by a progress operation.
///
/// All mutation methods are serialized by one CAS gate critical section.
/// A handle remains readable after its progress operation closes, but rejects
/// all later mutations.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, Progress, TextReporter};
///
/// let reporter = std::sync::Arc::new(TextReporter::new(Vec::new()));
/// let progress = Progress::builder_arc(reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// let tasks = progress.metric("tasks").expect("configured metric must exist");
/// tasks.start(1)?;
/// assert_eq!(tasks.snapshot().active(), 1);
/// tasks.complete(1)?;
/// progress.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct MetricHandle {
    /// Shared metadata and mutable count state.
    inner: Arc<MetricInner>,
    /// Shared lifecycle gate owned by the enclosing progress operation.
    operation_state: Arc<OperationState>,
}

impl MetricHandle {
    /// Creates one live handle from validated metric metadata.
    pub(crate) fn new(metric: Metric, operation_state: Arc<OperationState>) -> Self {
        Self {
            inner: Arc::new(MetricInner::new(metric)),
            operation_state,
        }
    }

    /// Returns the stable metric ID.
    ///
    /// # Returns
    ///
    /// The machine-readable ID supplied when the owning operation was built.
    #[must_use]
    #[inline]
    pub fn id(&self) -> &str {
        self.inner.metric.id()
    }

    /// Returns the stable metric display name.
    ///
    /// # Returns
    ///
    /// The human-readable name supplied when the owning operation was built.
    #[must_use]
    #[inline]
    pub fn name(&self) -> &str {
        self.inner.metric.name()
    }

    /// Returns one internally consistent immutable metric snapshot.
    ///
    /// This read remains available after the owning operation closes.
    ///
    /// # Returns
    ///
    /// A snapshot containing metadata and counts from one consistent state.
    #[must_use]
    pub fn snapshot(&self) -> MetricSnapshot {
        let counts = self.inner.snapshot_counts();
        MetricSnapshot::from_counts(&self.inner.metric, counts)
    }

    /// Moves work from the not-started state to the active state.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to start.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transition is applied.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    #[inline]
    pub fn start(&self, count: u64) -> Result<(), MetricError> {
        self.apply_delta(MetricDelta::new().started(count))
    }

    /// Moves work from the active state to unclassified completion.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of active work items to complete.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transition is applied.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    #[inline]
    pub fn complete(&self, count: u64) -> Result<(), MetricError> {
        self.apply_delta(MetricDelta::new().unclassified(count))
    }

    /// Moves work from the active state to the succeeded state.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of active work items to mark successful.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transition is applied.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    #[inline]
    pub fn succeed(&self, count: u64) -> Result<(), MetricError> {
        self.apply_delta(MetricDelta::new().succeeded(count))
    }

    /// Moves work from the active state to the failed state.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of active work items to mark failed.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transition is applied.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    #[inline]
    pub fn fail(&self, count: u64) -> Result<(), MetricError> {
        self.apply_delta(MetricDelta::new().failed(count))
    }

    /// Moves work from the active state to the cancelled state.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of active work items to cancel.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transition is applied.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    #[inline]
    pub fn cancel(&self, count: u64) -> Result<(), MetricError> {
        self.apply_delta(MetricDelta::new().cancelled(count))
    }

    /// Applies one atomic additive batch of lifecycle changes.
    ///
    /// # Parameters
    ///
    /// - `delta`: Counts to add to the corresponding lifecycle states.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the full delta is applied atomically.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the delta exceeds active work, violates a
    /// configured total, overflows, or the owning operation is closed. The
    /// metric is unchanged whenever an error is returned.
    pub fn apply_delta(&self, delta: MetricDelta) -> Result<(), MetricError> {
        let metric_id = self.id();
        let total = self.inner.metric.configured_total();
        let _update_guard = self.operation_state.enter_update(metric_id)?;

        self.inner.with_update(|counts| {
            let mut next = *counts;
            apply_delta_to_counts(&mut next, delta, metric_id)?;
            next.validate(metric_id, total)?;
            *counts = next;
            Ok(())
        })
    }
}
