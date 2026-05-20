/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::model::ProgressEvent;

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
///     fn report(&self, event: &ProgressEvent) {
///         self.phases.lock().expect("phase list should lock").push(event.phase());
///     }
/// }
///
/// let reporter = RecordingReporter::default();
/// let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
/// reporter.report(&ProgressEvent::started(schema, Vec::new(), Duration::ZERO));
///
/// assert_eq!(
///     reporter.phases.lock().expect("phase list should lock").as_slice(),
///     &[ProgressPhase::Started],
/// );
/// ```
pub trait ProgressReporter: Send + Sync {
    /// Reports one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Immutable progress event to report.
    ///
    /// # Panics
    ///
    /// Reporter implementations may panic if their output sink fails. Callers
    /// decide whether reporter panics are propagated or isolated.
    fn report(&self, event: &ProgressEvent);
}
