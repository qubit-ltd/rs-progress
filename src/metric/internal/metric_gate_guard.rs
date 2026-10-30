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
    /// Gate whose version is held odd while the update is in progress.
    gate: &'gate G,
    /// Even version to publish when releasing the gate.
    next_version: u64,
}

impl<'gate, G: MetricAtomicWord> MetricGateGuard<'gate, G> {
    /// Creates a guard that releases the gate at `next_version` on drop.
    ///
    /// The caller supplies the next even version; this guard stores it with
    /// release ordering when dropped, including during unwinding. Version
    /// calculation, including wrapping, is performed by the caller.
    ///
    /// # Parameters
    ///
    /// * `gate` is the atomic gate to release.
    /// * `next_version` is the even version to publish when this guard drops.
    ///
    /// # Returns
    ///
    /// A guard that must remain alive until the protected update is complete.
    #[must_use = "dropping the guard releases the gate"]
    #[inline]
    pub fn new(gate: &'gate G, next_version: u64) -> Self {
        Self { gate, next_version }
    }
}

impl<G: MetricAtomicWord> Drop for MetricGateGuard<'_, G> {
    /// Releases the gate by publishing the supplied next even version.
    ///
    /// The store uses release ordering through [`MetricAtomicWord::store`].
    fn drop(&mut self) {
        self.gate.store(self.next_version);
    }
}
