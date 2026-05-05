/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
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
        ProgressCounters,
        ProgressEvent,
        ProgressStage,
    },
    reporter::{
        ProgressReporter,
        WriterProgressReporter,
    },
};

#[test]
fn test_writer_progress_reporter_writes_human_readable_event() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());
    let event = ProgressEvent::running(
        ProgressCounters::new(Some(4))
            .with_active_count(1)
            .with_completed_count(2),
        Duration::from_millis(1_500),
    )
    .with_stage(ProgressStage::new("install", "Install package"));

    reporter.report(&event);

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("running"));
    assert!(text.contains("Install package"));
    assert!(text.contains("2/4"));
    assert!(text.contains("50.00%"));
}

#[test]
fn test_writer_progress_reporter_handles_unknown_total_output() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());

    assert!(Arc::ptr_eq(reporter.writer(), &output));
    reporter.report(&ProgressEvent::running(
        ProgressCounters::new(None).with_completed_count(7),
        Duration::from_millis(0),
    ));
    reporter.report(&ProgressEvent::finished(
        ProgressCounters::new(Some(7)).with_completed_count(7),
        Duration::from_secs(61),
    ));

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("7 completed"));
    assert!(text.contains("running"));
    assert!(text.contains("finished"));
}

#[test]
fn test_writer_progress_reporter_supports_owned_writer() {
    let owned_reporter = WriterProgressReporter::from_writer(Cursor::new(Vec::new()));
    owned_reporter.report(&ProgressEvent::canceled(
        ProgressCounters::new(Some(1)),
        Duration::from_millis(5),
    ));
}
