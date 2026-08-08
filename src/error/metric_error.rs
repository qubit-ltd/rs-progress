// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by live metric transitions.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;

use crate::OperationLifecycle;

/// Failure while reading or mutating one stateful metric.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricError {
    /// The enclosing operation is freezing or already closed.
    OperationNotOpen {
        /// Stable metric ID.
        metric_id: String,
        /// Lifecycle state that rejected the transition.
        state: OperationLifecycle,
    },
    /// A completion delta attempted to consume more active work than exists.
    InsufficientActive {
        /// Stable metric ID.
        metric_id: String,
        /// Total terminal work requested by the delta.
        requested: u64,
        /// Active work available at the linearization point.
        available: u64,
    },
    /// A transition would exceed the configured total.
    TotalExceeded {
        /// Stable metric ID.
        metric_id: String,
        /// Configured total.
        total: u64,
        /// Attempted occupied count.
        attempted: u64,
    },
    /// Metric arithmetic overflowed.
    CountOverflow {
        /// Stable metric ID.
        metric_id: String,
    },
}

impl fmt::Display for MetricError {
    /// Formats the metric transition failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationNotOpen { metric_id, state } => write!(
                formatter,
                "metric {metric_id:?} is not open because operation is {state:?}"
            ),
            Self::InsufficientActive {
                metric_id,
                requested,
                available,
            } => write!(
                formatter,
                "metric {metric_id:?} cannot complete {requested} work items because only {available} are active"
            ),
            Self::TotalExceeded {
                metric_id,
                total,
                attempted,
            } => write!(
                formatter,
                "metric {metric_id:?} would occupy {attempted} work items above total {total}"
            ),
            Self::CountOverflow { metric_id } => {
                write!(formatter, "counts for metric {metric_id:?} overflowed")
            }
        }
    }
}

impl Error for MetricError {}
