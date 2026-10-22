// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Metric configuration and immutable metric snapshots.

mod internal;
mod metric;
mod metric_delta;
mod metric_handle;
mod metric_snapshot;

pub use metric::Metric;
pub use metric_delta::MetricDelta;
pub use metric_handle::MetricHandle;
pub use metric_snapshot::MetricSnapshot;
