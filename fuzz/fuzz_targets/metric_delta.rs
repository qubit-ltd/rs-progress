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

fuzz_target!(|input: &[u8]| {
    if input.len() > MAX_INPUT_BYTES {
        return;
    }

    let total = match input.first().copied().unwrap_or_default() % 4 {
        0 => None,
        1 => Some(0),
        2 => Some(1),
        _ => Some(u64::MAX),
    };
    let (progress, metric) = create_metric(total);
    for operation in input
        .get(1..)
        .unwrap_or_default()
        .chunks(OPERATION_BYTES)
        .take(MAX_OPERATIONS)
    {
        let mut counters = [0; COUNTER_COUNT];
        for (index, counter) in counters.iter_mut().enumerate() {
            let start = 1 + index * COUNTER_BYTES;
            *counter = read_counter(operation.get(start..).unwrap_or_default());
        }
        let [started, unclassified, succeeded, failed, cancelled] = counters;
        let delta = MetricDelta::new()
            .started(started)
            .unclassified(unclassified)
            .succeeded(succeeded)
            .failed(failed)
            .cancelled(cancelled);
        let before = metric.snapshot();
        let result = metric.apply_delta(delta);
        let after = metric.snapshot();
        if result.is_err() {
            assert_eq!(after, before, "failed delta must not partially commit");
        } else {
            assert_eq!(
                after.active(),
                before.active() + started - unclassified - succeeded - failed - cancelled
            );
            assert_eq!(after.unclassified(), before.unclassified() + unclassified);
            assert_eq!(after.succeeded(), before.succeeded() + succeeded);
            assert_eq!(after.failed(), before.failed() + failed);
            assert_eq!(after.cancelled(), before.cancelled() + cancelled);
            assert_eq!(
                after.completed(),
                after.unclassified() + after.succeeded() + after.failed() + after.cancelled()
            );
        }
    }

    progress.finish_unchecked().expect("disabled fuzz progress must finish");
});
