/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `JsonStdoutProgressReporter`.

use std::{
    env,
    process::{
        Command,
        Output,
    },
    time::Duration,
};

use qubit_progress::{
    JsonStdoutProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

const JSON_STDOUT_CHILD_ENV: &str = "QUBIT_PROGRESS_JSON_STDOUT_REPORTER_CHILD";

#[test]
fn test_json_stdout_progress_reporter_can_report() {
    if env::var_os(JSON_STDOUT_CHILD_ENV).is_some() {
        report_json_to_stdout();
        return;
    }

    let output = run_current_test_in_child(
        "reporter::impls::json_stdout_progress_reporter_tests::test_json_stdout_progress_reporter_can_report",
        JSON_STDOUT_CHILD_ENV,
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.contains("\"metric\":{\"id\":\"entries\",\"name\":\"Entries\"}"),
        "{stdout}"
    );
}

fn report_json_to_stdout() {
    let reporter = JsonStdoutProgressReporter::default();
    reporter.report(&ProgressEvent::running(
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
