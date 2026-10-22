// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle state values for one progress operation.

/// Lifecycle state visible to metric transition errors.
///
/// `Open` permits metric transitions, `Finishing` rejects new transitions
/// while terminal validation runs, and `Closed` marks an immutable operation.
///
/// # Examples
///
/// ```
/// use qubit_progress::OperationLifecycle;
///
/// assert_eq!(OperationLifecycle::Open, OperationLifecycle::Open);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum OperationLifecycle {
    /// Metric transitions may enter the operation.
    Open = 0,
    /// New transitions are rejected while terminal validation runs.
    Finishing = 1,
    /// The operation can no longer be mutated.
    Closed = 2,
}
