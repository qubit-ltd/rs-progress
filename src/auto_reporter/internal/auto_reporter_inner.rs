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

/// Shared control state held by the handle and weakly by worker notifiers.
pub(in crate::auto_reporter) struct AutoReporterInner {
    /// Whether state-change notifications drive running reports.
    pub(in crate::auto_reporter) notification_driven: bool,
    /// Bounded wake channel sender.
    pub(in crate::auto_reporter) wake_sender: SyncSender<()>,
    /// Stop request flag.
    pub(in crate::auto_reporter) stopped: AtomicBool,
    /// Coalesced notification flag.
    pub(in crate::auto_reporter) pending: AtomicBool,
}
