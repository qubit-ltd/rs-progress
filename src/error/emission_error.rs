// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors while constructing or delivering non-start events.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;

use crate::error::DeliveryError;

/// Failure while emitting a Running or terminal event.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EmissionError {
    /// The operation's event sequence space is exhausted.
    SequenceExhausted,
    /// A reporter rejected one complete event.
    Delivery(DeliveryError),
}

impl fmt::Display for EmissionError {
    /// Formats the emission failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SequenceExhausted => formatter.write_str("progress event sequence is exhausted"),
            Self::Delivery(error) => error.fmt(formatter),
        }
    }
}

impl Error for EmissionError {
    /// Returns the nested delivery failure when present.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SequenceExhausted => None,
            Self::Delivery(error) => Some(error),
        }
    }
}
