// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Line-oriented human-readable event reporter.

use std::fmt::Write as _;
use std::io::Write;
use std::sync::Mutex;
use std::sync::PoisonError;

use crate::Event;
use crate::Reporter;
use crate::ReporterError;

/// Writes one complete human-readable record for each reported event.
///
/// A successful report means the complete newline-terminated record was
/// accepted by the [`Write`] interface; it does not flush the writer. A write
/// error may leave a prefix of the record in the destination, without rollback
/// or repair of later line boundaries. After such an error, replace or reopen
/// the destination, or recover its format-specific position before continuing;
/// do not blindly retry the event. Flushing does not guarantee durable storage.
///
/// # Type Parameters
///
/// - `W`: The owned writer receiving the formatted event records.
///
/// # Examples
///
/// ```
/// use std::io::BufWriter;
/// use std::io::Write;
/// use qubit_progress::{Metric, Progress, TextReporter};
///
/// let reporter = TextReporter::new(BufWriter::new(Vec::<u8>::new()));
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
pub struct TextReporter<W> {
    /// Writer serialized across concurrent reporter calls.
    writer: Mutex<W>,
}

impl<W> TextReporter<W> {
    /// Creates a text reporter that owns `writer`.
    ///
    /// # Parameters
    ///
    /// - `writer`: The destination for complete newline-terminated event
    ///   records.
    ///
    /// # Returns
    ///
    /// A reporter that serializes writes to the owned destination.
    #[must_use]
    #[inline]
    pub const fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    /// Consumes the reporter and returns its writer without flushing it.
    ///
    /// Returns the contained writer in the error when a reporting thread
    /// panicked while holding the mutex.
    /// Flush the returned writer explicitly and handle its error. A successful
    /// flush only passes buffered data to the underlying writer; it does not
    /// guarantee durable storage.
    ///
    /// # Returns
    ///
    /// The writer when the mutex was not poisoned.
    ///
    /// # Errors
    ///
    /// Returns the writer inside [`PoisonError`] if a reporting thread
    /// panicked while holding the mutex.
    #[inline]
    pub fn into_inner(self) -> Result<W, PoisonError<W>> {
        self.writer.into_inner()
    }
}

impl<W> Reporter for TextReporter<W>
where
    W: Write + Send,
{
    /// Formats and writes one complete event line under the writer lock.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the complete line has been accepted by the writer. This
    /// does not flush the writer.
    ///
    /// # Errors
    ///
    /// Returns a [`ReporterError`] if the writer is poisoned or writing the
    /// line fails. A failed `write_all` may have already written a prefix; that
    /// prefix is not rolled back, and later reports do not restore line
    /// boundaries.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        let mut line = format_event(event);
        line.push('\n');
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| ReporterError::message("text reporter mutex is poisoned"))?;
        writer.write_all(line.as_bytes()).map_err(ReporterError::new)
    }
}

/// Produces one complete human-readable event record with escaped metadata.
fn format_event(event: &Event) -> String {
    let mut line = format!(
        "operation={} sequence={} phase={} elapsed={:?}",
        event.operation_id(),
        event.sequence(),
        event.phase().as_str(),
        event.elapsed(),
    );
    if let Some(stage) = event.stage() {
        let _ = write!(
            line,
            " stage={}({}) position={:?} total={:?}",
            stage.id().escape_default(),
            stage.name().escape_default(),
            stage.position_value(),
            stage.total(),
        );
    }
    for (key, value) in event.attributes().iter() {
        let _ = write!(line, " attribute={}({})", key.escape_default(), value.escape_default(),);
    }
    for metric in event.metrics() {
        let _ = write!(
            line,
            " metric={}({}) total={:?} completed={} active={} succeeded={} failed={} cancelled={}",
            metric.id().escape_default(),
            metric.name().escape_default(),
            metric.total(),
            metric.completed(),
            metric.active(),
            metric.succeeded(),
            metric.failed(),
            metric.cancelled(),
        );
    }
    line
}
