// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Optional operation stage metadata.

use std::sync::Arc;

/// Human-readable sub-stage attached to subsequently emitted events.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stage {
    /// Machine-readable stage identifier.
    pub(crate) id: Arc<str>,
    /// Human-readable stage name.
    pub(crate) name: Arc<str>,
    /// One-based stage position when present.
    pub(crate) position: Option<u64>,
    /// Number of stages when present.
    pub(crate) total: Option<u64>,
}

impl Stage {
    /// Creates stage metadata without sequence position information.
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: Arc::from(id),
            name: Arc::from(name),
            position: None,
            total: None,
        }
    }
    /// Sets one-based stage position and total number of stages.
    #[must_use]
    pub const fn position(mut self, position: u64, total: u64) -> Self {
        self.position = Some(position);
        self.total = Some(total);
        self
    }
    /// Returns the stage's stable ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the stage's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the one-based position, if present.
    #[must_use]
    pub const fn position_value(&self) -> Option<u64> {
        self.position
    }
    /// Returns the stage count, if present.
    #[must_use]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }
}
