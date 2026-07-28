// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Error types for progress configuration and delivery.
// qubit-style: allow multiple-public-types

use std::{
    error::Error,
    fmt,
    time::Duration,
};

/// Structured validation failure for progress configuration or snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ValidationError {
    /// An operation was started without metrics.
    NoMetrics,
    /// A metric ID is empty or whitespace only.
    EmptyMetricId {
        /// Zero-based metric position.
        index: usize,
    },
    /// A metric name is empty or whitespace only.
    EmptyMetricName {
        /// ID of the metric with the missing name.
        metric_id: String,
    },
    /// Two configured metrics have the same ID.
    DuplicateMetricId {
        /// Repeated metric ID.
        metric_id: String,
    },
    /// A snapshot addressed a metric not declared by the operation.
    UnknownMetricId {
        /// Metric ID not present in the operation configuration.
        metric_id: String,
    },
    /// A snapshot configured the same metric more than once.
    DuplicateMetricUpdate {
        /// Metric ID configured more than once in one snapshot.
        metric_id: String,
    },
    /// A snapshot omitted a metric declared by the operation.
    MissingMetricUpdate {
        /// Metric ID not configured in one snapshot.
        metric_id: String,
    },
    /// Arithmetic required to validate a count set overflowed.
    CountOverflow {
        /// Metric whose count arithmetic overflowed.
        metric_id: String,
    },
    /// Classified successful and failed counts exceed completed work.
    ClassifiedExceedsCompleted {
        /// Metric with contradictory classified counts.
        metric_id: String,
    },
    /// Completed plus active work exceeds a known total.
    CountsExceedTotal {
        /// Metric whose occupied work exceeds its total.
        metric_id: String,
    },
    /// A zero total was paired with nonzero dynamic counts.
    NonZeroCountsForZeroTotal {
        /// Zero-total metric containing dynamic counts.
        metric_id: String,
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
        /// Configured stage count.
        total: u64,
    },
    /// The process-local operation ID space has been exhausted.
    OperationIdExhausted,
    /// The event sequence space for one operation has been exhausted.
    SequenceExhausted,
}

impl fmt::Display for ValidationError {
    /// Formats a concise validation explanation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMetrics => write!(
                formatter,
                "a progress operation requires at least one metric"
            ),
            Self::EmptyMetricId { index } => {
                write!(formatter, "metric at index {index} has an empty ID")
            }
            Self::EmptyMetricName { metric_id } => {
                write!(formatter, "metric {metric_id:?} has an empty name")
            }
            Self::DuplicateMetricId { metric_id } => {
                write!(formatter, "metric ID {metric_id:?} is duplicated")
            }
            Self::UnknownMetricId { metric_id } => {
                write!(formatter, "metric ID {metric_id:?} is not configured")
            }
            Self::DuplicateMetricUpdate { metric_id } => write!(
                formatter,
                "metric ID {metric_id:?} was configured twice in one snapshot"
            ),
            Self::MissingMetricUpdate { metric_id } => write!(
                formatter,
                "metric ID {metric_id:?} was not configured in one snapshot"
            ),
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
            Self::NonZeroCountsForZeroTotal { metric_id } => write!(
                formatter,
                "zero-total metric {metric_id:?} has nonzero counts"
            ),
            Self::EmptyStageId => write!(formatter, "stage ID is empty"),
            Self::EmptyStageName => write!(formatter, "stage name is empty"),
            Self::IncompleteStagePosition => write!(
                formatter,
                "stage position and total must be supplied together"
            ),
            Self::InvalidStagePosition { position, total } => write!(
                formatter,
                "stage position {position} is outside 1..={total}"
            ),
            Self::OperationIdExhausted => {
                write!(formatter, "progress operation IDs are exhausted")
            }
            Self::SequenceExhausted => {
                write!(formatter, "progress event sequence is exhausted")
            }
        }
    }
}
impl Error for ValidationError {}

/// Reporter failure that preserves its original error chain.
#[derive(Debug)]
pub struct ReportError {
    source: Box<dyn Error + Send + Sync + 'static>,
}
impl Clone for ReportError {
    /// Clones the stable error message when an enclosing error needs cloning.
    fn clone(&self) -> Self {
        Self::message(&self.to_string())
    }
}
impl PartialEq for ReportError {
    /// Compares the stable display representation of opaque reporter errors.
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
impl Eq for ReportError {}
impl ReportError {
    /// Wraps a concrete reporter failure without discarding its source.
    pub fn new<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            source: Box::new(source),
        }
    }
    /// Creates a reporter error from a stable message.
    pub fn message(message: &str) -> Self {
        Self::new(MessageError(message.into()))
    }
    /// Returns the original reporter error.
    #[must_use]
    pub fn source_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        &*self.source
    }
}
impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}
impl Error for ReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.source)
    }
}

/// Complete error returned from non-terminal progress operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressError {
    /// Caller supplied invalid state.
    Validation(ValidationError),
    /// The reporter rejected an event.
    Report(ReportError),
}
impl fmt::Display for ProgressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
        }
    }
}
impl Error for ProgressError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Report(error) => Some(error),
        }
    }
}
impl From<ValidationError> for ProgressError {
    /// Converts validation failure.
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}
impl From<ReportError> for ProgressError {
    /// Converts reporter failure.
    fn from(error: ReportError) -> Self {
        Self::Report(error)
    }
}

/// Terminal delivery failure paired with the elapsed operation time.
#[derive(Debug)]
pub struct TerminalError {
    elapsed: Duration,
    error: ProgressError,
}
impl TerminalError {
    /// Creates a terminal error with its elapsed operation duration.
    pub(crate) const fn new(elapsed: Duration, error: ProgressError) -> Self {
        Self { elapsed, error }
    }
    /// Returns elapsed duration at terminal failure.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
    /// Returns the retained progress error.
    #[must_use]
    pub const fn progress_error(&self) -> &ProgressError {
        &self.error
    }
    /// Consumes this terminal error and returns its underlying progress error.
    #[must_use]
    pub fn into_progress_error(self) -> ProgressError {
        self.error
    }
}
impl fmt::Display for TerminalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "terminal progress report failed after {:?}: {}",
            self.elapsed, self.error
        )
    }
}
impl Error for TerminalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// Message-backed error used only by [`ReportError::message`].
#[derive(Debug)]
struct MessageError(String);
impl fmt::Display for MessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl Error for MessageError {}
