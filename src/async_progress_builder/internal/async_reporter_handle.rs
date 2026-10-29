// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter ownership used while building asynchronous progress operations.

use std::sync::Arc;

use crate::reporter::AsyncReporter;

/// Selects an asynchronous reporter borrowed for the operation or owned by it.
pub(crate) enum AsyncReporterHandle<'reporter> {
    /// Borrows a reporter whose lifetime covers the progress operation.
    Borrowed(&'reporter dyn AsyncReporter),
    /// Shares ownership of a reporter for the progress operation.
    Owned(Arc<dyn AsyncReporter>),
}

impl AsyncReporterHandle<'_> {
    /// Borrows the selected reporter without changing its ownership.
    #[must_use]
    #[inline]
    pub(crate) fn as_reporter(&self) -> &dyn AsyncReporter {
        match self {
            Self::Borrowed(reporter) => *reporter,
            Self::Owned(reporter) => reporter.as_ref(),
        }
    }
}
