// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors explaining why checked successful completion is unavailable.
// qubit-style: allow source-test-pair

use std::{
    error::Error,
    fmt,
};

/// Metric state that prevents checked successful completion.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompletionError {
    /// Active work remains for one metric.
    ActiveWork {
        /// Stable metric ID.
        metric_id: String,
        /// Number of active work items.
        active: u64,
    },
    /// A known metric total has not been reached.
    IncompleteTotal {
        /// Stable metric ID.
        metric_id: String,
        /// Number of completed work items.
        completed: u64,
        /// Required total work items.
        total: u64,
    },
}

impl fmt::Display for CompletionError {
    /// Formats the first completion invariant that failed.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveWork { metric_id, active } => write!(
                formatter,
                "metric {metric_id:?} still has {active} active work items at finish"
            ),
            Self::IncompleteTotal {
                metric_id,
                completed,
                total,
            } => write!(
                formatter,
                "metric {metric_id:?} completed {completed} work items but total is {total}"
            ),
        }
    }
}

impl Error for CompletionError {}
