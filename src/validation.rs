// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared validation for operation configuration and report snapshots.

use std::collections::HashSet;

use crate::ConfigurationError;
use crate::Metric;
#[cfg(feature = "serde")]
use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Stage;

#[cfg(feature = "serde")]
mod internal;
#[cfg(feature = "serde")]
use internal::SnapshotValidationError;

/// Validates that an operation has unique metric IDs and nonblank IDs/names.
///
/// # Parameters
///
/// - `metrics`: Metric definitions in their configured order.
///
/// # Returns
///
/// `Ok(())` when every metric is valid.
///
/// # Errors
///
/// Returns the first configuration error for an empty list, blank ID or name,
/// or duplicate ID.
pub(crate) fn validate_metrics(metrics: &[Metric]) -> Result<(), ConfigurationError> {
    if metrics.is_empty() {
        return Err(ConfigurationError::NoMetrics);
    }
    let mut ids = HashSet::with_capacity(metrics.len());
    for (index, metric) in metrics.iter().enumerate() {
        if metric.id.trim().is_empty() {
            return Err(ConfigurationError::EmptyMetricId { index });
        }
        if metric.name.trim().is_empty() {
            return Err(ConfigurationError::EmptyMetricName {
                metric_id: metric.id.to_string(),
            });
        }
        if !ids.insert(&metric.id) {
            return Err(ConfigurationError::DuplicateMetricId {
                metric_id: metric.id.to_string(),
            });
        }
    }
    Ok(())
}

/// Validates stage identifiers and optional one-based sequence metadata.
///
/// # Parameters
///
/// - `stage`: Stage metadata to validate.
///
/// # Returns
///
/// `Ok(())` when identifiers are nonblank and position metadata is valid.
///
/// # Errors
///
/// Returns a configuration error for blank identifiers, incomplete position
/// metadata, or a position outside `1..=total`.
pub(crate) fn validate_stage(stage: &Stage) -> Result<(), ConfigurationError> {
    if stage.id.trim().is_empty() {
        return Err(ConfigurationError::EmptyStageId);
    }
    if stage.name.trim().is_empty() {
        return Err(ConfigurationError::EmptyStageName);
    }
    match (stage.position, stage.total) {
        (None, None) => Ok(()),
        (Some(position), Some(total)) if position > 0 && position <= total => Ok(()),
        (Some(position), Some(total)) => Err(ConfigurationError::InvalidStagePosition { position, total }),
        _ => Err(ConfigurationError::IncompleteStagePosition),
    }
}

/// Ensures operation correlation attributes use nonblank keys.
///
/// # Parameters
///
/// - `attributes`: Attributes attached to the operation.
///
/// # Returns
///
/// `Ok(())` when every key contains a non-whitespace character.
///
/// # Errors
///
/// Returns [`ConfigurationError::EmptyAttributeKey`] for the first blank key.
pub(crate) fn validate_attributes(attributes: &OperationAttributes) -> Result<(), ConfigurationError> {
    if let Some((key, _)) = attributes.iter().find(|(key, _)| key.trim().is_empty()) {
        return Err(ConfigurationError::EmptyAttributeKey { key: key.to_owned() });
    }
    Ok(())
}

/// Checks snapshot count consistency before accepting serialized data.
///
/// # Parameters
///
/// - `snapshot`: Dynamic metric counts to validate.
///
/// # Returns
///
/// `Ok(())` when classified counts fit within completed work and all counts
/// fit within the configured total.
///
/// # Errors
///
/// Returns a snapshot validation error for overflow or inconsistent counts.
#[cfg(feature = "serde")]
pub(crate) fn validate_snapshot_counts(snapshot: &MetricSnapshot) -> Result<(), SnapshotValidationError> {
    let classified = snapshot
        .succeeded()
        .checked_add(snapshot.failed())
        .and_then(|value| value.checked_add(snapshot.cancelled()))
        .ok_or_else(|| SnapshotValidationError::CountOverflow {
            metric_id: snapshot.id().into(),
        })?;
    if classified > snapshot.completed() {
        return Err(SnapshotValidationError::ClassifiedExceedsCompleted {
            metric_id: snapshot.id().into(),
        });
    }
    if let Some(total) = snapshot.total() {
        if total == 0
            && (snapshot.completed() != 0
                || snapshot.active() != 0
                || snapshot.succeeded() != 0
                || snapshot.failed() != 0
                || snapshot.cancelled() != 0)
        {
            return Err(SnapshotValidationError::NonZeroCountsForZeroTotal {
                metric_id: snapshot.id().into(),
            });
        }
        let occupied = snapshot.completed().checked_add(snapshot.active()).ok_or_else(|| {
            SnapshotValidationError::CountOverflow {
                metric_id: snapshot.id().into(),
            }
        })?;
        if occupied > total {
            return Err(SnapshotValidationError::CountsExceedTotal {
                metric_id: snapshot.id().into(),
            });
        }
    }
    Ok(())
}
