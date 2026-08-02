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
    sync::atomic::{AtomicUsize, Ordering},
};

use qubit_progress::{
    CompletionError, ConfigurationError, EmissionError, Event, FinishError, Metric, MetricError,
    MetricTransition, OperationLifecycle, Progress, Reporter, ReporterError, StartError,
};

#[derive(Debug)]
struct OriginalReporterError;

impl fmt::Display for OriginalReporterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("original reporter error")
    }
}

impl Error for OriginalReporterError {}

struct TerminalFailingReporter {
    attempts: AtomicUsize,
}

impl TerminalFailingReporter {
    const fn new() -> Self {
        Self {
            attempts: AtomicUsize::new(0),
        }
    }
}

impl Reporter for TerminalFailingReporter {
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.attempts.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReporterError::message("terminal delivery failed"))
        }
    }
}

#[test]
fn test_reporter_error_clone_preserves_original_source() {
    let original = ReporterError::new(OriginalReporterError);
    let cloned = original.clone();
    assert!(
        cloned
            .source_error()
            .downcast_ref::<OriginalReporterError>()
            .is_some()
    );
}

#[test]
fn test_terminal_error_retains_elapsed_and_emission_source() {
    let reporter = TerminalFailingReporter::new();
    let error = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed")
        .finish()
        .expect_err("terminal reporter failure must be retained");
    let FinishError::Terminal(terminal) = error else {
        panic!("terminal delivery must produce FinishError::Terminal");
    };
    let (elapsed, emission) = terminal.into_parts();
    assert!(elapsed < std::time::Duration::from_secs(1));
    assert!(matches!(emission, EmissionError::Delivery(_)));
}

#[test]
fn test_public_error_variants_format() {
    let configuration = [
        ConfigurationError::NoMetrics,
        ConfigurationError::EmptyMetricId { index: 3 },
        ConfigurationError::EmptyMetricName {
            metric_id: "tasks".into(),
        },
        ConfigurationError::DuplicateMetricId {
            metric_id: "tasks".into(),
        },
        ConfigurationError::EmptyStageId,
        ConfigurationError::EmptyStageName,
        ConfigurationError::IncompleteStagePosition,
        ConfigurationError::InvalidStagePosition {
            position: 3,
            total: 2,
        },
    ];
    for error in configuration {
        assert!(!error.to_string().is_empty());
        assert!(Error::source(&error).is_none());
    }

    let completion = [
        CompletionError::ActiveWork {
            metric_id: "tasks".into(),
            active: 1,
        },
        CompletionError::IncompleteTotal {
            metric_id: "tasks".into(),
            completed: 1,
            total: 2,
        },
    ];
    for error in completion {
        assert!(!error.to_string().is_empty());
    }

    let metric = [
        MetricError::OperationNotOpen {
            metric_id: "tasks".into(),
            state: OperationLifecycle::Closed,
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
        MetricError::CountOverflow {
            metric_id: "tasks".into(),
        },
    ];
    for error in metric {
        assert!(!error.to_string().is_empty());
        assert!(Error::source(&error).is_none());
    }
}

#[test]
fn test_start_error_preserves_delivery_source() {
    struct RejectingReporter;
    impl Reporter for RejectingReporter {
        fn report(&self, _event: &Event) -> Result<(), ReporterError> {
            Err(ReporterError::message("sink unavailable"))
        }
    }

    let result = Progress::builder(&RejectingReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("Started delivery must fail"),
        Err(error) => error,
    };
    assert!(matches!(error, StartError::Delivery(_)));
    assert!(Error::source(&error).is_some());
    assert!(error.to_string().contains("sink unavailable"));
}

#[test]
fn test_reporter_error_message_exposes_source() {
    let first = ReporterError::message("sink unavailable");
    let second = ReporterError::message("sink unavailable");
    assert_eq!(first.to_string(), second.to_string());
    assert!(Error::source(&first).is_some());
}
