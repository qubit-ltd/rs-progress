// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::fmt;

use serde::{
    Deserialize,
    Serialize,
};

/// Lifecycle phase of a progress event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhase {
    /// Operation has started.
    Started,
    /// Operation is still running.
    Running,
    /// Operation finished successfully.
    Finished,
    /// Operation failed.
    Failed,
    /// Operation was canceled.
    Canceled,
}

impl ProgressPhase {
    /// Returns the phase as a stable lower-case string.
    ///
    /// # Returns
    ///
    /// A stable status string suitable for text output.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Running => "running",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    /// Returns `true` for terminal phases.
    ///
    /// # Returns
    ///
    /// `true` for finished, failed, or canceled phases.
    #[inline]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Finished | Self::Failed | Self::Canceled)
    }
}

impl fmt::Display for ProgressPhase {
    /// Formats the phase as a lower-case status word.
    ///
    /// # Parameters
    ///
    /// * `f` - Formatter receiving the phase text.
    ///
    /// # Returns
    ///
    /// Formatter result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
