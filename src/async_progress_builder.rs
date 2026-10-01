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
use crate::error::StartError;
use crate::internal::OperationState;
use crate::internal::metric_snapshots;
use crate::progress::allocate_operation_id;
use crate::reporter::AsyncReporter;
use crate::validation::validate_attributes;
use crate::validation::validate_metrics;
use crate::validation::validate_stage;

/// Selects an asynchronous reporter borrowed for the operation or owned by it.
pub(crate) enum AsyncReporterHandle<'reporter> {
    Borrowed(&'reporter dyn AsyncReporter),
    Owned(Arc<dyn AsyncReporter>),
}

impl AsyncReporterHandle<'_> {
    /// Returns the selected reporter.
    pub(crate) fn as_reporter(&self) -> &dyn AsyncReporter {
        match self {
            Self::Borrowed(reporter) => *reporter,
            Self::Owned(reporter) => reporter.as_ref(),
        }
    }
}

/// Configures an asynchronous progress operation before it starts.
pub struct AsyncProgressBuilder<'reporter> {
    /// Reporter receiving complete events.
    reporter: AsyncReporterHandle<'reporter>,
    /// Minimum interval between due-based running reports.
    interval: Duration,
    /// Stable operation metrics.
    metrics: Vec<Metric>,
    /// Optional initial stage.
    stage: Option<Stage>,
    /// Correlation attributes shared by all operation events.
    attributes: OperationAttributes,
}

impl<'reporter> AsyncProgressBuilder<'reporter> {
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

    /// Validates configuration and asynchronously emits Started when enabled.
    ///
    /// Returns a start error when metadata is invalid, operation IDs are
    /// exhausted, or the reporter rejects the Started event.
    pub async fn start_async(self) -> Result<crate::async_progress::AsyncProgress<'reporter>, StartError> {
        validate_metrics(&self.metrics)?;
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }
        validate_attributes(&self.attributes)?;

        let enabled = self.reporter.as_reporter().is_enabled();
        let operation_state = OperationState::new();
        let operation_id = enabled.then(allocate_operation_id).transpose()?;
        let mut progress = crate::async_progress::AsyncProgress {
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

impl<'reporter> crate::async_progress::AsyncProgress<'reporter> {
    /// Creates a builder borrowing one asynchronous reporter.
    #[must_use]
    pub fn builder(reporter: &'reporter dyn AsyncReporter) -> AsyncProgressBuilder<'reporter> {
        AsyncProgressBuilder {
            reporter: AsyncReporterHandle::Borrowed(reporter),
            interval: Duration::ZERO,
            metrics: Vec::new(),
            stage: None,
            attributes: OperationAttributes::new(),
        }
    }

    /// Creates a builder that owns one shared asynchronous reporter.
    #[must_use]
    pub fn builder_arc(reporter: Arc<dyn AsyncReporter>) -> AsyncProgressBuilder<'static> {
        AsyncProgressBuilder {
            reporter: AsyncReporterHandle::Owned(reporter),
            interval: Duration::ZERO,
            metrics: Vec::new(),
            stage: None,
            attributes: OperationAttributes::new(),
        }
    }
}
