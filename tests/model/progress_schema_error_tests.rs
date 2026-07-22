// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressSchemaError`.

use qubit_progress::ProgressSchemaError;

#[test]
fn test_progress_schema_error_displays_duplicate_id() {
    let error = ProgressSchemaError::DuplicateMetricId {
        metric_id: "entries".to_owned(),
    };

    assert_eq!(error.to_string(), "duplicate progress metric id: entries");
}
