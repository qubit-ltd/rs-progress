// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle phases for immutable progress events.

// Defines the lifecycle phase type and its behavior.
#[path = "phase/phase.rs"]
mod phase_type;

pub use phase_type::Phase;
