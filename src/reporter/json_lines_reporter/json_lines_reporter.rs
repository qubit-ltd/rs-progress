// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! JSON Lines reporter implementation.

use std::io::Write;
use std::sync::Mutex;
use std::sync::PoisonError;

use crate::Event;
use crate::Reporter;
use crate::ReporterError;

/// Writes one complete event as one JSON line.
///
/// Available with the `json-lines` feature.
///
/// A successful report means the complete newline-terminated record was
/// accepted by the [`Write`] interface; it does not flush the writer. A write
/// error may leave a prefix of the record in the destination, without rollback
/// or repair of later line boundaries. After such an error, replace or reopen
/// the destination, or recover its format-specific position before continuing;
/// do not blindly retry the event. Flushing does not guarantee durable storage.
///
/// # Examples
///
/// ```
/// use std::io::BufWriter;
/// use std::io::Write;
/// use qubit_progress::{JsonLinesReporter, Metric, Progress};
///
/// let reporter = JsonLinesReporter::new(BufWriter::new(Vec::<u8>::new()));
/// let mut progress = Progress::builder(&reporter)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// let tasks = progress.metric("tasks").expect("configured metric must exist");
/// tasks.start(1)?;
/// tasks.succeed(1)?;
/// progress.report()?;
/// progress.finish()?;
/// let mut writer = reporter.into_inner()?;
/// writer.flush()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Type Parameters
///
/// * `W` — writer that receives each serialized event record.
pub struct JsonLinesReporter<W> {
    /// Writer serialized across concurrent report calls.
    writer: Mutex<W>,
}

impl<W> JsonLinesReporter<W> {
    /// Creates a JSON Lines reporter that owns `writer`.
    ///
    /// # Parameters
    ///
    /// * `writer` — destination for serialized event records.
    #[must_use]
    #[inline]
    pub const fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    /// Consumes the reporter and returns its writer without flushing it.
    ///
    /// # Returns
    ///
    /// Returns the writer when no reporting thread panicked while holding the
    /// mutex.
    ///
    /// # Errors
    ///
    /// Returns the writer in [`PoisonError`] when a reporting thread panicked
    /// while holding the mutex.
    /// Flush the returned writer explicitly and handle its error. A successful
    /// flush only passes buffered data to the underlying writer; it does not
    /// guarantee durable storage.
    #[inline]
    pub fn into_inner(self) -> Result<W, PoisonError<W>> {
        self.writer.into_inner()
    }
}

impl<W> Reporter for JsonLinesReporter<W>
where
    W: Write + Send,
{
    /// Serializes one complete event and writes one newline-delimited record.
    ///
    /// # Parameters
    ///
    /// * `event` — complete event to serialize and write.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or writing fails, or if a reporting
    /// thread previously panicked while holding the writer mutex. A failed
    /// `write_all` may have already written a prefix; that prefix is not rolled
    /// back, and later reports do not restore line boundaries.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        let mut encoded = serde_json::to_vec(event).map_err(ReporterError::new)?;
        encoded.push(b'\n');
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| ReporterError::message("JSON Lines reporter mutex is poisoned"))?;
        writer.write_all(&encoded).map_err(ReporterError::new)
    }
}
