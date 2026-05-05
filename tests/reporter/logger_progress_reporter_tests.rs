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
        ProgressCounters,
        ProgressEvent,
        ProgressStage,
    },
    reporter::{
        LoggerProgressReporter,
        ProgressReporter,
    },
};

#[test]
fn test_logger_progress_reporter_accessors_and_report_paths() {
    let default_reporter = LoggerProgressReporter::default();
    assert_eq!(default_reporter.target(), "qubit_progress");
    assert_eq!(default_reporter.level(), log::Level::Info);

    let reporter = LoggerProgressReporter::new("qubit_progress_test").with_level(log::Level::Warn);
    assert_eq!(reporter.target(), "qubit_progress_test");
    assert_eq!(reporter.level(), log::Level::Warn);

    reporter.report(&ProgressEvent::running(
        ProgressCounters::new(None).with_completed_count(3),
        Duration::from_secs(1),
    ));
    reporter.report(
        &ProgressEvent::finished(
            ProgressCounters::new(Some(3)).with_completed_count(3),
            Duration::from_secs(2),
        )
        .with_stage(ProgressStage::new("cleanup", "Cleanup")),
    );
}
