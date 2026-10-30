// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Loom models for the production operation freeze protocol.
// qubit-style: allow coverage-cfg

use std::sync::Arc as StdArc;
use std::sync::Barrier;
use std::sync::atomic::AtomicU8 as StdAtomicU8;
use std::sync::atomic::AtomicUsize as StdAtomicUsize;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[cfg(not(coverage))]
use loom::cell::UnsafeCell as LoomUnsafeCell;
#[cfg(not(coverage))]
use loom::sync::Arc;
#[cfg(not(coverage))]
use loom::sync::atomic::AtomicU8;
#[cfg(not(coverage))]
use loom::sync::atomic::AtomicUsize;
#[cfg(not(coverage))]
use loom::sync::atomic::Ordering as LoomOrdering;
#[cfg(not(coverage))]
use loom::sync::atomic::fence as loom_fence;
#[cfg(not(coverage))]
use loom::thread as loom_thread;

#[cfg(not(coverage))]
use super::AtomicU8Like;
#[cfg(not(coverage))]
use super::AtomicUsizeLike;
use super::GateLifecycle;
use super::OperationGate;
use super::StdScheduler;
use super::YieldLike;
#[test]
fn test_standard_operation_gate_lifecycle() {
    type StandardGate = OperationGate<StdAtomicU8, StdAtomicUsize, StdScheduler>;

    let state = StandardGate::new();
    assert_eq!(state.lifecycle(), GateLifecycle::Open);
    assert_eq!(state.enter_update(), Ok(()));
    assert_eq!(state.active_updates(), 1);
    state.leave_update();
    assert!(state.try_begin_finish());
    state.reopen();
    state.close();
    assert_eq!(state.lifecycle(), GateLifecycle::Closed);
    assert!(!state.try_begin_finish());

    let _: StandardGate = Default::default();
    <StdScheduler as YieldLike>::spin_loop();
    <StdScheduler as YieldLike>::yield_now();
}

#[test]
fn test_standard_operation_gate_drains_updates_and_rejects_new_work() {
    type StandardGate = OperationGate<StdAtomicU8, StdAtomicUsize, StdScheduler>;
    let state = StdArc::new(StandardGate::new());
    assert_eq!(state.enter_update(), Ok(()));
    let ready = StdArc::new(Barrier::new(2));
    let finisher_state = StdArc::clone(&state);
    let finisher_ready = StdArc::clone(&ready);
    let finisher = thread::spawn(move || {
        finisher_ready.wait();
        finisher_state.try_begin_finish()
    });
    ready.wait();
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.lifecycle() == GateLifecycle::Open && Instant::now() < deadline {
        thread::yield_now();
    }
    let observed_finishing = state.lifecycle() == GateLifecycle::Finishing;
    if !observed_finishing {
        state.leave_update();
        let _ = finisher.join().expect("finisher must join after timeout");
        panic!("finisher did not enter Finishing in time");
    }
    let rejected_update = state.enter_update();
    if rejected_update == Ok(()) {
        state.leave_update();
    }
    state.leave_update();
    let finish_result = finisher.join().expect("finisher must join");
    assert!(observed_finishing, "finisher did not enter Finishing");
    assert_eq!(rejected_update, Err(GateLifecycle::Finishing));
    assert!(finish_result);
    state.close();
    assert_eq!(state.enter_update(), Err(GateLifecycle::Closed));
}

