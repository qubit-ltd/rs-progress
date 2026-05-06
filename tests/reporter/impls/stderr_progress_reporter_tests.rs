/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `StderrProgressReporter`.

use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    reporter::{
        ProgressReporter,
        StderrProgressReporter,
    },
};

#[test]
fn test_stderr_progress_reporter_default_can_report() {
    let reporter = StderrProgressReporter::default();
    reporter.report(&ProgressEvent::failed(
        ProgressCounters::new(Some(2)).with_completed_count(1),
        Duration::from_millis(10),
    ));
}
