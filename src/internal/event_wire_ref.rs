// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed event representation used to serialize event data.

use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Stage;
use crate::event::Phase;

/// Borrowed event representation used for serialization.
#[derive(serde::Serialize)]
pub(super) struct EventWireRef<'a> {
    /// Process-local operation identifier.
    pub(super) operation_id: u64,
    /// Attempted-delivery sequence.
    pub(super) sequence: u64,
    /// Event lifecycle phase.
    pub(super) phase: Phase,
    /// Optional stage metadata.
    pub(super) stage: Option<&'a Stage>,
    /// Stable operation correlation attributes.
    #[serde(skip_serializing_if = "OperationAttributes::is_empty")]
    pub(super) attributes: &'a OperationAttributes,
    /// Complete metric snapshots.
    pub(super) metrics: &'a [MetricSnapshot],
    /// Canonical elapsed duration.
    pub(super) elapsed: String,
}
