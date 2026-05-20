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
    ProgressCounter,
    ProgressEvent,
    ProgressEventBuilder,
    ProgressMetric,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};

fn schema() -> ProgressSchema {
    ProgressSchema::new(vec![
        ProgressMetric::new("entries", "Entries"),
        ProgressMetric::new("bytes", "Bytes"),
    ])
}

#[test]
fn test_progress_event_builder_uses_running_empty_counter_defaults() {
    let schema = schema();
    let event = ProgressEventBuilder::new(schema.clone()).build();

    assert_eq!(event.schema(), &schema);
    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), None);
    assert!(event.counters().is_empty());
    assert_eq!(event.elapsed(), Duration::ZERO);
}

#[test]
fn test_progress_event_builder_configures_counters_stage_and_elapsed() {
    let stage = ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(4)
        .with_weight(0.5);
    let event = ProgressEventBuilder::new(schema())
        .started()
        .running()
        .counter("entries", |counter| {
            counter
                .total(10)
                .completed(4)
                .active(1)
                .succeeded(3)
                .failed(1)
        })
        .counter("bytes", |counter| counter.total(100).completed(40))
        .stage(stage.clone())
        .elapsed(Duration::from_secs(5))
        .build();

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), Some(&stage));
    assert_eq!(
        event.counter("entries").map(ProgressCounter::total_count),
        Some(Some(10))
    );
    assert_eq!(
        event
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(4)
    );
    assert_eq!(
        event.counter("entries").map(ProgressCounter::active_count),
        Some(1)
    );
    assert_eq!(
        event
            .counter("entries")
            .map(ProgressCounter::succeeded_count),
        Some(3)
    );
    assert_eq!(
        event.counter("entries").map(ProgressCounter::failed_count),
        Some(1)
    );
    assert_eq!(
        event.counter("bytes").map(ProgressCounter::completed_count),
        Some(40)
    );
    assert_eq!(event.elapsed(), Duration::from_secs(5));
}

#[test]
fn test_progress_event_builder_accepts_prebuilt_counters_and_named_stage() {
    let counter = ProgressCounter::new("entries")
        .total(8)
        .completed(6)
        .failed(2);
    let event = ProgressEventBuilder::new(schema())
        .finished()
        .add_counter(counter.clone())
        .counters(vec![counter.unknown_total()])
        .stage_named("verify", "Verify installation")
        .build();

    assert_eq!(event.phase(), ProgressPhase::Finished);
    assert_eq!(
        event.counter("entries").map(ProgressCounter::total_count),
        Some(None)
    );
    assert_eq!(
        event
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(6)
    );
    assert_eq!(
        event.counter("entries").map(ProgressCounter::failed_count),
        Some(2)
    );

    let stage = event.stage().expect("builder should configure named stage");
    assert_eq!(stage.id(), "verify");
    assert_eq!(stage.name(), "Verify installation");
}

#[test]
fn test_progress_event_builder_phase_helpers_initialize_expected_phase() {
    assert_eq!(
        ProgressEventBuilder::new(schema())
            .started()
            .build()
            .phase(),
        ProgressPhase::Started
    );
    assert_eq!(
        ProgressEventBuilder::new(schema())
            .running()
            .build()
            .phase(),
        ProgressPhase::Running
    );
    assert_eq!(
        ProgressEventBuilder::new(schema())
            .finished()
            .build()
            .phase(),
        ProgressPhase::Finished
    );
    assert_eq!(
        ProgressEventBuilder::new(schema()).failed().build().phase(),
        ProgressPhase::Failed
    );
    assert_eq!(
        ProgressEventBuilder::new(schema())
            .canceled()
            .build()
            .phase(),
        ProgressPhase::Canceled
    );
    assert_eq!(
        ProgressEventBuilder::new(schema())
            .phase(ProgressPhase::Canceled)
            .build()
            .phase(),
        ProgressPhase::Canceled
    );
}

#[test]
fn test_progress_event_builder_is_created_from_event_type() {
    let event = ProgressEvent::builder(schema())
        .counter("entries", |counter| counter.completed(1))
        .build();

    assert_eq!(
        event
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(1)
    );
}
