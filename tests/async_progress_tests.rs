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
use qubit_progress::CompletionError;
use qubit_progress::EmissionError;
use qubit_progress::Event;
use qubit_progress::FinishError;
use qubit_progress::Metric;
use qubit_progress::MetricError;
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

#[derive(Clone, Copy)]
enum TerminalBehavior {
    Pending,
    Reject,
}

struct TerminalReporter {
    phase: Phase,
    behavior: TerminalBehavior,
    events: Mutex<Vec<Event>>,
}

impl TerminalReporter {
    fn new(phase: Phase, behavior: TerminalBehavior) -> Self {
        Self {
            phase,
            behavior,
            events: Mutex::new(Vec::new()),
        }
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("event lock must work").clone()
    }
}

impl AsyncReporter for TerminalReporter {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        self.events.lock().expect("event lock must work").push(event.clone());
        if event.phase() != self.phase {
            return Box::pin(async { Ok(()) });
        }
        match self.behavior {
            TerminalBehavior::Pending => Box::pin(std::future::pending()),
            TerminalBehavior::Reject => Box::pin(async { Err(ReporterError::message("terminal rejected")) }),
        }
    }
}

struct DisabledReporter {
    calls: AtomicUsize,
}

impl AsyncReporter for DisabledReporter {
    fn is_enabled(&self) -> bool {
        false
    }

    fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        panic!("disabled reporter must not receive events")
    }
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

macro_rules! assert_pending_terminal_closes_handle {
    ($method:ident, $phase:expr) => {{
        let reporter = TerminalReporter::new($phase, TerminalBehavior::Pending);
        let progress = block_on(
            AsyncProgress::builder(&reporter)
                .metric(Metric::new("tasks", "Tasks").total(1))
                .start_async(),
        )
        .expect("progress must start");
        let handle = progress.metric("tasks").expect("metric must exist");
        handle.start(1).expect("work must start");
        handle.succeed(1).expect("work must finish");

        let mut terminal = Box::pin(progress.$method());
        assert!(poll_once(terminal.as_mut()).is_pending());
        drop(terminal);

        assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
        let events = reporter.events();
        assert_eq!(
            events.iter().map(Event::phase).collect::<Vec<_>>(),
            [Phase::Started, $phase]
        );
        assert_eq!(events.iter().map(Event::sequence).collect::<Vec<_>>(), [0, 1]);
    }};
}

#[test]
fn test_each_polled_pending_terminal_future_closes_its_operation_without_retry() {
    assert_pending_terminal_closes_handle!(finish_async, Phase::Succeeded);
    assert_pending_terminal_closes_handle!(finish_unchecked_async, Phase::Succeeded);
    assert_pending_terminal_closes_handle!(fail_async, Phase::Failed);
    assert_pending_terminal_closes_handle!(cancel_async, Phase::Cancelled);
}

macro_rules! assert_unpolled_terminal_drop_is_local_only {
    ($method:ident, $phase:expr) => {{
        let reporter = TerminalReporter::new($phase, TerminalBehavior::Reject);
        let progress = block_on(
            AsyncProgress::builder(&reporter)
                .metric(Metric::new("tasks", "Tasks"))
                .start_async(),
        )
        .expect("progress must start");
        let handle = progress.metric("tasks").expect("metric must exist");
        let terminal = progress.$method();
        drop(terminal);

        assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
        let events = reporter.events();
        assert_eq!(events.iter().map(Event::phase).collect::<Vec<_>>(), [Phase::Started]);
        assert_eq!(events.iter().map(Event::sequence).collect::<Vec<_>>(), [0]);
    }};
}

#[test]
fn test_unpolled_terminal_futures_close_locally_without_remote_attempt() {
    assert_unpolled_terminal_drop_is_local_only!(finish_async, Phase::Succeeded);
    assert_unpolled_terminal_drop_is_local_only!(finish_unchecked_async, Phase::Succeeded);
    assert_unpolled_terminal_drop_is_local_only!(fail_async, Phase::Failed);
    assert_unpolled_terminal_drop_is_local_only!(cancel_async, Phase::Cancelled);
}

