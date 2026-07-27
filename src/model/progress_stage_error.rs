// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

/// Error produced when a progress stage has an invalid relative weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStageError {
    /// The supplied weight is NaN or infinite.
    NonFiniteWeight,
    /// The supplied finite weight is negative.
    NegativeWeight,
}

impl Display for ProgressStageError {
    /// Formats a concise stage weight validation error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteWeight => {
                formatter.write_str("progress stage weight must be finite")
            }
            Self::NegativeWeight => formatter
                .write_str("progress stage weight must be non-negative"),
        }
    }
}

impl Error for ProgressStageError {}
