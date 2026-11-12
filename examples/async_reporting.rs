// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Demonstrates asynchronous progress reporting without a runtime dependency.

use std::error::Error;
use std::future::Future;
use std::io;
use std::pin::pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;
use std::thread;

use qubit_progress::AsyncProgress;
use qubit_progress::AsyncReporter;
use qubit_progress::Event;
use qubit_progress::Metric;
use qubit_progress::Phase;
use qubit_progress::ReportFuture;
use qubit_progress::ReporterError;

#[derive(Default)]
struct RecordingSink(Mutex<Vec<Event>>);

impl AsyncReporter for RecordingSink {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async move {
            self.0.lock().expect("recording lock must work").push(event.clone());
            Ok(())
        })
    }
}

#[derive(Clone, Copy)]
struct RejectPhases {
    running: bool,
    succeeded: bool,
}

struct FailingSink {
    rejected: RejectPhases,
    attempts: Mutex<Vec<Phase>>,
}

impl FailingSink {
    fn new(rejected: RejectPhases) -> Self {
        Self {
            rejected,
            attempts: Mutex::new(Vec::new()),
        }
    }
}

impl AsyncReporter for FailingSink {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async move {
            let phase = event.phase();
            self.attempts.lock().expect("attempt lock must work").push(phase);
            match phase {
                Phase::Running if self.rejected.running => Err(ReporterError::message("running rejected")),
                Phase::Succeeded if self.rejected.succeeded => Err(ReporterError::message("succeeded rejected")),
                _ => Ok(()),
            }
        })
    }
}

// This wake implementation supports the local thread-based teaching executor.
struct ThreadWake(thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

// This executor is only for this standalone example; it has no async I/O
// reactor.
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

async fn run_batch(reporter: &dyn AsyncReporter, fail_work: bool) -> Result<(), Box<dyn Error>> {
    let mut progress = AsyncProgress::builder(reporter)
        .metric(Metric::new("items", "Items").total(1))
        .start_async()
        .await?;
    let items = progress.metric("items").expect("configured metric must exist");
    items.start(1)?;

    if fail_work {
        let work_error = io::Error::other("demonstration work failure");
        items.fail(1)?;
        if let Err(report_error) = progress.fail_async().await {
            // Preserve the business error while reporting the separate sink failure.
            eprintln!("terminal reporting also failed: {report_error}");
        }
        return Err(Box::new(work_error));
    }

    items.succeed(1)?;
    let running_result = progress.report_if_due_async().await;
    let terminal_result = progress.finish_async().await;
    if let Err(error) = running_result {
        if let Err(terminal_error) = terminal_result {
            eprintln!("terminal reporting also failed: {terminal_error}");
        }
        return Err(Box::new(error));
    }
    terminal_result?;
    Ok(())
}

fn assert_success_case(rejected: RejectPhases, expected: Result<(), &str>) {
    let sink = FailingSink::new(rejected);
    let result = block_on(run_batch(&sink, false));
    let attempts = sink.attempts.lock().expect("attempt lock must work");
    assert_eq!(*attempts, vec![Phase::Started, Phase::Running, Phase::Succeeded]);
    match (result, expected) {
        (Ok(()), Ok(())) => {}
        (Err(error), Err(message)) => assert!(error.to_string().contains(message), "unexpected error: {error}"),
        (result, expected) => panic!("result {result:?} did not match {expected:?}"),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    assert_success_case(
        RejectPhases {
            running: false,
            succeeded: false,
        },
        Ok(()),
    );
    assert_success_case(
        RejectPhases {
            running: false,
            succeeded: true,
        },
        Err("succeeded rejected"),
    );
    assert_success_case(
        RejectPhases {
            running: true,
            succeeded: false,
        },
        Err("running rejected"),
    );
    assert_success_case(
        RejectPhases {
            running: true,
            succeeded: true,
        },
        Err("running rejected"),
    );

    let sink = RecordingSink::default();
    block_on(run_batch(&sink, false))?;
    {
        let events = sink.0.lock().expect("recording lock must work");
        let phases: Vec<_> = events.iter().map(Event::phase).collect();
        assert_eq!(phases, vec![Phase::Started, Phase::Running, Phase::Succeeded]);
        assert_eq!(events[2].metrics()[0].succeeded(), 1);
    }
    assert!(block_on(run_batch(&sink, true)).is_err());
    assert_eq!(
        sink.0
            .lock()
            .expect("recording lock must work")
            .last()
            .expect("failed event must exist")
            .phase(),
        Phase::Failed
    );
    println!("success: Started -> Running -> Succeeded; work error: Started -> Failed");
    Ok(())
}
