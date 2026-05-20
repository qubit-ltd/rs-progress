/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `HumanReadableMetricSnapshotFormatter`.

use std::time::Duration;

use qubit_progress::{
    HumanReadableMetricSnapshotFormatter,
    MetricSnapshotFormatter,
    ProgressCounter,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressStage,
};

#[test]
fn test_human_readable_metric_snapshot_formatter_formats_known_total() {
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Running,
        Some(ProgressStage::new("copy", "Copy files")),
        &ProgressCounter::new("entries")
            .total(4)
            .completed(2)
            .active(1)
            .succeeded(1),
        Duration::from_millis(1_500),
    );

    let text = HumanReadableMetricSnapshotFormatter::new().format(&snapshot);

    assert!(text.contains("running [Copy files] Entries 2/4 (50.00%)"));
    assert!(text.contains("active 1"));
    assert!(text.contains("succeeded 1"));
    assert!(text.contains("failed 0"));
    assert!(text.contains("elapsed 1.500s"));
}

#[test]
fn test_human_readable_metric_snapshot_formatter_formats_unknown_total() {
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("records", "Records"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("records").completed(7),
        Duration::from_millis(0),
    );

    let text = HumanReadableMetricSnapshotFormatter::new().format(&snapshot);

    assert!(text.contains("running Records 7 completed"));
    assert!(text.contains("elapsed 0ms"));
}
