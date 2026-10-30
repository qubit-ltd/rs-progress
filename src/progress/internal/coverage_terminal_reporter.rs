// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter used to exercise terminal delivery failure coverage.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::Event;
use crate::Reporter;
use crate::ReporterError;

/// Test reporter that allows the first event and rejects every later delivery.
///
/// This helper drives the terminal-delivery error path in the coverage entry
/// point without changing the behavior of production reporters.
pub(crate) struct CoverageTerminalReporter {
    /// Number of calls to [`Reporter::report`] observed so far.
    ///
    /// The first call is accepted; each subsequent call returns a stable
    /// coverage-only failure.
    pub(crate) attempts: AtomicU64,
}

impl Reporter for CoverageTerminalReporter {
    /// Accepts the first event and reports a failure for later events.
    ///
    /// The atomic counter is relaxed because it only selects which call fails;
    /// it does not publish or synchronize any other data.
    ///
    /// # Parameters
    ///
    /// - `event`: Event whose delivery attempt is counted.
    ///
    /// # Returns
    ///
    /// `Ok(())` for the first attempt, then a stable coverage-only failure.
    ///
    /// # Errors
    ///
    /// Returns a reporter error for every attempt after the first.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.attempts.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReporterError::message("coverage terminal failure"))
        }
    }
}
