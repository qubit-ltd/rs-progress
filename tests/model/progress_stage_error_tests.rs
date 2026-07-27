// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressStageError`.

use qubit_progress::ProgressStageError;

#[test]
fn test_progress_stage_error_displays_weight_constraint() {
    assert_eq!(
        ProgressStageError::NegativeWeight.to_string(),
        "progress stage weight must be non-negative",
    );
}

#[test]
fn test_progress_stage_error_displays_non_finite_constraint() {
    assert_eq!(
        ProgressStageError::NonFiniteWeight.to_string(),
        "progress stage weight must be finite",
    );
}
