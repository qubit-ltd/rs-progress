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
        ProgressCounter,
        ProgressEvent,
        ProgressPhase,
        ProgressSchema,
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

fn schema() -> ProgressSchema {
    ProgressSchema::single("entries", "Entries")
}

fn run<'a>(reporter: &'a dyn ProgressReporter, interval: Duration) -> Progress<'a> {
    Progress::new(reporter, interval, schema())
}

#[test]
fn test_progress_reports_lifecycle_events() {
    let reporter = RecordingReporter::default();
    let mut run = run(&reporter, Duration::from_secs(5));

    let started = run.report_started(|event| event.counter("entries", |c| c.total(4)));
    let running = run.report_running(|event| event.counter("entries", |c| c.total(4).active(2)));
    let finished =
        run.report_finished(|event| event.counter("entries", |c| c.total(4).completed(4)));

    let events = reporter.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], started);
    assert_eq!(events[1], running);
    assert_eq!(events[2], finished);
    assert_eq!(events[0].phase(), ProgressPhase::Started);
    assert_eq!(events[0].elapsed(), Duration::ZERO);
    assert_eq!(events[1].phase(), ProgressPhase::Running);
    assert_eq!(
        events[1]
            .counter("entries")
            .map(ProgressCounter::active_count),
        Some(2)
    );
    assert_eq!(events[2].phase(), ProgressPhase::Finished);
    assert_eq!(
        events[2]
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(4)
    );
    assert!(events[1].elapsed() <= events[2].elapsed());
}

#[test]
fn test_progress_report_running_if_due_respects_interval() {
    let reporter = RecordingReporter::default();
    let mut not_due = run(&reporter, Duration::from_secs(60));

    assert_eq!(
        not_due.report_running_if_due(|event| event.counter("entries", |c| c.total(2))),
        None
    );
    assert!(reporter.events().is_empty());

    let mut due = run(&reporter, Duration::ZERO);

    let reported = due.report_running_if_due(|event| {
        event.counter("entries", |counter| counter.total(2).completed(1))
    });
    assert!(reported.is_some());
    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase(), ProgressPhase::Running);
    assert_eq!(
        events[0]
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(1)
    );
}

#[test]
fn test_progress_report_running_resets_due_deadline() {
    let reporter = RecordingReporter::default();
    let mut progress = run(&reporter, Duration::from_secs(60));

    let running = progress
        .report_running(|event| event.counter("entries", |counter| counter.total(2).completed(1)));
    let not_due = progress.report_running_if_due(|event| {
        event.counter("entries", |counter| counter.total(2).completed(2))
    });

    assert_eq!(not_due, None);
    let events = reporter.events();
    assert_eq!(events, vec![running]);
}

#[test]
fn test_progress_attaches_stage_to_reported_events() {
    let reporter = RecordingReporter::default();
    let stage = ProgressStage::new("copy", "Copy files");
    let run = run(&reporter, Duration::from_secs(5)).with_stage(stage.clone());

    let failed = run.report_failed(|event| event.counter("entries", |c| c.total(1).failed(1)));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], failed);
    assert_eq!(events[0].phase(), ProgressPhase::Failed);
    assert_eq!(events[0].stage(), Some(&stage));
}

#[test]
fn test_progress_accessors_stage_removal_and_event_builder() {
    let reporter = RecordingReporter::default();
    let before_start = Instant::now();
    let stage = ProgressStage::new("load", "Load data");
    let run = Progress::new(&reporter, Duration::from_millis(250), schema())
        .with_stage(stage)
        .without_stage();

    assert!(run.started_at() >= before_start);
    assert_eq!(run.report_interval(), Duration::from_millis(250));
    assert_eq!(run.stage(), None);
    assert_eq!(run.schema().metric_name("entries"), Some("Entries"));
    assert!(run.elapsed() >= Duration::ZERO);

    let preview = run
        .event_builder()
        .counter("entries", |counter| counter.total(9).completed(3))
        .build();
    assert_eq!(
        preview
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(3)
    );

    let canceled =
        run.report_canceled(|event| event.counter("entries", |c| c.total(9).completed(3)));

    let events = reporter.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], canceled);
    assert_eq!(events[0].phase(), ProgressPhase::Canceled);
    assert_eq!(events[0].stage(), None);
    assert_eq!(
        events[0]
            .counter("entries")
            .map(ProgressCounter::completed_count),
        Some(3)
    );
}

#[test]
fn test_progress_zero_interval_running_is_always_due() {
    let reporter = RecordingReporter::default();
    let mut run = run(&reporter, Duration::ZERO);

    assert!(
        run.report_running_if_due(|event| event.counter("entries", |c| c.total(1)))
            .is_some()
    );
    assert!(
        run.report_running_if_due(|event| event.counter("entries", |c| c.total(1)))
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
    let run: qubit_progress::Progress<'_> =
        Progress::single_metric(&reporter, Duration::from_secs(1), "entries", "Entries");

    assert_eq!(run.report_interval(), Duration::from_secs(1));
}
