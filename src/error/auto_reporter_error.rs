// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned when a scoped automatic reporter stops.

use std::error::Error;
use std::fmt;

use super::worker_panic::WorkerPanic;
use crate::EmissionError;

/// Failure returned by [`crate::AutoReporter::stop`].
///
/// Pass the error returned by `stop` to a handler to inspect its failure kind.
///
/// # Examples
///
/// ```
/// use qubit_progress::AutoReporterError;
///
/// fn inspect_stop_error(error: AutoReporterError) {
///     match error {
///         AutoReporterError::Emission(error) => eprintln!("{error}"),
///         AutoReporterError::Panicked(error) => eprintln!("{error}"),
///         _ => {}
///     }
/// }
/// ```
#[must_use]
#[derive(Debug)]
#[non_exhaustive]
pub enum AutoReporterError {
    /// The worker returned a normal emission failure.
    Emission(EmissionError),
    /// The worker panicked while reporting.
    Panicked(WorkerPanic),
}

impl From<EmissionError> for AutoReporterError {
    /// Wraps a normal worker emission failure.
    fn from(error: EmissionError) -> Self {
        Self::Emission(error)
    }
}

impl fmt::Display for AutoReporterError {
    /// Formats the structured worker failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Emission(error) => error.fmt(formatter),
            Self::Panicked(error) => error.fmt(formatter),
        }
    }
}

impl Error for AutoReporterError {
    /// Returns the nested emission failure when present.
    ///
    /// # Returns
    ///
    /// `Some(error)` when this error wraps an emission failure; `None` when it
    /// wraps a worker panic.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Emission(error) => Some(error),
            Self::Panicked(_) => None,
        }
    }
}
