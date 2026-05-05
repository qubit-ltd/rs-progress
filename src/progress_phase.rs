/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::fmt;

/// Lifecycle phase of a progress-producing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressPhase {
    /// The operation has started.
    Started,
    /// The operation is still running.
    Running,
    /// The operation finished successfully.
    Finished,
    /// The operation reached a failed terminal state.
    Failed,
    /// The operation was canceled before normal completion.
    Canceled,
}

impl ProgressPhase {
    /// Returns the stable lowercase name of this phase.
    ///
    /// # Returns
    ///
    /// A static string suitable for logs and human-readable reporter output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Running => "running",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

impl fmt::Display for ProgressPhase {
    /// Formats this phase as its stable lowercase name.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Formatter receiving the phase text.
    ///
    /// # Returns
    ///
    /// The formatter result.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
