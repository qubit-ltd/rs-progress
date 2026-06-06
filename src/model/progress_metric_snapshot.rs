// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::Duration;

use serde::{
    Deserialize,
    Serialize,
};

use super::{
    ProgressCounter,
    ProgressMetric,
    ProgressPhase,
    ProgressStage,
};

/// Snapshot of one metric counter within a progress event.
///
/// A progress event may carry multiple counters using different metric units,
/// such as entries and bytes. `ProgressMetricSnapshot` flattens one counter
/// together with the event-level phase, stage, and elapsed time so formatters
/// and consumers can handle one metric record at a time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressMetricSnapshot {
    /// Complete metric metadata for this snapshot.
    metric: ProgressMetric,
    /// Lifecycle phase inherited from the source progress event.
    phase: ProgressPhase,
    /// Optional stage inherited from the source progress event.
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<ProgressStage>,
    /// Total work-unit count when known.
    total_count: Option<u64>,
    /// Completed work-unit count.
    completed_count: u64,
    /// Active work-unit count.
    active_count: u64,
    /// Successful work-unit count.
    succeeded_count: u64,
    /// Failed work-unit count.
    failed_count: u64,
    /// Monotonic elapsed duration inherited from the source progress event.
    #[serde(with = "qubit_serde::serde::duration_with_unit")]
    elapsed: Duration,
}

impl ProgressMetricSnapshot {
    /// Creates a metric snapshot from explicit values.
    ///
    /// # Parameters
    ///
    /// * `metric` - Complete metric metadata.
    /// * `phase` - Lifecycle phase inherited from the source event.
    /// * `stage` - Optional stage inherited from the source event.
    /// * `counter` - Counter values copied into the snapshot.
    /// * `elapsed` - Elapsed duration inherited from the source event.
    ///
    /// # Returns
    ///
    /// A flattened metric snapshot.
    #[inline]
    pub fn new(
        metric: ProgressMetric,
        phase: ProgressPhase,
        stage: Option<ProgressStage>,
        counter: &ProgressCounter,
        elapsed: Duration,
    ) -> Self {
        Self {
            metric,
            phase,
            stage,
            total_count: counter.total_count(),
            completed_count: counter.completed_count(),
            active_count: counter.active_count(),
            succeeded_count: counter.succeeded_count(),
            failed_count: counter.failed_count(),
            elapsed,
        }
    }

    /// Returns the complete metric metadata.
    ///
    /// # Returns
    ///
    /// The metric metadata associated with this snapshot.
    #[inline]
    pub const fn metric(&self) -> &ProgressMetric {
        &self.metric
    }

    /// Returns the stable metric id.
    ///
    /// # Returns
    ///
    /// The metric id associated with this snapshot.
    #[inline]
    pub fn metric_id(&self) -> &str {
        self.metric.id()
    }

    /// Returns the human-readable metric name.
    ///
    /// # Returns
    ///
    /// The metric display name associated with this snapshot.
    #[inline]
    pub fn metric_name(&self) -> &str {
        self.metric.name()
    }

    /// Returns the lifecycle phase.
    ///
    /// # Returns
    ///
    /// The phase inherited from the source event.
    #[inline]
    pub const fn phase(&self) -> ProgressPhase {
        self.phase
    }

    /// Returns the optional stage.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when the source event carried stage metadata, otherwise
    /// `None`.
    #[inline]
    pub const fn stage(&self) -> Option<&ProgressStage> {
        self.stage.as_ref()
    }

    /// Returns the total work-unit count when known.
    ///
    /// # Returns
    ///
    /// `Some(total)` for known-total progress, or `None` for open-ended
    /// progress.
    #[inline]
    pub const fn total_count(&self) -> Option<u64> {
        self.total_count
    }

    /// Returns the completed work-unit count.
    ///
    /// # Returns
    ///
    /// The number of completed work units.
    #[inline]
    pub const fn completed_count(&self) -> u64 {
        self.completed_count
    }

    /// Returns the active work-unit count.
    ///
    /// # Returns
    ///
    /// The number of currently active work units.
    #[inline]
    pub const fn active_count(&self) -> u64 {
        self.active_count
    }

    /// Returns the successful work-unit count.
    ///
    /// # Returns
    ///
    /// The number of successful work units.
    #[inline]
    pub const fn succeeded_count(&self) -> u64 {
        self.succeeded_count
    }

    /// Returns the failed work-unit count.
    ///
    /// # Returns
    ///
    /// The number of failed work units.
    #[inline]
    pub const fn failed_count(&self) -> u64 {
        self.failed_count
    }

    /// Returns the remaining work-unit count when the total is known.
    ///
    /// # Returns
    ///
    /// `Some(total - completed - active)` using saturating arithmetic for
    /// known-total progress, or `None` when the total is unknown.
    #[inline]
    pub const fn remaining_count(&self) -> Option<u64> {
        match self.total_count {
            Some(total_count) => Some(
                total_count
                    .saturating_sub(self.completed_count)
                    .saturating_sub(self.active_count),
            ),
            None => None,
        }
    }

    /// Returns completed progress as a fraction in `0.0..=1.0`.
    ///
    /// # Returns
    ///
    /// `Some(fraction)` for known-total progress, `Some(1.0)` when the known
    /// total is zero, or `None` when the total is unknown.
    #[inline]
    pub fn progress_fraction(&self) -> Option<f64> {
        self.total_count.map(|total_count| {
            if total_count == 0 {
                1.0
            } else {
                (self.completed_count as f64 / total_count as f64)
                    .clamp(0.0, 1.0)
            }
        })
    }

    /// Returns completed progress as a percentage in `0.0..=100.0`.
    ///
    /// # Returns
    ///
    /// `Some(percent)` for known-total progress, or `None` when the total is
    /// unknown.
    #[inline]
    pub fn progress_percent(&self) -> Option<f64> {
        self.progress_fraction().map(|fraction| fraction * 100.0)
    }

    /// Returns the elapsed duration.
    ///
    /// # Returns
    ///
    /// The elapsed duration inherited from the source event.
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}
