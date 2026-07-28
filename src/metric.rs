// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Metric configuration and immutable metric snapshots.
// qubit-style: allow multiple-public-types

use std::sync::Arc;

/// Stable metadata for one metric in a progress operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metric {
    /// Machine-readable identifier.
    pub(crate) id: Arc<str>,
    /// Human-readable name.
    pub(crate) name: Arc<str>,
    /// Optional configured total.
    pub(crate) total: Option<u64>,
}

impl Metric {
    /// Creates metric metadata without a known total.
    ///
    /// The ID and name are validated when the enclosing progress operation is
    /// started, so this constructor never panics.
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: Arc::from(id),
            name: Arc::from(name),
            total: None,
        }
    }

    /// Records the total work for this metric.
    ///
    /// The value is carried automatically by all future events from the
    /// operation that owns this metric.
    #[must_use]
    pub const fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Returns the metric's stable ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the metric's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the configured total, if it is known.
    #[must_use]
    pub const fn configured_total(&self) -> Option<u64> {
        self.total
    }
}

/// Mutable dynamic counts available only while configuring a report snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricCounts {
    /// Work items that are no longer active.
    pub(crate) completed: u64,
    /// Work items currently in flight.
    pub(crate) active: u64,
    /// Completed work items explicitly known to have succeeded.
    pub(crate) succeeded: u64,
    /// Completed work items explicitly known to have failed.
    pub(crate) failed: u64,
}

impl MetricCounts {
    /// Sets the number of completed work items.
    pub fn completed(&mut self, completed: u64) -> &mut Self {
        self.completed = completed;
        self
    }
    /// Sets the number of active work items.
    pub fn active(&mut self, active: u64) -> &mut Self {
        self.active = active;
        self
    }
    /// Sets the number of completed work items known to have succeeded.
    pub fn succeeded(&mut self, succeeded: u64) -> &mut Self {
        self.succeeded = succeeded;
        self
    }
    /// Sets the number of completed work items known to have failed.
    pub fn failed(&mut self, failed: u64) -> &mut Self {
        self.failed = failed;
        self
    }
}

/// Immutable complete state for one metric in an emitted event.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricSnapshot {
    /// Machine-readable metric ID.
    id: Arc<str>,
    /// Human-readable metric name.
    name: Arc<str>,
    /// Configured total, if known.
    total: Option<u64>,
    /// Completed count.
    completed: u64,
    /// Active count.
    active: u64,
    /// Succeeded count.
    succeeded: u64,
    /// Failed count.
    failed: u64,
}

impl MetricSnapshot {
    /// Combines one stable metric definition with one validated count set.
    pub(crate) fn new(metric: &Metric, counts: MetricCounts) -> Self {
        Self {
            id: Arc::clone(&metric.id),
            name: Arc::clone(&metric.name),
            total: metric.total,
            completed: counts.completed,
            active: counts.active,
            succeeded: counts.succeeded,
            failed: counts.failed,
        }
    }
    /// Returns the metric's stable ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the metric's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the total configured for this event's metric.
    #[must_use]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }
    /// Returns the number of completed work items.
    #[must_use]
    pub const fn completed(&self) -> u64 {
        self.completed
    }
    /// Returns the number of active work items.
    #[must_use]
    pub const fn active(&self) -> u64 {
        self.active
    }
    /// Returns the number of explicitly successful work items.
    #[must_use]
    pub const fn succeeded(&self) -> u64 {
        self.succeeded
    }
    /// Returns the number of explicitly failed work items.
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }
    /// Returns the completed fraction when the total is positive and known.
    #[must_use]
    pub fn completion_fraction(&self) -> Option<f64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.completed as f64 / total as f64)
    }
}
