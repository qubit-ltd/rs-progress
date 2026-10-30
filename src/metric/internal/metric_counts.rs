// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::MetricError;

/// Tracks the dynamic lifecycle counts used to validate a metric update.
///
/// The fields partition work into active and terminal states. The terminal
/// counts are kept separately so an update can preserve both the total and
/// outcome-specific counts without storing a redundant completed value.
#[derive(Clone, Copy)]
pub struct MetricCounts {
    /// Work that has started but is not terminal.
    pub(in crate::metric) active: u64,
    /// Terminal work without explicit success, failure, or cancellation.
    pub(in crate::metric) completed_unclassified: u64,
    /// Terminal work classified as successful.
    pub(in crate::metric) succeeded: u64,
    /// Terminal work classified as failed.
    pub(in crate::metric) failed: u64,
    /// Terminal work classified as cancelled.
    pub(in crate::metric) cancelled: u64,
}

impl MetricCounts {
    /// Returns the derived public completed count.
    ///
    /// Every transition conserves the total count, which cannot exceed `u64`.
    ///
    /// # Returns
    ///
    /// `Some` contains the sum of all terminal states when it fits in `u64`;
    /// `None` indicates that adding the terminal counts overflowed.
    #[inline]
    pub fn completed(self) -> Option<u64> {
        self.completed_unclassified
            .checked_add(self.succeeded)?
            .checked_add(self.failed)?
            .checked_add(self.cancelled)
    }

    /// Returns active plus completed work.
    ///
    /// Returns `None` if either the completed count or the occupied count
    /// overflows `u64`.
    ///
    /// # Returns
    ///
    /// `Some` contains the total active and terminal work when it fits in
    /// `u64`; `None` indicates that either addition overflowed.
    #[inline]
    pub fn occupied(self) -> Option<u64> {
        self.completed()?.checked_add(self.active)
    }

    /// Validates the aggregate conservation invariants for one pending state.
    ///
    /// # Errors
    ///
    /// Returns [`MetricError::CountOverflow`] if the aggregate count overflows,
    /// or [`MetricError::TotalExceeded`] if a configured total is smaller than
    /// the occupied count.
    ///
    /// # Parameters
    ///
    /// - `metric_id`: Identifier attached to any validation error.
    /// - `total`: Optional configured upper bound for active and completed
    ///   work.
    pub fn validate(self, metric_id: &str, total: Option<u64>) -> Result<(), MetricError> {
        let occupied = self.occupied().ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
        if let Some(total) = total
            && occupied > total
        {
            return Err(MetricError::TotalExceeded {
                metric_id: metric_id.into(),
                total,
                attempted: occupied,
            });
        }
        Ok(())
    }
}
