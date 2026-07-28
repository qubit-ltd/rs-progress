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
    MetricCounts,
    Stage,
    ValidationError,
};

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
pub(crate) fn validate_counts(
    metric: &Metric,
    counts: MetricCounts,
) -> Result<(), ValidationError> {
    let classified =
        counts.succeeded.checked_add(counts.failed).ok_or_else(|| {
            ValidationError::CountOverflow {
                metric_id: metric.id.to_string(),
            }
        })?;
    if classified > counts.completed {
        return Err(ValidationError::ClassifiedExceedsCompleted {
            metric_id: metric.id.to_string(),
        });
    }
    if let Some(total) = metric.total {
        if total == 0
            && (counts.completed != 0
                || counts.active != 0
                || counts.succeeded != 0
                || counts.failed != 0)
        {
            return Err(ValidationError::NonZeroCountsForZeroTotal {
                metric_id: metric.id.to_string(),
            });
        }
        let occupied =
            counts.completed.checked_add(counts.active).ok_or_else(|| {
                ValidationError::CountOverflow {
                    metric_id: metric.id.to_string(),
                }
            })?;
        if occupied > total {
            return Err(ValidationError::CountsExceedTotal {
                metric_id: metric.id.to_string(),
            });
        }
    }
    Ok(())
}
