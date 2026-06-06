// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `FormattedProgressReporter`.

use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};

use qubit_function::{
    ArcConsumer,
    Consumer,
};
use qubit_progress::{
    FormattedProgressReporter,
    HumanReadableMetricSnapshotFormatter,
    MetricSnapshotFormatter,
    ProgressCounter,
    ProgressEvent,
    ProgressMetric,
    ProgressMetricSnapshot,
    ProgressPhase,
    ProgressReporter,
    ProgressSchema,
};

#[test]
fn test_formatted_progress_reporter_formats_each_metric_snapshot() {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let captured_lines = Arc::clone(&lines);
    let consumer = ArcConsumer::new(move |line: &String| {
        captured_lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(line.clone());
    });
    let reporter = FormattedProgressReporter::new(
        HumanReadableMetricSnapshotFormatter::new(),
        consumer,
    );
    let preview = reporter.formatter().format(&ProgressMetricSnapshot::new(
        ProgressMetric::new("entries", "Entries"),
        ProgressPhase::Running,
        None,
        &ProgressCounter::new("entries").total(2).completed(1),
        Duration::ZERO,
    ));
    assert!(preview.contains("Entries 1/2"));
    reporter.consumer().accept(&"manual line".to_owned());
    let event = ProgressEvent::running(
        ProgressSchema::new(vec![
            ProgressMetric::new("entries", "Entries"),
            ProgressMetric::new("bytes", "Bytes"),
        ]),
        vec![
            ProgressCounter::new("entries").total(2).completed(1),
            ProgressCounter::new("bytes").total(10).completed(5),
        ],
        Duration::from_millis(10),
    );

    reporter.report(&event);

    let lines = lines
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "manual line");
    assert!(lines[1].contains("Entries 1/2"));
    assert!(lines[2].contains("Bytes 5/10"));
}
