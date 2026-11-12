// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Independent integer model for metric delta validity and committed counts.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ModelState {
    pub(crate) active: u128,
    pub(crate) unclassified: u128,
    pub(crate) succeeded: u128,
    pub(crate) failed: u128,
    pub(crate) cancelled: u128,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ModelDelta {
    pub(crate) started: u64,
    pub(crate) unclassified: u64,
    pub(crate) succeeded: u64,
    pub(crate) failed: u64,
    pub(crate) cancelled: u64,
}

impl ModelState {
    /// Returns the next state if the public metric count contract permits it.
    pub(crate) fn transition(self, delta: ModelDelta, total: Option<u64>) -> Option<Self> {
        let limit = u128::from(u64::MAX);
        let terminal = u128::from(delta.unclassified)
            + u128::from(delta.succeeded)
            + u128::from(delta.failed)
            + u128::from(delta.cancelled);
        let available = self.active + u128::from(delta.started);
        if available > limit || terminal > limit || terminal > available {
            return None;
        }

        let next = Self {
            active: available - terminal,
            unclassified: self.unclassified + u128::from(delta.unclassified),
            succeeded: self.succeeded + u128::from(delta.succeeded),
            failed: self.failed + u128::from(delta.failed),
            cancelled: self.cancelled + u128::from(delta.cancelled),
        };
        let completed = next.unclassified + next.succeeded + next.failed + next.cancelled;
        let occupied = next.active + completed;
        if [
            next.active,
            next.unclassified,
            next.succeeded,
            next.failed,
            next.cancelled,
            completed,
            occupied,
        ]
        .into_iter()
        .any(|count| count > limit)
        {
            return None;
        }
        if total.is_some_and(|total| occupied > u128::from(total)) {
            return None;
        }
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::ModelDelta;
    use super::ModelState;

    #[test]
    fn test_transition_table_covers_valid_invalid_and_boundary_states() {
        let cases = [
            (
                ModelState::default(),
                ModelDelta::default(),
                None,
                Some(ModelState::default()),
            ),
            (
                ModelState::default(),
                ModelDelta {
                    started: 3,
                    succeeded: 1,
                    ..ModelDelta::default()
                },
                Some(3),
                Some(ModelState {
                    active: 2,
                    succeeded: 1,
                    ..ModelState::default()
                }),
            ),
            (
                ModelState {
                    active: 2,
                    ..ModelState::default()
                },
                ModelDelta {
                    succeeded: 1,
                    failed: 1,
                    ..ModelDelta::default()
                },
                Some(2),
                Some(ModelState {
                    succeeded: 1,
                    failed: 1,
                    ..ModelState::default()
                }),
            ),
            (
                ModelState::default(),
                ModelDelta {
                    succeeded: 1,
                    ..ModelDelta::default()
                },
                None,
                None,
            ),
            (
                ModelState::default(),
                ModelDelta {
                    started: 2,
                    ..ModelDelta::default()
                },
                Some(1),
                None,
            ),
            (
                ModelState {
                    succeeded: u128::from(u64::MAX),
                    ..ModelState::default()
                },
                ModelDelta {
                    started: 1,
                    ..ModelDelta::default()
                },
                None,
                None,
            ),
            (
                ModelState {
                    active: u128::from(u64::MAX),
                    ..ModelState::default()
                },
                ModelDelta {
                    started: 1,
                    ..ModelDelta::default()
                },
                None,
                None,
            ),
            (
                ModelState {
                    active: u128::from(u64::MAX),
                    ..ModelState::default()
                },
                ModelDelta {
                    failed: u64::MAX,
                    cancelled: 1,
                    ..ModelDelta::default()
                },
                None,
                None,
            ),
            (
                ModelState {
                    active: 1,
                    succeeded: u128::from(u64::MAX),
                    ..ModelState::default()
                },
                ModelDelta {
                    succeeded: 1,
                    ..ModelDelta::default()
                },
                None,
                None,
            ),
        ];
        for (state, delta, total, expected) in cases {
            assert_eq!(state.transition(delta, total), expected);
        }
    }

    #[test]
    fn test_valid_invalid_valid_sequence_keeps_model_state() {
        let initial = ModelState::default();
        let first = initial
            .transition(
                ModelDelta {
                    started: 2,
                    succeeded: 1,
                    ..ModelDelta::default()
                },
                Some(4),
            )
            .expect("first valid transition must produce a model state");
        assert_eq!(first.active, 1);
        assert_eq!(first.succeeded, 1);

        let rejected = first.transition(
            ModelDelta {
                failed: 2,
                ..ModelDelta::default()
            },
            Some(4),
        );
        assert_eq!(rejected, None);

        let last = first
            .transition(
                ModelDelta {
                    started: 2,
                    unclassified: 1,
                    cancelled: 1,
                    ..ModelDelta::default()
                },
                Some(4),
            )
            .expect("valid transition after rejection must use the prior state");
        assert_eq!(
            last,
            ModelState {
                active: 1,
                unclassified: 1,
                succeeded: 1,
                cancelled: 1,
                ..ModelState::default()
            }
        );
    }
}
