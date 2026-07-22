// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Built-in progress reporter implementations.

#[cfg(feature = "consumer-reporters")]
mod formatted_progress_reporter;
#[cfg(feature = "consumer-reporters")]
mod human_readable_progress_reporter;
#[cfg(all(feature = "json", feature = "log"))]
mod json_logger_progress_reporter;
#[cfg(feature = "json")]
mod json_progress_reporter;
#[cfg(feature = "json")]
mod json_stderr_progress_reporter;
#[cfg(feature = "json")]
mod json_stdout_progress_reporter;
#[cfg(feature = "json")]
mod json_writer_progress_reporter;
#[cfg(feature = "log")]
mod logger_progress_reporter;
#[cfg(feature = "consumer-reporters")]
mod metric_snapshot_progress_reporter;
mod no_op_progress_reporter;
mod stderr_progress_reporter;
mod stdout_progress_reporter;
mod writer_progress_reporter;

#[cfg(feature = "consumer-reporters")]
pub use formatted_progress_reporter::FormattedProgressReporter;
#[cfg(feature = "consumer-reporters")]
pub use human_readable_progress_reporter::HumanReadableProgressReporter;
#[cfg(all(feature = "json", feature = "log"))]
pub use json_logger_progress_reporter::JsonLoggerProgressReporter;
#[cfg(feature = "json")]
pub use json_progress_reporter::JsonProgressReporter;
#[cfg(feature = "json")]
pub use json_stderr_progress_reporter::JsonStderrProgressReporter;
#[cfg(feature = "json")]
pub use json_stdout_progress_reporter::JsonStdoutProgressReporter;
#[cfg(feature = "json")]
pub use json_writer_progress_reporter::JsonWriterProgressReporter;
#[cfg(feature = "log")]
pub use logger_progress_reporter::LoggerProgressReporter;
#[cfg(feature = "consumer-reporters")]
pub use metric_snapshot_progress_reporter::MetricSnapshotProgressReporter;
pub use no_op_progress_reporter::NoOpProgressReporter;
pub use stderr_progress_reporter::StderrProgressReporter;
pub use stdout_progress_reporter::StdoutProgressReporter;
pub use writer_progress_reporter::WriterProgressReporter;
