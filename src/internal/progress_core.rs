// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! State-independent event construction shared by synchronous and async
//! progress.

use std::sync::Arc;
use std::time::Duration;

use crate::Event;
use crate::MetricHandle;
use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Phase;
use crate::Stage;
use crate::error::CompletionError;
use crate::error::EmissionError;

/// Copies all metrics into one independently consistent event snapshot.
pub(crate) fn metric_snapshots(metrics: &[MetricHandle]) -> Vec<MetricSnapshot> {
    metrics.iter().map(MetricHandle::snapshot).collect()
}

/// Validates that no work remains and every known total has been completed.
pub(crate) fn validate_finish(metrics: &[MetricHandle]) -> Result<(), CompletionError> {
    for metric in metrics {
        let snapshot = metric.snapshot();
        if snapshot.active() != 0 {
            return Err(CompletionError::ActiveWork {
                metric_id: snapshot.id().to_owned(),
                active: snapshot.active(),
            });
        }
        if let Some(total) = snapshot.total()
            && snapshot.completed() != total
        {
            return Err(CompletionError::IncompleteTotal {
                metric_id: snapshot.id().to_owned(),
                completed: snapshot.completed(),
                total,
            });
        }
    }
    Ok(())
}

/// Builds one immutable event while reserving its delivery sequence.
pub(crate) fn build_event(
    operation_id: Option<u64>,
    next_sequence: &mut u64,
    phase: Phase,
    stage: Option<&Stage>,
    attributes: &Arc<OperationAttributes>,
    metrics: Vec<MetricSnapshot>,
    elapsed: Duration,
) -> Result<Event, EmissionError> {
    let operation_id = operation_id.ok_or(EmissionError::SequenceExhausted)?;
    let sequence = *next_sequence;
    *next_sequence = sequence.checked_add(1).ok_or(EmissionError::SequenceExhausted)?;
    Ok(Event::new(
        operation_id,
        sequence,
        phase,
        stage.cloned(),
        Arc::clone(attributes),
        metrics,
        elapsed,
    ))
}
