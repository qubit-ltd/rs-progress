// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Loom models for the operation freeze protocol.

use loom::sync::{
    Arc,
    atomic::{AtomicU8, AtomicUsize, Ordering},
};

const OPEN: u8 = 0;
const FINISHING: u8 = 1;
const CLOSED: u8 = 2;

#[test]
fn operation_state_never_closes_before_registered_updates_leave() {
    loom::model(|| {
        let lifecycle = Arc::new(AtomicU8::new(OPEN));
        let active = Arc::new(AtomicUsize::new(0));
        let updater_lifecycle = Arc::clone(&lifecycle);
        let updater_active = Arc::clone(&active);
        let updater = loom::thread::spawn(move || {
            if updater_lifecycle.load(Ordering::Acquire) != OPEN {
                return;
            }
            updater_active.fetch_add(1, Ordering::AcqRel);
            if updater_lifecycle.load(Ordering::Acquire) != OPEN {
                loom::thread::yield_now();
            }
            updater_active.fetch_sub(1, Ordering::Release);
        });

        let finisher_lifecycle = Arc::clone(&lifecycle);
        let finisher_active = Arc::clone(&active);
        let finisher = loom::thread::spawn(move || {
            if finisher_lifecycle
                .compare_exchange(OPEN, FINISHING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                while finisher_active.load(Ordering::Acquire) != 0 {
                    loom::thread::yield_now();
                }
                assert_eq!(finisher_lifecycle.load(Ordering::Acquire), FINISHING);
                finisher_lifecycle.store(CLOSED, Ordering::Release);
            }
        });

        updater.join().expect("updater model must join");
        finisher.join().expect("finisher model must join");
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert!(matches!(lifecycle.load(Ordering::Acquire), OPEN | CLOSED));
    });
}

#[test]
fn operation_state_can_reopen_after_validation_and_close_later() {
    loom::model(|| {
        let lifecycle = AtomicU8::new(OPEN);
        assert_eq!(
            lifecycle.compare_exchange(OPEN, FINISHING, Ordering::AcqRel, Ordering::Acquire),
            Ok(OPEN)
        );
        lifecycle.store(OPEN, Ordering::Release);
        assert_eq!(lifecycle.load(Ordering::Acquire), OPEN);
        assert_eq!(
            lifecycle.compare_exchange(OPEN, FINISHING, Ordering::AcqRel, Ordering::Acquire),
            Ok(OPEN)
        );
        lifecycle.store(CLOSED, Ordering::Release);
        assert_eq!(lifecycle.load(Ordering::Acquire), CLOSED);
    });
}
