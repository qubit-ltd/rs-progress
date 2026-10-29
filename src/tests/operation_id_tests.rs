// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::thread;

use crate::error::StartError;
use crate::progress::allocate_operation_id_from;

#[test]
fn test_operation_id_zero_is_exhausted() {
    let counter = AtomicU64::new(0);

    assert!(matches!(
        allocate_operation_id_from(&counter),
        Err(StartError::OperationIdExhausted)
    ));
}

#[test]
fn test_operation_id_allocates_consecutive_values() {
    let counter = AtomicU64::new(1);

    assert_eq!(allocate_operation_id_from(&counter).expect("ID must be available"), 1);
    assert_eq!(allocate_operation_id_from(&counter).expect("ID must be available"), 2);
}

#[test]
fn test_operation_id_allocates_maximum_before_exhaustion() {
    let counter = AtomicU64::new(u64::MAX - 1);

    assert_eq!(
        allocate_operation_id_from(&counter).expect("maximum ID must be available"),
        u64::MAX - 1
    );
    assert_eq!(
        allocate_operation_id_from(&counter).expect("last ID must be available"),
        u64::MAX
    );
    assert!(matches!(
        allocate_operation_id_from(&counter),
        Err(StartError::OperationIdExhausted)
    ));
}

#[test]
fn test_operation_id_allocates_unique_values_across_threads() {
    let counter = Arc::new(AtomicU64::new(1));
    let workers = (0..4)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                (0..32)
                    .map(|_| allocate_operation_id_from(&counter).expect("ID must be available"))
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut ids = workers
        .into_iter()
        .flat_map(|worker| worker.join().expect("worker must join"))
        .collect::<Vec<_>>();

    ids.sort_unstable();
    ids.dedup();

    assert_eq!(ids, (1..=128).collect::<Vec<_>>());
}
