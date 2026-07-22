// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `MetricSnapshotFormatter` trait dispatch.

use std::time::Duration;

use qubit_progress::{
    HumanReadableMetricSnapshotFormatter,
    MetricSnapshotFormatter,
    ProgressCounter,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
};

#[test]
fn test_metric_snapshot_formatter_supports_trait_objects() {
    let formatter: &dyn MetricSnapshotFormatter =
        &HumanReadableMetricSnapshotFormatter::new();
    let snapshot = ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("entries").total(2).completed(1),
        Duration::ZERO,
    );

    assert!(formatter.format(&snapshot).contains("Entries 1/2"));
}
