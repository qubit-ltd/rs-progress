/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `LoggerProgressReporter`.

use std::time::Duration;

use qubit_progress::{
    model::{
        ProgressCounter,
        ProgressEvent,
        ProgressSchema,
        ProgressStage,
    },
    reporter::{
        LoggerProgressReporter,
        ProgressReporter,
    },
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

fn schema() -> ProgressSchema {
    ProgressSchema::single("entries", "Entries")
}

fn ensure_test_logger() {
    let _ = log::set_logger(&TEST_LOGGER);
    log::set_max_level(log::LevelFilter::Trace);
}

#[test]
fn test_logger_progress_reporter_accessors_and_report_paths() {
    ensure_test_logger();

    let default_reporter = LoggerProgressReporter::default();
    assert_eq!(default_reporter.target(), "qubit_progress");
    assert_eq!(default_reporter.level(), log::Level::Info);

    let reporter = LoggerProgressReporter::new("qubit_progress_test").with_level(log::Level::Warn);
    assert_eq!(reporter.target(), "qubit_progress_test");
    assert_eq!(reporter.level(), log::Level::Warn);

    reporter.report(&ProgressEvent::running(
        schema(),
        vec![ProgressCounter::new("entries").completed(3)],
        Duration::from_secs(1),
    ));
    reporter.report(
        &ProgressEvent::finished(
            schema(),
            vec![ProgressCounter::new("entries").total(3).completed(3)],
            Duration::from_secs(2),
        )
        .with_stage(ProgressStage::new("cleanup", "Cleanup")),
    );
}

#[test]
fn test_logger_progress_reporter_handles_empty_and_unknown_metric_paths() {
    ensure_test_logger();

    let reporter = LoggerProgressReporter::new("qubit_progress_test").with_level(log::Level::Info);
    reporter.report(&ProgressEvent::running(
        schema(),
        Vec::new(),
        Duration::from_millis(1),
    ));
    reporter.report(&ProgressEvent::running(
        schema(),
        vec![ProgressCounter::new("missing").completed(3)],
        Duration::from_millis(2),
    ));
}
