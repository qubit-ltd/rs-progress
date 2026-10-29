// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Atomic counter operations used by the operation gate.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Atomic counter operations required by
/// [`crate::internal::operation_gate::OperationGate`].
pub trait AtomicUsizeLike {
    /// Creates a counter initialized to `value`.
    ///
    /// The returned counter must be retained for later operations.
    #[must_use]
    fn new(value: usize) -> Self;

    /// Returns the current counter value using sequentially consistent ordering.
    ///
    /// The loaded value must be observed by the caller.
    #[must_use]
    fn load(&self) -> usize;

    /// Adds `value` to the counter using sequentially consistent ordering.
    ///
    /// The operation updates shared state, so its return value is not needed.
    fn fetch_add(&self, value: usize);

    /// Subtracts `value` from the counter using sequentially consistent ordering.
    ///
    /// The operation updates shared state, so its return value is not needed.
    fn fetch_sub(&self, value: usize);
}

impl AtomicUsizeLike for AtomicUsize {
    /// Creates a standard atomic counter.
    #[inline]
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    /// Loads the counter with sequentially consistent ordering.
    #[inline]
    fn load(&self) -> usize {
        self.load(Ordering::SeqCst)
    }

    /// Adds with sequentially consistent ordering.
    #[inline]
    fn fetch_add(&self, value: usize) {
        self.fetch_add(value, Ordering::SeqCst);
    }

    /// Subtracts with sequentially consistent ordering.
    #[inline]
    fn fetch_sub(&self, value: usize) {
        self.fetch_sub(value, Ordering::SeqCst);
    }
}
