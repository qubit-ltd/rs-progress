// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Log reporter behavior.

#[cfg(feature = "log")]
use qubit_progress::{LogReporter, Reporter};

/// Verifies that the log sink samples the facade info-level setting.
#[cfg(feature = "log")]
#[test]
fn test_log_reporter_samples_info_level_enablement() {
    let reporter = LogReporter;
    assert_eq!(reporter.is_enabled(), log::log_enabled!(log::Level::Info));
}
