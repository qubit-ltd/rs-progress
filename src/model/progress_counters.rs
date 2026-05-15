/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Generic progress counters for a running operation.
///
/// # Examples
///
/// ```
/// use qubit_progress::ProgressCounters;
///
/// let counters = ProgressCounters::new(Some(10))
///     .with_completed_count(4)
///     .with_active_count(2)
///     .with_succeeded_count(3);
///
/// assert_eq!(counters.total_count(), Some(10));
/// assert_eq!(counters.remaining_count(), Some(4));
/// assert_eq!(counters.progress_percent(), Some(40.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgressCounters {
    /// Total work-unit count when known.
    total_count: Option<usize>,
    /// Completed work-unit count.
    completed_count: usize,
    /// Active work-unit count.
    active_count: usize,
    /// Successful work-unit count.
    succeeded_count: usize,
    /// Failed work-unit count.
    failed_count: usize,
}

impl ProgressCounters {
    /// Creates counters with a known or unknown total count.
    ///
    /// # Parameters
    ///
    /// * `total_count` - Total work-unit count, or `None` when unknown.
    ///
    /// # Returns
    ///
    /// Zeroed counters with the supplied total count.
    #[inline]
    pub const fn new(total_count: Option<usize>) -> Self {
        Self {
            total_count,
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
    /// This counter set with `total_count` recorded.
    #[inline]
    pub const fn with_total_count(mut self, total_count: Option<usize>) -> Self {
        self.total_count = total_count;
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
    /// This counter set with `completed_count` recorded.
    #[inline]
    pub const fn with_completed_count(mut self, completed_count: usize) -> Self {
        self.completed_count = completed_count;
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
    /// This counter set with `active_count` recorded.
    #[inline]
    pub const fn with_active_count(mut self, active_count: usize) -> Self {
        self.active_count = active_count;
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
    /// This counter set with `succeeded_count` recorded.
    #[inline]
    pub const fn with_succeeded_count(mut self, succeeded_count: usize) -> Self {
        self.succeeded_count = succeeded_count;
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
    /// This counter set with `failed_count` recorded.
    #[inline]
    pub const fn with_failed_count(mut self, failed_count: usize) -> Self {
        self.failed_count = failed_count;
        self
    }

    /// Returns the total work-unit count when known.
    ///
    /// # Returns
    ///
    /// `Some(total)` for known-total progress, or `None` for open-ended
    /// progress.
    #[inline]
    pub const fn total_count(&self) -> Option<usize> {
        self.total_count
    }

    /// Returns the completed work-unit count.
    ///
    /// # Returns
    ///
    /// The number of completed work units.
    #[inline]
    pub const fn completed_count(&self) -> usize {
        self.completed_count
    }

    /// Returns the active work-unit count.
    ///
    /// # Returns
    ///
    /// The number of currently active work units.
    #[inline]
    pub const fn active_count(&self) -> usize {
        self.active_count
    }

    /// Returns the successful work-unit count.
    ///
    /// # Returns
    ///
    /// The number of successful work units.
    #[inline]
    pub const fn succeeded_count(&self) -> usize {
        self.succeeded_count
    }

    /// Returns the failed work-unit count.
    ///
    /// # Returns
    ///
    /// The number of failed work units.
    #[inline]
    pub const fn failed_count(&self) -> usize {
        self.failed_count
    }

    /// Returns the remaining work-unit count when the total is known.
    ///
    /// # Returns
    ///
    /// `Some(total - completed - active)` using saturating arithmetic for
    /// known-total progress, or `None` when the total is unknown.
    #[inline]
    pub const fn remaining_count(&self) -> Option<usize> {
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
                (self.completed_count as f64 / total_count as f64).clamp(0.0, 1.0)
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
