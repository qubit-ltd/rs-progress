// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared operation lifecycle and transition-freezing protocol.
// qubit-style: allow coverage-cfg

use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicUsize;

use crate::MetricError;
use crate::internal::FinishGuard;
use crate::internal::OperationLifecycle;
use crate::internal::UpdateGuard;
use crate::internal::operation_gate::GateLifecycle;
use crate::internal::operation_gate::OperationGate;
use crate::internal::operation_gate::StdScheduler;

/// Lifecycle and in-flight update counters shared by one operation.
pub(crate) struct OperationState {
    /// Generic atomic lifecycle protocol used by production and Loom tests.
    pub(super) gate: OperationGate<AtomicU8, AtomicUsize, StdScheduler>,
}

impl OperationState {
    /// Creates a new operation state whose lifecycle is open and update count
    /// is zero.
    ///
    /// # Returns
    ///
    /// A shared operation state ready to admit metric updates.
    #[must_use]
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            gate: OperationGate::new(),
        })
    }

    /// Returns the currently published lifecycle state.
    ///
    /// # Returns
    ///
    /// The lifecycle state observed from the operation gate.
    #[inline(never)]
    pub(crate) fn lifecycle(&self) -> OperationLifecycle {
        match self.gate.lifecycle() {
            GateLifecycle::Open => OperationLifecycle::Open,
            GateLifecycle::Finishing => OperationLifecycle::Finishing,
            GateLifecycle::Closed => OperationLifecycle::Closed,
        }
    }

    /// Registers one metric update while the operation remains open.
    ///
    /// # Parameters
    ///
    /// - `metric_id`: stable identifier included in a rejection error.
    ///
    /// # Returns
    ///
    /// An update guard when registration succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`MetricError::OperationNotOpen`] with the observed lifecycle
    /// state when the operation is finishing or closed.
    #[inline]
    pub(crate) fn enter_update<'state>(&'state self, metric_id: &str) -> Result<UpdateGuard<'state>, MetricError> {
        match self.gate.enter_update() {
            Ok(()) => Ok(UpdateGuard::new(self)),
            Err(state) => Err(MetricError::OperationNotOpen {
                metric_id: metric_id.into(),
                state: operation_lifecycle(state),
            }),
        }
    }

    /// Freezes new updates and waits for already registered updates to leave.
    ///
    /// # Returns
    ///
    /// A guard that lets terminal validation reopen the operation or close it
    /// when the guard is dropped.
    ///
    /// # Panics
    ///
    /// Panics if the operation is not open, because only the owning progress
    /// operation may begin the finishing transition.
    pub(crate) fn begin_finish(&self) -> FinishGuard<'_> {
        assert!(
            self.gate.try_begin_finish(),
            "Progress owns the only lifecycle transition authority",
        );
        FinishGuard { state: self }
    }

    /// Permanently closes the operation without waiting for metric updates.
    ///
    /// New updates are rejected after the lifecycle changes to closed. Updates
    /// already registered may still leave normally.
    #[inline]
    pub(crate) fn close(&self) {
        self.gate.close();
    }

    /// Releases one metric update registered with this operation.
    ///
    /// The caller must invoke this exactly once for each successful
    /// registration after its update critical section ends.
    #[inline]
    pub(crate) fn leave_update(&self) {
        self.gate.leave_update();
    }
}

/// Converts the generic operation-gate state into its public lifecycle state.
#[inline(never)]
fn operation_lifecycle(state: GateLifecycle) -> OperationLifecycle {
    match state {
        GateLifecycle::Open => OperationLifecycle::Open,
        GateLifecycle::Finishing => OperationLifecycle::Finishing,
        GateLifecycle::Closed => OperationLifecycle::Closed,
    }
}

/// Exercises the finishing guard and lifecycle mapping in coverage builds.
#[cfg(coverage)]
pub(crate) fn __coverage_operation_state() {
    let state = OperationState::new();
    let guard = state.begin_finish();
    assert_eq!(state.lifecycle(), OperationLifecycle::Finishing);
    assert!(matches!(
        state.enter_update("coverage"),
        Err(MetricError::OperationNotOpen {
            state: OperationLifecycle::Finishing,
            ..
        })
    ));
    drop(guard);
    assert_eq!(state.lifecycle(), OperationLifecycle::Closed);
}

/// Runs internal lifecycle coverage hooks in coverage builds.
#[doc(hidden)]
#[cfg(coverage)]
pub fn __coverage_internal() {
    __coverage_operation_state();
}
