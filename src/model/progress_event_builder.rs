// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::Duration;

use super::{
    ProgressCounter,
    ProgressEvent,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};

/// Builder for [`ProgressEvent`].
///
/// The builder keeps the common path compact by carrying the event schema and
/// letting callers append named metric counters with a closure.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use qubit_progress::{
///     ProgressEvent,
///     ProgressMetric,
///     ProgressPhase,
///     ProgressSchema,
/// };
///
/// let schema = ProgressSchema::new(vec![ProgressMetric::new("bytes", "Bytes")]);
/// let event = ProgressEvent::builder(schema)
///     .running()
///     .counter("bytes", |counter| counter.total(8).completed(3).active(1))
///     .stage_named("copy", "Copy files")
///     .elapsed(Duration::from_secs(2))
///     .build();
///
/// assert_eq!(event.phase(), ProgressPhase::Running);
/// assert_eq!(event.counter("bytes").map(|c| c.completed_count()), Some(3));
/// assert_eq!(event.stage().map(|stage| stage.id()), Some("copy"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressEventBuilder {
    /// Metric schema carried by the event being built.
    pub(crate) schema: ProgressSchema,
    /// Lifecycle phase of the event being built.
    pub(crate) phase: ProgressPhase,
    /// Metric counters for the event being built.
    pub(crate) counters: Vec<ProgressCounter>,
    /// Optional current stage.
    pub(crate) stage: Option<ProgressStage>,
    /// Monotonic elapsed duration.
    pub(crate) elapsed: Duration,
}

impl ProgressEventBuilder {
    /// Creates a builder for a schema.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the built event.
    ///
    /// # Returns
    ///
    /// A builder whose phase is [`ProgressPhase::Running`], elapsed duration is
    /// zero, and counters are empty.
    #[inline]
    pub fn new(schema: ProgressSchema) -> Self {
        Self {
            schema,
            phase: ProgressPhase::Running,
            counters: Vec::new(),
            stage: None,
            elapsed: Duration::ZERO,
        }
    }

    /// Configures the lifecycle phase.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    ///
    /// # Returns
    ///
    /// This builder with `phase` recorded.
    #[inline]
    #[must_use]
    pub const fn phase(mut self, phase: ProgressPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Configures the event as started.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Started`].
    #[inline]
    #[must_use]
    pub const fn started(self) -> Self {
        self.phase(ProgressPhase::Started)
    }

    /// Configures the event as running.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Running`].
    #[inline]
    #[must_use]
    pub const fn running(self) -> Self {
        self.phase(ProgressPhase::Running)
    }

    /// Configures the event as finished.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Finished`].
    #[inline]
    #[must_use]
    pub const fn finished(self) -> Self {
        self.phase(ProgressPhase::Finished)
    }

    /// Configures the event as failed.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Failed`].
    #[inline]
    #[must_use]
    pub const fn failed(self) -> Self {
        self.phase(ProgressPhase::Failed)
    }

    /// Configures the event as canceled.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Canceled`].
    #[inline]
    #[must_use]
    pub const fn canceled(self) -> Self {
        self.phase(ProgressPhase::Canceled)
    }

    /// Replaces the current counter list.
    ///
    /// # Parameters
    ///
    /// * `counters` - Complete counter list to carry in the built event.
    ///
    /// # Returns
    ///
    /// This builder with `counters` recorded.
    #[inline]
    #[must_use]
    pub fn counters(mut self, counters: Vec<ProgressCounter>) -> Self {
        self.counters = counters;
        self
    }

    /// Appends one configured counter.
    ///
    /// # Parameters
    ///
    /// * `metric_id` - Metric identifier for the counter.
    /// * `configure` - Closure that fills the counter values.
    ///
    /// # Returns
    ///
    /// This builder with the configured counter appended.
    #[inline]
    #[must_use]
    pub fn counter<F>(mut self, metric_id: &str, configure: F) -> Self
    where
        F: FnOnce(ProgressCounter) -> ProgressCounter,
    {
        self.counters
            .push(configure(ProgressCounter::new(metric_id)));
        self
    }

    /// Appends a prebuilt counter.
    ///
    /// # Parameters
    ///
    /// * `counter` - Counter to append.
    ///
    /// # Returns
    ///
    /// This builder with `counter` appended.
    #[inline]
    #[must_use]
    pub fn add_counter(mut self, counter: ProgressCounter) -> Self {
        self.counters.push(counter);
        self
    }

    /// Configures the current stage.
    ///
    /// # Parameters
    ///
    /// * `stage` - Stage metadata to carry in the built event.
    ///
    /// # Returns
    ///
    /// This builder with `stage` recorded.
    #[inline]
    #[must_use]
    pub fn stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Configures the current stage from an id and display name.
    ///
    /// # Parameters
    ///
    /// * `id` - Stable machine-readable stage identifier.
    /// * `name` - Human-readable stage name.
    ///
    /// # Returns
    ///
    /// This builder with a stage created from `id` and `name`.
    #[inline]
    #[must_use]
    pub fn stage_named(self, id: &str, name: &str) -> Self {
        self.stage(ProgressStage::new(id, name))
    }

    /// Configures the elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Monotonic elapsed duration to carry in the event.
    ///
    /// # Returns
    ///
    /// This builder with `elapsed` recorded.
    #[inline]
    #[must_use]
    pub const fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = elapsed;
        self
    }

    /// Builds the progress event.
    ///
    /// # Returns
    ///
    /// An immutable [`ProgressEvent`] with the configured values.
    #[inline]
    pub fn build(self) -> ProgressEvent {
        ProgressEvent::new(self)
    }
}
