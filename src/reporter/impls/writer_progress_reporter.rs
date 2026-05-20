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
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
    },
    reporter::ProgressReporter,
};

/// Progress reporter that writes human-readable events to a writer.
///
/// # Type Parameters
///
/// * `W` - Writer receiving formatted progress events.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use std::sync::{
///     Arc,
///     Mutex,
/// };
/// use std::time::Duration;
///
/// use qubit_progress::{
///     ProgressCounter,
///     ProgressEvent,
///     ProgressMetric,
///     ProgressReporter,
///     ProgressSchema,
///     WriterProgressReporter,
/// };
///
/// let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
/// let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
/// let reporter = WriterProgressReporter::new(output.clone());
/// reporter.report(&ProgressEvent::running(
///     schema,
///     vec![ProgressCounter::new("entries").total(4).completed(2)],
///     Duration::from_secs(1),
/// ));
///
/// let bytes = output.lock().expect("output should lock").get_ref().clone();
/// let text = String::from_utf8(bytes).expect("progress output should be UTF-8");
/// assert!(text.contains("running"));
/// assert!(text.contains("Entries 2/4"));
/// ```
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
    /// Recovers the inner writer when the writer mutex is poisoned, and panics
    /// only when writing to the configured writer fails.
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
    let counters = format_counters(event.schema(), event.counters());
    let elapsed = format_duration(event.elapsed());
    match event.stage() {
        Some(stage) => format!(
            "{} [{}] {counters}, elapsed {elapsed}",
            event.phase(),
            stage.name(),
        ),
        None => format!("{} {counters}, elapsed {elapsed}", event.phase()),
    }
}

/// Formats all counters in one event.
///
/// # Parameters
///
/// * `schema` - Schema used to resolve metric names.
/// * `counters` - Counters to format.
///
/// # Returns
///
/// A compact counter list.
fn format_counters(schema: &ProgressSchema, counters: &[ProgressCounter]) -> String {
    if counters.is_empty() {
        return "no counters".to_owned();
    }
    counters
        .iter()
        .map(|counter| format_counter(schema, counter))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Formats one counter.
///
/// # Parameters
///
/// * `schema` - Schema used to resolve the metric name.
/// * `counter` - Counter to format.
///
/// # Returns
///
/// A compact counter description.
fn format_counter(schema: &ProgressSchema, counter: &ProgressCounter) -> String {
    let metric_name = schema
        .metric_name(counter.metric_id())
        .unwrap_or_else(|| counter.metric_id());
    let progress = match (counter.completed_count(), counter.total_count()) {
        (completed, Some(total)) => format!(
            "{completed}/{total} ({:.2}%)",
            counter.progress_percent().unwrap_or(100.0)
        ),
        (completed, None) => format!("{completed} completed"),
    };
    format!(
        "{metric_name} {progress}, active {}, failed {}",
        counter.active_count(),
        counter.failed_count(),
    )
}
