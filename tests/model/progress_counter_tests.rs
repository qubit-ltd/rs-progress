// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressCounter`.

use qubit_progress::model::ProgressCounter;

#[test]
fn test_progress_counter_calculate_remaining_and_fraction() {
    let counter = ProgressCounter::new("entries")
        .total(10)
        .completed(4)
        .active(2)
        .succeeded(3)
        .failed(1);

    assert_eq!(counter.metric_id(), "entries");
    assert_eq!(counter.total_count(), Some(10));
    assert_eq!(counter.completed_count(), 4);
    assert_eq!(counter.active_count(), 2);
    assert_eq!(counter.succeeded_count(), 3);
    assert_eq!(counter.failed_count(), 1);
    assert_eq!(counter.remaining_count(), Some(4));
    assert_eq!(counter.progress_fraction(), Some(0.4));
    assert_eq!(counter.progress_percent(), Some(40.0));
}

#[test]
fn test_progress_counter_handle_unknown_and_zero_total() {
    let unknown = ProgressCounter::new("entries").completed(3);
    assert_eq!(unknown.total_count(), None);
    assert_eq!(unknown.remaining_count(), None);
    assert_eq!(unknown.progress_fraction(), None);

    let empty = ProgressCounter::new("entries").total(0);
    assert_eq!(empty.remaining_count(), Some(0));
    assert_eq!(empty.progress_fraction(), Some(1.0));
    assert_eq!(empty.progress_percent(), Some(100.0));
}

#[test]
fn test_progress_counter_remaining_count_saturates_without_overflow() {
    let counter = ProgressCounter::new("bytes")
        .total(u64::MAX)
        .completed(u64::MAX)
        .active(1);

    assert_eq!(counter.remaining_count(), Some(0));
}
