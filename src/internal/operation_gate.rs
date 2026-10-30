// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Generic atomic operation lifecycle protocol.

use std::marker::PhantomData;

pub use super::atomic_u8_like::AtomicU8Like;
pub use super::atomic_usize_like::AtomicUsizeLike;
pub use super::gate_lifecycle::GateLifecycle;
pub use super::std_scheduler::StdScheduler;
pub use super::yield_like::YieldLike;

/// Coordinates lifecycle transitions with updates that are already in flight.
///
/// The gate admits updates only while open. Finishing rejects new updates and
/// waits until admitted updates leave; closing permanently rejects updates.
/// Its atomic adapters use sequentially consistent operations so the second
/// open-state check, registration counter, finishing transition, and drain
/// observation share one total order. This ensures an admitted update is
/// drained and its writes happen before a successful finish returns: an update
/// either checks open before the finishing transition and remains counted until
/// its sequentially consistent decrement, or checks after the transition and
/// withdraws its registration without being admitted.
///
/// # Type Parameters
///
/// - `A8` stores the lifecycle discriminant with atomic ordering.
/// - `ASize` tracks the number of admitted updates.
/// - `Scheduler` supplies the spin and yield operations used while draining.
#[must_use]
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
    pub fn new() -> Self {
        Self {
            lifecycle: A8::new(GateLifecycle::Open as u8),
            active_updates: ASize::new(0),
            scheduler: PhantomData,
        }
    }

    /// Returns the currently published lifecycle state.
    ///
    /// # Returns
    ///
    /// The lifecycle state most recently published by the gate.
    ///
    /// # Panics
    ///
    /// Panics if an implementation publishes a value outside the three
    /// [`GateLifecycle`] discriminants.
    #[inline]
    pub fn lifecycle(&self) -> GateLifecycle {
        match self.lifecycle.load() {
            value if value == GateLifecycle::Open as u8 => GateLifecycle::Open,
            value if value == GateLifecycle::Finishing as u8 => GateLifecycle::Finishing,
            value if value == GateLifecycle::Closed as u8 => GateLifecycle::Closed,
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
    ///
    /// # Returns
    ///
    /// The number of updates that have entered and not yet left.
    #[inline]
    #[must_use]
    pub fn active_updates(&self) -> usize {
        self.active_updates.load()
    }

    /// Transitions an open gate to finishing and waits for active updates.
    ///
    /// The sequentially consistent order ensures that an update either checks
    /// open before the finishing transition and remains counted until its
    /// decrement, or checks after the transition and withdraws its
    /// registration. The decrement follows the update's writes, and this
    /// method observes the drained counter before returning, publishing
    /// those writes to the finisher.
    ///
    /// # Returns
    ///
    /// `true` if this call changed the gate from open to finishing and all
    /// active updates drained; `false` if the gate was not open.
    #[must_use]
    pub fn try_begin_finish(&self) -> bool {
        if self
            .lifecycle
            .compare_exchange(GateLifecycle::Open as u8, GateLifecycle::Finishing as u8)
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

#[cfg(test)]
mod tests;
