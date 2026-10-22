// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! `log` facade reporter for complete events.

use log::Level;
use log::info;
use log::log_enabled;

use crate::Event;
use crate::Reporter;
use crate::ReporterError;

/// Reports each complete event through the `log` facade at info level.
///
/// # Examples
///
/// ```
/// use qubit_progress::{LogReporter, Metric, Progress};
///
/// let progress = Progress::builder(&LogReporter)
///     .metric(Metric::new("tasks", "Tasks").total(1))
///     .start()?;
/// progress.finish_unchecked()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LogReporter;

impl Reporter for LogReporter {
    /// Samples whether the `log` facade accepts info-level output.
    fn is_enabled(&self) -> bool {
        log_enabled!(Level::Info)
    }

    /// Writes the event's debug representation through `log::info!`.
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        info!("{event:?}");
        Ok(())
    }
}
