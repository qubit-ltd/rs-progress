// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressMetricSnapshot`.

use std::time::Duration;

use qubit_progress::{
    ProgressCounter,
    ProgressEvent,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};

#[test]
fn test_progress_metric_snapshot_flattens_metric_counter_and_event_state() {
    let stage = ProgressStage::new("copy", "Copy files");
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Running,
        Some(stage.clone()),
        &ProgressCounter::new("entries")
            .total(10)
            .completed(4)
            .active(2)
            .succeeded(3)
            .failed(1),
        Duration::from_millis(250),
    );

    assert_eq!(snapshot.metric_id(), "entries");
    assert_eq!(snapshot.metric().name(), "Entries");
    assert_eq!(snapshot.metric_name(), "Entries");
    assert_eq!(snapshot.phase(), ProgressPhase::Running);
    assert_eq!(snapshot.stage(), Some(&stage));
    assert_eq!(snapshot.total_count(), Some(10));
    assert_eq!(snapshot.completed_count(), 4);
    assert_eq!(snapshot.active_count(), 2);
    assert_eq!(snapshot.succeeded_count(), 3);
    assert_eq!(snapshot.failed_count(), 1);
    assert_eq!(snapshot.remaining_count(), Some(4));
    assert_eq!(snapshot.progress_fraction(), Some(0.4));
    assert_eq!(snapshot.progress_percent(), Some(40.0));
    assert_eq!(snapshot.elapsed(), Duration::from_millis(250));
}

#[test]
fn test_progress_metric_snapshot_handles_unknown_and_zero_total_progress() {
    let unknown = ProgressMetricSnapshot::new(
        ProgressMetric::new("records", "Records"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("records").completed(3),
        Duration::ZERO,
    );
    assert_eq!(unknown.remaining_count(), None);
    assert_eq!(unknown.progress_fraction(), None);
    assert_eq!(unknown.progress_percent(), None);

    let empty = ProgressMetricSnapshot::new(
        ProgressMetric::new("records", "Records"),
        ProgressPhase::Finished,
        None,
        &ProgressCounter::new("records").total(0),
        Duration::ZERO,
    );
    assert_eq!(empty.remaining_count(), Some(0));
    assert_eq!(empty.progress_fraction(), Some(1.0));
    assert_eq!(empty.progress_percent(), Some(100.0));
}

#[test]
fn test_progress_event_metric_snapshots_resolve_schema_and_fallback_metrics() {
    let event =
        ProgressEvent::builder(ProgressSchema::single("entries", "Entries"))
            .running()
            .stage_named("scan", "Scan files")
            .counter("entries", |counter| counter.total(5).completed(2))
            .counter("missing", |counter| counter.completed(3))
            .elapsed(Duration::from_millis(110))
            .build();

    let snapshots = event.metric_snapshots();

    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].metric_id(), "entries");
    assert_eq!(snapshots[0].metric_name(), "Entries");
    assert_eq!(snapshots[0].completed_count(), 2);
    assert_eq!(
        snapshots[0].stage().map(ProgressStage::name),
        Some("Scan files")
    );
    assert_eq!(snapshots[0].elapsed(), Duration::from_millis(110));
    assert_eq!(snapshots[1].metric_id(), "missing");
    assert_eq!(snapshots[1].metric_name(), "missing");
    assert_eq!(snapshots[1].total_count(), None);
}

#[test]
fn test_progress_metric_snapshot_serializes_elapsed_with_unit() {
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Finished,
        None,
        &ProgressCounter::new("entries").total(1).completed(1),
        Duration::from_millis(110),
    );

    let json =
        serde_json::to_string(&snapshot).expect("snapshot should serialize");

    assert!(
        json.contains("\"metric\":{\"id\":\"entries\",\"name\":\"Entries\"}")
    );
    assert!(json.contains("\"phase\":\"finished\""));
    assert!(json.contains("\"elapsed\":\"110ms\""));
}
