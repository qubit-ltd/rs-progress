// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle states published by the operation gate.

/// Lifecycle states published by an
/// [`crate::internal::operation_gate::OperationGate`].
#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateLifecycle {
    /// New updates may be admitted.
    Open = 0,
    /// New updates are rejected while admitted updates drain.
    Finishing = 1,
    /// The operation is permanently closed.
    Closed = 2,
}
