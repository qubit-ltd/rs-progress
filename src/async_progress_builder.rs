// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for [`crate::async_progress::AsyncProgress`].

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::Metric;
use crate::MetricHandle;
use crate::OperationAttributes;
use crate::Phase;
use crate::Stage;
use crate::async_progress::AsyncProgress;
use crate::error::StartError;
use crate::internal::OperationState;
use crate::internal::metric_snapshots;
use crate::progress::allocate_operation_id;
use crate::validation::validate_attributes;
use crate::validation::validate_metrics;
use crate::validation::validate_stage;

#[path = "async_progress_builder/internal/async_reporter_handle.rs"]
mod async_reporter_handle;
pub(crate) use self::async_reporter_handle::AsyncReporterHandle;

/// Configures an asynchronous progress operation before it starts.
///
/// # Type Parameters
///
/// * `'reporter` - Lifetime of a borrowed reporter; owned reporters use
///   `'static`.
///
/// # Examples
///
/// ```
/// use qubit_progress::AsyncProgress;
/// use qubit_progress::AsyncReporter;
/// use qubit_progress::Event;
/// use qubit_progress::ReportFuture;
/// use std::time::Duration;
///
/// struct Sink;
///
/// impl AsyncReporter for Sink {
///     fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
///         Box::pin(async { Ok(()) })
///     }
/// }
///
/// async fn start_operation() {
///     let sink = Sink;
///     let _progress = AsyncProgress::builder(&sink)
///         .interval(Duration::from_secs(1))
///         .start_async()
///         .await
///         .expect("valid default metadata starts successfully");
/// }
/// ```
#[must_use]
pub struct AsyncProgressBuilder<'reporter> {
    /// Reporter receiving complete events.
    pub(super) reporter: AsyncReporterHandle<'reporter>,
    /// Minimum interval between due-based running reports.
    pub(super) interval: Duration,
    /// Stable operation metrics.
    pub(super) metrics: Vec<Metric>,
    /// Optional initial stage.
    pub(super) stage: Option<Stage>,
    /// Correlation attributes shared by all operation events.
    pub(super) attributes: OperationAttributes,
}

impl<'reporter> AsyncProgressBuilder<'reporter> {
    /// Sets the minimum interval between due-based running reports.
    ///
    /// # Parameters
    ///
    /// * `interval` - Minimum spacing used when deciding whether a running
    ///   report is due.
    ///
    /// # Returns
    ///
    /// The builder with the new interval.
    #[inline]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Adds one stable metric to the operation.
    ///
    /// # Parameters
    ///
    /// * `metric` - Metric definition to attach to each operation event.
    ///
    /// # Returns
    ///
    /// The builder with the metric appended.
    pub fn metric(mut self, metric: Metric) -> Self {
        self.metrics.push(metric);
        self
    }

    /// Adds stage metadata to the Started event and subsequent events.
    ///
    /// # Parameters
    ///
    /// * `stage` - Initial stage metadata.
    ///
    /// # Returns
    ///
    /// The builder with the initial stage configured.
    #[inline]
    pub fn stage(mut self, stage: Stage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Adds or replaces one operation correlation attribute.
    ///
    /// # Parameters
    ///
    /// * `key` - Attribute key.
    /// * `value` - Attribute value.
    ///
    /// # Returns
    ///
    /// The builder with this attribute inserted or replaced.
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Replaces all operation correlation attributes.
    ///
    /// # Parameters
    ///
    /// * `attributes` - Complete replacement set of operation attributes.
    ///
    /// # Returns
    ///
    /// The builder with the supplied attributes.
    #[inline]
    pub fn attributes(mut self, attributes: OperationAttributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// Validates configuration and asynchronously emits Started when enabled.
    ///
    /// Returns a start error when metadata is invalid, operation IDs are
    /// exhausted, or the reporter rejects the Started event.
    ///
    /// # Returns
    ///
    /// Returns the started progress operation after the Started event has been
    /// accepted, or immediately when reporting is disabled.
    ///
    /// # Errors
    ///
    /// Returns [`StartError`] when metadata validation fails, no operation ID
    /// is available, or the reporter rejects the Started event.
    pub async fn start_async(self) -> Result<AsyncProgress<'reporter>, StartError> {
        validate_metrics(&self.metrics)?;
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }
        validate_attributes(&self.attributes)?;

        let enabled = self.reporter.as_reporter().is_enabled();
        let operation_state = OperationState::new();
        let operation_id = enabled.then(allocate_operation_id).transpose()?;
        let mut progress = AsyncProgress {
            reporter: self.reporter,
            enabled,
            metrics: self
                .metrics
                .into_iter()
                .map(|metric| MetricHandle::new(metric, Arc::clone(&operation_state)))
                .collect(),
            operation_state,
            stage: self.stage,
            attributes: Arc::new(self.attributes),
            interval: self.interval,
            started_at: Instant::now(),
            next_due_elapsed: None,
            operation_id,
            next_sequence: 0,
        };
        if enabled {
            let metrics = metric_snapshots(&progress.metrics);
            progress.emit(Phase::Started, metrics, Duration::ZERO).await?;
            progress.next_due_elapsed = Some(progress.interval);
            progress.started_at = Instant::now();
        }
        Ok(progress)
    }
}
