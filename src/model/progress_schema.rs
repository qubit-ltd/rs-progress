// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::collections::HashSet;

#[cfg(feature = "serde")]
use serde::{
    Deserialize,
    Serialize,
};

#[cfg(feature = "serde")]
use super::internal::ProgressSchemaUnchecked;
use super::{
    ProgressCounter,
    ProgressMetric,
    ProgressSchemaError,
};

/// Metric dictionary for one logical progress operation.
///
/// A schema defines which metric ids are valid for the operation. Events carry
/// a schema so every serialized progress event is self-describing and reporters
/// can resolve metric ids to display names without external state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "ProgressSchemaUnchecked"))]
pub struct ProgressSchema {
    /// Metric definitions available in this progress stream.
    metrics: Vec<ProgressMetric>,
}

impl ProgressSchema {
    /// Creates a schema from metric definitions.
    ///
    /// # Parameters
    ///
    /// * `metrics` - Metric definitions available to events using this schema.
    ///
    /// # Returns
    ///
    /// A schema containing the supplied metric definitions.
    ///
    /// # Panics
    ///
    /// Panics when more than one metric has the same identifier. Use
    /// [`Self::try_new`] to handle invalid input without panicking.
    #[inline]
    pub fn new(metrics: Vec<ProgressMetric>) -> Self {
        Self::try_new(metrics)
            .expect("progress schema must have unique metric ids")
    }

    /// Tries to create a schema from metric definitions.
    ///
    /// # Parameters
    ///
    /// * `metrics` - Metric definitions available to events using this schema.
    ///
    /// # Returns
    ///
    /// A validated schema containing the supplied metric definitions.
    ///
    /// # Errors
    ///
    /// Returns [`ProgressSchemaError::DuplicateMetricId`] when more than one
    /// metric has the same identifier.
    pub fn try_new(
        metrics: Vec<ProgressMetric>,
    ) -> Result<Self, ProgressSchemaError> {
        let mut seen = HashSet::with_capacity(metrics.len());
        for metric in &metrics {
            if !seen.insert(metric.id()) {
                return Err(ProgressSchemaError::DuplicateMetricId {
                    metric_id: metric.id().to_owned(),
                });
            }
        }
        Ok(Self { metrics })
    }

    /// Creates a schema containing one metric.
    ///
    /// # Parameters
    ///
    /// * `id` - Stable metric identifier.
    /// * `name` - Human-readable metric name.
    ///
    /// # Returns
    ///
    /// A schema with a single metric definition.
    #[inline]
    pub fn single(id: &str, name: &str) -> Self {
        Self::new(vec![ProgressMetric::new(id, name)])
    }

    /// Returns all metric definitions.
    ///
    /// # Returns
    ///
    /// A slice of metrics declared by this schema.
    #[inline]
    pub fn metrics(&self) -> &[ProgressMetric] {
        self.metrics.as_slice()
    }

    /// Finds a metric by id.
    ///
    /// # Parameters
    ///
    /// * `id` - Metric identifier to search for.
    ///
    /// # Returns
    ///
    /// `Some(metric)` when the schema contains the id, otherwise `None`.
    #[inline]
    pub fn metric(&self, id: &str) -> Option<&ProgressMetric> {
        self.metrics.iter().find(|metric| metric.id() == id)
    }

    /// Finds a metric display name by id.
    ///
    /// # Parameters
    ///
    /// * `id` - Metric identifier to search for.
    ///
    /// # Returns
    ///
    /// `Some(name)` when the schema contains the id, otherwise `None`.
    #[inline]
    pub fn metric_name(&self, id: &str) -> Option<&str> {
        self.metric(id).map(ProgressMetric::name)
    }

    /// Checks whether the schema contains a metric id.
    ///
    /// # Parameters
    ///
    /// * `id` - Metric identifier to test.
    ///
    /// # Returns
    ///
    /// `true` when this schema contains `id`.
    #[inline]
    pub fn contains_metric(&self, id: &str) -> bool {
        self.metric(id).is_some()
    }

    /// Validates one counter against this schema.
    ///
    /// # Parameters
    ///
    /// * `counter` - Counter to validate.
    ///
    /// # Returns
    ///
    /// `true` when `counter.metric_id()` exists in this schema.
    #[inline]
    pub fn validate_counter(&self, counter: &ProgressCounter) -> bool {
        self.contains_metric(counter.metric_id())
    }

    /// Validates counters against this schema.
    ///
    /// Validation is intentionally light-weight: every counter metric id must
    /// exist in the schema and each metric id may appear at most once in the
    /// event. Numeric relationships are not validated because many operations
    /// discover totals dynamically or apply retry and compensation logic.
    ///
    /// # Parameters
    ///
    /// * `counters` - Counters to validate.
    ///
    /// # Returns
    ///
    /// `true` when all counters reference declared metrics and no metric id is
    /// duplicated.
    pub fn validate_counters(&self, counters: &[ProgressCounter]) -> bool {
        let mut seen = HashSet::with_capacity(counters.len());
        counters.iter().all(|counter| {
            self.validate_counter(counter) && seen.insert(counter.metric_id())
        })
    }
}

#[cfg(feature = "serde")]
impl TryFrom<ProgressSchemaUnchecked> for ProgressSchema {
    type Error = ProgressSchemaError;

    /// Validates and converts an unchecked serialized schema.
    ///
    /// # Parameters
    ///
    /// * `unchecked` - Schema representation produced by deserialization.
    ///
    /// # Returns
    ///
    /// A validated schema.
    ///
    /// # Errors
    ///
    /// Returns an error when metric identifiers are duplicated.
    #[inline(always)]
    fn try_from(
        unchecked: ProgressSchemaUnchecked,
    ) -> Result<Self, Self::Error> {
        Self::try_new(unchecked.metrics)
    }
}

impl Default for ProgressSchema {
    /// Creates an empty schema.
    ///
    /// # Returns
    ///
    /// A schema without declared metrics.
    #[inline]
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
