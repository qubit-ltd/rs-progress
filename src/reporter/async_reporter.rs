// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Runtime-independent asynchronous event delivery.

use std::future::Future;
use std::pin::Pin;

use crate::Event;
use crate::ReporterError;

/// Boxed future returned by [`AsyncReporter::report`].
pub type ReportFuture<'a> = Pin<Box<dyn Future<Output = Result<(), ReporterError>> + Send + 'a>>;

/// Asynchronously delivers complete immutable progress events.
///
/// The future may borrow both the reporter and event until it resolves. If a
/// caller drops it before completion, delivery status is unknown: the sink may
/// already have accepted the event. Callers should not retry terminal events
/// automatically because that could duplicate a delivered event.
///
/// # Examples
///
/// Implement an asynchronous sink by returning a future that completes the
/// delivery and preserves any sink error in [`ReporterError`].
///
/// ```
/// use qubit_progress::{AsyncReporter, Event, ReportFuture, ReporterError};
///
/// struct Sink;
///
/// impl AsyncReporter for Sink {
///     fn report<'a>(&'a self, _event: &'a Event) -> ReportFuture<'a> {
///         Box::pin(async { Ok(()) })
///     }
/// }
///
/// let sink = Sink;
/// assert!(sink.is_enabled());
/// ```
pub trait AsyncReporter: Send + Sync {
    /// Returns whether a new operation should emit events.
    ///
    /// [`crate::AsyncProgressBuilder::start_async`] samples this once per
    /// operation.
    ///
    /// # Returns
    ///
    /// `true` when the operation should emit events; otherwise, `false`.
    #[must_use]
    fn is_enabled(&self) -> bool {
        true
    }

    /// Delivers one complete event.
    ///
    /// # Parameters
    ///
    /// * `event` - Complete immutable progress data to deliver.
    ///
    /// # Returns
    ///
    /// A future that resolves when delivery completes. Dropping it early leaves
    /// the delivery status unknown.
    ///
    /// # Errors
    ///
    /// Returns the sink's failure while preserving its original error source.
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a>;
}
