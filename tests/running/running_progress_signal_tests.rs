/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for running progress stop signals through the public guard API.

use std::{
    sync::Mutex,
    thread,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
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
fn test_running_progress_guard_stop_signal_stops_loop() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(&reporter, Duration::ZERO);

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            ProgressCounters::new(Some(1)).with_active_count(1)
        });

        running_progress.stop_and_join();
    });

    let events = reporter.events();
    assert!(
        events
            .iter()
            .all(|event| event.phase() != ProgressPhase::Running)
    );
}

#[test]
fn test_running_progress_guard_drop_disconnects_loop() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(&reporter, Duration::ZERO);

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            ProgressCounters::new(Some(1)).with_active_count(1)
        });

        drop(running_progress);
    });

    let events = reporter.events();
    assert!(
        events
            .iter()
            .all(|event| event.phase() != ProgressPhase::Running)
    );
}
