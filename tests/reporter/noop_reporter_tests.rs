// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! No-op reporter behavior.

use qubit_progress::{Metric, NoopReporter, Progress};

/// Verifies that the no-op reporter disables delivery but keeps metric state.
#[test]
fn test_noop_reporter_preserves_metric_state_without_events() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(1).expect("work must start");
    tasks.succeed(1).expect("work must succeed");
    progress.finish().expect("disabled progress must finish");
    assert_eq!(tasks.snapshot().completed(), 1,);
}
