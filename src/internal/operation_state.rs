// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared operation lifecycle and transition-freezing protocol.
// qubit-style: allow multiple-public-types
// qubit-style: allow coverage-cfg

use std::sync::Arc;

use crate::{
    MetricError,
    internal::{
        OperationLifecycle,
        UpdateGuard,
        operation_gate::{
            GateLifecycle,
            OperationGate,
            StdScheduler,
        },
    },
};

/// Lifecycle and in-flight update counters shared by one operation.
pub(crate) struct OperationState {
    /// Generic atomic lifecycle protocol used by production and Loom tests.
    gate: OperationGate<
        std::sync::atomic::AtomicU8,
        std::sync::atomic::AtomicUsize,
        StdScheduler,
    >,
}

impl OperationState {
    /// Creates a new open operation state.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            gate: OperationGate::new(),
        })
    }

    /// Returns the currently published lifecycle state.
    #[inline(never)]
    pub(crate) fn lifecycle(&self) -> OperationLifecycle {
        match self.gate.lifecycle() {
            GateLifecycle::Open => OperationLifecycle::Open,
            GateLifecycle::Finishing => OperationLifecycle::Finishing,
            GateLifecycle::Closed => OperationLifecycle::Closed,
        }
    }

    /// Registers one metric update while the operation remains open.
    #[inline]
    pub(crate) fn enter_update<'state>(
        &'state self,
        metric_id: &str,
    ) -> Result<UpdateGuard<'state>, MetricError> {
        match self.gate.enter_update() {
            Ok(()) => Ok(UpdateGuard::new(self)),
            Err(state) => Err(MetricError::OperationNotOpen {
                metric_id: metric_id.into(),
                state: operation_lifecycle(state),
            }),
        }
    }

    /// Freezes new updates and waits for already registered updates.
    pub(crate) fn begin_finish(&self) -> FinishGuard<'_> {
        assert!(
            self.gate.try_begin_finish(),
            "Progress owns the only lifecycle transition authority",
        );
        FinishGuard { state: self }
    }

    /// Permanently closes the operation without waiting for metric updates.
    #[inline]
    pub(crate) fn close(&self) {
        self.gate.close();
    }

    /// Releases one metric update registered with this operation.
    #[inline]
    pub(crate) fn leave_update(&self) {
        self.gate.leave_update();
    }
}

/// Temporary freeze guard used while validating or emitting a terminal event.
pub(crate) struct FinishGuard<'state> {
    state: &'state OperationState,
}

impl FinishGuard<'_> {
    /// Reopens the operation after completion validation fails.
    pub(crate) fn reopen(self) {
        self.state.gate.reopen();
    }

    /// Permanently closes the operation before terminal emission.
    pub(crate) fn close(self) {
        self.state.close();
    }
}

impl Drop for FinishGuard<'_> {
    /// Closes an abandoned freeze guard conservatively.
    #[inline(never)]
    fn drop(&mut self) {
        if self.state.lifecycle() == OperationLifecycle::Finishing {
            self.state.close();
        }
    }
}

#[inline(never)]
fn operation_lifecycle(state: GateLifecycle) -> OperationLifecycle {
    match state {
        GateLifecycle::Open => OperationLifecycle::Open,
        GateLifecycle::Finishing => OperationLifecycle::Finishing,
        GateLifecycle::Closed => OperationLifecycle::Closed,
    }
}

/// Exercises the finishing guard and lifecycle mapping from the library build.
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

/// Runs internal lifecycle coverage hooks in the instrumented build.
#[cfg(coverage)]
#[doc(hidden)]
pub fn __coverage_internal() {
    super::operation_gate::__coverage_operation_gate();
    __coverage_operation_state();
}
