// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned by checked successful completion.
// qubit-style: allow source-test-pair

use std::{
    error::Error,
    fmt,
    time::Duration,
};

use crate::error::{
    CompletionError,
    TerminalError,
};

/// Failure from checked finish after the operation has been consumed.
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
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        match self {
            Self::Incomplete { elapsed, .. } => *elapsed,
            Self::Terminal(error) => error.elapsed(),
        }
    }

    /// Returns the completion error when validation failed.
    #[must_use]
    pub const fn completion_error(&self) -> Option<&CompletionError> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(_) => None,
        }
    }
}

impl fmt::Display for FinishError {
    /// Formats the completion or terminal failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete { source, .. } => source.fmt(formatter),
            Self::Terminal(error) => error.fmt(formatter),
        }
    }
}

impl Error for FinishError {
    /// Returns the nested completion or terminal error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Incomplete { source, .. } => Some(source),
            Self::Terminal(error) => Some(error),
        }
    }
}
