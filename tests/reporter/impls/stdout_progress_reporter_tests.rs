/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `StdoutProgressReporter`.

use std::{
    env,
    process::{
        Command,
        Output,
    },
    time::Duration,
};

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    reporter::{
        ProgressReporter,
        StdoutProgressReporter,
    },
};

const STDOUT_CHILD_ENV: &str = "QUBIT_PROGRESS_STDOUT_REPORTER_CHILD";
const STDOUT_DEFAULT_CHILD_ENV: &str = "QUBIT_PROGRESS_STDOUT_DEFAULT_REPORTER_CHILD";

#[test]
fn test_stdout_progress_reporter_can_report() {
    if env::var_os(STDOUT_CHILD_ENV).is_some() {
        report_running_to_stdout();
        return;
    }

    let output = run_current_test_in_child(
        "reporter::impls::stdout_progress_reporter_tests::test_stdout_progress_reporter_can_report",
        STDOUT_CHILD_ENV,
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("running 1/2 (50.00%)"), "{stdout}");
}

#[test]
fn test_stdout_progress_reporter_default_can_report() {
    if env::var_os(STDOUT_DEFAULT_CHILD_ENV).is_some() {
        report_finished_to_stdout();
        return;
    }

    let output = run_current_test_in_child(
        "reporter::impls::stdout_progress_reporter_tests::test_stdout_progress_reporter_default_can_report",
        STDOUT_DEFAULT_CHILD_ENV,
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("finished 2/2 (100.00%)"), "{stdout}");
}

fn report_running_to_stdout() {
    let reporter = StdoutProgressReporter::new();
    reporter.report(&ProgressEvent::running(
        ProgressCounters::new(Some(2)).with_completed_count(1),
        Duration::from_millis(10),
    ));
}

fn report_finished_to_stdout() {
    let reporter = StdoutProgressReporter::default();
    reporter.report(&ProgressEvent::finished(
        ProgressCounters::new(Some(2)).with_completed_count(2),
        Duration::from_millis(10),
    ));
}

fn run_current_test_in_child(test_name: &str, env_name: &str) -> Output {
    let output = Command::new(env::current_exe().expect("test executable path should be known"))
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(env_name, "1")
        .output()
        .expect("child test process should run");
    assert!(
        output.status.success(),
        "child test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
