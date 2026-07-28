// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable, lifecycle-safe progress reporting.
//!
//! A [`Progress`] operation owns its stable configuration, timing and reporter.
//! Callers configure totals once with [`Metric`] and provide only current
//! dynamic counts through report closures. Every emitted [`Event`] is complete.
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
//! progress.finish(|snapshot| {
//!     snapshot.metric("tasks", |counts| {
//!         counts.completed(1).succeeded(1);
//!     });
//! })?;
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
    MetricCounts,
    MetricSnapshot,
};
pub use progress::{
    Progress,
    ProgressBuilder,
    Snapshot,
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
