// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Runtime-independent asynchronous progress lifecycle.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::MetricHandle;
use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Phase;
use crate::Stage;
use crate::error::ConfigurationError;
use crate::error::DeliveryError;
use crate::error::EmissionError;
use crate::error::FinishError;
use crate::error::TerminalError;
use crate::internal::OperationState;
use crate::internal::build_event;
use crate::internal::metric_snapshots;
use crate::internal::validate_finish;
use crate::validation::validate_stage;

// Owns the builder and its AsyncProgress constructors.
#[path = "async_progress_builder.rs"]
mod async_progress_builder;
pub use self::async_progress_builder::AsyncProgressBuilder;
use self::async_progress_builder::AsyncReporterHandle;

/// One started asynchronous progress operation.
///
/// Terminal methods consume the operation. Dropping a pending terminal future
/// therefore ends the operation locally while leaving remote delivery status
/// unknown.
///
/// # Examples
///
/// ```
/// use qubit_progress::AsyncProgress;
/// use qubit_progress::AsyncReporter;
/// use qubit_progress::Event;
/// use qubit_progress::ReportFuture;
///
/// struct Sink;
///
/// impl AsyncReporter for Sink {
///     fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
///         Box::pin(async { Ok(()) })
///     }
/// }
///
/// async fn report_once() {
///     let sink = Sink;
///     let mut progress = AsyncProgress::builder(&sink)
///         .start_async()
///         .await
///         .expect("starting with valid defaults succeeds");
///     progress.report_async().await.expect("the sink accepts events");
///     progress
///         .finish_unchecked_async()
///         .await
///         .expect("the sink accepts terminal events");
/// }
/// ```
#[must_use]
pub struct AsyncProgress<'reporter> {
    /// Reporter selected by the builder.
    pub(crate) reporter: AsyncReporterHandle<'reporter>,
    /// Stable enablement sampled once at start.
    pub(crate) enabled: bool,
    /// Live metrics carried by each event.
    pub(crate) metrics: Vec<MetricHandle>,
    /// Shared lifecycle and in-flight update gate.
    pub(crate) operation_state: Arc<OperationState>,
    /// Optional current stage.
    pub(crate) stage: Option<Stage>,
    /// Immutable correlation attributes shared by all events.
    pub(crate) attributes: Arc<OperationAttributes>,
    /// Minimum due-report spacing.
    pub(crate) interval: Duration,
    /// Monotonic operation start time.
    pub(crate) started_at: Instant,
    /// Next due elapsed deadline for a positive interval.
    pub(crate) next_due_elapsed: Option<Duration>,
    /// Nonzero identifier for enabled operations.
    pub(crate) operation_id: Option<u64>,
    /// Sequence reserved for the next event attempt.
    pub(crate) next_sequence: u64,
}

impl<'reporter> AsyncProgress<'reporter> {
    /// Returns enablement sampled when this operation started.
    ///
    /// # Returns
    ///
    /// `true` when events are emitted for this operation.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns monotonic elapsed time since `start_async()`.
    ///
    /// # Returns
    ///
    /// The time elapsed according to a monotonic clock.
    #[inline]
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Returns a cloneable live metric selected by its stable ID.
    ///
    /// # Parameters
    ///
    /// * `metric_id` - Stable ID assigned to the metric when configuring the
    ///   operation.
    ///
    /// # Returns
    ///
    /// `Some` contains a cloneable handle when the ID belongs to this
    /// operation; `None` means no configured metric has that ID.
    #[inline]
    pub fn metric(&self, metric_id: &str) -> Option<MetricHandle> {
        self.metrics.iter().find(|metric| metric.id() == metric_id).cloned()
    }

    /// Immediately emits a Running event from current metric state.
    ///
    /// A reporter error contains the complete attempted event. If this future
    /// is cancelled while pending, delivery status is unknown and the event
    /// sequence remains consumed.
    ///
    /// # Errors
    ///
    /// Returns [`EmissionError`] when event construction fails or the reporter
    /// rejects delivery. The error retains the attempted event and delivery
    /// source when available.
    pub async fn report_async(&mut self) -> Result<(), EmissionError> {
        if !self.enabled {
            return Ok(());
        }
        let metrics = metric_snapshots(&self.metrics);
        let elapsed = self.elapsed();
        let result = self.emit(Phase::Running, metrics, elapsed).await;
        self.reset_deadline();
        result
    }

    /// Emits a Running event only when the configured interval is due.
    ///
    /// # Errors
    ///
    /// Returns [`EmissionError`] when a due event cannot be constructed or
    /// delivered. No error is returned when reporting is disabled or not yet
    /// due.
    pub async fn report_if_due_async(&mut self) -> Result<(), EmissionError> {
        if !self.enabled || !self.is_due() {
            return Ok(());
        }
        self.report_async().await
    }

