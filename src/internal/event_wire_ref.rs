// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed event representation used to serialize event data.

use serde::Serialize;

use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Stage;
use crate::event::Phase;

/// Borrowed event representation used for serialization.
#[derive(Serialize)]
pub(crate) struct EventWireRef<'a> {
    /// Process-local operation identifier.
    pub(crate) operation_id: u64,
    /// Attempted-delivery sequence.
    pub(crate) sequence: u64,
    /// Event lifecycle phase.
    pub(crate) phase: Phase,
    /// Optional stage metadata.
    pub(crate) stage: Option<&'a Stage>,
    /// Stable operation correlation attributes.
    #[serde(skip_serializing_if = "OperationAttributes::is_empty")]
    pub(crate) attributes: &'a OperationAttributes,
    /// Complete metric snapshots.
    pub(crate) metrics: &'a [MetricSnapshot],
    /// Canonical elapsed duration.
    pub(crate) elapsed: String,
}
