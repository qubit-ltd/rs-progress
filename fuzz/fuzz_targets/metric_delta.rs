// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzz target for atomic metric delta transitions.

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_progress::Metric;
use qubit_progress::MetricDelta;
use qubit_progress::MetricHandle;
use qubit_progress::NoopReporter;
use qubit_progress::Progress;

#[path = "metric_delta/reference_model.rs"]
mod reference_model;

use reference_model::ModelDelta;
use reference_model::ModelState;

/// Limits parser and state-machine work for one fuzz input.
const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Limits the number of metric transitions for one fuzz input.
const MAX_OPERATIONS: usize = 512;
/// Number of encoded counters in one operation.
const COUNTER_COUNT: usize = 5;
/// Number of bytes reserved for each encoded counter.
const COUNTER_BYTES: usize = 8;
/// Number of bytes reserved for one encoded operation.
const OPERATION_BYTES: usize = 1 + COUNTER_COUNT * COUNTER_BYTES;

/// Reads one little-endian counter from a possibly short fuzz operation.
#[must_use]
#[inline]
fn read_u64(bytes: &[u8]) -> u64 {
    let mut value = 0;
    for (index, byte) in bytes.iter().copied().take(COUNTER_BYTES).enumerate() {
        value |= u64::from(byte) << (index * 8);
    }
    value
}

/// Maps fuzz bytes to ordinary and boundary counter values.
#[must_use]
#[inline]
fn read_counter(bytes: &[u8]) -> u64 {
    match bytes.first().copied().unwrap_or_default() % 8 {
        0 => 0,
        1 => 1,
        2 => u64::MAX,
        3 => u64::MAX - 1,
        _ => read_u64(bytes),
    }
}

/// Builds the configured metric and its live handle for one fuzz run.
#[must_use = "the progress and metric handle must be used"]
fn create_metric(total: Option<u64>) -> (Progress<'static>, MetricHandle) {
    let metric = match total {
        Some(total) => Metric::new("items", "Items").total(total),
        None => Metric::new("items", "Items"),
    };
    let progress = Progress::builder(&NoopReporter)
        .metric(metric)
        .start()
        .expect("fuzz metric configuration must start");
    let handle = progress.metric("items").expect("configured fuzz metric must exist");
    (progress, handle)
}

/// Maps one operation to a valid delta while preserving the configured limit.
fn legal_delta(operation: &[u8], state: ModelState, total: Option<u64>) -> ModelDelta {
    let limit = u128::from(u64::MAX);
    let completed = state.unclassified + state.succeeded + state.failed + state.cancelled;
    let occupied = state.active + completed;
    let mut start_capacity = limit - occupied;
    if let Some(total) = total {
        start_capacity = start_capacity.min(u128::from(total) - occupied);
    }
    let started = bounded_counter(operation.get(1..9).unwrap_or_default(), start_capacity);
    let available = state.active + u128::from(started);
    let mut remaining = available;
    let mut terminal = [0; 4];
    for (index, value) in terminal.iter_mut().enumerate() {
        *value = bounded_counter(
            operation.get(9 + index * 8..17 + index * 8).unwrap_or_default(),
            remaining,
        );
        remaining -= u128::from(*value);
    }
    ModelDelta {
        started,
        unclassified: terminal[0],
        succeeded: terminal[1],
        failed: terminal[2],
        cancelled: terminal[3],
    }
}

/// Selects a value within an inclusive u128 capacity without overflowing.
fn bounded_counter(bytes: &[u8], capacity: u128) -> u64 {
    let value = u128::from(read_u64(bytes));
    (value % (capacity + 1)) as u64
}

/// Compares the entire public snapshot with a successfully predicted model
/// state.
fn assert_snapshot(metric: &MetricHandle, state: ModelState, total: Option<u64>) {
    let snapshot = metric.snapshot();
    assert_eq!(snapshot.total(), total);
    assert_eq!(u128::from(snapshot.active()), state.active);
    assert_eq!(u128::from(snapshot.unclassified()), state.unclassified);
    assert_eq!(u128::from(snapshot.succeeded()), state.succeeded);
    assert_eq!(u128::from(snapshot.failed()), state.failed);
    assert_eq!(u128::from(snapshot.cancelled()), state.cancelled);
    assert_eq!(
        u128::from(snapshot.completed()),
        state.unclassified + state.succeeded + state.failed + state.cancelled
    );
}

fuzz_target!(|input: &[u8]| {
    if input.len() > MAX_INPUT_BYTES {
        return;
    }

    let total = match input.first().copied().unwrap_or_default() % 5 {
        0 => None,
        1 => Some(0),
        2 => Some(1),
        3 => Some(u64::MAX),
        _ => Some(read_u64(input.get(1..).unwrap_or_default())),
    };
    let (progress, metric) = create_metric(total);
    let mut model = ModelState::default();
    let operation_start = if input.first().copied().unwrap_or_default() % 5 == 4 {
        9
    } else {
        1
    };
    for operation in input
        .get(operation_start..)
        .unwrap_or_default()
        .chunks(OPERATION_BYTES)
        .take(MAX_OPERATIONS)
    {
        let legal = operation.first().copied().unwrap_or_default() % 2 == 1;
        let mut counters = [0; COUNTER_COUNT];
        let delta = if legal {
            let delta = legal_delta(operation, model, total);
            counters = [
                delta.started,
                delta.unclassified,
                delta.succeeded,
                delta.failed,
                delta.cancelled,
            ];
            delta
        } else {
            for (index, counter) in counters.iter_mut().enumerate() {
                let start = 1 + index * COUNTER_BYTES;
                *counter = read_counter(operation.get(start..).unwrap_or_default());
            }
            let [started, unclassified, succeeded, failed, cancelled] = counters;
            ModelDelta {
                started,
                unclassified,
                succeeded,
                failed,
                cancelled,
            }
        };
        let [started, unclassified, succeeded, failed, cancelled] = counters;
        let expected = model.transition(delta, total);
        let delta = MetricDelta::new()
            .started(started)
            .unclassified(unclassified)
            .succeeded(succeeded)
            .failed(failed)
            .cancelled(cancelled);
        let result = metric.apply_delta(delta);
        assert_eq!(
            result.is_ok(),
            expected.is_some(),
            "oracle and apply_delta acceptance must agree"
        );
        if let Some(next) = expected {
            model = next;
        }
        assert_snapshot(&metric, model, total);
    }

    progress.finish_unchecked().expect("disabled fuzz progress must finish");
});
