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
    let mut run = Progress::new(&reporter, Duration::from_secs(5));
    let counters = ProgressCounters::new(Some(4));

    let started = run.report_started(counters);
    let running = run.report_running(counters.with_active_count(2));
    let finished = run.report_finished(counters.with_completed_count(4));

    let events = reporter.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], started);
    assert_eq!(events[1], running);
    assert_eq!(events[2], finished);
    assert_eq!(events[0].phase(), ProgressPhase::Started);
    assert_eq!(events[0].elapsed(), Duration::ZERO);
    assert_eq!(events[1].phase(), ProgressPhase::Running);
    assert_eq!(events[1].counters().active_count(), 2);
    assert_eq!(events[2].phase(), ProgressPhase::Finished);
    assert_eq!(events[2].counters().completed_count(), 4);
    assert!(events[1].elapsed() <= events[2].elapsed());
}

#[test]
fn test_progress_report_running_if_due_respects_interval() {
    let reporter = RecordingReporter::default();
    let mut not_due = Progress::new(&reporter, Duration::from_secs(60));

    assert_eq!(
        not_due.report_running_if_due(ProgressCounters::new(Some(2))),
        None
    );
    assert!(reporter.events().is_empty());

    let mut due = Progress::new(&reporter, Duration::ZERO);

    let reported =
        due.report_running_if_due(ProgressCounters::new(Some(2)).with_completed_count(1));
    assert!(reported.is_some());
    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Running);
    assert_eq!(events[0].counters().completed_count(), 1);
}

#[test]
fn test_progress_report_running_resets_due_deadline() {
    let reporter = RecordingReporter::default();
    let mut progress = Progress::new(&reporter, Duration::from_secs(60));

    let running = progress.report_running(ProgressCounters::new(Some(2)).with_completed_count(1));
    let not_due =
        progress.report_running_if_due(ProgressCounters::new(Some(2)).with_completed_count(2));

    assert_eq!(not_due, None);
    let events = reporter.events();
    assert_eq!(events, vec![running]);
}

#[test]
fn test_progress_attaches_stage_to_reported_events() {
    let reporter = RecordingReporter::default();
    let stage = ProgressStage::new("copy", "Copy files");
    let run = Progress::new(&reporter, Duration::from_secs(5)).with_stage(stage.clone());

    let failed = run.report_failed(ProgressCounters::new(Some(1)).with_failed_count(1));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], failed);
    assert_eq!(events[0].phase(), ProgressPhase::Failed);
    assert_eq!(events[0].stage(), Some(&stage));
}

#[test]
fn test_progress_accessors_and_stage_removal() {
    let reporter = RecordingReporter::default();
    let before_start = Instant::now();
    let stage = ProgressStage::new("load", "Load data");
    let run = Progress::new(&reporter, Duration::from_millis(250))
        .with_stage(stage)
        .without_stage();

    assert!(run.started_at() >= before_start);
    assert_eq!(run.report_interval(), Duration::from_millis(250));
    assert_eq!(run.stage(), None);
    assert!(run.elapsed() >= Duration::ZERO);

    let canceled = run.report_canceled(ProgressCounters::new(Some(9)).with_completed_count(3));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], canceled);
    assert_eq!(events[0].phase(), ProgressPhase::Canceled);
    assert_eq!(events[0].stage(), None);
    assert_eq!(events[0].counters().completed_count(), 3);
}

#[test]
fn test_progress_zero_interval_running_is_always_due() {
    let reporter = RecordingReporter::default();
    let mut run = Progress::new(&reporter, Duration::ZERO);

    assert!(
        run.report_running_if_due(ProgressCounters::new(Some(1)))
            .is_some()
    );
    assert!(
        run.report_running_if_due(ProgressCounters::new(Some(1)))
            .is_some()
    );

    let events = reporter.events();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.phase() == ProgressPhase::Running)
    );
}

#[test]
fn test_progress_is_reexported_from_crate_root() {
    let reporter = RecordingReporter::default();
    let run: qubit_progress::Progress<'_> = Progress::new(&reporter, Duration::from_secs(1));

    assert_eq!(run.report_interval(), Duration::from_secs(1));
}
