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
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::AsyncProgress;
use crate::AsyncReporter;
use crate::Event;
use crate::Metric;
use crate::Phase;
use crate::ReportFuture;
use crate::ReporterError;

#[derive(Clone, Copy)]
enum RunningResult {
    Pending,
    Success,
    Error,
}

struct TestReporter {
    running_result: RunningResult,
    events: Mutex<Vec<Event>>,
    enabled: bool,
}

impl TestReporter {
    fn new(running_result: RunningResult) -> Self {
        Self {
            running_result,
            events: Mutex::new(Vec::new()),
            enabled: true,
        }
    }

    fn disabled() -> Self {
        Self {
            running_result: RunningResult::Pending,
            events: Mutex::new(Vec::new()),
            enabled: false,
        }
    }

    fn sequences(&self) -> Vec<u64> {
        self.events
            .lock()
            .expect("event lock must work")
            .iter()
            .map(Event::sequence)
            .collect()
    }
}

impl AsyncReporter for TestReporter {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        self.events.lock().expect("event lock must work").push(event.clone());
        if event.phase() != Phase::Running {
            return Box::pin(async { Ok(()) });
        }
        match self.running_result {
            RunningResult::Pending => Box::pin(std::future::pending()),
            RunningResult::Success => Box::pin(async { Ok(()) }),
            RunningResult::Error => Box::pin(async { Err(ReporterError::message("running rejected")) }),
        }
    }
}

fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

struct ResumableReporter {
    result: Mutex<Option<Result<(), ReporterError>>>,
    waker: Mutex<Option<Waker>>,
}

impl ResumableReporter {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            waker: Mutex::new(None),
        }
    }

    fn complete(&self, result: Result<(), ReporterError>) {
        *self.result.lock().expect("result lock must work") = Some(result);
        let waker = self.waker.lock().expect("waker lock must work").take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl AsyncReporter for ResumableReporter {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        if event.phase() != Phase::Running {
            return Box::pin(async { Ok(()) });
        }
        Box::pin(std::future::poll_fn(move |context| {
            let mut result = self.result.lock().expect("result lock must work");
            if let Some(result) = result.take() {
                Poll::Ready(result)
            } else {
                *self.waker.lock().expect("waker lock must work") = Some(context.waker().clone());
                Poll::Pending
            }
        }))
    }
}

async fn started_progress(reporter: &TestReporter) -> AsyncProgress<'_> {
    AsyncProgress::builder(reporter)
        .interval(Duration::MAX)
        .metric(Metric::new("tasks", "Tasks"))
        .start_async()
        .await
        .expect("progress must start")
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(output) = poll_once(future.as_mut()) {
            return output;
        }
        thread::yield_now();
    }
}

fn make_due(progress: &mut AsyncProgress<'_>) {
    progress.started_at = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("test clock must support one second of history");
    progress.next_due_elapsed = Some(Duration::ZERO);
}

#[test]
fn test_cancelled_due_running_attempt_consumes_sequence_without_resetting_deadline() {
    let reporter = TestReporter::new(RunningResult::Pending);
    let mut progress = block_on(started_progress(&reporter));
    make_due(&mut progress);

    let mut running = Box::pin(progress.report_if_due_async());
    assert!(poll_once(running.as_mut()).is_pending());
    drop(running);

    assert_eq!(progress.next_sequence, 2);
    assert_eq!(progress.next_due_elapsed, Some(Duration::ZERO));
    assert_eq!(reporter.sequences(), [0, 1]);

    let mut next = Box::pin(progress.report_if_due_async());
    assert!(poll_once(next.as_mut()).is_pending());
    drop(next);
    assert_eq!(progress.next_sequence, 3);
    assert_eq!(reporter.sequences(), [0, 1, 2]);
}

