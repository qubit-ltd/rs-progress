// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Explicit disabled reporter.

use crate::Event;
use crate::Reporter;
use crate::ReporterError;

/// Reporter that disables an entire operation at start.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopReporter;
impl Reporter for NoopReporter {
    /// Disables operations that use this reporter.
    fn is_enabled(&self) -> bool {
        false
    }
    /// Accepts events defensively when called outside [`crate::Progress`].
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        Ok(())
    }
}
