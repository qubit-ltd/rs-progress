// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the running progress loop through the public `Progress` API.

use std::{
    sync::{
        Mutex,
        mpsc::{
            self,
            Receiver,
            SyncSender,
        },
    },
    thread,
    time::{
        Duration,
        Instant,
    },
};

use qubit_progress::{
    Progress,
    ProgressCounter,
    ProgressEvent,
    ProgressEventBuildError,
    ProgressPhase,
    ProgressReportError,
    ProgressReporter,
    ProgressSchema,
};

#[derive(Debug, Default)]
struct RecordingReporter {
    events: Mutex<Vec<ProgressEvent>>,
    entered: Option<SyncSender<()>>,
    release: Option<Mutex<Receiver<()>>>,
}

impl RecordingReporter {
    fn blocking() -> (Self, Receiver<()>, SyncSender<()>) {
        let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        (
            Self {
                events: Mutex::default(),
                entered: Some(entered_sender),
                release: Some(Mutex::new(release_receiver)),
            },
            entered_receiver,
            release_sender,
        )
    }

    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProgressReporter for RecordingReporter {
    fn report(
        &self,
        event: &ProgressEvent,
    ) -> Result<(), qubit_progress::ProgressReportError> {
        let is_first = {
            let mut events = self
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            events.push(event.clone());
            events.len() == 1
        };
        if is_first {
            if let Some(entered) = &self.entered {
                entered.send(()).expect("test should observe first report");
            }
            if let Some(release) = &self.release {
                release
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .recv()
                    .expect("test should release first report");
            }
        }
        Ok(())
    }
}

#[test]
fn test_running_progress_loop_coalesces_points_and_prioritizes_stop() {
    let (reporter, entered, release) = RecordingReporter::blocking();
    let progress = Progress::new(
        &reporter,
        Duration::ZERO,
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1).active(1)]
        });
        let point = running_progress.point_handle();

        point.report();
        entered.recv().expect("reporter should enter first report");
        for _ in 0..10_000 {
            point.report();
        }

        let stopper = scope.spawn(move || running_progress.stop_and_join());
        while point.try_report() {
            thread::yield_now();
        }
        release.send(()).expect("reporter should still be waiting");
        stopper
            .join()
            .expect("stopper should not panic")
            .expect("progress reporter should stop cleanly");
    });

    assert!(reporter.events().len() <= 2);
}

#[test]
fn test_running_progress_loop_reports_positive_interval_timeouts() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(
        &reporter,
        Duration::from_millis(5),
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1).active(1)]
        });

        thread::sleep(Duration::from_millis(20));
        running_progress
            .stop_and_join()
            .expect("progress reporter should stop cleanly");
    });

    let events = reporter.events();
    assert!(
        events
            .iter()
            .any(|event| event.phase() == ProgressPhase::Running
                && event.counter("entries").map(ProgressCounter::active_count)
                    == Some(1))
    );
}

#[test]
fn test_running_progress_loop_exits_when_all_notifiers_are_dropped() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(
        &reporter,
        Duration::ZERO,
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1)]
        });
        drop(running_progress);
    });

    assert!(reporter.events().is_empty());
}

#[test]
fn test_running_progress_loop_exits_when_positive_interval_guard_is_dropped() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(
        &reporter,
        Duration::from_secs(1),
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1)]
        });
        drop(running_progress);
    });

    assert!(reporter.events().is_empty());
}

#[test]
fn test_running_progress_loop_propagates_snapshot_build_errors() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(
        &reporter,
        Duration::ZERO,
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("missing").total(1)]
        });
        let status = running_progress.status();
        let point = running_progress.point_handle();

        assert!(point.try_report());
        let deadline = Instant::now() + Duration::from_secs(1);
        while !status.is_failed() && Instant::now() < deadline {
            thread::yield_now();
        }

        assert!(status.is_failed());
        assert!(matches!(
            running_progress.stop_and_join(),
            Err(ProgressReportError::EventBuild(
                ProgressEventBuildError::UnknownMetricId { metric_id },
            )) if metric_id == "missing",
        ));
    });

    assert!(reporter.events().is_empty());
}
