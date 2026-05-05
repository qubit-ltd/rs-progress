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
    ProgressEvent,
    ProgressReporter,
};

/// Progress reporter that emits progress events through the `log` crate.
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
    pub const fn with_level(mut self, level: log::Level) -> Self {
        self.level = level;
        self
    }

    /// Returns the log target.
    ///
    /// # Returns
    ///
    /// The target used for emitted log records.
    pub fn target(&self) -> &str {
        self.target.as_str()
    }

    /// Returns the log level.
    ///
    /// # Returns
    ///
    /// The level used for emitted log records.
    pub const fn level(&self) -> log::Level {
        self.level
    }

    /// Emits one message through the `log` crate.
    ///
    /// # Parameters
    ///
    /// * `message` - Preformatted progress message.
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
    fn default() -> Self {
        Self::new("qubit_progress")
    }
}

impl<C> ProgressReporter<C> for LoggerProgressReporter {
    /// Logs one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to log.
    fn report(&self, event: &ProgressEvent<C>) {
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
fn format_event<C>(event: &ProgressEvent<C>) -> String {
    let counters = event.counters();
    let progress = match counters.progress_percent() {
        Some(percent) => format!("{percent:.2}%"),
        None => "unknown".to_owned(),
    };
    match event.stage() {
        Some(stage) => format!(
            "progress phase={}, stage={}, completed={}, total={:?}, active={}, failed={}, progress={}",
            event.phase(),
            stage.name(),
            counters.completed_count(),
            counters.total_count(),
            counters.active_count(),
            counters.failed_count(),
            progress,
        ),
        None => format!(
            "progress phase={}, completed={}, total={:?}, active={}, failed={}, progress={}",
            event.phase(),
            counters.completed_count(),
            counters.total_count(),
            counters.active_count(),
            counters.failed_count(),
            progress,
        ),
    }
}
