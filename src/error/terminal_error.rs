// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned after a terminal emission is attempted.

use std::error::Error;
use std::fmt;
use std::time::Duration;

use crate::error::EmissionError;

/// Terminal event delivery failure paired with the elapsed operation time
/// sampled at the beginning of the terminal attempt.
///
/// The error preserves the underlying emission failure and the time measured
/// sampled before metric updates are frozen or the event is delivered. It is
/// returned by terminal progress operations when the reporter rejects the final
/// event.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Event, Metric, Phase, Progress, ReporterError};
///
/// let reporter = |event: &Event| {
///     if event.phase() == Phase::Failed {
///         Err(ReporterError::message("output stream is closed"))
///     } else {
///         Ok(())
///     }
/// };
/// let progress = Progress::builder(&reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// let error = progress.fail().expect_err("the reporter rejects failed events");
/// assert!(error
///     .emission_error()
///     .to_string()
///     .contains("output stream is closed"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
#[derive(Clone, Debug)]
pub struct TerminalError {
    /// Operation time sampled at the beginning of the terminal attempt.
    elapsed: Duration,
    /// Failure returned while delivering the final event.
    source: EmissionError,
}

impl TerminalError {
    /// Pairs the attempted delivery time with its emission failure.
    pub(crate) const fn new(elapsed: Duration, source: EmissionError) -> Self {
        Self { elapsed, source }
    }

    /// Returns the operation time sampled at the beginning of the terminal
    /// attempt.
    ///
    /// # Returns
    ///
    /// The elapsed time recorded before metric updates were frozen or delivery
    /// was attempted.
    #[must_use]
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Borrows the emission failure that caused terminal delivery to fail.
    ///
    /// # Returns
    ///
    /// The underlying emission failure.
    #[must_use = "inspect the emission failure that caused terminal delivery to fail"]
    #[inline]
    pub const fn emission_error(&self) -> &EmissionError {
        &self.source
    }

    /// Consumes the terminal error and returns its emission failure.
    ///
    /// # Returns
    ///
    /// The underlying emission failure, detached from the elapsed time.
    pub fn into_emission_error(self) -> EmissionError {
        self.source
    }

    /// Consumes the terminal error and returns elapsed time and failure.
    ///
    /// # Returns
    ///
    /// The elapsed time followed by the underlying emission failure.
    pub fn into_parts(self) -> (Duration, EmissionError) {
        (self.elapsed, self.source)
    }
}

impl fmt::Display for TerminalError {
    /// Formats terminal elapsed time and the nested failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "terminal progress report failed after {:?}: {}",
            self.elapsed, self.source
        )
    }
}

impl Error for TerminalError {
    /// Returns the emission failure.
    ///
    /// # Returns
    ///
    /// `Some` containing the emission failure that caused terminal delivery to
    /// fail; this implementation never returns `None`.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
