/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `RunningProgressLoop`.

use std::{
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicUsize,
            Ordering,
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
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
    RunningProgressLoop,
};

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
    fn report(&self, event: &ProgressEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
    }
}

#[test]
fn test_running_progress_loop_reports_zero_interval_running_points() {
    let reporter = RecordingReporter::default();
    let completed_count = Arc::new(AtomicUsize::new(0));
    let (progress_loop, notifier) = RunningProgressLoop::channel();

    thread::scope(|scope| {
        let loop_completed_count = Arc::clone(&completed_count);
        let reporter_ref = &reporter;
        let handle = scope.spawn(move || {
            let progress = Progress::new(reporter_ref, Duration::ZERO);
            progress_loop.run(progress, || {
                ProgressCounters::new(Some(2))
                    .with_completed_count(loop_completed_count.load(Ordering::Acquire))
            });
        });

        completed_count.store(1, Ordering::Release);
        assert!(notifier.running_point());
        assert!(notifier.stop());
        handle
            .join()
            .expect("running progress loop should stop cleanly");
    });

    let events = reporter.events();
    assert!(
        events
            .iter()
            .any(|event| event.phase() == ProgressPhase::Running
                && event.counters().completed_count() == 1)
    );
}

#[test]
fn test_running_progress_loop_reports_positive_interval_timeouts() {
    let reporter = RecordingReporter::default();
    let (progress_loop, notifier) = RunningProgressLoop::channel();

    thread::scope(|scope| {
        let reporter_ref = &reporter;
        let handle = scope.spawn(move || {
            let started_at = Instant::now() - Duration::from_millis(10);
            let progress = Progress::from_start(reporter_ref, Duration::from_millis(5), started_at);
            progress_loop.run(progress, || {
                ProgressCounters::new(Some(1)).with_active_count(1)
            });
        });

        thread::sleep(Duration::from_millis(20));
        assert!(notifier.stop());
        handle
            .join()
            .expect("running progress loop should stop cleanly");
    });

    let events = reporter.events();
    assert!(events.iter().any(
        |event| event.phase() == ProgressPhase::Running && event.counters().active_count() == 1
    ));
}
