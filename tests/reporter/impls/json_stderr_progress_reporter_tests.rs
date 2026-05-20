/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `JsonStderrProgressReporter`.

use std::{
    env,
    process::{
        Command,
        Output,
    },
    time::Duration,
};

use qubit_progress::{
    JsonStderrProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

const JSON_STDERR_CHILD_ENV: &str = "QUBIT_PROGRESS_JSON_STDERR_REPORTER_CHILD";

#[test]
fn test_json_stderr_progress_reporter_default_can_report() {
    if env::var_os(JSON_STDERR_CHILD_ENV).is_some() {
        report_json_to_stderr();
        return;
    }

    let output = run_current_test_in_child(
        "reporter::impls::json_stderr_progress_reporter_tests::test_json_stderr_progress_reporter_default_can_report",
        JSON_STDERR_CHILD_ENV,
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("\"phase\":\"failed\""), "{stderr}");
}

fn report_json_to_stderr() {
    let reporter = JsonStderrProgressReporter::default();
    reporter.report(&ProgressEvent::failed(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").total(2).completed(1)],
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
