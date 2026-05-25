/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_function::{
    ArcConsumer,
    Consumer,
};

use super::formatted_progress_reporter::FormattedProgressReporter;
use crate::{
    model::ProgressEvent,
    reporter::{
        ProgressReporter,
        format::HumanReadableMetricSnapshotFormatter,
    },
};

/// Progress reporter that emits human-readable strings to a consumer.
pub struct HumanReadableProgressReporter<C = ArcConsumer<String>> {
    /// Formatted reporter using the human-readable formatter.
    inner: FormattedProgressReporter<HumanReadableMetricSnapshotFormatter, C>,
}

impl<C> HumanReadableProgressReporter<C> {
    /// Creates a human-readable progress reporter.
    ///
    /// # Parameters
    ///
    /// * `consumer` - Consumer receiving formatted human-readable strings.
    ///
    /// # Returns
    ///
    /// A human-readable progress reporter using the default formatter.
    #[inline]
    pub fn new(consumer: C) -> Self {
        Self {
            inner: FormattedProgressReporter::new(HumanReadableMetricSnapshotFormatter::new(), consumer),
        }
    }

    /// Returns the inner formatted reporter.
    ///
    /// # Returns
    ///
    /// A shared reference to the inner reporter.
    #[inline]
    pub const fn inner(&self) -> &FormattedProgressReporter<HumanReadableMetricSnapshotFormatter, C> {
        &self.inner
    }
}

impl<C> ProgressReporter for HumanReadableProgressReporter<C>
where
    C: Consumer<String> + Send + Sync,
{
    /// Reports one event through the human-readable formatter.
    ///
    /// # Parameters
    ///
    /// * `event` - Event to report.
    #[inline]
    fn report(&self, event: &ProgressEvent) {
        self.inner.report(event);
    }
}
