// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable event snapshots delivered to progress reporters.

use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Deserializer;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::Serializer;
#[cfg(feature = "serde")]
use serde::de::Error;

#[cfg(feature = "serde")]
use super::EventWire;
#[cfg(feature = "serde")]
use super::EventWireRef;
use super::Phase;
#[cfg(feature = "serde")]
use crate::Metric;
use crate::MetricSnapshot;
use crate::OperationAttributes;
use crate::Stage;
#[cfg(feature = "serde")]
use crate::validation::validate_attributes;
#[cfg(feature = "serde")]
use crate::validation::validate_metrics;

/// Complete immutable snapshot delivered to one reporter call.
///
/// Events preserve the operation metadata and metric snapshots as observed at
/// one attempted delivery. They are created by [`Progress`](crate::Progress)
/// and exposed to a [`Reporter`](crate::Reporter); callers cannot construct
/// events directly.
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use qubit_progress::{Event, Metric, Progress, ReporterError};
///
/// let observed = Arc::new(Mutex::new(None::<Event>));
/// let sink = Arc::clone(&observed);
/// let reporter = Arc::new(move |event: &Event| {
///     *sink.lock().expect("event lock must be available") = Some(event.clone());
///     Ok::<(), ReporterError>(())
/// });
/// let progress = Progress::builder_arc(reporter)
///     .metric(Metric::new("tasks", "Tasks"))
///     .start()?;
/// progress.finish()?;
/// let event = observed.lock().expect("event lock must be available");
/// assert_eq!(event.as_ref().expect("finish emits an event").phase(), qubit_progress::Phase::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Process-local identifier shared by events from one operation.
    operation_id: u64,
    /// Monotonically increasing attempted-delivery sequence within the
    /// operation.
    sequence: u64,
    /// Lifecycle state represented by this snapshot.
    phase: Phase,
    /// Stage active when the event was emitted, if one was set.
    stage: Option<Stage>,
    /// Immutable correlation metadata shared by events from the operation.
    attributes: Arc<OperationAttributes>,
    /// Metric snapshots in the operation's declaration order.
    metrics: Vec<MetricSnapshot>,
    /// Monotonic elapsed time since the operation started.
    elapsed: Duration,
}

impl Event {
    /// Creates an event after the progress operation has validated state.
    ///
    /// # Parameters
    ///
    /// - `operation_id`: process-local identifier for the operation.
    /// - `sequence`: attempted-delivery sequence within the operation.
    /// - `phase`: lifecycle phase represented by the event.
    /// - `stage`: stage active at delivery, if any.
    /// - `attributes`: immutable operation correlation metadata.
    /// - `metrics`: metric snapshots in declaration order.
    /// - `elapsed`: monotonic time since the operation started.
    ///
    /// # Returns
    ///
    /// The immutable event snapshot containing the supplied validated state.
    #[must_use]
    pub(crate) fn new(
        operation_id: u64,
        sequence: u64,
        phase: Phase,
        stage: Option<Stage>,
        attributes: Arc<OperationAttributes>,
        metrics: Vec<MetricSnapshot>,
        elapsed: Duration,
    ) -> Self {
        Self {
            operation_id,
            sequence,
            phase,
            stage,
            attributes,
            metrics,
            elapsed,
        }
    }
    /// Returns the process-local operation identifier.
    ///
    /// # Returns
    ///
    /// The identifier shared by events emitted for this operation.
    #[must_use]
    #[inline]
    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }
    /// Returns the attempted-delivery sequence.
    ///
    /// # Returns
    ///
    /// The monotonically increasing sequence assigned to this event.
    #[must_use]
    #[inline]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the event lifecycle phase.
    ///
    /// # Returns
    ///
    /// The lifecycle phase represented by this snapshot.
    #[must_use]
    #[inline]
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    /// Returns the active stage, or `None` when the operation has no stage.
    ///
    /// # Returns
    ///
    /// `Some(stage)` when the event was emitted during a stage, or `None` if
    /// no stage was active.
    #[inline]
    pub const fn stage(&self) -> Option<&Stage> {
        self.stage.as_ref()
    }
    /// Returns immutable operation correlation attributes.
    ///
    /// # Returns
    ///
    /// The attributes shared by events emitted for this operation.
    #[must_use]
    #[inline]
    pub fn attributes(&self) -> &OperationAttributes {
        &self.attributes
    }
    /// Returns an attribute value by key, or `None` when the key is absent.
    ///
    /// # Parameters
    ///
    /// - `key`: attribute key to look up.
    ///
    /// # Returns
    ///
    /// `Some(value)` when the key exists, or `None` when it is absent.
    #[inline]
    pub fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key)
    }
    /// Returns all metric snapshots in declaration order.
    ///
    /// # Returns
    ///
    /// The metric snapshots captured for this event.
    #[must_use]
    #[inline]
    pub fn metrics(&self) -> &[MetricSnapshot] {
        &self.metrics
    }
    /// Returns one metric snapshot by stable ID.
    ///
    /// # Parameters
    ///
    /// - `metric_id`: stable metric ID to look up.
    ///
    /// # Returns
    ///
    /// `Some(snapshot)` when a metric has the requested ID, or `None` when no
    /// metric matches.
    pub fn metric(&self, metric_id: &str) -> Option<&MetricSnapshot> {
        self.metrics.iter().find(|metric| metric.id() == metric_id)
    }
    /// Returns monotonic time elapsed since the operation started.
    ///
    /// # Returns
    ///
    /// The elapsed duration captured when this event was emitted.
    #[must_use]
    #[inline]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Serializes an event with its canonical duration representation.
