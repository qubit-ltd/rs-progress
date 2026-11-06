// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::atomic::AtomicU64 as StdAtomicU64;

use loom::sync::Arc;
use loom::sync::atomic::AtomicU64 as LoomAtomicU64;
use loom::sync::atomic::Ordering as LoomOrdering;
use loom::thread;

use crate::MetricError;
use crate::metric::internal::MetricCounts;
use crate::metric::internal::metric_atomic_word::MetricAtomicWord;
use crate::metric::internal::metric_counter_state::MetricCounterState;

impl MetricAtomicWord for LoomAtomicU64 {
    fn new(value: u64) -> Self {
        Self::new(value)
    }

    fn load(&self) -> u64 {
        self.load(LoomOrdering::Acquire)
    }

    fn store(&self, value: u64) {
        self.store(value, LoomOrdering::Release);
    }

    fn compare_set(&self, expected: u64, next: u64) -> Result<(), u64> {
        self.compare_exchange(expected, next, LoomOrdering::AcqRel, LoomOrdering::Acquire)
            .map(|_| ())
    }
}

fn loom_model(run: impl Fn() + Send + Sync + 'static) {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.max_branches = 10_000;
    builder.max_permutations = Some(100_000);
    builder.preemption_bound = Some(3);
    builder.check(run);
}

#[test]
fn test_loom_metric_snapshot_conserves_counts() {
    loom_model(|| {
        let state = Arc::new(MetricCounterState::<LoomAtomicU64, LoomAtomicU64>::new());
        state
            .with_update(
                |counts| {
                    counts.active = 1;
                    Ok(())
                },
                |_| thread::yield_now(),
            )
            .expect("initial count must be committed");

        let writer = thread::spawn({
            let state = Arc::clone(&state);
            move || {
                state
                    .with_update(
                        |counts| {
                            counts.active -= 1;
                            counts.completed_unclassified += 1;
                            Ok(())
                        },
                        |_| thread::yield_now(),
                    )
                    .expect("transition must be committed");
            }
        });
        let reader = thread::spawn({
            let state = Arc::clone(&state);
            move || {
                let counts = state.snapshot_counts(|_| thread::yield_now());
                assert_eq!(counts.active + counts.completed_unclassified, 1);
            }
        });

        writer.join().expect("writer must not panic");
        reader.join().expect("reader must not panic");
    });
}

#[test]
fn test_loom_metric_two_writers_do_not_lose_updates() {
    loom_model(|| {
        let state = Arc::new(MetricCounterState::<LoomAtomicU64, LoomAtomicU64>::new());
        let writers: Vec<_> = (0..2)
            .map(|_| {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    state
                        .with_update(
                            |counts| {
                                counts.active += 1;
                                Ok(())
                            },
                            |_| thread::yield_now(),
                        )
                        .expect("writer update must be committed");
                })
            })
            .collect();
        for writer in writers {
            writer.join().expect("writer must not panic");
        }
        let counts = state.snapshot_counts(|_| thread::yield_now());
        assert_eq!(counts.active, 2);
        assert_eq!(counts.completed_unclassified, 0);
        assert_eq!(counts.succeeded, 0);
        assert_eq!(counts.failed, 0);
        assert_eq!(counts.cancelled, 0);
    });
}

#[test]
fn test_loom_metric_rejected_update_is_not_committed() {
    loom_model(|| {
        let state = Arc::new(MetricCounterState::<LoomAtomicU64, LoomAtomicU64>::new());
        let rejected = thread::spawn({
            let state = Arc::clone(&state);
            move || {
                let result: Result<(), MetricError> = state.with_update(
                    |counts| {
                        counts.active += 1;
                        Err(MetricError::CountOverflow {
                            metric_id: "tasks".into(),
                        })
                    },
                    |_| thread::yield_now(),
                );
                assert!(matches!(result, Err(MetricError::CountOverflow { .. })));
            }
        });
        let successful = thread::spawn({
            let state = Arc::clone(&state);
            move || {
                state
                    .with_update(
                        |counts| {
                            counts.active += 1;
                            Ok(())
                        },
                        |_| thread::yield_now(),
                    )
                    .expect("successful update must be committed");
            }
        });
        rejected.join().expect("rejected update must not panic");
        successful.join().expect("successful update must not panic");
        assert_eq!(state.snapshot_counts(|_| thread::yield_now()).active, 1);
    });
}

#[test]
fn test_metric_update_panic_releases_gate_without_commit() {
    let state = MetricCounterState::<StdAtomicU64, StdAtomicU64>::new();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), MetricError> = state.with_update(
            |counts| {
                counts.active = 7;
                panic!("intentional update panic");
            },
            |_| {},
        );
    }));
    assert!(panic.is_err());

    state
        .with_update(
            |counts| {
                counts.active = 1;
                Ok(())
            },
            |_| {},
        )
        .expect("gate must be released after panic");
    let counts: MetricCounts = state.snapshot_counts(|_| {});
    assert_eq!(counts.active, 1);
}
