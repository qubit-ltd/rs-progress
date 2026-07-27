// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

/// Error produced when metric metadata and counter values do not correspond.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressMetricSnapshotError {
    /// The metric and counter carry different stable identifiers.
    MetricIdMismatch {
        /// Identifier carried by the metric metadata.
        metric_id: String,
        /// Identifier carried by the counter.
        counter_metric_id: String,
    },
}

impl Display for ProgressMetricSnapshotError {
    /// Formats a concise snapshot validation error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetricIdMismatch {
                metric_id,
                counter_metric_id,
            } => write!(
                formatter,
                "progress metric id {metric_id} does not match counter metric id {counter_metric_id}",
            ),
        }
    }
}

impl Error for ProgressMetricSnapshotError {}
