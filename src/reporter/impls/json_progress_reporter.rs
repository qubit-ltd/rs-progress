// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_function::{
    ArcConsumer,
    Consumer,
};

use super::formatted_progress_reporter::FormattedProgressReporter;
use crate::{
    model::ProgressEvent,
    reporter::{
        ProgressReporter,
        format::JsonMetricSnapshotFormatter,
    },
};

/// Progress reporter that emits JSON metric snapshot strings to a consumer.
pub struct JsonProgressReporter<C = ArcConsumer<String>> {
    /// Formatted reporter using the JSON metric snapshot formatter.
    inner: FormattedProgressReporter<JsonMetricSnapshotFormatter, C>,
}

impl<C> JsonProgressReporter<C> {
    /// Creates a JSON progress reporter.
    ///
    /// # Parameters
    ///
    /// * `consumer` - Consumer receiving compact JSON strings.
    ///
    /// # Returns
    ///
    /// A JSON progress reporter using the default snapshot JSON formatter.
    #[inline]
    pub fn new(consumer: C) -> Self {
        Self {
            inner: FormattedProgressReporter::new(
                JsonMetricSnapshotFormatter::new(),
                consumer,
            ),
        }
    }

    /// Returns the inner formatted reporter.
    ///
    /// # Returns
    ///
    /// A shared reference to the inner reporter.
    #[inline]
    pub const fn inner(
        &self,
    ) -> &FormattedProgressReporter<JsonMetricSnapshotFormatter, C> {
        &self.inner
    }
}

impl<C> ProgressReporter for JsonProgressReporter<C>
where
    C: Consumer<String> + Send + Sync,
{
    /// Reports one event through the JSON formatter.
    ///
    /// # Parameters
    ///
    /// * `event` - Event to report.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
