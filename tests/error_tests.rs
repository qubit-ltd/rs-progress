// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for progress error values.

use std::error::Error;
use std::fmt;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_progress::CompletionError;
use qubit_progress::ConfigurationError;
use qubit_progress::DeliveryError;
use qubit_progress::EmissionError;
use qubit_progress::Event;
use qubit_progress::FinishError;
use qubit_progress::Metric;
use qubit_progress::MetricDelta;
use qubit_progress::MetricError;
use qubit_progress::OperationLifecycle;
use qubit_progress::Phase;
use qubit_progress::Progress;
use qubit_progress::Reporter;
use qubit_progress::ReporterError;
use qubit_progress::StartError;

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

struct RejectingReporter;

impl Reporter for RejectingReporter {
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        Err(ReporterError::message("sink unavailable"))
    }
}

fn start_delivery_error() -> DeliveryError {
    let result = Progress::builder(&RejectingReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    match result {
        Err(StartError::Delivery(error)) => error,
        Err(_) => panic!("start must return a delivery error"),
        Ok(_) => panic!("start must fail"),
    }
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
    assert!(cloned.source_error().downcast_ref::<OriginalReporterError>().is_some());
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
    assert!(terminal.elapsed() < std::time::Duration::from_secs(1));
    assert!(terminal.emission_error().to_string().contains("terminal"));
    assert!(terminal.to_string().contains("terminal progress report failed"));
    assert!(Error::source(&terminal).is_some());
    let (elapsed, emission) = terminal.into_parts();
    assert!(elapsed < std::time::Duration::from_secs(1));
    assert!(matches!(emission, EmissionError::Delivery(_)));
}

#[test]
fn test_terminal_error_can_extract_only_its_emission_error() {
    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");
    let FinishError::Terminal(terminal) = progress
        .finish()
        .expect_err("terminal reporter failure must be retained")
    else {
        panic!("terminal delivery must produce FinishError::Terminal");
    };
    assert!(matches!(terminal.into_emission_error(), EmissionError::Delivery(_)));
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
        ConfigurationError::EmptyAttributeKey { key: "   ".into() },
        ConfigurationError::EmptyStageId,
        ConfigurationError::EmptyStageName,
        ConfigurationError::IncompleteStagePosition,
        ConfigurationError::InvalidStagePosition { position: 3, total: 2 },
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
        MetricError::InsufficientActive {
            metric_id: "tasks".into(),
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

    let delta = MetricDelta::new().started(2).unclassified(1).succeeded(1);
    assert_eq!(delta, MetricDelta::new().started(2).unclassified(1).succeeded(1));
}

#[test]
fn test_start_error_preserves_delivery_source() {
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
fn test_delivery_error_exposes_all_accessors_and_error_chain() {
    let error = start_delivery_error();
    assert_eq!(error.event().phase(), Phase::Started);
    assert_eq!(error.reporter_error().to_string(), "sink unavailable");
    assert!(error.to_string().contains("delivery of started event"));
    assert!(Error::source(&error).is_some());

    let event = error.into_event();
    assert_eq!(event.sequence(), 0);

    let error = start_delivery_error();
    let reporter_error = error.into_reporter_error();
    assert_eq!(reporter_error.to_string(), "sink unavailable");
}

#[test]
fn test_emission_error_formats_sequence_and_delivery_failures() {
    let exhausted = EmissionError::SequenceExhausted;
    assert_eq!(exhausted.to_string(), "progress event sequence is exhausted");
    assert!(Error::source(&exhausted).is_none());

    let delivery = EmissionError::Delivery(start_delivery_error());
    assert!(delivery.to_string().contains("delivery of started event"));
    assert!(Error::source(&delivery).is_some());
}

#[test]
fn test_finish_error_supports_recovery_and_terminal_parts() {
    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    progress
        .metric("tasks")
        .expect("metric must exist")
        .start(1)
        .expect("work must start");
    progress
        .metric("tasks")
        .expect("metric must exist")
        .succeed(1)
        .expect("work must succeed");
    let incomplete = progress
        .finish_recoverable()
        .expect_err("incomplete work must reject checked finish");
    assert!(incomplete.completion_error().is_some());
    assert!(incomplete.to_string().contains("work items"));
    assert!(format!("{incomplete:?}").contains("RecoverableFinishError::Incomplete"));
    assert!(Error::source(&incomplete).is_some());
    let returned = incomplete
        .into_progress()
        .expect("incomplete finish must return progress");
    drop(returned);

    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    let terminal = progress.finish_recoverable().expect_err("terminal delivery must fail");
    assert!(terminal.completion_error().is_none());
    assert!(format!("{terminal:?}").contains("RecoverableFinishError::Terminal"));
    assert!(terminal.to_string().contains("terminal progress report failed"));
    assert!(Error::source(&terminal).is_some());
    assert!(terminal.into_progress().is_err());

    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    progress
        .metric("tasks")
        .expect("metric must exist")
        .start(1)
        .expect("work must start");
    let incomplete = progress
        .finish()
        .expect_err("incomplete work must reject checked finish");
    assert!(incomplete.completion_error().is_some());
    let incomplete_elapsed = incomplete.elapsed();
    assert!(incomplete_elapsed < std::time::Duration::from_secs(1));
    assert!(incomplete.to_string().contains("work items"));
    assert!(format!("{incomplete:?}").contains("Incomplete"));
    assert!(Error::source(&incomplete).is_some());
    let FinishError::Incomplete { elapsed, source } = incomplete else {
        panic!("incomplete finish must return its completion error");
    };
    assert_eq!(elapsed, incomplete_elapsed);
    assert!(matches!(source, CompletionError::ActiveWork { .. }));

    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    let terminal = progress.finish().expect_err("terminal delivery must fail");
    assert!(terminal.completion_error().is_none());
    let finish_elapsed = terminal.elapsed();
    assert!(finish_elapsed < std::time::Duration::from_secs(1));
    assert!(terminal.to_string().contains("terminal progress report failed"));
    assert!(format!("{terminal:?}").contains("Terminal"));
    assert!(Error::source(&terminal).is_some());
    let FinishError::Terminal(error) = terminal else {
        panic!("terminal delivery must return FinishError::Terminal");
    };
    assert_eq!(finish_elapsed, error.elapsed());
}

#[test]
fn test_start_error_conversions_cover_configuration_and_emission() {
    let invalid = StartError::from(ConfigurationError::NoMetrics);
    assert!(matches!(invalid, StartError::InvalidConfiguration(_)));
    assert!(invalid.to_string().contains("at least one metric"));
    assert!(Error::source(&invalid).is_some());

    let exhausted = StartError::OperationIdExhausted;
    assert_eq!(exhausted.to_string(), "progress operation IDs are exhausted");
    assert!(Error::source(&exhausted).is_none());

    let sequence = StartError::from(EmissionError::SequenceExhausted);
    assert!(matches!(sequence, StartError::OperationIdExhausted));

    let delivery = StartError::from(EmissionError::Delivery(start_delivery_error()));
    assert!(matches!(delivery, StartError::Delivery(_)));
}

#[test]
fn test_reporter_error_message_exposes_source() {
    let first = ReporterError::message("sink unavailable");
    let second = ReporterError::message("sink unavailable");
    assert_eq!(first.to_string(), second.to_string());
    assert!(Error::source(&first).is_some());
}
