// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Progress event data model.

#[cfg(feature = "serde")]
mod internal;
mod progress_counter;
mod progress_event;
mod progress_event_build_error;
mod progress_event_builder;
mod progress_metric;
mod progress_metric_snapshot;
mod progress_metric_snapshot_error;
mod progress_phase;
mod progress_schema;
mod progress_schema_error;
mod progress_stage;
mod progress_stage_error;

pub use progress_counter::ProgressCounter;
pub use progress_event::ProgressEvent;
pub(crate) use progress_event::next_operation_id;
pub use progress_event_build_error::ProgressEventBuildError;
pub use progress_event_builder::ProgressEventBuilder;
pub use progress_metric::ProgressMetric;
pub use progress_metric_snapshot::ProgressMetricSnapshot;
pub use progress_metric_snapshot_error::ProgressMetricSnapshotError;
pub use progress_phase::ProgressPhase;
pub use progress_schema::ProgressSchema;
pub use progress_schema_error::ProgressSchemaError;
pub use progress_stage::ProgressStage;
pub use progress_stage_error::ProgressStageError;
