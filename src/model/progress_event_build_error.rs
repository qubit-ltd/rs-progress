// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Error produced when progress event counters violate schema constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEventBuildError {
    /// A counter references a metric absent from the event schema.
    UnknownMetricId {
        /// Undeclared metric identifier.
        metric_id: String,
    },
    /// More than one counter references the same metric identifier.
    DuplicateCounterMetricId {
        /// Duplicated counter metric identifier.
        metric_id: String,
    },
}

impl Display for ProgressEventBuildError {
    /// Formats a concise event validation error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMetricId { metric_id } => {
                write!(formatter, "unknown progress metric id: {metric_id}")
            }
            Self::DuplicateCounterMetricId { metric_id } => write!(
                formatter,
                "duplicate progress counter metric id: {metric_id}",
            ),
        }
    }
}

impl Error for ProgressEventBuildError {}
