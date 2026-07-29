// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Operation lifecycle, metric state and report scheduling.
// qubit-style: allow multiple-public-types

use std::{
    sync::Arc,
    sync::atomic::{
        AtomicBool,
        AtomicU64,
        Ordering,
    },
    time::{
        Duration,
        Instant,
    },
};

use crate::{
    Event,
    Metric,
    MetricHandle,
    MetricSnapshot,
    Phase,
    ProgressError,
    Reporter,
    Stage,
    TerminalError,
    ValidationError,
    auto_reporter::{
        self,
        AutoReporter,
    },
    validation::{
        validate_metrics,
        validate_stage,
    },
};

/// Process-local source of nonzero operation identifiers.
static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Configures one [`Progress`] operation before it starts.
pub struct ProgressBuilder<'reporter> {
    /// Reporter receiving complete events.
    reporter: &'reporter dyn Reporter,
    /// Minimum interval between due reports.
    interval: Duration,
    /// Stable operation metrics.
    metrics: Vec<Metric>,
    /// Optional initial stage.
    stage: Option<Stage>,
}

impl<'reporter> ProgressBuilder<'reporter> {
    /// Sets the minimum interval between due-based running reports.
    #[must_use]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
    /// Adds one stable metric to the operation.
    #[must_use]
    pub fn metric(mut self, metric: Metric) -> Self {
        self.metrics.push(metric);
        self
    }
    /// Adds stage metadata to the Started event and subsequent events.
    #[must_use]
    pub fn stage(mut self, stage: Stage) -> Self {
        self.stage = Some(stage);
        self
    }
    /// Validates configuration, samples enablement and emits Started when
    /// enabled.
    ///
    /// Returns [`ProgressError::Validation`] for invalid fixed metadata and
    /// [`ProgressError::Report`] when the reporter rejects Started.
    pub fn start(self) -> Result<Progress<'reporter>, ProgressError> {
        validate_metrics(&self.metrics)?;
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }
        let enabled = self.reporter.is_enabled();
        let started_at = Instant::now();
        let operation_open = Arc::new(AtomicBool::new(true));
        let mut progress = Progress {
            reporter: self.reporter,
            enabled,
            metrics: self
                .metrics
                .into_iter()
                .map(|metric| {
                    MetricHandle::new(metric, Arc::clone(&operation_open))
                })
                .collect(),
            operation_open,
            stage: self.stage,
            interval: self.interval,
            started_at,
            next_running_at: next_deadline(started_at, self.interval),
            operation_id: enabled.then(allocate_operation_id).transpose()?,
            next_sequence: 0,
        };
        if enabled {
            let metrics = progress.metric_snapshots()?;
            progress.send(Phase::Started, metrics, Duration::ZERO)?;
        }
        Ok(progress)
    }
}

/// One started progress operation.
///
/// Terminal methods consume this value, preventing reports after a terminal
/// phase and preventing duplicate terminal events in safe Rust.
#[must_use]
pub struct Progress<'reporter> {
    /// Reporter selected by the builder.
    reporter: &'reporter dyn Reporter,
    /// Stable enablement sampled once at start.
    enabled: bool,
    /// Live metrics carried by each event.
    metrics: Vec<MetricHandle>,
    /// Shared gate that prevents handle updates after operation closure.
    operation_open: Arc<AtomicBool>,
    /// Optional current stage.
    stage: Option<Stage>,
    /// Minimum due-report spacing.
    interval: Duration,
    /// Monotonic operation start time.
    started_at: Instant,
    /// Next due running deadline for a positive interval.
    next_running_at: Option<Instant>,
    /// Nonzero identifier for enabled operations.
    operation_id: Option<u64>,
    /// Sequence reserved for the next event attempt.
    next_sequence: u64,
}

