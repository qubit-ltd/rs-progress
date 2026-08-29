// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for the replacement progress lifecycle API.

use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_progress::DeliveryError;
use qubit_progress::EmissionError;
use qubit_progress::Event;
use qubit_progress::Metric;
use qubit_progress::NoopReporter;
use qubit_progress::Phase;
use qubit_progress::Progress;
use qubit_progress::RecoverableFinishError;
use qubit_progress::Reporter;
use qubit_progress::ReporterError;
use qubit_progress::StartError;

#[derive(Default)]
struct RecordingReporter {
    events: Mutex<Vec<Event>>,
}

impl RecordingReporter {
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("events mutex must not poison").clone()
    }
}

impl Reporter for RecordingReporter {
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self.events
            .lock()
            .expect("events mutex must not poison")
            .push(event.clone());
        Ok(())
    }
}

struct RejectingReporter;

impl Reporter for RejectingReporter {
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        Err(ReporterError::message("delivery rejected"))
    }
}

struct RunningRejectingReporter {
    calls: AtomicUsize,
    events: Mutex<Vec<Event>>,
}

impl RunningRejectingReporter {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            events: Mutex::new(Vec::new()),
        }
    }
}

impl Reporter for RunningRejectingReporter {
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self.events
            .lock()
            .expect("events mutex must not poison")
            .push(event.clone());
        if self.calls.fetch_add(1, Ordering::Relaxed) == 1 {
            Err(ReporterError::message("running delivery rejected"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn test_started_delivery_error_retains_complete_event() {
    let result = Progress::builder(&RejectingReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("started delivery must fail"),
        Err(error) => error,
    };
    let StartError::Delivery(delivery) = error else {
        panic!("started delivery must use StartError::Delivery");
    };
    assert_eq!(delivery.event().phase(), Phase::Started);
    assert_eq!(delivery.event().sequence(), 0);
    assert!(delivery.event().elapsed().is_zero());
}

#[test]
fn test_running_delivery_error_consumes_sequence_and_keeps_progress_open() {
    let reporter = RunningRejectingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("started delivery must succeed");

    let first = progress.report().expect_err("first running delivery must fail");
    let EmissionError::Delivery(first) = first else {
        panic!("running failure must use EmissionError::Delivery");
    };
    assert_eq!(first.event().sequence(), 1);
    progress.report().expect("progress must remain usable");
    progress.cancel().expect("terminal delivery must remain possible");

    let events = reporter.events.lock().expect("events mutex must not poison");
    assert_eq!(events.iter().map(Event::sequence).collect::<Vec<_>>(), [0, 1, 2, 3]);
}

#[test]
fn test_incomplete_finish_returns_reusable_progress() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");
    tasks.start(1).expect("task must start");

    let Err(RecoverableFinishError::Incomplete { progress: returned, .. }) = progress.finish_recoverable() else {
        panic!("incomplete finish must return Progress");
    };
    progress = returned;
    tasks.succeed(1).expect("returned progress must reopen metric updates");
    progress.finish().expect("repaired progress must finish");
}

#[test]
fn test_maximum_interval_does_not_fail_start_or_emit_early() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::MAX)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("maximum duration must be a valid interval");
    progress
        .report_if_due()
        .expect("maximum interval must not be due immediately");
    assert_eq!(reporter.events().len(), 1);
}

#[test]
fn test_delivery_error_can_be_split_into_event_and_reporter_error() {
    let reporter = RejectingReporter;
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let StartError::Delivery(error) = (match result {
        Ok(_) => panic!("start must fail"),
        Err(error) => error,
    }) else {
        panic!("expected delivery error");
    };
    let (event, source): (Event, ReporterError) = error.into_parts();
    assert_eq!(event.phase(), Phase::Started);
    assert_eq!(source.to_string(), "delivery rejected");
}

fn assert_delivery_error_is_send_sync<T: Send + Sync>() {}

#[test]
fn test_delivery_error_is_send_sync() {
    assert_delivery_error_is_send_sync::<DeliveryError>();
}

#[test]
fn test_disabled_terminal_finish_returns_without_delivery() {
    let progress = Progress::builder(&NoopReporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("disabled progress must start");
    let elapsed = progress
        .finish_unchecked()
        .expect("disabled terminal finish must not deliver");
    assert!(elapsed < Duration::from_secs(1));
}
