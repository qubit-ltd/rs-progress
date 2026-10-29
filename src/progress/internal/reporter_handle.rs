// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter ownership retained by a progress operation.

use std::sync::Arc;

use crate::Reporter;

/// Reporter retained by a progress operation.
///
/// # Type Parameters
///
/// * `'reporter` — lifetime of a borrowed reporter. An owned reporter uses
///   `'static`.
pub(crate) enum ReporterHandle<'reporter> {
    /// Reporter borrowed for the operation lifetime.
    Borrowed(&'reporter dyn Reporter),
    /// Shared reporter owned by the operation.
    Owned(Arc<dyn Reporter>),
}

impl ReporterHandle<'_> {
    /// Borrows the reporter regardless of whether this handle owns it.
    ///
    /// # Returns
    ///
    /// A borrow that remains valid for the lifetime of this handle borrow.
    #[must_use]
    #[inline]
    pub(crate) fn as_reporter(&self) -> &dyn Reporter {
        match self {
            Self::Borrowed(reporter) => *reporter,
            Self::Owned(reporter) => reporter.as_ref(),
        }
    }
}
