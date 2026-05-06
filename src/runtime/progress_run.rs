/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::{
    Duration,
    Instant,
};

use crate::{
    model::{
        ProgressCounters,
        ProgressEvent,
        ProgressPhase,
        ProgressStage,
    },
    reporter::ProgressReporter,
};

/// Tracks one progress-producing operation and reports lifecycle events.
///
/// `ProgressRun` owns no operation-specific counters. Callers keep their own
/// domain state and pass freshly built [`ProgressCounters`] when reporting.
/// The run only manages elapsed time, periodic running-event throttling,
/// optional stage metadata, and forwarding immutable events to a reporter.
pub struct ProgressRun<'a> {
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

impl<'a> ProgressRun<'a> {
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
    pub fn from_start(
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
    pub fn report_started(&self, counters: ProgressCounters) {
        self.report(ProgressPhase::Started, counters);
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
    #[inline]
    pub fn report_running(&self, counters: ProgressCounters) {
        self.report(ProgressPhase::Running, counters);
    }

    /// Reports a running lifecycle event if the configured interval has passed.
    ///
    /// # Parameters
    ///
    /// * `counters` - Current counters for the operation.
    ///
    /// # Returns
    ///
    /// `true` when a running event was emitted, or `false` when the next
    /// running-event deadline has not been reached.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter when an event is due.
    pub fn report_running_if_due(&mut self, counters: ProgressCounters) -> bool {
        let now = Instant::now();
        if now < self.next_running_at {
            return false;
        }
        self.report_with_elapsed(
            ProgressPhase::Running,
            counters,
            now.saturating_duration_since(self.started_at),
        );
        self.next_running_at = next_instant(now, self.report_interval);
        true
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
    pub fn report_finished(&self, counters: ProgressCounters) {
        self.report(ProgressPhase::Finished, counters);
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
    pub fn report_failed(&self, counters: ProgressCounters) {
        self.report(ProgressPhase::Failed, counters);
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
    pub fn report_canceled(&self, counters: ProgressCounters) {
        self.report(ProgressPhase::Canceled, counters);
    }

    /// Reports a lifecycle event with the run's current elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    /// * `counters` - Counters carried by the event.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    #[inline]
    pub fn report(&self, phase: ProgressPhase, counters: ProgressCounters) {
        self.report_with_elapsed(phase, counters, self.elapsed());
    }

    /// Reports a lifecycle event with an explicit elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    /// * `counters` - Counters carried by the event.
    /// * `elapsed` - Elapsed duration carried by the event.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter.
    pub fn report_with_elapsed(
        &self,
        phase: ProgressPhase,
        counters: ProgressCounters,
        elapsed: Duration,
    ) {
        let event = self.event_with_elapsed(phase, counters, elapsed);
        self.reporter.report(&event);
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
