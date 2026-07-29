// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavioral coverage for progress error values.

use std::{
    error::Error,
    fmt,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use qubit_progress::{
    Event,
    Metric,
    Progress,
    ProgressError,
    ReportError,
    Reporter,
};

/// Error type used to verify that clones preserve the original error object.
#[derive(Debug)]
struct OriginalReporterError;

impl fmt::Display for OriginalReporterError {
    /// Formats the stable test error message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("original reporter error")
    }
}

impl Error for OriginalReporterError {}

/// Reporter that accepts Started and rejects the terminal event.
struct TerminalFailingReporter {
    /// Number of delivery attempts observed by this reporter.
    attempts: AtomicUsize,
}

impl TerminalFailingReporter {
    /// Creates a reporter that fails after accepting Started.
    const fn new() -> Self {
        Self {
            attempts: AtomicUsize::new(0),
        }
    }
}

impl Reporter for TerminalFailingReporter {
    /// Rejects every event after the first Started delivery.
    fn report(&self, _event: &Event) -> Result<(), ReportError> {
        if self.attempts.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReportError::message("terminal delivery failed"))
        }
    }
}

/// Verifies that cloning a reporter error retains its concrete source type.
#[test]
fn test_report_error_clone_preserves_original_source() {
    let original = ReportError::new(OriginalReporterError);
    let cloned = original.clone();

    assert!(
        cloned
            .source_error()
            .downcast_ref::<OriginalReporterError>()
            .is_some(),
        "cloned reporter error must retain the original error source",
    );
}

/// Verifies that terminal errors return elapsed time and the original cause.
#[test]
fn test_terminal_error_into_parts_returns_elapsed_and_progress_error() {
    let reporter = TerminalFailingReporter::new();
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    let error = progress
        .finish()
        .expect_err("terminal reporter failure must be retained");
    let expected_elapsed = error.elapsed();
    let (elapsed, progress_error) = error.into_parts();

    assert_eq!(elapsed, expected_elapsed);
    assert!(matches!(progress_error, ProgressError::Report(_)));
}
