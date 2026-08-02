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

use loom::sync::{Arc, atomic::{AtomicU8, AtomicUsize}};
use operation_gate::{AtomicU8Like, AtomicUsizeLike, GateLifecycle, OperationGate, YieldLike};

impl AtomicU8Like for AtomicU8 {
    fn new(value: u8) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u8 {
        self.load(loom::sync::atomic::Ordering::Acquire)
    }

    fn compare_exchange(&self, current: u8, new: u8) -> Result<u8, u8> {
        self.compare_exchange(
            current,
            new,
            loom::sync::atomic::Ordering::AcqRel,
            loom::sync::atomic::Ordering::Acquire,
        )
    }

    fn store(&self, value: u8) {
        self.store(value, loom::sync::atomic::Ordering::Release);
    }
}

impl AtomicUsizeLike for AtomicUsize {
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    fn load(&self) -> usize {
        self.load(loom::sync::atomic::Ordering::Acquire)
    }

    fn fetch_add(&self, value: usize) {
        self.fetch_add(value, loom::sync::atomic::Ordering::AcqRel);
    }

    fn fetch_sub(&self, value: usize) {
        self.fetch_sub(value, loom::sync::atomic::Ordering::Release);
    }
}

struct LoomScheduler;

impl YieldLike for LoomScheduler {
    fn spin_loop() {
        std::hint::spin_loop();
    }

    fn yield_now() {
        loom::thread::yield_now();
    }
}

type LoomOperationGate = OperationGate<AtomicU8, AtomicUsize, LoomScheduler>;

#[test]
fn operation_state_never_closes_before_registered_updates_leave() {
    loom::model(|| {
        let state = Arc::new(LoomOperationGate::new());
        let updater_state = Arc::clone(&state);
        let updater = loom::thread::spawn(move || {
            if updater_state.enter_update().is_ok() {
                updater_state.leave_update();
            }
        });

        let finisher_state = Arc::clone(&state);
        let finisher = loom::thread::spawn(move || {
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
fn operation_state_can_reopen_after_validation_and_close_later() {
    loom::model(|| {
        let state = LoomOperationGate::new();
        assert!(state.try_begin_finish());
        state.reopen();
        assert_eq!(state.lifecycle(), GateLifecycle::Open);
        assert!(state.try_begin_finish());
        state.close();
        assert_eq!(state.lifecycle(), GateLifecycle::Closed);
    });
}
