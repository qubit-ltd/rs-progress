// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `HumanReadableProgressReporter`.

use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_function::ArcConsumer;
use qubit_progress::{
    HumanReadableProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

#[test]
fn test_human_readable_progress_reporter_uses_default_formatter() {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let captured_lines = Arc::clone(&lines);
    let consumer = ArcConsumer::new(move |line: &String| {
        captured_lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(line.clone());
    });
    let reporter = HumanReadableProgressReporter::new(consumer);
    let _ = reporter.inner().formatter();

    reporter.report(&ProgressEvent::running(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(4).completed(2)],
        Duration::from_millis(10),
    ));

    let lines = lines
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("running Entries 2/4 (50.00%)"));
}
