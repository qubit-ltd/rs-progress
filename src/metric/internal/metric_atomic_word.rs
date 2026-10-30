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
    fn new(value: u64) -> Self;

    /// Loads the word with acquire ordering.
    fn load(&self) -> u64;

    /// Stores the word with release ordering.
    fn store(&self, value: u64);

    /// Replaces the expected word with the next word using AcqRel/Acquire
    /// orderings.
    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64>;
}

impl MetricAtomicWord for CasCell {
    fn new(value: u64) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u64 {
        CasCell::load(self)
    }

    fn store(&self, value: u64) {
        CasCell::store(self, value);
    }

    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64> {
        CasCell::compare_set(self, expected, next)
    }
}

impl MetricAtomicWord for AtomicU64 {
    fn new(value: u64) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u64 {
        self.load(Ordering::Acquire)
    }

    fn store(&self, value: u64) {
        self.store(value, Ordering::Release);
    }

    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64> {
        self.compare_exchange(expected, next, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
    }
}
