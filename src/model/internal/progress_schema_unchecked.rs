// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use serde::Deserialize;

use crate::ProgressMetric;

/// Deserialized schema representation before duplicate-id validation.
#[derive(Deserialize)]
pub(crate) struct ProgressSchemaUnchecked {
    /// Metric definitions supplied by serialized input.
    pub(crate) metrics: Vec<ProgressMetric>,
}
