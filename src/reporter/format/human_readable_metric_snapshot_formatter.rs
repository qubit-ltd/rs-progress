/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use super::{
    format_duration::format_duration,
    metric_snapshot_formatter::MetricSnapshotFormatter,
};
use crate::model::ProgressMetricSnapshot;

/// Formats metric snapshots as compact human-readable progress lines.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HumanReadableMetricSnapshotFormatter;

impl HumanReadableMetricSnapshotFormatter {
    /// Creates a human-readable metric snapshot formatter.
    ///
    /// # Returns
    ///
    /// A formatter using the default human-readable output style.
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl MetricSnapshotFormatter for HumanReadableMetricSnapshotFormatter {
    /// Formats one metric snapshot as a human-readable line.
    ///
    /// # Parameters
    ///
    /// * `snapshot` - Snapshot to format.
    ///
    /// # Returns
    ///
    /// A compact line containing phase, stage, metric progress, counters, and elapsed time.
    fn format(&self, snapshot: &ProgressMetricSnapshot) -> String {
        let progress = match (snapshot.completed_count(), snapshot.total_count()) {
            (completed, Some(total)) => format!(
                "{completed}/{total} ({:.2}%)",
                snapshot.progress_percent().unwrap_or(100.0)
            ),
            (completed, None) => format!("{completed} completed"),
        };
        let elapsed = format_duration(snapshot.elapsed());
        let stage = snapshot
            .stage()
            .map(|stage| format!(" [{}]", stage.name()))
            .unwrap_or_default();
        format!(
            "{}{} {} {progress}, active {}, succeeded {}, failed {}, elapsed {elapsed}",
            snapshot.phase(),
            stage,
            snapshot.metric_name(),
            snapshot.active_count(),
            snapshot.succeeded_count(),
            snapshot.failed_count(),
        )
    }
}
