/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Structural test pair for the internal running progress signal module.

use std::{
    thread,
    time::Duration,
};

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounters,
    RunningProgressLoop,
};

#[test]
fn test_running_progress_signal_disconnect_stops_loop() {
    let reporter = NoOpProgressReporter;
    let (progress_loop, notifier) = RunningProgressLoop::channel();
    drop(notifier);

    thread::scope(|scope| {
        let handle = scope.spawn(|| {
            let progress = Progress::new(&reporter, Duration::ZERO);
            progress_loop.run(progress, || ProgressCounters::new(Some(1)));
        });

        handle
            .join()
            .expect("running progress loop should stop when notifiers are dropped");
    });
}
