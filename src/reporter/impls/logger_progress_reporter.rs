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
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
    },
    reporter::ProgressReporter,
};

/// Progress reporter that emits progress events through the `log` crate.
///
/// # Examples
///
/// ```
/// use log::Level;
/// use qubit_progress::LoggerProgressReporter;
///
/// let reporter = LoggerProgressReporter::new("my_app::progress")
///     .with_level(Level::Debug);
///
/// assert_eq!(reporter.target(), "my_app::progress");
/// assert_eq!(reporter.level(), Level::Debug);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggerProgressReporter {
    /// Log target used for emitted records.
    target: String,
    /// Log level used for emitted records.
    level: log::Level,
}

impl LoggerProgressReporter {
    /// Creates a logger reporter at [`log::Level::Info`].
    ///
    /// # Parameters
    ///
    /// * `target` - Log target used for emitted records.
    ///
    /// # Returns
    ///
    /// A logger-backed progress reporter.
    #[inline]
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_owned(),
            level: log::Level::Info,
        }
    }

    /// Returns a copy configured with a log level.
    ///
    /// # Parameters
    ///
    /// * `level` - Log level used for emitted records.
    ///
    /// # Returns
    ///
    /// This reporter configured with `level`.
    #[inline]
    pub const fn with_level(mut self, level: log::Level) -> Self {
        self.level = level;
        self
    }

    /// Returns the log target.
    ///
    /// # Returns
    ///
    /// The target used for emitted log records.
    #[inline]
    pub fn target(&self) -> &str {
        self.target.as_str()
    }

    /// Returns the log level.
    ///
    /// # Returns
    ///
    /// The level used for emitted log records.
    #[inline]
    pub const fn level(&self) -> log::Level {
        self.level
    }

    /// Emits one message through the `log` crate.
    ///
    /// # Parameters
    ///
    /// * `message` - Preformatted progress message.
    #[inline]
    fn log_line(&self, message: &str) {
        log::log!(target: self.target.as_str(), self.level, "{message}");
    }
}

impl Default for LoggerProgressReporter {
    /// Creates a logger reporter with the default target.
    ///
    /// # Returns
    ///
    /// A logger-backed reporter at [`log::Level::Info`].
    #[inline]
    fn default() -> Self {
        Self::new("qubit_progress")
    }
}

impl ProgressReporter for LoggerProgressReporter {
    /// Logs one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to log.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.log_line(&format_event(event));
    }
}

/// Formats one progress event for log output.
///
/// # Parameters
///
/// * `event` - Event to format.
///
/// # Returns
///
/// A single-line log message.
fn format_event(event: &ProgressEvent) -> String {
    let counters = format_counters(event.schema(), event.counters());
    match event.stage() {
        Some(stage) => format!(
            "progress phase={}, stage={}, counters={}",
            event.phase(),
            stage.name(),
            counters,
        ),
        None => format!("progress phase={}, counters={}", event.phase(), counters),
    }
}

/// Formats all counters for log output.
///
/// # Parameters
///
/// * `schema` - Schema used to resolve metric names.
/// * `counters` - Counters to format.
///
/// # Returns
///
/// A comma-separated counter list.
fn format_counters(schema: &ProgressSchema, counters: &[ProgressCounter]) -> String {
    if counters.is_empty() {
        return "[]".to_owned();
    }
    counters
        .iter()
        .map(|counter| {
            let name = schema
                .metric_name(counter.metric_id())
                .unwrap_or_else(|| counter.metric_id());
            let progress = match counter.progress_percent() {
                Some(percent) => format!("{percent:.2}%"),
                None => "unknown".to_owned(),
            };
            format!(
                "{}: completed={}, total={:?}, active={}, failed={}, progress={}",
                name,
                counter.completed_count(),
                counter.total_count(),
                counter.active_count(),
                counter.failed_count(),
                progress,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
