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
    progress.report_if_due_async().await?;
    progress.finish_async().await?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
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
