// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Validation failure for serialized dynamic metric counts.

use std::error::Error;
use std::fmt;

/// Validation failure for serialized dynamic metric counts.
#[derive(Debug)]
pub(crate) enum SnapshotValidationError {
    /// Derived classified counts overflowed.
    CountOverflow {
        /// Identifies the metric whose count arithmetic overflowed.
        metric_id: String,
    },
    /// Classified counts exceed completed work.
    ClassifiedExceedsCompleted {
        /// Identifies the metric whose classified count exceeds completed work.
        metric_id: String,
    },
    /// Occupied work exceeds a known total.
    CountsExceedTotal {
        /// Identifies the metric whose occupied work exceeds its total.
        metric_id: String,
    },
    /// A zero total contains dynamic counts.
    NonZeroCountsForZeroTotal {
        /// Identifies the zero-total metric that contains nonzero counts.
        metric_id: String,
    },
}

impl fmt::Display for SnapshotValidationError {
    /// Formats the snapshot validation failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountOverflow { metric_id } => {
                write!(formatter, "counts for metric {metric_id:?} overflowed")
            }
            Self::ClassifiedExceedsCompleted { metric_id } => write!(
                formatter,
                "classified counts exceed completed work for metric {metric_id:?}"
            ),
            Self::CountsExceedTotal { metric_id } => write!(
                formatter,
                "completed plus active work exceeds total for metric {metric_id:?}"
            ),
            Self::NonZeroCountsForZeroTotal { metric_id } => {
                write!(formatter, "zero-total metric {metric_id:?} has nonzero counts")
            }
        }
    }
}

impl Error for SnapshotValidationError {}
