// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Text reporter behavior for lifecycle counters.

use qubit_progress::Metric;
use qubit_progress::Progress;
use qubit_progress::Stage;
use qubit_progress::TextReporter;

/// Verifies that text output includes the cancelled metric count.
#[test]
fn test_text_reporter_includes_cancelled_count() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    let tasks = progress.metric("tasks").expect("configured metric must exist");
    tasks.start(1).expect("work must start");
    tasks.cancel(1).expect("work must cancel");
    progress.finish().expect("progress must finish");

    let output = String::from_utf8(
        reporter
            .into_inner()
            .expect("text reporter writer mutex must not be poisoned"),
    )
    .expect("text output must be UTF-8");
    assert!(
        output.contains("cancelled=1"),
        "text output must include the cancelled metric count"
    );
}

/// Verifies that text records escape control characters and retain stage
/// progress.
#[test]
fn test_text_reporter_escapes_metadata_and_includes_stage_progress() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .stage(Stage::new("copy\nfiles", "Copy\rFiles").position(2, 3))
        .metric(Metric::new("tasks\nall", "Tasks\rAll").total(1))
        .start()
        .expect("progress must start");
    drop(progress);

    let output = String::from_utf8(
        reporter
            .into_inner()
            .expect("text reporter writer mutex must not be poisoned"),
    )
    .expect("text output must be UTF-8");
    assert_eq!(
        output.lines().count(),
        1,
        "escaped control characters must not create additional output lines"
    );
    assert!(
        output.contains("stage=copy\\nfiles(Copy\\rFiles) position=Some(2) total=Some(3)"),
        "text output must escape stage metadata and include stage progress"
    );
    assert!(
        output.contains("metric=tasks\\nall(Tasks\\rAll)"),
        "text output must escape metric metadata"
    );
}

/// Verifies text output includes escaped operation attributes.
#[test]
fn test_text_reporter_includes_operation_attributes() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .attribute("trace\nid", "value\r1")
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress must start");
    drop(progress);
    let output = String::from_utf8(
        reporter
            .into_inner()
            .expect("text reporter writer mutex must not be poisoned"),
    )
    .expect("text output must be UTF-8");
    assert!(
        output.contains("attribute=trace\\nid(value\\r1)"),
        "text output must escape operation attribute names and values"
    );
}

/// Keeps readable Unicode while preventing metadata from adding record lines.
#[test]
fn test_text_reporter_keeps_unicode_and_escapes_controls() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .stage(Stage::new("复制", "复制\n文件").position(1, 1))
        .attribute("租户", "甲\r乙\t丙\\丁")
        .metric(Metric::new("文件", "文件\n数量").total(0))
        .start()
        .expect("Unicode metadata must be accepted");
    progress.finish().expect("zero-total operation must finish");
    let output =
        String::from_utf8(reporter.into_inner().expect("writer lock must work")).expect("text output must be UTF-8");
    assert_eq!(output.lines().count(), 2, "one line per event is required");
    assert!(output.contains("stage=复制(复制\\n文件)"));
    assert!(output.contains("attribute=租户(甲\\r乙\\t丙\\\\丁)"));
    assert!(output.contains("metric=文件(文件\\n数量)"));
    assert!(!output.contains("\\u{6587}"), "ordinary Chinese must stay readable");
    assert!(!output.contains('\r'), "metadata must not emit a raw carriage return");
    assert!(!output.contains('\t'), "metadata must not emit a raw tab");
}
