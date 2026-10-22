// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::hint::spin_loop;
use std::thread;

/// Busy-wait helper for writer contention and snapshot retries.
#[inline]
pub fn wait_for_contention(attempts: usize) {
    if attempts > 0 && attempts.is_multiple_of(16) {
        thread::yield_now();
    } else {
        spin_loop();
    }
}
