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
#[must_use = "dropping the guard releases the registered update"]
pub(crate) struct UpdateGuard<'state> {
    /// Operation state whose registered update this guard releases.
    state: &'state OperationState,
}

impl<'state> UpdateGuard<'state> {
    /// Creates a guard for an already registered update.
    ///
    /// # Parameters
    ///
    /// - `state`: the operation state with the update already registered.
    ///
    /// # Returns
    ///
    /// A guard that releases the registration when dropped.
    #[inline]
    pub(crate) const fn new(state: &'state OperationState) -> Self {
        Self { state }
    }
}

impl Drop for UpdateGuard<'_> {
    /// Releases the registered update on every return and panic path.
    #[inline]
    fn drop(&mut self) {
        self.state.leave_update();
    }
}