#[cfg(feature = "serde")]
impl Serialize for Event {
    /// Serializes one complete event without exposing internal representation.
    ///
    /// # Type Parameters
    ///
    /// - `S`: serializer implementation that writes the event representation.
    ///
    /// # Parameters
    ///
    /// - `serializer`: destination that encodes the event fields.
    ///
    /// # Returns
    ///
    /// Returns the serializer's success value after writing the event fields.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if any event field cannot be serialized.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(
            &EventWireRef {
                operation_id: self.operation_id,
                sequence: self.sequence,
                phase: self.phase,
                stage: self.stage.as_ref(),
                attributes: self.attributes.as_ref(),
                metrics: &self.metrics,
                elapsed: format_duration(self.elapsed),
            },
            serializer,
        )
    }
}

/// Deserializes and validates a complete event wire representation.
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Event {
    /// Rejects malformed durations and any event that violates public
    /// invariants.
    ///
    /// # Type Parameters
    ///
    /// - `D`: deserializer implementation that provides the event
    ///   representation.
    ///
    /// # Parameters
    ///
    /// - `deserializer`: source that provides the event's wire representation.
    ///
    /// # Returns
    ///
    /// Returns the validated event when deserialization succeeds.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error for malformed fields, invalid
    /// durations, or event data that violates lifecycle and metadata
    /// invariants.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = <EventWire as Deserialize>::deserialize(deserializer)?;
        let elapsed = parse_duration(&wire.elapsed).map_err(Error::custom)?;
        validate_wire_event(&wire, elapsed).map_err(Error::custom)?;
        Ok(Self::new(
            wire.operation_id,
            wire.sequence,
            wire.phase,
            wire.stage,
            Arc::new(wire.attributes),
            wire.metrics,
            elapsed,
        ))
    }
}

/// Produces the shortest exact unit representation used by Event JSON.
///
/// # Parameters
///
/// - `duration`: elapsed time to encode.
///
/// # Returns
///
/// The shortest integer-unit representation that exactly preserves the
/// duration.
#[cfg(feature = "serde")]
#[must_use]
fn format_duration(duration: Duration) -> String {
    if duration.is_zero() {
        return "0s".into();
    }
    let nanoseconds = duration.as_nanos();
    for (unit, suffix) in [
        (3_600_000_000_000_u128, "h"),
        (60_000_000_000_u128, "m"),
        (1_000_000_000_u128, "s"),
        (1_000_000_u128, "ms"),
        (1_000_u128, "us"),
        (1_u128, "ns"),
    ] {
        if nanoseconds.is_multiple_of(unit) {
            return format!("{}{}", nanoseconds / unit, suffix);
        }
    }
    unreachable!("nanosecond duration is always divisible by one nanosecond")
}

