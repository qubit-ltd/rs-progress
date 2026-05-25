/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for internal running progress notifier behavior through public APIs.

use std::{
    sync::Mutex,
    thread,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
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
fn test_running_progress_notifier_stop_is_idempotent_through_guard() {
    let reporter = RecordingReporter::default();
    let progress = Progress::new(&reporter, Duration::ZERO, ProgressSchema::single("entries", "Entries"));

    thread::scope(|scope| {
        let running_progress =
            progress.spawn_running_reporter(scope, || vec![ProgressCounter::new("entries").total(1)]);
        running_progress.stop_and_join();
    });
}
