// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Atomic byte operations used by the operation gate.

/// Atomic byte operations required by
/// [`crate::internal::operation_gate::OperationGate`].
pub trait AtomicU8Like {
    /// Creates an atomic value with the supplied initial state.
    fn new(value: u8) -> Self;

    /// Loads the value with acquire ordering.
    fn load(&self) -> u8;

    /// Performs an acquire-release compare-and-exchange.
    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8>;

    /// Stores the value with release ordering.
    fn store(&self, value: u8);
}
