// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Named metric state transitions.

use std::fmt;

/// Kind of constrained metric state transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricTransition {
    /// Moves work from not-started to active.
    Start,
    /// Moves work from active to unclassified completion.
    Complete,
    /// Moves work from active to succeeded.
    Succeed,
    /// Moves work from active to failed.
    Fail,
    /// Moves work from active to cancelled.
    Cancel,
}

impl fmt::Display for MetricTransition {
    /// Formats the stable transition name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Start => "start",
            Self::Complete => "complete",
            Self::Succeed => "succeed",
            Self::Fail => "fail",
            Self::Cancel => "cancel",
        })
    }
}
