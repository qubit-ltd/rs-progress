// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors encountered while starting a progress operation.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;

use crate::error::ConfigurationError;
use crate::error::DeliveryError;
use crate::error::EmissionError;

/// Failure before a usable progress operation can be returned.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum StartError {
    /// Fixed operation metadata is invalid.
    InvalidConfiguration(ConfigurationError),
    /// The process-local operation ID space is exhausted.
    OperationIdExhausted,
    /// Started was attempted but rejected by the reporter.
    Delivery(DeliveryError),
}

impl fmt::Display for StartError {
    /// Formats the start failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(error) => error.fmt(formatter),
            Self::OperationIdExhausted => formatter.write_str("progress operation IDs are exhausted"),
            Self::Delivery(error) => error.fmt(formatter),
        }
    }
}

impl Error for StartError {
    /// Returns the nested configuration or delivery failure.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidConfiguration(error) => Some(error),
            Self::OperationIdExhausted => None,
            Self::Delivery(error) => Some(error),
        }
    }
}

impl From<ConfigurationError> for StartError {
    /// Converts configuration validation into a start failure.
    fn from(error: ConfigurationError) -> Self {
        Self::InvalidConfiguration(error)
    }
}

impl From<EmissionError> for StartError {
    /// Converts the only possible start-time emission failures.
    fn from(error: EmissionError) -> Self {
        match error {
            EmissionError::Delivery(error) => Self::Delivery(error),
            EmissionError::SequenceExhausted => Self::OperationIdExhausted,
        }
    }
}
