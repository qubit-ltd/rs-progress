// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Operation lifecycle, metric state and report scheduling.
// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

#[cfg(coverage)]
use crate::Metric;
#[cfg(coverage)]
use crate::NoopReporter;
use crate::error::StartError;

mod internal;
mod progress_builder;
#[path = "progress/progress.rs"]
mod progress_impl;

#[cfg(coverage)]
use internal::coverage_terminal_reporter::CoverageTerminalReporter;
pub(crate) use internal::operation_id::allocate_operation_id_from;
#[cfg(coverage)]
use internal::operation_id::allocate_operation_id_from as allocate_operation_id_from_coverage;
pub use progress_builder::ProgressBuilder;
pub use progress_impl::Progress;

/// Process-local source of nonzero operation identifiers.
pub(crate) static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Allocates a nonzero operation ID without wrapping or reuse.
#[inline(never)]
pub(crate) fn allocate_operation_id() -> Result<u64, StartError> {
    allocate_operation_id_from(&NEXT_OPERATION_ID)
}

/// Exercises progress-only edge paths in coverage builds.
///
/// # Panics
///
/// Panics if an expected operation-ID or terminal-delivery edge case is not
/// observed.
#[doc(hidden)]
#[cfg(coverage)]
pub fn __coverage_progress_edges() {
    assert!(matches!(
        allocate_operation_id_from_coverage(&AtomicU64::new(0)),
        Err(StartError::OperationIdExhausted)
    ));

    let progress = Progress::builder(&NoopReporter)
        .metric(Metric::new("coverage", "Coverage"))
        .start()
        .expect("coverage progress must start");
    progress.cancel().expect("coverage progress must cancel");

    let reporter = CoverageTerminalReporter {
        attempts: AtomicU64::new(0),
    };
    let progress = Progress::builder_arc(Arc::new(reporter))
        .metric(Metric::new("coverage-terminal", "Coverage terminal"))
        .start()
        .expect("coverage terminal progress must start");
    assert!(progress.cancel().is_err());
}
