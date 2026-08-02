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
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use qubit_progress::{
    CompletionError, ConfigurationError, EmissionError, Event, Metric, MetricError, Phase,
    Progress, Reporter, ReporterError, Stage, StartError,
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
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
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

/// Verifies that the explicitly unchecked finish behavior remains available.
#[test]
fn test_progress_finish_unchecked_allows_incomplete_metrics() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress run must start");

    progress
        .finish_unchecked()
        .expect("unchecked finish must allow incomplete metrics");
    assert_eq!(
        reporter.events().last().expect("terminal event").phase(),
        Phase::Succeeded
    );
}

/// Verifies that finish rejects work that remains active.
#[test]
fn test_progress_finish_requires_active_work_to_be_zero() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress run must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(1).expect("work must start");

    let error = progress
        .finish()
        .expect_err("finish must reject active work");
    let (returned, completion) = error
        .into_parts()
        .expect("incomplete finish is recoverable");
    assert!(matches!(
        completion,
        CompletionError::ActiveWork {
            metric_id,
            active: 1,
        } if metric_id == "tasks"
    ));
    assert_eq!(reporter.events().len(), 1);
    assert!(matches!(
        tasks.start(1),
        Err(MetricError::TotalExceeded { metric_id, .. }) if metric_id == "tasks"
    ));
    returned
        .cancel()
        .expect("reopened progress must remain terminally usable");
}

/// Verifies that finish rejects an incompletely satisfied known total.
#[test]
fn test_progress_finish_requires_known_total_to_be_completed() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress run must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(1).expect("work must start");
    tasks.succeed(1).expect("work must succeed");

    let error = progress
        .finish()
        .expect_err("finish must reject an incomplete total");
    let (returned, completion) = error
        .into_parts()
        .expect("incomplete finish is recoverable");
    assert!(matches!(
        completion,
        CompletionError::IncompleteTotal {
            metric_id,
            completed: 1,
            total: 2,
        } if metric_id == "tasks"
    ));
    assert_eq!(reporter.events().len(), 1);
    returned
        .cancel()
        .expect("reopened progress must remain terminally usable");
}

/// Verifies that finish accepts complete known and unknown totals.
#[test]
fn test_progress_finish_accepts_complete_known_and_unknown_metrics() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .metric(Metric::new("bytes", "Bytes"))
        .start()
        .expect("progress run must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured task metric must exist");
    tasks.start(1).expect("task must start");
    tasks.succeed(1).expect("task must succeed");
    let bytes = progress
        .metric("bytes")
        .expect("configured byte metric must exist");
    bytes.start(4).expect("bytes must start");
    bytes.complete(4).expect("bytes must complete");

    progress
        .finish()
        .expect("complete metrics must pass finish");
    assert_eq!(
        reporter.events().last().expect("terminal event").phase(),
        Phase::Succeeded
    );
}

/// Verifies that a maximum interval remains valid without creating an
/// unrepresentable absolute deadline.
#[test]
fn test_progress_accepts_maximum_interval_without_absolute_deadline() {
    let reporter = RecordingReporter::default();
    let result = Progress::builder(&reporter)
        .interval(Duration::MAX)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let mut progress = result.expect("maximum interval must be valid");
    progress
        .report_if_due()
        .expect("maximum interval is not due immediately");
    assert_eq!(reporter.events().len(), 1);
    progress
        .cancel()
        .expect("terminal event must remain available");
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
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
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

    assert_eq!(tasks.snapshot().succeeded(), 1,);
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
        StartError::InvalidConfiguration(ConfigurationError::DuplicateMetricId { .. })
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
    progress
        .cancel()
        .expect("a valid terminal snapshot must be accepted after validation failure");
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
    progress
        .report_if_due()
        .expect("early report must be skipped");
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

/// Reporter that rejects every delivery to exercise start errors.
struct RejectingReporter;

impl Reporter for RejectingReporter {
    /// Rejects the supplied event with a stable test error.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        Err(ReporterError::message("delivery rejected"))
    }
}

/// Verifies reporter failures propagate from Started delivery.
#[test]
fn test_progress_propagates_reporter_failures() {
    let result = Progress::builder(&RejectingReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("Started delivery must fail"),
        Err(error) => error,
    };
    assert!(matches!(error, StartError::Delivery(_)));
}

/// Reporter that accepts Started and rejects the first Running event.
struct RunningRejectingReporter {
    /// Number of delivery attempts accepted or rejected so far.
    reports: AtomicUsize,
    /// Sequences observed before deciding whether to reject delivery.
    sequences: Mutex<Vec<u64>>,
}

impl RunningRejectingReporter {
    /// Creates a reporter whose first report succeeds and second report fails.
    const fn new() -> Self {
        Self {
            reports: AtomicUsize::new(0),
            sequences: Mutex::new(Vec::new()),
        }
    }
}

impl Reporter for RunningRejectingReporter {
    /// Accepts Started, rejects the first Running event, then accepts later
    /// events.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        let report_index = self.reports.fetch_add(1, Ordering::Relaxed);
        self.sequences
            .lock()
            .expect("sequence collection mutex must not be poisoned")
            .push(event.sequence());
        if report_index == 1 {
            Err(ReporterError::message("running delivery rejected"))
        } else {
            Ok(())
        }
    }
}

/// Verifies failed Running delivery consumes its sequence and remains
/// recoverable.
#[test]
fn test_progress_propagates_running_report_failure_and_preserves_sequence() {
    let reporter = RunningRejectingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started delivery must succeed");

    let error = progress
        .report()
        .expect_err("the first Running delivery must fail");
    assert!(matches!(error, EmissionError::Delivery(_)));

    progress
        .report()
        .expect("a later Running delivery must remain possible");
    progress
        .finish()
        .expect("terminal delivery must remain possible after a Running failure");
    assert_eq!(reporter.reports.load(Ordering::Relaxed), 4);
    assert_eq!(
        reporter
            .sequences
            .lock()
            .expect("sequence collection mutex must not be poisoned")
            .as_slice(),
        &[0, 1, 2, 3],
    );
}
