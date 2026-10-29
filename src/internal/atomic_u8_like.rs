// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Atomic byte operations used by the operation gate.

use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;

/// Atomic byte operations required by
/// [`crate::internal::operation_gate::OperationGate`].
pub trait AtomicU8Like {
    /// Creates an atomic value with the supplied initial state.
    ///
    /// The returned value must be retained for subsequent atomic operations.
    #[must_use]
    fn new(value: u8) -> Self;

    /// Loads the value with sequentially consistent ordering.
    ///
    /// The caller must observe the loaded state to use this operation's result.
    #[must_use]
    fn load(&self) -> u8;

    /// Replaces `current` with `new` using sequentially consistent ordering.
    ///
    /// # Returns
    ///
    /// Returns `Ok(current)` when the stored value matched `current` and was
    /// replaced with `new`. Returns `Err(actual)` when the stored value did not
    /// match `current`; `actual` is the value observed by the failed operation.
    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8>;

    /// Stores the value with sequentially consistent ordering.
    fn store(&self, value: u8);
}

impl AtomicU8Like for AtomicU8 {
    /// Creates a standard atomic byte.
    #[inline]
    fn new(value: u8) -> Self {
        Self::new(value)
    }

    /// Loads the byte with sequentially consistent ordering.
    #[inline]
    fn load(&self) -> u8 {
        self.load(Ordering::SeqCst)
    }

    /// Compares and exchanges with sequentially consistent ordering.
    #[inline]
    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8> {
        self.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
    }

    /// Stores the byte with sequentially consistent ordering.
    #[inline]
    fn store(&self, value: u8) {
        self.store(value, Ordering::SeqCst);
    }
}
