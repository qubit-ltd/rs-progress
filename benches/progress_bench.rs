// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Benchmarks for progress event construction and reporting hot paths.

use std::time::Duration;

use criterion::{
    Criterion,
    criterion_group,
    criterion_main,
};
use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounter,
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};
use std::hint::black_box;

/// Benchmarks a due-check that must avoid constructing counters and events.
fn benchmark_running_report_due_check(c: &mut Criterion) {
    let reporter = NoOpProgressReporter;
    let mut progress = Progress::single_metric(
        &reporter,
        Duration::from_secs(60),
        "entries",
        "Entries",
    );

    c.bench_function("running_report_due_check_not_due", |b| {
        b.iter(|| {
            black_box(
                progress
                    .report_running_if_due(|event| {
                        event.counter("entries", |counter| counter.total(1))
                    })
                    .expect("no-op reporter should not fail"),
            );
        });
    });
}

/// Benchmarks construction of a validated single-metric event.
fn benchmark_single_metric_event_build(c: &mut Criterion) {
    c.bench_function("single_metric_event_build", |b| {
        b.iter(|| {
            black_box(ProgressEvent::running(
                ProgressSchema::single("entries", "Entries"),
                vec![ProgressCounter::new("entries").total(10).completed(5)],
                Duration::from_millis(1),
            ));
        });
    });
}

/// Benchmarks lazy snapshot conversion for a representative multi-metric event.
fn benchmark_multi_metric_snapshots(c: &mut Criterion) {
    let event = ProgressEvent::running(
        ProgressSchema::new(vec![
            ProgressMetric::new("entries", "Entries"),
            ProgressMetric::new("bytes", "Bytes"),
            ProgressMetric::new("errors", "Errors"),
        ]),
        vec![
            ProgressCounter::new("entries").total(10).completed(5),
            ProgressCounter::new("bytes").total(1024).completed(512),
            ProgressCounter::new("errors").completed(1).failed(1),
        ],
        Duration::from_millis(1),
    );

    c.bench_function("multi_metric_snapshot_collection", |b| {
        b.iter(|| black_box(event.metric_snapshots()));
    });
}

criterion_group!(
    benches,
    benchmark_running_report_due_check,
    benchmark_single_metric_event_build,
    benchmark_multi_metric_snapshots,
);
criterion_main!(benches);
