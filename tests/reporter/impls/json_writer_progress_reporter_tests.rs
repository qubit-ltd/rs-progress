// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `JsonWriterProgressReporter`.

use std::{
    io::Cursor,
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_progress::{
    JsonWriterProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

#[test]
fn test_json_writer_progress_reporter_writes_one_json_line_per_metric() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = JsonWriterProgressReporter::new(output.clone());

    assert!(Arc::ptr_eq(reporter.writer(), &output));
    reporter.report(&ProgressEvent::running(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(4).completed(2)],
        Duration::from_millis(110),
    ));

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("JSON output should be UTF-8");
    let lines = text.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);

    let value: serde_json::Value =
        serde_json::from_str(lines[0]).expect("JSON should parse");
    assert_eq!(value["metric"]["id"], "entries");
    assert_eq!(value["metric"]["name"], "Entries");
    assert_eq!(value["phase"], "running");
    assert_eq!(value["completed_count"], 2);
    assert_eq!(value["elapsed"], "110ms");
}

#[test]
fn test_json_writer_progress_reporter_supports_owned_writer() {
    let reporter =
        JsonWriterProgressReporter::from_writer(Cursor::new(Vec::new()));
    reporter.report(&ProgressEvent::finished(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(1).completed(1)],
        Duration::from_millis(5),
    ));
}
