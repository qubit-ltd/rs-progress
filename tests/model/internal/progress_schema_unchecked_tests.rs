// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for unchecked schema input through `ProgressSchema` deserialization.

use qubit_progress::ProgressSchema;

#[test]
fn test_progress_schema_unchecked_accepts_unique_metrics() {
    let json = r#"{"metrics":[{"id":"entries","name":"Entries"}]}"#;

    let schema = serde_json::from_str::<ProgressSchema>(json)
        .expect("unique metric ids should deserialize");
    assert_eq!(schema.metric_name("entries"), Some("Entries"));
}
