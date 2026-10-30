// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::metric::internal::metric_atomic_word::MetricAtomicWord;

/// RAII wrapper that always releases a locked gate.
pub(in crate::metric) struct MetricGateGuard<'gate, G: MetricAtomicWord> {
    gate: &'gate G,
    next_version: u64,
}

impl<'gate, G: MetricAtomicWord> MetricGateGuard<'gate, G> {
    /// Creates a guard that releases the gate at `next_version` on drop.
    #[inline]
    pub fn new(gate: &'gate G, next_version: u64) -> Self {
        Self { gate, next_version }
    }
}

impl<G: MetricAtomicWord> Drop for MetricGateGuard<'_, G> {
    /// Releases the gate by publishing the next even version.
    fn drop(&mut self) {
        self.gate.store(self.next_version);
    }
}
