// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by recoverable checked successful completion.

use std::error::Error;
use std::fmt;

use crate::Progress;
use crate::error::CompletionError;
use crate::error::TerminalError;

/// Failure from checked finish while preserving a reusable progress operation.
///
/// An incomplete operation retains its [`Progress`] value so the caller can
/// correct the metric state and retry, or terminate the operation explicitly.
/// Terminal delivery failures cannot be recovered because emission may have
/// partially reached the reporter.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, NoopReporter, Progress, RecoverableFinishError};
///
/// let progress = Progress::builder(&NoopReporter)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// let error = progress
///     .finish_recoverable()
///     .expect_err("unfinished work must prevent completion");
/// assert!(matches!(error, RecoverableFinishError::Incomplete { .. }));
/// let progress = error.into_progress()?;
/// progress.cancel()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum RecoverableFinishError<'reporter> {
    /// Completion validation failed and the operation remains reusable.
    Incomplete {
        /// Progress operation returned to the caller.
        progress: Progress<'reporter>,
        /// First completion invariant that failed.
        source: CompletionError,
    },
    /// A terminal emission was attempted and failed permanently.
    Terminal(TerminalError),
}

impl<'reporter> RecoverableFinishError<'reporter> {
    /// Returns the completion validation error, or `None` after terminal
    /// delivery failed.
    ///
    /// # Returns
    ///
    /// `Some` contains the completion validation error; `None` means terminal
    /// delivery was attempted.
    #[must_use]
    #[inline]
    pub fn completion_error(&self) -> Option<&CompletionError> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(_) => None,
        }
    }

    /// Consumes an incomplete finish error and returns its reusable progress
    /// operation.
    ///
    /// # Errors
    ///
    /// Returns the terminal delivery error when emission failed and the
    /// operation cannot be recovered.
    ///
    /// # Returns
    ///
    /// The reusable progress operation after successful validation.
    pub fn into_progress(self) -> Result<Progress<'reporter>, TerminalError> {
        match self {
            Self::Incomplete { progress, .. } => Ok(progress),
            Self::Terminal(error) => Err(error),
        }
    }

    /// Consumes this error and returns its recoverable or terminal parts.
    ///
    /// # Errors
    ///
    /// Returns the terminal delivery error when emission failed and no
    /// reusable progress operation or completion validation error exists.
    ///
    /// # Returns
    ///
    /// The reusable progress operation and completion validation error.
    pub fn into_parts(self) -> Result<(Progress<'reporter>, CompletionError), TerminalError> {
        match self {
            Self::Incomplete { progress, source } => Ok((progress, source)),
            Self::Terminal(error) => Err(error),
        }
    }
}

impl fmt::Debug for RecoverableFinishError<'_> {
    /// Formats the error without requiring the reporter to implement Debug.
    ///
    /// # Returns
    ///
    /// The formatter result, including any formatting error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete { source, .. } => formatter
                .debug_struct("RecoverableFinishError::Incomplete")
                .field("source", source)
                .finish(),
            Self::Terminal(error) => formatter
                .debug_tuple("RecoverableFinishError::Terminal")
                .field(error)
                .finish(),
        }
    }
}

impl fmt::Display for RecoverableFinishError<'_> {
    /// Formats the completion or terminal failure.
    ///
    /// # Returns
    ///
    /// The formatter result, including any formatting error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete { source, .. } => source.fmt(formatter),
            Self::Terminal(error) => error.fmt(formatter),
        }
    }
}

impl Error for RecoverableFinishError<'_> {
    /// Returns the nested completion or terminal error.
    ///
    /// # Returns
    ///
    /// The nested completion or terminal error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(error) => Some(error),
        }
    }
}
