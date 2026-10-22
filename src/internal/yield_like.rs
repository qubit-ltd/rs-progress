// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Scheduling hooks used by the operation gate.

/// Scheduling hooks used while waiting for registered updates to leave.
pub trait YieldLike {
    /// Gives the processor a hint that the current thread is spinning.
    fn spin_loop();

    /// Yields the current thread to make progress in a model scheduler.
    fn yield_now();
}
