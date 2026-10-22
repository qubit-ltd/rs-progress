// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors while constructing or delivering non-start events.

use std::error::Error;
use std::fmt;

use crate::error::DeliveryError;

/// Failure while constructing or delivering a Running or terminal event.
///
/// Sequence exhaustion means no further event can be assigned a sequence
/// number. A delivery failure retains the rejected event and the reporter's
/// original error for inspection.
///
/// # Examples
///
/// ```
/// use qubit_progress::EmissionError;
///
/// let error = EmissionError::SequenceExhausted;
/// assert_eq!(error.to_string(), "progress event sequence is exhausted");
/// ```
#[must_use]
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