#[cfg(not(coverage))]
impl AtomicU8Like for AtomicU8 {
    fn new(value: u8) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u8 {
        // Loom 0.7 does not include SeqCst accesses in its causality order;
        // fences expose the production total order to UnsafeCell checking.
        loom_fence(LoomOrdering::SeqCst);
        let value = self.load(LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
        value
    }

    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8> {
        loom_fence(LoomOrdering::SeqCst);
        let result = self.compare_exchange(current, new, LoomOrdering::SeqCst, LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
        result
    }

    fn store(&self, value: u8) {
        loom_fence(LoomOrdering::SeqCst);
        self.store(value, LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
    }
}

#[cfg(not(coverage))]
impl AtomicUsizeLike for AtomicUsize {
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    fn load(&self) -> usize {
        loom_fence(LoomOrdering::SeqCst);
        let value = self.load(LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
        value
    }

    fn fetch_add(&self, value: usize) {
        loom_fence(LoomOrdering::SeqCst);
        self.fetch_add(value, LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
    }

    fn fetch_sub(&self, value: usize) {
        loom_fence(LoomOrdering::SeqCst);
        self.fetch_sub(value, LoomOrdering::SeqCst);
        loom_fence(LoomOrdering::SeqCst);
    }
}

#[cfg(not(coverage))]
struct LoomScheduler;

#[cfg(not(coverage))]
impl YieldLike for LoomScheduler {
    fn spin_loop() {
        loom_thread::yield_now();
    }

    fn yield_now() {
        loom_thread::yield_now();
    }
}

#[cfg(not(coverage))]
type LoomOperationGate = OperationGate<AtomicU8, AtomicUsize, LoomScheduler>;

#[cfg(not(coverage))]
fn check_loom_model(model_fn: impl Fn() + Send + Sync + 'static) {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 4;
    builder.max_branches = 10_000;
    builder.max_permutations = Some(100_000);
    builder.preemption_bound = Some(3);
    builder.check(model_fn);
}

#[cfg(not(coverage))]
#[test]
fn test_loom_operation_gate_rejects_updates_after_finish_and_close() {
    check_loom_model(|| {
        let state: LoomOperationGate = Default::default();
        assert_eq!(state.lifecycle(), GateLifecycle::Open);
        assert_eq!(state.enter_update(), Ok(()));
        assert_eq!(state.active_updates(), 1);
        state.leave_update();
        assert_eq!(state.active_updates(), 0);

        assert!(state.try_begin_finish());
        assert_eq!(state.enter_update(), Err(GateLifecycle::Finishing));
        state.reopen();
        assert_eq!(state.lifecycle(), GateLifecycle::Open);
        assert!(state.try_begin_finish());
        state.close();
        assert!(!state.try_begin_finish());
        assert_eq!(state.enter_update(), Err(GateLifecycle::Closed));
    });
}

#[cfg(not(coverage))]
#[test]
fn test_loom_operation_state_never_closes_before_registered_updates_leave() {
    check_loom_model(|| {
        let state = Arc::new(LoomOperationGate::new());
        let updater_state = Arc::clone(&state);
        let payload = Arc::new(LoomUnsafeCell::new(0usize));
        let updater_payload = Arc::clone(&payload);
        let updater = loom_thread::spawn(move || {
            if updater_state.enter_update().is_ok() {
                updater_payload.with_mut(|value| unsafe { *value = 7 });
                updater_state.leave_update();
            }
        });

        let finisher_state = Arc::clone(&state);
        let finisher = loom_thread::spawn(move || {
            if finisher_state.try_begin_finish() {
                assert_eq!(finisher_state.lifecycle(), GateLifecycle::Finishing);
                let finished_value = payload.with(|value| unsafe { *value });
                assert!(matches!(finished_value, 0 | 7));
                finisher_state.close();
            }
        });

        updater.join().expect("updater model must join");
        finisher.join().expect("finisher model must join");
        assert_eq!(state.active_updates(), 0);
        assert!(matches!(state.lifecycle(), GateLifecycle::Open | GateLifecycle::Closed));
    });
}

#[cfg(not(coverage))]
#[test]
fn test_loom_operation_state_can_reopen_after_validation_and_close_later() {
    check_loom_model(|| {
        let state = LoomOperationGate::new();
        assert!(state.try_begin_finish());
        state.reopen();
        assert_eq!(state.lifecycle(), GateLifecycle::Open);
        assert!(state.try_begin_finish());
        state.close();
        assert_eq!(state.lifecycle(), GateLifecycle::Closed);
    });
}

#[cfg(not(coverage))]
#[test]
fn test_loom_operation_state_rejects_updates_while_finishing() {
    check_loom_model(|| {
        let state = Arc::new(LoomOperationGate::new());
        assert_eq!(state.enter_update(), Ok(()));
        let finisher_state = Arc::clone(&state);
        let finisher = loom_thread::spawn(move || finisher_state.try_begin_finish());
        while state.lifecycle() == GateLifecycle::Open {
            loom_thread::yield_now();
        }
        assert_eq!(state.lifecycle(), GateLifecycle::Finishing);
        assert_eq!(state.enter_update(), Err(GateLifecycle::Finishing));
        state.leave_update();
        assert!(finisher.join().expect("finisher model must join"));
    });
}

#[cfg(not(coverage))]
#[test]
fn test_loom_operation_state_publishes_completed_update_to_finisher() {
    check_loom_model(|| {
        let state = LoomOperationGate::new();
        let payload = Arc::new(LoomUnsafeCell::new(0usize));
        assert_eq!(state.enter_update(), Ok(()));
        let updater_payload = Arc::clone(&payload);
        let updater_state = Arc::new(state);
        let worker_state = Arc::clone(&updater_state);
        let updater = loom_thread::spawn(move || {
            updater_payload.with_mut(|value| unsafe { *value = 42 });
            worker_state.leave_update();
        });

        assert!(updater_state.try_begin_finish());
        let published = payload.with(|value| unsafe { *value });
        assert_eq!(published, 42);
        assert!(updater.join().is_ok());
    });
}
