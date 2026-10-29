// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Worker lifecycle and reporting loops for automatic progress reporting.

use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::panic::resume_unwind;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::sync_channel;
use std::thread;

use super::AutoReporterInner;
use crate::AutoReporter;
use crate::AutoReporterStatus;
use crate::EmissionError;
use crate::Progress;

/// Spawns the worker for one enabled progress operation.
pub(crate) fn spawn<'scope, 'env, 'reporter>(
    progress: &'scope mut Progress<'reporter>,
    scope: &'scope thread::Scope<'scope, 'env>,
) -> AutoReporter<'scope, 'reporter>
where
    'reporter: 'scope,
{
    let status = AutoReporterStatus::healthy();
    if !progress.is_enabled() {
        return AutoReporter {
            join: None,
            inner: None,
            status,
            progress_borrow: PhantomData,
        };
    }
    let (wake_sender, wake_receiver) = sync_channel(1);
    let inner = Arc::new(AutoReporterInner {
        notification_driven: progress.report_interval().is_zero(),
        wake_sender,
        stopped: AtomicBool::new(false),
        pending: AtomicBool::new(false),
    });
    let worker_inner = Arc::clone(&inner);
    let worker_status = status.clone();
    let join = scope.spawn(move || {
        match catch_unwind(AssertUnwindSafe(|| {
            run(progress, Arc::clone(&worker_inner), wake_receiver)
        })) {
            Ok(result) => {
                if result.is_err() {
                    worker_status.mark_failed();
                    worker_inner.stopped.store(true, Ordering::Release);
                }
                result
            }
            Err(payload) => {
                worker_status.mark_failed();
                worker_inner.stopped.store(true, Ordering::Release);
                resume_unwind(payload)
            }
        }
    });
    AutoReporter {
        join: Some(join),
        inner: Some(inner),
        status,
        progress_borrow: PhantomData,
    }
}

/// Runs one background reporting loop until stopped or a report fails.
fn run(
    progress: &mut Progress<'_>,
    inner: Arc<AutoReporterInner>,
    receiver: Receiver<()>,
) -> Result<(), EmissionError> {
    if progress.report_interval().is_zero() {
        run_notified(progress, &inner, receiver)
    } else {
        run_heartbeat(progress, &inner, receiver)
    }
}

/// Runs notification-driven reporting for a zero interval.
fn run_notified(
    progress: &mut Progress<'_>,
    inner: &AutoReporterInner,
    receiver: Receiver<()>,
) -> Result<(), EmissionError> {
    loop {
        receiver
            .recv()
            .expect("notification sender must outlive the reporter worker");
        if inner.pending.swap(false, Ordering::AcqRel) {
            progress.report()?;
        }
        if inner.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
    }
}

/// Runs deadline-based heartbeat reporting for a positive interval.
fn run_heartbeat(
    progress: &mut Progress<'_>,
    inner: &AutoReporterInner,
    receiver: Receiver<()>,
) -> Result<(), EmissionError> {
    loop {
        if inner.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        let timeout = progress.time_until_due();
        if receiver.recv_timeout(timeout).is_ok() {
            return Ok(());
        }
        progress.report_if_due()?;
    }
}

/// Sends one coalesced wake signal without blocking a worker.
pub(in crate::auto_reporter) fn wake(sender: &SyncSender<()>) {
    let _ = sender.try_send(());
}
