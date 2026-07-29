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
    sync::Arc,
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

/// Kind of constrained metric state transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricTransition {
    /// Moves work between the not-started and active states.
    Start,
    /// Moves work between the active and unclassified-completed states.
    Complete,
    /// Moves work between the active and succeeded states.
    Succeed,
    /// Moves work between the active and failed states.
    Fail,
    /// Moves work between the active and cancelled states.
    Cancel,
}

impl fmt::Display for MetricTransition {
    /// Formats the stable transition name used in metric errors.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => formatter.write_str("start"),
            Self::Complete => formatter.write_str("complete"),
            Self::Succeed => formatter.write_str("succeed"),
            Self::Fail => formatter.write_str("fail"),
            Self::Cancel => formatter.write_str("cancel"),
        }
    }
}

/// Failure while reading or mutating one stateful metric.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricError {
    /// The enclosing progress operation has already been closed.
    Closed {
        /// Stable ID of the metric that rejected the update.
        metric_id: String,
    },
    /// A transition attempted to remove more work than its source state held.
    InsufficientCount {
        /// Stable ID of the metric that rejected the update.
        metric_id: String,
        /// Requested constrained state transition.
        transition: MetricTransition,
        /// Absolute amount requested by the signed transition count.
        requested: u64,
        /// Work available in the source state.
        available: u64,
    },
    /// A transition would occupy more work than the configured total permits.
    TotalExceeded {
        /// Stable ID of the metric that rejected the update.
        metric_id: String,
        /// Configured metric total.
        total: u64,
        /// Active plus completed work after the rejected transition.
        attempted: u64,
    },
    /// A requested total is lower than active plus completed work.
    TotalBelowOccupied {
        /// Stable ID of the metric that rejected the total update.
        metric_id: String,
        /// Requested replacement total.
        total: u64,
        /// Active plus completed work already present.
        occupied: u64,
    },
    /// Arithmetic needed to preserve metric invariants overflowed.
    CountOverflow {
        /// Stable ID of the metric whose arithmetic overflowed.
        metric_id: String,
    },
    /// A thread panicked while holding this metric's state lock.
    StatePoisoned {
        /// Stable ID of the metric whose state cannot be read safely.
        metric_id: String,
    },
}

impl fmt::Display for MetricError {
    /// Formats a concise explanation of the rejected metric operation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed { metric_id } => {
                write!(formatter, "metric {metric_id:?} is closed")
            }
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
            Self::TotalBelowOccupied {
                metric_id,
                total,
                occupied,
            } => write!(
                formatter,
                "metric {metric_id:?} total {total} is below occupied work {occupied}"
            ),
            Self::CountOverflow { metric_id } => {
                write!(formatter, "counts for metric {metric_id:?} overflowed")
            }
            Self::StatePoisoned { metric_id } => {
                write!(
                    formatter,
                    "state lock for metric {metric_id:?} is poisoned"
                )
            }
        }
    }
}

impl Error for MetricError {}

/// Reporter failure that preserves its original error chain.
#[derive(Clone, Debug)]
pub struct ReportError {
    source: Arc<dyn Error + Send + Sync + 'static>,
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
            source: Arc::new(source),
        }
    }
    /// Creates a reporter error from a stable message.
    pub fn message(message: &str) -> Self {
        Self::new(MessageError(message.into()))
    }
    /// Returns the original reporter error.
    #[must_use]
    pub fn source_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.source.as_ref()
    }
}
impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}
impl Error for ReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Complete error returned from non-terminal progress operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressError {
    /// Caller supplied invalid state.
    Validation(ValidationError),
    /// A stateful metric rejected an operation.
    Metric(Box<MetricError>),
    /// The reporter rejected an event.
    Report(ReportError),
}
impl fmt::Display for ProgressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Metric(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
        }
    }
}
impl Error for ProgressError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Metric(error) => Some(error),
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
impl From<MetricError> for ProgressError {
    /// Converts a metric state failure.
    fn from(error: MetricError) -> Self {
        Self::Metric(Box::new(error))
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
    /// Consumes this error and returns the elapsed duration with its cause.
    #[must_use]
    pub fn into_parts(self) -> (Duration, ProgressError) {
        (self.elapsed, self.error)
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
