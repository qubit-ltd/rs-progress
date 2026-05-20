/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
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
    ProgressEventBuilder,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressSchema,
    ProgressStage,
};
pub use progress::Progress;
pub use reporter::{
    FormattedProgressReporter,
    HumanReadableMetricSnapshotFormatter,
    HumanReadableProgressReporter,
    JsonLoggerProgressReporter,
    JsonMetricSnapshotFormatter,
    JsonProgressReporter,
    JsonStderrProgressReporter,
    JsonStdoutProgressReporter,
    JsonWriterProgressReporter,
    LoggerProgressReporter,
    MetricSnapshotFormatter,
    MetricSnapshotProgressReporter,
    NoOpProgressReporter,
    ProgressReporter,
    StderrProgressReporter,
    StdoutProgressReporter,
    WriterProgressReporter,
};
pub use running::{
    RunningProgressGuard,
    RunningProgressPointHandle,
};
