// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable progress events and lifecycle phases.
// qubit-style: allow multiple-public-types
// qubit-style: allow coverage-cfg

use std::{
    sync::Arc,
    time::Duration,
};

use crate::{
    MetricSnapshot,
    OperationAttributes,
    Stage,
};

#[cfg(feature = "serde")]
use crate::{
    Metric,
    validation::{
        validate_attributes,
        validate_metrics,
    },
};

/// Lifecycle phase of one immutable progress event.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// The unique first event.
    Started,
    /// A non-terminal snapshot.
    Running,
    /// Successful terminal event.
    Succeeded,
    /// Failed terminal event.
    Failed,
    /// Cancelled terminal event.
    Cancelled,
}
impl Phase {
    /// Returns the stable lowercase wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Complete immutable snapshot delivered to one reporter call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    operation_id: u64,
    sequence: u64,
    phase: Phase,
    stage: Option<Stage>,
    attributes: Arc<OperationAttributes>,
    metrics: Vec<MetricSnapshot>,
    elapsed: Duration,
}
impl Event {
    /// Creates an event after the progress operation has validated state.
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
    #[must_use]
    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }
    /// Returns the attempted-delivery sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the event lifecycle phase.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    /// Returns optional stage metadata.
    #[must_use]
    pub const fn stage(&self) -> Option<&Stage> {
        self.stage.as_ref()
    }
    /// Returns immutable operation correlation attributes.
    #[must_use]
    pub fn attributes(&self) -> &OperationAttributes {
        &self.attributes
    }
    /// Returns one operation correlation attribute by key.
    #[must_use]
    pub fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key)
    }
    /// Returns all metric snapshots in declaration order.
    #[must_use]
    pub fn metrics(&self) -> &[MetricSnapshot] {
        &self.metrics
    }
    /// Returns one metric snapshot by stable ID.
    #[must_use]
    pub fn metric(&self, metric_id: &str) -> Option<&MetricSnapshot> {
        self.metrics.iter().find(|metric| metric.id() == metric_id)
    }
    /// Returns elapsed monotonic operation time.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Serializes an event with its canonical duration representation.
#[cfg(feature = "serde")]
impl serde::Serialize for Event {
    /// Serializes one complete event without exposing internal representation.
    #[cfg_attr(coverage, inline(never))]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
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
impl<'de> serde::Deserialize<'de> for Event {
    /// Rejects malformed durations and any event that violates public
    /// invariants.
    #[cfg_attr(coverage, inline(never))]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire =
            <EventWire as serde::Deserialize>::deserialize(deserializer)?;
        let elapsed =
            parse_duration(&wire.elapsed).map_err(serde::de::Error::custom)?;
        validate_wire_event(&wire, elapsed)
            .map_err(serde::de::Error::custom)?;
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

/// Borrowed event representation used for serialization.
#[cfg(feature = "serde")]
#[derive(serde::Serialize)]
struct EventWireRef<'a> {
    /// Process-local operation identifier.
    operation_id: u64,
    /// Attempted-delivery sequence.
    sequence: u64,
    /// Event lifecycle phase.
    phase: Phase,
    /// Optional stage metadata.
    stage: Option<&'a Stage>,
    /// Stable operation correlation attributes.
    #[serde(skip_serializing_if = "OperationAttributes::is_empty")]
    attributes: &'a OperationAttributes,
    /// Complete metric snapshots.
    metrics: &'a [MetricSnapshot],
    /// Canonical elapsed duration.
    elapsed: String,
}

/// Owned event representation used before validation during deserialization.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct EventWire {
    /// Process-local operation identifier.
    operation_id: u64,
    /// Attempted-delivery sequence.
    sequence: u64,
    /// Event lifecycle phase.
    phase: Phase,
    /// Optional stage metadata.
    stage: Option<Stage>,
    /// Stable operation correlation attributes.
    #[serde(default)]
    attributes: OperationAttributes,
    /// Complete metric snapshots.
    metrics: Vec<MetricSnapshot>,
    /// Canonical elapsed duration.
    elapsed: String,
}

