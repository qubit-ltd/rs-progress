// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `Progress`.

use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use qubit_progress::{
    NoOpProgressReporter, Progress,
    model::{
        ProgressCounter, ProgressEvent, ProgressEventBuildError, ProgressPhase, ProgressSchema,
        ProgressStage,
    },
    reporter::{ProgressReportError, ProgressReporter, WriterProgressReporter},
};

use crate::support::FailingWriter;

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
    fn report(&self, event: &ProgressEvent) -> Result<(), qubit_progress::ProgressReportError> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
        Ok(())
    }
}

#[derive(Debug, Default)]
struct FlippingEnabledReporter {
    enabled_checks: AtomicUsize,
    events: Mutex<Vec<ProgressEvent>>,
}

impl FlippingEnabledReporter {
    fn enabled_check_count(&self) -> usize {
        self.enabled_checks.load(Ordering::SeqCst)
    }

    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProgressReporter for FlippingEnabledReporter {
    fn is_enabled(&self) -> bool {
        self.enabled_checks.fetch_add(1, Ordering::SeqCst) == 0
    }

    fn report(&self, event: &ProgressEvent) -> Result<(), qubit_progress::ProgressReportError> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
        Ok(())
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

    let started = run
        .report_started(|event| event.counter("entries", |c| c.total(4)))
        .expect("recording reporter should accept started event");
    let running = run
        .report_running(|event| event.counter("entries", |c| c.total(4).active(2)))
        .expect("recording reporter should accept running event");
    let finished = run
        .report_finished(|event| event.counter("entries", |c| c.total(4).completed(4)))
        .expect("recording reporter should accept finished event");

    let events = reporter.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], started);
    assert_eq!(events[1], running);
    assert_eq!(events[2], finished);
    assert_eq!(events[0].phase(), ProgressPhase::Started);
    assert_eq!(events[0].operation_id(), run.operation_id());
    assert_eq!(events[1].operation_id(), run.operation_id());
    assert_eq!(events[2].operation_id(), run.operation_id());
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
fn test_progress_propagates_reporter_errors() {
    let reporter = WriterProgressReporter::from_writer(FailingWriter);
    let progress = Progress::new(&reporter, Duration::ZERO, schema());

    let result =
        progress.report_started(|event| event.counter("entries", |counter| counter.total(1)));

    assert!(matches!(result, Err(ProgressReportError::Io(_))));
}

