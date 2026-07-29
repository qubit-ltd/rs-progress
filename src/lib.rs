// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable, lifecycle-safe progress reporting.
//!
//! A [`Progress`] operation owns its metric state, timing and reporter.
//! Callers configure stable metadata with [`Metric`] and update dynamic counts
//! through cloneable [`MetricHandle`] values. Every emitted [`Event`] is
//! complete.
//!
//! # Examples
//!
//! ```
//! use qubit_progress::{Metric, Progress, TextReporter};
//!
//! let reporter = TextReporter::new(Vec::new());
//! let progress = Progress::builder(&reporter)
//!     .metric(Metric::new("tasks", "Tasks").total(1))
//!     .start()?;
//! let tasks = progress.metric("tasks").expect("configured metric must exist");
//! tasks.start(1)?;
//! tasks.succeed(1)?;
//! progress.finish()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod auto_reporter;
mod error;
mod event;
mod metric;
mod progress;
pub mod reporter;
mod stage;
mod validation;

pub use auto_reporter::{
    AutoReporter,
    Notifier,
    Status,
};
pub use error::{
    MetricError,
    MetricTransition,
    ProgressError,
    ReportError,
    TerminalError,
    ValidationError,
};
pub use event::{
    Event,
    Phase,
};
pub use metric::{
    Metric,
    MetricHandle,
    MetricSnapshot,
};
pub use progress::{
    Progress,
    ProgressBuilder,
};
#[cfg(feature = "json-lines")]
pub use reporter::JsonLinesReporter;
#[cfg(feature = "log")]
pub use reporter::LogReporter;
pub use reporter::{
    NoopReporter,
    Reporter,
    TextReporter,
};
pub use stage::Stage;
