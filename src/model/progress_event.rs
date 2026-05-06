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

use super::{
    ProgressCounters,
    ProgressEventBuilder,
    ProgressPhase,
    ProgressStage,
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
    #[inline]
    pub const fn builder() -> ProgressEventBuilder {
        ProgressEventBuilder::new()
    }

    /// Creates a progress event.
    ///
    /// # Parameters
    ///
    /// * `builder` - Builder containing configured event fields.
    ///
    /// # Returns
    ///
    /// A progress event built from `builder`.
    #[inline]
    pub fn new(builder: ProgressEventBuilder) -> Self {
        Self {
            phase: builder.phase,
            stage: builder.stage,
            counters: builder.counters,
            elapsed: builder.elapsed,
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
    #[inline]
    pub const fn started(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase: ProgressPhase::Started,
            stage: None,
            counters,
            elapsed,
        }
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
    #[inline]
    pub const fn running(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase: ProgressPhase::Running,
            stage: None,
            counters,
            elapsed,
        }
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
    #[inline]
    pub const fn finished(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase: ProgressPhase::Finished,
            stage: None,
            counters,
            elapsed,
        }
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
    #[inline]
    pub const fn failed(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase: ProgressPhase::Failed,
            stage: None,
            counters,
            elapsed,
        }
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
    #[inline]
    pub const fn canceled(counters: ProgressCounters, elapsed: Duration) -> Self {
        Self {
            phase: ProgressPhase::Canceled,
            stage: None,
            counters,
            elapsed,
        }
    }

    /// Creates a progress event for the supplied lifecycle phase.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase for the event.
    /// * `counters` - Progress counters carried by the event.
    /// * `elapsed` - Elapsed duration carried by the event.
    ///
    /// # Returns
    ///
    /// A progress event with `phase`, `counters`, and `elapsed`.
    #[inline]
    pub const fn from_phase(
        phase: ProgressPhase,
        counters: ProgressCounters,
        elapsed: Duration,
    ) -> Self {
        match phase {
            ProgressPhase::Started => Self::started(counters, elapsed),
            ProgressPhase::Running => Self::running(counters, elapsed),
            ProgressPhase::Finished => Self::finished(counters, elapsed),
            ProgressPhase::Failed => Self::failed(counters, elapsed),
            ProgressPhase::Canceled => Self::canceled(counters, elapsed),
        }
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
    #[inline]
    pub fn with_stage(mut self, stage: ProgressStage) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Returns the event phase.
    ///
    /// # Returns
    ///
    /// The lifecycle phase carried by this event.
    #[inline]
    pub const fn phase(&self) -> ProgressPhase {
        self.phase
    }

    /// Returns the current stage when known.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when this event carries stage information, otherwise
    /// `None`.
    #[inline]
    pub const fn stage(&self) -> Option<&ProgressStage> {
        self.stage.as_ref()
    }

    /// Returns the progress counters.
    ///
    /// # Returns
    ///
    /// The counters carried by this event.
    #[inline]
    pub const fn counters(&self) -> ProgressCounters {
        self.counters
    }

    /// Returns the elapsed duration.
    ///
    /// # Returns
    ///
    /// The monotonic elapsed duration carried by this event.
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}
