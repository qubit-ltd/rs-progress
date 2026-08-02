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
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::{
    Event, Metric, MetricHandle, MetricSnapshot, Phase, Reporter, Stage,
    auto_reporter::{self, AutoReporter},
    error::{
        CompletionError, ConfigurationError, DeliveryError, EmissionError, FinishError, StartError,
        TerminalError,
    },
    internal::OperationState,
    validation::{validate_metrics, validate_stage},
};

/// Process-local source of nonzero operation identifiers.
static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Configures one [`Progress`] operation before it starts.
pub struct ProgressBuilder<'reporter> {
    /// Reporter receiving complete events.
    reporter: &'reporter dyn Reporter,
    /// Minimum interval between due-based running reports.
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
    pub fn start(self) -> Result<Progress<'reporter>, StartError> {
        validate_metrics(&self.metrics)?;
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }

        let enabled = self.reporter.is_enabled();
        let operation_state = OperationState::new();
        let operation_id = enabled.then(allocate_operation_id).transpose()?;
        let mut progress = Progress {
            reporter: self.reporter,
            enabled,
            metrics: self
                .metrics
                .into_iter()
                .map(|metric| MetricHandle::new(metric, Arc::clone(&operation_state)))
                .collect(),
            operation_state,
            stage: self.stage,
            interval: self.interval,
            started_at: Instant::now(),
            next_due_elapsed: None,
            operation_id,
            next_sequence: 0,
        };

        if enabled {
            let metrics = progress.metric_snapshots();
            progress
                .emit(Phase::Started, metrics, Duration::ZERO)
                .map_err(StartError::from)?;
            progress.next_due_elapsed = Some(progress.interval);
            progress.started_at = Instant::now();
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
    /// Shared lifecycle and in-flight update gate.
    operation_state: Arc<OperationState>,
    /// Optional current stage.
    stage: Option<Stage>,
    /// Minimum due-report spacing.
    interval: Duration,
    /// Monotonic operation start time.
    started_at: Instant,
    /// Next due elapsed deadline for a positive interval.
    next_due_elapsed: Option<Duration>,
    /// Nonzero identifier for enabled operations.
    operation_id: Option<u64>,
    /// Sequence reserved for the next event attempt.
    next_sequence: u64,
}

impl<'reporter> Progress<'reporter> {
    /// Creates a builder bound to one reporter.
    #[must_use]
    pub fn builder(reporter: &'reporter dyn Reporter) -> ProgressBuilder<'reporter> {
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
    pub fn report(&mut self) -> Result<(), EmissionError> {
        if !self.enabled {
            return Ok(());
        }
        let metrics = self.metric_snapshots();
        let elapsed = self.elapsed();
        let result = self.emit(Phase::Running, metrics, elapsed);
        self.reset_deadline();
        result
    }
    /// Emits a Running event only when the configured interval is due.
    pub fn report_if_due(&mut self) -> Result<(), EmissionError> {
        if !self.enabled || !self.is_due() {
            return Ok(());
        }
        self.report()
    }
    /// Replaces stage metadata attached to subsequent events.
    pub fn set_stage(&mut self, stage: Stage) -> Result<(), ConfigurationError> {
        validate_stage(&stage)?;
        self.stage = Some(stage);
        Ok(())
    }
    /// Removes stage metadata from subsequent events.
    pub fn clear_stage(&mut self) {
        self.stage = None;
    }
    /// Consumes this operation and emits a successful terminal event without
    /// checking whether metric work is complete.
    pub fn finish_unchecked(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Succeeded)
    }
    /// Consumes this operation and emits a successful terminal event only when
    /// no metric has active work and every known total has been completed.
    #[allow(clippy::result_large_err)]
    pub fn finish(mut self) -> Result<Duration, FinishError<'reporter>> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        if let Err(source) = self.validate_finish() {
            finish_guard.reopen();
            return Err(FinishError::Incomplete {
                progress: self,
                source,
            });
        }
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(Phase::Succeeded, self.metric_snapshots(), elapsed)
            .map(|()| elapsed)
            .map_err(|source| FinishError::Terminal(TerminalError::new(elapsed, source)))
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
    fn metric_snapshots(&self) -> Vec<MetricSnapshot> {
        self.metrics.iter().map(MetricHandle::snapshot).collect()
    }
    /// Validates the metric invariants required for successful finish.
    fn validate_finish(&self) -> Result<(), CompletionError> {
        for metric in &self.metrics {
            let snapshot = metric.snapshot();
            if snapshot.active() != 0 {
                return Err(CompletionError::ActiveWork {
                    metric_id: snapshot.id().to_owned(),
                    active: snapshot.active(),
                });
            }
            if let Some(total) = snapshot.total()
                && snapshot.completed() != total
            {
                return Err(CompletionError::IncompleteTotal {
                    metric_id: snapshot.id().to_owned(),
                    completed: snapshot.completed(),
                    total,
                });
            }
        }
        Ok(())
    }
    /// Delivers one complete event after reserving its delivery sequence.
    fn emit(
        &mut self,
        phase: Phase,
        metrics: Vec<MetricSnapshot>,
        elapsed: Duration,
    ) -> Result<(), EmissionError> {
        let operation_id = self.operation_id.ok_or(EmissionError::SequenceExhausted)?;
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or(EmissionError::SequenceExhausted)?;
        let event = Event::new(
            operation_id,
            sequence,
            phase,
            self.stage.clone(),
            metrics,
            elapsed,
        );
        match self.reporter.report(&event) {
            Ok(()) => Ok(()),
            Err(source) => Err(EmissionError::Delivery(DeliveryError::new(event, source))),
        }
    }
    /// Tests whether a due-based running report can run now.
    fn is_due(&self) -> bool {
        self.interval.is_zero()
            || self
                .next_due_elapsed
                .is_some_and(|deadline| self.elapsed() >= deadline)
    }
    /// Returns the configured interval to the crate-private background loop.
    pub(crate) const fn report_interval(&self) -> Duration {
        self.interval
    }
    /// Returns how long the background loop should wait for the next deadline.
    pub(crate) fn time_until_due(&self) -> Duration {
        self.next_due_elapsed
            .map(|deadline| deadline.saturating_sub(self.elapsed()))
            .unwrap_or(Duration::MAX)
    }
    /// Pushes the next positive-interval deadline after a running attempt.
    fn reset_deadline(&mut self) {
        self.next_due_elapsed = self.elapsed().checked_add(self.interval);
    }
    /// Emits one terminal phase while retaining elapsed time on failure.
    fn terminal(mut self, phase: Phase) -> Result<Duration, TerminalError> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(phase, self.metric_snapshots(), elapsed)
            .map(|()| elapsed)
            .map_err(|source| TerminalError::new(elapsed, source))
    }
}

impl Drop for Progress<'_> {
    /// Closes live handles when a caller abandons an unfinished operation.
    fn drop(&mut self) {
        self.operation_state.close();
    }
}

/// Allocates a nonzero operation ID without wrapping or reuse.
fn allocate_operation_id() -> Result<u64, StartError> {
    loop {
        let current = NEXT_OPERATION_ID.load(Ordering::Relaxed);
        if current == 0 {
            return Err(StartError::OperationIdExhausted);
        }
        let next = current.checked_add(1).unwrap_or(0);
        if NEXT_OPERATION_ID
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(current);
        }
    }
}
