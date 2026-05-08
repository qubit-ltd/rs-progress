/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `RunningProgressPoints`.

use std::{
    sync::Mutex,
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
fn test_running_progress_points_are_noop_for_positive_interval() {
    let reporter = RecordingReporter::default();

    thread::scope(|scope| {
        let started_at = Instant::now() - Duration::from_millis(10);
        let progress = Progress::from_start(&reporter, Duration::from_millis(5), started_at);
        let running_progress = RunningProgressLoop::spawn_scoped(scope, progress, || {
            ProgressCounters::new(Some(1)).with_active_count(1)
        });
        let progress_points = running_progress.points();

        assert!(progress_points.running_point());
        thread::sleep(Duration::from_millis(20));
        running_progress.stop_and_join();
        assert!(progress_points.running_point());
    });

    let events = reporter.events();
    assert!(events.iter().any(
        |event| event.phase() == ProgressPhase::Running && event.counters().active_count() == 1
    ));
}
