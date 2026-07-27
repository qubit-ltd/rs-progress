// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crate::{
    model::ProgressEvent,
    reporter::{
        HumanReadableMetricSnapshotFormatter, MetricSnapshotFormatter, ProgressReportError,
        ProgressReporter,
    },
};

/// Progress reporter that writes human-readable metric snapshots to a writer.
///
/// One input event can produce multiple output lines: one line for each metric
/// counter carried by the event.
pub struct WriterProgressReporter<W> {
    /// Shared writer receiving progress lines.
    writer: Arc<Mutex<W>>,
}

impl<W> WriterProgressReporter<W> {
    /// Returns the shared writer used by this reporter.
    ///
    /// # Returns
    ///
    /// A shared reference to the writer mutex.
    #[inline]
    pub const fn writer(&self) -> &Arc<Mutex<W>> {
        &self.writer
    }
}

impl<W> WriterProgressReporter<W>
where
    W: Write + Send,
{
    /// Creates a reporter from a shared writer.
    ///
    /// # Parameters
    ///
    /// * `writer` - Shared writer receiving progress output.
    ///
    /// # Returns
    ///
    /// A writer-backed progress reporter.
    pub fn new(writer: Arc<Mutex<W>>) -> Self {
        Self { writer }
    }

    /// Creates a reporter from an owned writer.
    ///
    /// # Parameters
    ///
    /// * `writer` - Owned writer receiving progress output.
    ///
    /// # Returns
    ///
    /// A writer-backed progress reporter.
    #[inline]
    pub fn from_writer(writer: W) -> Self {
        Self::new(Arc::new(Mutex::new(writer)))
    }
}

impl<W> ProgressReporter for WriterProgressReporter<W>
where
    W: Write + Send,
{
    /// Writes one line for every metric snapshot in the event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to format and write.
    ///
    /// # Errors
    ///
    /// Returns [`ProgressReportError::Io`] when writing a formatted line
    /// fails. A poisoned writer mutex is recovered.
    fn report(&self, event: &ProgressEvent) -> Result<(), ProgressReportError> {
        let formatter = HumanReadableMetricSnapshotFormatter::new();
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for snapshot in event.metric_snapshots_iter() {
            writeln!(writer, "{}", formatter.format(&snapshot))?;
        }
        Ok(())
    }
}
