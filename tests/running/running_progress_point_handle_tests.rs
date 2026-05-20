/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `RunningProgressPointHandle`.

use std::{
    sync::Mutex,
    thread,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressCounter,
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
    ProgressSchema,
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
fn test_running_progress_point_handle_is_noop_for_positive_interval() {
    let reporter = RecordingReporter::default();

    thread::scope(|scope| {
        let progress = Progress::new(
            &reporter,
            Duration::from_millis(5),
            ProgressSchema::single("entries", "Entries"),
        );
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1).active(1)]
        });
        let progress_point_handle = running_progress.point_handle();

        assert!(progress_point_handle.report());
        thread::sleep(Duration::from_millis(20));
        running_progress.stop_and_join();
        assert!(progress_point_handle.report());
    });

    let events = reporter.events();
    assert!(
        events
            .iter()
            .any(|event| event.phase() == ProgressPhase::Running
                && event.counter("entries").map(ProgressCounter::active_count) == Some(1))
    );
}
