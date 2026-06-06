// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::model::ProgressMetricSnapshot;

/// Formats one progress metric snapshot into a string.
///
/// Formatter implementations decide whether the returned string is human
/// readable text, JSON, line protocol, CSV, or another textual representation.
pub trait MetricSnapshotFormatter: Send + Sync {
    /// Formats a metric snapshot.
    ///
    /// # Parameters
    ///
    /// * `snapshot` - Metric snapshot to format.
    ///
    /// # Returns
    ///
    /// A formatted string for downstream consumers.
    fn format(&self, snapshot: &ProgressMetricSnapshot) -> String;
}
