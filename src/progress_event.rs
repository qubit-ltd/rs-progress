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
};

/// Immutable progress event delivered to reporters.
///
/// # Type Parameters
///
/// * `C` - Caller-defined context attached to the progress event.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressEvent<C> {
    /// Lifecycle phase of the reported operation.
    phase: ProgressPhase,
    /// Optional current stage.
    stage: Option<ProgressStage>,
    /// Generic counters for the operation.
    counters: ProgressCounters,
    /// Monotonic elapsed duration.
    elapsed: Duration,
    /// Caller-defined context.
    context: C,
}

impl<C> ProgressEvent<C> {
    /// Creates a progress event.
    ///
    /// # Parameters
    ///
    /// * `phase` - Lifecycle phase of the operation.
    /// * `counters` - Generic progress counters.
    /// * `elapsed` - Monotonic elapsed duration.
    /// * `context` - Caller-defined context carried by the event.
    ///
    /// # Returns
    ///
    /// A progress event with no stage.
    pub const fn new(
        phase: ProgressPhase,
        counters: ProgressCounters,
        elapsed: Duration,
        context: C,
    ) -> Self {
        Self {
            phase,
            stage: None,
            counters,
            elapsed,
            context,
        }
    }

    /// Creates a started progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Initial progress counters.
    /// * `elapsed` - Elapsed duration at start, usually zero.
    /// * `context` - Caller-defined context.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Started`].
    pub const fn started(counters: ProgressCounters, elapsed: Duration, context: C) -> Self {
        Self::new(ProgressPhase::Started, counters, elapsed, context)
    }

    /// Creates a running progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Current progress counters.
    /// * `elapsed` - Elapsed duration since operation start.
    /// * `context` - Caller-defined context.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Running`].
    pub const fn running(counters: ProgressCounters, elapsed: Duration, context: C) -> Self {
        Self::new(ProgressPhase::Running, counters, elapsed, context)
    }

    /// Creates a finished progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final progress counters.
    /// * `elapsed` - Total elapsed duration.
    /// * `context` - Caller-defined context.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Finished`].
    pub const fn finished(counters: ProgressCounters, elapsed: Duration, context: C) -> Self {
        Self::new(ProgressPhase::Finished, counters, elapsed, context)
    }

    /// Creates a failed progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at failure.
    /// * `context` - Caller-defined context.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Failed`].
    pub const fn failed(counters: ProgressCounters, elapsed: Duration, context: C) -> Self {
        Self::new(ProgressPhase::Failed, counters, elapsed, context)
    }

    /// Creates a canceled progress event.
    ///
    /// # Parameters
    ///
    /// * `counters` - Final or current progress counters.
    /// * `elapsed` - Elapsed duration at cancellation.
    /// * `context` - Caller-defined context.
    ///
    /// # Returns
    ///
    /// A progress event with [`ProgressPhase::Canceled`].
    pub const fn canceled(counters: ProgressCounters, elapsed: Duration, context: C) -> Self {
        Self::new(ProgressPhase::Canceled, counters, elapsed, context)
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

    /// Returns the caller-defined context.
    ///
    /// # Returns
    ///
    /// A shared reference to this event's context.
    pub const fn context(&self) -> &C {
        &self.context
    }

    /// Consumes this event and returns its context.
    ///
    /// # Returns
    ///
    /// The caller-defined context carried by this event.
    pub fn into_context(self) -> C {
        self.context
    }
}
