// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stage metadata behavior.

use qubit_progress::Stage;

/// Verifies that stage builders retain their supplied optional position.
#[test]
fn test_stage_retains_position_metadata() {
    let stage = Stage::new("copy", "Copy").position(2, 3);
    assert_eq!(stage.id(), "copy");
    assert_eq!(stage.name(), "Copy");
    assert_eq!(stage.position_value(), Some(2));
    assert_eq!(stage.total(), Some(3));
}
