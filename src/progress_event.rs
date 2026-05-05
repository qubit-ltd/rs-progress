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
    progress_event_builder::ProgressEventBuilder,
};

/// Immutable progress event delivered to reporters.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressEvent {
    /// Lifecycle phase of the reported operation.
    phase: ProgressPhase,
    /// Optional current stage.
    stage: Option<ProgressStage>,
    /// Generic counters for the operation.
    counters: ProgressCounters,
    /// Monotonic elapsed duration.
    elapsed: Duration,
}

impl ProgressEvent {
    /// Creates a progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized as running, unknown-total progress with zeroed
    /// counters and zero elapsed time.
    pub const fn builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new()
    }

    /// Creates a started progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized with [`ProgressPhase::Started`].
    pub const fn started_builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new().started()
    }

    /// Creates a running progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized with [`ProgressPhase::Running`].
    pub const fn running_builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new().running()
    }

    /// Creates a finished progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized with [`ProgressPhase::Finished`].
    pub const fn finished_builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new().finished()
    }

    /// Creates a failed progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized with [`ProgressPhase::Failed`].
    pub const fn failed_builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new().failed()
    }

    /// Creates a canceled progress event builder.
    ///
    /// # Returns
    ///
    /// A builder initialized with [`ProgressPhase::Canceled`].
    pub const fn canceled_builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new().canceled()
    }

    /// Creates a progress event.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase of the operation.
    /// * `counters` - Generic progress counters.
    /// * `elapsed` - Monotonic elapsed duration.
    ///
    /// # Returns
    ///
    /// A progress event with no stage.
    pub const fn new(phase: ProgressPhase, counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase,
            stage: None,
            counters,
            elapsed,
        }
    }

    /// Creates a started progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Initial progress counters.
    /// * `elapsed` - Elapsed duration at start, usually zero.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Started`].
    pub const fn started(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self::new(ProgressPhase::Started, counters, elapsed)
    }

    /// Creates a running progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Current progress counters.
    /// * `elapsed` - Elapsed duration since operation start.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Running`].
    pub const fn running(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self::new(ProgressPhase::Running, counters, elapsed)
    }

    /// Creates a finished progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final progress counters.
    /// * `elapsed` - Total elapsed duration.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Finished`].
    pub const fn finished(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self::new(ProgressPhase::Finished, counters, elapsed)
    }

    /// Creates a failed progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at failure.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Failed`].
    pub const fn failed(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self::new(ProgressPhase::Failed, counters, elapsed)
    }

    /// Creates a canceled progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at cancellation.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Canceled`].
    pub const fn canceled(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self::new(ProgressPhase::Canceled, counters, elapsed)
    }

    /// Returns a copy configured with the current stage.
    ///
    /// # Parameters
    ///
    /// * `stage` - Current operation stage.
    ///
    /// # Returns
    ///
    /// This event with `stage` recorded.
    pub fn with_stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Returns the event phase.
    ///
    /// # Returns
    ///
    /// The lifecycle phase carried by this event.
    pub const fn phase(&self) -> ProgressPhase {
        self.phase
    }

    /// Returns the current stage when known.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when this event carries stage information, otherwise
    /// `None`.
    pub const fn stage(&self) -> Option<&ProgressStage> {
        self.stage.as_ref()
    }

    /// Returns the progress counters.
    ///
    /// # Returns
    ///
    /// The counters carried by this event.
    pub const fn counters(&self) -> ProgressCounters {
        self.counters
    }

    /// Returns the elapsed duration.
    ///
    /// # Returns
    ///
    /// The monotonic elapsed duration carried by this event.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}
