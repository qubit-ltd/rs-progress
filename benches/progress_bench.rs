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
use std::sync::Barrier;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_progress::Event;
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
    let reporter = |event: &Event| {
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
    let reporter = |event: &Event| {
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
    let reporter = |event: &Event| {
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
    let reporter = |event: &Event| {
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
    let reporter = |event: &Event| {
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
    let reporter = |event: &Event| {
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

/// Measures persistent metric update workers with and without concurrent
/// snapshots.
///
/// Reported throughput includes barrier and scheduling costs. It measures the
/// shared metric path, not automatic reporter scheduling or I/O.
fn bench_metric_update_with_sampling(criterion: &mut Criterion) {
    const UPDATES_PER_WORKER: u64 = 1_048_576;
    let reporter = |_event: &Event| Ok::<(), ReporterError>(());
    let mut group = criterion.benchmark_group("metric_update_with_sampling");

    for workers in [1usize, 2, 4, 8, 16] {
        let updates_per_round = u64::try_from(workers)
            .expect("worker count must fit in u64")
            .checked_mul(UPDATES_PER_WORKER)
            .expect("round update count must fit in u64");
        group.throughput(Throughput::Elements(updates_per_round));

        for sampling in [false, true] {
            let name = if sampling { "with_sampler" } else { "without_sampler" };
            group.bench_with_input(BenchmarkId::new(name, workers), &workers, |bencher, &workers| {
                bencher.iter_custom(|rounds| {
                    let rounds = usize::try_from(rounds).expect("Criterion iteration count must fit in usize");
                    let progress = Progress::builder(&reporter)
                        .metric(Metric::new("entries", "Entries"))
                        .start()
                        .expect("enabled progress must start");
                    let metric = progress.metric("entries").expect("configured metric must exist");
                    let participants = workers.checked_add(2).expect("participant count must fit in usize");
                    let start = Arc::new(Barrier::new(participants));
                    let end = Arc::new(Barrier::new(participants));
                    let remaining = Arc::new(AtomicUsize::new(0));
                    let sampled = Arc::new(AtomicU64::new(0));
                    let error = Arc::new(Mutex::new(None::<String>));
                    let mut measured = Duration::ZERO;
                    let mut completed_rounds = 0u64;
                    let mut sampled_rounds = 0u64;

                    thread::scope(|scope| {
                        for _ in 0..workers {
                            let metric = metric.clone();
                            let start = Arc::clone(&start);
                            let end = Arc::clone(&end);
                            let remaining = Arc::clone(&remaining);
                            let error = Arc::clone(&error);
                            scope.spawn(move || {
                                for _ in 0..rounds {
                                    start.wait();
                                    for _ in 0..UPDATES_PER_WORKER {
                                        if let Err(update_error) = metric.complete(1) {
                                            let mut first_error =
                                                error.lock().expect("benchmark error mutex must not poison");
                                            if first_error.is_none() {
                                                *first_error = Some(update_error.to_string());
                                            }
                                        }
                                    }
                                    remaining.fetch_sub(1, Ordering::AcqRel);
                                    end.wait();
                                }
                            });
                        }

                        let sampler_start = Arc::clone(&start);
                        let sampler_end = Arc::clone(&end);
                        let sampler_remaining = Arc::clone(&remaining);
                        let sampler_sampled = Arc::clone(&sampled);
                        let sampler_metric = metric.clone();
                        scope.spawn(move || {
                            for _ in 0..rounds {
                                sampler_start.wait();
                                if sampling {
                                    while sampler_remaining.load(Ordering::Acquire) != 0 {
                                        let snapshot = sampler_metric.snapshot();
                                        if snapshot.active() > 0 {
                                            sampler_sampled.fetch_add(1, Ordering::Relaxed);
                                        }
                                        black_box(snapshot);
                                        thread::yield_now();
                                    }
                                } else {
                                    while sampler_remaining.load(Ordering::Acquire) != 0 {
                                        thread::yield_now();
                                    }
                                }
                                sampler_end.wait();
                            }
                        });

                        for _ in 0..rounds {
                            if let Err(start_error) = metric.start(updates_per_round) {
                                let mut first_error = error.lock().expect("benchmark error mutex must not poison");
                                if first_error.is_none() {
                                    *first_error = Some(start_error.to_string());
                                }
                            }
                            remaining.store(workers, Ordering::Release);
                            sampled.store(0, Ordering::Relaxed);
                            let started = Instant::now();
                            start.wait();
                            end.wait();
                            measured += started.elapsed();

                            let snapshot = metric.snapshot();
                            completed_rounds = match completed_rounds.checked_add(updates_per_round) {
                                Some(completed) => completed,
                                None => {
                                    let mut first_error = error.lock().expect("benchmark error mutex must not poison");
                                    if first_error.is_none() {
                                        *first_error = Some("completed count overflowed u64".to_owned());
                                    }
                                    completed_rounds
                                }
                            };
                            if snapshot.active() != 0 || snapshot.completed() != completed_rounds {
                                let mut first_error = error.lock().expect("benchmark error mutex must not poison");
                                if first_error.is_none() {
                                    *first_error = Some(format!(
                                        "round invariant failed: active={}, completed={}, expected={completed_rounds}",
                                        snapshot.active(),
                                        snapshot.completed()
                                    ));
                                }
                            }
                            if sampling {
                                sampled_rounds = sampled_rounds
                                    .checked_add(sampled.load(Ordering::Relaxed))
                                    .expect("sample count must fit in u64");
                            }
                        }
                    });

                    let final_error = error.lock().expect("benchmark error mutex must not poison").take();
                    assert!(final_error.is_none(), "metric update failed: {final_error:?}");
                    if sampling {
                        assert!(sampled_rounds > 0, "invalid measurement: no running snapshots");
                    }
                    progress.finish().expect("terminal event must report");
                    measured
                });
            });
        }
    }
    group.finish();
}

/// Measures average snapshot time while a persistent worker updates the metric.
///
/// The reported value is the batch average for Criterion's requested snapshots;
/// it does not represent a latency percentile or automatic reporter delay.
fn bench_metric_snapshot_under_updates(criterion: &mut Criterion) {
    // A minimum batch gives the update workers time to run even when Criterion
    // requests only one iteration during smoke tests or warm-up.
    const MIN_SNAPSHOT_SAMPLES: u64 = 1_048_576;
    let reporter = |_event: &Event| Ok::<(), ReporterError>(());
    let mut group = criterion.benchmark_group("metric_snapshot_under_updates");

    for workers in [1usize, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("snapshot_average", workers),
            &workers,
            |bencher, &workers| {
                bencher.iter_custom(|snapshots| {
                    let sample_count = snapshots.max(MIN_SNAPSHOT_SAMPLES);
                    let progress = Progress::builder(&reporter)
                        .metric(Metric::new("entries", "Entries"))
                        .start()
                        .expect("enabled progress must start");
                    let metric = progress.metric("entries").expect("configured metric must exist");
                    metric.start(u64::MAX).expect("metric start must succeed");

                    let participant_count = workers.checked_add(1).expect("participant count must fit in usize");
                    let ready = Arc::new(Barrier::new(participant_count));
                    let started_workers = Arc::new(AtomicUsize::new(0));
                    let stop = Arc::new(AtomicBool::new(false));
                    let error = Arc::new(Mutex::new(None::<String>));
                    let (before, after, measured) = thread::scope(|scope| {
                        for _ in 0..workers {
                            let worker_metric = metric.clone();
                            let worker_ready = Arc::clone(&ready);
                            let worker_started = Arc::clone(&started_workers);
                            let worker_stop = Arc::clone(&stop);
                            let worker_error = Arc::clone(&error);
                            scope.spawn(move || {
                                worker_ready.wait();
                                let first_update = worker_metric.complete(1);
                                if let Err(update_error) = &first_update {
                                    let mut first_error =
                                        worker_error.lock().expect("benchmark error mutex must not poison");
                                    if first_error.is_none() {
                                        *first_error = Some(update_error.to_string());
                                    }
                                }
                                worker_started.fetch_add(1, Ordering::Release);
                                if first_update.is_err() {
                                    return;
                                }
                                while !worker_stop.load(Ordering::Acquire) {
                                    if let Err(update_error) = worker_metric.complete(1) {
                                        let mut first_error =
                                            worker_error.lock().expect("benchmark error mutex must not poison");
                                        if first_error.is_none() {
                                            *first_error = Some(update_error.to_string());
                                        }
                                        break;
                                    }
                                }
                            });
                        }

                        ready.wait();
                        while started_workers.load(Ordering::Acquire) != workers {
                            thread::yield_now();
                        }
                        let before = metric.snapshot().completed();
                        let started = Instant::now();
                        for _ in 0..sample_count {
                            black_box(metric.snapshot());
                        }
                        let measured = started.elapsed();
                        let after = metric.snapshot().completed();
                        stop.store(true, Ordering::Release);
                        (before, after, measured)
                    });

                    let final_error = error.lock().expect("benchmark error mutex must not poison").take();
                    progress.cancel().expect("active work must be cancelled cleanly");
                    assert!(final_error.is_none(), "metric update failed: {final_error:?}");
                    assert!(after > before, "invalid measurement: worker made no progress");
                    measured.mul_f64(snapshots as f64 / sample_count as f64)
                });
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
    bench_metric_update_with_sampling,
    bench_metric_snapshot_under_updates,
    bench_mutex_metric_contention
);
criterion_main!(progress_benches);
