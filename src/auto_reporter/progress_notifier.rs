// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Coalesced notification handle for an automatic reporter.

use std::sync::Weak;
use std::sync::atomic::Ordering;

use super::internal::AutoReporterInner;
use super::internal::wake;

/// Notification handle that coalesces state changes without claiming delivery.
///
/// # Examples
///
/// ```
/// use qubit_progress::Metric;
/// use qubit_progress::Progress;
/// use qubit_progress::TextReporter;
///
/// let reporter = TextReporter::new(Vec::new());
/// let mut progress = Progress::builder(&reporter)
///     .interval(std::time::Duration::ZERO)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// std::thread::scope(|scope| {
///     let auto_reporter = progress.spawn_auto_reporter(scope);
///     auto_reporter.notifier().notify();
///     auto_reporter.stop()?;
///     Ok::<(), qubit_progress::AutoReporterError>(())
/// })?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct ProgressNotifier {
    /// Non-owning link present only for notification-driven reporters.
    pub(super) inner: Option<Weak<AutoReporterInner>>,
}

impl ProgressNotifier {
    /// Records that shared work state changed and wakes a zero-interval loop.
    ///
    /// The method is a no-op for disabled and heartbeat-driven reporters, after
    /// the reporter stops, and when no worker remains. Multiple calls merge
    /// into at most one pending report.
    pub fn notify(&self) {
        let Some(inner) = self.inner.as_ref().and_then(Weak::upgrade) else {
            return;
        };
        if inner.stopped.load(Ordering::Acquire) {
            return;
        }
        inner.pending.store(true, Ordering::Release);
        wake(&inner.wake_sender);
    }
}
