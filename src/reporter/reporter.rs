// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter trait for complete immutable events.

use crate::{Event, ReporterError};

/// Consumes complete immutable progress events.
pub trait Reporter: Send + Sync {
    /// Returns whether a new operation should emit events.
    ///
    /// [`crate::ProgressBuilder::start`] samples this once per operation.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Delivers one complete event.
    ///
    /// Returns [`ReporterError`] without discarding the sink's error source.
    fn report(&self, event: &Event) -> Result<(), ReporterError>;
}

impl<F> Reporter for F
where
    F: Fn(&Event) -> Result<(), ReporterError> + Send + Sync,
{
    /// Invokes this closure for one complete event.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self(event)
    }
}
