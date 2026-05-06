/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `StdoutProgressReporter`.

use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    reporter::{
        ProgressReporter,
        StdoutProgressReporter,
    },
};

#[test]
fn test_stdout_progress_reporter_can_report() {
    let reporter = StdoutProgressReporter::new();
    reporter.report(&ProgressEvent::running(
        ProgressCounters::new(Some(2)).with_completed_count(1),
        Duration::from_millis(10),
    ));
}

#[test]
fn test_stdout_progress_reporter_default_can_report() {
    let reporter = StdoutProgressReporter::default();
    reporter.report(&ProgressEvent::finished(
        ProgressCounters::new(Some(2)).with_completed_count(2),
        Duration::from_millis(10),
    ));
}
