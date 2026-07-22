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
    io,
};

/// Error produced while delivering a progress event to an output sink.
#[derive(Debug)]
pub enum ProgressReportError {
    /// Writing a formatted progress record failed.
    Io(io::Error),
}

impl Clone for ProgressReportError {
    /// Clones the error kind and rendered operating-system context.
    fn clone(&self) -> Self {
        match self {
            Self::Io(error) => {
                Self::Io(io::Error::new(error.kind(), error.to_string()))
            }
        }
    }
}

impl PartialEq for ProgressReportError {
    /// Compares error kind and rendered context.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(left), Self::Io(right)) => {
                left.kind() == right.kind()
                    && left.to_string() == right.to_string()
            }
        }
    }
}

impl Eq for ProgressReportError {}

impl Display for ProgressReportError {
    /// Formats a concise reporter failure description.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(formatter, "progress output failed: {error}")
            }
        }
    }
}

impl Error for ProgressReportError {
    /// Returns the underlying output error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
        }
    }
}

impl From<io::Error> for ProgressReportError {
    /// Wraps a progress output I/O error.
    #[inline(always)]
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
