// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde representation used to validate standalone stages.

use std::sync::Arc;

use serde::Deserialize;

/// Serializable wire representation used to validate standalone stages.
#[derive(Deserialize)]
pub(in crate::stage) struct StageWire {
    /// Machine-readable stage identifier.
    pub(in crate::stage) id: Arc<str>,
    /// Human-readable stage name.
    pub(in crate::stage) name: Arc<str>,
    /// One-based stage position when present.
    pub(in crate::stage) position: Option<u64>,
    /// Number of stages when present.
    pub(in crate::stage) total: Option<u64>,
}
