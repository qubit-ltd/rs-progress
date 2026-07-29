// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Metric configuration and immutable metric snapshots.
// qubit-style: allow multiple-public-types

use std::sync::{
    Arc,
    Mutex,
    MutexGuard,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

use crate::{
    MetricError,
    MetricTransition,
};

/// Stable metadata for one metric in a progress operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metric {
    /// Machine-readable identifier.
    pub(crate) id: Arc<str>,
    /// Human-readable name.
    pub(crate) name: Arc<str>,
    /// Optional configured total.
    pub(crate) total: Option<u64>,
}

impl Metric {
    /// Creates metric metadata without a known total.
    ///
    /// The ID and name are validated when the enclosing progress operation is
    /// started, so this constructor never panics.
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: Arc::from(id),
            name: Arc::from(name),
            total: None,
        }
    }

    /// Records the total work for this metric.
    ///
    /// The value is carried automatically by all future events from the
    /// operation that owns this metric.
    #[must_use]
    pub const fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Returns the metric's stable ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the metric's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the configured total, if it is known.
    #[must_use]
    pub const fn configured_total(&self) -> Option<u64> {
        self.total
    }
}

/// Cloneable capability for one live metric owned by a progress operation.
///
/// All mutation methods are serialized by one short mutex critical section.
/// A handle remains readable after its progress operation closes, but rejects
/// all later mutations.
#[derive(Clone)]
pub struct MetricHandle {
    /// Shared metadata and mutable count state.
    inner: Arc<MetricInner>,
    /// Shared lifecycle gate owned by the enclosing progress operation.
    operation_open: Arc<AtomicBool>,
}

impl MetricHandle {
    /// Creates one live handle from validated metric metadata.
    pub(crate) fn new(metric: Metric, operation_open: Arc<AtomicBool>) -> Self {
        let total = metric.total;
        Self {
            inner: Arc::new(MetricInner {
                metric,
                state: Mutex::new(MetricState::new(total)),
            }),
            operation_open,
        }
    }

    /// Returns the stable metric ID.
    #[must_use]
    pub fn id(&self) -> &str {
        self.inner.metric.id()
    }

