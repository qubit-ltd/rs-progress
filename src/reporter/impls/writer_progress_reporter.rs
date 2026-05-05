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

use super::format::format_duration;
use crate::{
    model::ProgressEvent,
    reporter::ProgressReporter,
};

/// Progress reporter that writes human-readable events to a writer.
///
/// # Type Parameters
///
/// * `W` - Writer receiving formatted progress events.
#[derive(Debug)]
pub struct WriterProgressReporter<W> {
    /// Shared writer receiving progress lines.
    writer: Arc<Mutex<W>>,
}

impl<W> WriterProgressReporter<W> {
    /// Creates a reporter from a shared writer.
    ///
    /// # Parameters
    ///
    /// * `writer` - Shared writer receiving progress output.
    ///
    /// # Returns
    ///
    /// A writer-backed progress reporter.
    #[inline]
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

impl<W> ProgressReporter for WriterProgressReporter<W>
where
    W: Write + Send,
{
    /// Writes one progress event as a single human-readable line.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to format and write.
    ///
    /// # Panics
    ///
    /// Panics when the writer mutex is poisoned or writing to the configured
    /// writer fails.
    fn report(&self, event: &ProgressEvent) {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        writeln!(writer, "{}", format_event(event)).expect("progress reporter should write event");
    }
}

/// Formats one progress event.
///
/// # Parameters
///
/// * `event` - Event to format.
///
/// # Returns
///
/// A compact human-readable line.
fn format_event(event: &ProgressEvent) -> String {
    let counters = event.counters();
    let progress = match (counters.completed_count(), counters.total_count()) {
        (completed, Some(total)) => format!(
            "{completed}/{total} ({:.2}%)",
            counters.progress_percent().unwrap_or(100.0)
        ),
        (completed, None) => format!("{completed} completed"),
    };
    let active = counters.active_count();
    let failed = counters.failed_count();
    let elapsed = format_duration(event.elapsed());
    match event.stage() {
        Some(stage) => format!(
            "{} [{}] {progress}, active {active}, failed {failed}, elapsed {elapsed}",
            event.phase(),
            stage.name(),
        ),
        None => format!(
            "{} {progress}, active {active}, failed {failed}, elapsed {elapsed}",
            event.phase(),
        ),
    }
}
