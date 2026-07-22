// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for unchecked stage input through `ProgressStage` deserialization.

use qubit_progress::ProgressStage;

#[test]
fn test_progress_stage_unchecked_accepts_non_negative_weight() {
    let json = r#"{"id":"copy","name":"Copy files","weight":0.5}"#;

    let stage = serde_json::from_str::<ProgressStage>(json)
        .expect("finite non-negative weight should deserialize");
    assert_eq!(stage.weight(), Some(0.5));
}
