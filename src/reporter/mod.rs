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
mod progress_report_error;
mod progress_reporter;

#[cfg(feature = "json")]
pub use format::JsonMetricSnapshotFormatter;
pub use format::{
    HumanReadableMetricSnapshotFormatter,
    MetricSnapshotFormatter,
};
#[cfg(all(feature = "json", feature = "log"))]
pub use impls::JsonLoggerProgressReporter;
#[cfg(feature = "log")]
pub use impls::LoggerProgressReporter;
#[cfg(feature = "consumer-reporters")]
pub use impls::{
    FormattedProgressReporter,
    HumanReadableProgressReporter,
    MetricSnapshotProgressReporter,
};
#[cfg(feature = "json")]
pub use impls::{
    JsonProgressReporter,
    JsonStderrProgressReporter,
    JsonStdoutProgressReporter,
    JsonWriterProgressReporter,
};
pub use impls::{
    NoOpProgressReporter,
    StderrProgressReporter,
    StdoutProgressReporter,
    WriterProgressReporter,
};
pub use progress_report_error::ProgressReportError;
pub use progress_reporter::ProgressReporter;
