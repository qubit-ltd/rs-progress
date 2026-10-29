// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared wake and lifecycle state for an automatic reporter.

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;

/// Coordinates wakeups and shutdown between the reporter handle and worker.
///
/// The handle owns this state while notification handles keep only weak
/// references, so a notifier cannot extend the reporter worker's lifetime.
pub(in crate::auto_reporter) struct AutoReporterInner {
    /// Whether this reporter runs only when progress notifications arrive.
    pub(in crate::auto_reporter) notification_driven: bool,
    /// Bounded channel sender used to wake the worker without blocking callers.
    pub(in crate::auto_reporter) wake_sender: SyncSender<()>,
    /// Stop request published by the handle and observed by the worker.
    pub(in crate::auto_reporter) stopped: AtomicBool,
    /// Pending notification marker, merging multiple changes into one report.
    pub(in crate::auto_reporter) pending: AtomicBool,
}
