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
//! # Contention benchmark
//!
//! The contention benchmark compares concurrent [`MetricHandle`] updates with
//! a mutex-protected counter baseline. The measurements below were collected
//! with `cargo bench --bench progress_bench -- --noplot` on a six-CPU machine,
//! using 2,048 updates per worker. Throughput is reported in million elements
//! per second; each value is the Criterion median from the benchmark run.
//!
//! | Workers | `MetricHandle` | Mutex baseline |
//! | -------: | --------------: | --------------: |
//! | 1        | 39.1            | 39.9            |
//! | 2        | 32.7            | 32.6            |
//! | 4        | 15.1            | 14.6            |
//! | 8        | 15.6            | 12.7            |
//! | 16       | 13.9            | 15.4            |
//! | 32       | 10.2            | 19.5            |
//! | 64       | 12.3            | 15.9            |
//!
//! The CAS-based metric path is competitive for ordinary concurrency and does
//! not currently require further optimization. Revisit the implementation if
//! an application sustains substantially more update workers than available
//! CPU cores, especially around 32 or more workers, where retry and scheduler
//! contention can make the mutex baseline faster. In that case, measure on the
//! target hardware before considering bounded backoff, yielding, or sharded
//! counters.
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

pub use auto_reporter::{AutoReporter, Notifier, Status};
pub use error::{
    MetricError, MetricTransition, ProgressError, ReportError, TerminalError, ValidationError,
};
pub use event::{Event, Phase};
pub use metric::{Metric, MetricHandle, MetricSnapshot};
pub use progress::{Progress, ProgressBuilder};
#[cfg(feature = "json-lines")]
pub use reporter::JsonLinesReporter;
#[cfg(feature = "log")]
pub use reporter::LogReporter;
pub use reporter::{NoopReporter, Reporter, TextReporter};
pub use stage::Stage;
