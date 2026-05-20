/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `JsonMetricSnapshotFormatter`.

use std::time::Duration;

use qubit_progress::{
    JsonMetricSnapshotFormatter,
    MetricSnapshotFormatter,
    ProgressCounter,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
};

#[test]
fn test_json_metric_snapshot_formatter_formats_compact_json() {
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("entries").total(5).completed(2),
        Duration::from_millis(110),
    );

    let json = JsonMetricSnapshotFormatter::new().format(&snapshot);
    let value: serde_json::Value = serde_json::from_str(&json).expect("JSON should parse");

    assert_eq!(value["metric"]["id"], "entries");
    assert_eq!(value["metric"]["name"], "Entries");
    assert_eq!(value["phase"], "running");
    assert_eq!(value["total_count"], 5);
    assert_eq!(value["completed_count"], 2);
    assert_eq!(value["elapsed"], "110ms");
}
