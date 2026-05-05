/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Generic progress reporting abstractions.
//!
//! This crate models progress as immutable events carrying lifecycle phase,
//! optional stage information, counters, timing, and caller-defined context.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod logger_progress_reporter;
mod no_op_progress_reporter;
mod progress_counters;
mod progress_event;
mod progress_format;
mod progress_phase;
mod progress_reporter;
mod progress_stage;
mod writer_progress_reporter;

pub use logger_progress_reporter::LoggerProgressReporter;
pub use no_op_progress_reporter::NoOpProgressReporter;
pub use progress_counters::ProgressCounters;
pub use progress_event::ProgressEvent;
pub use progress_phase::ProgressPhase;
pub use progress_reporter::ProgressReporter;
pub use progress_stage::ProgressStage;
pub use writer_progress_reporter::WriterProgressReporter;
