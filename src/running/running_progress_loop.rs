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
    sync::mpsc::{
        self,
        Receiver,
        RecvTimeoutError,
    },
    thread,
    time::Duration,
};

use crate::{
    Progress,
    model::ProgressCounters,
};

use super::{
    running_progress_notifier::RunningProgressNotifier,
    running_progress_signal::RunningProgressSignal,
    scoped_running_progress::ScopedRunningProgress,
};

/// Runs periodic `running` progress reports for work tracked elsewhere.
///
/// `RunningProgressLoop` is useful when worker threads update shared state and
/// a separate reporter thread should periodically emit `running` events. The
/// loop owns only the signal receiver. Callers provide a [`Progress`] instance
/// and a snapshot closure that converts their domain state into
/// [`ProgressCounters`].
///
/// Use [`Self::spawn_scoped`] when the reporter thread can be scoped to the
/// operation call. It returns a [`ScopedRunningProgress`] guard and cloneable
/// [`crate::RunningProgressPoints`] handles for workers. Use [`Self::channel`]
/// only when callers need to own the lower-level loop and notifier directly.
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
///             // The background reporter thread does not own the operation
///             // state. It only reads a fresh counter snapshot when the
///             // interval is due or a worker sends a running point.
///             ProgressCounters::new(Some(3))
///                 .with_completed_count(loop_completed.load(Ordering::Acquire))
///         });
///     let progress_points = running_progress.points();
///
///     // Worker code updates domain state first, then wakes the loop. With a
///     // zero interval, each running point may emit a `running` event.
///     for _ in 0..3 {
///         completed.fetch_add(1, Ordering::AcqRel);
///         assert!(progress_points.running_point());
///     }
///
///     // Stop the loop before leaving the scope. Reporter panics are
///     // propagated by `stop_and_join`.
///     running_progress.stop_and_join();
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct RunningProgressLoop {
    /// Signal receiver owned by the reporter loop.
    signal_receiver: Receiver<RunningProgressSignal>,
}

/// Result of waiting for a running progress loop signal.
enum RunningProgressWait {
    /// A worker or stop signal was received.
    Signal(RunningProgressSignal),
    /// No signal arrived before the positive report interval elapsed.
    Timeout,
    /// All senders were dropped.
    Disconnected,
}

impl RunningProgressLoop {
    /// Creates a paired running progress loop and notifier.
    ///
    /// # Returns
    ///
    /// A loop that owns the signal receiver and a notifier that sends wakeup or
    /// stop signals to that loop.
    #[inline]
    pub fn channel() -> (Self, RunningProgressNotifier) {
        let (signal_sender, signal_receiver) = mpsc::channel();
        (
            Self { signal_receiver },
            RunningProgressNotifier { signal_sender },
        )
    }

    /// Spawns a scoped reporter thread for running progress events.
    ///
    /// # Parameters
    ///
    /// * `scope` - Thread scope that owns the reporter thread.
    /// * `progress` - Progress run used by the reporter thread.
    /// * `snapshot` - Closure that returns current counters whenever a
    ///   `running` event may be due.
    ///
    /// # Returns
    ///
    /// A guard that can create worker point handles and stop the scoped
    /// reporter thread.
    #[inline]
    pub fn spawn_scoped<'scope, 'env, 'progress, F>(
        scope: &'scope thread::Scope<'scope, 'env>,
        progress: Progress<'progress>,
        snapshot: F,
    ) -> ScopedRunningProgress<'scope>
    where
        'progress: 'scope,
        F: FnMut() -> ProgressCounters + Send + 'scope,
    {
        let report_points = progress.report_interval().is_zero();
        let (progress_loop, notifier) = Self::channel();
        let progress_thread = scope.spawn(move || {
            progress_loop.run(progress, snapshot);
        });
        ScopedRunningProgress::new(notifier, progress_thread, report_points)
    }

    /// Runs until a stop signal is received or every notifier is dropped.
    ///
    /// # Parameters
    ///
    /// * `progress` - Progress run used to emit `running` events.
    /// * `snapshot` - Closure that returns the current counters whenever a
    ///   `running` event may be due.
    ///
    /// # Panics
    ///
    /// Propagates panics from the configured reporter when a `running` event is
    /// due.
    pub fn run<F>(self, mut progress: Progress<'_>, mut snapshot: F)
    where
        F: FnMut() -> ProgressCounters,
    {
        let report_interval = progress.report_interval();
        while let RunningProgressWait::Signal(RunningProgressSignal::RunningPoint)
        | RunningProgressWait::Timeout =
            receive_running_progress_signal(&self.signal_receiver, report_interval)
        {
            progress.report_running_if_due(snapshot());
        }
    }
}

/// Receives one running progress loop signal.
///
/// # Parameters
///
/// * `signal_receiver` - Signal receiver shared with notifiers.
/// * `report_interval` - Configured progress-report interval.
///
/// # Returns
///
/// A worker or stop signal, a timeout marker for positive intervals, or a
/// disconnected marker when all notifiers have disconnected.
fn receive_running_progress_signal(
    signal_receiver: &Receiver<RunningProgressSignal>,
    report_interval: Duration,
) -> RunningProgressWait {
    if report_interval.is_zero() {
        return match signal_receiver.recv() {
            Ok(signal) => RunningProgressWait::Signal(signal),
            Err(_) => RunningProgressWait::Disconnected,
        };
    }
    match signal_receiver.recv_timeout(report_interval) {
        Ok(signal) => RunningProgressWait::Signal(signal),
        Err(RecvTimeoutError::Timeout) => RunningProgressWait::Timeout,
        Err(RecvTimeoutError::Disconnected) => RunningProgressWait::Disconnected,
    }
}
