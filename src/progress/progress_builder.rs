// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configuration and startup for progress operations.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use super::Progress;
use super::internal::reporter_handle::ReporterHandle;
use crate::Metric;
use crate::MetricHandle;
use crate::OperationAttributes;
use crate::Phase;
use crate::Stage;
use crate::error::StartError;
use crate::internal::OperationState;
use crate::internal::metric_snapshots;
use crate::validation::validate_attributes;
use crate::validation::validate_metrics;
use crate::validation::validate_stage;

/// Configures one [`Progress`] operation before it starts.
///
/// The builder borrows a reporter or owns a shared reporter, then validates the
/// selected metrics and metadata before emitting the initial event.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, NoopReporter, Progress};
///
/// let progress = Progress::builder(&NoopReporter)
///     .interval(std::time::Duration::from_secs(1))
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// progress.finish_unchecked()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Type Parameters
///
/// * `'reporter` — lifetime of a borrowed reporter. An owned reporter uses
///   `'static`.
pub struct ProgressBuilder<'reporter> {
    /// Reporter receiving complete events.
    pub(super) reporter: ReporterHandle<'reporter>,
    /// Minimum interval between due-based running reports.
    pub(super) interval: Duration,
    /// Stable operation metrics.
    pub(super) metrics: Vec<Metric>,
    /// Optional initial stage.
    pub(super) stage: Option<Stage>,
    /// Correlation attributes shared by all operation events.
    pub(super) attributes: OperationAttributes,
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
    /// Adds or replaces one operation correlation attribute.
    #[must_use]
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key, value);
        self
    }
    /// Replaces all operation correlation attributes.
    #[must_use]
    pub fn attributes(mut self, attributes: OperationAttributes) -> Self {
        self.attributes = attributes;
        self
    }
    /// Validates configuration, samples enablement and emits Started when
    /// enabled.
    ///
    /// # Errors
    ///
    /// Returns `StartError` when metric or metadata validation fails, the
    /// operation ID space is exhausted, or the reporter rejects the Started
    /// event. Delivery errors retain the attempted event.
    pub fn start(self) -> Result<Progress<'reporter>, StartError> {
        validate_metrics(&self.metrics)?;
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }
        validate_attributes(&self.attributes)?;

        let enabled = self.reporter.as_reporter().is_enabled();
        let operation_state = OperationState::new();
        let operation_id = enabled.then(super::allocate_operation_id).transpose()?;
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
            attributes: Arc::new(self.attributes),
            interval: self.interval,
            started_at: Instant::now(),
            next_due_elapsed: None,
            operation_id,
            next_sequence: 0,
        };

        if enabled {
            let metrics = metric_snapshots(&progress.metrics);
            progress
                .emit(Phase::Started, metrics, Duration::ZERO)
                .map_err(StartError::from)?;
            progress.next_due_elapsed = Some(progress.interval);
            progress.started_at = Instant::now();
        }
        Ok(progress)
    }
}
