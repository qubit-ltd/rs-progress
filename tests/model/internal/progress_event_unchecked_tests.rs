// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for unchecked event input through `ProgressEvent` deserialization.

use qubit_progress::ProgressEvent;

#[test]
fn test_progress_event_unchecked_rejects_duplicate_counters() {
    let json = r#"{"schema":{"metrics":[{"id":"entries","name":"Entries"}]},"phase":"running","counters":[{"metric_id":"entries","total_count":null,"completed_count":1,"active_count":0,"succeeded_count":0,"failed_count":0},{"metric_id":"entries","total_count":null,"completed_count":2,"active_count":0,"succeeded_count":0,"failed_count":0}],"elapsed":"0ns"}"#;

    assert!(serde_json::from_str::<ProgressEvent>(json).is_err());
}
