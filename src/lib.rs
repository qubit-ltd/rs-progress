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
//! # Benchmark interpretation
//!
//! Run `cargo bench --bench progress_bench -- --noplot` to compare complete
//! event delivery, scheduling paths, and concurrent [`MetricHandle`] updates
//! with a mutex-protected counter baseline. The contention benchmarks use
//! Criterion's batched iteration so setup allocation is excluded from the
//! measured update path; worker thread creation and joining remain part of the
//! workload. Throughput is reported in elements per second for 2,048 updates
//! per worker across 1, 2, 4, 8, 16, 32, and 64 workers.
//!
//! These measurements are workload- and hardware-dependent. The CAS-based
//! metric path is not assumed to beat a mutex at every worker count; measure
//! on target hardware before changing the synchronization strategy or adding
//! backoff, yielding, or sharded counters.
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
//! progress.finish().expect("complete operation must finish");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod auto_reporter;
mod error;
mod event;
mod internal;
mod metric;
mod progress;
pub mod reporter;
mod stage;
mod validation;

pub use auto_reporter::{AutoReporter, AutoReporterStatus, ProgressNotifier};
pub use error::{
    AutoReporterError, CompletionError, ConfigurationError, DeliveryError, EmissionError,
    FinishError, MetricError, MetricTransition, ReporterError, StartError, TerminalError,
    WorkerPanic,
};
pub use event::{Event, Phase};
pub use internal::OperationLifecycle;
pub use metric::{Metric, MetricHandle, MetricSnapshot};
pub use progress::{Progress, ProgressBuilder};
#[cfg(feature = "json-lines")]
pub use reporter::JsonLinesReporter;
#[cfg(feature = "log")]
pub use reporter::LogReporter;
pub use reporter::{NoopReporter, Reporter, TextReporter};
pub use stage::Stage;
