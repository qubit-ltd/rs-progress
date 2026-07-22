// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for running wait outcomes through an inactive progress guard.

use std::{
    thread,
    time::Duration,
};

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressSchema,
};

#[test]
fn test_running_progress_wait_is_bypassed_for_inactive_reporter() {
    let reporter = NoOpProgressReporter;
    let progress = Progress::new(
        &reporter,
        Duration::ZERO,
        ProgressSchema::single("entries", "Entries"),
    );

    thread::scope(|scope| {
        let guard = progress.spawn_running_reporter(scope, Vec::new);
        guard
            .stop_and_join()
            .expect("inactive guard should stop without output");
    });
}
