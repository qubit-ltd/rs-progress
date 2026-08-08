// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable, lifecycle-safe progress reporting.
// qubit-style: allow coverage-cfg
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
//! let reporter = std::sync::Arc::new(TextReporter::new(Vec::new()));
//! let progress = Progress::builder_arc(reporter)
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
mod internal;
mod metric;
mod operation_attributes;
mod progress;
pub mod reporter;
mod stage;
mod validation;

pub use auto_reporter::AutoReporter;
pub use auto_reporter::AutoReporterStatus;
pub use auto_reporter::ProgressNotifier;
pub use error::AutoReporterError;
pub use error::CompletionError;
pub use error::ConfigurationError;
pub use error::DeliveryError;
pub use error::EmissionError;
pub use error::FinishError;
pub use error::MetricError;
pub use error::RecoverableFinishError;
pub use error::ReporterError;
pub use error::StartError;
pub use error::TerminalError;
pub use error::WorkerPanic;
#[cfg(all(feature = "json-lines", coverage))]
#[doc(hidden)]
pub use event::__coverage_event_serde;
pub use event::Event;
pub use event::Phase;
#[cfg(coverage)]
#[doc(hidden)]
pub use internal::__coverage_internal;
pub use internal::OperationLifecycle;
pub use metric::Metric;
pub use metric::MetricDelta;
pub use metric::MetricHandle;
pub use metric::MetricSnapshot;
pub use operation_attributes::OperationAttributes;
#[cfg(coverage)]
#[doc(hidden)]
pub use progress::__coverage_progress_edges;
pub use progress::Progress;
pub use progress::ProgressBuilder;
#[cfg(feature = "json-lines")]
pub use reporter::JsonLinesReporter;
#[cfg(feature = "log")]
pub use reporter::LogReporter;
pub use reporter::NoopReporter;
pub use reporter::Reporter;
pub use reporter::TextReporter;
pub use stage::Stage;
