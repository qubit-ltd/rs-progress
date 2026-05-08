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
/// [`crate::RunningProgressPointHandle`] handles for workers. Use [`Self::channel`]
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
///     let progress_point_handle = running_progress.point_handle();
///
///     // Worker code updates domain state first, then wakes the loop. With a
///     // zero interval, each running point may emit a `running` event.
///     for _ in 0..3 {
///         completed.fetch_add(1, Ordering::AcqRel);
///         assert!(progress_point_handle.report());
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

impl RunningProgressWait {
    /// Returns `true` when the running progress loop should call
    /// [`Progress::report_running_if_due`] after this wait result.
    #[inline]
    fn should_report(self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Disconnected => false,
            Self::Signal(signal) => match signal {
                RunningProgressSignal::RunningPoint => true,
                RunningProgressSignal::Stop => false,
            },
        }
    }
}

impl RunningProgressLoop {
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

    /// Creates a paired running progress loop and notifier.
    ///
    /// # Returns
    ///
    /// A loop that owns the signal receiver and a notifier that sends wakeup or
    /// stop signals to that loop.
    pub fn channel() -> (Self, RunningProgressNotifier) {
        let (signal_sender, signal_receiver) = mpsc::channel();
        (
            Self { signal_receiver },
            RunningProgressNotifier { signal_sender },
        )
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
        while self.receive_wait(report_interval).should_report() {
            progress.report_running_if_due(snapshot());
        }
    }

    /// Waits once on the signal channel and maps the outcome to [`RunningProgressWait`].
    ///
    /// The calling thread blocks until this wait completes.
    ///
    /// When `report_interval` is [`Duration::ZERO`], uses [`Receiver::recv`]: the call returns when a
    /// [`RunningProgressSignal`] is received or when every notifier sender has been dropped. In this
    /// mode no [`RunningProgressWait::Timeout`] is produced.
    ///
    /// When `report_interval` is positive, uses [`Receiver::recv_timeout`]: if no message arrives
    /// before the deadline, returns [`RunningProgressWait::Timeout`] so [`Self::run`] can drive periodic
    /// `running` progress; if a message arrives first, returns that signal wrapped in
    /// [`RunningProgressWait::Signal`], or [`RunningProgressWait::Disconnected`] if the channel closes.
    ///
    /// # Parameters
    ///
    /// * `report_interval` - Configured report interval from the [`Progress`] run passed to [`Self::run`];
    ///   [`Duration::ZERO`] selects unbounded waits (event-driven only), otherwise each wait is capped
    ///   by this duration and may time out.
    ///
    /// # Returns
    ///
    /// * [`RunningProgressWait::Signal`] - The next [`RunningProgressSignal`] from a notifier.
    /// * [`RunningProgressWait::Timeout`] - Only when `report_interval` is positive and the wait reached
    ///   the deadline without a message.
    /// * [`RunningProgressWait::Disconnected`] - The MPSC channel has no senders left.
    fn receive_wait(&self, report_interval: Duration) -> RunningProgressWait {
        if report_interval.is_zero() {
            return match self.signal_receiver.recv() {
                Ok(signal) => RunningProgressWait::Signal(signal),
                Err(_) => RunningProgressWait::Disconnected,
            };
        }
        match self.signal_receiver.recv_timeout(report_interval) {
            Ok(signal) => RunningProgressWait::Signal(signal),
            Err(RecvTimeoutError::Timeout) => RunningProgressWait::Timeout,
            Err(RecvTimeoutError::Disconnected) => RunningProgressWait::Disconnected,
        }
    }
}
