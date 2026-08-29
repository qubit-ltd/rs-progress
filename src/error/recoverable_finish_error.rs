// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by recoverable checked successful completion.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;

use crate::Progress;
use crate::error::CompletionError;
use crate::error::TerminalError;

/// Failure from checked finish while preserving a reusable progress operation.
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
    /// Returns the completion error when validation failed.
    #[must_use]
    pub fn completion_error(&self) -> Option<&CompletionError> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(_) => None,
        }
    }

    /// Consumes an incomplete finish error and returns the reusable Progress.
    pub fn into_progress(self) -> Result<Progress<'reporter>, TerminalError> {
        match self {
            Self::Incomplete { progress, .. } => Ok(progress),
            Self::Terminal(error) => Err(error),
        }
    }

    /// Consumes this error and returns its recoverable or terminal parts.
    pub fn into_parts(self) -> Result<(Progress<'reporter>, CompletionError), TerminalError> {
        match self {
            Self::Incomplete { progress, source } => Ok((progress, source)),
            Self::Terminal(error) => Err(error),
        }
    }
}

impl fmt::Debug for RecoverableFinishError<'_> {
    /// Formats the error without requiring the reporter to implement Debug.
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete { source, .. } => source.fmt(formatter),
            Self::Terminal(error) => error.fmt(formatter),
        }
    }
}

impl Error for RecoverableFinishError<'_> {
    /// Returns the nested completion or terminal error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(error) => Some(error),
        }
    }
}
