// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned directly by reporter sinks.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use crate::error::internal::MessageError;

/// Reporter failure that preserves the original sink error and its source
/// chain.
///
/// Use [`ReporterError::new`] when a reporter has a concrete error value, or
/// [`ReporterError::message`] when it only has a stable diagnostic message.
/// The wrapped error remains available through [`std::error::Error::source`]
/// and [`ReporterError::source_error`].
///
/// # Examples
///
/// ```
/// use qubit_progress::ReporterError;
///
/// let error = ReporterError::message("output stream is closed");
/// assert_eq!(error.to_string(), "output stream is closed");
/// assert_eq!(error.source_error().to_string(), "output stream is closed");
/// ```
#[must_use]
#[derive(Clone, Debug)]
pub struct ReporterError {
    /// Original reporter failure retained for source-chain inspection.
    source: Arc<dyn Error + Send + Sync + 'static>,
}

impl ReporterError {
    /// Wraps a concrete reporter failure without discarding its source.
    ///
    /// # Type Parameters
    ///
    /// `E` is the concrete error type produced by the reporter.
    ///
    /// # Parameters
    ///
    /// - `source`: the original error, retained as the error source.
    ///
    /// # Returns
    ///
    /// A reporter error that preserves `source` and its source chain.
    pub fn new<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            source: Arc::new(source),
        }
    }

    /// Creates a reporter error from a stable message.
    ///
    /// # Parameters
    ///
    /// - `message`: a stable diagnostic message describing the reporter
    ///   failure.
    ///
    /// # Returns
    ///
    /// A reporter error containing the supplied message.
    pub fn message(message: &str) -> Self {
        Self::new(MessageError::new(message.into()))
    }

    /// Returns the original reporter error.
    ///
    /// # Returns
    ///
    /// A borrowed view of the wrapped error, including its concrete source
    /// chain.
    #[must_use]
    #[inline]
    pub fn source_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.source.as_ref()
    }
}

impl fmt::Display for ReporterError {
    /// Formats the original reporter error.
    ///
    /// # Returns
    ///
    /// `Ok(())` if formatting succeeds, or the formatter's error otherwise.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl Error for ReporterError {
    /// Returns the original reporter error as the source.
    ///
    /// # Returns
    ///
    /// `Some` containing the wrapped error.
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
