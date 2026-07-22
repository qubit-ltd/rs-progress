// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `WriterProgressReporter`.

use std::{
    io::Cursor,
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_progress::{
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
        ProgressStage,
    },
    reporter::{
        ProgressReportError,
        ProgressReporter,
        WriterProgressReporter,
    },
};

use crate::support::FailingWriter;

fn schema() -> ProgressSchema {
    ProgressSchema::single("entries", "Entries")
}

#[test]
fn test_writer_progress_reporter_writes_human_readable_event() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());
    let event = ProgressEvent::running(
        schema(),
        vec![
            ProgressCounter::new("entries")
                .total(4)
                .active(1)
                .completed(2),
        ],
        Duration::from_millis(1_500),
    )
    .with_stage(ProgressStage::new("install", "Install package"));

    let _ = reporter.report(&event);

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("running"));
    assert!(text.contains("Install package"));
    assert!(text.contains("Entries 2/4"));
    assert!(text.contains("50.00%"));
}

#[test]
fn test_writer_progress_reporter_handles_unknown_total_output() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());

    assert!(Arc::ptr_eq(reporter.writer(), &output));
    let _ = reporter.report(&ProgressEvent::running(
        schema(),
        vec![ProgressCounter::new("entries").completed(7)],
        Duration::from_millis(0),
    ));
    let _ = reporter.report(&ProgressEvent::finished(
        schema(),
        vec![ProgressCounter::new("entries").total(7).completed(7)],
        Duration::from_secs(61),
    ));

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("Entries 7 completed"));
    assert!(text.contains("running"));
    assert!(text.contains("finished"));
}

#[test]
fn test_writer_progress_reporter_handles_empty_event_output() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());

    let _ = reporter.report(&ProgressEvent::running(
        schema(),
        Vec::new(),
        Duration::from_millis(1),
    ));

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.is_empty());
}

#[test]
fn test_writer_progress_reporter_supports_owned_writer() {
    let owned_reporter =
        WriterProgressReporter::from_writer(Cursor::new(Vec::new()));
    let _ = owned_reporter.report(&ProgressEvent::canceled(
        schema(),
        vec![ProgressCounter::new("entries").total(1)],
        Duration::from_millis(5),
    ));
}

#[test]
fn test_writer_progress_reporter_returns_output_errors() {
    let reporter = WriterProgressReporter::from_writer(FailingWriter);
    let result = reporter.report(&ProgressEvent::running(
        schema(),
        vec![ProgressCounter::new("entries").completed(1)],
        Duration::ZERO,
    ));

    assert!(matches!(result, Err(ProgressReportError::Io(_))));
}
