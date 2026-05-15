/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `ProgressCounters`.

use qubit_progress::model::ProgressCounters;

#[test]
fn test_progress_counters_calculate_remaining_and_fraction() {
    let counters = ProgressCounters::new(Some(10))
        .with_completed_count(4)
        .with_active_count(2)
        .with_succeeded_count(3)
        .with_failed_count(1);

    assert_eq!(counters.total_count(), Some(10));
    assert_eq!(counters.completed_count(), 4);
    assert_eq!(counters.active_count(), 2);
    assert_eq!(counters.succeeded_count(), 3);
    assert_eq!(counters.failed_count(), 1);
    assert_eq!(counters.remaining_count(), Some(4));
    assert_eq!(counters.progress_fraction(), Some(0.4));
    assert_eq!(counters.progress_percent(), Some(40.0));
}

#[test]
fn test_progress_counters_handle_unknown_and_zero_total() {
    let unknown = ProgressCounters::new(None).with_completed_count(3);
    assert_eq!(unknown.remaining_count(), None);
    assert_eq!(unknown.progress_fraction(), None);

    let empty = ProgressCounters::new(Some(0));
    assert_eq!(empty.remaining_count(), Some(0));
    assert_eq!(empty.progress_fraction(), Some(1.0));
    assert_eq!(empty.progress_percent(), Some(100.0));
}

#[test]
fn test_progress_counters_remaining_count_saturates_without_overflow() {
    let counters = ProgressCounters::new(Some(usize::MAX))
        .with_completed_count(usize::MAX)
        .with_active_count(1);

    assert_eq!(counters.remaining_count(), Some(0));
}
