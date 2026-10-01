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
pub trait AsyncReporter: Send + Sync {
    /// Returns whether a new operation should emit events.
    ///
    /// [`crate::AsyncProgressBuilder::start_async`] samples this once per
    /// operation.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Delivers one complete event.
    ///
    /// The returned error preserves the sink's original error source.
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a>;
}
