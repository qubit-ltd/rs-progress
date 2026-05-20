/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::{
    io::Write,
    sync::{
        Arc,
        Mutex,
    },
};

use qubit_function::ArcConsumer;

use super::json_progress_reporter::JsonProgressReporter;
use crate::{
    model::ProgressEvent,
    reporter::ProgressReporter,
};

/// Progress reporter that writes JSON metric snapshots to a writer.
///
/// One input event can produce multiple JSON lines: one line for each metric
/// counter carried by the event.
pub struct JsonWriterProgressReporter<W> {
    /// Shared writer receiving JSON lines.
    writer: Arc<Mutex<W>>,
    /// JSON reporter that consumes formatted strings.
    inner: JsonProgressReporter,
}

impl<W> JsonWriterProgressReporter<W> {
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

impl<W> JsonWriterProgressReporter<W>
where
    W: Write + Send + 'static,
{
    /// Creates a reporter from a shared writer.
    ///
    /// # Parameters
    ///
    /// * `writer` - Shared writer receiving JSON progress output.
    ///
    /// # Returns
    ///
    /// A JSON writer-backed progress reporter.
    pub fn new(writer: Arc<Mutex<W>>) -> Self {
        let consumer_writer = Arc::clone(&writer);
        let consumer = ArcConsumer::new(move |line: &String| {
            let mut writer = consumer_writer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            writeln!(writer, "{line}").expect("JSON progress reporter should write event");
        });
        Self {
            writer,
            inner: JsonProgressReporter::new(consumer),
        }
    }

    /// Creates a reporter from an owned writer.
    ///
    /// # Parameters
    ///
    /// * `writer` - Owned writer receiving JSON progress output.
    ///
    /// # Returns
    ///
    /// A JSON writer-backed progress reporter.
    #[inline]
    pub fn from_writer(writer: W) -> Self {
        Self::new(Arc::new(Mutex::new(writer)))
    }
}

impl<W> ProgressReporter for JsonWriterProgressReporter<W>
where
    W: Write + Send + 'static,
{
    /// Writes one JSON line for every metric snapshot in the event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to format and write.
    ///
    /// # Panics
    ///
    /// Recovers the inner writer when the writer mutex is poisoned, and panics
    /// only when writing to the configured writer fails.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
