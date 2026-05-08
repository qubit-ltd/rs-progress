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
    running_progress_point_handle::RunningProgressPointHandle,
};

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
///     ProgressCounters,
/// };
///
/// let reporter = NoOpProgressReporter;
/// let completed = Arc::new(AtomicUsize::new(0));
///
/// thread::scope(|scope| {
///     let loop_completed = Arc::clone(&completed);
///     let progress = Progress::new(&reporter, Duration::ZERO);
///     let running_progress =
///         progress.spawn_running_reporter(scope, move || {
///             ProgressCounters::new(Some(3))
///                 .with_completed_count(loop_completed.load(Ordering::Acquire))
///         });
///     let progress_point_handle = running_progress.point_handle();
///
///     let mut handles = Vec::new();
///     for _ in 0..3 {
///         let c = Arc::clone(&completed);
///         let p = progress_point_handle.clone();
///         handles.push(scope.spawn(move || {
///             c.fetch_add(1, Ordering::AcqRel);
///             assert!(p.report());
///         }));
///     }
///     for h in handles {
///         h.join().unwrap();
///     }
///
///     running_progress.stop_and_join();
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct RunningProgressGuard<'scope> {
    /// Notifier used to stop the reporter thread.
    notifier: RunningProgressNotifier,
    /// Scoped reporter thread handle.
    progress_thread: ScopedJoinHandle<'scope, ()>,
    /// Whether worker point notifications should wake the reporter loop.
    report_points: bool,
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
    pub fn point_handle(&self) -> RunningProgressPointHandle {
        RunningProgressPointHandle::new(self.report_points.then(|| self.notifier.clone()))
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