    /// Returns the stable metric display name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.inner.metric.name()
    }

    /// Replaces the known total for this metric.
    ///
    /// # Errors
    ///
    /// Returns a metric error after closure, when total is below occupied
    /// work, or when a previous panic poisoned the state lock.
    pub fn set_total(&self, total: u64) -> Result<(), MetricError> {
        let mut state = self.lock_state()?;
        self.ensure_open()?;
        let occupied = state.occupied(self.id())?;
        if total < occupied {
            return Err(MetricError::TotalBelowOccupied {
                metric_id: self.id().into(),
                total,
                occupied,
            });
        }
        state.total = Some(total);
        Ok(())
    }

    /// Moves signed work between the not-started and active states.
    ///
    /// A positive value starts work; a negative value rolls active work back
    /// to not-started.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn start(&self, count: i64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Start, count)
    }

    /// Moves signed work between the active and unclassified-completed states.
    ///
    /// A positive value completes active work without a terminal
    /// classification; a negative value rolls that completion back to active.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn complete(&self, count: i64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Complete, count)
    }

    /// Moves signed work between the active and succeeded states.
    ///
    /// A positive value marks active work as succeeded; a negative value rolls
    /// succeeded work back to active.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn succeed(&self, count: i64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Succeed, count)
    }

    /// Moves signed work between the active and failed states.
    ///
    /// A positive value marks active work as failed; a negative value rolls
    /// failed work back to active.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn fail(&self, count: i64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Fail, count)
    }

    /// Moves signed work between the active and cancelled states.
    ///
    /// A positive value marks active work as cancelled; a negative value rolls
    /// cancelled work back to active.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn cancel(&self, count: i64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Cancel, count)
    }

    /// Returns one internally consistent immutable metric snapshot.
    ///
    /// This read remains available after the owning operation closes.
    ///
    /// # Errors
    ///
    /// Returns a metric error if a previous panic poisoned the lock or if
    /// invalid internal state cannot be represented in a snapshot.
    pub fn snapshot(&self) -> Result<MetricSnapshot, MetricError> {
        let state = self.lock_state()?;
        MetricSnapshot::from_state(&self.inner.metric, *state)
    }

    /// Updates state through one validated signed transition.
    fn transition(
        &self,
        transition: MetricTransition,
        count: i64,
    ) -> Result<(), MetricError> {
        let mut state = self.lock_state()?;
        self.ensure_open()?;
        let mut next = *state;
        let amount = count.unsigned_abs();
        let reverse = count.is_negative();
        match transition {
            MetricTransition::Start => {
                move_count(
                    &mut next.active,
                    None,
                    amount,
                    reverse,
                    transition,
                    self.id(),
                )?;
            }
            MetricTransition::Complete => {
                move_count(
                    &mut next.active,
                    Some(&mut next.completed_unclassified),
                    amount,
                    reverse,
                    transition,
                    self.id(),
                )?;
            }
            MetricTransition::Succeed => {
                move_count(
                    &mut next.active,
                    Some(&mut next.succeeded),
                    amount,
                    reverse,
                    transition,
                    self.id(),
                )?;
            }
            MetricTransition::Fail => {
                move_count(
                    &mut next.active,
                    Some(&mut next.failed),
                    amount,
                    reverse,
                    transition,
                    self.id(),
                )?;
            }
            MetricTransition::Cancel => {
                move_count(
                    &mut next.active,
                    Some(&mut next.cancelled),
                    amount,
                    reverse,
                    transition,
                    self.id(),
                )?;
            }
        }
        next.validate(self.id())?;
        *state = next;
        Ok(())
    }

    /// Locks the mutable state or maps lock poisoning to a public error.
    fn lock_state(&self) -> Result<MutexGuard<'_, MetricState>, MetricError> {
        self.inner
            .state
            .lock()
            .map_err(|_| MetricError::StatePoisoned {
                metric_id: self.id().into(),
            })
    }

    /// Rejects writes after the owning progress operation has closed.
    fn ensure_open(&self) -> Result<(), MetricError> {
        if self.operation_open.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(MetricError::Closed {
                metric_id: self.id().into(),
            })
        }
    }
}

/// Immutable metadata and mutex-protected dynamic state for one handle.
struct MetricInner {
    /// Fixed metric definition supplied to the progress builder.
    metric: Metric,
    /// Dynamic state guarded for transactional changes and snapshots.
    state: Mutex<MetricState>,
}

/// Dynamic metric counts stored only inside one metric transaction lock.
#[derive(Clone, Copy)]
struct MetricState {
    /// Current optional total, mutable through the handle.
    total: Option<u64>,
    /// Work that has started but is not terminal.
    active: u64,
    /// Terminal work without explicit success, failure, or cancellation.
    completed_unclassified: u64,
    /// Terminal work classified as successful.
    succeeded: u64,
    /// Terminal work classified as failed.
    failed: u64,
    /// Terminal work classified as cancelled.
    cancelled: u64,
}

impl MetricState {
    /// Creates a zeroed state carrying the builder-provided total.
    const fn new(total: Option<u64>) -> Self {
        Self {
            total,
            active: 0,
            completed_unclassified: 0,
            succeeded: 0,
            failed: 0,
            cancelled: 0,
        }
    }

    /// Returns the derived public completed count with checked arithmetic.
    fn completed(&self, metric_id: &str) -> Result<u64, MetricError> {
        self.completed_unclassified
            .checked_add(self.succeeded)
            .and_then(|value| value.checked_add(self.failed))
            .and_then(|value| value.checked_add(self.cancelled))
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })
    }

    /// Returns active plus completed work with checked arithmetic.
    fn occupied(&self, metric_id: &str) -> Result<u64, MetricError> {
        self.completed(metric_id)?
            .checked_add(self.active)
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })
    }

    /// Validates the aggregate conservation invariants for one pending state.
    fn validate(&self, metric_id: &str) -> Result<(), MetricError> {
        let occupied = self.occupied(metric_id)?;
        if let Some(total) = self.total
            && occupied > total
        {
            return Err(MetricError::TotalExceeded {
                metric_id: metric_id.into(),
                total,
                attempted: occupied,
            });
        }
        Ok(())
    }
}

