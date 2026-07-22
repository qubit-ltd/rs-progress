// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::running_progress_notifier::RunningProgressNotifier;

/// Worker-side handle for reporting running progress points.
///
/// `RunningProgressPointHandle` deliberately cannot stop or join the reporter
/// thread. It only wakes the reporter loop for zero-interval progress. For
/// positive intervals, [`Self::report`] is a no-op because the reporter
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
///     ProgressCounter,
///     ProgressSchema,
/// };
///
/// let reporter = NoOpProgressReporter;
///
/// thread::scope(|scope| {
///     let progress = Progress::new(
///         &reporter,
///         Duration::ZERO,
///         ProgressSchema::single("entries", "Entries"),
///     );
///     let running_progress = progress.spawn_running_reporter(scope, || {
///         vec![ProgressCounter::new("entries").total(1).completed(1)]
///     });
///     let progress_point_handle = running_progress.point_handle();
///
///     let worker = scope.spawn({
///         let progress_point_handle = progress_point_handle.clone();
///         move || {
///             progress_point_handle.report();
///         }
///     });
///     worker.join().unwrap();
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
#[derive(Clone)]
pub struct RunningProgressPointHandle {
    /// Optional notifier used only when worker points should wake the loop.
    notifier: Option<RunningProgressNotifier>,
}

impl RunningProgressPointHandle {
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

    /// Reports one worker running progress point on a best-effort basis.
    ///
    /// Repeated points may be coalesced while a report is already pending. Use
    /// [`Self::try_report`] only when the caller needs to know whether the
    /// reporter loop is still accepting notifications.
    #[inline(always)]
    pub fn report(&self) {
        let _ = self.try_report();
    }

    /// Attempts to report one worker running progress point.
    ///
    /// # Returns
    ///
    /// `true` when the point was accepted, coalesced with a pending point, or
    /// no point notification is required. Returns `false` after the reporter
    /// loop stops or disconnects.
    #[inline(always)]
    pub fn try_report(&self) -> bool {
        match self.notifier.as_ref() {
            Some(notifier) => notifier.running_point(),
            None => true,
        }
    }
}
