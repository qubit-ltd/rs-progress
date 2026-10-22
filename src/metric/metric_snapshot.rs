// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Deserializer;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::de::Error;

use crate::metric::Metric;
use crate::metric::internal::MetricCounts;
#[cfg(feature = "serde")]
use crate::metric::internal::MetricSnapshotWire;
#[cfg(feature = "serde")]
use crate::validation::validate_metrics;
#[cfg(feature = "serde")]
use crate::validation::validate_snapshot_counts;

/// Immutable complete state for one metric in an emitted event.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, Progress, TextReporter};
///
/// let reporter = std::sync::Arc::new(TextReporter::new(Vec::new()));
/// let progress = Progress::builder_arc(reporter)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// let tasks = progress.metric("tasks").expect("configured metric must exist");
/// tasks.start(1)?;
/// let snapshot = tasks.snapshot();
/// assert_eq!(snapshot.active(), 1);
/// assert_eq!(snapshot.total(), Some(1));
/// tasks.complete(1)?;
/// progress.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricSnapshot {
    /// Machine-readable metric ID.
    pub(super) id: Arc<str>,
    /// Human-readable metric name.
    pub(super) name: Arc<str>,
    /// Configured total, if known.
    pub(super) total: Option<u64>,
    /// Completed count.
    pub(super) completed: u64,
    /// Active count.
    pub(super) active: u64,
    /// Succeeded count.
    pub(super) succeeded: u64,
    /// Failed count.
    pub(super) failed: u64,
    /// Cancelled count.
    pub(super) cancelled: u64,
}

impl MetricSnapshot {
    /// Builds an immutable snapshot from one internally validated metric state.
    pub(super) fn from_counts(metric: &Metric, counts: MetricCounts) -> Self {
        Self {
            id: Arc::clone(&metric.id),
            name: Arc::clone(&metric.name),
            total: metric.total,
            completed: counts.completed().expect("validated metric counts must fit in u64"),
            active: counts.active,
            succeeded: counts.succeeded,
            failed: counts.failed,
            cancelled: counts.cancelled,
        }
    }

    /// Returns the metric's stable ID.
    ///
    /// # Returns
    ///
    /// The machine-readable ID copied from the metric definition.
    #[must_use]
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the metric's display name.
    ///
    /// # Returns
    ///
    /// The human-readable name copied from the metric definition.
    #[must_use]
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the total configured for this event's metric.
    ///
    /// Returns `None` when no total was configured.
    ///
    /// # Returns
    ///
    /// `Some(total)` when a total was configured, or `None` otherwise.
    #[must_use]
    #[inline]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }
    /// Returns the number of completed work items.
    ///
    /// # Returns
    ///
    /// The total count of all terminal work items.
    #[must_use]
    #[inline]
    pub const fn completed(&self) -> u64 {
        self.completed
    }
    /// Returns completed work without an explicit outcome classification.
    ///
    /// # Returns
    ///
    /// The completed count remaining after explicitly classified outcomes.
    #[must_use]
    #[inline]
    pub const fn unclassified(&self) -> u64 {
        let classified = self
            .succeeded
            .saturating_add(self.failed)
            .saturating_add(self.cancelled);
        self.completed.saturating_sub(classified)
    }
    /// Returns the number of active work items.
    ///
    /// # Returns
    ///
    /// The count of work items that have started but are not terminal.
    #[must_use]
    #[inline]
    pub const fn active(&self) -> u64 {
        self.active
    }
    /// Returns the number of explicitly successful work items.
    ///
    /// # Returns
    ///
    /// The count of completed work items marked successful.
    #[must_use]
    #[inline]
    pub const fn succeeded(&self) -> u64 {
        self.succeeded
    }
    /// Returns the number of explicitly failed work items.
    ///
    /// # Returns
    ///
    /// The count of completed work items marked failed.
    #[must_use]
    #[inline]
    pub const fn failed(&self) -> u64 {
        self.failed
    }
    /// Returns the number of explicitly cancelled work items.
    ///
    /// # Returns
    ///
    /// The count of completed work items marked cancelled.
    #[must_use]
    #[inline]
    pub const fn cancelled(&self) -> u64 {
        self.cancelled
    }
    /// Returns the completed fraction when the total is positive and known.
    ///
    /// Returns `None` when no total is configured or when the configured total
    /// is zero.
    ///
    /// # Returns
    ///
    /// `Some(fraction)` for a positive configured total, or `None` otherwise.
    #[must_use]
    #[inline]
    pub fn completion_fraction(&self) -> Option<f64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.completed as f64 / total as f64)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MetricSnapshot {
    /// Deserializes and validates one standalone metric snapshot.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = MetricSnapshotWire::deserialize(deserializer)?;
        let snapshot = Self {
            id: wire.id,
            name: wire.name,
            total: wire.total,
            completed: wire.completed,
            active: wire.active,
            succeeded: wire.succeeded,
            failed: wire.failed,
            cancelled: wire.cancelled,
        };
        let metric = Metric {
            id: Arc::clone(&snapshot.id),
            name: Arc::clone(&snapshot.name),
            total: snapshot.total,
        };
        validate_metrics(std::slice::from_ref(&metric)).map_err(Error::custom)?;
        validate_snapshot_counts(&snapshot).map_err(Error::custom)?;
        Ok(snapshot)
    }
}
