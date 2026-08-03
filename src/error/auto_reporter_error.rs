// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned when a scoped automatic reporter stops.
// qubit-style: allow source-test-pair
// qubit-style: allow multiple-public-types

use std::{
    any::Any,
    error::Error,
    fmt,
    panic,
};

use crate::EmissionError;

/// Failure returned by [`crate::AutoReporter::stop`].
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
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Emission(error) => Some(error),
            Self::Panicked(_) => None,
        }
    }
}

/// An opaque panic payload captured from an automatic reporter worker.
pub struct WorkerPanic {
    /// Original payload retained for callers that need to resume unwinding.
    payload: Box<dyn Any + Send + 'static>,
}

impl WorkerPanic {
    /// Creates a structured panic from an unwound worker payload.
    pub(crate) fn new(payload: Box<dyn Any + Send + 'static>) -> Self {
        Self { payload }
    }

    /// Returns a borrowed message for standard string panic payloads.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| self.payload.downcast_ref::<&'static str>().copied())
    }

    /// Consumes the error and returns the original panic payload.
    #[must_use]
    pub fn into_payload(self) -> Box<dyn Any + Send + 'static> {
        self.payload
    }

    /// Resumes unwinding with the original panic payload.
    pub fn resume_unwind(self) -> ! {
        panic::resume_unwind(self.into_payload())
    }
}

impl fmt::Debug for WorkerPanic {
    /// Formats the payload without requiring the payload itself to implement
    /// `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkerPanic")
            .field("message", &self.message())
            .finish()
    }
}

impl fmt::Display for WorkerPanic {
    /// Formats the panic and includes its standard string message when known.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message() {
            Some(message) => write!(
                formatter,
                "background reporter worker panicked: {message}"
            ),
            None => formatter.write_str("background reporter worker panicked"),
        }
    }
}

impl Error for WorkerPanic {}
