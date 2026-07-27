// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Generic progress reporting data model and reporter abstractions.
//!
//! This crate models progress as immutable, self-describing events carrying a
//! metric schema, lifecycle phase, optional stage information, metric counters,
//! and elapsed time.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod model;
/// Lifecycle helper for one progress-producing operation.
pub mod progress;
pub mod reporter;
/// Helpers for running progress reporting loops.
pub mod running;

pub use model::{
    ProgressCounter,
    ProgressEvent,
    ProgressEventBuildError,
    ProgressEventBuilder,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressMetricSnapshotError,
    ProgressPhase,
    ProgressSchema,
    ProgressSchemaError,
    ProgressStage,
    ProgressStageError,
};
pub use progress::Progress;
#[cfg(all(feature = "json", feature = "log"))]
pub use reporter::JsonLoggerProgressReporter;
#[cfg(feature = "log")]
pub use reporter::LoggerProgressReporter;
#[cfg(feature = "consumer-reporters")]
pub use reporter::{
    FormattedProgressReporter,
    HumanReadableProgressReporter,
    MetricSnapshotProgressReporter,
};
pub use reporter::{
    HumanReadableMetricSnapshotFormatter,
    MetricSnapshotFormatter,
    NoOpProgressReporter,
    ProgressReportError,
    ProgressReporter,
    StderrProgressReporter,
    StdoutProgressReporter,
    WriterProgressReporter,
};
#[cfg(feature = "json")]
pub use reporter::{
    JsonMetricSnapshotFormatter,
    JsonProgressReporter,
    JsonStderrProgressReporter,
    JsonStdoutProgressReporter,
    JsonWriterProgressReporter,
};
pub use running::{
    RunningProgressGuard,
    RunningProgressPointHandle,
    RunningProgressStatus,
};
