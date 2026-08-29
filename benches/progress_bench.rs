// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Criterion benchmarks for the redesigned progress reporting paths.

use std::hint::black_box;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_progress::Metric;
use qubit_progress::NoopReporter;
use qubit_progress::Progress;
use qubit_progress::ReporterError;

/// Benchmarks the disabled report fast path.
fn bench_disabled_report(criterion: &mut Criterion) {
    let reporter = NoopReporter;
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("disabled progress must start");
    let metric = progress.metric("entries").expect("configured metric must exist");
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
        Ok::<(), ReporterError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    let metric = progress.metric("entries").expect("configured metric must exist");
    metric.start(black_box(1)).expect("metric update must succeed");
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
        Ok::<(), ReporterError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    criterion.bench_function("not_due_report", |bencher| {
        bencher.iter(|| {
            progress.report_if_due().expect("undued report must succeed");
        });
    });
}

/// Benchmarks creation and terminal delivery of a complete multi-metric event.
fn bench_multi_metric_terminal(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReporterError>(())
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
        Ok::<(), ReporterError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries"))
        .start()
        .expect("enabled progress must start");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        let notifier = auto.notifier();
        criterion.bench_function("heartbeat_auto_reporter_notification", |bencher| {
            bencher.iter(|| notifier.notify());
        });
        auto.stop().expect("heartbeat reporter must stop cleanly");
    });
}

/// Measures the lifecycle cost of one enabled automatic reporter thread.
fn bench_enabled_auto_reporter_spawn_stop(criterion: &mut Criterion) {
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReporterError>(())
    };
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries"))
        .start()
        .expect("enabled progress must start");

    criterion.bench_function("enabled_auto_reporter_spawn_stop", |bencher| {
        bencher.iter(|| {
            thread::scope(|scope| {
                let auto = progress.spawn_auto_reporter(scope);
                auto.stop().expect("auto reporter must stop cleanly");
            });
        });
    });
    progress.cancel().expect("terminal event must report");
}

