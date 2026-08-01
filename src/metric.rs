// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Metric configuration and immutable metric snapshots.
// qubit-style: allow multiple-public-types

use std::{
    hint::spin_loop,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

use qubit_fast_cas::CasCell;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer};

#[cfg(feature = "serde")]
use crate::validation::{validate_metrics, validate_snapshot_counts};
use crate::{MetricError, MetricTransition};

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
/// All mutation methods are serialized by one CAS gate critical section.
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
        Self {
            inner: Arc::new(MetricInner::new(metric)),
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

    /// Moves work from the not-started state to the active state.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn start(&self, count: u64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Start, count, Direction::Forward)
    }

    /// Moves work from the active state to unclassified completion.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn complete(&self, count: u64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Complete, count, Direction::Forward)
    }

    /// Moves work from the active state to the succeeded state.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn succeed(&self, count: u64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Succeed, count, Direction::Forward)
    }

    /// Moves work from the active state to the failed state.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn fail(&self, count: u64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Fail, count, Direction::Forward)
    }

    /// Moves work from the active state to the cancelled state.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the transition violates aggregate state
    /// invariants or when the owning operation is closed.
    pub fn cancel(&self, count: u64) -> Result<(), MetricError> {
        self.transition(MetricTransition::Cancel, count, Direction::Forward)
    }

    /// Rolls work back along one named transition.
    ///
    /// `Start` returns active work to the not-started state. Every other
    /// transition returns work from its terminal state to the active state.
    ///
    /// # Errors
    ///
    /// Returns a metric error when the selected source state does not contain
    /// `count` work or when the owning operation is closed.
    pub fn rollback(&self, transition: MetricTransition, count: u64) -> Result<(), MetricError> {
        self.transition(transition, count, Direction::Rollback)
    }

    /// Returns one internally consistent immutable metric snapshot.
    ///
    /// This read remains available after the owning operation closes.
    #[must_use]
    pub fn snapshot(&self) -> MetricSnapshot {
        let counts = self.inner.snapshot_counts();
        MetricSnapshot::from_counts(&self.inner.metric, counts)
            .expect("metric counts must remain internally consistent")
    }

    /// Updates state through one validated directional transition.
    fn transition(
        &self,
        transition: MetricTransition,
        count: u64,
        direction: Direction,
    ) -> Result<(), MetricError> {
        self.ensure_open()?;
        let metric_id = self.id();
        let total = self.inner.metric.configured_total();

        self.inner.with_update(|counts| {
            if !self.operation_open.load(Ordering::Acquire) {
                return Err(MetricError::Closed {
                    metric_id: metric_id.into(),
                });
            }
            let mut next = *counts;
            match transition {
                MetricTransition::Start => {
                    move_count(
                        &mut next.active,
                        None,
                        count,
                        direction,
                        transition,
                        metric_id,
                    )?;
                }
                MetricTransition::Complete => {
                    move_count(
                        &mut next.active,
                        Some(&mut next.completed_unclassified),
                        count,
                        direction,
                        transition,
                        metric_id,
                    )?;
                }
                MetricTransition::Succeed => {
                    move_count(
                        &mut next.active,
                        Some(&mut next.succeeded),
                        count,
                        direction,
                        transition,
                        metric_id,
                    )?;
                }
                MetricTransition::Fail => {
                    move_count(
                        &mut next.active,
                        Some(&mut next.failed),
                        count,
                        direction,
                        transition,
                        metric_id,
                    )?;
                }
                MetricTransition::Cancel => {
                    move_count(
                        &mut next.active,
                        Some(&mut next.cancelled),
                        count,
                        direction,
                        transition,
                        metric_id,
                    )?;
                }
            }
            next.validate(metric_id, total)?;
            *counts = next;
            Ok(())
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

/// Direction in which a metric state transition moves work.
#[derive(Clone, Copy)]
enum Direction {
    /// Moves work from the transition source to its target.
    Forward,
    /// Moves work from the transition target back to its source.
    Rollback,
}

/// Immutable metadata and atomic dynamic state for one handle.
struct MetricInner {
    /// Fixed metric definition supplied to the progress builder.
    metric: Metric,
    /// Dynamic updates are serialized by this gate.
    gate: CasCell,
    /// Work that has started but is not terminal.
    active: AtomicU64,
    /// Terminal work without explicit success, failure, or cancellation.
    completed_unclassified: AtomicU64,
    /// Terminal work classified as successful.
    succeeded: AtomicU64,
    /// Terminal work classified as failed.
    failed: AtomicU64,
    /// Terminal work classified as cancelled.
    cancelled: AtomicU64,
}

impl MetricInner {
    /// Builds one live metric inner state with zeroed counters.
    fn new(metric: Metric) -> Self {
        Self {
            metric,
            gate: CasCell::new(0),
            active: AtomicU64::new(0),
            completed_unclassified: AtomicU64::new(0),
            succeeded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        }
    }

    /// Runs one validated update while exclusively holding the gate.
    fn with_update<R, F>(&self, mut update: F) -> Result<R, MetricError>
    where
        F: FnMut(&mut MetricCounts) -> Result<R, MetricError>,
    {
        let mut attempts = 0;
        loop {
            let version = self.gate.load();
            if version & 1 != 0 {
                wait_for_contention(attempts);
                attempts += 1;
                continue;
            }

            match self.gate.compare_set(version, version.wrapping_add(1)) {
                Ok(()) => {
                    let _guard = MetricGateGuard::new(&self.gate, version.wrapping_add(2));
                    let mut counts = self.read_counts();
                    let result = update(&mut counts);
                    if result.is_ok() {
                        self.write_counts(&counts);
                    }
                    return result;
                }
                Err(_) => {
                    wait_for_contention(attempts);
                    attempts += 1;
                }
            }
        }
    }

    /// Reads all counter fields with acquire order and copies them by value.
    fn read_counts(&self) -> MetricCounts {
        MetricCounts {
            active: self.active.load(Ordering::Acquire),
            completed_unclassified: self.completed_unclassified.load(Ordering::Acquire),
            succeeded: self.succeeded.load(Ordering::Acquire),
            failed: self.failed.load(Ordering::Acquire),
            cancelled: self.cancelled.load(Ordering::Acquire),
        }
    }

    /// Writes all counter fields after successful validation.
    fn write_counts(&self, counts: &MetricCounts) {
        self.active.store(counts.active, Ordering::Release);
        self.completed_unclassified
            .store(counts.completed_unclassified, Ordering::Release);
        self.succeeded.store(counts.succeeded, Ordering::Release);
        self.failed.store(counts.failed, Ordering::Release);
        self.cancelled.store(counts.cancelled, Ordering::Release);
    }

    /// Repeatedly reads counts and validates version stability.
    fn snapshot_counts(&self) -> MetricCounts {
        let mut attempts = 0;
        loop {
            let start = self.gate.load();
            if start & 1 != 0 {
                wait_for_contention(attempts);
                attempts += 1;
                continue;
            }

            let counts = self.read_counts();
            if start == self.gate.load() {
                return counts;
            }

            wait_for_contention(attempts);
            attempts += 1;
        }
    }
}

/// Dynamic metric counts for a CAS transaction.
#[derive(Clone, Copy)]
struct MetricCounts {
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

impl MetricCounts {
    /// Returns the derived public completed count with checked arithmetic.
    fn completed(self, metric_id: &str) -> Result<u64, MetricError> {
        self.completed_unclassified
            .checked_add(self.succeeded)
            .and_then(|value| value.checked_add(self.failed))
            .and_then(|value| value.checked_add(self.cancelled))
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })
    }

    /// Returns active plus completed work with checked arithmetic.
    fn occupied(self, metric_id: &str) -> Result<u64, MetricError> {
        self.completed(metric_id)?
            .checked_add(self.active)
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })
    }

    /// Validates the aggregate conservation invariants for one pending state.
    fn validate(self, metric_id: &str, total: Option<u64>) -> Result<(), MetricError> {
        let occupied = self.occupied(metric_id)?;
        if let Some(total) = total
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
    direction: Direction,
    transition: MetricTransition,
    metric_id: &str,
) -> Result<(), MetricError> {
    if matches!(direction, Direction::Rollback) {
        if let Some(terminal) = terminal {
            let available = *terminal;
            *terminal =
                terminal
                    .checked_sub(amount)
                    .ok_or_else(|| MetricError::InsufficientCount {
                        metric_id: metric_id.into(),
                        transition,
                        requested: amount,
                        available,
                    })?;
            *active = active
                .checked_add(amount)
                .ok_or_else(|| MetricError::CountOverflow {
                    metric_id: metric_id.into(),
                })?;
        } else {
            let available = *active;
            *active = active
                .checked_sub(amount)
                .ok_or_else(|| MetricError::InsufficientCount {
                    metric_id: metric_id.into(),
                    transition,
                    requested: amount,
                    available,
                })?;
        }
        return Ok(());
    }

    if let Some(terminal) = terminal {
        let available = *active;
        *active = active
            .checked_sub(amount)
            .ok_or_else(|| MetricError::InsufficientCount {
                metric_id: metric_id.into(),
                transition,
                requested: amount,
                available,
            })?;
        *terminal = terminal
            .checked_add(amount)
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })?;
    } else {
        *active = active
            .checked_add(amount)
            .ok_or_else(|| MetricError::CountOverflow {
                metric_id: metric_id.into(),
            })?;
    }
    Ok(())
}

