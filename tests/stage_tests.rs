// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stage metadata behavior.

use qubit_progress::Stage;

#[cfg(feature = "serde")]
use serde_json::json;

/// Verifies that stage builders retain their supplied optional position.
#[test]
fn test_stage_retains_position_metadata() {
    let stage = Stage::new("copy", "Copy").position(2, 3);
    assert_eq!(stage.id(), "copy");
    assert_eq!(stage.name(), "Copy");
    assert_eq!(stage.position_value(), Some(2));
    assert_eq!(stage.total(), Some(3));
}

/// Verifies standalone stage deserialization rejects invalid metadata.
#[cfg(feature = "serde")]
#[test]
fn test_stage_deserialization_rejects_invalid_metadata() {
    for value in [
        json!({"id": "", "name": "Copy", "position": null, "total": null}),
        json!({"id": "copy", "name": "", "position": null, "total": null}),
        json!({"id": "copy", "name": "Copy", "position": 1, "total": null}),
        json!({"id": "copy", "name": "Copy", "position": 0, "total": 1}),
        json!({"id": "copy", "name": "Copy", "position": 2, "total": 1}),
    ] {
        assert!(serde_json::from_value::<Stage>(value).is_err());
    }
}