/// Produces the shortest exact unit representation used by Event JSON.
#[cfg(feature = "serde")]
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
#[cfg(feature = "serde")]
fn parse_duration(text: &str) -> Result<Duration, String> {
    let (amount, unit) = ["ms", "us", "ns", "h", "m", "s"]
        .into_iter()
        .find_map(|unit| text.strip_suffix(unit).map(|amount| (amount, unit)))
        .ok_or_else(|| {
            "elapsed must end in h, m, s, ms, us, or ns".to_owned()
        })?;
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
        .map_err(|_| {
            "elapsed amount is outside the supported range".to_owned()
        })?
        .checked_mul(multiplier)
        .ok_or_else(|| "elapsed duration overflows".to_owned())?;
    let seconds = nanoseconds / 1_000_000_000;
    if seconds > u128::from(u64::MAX) {
        return Err("elapsed duration overflows".into());
    }
    Ok(Duration::new(
        seconds as u64,
        (nanoseconds % 1_000_000_000) as u32,
    ))
}

/// Validates fields that are only available after Event JSON deserialization.
#[cfg(feature = "serde")]
fn validate_wire_event(
    wire: &EventWire,
    elapsed: Duration,
) -> Result<(), String> {
    if wire.operation_id == 0 {
        return Err("operation_id must be nonzero".into());
    }
    let definitions = wire
        .metrics
        .iter()
        .map(metric_definition)
        .collect::<Vec<_>>();
    validate_attributes(&wire.attributes).map_err(|error| error.to_string())?;
    validate_metrics(&definitions).map_err(|error| error.to_string())?;
    match wire.phase {
        Phase::Started => {
            if wire.sequence != 0
                || !elapsed.is_zero()
                || wire.metrics.iter().any(has_dynamic_counts)
            {
                return Err(
                    "started event must have sequence 0, zero elapsed, and zero counts".into(),
                );
            }
        }
        Phase::Running
        | Phase::Succeeded
        | Phase::Failed
        | Phase::Cancelled
            if wire.sequence == 0 =>
        {
            return Err(
                "non-started event must have a positive sequence".into()
            );
        }
        Phase::Running
        | Phase::Succeeded
        | Phase::Failed
        | Phase::Cancelled => {}
    }
    Ok(())
}

/// Reconstructs stable metric metadata from an immutable metric snapshot.
#[cfg(feature = "serde")]
fn metric_definition(snapshot: &MetricSnapshot) -> Metric {
    let metric = Metric::new(snapshot.id(), snapshot.name());
    match snapshot.total() {
        Some(total) => metric.total(total),
        None => metric,
    }
}

/// Returns whether one metric snapshot carries any dynamic count.
#[cfg(feature = "serde")]
const fn has_dynamic_counts(snapshot: &MetricSnapshot) -> bool {
    snapshot.completed() != 0
        || snapshot.active() != 0
        || snapshot.succeeded() != 0
        || snapshot.failed() != 0
        || snapshot.cancelled() != 0
}

/// Exercises serde entry points from the instrumented library build.
#[cfg(all(feature = "json-lines", coverage))]
#[doc(hidden)]
pub fn __coverage_event_serde() {
    let value = serde_json::json!({
        "operation_id": 1,
        "sequence": 0,
        "phase": "started",
        "stage": null,
        "metrics": [{
            "id": "tasks",
            "name": "Tasks",
            "total": null,
            "completed": 0,
            "active": 0,
            "succeeded": 0,
            "failed": 0,
            "cancelled": 0
        }],
        "elapsed": "0s"
    });
    let text =
        serde_json::to_string(&value).expect("coverage JSON must serialize");
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    let event = <Event as serde::Deserialize>::deserialize(&mut deserializer)
        .expect("coverage event must deserialize");
    let mut output = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut output);
    coverage_serialize_event(&event, &mut serializer)
        .expect("coverage event must serialize");
}

#[cfg(all(feature = "json-lines", coverage))]
#[inline(never)]
fn coverage_serialize_event(
    event: &Event,
    serializer: &mut serde_json::Serializer<&mut Vec<u8>>,
) -> Result<(), serde_json::Error> {
    <Event as serde::Serialize>::serialize(event, serializer)
}
