// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
/// One atomic batch of additive metric lifecycle changes.
///
/// # Examples
///
/// ```
/// use qubit_progress::MetricDelta;
///
/// let _delta = MetricDelta::new().started(3).succeeded(1);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricDelta {
    /// Work moving from not-started to active.
    pub(super) started: u64,
    /// Active work becoming completed without an outcome classification.
    pub(super) unclassified: u64,
    /// Active work becoming successful.
    pub(super) succeeded: u64,
    /// Active work becoming failed.
    pub(super) failed: u64,
    /// Active work becoming cancelled.
    pub(super) cancelled: u64,
}

impl MetricDelta {
    /// Creates a zero delta.
    ///
    /// # Returns
    ///
    /// A delta whose lifecycle counts are all zero.
    #[inline]
    pub const fn new() -> Self {
        Self {
            started: 0,
            unclassified: 0,
            succeeded: 0,
            failed: 0,
            cancelled: 0,
        }
    }

    /// Sets the number of work items entering the active state.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to start.
    ///
    /// # Returns
    ///
    /// The delta with the starting count replaced by `count`.
    #[inline]
    pub const fn started(mut self, count: u64) -> Self {
        self.started = count;
        self
    }

    /// Sets the number of work items completing without classification.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to complete without classification.
    ///
    /// # Returns
    ///
    /// The delta with the unclassified completion count replaced by `count`.
    #[inline]
    pub const fn unclassified(mut self, count: u64) -> Self {
        self.unclassified = count;
        self
    }

    /// Sets the number of work items completing successfully.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to mark successful.
    ///
    /// # Returns
    ///
    /// The delta with the success count replaced by `count`.
    #[inline]
    pub const fn succeeded(mut self, count: u64) -> Self {
        self.succeeded = count;
        self
    }

    /// Sets the number of work items completing with failure.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to mark failed.
    ///
    /// # Returns
    ///
    /// The delta with the failure count replaced by `count`.
    #[inline]
    pub const fn failed(mut self, count: u64) -> Self {
        self.failed = count;
        self
    }

    /// Sets the number of work items completing by cancellation.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of work items to cancel.
    ///
    /// # Returns
    ///
    /// The delta with the cancellation count replaced by `count`.
    #[inline]
    pub const fn cancelled(mut self, count: u64) -> Self {
        self.cancelled = count;
        self
    }
}
