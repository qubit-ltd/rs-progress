/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::{
    model::ProgressEvent,
    reporter::{
        JsonWriterProgressReporter,
        ProgressReporter,
    },
};

/// Progress reporter that writes JSON metric snapshots to stdout.
pub struct JsonStdoutProgressReporter {
    /// JSON writer-backed reporter targeting standard output.
    inner: JsonWriterProgressReporter<std::io::Stdout>,
}

impl JsonStdoutProgressReporter {
    /// Creates a JSON reporter writing to standard output.
    ///
    /// # Returns
    ///
    /// A JSON stdout progress reporter.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: JsonWriterProgressReporter::from_writer(std::io::stdout()),
        }
    }
}

impl Default for JsonStdoutProgressReporter {
    /// Creates a default JSON stdout progress reporter.
    ///
    /// # Returns
    ///
    /// A JSON stdout progress reporter.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for JsonStdoutProgressReporter {
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
