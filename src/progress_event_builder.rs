/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use crate::{
    ProgressCounters,
    ProgressPhase,
    ProgressStage,
    progress_event::ProgressEvent,
};

/// Builder for [`ProgressEvent`].
///
/// The builder keeps the common path compact by letting callers configure
/// phase, counters, optional stage information, and elapsed time in a single
/// chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressEventBuilder {
    /// Lifecycle phase of the event being built.
    pub(crate) phase: ProgressPhase,
    /// Generic counters for the event being built.
    pub(crate) counters: ProgressCounters,
    /// Optional current stage.
    pub(crate) stage: Option<ProgressStage>,
    /// Monotonic elapsed duration.
    pub(crate) elapsed: Duration,
}

impl ProgressEventBuilder {
    /// Creates a builder with default running progress state.
    ///
    /// # Returns
    ///
    /// A builder whose phase is [`ProgressPhase::Running`], elapsed duration is
    /// zero, total count is unknown, and all counters are zero.
    #[inline]
    pub const fn new() -> Self {
        Self {
            phase: ProgressPhase::Running,
            counters: ProgressCounters::new(None),
            stage: None,
            elapsed: Duration::ZERO,
        }
    }

    /// Configures the lifecycle phase.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase to report.
    ///
    /// # Returns
    ///
    /// This builder with `phase` recorded.
    #[inline]
    pub const fn phase(mut self, phase: ProgressPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Configures the event as started.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Started`].
    #[inline]
    pub const fn started(self) -> Self {
        self.phase(ProgressPhase::Started)
    }

    /// Configures the event as running.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Running`].
    #[inline]
    pub const fn running(self) -> Self {
        self.phase(ProgressPhase::Running)
    }

    /// Configures the event as finished.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Finished`].
    #[inline]
    pub const fn finished(self) -> Self {
        self.phase(ProgressPhase::Finished)
    }

    /// Configures the event as failed.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Failed`].
    #[inline]
    pub const fn failed(self) -> Self {
        self.phase(ProgressPhase::Failed)
    }

    /// Configures the event as canceled.
    ///
    /// # Returns
    ///
    /// This builder with [`ProgressPhase::Canceled`].
    #[inline]
    pub const fn canceled(self) -> Self {
        self.phase(ProgressPhase::Canceled)
    }

    /// Replaces the current counter set.
    ///
    /// # Parameters
    ///
    /// * `counters` - Complete counter set to carry in the built event.
    ///
    /// # Returns
    ///
    /// This builder with `counters` recorded.
    #[inline]
    pub const fn counters(mut self, counters: ProgressCounters) -> Self {
        self.counters = counters;
        self
    }

    /// Configures a known total work-unit count.
    ///
    /// # Parameters
    ///
    /// * `total_count` - Total number of work units.
    ///
    /// # Returns
    ///
    /// This builder with a known total count.
    #[inline]
    pub const fn total(mut self, total_count: usize) -> Self {
        self.counters.set_total_count(Some(total_count));
        self
    }

    /// Configures the event as unknown-total progress.
    ///
    /// # Returns
    ///
    /// This builder with no total count.
    #[inline]
    pub const fn unknown_total(mut self) -> Self {
        self.counters.set_total_count(None);
        self
    }

    /// Configures the completed work-unit count.
    ///
    /// # Parameters
    ///
    /// * `completed_count` - Number of completed work units.
    ///
    /// # Returns
    ///
    /// This builder with `completed_count` recorded.
    #[inline]
    pub const fn completed(mut self, completed_count: usize) -> Self {
        self.counters.set_completed_count(completed_count);
        self
    }

    /// Configures the active work-unit count.
    ///
    /// # Parameters
    ///
    /// * `active_count` - Number of currently active work units.
    ///
    /// # Returns
    ///
    /// This builder with `active_count` recorded.
    #[inline]
    pub const fn active(mut self, active_count: usize) -> Self {
        self.counters.set_active_count(active_count);
        self
    }

    /// Configures the successful work-unit count.
    ///
    /// # Parameters
    ///
    /// * `succeeded_count` - Number of successful work units.
    ///
    /// # Returns
    ///
    /// This builder with `succeeded_count` recorded.
    #[inline]
    pub const fn succeeded(mut self, succeeded_count: usize) -> Self {
        self.counters.set_succeeded_count(succeeded_count);
        self
    }

    /// Configures the failed work-unit count.
    ///
    /// # Parameters
    ///
    /// * `failed_count` - Number of failed work units.
    ///
    /// # Returns
    ///
    /// This builder with `failed_count` recorded.
    #[inline]
    pub const fn failed_count(mut self, failed_count: usize) -> Self {
        self.counters.set_failed_count(failed_count);
        self
    }

    /// Configures the current stage.
    ///
    /// # Parameters
    ///
    /// * `stage` - Stage metadata to carry in the built event.
    ///
    /// # Returns
    ///
    /// This builder with `stage` recorded.
    #[inline]
    pub fn stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Configures the current stage from an id and display name.
    ///
    /// # Parameters
    ///
    /// * `id` - Stable machine-readable stage identifier.
    /// * `name` - Human-readable stage name.
    ///
    /// # Returns
    ///
    /// This builder with a stage created from `id` and `name`.
    #[inline]
    pub fn stage_named(self, id: &str, name: &str) -> Self {
        self.stage(ProgressStage::new(id, name))
    }

    /// Configures the elapsed duration.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Monotonic elapsed duration to carry in the event.
    ///
    /// # Returns
    ///
    /// This builder with `elapsed` recorded.
    #[inline]
    pub const fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = elapsed;
        self
    }

    /// Builds the progress event.
    ///
    /// # Returns
    ///
    /// An immutable [`ProgressEvent`] with the configured values.
    #[inline]
    pub fn build(self) -> ProgressEvent {
        ProgressEvent::new(self)
    }
}

impl Default for ProgressEventBuilder {
    /// Creates a builder with default running progress state.
    ///
    /// # Returns
    ///
    /// A builder equivalent to [`Self::new`].
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
