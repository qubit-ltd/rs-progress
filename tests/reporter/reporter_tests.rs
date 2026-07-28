// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter output and event serialization tests.

use qubit_progress::{
    Metric,
    Progress,
    TextReporter,
};

#[cfg(feature = "log")]
use qubit_progress::Reporter;

/// Verifies that the text sink emits one complete event record per delivery.
#[test]
fn test_text_reporter_writes_one_complete_line_per_event() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    drop(progress);
    let bytes = reporter
        .into_inner()
        .expect("text reporter writer mutex must not be poisoned");
    let output = String::from_utf8(bytes).expect("text output must be UTF-8");
    assert!(output.contains("phase=started"));
    assert!(output.contains("total=Some(1)"));
}

/// Verifies that the optional JSON Lines sink preserves one complete event.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_serializes_complete_event() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    drop(progress);
    let bytes = reporter
        .into_inner()
        .expect("JSON Lines reporter writer mutex must not be poisoned");
    let output = String::from_utf8(bytes).expect("JSON output must be UTF-8");
    assert!(output.contains("\"phase\":\"started\""));
    assert!(output.contains("\"total\":1"));
}

/// Verifies that the log reporter samples the facade's info-level enablement.
#[cfg(feature = "log")]
#[test]
fn test_log_reporter_matches_info_level_enablement() {
    use qubit_progress::LogReporter;

    let reporter = LogReporter;
    assert_eq!(
        reporter.is_enabled(),
        log::log_enabled!(log::Level::Info),
        "log reporting must not start operations when info output is disabled",
    );
}
