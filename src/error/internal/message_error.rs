// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! String-backed implementation of a reporter error.

use std::error::Error;
use std::fmt;

/// Message-backed error used by [`crate::ReporterError::message`].
#[must_use]
#[derive(Debug)]
pub(in crate::error) struct MessageError {
    /// Owned diagnostic text shown to reporter error consumers.
    message: String,
}

impl MessageError {
    /// Creates a message error from its owned diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `message`: the diagnostic text retained by this error.
    ///
    /// # Returns
    ///
    /// An error that owns the supplied diagnostic text.
    #[inline]
    pub(in crate::error) fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for MessageError {
    /// Formats the stored message.
    ///
    /// # Parameters
    ///
    /// - `formatter`: the destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the message is written, or the formatter's error.
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for MessageError {}
