// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Generic atomic operation lifecycle protocol.

use std::{
    marker::PhantomData,
    sync::atomic::{
        AtomicU8,
        AtomicUsize,
        Ordering,
    },
};

/// Atomic byte operations required by [`OperationGate`].
pub trait AtomicU8Like {
    /// Creates an atomic value with the supplied initial state.
    fn new(value: u8) -> Self;

    /// Loads the value with acquire ordering.
    fn load(&self) -> u8;

    /// Performs an acquire-release compare-and-exchange.
    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8>;

    /// Stores the value with release ordering.
    fn store(&self, value: u8);
}

/// Atomic counter operations required by [`OperationGate`].
pub trait AtomicUsizeLike {
    /// Creates an atomic counter with the supplied initial value.
    fn new(value: usize) -> Self;

    /// Loads the counter with acquire ordering.
    fn load(&self) -> usize;

    /// Adds to the counter with acquire-release ordering.
    fn fetch_add(&self, value: usize);

    /// Subtracts from the counter with release ordering.
    fn fetch_sub(&self, value: usize);
}

/// Scheduling hooks used while waiting for registered updates to leave.
pub trait YieldLike {
    /// Gives the processor a hint that the current thread is spinning.
    fn spin_loop();

    /// Yields the current thread to make progress in a model scheduler.
    fn yield_now();
}

/// Lifecycle states published by an [`OperationGate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateLifecycle {
    /// New updates may be admitted.
    Open = 0,
    /// New updates are rejected while admitted updates drain.
    Finishing = 1,
    /// The operation is permanently closed.
    Closed = 2,
}

/// Atomic lifecycle gate shared by operation and metric handles.
pub struct OperationGate<A8, ASize, Scheduler> {
    /// Published lifecycle state.
    lifecycle: A8,
    /// Number of updates that passed the first open check.
    active_updates: ASize,
    /// Scheduling implementation used by the drain loop.
    scheduler: PhantomData<Scheduler>,
}

impl<A8, ASize, Scheduler> OperationGate<A8, ASize, Scheduler>
where
    A8: AtomicU8Like,
    ASize: AtomicUsizeLike,
    Scheduler: YieldLike,
{
    /// Creates an open gate with no registered updates.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            lifecycle: A8::new(GateLifecycle::Open as u8),
            active_updates: ASize::new(0),
            scheduler: PhantomData,
        }
    }

    /// Returns the currently published lifecycle state.
    ///
    /// # Panics
    ///
    /// Panics if an implementation publishes a value outside the three
    /// [`GateLifecycle`] discriminants.
    #[inline]
    #[must_use]
    pub fn lifecycle(&self) -> GateLifecycle {
        match self.lifecycle.load() {
            value if value == GateLifecycle::Open as u8 => GateLifecycle::Open,
            value if value == GateLifecycle::Finishing as u8 => {
                GateLifecycle::Finishing
            }
            value if value == GateLifecycle::Closed as u8 => {
                GateLifecycle::Closed
            }
            _ => unreachable!("operation lifecycle must contain a known value"),
        }
    }

    /// Registers one update if the operation remains open across both checks.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the update is registered, or the lifecycle state observed
    /// by the rejecting check.
    #[inline]
    pub fn enter_update(&self) -> Result<(), GateLifecycle> {
        let state = self.lifecycle();
        if state != GateLifecycle::Open {
            return Err(state);
        }
        self.active_updates.fetch_add(1);
        let state = self.lifecycle();
        if state == GateLifecycle::Open {
            Ok(())
        } else {
            self.active_updates.fetch_sub(1);
            Err(state)
        }
    }

    /// Releases one previously registered update.
    #[inline]
    pub fn leave_update(&self) {
        self.active_updates.fetch_sub(1);
    }

    /// Returns the number of updates currently registered with the gate.
    #[inline]
    #[must_use]
    pub fn active_updates(&self) -> usize {
        self.active_updates.load()
    }

    /// Transitions an open gate to finishing and waits for active updates.
    #[must_use]
    pub fn try_begin_finish(&self) -> bool {
        if self
            .lifecycle
            .compare_exchange(
                GateLifecycle::Open as u8,
                GateLifecycle::Finishing as u8,
            )
            .is_err()
        {
            return false;
        }
        let mut attempts = 0u32;
        while self.active_updates() != 0 {
            if attempts > 0 && attempts.is_multiple_of(16) {
                Scheduler::yield_now();
            } else {
                Scheduler::spin_loop();
            }
            attempts += 1;
        }
        true
    }

    /// Reopens a finishing gate after validation fails.
    #[inline]
    pub fn reopen(&self) {
        self.lifecycle.store(GateLifecycle::Open as u8);
    }

    /// Permanently closes the gate.
    #[inline]
    pub fn close(&self) {
        self.lifecycle.store(GateLifecycle::Closed as u8);
    }
}

impl<A8, ASize, Scheduler> Default for OperationGate<A8, ASize, Scheduler>
where
    A8: AtomicU8Like,
    ASize: AtomicUsizeLike,
    Scheduler: YieldLike,
{
    /// Creates an open gate with no registered updates.
    fn default() -> Self {
        Self::new()
    }
}

/// Standard-library scheduling implementation.
#[allow(dead_code)]
pub struct StdScheduler;

impl YieldLike for StdScheduler {
    /// Uses the standard processor spin hint.
    fn spin_loop() {
        std::hint::spin_loop();
    }

    /// Yields to another runnable operating-system thread.
    fn yield_now() {
        std::thread::yield_now();
    }
}

impl AtomicU8Like for AtomicU8 {
    /// Creates a standard atomic byte.
    #[inline]
    fn new(value: u8) -> Self {
        Self::new(value)
    }

    /// Loads the byte with acquire ordering.
    #[inline]
    fn load(&self) -> u8 {
        self.load(Ordering::Acquire)
    }

    /// Compares and exchanges with acquire-release ordering.
    #[inline]
    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8> {
        self.compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
    }

    /// Stores the byte with release ordering.
    #[inline]
    fn store(&self, value: u8) {
        self.store(value, Ordering::Release);
    }
}

impl AtomicUsizeLike for AtomicUsize {
    /// Creates a standard atomic counter.
    #[inline]
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    /// Loads the counter with acquire ordering.
    #[inline]
    fn load(&self) -> usize {
        self.load(Ordering::Acquire)
    }

    /// Adds with acquire-release ordering.
    #[inline]
    fn fetch_add(&self, value: usize) {
        self.fetch_add(value, Ordering::AcqRel);
    }

    /// Subtracts with release ordering.
    #[inline]
    fn fetch_sub(&self, value: usize) {
        self.fetch_sub(value, Ordering::Release);
    }
}
