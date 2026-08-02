// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! JSON Lines reporter for complete events.

use std::{
    io::Write,
    sync::{Mutex, PoisonError},
};

use crate::{Event, Reporter, ReporterError};

/// Writes one complete event as one JSON line.
pub struct JsonLinesReporter<W> {
    /// Writer serialized across concurrent report calls.
    writer: Mutex<W>,
}

impl<W> JsonLinesReporter<W> {
    /// Creates a JSON Lines reporter that owns `writer`.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    /// Consumes the reporter and returns its writer.
    ///
    /// Returns the writer in the error when a reporting thread panicked while
    /// holding the mutex.
    pub fn into_inner(self) -> Result<W, PoisonError<W>> {
        self.writer.into_inner()
    }
}

impl<W> Reporter for JsonLinesReporter<W>
where
    W: Write + Send,
{
    /// Serializes one complete event and writes one newline-delimited record.
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
