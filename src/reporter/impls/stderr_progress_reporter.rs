// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{
    model::ProgressEvent,
    reporter::{
        ProgressReporter,
        WriterProgressReporter,
    },
};

/// Progress reporter that writes human-readable metric snapshots to stderr.
pub struct StderrProgressReporter {
    /// Writer-backed reporter targeting standard error.
    inner: WriterProgressReporter<std::io::Stderr>,
}

impl StderrProgressReporter {
    /// Creates a reporter writing to standard error.
    ///
    /// # Returns
    ///
    /// A stderr progress reporter.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: WriterProgressReporter::from_writer(std::io::stderr()),
        }
    }
}

impl Default for StderrProgressReporter {
    /// Creates a default stderr progress reporter.
    ///
    /// # Returns
    ///
    /// A stderr progress reporter.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for StderrProgressReporter {
    /// Writes one line for every metric snapshot in the event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to report.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
