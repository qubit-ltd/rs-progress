// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressSchema`.

use qubit_progress::{
    ProgressCounter,
    ProgressMetric,
    ProgressSchema,
};

#[test]
fn test_progress_schema_resolves_metrics() {
    let schema = ProgressSchema::new(vec![
        ProgressMetric::new("entries", "Entries"),
        ProgressMetric::new("bytes", "Bytes"),
    ]);

    assert_eq!(schema.metrics().len(), 2);
    assert_eq!(schema.metric_name("bytes"), Some("Bytes"));
    assert!(schema.contains_metric("entries"));
    assert!(!schema.contains_metric("missing"));
}

#[test]
fn test_progress_schema_validates_counter_metric_ids_and_duplicates() {
    let schema = ProgressSchema::single("entries", "Entries");
    let valid = ProgressCounter::new("entries").total(2).completed(1);
    let invalid = ProgressCounter::new("bytes").total(10);

    assert!(schema.validate_counter(&valid));
    assert!(!schema.validate_counter(&invalid));
    assert!(schema.validate_counters(std::slice::from_ref(&valid)));
    assert!(!schema.validate_counters(&[valid.clone(), valid]));
    assert!(!schema.validate_counters(&[invalid]));
}

#[test]
fn test_progress_schema_default_is_empty() {
    let schema = ProgressSchema::default();

    assert!(schema.metrics().is_empty());
    assert_eq!(schema.metric_name("entries"), None);
    assert!(schema.validate_counters(&[]));
}