impl<'reporter> Progress<'reporter> {
    /// Creates a builder bound to one reporter.
    #[must_use]
    pub fn builder(
        reporter: &'reporter dyn Reporter,
    ) -> ProgressBuilder<'reporter> {
        ProgressBuilder {
            reporter,
            interval: Duration::ZERO,
            metrics: Vec::new(),
            stage: None,
        }
    }
    /// Returns enablement sampled when this operation started.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
    /// Returns monotonic elapsed time since `start()`.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
    /// Returns a cloneable live metric selected by its stable ID.
    pub fn metric(&self, metric_id: &str) -> Option<MetricHandle> {
        self.metrics
            .iter()
            .find(|metric| metric.id() == metric_id)
            .cloned()
    }
    /// Immediately emits a Running event from current metric state.
    ///
    /// # Errors
    ///
    /// Returns a metric-state or reporter delivery error. Disabled
    /// operations return success without sending an event.
    pub fn report(&mut self) -> Result<(), ProgressError> {
        if !self.enabled {
            return Ok(());
        }
        let metrics = self.metric_snapshots()?;
        let elapsed = self.elapsed();
        let result = self.send(Phase::Running, metrics, elapsed);
        self.reset_deadline();
        result
    }
    /// Emits a Running event only when the configured interval is due.
    ///
    /// A zero interval is always due. Disabled and not-yet-due operations
    /// return success without taking a metric snapshot.
    pub fn report_if_due(&mut self) -> Result<(), ProgressError> {
        if !self.enabled || !self.is_due() {
            return Ok(());
        }
        self.report()
    }
    /// Replaces stage metadata attached to subsequent events.
    ///
    /// Returns [`ProgressError::Validation`] for malformed stage metadata.
    pub fn set_stage(&mut self, stage: Stage) -> Result<(), ProgressError> {
        validate_stage(&stage)?;
        self.stage = Some(stage);
        Ok(())
    }
    /// Removes stage metadata from subsequent events.
    pub fn clear_stage(&mut self) {
        self.stage = None;
    }
    /// Consumes this operation and emits a successful terminal event.
    pub fn finish(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Succeeded)
    }
    /// Consumes this operation and emits a failed terminal event.
    pub fn fail(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Failed)
    }
    /// Consumes this operation and emits a cancelled terminal event.
    pub fn cancel(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Cancelled)
    }
    /// Spawns a scoped automatic Running reporter that exclusively borrows this
    /// operation.
    ///
    /// While the returned [`AutoReporter`] exists, Rust prevents manual
    /// reports, configuration changes and terminal delivery. Call
    /// [`AutoReporter::stop`] before sending a terminal event.
    pub fn spawn_auto_reporter<'scope, 'env>(
        &'scope mut self,
        scope: &'scope std::thread::Scope<'scope, 'env>,
    ) -> AutoReporter<'scope, 'reporter>
    where
        'reporter: 'scope,
    {
        auto_reporter::spawn(self, scope)
    }
    /// Copies each metric into one independently consistent event snapshot.
    fn metric_snapshots(&self) -> Result<Vec<MetricSnapshot>, ProgressError> {
        self.metrics
            .iter()
            .map(MetricHandle::snapshot)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgressError::from)
    }
    /// Delivers one complete event after reserving its delivery sequence.
    fn send(
        &mut self,
        phase: Phase,
        metrics: Vec<MetricSnapshot>,
        elapsed: Duration,
    ) -> Result<(), ProgressError> {
        let operation_id = self
            .operation_id
            .ok_or(ValidationError::OperationIdExhausted)?;
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or(ValidationError::SequenceExhausted)?;
        let event = Event::new(
            operation_id,
            sequence,
            phase,
            self.stage.clone(),
            metrics,
            elapsed,
        );
        self.reporter.report(&event).map_err(ProgressError::from)
    }
    /// Tests whether a due-based running report can run now.
    fn is_due(&self) -> bool {
        self.interval.is_zero()
            || self
                .next_running_at
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
    /// Returns the configured interval to the crate-private background loop.
    pub(crate) const fn report_interval(&self) -> Duration {
        self.interval
    }
    /// Returns how long the background loop should wait for the next deadline.
    pub(crate) fn time_until_due(&self) -> Duration {
        self.next_running_at
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::MAX)
    }
    /// Pushes the next positive-interval deadline after a running attempt.
    fn reset_deadline(&mut self) {
        self.next_running_at = next_deadline(Instant::now(), self.interval);
    }
    /// Emits one terminal phase while retaining elapsed time on failure.
    fn terminal(mut self, phase: Phase) -> Result<Duration, TerminalError> {
        let elapsed = self.elapsed();
        self.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.metric_snapshots()
            .and_then(|metrics| self.send(phase, metrics, elapsed))
            .map(|()| elapsed)
            .map_err(|error| TerminalError::new(elapsed, error))
    }

    /// Closes all live metric handles owned by this operation.
    fn close(&self) {
        self.operation_open.store(false, Ordering::Release);
    }
}

impl Drop for Progress<'_> {
    /// Closes live handles when a caller abandons an unfinished operation.
    fn drop(&mut self) {
        self.close();
    }
}

/// Computes the first deadline after a reference instant.
fn next_deadline(reference: Instant, interval: Duration) -> Option<Instant> {
    (!interval.is_zero())
        .then(|| reference.checked_add(interval))
        .flatten()
}
/// Allocates a nonzero operation ID without wrapping or reuse.
fn allocate_operation_id() -> Result<u64, ValidationError> {
    loop {
        let current = NEXT_OPERATION_ID.load(Ordering::Relaxed);
        if current == 0 {
            return Err(ValidationError::OperationIdExhausted);
        }
        let next = current.checked_add(1).unwrap_or(0);
        if NEXT_OPERATION_ID
            .compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            return Ok(current);
        }
    }
}
