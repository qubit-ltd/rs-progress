// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Result of waiting for a running-progress wakeup.
pub(crate) enum RunningProgressWait {
    /// A worker requested a running report.
    Wake,
    /// No wakeup arrived before the positive report interval elapsed.
    Timeout,
    /// The loop was explicitly stopped.
    Stopped,
    /// Every notifier was dropped.
    Disconnected,
}

impl RunningProgressWait {
    /// Reports whether this wait outcome should produce a running event.
    ///
    /// # Returns
    ///
    /// `true` for a worker wakeup or interval timeout; otherwise, `false`.
    #[inline(always)]
    pub(crate) const fn should_report(self) -> bool {
        matches!(self, Self::Wake | Self::Timeout)
    }
}
