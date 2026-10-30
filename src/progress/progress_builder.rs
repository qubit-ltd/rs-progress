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
    ///
    /// # Parameters
    ///
    /// - `interval`: Minimum spacing for [`Progress::report_if_due`] calls. A
    ///   zero interval makes every enabled call due. A positive interval does
    ///   not restrict manual [`Progress::report`] calls. Every duration,
    ///   including [`Duration::MAX`], is accepted. Deadlines use elapsed
    ///   durations rather than adding the interval to an [`Instant`], so even
    ///   the maximum duration does not cause `Instant` overflow.
    ///
    /// # Returns
    ///
    /// The builder with the due-report interval set to `interval`.
    #[must_use]
    #[inline]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
    /// Appends one stable metric in declaration order.
    ///
    /// Duplicate IDs are not replaced; they are rejected by [`Self::start`].
    ///
    /// # Parameters
    ///
    /// - `metric`: Metric definition to attach to every operation event. Its ID
    ///   and name are validated when the operation starts.
    ///
    /// # Returns
    ///
    /// The builder with the metric appended.
    #[must_use]
    pub fn metric(mut self, metric: Metric) -> Self {
        self.metrics.push(metric);
        self
    }
    /// Replaces the initial stage attached to operation events.
    ///
    /// Repeated calls keep the last stage. Metadata is validated by
    /// [`Self::start`].
    ///
    /// # Parameters
    ///
    /// - `stage`: Initial stage metadata to install.
    ///
    /// # Returns
    ///
    /// The builder with the initial stage replaced.
    #[must_use]
    #[inline]
    pub fn stage(mut self, stage: Stage) -> Self {
        self.stage = Some(stage);
        self
    }
    /// Inserts or replaces one operation correlation attribute.
    ///
    /// Other keys are preserved. Blank keys are rejected by [`Self::start`].
    ///
    /// # Parameters
    ///
    /// - `key`: Key whose value is inserted or replaced.
    /// - `value`: String value shared by all operation events.
    ///
    /// # Returns
    ///
    /// The builder with this attribute inserted or replaced.
    #[must_use]
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key, value);
        self
    }
    /// Replaces the complete operation correlation attribute set.
    ///
    /// Existing entries are discarded rather than merged. Attribute keys are
    /// validated by [`Self::start`].
    ///
    /// # Parameters
    ///
    /// - `attributes`: Complete replacement set shared by all operation events.
    ///
    /// # Returns
    ///
    /// The builder with the supplied attributes installed.
    #[must_use]
    pub fn attributes(mut self, attributes: OperationAttributes) -> Self {
        self.attributes = attributes;
        self
    }
    /// Validates configuration, samples enablement and emits Started when
    /// enabled.
    ///
    /// # Returns
    ///
    /// The started operation after configuration validation and enablement
    /// sampling. When enabled, its Started event has been accepted by the
    /// reporter before this method returns successfully. Disabled operations
    /// return without delivering an event.
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
