/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use serde::{
    Deserialize,
    Serialize,
};

use super::{
    ProgressCounter,
    ProgressEventBuilder,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};

/// Immutable progress event delivered to reporters.
///
/// Each event carries its [`ProgressSchema`], making serialized events
/// self-describing for logs, databases, and agent-readable JSON streams.
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
/// let schema = ProgressSchema::new(vec![
///     ProgressMetric::new("entries", "Entries"),
///     ProgressMetric::new("bytes", "Bytes"),
/// ]);
/// let event = ProgressEvent::builder(schema)
///     .running()
///     .counter("entries", |counter| counter.total(5).completed(2))
///     .counter("bytes", |counter| counter.total(500).completed(200))
///     .elapsed(Duration::from_millis(500))
///     .build();
///
/// assert_eq!(event.phase(), ProgressPhase::Running);
/// assert_eq!(event.counter("entries").map(|c| c.completed_count()), Some(2));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Metric schema that describes every counter in this event.
    schema: ProgressSchema,
    /// Lifecycle phase of the reported operation.
    phase: ProgressPhase,
    /// Optional current stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<ProgressStage>,
    /// Metric counters for this event.
    counters: Vec<ProgressCounter>,
    /// Monotonic elapsed duration.
    #[serde(with = "qubit_serde::serde::duration_with_unit")]
    elapsed: Duration,
}

impl ProgressEvent {
    /// Creates a progress event builder for a schema.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the built event.
    ///
    /// # Returns
    ///
    /// A builder initialized as running progress with no counters and zero
    /// elapsed time.
    #[inline]
    pub fn builder(schema: ProgressSchema) -> ProgressEventBuilder {
        ProgressEventBuilder::new(schema)
    }

    /// Creates a progress event from a builder.
    ///
    /// # Parameters
    ///
    /// * `builder` - Builder containing configured event fields.
    ///
    /// # Returns
    ///
    /// A progress event built from `builder`.
    #[inline]
    pub fn new(builder: ProgressEventBuilder) -> Self {
        Self {
            schema: builder.schema,
            phase: builder.phase,
            stage: builder.stage,
            counters: builder.counters,
            elapsed: builder.elapsed,
        }
    }

    /// Creates a progress event for the supplied lifecycle phase.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `phase` - Lifecycle phase for the event.
    /// * `counters` - Metric counters carried by the event.
    /// * `elapsed` - Elapsed duration carried by the event.
    ///
    /// # Returns
    ///
    /// A progress event with `schema`, `phase`, `counters`, and `elapsed`.
    #[inline]
    pub fn from_phase(
        schema: ProgressSchema,
        phase: ProgressPhase,
        counters: Vec<ProgressCounter>,
        elapsed: Duration,
    ) -> Self {
        Self::builder(schema)
            .phase(phase)
            .counters(counters)
            .elapsed(elapsed)
            .build()
    }

    /// Creates a started progress event.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `counters` - Initial progress counters.
    /// * `elapsed` - Elapsed duration at start, usually zero.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Started`].
    #[inline]
    pub fn started(schema: ProgressSchema, counters: Vec<ProgressCounter>, elapsed: Duration) -> Self {
        Self::from_phase(schema, ProgressPhase::Started, counters, elapsed)
    }

    /// Creates a running progress event.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `counters` - Current progress counters.
    /// * `elapsed` - Elapsed duration since operation start.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Running`].
    #[inline]
    pub fn running(schema: ProgressSchema, counters: Vec<ProgressCounter>, elapsed: Duration) -> Self {
        Self::from_phase(schema, ProgressPhase::Running, counters, elapsed)
    }

    /// Creates a finished progress event.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `counters` - Final progress counters.
    /// * `elapsed` - Total elapsed duration.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Finished`].
    #[inline]
    pub fn finished(schema: ProgressSchema, counters: Vec<ProgressCounter>, elapsed: Duration) -> Self {
        Self::from_phase(schema, ProgressPhase::Finished, counters, elapsed)
    }

    /// Creates a failed progress event.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at failure.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Failed`].
    #[inline]
    pub fn failed(schema: ProgressSchema, counters: Vec<ProgressCounter>, elapsed: Duration) -> Self {
        Self::from_phase(schema, ProgressPhase::Failed, counters, elapsed)
    }

    /// Creates a canceled progress event.
    ///
    /// # Parameters
    ///
    /// * `schema` - Metric schema carried by the event.
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at cancellation.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Canceled`].
    #[inline]
    pub fn canceled(schema: ProgressSchema, counters: Vec<ProgressCounter>, elapsed: Duration) -> Self {
        Self::from_phase(schema, ProgressPhase::Canceled, counters, elapsed)
    }

    /// Returns a copy configured with the current stage.
    ///
    /// # Parameters
    ///
    /// * `stage` - Current operation stage.
    ///
    /// # Returns
    ///
    /// This event with `stage` recorded.
    #[inline]
    #[must_use]
    pub fn with_stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Returns the event schema.
    ///
    /// # Returns
    ///
    /// The metric schema carried by this event.
    #[inline]
    pub const fn schema(&self) -> &ProgressSchema {
        &self.schema
    }

    /// Returns the event phase.
    ///
    /// # Returns
    ///
    /// The lifecycle phase carried by this event.
    #[inline]
    pub const fn phase(&self) -> ProgressPhase {
        self.phase
    }

    /// Returns the current stage when known.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when this event carries stage information, otherwise
    /// `None`.
    #[inline]
    pub const fn stage(&self) -> Option<&ProgressStage> {
        self.stage.as_ref()
    }

    /// Returns the progress counters.
    ///
    /// # Returns
    ///
    /// The counters carried by this event.
    #[inline]
    pub fn counters(&self) -> &[ProgressCounter] {
        self.counters.as_slice()
    }

    /// Finds a counter by metric id.
    ///
    /// # Parameters
    ///
    /// * `metric_id` - Metric identifier to search for.
    ///
    /// # Returns
    ///
    /// `Some(counter)` when the event contains a matching counter, otherwise
    /// `None`.
    #[inline]
    pub fn counter(&self, metric_id: &str) -> Option<&ProgressCounter> {
        self.counters.iter().find(|counter| counter.metric_id() == metric_id)
    }

    /// Creates metric snapshots for all counters in this event.
    ///
    /// Each snapshot flattens one counter with the event phase, stage, elapsed
    /// duration, and complete metric metadata. If a counter references a metric
    /// id that is not present in the schema, the snapshot uses the metric id as
    /// both the fallback id and display name.
    ///
    /// # Returns
    ///
    /// One snapshot per counter carried by this event.
    pub fn metric_snapshots(&self) -> Vec<ProgressMetricSnapshot> {
        self.counters
            .iter()
            .map(|counter| {
                let metric = self
                    .schema
                    .metric(counter.metric_id())
                    .cloned()
                    .unwrap_or_else(|| ProgressMetric::new(counter.metric_id(), counter.metric_id()));
                ProgressMetricSnapshot::new(metric, self.phase, self.stage.clone(), counter, self.elapsed)
            })
            .collect()
    }

    /// Returns the elapsed duration.
    ///
    /// # Returns
    ///
    /// The monotonic elapsed duration carried by this event.
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}
