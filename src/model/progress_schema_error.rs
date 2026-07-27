// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Error produced when a progress schema violates structural constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressSchemaError {
    /// More than one metric declares the same stable identifier.
    DuplicateMetricId {
        /// Duplicated metric identifier.
        metric_id: String,
    },
}

impl Display for ProgressSchemaError {
    /// Formats a concise schema validation error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMetricId { metric_id } => {
                write!(formatter, "duplicate progress metric id: {metric_id}")
            }
        }
    }
}

impl Error for ProgressSchemaError {}
