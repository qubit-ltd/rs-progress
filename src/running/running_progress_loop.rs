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
    time::Duration,
};

use crate::{
    Progress,
    model::ProgressCounters,
};

use super::{
    running_progress_notifier::RunningProgressNotifier,
    running_progress_signal::RunningProgressSignal,
};

/// Runs periodic `running` progress reports for work tracked elsewhere.
///
/// `RunningProgressLoop` is useful when worker threads update shared state and
/// a separate reporter thread should periodically emit `running` events. The
/// loop owns only the signal receiver. Callers provide a [`Progress`] instance
/// and a snapshot closure that converts their domain state into
/// [`ProgressCounters`].
///
/// Use [`Self::channel`] to create a loop and its matching
/// [`RunningProgressNotifier`]. Move the loop into a reporter thread, clone the
/// notifier into workers when zero-interval wakeups are needed, and send
/// [`RunningProgressNotifier::stop`] when the operation is complete.
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
/// let (progress_loop, notifier) = RunningProgressLoop::channel();
///
/// thread::scope(|scope| {
///     let loop_completed = Arc::clone(&completed);
///     let reporter_ref = &reporter;
///     let progress_thread = scope.spawn(move || {
///         // This background reporter thread owns the loop. It does not own
///         // the operation state; it only reads a fresh counter snapshot when
///         // the interval is due or a worker sends a running point.
///         let progress = Progress::new(reporter_ref, Duration::ZERO);
///         progress_loop.run(progress, || {
///             ProgressCounters::new(Some(3))
///                 .with_completed_count(loop_completed.load(Ordering::Acquire))
///         });
///     });
///
///     // Worker code updates domain state first, then wakes the loop. With a
///     // zero interval, each running point may emit a `running` event.
///     for _ in 0..3 {
///         completed.fetch_add(1, Ordering::AcqRel);
///         assert!(notifier.running_point());
///     }
///
///     // Stop the loop before leaving the scope so reporter panics can be
///     // propagated through the join handle.
///     assert!(notifier.stop());
///     progress_thread.join().expect("progress loop should stop");
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
