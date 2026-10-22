// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Handle controlling one scoped automatic reporter.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::ScopedJoinHandle;

use super::AutoReporterInner;
use super::AutoReporterStatus;
use super::ProgressNotifier;
use super::wake;
use crate::AutoReporterError;
use crate::EmissionError;
use crate::Progress;
use crate::WorkerPanic;

/// Handle controlling one scoped automatic reporter.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, Progress};
/// use qubit_progress::TextReporter;
///
/// let reporter = TextReporter::new(Vec::new());
/// let mut progress = Progress::builder(&reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// std::thread::scope(|scope| {
///     let auto_reporter = progress.spawn_auto_reporter(scope);
///     auto_reporter.stop()?;
///     Ok::<(), qubit_progress::AutoReporterError>(())
/// })?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct AutoReporter<'scope, 'reporter> {
    /// Scoped worker result, present only for enabled operations.
    pub(super) join: Option<ScopedJoinHandle<'scope, Result<(), EmissionError>>>,
    /// Shared wake and stop controls.
    pub(super) inner: Option<Arc<AutoReporterInner>>,
    /// State observable by workers.
    pub(super) status: AutoReporterStatus,
    /// Ties the original mutable Progress borrow to this handle's lifetime.
    pub(super) progress_borrow: PhantomData<&'scope mut Progress<'reporter>>,
}

impl<'scope, 'reporter> AutoReporter<'scope, 'reporter> {
    /// Returns a cloneable worker notification handle.
    #[must_use]
    #[inline]
    pub fn notifier(&self) -> ProgressNotifier {
        ProgressNotifier {
            inner: self
                .inner
                .as_ref()
                .and_then(|inner| inner.notification_driven.then(|| Arc::downgrade(inner))),
        }
    }

    /// Returns a cloneable status view for workers.
    #[must_use]
    #[inline]
    pub fn status(&self) -> AutoReporterStatus {
        self.status.clone()
    }

    /// Stops, joins and returns the background report result.
    ///
    /// A reporter or snapshot error is returned as
    /// [`AutoReporterError::Emission`]. A worker panic is captured as
    /// [`AutoReporterError::Panicked`] after the worker has been joined.
    ///
    /// # Errors
    ///
    /// Returns a reporter emission failure or a structured worker panic.
    pub fn stop(mut self) -> Result<(), AutoReporterError> {
        self.signal_stop();
        match self.join_worker() {
            Ok(result) => result.map_err(AutoReporterError::Emission),
            Err(panic) => Err(AutoReporterError::Panicked(panic)),
        }
    }

    /// Marks stop and wakes the reporter if it is blocked.
    fn signal_stop(&self) {
        if let Some(inner) = &self.inner {
            inner.stopped.store(true, Ordering::Release);
            wake(&inner.wake_sender);
        }
    }

    /// Joins the scoped worker once and returns either its result or panic.
    fn join_worker(&mut self) -> Result<Result<(), EmissionError>, WorkerPanic> {
        let Some(join) = self.join.take() else {
            return Ok(Ok(()));
        };
        join.join().map_err(WorkerPanic::new)
    }
}

impl Drop for AutoReporter<'_, '_> {
    /// Stops and joins a forgotten reporter without silently leaving a thread.
    fn drop(&mut self) {
        self.signal_stop();
        match self.join_worker() {
            Ok(Ok(())) => {}
            Ok(Err(_)) => self.status.mark_failed(),
            Err(_) => self.status.mark_failed(),
        }
    }
}
