// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared validation for operation configuration and report snapshots.

use std::collections::HashSet;

use crate::{
    Metric,
    Stage,
    ValidationError,
};

#[cfg(feature = "serde")]
use crate::MetricSnapshot;

/// Validates the fixed metric configuration for one operation.
pub(crate) fn validate_metrics(
    metrics: &[Metric],
) -> Result<(), ValidationError> {
    if metrics.is_empty() {
        return Err(ValidationError::NoMetrics);
    }
    let mut ids = HashSet::with_capacity(metrics.len());
    for (index, metric) in metrics.iter().enumerate() {
        if metric.id.trim().is_empty() {
            return Err(ValidationError::EmptyMetricId { index });
        }
        if metric.name.trim().is_empty() {
            return Err(ValidationError::EmptyMetricName {
                metric_id: metric.id.to_string(),
            });
        }
        if !ids.insert(&metric.id) {
            return Err(ValidationError::DuplicateMetricId {
                metric_id: metric.id.to_string(),
            });
        }
    }
    Ok(())
}

/// Validates optional stage metadata.
pub(crate) fn validate_stage(stage: &Stage) -> Result<(), ValidationError> {
    if stage.id.trim().is_empty() {
        return Err(ValidationError::EmptyStageId);
    }
    if stage.name.trim().is_empty() {
        return Err(ValidationError::EmptyStageName);
    }
    match (stage.position, stage.total) {
        (None, None) => Ok(()),
        (Some(position), Some(total)) if position > 0 && position <= total => {
            Ok(())
        }
        (Some(position), Some(total)) => {
            Err(ValidationError::InvalidStagePosition { position, total })
        }
        _ => Err(ValidationError::IncompleteStagePosition),
    }
}

/// Validates one metric's dynamic counts against its configured total.
#[cfg(feature = "serde")]
pub(crate) fn validate_snapshot_counts(
    snapshot: &MetricSnapshot,
) -> Result<(), ValidationError> {
    let classified = snapshot
        .succeeded()
        .checked_add(snapshot.failed())
        .and_then(|value| value.checked_add(snapshot.cancelled()))
        .ok_or_else(|| ValidationError::CountOverflow {
            metric_id: snapshot.id().into(),
        })?;
    if classified > snapshot.completed() {
        return Err(ValidationError::ClassifiedExceedsCompleted {
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
            return Err(ValidationError::NonZeroCountsForZeroTotal {
                metric_id: snapshot.id().into(),
            });
        }
        let occupied = snapshot
            .completed()
            .checked_add(snapshot.active())
            .ok_or_else(|| ValidationError::CountOverflow {
                metric_id: snapshot.id().into(),
            })?;
        if occupied > total {
            return Err(ValidationError::CountsExceedTotal {
                metric_id: snapshot.id().into(),
            });
        }
    }
    Ok(())
}
