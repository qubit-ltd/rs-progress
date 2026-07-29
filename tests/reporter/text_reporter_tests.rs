// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Text reporter behavior for lifecycle counters.

use qubit_progress::{
    Metric,
    Progress,
    TextReporter,
};

/// Verifies that text output includes the cancelled metric count.
#[test]
fn test_text_reporter_includes_cancelled_count() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    tasks.start(1).expect("work must start");
    tasks.cancel(1).expect("work must cancel");
    progress.finish().expect("progress must finish");

    let output = String::from_utf8(
        reporter
            .into_inner()
            .expect("text reporter writer mutex must not be poisoned"),
    )
    .expect("text output must be UTF-8");
    assert!(output.contains("cancelled=1"));
}
