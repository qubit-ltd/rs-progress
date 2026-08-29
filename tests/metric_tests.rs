// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for stateful progress metrics.

use qubit_progress::Metric;
use qubit_progress::MetricDelta;
use qubit_progress::MetricError;
use qubit_progress::NoopReporter;
use qubit_progress::Progress;

/// Verifies that constrained state transitions expose one coherent snapshot.
#[test]
fn test_metric_handle_transitions_publish_one_consistent_snapshot() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(4))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("configured metric must exist");

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

/// Verifies one delta can start and finish work atomically.
#[test]
fn test_metric_delta_commits_start_and_terminal_counts_atomically() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(10))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("configured metric must exist");

    tasks
        .apply_delta(MetricDelta::new().started(10).succeeded(7).unclassified(2).cancelled(1))
        .expect("compound update must succeed");
    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.completed(), 10);
    assert_eq!(snapshot.unclassified(), 2);
    assert_eq!(snapshot.succeeded(), 7);
    assert_eq!(snapshot.cancelled(), 1);
}

/// Verifies an invalid compound update leaves every counter unchanged.
#[test]
fn test_metric_delta_rejects_without_partial_commit() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    let error = tasks
        .apply_delta(MetricDelta::new().started(1).succeeded(2))
        .expect_err("a delta cannot complete more than its newly started work");
    assert!(matches!(error, MetricError::InsufficientActive { .. }));
    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.active(), 0);
    assert_eq!(snapshot.completed(), 0);
}

/// Verifies metric metadata and every public forward transition.
#[test]
fn test_metric_metadata_and_forward_transitions() {
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
    tasks.succeed(1).expect("work must succeed");
    tasks.fail(1).expect("work must fail");
    tasks.cancel(1).expect("work must cancel");

    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.id(), "tasks");
    assert_eq!(snapshot.name(), "Tasks");
    assert_eq!(snapshot.total(), Some(6));
    assert_eq!(snapshot.completed(), 4);
    assert_eq!(snapshot.active(), 2);
    assert_eq!(snapshot.unclassified(), 1);
    assert_eq!(snapshot.succeeded(), 1);
    assert_eq!(snapshot.failed(), 1);
    assert_eq!(snapshot.cancelled(), 1);
    assert_eq!(snapshot.completion_fraction(), Some(4.0 / 6.0));
}

/// Verifies configured totals and constrained transitions reject invalid work
/// counts.
#[test]
fn test_metric_rejects_invalid_counts_and_overflow() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    tasks.start(2).expect("work must start");
    assert!(matches!(tasks.start(1), Err(MetricError::TotalExceeded { .. })));
    assert!(matches!(tasks.complete(3), Err(MetricError::InsufficientActive { .. })));

    let overflow_progress = Progress::builder(&reporter)
        .metric(Metric::new("overflow", "Overflow"))
        .start()
        .expect("unbounded progress must start");
    let overflow = overflow_progress
        .metric("overflow")
        .expect("overflow metric must exist");
    overflow.start(u64::MAX).expect("first large count must fit");
    assert!(matches!(overflow.start(1), Err(MetricError::CountOverflow { .. })));
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
    tasks.succeed(u64::MAX).expect("maximum terminal count must succeed");
    assert_eq!(tasks.snapshot().succeeded(), u64::MAX,);
}

/// Verifies a new active count cannot overflow completed work.
#[test]
fn test_metric_rejects_start_after_maximum_completed_count() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("unbounded progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");

    tasks.start(u64::MAX).expect("large work must start");
    tasks.succeed(u64::MAX).expect("maximum terminal count must succeed");

    assert!(matches!(
        tasks.start(1),
        Err(MetricError::CountOverflow { metric_id }) if metric_id == "tasks"
    ));
    let snapshot = tasks.snapshot();
    assert_eq!(snapshot.completed(), u64::MAX);
    assert_eq!(snapshot.active(), 0);
}

/// Verifies every additive counter reports overflow without a partial update.
#[test]
fn test_metric_delta_rejects_each_counter_overflow() {
    let reporter = NoopReporter;
    for (name, delta) in [
        ("unclassified", MetricDelta::new().started(1).unclassified(1)),
        ("succeeded", MetricDelta::new().started(1).succeeded(1)),
        ("failed", MetricDelta::new().started(1).failed(1)),
        ("cancelled", MetricDelta::new().started(1).cancelled(1)),
    ] {
        let progress = Progress::builder(&reporter)
            .metric(Metric::new(name, name))
            .start()
            .expect("progress must start");
        let metric = progress.metric(name).expect("metric must exist");
        metric.start(u64::MAX).expect("maximum active count must fit");
        match name {
            "unclassified" => metric.complete(u64::MAX),
            "succeeded" => metric.succeed(u64::MAX),
            "failed" => metric.fail(u64::MAX),
            "cancelled" => metric.cancel(u64::MAX),
            _ => unreachable!(),
        }
        .expect("maximum terminal count must fit");
        assert!(matches!(
            metric.apply_delta(delta),
            Err(MetricError::CountOverflow { .. })
        ));
        let snapshot = metric.snapshot();
        assert_eq!(snapshot.active(), 0);
        assert_eq!(snapshot.completed(), u64::MAX);
    }

    let progress = Progress::builder(&reporter)
        .metric(Metric::new("terminal-overflow", "Terminal overflow"))
        .start()
        .expect("progress must start");
    let metric = progress.metric("terminal-overflow").expect("metric must exist");
    assert!(matches!(
        metric.apply_delta(MetricDelta::new().unclassified(u64::MAX).succeeded(1),),
        Err(MetricError::CountOverflow { .. })
    ));
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
    use std::sync::Arc;
    use std::sync::Barrier;

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
    assert_eq!(snapshot.unclassified(), TOTAL);
}
