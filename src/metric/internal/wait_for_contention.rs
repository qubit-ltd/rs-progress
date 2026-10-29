// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::hint::spin_loop;
use std::thread;

/// Reduces CPU pressure while a metric operation retries after contention.
///
/// The caller increments `attempts` after each unsuccessful gate observation
/// or update claim. Most retries use the processor spin hint; every sixteenth
/// retry yields the current thread so other work can run.
///
/// # Parameters
///
/// * `attempts` - Number of contention retries already observed by the caller.
#[inline]
pub fn wait_for_contention(attempts: usize) {
    if attempts > 0 && attempts.is_multiple_of(16) {
        thread::yield_now();
    } else {
        spin_loop();
    }
}
