/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the running progress loop through the public `Progress` API.

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
fn test_running_progress_loop_reports_positive_interval_timeouts() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(&reporter, Duration::from_millis(5));

    thread::scope(|scope| {
        let running_progress = progress.spawn_running_reporter(scope, || {
            ProgressCounters::new(Some(1)).with_active_count(1)
        });

        thread::sleep(Duration::from_millis(20));
        running_progress.stop_and_join();
    });

    let events = reporter.events();
    assert!(events.iter().any(
        |event| event.phase() == ProgressPhase::Running && event.counters().active_count() == 1
    ));
}
