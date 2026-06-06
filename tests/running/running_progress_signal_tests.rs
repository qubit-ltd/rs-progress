// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for running progress signal behavior through public APIs.

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
fn test_running_progress_signal_running_point_reports_for_zero_interval() {
    let reporter = RecordingReporter::default();
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
        assert!(point.report());
        running_progress.stop_and_join();
    });

    assert!(
        reporter
            .events()
            .iter()
            .any(|event| event.phase() == ProgressPhase::Running)
    );
}

#[test]
fn test_running_progress_signal_stop_prevents_further_zero_interval_points() {
    let reporter = RecordingReporter::default();
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
        running_progress.stop_and_join();
        assert!(!point.report());
    });
}
