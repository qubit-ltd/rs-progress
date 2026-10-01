// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

use qubit_progress::AsyncProgress;
use qubit_progress::AsyncReporter;
use qubit_progress::EmissionError;
use qubit_progress::Event;
use qubit_progress::Metric;
use qubit_progress::Phase;
use qubit_progress::ReportFuture;
use qubit_progress::ReporterError;
use qubit_progress::StartError;

#[derive(Default)]
struct RecordingReporter {
    events: Mutex<Vec<Event>>,
}

impl RecordingReporter {
    /// Copies the recorded events for assertions.
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("event lock must work").clone()
    }
}

impl AsyncReporter for RecordingReporter {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async move {
            self.events.lock().expect("event lock must work").push(event.clone());
            Ok(())
        })
    }
}

struct RejectingReporter;

impl AsyncReporter for RejectingReporter {
    fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async { Err(ReporterError::message("async delivery rejected")) })
    }
}

struct RejectingRunningReporter {
    calls: AtomicUsize,
}

impl AsyncReporter for RejectingRunningReporter {
    fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
        if self.calls.fetch_add(1, Ordering::Relaxed) == 1 {
            Box::pin(async { Err(ReporterError::message("running event rejected")) })
        } else {
            Box::pin(async { Ok(()) })
        }
    }
}

struct PendingRunningReporter {
    events: Mutex<Vec<Event>>,
}

impl AsyncReporter for PendingRunningReporter {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        self.events.lock().expect("event lock must work").push(event.clone());
        if event.phase() == Phase::Running {
            Box::pin(std::future::pending())
        } else {
            Box::pin(async { Ok(()) })
        }
    }
}

/// Polls a future once to verify pending behavior without a runtime.
fn poll_once<F: Future>(mut future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.as_mut().poll(&mut context)
}

/// Runs test futures that are guaranteed to complete promptly.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}

#[test]
fn test_async_lifecycle_emits_started_running_and_succeeded_in_sequence() {
    let reporter = RecordingReporter::default();
    let mut progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    )
    .expect("async progress must start");
    block_on(progress.report_async()).expect("running event must be delivered");
    block_on(progress.finish_async()).expect("terminal event must be delivered");
    assert_eq!(
        reporter.events().iter().map(Event::phase).collect::<Vec<_>>(),
        [Phase::Started, Phase::Running, Phase::Succeeded]
    );
    assert_eq!(
        reporter.events().iter().map(Event::sequence).collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn test_async_reporter_failure_retains_complete_event() {
    let result = block_on(
        AsyncProgress::builder(&RejectingReporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    );
    let StartError::Delivery(error) = (match result {
        Ok(_) => panic!("started delivery must fail"),
        Err(error) => error,
    }) else {
        panic!("started delivery must produce a delivery error");
    };
    assert_eq!(error.reporter_error().to_string(), "async delivery rejected");
    assert_eq!(error.event().phase(), Phase::Started);
    assert_eq!(error.event().sequence(), 0);
}

#[test]
fn test_async_running_error_keeps_event_and_operation_usable() {
    let reporter = RejectingRunningReporter {
        calls: AtomicUsize::new(0),
    };
    let mut progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    )
    .expect("started event must be delivered");
    let EmissionError::Delivery(error) = block_on(progress.report_async()).expect_err("running event must be rejected")
    else {
        panic!("running delivery must preserve its failed event");
    };
    assert_eq!(error.event().phase(), Phase::Running);
    assert_eq!(error.event().sequence(), 1);
    block_on(progress.cancel_async()).expect("progress must remain terminally usable");
}

#[test]
fn test_cancelled_running_future_does_not_consume_progress_operation() {
    let reporter = PendingRunningReporter {
        events: Mutex::new(Vec::new()),
    };
    let mut progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    )
    .expect("start delivery must finish");
    let mut running = Box::pin(progress.report_async());
    assert!(poll_once(running.as_mut()).is_pending());
    drop(running);

    block_on(progress.finish_async()).expect("progress remains finishable after cancellation");
    let events = reporter.events.lock().expect("event lock must work");
    assert_eq!(
        events.iter().map(Event::phase).collect::<Vec<_>>(),
        [Phase::Started, Phase::Running, Phase::Succeeded]
    );
    assert_eq!(events.iter().map(Event::sequence).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn test_async_report_if_due_respects_interval() {
    let reporter = RecordingReporter::default();
    let mut progress = block_on(
        AsyncProgress::builder(&reporter)
            .interval(Duration::MAX)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    )
    .expect("async progress must start");
    block_on(progress.report_if_due_async()).expect("not-due report must succeed");
    assert_eq!(reporter.events().len(), 1);
    block_on(progress.cancel_async()).expect("cancel event must be delivered");
}
