// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by live metric transitions.

use std::{error::Error, fmt};

use crate::{OperationLifecycle, error::MetricTransition};

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
    /// A transition attempted to remove more work than its source state held.
    InsufficientCount {
        /// Stable metric ID.
        metric_id: String,
        /// Requested transition.
        transition: MetricTransition,
        /// Requested amount.
        requested: u64,
        /// Available source state.
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
            Self::InsufficientCount {
                metric_id,
                transition,
                requested,
                available,
            } => write!(
                formatter,
                "metric {metric_id:?} cannot {transition} {requested} work items because only {available} are available"
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
