// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter abstraction and built-in sinks.

#[cfg(feature = "json-lines")]
mod json_lines_reporter;
#[cfg(feature = "log")]
mod log_reporter;
mod noop_reporter;
#[allow(clippy::module_inception)]
mod reporter;
mod text_reporter;

#[cfg(feature = "json-lines")]
pub use json_lines_reporter::JsonLinesReporter;
#[cfg(feature = "log")]
pub use log_reporter::LogReporter;
pub use noop_reporter::NoopReporter;
pub use reporter::Reporter;
pub use text_reporter::TextReporter;