macro_rules! assert_terminal_rejection_is_preserved {
    (finish_async, $phase:expr) => {{
        let reporter = TerminalReporter::new($phase, TerminalBehavior::Reject);
        let progress = block_on(
            AsyncProgress::builder(&reporter)
                .metric(Metric::new("tasks", "Tasks"))
                .attribute("request", "terminal-test")
                .start_async(),
        )
        .expect("progress must start");
        let handle = progress.metric("tasks").expect("metric must exist");
        let result = block_on(progress.finish_async());
        let FinishError::Terminal(error) = result.expect_err("terminal report must fail") else {
            panic!("finish must wrap terminal delivery failure");
        };
        let elapsed = error.elapsed();
        let qubit_progress::EmissionError::Delivery(delivery) = error.emission_error() else {
            panic!("terminal failure must preserve delivery error");
        };
        let event = delivery.event().clone();
        let source = delivery.reporter_error().to_string();
        assert!(elapsed >= Duration::ZERO);
        assert_eq!(source, "terminal rejected");
        assert_eq!(event.phase(), $phase);
        assert_eq!(event.sequence(), 1);
        assert!(event.operation_id() > 0);
        assert_eq!(event.elapsed(), elapsed);
        assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
        let events = reporter.events();
        assert_eq!(
            events.iter().map(Event::phase).collect::<Vec<_>>(),
            [Phase::Started, $phase]
        );
        assert_eq!(events[1].operation_id(), events[0].operation_id());
        assert_eq!(events[1].sequence(), 1);
    }};
    ($method:ident, $phase:expr) => {{
        let reporter = TerminalReporter::new($phase, TerminalBehavior::Reject);
        let progress = block_on(
            AsyncProgress::builder(&reporter)
                .metric(Metric::new("tasks", "Tasks"))
                .attribute("request", "terminal-test")
                .start_async(),
        )
        .expect("progress must start");
        let handle = progress.metric("tasks").expect("metric must exist");
        let error = block_on(progress.$method()).expect_err("terminal report must fail");
        let elapsed = error.elapsed();
        let qubit_progress::EmissionError::Delivery(delivery) = error.emission_error() else {
            panic!("terminal failure must preserve delivery error");
        };
        let event = delivery.event().clone();
        assert_eq!(delivery.reporter_error().to_string(), "terminal rejected");
        assert_eq!(event.phase(), $phase);
        assert_eq!(event.sequence(), 1);
        assert!(event.operation_id() > 0);
        assert_eq!(event.elapsed(), elapsed);
        assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
        let events = reporter.events();
        assert_eq!(
            events.iter().map(Event::phase).collect::<Vec<_>>(),
            [Phase::Started, $phase]
        );
        assert_eq!(events[1].operation_id(), events[0].operation_id());
        assert_eq!(events[1].sequence(), 1);
    }};
}

#[test]
fn test_terminal_reporter_errors_preserve_error_context_and_close_handles() {
    assert_terminal_rejection_is_preserved!(finish_async, Phase::Succeeded);
    assert_terminal_rejection_is_preserved!(finish_unchecked_async, Phase::Succeeded);
    assert_terminal_rejection_is_preserved!(fail_async, Phase::Failed);
    assert_terminal_rejection_is_preserved!(cancel_async, Phase::Cancelled);
}

#[test]
fn test_finish_async_active_work_is_incomplete_and_consumes_operation() {
    let reporter = TerminalReporter::new(Phase::Succeeded, TerminalBehavior::Reject);
    let progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start_async(),
    )
    .expect("progress must start");
    let handle = progress.metric("tasks").expect("metric must exist");
    handle.start(2).expect("work must start");
    let error = block_on(progress.finish_async()).expect_err("active work must prevent success");
    assert!(matches!(
        error,
        FinishError::Incomplete {
            source: CompletionError::ActiveWork { active: 2, .. },
            ..
        }
    ));
    assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
    assert_eq!(
        reporter.events().iter().map(Event::phase).collect::<Vec<_>>(),
        [Phase::Started]
    );
}

#[test]
fn test_finish_async_incomplete_total_without_active_work_consumes_operation() {
    let reporter = TerminalReporter::new(Phase::Succeeded, TerminalBehavior::Reject);
    let progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks").total(2))
            .start_async(),
    )
    .expect("progress must start");
    let handle = progress.metric("tasks").expect("metric must exist");
    handle.start(1).expect("work must start");
    handle.complete(1).expect("work must complete");
    let error = block_on(progress.finish_async()).expect_err("unfinished total must prevent success");
    assert!(matches!(
        error,
        FinishError::Incomplete {
            source: CompletionError::IncompleteTotal {
                completed: 1,
                total: 2,
                ..
            },
            ..
        }
    ));
    assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
    assert_eq!(
        reporter.events().iter().map(Event::phase).collect::<Vec<_>>(),
        [Phase::Started]
    );
}

#[test]
fn test_disabled_async_reporter_skips_events_but_keeps_metric_lifecycle() {
    let reporter = DisabledReporter {
        calls: AtomicUsize::new(0),
    };
    let progress = block_on(
        AsyncProgress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks").total(1))
            .start_async(),
    )
    .expect("disabled progress must start");
    assert!(!progress.is_enabled());
    let handle = progress.metric("tasks").expect("configured metrics are available");
    handle.start(1).expect("disabled metrics remain mutable");
    handle.succeed(1).expect("disabled metrics remain mutable");
    block_on(progress.finish_async()).expect("complete disabled progress must finish");
    assert_eq!(handle.snapshot().succeeded(), 1);
    assert!(matches!(handle.start(1), Err(MetricError::OperationNotOpen { .. })));
    assert_eq!(reporter.calls.load(Ordering::Relaxed), 0);

    let error = match block_on(AsyncProgress::builder(&reporter).start_async()) {
        Ok(_) => panic!("disabled reporter must not bypass metric validation"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        StartError::InvalidConfiguration(qubit_progress::ConfigurationError::NoMetrics)
    ));
    assert_eq!(reporter.calls.load(Ordering::Relaxed), 0);
}
