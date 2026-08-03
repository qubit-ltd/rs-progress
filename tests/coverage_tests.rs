// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Coverage-only entry points for library paths that need deterministic setup.

#[cfg(coverage)]
#[test]
fn test_library_coverage_hooks() {
    qubit_progress::__coverage_internal();
    qubit_progress::__coverage_progress_edges();
    #[cfg(feature = "json-lines")]
    qubit_progress::__coverage_event_serde();
}
