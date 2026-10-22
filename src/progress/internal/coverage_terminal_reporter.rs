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

/// Reporter that fails only after the Started event.
pub(crate) struct CoverageTerminalReporter {
    /// Number of delivery attempts observed by the reporter.
    pub(crate) attempts: AtomicU64,
}

impl Reporter for CoverageTerminalReporter {
    /// Fails terminal delivery after allowing operation startup.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.attempts.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReporterError::message("coverage terminal failure"))
        }
    }
}
