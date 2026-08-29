// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzz target for strict Event JSON deserialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_progress::Event;

/// Limits parser work while retaining representative structured JSON inputs.
const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|input: &[u8]| {
    if input.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(event) = serde_json::from_slice::<Event>(input) {
        let encoded = serde_json::to_vec(&event).expect("a deserialized event must serialize to JSON");
        let decoded = serde_json::from_slice::<Event>(&encoded).expect("serialized event JSON must deserialize");
        assert_eq!(decoded, event, "event JSON round trips exactly");
    }
});
