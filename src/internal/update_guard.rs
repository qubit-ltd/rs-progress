// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! RAII guard for one registered metric update.

use crate::internal::OperationState;

/// Keeps one metric update counted until its critical section exits.
pub(crate) struct UpdateGuard<'state> {
    state: &'state OperationState,
}

impl<'state> UpdateGuard<'state> {
    /// Creates a guard for an already registered update.
    pub(crate) const fn new(state: &'state OperationState) -> Self {
        Self { state }
    }
}

impl Drop for UpdateGuard<'_> {
    /// Releases the registered update on every return and panic path.
    fn drop(&mut self) {
        self.state.leave_update();
    }
}
