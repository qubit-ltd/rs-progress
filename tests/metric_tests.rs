// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for stateful progress metrics.

use qubit_progress::{
    Metric,
    MetricError,
    NoopReporter,
    Progress,
};

/// Verifies that constrained state transitions expose one coherent snapshot.
#[test]
fn test_metric_handle_transitions_publish_one_consistent_snapshot() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(4))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");

    tasks.start(4).expect("starting work must succeed");
    tasks.succeed(2).expect("successful work must succeed");
    tasks.fail(1).expect("failed work must succeed");

    let snapshot = tasks.snapshot().expect("metric snapshot must succeed");
    assert_eq!(snapshot.active(), 1);
    assert_eq!(snapshot.completed(), 3);
    assert_eq!(snapshot.succeeded(), 2);
    assert_eq!(snapshot.failed(), 1);
    assert_eq!(snapshot.cancelled(), 0);
}

/// Verifies that signed transitions roll back only their matching state.
#[test]
fn test_metric_handle_reverses_matching_transitions_and_rejects_closed_updates()
{
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");

    tasks.start(2).expect("work must start");
    tasks.cancel(1).expect("work must cancel");
    tasks.cancel(-1).expect("cancellation must roll back");
    tasks.complete(1).expect("work must complete");
    tasks.complete(-1).expect("completion must roll back");
    let snapshot = tasks.snapshot().expect("snapshot must remain readable");
    assert_eq!(snapshot.active(), 2);
    assert_eq!(snapshot.completed(), 0);

    drop(progress);
    assert!(matches!(tasks.start(1), Err(MetricError::Closed { .. })));
}
