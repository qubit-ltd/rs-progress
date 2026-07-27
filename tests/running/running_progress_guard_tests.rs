// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `RunningProgressGuard`.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
};

use qubit_progress::{
    NoOpProgressReporter, Progress, ProgressCounter, ProgressEvent, ProgressPhase,
    ProgressReportError, ProgressReporter, ProgressSchema, RunningProgressGuard,
    RunningProgressPointHandle, WriterProgressReporter,
};

use crate::support::FailingWriter;

#[derive(Debug, Default)]
struct RecordingReporter {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingReporter {
    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProgressReporter for RecordingReporter {
    fn report(&self, event: &ProgressEvent) -> Result<(), qubit_progress::ProgressReportError> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
        Ok(())
    }
}

#[derive(Debug)]
struct PanickingReporter;

impl ProgressReporter for PanickingReporter {
    fn report(&self, _event: &ProgressEvent) -> Result<(), qubit_progress::ProgressReportError> {
        panic!("progress reporter panic");
    }
}

fn schema() -> ProgressSchema {
    ProgressSchema::single("entries", "Entries")
}

#[test]
fn test_running_progress_guard_reports_zero_interval_running_points() {
    let reporter = RecordingReporter::default();
    let completed_count = Arc::new(AtomicUsize::new(0));

    thread::scope(|scope| {
        let loop_completed_count = Arc::clone(&completed_count);
        let progress = Progress::new(&reporter, Duration::ZERO, schema());
        let running_progress: RunningProgressGuard<'_> =
            progress.spawn_running_reporter(scope, move || {
                vec![
                    ProgressCounter::new("entries")
                        .total(2)
                        .completed(loop_completed_count.load(Ordering::Acquire) as u64),
                ]
            });
        let progress_point_handle: RunningProgressPointHandle = running_progress.point_handle();

        completed_count.store(1, Ordering::Release);
        assert!(progress_point_handle.try_report());
        let deadline = Instant::now() + Duration::from_secs(1);
        while reporter.events().is_empty() && Instant::now() < deadline {
            thread::yield_now();
        }
        running_progress
            .stop_and_join()
            .expect("progress reporter should stop cleanly");
    });

    let events = reporter.events();
    assert!(events.iter().any(|event| {
        event.phase() == ProgressPhase::Running
            && event
                .counter("entries")
                .map(ProgressCounter::completed_count)
                == Some(1)
    }));
}

#[test]
fn test_disabled_reporter_does_not_evaluate_running_snapshot() {
    let reporter = NoOpProgressReporter;
    let (snapshot_sender, snapshot_receiver) = mpsc::sync_channel(1);

    thread::scope(|scope| {
        let progress = Progress::new(&reporter, Duration::ZERO, schema());
        let running_progress = progress.spawn_running_reporter(scope, || {
            snapshot_sender
                .send(())
                .expect("test should observe snapshot evaluation");
            vec![ProgressCounter::new("entries").total(1)]
        });
        let point = running_progress.point_handle();

        point.report();
        assert!(!running_progress.status().is_failed());
        assert_eq!(
            snapshot_receiver.recv_timeout(Duration::from_millis(100)),
            Err(RecvTimeoutError::Timeout),
        );
        running_progress
            .stop_and_join()
            .expect("progress reporter should stop cleanly");
    });
}

#[test]
fn test_running_progress_guard_stop_and_join_propagates_reporter_panic() {
    let reporter = PanickingReporter;
    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        thread::scope(|scope| {
            let progress = Progress::new(&reporter, Duration::ZERO, schema());
            let running_progress = progress
                .spawn_running_reporter(scope, || vec![ProgressCounter::new("entries").total(1)]);
            let status = running_progress.status();
            let progress_point_handle = running_progress.point_handle();

            assert!(progress_point_handle.try_report());
            let deadline = Instant::now() + Duration::from_secs(1);
            while progress_point_handle.try_report() && Instant::now() < deadline {
                thread::yield_now();
            }
            while !status.is_failed() && Instant::now() < deadline {
                thread::yield_now();
            }
            assert!(status.is_failed());
            running_progress
                .stop_and_join()
                .expect("progress reporter should stop cleanly");
        });
    }));

    assert!(panic_result.is_err());
}

#[test]
fn test_running_progress_guard_stop_and_join_returns_reporter_error() {
    let reporter = WriterProgressReporter::from_writer(FailingWriter);

    thread::scope(|scope| {
        let progress = Progress::new(&reporter, Duration::ZERO, schema());
        let running_progress = progress
            .spawn_running_reporter(scope, || vec![ProgressCounter::new("entries").total(1)]);
        let point = running_progress.point_handle();
        point.report();

        let deadline = Instant::now() + Duration::from_secs(1);
        while point.try_report() && Instant::now() < deadline {
            thread::yield_now();
        }

        assert!(matches!(
            running_progress.stop_and_join(),
            Err(ProgressReportError::Io(_)),
        ));
    });
}
