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
    io,
};

use crate::model::ProgressEventBuildError;

/// Error produced while preparing or delivering a progress event.
#[derive(Debug)]
pub enum ProgressReportError {
    /// Building an event violated its metric-schema constraints.
    EventBuild(ProgressEventBuildError),
    /// Writing a formatted progress record failed.
    Io(io::Error),
    /// A custom reporter rejected the progress event.
    Message(String),
}

impl ProgressReportError {
    /// Creates an error reported by a non-I/O progress sink.
    ///
    /// # Parameters
    ///
    /// * `message` - Human-readable explanation supplied by the reporter.
    ///
    /// # Returns
    ///
    /// A progress reporter error preserving `message`.
    #[inline]
    pub fn message(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}

impl Clone for ProgressReportError {
    /// Clones the error kind and rendered operating-system context.
    fn clone(&self) -> Self {
        match self {
            Self::EventBuild(error) => Self::EventBuild(error.clone()),
            Self::Io(error) => Self::Io(io::Error::new(error.kind(), error.to_string())),
            Self::Message(message) => Self::Message(message.clone()),
        }
    }
}

impl PartialEq for ProgressReportError {
    /// Compares error kind and rendered context.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EventBuild(left), Self::EventBuild(right)) => left == right,
            (Self::Io(left), Self::Io(right)) => {
                left.kind() == right.kind() && left.to_string() == right.to_string()
            }
            (Self::Message(left), Self::Message(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for ProgressReportError {}

impl Display for ProgressReportError {
    /// Formats a concise reporter failure description.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventBuild(error) => {
                write!(formatter, "progress event is invalid: {error}")
            }
            Self::Io(error) => {
                write!(formatter, "progress output failed: {error}")
            }
            Self::Message(message) => {
                write!(formatter, "progress reporter failed: {message}")
            }
        }
    }
}

impl Error for ProgressReportError {
    /// Returns the underlying output error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EventBuild(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Message(_) => None,
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

impl From<ProgressEventBuildError> for ProgressReportError {
    /// Wraps an invalid progress event.
    #[inline(always)]
    fn from(error: ProgressEventBuildError) -> Self {
        Self::EventBuild(error)
    }
}
