// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned directly by reporter sinks.
// qubit-style: allow source-test-pair

use std::{
    error::Error,
    fmt,
    sync::Arc,
};

/// Reporter failure that preserves its original error chain.
#[derive(Clone, Debug)]
pub struct ReporterError {
    source: Arc<dyn Error + Send + Sync + 'static>,
}

impl ReporterError {
    /// Wraps a concrete reporter failure without discarding its source.
    pub fn new<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            source: Arc::new(source),
        }
    }

    /// Creates a reporter error from a stable message.
    pub fn message(message: &str) -> Self {
        Self::new(MessageError(message.into()))
    }

    /// Returns the original reporter error.
    #[must_use]
    pub fn source_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.source.as_ref()
    }
}

impl fmt::Display for ReporterError {
    /// Formats the original reporter error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl Error for ReporterError {
    /// Returns the original reporter error as the source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Message-backed error used by [`ReporterError::message`].
#[derive(Debug)]
struct MessageError(String);

impl fmt::Display for MessageError {
    /// Formats the stored message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for MessageError {}