/// Parses the strict integer-unit duration grammar used by Event JSON.
///
/// # Parameters
///
/// - `text`: integer duration followed by a supported unit suffix.
///
/// # Returns
///
/// Returns the parsed duration when the text follows the supported grammar.
///
/// # Errors
///
/// Returns an explanation when the unit is unsupported, the amount is not an
/// unsigned integer, or the resulting duration exceeds the supported range.
#[cfg(feature = "serde")]
fn parse_duration(text: &str) -> Result<Duration, String> {
    let (amount, unit) = ["ms", "us", "ns", "h", "m", "s"]
        .into_iter()
        .find_map(|unit| text.strip_suffix(unit).map(|amount| (amount, unit)))
        .ok_or_else(|| "elapsed must end in h, m, s, ms, us, or ns".to_owned())?;
    if amount.is_empty() || !amount.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("elapsed amount must be an unsigned integer".into());
    }
    let multiplier = match unit {
        "h" => 3_600_000_000_000_u128,
        "m" => 60_000_000_000_u128,
        "s" => 1_000_000_000_u128,
        "ms" => 1_000_000_u128,
        "us" => 1_000_u128,
        "ns" => 1_u128,
        _ => unreachable!("unit list is exhaustive"),
    };
    let nanoseconds = amount
        .parse::<u128>()
        .map_err(|_| "elapsed amount is outside the supported range".to_owned())?
        .checked_mul(multiplier)
        .ok_or_else(|| "elapsed duration overflows".to_owned())?;
    let seconds = nanoseconds / 1_000_000_000;
    if seconds > u128::from(u64::MAX) {
        return Err("elapsed duration overflows".into());
    }
    Ok(Duration::new(seconds as u64, (nanoseconds % 1_000_000_000) as u32))
}

/// Validates fields that are only available after Event JSON deserialization.
///
/// # Parameters
///
/// - `wire`: owned event fields decoded from the serialized representation.
/// - `elapsed`: parsed duration to validate against the event phase.
///
/// # Returns
///
/// Returns `Ok(())` when all event invariants hold.
///
/// # Errors
///
/// Returns a validation message when the operation ID, metadata, metric
/// definitions, sequence, elapsed time, or started-event counts are invalid.
#[cfg(feature = "serde")]
fn validate_wire_event(wire: &EventWire, elapsed: Duration) -> Result<(), String> {
    if wire.operation_id == 0 {
        return Err("operation_id must be nonzero".into());
    }
    let definitions = wire.metrics.iter().map(metric_definition).collect::<Vec<_>>();
    validate_attributes(&wire.attributes).map_err(|error| error.to_string())?;
    validate_metrics(&definitions).map_err(|error| error.to_string())?;
    match wire.phase {
        Phase::Started => {
            if wire.sequence != 0 || !elapsed.is_zero() || wire.metrics.iter().any(has_dynamic_counts) {
                return Err("started event must have sequence 0, zero elapsed, and zero counts".into());
            }
        }
        Phase::Running | Phase::Succeeded | Phase::Failed | Phase::Cancelled if wire.sequence == 0 => {
            return Err("non-started event must have a positive sequence".into());
        }
        Phase::Running | Phase::Succeeded | Phase::Failed | Phase::Cancelled => {}
    }
    Ok(())
}

/// Reconstructs stable metric metadata from an immutable metric snapshot.
///
/// # Parameters
///
/// - `snapshot`: immutable metric state whose definition is reconstructed.
///
/// # Returns
///
/// The metric definition carrying the snapshot's stable ID, name, and total.
#[cfg(feature = "serde")]
#[must_use]
fn metric_definition(snapshot: &MetricSnapshot) -> Metric {
    let metric = Metric::new(snapshot.id(), snapshot.name());
    match snapshot.total() {
        Some(total) => metric.total(total),
        None => metric,
    }
}

/// Returns whether one metric snapshot carries any dynamic count.
///
/// # Parameters
///
/// - `snapshot`: metric state to inspect.
///
/// # Returns
///
/// `true` when any completed, active, succeeded, failed, or cancelled count is
/// nonzero.
#[cfg(feature = "serde")]
#[must_use]
const fn has_dynamic_counts(snapshot: &MetricSnapshot) -> bool {
    snapshot.completed() != 0
        || snapshot.active() != 0
        || snapshot.succeeded() != 0
        || snapshot.failed() != 0
        || snapshot.cancelled() != 0
}
