// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Progress reporter trait and built-in implementations.

pub mod format;
mod impls;
mod progress_reporter;

pub use format::{
    HumanReadableMetricSnapshotFormatter,
    JsonMetricSnapshotFormatter,
    MetricSnapshotFormatter,
};
pub use impls::{
    FormattedProgressReporter,
    HumanReadableProgressReporter,
    JsonLoggerProgressReporter,
    JsonProgressReporter,
    JsonStderrProgressReporter,
    JsonStdoutProgressReporter,
    JsonWriterProgressReporter,
    LoggerProgressReporter,
    MetricSnapshotProgressReporter,
    NoOpProgressReporter,
    StderrProgressReporter,
    StdoutProgressReporter,
    WriterProgressReporter,
};
pub use progress_reporter::ProgressReporter;
