// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Operation lifecycle, metric state and report scheduling.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use crate::Metric;
use crate::MetricHandle;
use crate::MetricSnapshot;
use crate::NoopReporter;
use crate::OperationAttributes;
use crate::Phase;
use crate::Reporter;
use crate::Stage;
use crate::auto_reporter;
use crate::auto_reporter::AutoReporter;
use crate::error::ConfigurationError;
use crate::error::DeliveryError;
use crate::error::EmissionError;
use crate::error::FinishError;
use crate::error::RecoverableFinishError;
use crate::error::StartError;
use crate::error::TerminalError;
use crate::internal::OperationState;
use crate::internal::build_event;
use crate::internal::metric_snapshots;
use crate::internal::validate_finish;
use crate::validation::validate_stage;

mod internal;
mod progress_builder;

use internal::coverage_terminal_reporter::CoverageTerminalReporter;
use internal::reporter_handle::ReporterHandle;
pub use progress_builder::ProgressBuilder;

/// Process-local source of nonzero operation identifiers.
pub(crate) static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// One started progress operation.
///
/// Terminal methods consume this value, preventing reports after a terminal
/// phase and preventing duplicate terminal events in safe Rust.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, NoopReporter, Progress};
///
/// let progress = Progress::builder(&NoopReporter)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// let task = progress.metric("tasks").expect("configured metric must exist");
/// task.start(1)?;
/// task.succeed(1)?;
/// progress.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Type Parameters
///
/// * `'reporter` — lifetime of a borrowed reporter. An owned reporter uses
///   `'static`.
#[must_use]
pub struct Progress<'reporter> {
    /// Reporter selected by the builder.
    reporter: ReporterHandle<'reporter>,
    /// Stable enablement sampled once at start.
    enabled: bool,
    /// Live metrics carried by each event.
    metrics: Vec<MetricHandle>,
    /// Shared lifecycle and in-flight update gate.
    operation_state: Arc<OperationState>,
    /// Optional current stage.
    stage: Option<Stage>,
    /// Immutable correlation attributes shared by all events.
    attributes: Arc<OperationAttributes>,
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
    /// Creates a builder borrowing one reporter.
    #[must_use]
    pub fn builder(reporter: &'reporter dyn Reporter) -> ProgressBuilder<'reporter> {
        ProgressBuilder {
            reporter: ReporterHandle::Borrowed(reporter),
            interval: Duration::ZERO,
            metrics: Vec::new(),
            stage: None,
            attributes: OperationAttributes::new(),
        }
    }
    /// Creates a builder that owns one shared reporter.
    #[must_use]
    pub fn builder_arc(reporter: Arc<dyn Reporter>) -> ProgressBuilder<'static> {
        ProgressBuilder {
            reporter: ReporterHandle::Owned(reporter),
            interval: Duration::ZERO,
            metrics: Vec::new(),
            stage: None,
            attributes: OperationAttributes::new(),
        }
    }
    /// Returns enablement sampled when this operation started.
    #[must_use]
    #[inline]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
    /// Returns monotonic elapsed time since `start()`.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
    /// Returns a cloneable live metric selected by its stable ID.
    ///
    /// # Returns
    ///
    /// Returns a handle when a metric has the requested ID, or `None` when no
    /// configured metric matches.
    pub fn metric(&self, metric_id: &str) -> Option<MetricHandle> {
        self.metrics.iter().find(|metric| metric.id() == metric_id).cloned()
    }
    /// Immediately emits a Running event from current metric state.
    ///
    /// # Errors
    ///
    /// Returns an `EmissionError` containing the attempted event and reporter
    /// error if event delivery fails. Disabled operations return successfully
    /// without emitting an event.
    pub fn report(&mut self) -> Result<(), EmissionError> {
        if !self.enabled {
            return Ok(());
        }
        let metrics = metric_snapshots(&self.metrics);
        let elapsed = self.elapsed();
        let result = self.emit(Phase::Running, metrics, elapsed);
        self.reset_deadline();
        result
    }
    /// Emits a Running event only when the configured interval is due.
    ///
    /// # Errors
    ///
    /// Returns an `EmissionError` containing the attempted event and reporter
    /// error if a due event cannot be delivered. When the interval is not due
    /// or reporting is disabled, it returns successfully without emitting.
    pub fn report_if_due(&mut self) -> Result<(), EmissionError> {
        if !self.enabled || !self.is_due() {
            return Ok(());
        }
        self.report()
    }
    /// Replaces stage metadata attached to subsequent events.
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError` when the stage metadata is invalid; the
    /// current stage remains unchanged in that case.
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
    ///
    /// # Errors
    ///
    /// Returns a `TerminalError` containing the attempted event and reporter
    /// error if terminal delivery fails.
    pub fn finish_unchecked(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Succeeded)
    }
    /// Consumes this operation and emits a successful terminal event only when
    /// no metric has active work and every known total has been completed.
    ///
    /// # Errors
    ///
    /// Returns `FinishError::Incomplete` with the unfinished metric details if
    /// validation fails, or `FinishError::Terminal` if event delivery fails.
    #[allow(clippy::result_large_err)]
    pub fn finish(mut self) -> Result<Duration, FinishError> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        if let Err(source) = validate_finish(&self.metrics) {
            finish_guard.close();
            return Err(FinishError::Incomplete { elapsed, source });
        }
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(Phase::Succeeded, metric_snapshots(&self.metrics), elapsed)
            .map(|()| elapsed)
            .map_err(|source| FinishError::Terminal(TerminalError::new(elapsed, source)))
    }
    /// Consumes this operation and emits a successful terminal event while
    /// preserving the operation when completion validation fails.
    ///
    /// # Errors
    ///
    /// Returns `RecoverableFinishError::Incomplete` with this operation when
    /// validation fails, or `RecoverableFinishError::Terminal` if delivery
    /// fails after completion validation.
    #[allow(clippy::result_large_err)]
    pub fn finish_recoverable(mut self) -> Result<Duration, RecoverableFinishError<'reporter>> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        if let Err(source) = validate_finish(&self.metrics) {
            finish_guard.reopen();
            return Err(RecoverableFinishError::Incomplete { progress: self, source });
        }
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(Phase::Succeeded, metric_snapshots(&self.metrics), elapsed)
            .map(|()| elapsed)
            .map_err(|source| RecoverableFinishError::Terminal(TerminalError::new(elapsed, source)))
    }
    /// Consumes this operation and emits a failed terminal event.
    ///
    /// # Errors
    ///
    /// Returns a `TerminalError` containing the attempted event and reporter
    /// error if terminal delivery fails.
    pub fn fail(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Failed)
    }
    /// Consumes this operation and emits a cancelled terminal event.
    ///
    /// # Errors
    ///
    /// Returns a `TerminalError` containing the attempted event and reporter
    /// error if terminal delivery fails.
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
    /// Delivers one complete event after reserving its delivery sequence.
    fn emit(&mut self, phase: Phase, metrics: Vec<MetricSnapshot>, elapsed: Duration) -> Result<(), EmissionError> {
        let event = build_event(
            self.operation_id,
            &mut self.next_sequence,
            phase,
            self.stage.as_ref(),
            &self.attributes,
            metrics,
            elapsed,
        )?;
        match self.reporter.as_reporter().report(&event) {
            Ok(()) => Ok(()),
            Err(source) => Err(EmissionError::Delivery(DeliveryError::new(event, source))),
        }
    }
    /// Tests whether a due-based running report can run now.
    fn is_due(&self) -> bool {
        self.interval.is_zero() || self.next_due_elapsed.is_some_and(|deadline| self.elapsed() >= deadline)
    }
    /// Returns the configured interval to the crate-private background loop.
    #[must_use]
    #[inline]
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
    #[inline(never)]
    fn terminal(mut self, phase: Phase) -> Result<Duration, TerminalError> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(phase, metric_snapshots(&self.metrics), elapsed)
            .map(|()| elapsed)
            .map_err(|source| TerminalError::new(elapsed, source))
    }
}

impl Drop for Progress<'_> {
    /// Closes live handles when a caller abandons an unfinished operation.
    #[inline(never)]
    fn drop(&mut self) {
        self.operation_state.close();
    }
}

/// Allocates a nonzero operation ID without wrapping or reuse.
#[inline(never)]
pub(crate) fn allocate_operation_id() -> Result<u64, StartError> {
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

/// Exercises progress-only edge paths from the instrumented library build.
#[doc(hidden)]
pub fn __coverage_progress_edges() {
    let previous = NEXT_OPERATION_ID.swap(0, Ordering::Relaxed);
    assert!(matches!(allocate_operation_id(), Err(StartError::OperationIdExhausted)));
    NEXT_OPERATION_ID.store(previous.max(1), Ordering::Relaxed);

    let progress = Progress::builder(&NoopReporter)
        .metric(Metric::new("coverage", "Coverage"))
        .start()
        .expect("coverage progress must start");
    progress.cancel().expect("coverage progress must cancel");

    let reporter = CoverageTerminalReporter {
        attempts: AtomicU64::new(0),
    };
    let progress = Progress::builder_arc(Arc::new(reporter))
        .metric(Metric::new("coverage-terminal", "Coverage terminal"))
        .start()
        .expect("coverage terminal progress must start");
    assert!(progress.cancel().is_err());
}
