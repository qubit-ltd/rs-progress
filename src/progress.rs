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
        ProgressCounters,
        ProgressEvent,
        ProgressPhase,
        ProgressStage,
    },
    reporter::ProgressReporter,
    running::{
        RunningProgressGuard,
        RunningProgressLoop,
    },
};

/// Tracks one progress-producing operation and reports lifecycle events.
///
/// `Progress` owns no operation-specific counters. Callers keep their own
/// domain state and pass freshly built [`ProgressCounters`] when reporting.
/// The run only manages elapsed time, periodic running-event throttling,
/// optional stage metadata, and forwarding immutable events to a reporter.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use qubit_progress::{
///     ProgressCounters,
///     Progress,
///     WriterProgressReporter,
/// };
///
/// let reporter = WriterProgressReporter::from_writer(std::io::stdout());
/// let mut progress = Progress::new(&reporter, Duration::from_secs(5));
///
/// let started = progress.report_started(ProgressCounters::new(Some(2)));
/// assert!(started.elapsed().is_zero());
///
/// let running = ProgressCounters::new(Some(2))
///     .with_completed_count(1)
///     .with_active_count(1);
/// let _reported = progress.report_running_if_due(running);
///
/// let finished = ProgressCounters::new(Some(2))
///     .with_completed_count(2)
///     .with_succeeded_count(2);
/// let finished_event = progress.report_finished(finished);
/// assert!(finished_event.elapsed() >= started.elapsed());
/// ```
pub struct Progress<'a> {
    /// Reporter receiving lifecycle callbacks for this run.
    reporter: &'a dyn ProgressReporter,
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
    ///
    /// # Returns
    ///
    /// A progress run whose elapsed time is measured from now.
    #[inline]
    pub fn new(reporter: &'a dyn ProgressReporter, report_interval: Duration) -> Self {
        Self::from_start(reporter, report_interval, Instant::now())
    }

    /// Creates a progress run from an explicit start instant.
    ///
    /// # Parameters
    ///
    /// * `reporter` - Reporter receiving progress events.
    /// * `report_interval` - Minimum delay between due-based running events.
    /// * `started_at` - Monotonic instant representing operation start.
    ///
    /// # Returns
    ///
    /// A progress run using `started_at` for elapsed-time calculations.
    #[inline]
    fn from_start(
        reporter: &'a dyn ProgressReporter,
        report_interval: Duration,
        started_at: Instant,
    ) -> Self {
        Self {
            reporter,
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
    pub fn without_stage(mut self) -> Self {
        self.stage = None;
        self
    }

    /// Reports a started lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Initial counters for the operation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_started(&self, counters: ProgressCounters) -> ProgressEvent {
        self.report_with_elapsed(ProgressPhase::Started, counters, Duration::ZERO)
    }

    /// Reports a running lifecycle event immediately.
    ///
    /// # Parameters
    ///
    /// * `counters` - Current counters for the operation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    pub fn report_running(&mut self, counters: ProgressCounters) -> ProgressEvent {
        let now = Instant::now();
        let event = self.report_with_elapsed(
            ProgressPhase::Running,
            counters,
            now.saturating_duration_since(self.started_at),
        );
        self.next_running_at = next_instant(now, self.report_interval);
        event
    }

    /// Reports a running lifecycle event if the configured interval has passed.
    ///
    /// # Parameters
    ///
    /// * `counters` - Current counters for the operation.
    ///
    /// # Returns
    ///
    /// `Some(event)` when a running event was emitted, or `None` when the next
    /// running-event deadline has not been reached.
    ///
    /// This method does not block waiting for the next deadline. It returns
    /// immediately when not due, and when due it synchronously calls the
    /// configured reporter. Any blocking behavior therefore comes from the
    /// reporter implementation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter when an event is due.
    pub fn report_running_if_due(&mut self, counters: ProgressCounters) -> Option<ProgressEvent> {
        let now = Instant::now();
        if now < self.next_running_at {
            return None;
        }
        let event = self.report_with_elapsed(
            ProgressPhase::Running,
            counters,
            now.saturating_duration_since(self.started_at),
        );
        self.next_running_at = next_instant(now, self.report_interval);
        Some(event)
    }

    /// Reports a finished lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final counters for a successfully completed operation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_finished(&self, counters: ProgressCounters) -> ProgressEvent {
        self.report_with_elapsed(ProgressPhase::Finished, counters, self.elapsed())
    }

    /// Reports a failed lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current counters for a failed operation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_failed(&self, counters: ProgressCounters) -> ProgressEvent {
        self.report_with_elapsed(ProgressPhase::Failed, counters, self.elapsed())
    }

    /// Reports a canceled lifecycle event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current counters for a canceled operation.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report_canceled(&self, counters: ProgressCounters) -> ProgressEvent {
        self.report_with_elapsed(ProgressPhase::Canceled, counters, self.elapsed())
    }

    /// Spawns a scoped background reporter for periodic running events.
    ///
    /// The background reporter shares this progress run's reporter, start time,
    /// interval, and stage metadata. Worker threads should update their own
    /// domain state first, then call
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
    /// A guard that owns the background reporter thread and can create
    /// worker-side point handles.
    ///
    /// # Panics
    ///
    /// Panics raised by the reporter thread are propagated by
    /// [`RunningProgressGuard::stop_and_join`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{
    ///     sync::{
    ///         Arc,
    ///         atomic::{
    ///             AtomicUsize,
    ///             Ordering,
    ///         },
    ///     },
    ///     thread,
    ///     time::Duration,
    /// };
    ///
    /// use qubit_progress::{
    ///     NoOpProgressReporter,
    ///     Progress,
    ///     ProgressCounters,
    /// };
    ///
    /// let reporter = NoOpProgressReporter;
    /// let completed = Arc::new(AtomicUsize::new(0));
    /// let progress = Progress::new(&reporter, Duration::ZERO);
    ///
    /// thread::scope(|scope| {
    ///     let loop_completed = Arc::clone(&completed);
    ///     let running = progress.spawn_running_reporter(scope, move || {
    ///         ProgressCounters::new(Some(1))
    ///             .with_completed_count(loop_completed.load(Ordering::Acquire))
    ///     });
    ///     let point = running.point_handle();
    ///
    ///     completed.store(1, Ordering::Release);
    ///     assert!(point.report());
    ///
    ///     running.stop_and_join();
    /// });
    /// ```
    pub fn spawn_running_reporter<'scope, 'env, F>(
        &self,
        scope: &'scope thread::Scope<'scope, 'env>,
        snapshot: F,
    ) -> RunningProgressGuard<'scope>
    where
        'a: 'scope,
        F: FnMut() -> ProgressCounters + Send + 'scope,
    {
        RunningProgressLoop::spawn_scoped(scope, self.fork_for_running(), snapshot)
    }

    /// Reports a lifecycle event with an explicit elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    /// * `counters` - Counters carried by the event.
    /// * `elapsed` - Elapsed duration carried by the event.
    ///
    /// # Returns
    ///
    /// The event sent to the configured reporter.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    fn report_with_elapsed(
        &self,
        phase: ProgressPhase,
        counters: ProgressCounters,
        elapsed: Duration,
    ) -> ProgressEvent {
        let event = self.event_with_elapsed(phase, counters, elapsed);
        self.reporter.report(&event);
        event
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
    /// A progress run with the same reporter, start time, interval, stage, and
    /// next running deadline as this run.
    fn fork_for_running(&self) -> Self {
        Self {
            reporter: self.reporter,
            started_at: self.started_at,
            report_interval: self.report_interval,
            next_running_at: self.next_running_at,
            stage: self.stage.clone(),
        }
    }

    /// Builds a progress event with optional stage metadata.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase for the event.
    /// * `counters` - Counters carried by the event.
    /// * `elapsed` - Elapsed duration carried by the event.
    ///
    /// # Returns
    ///
    /// A progress event ready to be sent to the reporter.
    fn event_with_elapsed(
        &self,
        phase: ProgressPhase,
        counters: ProgressCounters,
        elapsed: Duration,
    ) -> ProgressEvent {
        let event = ProgressEvent::from_phase(phase, counters, elapsed);
        match self.stage.clone() {
            Some(stage) => event.with_stage(stage),
            None => event,
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
