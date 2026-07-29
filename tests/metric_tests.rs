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

/// Verifies metric metadata and every public transition direction.
#[test]
fn test_metric_metadata_and_all_transition_directions() {
    let metric = Metric::new("tasks", "Tasks").total(6);
    assert_eq!(metric.id(), "tasks");
    assert_eq!(metric.name(), "Tasks");
    assert_eq!(metric.configured_total(), Some(6));

    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(metric)
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");
    assert_eq!(tasks.id(), "tasks");
    assert_eq!(tasks.name(), "Tasks");

    tasks.start(6).expect("work must start");
    tasks.complete(1).expect("work must complete");
    tasks.complete(-1).expect("completion must reverse");
    tasks.succeed(1).expect("work must succeed");
    tasks.succeed(-1).expect("success must reverse");
    tasks.fail(1).expect("work must fail");
    tasks.fail(-1).expect("failure must reverse");
    tasks.cancel(1).expect("work must cancel");
    tasks.cancel(-1).expect("cancellation must reverse");
    tasks.start(-6).expect("start must reverse");

    let snapshot = tasks.snapshot().expect("snapshot must succeed");
    assert_eq!(snapshot.id(), "tasks");
    assert_eq!(snapshot.name(), "Tasks");
    assert_eq!(snapshot.total(), Some(6));
    assert_eq!(snapshot.completed(), 0);
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.succeeded(), 0);
    assert_eq!(snapshot.failed(), 0);
    assert_eq!(snapshot.cancelled(), 0);
}

/// Verifies dynamic totals reject occupied work and constrained transitions.
#[test]
fn test_metric_rejects_invalid_totals_and_counts() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    tasks.start(2).expect("work must start");
    assert!(matches!(
        tasks.set_total(1),
        Err(MetricError::TotalBelowOccupied { .. })
    ));
    tasks.set_total(2).expect("occupied total must be accepted");
    assert!(matches!(
        tasks.start(1),
        Err(MetricError::TotalExceeded { .. })
    ));
    assert!(matches!(
        tasks.complete(3),
        Err(MetricError::InsufficientCount { .. })
    ));
    assert!(matches!(
        tasks.start(i64::MIN),
        Err(MetricError::InsufficientCount { .. })
    ));

    let overflow_progress = Progress::builder(&reporter)
        .metric(Metric::new("overflow", "Overflow"))
        .start()
        .expect("unbounded progress must start");
    let overflow = overflow_progress
        .metric("overflow")
        .expect("overflow metric must exist");
    overflow.start(i64::MAX).expect("first large count must fit");
    overflow.start(i64::MAX).expect("second large count must fit");
    overflow.start(1).expect("maximum count must fit");
    assert!(matches!(
        overflow.start(1),
        Err(MetricError::CountOverflow { .. })
    ));
}