/// Moves an amount between active work and an optional terminal state.
fn move_count(
    active: &mut u64,
    terminal: Option<&mut u64>,
    amount: u64,
    reverse: bool,
    transition: MetricTransition,
    metric_id: &str,
) -> Result<(), MetricError> {
    if reverse {
        if let Some(terminal) = terminal {
            let available = *terminal;
            *terminal = terminal.checked_sub(amount).ok_or_else(|| {
                MetricError::InsufficientCount {
                    metric_id: metric_id.into(),
                    transition,
                    requested: amount,
                    available,
                }
            })?;
            *active = active.checked_add(amount).ok_or_else(|| {
                MetricError::CountOverflow {
                    metric_id: metric_id.into(),
                }
            })?;
        } else {
            let available = *active;
            *active = active.checked_sub(amount).ok_or_else(|| {
                MetricError::InsufficientCount {
                    metric_id: metric_id.into(),
                    transition,
                    requested: amount,
                    available,
                }
            })?;
        }
        return Ok(());
    }

    if let Some(terminal) = terminal {
        let available = *active;
        *active = active.checked_sub(amount).ok_or_else(|| {
            MetricError::InsufficientCount {
                metric_id: metric_id.into(),
                transition,
                requested: amount,
                available,
            }
        })?;
        *terminal = terminal.checked_add(amount).ok_or_else(|| {
            MetricError::CountOverflow {
                metric_id: metric_id.into(),
            }
        })?;
    } else {
        *active = active.checked_add(amount).ok_or_else(|| {
            MetricError::CountOverflow {
                metric_id: metric_id.into(),
            }
        })?;
    }
    Ok(())
}

/// Immutable complete state for one metric in an emitted event.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricSnapshot {
    /// Machine-readable metric ID.
    id: Arc<str>,
    /// Human-readable metric name.
    name: Arc<str>,
    /// Configured total, if known.
    total: Option<u64>,
    /// Completed count.
    completed: u64,
    /// Active count.
    active: u64,
    /// Succeeded count.
    succeeded: u64,
    /// Failed count.
    failed: u64,
    /// Cancelled count.
    cancelled: u64,
}

impl MetricSnapshot {
    /// Builds an immutable snapshot from one internally validated metric state.
    fn from_state(
        metric: &Metric,
        state: MetricState,
    ) -> Result<Self, MetricError> {
        Ok(Self {
            id: Arc::clone(&metric.id),
            name: Arc::clone(&metric.name),
            total: state.total,
            completed: state.completed(metric.id())?,
            active: state.active,
            succeeded: state.succeeded,
            failed: state.failed,
            cancelled: state.cancelled,
        })
    }
    /// Returns the metric's stable ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the metric's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the total configured for this event's metric.
    #[must_use]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }
    /// Returns the number of completed work items.
    #[must_use]
    pub const fn completed(&self) -> u64 {
        self.completed
    }
    /// Returns the number of active work items.
    #[must_use]
    pub const fn active(&self) -> u64 {
        self.active
    }
    /// Returns the number of explicitly successful work items.
    #[must_use]
    pub const fn succeeded(&self) -> u64 {
        self.succeeded
    }
    /// Returns the number of explicitly failed work items.
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }
    /// Returns the number of explicitly cancelled work items.
    #[must_use]
    pub const fn cancelled(&self) -> u64 {
        self.cancelled
    }
    /// Returns the completed fraction when the total is positive and known.
    #[must_use]
    pub fn completion_fraction(&self) -> Option<f64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.completed as f64 / total as f64)
    }
}