/// Measures worker fan-in through one automatic reporter and one shared metric.
fn bench_auto_reporter_worker_fan_in(criterion: &mut Criterion) {
    const UPDATES_PER_WORKER: u64 = 256;
    let reporter = |event: &qubit_progress::Event| {
        black_box(event);
        Ok::<(), ReporterError>(())
    };
    let mut group = criterion.benchmark_group("auto_reporter_worker_fan_in");
    for workers in [1usize, 2, 4, 8, 16, 32, 64] {
        let total = UPDATES_PER_WORKER * workers as u64;
        group.throughput(Throughput::Elements(total));
        group.bench_with_input(
            BenchmarkId::new("auto_reporter_worker_fan_in", workers),
            &workers,
            |bencher, &workers| {
                bencher.iter_batched(
                    || {
                        let progress = Progress::builder(&reporter)
                            .interval(Duration::ZERO)
                            .metric(Metric::new("entries", "Entries").total(total))
                            .start()
                            .expect("enabled progress must start");
                        let metric = progress.metric("entries").expect("configured metric must exist");
                        metric.start(total).expect("metric start must succeed");
                        (progress, metric)
                    },
                    |(mut progress, metric)| {
                        thread::scope(|scope| {
                            let auto = progress.spawn_auto_reporter(scope);
                            let notifier = auto.notifier();
                            let mut handles = Vec::with_capacity(workers);
                            for _ in 0..workers {
                                let metric = metric.clone();
                                let notifier = notifier.clone();
                                handles.push(scope.spawn(move || {
                                    metric.complete(UPDATES_PER_WORKER).expect("metric update must succeed");
                                    notifier.notify();
                                }));
                            }
                            for handle in handles {
                                handle.join().expect("worker must finish");
                            }
                            auto.stop().expect("auto reporter must stop cleanly");
                        });
                        black_box(metric.snapshot());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks concurrent metric updates after setup has been removed from
/// timing.
fn bench_metric_handle_contention(criterion: &mut Criterion) {
    const UPDATES_PER_WORKER: u64 = 2048;
    let reporter = NoopReporter;
    let mut group = criterion.benchmark_group("metric_handle_contention");
    for workers in [1usize, 2, 4, 8, 16, 32, 64] {
        let total = UPDATES_PER_WORKER * workers as u64;
        group.throughput(Throughput::Elements(total));
        group.bench_with_input(
            BenchmarkId::new("metric_handle_contention", workers),
            &workers,
            |bencher, &workers| {
                bencher.iter_batched(
                    || {
                        let progress = Progress::builder(&reporter)
                            .metric(Metric::new("entries", "Entries").total(total))
                            .start()
                            .expect("enabled progress must start");
                        let metric = progress.metric("entries").expect("configured metric must exist");
                        metric.start(total).expect("metric start must succeed");
                        (progress, metric)
                    },
                    |(_progress, metric)| {
                        thread::scope(|scope| {
                            for _ in 0..workers {
                                let metric = metric.clone();
                                scope.spawn(move || {
                                    for _ in 0..UPDATES_PER_WORKER {
                                        metric.complete(1).expect("metric update must succeed");
                                    }
                                });
                            }
                        });
                        black_box(metric.snapshot());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Counts completed work for the mutex contention baseline.
#[derive(Debug)]
struct MutexMetricCounts {
    /// Configured total used by the same conservation check as `MetricCounts`.
    total: u64,
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

impl MutexMetricCounts {
    /// Creates zeroed counts and applies the initial `start(total)` transition.
    fn new(total: u64) -> Self {
        let mut counts = Self {
            total,
            active: 0,
            completed_unclassified: 0,
            succeeded: 0,
            failed: 0,
            cancelled: 0,
        };
        counts.start(total);
        counts
    }

    /// Applies the same active-work increment used by `MetricHandle::start`.
    fn start(&mut self, count: u64) {
        self.active = self
            .active
            .checked_add(count)
            .expect("mutex metric active count must not overflow");
        self.validate();
    }

    /// Applies the same active-to-completed transition used by
    /// `MetricHandle::complete`.
    fn complete(&mut self, count: u64) {
        self.active = self
            .active
            .checked_sub(count)
            .expect("mutex metric active count must not underflow");
        self.completed_unclassified = self
            .completed_unclassified
            .checked_add(count)
            .expect("mutex metric completed count must not overflow");
        self.validate();
    }

    /// Checks conservation and the configured total after each transition.
    fn validate(&self) {
        let completed = self
            .completed_unclassified
            .checked_add(self.succeeded)
            .and_then(|value| value.checked_add(self.failed))
            .and_then(|value| value.checked_add(self.cancelled))
            .expect("mutex metric completed count must not overflow");
        let occupied = completed
            .checked_add(self.active)
            .expect("mutex metric occupied count must not overflow");
        assert!(occupied <= self.total, "mutex metric total must not be exceeded");
    }

    /// Returns all counters in the same shape as a metric snapshot.
    fn snapshot(&self) -> (u64, u64, u64, u64, u64) {
        (
            self.active,
            self.completed_unclassified,
            self.succeeded,
            self.failed,
            self.cancelled,
        )
    }
}

/// Benchmarks a mutex-protected update path with setup removed from timing.
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
                bencher.iter_batched(
                    || Arc::new(Mutex::new(MutexMetricCounts::new(total))),
                    |state| {
                        thread::scope(|scope| {
                            for _ in 0..workers {
                                let state = Arc::clone(&state);
                                scope.spawn(move || {
                                    for _ in 0..UPDATES_PER_WORKER {
                                        let mut state = state.lock().expect("mutex must not poison");
                                        state.complete(1);
                                    }
                                });
                            }
                        });
                        let state = state.lock().expect("mutex must not poison");
                        black_box(state.snapshot());
                    },
                    BatchSize::SmallInput,
                );
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
    bench_enabled_auto_reporter_spawn_stop,
    bench_auto_reporter_worker_fan_in,
    bench_metric_handle_contention,
    bench_mutex_metric_contention
);
criterion_main!(progress_benches);
