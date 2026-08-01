// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Event behavior for internally owned metric snapshots.

use std::sync::Mutex;

use qubit_progress::{Event, Metric, Phase, Progress, ReportError, Reporter};

#[cfg(feature = "serde")]
use qubit_progress::MetricSnapshot;

#[cfg(feature = "serde")]
use serde_json::json;

/// Stores delivered events for one test operation.
#[derive(Default)]
struct RecordingReporter {
    /// Events captured from the reporter callback.
    events: Mutex<Vec<Event>>,
}

impl Reporter for RecordingReporter {
    /// Stores each complete immutable event.
    fn report(&self, event: &Event) -> Result<(), ReportError> {
        self.events
            .lock()
            .expect("recording reporter mutex must not be poisoned")
            .push(event.clone());
        Ok(())
    }
}

/// Verifies that events expose the cancelled terminal count from metric state.
#[test]
fn test_event_carries_cancelled_metric_count() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(2).expect("work must start");
    tasks.cancel(2).expect("work must cancel");
    progress.finish().expect("progress must finish");

    let events = reporter
        .events
        .lock()
        .expect("recording reporter mutex must not be poisoned");
    let terminal = events.last().expect("terminal event must exist");
    assert_eq!(terminal.phase(), Phase::Succeeded);
    assert_eq!(
        terminal
            .metric("tasks")
            .expect("metric must exist")
            .cancelled(),
        2,
    );
}

/// Verifies all event accessors and phase names through delivered events.
#[test]
fn test_event_accessors_and_phase_names() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    progress.report().expect("running event must report");
    progress.fail().expect("failed event must report");

    let events = reporter
        .events
        .lock()
        .expect("recording reporter mutex must not be poisoned");
    assert_eq!(events[0].phase().as_str(), "started");
    assert_eq!(events[1].phase().as_str(), "running");
    assert_eq!(events[2].phase().as_str(), "failed");
    assert!(events[0].operation_id() > 0);
    assert_eq!(events[0].sequence(), 0);
    assert!(events[0].stage().is_none());
    assert_eq!(events[0].metrics().len(), 1);
    assert!(events[0].metric("unknown").is_none());
    assert!(events[2].elapsed() >= events[0].elapsed());
    assert_eq!(Phase::Succeeded.as_str(), "succeeded");
    assert_eq!(Phase::Cancelled.as_str(), "cancelled");
}

/// Verifies JSON events accept every canonical duration unit and phase shape.
#[cfg(feature = "serde")]
#[test]
fn test_event_json_deserializes_canonical_durations() {
    for (phase, sequence, elapsed) in [
        ("started", 0, "0ns"),
        ("running", 1, "1ns"),
        ("succeeded", 1, "1us"),
        ("failed", 1, "1ms"),
        ("cancelled", 1, "1s"),
        ("running", 1, "1m"),
        ("running", 1, "1h"),
    ] {
        let value = json!({
            "operation_id": 1,
            "sequence": sequence,
            "phase": phase,
            "stage": { "id": "copy", "name": "Copy", "position": 1, "total": 1 },
            "metrics": [{
                "id": "tasks", "name": "Tasks", "total": 2,
                "completed": 0, "active": 0, "succeeded": 0, "failed": 0, "cancelled": 0
            }],
            "elapsed": elapsed,
        });
        let event: Event = serde_json::from_value(value).expect("event JSON must deserialize");
        assert_eq!(event.phase().as_str(), phase);
        assert_eq!(event.sequence(), sequence);
        assert_eq!(event.stage().expect("stage must exist").total(), Some(1));
        assert_eq!(
            serde_json::to_value(&event)
                .expect("event JSON must serialize")
                .pointer("/elapsed")
                .expect("elapsed must serialize"),
            if elapsed == "0ns" { "0s" } else { elapsed },
        );
    }
}

/// Verifies JSON validation rejects malformed durations and event invariants.
#[cfg(feature = "serde")]
#[test]
fn test_event_json_rejects_invalid_invariants() {
    let valid = json!({
        "operation_id": 1,
        "sequence": 0,
        "phase": "started",
        "stage": null,
        "metrics": [{
            "id": "tasks", "name": "Tasks", "total": 1,
            "completed": 0, "active": 0, "succeeded": 0, "failed": 0, "cancelled": 0
        }],
        "elapsed": "0ns",
    });
    for (pointer, replacement) in [
        ("/operation_id", json!(0)),
        ("/elapsed", json!("1xs")),
        ("/elapsed", json!("-1s")),
        ("/elapsed", json!("")),
        (
            "/elapsed",
            json!("340282366920938463463374607431768211456ns"),
        ),
        ("/elapsed", json!("18446744073709551615h")),
        ("/sequence", json!(1)),
        ("/metrics/0/active", json!(1)),
        (
            "/stage",
            json!({"id":"copy", "name":"Copy", "position":1, "total":0}),
        ),
        (
            "/stage",
            json!({"id":"copy", "name":"Copy", "position":null, "total":1}),
        ),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).expect("field must exist") = replacement;
        assert!(serde_json::from_value::<Event>(invalid).is_err());
    }

    for invalid in [
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":1, "completed":1, "active":1, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":null, "completed":18446744073709551615_u64, "active":0, "succeeded":18446744073709551615_u64, "failed":1, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":18446744073709551615_u64, "completed":18446744073709551615_u64, "active":1, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":1, "completed":1, "active":0, "succeeded":2, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":0, "completed":0, "active":1, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0},
                {"id":"tasks", "name":"Tasks", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 1, "phase": "running",
            "stage": {"id":"copy", "name":"Copy", "position":1, "total":null},
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
        json!({
            "operation_id": 1, "sequence": 0, "phase": "running", "stage": null,
            "metrics": [
                {"id":"tasks", "name":"Tasks", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0}
            ], "elapsed": "1ns"
        }),
    ] {
        assert!(serde_json::from_value::<Event>(invalid).is_err());
    }
}

/// Verifies standalone metric snapshots reject invalid definitions and counts.
#[cfg(feature = "serde")]
#[test]
fn test_metric_snapshot_deserialization_rejects_invalid_invariants() {
    for value in [
        json!({"id":"", "name":"Tasks", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0}),
        json!({"id":"tasks", "name":"", "total":null, "completed":0, "active":0, "succeeded":0, "failed":0, "cancelled":0}),
        json!({"id":"tasks", "name":"Tasks", "total":1, "completed":1, "active":1, "succeeded":0, "failed":0, "cancelled":0}),
        json!({"id":"tasks", "name":"Tasks", "total":1, "completed":1, "active":0, "succeeded":2, "failed":0, "cancelled":0}),
        json!({"id":"tasks", "name":"Tasks", "total":0, "completed":0, "active":1, "succeeded":0, "failed":0, "cancelled":0}),
    ] {
        assert!(serde_json::from_value::<MetricSnapshot>(value).is_err());
    }
}
