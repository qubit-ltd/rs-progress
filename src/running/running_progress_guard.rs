// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{panic::resume_unwind, thread::ScopedJoinHandle};

use super::{
    running_progress_notifier::RunningProgressNotifier,
    running_progress_point_handle::RunningProgressPointHandle,
    running_progress_status::RunningProgressStatus,
};
use crate::ProgressReportError;

/// Owns a scoped running progress reporter thread.
///
/// `RunningProgressGuard` is created by
/// [`Progress::spawn_running_reporter`](crate::Progress::spawn_running_reporter).
/// Keep this guard on the coordinating thread, pass
/// [`RunningProgressPointHandle`] clones to workers, and call
/// [`Self::stop_and_join`] after worker execution completes.
///
/// # Examples
///
/// ```
/// use std::{
///     sync::{
///         Arc,
///         atomic::{
///             AtomicUsize,
///             Ordering,
///         },
///     },
///     thread,
///     time::Duration,
/// };
///
/// use qubit_progress::{
///     NoOpProgressReporter,
///     Progress,
///     ProgressCounter,
///     ProgressSchema,
/// };
///
/// let reporter = NoOpProgressReporter;
/// let completed = Arc::new(AtomicUsize::new(0));
///
/// thread::scope(|scope| {
///     let loop_completed = Arc::clone(&completed);
///     let progress = Progress::new(
///         &reporter,
///         Duration::ZERO,
///         ProgressSchema::single("entries", "Entries"),
///     );
///     let running_progress = progress.spawn_running_reporter(scope, move || {
///         vec![ProgressCounter::new("entries")
///             .total(3)
///             .completed(loop_completed.load(Ordering::Acquire) as u64)]
///     });
///     let progress_point_handle = running_progress.point_handle();
///
///     let mut handles = Vec::new();
///     for _ in 0..3 {
///         let c = Arc::clone(&completed);
///         let p = progress_point_handle.clone();
///         handles.push(scope.spawn(move || {
///             c.fetch_add(1, Ordering::AcqRel);
///             p.report();
///         }));
///     }
///     for h in handles {
///         h.join().unwrap();
///     }
///
///     running_progress
///         .stop_and_join()
///         .expect("progress output should succeed");
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
#[must_use = "the guard must be stopped and joined to finish reporter work"]
pub struct RunningProgressGuard<'scope> {
    /// Notifier used to stop the reporter thread.
    notifier: Option<RunningProgressNotifier>,
    /// Scoped reporter thread handle.
    progress_thread: Option<ScopedJoinHandle<'scope, Result<(), ProgressReportError>>>,
    /// Whether worker point notifications should wake the reporter loop.
    report_points: bool,
    /// Shared failure state for the reporter thread.
    status: RunningProgressStatus,
}

impl<'scope> RunningProgressGuard<'scope> {
    /// Creates a scoped running progress guard.
    ///
    /// # Parameters
    ///
    /// * `notifier` - Notifier used to stop the reporter thread.
    /// * `progress_thread` - Scoped reporter thread handle.
    /// * `report_points` - Whether worker point notifications wake the loop.
    ///
    /// # Returns
    ///
    /// A guard owning the reporter thread lifecycle.
    #[inline]
    pub(crate) const fn new(
        notifier: RunningProgressNotifier,
        progress_thread: ScopedJoinHandle<'scope, Result<(), ProgressReportError>>,
        report_points: bool,
        status: RunningProgressStatus,
    ) -> Self {
        Self {
            notifier: Some(notifier),
            progress_thread: Some(progress_thread),
            report_points,
            status,
        }
    }

    /// Creates an inactive guard for a disabled reporter.
    ///
    /// # Returns
    ///
    /// A guard that owns no notifier or reporter thread.
    #[inline]
    pub(crate) fn inactive() -> Self {
        Self {
            notifier: None,
            progress_thread: None,
            report_points: false,
            status: RunningProgressStatus::inactive(),
        }
    }

    /// Returns a worker-side running point handle.
    ///
    /// # Returns
    ///
    /// A cloneable handle that wakes the reporter loop for zero intervals and
    /// becomes a no-op for positive intervals.
    #[inline]
    pub fn point_handle(&self) -> RunningProgressPointHandle {
        let notifier = self
            .report_points
            .then(|| self.notifier.as_ref().cloned())
            .flatten();
        RunningProgressPointHandle::new(notifier)
    }

    /// Returns a shared status for the background reporter.
    ///
    /// # Returns
    ///
    /// A cloneable status that reports whether background progress delivery has
    /// failed. Use [`Self::stop_and_join`] to retrieve the concrete error.
    #[inline]
    pub fn status(&self) -> RunningProgressStatus {
        self.status.clone()
    }

    /// Stops the reporter loop and joins the scoped reporter thread.
    ///
    /// # Errors
    ///
    /// Returns the first output error produced by the background reporter.
    ///
    /// # Panics
    ///
    /// Propagates any panic raised by the reporter thread.
    #[inline]
    pub fn stop_and_join(self) -> Result<(), ProgressReportError> {
        if let Some(notifier) = self.notifier {
            notifier.stop();
        }
        if let Some(progress_thread) = self.progress_thread {
            match progress_thread.join() {
                Ok(result) => result?,
                Err(payload) => resume_unwind(payload),
            }
        }
        Ok(())
    }
}
