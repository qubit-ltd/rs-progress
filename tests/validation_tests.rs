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