#[test]
fn test_unpolled_running_future_drop_keeps_sequence_and_deadline() {
    let reporter = TestReporter::new(RunningResult::Success);
    let mut progress = block_on(started_progress(&reporter));
    make_due(&mut progress);
    let before_sequence = progress.next_sequence;
    let before_deadline = progress.next_due_elapsed;

    drop(progress.report_if_due_async());

    assert_eq!(progress.next_sequence, before_sequence);
    assert_eq!(progress.next_due_elapsed, before_deadline);
    assert_eq!(reporter.sequences(), [0]);
}

#[test]
fn test_completed_running_attempt_resets_deadline_on_success_and_error() {
    for result in [RunningResult::Success, RunningResult::Error] {
        let reporter = TestReporter::new(result);
        let mut progress = block_on(started_progress(&reporter));
        make_due(&mut progress);
        let mut running = Box::pin(progress.report_async());
        let output = poll_once(running.as_mut());
        match result {
            RunningResult::Success => assert!(matches!(output, Poll::Ready(Ok(())))),
            RunningResult::Error => assert!(matches!(output, Poll::Ready(Err(_)))),
            RunningResult::Pending => unreachable!(),
        }
        drop(running);

        assert_eq!(progress.next_sequence, 2);
        assert_eq!(progress.next_due_elapsed, None);
        assert_eq!(reporter.sequences(), [0, 1]);
    }
}

#[test]
fn test_resumed_running_attempt_resets_deadline_from_completion() {
    for due_only in [false, true] {
        for fail in [false, true] {
            let reporter = ResumableReporter::new();
            let interval = Duration::from_secs(60);
            let mut progress = block_on(
                AsyncProgress::builder(&reporter)
                    .interval(interval)
                    .metric(Metric::new("tasks", "Tasks"))
                    .start_async(),
            )
            .expect("progress must start");
            make_due(&mut progress);
            let origin = progress.started_at;

            let mut running: Pin<Box<dyn Future<Output = Result<(), crate::EmissionError>> + '_>> = if due_only {
                Box::pin(progress.report_if_due_async())
            } else {
                Box::pin(progress.report_async())
            };
            assert!(poll_once(running.as_mut()).is_pending());
            let pending_lower_bound = origin.elapsed();
            thread::sleep(Duration::from_millis(10));
            let completion_lower_bound = origin.elapsed();
            assert!(completion_lower_bound > pending_lower_bound);

            reporter.complete(if fail {
                Err(ReporterError::message("resumed delivery failed"))
            } else {
                Ok(())
            });
            let output = poll_once(running.as_mut());
            if fail {
                match output {
                    Poll::Ready(Err(crate::EmissionError::Delivery(delivery))) => {
                        assert_eq!(delivery.event().phase(), Phase::Running);
                        assert_eq!(delivery.event().sequence(), 1);
                    }
                    other => panic!("expected resumed delivery error, got {other:?}"),
                }
            } else {
                assert!(matches!(output, Poll::Ready(Ok(()))));
            }
            drop(running);

            let completion_upper_bound = progress.elapsed();
            let deadline = progress.next_due_elapsed.expect("finite interval must fit");
            assert!(deadline >= completion_lower_bound.checked_add(interval).unwrap());
            assert!(deadline <= completion_upper_bound.checked_add(interval).unwrap());
            assert_eq!(progress.next_sequence, 2);
            assert_eq!(progress.metric("tasks").unwrap().snapshot().active(), 0);
            block_on(progress.cancel_async()).expect("operation remains usable");
        }
    }
}

#[test]
fn test_disabled_running_report_keeps_sequence_and_deadline() {
    let reporter = TestReporter::disabled();
    let mut progress = block_on(started_progress(&reporter));
    make_due(&mut progress);
    let before_deadline = progress.next_due_elapsed;

    let mut running = Box::pin(progress.report_if_due_async());
    assert!(matches!(poll_once(running.as_mut()), Poll::Ready(Ok(()))));
    drop(running);

    assert_eq!(progress.next_sequence, 0);
    assert_eq!(progress.next_due_elapsed, before_deadline);
    assert!(reporter.sequences().is_empty());
}
