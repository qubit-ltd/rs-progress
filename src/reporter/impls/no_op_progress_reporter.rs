// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{
    model::ProgressEvent,
    reporter::ProgressReporter,
};

/// Progress reporter that ignores all events.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoOpProgressReporter;

impl ProgressReporter for NoOpProgressReporter {
    /// Ignores one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Event accepted and ignored.
    #[inline]
    fn report(&self, _event: &ProgressEvent) {}
}
