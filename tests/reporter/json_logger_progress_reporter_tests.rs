/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `JsonLoggerProgressReporter`.

use std::time::Duration;

use qubit_progress::{
    JsonLoggerProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

struct TestLogger;

impl log::Log for TestLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, _record: &log::Record<'_>) {}

    fn flush(&self) {}
}

static TEST_LOGGER: TestLogger = TestLogger;

fn ensure_test_logger() {
    let _ = log::set_logger(&TEST_LOGGER);
    log::set_max_level(log::LevelFilter::Trace);
}

#[test]
fn test_json_logger_progress_reporter_accessors_and_report_paths() {
    ensure_test_logger();

    let default_reporter = JsonLoggerProgressReporter::default();
    assert_eq!(default_reporter.target(), "qubit_progress");
    assert_eq!(default_reporter.level(), log::Level::Info);

    let reporter = JsonLoggerProgressReporter::new("qubit_progress_json_test").with_level(log::Level::Warn);
    assert_eq!(reporter.target(), "qubit_progress_json_test");
    assert_eq!(reporter.level(), log::Level::Warn);

    reporter.report(&ProgressEvent::running(
        ProgressSchema::single("entries", "Entries"),
        vec![ProgressCounter::new("entries").completed(3)],
        Duration::from_secs(1),
    ));
}
