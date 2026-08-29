// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Loom models for the production operation freeze protocol.

#[path = "../src/internal/operation_gate.rs"]
mod operation_gate;

use loom::model;
use loom::sync::Arc;
use loom::sync::atomic::AtomicU8;
use loom::sync::atomic::AtomicUsize;
use loom::sync::atomic::Ordering as LoomOrdering;
use loom::thread as loom_thread;
use operation_gate::AtomicU8Like;
use operation_gate::AtomicUsizeLike;
use operation_gate::GateLifecycle;
use operation_gate::OperationGate;
use operation_gate::StdScheduler;
use operation_gate::YieldLike;

#[test]
fn test_standard_operation_gate_lifecycle() {
    use std::sync::atomic::AtomicU8;
    use std::sync::atomic::AtomicUsize;

    type StandardGate = OperationGate<AtomicU8, AtomicUsize, StdScheduler>;

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
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;
    use std::time::Duration;

    type StandardGate = OperationGate<std::sync::atomic::AtomicU8, std::sync::atomic::AtomicUsize, StdScheduler>;
    let state = Arc::new(StandardGate::new());
    assert_eq!(state.enter_update(), Ok(()));
    let ready = Arc::new(Barrier::new(2));
    let finisher_state = Arc::clone(&state);
    let finisher_ready = Arc::clone(&ready);
    let finisher = thread::spawn(move || {
        finisher_ready.wait();
        finisher_state.try_begin_finish()
    });
    ready.wait();
    for _ in 0..100 {
        if state.lifecycle() == GateLifecycle::Finishing {
            break;
        }
        thread::yield_now();
    }
    assert_eq!(state.lifecycle(), GateLifecycle::Finishing);
    assert_eq!(state.enter_update(), Err(GateLifecycle::Finishing));
    thread::sleep(Duration::from_millis(1));
    state.leave_update();
    assert!(finisher.join().expect("finisher must join"));
    state.close();
    assert_eq!(state.enter_update(), Err(GateLifecycle::Closed));
}

impl AtomicU8Like for AtomicU8 {
    fn new(value: u8) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u8 {
        self.load(LoomOrdering::Acquire)
    }

    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8> {
        self.compare_exchange(current, new, LoomOrdering::AcqRel, LoomOrdering::Acquire)
    }

    fn store(&self, value: u8) {
        self.store(value, LoomOrdering::Release);
    }
}

impl AtomicUsizeLike for AtomicUsize {
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    fn load(&self) -> usize {
        self.load(LoomOrdering::Acquire)
    }

    fn fetch_add(&self, value: usize) {
        self.fetch_add(value, LoomOrdering::AcqRel);
    }

    fn fetch_sub(&self, value: usize) {
        self.fetch_sub(value, LoomOrdering::Release);
    }
}

struct LoomScheduler;

impl YieldLike for LoomScheduler {
    fn spin_loop() {
        std::hint::spin_loop();
    }

    fn yield_now() {
        loom_thread::yield_now();
    }
}

type LoomOperationGate = OperationGate<AtomicU8, AtomicUsize, LoomScheduler>;

#[test]
fn test_loom_operation_gate_rejects_updates_after_finish_and_close() {
    model(|| {
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

#[test]
fn test_loom_operation_state_never_closes_before_registered_updates_leave() {
    model(|| {
        let state = Arc::new(LoomOperationGate::new());
        let updater_state = Arc::clone(&state);
        let updater = loom_thread::spawn(move || {
            if updater_state.enter_update().is_ok() {
                updater_state.leave_update();
            }
        });

        let finisher_state = Arc::clone(&state);
        let finisher = loom_thread::spawn(move || {
            if finisher_state.try_begin_finish() {
                assert_eq!(finisher_state.lifecycle(), GateLifecycle::Finishing);
                while finisher_state.active_updates() != 0 {
                    LoomScheduler::yield_now();
                }
                finisher_state.close();
            }
        });

        updater.join().expect("updater model must join");
        finisher.join().expect("finisher model must join");
        assert_eq!(state.active_updates(), 0);
        assert!(matches!(state.lifecycle(), GateLifecycle::Open | GateLifecycle::Closed));
    });
}

#[test]
fn test_loom_operation_state_can_reopen_after_validation_and_close_later() {
    model(|| {
        let state = LoomOperationGate::new();
        assert!(state.try_begin_finish());
        state.reopen();
        assert_eq!(state.lifecycle(), GateLifecycle::Open);
        assert!(state.try_begin_finish());
        state.close();
        assert_eq!(state.lifecycle(), GateLifecycle::Closed);
    });
}
