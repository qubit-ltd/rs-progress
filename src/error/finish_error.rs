// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by checked successful completion.

use std::error::Error;
use std::fmt;
use std::time::Duration;

use crate::error::CompletionError;
use crate::error::TerminalError;

/// Failure from checked finish after the operation has been consumed.
///
/// `Progress::finish` returns this error when completion validation or terminal
/// event delivery fails.
///
/// # Examples
///
/// A `FinishError` value is returned by [`crate::Progress::finish`] when
/// checked completion fails.
///
/// ```
/// use qubit_progress::FinishError;
///
/// // `Progress::finish` supplies the error when checked completion fails.
/// fn inspect_finish_error(error: &FinishError) {
///     assert!(error.elapsed() >= std::time::Duration::ZERO);
///     if let Some(completion_error) = error.completion_error() {
///         eprintln!("completion failed: {completion_error}");
///     }
/// }
/// ```
#[must_use]
#[derive(Debug)]
pub enum FinishError {
    /// Completion validation failed before terminal emission.
    Incomplete {
        /// Elapsed operation time sampled before completion validation.
        elapsed: Duration,
        /// First completion invariant that failed.
        source: CompletionError,
    },
    /// A terminal emission was attempted and failed permanently.
    Terminal(TerminalError),
}

impl FinishError {
    /// Returns elapsed operation time sampled by the finish attempt.
    ///
    /// # Returns
    ///
    /// The elapsed operation time recorded by the finish attempt.
    #[must_use]
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        match self {
            Self::Incomplete { elapsed, .. } => *elapsed,
            Self::Terminal(error) => error.elapsed(),
        }
    }

    /// Returns the completion error when validation failed before terminal
    /// emission.
    ///
    /// Returns `None` when terminal emission was attempted instead.
    ///
    /// # Returns
    ///
    /// `Some` contains the completion validation error; `None` means terminal
    /// emission was attempted.
    #[inline]
    pub const fn completion_error(&self) -> Option<&CompletionError> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(_) => None,
        }
    }
}

impl fmt::Display for FinishError {
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

impl Error for FinishError {
    /// Returns the nested completion or terminal error.
    ///
    /// # Returns
    ///
    /// Always returns the nested error that caused finish to fail.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(error) => Some(error),
        }
    }
}
