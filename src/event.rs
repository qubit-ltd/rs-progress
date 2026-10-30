// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable event types and their serialized wire representations.
// qubit-style: allow coverage-cfg

#[cfg(all(feature = "json-lines", coverage))]
use serde::Serialize;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::Error;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::Serializer;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::from_str;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::from_value;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::json;
#[cfg(all(feature = "json-lines", coverage))]
use serde_json::to_string;

#[path = "event/event.rs"]
mod definition;
#[cfg(feature = "serde")]
#[path = "internal/event_wire.rs"]
mod event_wire;
#[cfg(feature = "serde")]
#[path = "internal/event_wire_ref.rs"]
mod event_wire_ref;
#[path = "event/phase_mod.rs"]
mod phase_module;

pub use definition::Event;
#[cfg(feature = "serde")]
pub(super) use event_wire::EventWire;
#[cfg(feature = "serde")]
pub(super) use event_wire_ref::EventWireRef;
pub use phase_module::Phase;

/// Exercises serde entry points from the instrumented library build.
#[cfg(all(feature = "json-lines", coverage))]
#[doc(hidden)]
pub fn __coverage_event_serde() {
    let mut value = json!({
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
    let text = to_string(&value).expect("coverage JSON must serialize");
    let event = from_str::<Event>(&text).expect("coverage event must deserialize");

    value["attributes"] = json!({" ": "invalid"});
    assert!(from_value::<Event>(value).is_err());

    let mut output = Vec::new();
    let mut serializer = Serializer::new(&mut output);
    coverage_serialize_event(&event, &mut serializer).expect("coverage event must serialize");
}

/// Serializes an event through the instrumented serde implementation.
///
/// # Parameters
///
/// - `event`: event to serialize.
/// - `serializer`: serializer used by the coverage exercise.
///
/// # Returns
///
/// Returns `Ok(())` when serialization succeeds.
///
/// # Errors
///
/// Returns the serializer's error when writing the event fails.
#[cfg(all(feature = "json-lines", coverage))]
#[inline(never)]
fn coverage_serialize_event(event: &Event, serializer: &mut Serializer<&mut Vec<u8>>) -> Result<(), Error> {
    Serialize::serialize(event, serializer)
}
