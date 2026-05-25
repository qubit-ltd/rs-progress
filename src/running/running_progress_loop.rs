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
    model::ProgressCounter,
};

use super::{
    running_progress_guard::RunningProgressGuard,
    running_progress_notifier::RunningProgressNotifier,
    running_progress_signal::RunningProgressSignal,
};

/// Runs periodic `running` progress reports for work tracked elsewhere.
///
/// `RunningProgressLoop` is useful when worker threads update shared state and
/// a separate reporter thread should periodically emit `running` events. The
/// loop owns only the signal receiver. Callers provide a [`Progress`] instance
/// and a snapshot closure that converts their domain state into metric counters.
pub(crate) struct RunningProgressLoop {
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
    pub(crate) fn spawn_scoped<'scope, 'env, 'progress, F>(
        scope: &'scope thread::Scope<'scope, 'env>,
        progress: Progress<'progress>,
        snapshot: F,
    ) -> RunningProgressGuard<'scope>
    where
        'progress: 'scope,
        F: FnMut() -> Vec<ProgressCounter> + Send + 'scope,
    {
        let report_points = progress.report_interval().is_zero();
        let (progress_loop, notifier) = Self::channel();
        let progress_thread = scope.spawn(move || {
            progress_loop.run(progress, snapshot);
        });
        RunningProgressGuard::new(notifier, progress_thread, report_points)
    }

    /// Creates a paired running progress loop and notifier.
    ///
    /// # Returns
    ///
    /// A loop that owns the signal receiver and a notifier that sends wakeup or
    /// stop signals to that loop.
    pub(crate) fn channel() -> (Self, RunningProgressNotifier) {
        let (signal_sender, signal_receiver) = mpsc::channel();
        (Self { signal_receiver }, RunningProgressNotifier { signal_sender })
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
    /// Propagates panics from the configured reporter when a `running` event is due.
    pub(crate) fn run<F>(self, mut progress: Progress<'_>, mut snapshot: F)
    where
        F: FnMut() -> Vec<ProgressCounter>,
    {
        let report_interval = progress.report_interval();
        while self.receive_wait(report_interval).should_report() {
            progress.report_running_if_due(|event| event.counters(snapshot()));
        }
    }

    /// Waits once on the signal channel and maps the outcome to [`RunningProgressWait`].
    ///
    /// The calling thread blocks until this wait completes.
    ///
    /// # Parameters
    ///
    /// * `report_interval` - Configured report interval from the [`Progress`] run.
    ///
    /// # Returns
    ///
    /// The wait outcome used by the running loop.
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
