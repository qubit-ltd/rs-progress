// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::Duration;

/// Formats a duration for progress output.
///
/// # Parameters
///
/// * `duration` - Duration to format.
///
/// # Returns
///
/// A compact human-readable duration string.
pub(crate) fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let millis = duration.subsec_millis();
    if total_seconds >= 3_600 {
        format!(
            "{}h {}m {}s",
            total_seconds / 3_600,
            total_seconds % 3_600 / 60,
            total_seconds % 60
        )
    } else if total_seconds >= 60 {
        format!("{}m {}s", total_seconds / 60, total_seconds % 60)
    } else if total_seconds > 0 {
        format!("{total_seconds}.{millis:03}s")
    } else {
        format!("{millis}ms")
    }
}
