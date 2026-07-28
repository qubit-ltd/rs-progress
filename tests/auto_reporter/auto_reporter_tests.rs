// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for scoped automatic progress reporting.

use std::{
    sync::mpsc::{
        Receiver,
        SyncSender,
        sync_channel,
    },
    thread,
    time::Duration,
};

use qubit_progress::{
    Event,
    Metric,
    Phase,
    Progress,
    ReportError,
    Reporter,
};

/// Reporter that exposes emitted phases to the waiting integration test.
struct SignalingReporter {
    /// Sender for event phases.
    sender: SyncSender<Phase>,
}

impl SignalingReporter {
    /// Creates a reporter and the matching phase receiver.
    fn new() -> (Self, Receiver<Phase>) {
        let (sender, receiver) = sync_channel(8);
        (Self { sender }, receiver)
    }
}

impl Reporter for SignalingReporter {
    /// Records one phase through the bounded test channel.
    fn report(&self, event: &Event) -> Result<(), ReportError> {
        self.sender
            .send(event.phase())
            .map_err(|error| ReportError::message(&error.to_string()))
    }
}

/// Verifies that a zero-interval auto reporter emits only after notification.
#[test]
fn test_auto_reporter_reports_zero_interval_after_notification() {
    let (reporter, phases) = SignalingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::ZERO)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress run must start");
    assert_eq!(
        phases
            .recv_timeout(Duration::from_secs(1))
            .expect("Started event must arrive"),
        Phase::Started,
    );

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope, |snapshot| {
            snapshot.metric("tasks", |counts| {
                counts.completed(1).succeeded(1);
            });
        });
        auto.notifier().notify();
        assert_eq!(
            phases
                .recv_timeout(Duration::from_secs(1))
                .expect("notified running event must arrive"),
            Phase::Running,
        );
        auto.stop().expect("auto reporter must stop cleanly");
    });

    progress
        .finish(|snapshot| {
            snapshot.metric("tasks", |counts| {
                counts.completed(1).succeeded(1);
            });
        })
        .expect("terminal event must report after auto reporter stops");
    assert_eq!(
        phases
            .recv_timeout(Duration::from_secs(1))
            .expect("terminal event must arrive"),
        Phase::Succeeded,
    );
}
