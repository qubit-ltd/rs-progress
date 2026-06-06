// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use serde::{
    Deserialize,
    Serialize,
};

/// Counters for one metric in a progress event.
///
/// A counter is identified by `metric_id`. The corresponding display name and
/// metric dictionary entry live in [`ProgressSchema`](crate::ProgressSchema).
/// Counter values are `u64` so they can represent domain quantities such as
/// bytes, records, objects, or files independently of the current platform's
/// pointer width.
///
/// # Examples
///
/// ```
/// use qubit_progress::ProgressCounter;
///
/// let counter = ProgressCounter::new("bytes")
///     .total(10_000)
///     .completed(4_000)
///     .active(1);
///
/// assert_eq!(counter.metric_id(), "bytes");
/// assert_eq!(counter.total_count(), Some(10_000));
/// assert_eq!(counter.progress_percent(), Some(40.0));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProgressCounter {
    /// Identifier of the metric this counter reports.
    metric_id: String,
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
}

impl ProgressCounter {
    /// Creates a counter for a metric with an unknown total count.
    ///
    /// # Parameters
    ///
    /// * `metric_id` - Metric identifier declared by the progress schema.
    ///
    /// # Returns
    ///
    /// A zeroed counter associated with `metric_id`.
    #[inline]
    pub fn new(metric_id: &str) -> Self {
        Self {
            metric_id: metric_id.to_owned(),
            total_count: None,
            completed_count: 0,
            active_count: 0,
            succeeded_count: 0,
            failed_count: 0,
        }
    }

    /// Returns a copy configured with the known or unknown total count.
    ///
    /// # Parameters
    ///
    /// * `total_count` - Total work-unit count, or `None` when unknown.
    ///
    /// # Returns
    ///
    /// This counter with `total_count` recorded.
    #[inline]
    #[must_use]
    pub const fn with_total_count(mut self, total_count: Option<u64>) -> Self {
        self.total_count = total_count;
        self
    }

    /// Returns a copy configured with a known total count.
    ///
    /// # Parameters
    ///
    /// * `total_count` - Total work-unit count.
    ///
    /// # Returns
    ///
    /// This counter with a known total count.
    #[inline]
    #[must_use]
    pub const fn total(self, total_count: u64) -> Self {
        self.with_total_count(Some(total_count))
    }

    /// Returns a copy configured with an unknown total count.
    ///
    /// # Returns
    ///
    /// This counter with no total count.
    #[inline]
    #[must_use]
    pub const fn unknown_total(self) -> Self {
        self.with_total_count(None)
    }

    /// Returns a copy configured with the completed count.
    ///
    /// # Parameters
    ///
    /// * `completed_count` - Number of completed work units.
    ///
    /// # Returns
    ///
    /// This counter with `completed_count` recorded.
    #[inline]
    #[must_use]
    pub const fn with_completed_count(mut self, completed_count: u64) -> Self {
        self.completed_count = completed_count;
        self
    }

    /// Returns a copy configured with the completed count.
    ///
    /// # Parameters
    ///
    /// * `completed_count` - Number of completed work units.
    ///
    /// # Returns
    ///
    /// This counter with `completed_count` recorded.
    #[inline]
    #[must_use]
    pub const fn completed(self, completed_count: u64) -> Self {
        self.with_completed_count(completed_count)
    }

    /// Returns a copy configured with the active count.
    ///
    /// # Parameters
    ///
    /// * `active_count` - Number of currently active work units.
    ///
    /// # Returns
    ///
    /// This counter with `active_count` recorded.
    #[inline]
    #[must_use]
    pub const fn with_active_count(mut self, active_count: u64) -> Self {
        self.active_count = active_count;
        self
    }

    /// Returns a copy configured with the active count.
    ///
    /// # Parameters
    ///
    /// * `active_count` - Number of currently active work units.
    ///
    /// # Returns
    ///
    /// This counter with `active_count` recorded.
    #[inline]
    #[must_use]
    pub const fn active(self, active_count: u64) -> Self {
        self.with_active_count(active_count)
    }

    /// Returns a copy configured with the succeeded count.
    ///
    /// # Parameters
    ///
    /// * `succeeded_count` - Number of successful work units.
    ///
    /// # Returns
    ///
    /// This counter with `succeeded_count` recorded.
    #[inline]
    #[must_use]
    pub const fn with_succeeded_count(mut self, succeeded_count: u64) -> Self {
        self.succeeded_count = succeeded_count;
        self
    }

    /// Returns a copy configured with the succeeded count.
    ///
    /// # Parameters
    ///
    /// * `succeeded_count` - Number of successful work units.
    ///
    /// # Returns
    ///
    /// This counter with `succeeded_count` recorded.
    #[inline]
    #[must_use]
    pub const fn succeeded(self, succeeded_count: u64) -> Self {
        self.with_succeeded_count(succeeded_count)
    }

    /// Returns a copy configured with the failed count.
    ///
    /// # Parameters
    ///
    /// * `failed_count` - Number of failed work units.
    ///
    /// # Returns
    ///
    /// This counter with `failed_count` recorded.
    #[inline]
    #[must_use]
    pub const fn with_failed_count(mut self, failed_count: u64) -> Self {
        self.failed_count = failed_count;
        self
    }

    /// Returns a copy configured with the failed count.
    ///
    /// # Parameters
    ///
    /// * `failed_count` - Number of failed work units.
    ///
    /// # Returns
    ///
    /// This counter with `failed_count` recorded.
    #[inline]
    #[must_use]
    pub const fn failed(self, failed_count: u64) -> Self {
        self.with_failed_count(failed_count)
    }

    /// Returns the metric identifier.
    ///
    /// # Returns
    ///
    /// The id of the metric this counter reports.
    #[inline]
    pub fn metric_id(&self) -> &str {
        self.metric_id.as_str()
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
}
