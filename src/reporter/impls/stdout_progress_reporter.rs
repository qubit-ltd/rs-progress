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
        ProgressReporter,
        WriterProgressReporter,
    },
};

/// Progress reporter that writes human-readable metric snapshots to stdout.
pub struct StdoutProgressReporter {
    /// Writer-backed reporter targeting standard output.
    inner: WriterProgressReporter<std::io::Stdout>,
}

impl StdoutProgressReporter {
    /// Creates a reporter writing to standard output.
    ///
    /// # Returns
    ///
    /// A stdout progress reporter.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: WriterProgressReporter::from_writer(std::io::stdout()),
        }
    }
}

impl Default for StdoutProgressReporter {
    /// Creates a default stdout progress reporter.
    ///
    /// # Returns
    ///
    /// A stdout progress reporter.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for StdoutProgressReporter {
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
