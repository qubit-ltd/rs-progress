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

/// Terminal event delivery failure paired with the elapsed operation time.
///
/// The error preserves the underlying emission failure and the time measured
/// when delivery was attempted. It is returned by terminal progress operations
/// when the reporter rejects the final event.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Event, Phase, Progress, ReporterError};
///
/// let reporter = |event: &Event| {
///     if event.phase() == Phase::Failed {
///         Err(ReporterError::message("output stream is closed"))
///     } else {
///         Ok(())
///     }
/// };
/// let progress = Progress::builder(&reporter).start()?;
/// let error = progress.fail().expect_err("the reporter rejects failed events");
/// assert_eq!(error.emission_error().to_string(), "output stream is closed");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
#[derive(Clone, Debug)]
pub struct TerminalError {
    /// Operation time measured when the final event was attempted.
    elapsed: Duration,
    /// Failure returned while delivering the final event.
    source: EmissionError,
}

impl TerminalError {
    /// Pairs the attempted delivery time with its emission failure.
    pub(crate) const fn new(elapsed: Duration, source: EmissionError) -> Self {
        Self { elapsed, source }
    }

    /// Returns the operation time measured when terminal delivery failed.
    #[must_use]
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Borrows the emission failure that caused terminal delivery to fail.
    #[must_use]
    #[inline]
    pub const fn emission_error(&self) -> &EmissionError {
        &self.source
    }

    /// Consumes the terminal error and returns its emission failure.
    #[must_use]
    pub fn into_emission_error(self) -> EmissionError {
        self.source
    }

    /// Consumes the terminal error and returns elapsed time and failure.
    #[must_use]
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
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
