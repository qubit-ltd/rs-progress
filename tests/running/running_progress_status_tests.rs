// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `RunningProgressStatus`.

use std::{
    thread,
    time::{
        Duration,
        Instant,
    },
};

use qubit_progress::{
    Progress,
    ProgressCounter,
    ProgressReportError,
    ProgressSchema,
    WriterProgressReporter,
};

use crate::support::FailingWriter;

#[test]
fn test_running_progress_status_reports_output_failure() {
    let reporter = WriterProgressReporter::from_writer(FailingWriter);

    thread::scope(|scope| {
        let progress = Progress::new(
            &reporter,
            Duration::ZERO,
            ProgressSchema::single("entries", "Entries"),
        );
        let running_progress = progress.spawn_running_reporter(scope, || {
            vec![ProgressCounter::new("entries").total(1)]
        });
        let status = running_progress.status();
        let point = running_progress.point_handle();

        assert!(point.try_report());
        let deadline = Instant::now() + Duration::from_secs(1);
        while !status.is_failed() && Instant::now() < deadline {
            thread::yield_now();
        }

        assert!(status.is_failed());
        assert!(matches!(
            running_progress.stop_and_join(),
            Err(ProgressReportError::Io(_)),
        ));
    });
}
