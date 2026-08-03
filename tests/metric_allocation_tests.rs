// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Allocation behavior for hot metric transitions.

use std::{
    alloc::{
        GlobalAlloc,
        Layout,
        System,
    },
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use qubit_progress::{
    Metric,
    NoopReporter,
    Progress,
};

/// Counts allocations made by this integration-test binary.
struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    /// Increments the test counter before delegating to the system allocator.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    /// Delegates deallocation to the system allocator without changing the
    /// count.
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

/// Successful transitions must not allocate an error string on the hot path.
#[test]
fn test_successful_metric_transition_does_not_allocate() {
    let reporter = NoopReporter;
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(2))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("metric must exist");
    tasks.start(1).expect("work must start");

    ALLOCATIONS.store(0, Ordering::Relaxed);
    tasks.complete(1).expect("work must complete");

    assert_eq!(
        ALLOCATIONS.load(Ordering::Relaxed),
        0,
        "successful transitions must not allocate"
    );
}
