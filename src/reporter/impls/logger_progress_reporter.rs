// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_function::ArcConsumer;

use super::human_readable_progress_reporter::HumanReadableProgressReporter;
use crate::{
    model::ProgressEvent,
    reporter::ProgressReporter,
};

/// Progress reporter that emits human-readable metric snapshots through `log`.
pub struct LoggerProgressReporter {
    /// Log target used for emitted records.
    target: String,
    /// Log level used for emitted records.
    level: log::Level,
    /// Human-readable reporter that consumes formatted strings.
    inner: HumanReadableProgressReporter,
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
        Self::with_target_and_level(target, log::Level::Info)
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
    #[must_use]
    pub fn with_level(self, level: log::Level) -> Self {
        Self::with_target_and_level(self.target.as_str(), level)
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

    /// Creates a logger reporter from target and level.
    ///
    /// # Parameters
    ///
    /// * `target` - Log target used for emitted records.
    /// * `level` - Log level used for emitted records.
    ///
    /// # Returns
    ///
    /// A configured logger reporter.
    fn with_target_and_level(target: &str, level: log::Level) -> Self {
        let target = target.to_owned();
        let log_target = target.clone();
        let consumer = ArcConsumer::new(move |line: &String| {
            log::log!(target: log_target.as_str(), level, "{line}");
        });
        Self {
            target,
            level,
            inner: HumanReadableProgressReporter::new(consumer),
        }
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
    /// Logs one line for every metric snapshot in the event.
    ///
    /// # Parameters
    ///
    /// * `event` - Progress event to log.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
