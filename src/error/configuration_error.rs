// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors found while validating fixed progress configuration.
// qubit-style: allow source-test-pair

use std::{
    error::Error,
    fmt,
};

/// Invalid fixed metadata supplied to a progress operation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConfigurationError {
    /// An operation was started without metrics.
    NoMetrics,
    /// A metric ID is empty or whitespace only.
    EmptyMetricId {
        /// Zero-based metric position in the builder.
        index: usize,
    },
    /// A metric name is empty or whitespace only.
    EmptyMetricName {
        /// ID of the malformed metric.
        metric_id: String,
    },
    /// Two configured metrics have the same ID.
    DuplicateMetricId {
        /// Duplicated stable metric ID.
        metric_id: String,
    },
    /// An operation attribute key is empty or whitespace only.
    EmptyAttributeKey {
        /// Malformed attribute key.
        key: String,
    },
    /// A stage ID is empty or whitespace only.
    EmptyStageId,
    /// A stage name is empty or whitespace only.
    EmptyStageName,
    /// Stage position and total were not supplied together.
    IncompleteStagePosition,
    /// Stage position is outside its one-based total range.
    InvalidStagePosition {
        /// Invalid one-based position.
        position: u64,
        /// Declared number of stages.
        total: u64,
    },
}

impl fmt::Display for ConfigurationError {
    /// Formats a concise configuration explanation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMetrics => formatter
                .write_str("a progress operation requires at least one metric"),
            Self::EmptyMetricId { index } => {
                write!(formatter, "metric at index {index} has an empty ID")
            }
            Self::EmptyMetricName { metric_id } => {
                write!(formatter, "metric {metric_id:?} has an empty name")
            }
            Self::DuplicateMetricId { metric_id } => {
                write!(formatter, "metric ID {metric_id:?} is duplicated")
            }
            Self::EmptyAttributeKey { key } => {
                write!(formatter, "operation attribute key {key:?} is empty")
            }
            Self::EmptyStageId => formatter.write_str("stage ID is empty"),
            Self::EmptyStageName => formatter.write_str("stage name is empty"),
            Self::IncompleteStagePosition => formatter.write_str(
                "stage position and total must be supplied together",
            ),
            Self::InvalidStagePosition { position, total } => write!(
                formatter,
                "stage position {position} is outside 1..={total}"
            ),
        }
    }
}

impl Error for ConfigurationError {}
