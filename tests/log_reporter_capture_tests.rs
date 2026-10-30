// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Captures the records emitted by the log reporter.

#![cfg(feature = "log")]

use std::sync::Mutex;

use log::Level;
use log::LevelFilter;
use log::Log;
use log::Metadata;
use log::Record;
use qubit_progress::LogReporter;
use qubit_progress::Metric;
use qubit_progress::Progress;

/// Stores emitted log records so the integration test can inspect real output.
struct CaptureLogger {
    records: Mutex<Vec<(Level, String)>>,
}

static LOGGER: CaptureLogger = CaptureLogger {
    records: Mutex::new(Vec::new()),
};

impl Log for CaptureLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            self.records
                .lock()
                .expect("capture lock must work")
                .push((record.level(), record.args().to_string()));
        }
    }

    fn flush(&self) {}
}

#[test]
fn test_log_reporter_emits_lifecycle_and_respects_disabled_info() {
    log::set_logger(&LOGGER).expect("isolated test installs logger once");
    log::set_max_level(LevelFilter::Info);

    let mut progress = Progress::builder(&LogReporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("operation must start");
    let tasks = progress.metric("tasks").expect("metric must exist");
    tasks.start(1).expect("work must start");
    tasks.succeed(1).expect("work must succeed");
    progress.report().expect("running event must be logged");
    progress.finish().expect("terminal event must be logged");

    {
        let records = LOGGER.records.lock().expect("capture lock must work");
        assert_eq!(records.len(), 3, "all lifecycle records must be emitted");
        for (index, phase) in ["Started", "Running", "Succeeded"].iter().enumerate() {
            assert_eq!(records[index].0, Level::Info);
            assert!(records[index].1.contains(&format!("phase: {phase}")));
            assert!(records[index].1.contains(&format!("sequence: {index}")));
            assert!(records[index].1.contains("tasks"));
            assert!(records[index].1.contains("Tasks"));
        }
        assert!(records[2].1.contains("succeeded: 1"));
    }

    log::set_max_level(LevelFilter::Warn);
    let progress = Progress::builder(&LogReporter)
        .metric(Metric::new("quiet", "Quiet").total(0))
        .start()
        .expect("disabled operation must start");
    progress.finish().expect("disabled operation must finish");
    assert_eq!(LOGGER.records.lock().expect("capture lock must work").len(), 3);
    log::set_max_level(LevelFilter::Off);
}
