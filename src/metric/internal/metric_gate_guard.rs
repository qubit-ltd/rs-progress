// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_fast_cas::CasCell;

/// RAII wrapper that always releases a locked gate.
pub struct MetricGateGuard<'gate> {
    gate: &'gate CasCell,
    next_version: u64,
}

impl<'gate> MetricGateGuard<'gate> {
    /// Creates a guard that releases the gate at `next_version` on drop.
    #[inline]
    pub fn new(gate: &'gate CasCell, next_version: u64) -> Self {
        Self { gate, next_version }
    }
}

impl Drop for MetricGateGuard<'_> {
    /// Releases the gate by publishing the next even version.
    fn drop(&mut self) {
        self.gate.store(self.next_version);
    }
}