#[test]
fn test_progress_report_running_if_due_respects_interval() {
    let reporter = RecordingReporter::default();
    let mut not_due = run(&reporter, Duration::from_secs(60));

    assert_eq!(
        not_due
            .report_running_if_due(|event| { event.counter("entries", |c| c.total(2)) })
            .expect("recording reporter should accept due checks"),
        None
    );
    assert!(reporter.events().is_empty());

    let mut due = run(&reporter, Duration::ZERO);

    let reported = due
        .report_running_if_due(|event| {
            event.counter("entries", |counter| counter.total(2).completed(1))
        })
        .expect("recording reporter should accept running event");
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
fn test_progress_report_running_if_due_skips_disabled_reporter_configuration() {
    let reporter = NoOpProgressReporter;
    let mut progress = run(&reporter, Duration::ZERO);
    let configured = std::cell::Cell::new(false);

    let event = progress
        .report_running_if_due(|event| {
            configured.set(true);
            event.counter("entries", |counter| counter.total(1))
        })
        .expect("disabled reporter should not fail");

    assert_eq!(event, None);
    assert!(!configured.get());
}

#[test]
fn test_progress_report_running_if_due_reports_after_enabled_decision() {
    let reporter = FlippingEnabledReporter::default();
    let mut progress = run(&reporter, Duration::ZERO);

    let event = progress
        .report_running_if_due(|event| event.counter("entries", |counter| counter.total(1)))
        .expect("enabled reporter should accept running event")
        .expect("first enabled decision should report an event");

    assert_eq!(
        event
            .counter("entries")
            .and_then(ProgressCounter::total_count),
        Some(1),
    );
    assert_eq!(reporter.events(), vec![event]);
    assert_eq!(reporter.enabled_check_count(), 1);
}

#[test]
fn test_progress_reports_event_build_errors_without_panicking() {
    let reporter = RecordingReporter::default();
    let progress = run(&reporter, Duration::ZERO);

    let result =
        progress.report_started(|event| event.counter("missing", |counter| counter.total(1)));

    assert_eq!(
        result,
        Err(ProgressReportError::EventBuild(
            ProgressEventBuildError::UnknownMetricId {
                metric_id: "missing".to_owned(),
            },
        )),
    );
    assert!(reporter.events().is_empty());
}

#[test]
fn test_progress_report_started_if_enabled_skips_disabled_configuration() {
    let disabled_reporter = NoOpProgressReporter;
    let disabled_progress = run(&disabled_reporter, Duration::ZERO);
    let configured = std::cell::Cell::new(false);

    let skipped = disabled_progress
        .report_started_if_enabled(|event| {
            configured.set(true);
            event.counter("entries", |counter| counter.total(1))
        })
        .expect("disabled reporter should not fail");

    assert_eq!(skipped, None);
    assert!(!configured.get());

    let enabled_reporter = RecordingReporter::default();
    let enabled_progress = run(&enabled_reporter, Duration::ZERO);
    let reported = enabled_progress
        .report_started_if_enabled(|event| event.counter("entries", |counter| counter.total(1)))
        .expect("enabled reporter should accept the event")
        .expect("enabled reporter should receive the event");

    assert_eq!(reported.phase(), ProgressPhase::Started);
    assert_eq!(enabled_reporter.events(), vec![reported]);
}

#[test]
fn test_progress_other_if_enabled_lifecycle_methods_report_events() {
    let reporter = RecordingReporter::default();
    let mut progress = run(&reporter, Duration::ZERO);

    let running = progress
        .report_running_if_enabled(|event| {
            event.counter("entries", |counter| counter.total(4).active(1))
        })
        .expect("recording reporter should accept running event")
        .expect("enabled reporter should receive running event");
    let finished = progress
        .report_finished_if_enabled(|event| {
            event.counter("entries", |counter| counter.total(4).completed(4))
        })
        .expect("recording reporter should accept finished event")
        .expect("enabled reporter should receive finished event");
    let failed = progress
        .report_failed_if_enabled(|event| {
            event.counter("entries", |counter| counter.total(4).failed(1))
        })
        .expect("recording reporter should accept failed event")
        .expect("enabled reporter should receive failed event");
    let canceled = progress
        .report_canceled_if_enabled(|event| {
            event.counter("entries", |counter| counter.total(4).completed(2))
        })
        .expect("recording reporter should accept canceled event")
        .expect("enabled reporter should receive canceled event");

    assert_eq!(running.phase(), ProgressPhase::Running);
    assert_eq!(finished.phase(), ProgressPhase::Finished);
    assert_eq!(failed.phase(), ProgressPhase::Failed);
    assert_eq!(canceled.phase(), ProgressPhase::Canceled);
    assert_eq!(reporter.events(), vec![running, finished, failed, canceled]);

    let disabled_reporter = NoOpProgressReporter;
    let mut disabled_progress = run(&disabled_reporter, Duration::ZERO);
    let configured = std::cell::Cell::new(false);
    assert_eq!(
        disabled_progress
            .report_running_if_enabled(|event| {
                configured.set(true);
                event.counter("entries", |counter| counter.total(1))
            })
            .expect("disabled reporter should not fail"),
        None,
    );
    assert!(!configured.get());
}

#[test]
fn test_progress_lifecycle_reports_skip_disabled_configuration() {
    let reporter = NoOpProgressReporter;
    let progress = run(&reporter, Duration::ZERO);
    let configured = std::cell::Cell::new(false);

    let event = progress
        .report_finished(|event| {
            configured.set(true);
            event.counter("entries", |counter| counter.total(1).completed(1))
        })
        .expect("disabled reporter should not fail");

    assert!(!configured.get());
    assert!(event.counters().is_empty());
}

#[test]
fn test_progress_report_running_propagates_reporter_errors() {
    let reporter = WriterProgressReporter::from_writer(FailingWriter);
    let mut progress = Progress::new(&reporter, Duration::ZERO, schema());

    let result =
        progress.report_running(|event| event.counter("entries", |counter| counter.total(1)));

    assert!(matches!(result, Err(ProgressReportError::Io(_))));

    let result = progress
        .report_running_if_enabled(|event| event.counter("entries", |counter| counter.total(1)));

    assert!(matches!(result, Err(ProgressReportError::Io(_))));

    let reporter = RecordingReporter::default();
    let mut progress = run(&reporter, Duration::ZERO);
    let result = progress
        .report_running_if_enabled(|event| event.counter("missing", |counter| counter.total(1)));

    assert!(matches!(result, Err(ProgressReportError::EventBuild(_))));
}

#[test]
fn test_progress_report_running_resets_due_deadline() {
    let reporter = RecordingReporter::default();
    let mut progress = run(&reporter, Duration::from_secs(60));

    let running = progress
        .report_running(|event| event.counter("entries", |counter| counter.total(2).completed(1)))
        .expect("recording reporter should accept running event");
    let not_due = progress
        .report_running_if_due(|event| {
            event.counter("entries", |counter| counter.total(2).completed(2))
        })
        .expect("recording reporter should accept due checks");

    assert_eq!(not_due, None);
    let events = reporter.events();
    assert_eq!(events, vec![running]);
}

#[test]
fn test_progress_attaches_stage_to_reported_events() {
    let reporter = RecordingReporter::default();
    let stage = ProgressStage::new("copy", "Copy files");
    let run = run(&reporter, Duration::from_secs(5)).with_stage(stage.clone());

    let failed = run
        .report_failed(|event| event.counter("entries", |c| c.total(1).failed(1)))
        .expect("recording reporter should accept failed event");

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
    assert!(run.is_enabled());
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

    let canceled = run
        .report_canceled(|event| event.counter("entries", |c| c.total(9).completed(3)))
        .expect("recording reporter should accept canceled event");

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
        run.report_running_if_due(|event| { event.counter("entries", |c| c.total(1)) })
            .expect("recording reporter should accept running event")
            .is_some()
    );
    assert!(
        run.report_running_if_due(|event| { event.counter("entries", |c| c.total(1)) })
            .expect("recording reporter should accept running event")
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

#[test]
fn test_progress_overflowing_interval_is_not_due() {
    let reporter = RecordingReporter::default();
    let mut progress = run(&reporter, Duration::MAX);

    assert_eq!(
        progress
            .report_running_if_due(|event| { event.counter("entries", |counter| counter.total(1)) })
            .expect("recording reporter should accept due checks"),
        None,
    );
    assert!(reporter.events().is_empty());
}
