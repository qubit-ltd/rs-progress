// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Validation behavior for fixed progress configuration.

use qubit_progress::{
    Metric,
    NoopReporter,
    Progress,
    ProgressError,
    ValidationError,
    Stage,
};

/// Verifies that operations require at least one configured metric.
#[test]
fn test_progress_rejects_missing_metrics() {
    let error = match Progress::builder(&NoopReporter).start() {
        Ok(_) => panic!("a progress operation requires one metric"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ProgressError::Validation(ValidationError::NoMetrics)
    ));
}

/// Verifies that blank metric metadata is rejected before reporter delivery.
#[test]
fn test_progress_rejects_blank_metric_metadata() {
    let result = Progress::builder(&NoopReporter)
        .metric(Metric::new(" ", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("a blank metric ID must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ProgressError::Validation(ValidationError::EmptyMetricId { .. })
    ));
}

/// Verifies fixed metric and stage metadata reject every invalid shape.
#[test]
fn test_progress_rejects_invalid_fixed_metadata() {
    let reporter = NoopReporter;
    let cases = [
        (
            Progress::builder(&reporter).metric(Metric::new("tasks", " ")).start(),
            "empty metric name",
        ),
        (
            Progress::builder(&reporter)
                .metric(Metric::new("tasks", "Tasks"))
                .metric(Metric::new("tasks", "Other"))
                .start(),
            "duplicate metric ID",
        ),
        (
            Progress::builder(&reporter)
                .stage(Stage::new(" ", "Copy"))
                .metric(Metric::new("tasks", "Tasks"))
                .start(),
            "empty stage ID",
        ),
        (
            Progress::builder(&reporter)
                .stage(Stage::new("copy", " "))
                .metric(Metric::new("tasks", "Tasks"))
                .start(),
            "empty stage name",
        ),
        (
            Progress::builder(&reporter)
                .stage(Stage::new("copy", "Copy").position(0, 1))
                .metric(Metric::new("tasks", "Tasks"))
                .start(),
            "zero stage position",
        ),
        (
            Progress::builder(&reporter)
                .stage(Stage::new("copy", "Copy").position(2, 1))
                .metric(Metric::new("tasks", "Tasks"))
                .start(),
            "out of range stage position",
        ),
    ];
    for (result, description) in cases {
        assert!(result.is_err(), "{description} must be rejected");
    }
}
