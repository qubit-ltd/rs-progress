/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for generic progress events and reporters.

use std::{
    io::Cursor,
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_progress::{
    LoggerProgressReporter,
    NoOpProgressReporter,
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
    ProgressStage,
    WriterProgressReporter,
};

#[test]
fn test_progress_counters_calculate_remaining_and_fraction() {
    let counters = ProgressCounters::new(Some(10))
        .with_completed_count(4)
        .with_active_count(2)
        .with_succeeded_count(3)
        .with_failed_count(1);

    assert_eq!(counters.total_count(), Some(10));
    assert_eq!(counters.completed_count(), 4);
    assert_eq!(counters.active_count(), 2);
    assert_eq!(counters.succeeded_count(), 3);
    assert_eq!(counters.failed_count(), 1);
    assert_eq!(counters.remaining_count(), Some(4));
    assert_eq!(counters.progress_fraction(), Some(0.4));
    assert_eq!(counters.progress_percent(), Some(40.0));
}

#[test]
fn test_progress_counters_handle_unknown_and_zero_total() {
    let unknown = ProgressCounters::new(None).with_completed_count(3);
    assert_eq!(unknown.remaining_count(), None);
    assert_eq!(unknown.progress_fraction(), None);

    let empty = ProgressCounters::new(Some(0));
    assert_eq!(empty.remaining_count(), Some(0));
    assert_eq!(empty.progress_fraction(), Some(1.0));
    assert_eq!(empty.progress_percent(), Some(100.0));
}

#[test]
fn test_progress_phase_formats_all_variants() {
    assert_eq!(ProgressPhase::Started.as_str(), "started");
    assert_eq!(ProgressPhase::Running.as_str(), "running");
    assert_eq!(ProgressPhase::Finished.as_str(), "finished");
    assert_eq!(ProgressPhase::Failed.as_str(), "failed");
    assert_eq!(ProgressPhase::Canceled.as_str(), "canceled");
    assert_eq!(ProgressPhase::Finished.to_string(), "finished");
}

#[test]
fn test_progress_stage_accessors_return_configured_values() {
    let default_stage = ProgressStage::new("prepare", "Prepare");
    assert_eq!(default_stage.id(), "prepare");
    assert_eq!(default_stage.name(), "Prepare");
    assert_eq!(default_stage.index(), None);
    assert_eq!(default_stage.total_stages(), None);
    assert_eq!(default_stage.weight(), None);

    let configured_stage = default_stage
        .clone()
        .with_index(0)
        .with_total_stages(3)
        .with_weight(2.5);
    assert_eq!(configured_stage.id(), "prepare");
    assert_eq!(configured_stage.name(), "Prepare");
    assert_eq!(configured_stage.index(), Some(0));
    assert_eq!(configured_stage.total_stages(), Some(3));
    assert_eq!(configured_stage.weight(), Some(2.5));
}

#[test]
fn test_progress_event_carries_phase_stage_timing_and_context() {
    let stage = ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(4)
        .with_weight(0.5);
    let counters = ProgressCounters::new(Some(8)).with_completed_count(2);
    let event = ProgressEvent::running(counters, Duration::from_secs(3), "copying")
        .with_stage(stage.clone());

    assert_eq!(event.phase(), ProgressPhase::Running);
    assert_eq!(event.stage(), Some(&stage));
    assert_eq!(event.elapsed(), Duration::from_secs(3));
    assert_eq!(event.context(), &"copying");
}

#[test]
fn test_progress_event_constructors_cover_terminal_phases() {
    let counters = ProgressCounters::new(None).with_completed_count(5);
    let started = ProgressEvent::started(counters, Duration::ZERO, "started");
    let finished = ProgressEvent::finished(counters, Duration::from_secs(1), "finished");
    let failed = ProgressEvent::failed(counters, Duration::from_secs(2), "failed");
    let canceled = ProgressEvent::canceled(counters, Duration::from_secs(3), "canceled");

    assert_eq!(started.phase(), ProgressPhase::Started);
    assert_eq!(started.stage(), None);
    assert_eq!(started.counters(), counters);
    assert_eq!(finished.phase(), ProgressPhase::Finished);
    assert_eq!(failed.phase(), ProgressPhase::Failed);
    assert_eq!(canceled.phase(), ProgressPhase::Canceled);
    assert_eq!(canceled.into_context(), "canceled");
}

#[test]
fn test_no_op_progress_reporter_accepts_events() {
    let reporter = NoOpProgressReporter;
    let event = ProgressEvent::started(ProgressCounters::new(Some(1)), Duration::ZERO, ());

    reporter.report(&event);
}

#[test]
fn test_writer_progress_reporter_writes_human_readable_event() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());
    let event = ProgressEvent::running(
        ProgressCounters::new(Some(4))
            .with_active_count(1)
            .with_completed_count(2),
        Duration::from_millis(1500),
        (),
    )
    .with_stage(ProgressStage::new("install", "Install package"));

    reporter.report(&event);

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("running"));
    assert!(text.contains("Install package"));
    assert!(text.contains("2/4"));
    assert!(text.contains("50.00%"));
}

#[test]
fn test_writer_progress_reporter_handles_unknown_total_and_duration_formats() {
    let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let reporter = WriterProgressReporter::new(output.clone());

    assert!(Arc::ptr_eq(reporter.writer(), &output));
    reporter.report(&ProgressEvent::running(
        ProgressCounters::new(None).with_completed_count(7),
        Duration::from_millis(0),
        (),
    ));
    reporter.report(&ProgressEvent::finished(
        ProgressCounters::new(Some(7)).with_completed_count(7),
        Duration::from_secs(61),
        (),
    ));
    reporter.report(&ProgressEvent::failed(
        ProgressCounters::new(Some(7)).with_completed_count(6),
        Duration::from_secs(3_661),
        (),
    ));

    let bytes = output
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_ref()
        .clone();
    let text = String::from_utf8(bytes).expect("writer output should be UTF-8");
    assert!(text.contains("7 completed"));
    assert!(text.contains("0ms"));
    assert!(text.contains("1m 1s"));
    assert!(text.contains("1h 1m 1s"));

    let owned_reporter = WriterProgressReporter::from_writer(Cursor::new(Vec::new()));
    owned_reporter.report(&ProgressEvent::canceled(
        ProgressCounters::new(Some(1)),
        Duration::from_millis(5),
        (),
    ));
}

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
        (),
    ));
    reporter.report(
        &ProgressEvent::finished(
            ProgressCounters::new(Some(3)).with_completed_count(3),
            Duration::from_secs(2),
            (),
        )
        .with_stage(ProgressStage::new("cleanup", "Cleanup")),
    );
}
