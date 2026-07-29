// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for progress error values.

use std::{
    error::Error,
    fmt,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use qubit_progress::{
    Event,
    Metric,
    MetricError,
    MetricTransition,
    Progress,
    ProgressError,
    ReportError,
    Reporter,
    ValidationError,
};

/// Error type used to verify that clones preserve the original error object.
#[derive(Debug)]
struct OriginalReporterError;

impl fmt::Display for OriginalReporterError {
    /// Formats the stable test error message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("original reporter error")
    }
}

impl Error for OriginalReporterError {}

/// Reporter that accepts Started and rejects the terminal event.
struct TerminalFailingReporter {
    /// Number of delivery attempts observed by this reporter.
    attempts: AtomicUsize,
}

impl TerminalFailingReporter {
    /// Creates a reporter that fails after accepting Started.
    const fn new() -> Self {
        Self {
            attempts: AtomicUsize::new(0),
        }
    }
}

impl Reporter for TerminalFailingReporter {
    /// Rejects every event after the first Started delivery.
    fn report(&self, _event: &Event) -> Result<(), ReportError> {
        if self.attempts.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReportError::message("terminal delivery failed"))
        }
    }
}

/// Verifies that cloning a reporter error retains its concrete source type.
#[test]
fn test_report_error_clone_preserves_original_source() {
    let original = ReportError::new(OriginalReporterError);
    let cloned = original.clone();

    assert!(
        cloned
            .source_error()
            .downcast_ref::<OriginalReporterError>()
            .is_some(),
        "cloned reporter error must retain the original error source",
    );
}

/// Verifies that terminal errors return elapsed time and the original cause.
#[test]
fn test_terminal_error_into_parts_returns_elapsed_and_progress_error() {
    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    let error = progress
        .finish()
        .expect_err("terminal reporter failure must be retained");
    let expected_elapsed = error.elapsed();
    let (elapsed, progress_error) = error.into_parts();

    assert_eq!(elapsed, expected_elapsed);
    assert!(matches!(progress_error, ProgressError::Report(_)));
}

/// Verifies that every public validation error has a stable explanation.
#[test]
fn test_validation_errors_format_every_variant() {
    let errors = [
        ValidationError::NoMetrics,
        ValidationError::EmptyMetricId { index: 3 },
        ValidationError::EmptyMetricName {
            metric_id: "tasks".into(),
        },
        ValidationError::DuplicateMetricId {
            metric_id: "tasks".into(),
        },
        ValidationError::CountOverflow {
            metric_id: "tasks".into(),
        },
        ValidationError::ClassifiedExceedsCompleted {
            metric_id: "tasks".into(),
        },
        ValidationError::CountsExceedTotal {
            metric_id: "tasks".into(),
        },
        ValidationError::NonZeroCountsForZeroTotal {
            metric_id: "tasks".into(),
        },
        ValidationError::EmptyStageId,
        ValidationError::EmptyStageName,
        ValidationError::IncompleteStagePosition,
        ValidationError::InvalidStagePosition {
            position: 3,
            total: 2,
        },
        ValidationError::OperationIdExhausted,
        ValidationError::SequenceExhausted,
    ];

    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(Error::source(&error).is_none());
    }
}

/// Verifies that every metric error and transition preserves its context.
#[test]
fn test_metric_errors_and_transitions_format_every_variant() {
    for (transition, name) in [
        (MetricTransition::Start, "start"),
        (MetricTransition::Complete, "complete"),
        (MetricTransition::Succeed, "succeed"),
        (MetricTransition::Fail, "fail"),
        (MetricTransition::Cancel, "cancel"),
    ] {
        assert_eq!(transition.to_string(), name);
    }

    let errors = [
        MetricError::Closed {
            metric_id: "tasks".into(),
        },
        MetricError::InsufficientCount {
            metric_id: "tasks".into(),
            transition: MetricTransition::Start,
            requested: 2,
            available: 1,
        },
        MetricError::TotalExceeded {
            metric_id: "tasks".into(),
            total: 1,
            attempted: 2,
        },
        MetricError::TotalBelowOccupied {
            metric_id: "tasks".into(),
            total: 1,
            occupied: 2,
        },
        MetricError::CountOverflow {
            metric_id: "tasks".into(),
        },
        MetricError::StatePoisoned {
            metric_id: "tasks".into(),
        },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(Error::source(&error).is_none());
    }
}

/// Verifies error conversion, source chains, and terminal accessors.
#[test]
fn test_progress_and_terminal_errors_preserve_sources() {
    let validation = ProgressError::from(ValidationError::NoMetrics);
    let metric = ProgressError::from(MetricError::Closed {
        metric_id: "tasks".into(),
    });
    let report = ProgressError::from(ReportError::message("sink unavailable"));
    for error in [&validation, &metric, &report] {
        assert!(!error.to_string().is_empty());
        assert!(Error::source(error).is_some());
    }

    let reporter = TerminalFailingReporter::new();
    let terminal = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed")
        .finish()
        .expect_err("terminal event must fail");
    assert!(terminal.elapsed() < std::time::Duration::from_secs(1));
    assert!(matches!(terminal.progress_error(), ProgressError::Report(_)));
    assert!(terminal.to_string().contains("terminal progress report failed"));
    assert!(Error::source(&terminal).is_some());
    assert!(matches!(terminal.into_progress_error(), ProgressError::Report(_)));
}

/// Verifies message-backed errors expose the original text and error source.
#[test]
fn test_report_error_message_compares_and_exposes_source() {
    let first = ReportError::message("sink unavailable");
    let second = ReportError::message("sink unavailable");
    assert_eq!(first, second);
    assert_eq!(first.to_string(), "sink unavailable");
    assert!(Error::source(&first).is_some());
}
