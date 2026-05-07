/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::sync::mpsc::Sender;

use crate::running_progress_signal::RunningProgressSignal;

/// Notifies a [`RunningProgressLoop`](crate::RunningProgressLoop) about progress
/// points and completion.
///
/// The notifier is cloneable so workers can share it cheaply. Sending a signal
/// returns `false` when the loop has already stopped or its receiver was
/// dropped.
///
/// # Examples
///
/// ```
/// use qubit_progress::RunningProgressLoop;
///
/// let (_progress_loop, notifier) = RunningProgressLoop::channel();
///
/// assert!(notifier.running_point());
/// assert!(notifier.stop());
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Clone)]
pub struct RunningProgressNotifier {
    /// Signal sender shared by callers and workers.
    pub(crate) signal_sender: Sender<RunningProgressSignal>,
}

impl RunningProgressNotifier {
    /// Sends a running progress point signal.
    ///
    /// # Returns
    ///
    /// `true` when the signal was sent, or `false` when the matching loop has
    /// already stopped.
    #[inline]
    pub fn running_point(&self) -> bool {
        self.signal_sender
            .send(RunningProgressSignal::RunningPoint)
            .is_ok()
    }

    /// Sends a stop signal.
    ///
    /// # Returns
    ///
    /// `true` when the signal was sent, or `false` when the matching loop has
    /// already stopped.
    #[inline]
    pub fn stop(&self) -> bool {
        self.signal_sender.send(RunningProgressSignal::Stop).is_ok()
    }
}
