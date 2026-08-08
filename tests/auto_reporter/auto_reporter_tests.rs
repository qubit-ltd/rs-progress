// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for scoped automatic progress reporting.

use std::error::Error;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

use qubit_progress::AutoReporterError;
use qubit_progress::EmissionError;
use qubit_progress::Event;
use qubit_progress::Metric;
use qubit_progress::Phase;
use qubit_progress::Progress;
use qubit_progress::Reporter;
use qubit_progress::ReporterError;

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
    fn report(&self, event: &Event) -> Result<(), ReporterError> {
        self.sender
            .send(event.phase())
            .map_err(|error| ReporterError::message(&error.to_string()))
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
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.reports.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(ReporterError::message("running delivery failed"))
        }
    }
}

/// Reporter that panics on its first running event.
struct RunningPanickingReporter {
    /// Number of report attempts observed by this reporter.
    reports: AtomicUsize,
}

impl RunningPanickingReporter {
    /// Creates a reporter that accepts Started and panics on Running.
    const fn new() -> Self {
        Self {
            reports: AtomicUsize::new(0),
        }
    }
}

impl Reporter for RunningPanickingReporter {
    /// Panics after the initial lifecycle event to exercise worker propagation.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.reports.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            panic!("running reporter panicked");
        }
    }
}

/// Reporter that declines event delivery before an operation starts.
struct DisabledReporter;

impl Reporter for DisabledReporter {
    /// Prevents the bound operation from emitting events.
    fn is_enabled(&self) -> bool {
        false
    }

    /// Is never called because the operation remains disabled.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        panic!("disabled progress must not deliver events")
    }
}

/// Verifies a disabled operation creates an inert automatic reporter.
#[test]
fn test_auto_reporter_is_inert_for_disabled_progress() {
    let reporter = DisabledReporter;
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("disabled progress must start");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        auto.notifier().notify();
        auto.stop().expect("inert reporter must stop cleanly");
    });
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
        let notifier = auto.notifier();
        notifier.notify();
        for _ in 0..100 {
            if auto.status().is_failed() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(auto.status().is_failed());
        notifier.notify();
        let error = auto
            .stop()
            .expect_err("background delivery failure must be returned");
        assert!(Error::source(&error).is_some());
        assert!(error.to_string().contains("running delivery failed"));
        assert!(matches!(
            error,
            AutoReporterError::Emission(EmissionError::Delivery(_))
        ));
    });
}

#[test]
fn test_auto_reporter_error_wraps_emission_failures() {
    let error = AutoReporterError::from(EmissionError::SequenceExhausted);
    assert_eq!(error.to_string(), "progress event sequence is exhausted");
    assert!(Error::source(&error).is_some());
}

/// Verifies dropping a failed automatic reporter records its failed status.
#[test]
fn test_auto_reporter_drop_joins_a_failed_worker() {
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
    });
}

/// Verifies that stopping an automatic reporter returns a structured worker
/// panic.
#[test]
fn test_auto_reporter_stop_returns_worker_panic() {
    let reporter = RunningPanickingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::ZERO)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        auto.notifier().notify();
        let error = auto.stop().expect_err("worker panic must be returned");
        let AutoReporterError::Panicked(panic) = error else {
            panic!("expected a structured worker panic");
        };
        assert_eq!(panic.message(), Some("running reporter panicked"));
        assert!(panic.to_string().contains("running reporter panicked"));
        assert!(format!("{panic:?}").contains("WorkerPanic"));
        let payload = panic.into_payload();
        assert_eq!(
            payload.downcast_ref::<&'static str>().copied(),
            Some("running reporter panicked")
        );
    });
}

/// Reporter that panics with a non-string payload.
struct NonStringPanickingReporter {
    /// Number of report attempts observed by this reporter.
    reports: AtomicUsize,
}

impl NonStringPanickingReporter {
    /// Creates a reporter that panics on its first running event.
    const fn new() -> Self {
        Self {
            reports: AtomicUsize::new(0),
        }
    }
}

impl Reporter for NonStringPanickingReporter {
    /// Panics with a payload that cannot be rendered as a standard message.
    fn report(&self, _event: &Event) -> Result<(), ReporterError> {
        if self.reports.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            std::panic::panic_any(7_u8);
        }
    }
}

#[test]
fn test_worker_panic_formats_unknown_payload_and_resumes_unwind() {
    let reporter = NonStringPanickingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::ZERO)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    thread::scope(|scope| {
        let auto = progress.spawn_auto_reporter(scope);
        auto.notifier().notify();
        let error = auto.stop().expect_err("worker panic must be returned");
        assert!(Error::source(&error).is_none());
        assert_eq!(error.to_string(), "background reporter worker panicked");
        let AutoReporterError::Panicked(panic) = error else {
            panic!("expected a structured worker panic");
        };
        assert_eq!(panic.message(), None);
        assert!(format!("{panic:?}").contains("message: None"));
        let result = catch_unwind(AssertUnwindSafe(|| panic.resume_unwind()));
        let payload = result.expect_err("resume_unwind must panic");
        assert_eq!(payload.downcast_ref::<u8>(), Some(&7_u8));
    });
}

/// Verifies that dropping an automatic reporter never propagates a worker
/// panic.
#[test]
fn test_auto_reporter_drop_swallows_worker_panic_after_joining() {
    let reporter = RunningPanickingReporter::new();
    let mut progress = Progress::builder(&reporter)
        .interval(Duration::ZERO)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("Started event must succeed");

    let result = catch_unwind(AssertUnwindSafe(|| {
        thread::scope(|scope| {
            let auto = progress.spawn_auto_reporter(scope);
            let status = auto.status();
            auto.notifier().notify();
            for _ in 0..100 {
                if status.is_failed() {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
            assert!(
                status.is_failed(),
                "worker panic must be observed before Drop"
            );
        });
    }));

    assert!(result.is_ok(), "Drop must not propagate worker panic");
}
