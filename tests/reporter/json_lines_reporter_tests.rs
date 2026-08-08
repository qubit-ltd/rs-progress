// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! JSON Lines reporter behavior.

#[cfg(feature = "json-lines")]
use qubit_progress::JsonLinesReporter;
#[cfg(feature = "json-lines")]
use qubit_progress::Metric;
#[cfg(feature = "json-lines")]
use qubit_progress::Progress;

/// Verifies that JSON Lines exposes the cancelled metric count.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_serializes_cancelled_count() {
    let reporter = JsonLinesReporter::new(Vec::new());
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
            .expect("JSON Lines reporter writer mutex must not be poisoned"),
    )
    .expect("JSON Lines output must be UTF-8");
    assert!(output.contains("\"cancelled\":1"));
}
