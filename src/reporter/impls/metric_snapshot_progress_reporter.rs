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

use crate::{
    model::{
        ProgressEvent,
        ProgressMetricSnapshot,
    },
    reporter::ProgressReporter,
};

/// Progress reporter that sends metric snapshot objects to a consumer.
///
/// This reporter is useful for GUI, database, metrics, and monitoring adapters
/// that want structured metric snapshots instead of preformatted strings.
pub struct MetricSnapshotProgressReporter<
    C = ArcConsumer<ProgressMetricSnapshot>,
> {
    /// Consumer receiving metric snapshots.
    consumer: C,
}

impl<C> MetricSnapshotProgressReporter<C> {
    /// Creates a metric snapshot reporter.
    ///
    /// # Parameters
    ///
    /// * `consumer` - Consumer receiving structured metric snapshots.
    ///
    /// # Returns
    ///
    /// A reporter that sends one snapshot per event counter to `consumer`.
    #[inline]
    pub const fn new(consumer: C) -> Self {
        Self { consumer }
    }

    /// Returns the configured consumer.
    ///
    /// # Returns
    ///
    /// A shared reference to the metric snapshot consumer.
    #[inline]
    pub const fn consumer(&self) -> &C {
        &self.consumer
    }
}

impl<C> ProgressReporter for MetricSnapshotProgressReporter<C>
where
    C: Consumer<ProgressMetricSnapshot> + Send + Sync,
{
    /// Sends every metric snapshot in the event to the configured consumer.
    ///
    /// # Parameters
    ///
    /// * `event` - Event whose metric snapshots should be consumed.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        for snapshot in event.metric_snapshots() {
            self.consumer.accept(&snapshot);
        }
    }
}
