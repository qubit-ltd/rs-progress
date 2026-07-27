// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{SyncSender, TrySendError},
};

/// Notifies a running progress loop about progress points and completion.
///
/// The notifier is cloneable so workers can share it cheaply. Sending a signal
/// returns `false` when the loop has already stopped or its receiver was
/// dropped.
///
/// # Examples
///
/// This is an internal helper used by
/// [`RunningProgressGuard`](crate::RunningProgressGuard).
///
/// # Author
///
/// Haixing Hu
#[derive(Clone)]
pub(crate) struct RunningProgressNotifier {
    /// Capacity-one wakeup sender shared by callers and workers.
    pub(crate) wake_sender: SyncSender<()>,
    /// Stop state checked independently from coalesced wakeups.
    pub(crate) stopped: Arc<AtomicBool>,
    /// Whether at least one worker point awaits a coalesced report.
    pub(crate) pending: Arc<AtomicBool>,
}

impl RunningProgressNotifier {
    /// Sends a running progress point signal.
    ///
    /// # Returns
    ///
    /// `true` when the signal was sent, or `false` when the matching loop has
    /// already stopped.
    #[inline]
    pub(crate) fn running_point(&self) -> bool {
        if self.stopped.load(Ordering::Acquire) {
            return false;
        }
        self.pending.store(true, Ordering::Release);
        match self.wake_sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => true,
            Err(TrySendError::Disconnected(())) => false,
        }
    }

    /// Sends a stop signal.
    ///
    /// # Returns
    ///
    /// `true` when the signal was sent, or `false` when the matching loop has
    /// already stopped.
    #[inline]
    pub(crate) fn stop(&self) -> bool {
        let was_running = !self.stopped.swap(true, Ordering::AcqRel);
        let _ = self.wake_sender.try_send(());
        was_running
    }
}
