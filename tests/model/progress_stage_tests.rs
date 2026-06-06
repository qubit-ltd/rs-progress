// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressStage`.

use qubit_progress::model::ProgressStage;

#[test]
fn test_progress_stage_accessors_return_configured_values() {
    let default_stage = ProgressStage::new("prepare", "Prepare");
    assert_eq!(default_stage.id(), "prepare");
    assert_eq!(default_stage.name(), "Prepare");
    assert_eq!(default_stage.index(), None);
    assert_eq!(default_stage.total_stages(), None);
    assert_eq!(default_stage.weight(), None);

    let configured_stage = default_stage
        .clone()
        .with_index(0)
        .with_total_stages(3)
        .with_weight(2.5);
    assert_eq!(configured_stage.id(), "prepare");
    assert_eq!(configured_stage.name(), "Prepare");
    assert_eq!(configured_stage.index(), Some(0));
    assert_eq!(configured_stage.total_stages(), Some(3));
    assert_eq!(configured_stage.weight(), Some(2.5));
}

#[test]
fn test_progress_stage_weight_records_supplied_value() {
    let stage = ProgressStage::new("copy", "Copy files").with_weight(f64::NAN);

    assert!(
        stage
            .weight()
            .expect("stage should carry supplied weight")
            .is_nan()
    );
}
