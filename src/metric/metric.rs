// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::Arc;

/// Stable metadata for one metric in a progress operation.
///
/// # Examples
///
/// ```
/// use qubit_progress::Metric;
///
/// let metric = Metric::new("tasks", "Tasks").total(10);
/// assert_eq!(metric.id(), "tasks");
/// assert_eq!(metric.configured_total(), Some(10));
/// ```
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
    ///
    /// # Parameters
    ///
    /// - `id`: Stable machine-readable identifier for the metric.
    /// - `name`: Human-readable display name for the metric.
    ///
    /// # Returns
    ///
    /// Metric metadata with no configured total.
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
    ///
    /// # Parameters
    ///
    /// - `total`: Maximum number of work items represented by this metric.
    ///
    /// # Returns
    ///
    /// The metric metadata with the configured total.
    #[must_use]
    #[inline]
    pub const fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Returns the metric's stable ID.
    ///
    /// # Returns
    ///
    /// The machine-readable ID supplied to [`Metric::new`].
    #[must_use]
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the metric's display name.
    ///
    /// # Returns
    ///
    /// The human-readable name supplied to [`Metric::new`].
    #[must_use]
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the configured total, if it is known.
    ///
    /// Returns `None` when the operation has no configured total for this
    /// metric.
    ///
    /// # Returns
    ///
    /// `Some(total)` when configured, or `None` otherwise.
    #[inline]
    pub const fn configured_total(&self) -> Option<u64> {
        self.total
    }
}
