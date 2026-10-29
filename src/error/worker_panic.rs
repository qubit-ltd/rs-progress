// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Panic payloads returned when an automatic reporter worker unwinds.

use std::any::Any;
use std::error::Error;
use std::fmt;
use std::panic;

/// An opaque panic payload captured from an automatic reporter worker.
///
/// The value is supplied as [`crate::AutoReporterError::Panicked`] by
/// [`crate::AutoReporter::stop`] when its worker panics. Callers can inspect a
/// standard string payload or resume unwinding.
///
/// # Examples
///
/// ```
/// use qubit_progress::WorkerPanic;
///
/// fn inspect_worker_panic(error: &WorkerPanic) {
///     if let Some(message) = error.message() {
///         eprintln!("worker panic: {message}");
///     }
/// }
/// ```
#[must_use]
pub struct WorkerPanic {
    /// Original payload retained for callers that need to resume unwinding.
    payload: Box<dyn Any + Send + 'static>,
}

impl WorkerPanic {
    /// Creates a structured panic from an unwound worker payload.
    #[inline]
    pub(crate) fn new(payload: Box<dyn Any + Send + 'static>) -> Self {
        Self { payload }
    }

    /// Returns the borrowed message when the payload is a standard string.
    ///
    /// # Returns
    ///
    /// Returns `Some` for `String` and `&'static str` payloads, and `None` for
    /// other payload types.
    #[inline]
    pub fn message(&self) -> Option<&str> {
        self.payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| self.payload.downcast_ref::<&'static str>().copied())
    }

    /// Consumes the error and returns the original panic payload.
    ///
    /// # Returns
    ///
    /// The boxed payload originally captured from the worker thread.
    #[must_use]
    #[inline]
    pub fn into_payload(self) -> Box<dyn Any + Send + 'static> {
        self.payload
    }

    /// Resumes unwinding with the original panic payload.
    ///
    /// # Panics
    ///
    /// This method begins unwinding with the retained payload; it never
    /// returns normally.
    #[inline]
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
            Some(message) => write!(formatter, "background reporter worker panicked: {message}"),
            None => formatter.write_str("background reporter worker panicked"),
        }
    }
}

impl Error for WorkerPanic {}
