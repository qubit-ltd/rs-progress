// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressMetricSnapshotError`.

use qubit_progress::ProgressMetricSnapshotError;

#[test]
fn test_progress_metric_snapshot_error_displays_both_ids() {
    let error = ProgressMetricSnapshotError::MetricIdMismatch {
        metric_id: "entries".to_owned(),
        counter_metric_id: "bytes".to_owned(),
    };

    assert!(error.to_string().contains("entries"));
    assert!(error.to_string().contains("bytes"));
}
