/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use super::running_progress_notifier::RunningProgressNotifier;

/// Worker-side handle for reporting running progress points.
///
/// `RunningProgressPoints` deliberately cannot stop or join the reporter
/// thread. It only wakes the reporter loop for zero-interval progress. For
/// positive intervals, [`Self::running_point`] is a no-op because the reporter
/// loop wakes itself on timeout.
///
/// # Examples
///
/// ```
/// use std::{
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
///
/// thread::scope(|scope| {
///     let progress = Progress::new(&reporter, Duration::ZERO);
///     let running_progress =
///         RunningProgressLoop::spawn_scoped(scope, progress, || {
///             ProgressCounters::new(Some(1)).with_completed_count(1)
///         });
///     let progress_points = running_progress.points();
///
///     assert!(progress_points.running_point());
///
///     running_progress.stop_and_join();
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Clone)]
pub struct RunningProgressPoints {
    /// Optional notifier used only when worker points should wake the loop.
    notifier: Option<RunningProgressNotifier>,
}

impl RunningProgressPoints {
    /// Creates a worker-side running point handle.
    ///
    /// # Parameters
    ///
    /// * `notifier` - Optional notifier used for zero-interval point signals.
    ///
    /// # Returns
    ///
    /// A worker-side handle that reports points or no-ops by interval policy.
    #[inline]
    pub(crate) const fn new(notifier: Option<RunningProgressNotifier>) -> Self {
        Self { notifier }
    }

    /// Reports one worker running progress point.
    ///
    /// # Returns
    ///
    /// `true` when the point was accepted or no point signal is required.
    /// Returns `false` only when a required zero-interval signal could not be
    /// sent because the reporter loop has already stopped.
    #[inline]
    pub fn running_point(&self) -> bool {
        match self.notifier.as_ref() {
            Some(notifier) => notifier.running_point(),
            None => true,
        }
    }
}
