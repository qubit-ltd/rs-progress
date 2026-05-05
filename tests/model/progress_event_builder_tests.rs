/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `ProgressEventBuilder`.

use std::time::Duration;

use qubit_progress::model::{
    ProgressCounters,
    ProgressEventBuilder,
    ProgressPhase,
    ProgressStage,
};

#[test]
fn test_progress_event_builder_uses_running_unknown_total_defaults() {
    let event = ProgressEventBuilder::new().build();

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), None);
    assert_eq!(event.counters(), ProgressCounters::new(None));
    assert_eq!(event.elapsed(), Duration::ZERO);
}

#[test]
fn test_progress_event_builder_default_matches_new() {
    assert_eq!(ProgressEventBuilder::default(), ProgressEventBuilder::new());
}

#[test]
fn test_progress_event_builder_configures_counts_stage_and_elapsed() {
    let stage = ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(4)
        .with_weight(0.5);
    let event = ProgressEventBuilder::new()
        .started()
        .running()
        .total(10)
        .completed(4)
        .active(1)
        .succeeded(3)
        .failed_count(1)
        .stage(stage.clone())
        .elapsed(Duration::from_secs(5))
        .build();

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), Some(&stage));
    assert_eq!(event.counters().total_count(), Some(10));
    assert_eq!(event.counters().completed_count(), 4);
    assert_eq!(event.counters().active_count(), 1);
    assert_eq!(event.counters().succeeded_count(), 3);
    assert_eq!(event.counters().failed_count(), 1);
    assert_eq!(event.elapsed(), Duration::from_secs(5));
}

#[test]
fn test_progress_event_builder_accepts_prebuilt_counters_and_named_stage() {
    let counters = ProgressCounters::new(Some(8))
        .with_completed_count(6)
        .with_failed_count(2);
    let event = ProgressEventBuilder::new()
        .finished()
        .counters(counters)
        .unknown_total()
        .stage_named("verify", "Verify installation")
        .build();

    assert_eq!(event.phase(), ProgressPhase::Finished);
    assert_eq!(event.counters().total_count(), None);
    assert_eq!(event.counters().completed_count(), 6);
    assert_eq!(event.counters().failed_count(), 2);

    let stage = event.stage().expect("builder should configure named stage");
    assert_eq!(stage.id(), "verify");
    assert_eq!(stage.name(), "Verify installation");
}

#[test]
fn test_progress_event_builder_phase_helpers_initialize_expected_phase() {
    assert_eq!(
        ProgressEventBuilder::new().started().build().phase(),
        ProgressPhase::Started
    );
    assert_eq!(
        ProgressEventBuilder::new().running().build().phase(),
        ProgressPhase::Running
    );
    assert_eq!(
        ProgressEventBuilder::new().finished().build().phase(),
        ProgressPhase::Finished
    );
    assert_eq!(
        ProgressEventBuilder::new().failed().build().phase(),
        ProgressPhase::Failed
    );
    assert_eq!(
        ProgressEventBuilder::new().canceled().build().phase(),
        ProgressPhase::Canceled
    );
    assert_eq!(
        ProgressEventBuilder::new()
            .phase(ProgressPhase::Canceled)
            .build()
            .phase(),
        ProgressPhase::Canceled
    );
}
