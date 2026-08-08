// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Coverage-only entry points for library paths that need deterministic setup.

#[cfg(coverage)]
use qubit_progress::__coverage_event_serde;
#[cfg(coverage)]
use qubit_progress::__coverage_internal;
#[cfg(coverage)]
use qubit_progress::__coverage_progress_edges;

#[cfg(coverage)]
#[test]
fn test_library_coverage_hooks() {
    __coverage_internal();
    __coverage_progress_edges();
    #[cfg(feature = "json-lines")]
    __coverage_event_serde();
}
