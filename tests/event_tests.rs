// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Event behavior for internally owned metric snapshots.

use std::sync::Mutex;

use qubit_progress::{
    Event,
    Metric,
    Phase,
    Progress,
    ReportError,
    Reporter,
};

/// Stores delivered events for one test operation.
#[derive(Default)]
struct RecordingReporter {
    /// Events captured from the reporter callback.
    events: Mutex<Vec<Event>>,
}

impl Reporter for RecordingReporter {
    /// Stores each complete immutable event.
    fn report(&self, event: &Event) -> Result<(), ReportError> {
        self.events
            .lock()
            .expect("recording reporter mutex must not be poisoned")
            .push(event.clone());
        Ok(())
    }
}

/// Verifies that events expose the cancelled terminal count from metric state.
#[test]
fn test_event_carries_cancelled_metric_count() {
    let reporter = RecordingReporter::default();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(2).expect("work must start");
    tasks.cancel(2).expect("work must cancel");
    progress.finish().expect("progress must finish");

    let events = reporter
        .events
        .lock()
        .expect("recording reporter mutex must not be poisoned");
    let terminal = events.last().expect("terminal event must exist");
    assert_eq!(terminal.phase(), Phase::Succeeded);
    assert_eq!(
        terminal
            .metric("tasks")
            .expect("metric must exist")
            .cancelled(),
        2,
    );
}
