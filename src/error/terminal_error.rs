// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned after a terminal emission is attempted.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;
use std::time::Duration;

use crate::error::EmissionError;

/// Terminal emission failure paired with elapsed operation time.
#[derive(Clone, Debug)]
pub struct TerminalError {
    elapsed: Duration,
    source: EmissionError,
}

impl TerminalError {
    /// Creates a terminal error.
    pub(crate) const fn new(elapsed: Duration, source: EmissionError) -> Self {
        Self { elapsed, source }
    }

    /// Returns elapsed operation time at terminal failure.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the emission failure.
    #[must_use]
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
