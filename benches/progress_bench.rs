// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Criterion benchmarks for the redesigned progress reporting paths.

use std::{
    hint::black_box,
    sync::{
        Arc,
        Mutex,
    },
    thread,
    time::Duration,
};

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_progress::{
    Metric,
    NoopReporter,
    Progress,
    ReportError,
};

/// Benchmarks the disabled report fast path.
fn bench_disabled_report(criterion: &mut Criterion) {
    let reporter = NoopReporter;
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("disabled progress must start");
    let metric = progress
        .metric("entries")
        .expect("configured metric must exist");
    metric.start(1).expect("metric update must succeed");
    metric.complete(1).expect("metric update must succeed");
    criterion.bench_function("disabled_report", |bencher| {
        bencher.iter(|| {
            progress.report().expect("disabled report must succeed");
        });
    });
}

/// Benchmarks an enabled running report with one fully configured metric.
fn bench_enabled_report(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReportError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    let metric = progress
        .metric("entries")
        .expect("configured metric must exist");
    metric
        .start(black_box(1))
        .expect("metric update must succeed");
    metric.complete(1).expect("metric update must succeed");
    criterion.bench_function("enabled_report", |bencher| {
        bencher.iter(|| {
            progress.report().expect("enabled report must succeed");
        });
    });
}

/// Benchmarks the positive-interval path that skips an undued report closure.
fn bench_not_due_report(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReportError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    criterion.bench_function("not_due_report", |bencher| {
        bencher.iter(|| {
            progress
                .report_if_due()
                .expect("undued report must succeed");
        });
    });
}

/// Benchmarks creation and terminal delivery of a complete multi-metric event.
fn bench_multi_metric_terminal(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReportError>(())
    };
    criterion.bench_function("multi_metric_terminal", |bencher| {
        bencher.iter(|| {
            let progress = Progress::builder(&reporter)
                .metric(Metric::new("entries", "Entries").total(1))
                .metric(Metric::new("bytes", "Bytes").total(1024))
                .start()
                .expect("progress must start");
            progress
                .metric("entries")
                .expect("configured metric must exist")
                .start(1)
                .and_then(|()| {
                    progress
                        .metric("entries")
                        .expect("configured metric must exist")
                        .succeed(1)
                })
                .expect("metric update must succeed");
            progress
                .metric("bytes")
                .expect("configured metric must exist")
                .start(1024)
                .and_then(|()| {
                    progress
                        .metric("bytes")
                        .expect("configured metric must exist")
                        .complete(1024)
                })
                .expect("metric update must succeed");
            progress.finish().expect("terminal event must report");
        });
    });
}

/// Benchmarks status polling for a disabled automatic reporter.
fn bench_disabled_auto_reporter_status(criterion: &mut Criterion) {
    let reporter = NoopReporter;
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries"))
        .start()
        .expect("disabled progress must start");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        let status = auto.status();
        criterion.bench_function("disabled_auto_reporter_status", |bencher| {
            bencher.iter(|| black_box(status.is_failed()));
        });
        auto.stop().expect("inert reporter must stop cleanly");
    });
}

/// Benchmarks notification calls for a heartbeat-driven automatic reporter.
fn bench_heartbeat_auto_reporter_notification(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReportError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries"))
        .start()
        .expect("enabled progress must start");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        let notifier = auto.notifier();
        criterion.bench_function(
            "heartbeat_auto_reporter_notification",
            |bencher| {
                bencher.iter(|| notifier.notify());
            },
        );
        auto.stop().expect("heartbeat reporter must stop cleanly");
    });
}

/// Benchmarks contention from concurrent worker updates on a single metric.
fn bench_metric_handle_contention(criterion: &mut Criterion) {
    const UPDATES_PER_WORKER: u64 = 2048;
    let mut group = criterion.benchmark_group("metric_handle_contention");
    for workers in [1usize, 2, 4, 8, 16, 32, 64] {
        let total = UPDATES_PER_WORKER * workers as u64;
        group.throughput(Throughput::Elements(total));
        group.bench_with_input(
            BenchmarkId::new("metric_handle_contention", workers),
            &workers,
            |bencher, &workers| {
                bencher.iter(|| {
                    let reporter = NoopReporter;
                    let progress = Progress::builder(&reporter)
                        .metric(Metric::new("entries", "Entries").total(total))
                        .start()
                        .expect("enabled progress must start");
                    let metric = progress
                        .metric("entries")
                        .expect("configured metric must exist");
                    metric.start(total).expect("metric start must succeed");

                    thread::scope(|scope| {
                        for _ in 0..workers {
                            let metric = metric.clone();
                            scope.spawn(move || {
                                for _ in 0..UPDATES_PER_WORKER {
                                    metric
                                        .complete(1)
                                        .expect("metric update must succeed");
                                }
                            });
                        }
                    });

                    progress.finish().expect("terminal event must report");
                });
            },
        );
    }
    group.finish();
}

/// Counts completed work for the mutex contention baseline.
#[derive(Debug)]
struct MutexMetricCounts {
    /// Work that remains active.
    active: u64,
    /// Terminal work without an explicit classification.
    completed_unclassified: u64,
    /// Terminal work classified as successful.
    succeeded: u64,
    /// Terminal work classified as failed.
    failed: u64,
    /// Terminal work classified as cancelled.
    cancelled: u64,
}

/// Benchmarks a mutex-protected equivalent of one metric's hot update path.
fn bench_mutex_metric_contention(criterion: &mut Criterion) {
    const UPDATES_PER_WORKER: u64 = 2048;
    let mut group = criterion.benchmark_group("mutex_metric_contention");
    for workers in [1usize, 2, 4, 8, 16, 32, 64] {
        let total = UPDATES_PER_WORKER * workers as u64;
        group.throughput(Throughput::Elements(total));
        group.bench_with_input(
            BenchmarkId::new("mutex_metric_contention", workers),
            &workers,
            |bencher, &workers| {
                bencher.iter(|| {
                    let state = Arc::new(Mutex::new(MutexMetricCounts {
                        active: total,
                        completed_unclassified: 0,
                        succeeded: 0,
                        failed: 0,
                        cancelled: 0,
                    }));
                    thread::scope(|scope| {
                        for _ in 0..workers {
                            let state = Arc::clone(&state);
                            scope.spawn(move || {
                                for _ in 0..UPDATES_PER_WORKER {
                                    let mut state = state
                                        .lock()
                                        .expect("mutex must not poison");
                                    state.active -= 1;
                                    state.completed_unclassified += 1;
                                }
                            });
                        }
                    });
                    let state = state.lock().expect("mutex must not poison");
                    black_box((
                        state.active,
                        state.completed_unclassified,
                        state.succeeded,
                        state.failed,
                        state.cancelled,
                    ));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    progress_benches,
    bench_disabled_report,
    bench_enabled_report,
    bench_not_due_report,
    bench_multi_metric_terminal,
    bench_disabled_auto_reporter_status,
    bench_heartbeat_auto_reporter_notification,
    bench_metric_handle_contention,
    bench_mutex_metric_contention
);
criterion_main!(progress_benches);
