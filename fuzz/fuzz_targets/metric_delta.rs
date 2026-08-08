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
use qubit_progress::MetricSnapshot;
use qubit_progress::NoopReporter;
use qubit_progress::Progress;

/// Limits parser and state-machine work for one fuzz input.
const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Limits the number of model transitions for one fuzz input.
const MAX_OPERATIONS: usize = 512;
/// Number of encoded counters in one operation.
const COUNTER_COUNT: usize = 5;
/// Number of bytes reserved for each encoded counter.
const COUNTER_BYTES: usize = 8;
/// Number of bytes reserved for one encoded operation.
const OPERATION_BYTES: usize = 1 + COUNTER_COUNT * COUNTER_BYTES;

/// Reference model for the public metric counters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ModelCounts {
    /// Work currently active.
    active: u64,
    /// Completed work without an explicit outcome.
    unclassified: u64,
    /// Successfully completed work.
    succeeded: u64,
    /// Failed work.
    failed: u64,
    /// Cancelled work.
    cancelled: u64,
}

impl ModelCounts {
    /// Applies a delta if all metric invariants accept it.
    fn apply(
        &mut self,
        delta: [u64; COUNTER_COUNT],
        total: Option<u64>,
    ) -> bool {
        let [started, unclassified, succeeded, failed, cancelled] = delta;
        let terminal = unclassified
            .checked_add(succeeded)
            .and_then(|value| value.checked_add(failed))
            .and_then(|value| value.checked_add(cancelled));
        let Some(terminal) = terminal else {
            return false;
        };

        let Some(available_active) = self.active.checked_add(started) else {
            return false;
        };
        if terminal > available_active {
            return false;
        }
        let active = available_active - terminal;
        let Some(next_unclassified) =
            self.unclassified.checked_add(unclassified)
        else {
            return false;
        };
        let Some(next_succeeded) = self.succeeded.checked_add(succeeded) else {
            return false;
        };
        let Some(next_failed) = self.failed.checked_add(failed) else {
            return false;
        };
        let Some(next_cancelled) = self.cancelled.checked_add(cancelled) else {
            return false;
        };
        let Some(completed) = next_unclassified
            .checked_add(next_succeeded)
            .and_then(|value| value.checked_add(next_failed))
            .and_then(|value| value.checked_add(next_cancelled))
        else {
            return false;
        };
        let Some(occupied) = completed.checked_add(active) else {
            return false;
        };
        if total.is_some_and(|limit| occupied > limit) {
            return false;
        }

        self.active = active;
        self.unclassified = next_unclassified;
        self.succeeded = next_succeeded;
        self.failed = next_failed;
        self.cancelled = next_cancelled;
        true
    }

    /// Verifies that a live metric snapshot matches the reference state.
    fn assert_matches(&self, snapshot: &MetricSnapshot) {
        assert_eq!(snapshot.active(), self.active);
        assert_eq!(snapshot.unclassified(), self.unclassified);
        assert_eq!(snapshot.succeeded(), self.succeeded);
        assert_eq!(snapshot.failed(), self.failed);
        assert_eq!(snapshot.cancelled(), self.cancelled);
        assert_eq!(snapshot.completed(), self.completed());
    }

    /// Returns the total completed count represented by the model.
    const fn completed(self) -> u64 {
        self.unclassified
            .saturating_add(self.succeeded)
            .saturating_add(self.failed)
            .saturating_add(self.cancelled)
    }
}

/// Reads one little-endian counter from a possibly short fuzz operation.
fn read_u64(bytes: &[u8]) -> u64 {
    let mut value = 0;
    for (index, byte) in bytes.iter().copied().take(COUNTER_BYTES).enumerate() {
        value |= u64::from(byte) << (index * 8);
    }
    value
}

/// Maps fuzz bytes to ordinary and boundary counter values.
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
fn create_metric(total: Option<u64>) -> (Progress<'static>, MetricHandle) {
    let metric = match total {
        Some(total) => Metric::new("items", "Items").total(total),
        None => Metric::new("items", "Items"),
    };
    let progress = Progress::builder(&NoopReporter)
        .metric(metric)
        .start()
        .expect("fuzz metric configuration must start");
    let handle = progress
        .metric("items")
        .expect("configured fuzz metric must exist");
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
    let mut model = ModelCounts::default();

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
        let delta = MetricDelta::new()
            .started(counters[0])
            .unclassified(counters[1])
            .succeeded(counters[2])
            .failed(counters[3])
            .cancelled(counters[4]);
        let before = metric.snapshot();
        let expected_success = model.apply(counters, total);
        let result = metric.apply_delta(delta);
        assert_eq!(
            result.is_ok(),
            expected_success,
            "delta result mismatch: operation={operation:?}, counters={counters:?}, total={total:?}, model={model:?}, result={result:?}",
        );
        let after = metric.snapshot();
        model.assert_matches(&after);
        if result.is_err() {
            assert_eq!(after, before, "failed delta must not partially commit");
        }
    }

    progress
        .finish_unchecked()
        .expect("disabled fuzz progress must finish");
});
