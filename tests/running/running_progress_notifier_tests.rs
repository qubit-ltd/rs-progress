/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for running progress notification through public point handles.

use std::{
    sync::Mutex,
    thread,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressCounters,
    ProgressEvent,
    ProgressReporter,
};

#[derive(Debug, Default)]
struct RecordingReporter {
    events: Mutex<Vec<ProgressEvent>>,
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
fn test_running_progress_point_handle_reports_send_failure_after_stop() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(&reporter, Duration::ZERO);

    thread::scope(|scope| {
        let running_progress =
            progress.spawn_running_reporter(scope, || ProgressCounters::new(Some(1)));
        let point_handle = running_progress.point_handle();

        assert!(point_handle.report());
        running_progress.stop_and_join();
        assert!(!point_handle.report());
    });
}