    /// Replaces stage metadata attached to subsequent events.
    ///
    /// # Parameters
    ///
    /// * `stage` - Stage metadata to validate and attach to later events.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigurationError`] when the stage ID or name is empty, or
    /// when its position and total are inconsistent; the current stage is
    /// unchanged.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after replacing the stage.
    pub fn set_stage(&mut self, stage: Stage) -> Result<(), ConfigurationError> {
        validate_stage(&stage)?;
        self.stage = Some(stage);
        Ok(())
    }

    /// Removes stage metadata from subsequent events.
    #[inline]
    pub fn clear_stage(&mut self) {
        self.stage = None;
    }

    /// Consumes this operation and emits successful completion without checking
    /// metrics.
    ///
    /// # Returns
    ///
    /// The elapsed operation time after successful delivery.
    ///
    /// # Errors
    ///
    /// Returns [`TerminalError`] when event construction or reporter delivery
    /// fails; delivery status can be unknown if the future is cancelled.
    pub async fn finish_unchecked_async(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Succeeded).await
    }

    /// Consumes this operation and emits successful completion only when metric
    /// work is complete.
    ///
    /// # Returns
    ///
    /// The elapsed operation time after successful delivery.
    ///
    /// # Errors
    ///
    /// Returns [`FinishError::Incomplete`] when a metric is unfinished, or
    /// [`FinishError::Terminal`] when the completion event cannot be delivered.
    #[allow(clippy::result_large_err)]
    pub async fn finish_async(mut self) -> Result<Duration, FinishError> {
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
            .await
            .map(|()| elapsed)
            .map_err(|source| FinishError::Terminal(TerminalError::new(elapsed, source)))
    }

    /// Consumes this operation and emits a failed terminal event.
    ///
    /// # Returns
    ///
    /// The elapsed operation time after successful delivery.
    ///
    /// # Errors
    ///
    /// Returns [`TerminalError`] when event construction or reporter delivery
    /// fails; delivery status can be unknown if the future is cancelled.
    pub async fn fail_async(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Failed).await
    }

    /// Consumes this operation and emits a cancelled terminal event.
    ///
    /// # Returns
    ///
    /// The elapsed operation time after successful delivery.
    ///
    /// # Errors
    ///
    /// Returns [`TerminalError`] when event construction or reporter delivery
    /// fails; delivery status can be unknown if the future is cancelled.
    pub async fn cancel_async(self) -> Result<Duration, TerminalError> {
        self.terminal(Phase::Cancelled).await
    }

    /// Delivers one complete event after reserving its delivery sequence.
    ///
    /// # Errors
    ///
    /// Returns [`EmissionError`] when event construction fails or the reporter
    /// rejects delivery; delivery errors retain the attempted event and source.
    pub(crate) async fn emit(
        &mut self,
        phase: Phase,
        metrics: Vec<MetricSnapshot>,
        elapsed: Duration,
    ) -> Result<(), EmissionError> {
        let event = build_event(
            self.operation_id,
            &mut self.next_sequence,
            phase,
            self.stage.as_ref(),
            &self.attributes,
            metrics,
            elapsed,
        )?;
        match self.reporter.as_reporter().report(&event).await {
            Ok(()) => Ok(()),
            Err(source) => Err(EmissionError::Delivery(DeliveryError::new(event, source))),
        }
    }

    /// Tests whether a due-based running report can run now.
    ///
    /// # Returns
    ///
    /// `true` when the interval is zero or the next deadline has elapsed.
    fn is_due(&self) -> bool {
        self.interval.is_zero() || self.next_due_elapsed.is_some_and(|deadline| self.elapsed() >= deadline)
    }

    /// Pushes the next positive-interval deadline after a running attempt.
    fn reset_deadline(&mut self) {
        self.next_due_elapsed = self.elapsed().checked_add(self.interval);
    }

    /// Emits one terminal phase while retaining elapsed time on failure.
    async fn terminal(mut self, phase: Phase) -> Result<Duration, TerminalError> {
        let elapsed = self.elapsed();
        let finish_guard = self.operation_state.begin_finish();
        finish_guard.close();
        if !self.enabled {
            return Ok(elapsed);
        }
        self.emit(phase, metric_snapshots(&self.metrics), elapsed)
            .await
            .map(|()| elapsed)
            .map_err(|source| TerminalError::new(elapsed, source))
    }
}

impl Drop for AsyncProgress<'_> {
    /// Closes live metric handles when an operation is abandoned.
    fn drop(&mut self) {
        self.operation_state.close();
    }
}
