// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#[cfg(feature = "serde")]
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::Deserialize;

/// Serializable wire representation used to validate standalone snapshots.
#[cfg(feature = "serde")]
#[derive(Deserialize)]
pub struct MetricSnapshotWire {
    /// Machine-readable metric ID.
    pub(in crate::metric) id: Arc<str>,
    /// Human-readable metric name.
    pub(in crate::metric) name: Arc<str>,
    /// Configured total, if known.
    pub(in crate::metric) total: Option<u64>,
    /// Completed count.
    pub(in crate::metric) completed: u64,
    /// Active count.
    pub(in crate::metric) active: u64,
    /// Succeeded count.
    pub(in crate::metric) succeeded: u64,
    /// Failed count.
    pub(in crate::metric) failed: u64,
    /// Cancelled count.
    pub(in crate::metric) cancelled: u64,
}
