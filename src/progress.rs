/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::{
    thread,
    time::{
        Duration,
        Instant,
    },
};

use crate::{
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressEventBuilder,
        ProgressMetric,
        ProgressPhase,
        ProgressSchema,
        ProgressStage,
    },
    reporter::ProgressReporter,
    running::{
        RunningProgressGuard,
        RunningProgressLoop,
    },
};

/// Tracks one logical progress-producing operation and reports events.
///
/// A `Progress` instance is scoped to one logical operation. It owns no domain
/// state; callers keep their own counters and convert them into
/// [`ProgressCounter`] values when reporting. The run manages elapsed time,
/// periodic running-event throttling, optional stage metadata, a metric schema,
/// and forwarding immutable events to one reporter.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use qubit_progress::{
///     Progress,
///     ProgressMetric,
///     ProgressSchema,
///     WriterProgressReporter,
/// };
///
/// let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
/// let reporter = WriterProgressReporter::from_writer(std::io::stdout());
/// let mut progress = Progress::new(&reporter, Duration::from_secs(5), schema);
///
/// let started = progress.report_started(|event| event.counter("entries", |c| c.total(2)));
/// assert!(started.elapsed().is_zero());
///
/// let _reported = progress.report_running_if_due(|event| {
///     event.counter("entries", |counter| counter.total(2).completed(1).active(1))
/// });
///
/// let finished = progress.report_finished(|event| {
///     event.counter("entries", |counter| counter.total(2).completed(2).succeeded(2))
/// });
/// assert!(finished.elapsed() >= started.elapsed());
/// ```
pub struct Progress<'a> {
    /// Reporter receiving lifecycle callbacks for this run.
    reporter: &'a dyn ProgressReporter,
    /// Metric schema carried by events emitted from this run.
    schema: ProgressSchema,
    /// Monotonic start time used to compute elapsed durations.
    started_at: Instant,
    /// Minimum interval between due-based running callbacks.
    report_interval: Duration,
    /// Next monotonic instant at which a due-based running callback may fire.
    next_running_at: Instant,
    /// Optional stage metadata attached to every event emitted by this run.
    stage: Option<ProgressStage>,
}

impl<'a> Progress<'a> {
    /// Creates a progress run starting at the current instant.
    ///
    /// # Parameters
    ///
    /// * `reporter` - Reporter receiving progress events.
    /// * `report_interval` - Minimum delay between due-based running events.
    /// * `schema` - Metric schema carried by emitted events.
    ///
    /// # Returns
    ///
    /// A progress run whose elapsed time is measured from now.
    #[inline]
    pub fn new(reporter: &'a dyn ProgressReporter, report_interval: Duration, schema: ProgressSchema) -> Self {
        Self::from_start(reporter, report_interval, schema, Instant::now())
    }

    /// Creates a single-metric progress run starting at the current instant.
    ///
    /// # Parameters
    ///
    /// * `reporter` - Reporter receiving progress events.
    /// * `report_interval` - Minimum delay between due-based running events.
    /// * `metric_id` - Stable metric identifier.
    /// * `metric_name` - Human-readable metric name.
    ///
    /// # Returns
    ///
    /// A progress run with a schema containing one metric.
    #[inline]
    pub fn single_metric(
        reporter: &'a dyn ProgressReporter,
        report_interval: Duration,
        metric_id: &str,
        metric_name: &str,
    ) -> Self {
        Self::new(
            reporter,
            report_interval,
            ProgressSchema::new(vec![ProgressMetric::new(metric_id, metric_name)]),
        )
    }

    /// Creates a progress run from an explicit start instant.
    ///
    /// # Parameters
    ///
    /// * `reporter` - Reporter receiving progress events.
    /// * `report_interval` - Minimum delay between due-based running events.
    /// * `schema` - Metric schema carried by emitted events.
    /// * `started_at` - Monotonic instant representing operation start.
    ///
    /// # Returns
    ///
    /// A progress run using `started_at` for elapsed-time calculations.
    #[inline]
    fn from_start(
        reporter: &'a dyn ProgressReporter,
        report_interval: Duration,
        schema: ProgressSchema,
        started_at: Instant,
    ) -> Self {
        Self {
            reporter,
            schema,
            started_at,
            report_interval,
            next_running_at: next_instant(started_at, report_interval),
            stage: None,
        }
    }

    /// Returns a copy configured with stage metadata.
    ///
    /// # Parameters
    ///
    /// * `stage` - Stage metadata attached to subsequently reported events.
    ///
    /// # Returns
    ///
    /// This progress run with `stage` recorded.
    #[inline]
    #[must_use]
    pub fn with_stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Returns a copy with stage metadata removed.
    ///
    /// # Returns
    ///
    /// This progress run without stage metadata.
    #[inline]
    #[must_use]
    pub fn without_stage(mut self) -> Self {
        self.stage = None;
        self
    }

    /// Creates an event builder preconfigured with this run's schema, stage, and elapsed time.
    ///
    /// # Returns
    ///
    /// A progress event builder for this run.
    #[inline]
    pub fn event_builder(&self) -> ProgressEventBuilder {
        self.event_builder_with_elapsed(self.elapsed())
    }

