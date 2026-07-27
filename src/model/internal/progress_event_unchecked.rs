// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{sync::Arc, time::Duration};

use serde::Deserialize;

use crate::{ProgressCounter, ProgressPhase, ProgressSchema, ProgressStage};
/// Deserialized event representation before counter validation.
#[derive(Deserialize)]
pub(crate) struct ProgressEventUnchecked {
    /// Validated metric schema supplied by serialized input.
    pub(crate) schema: Arc<ProgressSchema>,
    /// Optional operation identifier supplied by serialized input.
    #[serde(default)]
    pub(crate) operation_id: Option<u64>,
    /// Lifecycle phase supplied by serialized input.
    pub(crate) phase: ProgressPhase,
    /// Optional current stage supplied by serialized input.
    pub(crate) stage: Option<ProgressStage>,
    /// Metric counters supplied by serialized input.
    pub(crate) counters: Vec<ProgressCounter>,
    /// Elapsed duration supplied by serialized input.
    #[serde(with = "qubit_datatype::serde::duration_with_unit")]
    pub(crate) elapsed: Duration,
}
