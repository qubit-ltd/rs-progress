// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared operation lifecycle and transition-freezing protocol.

use std::{
    hint::spin_loop,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
    thread,
};

use crate::{
    MetricError,
    internal::{OperationLifecycle, UpdateGuard},
};

/// Lifecycle and in-flight update counters shared by one operation.
pub(crate) struct OperationState {
    /// Atomic lifecycle value.
    lifecycle: AtomicU8,
    /// Number of updates that passed the first open check.
    pub(crate) active_updates: AtomicUsize,
}

impl OperationState {
    /// Creates a new open operation state.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            lifecycle: AtomicU8::new(OperationLifecycle::Open as u8),
            active_updates: AtomicUsize::new(0),
        })
    }

    /// Returns the currently published lifecycle state.
    pub(crate) fn lifecycle(&self) -> OperationLifecycle {
        match self.lifecycle.load(Ordering::Acquire) {
            value if value == OperationLifecycle::Open as u8 => OperationLifecycle::Open,
            value if value == OperationLifecycle::Finishing as u8 => OperationLifecycle::Finishing,
            value if value == OperationLifecycle::Closed as u8 => OperationLifecycle::Closed,
            _ => unreachable!("operation lifecycle must contain a known value"),
        }
    }

    /// Registers one metric update while the operation remains open.
    pub(crate) fn enter_update<'state>(
        &'state self,
        metric_id: &str,
    ) -> Result<UpdateGuard<'state>, MetricError> {
        let state = self.lifecycle();
        if state != OperationLifecycle::Open {
            return Err(MetricError::OperationNotOpen {
                metric_id: metric_id.into(),
                state,
            });
        }
        self.active_updates.fetch_add(1, Ordering::AcqRel);
        let state = self.lifecycle();
        if state != OperationLifecycle::Open {
            self.active_updates.fetch_sub(1, Ordering::Release);
            return Err(MetricError::OperationNotOpen {
                metric_id: metric_id.into(),
                state,
            });
        }
        Ok(UpdateGuard::new(self))
    }

    /// Freezes new updates and waits for already registered updates.
    pub(crate) fn begin_finish(&self) -> FinishGuard<'_> {
        let previous = self.lifecycle.compare_exchange(
            OperationLifecycle::Open as u8,
            OperationLifecycle::Finishing as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        assert_eq!(
            previous,
            Ok(OperationLifecycle::Open as u8),
            "Progress owns the only lifecycle transition authority",
        );
        let mut attempts: u32 = 0;
        while self.active_updates.load(Ordering::Acquire) != 0 {
            if attempts > 0 && attempts.is_multiple_of(16) {
                thread::yield_now();
            } else {
                spin_loop();
            }
            attempts += 1;
        }
        FinishGuard { state: self }
    }

    /// Permanently closes the operation without waiting for metric updates.
    pub(crate) fn close(&self) {
        self.lifecycle
            .store(OperationLifecycle::Closed as u8, Ordering::Release);
    }
}

/// Temporary freeze guard used while validating or emitting a terminal event.
pub(crate) struct FinishGuard<'state> {
    state: &'state OperationState,
}

impl FinishGuard<'_> {
    /// Reopens the operation after completion validation fails.
    pub(crate) fn reopen(self) {
        self.state
            .lifecycle
            .store(OperationLifecycle::Open as u8, Ordering::Release);
    }

    /// Permanently closes the operation before terminal emission.
    pub(crate) fn close(self) {
        self.state.close();
    }
}

impl Drop for FinishGuard<'_> {
    /// Closes an abandoned freeze guard conservatively.
    fn drop(&mut self) {
        if self.state.lifecycle() == OperationLifecycle::Finishing {
            self.state.close();
        }
    }
}
