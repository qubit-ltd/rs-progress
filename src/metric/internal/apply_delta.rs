// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::MetricError;
use crate::metric::MetricDelta;
use crate::metric::internal::MetricCounts;

/// Applies an additive metric delta to the current counts.
///
/// The delta's terminal counts consume active operations after accounting for
/// newly started operations. On success, the counts are updated in place. The
/// caller should apply the delta to a temporary copy when it needs to preserve
/// the original counts if an error occurs.
///
/// # Parameters
///
/// - `counts`: Mutable counts to update.
/// - `delta`: Additions to the started, completed, and terminal counts.
/// - `metric_id`: Identifier included in count overflow errors.
///
/// # Returns
///
/// Returns `Ok(())` after all counts have been updated.
///
/// # Errors
///
/// Returns [`MetricError::CountOverflow`] if adding any delta or accumulated
/// count exceeds the integer range. Returns [`MetricError::InsufficientActive`]
/// if terminal operations exceed the active operations available after starts.
pub fn apply_delta_to_counts(
    counts: &mut MetricCounts,
    delta: MetricDelta,
    metric_id: &str,
) -> Result<(), MetricError> {
    let terminal_delta = delta
        .unclassified
        .checked_add(delta.succeeded)
        .and_then(|value| value.checked_add(delta.failed))
        .and_then(|value| value.checked_add(delta.cancelled))
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    let available_active = counts
        .active
        .checked_add(delta.started)
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    if terminal_delta > available_active {
        return Err(MetricError::InsufficientActive {
            metric_id: metric_id.into(),
            requested: terminal_delta,
            available: available_active,
        });
    }

    counts.active = available_active - terminal_delta;
    counts.completed_unclassified = counts
        .completed_unclassified
        .checked_add(delta.unclassified)
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    counts.succeeded = counts
        .succeeded
        .checked_add(delta.succeeded)
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    counts.failed = counts
        .failed
        .checked_add(delta.failed)
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    counts.cancelled = counts
        .cancelled
        .checked_add(delta.cancelled)
        .ok_or_else(|| MetricError::CountOverflow {
            metric_id: metric_id.into(),
        })?;
    Ok(())
}
