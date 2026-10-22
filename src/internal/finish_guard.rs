// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Temporary guard for an operation's finishing lifecycle state.

use crate::internal::OperationLifecycle;
use crate::internal::OperationState;

/// Temporary freeze guard used while validating or emitting a terminal event.
#[must_use = "dropping the guard closes the operation unless it is explicitly reopened"]
pub(crate) struct FinishGuard<'state> {
    /// Operation whose updates are frozen until this guard is consumed or
    /// dropped.
    pub(super) state: &'state OperationState,
}

impl FinishGuard<'_> {
    /// Reopens the operation after completion validation fails.
    #[inline]
    pub(crate) fn reopen(self) {
        self.state.gate.reopen();
    }

    /// Permanently closes the operation before terminal emission.
    #[inline]
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
