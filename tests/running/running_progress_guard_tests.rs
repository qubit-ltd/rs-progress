/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `RunningProgressGuard`.

use std::{
    panic::{
        AssertUnwindSafe,
        catch_unwind,
    },
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicUsize,
            Ordering,
        },
    },
    thread,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
    RunningProgressGuard,
    RunningProgressPointHandle,
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

#[derive(Debug)]
struct PanickingReporter;

impl ProgressReporter for PanickingReporter {
    fn report(&self, _event: &ProgressEvent) {
        panic!("progress reporter panic");
    }
}

#[test]
fn test_running_progress_guard_reports_zero_interval_running_points() {
    let reporter = RecordingReporter::default();
    let completed_count = Arc::new(AtomicUsize::new(0));

    thread::scope(|scope| {
        let loop_completed_count = Arc::clone(&completed_count);
        let progress = Progress::new(&reporter, Duration::ZERO);
        let running_progress: RunningProgressGuard<'_> =
            progress.spawn_running_reporter(scope, move || {
                ProgressCounters::new(Some(2))
                    .with_completed_count(loop_completed_count.load(Ordering::Acquire))
            });
        let progress_point_handle: RunningProgressPointHandle = running_progress.point_handle();

        completed_count.store(1, Ordering::Release);
        assert!(progress_point_handle.report());
        running_progress.stop_and_join();
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
fn test_running_progress_guard_stop_and_join_propagates_reporter_panic() {
    let reporter = PanickingReporter;
    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        thread::scope(|scope| {
            let progress = Progress::new(&reporter, Duration::ZERO);
            let running_progress =
                progress.spawn_running_reporter(scope, || ProgressCounters::new(Some(1)));
            let progress_point_handle = running_progress.point_handle();

            assert!(progress_point_handle.report());
            running_progress.stop_and_join();
        });
    }));

    assert!(panic_result.is_err());
}
