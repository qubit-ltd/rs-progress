/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `ProgressEvent`.

use std::time::Duration;

use qubit_progress::model::{
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressStage,
};

#[test]
fn test_progress_event_carries_phase_stage_counters_and_timing() {
    let stage = ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(4)
        .with_weight(0.5);
    let counters = ProgressCounters::new(Some(8)).with_completed_count(2);
    let event = ProgressEvent::running(counters, Duration::from_secs(3)).with_stage(stage.clone());

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), Some(&stage));
    assert_eq!(event.counters(), counters);
    assert_eq!(event.elapsed(), Duration::from_secs(3));
}

#[test]
fn test_progress_event_constructors_cover_terminal_phases() {
    let counters = ProgressCounters::new(None).with_completed_count(5);
    let started = ProgressEvent::started(counters, Duration::ZERO);
    let finished = ProgressEvent::finished(counters, Duration::from_secs(1));
    let failed = ProgressEvent::failed(counters, Duration::from_secs(2));
    let canceled = ProgressEvent::canceled(counters, Duration::from_secs(3));

    assert_eq!(started.phase(), ProgressPhase::Started);
    assert_eq!(started.stage(), None);
    assert_eq!(started.counters(), counters);
    assert_eq!(finished.phase(), ProgressPhase::Finished);
    assert_eq!(failed.phase(), ProgressPhase::Failed);
    assert_eq!(canceled.phase(), ProgressPhase::Canceled);
}

#[test]
fn test_progress_event_builder_and_new_constructor() {
    let built = ProgressEvent::builder()
        .phase(ProgressPhase::Finished)
        .total(2)
        .completed(2)
        .elapsed(Duration::from_millis(250))
        .build();
    assert_eq!(built.phase(), ProgressPhase::Finished);
    assert_eq!(built.counters().total_count(), Some(2));
    assert_eq!(built.counters().completed_count(), 2);
    assert_eq!(built.elapsed(), Duration::from_millis(250));

    let rebuilt = ProgressEvent::new(
        ProgressEvent::builder()
            .phase(ProgressPhase::Failed)
            .completed(1)
            .failed_count(1),
    );
    assert_eq!(rebuilt.phase(), ProgressPhase::Failed);
    assert_eq!(rebuilt.counters().completed_count(), 1);
    assert_eq!(rebuilt.counters().failed_count(), 1);
}
