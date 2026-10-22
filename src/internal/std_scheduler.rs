// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standard-library scheduling implementation for the operation gate.

use super::yield_like::YieldLike;

/// Standard-library scheduling implementation.
#[allow(dead_code)]
pub struct StdScheduler;

impl YieldLike for StdScheduler {
    /// Uses the standard processor spin hint.
    fn spin_loop() {
        std::hint::spin_loop();
    }

    /// Yields to another runnable operating-system thread.
    fn yield_now() {
        std::thread::yield_now();
    }
}
