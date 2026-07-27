// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for reporter abstractions and built-in implementations.

mod format;
mod format_duration_tests;
#[cfg(feature = "consumer-reporters")]
mod formatted_progress_reporter_tests;
#[cfg(feature = "consumer-reporters")]
mod human_readable_progress_reporter_tests;
mod impls;
#[cfg(all(feature = "json", feature = "log"))]
mod json_logger_progress_reporter_tests;
#[cfg(all(feature = "json", feature = "consumer-reporters"))]
mod json_progress_reporter_tests;
#[cfg(feature = "log")]
mod logger_progress_reporter_tests;
#[cfg(feature = "consumer-reporters")]
mod metric_snapshot_progress_reporter_tests;
mod no_op_progress_reporter_tests;
mod progress_report_error_tests;
mod progress_reporter_tests;
mod writer_progress_reporter_tests;
