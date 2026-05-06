/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `Progress`.

use std::{
    sync::Mutex,
    time::{
        Duration,
        Instant,
    },
};

use qubit_progress::{
    Progress,
    model::{
        ProgressCounters,
        ProgressEvent,
        ProgressPhase,
        ProgressStage,
    },
    reporter::ProgressReporter,
};

#[derive(Debug, Default)]
struct RecordingReporter {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingReporter {
    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProgressReporter for RecordingReporter {
    fn report(&self, event: &ProgressEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
    }
}

#[test]
fn test_progress_reports_lifecycle_events() {
    let reporter = RecordingReporter::default();
    let started_at = Instant::now() - Duration::from_millis(25);
    let run = Progress::from_start(&reporter, Duration::from_secs(5), started_at);
    let counters = ProgressCounters::new(Some(4));

    run.report_started(counters);
    run.report_running(counters.with_active_count(2));
    run.report_finished(counters.with_completed_count(4));

    let events = reporter.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].phase(), ProgressPhase::Started);
    assert_eq!(events[1].phase(), ProgressPhase::Running);
    assert_eq!(events[1].counters().active_count(), 2);
    assert_eq!(events[2].phase(), ProgressPhase::Finished);
    assert_eq!(events[2].counters().completed_count(), 4);
    assert!(
        events
            .iter()
            .all(|event| event.elapsed() >= Duration::from_millis(25))
    );
}

#[test]
fn test_progress_report_running_if_due_respects_interval() {
    let reporter = RecordingReporter::default();
    let not_due_start = Instant::now();
    let mut not_due = Progress::from_start(&reporter, Duration::from_secs(60), not_due_start);

    assert!(!not_due.report_running_if_due(ProgressCounters::new(Some(2))));
    assert!(reporter.events().is_empty());

    let due_start = Instant::now() - Duration::from_millis(10);
    let mut due = Progress::from_start(&reporter, Duration::from_millis(1), due_start);

    assert!(due.report_running_if_due(ProgressCounters::new(Some(2)).with_completed_count(1)));
    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Running);
    assert_eq!(events[0].counters().completed_count(), 1);
}

#[test]
fn test_progress_attaches_stage_to_reported_events() {
    let reporter = RecordingReporter::default();
    let stage = ProgressStage::new("copy", "Copy files");
    let run = Progress::new(&reporter, Duration::from_secs(5)).with_stage(stage.clone());

    run.report_failed(ProgressCounters::new(Some(1)).with_failed_count(1));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Failed);
    assert_eq!(events[0].stage(), Some(&stage));
}

#[test]
fn test_progress_accessors_and_stage_removal() {
    let reporter = RecordingReporter::default();
    let started_at = Instant::now() - Duration::from_millis(5);
    let stage = ProgressStage::new("load", "Load data");
    let run = Progress::from_start(&reporter, Duration::from_millis(250), started_at)
        .with_stage(stage)
        .without_stage();

    assert_eq!(run.started_at(), started_at);
    assert_eq!(run.report_interval(), Duration::from_millis(250));
    assert_eq!(run.stage(), None);
    assert!(run.elapsed() >= Duration::from_millis(5));

    run.report_canceled(ProgressCounters::new(Some(9)).with_completed_count(3));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Canceled);
    assert_eq!(events[0].stage(), None);
    assert_eq!(events[0].counters().completed_count(), 3);
}

#[test]
fn test_progress_handles_overflowed_next_running_deadline() {
    let reporter = RecordingReporter::default();
    let mut run = Progress::from_start(&reporter, Duration::MAX, Instant::now());

    assert!(run.report_running_if_due(ProgressCounters::new(Some(1))));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Running);
}

#[test]
fn test_progress_is_reexported_from_crate_root() {
    let reporter = RecordingReporter::default();
    let run: qubit_progress::Progress<'_> = Progress::new(&reporter, Duration::from_secs(1));

    assert_eq!(run.report_interval(), Duration::from_secs(1));
}
