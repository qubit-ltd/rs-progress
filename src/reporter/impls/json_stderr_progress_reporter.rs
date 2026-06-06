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
        JsonWriterProgressReporter,
        ProgressReporter,
    },
};

/// Progress reporter that writes JSON metric snapshots to stderr.
pub struct JsonStderrProgressReporter {
    /// JSON writer-backed reporter targeting standard error.
    inner: JsonWriterProgressReporter<std::io::Stderr>,
}

impl JsonStderrProgressReporter {
    /// Creates a JSON reporter writing to standard error.
    ///
    /// # Returns
    ///
    /// A JSON stderr progress reporter.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: JsonWriterProgressReporter::from_writer(std::io::stderr()),
        }
    }
}

impl Default for JsonStderrProgressReporter {
    /// Creates a default JSON stderr progress reporter.
    ///
    /// # Returns
    ///
    /// A JSON stderr progress reporter.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for JsonStderrProgressReporter {
    /// Writes one JSON line for every metric snapshot in the event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to report.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
