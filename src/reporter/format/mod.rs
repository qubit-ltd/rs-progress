// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Formatting support for progress metric snapshots.

pub(crate) mod format_duration;
mod human_readable_metric_snapshot_formatter;
mod json_metric_snapshot_formatter;
mod metric_snapshot_formatter;

pub use human_readable_metric_snapshot_formatter::HumanReadableMetricSnapshotFormatter;
pub use json_metric_snapshot_formatter::JsonMetricSnapshotFormatter;
pub use metric_snapshot_formatter::MetricSnapshotFormatter;
