// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use serde::Deserialize;

/// Deserialized stage representation before weight validation.
#[derive(Deserialize)]
pub(crate) struct ProgressStageUnchecked {
    /// Stable stage identifier supplied by serialized input.
    pub(crate) id: String,
    /// Human-readable stage name supplied by serialized input.
    pub(crate) name: String,
    /// Optional zero-based stage index supplied by serialized input.
    pub(crate) index: Option<usize>,
    /// Optional total stage count supplied by serialized input.
    pub(crate) total_stages: Option<usize>,
    /// Optional relative stage weight supplied by serialized input.
    pub(crate) weight: Option<f64>,
}
