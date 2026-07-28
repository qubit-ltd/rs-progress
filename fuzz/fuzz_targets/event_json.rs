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

fuzz_target!(|input: &[u8]| {
    let _ = serde_json::from_slice::<Event>(input);
});
