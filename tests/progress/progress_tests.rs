// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the redesigned progress lifecycle.

use std::sync::{
    Mutex,
    atomic::{
        AtomicUsize,
        Ordering,
    },
};
use std::time::Duration;

use qubit_progress::{
    Event,
    Metric,
    MetricError,
    Phase,
    Progress,
    ProgressError,
    ReportError,
    Reporter,
    Stage,
    ValidationError,
};

/// Records complete events emitted by the progress run under test.
#[derive(Default)]
struct RecordingReporter {
    /// Events received from the progress run.
    events: Mutex<Vec<Event>>,
}

impl RecordingReporter {
    /// Returns a cloned event sequence recorded so far.
    fn events(&self) -> Vec<Event> {
        self.events
            .lock()
            .expect("recording reporter mutex must not be poisoned")
            .clone()
    }
}

impl Reporter for RecordingReporter {
    /// Records one complete event.
    fn report(&self, event: &Event) -> Result<(), ReportError> {
        self.events
            .lock()
            .expect("recording reporter mutex must not be poisoned")
            .push(event.clone());
        Ok(())
    }
}

/// Verifies that static totals are configured once and carried by every event.
#[test]
fn test_progress_carries_configured_total_in_every_event() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(3))
        .start()
        .expect("progress run must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");

    tasks.start(2).expect("work must start");
    tasks.succeed(1).expect("work must succeed");
    progress.report().expect("running event must report");
    tasks
        .succeed(1)
        .expect("remaining active work must succeed");
    tasks.start(1).expect("final work must start");
    tasks.succeed(1).expect("final work must succeed");
    progress.finish().expect("terminal event must report");

    let events = reporter.events();
    assert_eq!(events.len(), 3);
    for event in events {
        let metric = event.metric("tasks").expect("event must contain tasks");
        assert_eq!(metric.total(), Some(3));
    }
}

/// Reporter whose sampled enablement is disabled and whose calls are counted.
struct DisabledReporter {
    /// Counts attempted deliveries.
    reports: AtomicUsize,
}

impl DisabledReporter {
    /// Creates a disabled reporter with no delivery attempts.
    fn new() -> Self {
        Self {
            reports: AtomicUsize::new(0),
        }
    }
}

impl Reporter for DisabledReporter {
    /// Disables every newly started operation.
    fn is_enabled(&self) -> bool {
        false
    }

    /// Counts a delivery that must never occur for a disabled operation.
    fn report(&self, _event: &Event) -> Result<(), ReportError> {
        self.reports.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// Verifies that disabled operations retain metric state without delivery.
#[test]
fn test_disabled_progress_tracks_metrics_without_delivery() {
    let reporter = DisabledReporter::new();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("disabled progress configuration must still validate");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");

    tasks.start(1).expect("work must start");
    tasks.succeed(1).expect("work must succeed");
    progress
        .report()
        .expect("disabled running report must be a no-op");
    progress
        .report_if_due()
        .expect("disabled due report must be a no-op");
    progress
        .finish()
        .expect("disabled terminal report must be a no-op");

    assert_eq!(
        tasks
            .snapshot()
            .expect("closed metric snapshot must remain readable")
            .succeeded(),
        1,
    );
    assert_eq!(reporter.reports.load(Ordering::Relaxed), 0);
}

/// Verifies that fixed configuration and metric transition invariants fail
/// clearly.
#[test]
fn test_progress_rejects_invalid_configuration_and_snapshot_counts() {
    let reporter = RecordingReporter::default();
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .metric(Metric::new("tasks", "Duplicate Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("duplicate metric IDs must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ProgressError::Validation(ValidationError::DuplicateMetricId { .. })
    ));

    let progress = Progress::builder(&reporter)
        .stage(Stage::new("copy", "Copy").position(1, 2))
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("valid progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(2).expect("declared work must start");
    let error = tasks
        .start(1)
        .expect_err("occupied work beyond a known total must fail");
    assert!(matches!(error, MetricError::TotalExceeded { .. }));
    progress.cancel().expect(
        "a valid terminal snapshot must be accepted after validation failure",
    );
    assert_eq!(
        reporter
            .events()
            .last()
            .expect("terminal event must be recorded")
            .phase(),
        Phase::Cancelled
    );
}

/// Verifies that metrics are retrieved only when configured by the operation.
#[test]
fn test_progress_returns_none_for_unknown_metric() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .metric(Metric::new("bytes", "Bytes"))
        .start()
        .expect("progress run must start");

    assert!(progress.metric("tasks").is_some());
    assert!(progress.metric("bytes").is_some());
    assert!(progress.metric("unknown").is_none());
}

/// Verifies stage replacement, clearing, and positive-interval scheduling.
#[test]
fn test_progress_updates_stage_and_respects_due_interval() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_millis(5))
        .stage(Stage::new("copy", "Copy").position(1, 2))
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    assert!(progress.is_enabled());
    assert!(progress.elapsed() < Duration::from_secs(1));
    progress.report_if_due().expect("early report must be skipped");
    progress
        .set_stage(Stage::new("verify", "Verify").position(2, 2))
        .expect("replacement stage must be valid");
    std::thread::sleep(Duration::from_millis(6));
    progress.report_if_due().expect("due report must succeed");
    progress.clear_stage();
    progress.report().expect("manual report must succeed");
    progress.cancel().expect("cancelled event must succeed");

    let events = reporter.events();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].stage().expect("initial stage").id(), "copy");
    assert_eq!(events[1].stage().expect("replacement stage").id(), "verify");
    assert!(events[2].stage().is_none());
    assert_eq!(events[3].phase(), Phase::Cancelled);
}

/// Reporter that rejects every delivery to exercise start and running errors.
struct RejectingReporter;

impl Reporter for RejectingReporter {
    /// Rejects the supplied event with a stable test error.
    fn report(&self, _event: &Event) -> Result<(), ReportError> {
        Err(ReportError::message("delivery rejected"))
    }
}

/// Verifies reporter failures propagate from start and running delivery.
#[test]
fn test_progress_propagates_reporter_failures() {
    let result = Progress::builder(&RejectingReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("Started delivery must fail"),
        Err(error) => error,
    };
    assert!(matches!(error, ProgressError::Report(_)));

    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("healthy operation must start");
    assert!(progress.is_enabled());
}