    /// Reports a started lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds counters or stage overrides to the event builder.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_started<F>(&self, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        self.report_with_elapsed(ProgressPhase::Started, Duration::ZERO, configure)
    }

    /// Reports a running lifecycle event immediately.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds counters or stage overrides to the event builder.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    pub fn report_running<F>(&mut self, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        let now = Instant::now();
        let event = self.report_with_elapsed(
            ProgressPhase::Running,
            now.saturating_duration_since(self.started_at),
            configure,
        );
        self.next_running_at = next_instant(now, self.report_interval);
        event
    }

    /// Reports a running lifecycle event if the configured interval has passed.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds counters or stage overrides when an event is due.
    ///
    /// # Returns
    ///
    /// `Some(event)` when a running event was emitted, or `None` when the next
    /// running-event deadline has not been reached.
    ///
    /// This method does not call `configure` unless an event is due. It returns
    /// immediately when not due, and when due it synchronously calls the
    /// configured reporter. Any blocking behavior therefore comes from the
    /// reporter implementation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter when an event is due.
    pub fn report_running_if_due<F>(&mut self, configure: F) -> Option<ProgressEvent>
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        let now = Instant::now();
        if now < self.next_running_at {
            return None;
        }
        let event = self.report_with_elapsed(
            ProgressPhase::Running,
            now.saturating_duration_since(self.started_at),
            configure,
        );
        self.next_running_at = next_instant(now, self.report_interval);
        Some(event)
    }

    /// Reports a finished lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds final counters to the event builder.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_finished<F>(&self, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        self.report_with_elapsed(ProgressPhase::Finished, self.elapsed(), configure)
    }

    /// Reports a failed lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds final or current counters to the event builder.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_failed<F>(&self, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        self.report_with_elapsed(ProgressPhase::Failed, self.elapsed(), configure)
    }

    /// Reports a canceled lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `configure` - Closure that adds final or current counters to the event builder.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_canceled<F>(&self, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        self.report_with_elapsed(ProgressPhase::Canceled, self.elapsed(), configure)
    }

    /// Spawns a scoped background reporter for periodic running events.
    ///
    /// The background reporter shares this progress run's reporter, schema,
    /// start time, interval, and stage metadata. Worker threads should update
    /// their own domain state first, then call
    /// [`RunningProgressPointHandle::report`](crate::RunningProgressPointHandle::report)
    /// on the handle returned by the guard. The guard must be stopped and
    /// joined before the thread scope exits.
    ///
    /// # Parameters
    ///
    /// * `scope` - Thread scope that owns the background reporter thread.
    /// * `snapshot` - Closure that builds fresh counters from caller-owned
    ///   domain state whenever a running event may be due.
    ///
    /// # Returns
    ///
    /// A guard that owns the reporter thread and can create worker-side point handles.
    ///
    /// # Panics
    ///
    /// Panics raised by the reporter thread are propagated by
    /// [`RunningProgressGuard::stop_and_join`].
    pub fn spawn_running_reporter<'scope, 'env, F>(
        &self,
        scope: &'scope thread::Scope<'scope, 'env>,
        snapshot: F,
    ) -> RunningProgressGuard<'scope>
    where
        'a: 'scope,
        F: FnMut() -> Vec<ProgressCounter> + Send + 'scope,
    {
        RunningProgressLoop::spawn_scoped(scope, self.fork_for_running(), snapshot)
    }

    /// Reports a lifecycle event with an explicit elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    /// * `elapsed` - Elapsed duration carried by the event.
    /// * `configure` - Closure that adds counters or stage overrides.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    fn report_with_elapsed<F>(&self, phase: ProgressPhase, elapsed: Duration, configure: F) -> ProgressEvent
    where
        F: FnOnce(ProgressEventBuilder) -> ProgressEventBuilder,
    {
        let event = configure(self.event_builder_with_elapsed(elapsed).phase(phase)).build();
        self.reporter.report(&event);
        event
    }

    /// Returns the metric schema for this progress run.
    ///
    /// # Returns
    ///
    /// The schema cloned into every event emitted by this run.
    #[inline]
    pub const fn schema(&self) -> &ProgressSchema {
        &self.schema
    }

    /// Returns the elapsed duration since this run started.
    ///
    /// # Returns
    ///
    /// The monotonic elapsed duration for this progress run.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Returns the start instant for this run.
    ///
    /// # Returns
    ///
    /// The monotonic instant used as this run's start time.
    #[inline]
    pub const fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Returns the configured running-event interval.
    ///
    /// # Returns
    ///
    /// The minimum delay between due-based running events.
    #[inline]
    pub const fn report_interval(&self) -> Duration {
        self.report_interval
    }

    /// Returns the optional stage metadata attached to events.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when stage metadata is configured, otherwise `None`.
    #[inline]
    pub const fn stage(&self) -> Option<&ProgressStage> {
        self.stage.as_ref()
    }

    /// Creates a background-thread copy that reports running events for this run.
    ///
    /// # Returns
    ///
    /// A progress run with the same reporter, schema, start time, interval,
    /// stage, and next running deadline as this run.
    fn fork_for_running(&self) -> Self {
        Self {
            reporter: self.reporter,
            schema: self.schema.clone(),
            started_at: self.started_at,
            report_interval: self.report_interval,
            next_running_at: self.next_running_at,
            stage: self.stage.clone(),
        }
    }

    /// Creates an event builder with a specific elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Elapsed duration to attach to the event.
    ///
    /// # Returns
    ///
    /// A builder carrying this run's schema and optional stage.
    fn event_builder_with_elapsed(&self, elapsed: Duration) -> ProgressEventBuilder {
        let builder = ProgressEvent::builder(self.schema.clone()).elapsed(elapsed);
        match self.stage.clone() {
            Some(stage) => builder.stage(stage),
            None => builder,
        }
    }
}

/// Computes the next reporting instant while avoiding overflow panics.
///
/// # Parameters
///
/// * `base` - Base instant for the deadline.
/// * `interval` - Duration added to `base`.
///
/// # Returns
///
/// `base + interval`, or `base` when the addition overflows.
fn next_instant(base: Instant, interval: Duration) -> Instant {
    base.checked_add(interval).unwrap_or(base)
}
