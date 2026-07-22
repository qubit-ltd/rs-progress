// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{
    model::ProgressEvent,
    reporter::ProgressReportError,
};

/// Receives immutable progress events for one logical operation.
///
/// A reporter normally receives one logical operation's event stream. If an
/// implementation multiplexes multiple operations into one sink, that routing
/// information is reporter-specific metadata and is intentionally not part of
/// [`ProgressEvent`].
///
/// # Examples
///
/// ```
/// use std::sync::Mutex;
/// use std::time::Duration;
///
/// use qubit_progress::{
///     ProgressEvent,
///     ProgressMetric,
///     ProgressPhase,
///     ProgressReporter,
///     ProgressSchema,
/// };
///
/// #[derive(Default)]
/// struct RecordingReporter {
///     phases: Mutex<Vec<ProgressPhase>>,
/// }
///
/// impl ProgressReporter for RecordingReporter {
///     fn report(
///         &self,
///         event: &ProgressEvent,
///     ) -> Result<(), qubit_progress::ProgressReportError> {
///         self.phases.lock().expect("phase list should lock").push(event.phase());
///         Ok(())
///     }
/// }
///
/// let reporter = RecordingReporter::default();
/// let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
/// reporter
///     .report(&ProgressEvent::started(schema, Vec::new(), Duration::ZERO))
///     .expect("recording reporter should accept event");
///
/// assert_eq!(
///     reporter.phases.lock().expect("phase list should lock").as_slice(),
///     &[ProgressPhase::Started],
/// );
/// ```
pub trait ProgressReporter: Send + Sync {
    /// Reports whether this reporter currently accepts events.
    ///
    /// Disabled reporters allow callers to skip snapshot creation, event
    /// formatting, and background reporter threads.
    ///
    /// # Returns
    ///
    /// `true` when reporting work should be performed; otherwise, `false`.
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        true
    }

    /// Reports one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Immutable progress event to report.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured output sink rejects the event.
    fn report(&self, event: &ProgressEvent) -> Result<(), ProgressReportError>;
}
