/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::ProgressEvent;

/// Receives immutable progress events.
///
/// # Type Parameters
///
/// * `C` - Caller-defined event context type.
pub trait ProgressReporter<C>: Send + Sync {
    /// Reports one progress event.
    ///
    /// # Parameters
    ///
    /// * `event` - Immutable progress event to report.
    ///
    /// # Panics
    ///
    /// Reporter implementations may panic if their output sink fails. Callers
    /// decide whether reporter panics are propagated or isolated.
    fn report(&self, event: &ProgressEvent<C>);
}
