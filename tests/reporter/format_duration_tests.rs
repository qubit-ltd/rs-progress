/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Integration tests covering elapsed time formatting output.

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
    },
    reporter::{
        ProgressReporter,
        WriterProgressReporter,
    },
};

fn render_line(elapsed: Duration) -> String {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());
    let event = ProgressEvent::running(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(3).active(1).completed(1)],
        elapsed,
    );
    reporter.report(&event);
    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    String::from_utf8(bytes).expect("writer output should be UTF-8")
}

#[test]
fn test_elapsed_format_handles_sub_second_and_seconds() {
    assert!(render_line(Duration::from_millis(0)).contains("elapsed 0ms"));
    assert!(render_line(Duration::from_millis(42)).contains("elapsed 42ms"));
    assert!(render_line(Duration::from_millis(1_500)).contains("elapsed 1.500s"));
}

#[test]
fn test_elapsed_format_handles_minutes_and_hours() {
    assert!(render_line(Duration::from_secs(61)).contains("elapsed 1m 1s"));
    assert!(render_line(Duration::from_secs(3_661)).contains("elapsed 1h 1m 1s"));
}
