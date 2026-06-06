// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressReporter` trait behavior.

use std::{
    sync::Arc,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
    time::Duration,
};

use qubit_progress::{
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
    },
    reporter::ProgressReporter,
};

struct CountingReporter {
    called: Arc<AtomicUsize>,
}

impl ProgressReporter for CountingReporter {
    fn report(&self, _event: &ProgressEvent) {
        self.called.fetch_add(1, Ordering::Relaxed);
    }
}

fn schema() -> ProgressSchema {
    ProgressSchema::single("entries", "Entries")
}

#[test]
fn test_progress_reporter_trait_object_dispatch() {
    let called = Arc::new(AtomicUsize::new(0));
    let concrete = CountingReporter {
        called: called.clone(),
    };
    let reporter: &dyn ProgressReporter = &concrete;

    reporter.report(&ProgressEvent::started(
        schema(),
        vec![ProgressCounter::new("entries").total(1)],
        Duration::ZERO,
    ));
    reporter.report(&ProgressEvent::finished(
        schema(),
        vec![ProgressCounter::new("entries").total(1).completed(1)],
        Duration::from_secs(1),
    ));

    assert_eq!(called.load(Ordering::Relaxed), 2);
}

#[test]
fn test_progress_reporter_requires_send_and_sync() {
    fn assert_send_sync<T: ProgressReporter>() {}
    assert_send_sync::<CountingReporter>();
}
