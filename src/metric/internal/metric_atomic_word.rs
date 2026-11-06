// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use qubit_fast_cas::CasCell;

/// Atomic operations required by the metric counter protocol.
pub(in crate::metric) trait MetricAtomicWord {
    /// Creates an atomic word with the specified initial value.
    ///
    /// # Parameters
    ///
    /// - `value`: Initial word value.
    ///
    /// # Returns
    ///
    /// A word initialized with `value`.
    #[must_use]
    fn new(value: u64) -> Self;

    /// Loads the word with acquire ordering.
    #[must_use]
    fn load(&self) -> u64;

    /// Stores the word with release ordering.
    ///
    /// # Parameters
    ///
    /// - `value`: Word value to publish.
    fn store(&self, value: u64);

    /// Replaces the expected word with the next word using AcqRel/Acquire
    /// orderings.
    ///
    /// # Parameters
    ///
    /// - `expected`: Value required for the replacement to succeed.
    /// - `next`: Value to store when the current word matches `expected`.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the replacement succeeds.
    ///
    /// # Errors
    ///
    /// Returns the observed word when it differs from `expected`.
    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64>;
}

impl MetricAtomicWord for CasCell {
    /// Creates a `CasCell` initialized with `value`.
    ///
    /// # Parameters
    ///
    /// - `value`: Initial word value.
    ///
    /// # Returns
    ///
    /// A cell initialized with `value`.
    #[inline]
    fn new(value: u64) -> Self {
        Self::new(value)
    }

    /// Loads the cell's word using acquire ordering.
    ///
    /// # Returns
    ///
    /// The currently stored word.
    #[inline]
    fn load(&self) -> u64 {
        CasCell::load(self)
    }

    /// Stores `value` in the cell using release ordering.
    ///
    /// # Parameters
    ///
    /// - `value`: Word value to publish.
    #[inline]
    fn store(&self, value: u64) {
        CasCell::store(self, value);
    }

    /// Replaces the cell's word using AcqRel/Acquire ordering.
    ///
    /// # Parameters
    ///
    /// - `expected`: Value required for the replacement to succeed.
    /// - `next`: Value to store when the current word matches `expected`.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the replacement succeeds.
    ///
    /// # Errors
    ///
    /// Returns the observed word when it differs from `expected`.
    #[inline]
    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64> {
        CasCell::compare_set(self, expected, next)
    }
}

impl MetricAtomicWord for AtomicU64 {
    /// Creates an atomic integer initialized with `value`.
    ///
    /// # Parameters
    ///
    /// - `value`: Initial word value.
    ///
    /// # Returns
    ///
    /// An atomic integer initialized with `value`.
    #[inline]
    fn new(value: u64) -> Self {
        Self::new(value)
    }

    /// Loads the integer using acquire ordering.
    ///
    /// # Returns
    ///
    /// The currently stored word.
    #[inline]
    fn load(&self) -> u64 {
        self.load(Ordering::Acquire)
    }

    /// Stores `value` using release ordering.
    ///
    /// # Parameters
    ///
    /// - `value`: Word value to publish.
    #[inline]
    fn store(&self, value: u64) {
        self.store(value, Ordering::Release);
    }

    /// Replaces the integer using AcqRel/Acquire ordering.
    ///
    /// # Parameters
    ///
    /// - `expected`: Value required for the replacement to succeed.
    /// - `next`: Value to store when the current word matches `expected`.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the replacement succeeds.
    ///
    /// # Errors
    ///
    /// Returns the observed word when it differs from `expected`.
    #[inline]
    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64> {
        self.compare_exchange(expected, next, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
    }
}
