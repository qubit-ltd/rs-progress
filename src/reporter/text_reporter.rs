// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Line-oriented human-readable event reporter.

use std::{
    fmt::Write as _,
    io::Write,
    sync::{
        Mutex,
        PoisonError,
    },
};

use crate::{
    Event,
    Reporter,
    ReporterError,
};

/// Writes one complete human-readable record for each reported event.
pub struct TextReporter<W> {
    /// Writer serialized across concurrent reporter calls.
    writer: Mutex<W>,
}

impl<W> TextReporter<W> {
    /// Creates a text reporter that owns `writer`.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    /// Consumes the reporter and returns its writer.
    ///
    /// Returns the contained writer in the error when a reporting thread
    /// panicked while holding the mutex.
    pub fn into_inner(self) -> Result<W, PoisonError<W>> {
        self.writer.into_inner()
    }
}

impl<W> Reporter for TextReporter<W>
where
    W: Write + Send,
{
    /// Formats and writes one complete event line under the writer lock.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        let mut line = format_event(event);
        line.push('\n');
        let mut writer = self.writer.lock().map_err(|_| {
            ReporterError::message("text reporter mutex is poisoned")
        })?;
        writer
            .write_all(line.as_bytes())
            .map_err(ReporterError::new)
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
        let _ = write!(
            line,
            " attribute={}({})",
            key.escape_default(),
            value.escape_default(),
        );
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
