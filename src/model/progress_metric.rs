// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use serde::{
    Deserialize,
    Serialize,
};

/// Describes one metric dimension reported by a progress operation.
///
/// The `id` is stable and machine-readable. It is stored in
/// [`ProgressCounter`](crate::ProgressCounter) values and therefore appears in
/// JSON progress events. The `name` is human-readable and is intended for text
/// reporters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProgressMetric {
    /// Stable machine-readable metric identifier.
    id: String,
    /// Human-readable metric name.
    name: String,
}

impl ProgressMetric {
    /// Creates a metric definition.
    ///
    /// # Parameters
    ///
    /// * `id` - Stable machine-readable metric identifier.
    /// * `name` - Human-readable metric name.
    ///
    /// # Returns
    ///
    /// A metric definition with `id` and `name` recorded.
    #[inline]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
        }
    }

    /// Returns the stable metric identifier.
    ///
    /// # Returns
    ///
    /// The machine-readable metric id.
    #[inline]
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns the human-readable metric name.
    ///
    /// # Returns
    ///
    /// The display name for this metric.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}
