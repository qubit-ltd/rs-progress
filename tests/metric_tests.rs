// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for stateful progress metrics.

use qubit_progress::{Metric, MetricError, MetricTransition, NoopReporter, Progress};

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

    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 1);
    assert_eq!(snapshot.completed(), 3);
    assert_eq!(snapshot.succeeded(), 2);
    assert_eq!(snapshot.failed(), 1);
    assert_eq!(snapshot.cancelled(), 0);
    assert_eq!(snapshot.completion_fraction(), Some(0.75));
}

/// Verifies that explicit rollbacks affect only their matching state.
#[test]
fn test_metric_handle_rolls_back_matching_transitions_and_rejects_closed_updates() {
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
    tasks
        .rollback(MetricTransition::Cancel, 1)
        .expect("cancellation must roll back");
    tasks.complete(1).expect("work must complete");
    tasks
        .rollback(MetricTransition::Complete, 1)
        .expect("completion must roll back");
    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 2);
    assert_eq!(snapshot.completed(), 0);

    drop(progress);
    assert!(matches!(tasks.start(1), Err(MetricError::Closed { .. })));
}

/// Verifies metric metadata and every public forward and rollback transition.
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
    tasks
        .rollback(MetricTransition::Complete, 1)
        .expect("completion must reverse");
    tasks.succeed(1).expect("work must succeed");
    tasks
        .rollback(MetricTransition::Succeed, 1)
        .expect("success must reverse");
    tasks.fail(1).expect("work must fail");
    tasks
        .rollback(MetricTransition::Fail, 1)
        .expect("failure must reverse");
    tasks.cancel(1).expect("work must cancel");
    tasks
        .rollback(MetricTransition::Cancel, 1)
        .expect("cancellation must reverse");
    tasks
        .rollback(MetricTransition::Start, 6)
        .expect("start must reverse");

    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.id(), "tasks");
    assert_eq!(snapshot.name(), "Tasks");
    assert_eq!(snapshot.total(), Some(6));
    assert_eq!(snapshot.completed(), 0);
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.succeeded(), 0);
    assert_eq!(snapshot.failed(), 0);
    assert_eq!(snapshot.cancelled(), 0);
    assert_eq!(snapshot.completion_fraction(), Some(0.0));
}

/// Verifies configured totals and constrained transitions reject invalid work counts.
#[test]
fn test_metric_rejects_invalid_counts_and_overflow() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    tasks.start(2).expect("work must start");
    assert!(matches!(
        tasks.start(1),
        Err(MetricError::TotalExceeded { .. })
    ));
    assert!(matches!(
        tasks.complete(3),
        Err(MetricError::InsufficientCount { .. })
    ));
    assert!(matches!(
        tasks.rollback(MetricTransition::Succeed, 1),
        Err(MetricError::InsufficientCount { .. })
    ));
    assert!(matches!(
        tasks.rollback(MetricTransition::Start, 3),
        Err(MetricError::InsufficientCount { .. })
    ));

    let overflow_progress = Progress::builder(&reporter)
        .metric(Metric::new("overflow", "Overflow"))
        .start()
        .expect("unbounded progress must start");
    let overflow = overflow_progress
        .metric("overflow")
        .expect("overflow metric must exist");
    overflow
        .start(u64::MAX)
        .expect("first large count must fit");
    assert!(matches!(
        overflow.start(1),
        Err(MetricError::CountOverflow { .. })
    ));
    assert_eq!(overflow.snapshot().completion_fraction(), None);
}

/// Verifies terminal count arithmetic accepts the largest representable count.
#[test]
fn test_metric_accepts_maximum_terminal_count() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    tasks.start(u64::MAX).expect("large work must start");
    tasks
        .succeed(u64::MAX)
        .expect("maximum terminal count must succeed");
    assert_eq!(tasks.snapshot().succeeded(), u64::MAX,);
}

/// Verifies concurrent updates preserve a coherent aggregate snapshot.
#[test]
fn test_metric_handle_concurrent_updates_preserve_snapshot_invariants() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(64))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    std::thread::scope(|scope| {
        for _ in 0..4 {
            let tasks = tasks.clone();
            scope.spawn(move || {
                tasks.start(16).expect("concurrent work must start");
                tasks.succeed(16).expect("concurrent work must succeed");
            });
        }
    });

    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.completed(), 64);
    assert_eq!(snapshot.succeeded(), 64);
    assert_eq!(snapshot.failed(), 0);
    assert_eq!(snapshot.cancelled(), 0);
}

/// Verifies readers observe conservation while writers update one metric.
#[test]
fn test_metric_handle_concurrent_snapshots_preserve_conservation() {
    use std::sync::{Arc, Barrier};

    const WRITERS: usize = 4;
    const READERS: usize = 4;
    const UPDATES_PER_WRITER: u64 = 25_000;
    const TOTAL: u64 = WRITERS as u64 * UPDATES_PER_WRITER;

    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(TOTAL))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");
    tasks.start(TOTAL).expect("work must start");

    let barrier = Arc::new(Barrier::new(WRITERS + READERS));
    std::thread::scope(|scope| {
        for _ in 0..WRITERS {
            let barrier = Arc::clone(&barrier);
            let tasks = tasks.clone();
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..UPDATES_PER_WRITER {
                    tasks.complete(1).expect("work must complete");
                }
            });
        }
        for _ in 0..READERS {
            let barrier = Arc::clone(&barrier);
            let tasks = tasks.clone();
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..UPDATES_PER_WRITER {
                    let snapshot = tasks.snapshot();
                    assert_eq!(
                        snapshot
                            .active()
                            .checked_add(snapshot.completed())
                            .expect("metric counts must not overflow"),
                        TOTAL,
                        "snapshot must conserve total work"
                    );
                }
            });
        }
    });

    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.completed(), TOTAL);
}
