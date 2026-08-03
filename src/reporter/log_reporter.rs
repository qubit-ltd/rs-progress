// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! `log` facade reporter for complete events.

use crate::{
    Event,
    Reporter,
    ReporterError,
};

/// Reports each complete event through the `log` facade at info level.
#[derive(Clone, Copy, Debug, Default)]
pub struct LogReporter;

impl Reporter for LogReporter {
    /// Samples whether the `log` facade accepts info-level output.
    fn is_enabled(&self) -> bool {
        log::log_enabled!(log::Level::Info)
    }

    /// Writes the event's debug representation through `log::info!`.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        log::info!("{event:?}");
        Ok(())
    }
}
