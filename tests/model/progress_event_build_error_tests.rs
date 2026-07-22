// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressEventBuildError`.

use qubit_progress::ProgressEventBuildError;

#[test]
fn test_progress_event_build_error_displays_metric_context() {
    let error = ProgressEventBuildError::UnknownMetricId {
        metric_id: "missing".to_owned(),
    };

    assert_eq!(error.to_string(), "unknown progress metric id: missing");
}
