// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle phases for immutable progress events.

/// Lifecycle phase of one immutable progress event.
///
/// A started event is unique and has sequence zero; all later phases use a
/// positive sequence. `Succeeded`, `Failed`, and `Cancelled` are terminal.
///
/// # Examples
///
/// ```
/// use qubit_progress::Phase;
///
/// assert_eq!(Phase::Started.as_str(), "started");
/// ```
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// The unique first event.
    Started,
    /// A non-terminal snapshot.
    Running,
    /// Successful terminal event.
    Succeeded,
    /// Failed terminal event.
    Failed,
    /// Cancelled terminal event.
    Cancelled,
}

impl Phase {
    /// Returns the stable lowercase wire name.
    #[must_use]
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}
