// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared failure status for an automatic reporter.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// Cloneable status exposed while an automatic reporter is active.
///
/// # Examples
///
/// ```
/// use qubit_progress::{Metric, Progress};
/// use qubit_progress::TextReporter;
///
/// let reporter = TextReporter::new(Vec::new());
/// let mut progress = Progress::builder(&reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// std::thread::scope(|scope| {
///     let auto_reporter = progress.spawn_auto_reporter(scope);
///     let status = auto_reporter.status();
///     assert!(!status.is_failed());
///     auto_reporter.stop()?;
///     Ok::<(), qubit_progress::AutoReporterError>(())
/// })?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct AutoReporterStatus {
    /// Shared failure flag.
    failed: Arc<AtomicBool>,
}

impl AutoReporterStatus {
    /// Creates a status flag initially representing a healthy reporter.
    #[must_use]
    pub(super) fn healthy() -> Self {
        Self {
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns whether the automatic reporter terminated with an error or
    /// panic.
    #[must_use]
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// Records that the reporter has failed.
    pub(super) fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }
}
