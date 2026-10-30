// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Owned event representation used to validate deserialized event data.

use serde::Deserialize;

use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Stage;
use crate::event::Phase;

/// Owned event representation used before validation during deserialization.
#[derive(Deserialize)]
pub(crate) struct EventWire {
    /// Process-local operation identifier.
    pub(crate) operation_id: u64,
    /// Attempted-delivery sequence.
    pub(crate) sequence: u64,
    /// Event lifecycle phase.
    pub(crate) phase: Phase,
    /// Optional stage metadata.
    pub(crate) stage: Option<Stage>,
    /// Stable operation correlation attributes.
    #[serde(default)]
    pub(crate) attributes: OperationAttributes,
    /// Complete metric snapshots.
    pub(crate) metrics: Vec<MetricSnapshot>,
    /// Canonical elapsed duration.
    pub(crate) elapsed: String,
}
