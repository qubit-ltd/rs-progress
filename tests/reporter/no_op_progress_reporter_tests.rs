// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `NoOpProgressReporter`.

use std::time::Duration;

use qubit_progress::{
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
    },
    reporter::{
        NoOpProgressReporter,
        ProgressReporter,
    },
};

#[test]
fn test_no_op_progress_reporter_accepts_events() {
    let reporter = NoOpProgressReporter;
    let event = ProgressEvent::started(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(1)],
        Duration::ZERO,
    );
    reporter.report(&event);
}
