/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `RunningProgressNotifier`.

use qubit_progress::RunningProgressLoop;

#[test]
fn test_running_progress_notifier_reports_send_failure_after_loop_is_dropped() {
    let (progress_loop, notifier) = RunningProgressLoop::channel();
    drop(progress_loop);

    assert!(!notifier.running_point());
    assert!(!notifier.stop());
}

#[test]
fn test_running_progress_notifier_is_cloneable() {
    let (_progress_loop, notifier) = RunningProgressLoop::channel();
    let cloned = notifier.clone();

    assert!(cloned.running_point());
    assert!(notifier.stop());
}
