// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::metric_snapshot_formatter::MetricSnapshotFormatter;
use crate::model::ProgressMetricSnapshot;

/// Formats metric snapshots as compact JSON strings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JsonMetricSnapshotFormatter;

impl JsonMetricSnapshotFormatter {
    /// Creates a JSON metric snapshot formatter.
    ///
    /// # Returns
    ///
    /// A formatter that serializes each snapshot as one compact JSON string.
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl MetricSnapshotFormatter for JsonMetricSnapshotFormatter {
    /// Formats one metric snapshot as compact JSON.
    ///
    /// # Parameters
    ///
    /// * `snapshot` - Snapshot to serialize.
    ///
    /// # Returns
    ///
    /// A JSON string representing `snapshot`.
    ///
    /// # Panics
    ///
    /// Panics if serde serialization unexpectedly fails.
    #[inline]
    fn format(&self, snapshot: &ProgressMetricSnapshot) -> String {
        serde_json::to_string(snapshot)
            .expect("progress metric snapshot should serialize")
    }
}
