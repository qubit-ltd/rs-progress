// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{
    model::ProgressEvent,
    reporter::{
        ProgressReportError,
        ProgressReporter,
    },
};

/// Progress reporter that ignores all events.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoOpProgressReporter;

impl ProgressReporter for NoOpProgressReporter {
    /// Reports that this reporter intentionally ignores all events.
    ///
    /// # Returns
    ///
    /// Always `false`.
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        false
    }

    /// Ignores one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Event accepted and ignored.
    #[inline]
    fn report(
        &self,
        _event: &ProgressEvent,
    ) -> Result<(), ProgressReportError> {
        Ok(())
    }
}
