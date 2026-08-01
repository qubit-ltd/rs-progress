// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Scoped background reporting for one exclusively borrowed progress operation.
// qubit-style: allow multiple-public-types

use std::{
    marker::PhantomData,
    panic::{
        AssertUnwindSafe,
        catch_unwind,
        resume_unwind,
    },
    sync::{
        Arc,
        Weak,
        atomic::{
            AtomicBool,
            Ordering,
        },
        mpsc::{
            Receiver,
            SyncSender,
            sync_channel,
        },
    },
    thread::{
        self,
        ScopedJoinHandle,
    },
};

use crate::{
    Progress,
    ProgressError,
};

/// Handle controlling one scoped automatic reporter.
#[must_use]
pub struct AutoReporter<'scope, 'reporter> {
    /// Scoped worker result, present only for enabled operations.
    join: Option<ScopedJoinHandle<'scope, Result<(), ProgressError>>>,
    /// Shared wake and stop controls.
    inner: Option<Arc<AutoReporterInner>>,
    /// State observable by workers.
    status: Status,
    /// Ties the original mutable Progress borrow to this handle's lifetime.
    progress_borrow: PhantomData<&'scope mut Progress<'reporter>>,
}

impl<'scope, 'reporter> AutoReporter<'scope, 'reporter> {
    /// Returns a cloneable worker notification handle.
    #[must_use]
    pub fn notifier(&self) -> Notifier {
        Notifier {
            inner: self.inner.as_ref().and_then(|inner| {
                inner.notification_driven.then(|| Arc::downgrade(inner))
            }),
        }
    }

    /// Returns a cloneable status view for workers.
    #[must_use]
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    /// Stops, joins and returns the background report result.
    ///
    /// A reporter or snapshot error is returned. A worker panic is resumed on
    /// the calling thread after joining.
    pub fn stop(mut self) -> Result<(), ProgressError> {
        self.signal_stop();
        match self.join_worker() {
            Ok(result) => result,
            Err(payload) => resume_unwind(payload),
        }
    }

    /// Marks stop and wakes the reporter if it is blocked.
    fn signal_stop(&self) {
        if let Some(inner) = &self.inner {
            inner.stopped.store(true, Ordering::Release);
            wake(&inner.wake_sender);
        }
    }

    /// Joins the scoped worker once and returns either its result or panic.
    fn join_worker(
        &mut self,
    ) -> Result<
        Result<(), ProgressError>,
        Box<dyn std::any::Any + Send + 'static>,
    > {
        let Some(join) = self.join.take() else {
            return Ok(Ok(()));
        };
        join.join()
    }
}

impl Drop for AutoReporter<'_, '_> {
    /// Stops and joins a forgotten reporter without silently leaving a thread.
    fn drop(&mut self) {
        self.signal_stop();
        match self.join_worker() {
            Ok(Ok(())) => {}
            Ok(Err(_)) => self.status.mark_failed(),
            Err(payload) if !thread::panicking() => resume_unwind(payload),
            Err(_) => {}
        }
    }
}

/// Notification handle that coalesces state changes without claiming delivery.
#[derive(Clone)]
pub struct Notifier {
    /// Non-owning link present only for notification-driven reporters.
    inner: Option<Weak<AutoReporterInner>>,
}

impl Notifier {
    /// Records that shared work state changed and wakes a zero-interval loop.
    ///
    /// The method is a no-op for disabled and heartbeat-driven reporters, after
    /// the reporter stops, and when no worker remains. Multiple calls merge
    /// into at most one pending report.
    pub fn notify(&self) {
        let Some(inner) = self.inner.as_ref().and_then(Weak::upgrade) else {
            return;
        };
        if inner.stopped.load(Ordering::Acquire) {
            return;
        }
        inner.pending.store(true, Ordering::Release);
        wake(&inner.wake_sender);
    }
}

/// Cloneable status exposed while an automatic reporter is active.
#[derive(Clone)]
pub struct Status {
    /// Shared failure flag.
    failed: Arc<AtomicBool>,
}

impl Status {
    /// Creates a status flag initially representing a healthy reporter.
    fn healthy() -> Self {
        Self {
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Records that the reporter has failed.
    fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }

    /// Returns whether the automatic reporter terminated with an error or
    /// panic.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

/// Shared control state held by the handle and weakly by worker notifiers.
struct AutoReporterInner {
    /// Whether state-change notifications drive running reports.
    notification_driven: bool,
    /// Bounded wake channel sender.
    wake_sender: SyncSender<()>,
    /// Stop request flag.
    stopped: AtomicBool,
    /// Coalesced notification flag.
    pending: AtomicBool,
}

/// Spawns the worker for one enabled progress operation.
pub(crate) fn spawn<'scope, 'env, 'reporter>(
    progress: &'scope mut Progress<'reporter>,
    scope: &'scope thread::Scope<'scope, 'env>,
) -> AutoReporter<'scope, 'reporter>
where
    'reporter: 'scope,
{
    let status = Status::healthy();
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
) -> Result<(), ProgressError> {
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
) -> Result<(), ProgressError> {
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
) -> Result<(), ProgressError> {
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
fn wake(sender: &SyncSender<()>) {
    let _ = sender.try_send(());
}
