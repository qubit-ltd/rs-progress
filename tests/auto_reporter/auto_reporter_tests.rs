// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for scoped automatic progress reporting.

use std::{
    sync::atomic::{AtomicUsize, Ordering},
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
    let tasks = progress
        .metric("tasks")
        .expect("configured metric must exist");
    assert_eq!(
        phases
            .recv_timeout(Duration::from_secs(1))
            .expect("Started event must arrive"),
        Phase::Started,
    );

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        tasks.start(1).expect("work must start");
        tasks.succeed(1).expect("work must succeed");
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
        .finish()
        .expect("terminal event must report after auto reporter stops");
    assert_eq!(
        phases
            .recv_timeout(Duration::from_secs(1))
            .expect("terminal event must arrive"),
        Phase::Succeeded,
    );
}

/// Verifies a positive interval emits heartbeats without notifications.
#[test]
fn test_auto_reporter_reports_positive_interval_heartbeat() {
    let (reporter, phases) = SignalingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::from_millis(5))
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("progress run must start");
    assert_eq!(
        phases
            .recv_timeout(Duration::from_secs(1))
            .expect("Started event must arrive"),
        Phase::Started,
    );

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        let notifier = auto.notifier();
        notifier.notify();
        assert_eq!(
            phases
                .recv_timeout(Duration::from_secs(1))
                .expect("heartbeat event must arrive"),
            Phase::Running,
        );
        assert!(!auto.status().is_failed());
        auto.stop().expect("heartbeat reporter must stop cleanly");
        notifier.notify();
    });
    progress.finish().expect("terminal event must report");
}

/// Reporter that rejects the first background Running report.
struct RunningFailingReporter {
    /// Number of report attempts observed by this reporter.
    reports: AtomicUsize,
}

impl RunningFailingReporter {
    /// Creates a reporter that accepts Started and rejects Running.
    const fn new() -> Self {
        Self {
            reports: AtomicUsize::new(0),
        }
    }
}

impl Reporter for RunningFailingReporter {
    /// Rejects every report after the first lifecycle event.
    fn report(&self, _event: &Event) -> Result<(), ReportError> {
        if self.reports.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReportError::message("running delivery failed"))
        }
    }
}

/// Verifies automatic reporter failures are exposed through status and stop.
#[test]
fn test_auto_reporter_exposes_background_delivery_failure() {
    let reporter = RunningFailingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::ZERO)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        auto.notifier().notify();
        for _ in 0..100 {
            if auto.status().is_failed() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(auto.status().is_failed());
        assert!(auto.stop().is_err());
    });
}
