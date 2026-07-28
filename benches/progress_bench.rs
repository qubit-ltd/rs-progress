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
    time::Duration,
};

use criterion::{
    Criterion,
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
    criterion.bench_function("disabled_report", |bencher| {
        bencher.iter(|| {
            progress
                .report(|snapshot| {
                    snapshot.metric("entries", |counts| {
                        counts.completed(1);
                    });
                })
                .expect("disabled report must succeed");
        });
    });
}

/// Benchmarks an enabled running report with one fully configured metric.
fn bench_enabled_report(criterion: &mut Criterion) {
    let reporter = |_event: &qubit_progress::Event| Ok::<(), ReportError>(());
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    criterion.bench_function("enabled_report", |bencher| {
        bencher.iter(|| {
            progress
                .report(|snapshot| {
                    snapshot.metric("entries", |counts| {
                        counts.completed(black_box(1));
                    });
                })
                .expect("enabled report must succeed");
        });
    });
}

/// Benchmarks the positive-interval path that skips an undued report closure.
fn bench_not_due_report(criterion: &mut Criterion) {
    let reporter = |_event: &qubit_progress::Event| Ok::<(), ReportError>(());
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_secs(60))
        .metric(Metric::new("entries", "Entries").total(1))
        .start()
        .expect("enabled progress must start");
    criterion.bench_function("not_due_report", |bencher| {
        bencher.iter(|| {
            progress
                .report_if_due(|snapshot| {
                    snapshot.metric("entries", |counts| {
                        counts.completed(1);
                    });
                })
                .expect("undued report must succeed");
        });
    });
}

/// Benchmarks creation and terminal delivery of a complete multi-metric event.
fn bench_multi_metric_terminal(criterion: &mut Criterion) {
    let reporter = |_event: &qubit_progress::Event| Ok::<(), ReportError>(());
    criterion.bench_function("multi_metric_terminal", |bencher| {
        bencher.iter(|| {
            Progress::builder(&reporter)
                .metric(Metric::new("entries", "Entries").total(1))
                .metric(Metric::new("bytes", "Bytes").total(1024))
                .start()
                .expect("progress must start")
                .finish(|snapshot| {
                    snapshot
                        .metric("entries", |counts| {
                            counts.completed(1).succeeded(1);
                        })
                        .metric("bytes", |counts| {
                            counts.completed(1024);
                        });
                })
                .expect("terminal event must report");
        });
    });
}

criterion_group!(
    progress_benches,
    bench_disabled_report,
    bench_enabled_report,
    bench_not_due_report,
    bench_multi_metric_terminal
);
criterion_main!(progress_benches);
