// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stage metadata and its public API.

use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Deserializer;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::de::Error;

#[cfg(feature = "serde")]
use super::internal::StageWire;
#[cfg(feature = "serde")]
use crate::validation::validate_stage;

/// Human-readable sub-stage attached to subsequently emitted events.
///
/// # Examples
///
/// ```
/// use qubit_progress::Stage;
///
/// let stage = Stage::new("copy", "Copy files").position(2, 3);
/// assert_eq!(stage.id(), "copy");
/// assert_eq!(stage.name(), "Copy files");
/// assert_eq!(stage.position_value(), Some(2));
/// assert_eq!(stage.total(), Some(3));
/// ```
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Stage {
    /// Deserializes and validates one standalone stage.
    ///
    /// # Parameters
    ///
    /// - `deserializer` - Serde deserializer supplying the stage fields.
    ///
    /// # Returns
    ///
    /// The validated stage when the wire data is valid.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error for malformed data or invalid stage
    /// metadata.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StageWire::deserialize(deserializer)?;
        let stage = Self {
            id: wire.id,
            name: wire.name,
            position: wire.position,
            total: wire.total,
        };
        validate_stage(&stage).map_err(Error::custom)?;
        Ok(stage)
    }
}

impl Stage {
    /// Creates stage metadata without sequence position information.
    ///
    /// # Parameters
    ///
    /// - `id` - Stable machine-readable identifier for this stage.
    /// - `name` - Human-readable name shown to users.
    ///
    /// # Returns
    ///
    /// A stage with no position or total configured.
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: Arc::from(id),
            name: Arc::from(name),
            position: None,
            total: None,
        }
    }

    /// Returns the stage's stable ID.
    ///
    /// # Returns
    ///
    /// The machine-readable identifier borrowed from this stage.
    #[must_use]
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the stage's display name.
    ///
    /// # Returns
    ///
    /// The human-readable name borrowed from this stage.
    #[must_use]
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the one-based position, if present.
    ///
    /// # Returns
    ///
    /// `Some(position)` when a sequence position was configured, or `None`
    /// when this stage has no position information.
    #[inline]
    pub const fn position_value(&self) -> Option<u64> {
        self.position
    }

    /// Returns the stage count, if present.
    ///
    /// # Returns
    ///
    /// `Some(total)` when a sequence total was configured, or `None` when this
    /// stage has no position information.
    #[inline]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }

    /// Sets one-based stage position and total number of stages.
    ///
    /// # Parameters
    ///
    /// - `position` - One-based position of this stage in the sequence.
    /// - `total` - Number of stages in the sequence.
    ///
    /// # Returns
    ///
    /// This stage with its position and total set to the supplied values.
    #[must_use]
    #[inline]
    pub const fn position(mut self, position: u64, total: u64) -> Self {
        self.position = Some(position);
        self.total = Some(total);
        self
    }
}
