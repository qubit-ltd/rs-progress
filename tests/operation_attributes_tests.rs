// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Direct behavioral coverage for immutable operation attributes.

use qubit_progress::OperationAttributes;

#[test]
fn test_operation_attributes_insert_replace_and_iterate_in_key_order() {
    let mut attributes = OperationAttributes::new();
    assert!(attributes.is_empty());
    attributes.insert("trace_id", "trace-1");
    attributes.insert("tenant", "acme");
    attributes.insert("trace_id", "trace-2");

    assert_eq!(attributes.get("trace_id"), Some("trace-2"));
    assert_eq!(attributes.get("missing"), None);
    assert_eq!(
        attributes.iter().collect::<Vec<_>>(),
        [("tenant", "acme"), ("trace_id", "trace-2")]
    );
    assert!(!attributes.is_empty());
}
