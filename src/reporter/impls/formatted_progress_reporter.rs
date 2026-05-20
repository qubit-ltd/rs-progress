/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_function::Consumer;

use crate::{
    model::ProgressEvent,
    reporter::{
        ProgressReporter,
        format::MetricSnapshotFormatter,
    },
};

/// Progress reporter that formats each metric snapshot and sends it to a consumer.
///
/// This reporter is the common adapter behind text and JSON progress reporters.
/// It converts every counter in an event into a metric snapshot, formats the
/// snapshot, then passes the formatted string to the configured consumer.
pub struct FormattedProgressReporter<F, C> {
    /// Formatter applied to each metric snapshot.
    formatter: F,
    /// Consumer receiving formatted snapshot strings.
    consumer: C,
}

impl<F, C> FormattedProgressReporter<F, C> {
    /// Creates a formatted progress reporter.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Formatter applied to each metric snapshot.
    /// * `consumer` - Consumer receiving formatted strings.
    ///
    /// # Returns
    ///
    /// A reporter that formats metric snapshots and sends strings to `consumer`.
    #[inline]
    pub const fn new(formatter: F, consumer: C) -> Self {
        Self {
            formatter,
            consumer,
        }
    }

    /// Returns the configured formatter.
    ///
    /// # Returns
    ///
    /// A shared reference to the metric snapshot formatter.
    #[inline]
    pub const fn formatter(&self) -> &F {
        &self.formatter
    }

    /// Returns the configured consumer.
    ///
    /// # Returns
    ///
    /// A shared reference to the formatted string consumer.
    #[inline]
    pub const fn consumer(&self) -> &C {
        &self.consumer
    }
}

impl<F, C> ProgressReporter for FormattedProgressReporter<F, C>
where
    F: MetricSnapshotFormatter,
    C: Consumer<String> + Send + Sync,
{
    /// Formats and consumes every metric snapshot in an event.
    ///
    /// # Parameters
    ///
    /// * `event` - Event whose metric snapshots should be formatted.
    fn report(&self, event: &ProgressEvent) {
        for snapshot in event.metric_snapshots() {
            let line = self.formatter.format(&snapshot);
            self.consumer.accept(&line);
        }
    }
}
