// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressEvent`.

use std::time::Duration;

use qubit_progress::model::{
    ProgressCounter,
    ProgressEvent,
    ProgressMetric,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};

fn schema() -> ProgressSchema {
    ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")])
}

fn counters() -> Vec<ProgressCounter> {
    vec![ProgressCounter::new("entries").total(8).completed(2)]
}

#[test]
fn test_progress_event_carries_schema_phase_stage_counters_and_timing() {
    let stage = ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(4)
        .with_weight(0.5);
    let event =
        ProgressEvent::running(schema(), counters(), Duration::from_secs(3))
            .with_stage(stage.clone());

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), Some(&stage));
    assert_eq!(event.schema().metric_name("entries"), Some("Entries"));
    assert_eq!(
        event
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(2)
    );
    assert_eq!(event.elapsed(), Duration::from_secs(3));
}

#[test]
fn test_progress_event_constructors_cover_terminal_phases() {
    let started = ProgressEvent::started(schema(), counters(), Duration::ZERO);
    let finished =
        ProgressEvent::finished(schema(), counters(), Duration::from_secs(1));
    let failed =
        ProgressEvent::failed(schema(), counters(), Duration::from_secs(2));
    let canceled =
        ProgressEvent::canceled(schema(), counters(), Duration::from_secs(3));

    assert_eq!(started.phase(), ProgressPhase::Started);
    assert_eq!(started.stage(), None);
    assert_eq!(
        started
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(2)
    );
    assert_eq!(finished.phase(), ProgressPhase::Finished);
    assert_eq!(failed.phase(), ProgressPhase::Failed);
    assert_eq!(canceled.phase(), ProgressPhase::Canceled);
}

#[test]
fn test_progress_event_from_phase_creates_matching_event() {
    let elapsed = Duration::from_millis(125);

    for phase in [
        ProgressPhase::Started,
        ProgressPhase::Running,
        ProgressPhase::Finished,
        ProgressPhase::Failed,
        ProgressPhase::Canceled,
    ] {
        let event =
            ProgressEvent::from_phase(schema(), phase, counters(), elapsed);

        assert_eq!(event.phase(), phase);
        assert_eq!(event.stage(), None);
        assert_eq!(
            event
                .counter("entries")
                .map(ProgressCounter::completed_count),
            Some(2)
        );
        assert_eq!(event.elapsed(), elapsed);
    }
}

#[test]
fn test_progress_event_builder_and_new_constructor() {
    let built = ProgressEvent::builder(schema())
        .phase(ProgressPhase::Finished)
        .counter("entries", |counter| counter.total(2).completed(2))
        .elapsed(Duration::from_millis(250))
        .build();
    assert_eq!(built.phase(), ProgressPhase::Finished);
    assert_eq!(
        built.counter("entries").map(ProgressCounter::total_count),
        Some(Some(2))
    );
    assert_eq!(
        built
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(2)
    );
    assert_eq!(built.elapsed(), Duration::from_millis(250));

    let rebuilt = ProgressEvent::new(
        ProgressEvent::builder(schema())
            .phase(ProgressPhase::Failed)
            .counter("entries", |counter| counter.completed(1).failed(1)),
    );
    assert_eq!(rebuilt.phase(), ProgressPhase::Failed);
    assert_eq!(
        rebuilt
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(1)
    );
    assert_eq!(
        rebuilt
            .counter("entries")
            .map(ProgressCounter::failed_count),
        Some(1)
    );
}

#[test]
fn test_progress_event_serializes_to_self_describing_json() {
    let event = ProgressEvent::builder(schema())
        .running()
        .counter("entries", |counter| counter.total(5).completed(2))
        .elapsed(Duration::from_millis(110))
        .build();

    let json = serde_json::to_string(&event).expect("event should serialize");
    assert!(json.contains("\"schema\""));
    assert!(json.contains("\"metric_id\":\"entries\""));
    assert!(json.contains("\"elapsed\":\"110ms\""));

    let decoded: ProgressEvent =
        serde_json::from_str(&json).expect("event should deserialize");
    assert_eq!(decoded.elapsed(), Duration::from_millis(110));
    assert_eq!(
        decoded
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(2)
    );
}

#[test]
fn test_progress_types_are_reexported_from_crate_root() {
    let event: qubit_progress::ProgressEvent = ProgressEvent::running(
        qubit_progress::ProgressSchema::single("entries", "Entries"),
        vec![qubit_progress::ProgressCounter::new("entries")],
        Duration::ZERO,
    );
    let phase: qubit_progress::ProgressPhase = event.phase();

    assert_eq!(phase, ProgressPhase::Running);
}
