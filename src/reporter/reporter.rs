// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter trait for complete immutable events.

use crate::Event;
use crate::ReporterError;

/// Synchronously receives complete immutable progress events from an operation.
///
/// Implementations must be safe to share across threads. A call to
/// [`report`](Self::report) runs synchronously on the caller's thread, so slow
/// sinks can delay progress updates. Return a [`ReporterError`] when delivery
/// fails; the progress operation retains that error for its terminal result.
///
/// # Examples
///
/// A closure can act as a reporter when it is thread-safe and returns the
/// reporter's concrete error type.
///
/// ```
/// use qubit_progress::{Event, Reporter, ReporterError};
///
/// let reporter = |_event: &Event| -> Result<(), ReporterError> { Ok(()) };
/// assert!(reporter.is_enabled());
/// ```
pub trait Reporter: Send + Sync {
    /// Reports whether a newly built operation should emit events.
    ///
    /// [`crate::ProgressBuilder::start`] samples this value once when starting
    /// an operation; changing it later does not affect that operation.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Delivers one complete immutable event before returning to the caller.
    ///
    /// Return [`ReporterError`] if the sink cannot accept the event. The error
    /// preserves the sink's original source for inspection by the operation's
    /// caller. The event is borrowed only for this call.
    fn report(&self, event: &Event) -> Result<(), ReporterError>;
}

impl<F> Reporter for F
where
    F: Fn(&Event) -> Result<(), ReporterError> + Send + Sync,
{
    /// Invokes this closure synchronously for one complete event.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self(event)
    }
}