/// RAII wrapper that always releases a locked gate.
struct MetricGateGuard<'gate> {
    gate: &'gate CasCell,
    next_version: u64,
}

impl<'gate> MetricGateGuard<'gate> {
    fn new(gate: &'gate CasCell, next_version: u64) -> Self {
        Self { gate, next_version }
    }
}

impl Drop for MetricGateGuard<'_> {
    fn drop(&mut self) {
        self.gate.store(self.next_version);
    }
}

/// Immutable complete state for one metric in an emitted event.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
    fn from_counts(metric: &Metric, counts: MetricCounts) -> Result<Self, MetricError> {
        Ok(Self {
            id: Arc::clone(&metric.id),
            name: Arc::clone(&metric.name),
            total: metric.total,
            completed: counts.completed(metric.id())?,
            active: counts.active,
            succeeded: counts.succeeded,
            failed: counts.failed,
            cancelled: counts.cancelled,
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

/// Serializable wire representation used to validate standalone snapshots.
#[cfg(feature = "serde")]
#[derive(Deserialize)]
struct MetricSnapshotWire {
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

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MetricSnapshot {
    /// Deserializes and validates one standalone metric snapshot.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = MetricSnapshotWire::deserialize(deserializer)?;
        let snapshot = Self {
            id: wire.id,
            name: wire.name,
            total: wire.total,
            completed: wire.completed,
            active: wire.active,
            succeeded: wire.succeeded,
            failed: wire.failed,
            cancelled: wire.cancelled,
        };
        let metric = Metric {
            id: Arc::clone(&snapshot.id),
            name: Arc::clone(&snapshot.name),
            total: snapshot.total,
        };
        validate_metrics(std::slice::from_ref(&metric)).map_err(serde::de::Error::custom)?;
        validate_snapshot_counts(&snapshot).map_err(serde::de::Error::custom)?;
        Ok(snapshot)
    }
}

/// Busy-wait helper for writer contention and snapshot retries.
#[inline]
fn wait_for_contention(attempts: usize) {
    if attempts > 0 && attempts.is_multiple_of(16) {
        thread::yield_now();
    } else {
        spin_loop();
    }
}
