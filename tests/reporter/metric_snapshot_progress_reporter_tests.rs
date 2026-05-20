/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `MetricSnapshotProgressReporter`.

use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_function::{
    ArcConsumer,
    Consumer,
};
use qubit_progress::{
    MetricSnapshotProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressReporter,
    ProgressSchema,
};

#[test]
fn test_metric_snapshot_progress_reporter_consumes_snapshot_objects() {
    let snapshots = Arc::new(Mutex::new(Vec::<ProgressMetricSnapshot>::new()));
    let captured_snapshots = Arc::clone(&snapshots);
    let consumer = ArcConsumer::new(move |snapshot: &ProgressMetricSnapshot| {
        captured_snapshots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(snapshot.clone());
    });
    let reporter = MetricSnapshotProgressReporter::new(consumer);
    reporter.consumer().accept(&ProgressMetricSnapshot::new(
        ProgressMetric::new("manual", "Manual"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("manual").completed(1),
        Duration::ZERO,
    ));

    reporter.report(&ProgressEvent::running(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(4).completed(2)],
        Duration::from_millis(10),
    ));

    let snapshots = snapshots
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].metric_id(), "manual");
    assert_eq!(snapshots[1].metric_id(), "entries");
    assert_eq!(snapshots[1].completed_count(), 2);
}
