/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::{
    panic::resume_unwind,
    thread::ScopedJoinHandle,
};

use super::{
    running_progress_notifier::RunningProgressNotifier,
    running_progress_points::RunningProgressPoints,
};

/// Owns a scoped running progress reporter thread.
///
/// `ScopedRunningProgress` is a lifecycle guard for a reporter thread created
/// by [`crate::RunningProgressLoop::spawn_scoped`]. Keep this guard on the
/// coordinating thread, pass [`RunningProgressPoints`] clones to workers, and
/// call [`Self::stop_and_join`] after worker execution completes.
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
///     ProgressCounters,
///     RunningProgressLoop,
/// };
///
/// let reporter = NoOpProgressReporter;
/// let completed = Arc::new(AtomicUsize::new(0));
///
/// thread::scope(|scope| {
///     let loop_completed = Arc::clone(&completed);
///     let progress = Progress::new(&reporter, Duration::ZERO);
///     let running_progress =
///         RunningProgressLoop::spawn_scoped(scope, progress, move || {
///             ProgressCounters::new(Some(1))
///                 .with_completed_count(loop_completed.load(Ordering::Acquire))
///         });
///     let progress_points = running_progress.points();
///
///     completed.store(1, Ordering::Release);
///     assert!(progress_points.running_point());
///
///     running_progress.stop_and_join();
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct ScopedRunningProgress<'scope> {
    /// Notifier used to stop the reporter thread.
    notifier: RunningProgressNotifier,
    /// Scoped reporter thread handle.
    progress_thread: ScopedJoinHandle<'scope, ()>,
    /// Whether worker point notifications should wake the reporter loop.
    report_points: bool,
}

impl<'scope> ScopedRunningProgress<'scope> {
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
        progress_thread: ScopedJoinHandle<'scope, ()>,
        report_points: bool,
    ) -> Self {
        Self {
            notifier,
            progress_thread,
            report_points,
        }
    }

    /// Returns a worker-side running point handle.
    ///
    /// # Returns
    ///
    /// A cloneable handle that wakes the reporter loop for zero intervals and
    /// becomes a no-op for positive intervals.
    #[inline]
    pub fn points(&self) -> RunningProgressPoints {
        RunningProgressPoints::new(self.report_points.then(|| self.notifier.clone()))
    }

    /// Stops the reporter loop and joins the scoped reporter thread.
    ///
    /// # Panics
    ///
    /// Propagates any panic raised by the reporter thread.
    #[inline]
    pub fn stop_and_join(self) {
        self.notifier.stop();
        if let Err(payload) = self.progress_thread.join() {
            resume_unwind(payload);
        }
    }
}
