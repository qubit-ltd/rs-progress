// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

/// Shared status for a background running-progress reporter.
///
/// A status becomes failed when the background reporter returns an error or
/// panics. It intentionally does not retain the concrete error; callers must
/// use [`RunningProgressGuard::stop_and_join`](crate::RunningProgressGuard::stop_and_join)
/// to obtain that error or resume the panic.
///
/// # Author
///
/// Haixing Hu
#[derive(Clone, Debug)]
pub struct RunningProgressStatus {
    /// Whether the background reporter has failed.
    failed: Arc<AtomicBool>,
}

impl RunningProgressStatus {
    /// Creates an inactive status that has not failed.
    ///
    /// # Returns
    ///
    /// A status used when no background reporter is running.
    #[inline]
    pub(crate) fn inactive() -> Self {
        Self {
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns whether the background reporter has failed.
    ///
    /// # Returns
    ///
    /// `true` after the reporter has returned an error or panicked; otherwise
    /// `false`.
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// Marks the background reporter as failed.
    #[inline]
    pub(crate) fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }
}
