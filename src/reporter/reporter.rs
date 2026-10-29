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
/// fails; each reporting call returns its error directly, and errors are not
/// accumulated for a later terminal result. Automatic reporting exposes worker
/// failures through `AutoReporter::stop`.
///
/// # Examples
///
/// A closure can act as a reporter when it is thread-safe and returns the
/// reporter's concrete error type. A progress operation supplies each event
/// to the reporter as it runs.
///
/// ```
/// use qubit_progress::{Event, Metric, Progress, Reporter, ReporterError};
///
/// let reporter = |_event: &Event| -> Result<(), ReporterError> { Ok(()) };
/// let progress = Progress::builder(&reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// progress.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Reporter: Send + Sync {
    /// Reports whether a newly built operation should emit events.
    ///
    /// [`crate::ProgressBuilder::start`] samples this value once when starting
    /// an operation; changing it later does not affect that operation.
    ///
    /// # Returns
    ///
    /// `true` when the operation should emit events; otherwise, `false`.
    #[must_use]
    #[inline]
    fn is_enabled(&self) -> bool {
        true
    }

    /// Delivers one complete immutable event before returning to the caller.
    ///
    /// The event is borrowed only for this call.
    ///
    /// # Parameters
    ///
    /// - `event`: Complete immutable progress data to deliver.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the sink accepts the event.
    ///
    /// # Errors
    ///
    /// Returns [`ReporterError`] when the sink cannot accept the event. The
    /// error preserves the sink's original source for inspection by the
    /// operation's caller.
    fn report(&self, event: &Event) -> Result<(), ReporterError>;
}

impl<F> Reporter for F
where
    F: Fn(&Event) -> Result<(), ReporterError> + Send + Sync,
{
    /// Invokes this closure synchronously for one complete event.
    #[inline]
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self(event)
    }
}
